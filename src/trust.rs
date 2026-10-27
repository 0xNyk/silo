//! Trust gate for repository-local `.claude-profile` markers.
//!
//! A marker is repository content. A cloned third-party repo can ship one, so the
//! shell hook only honours a marker when the directory that holds it is on a
//! user-owned allowlist (`silo trust`) *for the profile name the marker holds*.
//! Without that gate, `cd` into a hostile clone (or a `git pull` that edits the
//! marker) could switch the shell to an existing credential profile.

use crate::cli::{TrustArgs, UntrustArgs};
use crate::paths::{self, PROFILE_MARKER};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Profile names read from a marker are limited to this many bytes.
pub const MAX_MARKER_NAME_LEN: usize = 64;
/// A marker holds one name; anything larger is refused without parsing.
const MAX_MARKER_BYTES: u64 = 256;
/// Group- or world-writable: refused for the trust list, marker, and marker dir.
const UNSAFE_MODE_BITS: u32 = 0o022;
const TRUST_FILE_HEADER: &str = "# silo trusted directories: <realpath><TAB><profile> per line. Manage with `silo trust` / `silo untrust`.\n";

/// One allowlist line. `name` is `None` for a legacy (pre per-profile) entry,
/// which never activates until re-trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustEntry {
    pub path: PathBuf,
    pub name: Option<String>,
}

/// Why a valid marker in a safe directory is not honoured.
#[derive(Debug, PartialEq, Eq)]
pub enum Why {
    /// Directory not on the list.
    NotTrusted,
    /// Directory trusted for a different profile (marker changed after trust).
    NameChanged { trusted: String },
    /// Entry written before trust recorded the profile name.
    Legacy,
}

/// What the hook should do with a marker directory.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Trusted for this exact name, safe, valid: switch to this profile.
    Activate(String),
    /// Marker is valid but not trusted for the name it holds.
    Untrusted {
        root: PathBuf,
        wants: String,
        why: Why,
    },
    /// Marker or directory failed a safety check; never switch.
    Refused { root: PathBuf, reason: String },
}

impl Decision {
    /// One-line, human-readable explanation for anything but `Activate`.
    pub fn notice(&self) -> Option<String> {
        match self {
            Decision::Activate(_) => None,
            Decision::Untrusted { root, wants, why } => {
                let q = shell_quote(&root.display().to_string());
                Some(match why {
                    Why::NotTrusted => format!(
                        "ignoring untrusted {PROFILE_MARKER} in {} (wants `{wants}`); run `silo trust {q}` to allow it",
                        root.display()
                    ),
                    Why::NameChanged { trusted } => format!(
                        "ignoring {PROFILE_MARKER} in {}: marker now wants `{wants}` (trusted for `{trusted}`); re-run `silo trust {q}` to allow it",
                        root.display()
                    ),
                    Why::Legacy => format!(
                        "ignoring {PROFILE_MARKER} in {}: trust entry has no profile name (older silo); marker wants `{wants}`; re-run `silo trust {q}` to allow it",
                        root.display()
                    ),
                })
            }
            Decision::Refused { root, reason } => Some(format!(
                "ignoring {PROFILE_MARKER} in {}: {reason}",
                root.display()
            )),
        }
    }
}

/// `${XDG_CONFIG_HOME:-~/.config}/silo/trusted-dirs`. Deliberately outside the
/// profile tree and never inside a repository.
pub fn trust_file_path() -> Result<PathBuf> {
    if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(&v);
        if !v.is_empty() && p.is_absolute() {
            return Ok(p.join("silo").join("trusted-dirs"));
        }
    }
    Ok(paths::home_dir()?
        .join(".config")
        .join("silo")
        .join("trusted-dirs"))
}

/// Canonical (realpath) form of a directory, as stored in the allowlist.
pub fn canonical_dir(path: &Path) -> Result<PathBuf> {
    let canon = fs::canonicalize(path).with_context(|| format!("resolve {}", path.display()))?;
    if !canon.is_dir() {
        bail!("{} is not a directory", canon.display());
    }
    Ok(canon)
}

