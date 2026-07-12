# Agent playbook — silo

Give this repo (or file) to an agent. Goal: install silo, create profiles, guide human OAuth, then switch with one command.

## One-shot for the agent

```bash
# 1) Install binary
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"

# 2) Bootstrap profiles (defaults: personal + work)
silo bootstrap --wrap --hook
# or ten numbered silos:
# silo bootstrap --count 10 --wrap --hook
```

## Human step (cannot be skipped for OAuth)

For each line printed as `[    ] silo auth login <name>`:

1. Run that command in the user’s terminal.
2. Wait for them to finish browser `/login`.
3. Do **not** invent tokens or scrape Keychain secrets.

Check progress:

```bash
silo auth status
silo doctor --keychain --checklist
```

## Daily use (after login)

| Goal | Command |
|---|---|
| Start as profile | `silo run work` or `silo go work` |
| Wrapper (if `--wrap`) | `silo-work` |
| This shell only | `eval "$(silo use work)"` then `claude` |
| Pin a repo | `cd <repo> && silo link work` |
| Auto-pin on cd | user adds `eval "$(silo hook)"` to shell rc |

## Rules

- Prefer official `CLAUDE_CONFIG_DIR` isolation — never build Keychain vault swap tools.
- Never print OAuth tokens, refresh tokens, or `security … -g` output.
- Unlimited named profiles; soft bulk create cap is 256/command.
- If `doctor` reports Keychain class `shared`, do not promise concurrent dual OAuth.
- Multi-Max auto-rotate is out of scope for this product.

## Verify

```bash
silo -V
silo profile list
silo doctor --keychain --checklist
silo run personal -- --version
```
