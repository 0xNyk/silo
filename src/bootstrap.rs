use crate::cli::{BootstrapArgs, DoctorArgs, InitArgs};
use crate::config;
use crate::doctor;
use crate::wrap;
use anyhow::Result;

pub fn cmd_bootstrap(args: BootstrapArgs) -> Result<()> {
    println!("silo bootstrap {}", env!("CARGO_PKG_VERSION"));
    println!("agent-friendly one-shot setup (OAuth login still needs a human per account)\n");

    // Ensure claude exists early
    match which::which("claude") {
        Ok(p) => println!("[ok] claude: {}", p.display()),
        Err(_) => {
            println!("[!!] claude not on PATH — install Claude Code first: https://code.claude.com");
            println!("     continuing with profile layout only…\n");
        }
    }

    // Default to personal+work if nothing specified
    let mut with_defaults = args.with_defaults;
    let count = args.count;
    let names = args.names.clone();
    if !with_defaults && count.is_none() && names.is_empty() {
        with_defaults = true;
        println!("[i]  no profile flags — using --with-defaults (personal + work)");
        println!("     tip: silo bootstrap --count 10   or   --names a,b,c\n");
    }

    config::cmd_init(InitArgs {
        with_defaults,
        count,
        prefix: args.prefix,
        names,
        source: args.source,
    })?;

    if args.wrap {
        println!();
        wrap::cmd_install(&[])?;
    }

    if args.hook {
        println!();
        println!("# shell auto-pin (add to ~/.zshrc / ~/.bashrc):");
        println!("eval \"$(silo hook)\"");
    }

    if !args.no_doctor {
        println!();
        // Do not exit the process on doctor warnings during bootstrap.
        let _code = doctor::cmd_doctor(DoctorArgs {
            keychain: true,
            fix_perms: false,
            checklist: true,
        })?;
    } else {
        doctor::print_login_checklist()?;
    }

    println!();
    println!("=== daily use (after logins) ===");
    println!("  silo run <name>          # start Claude as that silo");
    println!("  silo go <name>           # same as run");
    if args.wrap {
        println!("  silo-<name>              # wrapper script (if installed)");
    } else {
        println!("  silo wrap install        # optional: silo-personal, silo-work, …");
    }
    println!("  silo link <name>         # pin current repo");
    println!("  eval \"$(silo hook)\"      # auto-activate .claude-profile on cd");
    println!();
    println!("bootstrap complete — finish any pending `silo auth login` steps above.");
    Ok(())
}


