use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, shells};

#[derive(Parser, Debug)]
#[command(
    name = "silo",
    about = "Isolated Claude Code profiles — as many silos as you need",
    long_about = "Run many Claude Code identities side by side (personal, work, clients, Max subs…).\n\
Each silo is an isolated CLAUDE_CONFIG_DIR. No Keychain vault swap. No multi-Max auto-rotate.\n\
Skill sharing is opt-in only. Create 2, 10, or 50 — same model.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// One-shot setup for agents/humans: init + doctor + login checklist (+ optional wrap/hook)
    Bootstrap(BootstrapArgs),
    /// Create ~/.silo layout and optional first profiles
    Init(InitArgs),
    /// Manage named profiles
    Profile {
        #[command(subcommand)]
        action: ProfileCmd,
    },
    /// Authentication helpers (delegates to claude under a profile)
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// Run claude as a profile in this process (exec)
    Run(RunArgs),
    /// Alias for `run` — one-word daily switch
    Go(RunArgs),
    /// Print shell exports to activate a profile in the current shell
    Use(UseArgs),
    /// Get/set the default profile
    Default {
        #[command(subcommand)]
        action: DefaultCmd,
    },
    /// Write .claude-profile in a directory (project bind)
    Link(LinkArgs),
    /// Remove .claude-profile
    Unlink(UnlinkArgs),
    /// Resolve which profile applies for a path (or cwd)
    Which(WhichArgs),
    /// Opt-in shared skills/commands/agents
    Share {
        #[command(subcommand)]
        action: ShareCmd,
    },
    /// Install silo-<name> launcher scripts for one-command starts
    Wrap {
        #[command(subcommand)]
        action: WrapCmd,
    },
    /// Diagnose paths, perms, auth presence, macOS Keychain class
    Doctor(DoctorArgs),
    /// Short status
    Status,
    /// Print shell hook that auto-activates .claude-profile on cd
    Hook,
    /// Generate shell completions
    Completions {
        shell: ShellKind,
    },
}

#[derive(Parser, Debug)]
pub struct BootstrapArgs {
    /// Create starter profiles: personal + work
    #[arg(long)]
    pub with_defaults: bool,
    /// Create N numbered silos (s01..sNN)
    #[arg(long, value_name = "N")]
    pub count: Option<u32>,
    /// Prefix for --count (default: s)
    #[arg(long, default_value = "s")]
    pub prefix: String,
    /// Comma-separated names
    #[arg(long, value_delimiter = ',')]
    pub names: Vec<String>,
    /// Shared source dir (default: ~/.claude)
    #[arg(long)]
    pub source: Option<String>,
    /// Also install silo-<name> wrappers into ~/.local/bin
    #[arg(long)]
    pub wrap: bool,
    /// Print hook install line (does not edit your shell rc)
    #[arg(long)]
    pub hook: bool,
    /// Skip doctor at the end
    #[arg(long)]
    pub no_doctor: bool,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    #[arg(long)]
    pub with_defaults: bool,
    #[arg(long, value_name = "N")]
    pub count: Option<u32>,
    #[arg(long, default_value = "s")]
    pub prefix: String,
    #[arg(long, value_delimiter = ',')]
    pub names: Vec<String>,
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ProfileCmd {
    Create {
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
        #[arg(long, value_enum, default_value = "oauth")]
        mode: AuthMode,
        #[arg(long)]
        copy_settings: bool,
    },
    List,
    Show { name: String },
    Delete {
        name: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    /// Launch claude under a profile so you can /login
    Login { name: String },
    /// List which profiles still need login (no secrets printed)
    Status,
}

#[derive(Parser, Debug)]
pub struct RunArgs {
    pub name: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub claude_args: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct UseArgs {
    pub name: String,
}

#[derive(Subcommand, Debug)]
pub enum DefaultCmd {
    Get,
    Set { name: String },
    Clear,
}

#[derive(Parser, Debug)]
pub struct LinkArgs {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Parser, Debug)]
pub struct UnlinkArgs {
    pub path: Option<String>,
}

#[derive(Parser, Debug)]
pub struct WhichArgs {
    pub path: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ShareCmd {
    On {
        #[arg(value_enum)]
        kind: ShareKind,
        profile: Option<String>,
    },
    Off {
        #[arg(value_enum)]
        kind: ShareKind,
        profile: Option<String>,
    },
    Status,
}

#[derive(Subcommand, Debug)]
pub enum WrapCmd {
    /// Write silo-<name> scripts for every profile (or named ones)
    Install {
        #[arg(num_args = 0..)]
        names: Vec<String>,
    },
    /// Remove silo-* wrappers managed by this tool
    Uninstall {
        #[arg(num_args = 0..)]
        names: Vec<String>,
    },
    /// List installed wrappers
    List,
}

#[derive(Parser, Debug)]
pub struct DoctorArgs {
    #[arg(long)]
    pub keychain: bool,
    #[arg(long)]
    pub fix_perms: bool,
    /// Print a short login checklist for profiles missing creds
    #[arg(long)]
    pub checklist: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum AuthMode {
    Oauth,
    SetupToken,
    ApiKey,
    Bedrock,
    Vertex,
    Foundry,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ShareKind {
    Skills,
    Commands,
    Agents,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

pub fn print_completions(shell: ShellKind) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    match shell {
        ShellKind::Bash => generate(shells::Bash, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Zsh => generate(shells::Zsh, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Fish => generate(shells::Fish, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Elvish => generate(shells::Elvish, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::PowerShell => {
            generate(shells::PowerShell, &mut cmd, name, &mut std::io::stdout())
        }
    }
    Ok(())
}
