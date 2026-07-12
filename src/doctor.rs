use crate::cli::DoctorArgs;
use crate::config;
use crate::keychain;
use crate::paths;
use crate::profile;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn cmd_status() -> Result<()> {
    let cfg = config::load()?;
    let root = paths::silo_root()?;
    println!("silo {}", env!("CARGO_PKG_VERSION"));
    println!("root:    {}", root.display());
    println!("source:  {}", config::source_path(&cfg).display());
    println!(
        "default: {}",
        cfg.default_profile.as_deref().unwrap_or("(none)")
    );
    println!(
        "share:   skills={} commands={} agents={}",
        cfg.share.skills, cfg.share.commands, cfg.share.agents
    );
    if let Ok(p) = std::env::var("SILO_PROFILE") {
        if !p.is_empty() {
            println!("active:  {p} (shell)");
        }
    }
    if let Ok(d) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !d.is_empty() {
            println!("CLAUDE_CONFIG_DIR={d}");
        }
    }
    let pdir = paths::profiles_dir()?;
    let n = if pdir.is_dir() {
        fs::read_dir(&pdir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .count()
    } else {
        0
    };
    println!("profiles: {n}  (unlimited named silos; soft bulk-create cap {})", crate::config::MAX_BULK_CREATE);
    Ok(())
}

pub fn cmd_doctor(args: DoctorArgs) -> Result<()> {
    let mut warnings = 0u32;
    let mut errors = 0u32;

    println!("silo doctor {}\n", env!("CARGO_PKG_VERSION"));
    println!("{}\n", keychain::redact_hint());

    match which::which("claude") {
        Ok(p) => println!("[ OK ] claude binary                 {}", p.display()),
        Err(_) => {
            println!("[ERR ] claude binary                 not found on PATH");
            errors += 1;
        }
    }

    let root = paths::silo_root()?;
    if root.is_dir() {
        println!("[ OK ] silo root                     {}", root.display());
        check_private_dir(&root, &mut warnings);
    } else {
        println!("[WARN] silo root                     missing — run `silo init`");
        warnings += 1;
    }

    let cfg_path = paths::config_path()?;
    let cfg = if cfg_path.is_file() {
        println!("[ OK ] config.toml                   {}", cfg_path.display());
        config::load()?
    } else {
        println!("[WARN] config.toml                   missing — run `silo init`");
        warnings += 1;
        config::Config::default()
    };

    let source = config::source_path(&cfg);
    if source.is_dir() {
        println!("[ OK ] source_dir                    {}", source.display());
    } else {
        println!(
            "[WARN] source_dir                    {} (missing)",
            source.display()
        );
        warnings += 1;
    }

    let pdir = paths::profiles_dir()?;
    if pdir.is_dir() {
        let mut names: Vec<_> = fs::read_dir(&pdir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        if names.is_empty() {
            println!("[WARN] profiles                      none");
            warnings += 1;
        }
        for name in names {
            let dir = paths::profile_dir(&name)?;
            check_private_dir(&dir, &mut warnings);
            if args.fix_perms {
                fix_private_dir(&dir)?;
            }
            let creds = if profile::has_credentials(&dir) {
                "creds~"
            } else {
                "no-creds"
            };
            let mode = profile::load_meta(&name)
                .map(|m| m.auth_mode.as_str().to_string())
                .unwrap_or_else(|_| "?".into());
            println!(
                "[ OK ] profile/{name:<18} {mode:<12} {creds}  {}",
                dir.display()
            );
            for risky in ["projects", "history.jsonl", ".credentials.json"] {
                let p = dir.join(risky);
                if p.is_symlink() {
                    println!(
                        "[WARN] profile/{name}  {risky} is a symlink — history may leak across identities"
                    );
                    warnings += 1;
                }
            }
        }
    } else {
        println!("[WARN] profiles dir                  missing");
        warnings += 1;
    }

    for key in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
    ] {
        if std::env::var_os(key).is_some() {
            println!("[WARN] env {key:<28} set — may override subscription OAuth");
            warnings += 1;
        }
    }

    if args.keychain || cfg!(target_os = "macos") {
        println!("\nKeychain");
        let report = keychain::inspect();
        println!("[ -- ] class                         {}", report.class.as_str());
        if report.services.is_empty() {
            println!("[ -- ] services                      (none listed)");
        } else {
            for s in &report.services {
                println!("[ -- ] service                       {s}");
            }
        }
        for n in &report.notes {
            println!("[WARN] {n}");
            warnings += 1;
        }
        if report.class == keychain::KeychainClass::Shared {
            println!(
                "[WARN] parallel multi OAuth          UNSAFE — one OAuth silo at a time, or setup-token/API for extras"
            );
            warnings += 1;
        }
    }

    println!();
    if errors > 0 {
        println!("doctor: {errors} error(s), {warnings} warning(s)");
        std::process::exit(2);
    } else if warnings > 0 {
        println!("doctor: {warnings} warning(s)");
    } else {
        println!("doctor: clean");
    }
    Ok(())
}

fn check_private_dir(path: &Path, warnings: &mut u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mode = meta.permissions().mode() & 0o777;
            if mode & 0o077 != 0 {
                println!(
                    "[WARN] perms                          {} is {:o} (want 700)",
                    path.display(),
                    mode
                );
                *warnings += 1;
            }
        }
    }
    let _ = (path, warnings);
}

fn fix_private_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        println!("[fix] perms                          {} -> 700", path.display());
    }
    let _ = path;
    Ok(())
}
