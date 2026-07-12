use crate::cli::{ShareCmd, ShareKind};
use crate::config::{self, Config};
use crate::paths;
use crate::profile;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn cmd_share(action: ShareCmd) -> Result<()> {
    match action {
        ShareCmd::On { kind, profile } => set_share(kind, true, profile.as_deref())?,
        ShareCmd::Off { kind, profile } => set_share(kind, false, profile.as_deref())?,
        ShareCmd::Status => {
            let cfg = config::load()?;
            println!(
                "global flags: skills={} commands={} agents={}",
                cfg.share.skills, cfg.share.commands, cfg.share.agents
            );
            println!("source: {}", config::source_path(&cfg).display());
            println!();
            println!("Never shared: credentials, history, projects, settings.local, .env");
        }
    }
    Ok(())
}

fn set_share(kind: ShareKind, on: bool, profile: Option<&str>) -> Result<()> {
    let mut cfg = config::load()?;
    match kind {
        ShareKind::Skills => cfg.share.skills = on,
        ShareKind::Commands => cfg.share.commands = on,
        ShareKind::Agents => cfg.share.agents = on,
    }
    config::save(&cfg)?;

    let names = match profile {
        Some(n) => {
            profile::require_profile(n)?;
            vec![n.to_string()]
        }
        None => list_profile_names()?,
    };

    for name in names {
        if on {
            link_kind(&name, kind, &cfg)?;
            println!("share on  {} → `{name}`", kind_name(kind));
        } else {
            unlink_kind(&name, kind)?;
            println!("share off {} → `{name}`", kind_name(kind));
        }
    }
    Ok(())
}

pub fn apply_share_for_profile(name: &str, cfg: &Config) -> Result<()> {
    if cfg.share.skills {
        link_kind(name, ShareKind::Skills, cfg)?;
    }
    if cfg.share.commands {
        link_kind(name, ShareKind::Commands, cfg)?;
    }
    if cfg.share.agents {
        link_kind(name, ShareKind::Agents, cfg)?;
    }
    Ok(())
}

fn link_kind(profile: &str, kind: ShareKind, cfg: &Config) -> Result<()> {
    let src_root = config::source_path(cfg);
    let name = kind_name(kind);
    let from = src_root.join(name);
    let to = paths::profile_dir(profile)?.join(name);

    if !from.exists() {
        println!("  skip {name}: source missing ({})", from.display());
        return Ok(());
    }
    if to.exists() || to.is_symlink() {
        if to.is_symlink() {
            fs::remove_file(&to)?;
        } else {
            bail!(
                "{} already exists as a real path in profile `{profile}`",
                to.display()
            );
        }
    }
    symlink_dir(&from, &to)?;
    Ok(())
}

fn unlink_kind(profile: &str, kind: ShareKind) -> Result<()> {
    let to = paths::profile_dir(profile)?.join(kind_name(kind));
    if to.is_symlink() {
        fs::remove_file(&to)?;
    } else if to.exists() {
        println!(
            "  leave {}: not a symlink ({})",
            kind_name(kind),
            to.display()
        );
    }
    Ok(())
}

fn kind_name(kind: ShareKind) -> &'static str {
    match kind {
        ShareKind::Skills => "skills",
        ShareKind::Commands => "commands",
        ShareKind::Agents => "agents",
    }
}

fn list_profile_names() -> Result<Vec<String>> {
    let dir = paths::profiles_dir()?;
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut names: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(unix)]
fn symlink_dir(from: &Path, to: &Path) -> Result<()> {
    std::os::unix::fs::symlink(from, to)
        .with_context(|| format!("symlink {} → {}", to.display(), from.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink_dir(from: &Path, to: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(from, to)
        .with_context(|| format!("symlink {} → {}", to.display(), from.display()))?;
    Ok(())
}
