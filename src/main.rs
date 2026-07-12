mod cli;
mod config;
mod doctor;
mod keychain;
mod launch;
mod paths;
mod profile;
mod share;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init(args) => config::cmd_init(args),
        Commands::Profile { action } => profile::cmd_profile(action),
        Commands::Auth { action } => launch::cmd_auth(action),
        Commands::Run(args) => launch::cmd_run(args),
        Commands::Use(args) => launch::cmd_use(args),
        Commands::Default { action } => profile::cmd_default(action),
        Commands::Link(args) => profile::cmd_link(args),
        Commands::Unlink(args) => profile::cmd_unlink(args),
        Commands::Which(args) => profile::cmd_which(args),
        Commands::Share { action } => share::cmd_share(action),
        Commands::Doctor(args) => doctor::cmd_doctor(args),
        Commands::Status => doctor::cmd_status(),
        Commands::Hook => launch::cmd_hook(),
        Commands::Completions { shell } => cli::print_completions(shell),
    }
}
