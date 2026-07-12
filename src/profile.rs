use crate::cli::{AuthMode, DefaultCmd, LinkArgs, ProfileCmd, UnlinkArgs, WhichArgs};
use crate::config::{self, Config};
use crate::paths::{self, ensure_dir, PROFILE_MARKER};
use crate::share;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    pub name: String,
    pub auth_mode: AuthModeMeta,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthModeMeta {
    Oauth,
    SetupToken,
    ApiKey,
    Bedrock,
    Vertex,
    Foundry,
}

impl From<AuthMode> for AuthModeMeta {
    fn from(m: AuthMode) -> Self {
        match m {
            AuthMode::Oauth => Self::Oauth,
            AuthMode::SetupToken => Self::SetupToken,
            AuthMode::ApiKey => Self::ApiKey,
            AuthMode::Bedrock => Self::Bedrock,
            AuthMode::Vertex => Self::Vertex,
            AuthMode::Foundry => Self::Foundry,
        }
    }
}

impl AuthModeMeta {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oauth => "oauth",
            Self::SetupToken => "setup-token",
            Self::ApiKey => "api-key",
            Self::Bedrock => "bedrock",
            Self::Vertex => "vertex",
            Self::Foundry => "foundry",
        }
    }
}

pub fn cmd_profile(action: ProfileCmd) -> Result<()> {
    match action {
        ProfileCmd::Create {
            names,
            mode,
            copy_settings,
        } => {
            let cfg = config::load()?;
            let mode_meta = AuthModeMeta::from(mode);
            if names.len() as u32 > config::MAX_BULK_CREATE {
                bail!(
                    "refusing to create {} profiles in one shot (soft limit {})",
                    names.len(),
                    config::MAX_BULK_CREATE
                );
            }
            for name in &names {
                if paths::profile_dir(name)?.exists() {
                    println!("exists:  {name}");
                    continue;
                }
                create_profile(name, mode_meta, copy_settings, &cfg)?;
                println!(
                    "created: {name}  ({})  {}",
                    mode_meta.as_str(),
                    paths::profile_dir(name)?.display()
                );
            }
            if names.len() == 1 {
                println!("next: silo auth login {}", names[0]);
            } else {
                println!(
                    "next: silo auth login <name>  ({} profiles in this batch)",
                    names.len()
                );
            }
        }
        ProfileCmd::List => list_profiles()?,
        ProfileCmd::Show { name } => show_profile(&name)?,
        ProfileCmd::Delete { name, yes } => delete_profile(&name, yes)?,
    }
    Ok(())
}

pub fn create_profile(
    name: &str,
    mode: AuthModeMeta,
    copy_settings: bool,
    cfg: &Config,
) -> Result<()> {
    paths::validate_name(name)?;
    let dir = paths::profile_dir(name)?;
    if dir.exists() {
        bail!("profile `{name}` already exists at {}", dir.display());
    }
    ensure_dir(&paths::profiles_dir()?, true)?;
    ensure_dir(&dir, true)?;

    let meta = ProfileMeta {
        name: name.to_string(),
        auth_mode: mode,
        created_at: now_stamp(),
    };
    let meta_path = paths::profile_meta_path(name)?;
    fs::write(&meta_path, toml::to_string_pretty(&meta)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&meta_path, fs::Permissions::from_mode(0o600));
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }

    if copy_settings {
        let src = config::source_path(cfg);
        for file in ["settings.json", "CLAUDE.md"] {
            let from = src.join(file);
            let to = dir.join(file);
            if from.is_file() {
                fs::copy(&from, &to)
                    .with_context(|| format!("copy {} → {}", from.display(), to.display()))?;
            }
        }
    }

    share::apply_share_for_profile(name, cfg)?;
    Ok(())
}

fn list_profiles() -> Result<()> {
    let dir = paths::profiles_dir()?;
    if !dir.exists() {
        println!("(no profiles - run `silo init` / `silo profile create`)");
        return Ok(());
    }
    let cfg = config::load()?;
    let mut names: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    if names.is_empty() {
        println!("(no profiles)");
        return Ok(());
    }
    println!(
        "{:<16} {:<12} {:<10} {}",
        "NAME", "AUTH", "CREDS", "CONFIG_DIR"
    );
    for name in names {
        let meta = load_meta(&name).ok();
        let mode = meta
            .as_ref()
            .map(|m| m.auth_mode.as_str())
            .unwrap_or("?");
        let creds = if has_credentials(&paths::profile_dir(&name)?) {
            "yes"
        } else {
            "need-login"
        };
        let def = if cfg.default_profile.as_deref() == Some(name.as_str()) {
            " *"
        } else {
            ""
        };
        println!(
            "{:<16} {:<12} {:<10} {}",
            format!("{name}{def}"),
            mode,
            creds,
            paths::profile_dir(&name)?.display()
        );
    }
    Ok(())
}

