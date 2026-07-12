<p align="center">
  <img src="assets/header.svg" alt="silo" width="720"/>
</p>

<p align="center">
  <strong>silo</strong> — as many isolated Claude Code profiles as you need
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-ffffff?style=flat-square&labelColor=0a0a0a" alt="MIT"/></a>
  <a href="https://github.com/0xNyk/silo/actions"><img src="https://img.shields.io/github/actions/workflow/status/0xNyk/silo/ci.yml?style=flat-square&label=ci&labelColor=0a0a0a" alt="ci"/></a>
</p>

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
```

Downloads the latest release binary (SHA-256 checked when available), installs to `~/.local/bin/silo`, falls back to `cargo install` if no prebuild matches your OS.

```bash
# pin version / install dir
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | VERSION=v0.1.1 bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | INSTALL_DIR=~/.local/bin bash

# force source build
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | SILO_FORCE_CARGO=1 bash
```

Requires [Claude Code](https://code.claude.com) on `PATH` to run profiles.

```
$ silo init --count 10                 # s01..s10
$ silo profile create personal work client-a client-b max-1 max-2
$ silo auth login s01
$ silo link client-a                   # in that repo
$ eval "$(silo hook)"                  # optional auto-activate on cd
$ silo run s07
$ silo doctor --keychain
```

**Not limited to two.** Personal + work is a common start. Ten Max subs, a dozen clients, or one-off experiment silos — same model: one directory, one identity, clean launch.

## Why

Claude Code is one identity per process. People thrash `/login`, swap Keychain vaults, or share history between “profiles.”

silo does the boring correct thing:

1. One directory per named silo (`CLAUDE_CONFIG_DIR`)
2. Clean environment, then `exec claude`
3. Project pin via `.claude-profile`
4. Doctor that says when macOS Keychain is still **shared**

It is not a multi-Max auto-rotator. That is intentional. You can still **hold** many subscriptions — you just switch silos deliberately.

## Daily use

| Goal | Command |
|---|---|
| Ten numbered silos | `silo init --count 10` |
| Named batch | `silo profile create a b c d e f g h i j` |
| Starter personal + work | `silo init --with-defaults` |
| Custom names on init | `silo init --names personal,work,c1,c2,c3,c4,c5,c6,c7,c8` |
| Log in a silo | `silo auth login s03` |
| One-off session | `silo run client-a` |
| This shell only | `eval "$(silo use work)"` then `claude` |
| Pin a repo | `silo link client-a` |
| Auto-pin on `cd` | `eval "$(silo hook)"` in `~/.zshrc` |
| Share skills (opt-in) | `silo share on skills` |
| List everything | `silo profile list` |
| Safety check | `silo doctor --keychain` |

### Layout

```
~/.silo/
  config.toml
  profiles/
    personal/
    work/
    client-a/
    s01/
    s02/
    …                 # as many as you create
```

No hard product cap. Bulk create has a **soft** safety limit of 256 per command (typo guard).

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

Auth modes per silo: `oauth` (default), `setup-token`, `api-key`, `bedrock`, `vertex`, `foundry`.

## Doctor and macOS Keychain

```
$ silo doctor --keychain
[ -- ] class                         shared
[WARN] parallel multi OAuth          UNSAFE
```

When class is `shared`, run **one OAuth silo at a time**, or put extras on `setup-token` / API key. Holding 10 logged-in silos on disk is fine; concurrent OAuth processes may not be.

silo never prints tokens and never runs `security … -g`.

## Not goals

- Multi-Max rate-limit auto-rotation
- Swapping a global Keychain vault as the daily switch
- Symlinking `~/.claude` as the concurrency model
- Local OAuth proxy pools
- Exporting multi-account credential packs
- Artificial “only two profiles” product limits

## Brand

| | |
|---|---|
| Name | **silo** |
| Mark | many outlined capsules (a field of silos, not a pair) |
| Palette | `#0a0a0a` · `#fafafa` · `#737373` |
| Type | system monospace |
| Assets | `assets/logo.svg` · `assets/header.svg` · `assets/blueprint.svg` |

## Security

See [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © 0xNyk
