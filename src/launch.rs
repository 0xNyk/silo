use crate::cli::{AuthCmd, RunArgs, UseArgs};
use crate::keychain;
use crate::profile::{self, AuthModeMeta};
use anyhow::{Context, Result};
#[cfg(not(unix))]
use anyhow::bail;
use std::collections::HashSet;
use std::env;
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const UNSET_PREFIXES: &[&str] = &["CLAUDE_", "ANTHROPIC_"];

pub fn cmd_run(args: RunArgs) -> Result<()> {
    let dir = profile::require_profile(&args.name)?;
    let meta = profile::load_meta(&args.name)?;
    maybe_warn_parallel(&meta);

    let claude = which::which("claude").context(
        "claude not found on PATH — install Claude Code first (https://code.claude.com)",
    )?;

    let mut cmd = Command::new(&claude);
    apply_clean_env(&mut cmd, &dir, &meta);
    cmd.args(&args.claude_args);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    #[cfg(unix)]
    {
        let err = cmd.exec();
        return Err(err).with_context(|| format!("exec {}", claude.display()));
    }
    #[cfg(not(unix))]
    {
        let status = cmd
            .status()
            .with_context(|| format!("run {}", claude.display()))?;
        if status.success() {
            Ok(())
        } else {
            bail!("claude exited with {status}");
        }
    }
}

pub fn cmd_use(args: UseArgs) -> Result<()> {
    let dir = profile::require_profile(&args.name)?;
    let meta = profile::load_meta(&args.name)?;
    println!("# silo profile: {}", args.name);
    for key in collect_env_keys_to_unset() {
        println!("unset {key}");
    }
    println!(
        "export CLAUDE_CONFIG_DIR='{}'",
        shell_escape(&dir.display().to_string())
    );
    println!("export SILO_PROFILE='{}'", shell_escape(&args.name));
    match meta.auth_mode {
        AuthModeMeta::ApiKey => {
            println!(
                "# auth_mode=api-key — set ANTHROPIC_API_KEY from your secret store before launching"
            );
        }
        AuthModeMeta::SetupToken => {
            println!(
                "# auth_mode=setup-token — export CLAUDE_CODE_OAUTH_TOKEN from your secret store"
            );
        }
        AuthModeMeta::Bedrock => println!("export CLAUDE_CODE_USE_BEDROCK=1"),
        AuthModeMeta::Vertex => println!("export CLAUDE_CODE_USE_VERTEX=1"),
        AuthModeMeta::Foundry => println!("export CLAUDE_CODE_USE_FOUNDRY=1"),
        AuthModeMeta::Oauth => {}
    }
    Ok(())
}

pub fn cmd_auth(action: AuthCmd) -> Result<()> {
    match action {
        AuthCmd::Status => crate::doctor::print_login_checklist(),
        AuthCmd::Login { name } => {
            let dir = profile::require_profile(&name)?;
            let meta = profile::load_meta(&name)?;
            println!("Launching claude under profile `{name}`.");
            println!("config_dir: {}", dir.display());
            println!("Complete /login in the session. silo does not store or swap tokens.");
            let claude = which::which("claude").context("claude not found on PATH")?;
            let mut cmd = Command::new(claude);
            apply_clean_env(&mut cmd, &dir, &meta);
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            #[cfg(unix)]
            {
                let err = cmd.exec();
                return Err(err).context("exec claude");
            }
            #[cfg(not(unix))]
            {
                let status = cmd.status().context("run claude")?;
                if status.success() {
                    Ok(())
                } else {
                    bail!("claude exited with {status}");
                }
            }
        }
    }
}

pub fn cmd_hook() -> Result<()> {
    print!(
        r#"
# silo shell hook — auto-activate .claude-profile
# Add to ~/.zshrc or ~/.bashrc:
#   eval "$(silo hook)"

_silo_auto() {{
  local dir="$PWD"
  local marker=""
  while [ "$dir" != "/" ]; do
    if [ -f "$dir/.claude-profile" ]; then
      marker="$dir/.claude-profile"
      break
    fi
    dir="$(dirname "$dir")"
  done
  if [ -n "$marker" ]; then
    local target
    target="$(tr -d '[:space:]' < "$marker")"
    if [ -n "$target" ] && [ "$target" != "${{SILO_PROFILE:-}}" ]; then
      eval "$(silo use "$target")"
    fi
  fi
}}

if [ -n "${{ZSH_VERSION:-}}" ]; then
  autoload -U add-zsh-hook 2>/dev/null || true
  add-zsh-hook chpwd _silo_auto 2>/dev/null || true
  _silo_auto
fi
if [ -n "${{BASH_VERSION:-}}" ]; then
  if ! echo "${{PROMPT_COMMAND:-}}" | grep -q _silo_auto; then
    PROMPT_COMMAND="_silo_auto${{PROMPT_COMMAND:+;$PROMPT_COMMAND}}"
  fi
  _silo_auto
fi
"#
    );
    Ok(())
}

fn apply_clean_env(
    cmd: &mut Command,
    config_dir: &std::path::Path,
    meta: &profile::ProfileMeta,
) {
    for key in collect_env_keys_to_unset() {
        cmd.env_remove(key);
    }
    cmd.env("CLAUDE_CONFIG_DIR", config_dir);
    cmd.env("SILO_PROFILE", &meta.name);
    match meta.auth_mode {
        AuthModeMeta::Bedrock => {
            cmd.env("CLAUDE_CODE_USE_BEDROCK", "1");
        }
        AuthModeMeta::Vertex => {
            cmd.env("CLAUDE_CODE_USE_VERTEX", "1");
        }
        AuthModeMeta::Foundry => {
            cmd.env("CLAUDE_CODE_USE_FOUNDRY", "1");
        }
        AuthModeMeta::Oauth | AuthModeMeta::SetupToken | AuthModeMeta::ApiKey => {}
    }
}

fn collect_env_keys_to_unset() -> Vec<String> {
    let mut keys = HashSet::new();
    for (k, _) in env::vars() {
        if UNSET_PREFIXES.iter().any(|p| k.starts_with(p)) {
            keys.insert(k);
        }
    }
    for k in [
        "CLAUDE_CONFIG_DIR",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
        "SILO_PROFILE",
        "CPRO_PROFILE",
        "CLAUDE_PROFILE",
    ] {
        keys.insert(k.to_string());
    }
    let mut v: Vec<_> = keys.into_iter().collect();
    v.sort();
    v
}

fn maybe_warn_parallel(meta: &profile::ProfileMeta) {
    if meta.auth_mode != AuthModeMeta::Oauth {
        return;
    }
    if let Ok(active) = env::var("SILO_PROFILE") {
        if !active.is_empty() && active != meta.name {
            let report = keychain::inspect();
            if !keychain::parallel_oauth_allowed(report.class) {
                eprintln!(
                    "warning: Keychain class is `{}` — concurrent OAuth may cross-poison credentials.",
                    report.class.as_str()
                );
                eprintln!(
                    "         Prefer sequential sessions, or setup-token/API for the second process."
                );
            }
        }
    }
}

fn shell_escape(s: &str) -> String {
    s.replace('\'', r"'\''")
}
