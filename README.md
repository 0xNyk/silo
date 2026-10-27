![silo](assets/header.svg)

# silo

Isolated [Claude Code](https://code.claude.com) profiles via `CLAUDE_CONFIG_DIR`. One directory per identity - personal, work, clients, extra Max subs, whatever you name - with separate credentials, settings, and history.

[![ci](https://github.com/0xNyk/silo/actions/workflows/ci.yml/badge.svg)](https://github.com/0xNyk/silo/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![release](https://img.shields.io/github/v/release/0xNyk/silo?label=release)](https://github.com/0xNyk/silo/releases)

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
```

Puts the binary at `~/.local/bin/silo`. Checks SHA-256 when the release ships a checksum file. Falls back to `cargo install` if there is no prebuild for your OS.

```bash
VERSION=v0.2.0 bash          # pin a release
INSTALL_DIR=~/bin bash       # custom install dir
SILO_FORCE_CARGO=1 bash      # source only
```

## Agent setup

Point an agent at [AGENTS.md](./AGENTS.md), or run this yourself:

```bash
curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
silo bootstrap --wrap --hook          # personal + work
# silo bootstrap --count 10 --wrap    # or ten numbered silos
```

Then finish OAuth in a browser for each profile (agents cannot do this alone):

```bash
silo auth login personal
silo auth login work
```

## Switch profiles

```bash
silo run work                 # or: silo go work
silo-work                     # after bootstrap --wrap / silo wrap install
eval "$(silo use personal)"   # this shell only; then claude
```

## First run

```bash
silo bootstrap --count 10 --wrap --hook
silo auth status
silo auth login s01           # once per profile
silo run s07
silo link work                # pin and trust the current repo
silo doctor --keychain --checklist
```

Needs `claude` on `PATH`.

## Commands

| Command | What it does |
|---|---|
| `silo bootstrap …` | Init, doctor, login checklist; optional `--wrap` / `--hook` |
| `silo init --count 10` | Create `s01`…`s10` |
| `silo profile create a b c …` | Create several names at once |
| `silo auth login <name>` | Open Claude under that silo for `/login` |
| `silo auth status` | Who still needs login |
| `silo run` / `silo go <name>` | Start Claude as that silo |
| `silo wrap install` | Install `silo-<name>` launcher scripts |
| `silo link <name>` | Write `.claude-profile` in the current repo and trust it |
| `silo trust [dir]` / `silo untrust [dir]` | Allow or stop the hook honouring a repo's `.claude-profile` (`--list` to show) |
| `silo hook` | Shell hook: auto-activate on `cd`, trusted repos only |
| `silo share on skills` | Opt-in shared skills (not history) |
| `silo doctor --keychain` | Keychain class + checklist |
| `silo completions zsh` | Shell completions |

No hard cap on how many profiles you keep. Soft limit: 256 creates in a single command (stops typos, not product intent).

## Per-repo pinning and trust

`.claude-profile` is a file inside the repository, so anyone who ships you a repo can ship one. The shell hook therefore only acts on a marker in a directory you have trusted:

```bash
silo link work                # writes .claude-profile and trusts this repo
silo trust ./path/to/repo    # trust a repo that already has a marker
silo trust --list
silo untrust ./path/to/repo
```

- Trust is for a directory *and* a profile name. `${XDG_CONFIG_HOME:-~/.config}/silo/trusted-dirs` (mode `0600`) holds one `realpath<TAB>profile` per line. Symlinks are resolved, so trust follows the real directory, not the path you typed.
- `silo trust` records the name currently in the marker (and fails if there is no valid marker or the profile does not exist). `silo link` records the name it writes. If a later `git pull` changes the marker to another profile, the hook stops switching and tells you the marker now wants a different profile until you re-run `silo trust`.
- In an untrusted repo the hook prints one line naming the profile it wants and the `silo trust` command, and leaves your shell as it was. A nested marker in a subdirectory of a trusted repo (for example a vendored dependency) needs its own trust.
- The hook never switches when the directory or the marker is owned by another user or is group/world-writable, when the marker is a symlink, not a regular file, or larger than 256 bytes, or when the name is not 1-64 characters of `[A-Za-z0-9_-]`. The trust list is ignored if another user owns it or it is group/world-writable.
- Marker content is never evaluated. The hook calls silo by the absolute path of the binary that printed it (so a repo's own `bin/silo` on a relative `PATH` entry is never run) and passes only the directory to `silo hook-env`, which prints exports only after these checks.
- The hook re-checks when you enter a different marker directory, not on every prompt, so a manual `eval "$(silo use …)"` inside a repo sticks until you `cd` elsewhere. Leaving a trusted repo keeps the active profile; the hook never unsets it.

**Upgrading from 0.2.0:** existing `.claude-profile` files stop auto-activating until you trust them. Re-run `eval "$(silo hook)"` in open shells (or start a new one), then run `silo trust <repo>` (or `silo link <name>` again) in each repo you pinned yourself. `silo which` shows whether a marker is ignored. Because the hook embeds the binary's absolute path, re-source it after moving or reinstalling silo to a different path. If your umask makes directories group-writable (`002`), `chmod g-w` the repos you want the hook to act on.

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

| Stays private | Shared only if you opt in |
|---|---|
| credentials, history, projects | `skills/`, `commands/`, `agents/` |

Auth modes: `oauth`, `setup-token`, `api-key`, `bedrock`, `vertex`, `foundry`.

![architecture](assets/blueprint.svg)

## macOS Keychain

If `silo doctor --keychain` prints class `shared`, use one OAuth silo at a time (or put extras on setup-token / API key). Keeping many logged-in profiles on disk is fine. Running two OAuth sessions at once may not be.

silo never prints tokens and never runs `security … -g`.

## Out of scope

- Multi-Max auto-rotation
- Global Keychain vault swap
- Symlink thrash of `~/.claude`
- Local OAuth proxies
- Token export packs

## silo vs other profile managers

Every tool in this category wraps the same mechanism: Claude Code reads credentials, settings, and history from `CLAUDE_CONFIG_DIR`, so one directory per identity gives full isolation. The differences are tooling around that fact.

- **Plain shell aliases** (`CLAUDE_CONFIG_DIR=~/.claude-work claude`) cover two stable profiles fine. silo starts paying off at ten numbered silos, wrappers, hooks, and doctor checks.
- **[claude-code-profiles](https://github.com/quinnjr/claude-code-profiles)** and **[@remeic/ccm](https://www.npmjs.com/package/@remeic/ccm)** are profile switchers around the same variable. silo adds bulk bootstrap (`--count 10`), six auth modes (oauth / setup-token / api-key / bedrock / vertex / foundry), macOS Keychain conflict detection, and per-repo pinning - shipped as a checksummed binary rather than an npm install.
- **[claude-multiprofile](https://www.npmjs.com/package/claude-multiprofile)** targets Claude Desktop and Claude Code side by side; silo is Claude Code only.

If one of those fits better, use it - the isolation mechanism is identical either way. Descriptions are positioning-level as of July 2026.

## Links

[AGENTS.md](./AGENTS.md) · [SECURITY.md](./SECURITY.md) · [CHANGELOG.md](./CHANGELOG.md) · [Releases](https://github.com/0xNyk/silo/releases)

## Support

Maintained by [Nyk](https://nyk.dev). If Silo helps your work, [sponsor its continued maintenance](https://github.com/sponsors/0xNyk) or follow [@nykdotdev](https://x.com/nykdotdev).

## License

[MIT](LICENSE) © 0xNyk

---

## From this repo to a working agent OS

Quiet commercial paths from [nyk.dev](https://www.nyk.dev) — same operator, fixed scope:

- Free checklist: https://www.nyk.dev/resources/agent-os-checklist?utm_source=github&utm_medium=readme&utm_campaign=oss_proof&utm_content=silo
- Self-serve Production Agent OS Kit (€149 founding): https://www.nyk.dev/go/agent-os-kit?utm_source=github&utm_medium=readme&utm_campaign=oss_proof&utm_content=silo
- Hands-on packages: https://www.nyk.dev/consulting?utm_source=github&utm_medium=readme&utm_campaign=oss_proof&utm_content=silo#agent-os