use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "silo",
    about = "Isolated Claude Code profiles for personal and work",
    long_about = "Keep personal and work Claude Code identities apart.\n\
Uses official CLAUDE_CONFIG_DIR isolation. Does not swap Keychain vaults.\n\
Does not auto-rotate Max subscriptions. Skill sharing is opt-in only.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
pub struct InitArgs {
    /// Also create personal + work profiles
    #[arg(long)]
    pub with_defaults: bool,
    /// Source of truth for optional shared assets (default: ~/.claude)
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ProfileCmd {
    Create {
        name: String,
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

#[derive(Parser, Debug)]
pub struct DoctorArgs {
    #[arg(long)]
    pub keychain: bool,
    #[arg(long)]
    pub fix_perms: bool,
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
}

pub fn print_completions(shell: ShellKind) -> anyhow::Result<()> {
    match shell {
        ShellKind::Zsh | ShellKind::Bash => {
            println!("# silo: eval \"$(silo hook)\"");
        }
        ShellKind::Fish => {
            println!("# silo fish: eval (silo hook)");
        }
    }
    Ok(())
}