/// Parse allowlist text. Only a trailing `\r` is stripped (`lines()` drops the
/// `\n`): paths are compared byte-for-byte, so `/x/foo ` never matches `/x/foo`.
/// A line without a valid `<TAB><name>` suffix is a legacy entry (fail closed).
pub fn parse_trusted(raw: &str) -> Vec<TrustEntry> {
    raw.lines()
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once('\t') {
            Some((p, n)) if validate_marker_name(n).is_ok() => TrustEntry {
                path: PathBuf::from(p),
                name: Some(n.to_string()),
            },
            Some((p, _)) => TrustEntry {
                path: PathBuf::from(p),
                name: None,
            },
            None => TrustEntry {
                path: PathBuf::from(l),
                name: None,
            },
        })
        .collect()
}

/// Load the allowlist. A missing file means nothing is trusted. A file that is
/// not owned by the current user, or is writable by group/others, is ignored.
pub fn load_trusted() -> Result<Vec<TrustEntry>> {
    let path = trust_file_path()?;
    let meta = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e).with_context(|| format!("stat {}", path.display())),
    };
    if !meta.file_type().is_file() {
        bail!(
            "{} is not a regular file; ignoring trust list",
            path.display()
        );
    }
    check_owner_and_mode(&meta, UNSAFE_MODE_BITS)
        .with_context(|| format!("unsafe trust list {}; ignoring it", path.display()))?;
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(parse_trusted(&raw))
}