fn show_profile(name: &str) -> Result<()> {
    let dir = paths::profile_dir(name)?;
    if !dir.exists() {
        bail!("profile `{name}` not found");
    }
    let meta = load_meta(name)?;
    println!("name:        {}", meta.name);
    println!("auth_mode:   {}", meta.auth_mode.as_str());
    println!("created:     {}", meta.created_at);
    println!("config_dir:  {}", dir.display());
    println!(
        "credentials: {}",
        if has_credentials(&dir) {
            "present (Claude-managed)"
        } else {
            "not found - run `silo auth login {name}`"
        }
    );
    println!("contents:");
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let fname = e.file_name().to_string_lossy().into_owned();
        if fname == ".credentials.json" || fname.ends_with(".env") {
            println!("  {fname}  [private]");
            continue;
        }
        let tag = if p.is_symlink() {
            let t = fs::read_link(&p)
                .map(|t| t.display().to_string())
                .unwrap_or_default();
            format!(" -> {t}")
        } else if p.is_dir() {
            "/".into()
        } else {
            String::new()
        };
        println!("  {fname}{tag}");
    }
    Ok(())
}

fn delete_profile(name: &str, yes: bool) -> Result<()> {
    let dir = paths::profile_dir(name)?;
    if !dir.exists() {
        bail!("profile `{name}` not found");
    }
    if !yes {
        bail!(
            "refusing to delete `{name}` without --yes (path: {})",
            dir.display()
        );
    }
    fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
    let mut cfg = config::load()?;
    if cfg.default_profile.as_deref() == Some(name) {
        cfg.default_profile = None;
        config::save(&cfg)?;
    }
    println!("deleted profile `{name}`");
    Ok(())
}

pub fn load_meta(name: &str) -> Result<ProfileMeta> {
    let path = paths::profile_meta_path(name)?;
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(toml::from_str(&raw)?)
}

pub fn require_profile(name: &str) -> Result<PathBuf> {
    paths::validate_name(name)?;
    let dir = paths::profile_dir(name)?;
    if !dir.exists() {
        bail!("profile `{name}` not found - create with `silo profile create {name}`");
    }
    Ok(dir)
}

/// Best-effort "looks logged in" signal without reading secret values.
/// macOS may keep OAuth only in Keychain for some setups - doctor notes that.
pub fn has_credentials(dir: &Path) -> bool {
    if dir.join(".credentials.json").is_file() {
        return true;
    }
    if dir.join(".env").is_file() {
        return true;
    }
    // .claude.json alone is weak; only count if non-trivial size (session/account meta)
    let claude_json = dir.join(".claude.json");
    if let Ok(meta) = fs::metadata(&claude_json) {
        if meta.len() > 64 {
            return true;
        }
    }
    false
}

pub fn creds_label(dir: &Path) -> &'static str {
    if has_credentials(dir) {
        "creds~"
    } else {
        "need-login"
    }
}

pub fn cmd_default(action: DefaultCmd) -> Result<()> {
    let mut cfg = config::load()?;
    match action {
        DefaultCmd::Get => match cfg.default_profile {
            Some(ref n) => println!("{n}"),
            None => println!("(none)"),
        },
        DefaultCmd::Set { name } => {
            require_profile(&name)?;
            cfg.default_profile = Some(name.clone());
            config::save(&cfg)?;
            println!("default profile: {name}");
        }
        DefaultCmd::Clear => {
            cfg.default_profile = None;
            config::save(&cfg)?;
            println!("default profile cleared");
        }
    }
    Ok(())
}

pub fn cmd_link(args: LinkArgs) -> Result<()> {
    require_profile(&args.name)?;
    let dir = match args.path {
        Some(p) => paths::expand_user(&p),
        None => std::env::current_dir()?,
    };
    ensure_dir(&dir, false)?;
    let marker = dir.join(PROFILE_MARKER);
    fs::write(&marker, format!("{}\n", args.name))?;
    println!("linked `{}` → {}", args.name, marker.display());
    println!("tip: eval \"$(silo hook)\"");
    Ok(())
}

pub fn cmd_unlink(args: UnlinkArgs) -> Result<()> {
    let dir = match args.path {
        Some(p) => paths::expand_user(&p),
        None => std::env::current_dir()?,
    };
    let marker = dir.join(PROFILE_MARKER);
    if marker.exists() {
        fs::remove_file(&marker)?;
        println!("removed {}", marker.display());
    } else {
        println!("no {PROFILE_MARKER} in {}", dir.display());
    }
    Ok(())
}

pub fn cmd_which(args: WhichArgs) -> Result<()> {
    let start = match args.path {
        Some(p) => paths::expand_user(&p),
        None => std::env::current_dir()?,
    };
    if let Some(marker) = paths::find_marker(&start) {
        let name = paths::read_marker(&marker)?;
        println!(
            "{name}\t{}\t(from {})",
            paths::profile_dir(&name)?.display(),
            marker.display()
        );
        return Ok(());
    }
    if let Ok(env) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !env.is_empty() {
            println!("(env)\t{env}\t(CLAUDE_CONFIG_DIR)");
            return Ok(());
        }
    }
    let cfg = config::load()?;
    if let Some(name) = cfg.default_profile {
        println!(
            "{name}\t{}\t(default)",
            paths::profile_dir(&name)?.display()
        );
        return Ok(());
    }
    println!("(none)\t~\t(claude default ~/.claude)");
    Ok(())
}

fn now_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}
