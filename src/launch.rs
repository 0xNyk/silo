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
        "claude not found on PATH - install Claude Code first (https://code.claude.com)",
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
    print!("{}", render_use(&args.name)?);
    Ok(())
}

/// Shell exports that activate `name`. `name` is validated by `require_profile`
/// ([A-Za-z0-9_-]+), and every value is single-quoted.
pub fn render_use(name: &str) -> Result<String> {
    use std::fmt::Write as _;
    let dir = profile::require_profile(name)?;
    let meta = profile::load_meta(name)?;
    let mut out = String::new();
    writeln!(out, "# silo profile: {name}")?;
    for key in collect_env_keys_to_unset() {
        writeln!(out, "unset {key}")?;
    }
    writeln!(
        out,
        "export CLAUDE_CONFIG_DIR='{}'",
        shell_escape(&dir.display().to_string())
    )?;
    writeln!(out, "export SILO_PROFILE='{}'", shell_escape(name))?;
    match meta.auth_mode {
        AuthModeMeta::ApiKey => writeln!(
            out,
            "# auth_mode=api-key - set ANTHROPIC_API_KEY from your secret store before launching"
        )?,
        AuthModeMeta::SetupToken => writeln!(
            out,
            "# auth_mode=setup-token - export CLAUDE_CODE_OAUTH_TOKEN from your secret store"
        )?,
        AuthModeMeta::Bedrock => writeln!(out, "export CLAUDE_CODE_USE_BEDROCK=1")?,
        AuthModeMeta::Vertex => writeln!(out, "export CLAUDE_CODE_USE_VERTEX=1")?,
        AuthModeMeta::Foundry => writeln!(out, "export CLAUDE_CODE_USE_FOUNDRY=1")?,
        AuthModeMeta::Oauth => {}
    }
    Ok(out)
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
    print!("{}", render_hook()?);
    Ok(())
}

/// The hook calls silo by the absolute path of this binary, so a relative
/// `PATH` entry (`.`, `bin`, `node_modules/.bin`) cannot make a repository's own
/// `silo` run on `cd`.
pub fn render_hook() -> Result<String> {
    let exe = env::current_exe().context("resolve path of the silo binary")?;
    let exe = std::fs::canonicalize(&exe)
        .with_context(|| format!("resolve {}", exe.display()))?;
    let exe = exe
        .to_str()
        .context("silo binary path is not UTF-8; cannot embed it in the hook")?;
    if exe.contains('\n') || exe.contains('\r') {
        anyhow::bail!("silo binary path contains a newline; refusing to emit a hook");
    }
    let quoted = format!("'{}'", exe.replace('\'', r"'\''"));
    Ok(HOOK_SCRIPT.replace("__SILO_BIN__", &quoted))
}

/// Shell hook template (`__SILO_BIN__` is replaced by the quoted absolute
/// path of this binary). It only locates the nearest `.claude-profile`; the trust check,
/// ownership/permission checks, and name validation all run in
/// `silo hook-env`, whose stdout is generated by silo (never marker content).
/// Re-evaluates only when the marker directory changes, so an untrusted notice
/// prints once per entry and a manual `silo use` is not overridden every prompt.
pub const HOOK_SCRIPT: &str = r#"
# silo shell hook - auto-activate .claude-profile in trusted directories
# Add to ~/.zshrc or ~/.bashrc:
#   eval "$(silo hook)"
# Trust a repo with: silo trust <dir>   (silo link does this for you)

_silo_auto() {
  local dir="$PWD"
  local root=""
  while [ -n "$dir" ] && [ "$dir" != "/" ]; do
    if [ -f "$dir/.claude-profile" ]; then
      root="$dir"
      break
    fi
    dir="$(dirname "$dir")"
  done
  if [ "$root" = "${_SILO_HOOK_ROOT:-}" ]; then
    return 0
  fi
  _SILO_HOOK_ROOT="$root"
  if [ -z "$root" ]; then
    return 0
  fi
  local out
  out="$(__SILO_BIN__ hook-env "$root")" || return 0
  if [ -n "$out" ]; then
    eval "$out"
  fi
}

if [ -n "${ZSH_VERSION:-}" ]; then
  autoload -U add-zsh-hook 2>/dev/null || true
  add-zsh-hook chpwd _silo_auto 2>/dev/null || true
  _silo_auto
fi
if [ -n "${BASH_VERSION:-}" ]; then
  case "${PROMPT_COMMAND:-}" in
    *_silo_auto*) ;;
    *) PROMPT_COMMAND="_silo_auto${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
  esac
  _silo_auto
fi
"#;

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
    // Only names a shell accepts unquoted; anything else could not have come
    // from an `export` and must not reach an eval'd `unset` line.
    let mut v: Vec<_> = keys.into_iter().filter(|k| is_shell_name(k)).collect();
    v.sort();
    v
}

/// `^[A-Za-z_][A-Za-z0-9_]*$`
fn is_shell_name(k: &str) -> bool {
    let mut chars = k.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
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
                    "warning: Keychain class is `{}` - concurrent OAuth may cross-poison credentials.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_names() {
        assert!(is_shell_name("CLAUDE_CONFIG_DIR"));
        assert!(is_shell_name("_x1"));
        for bad in ["", "1A", "CLAUDE_X;touch", "A B", "A-B", "A$(id)", "Ä"] {
            assert!(!is_shell_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn hook_embeds_absolute_binary() {
        let h = render_hook().unwrap();
        assert!(!h.contains("__SILO_BIN__"));
        assert!(!h.contains("command silo"));
        assert!(h.contains("out=\"$('/"), "{h}");
    }
}
