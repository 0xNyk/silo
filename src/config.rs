use crate::cli::InitArgs;
use crate::paths::{self, ensure_dir};
use crate::profile::{self, AuthModeMeta};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Soft upper bound to catch typos (e.g. --count 100000). Not a product limit.
pub const MAX_BULK_CREATE: u32 = 256;

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
    println!("As many silos as you want. Share off by default. Claude Code owns OAuth - silo never vault-swaps tokens.");

    let mut created: Vec<String> = Vec::new();
    let mut to_create: Vec<String> = Vec::new();

    if args.with_defaults {
        to_create.extend(["personal".into(), "work".into()]);
    }
    for n in &args.names {
        let t = n.trim();
        if !t.is_empty() {
            to_create.push(t.to_string());
        }
    }
    if let Some(count) = args.count {
        if count == 0 {
            bail!("--count must be >= 1");
        }
        if count > MAX_BULK_CREATE {
            bail!("--count {count} exceeds soft limit {MAX_BULK_CREATE} (raise only if intentional)");
        }
        let width = count.to_string().len().max(2);
        for i in 1..=count {
            to_create.push(format!("{prefix}{i:0width$}", prefix = args.prefix, width = width));
        }
    }

    // de-dupe preserving order
    let mut seen = std::collections::HashSet::new();
    to_create.retain(|n| seen.insert(n.clone()));

    for name in &to_create {
        if paths::profile_dir(name)?.exists() {
            println!("profile exists:  {name}");
        } else {
            profile::create_profile(name, AuthModeMeta::Oauth, false, &cfg)?;
            println!("created profile: {name}");
            created.push(name.clone());
        }
    }

    if cfg.default_profile.is_none() {
        let def = if paths::profile_dir("personal")?.exists() {
            Some("personal".into())
        } else {
            first_profile_name()?
        };
        if let Some(d) = def {
            cfg.default_profile = Some(d.clone());
            save(&cfg)?;
            println!("default profile: {d}");
        }
    }

    if to_create.is_empty() {
        println!();
        println!("Next (any number of silos):");
        println!("  silo profile create personal work client-a client-b");
        println!("  silo init --count 10              # s01..s10");
        println!("  silo init --names a,b,c,d,e,f,g,h,i,j");
        println!("  silo auth login <name>");
        println!("  silo link <name>                  # pin a repo");
        println!("  silo doctor --keychain");
    } else {
        let total = count_profiles()?;
        println!();
        println!("profiles ready: {} total under {}", total, paths::profiles_dir()?.display());
        println!("login each: silo auth login <name>");
    }
    Ok(())
}

fn first_profile_name() -> Result<Option<String>> {
    let dir = paths::profiles_dir()?;
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut names: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names.into_iter().next())
}

fn count_profiles() -> Result<usize> {
    let dir = paths::profiles_dir()?;
    if !dir.is_dir() {
        return Ok(0);
    }
    Ok(std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count())
}
