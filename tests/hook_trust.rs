//! End-to-end checks for the `.claude-profile` trust gate.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("silo-trust-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let root = fs::canonicalize(&base).unwrap();
        for d in ["home", "xdg", "repos"] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        let sb = Sandbox { root };
        let out = sb.silo(&["profile", "create", "work", "personal"]);
        assert!(out.status.success(), "{}", stderr(&out));
        sb
    }

    fn cmd(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_silo"));
        c.env("HOME", self.root.join("home"))
            .env("SILO_HOME", self.root.join("silo"))
            .env("XDG_CONFIG_HOME", self.root.join("xdg"))
            .env_remove("SILO_PROFILE")
            .env_remove("CLAUDE_CONFIG_DIR");
        c
    }

    fn silo(&self, args: &[&str]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    fn silo_os(&self, args: &[&Path]) -> Output {
        self.cmd().args(args).output().unwrap()
    }

    fn repo(&self, name: &str, marker: &[u8]) -> PathBuf {
        let dir = self.root.join("repos").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        set_marker(&dir, marker);
        dir
    }

    /// A repo whose marker was trusted as `work`, then rewritten to `content`
    /// (what a later `git pull` could do).
    fn trusted_then_changed(&self, name: &str, content: &[u8]) -> PathBuf {
        let dir = self.repo(name, b"work\n");
        let t = self.trust(&dir);
        assert!(t.status.success(), "{}", stderr(&t));
        set_marker(&dir, content);
        dir
    }

    fn hook_env(&self, dir: &Path) -> Output {
        self.cmd().arg("hook-env").arg(dir).output().unwrap()
    }

    fn trust(&self, dir: &Path) -> Output {
        self.silo_os(&[Path::new("trust"), dir])
    }

    fn trust_file(&self) -> PathBuf {
        self.root.join("xdg/silo/trusted-dirs")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn set_marker(dir: &Path, content: &[u8]) {
    let m = dir.join(".claude-profile");
    let _ = fs::remove_file(&m);
    fs::write(&m, content).unwrap();
    fs::set_permissions(&m, fs::Permissions::from_mode(0o644)).unwrap();
}

fn bin_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_silo"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}
fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn untrusted_dir_does_not_switch_and_prints_notice() {
    let sb = Sandbox::new();
    let repo = sb.repo("clone", b"work\n");
    let out = sb.hook_env(&repo);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "", "untrusted marker must produce no exports");
    let err = stderr(&out);
    assert!(err.contains("untrusted"), "{err}");
    assert!(err.contains("wants `work`"), "{err}");
    assert!(err.contains("silo trust"), "{err}");
    assert_eq!(err.lines().count(), 1, "notice should be one line: {err}");
}

#[test]
fn trusted_dir_switches() {
    let sb = Sandbox::new();
    let repo = sb.repo("mine", b"work\n");
    let t = sb.trust(&repo);
    assert!(t.status.success(), "{}", stderr(&t));
    let s = stdout(&sb.hook_env(&repo));
    assert!(s.contains("export SILO_PROFILE='work'"), "{s}");
    assert!(s.contains("export CLAUDE_CONFIG_DIR="), "{s}");

    // Already active: nothing to do.
    let again = sb
        .cmd()
        .env("SILO_PROFILE", "work")
        .arg("hook-env")
        .arg(&repo)
        .output()
        .unwrap();
    assert_eq!(stdout(&again), "");

    let mode = fs::metadata(sb.trust_file()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let raw = fs::read_to_string(sb.trust_file()).unwrap();
    assert!(
        raw.contains(&format!("{}\twork\n", repo.display())),
        "{raw}"
    );
}

#[test]
fn marker_change_after_trust_does_not_switch() {
    let sb = Sandbox::new();
    let repo = sb.trusted_then_changed("pulled", b"personal\n");
    let out = sb.hook_env(&repo);
    assert_eq!(stdout(&out), "", "changed marker must not switch");
    let err = stderr(&out);
    assert!(err.contains("marker now wants `personal`"), "{err}");
    assert!(err.contains("trusted for `work`"), "{err}");
    assert!(err.contains("re-run `silo trust"), "{err}");
    // Re-trusting records the new name.
    assert!(sb.trust(&repo).status.success());
    assert!(stdout(&sb.hook_env(&repo)).contains("SILO_PROFILE='personal'"));
}

#[test]
fn legacy_one_field_entry_fails_closed() {
    let sb = Sandbox::new();
    let repo = sb.repo("legacy", b"work");
    fs::create_dir_all(sb.trust_file().parent().unwrap()).unwrap();
    fs::write(sb.trust_file(), format!("{}\n", repo.display())).unwrap();
    fs::set_permissions(sb.trust_file(), fs::Permissions::from_mode(0o600)).unwrap();
    let out = sb.hook_env(&repo);
    assert_eq!(stdout(&out), "");
    let err = stderr(&out);
    assert!(err.contains("no profile name"), "{err}");
    assert_eq!(err.lines().count(), 1, "{err}");
    let list = stdout(&sb.silo(&["trust", "--list"]));
    assert!(list.contains("no profile recorded"), "{list}");
    assert!(sb.trust(&repo).status.success());
    assert!(stdout(&sb.hook_env(&repo)).contains("SILO_PROFILE='work'"));
}

#[test]
fn whitespace_does_not_trust_a_sibling() {
    let sb = Sandbox::new();
    let foo = sb.repo("foo", b"work");
    let foo_sp = sb.repo("foo ", b"work");
    // Refused at trust time.
    let t = sb.trust(&foo_sp);
    assert!(!t.status.success());
    assert!(stderr(&t).contains("whitespace"), "{}", stderr(&t));
    // A hand-written entry with a trailing space is not trimmed onto `foo`.
    fs::create_dir_all(sb.trust_file().parent().unwrap()).unwrap();
    fs::write(sb.trust_file(), format!("{} \twork\n", foo.display())).unwrap();
    fs::set_permissions(sb.trust_file(), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(stdout(&sb.hook_env(&foo)), "");
    assert!(stderr(&sb.hook_env(&foo)).contains("untrusted"));
}

#[test]
fn trust_requires_a_valid_marker() {
    let sb = Sandbox::new();
    let empty = sb.root.join("repos/nomarker");
    fs::create_dir_all(&empty).unwrap();
    let t = sb.trust(&empty);
    assert!(!t.status.success());
    assert!(stderr(&t).contains("no .claude-profile"), "{}", stderr(&t));
    let bad = sb.repo("bad", b"$(id)");
    assert!(!sb.trust(&bad).status.success());
    let missing = sb.repo("missing", b"nosuchprofile");
    assert!(!sb.trust(&missing).status.success());
    assert!(
        !sb.trust_file().exists()
            || !fs::read_to_string(sb.trust_file())
                .unwrap()
                .contains("repos/")
    );
}

#[test]
fn untrust_and_list() {
    let sb = Sandbox::new();
    let repo = sb.repo("mine", b"work");
    assert!(sb.trust(&repo).status.success());
    assert!(sb.trust(&repo).status.success(), "trust is idempotent");
    let list = stdout(&sb.silo(&["trust", "--list"]));
    let want = format!("{}\twork", repo.display());
    assert_eq!(list.lines().filter(|l| *l == want).count(), 1, "{list}");
    assert!(sb.silo_os(&[Path::new("untrust"), &repo]).status.success());
    assert_eq!(stdout(&sb.hook_env(&repo)), "");
}

#[test]
fn link_trusts_the_directory_for_that_name() {
    let sb = Sandbox::new();
    let dir = sb.root.join("repos/linked");
    fs::create_dir_all(&dir).unwrap();
    let out = sb.silo_os(&[Path::new("link"), Path::new("personal"), &dir]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&sb.hook_env(&dir)).contains("export SILO_PROFILE='personal'"));
    set_marker(&dir, b"work");
    assert_eq!(stdout(&sb.hook_env(&dir)), "");
}

#[test]
fn malicious_names_never_switch_even_when_dir_trusted() {
    let sb = Sandbox::new();
    let long = "a".repeat(65);
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("subst", b"$(touch pwned)".to_vec()),
        ("backtick", b"`touch pwned`".to_vec()),
        ("semi", b"work;touch pwned".to_vec()),
        ("traverse", b"../work".to_vec()),
        ("slash", b"profiles/work".to_vec()),
        ("quote", b"work'".to_vec()),
        ("twolines", b"work\npersonal".to_vec()),
        ("space", b"work personal".to_vec()),
        ("nonutf8", vec![0xff, 0xfe, b'w']),
        ("long", long.into_bytes()),
        ("huge", vec![b'a'; 4096]),
        ("empty", b"\n".to_vec()),
    ];
    for (name, content) in cases {
        let repo = sb.trusted_then_changed(name, &content);
        let out = sb.hook_env(&repo);
        assert!(out.status.success(), "{name}");
        assert_eq!(stdout(&out), "", "{name}: must not emit exports");
        assert!(
            stderr(&out).contains("ignoring"),
            "{name}: {}",
            stderr(&out)
        );
    }
}

#[test]
fn fifo_marker_does_not_hang() {
    let sb = Sandbox::new();
    let repo = sb.trusted_then_changed("fifo", b"work");
    let m = repo.join(".claude-profile");
    fs::remove_file(&m).unwrap();
    let st = Command::new("mkfifo").arg(&m).status().unwrap();
    assert!(st.success());
    fs::set_permissions(&m, fs::Permissions::from_mode(0o644)).unwrap();
    let out = sb.hook_env(&repo);
    assert_eq!(stdout(&out), "");
    assert!(
        stderr(&out).contains("not a regular file"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn symlinked_dirs_resolve_to_realpath() {
    let sb = Sandbox::new();
    let real = sb.repo("real", b"work");
    let link = sb.root.join("repos/alias");
    symlink(&real, &link).unwrap();

    assert!(sb.trust(&link).status.success());
    let list = stdout(&sb.silo(&["trust", "--list"]));
    assert!(
        list.lines()
            .any(|l| l == format!("{}\twork", real.display())),
        "{list}"
    );
    assert!(!list.contains("alias"), "{list}");

    assert!(stdout(&sb.hook_env(&link)).contains("SILO_PROFILE='work'"));
    assert!(stdout(&sb.hook_env(&real)).contains("SILO_PROFILE='work'"));

    // A symlink inside a trusted dir pointing at an untrusted repo is judged by
    // its target, not the trusted-looking path.
    let other = sb.repo("other", b"personal");
    let sneaky = real.join("vendor");
    symlink(&other, &sneaky).unwrap();
    assert_eq!(stdout(&sb.hook_env(&sneaky)), "");
    assert!(stderr(&sb.hook_env(&sneaky)).contains("untrusted"));
}

#[test]
fn nested_real_dir_marker_inside_trusted_repo_is_untrusted() {
    let sb = Sandbox::new();
    let repo = sb.repo("parent", b"work");
    assert!(sb.trust(&repo).status.success());
    let sub = repo.join("vendor/dep");
    fs::create_dir_all(&sub).unwrap();
    fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();
    set_marker(&sub, b"personal");
    let out = sb.hook_env(&sub);
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("untrusted"), "{}", stderr(&out));
    // Without an explicit dir, hook-env resolves the nearest marker (the nested one).
    let out = sb.cmd().arg("hook-env").current_dir(&sub).output().unwrap();
    assert_eq!(stdout(&out), "");
}

#[test]
fn symlinked_marker_is_refused() {
    let sb = Sandbox::new();
    let dir = sb.trusted_then_changed("markerlink", b"work");
    let target = sb.root.join("home/elsewhere");
    fs::write(&target, "work").unwrap();
    fs::remove_file(dir.join(".claude-profile")).unwrap();
    symlink(&target, dir.join(".claude-profile")).unwrap();
    let out = sb.hook_env(&dir);
    assert_eq!(stdout(&out), "");
    assert!(stderr(&out).contains("symlink"), "{}", stderr(&out));
}

#[test]
fn world_or_group_writable_dir_never_switches() {
    let sb = Sandbox::new();
    for (name, mode) in [("ww", 0o777), ("gw", 0o775)] {
        let repo = sb.repo(name, b"work");
        assert!(sb.trust(&repo).status.success());
        fs::set_permissions(&repo, fs::Permissions::from_mode(mode)).unwrap();
        let out = sb.hook_env(&repo);
        assert_eq!(stdout(&out), "", "{name}");
        assert!(
            stderr(&out).contains("unsafe permissions"),
            "{}",
            stderr(&out)
        );
        // And it cannot be newly trusted.
        assert!(!sb.trust(&repo).status.success(), "{name}");
    }
}

#[test]
fn world_or_group_writable_marker_never_switches() {
    let sb = Sandbox::new();
    for (name, mode) in [("wwm", 0o666), ("gwm", 0o664)] {
        let repo = sb.repo(name, b"work");
        assert!(sb.trust(&repo).status.success());
        fs::set_permissions(
            repo.join(".claude-profile"),
            fs::Permissions::from_mode(mode),
        )
        .unwrap();
        let out = sb.hook_env(&repo);
        assert_eq!(stdout(&out), "", "{name}");
        assert!(stderr(&out).contains("unsafe"), "{}", stderr(&out));
    }
}

#[test]
fn group_writable_trust_file_is_ignored() {
    let sb = Sandbox::new();
    let repo = sb.repo("mine", b"work");
    assert!(sb.trust(&repo).status.success());
    fs::set_permissions(sb.trust_file(), fs::Permissions::from_mode(0o666)).unwrap();
    let out = sb.hook_env(&repo);
    assert_eq!(stdout(&out), "");
    assert!(
        stderr(&out).contains("unsafe trust list"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn which_reports_untrusted_marker() {
    let sb = Sandbox::new();
    let repo = sb.repo("clone", b"work");
    let out = sb.silo_os(&[Path::new("which"), &repo]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("(ignored)"), "{}", stdout(&out));
    assert!(stdout(&out).contains("untrusted"), "{}", stdout(&out));
}

#[test]
fn use_skips_env_names_that_are_not_shell_identifiers() {
    let sb = Sandbox::new();
    let out = sb
        .cmd()
        .env("CLAUDE_EVIL;touch pwned;X", "1")
        .env("CLAUDE_OK_VAR", "1")
        .args(["use", "work"])
        .output()
        .unwrap();
    let s = stdout(&out);
    assert!(s.contains("unset CLAUDE_OK_VAR\n"), "{s}");
    assert!(!s.contains("EVIL"), "{s}");
}

#[test]
fn hook_uses_absolute_binary_path() {
    let sb = Sandbox::new();
    let h = stdout(&sb.silo(&["hook"]));
    let exe = fs::canonicalize(env!("CARGO_BIN_EXE_silo")).unwrap();
    assert!(h.contains(&format!("'{}' hook-env", exe.display())), "{h}");
    assert!(!h.contains("command silo"), "{h}");
}

/// Shell script shared by the bash and zsh end-to-end tests. The repo `evil`
/// ships its own `bin/silo` and the shell has a relative `bin` on PATH, so a
/// PATH lookup would run the repo's binary.
fn e2e_script(sb: &Sandbox, evil: &Path, clone: &Path, mine: &Path, call: &str) -> String {
    let _ = sb;
    format!(
        r#"set -u
eval "$(silo hook)"
PATH="bin:$PATH"
cd '{evil}'{call}
echo "evil=${{SILO_PROFILE:-none}}"
cd '{clone}'{call}
echo "clone=${{SILO_PROFILE:-none}}"
cd '{mine}'{call}
echo "mine=${{SILO_PROFILE:-none}}"
cd /{call}
cd '{clone}'{call}
echo "after=${{SILO_PROFILE:-none}}"
"#,
        evil = evil.display(),
        clone = clone.display(),
        mine = mine.display(),
    )
}

fn e2e_setup(sb: &Sandbox) -> (PathBuf, PathBuf, PathBuf) {
    let evil = sb.trusted_then_changed("evil", b"$(touch pwned)");
    let fake = evil.join("bin/silo");
    fs::create_dir_all(fake.parent().unwrap()).unwrap();
    fs::write(
        &fake,
        "#!/bin/sh\ntouch \"$PWD/pwned-bin\"\necho 'export SILO_PROFILE=hijacked'\n",
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let clone = sb.repo("clone", b"work");
    let mine = sb.repo("mine", b"work");
    assert!(sb.trust(&mine).status.success());
    (evil, clone, mine)
}

fn run_shell(sb: &Sandbox, shell: &str, flags: &[&str], script: &str) -> Output {
    Command::new(shell)
        .env("HOME", sb.root.join("home"))
        .env("SILO_HOME", sb.root.join("silo"))
        .env("XDG_CONFIG_HOME", sb.root.join("xdg"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin_dir().display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env_remove("SILO_PROFILE")
        .env_remove("CLAUDE_CONFIG_DIR")
        .current_dir(sb.root.join("home"))
        .args(flags)
        .arg(script)
        .output()
        .unwrap()
}

fn assert_e2e(out: &Output, dirs: &[&Path]) {
    let s = stdout(out);
    assert!(s.contains("evil=none"), "{s}\n{}", stderr(out));
    assert!(s.contains("clone=none"), "{s}");
    assert!(s.contains("mine=work"), "{s}");
    // Leaving a trusted repo keeps the profile; entering an untrusted one does
    // not switch it to anything else.
    assert!(s.contains("after=work"), "{s}");
    assert!(!s.contains("hijacked"), "{s}");
    assert!(stderr(out).contains("untrusted"), "{}", stderr(out));
    for d in dirs {
        assert!(
            !d.join("pwned").exists(),
            "marker content executed in {}",
            d.display()
        );
        assert!(
            !d.join("pwned-bin").exists(),
            "repo-local silo ran in {}",
            d.display()
        );
    }
}

#[test]
fn bash_hook_end_to_end() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let sb = Sandbox::new();
    let (evil, clone, mine) = e2e_setup(&sb);
    let script = e2e_script(&sb, &evil, &clone, &mine, " && _silo_auto");
    let out = run_shell(&sb, "bash", &["--norc", "--noprofile", "-c"], &script);
    assert_e2e(&out, &[&evil, &clone, &mine, &sb.root.join("home")]);
}

#[test]
fn zsh_hook_end_to_end() {
    if Command::new("zsh").arg("--version").output().is_err() {
        eprintln!("zsh not installed; skipping");
        return;
    }
    let sb = Sandbox::new();
    let (evil, clone, mine) = e2e_setup(&sb);
    // zsh: the chpwd hook fires on every cd, no explicit call.
    let script = e2e_script(&sb, &evil, &clone, &mine, "");
    let out = run_shell(&sb, "zsh", &["-f", "-c"], &script);
    assert_e2e(&out, &[&evil, &clone, &mine, &sb.root.join("home")]);
}
