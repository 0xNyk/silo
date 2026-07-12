![silo](assets/header.svg)

# silo

Isolated [Claude Code](https://code.claude.com) profiles. As many as you need — personal, work, clients, extra Max subs — each with its own `CLAUDE_CONFIG_DIR`.

[![ci](https://github.com/0xNyk/silo/actions/workflows/ci.yml/badge.svg)](https://github.com/0xNyk/silo/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![release](https://img.shields.io/github/v/release/0xNyk/silo?label=release)](https://github.com/0xNyk/silo/releases)

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
```

## Agent / one-shot setup

Hand this repo to Claude (or any agent) and point it at [AGENTS.md](./AGENTS.md). Short version:

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
silo bootstrap --wrap --hook          # personal + work
# silo bootstrap --count 10 --wrap    # or ten silos
```

Then **you** finish each:

```bash
silo auth login personal
silo auth login work
```

OAuth is browser-based — agents cannot complete it alone.

## Daily switch (one command)

```bash
silo run work                 # or: silo go work
silo-work                     # if you used --wrap / `silo wrap install`
eval "$(silo use personal)"   # this shell only, then `claude`
```

## Quick start

```bash
silo bootstrap --count 10 --wrap --hook
silo auth status                          # login checklist
silo auth login s01                       # repeat per profile
silo run s07
silo link work                            # pin a repo
silo doctor --keychain --checklist
```

Requires `claude` on `PATH`.

## Commands

| | |
|---|---|
| `silo bootstrap …` | Init + doctor + login checklist (+ `--wrap` / `--hook`) |
| `silo init --count 10` | Create `s01`…`s10` |
| `silo profile create a b c …` | Many names at once |
| `silo auth login <name>` | Browser OAuth under that silo |
| `silo auth status` | Who still needs login |
| `silo run` / `silo go <name>` | Start Claude as that silo |
| `silo wrap install` | `silo-<name>` launcher scripts |
| `silo link <name>` | Write `.claude-profile` |
| `silo hook` | Auto-pin on `cd` |
| `silo share on skills` | Opt-in shared skills |
| `silo doctor --keychain` | Safety class + checklist |
| `silo completions zsh` | Shell completions |

No product cap on profile count. Soft bulk guard: **256** creates per command.

## How it works

```
.claude-profile | silo use | silo run | silo-<name>
        │
        ▼
  unset CLAUDE_* / ANTHROPIC_*
  CLAUDE_CONFIG_DIR=~/.silo/profiles/<name>
        │
        ▼
  exec claude
```

| Always private | Opt-in share |
|---|---|
| credentials, history, projects | `skills/`, `commands/`, `agents/` |

Auth modes: `oauth`, `setup-token`, `api-key`, `bedrock`, `vertex`, `foundry`.

![architecture](assets/blueprint.svg)

## macOS Keychain

When `doctor` reports class `shared`, run **one OAuth silo at a time** (or use setup-token/API for extras). Many silos on disk is fine; concurrent OAuth may not be.

silo never prints tokens and never runs `security … -g`.

## What this is not

Multi-Max auto-rotate · Keychain vault swap · `~/.claude` symlink thrash · OAuth proxies · token export packs

## Links

[AGENTS.md](./AGENTS.md) · [SECURITY.md](./SECURITY.md) · [CHANGELOG.md](./CHANGELOG.md) · [Releases](https://github.com/0xNyk/silo/releases)

## License

[MIT](LICENSE) © 0xNyk
