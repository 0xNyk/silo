use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const PROFILE_MARKER: &str = ".claude-profile";
pub const ROOT_DIRNAME: &str = ".silo";

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("cannot resolve home directory")
}

pub fn silo_root() -> Result<PathBuf> {
    Ok(home_dir()?.join(ROOT_DIRNAME))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(silo_root()?.join("config.toml"))
}

pub fn profiles_dir() -> Result<PathBuf> {
    Ok(silo_root()?.join("profiles"))
}

pub fn profile_dir(name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    Ok(profiles_dir()?.join(name))
}

pub fn profile_meta_path(name: &str) -> Result<PathBuf> {
    Ok(profile_dir(name)?.join("silo.toml"))
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        bail!("invalid profile name: {name:?}");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("profile name must be [A-Za-z0-9_-]+, got {name:?}");
    }
    Ok(())
}

pub fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn ensure_dir(path: &Path, mode_private: bool) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("create dir {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if mode_private { 0o700 } else { 0o755 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    let _ = mode_private;
    Ok(())
}

pub fn find_marker(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = dir.join(PROFILE_MARKER);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn read_marker(path: &Path) -> Result<String> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(raw.trim().to_string())
}
