<p align="center">
  <img src="assets/header.svg" alt="silo" width="720"/>
</p>

<p align="center">
  <strong>silo</strong> — isolated Claude Code profiles for personal and work
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-ffffff?style=flat-square&labelColor=0a0a0a" alt="MIT"/></a>
  <a href="https://github.com/0xNyk/silo/actions"><img src="https://img.shields.io/github/actions/workflow/status/0xNyk/silo/ci.yml?style=flat-square&label=ci&labelColor=0a0a0a" alt="ci"/></a>
</p>

```
$ silo init --with-defaults
$ silo auth login personal
$ silo auth login work
$ silo link work                 # in a client repo
$ eval "$(silo hook)"            # optional: auto-activate on cd
$ silo run personal              # or: silo use work && claude
$ silo doctor --keychain
```

Keep identities apart. Share skills only when you choose. Never dump a token vault.

## Why

Claude Code is one identity per process. People solve that with `/login` thrash, Keychain swap scripts, or tools that share history between “profiles.”

silo does the boring correct thing:

1. One directory per profile (`CLAUDE_CONFIG_DIR`)
2. Clean environment, then `exec claude`
3. Project pin via `.claude-profile`
4. Doctor that tells you when macOS Keychain is still **shared**

It is not a multi-Max auto-rotator. That is intentional.

## Install

Requires [Claude Code](https://code.claude.com) on `PATH`. Binary name: `silo`.

### Prebuilt (macOS / Linux x86_64)

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/scripts/install.sh | bash
```

Pin a version or install dir:

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/scripts/install.sh \
  | VERSION=v0.1.0 INSTALL_DIR=~/.local/bin bash
```

The script verifies SHA-256 when `checksums.txt` is present on the release.

### From source

```bash
cargo install --git https://github.com/0xNyk/silo --locked
# or
git clone https://github.com/0xNyk/silo && cd silo && cargo install --path . --locked
```

## Daily use

| Goal | Command |
|---|---|
| Create personal + work | `silo init --with-defaults` |
| Log in a profile | `silo auth login work` |
| One-off session | `silo run work` |
| This shell only | `eval "$(silo use work)"` then `claude` |
| Pin a repo | `silo link work` |
| Auto-pin on `cd` | `eval "$(silo hook)"` in `~/.zshrc` |
| Share skills (opt-in) | `silo share on skills` |
| Check safety | `silo doctor --keychain` |

### Layout

```
~/.silo/
  config.toml
  profiles/
    personal/          # CLAUDE_CONFIG_DIR for personal
    work/              # CLAUDE_CONFIG_DIR for work
```

### What is private vs shared

| Always private | Opt-in share only |
|---|---|
| credentials | `skills/` |
| history / projects | `commands/` |
| `.env`, `settings.local` | `agents/` |

## Architecture

<p align="center">
  <img src="assets/blueprint.svg" alt="silo architecture blueprint" width="900"/>
</p>

```
.claude-profile / silo use / silo run
        │
        ▼
  unset competing CLAUDE_* / ANTHROPIC_*
  export CLAUDE_CONFIG_DIR=~/.silo/profiles/<name>
        │
        ▼
  exec claude
```

Auth modes per profile: `oauth` (default), `setup-token`, `api-key`, `bedrock`, `vertex`, `foundry`.

## Doctor and macOS Keychain

```
$ silo doctor --keychain
[ -- ] class                         shared
[WARN] parallel dual OAuth           UNSAFE
```

On several Claude Code builds, OAuth still uses a single Keychain service. When class is `shared`, run profiles **sequentially**, or put the second concurrent process on `setup-token` / API key.

silo never prints tokens and never runs `security … -g`.

## Not goals

- Multi-Max rate-limit auto-rotation
- Swapping a global Keychain vault as the daily switch
- Symlinking `~/.claude` as the concurrency model
- Local OAuth proxy pools
- Exporting multi-account credential packs

If you need max convenience + quota auto-switch, use something purpose-built for that and accept the risk surface. silo optimizes for **correct isolation**.

## Brand

| | |
|---|---|
| Name | **silo** |
| Mark | two outlined capsules with a hard gap |
| Palette | `#0a0a0a` · `#fafafa` · `#737373` |
| Type | system monospace |
| Assets | `assets/logo.svg` · `assets/header.svg` · `assets/blueprint.svg` · `assets/header.png` · `assets/og.png` |

No purple gradients. No “AI product” chrome. Isolation is the metaphor.

## Security

See [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © 0xNyk
