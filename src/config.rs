use crate::cli::InitArgs;
use crate::paths::{self, ensure_dir};
use crate::profile::{self, AuthModeMeta};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub source_dir: String,
    pub default_profile: Option<String>,
    #[serde(default)]
    pub share: ShareConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShareConfig {
    #[serde(default)]
    pub skills: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub agents: bool,
}

impl Default for Config {
    fn default() -> Self {
        let source = dirs::home_dir()
            .map(|h| h.join(".claude").display().to_string())
            .unwrap_or_else(|| "~/.claude".into());
        Self {
            version: 1,
            source_dir: source,
            default_profile: None,
            share: ShareConfig::default(),
        }
    }
}

pub fn load() -> Result<Config> {
    let path = paths::config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(toml::from_str(&raw).context("parse config.toml")?)
}

pub fn save(cfg: &Config) -> Result<()> {
    let root = paths::silo_root()?;
    ensure_dir(&root, true)?;
    ensure_dir(&paths::profiles_dir()?, true)?;
    let path = paths::config_path()?;
    let raw = toml::to_string_pretty(cfg).context("serialize config")?;
    std::fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn source_path(cfg: &Config) -> PathBuf {
    paths::expand_user(&cfg.source_dir)
}

pub fn cmd_init(args: InitArgs) -> Result<()> {
    let mut cfg = if paths::config_path()?.exists() {
        load()?
    } else {
        Config::default()
    };
    if let Some(src) = args.source {
        cfg.source_dir = src;
    }
    save(&cfg)?;
    println!("silo root:   {}", paths::silo_root()?.display());
    println!("config:      {}", paths::config_path()?.display());
    println!("profiles:    {}", paths::profiles_dir()?.display());
    println!("source_dir:  {}", source_path(&cfg).display());
    println!();
    println!("Share is off by default. Credentials stay with Claude Code — silo never vault-swaps tokens.");

    if args.with_defaults {
        for name in ["personal", "work"] {
            if !paths::profile_dir(name)?.exists() {
                profile::create_profile(name, AuthModeMeta::Oauth, false, &cfg)?;
                println!("created profile: {name}");
            } else {
                println!("profile exists:  {name}");
            }
        }
        if cfg.default_profile.is_none() {
            cfg.default_profile = Some("personal".into());
            save(&cfg)?;
            println!("default profile: personal");
        }
    } else {
        println!();
        println!("Next:");
        println!("  silo profile create personal");
        println!("  silo profile create work");
        println!("  silo auth login personal");
        println!("  silo auth login work");
        println!("  silo link work          # inside a work repo");
        println!("  silo doctor --keychain");
    }
    Ok(())
}
