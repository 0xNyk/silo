use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const PROFILE_MARKER: &str = ".claude-profile";
pub const ROOT_DIRNAME: &str = ".silo";

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("cannot resolve home directory")
}

/// Root for config + profiles. Override with `SILO_HOME` (tests / portable installs).
pub fn silo_root() -> Result<PathBuf> {
    if let Ok(v) = std::env::var("SILO_HOME") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
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

pub fn bin_dir() -> Result<PathBuf> {
    if let Ok(v) = std::env::var("SILO_BIN_DIR") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(home_dir()?.join(".local/bin"))
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

pub fn list_profile_names() -> Result<Vec<String>> {
    let dir = profiles_dir()?;
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut names: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

pub fn require_exists(name: &str) -> Result<PathBuf> {
    let dir = profile_dir(name)?;
    if !dir.exists() {
        bail!("profile `{name}` not found - create with `silo profile create {name}`");
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_ok() {
        assert!(validate_name("personal").is_ok());
        assert!(validate_name("s01").is_ok());
        assert!(validate_name("client_a-1").is_ok());
    }

    #[test]
    fn validate_name_bad() {
        assert!(validate_name("../x").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("").is_err());
        assert!(validate_name("has space").is_err());
    }

    #[test]
    fn silo_home_override() {
        let tmp = std::env::temp_dir().join(format!("silo-test-{}", std::process::id()));
        std::env::set_var("SILO_HOME", &tmp);
        assert_eq!(silo_root().unwrap(), tmp);
        std::env::remove_var("SILO_HOME");
    }
}
