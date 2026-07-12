mod bootstrap;
mod cli;
mod config;
mod doctor;
mod keychain;
mod launch;
mod paths;
mod profile;
mod share;
mod wrap;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, WrapCmd};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Bootstrap(args) => bootstrap::cmd_bootstrap(args),
        Commands::Init(args) => config::cmd_init(args),
        Commands::Profile { action } => profile::cmd_profile(action),
        Commands::Auth { action } => launch::cmd_auth(action),
        Commands::Run(args) | Commands::Go(args) => launch::cmd_run(args),
        Commands::Use(args) => launch::cmd_use(args),
        Commands::Default { action } => profile::cmd_default(action),
        Commands::Link(args) => profile::cmd_link(args),
        Commands::Unlink(args) => profile::cmd_unlink(args),
        Commands::Which(args) => profile::cmd_which(args),
        Commands::Share { action } => share::cmd_share(action),
        Commands::Wrap { action } => match action {
            WrapCmd::Install { names } => wrap::cmd_install(&names),
            WrapCmd::Uninstall { names } => wrap::cmd_uninstall(&names),
            WrapCmd::List => wrap::cmd_list(),
        },
        Commands::Doctor(args) => {
            let code = doctor::cmd_doctor(args)?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Commands::Status => doctor::cmd_status(),
        Commands::Hook => launch::cmd_hook(),
        Commands::Completions { shell } => cli::print_completions(shell),
    }
}
