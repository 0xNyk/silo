![silo](assets/header.svg)

# silo

Isolated [Claude Code](https://code.claude.com) profiles. As many as you need — personal, work, clients, extra Max subs — each with its own `CLAUDE_CONFIG_DIR`.

[![ci](https://github.com/0xNyk/silo/actions/workflows/ci.yml/badge.svg)](https://github.com/0xNyk/silo/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![release](https://img.shields.io/github/v/release/0xNyk/silo?label=release)](https://github.com/0xNyk/silo/releases)

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
```

Installs to `~/.local/bin/silo` (SHA-256 checked when present). Falls back to `cargo install` if no prebuild matches your OS.

```bash
# optional
VERSION=v0.1.1 bash          # pin
INSTALL_DIR=~/bin bash       # custom dir
SILO_FORCE_CARGO=1 bash      # source only
```

## Quick start

```bash
silo init --count 10                    # s01..s10
# or: silo profile create personal work client-a max-1 max-2 …
silo auth login s01                     # /login under that profile
silo link work                          # pin current repo → work
eval "$(silo hook)"                     # optional: auto-switch on cd
silo run s07                            # one-shot session
silo doctor --keychain                  # Keychain safety class
```

Requires `claude` on `PATH`.

## Commands

| | |
|---|---|
| `silo init --count 10` | Create `s01`…`s10` |
| `silo init --with-defaults` | Create `personal` + `work` |
| `silo init --names a,b,c,…` | Create a named set |
| `silo profile create …` | One or many names in one shot |
| `silo profile list` | All silos |
| `silo auth login <name>` | Browser OAuth under that silo |
| `silo run <name> [-- args]` | `exec claude` with clean env |
| `silo use <name>` | Print exports for `eval "$(…)"` |
| `silo link <name>` | Write `.claude-profile` (repo pin) |
| `silo hook` | Shell hook for auto-pin on `cd` |
| `silo share on skills` | Opt-in shared skills/commands/agents |
| `silo doctor --keychain` | Paths, perms, Keychain class |
| `silo status` | Short summary |

No product cap on how many silos you keep. Soft guard: max **256** creates per command (typo protection).

## How it works

```
.claude-profile | silo use | silo run
        │
        ▼
  unset CLAUDE_* / ANTHROPIC_*
  CLAUDE_CONFIG_DIR=~/.silo/profiles/<name>
        │
        ▼
  exec claude
```

```
~/.silo/
  config.toml
  profiles/
    personal/     # private creds, history, projects
    work/
    s01/
    …             # as many as you create
```

| Always private | Opt-in share |
|---|---|
| credentials, history, projects | `skills/`, `commands/`, `agents/` |
| `.env`, `settings.local` | |

Auth modes: `oauth` (default), `setup-token`, `api-key`, `bedrock`, `vertex`, `foundry`.

![architecture](assets/blueprint.svg)

## macOS Keychain

```text
$ silo doctor --keychain
[ -- ] class                         shared
[WARN] parallel multi OAuth          UNSAFE
```

| Class | Meaning |
|---|---|
| `shared` | One OAuth process at a time (or use setup-token/API for extras) |
| `isolated` | Concurrent OAuth may work — re-check after Claude Code upgrades |
| `unknown` | No credentials services found yet |

Holding many logged-in silos on disk is fine. Concurrent OAuth processes may not be when class is `shared`.

silo never prints tokens and never runs `security … -g`.

## What this is not

- Multi-Max auto-rotation / quota farming
- Global Keychain vault swap
- Symlink thrash of `~/.claude`
- OAuth proxy pools
- Token export packs
- A two-account product

## Links

- [SECURITY.md](SECURITY.md) · [CHANGELOG.md](CHANGELOG.md) · [CONTRIBUTING.md](CONTRIBUTING.md)
- [Releases](https://github.com/0xNyk/silo/releases)

## License

[MIT](LICENSE) © 0xNyk