fn save_trusted(entries: &[TrustEntry]) -> Result<()> {
    let path = trust_file_path()?;
    let parent = path.parent().context("trust file has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }
    let mut body = String::from(TRUST_FILE_HEADER);
    for e in entries {
        body.push_str(&e.path.display().to_string());
        if let Some(n) = &e.name {
            body.push('\t');
            body.push_str(n);
        }
        body.push('\n');
    }
    // Write to a sibling temp file, then rename, so a crash never truncates the list.
    let tmp = parent.join(format!(".trusted-dirs.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&tmp);
    {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts
            .open(&tmp)
            .with_context(|| format!("write {}", tmp.display()))?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

/// Strict validation for a name that came from repository content.
pub fn validate_marker_name(name: &str) -> Result<()> {
    if name.len() > MAX_MARKER_NAME_LEN {
        bail!("profile name in {PROFILE_MARKER} is longer than {MAX_MARKER_NAME_LEN} bytes");
    }
    paths::validate_name(name)
}

/// Open the marker without following a symlink (and without blocking on a
/// FIFO), then check type/owner/mode/size on the opened fd, so the file that
/// is checked is the file that is read.
fn open_marker(marker: &Path) -> Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    match opts.open(marker) {
        Ok(f) => Ok(f),
        #[cfg(unix)]
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => bail!(
            "{} is a symlink (symlinked markers are refused)",
            marker.display()
        ),
        Err(e) => Err(e).with_context(|| format!("open {}", marker.display())),
    }
}

fn read_marker_strict(marker: &Path) -> Result<String> {
    let f = open_marker(marker)?;
    let meta = f
        .metadata()
        .with_context(|| format!("stat {}", marker.display()))?;
    if !meta.file_type().is_file() {
        bail!("{} is not a regular file", marker.display());
    }
    check_owner_and_mode(&meta, UNSAFE_MODE_BITS)
        .with_context(|| format!("unsafe marker {}", marker.display()))?;
    let mut raw = Vec::new();
    f.take(MAX_MARKER_BYTES + 1)
        .read_to_end(&mut raw)
        .with_context(|| format!("read {}", marker.display()))?;
    if raw.len() as u64 > MAX_MARKER_BYTES {
        bail!(
            "{} is larger than {MAX_MARKER_BYTES} bytes",
            marker.display()
        );
    }
    let text = String::from_utf8(raw).context("marker is not UTF-8")?;
    let name = text.trim().to_string();
    validate_marker_name(&name)?;
    Ok(name)
}

/// Canonicalise `root` and require it to be owned by us and not group/world
/// writable. Returns the canonical path.
fn safe_marker_dir(root: &Path) -> Result<PathBuf> {
    let canon = canonical_dir(root)?;
    let meta = fs::metadata(&canon).with_context(|| format!("stat {}", canon.display()))?;
    check_owner_and_mode(&meta, UNSAFE_MODE_BITS).context("directory")?;
    Ok(canon)
}

/// Decide what the hook may do for the marker in `root` (the directory holding
/// `.claude-profile`). Returns `Activate` only when the directory is trusted for
/// exactly the name in the marker and every safety check passes.
pub fn evaluate(root: &Path) -> Decision {
    let canon = match safe_marker_dir(root) {
        Ok(c) => c,
        Err(e) => {
            return Decision::Refused {
                root: fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()),
                reason: format!("{e:#}"),
            }
        }
    };
    let refused = |reason: String| Decision::Refused {
        root: canon.clone(),
        reason,
    };
    let name = match read_marker_strict(&canon.join(PROFILE_MARKER)) {
        Ok(n) => n,
        Err(e) => return refused(format!("{e:#}")),
    };
    let entries = match load_trusted() {
        Ok(e) => e,
        Err(e) => return refused(format!("{e:#}")),
    };
    let untrusted = |why| Decision::Untrusted {
        root: canon.clone(),
        wants: name.clone(),
        why,
    };
    match entries.iter().find(|e| e.path == canon) {
        None => return untrusted(Why::NotTrusted),
        Some(TrustEntry { name: None, .. }) => return untrusted(Why::Legacy),
        Some(TrustEntry {
            name: Some(trusted),
            ..
        }) if trusted != &name => {
            return untrusted(Why::NameChanged {
                trusted: trusted.clone(),
            })
        }
        Some(_) => {}
    }
    if let Err(e) = paths::require_exists(&name) {
        return refused(format!("{e:#}"));
    }
    Decision::Activate(name)
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// Fails when `meta` is owned by another user or has any bit of `forbidden`
/// (e.g. 0o022 group/world-writable) set.
#[cfg(unix)]
fn check_owner_and_mode(meta: &fs::Metadata, forbidden: u32) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let uid = current_uid();
    if meta.uid() != uid {
        bail!("is owned by uid {} (you are uid {uid})", meta.uid());
    }
    if meta.mode() & forbidden != 0 {
        bail!("has unsafe permissions {:o}", meta.mode() & 0o7777);
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_owner_and_mode(_meta: &fs::Metadata, _forbidden: u32) -> Result<()> {
    Ok(())
}

fn resolve_arg(path: Option<String>) -> Result<PathBuf> {
    match path {
        Some(p) => Ok(paths::expand_user(&p)),
        None => std::env::current_dir().context("current dir"),
    }
}

/// Trust `dir` for exactly `name` (replacing any earlier entry for that dir).
/// Used by `silo trust` (with the marker's current name) and `silo link`.
pub fn trust_dir(dir: &Path, name: &str) -> Result<PathBuf> {
    validate_marker_name(name)?;
    let canon = canonical_dir(dir)?;
    let as_str = canon
        .to_str()
        .context("refusing to trust a non-UTF-8 path")?;
    if as_str.chars().any(|c| c == '\n' || c == '\r' || c == '\t') {
        bail!("refusing to trust a path containing a newline or tab");
    }
    if as_str.split('/').any(|seg| seg != seg.trim()) {
        bail!("refusing to trust a path with leading or trailing whitespace in a component: {as_str:?}");
    }
    let meta = fs::metadata(&canon)?;
    check_owner_and_mode(&meta, UNSAFE_MODE_BITS).with_context(|| {
        format!(
            "refusing to trust {}: the hook never switches in directories owned by another user or writable by group/others",
            canon.display()
        )
    })?;
    let mut entries = load_trusted()?;
    entries.retain(|e| e.path != canon);
    entries.push(TrustEntry {
        path: canon.clone(),
        name: Some(name.to_string()),
    });
    save_trusted(&entries)?;
    Ok(canon)
}

pub fn cmd_trust(args: TrustArgs) -> Result<()> {
    if args.list {
        let entries = load_trusted()?;
        if entries.is_empty() {
            println!("(no trusted directories)  {}", trust_file_path()?.display());
        }
        for e in entries {
            match e.name {
                Some(n) => println!("{}\t{n}", e.path.display()),
                None => println!(
                    "{}\t(no profile recorded - re-run `silo trust`)",
                    e.path.display()
                ),
            }
        }
        return Ok(());
    }
    let dir =
        safe_marker_dir(&resolve_arg(args.path)?).context("refusing to trust this directory")?;
    let marker = dir.join(PROFILE_MARKER);
    if fs::symlink_metadata(&marker).is_err() {
        bail!(
            "no {PROFILE_MARKER} in {} - use `silo link <name>` to create and trust one",
            dir.display()
        );
    }
    let name = read_marker_strict(&marker).context("refusing to trust")?;
    paths::require_exists(&name)?;
    let canon = trust_dir(&dir, &name)?;
    println!("trusted {} for `{name}`", canon.display());
    println!(
        "the hook applies it on next cd into this directory; now: eval \"$(silo use {name})\""
    );
    Ok(())
}

pub fn cmd_untrust(args: UntrustArgs) -> Result<()> {
    let raw = resolve_arg(args.path)?;
    // Canonicalise when the directory still exists; otherwise remove the literal
    // entry so stale paths can be cleaned up.
    let key = fs::canonicalize(&raw).unwrap_or(raw);
    let mut entries = load_trusted()?;
    let before = entries.len();
    entries.retain(|e| e.path != key);
    if entries.len() == before {
        println!("not trusted: {}", key.display());
        return Ok(());
    }
    save_trusted(&entries)?;
    println!("untrusted {}", key.display());
    Ok(())
}

/// Hidden command used by the shell hook. Prints shell exports on stdout only
/// when the marker is trusted and valid; prints a one-line notice on stderr
/// otherwise. Never prints marker content into the eval stream.
pub fn cmd_hook_env(path: Option<String>) -> Result<()> {
    let root = match path {
        Some(p) => PathBuf::from(p),
        None => {
            let cwd = std::env::current_dir()?;
            match paths::find_marker(&cwd).and_then(|m| m.parent().map(Path::to_path_buf)) {
                Some(r) => r,
                None => return Ok(()),
            }
        }
    };
    if fs::symlink_metadata(root.join(PROFILE_MARKER)).is_err() {
        return Ok(());
    }
    let decision = evaluate(&root);
    if let Decision::Activate(name) = &decision {
        if std::env::var("SILO_PROFILE").ok().as_deref() != Some(name.as_str()) {
            print!("{}", crate::launch::render_use(name)?);
        }
    } else if let Some(msg) = decision.notice() {
        eprintln!("silo: {msg}");
    }
    Ok(())
}

/// Quote for POSIX shells; bare when only safe characters are present.
pub fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+@".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_name_validation() {
        assert!(validate_marker_name("work").is_ok());
        assert!(validate_marker_name("client_a-1").is_ok());
        for bad in [
            "",
            "../work",
            "a/b",
            "a\\b",
            "work;rm -rf ~",
            "$(id)",
            "`id`",
            "work\nother",
            "a b",
            "wörk",
            "'",
            "work|cat",
        ] {
            assert!(validate_marker_name(bad).is_err(), "accepted {bad:?}");
        }
        assert!(validate_marker_name(&"a".repeat(MAX_MARKER_NAME_LEN)).is_ok());
        assert!(validate_marker_name(&"a".repeat(MAX_MARKER_NAME_LEN + 1)).is_err());
    }

    #[test]
    fn quote_paths() {
        assert_eq!(shell_quote("/a/b-c"), "/a/b-c");
        assert_eq!(shell_quote("/a b"), "'/a b'");
        assert_eq!(shell_quote("/it's"), r"'/it'\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn parse_keeps_whitespace_and_fails_closed() {
        let raw = "# header\n/x/foo \twork\r\n/x/bar\n/x/baz\t$(id)\n\n/x/ok\tpersonal\n";
        let e = parse_trusted(raw);
        assert_eq!(e.len(), 4);
        assert_eq!(e[0].path, PathBuf::from("/x/foo "));
        assert_eq!(e[0].name.as_deref(), Some("work"));
        assert_eq!(e[1].name, None, "one-field legacy entry");
        assert_eq!(e[2].name, None, "invalid name fails closed");
        assert_eq!(e[3].name.as_deref(), Some("personal"));
        assert!(!e.iter().any(|x| x.path == Path::new("/x/foo")));
    }
}
