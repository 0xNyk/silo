# Changelog

## Unreleased

### Security

- The shell hook no longer switches profiles just because a repository contains `.claude-profile`. A cloned third-party repo could previously select any existing local profile (for example `work`) on `cd`. Markers now take effect only in directories you trust, for the profile name you trusted.
- New `silo trust [dir]`, `silo untrust [dir]`, `silo trust --list`. The list lives in `${XDG_CONFIG_HOME:-~/.config}/silo/trusted-dirs` as `realpath<TAB>profile`. `silo trust` records the marker's current name and errors when there is no valid marker; `silo link` records the name it writes. A marker that later changes to another name is ignored until re-trusted.
- Entries without a profile name are treated as untrusted (fail closed) and the hook says so in one line.
- Trust-list paths are compared exactly (no whitespace trimming), and `silo trust` refuses paths with leading or trailing whitespace in a component.
- The hook refuses markers in directories owned by another user or group/world-writable, markers that are symlinks, non-regular files, group/world-writable or over 256 bytes, and names outside 1-64 chars of `[A-Za-z0-9_-]`. The marker is opened with `O_NOFOLLOW|O_NONBLOCK` and checked on the open file. Resolution moved into `silo hook-env`; the shell never evaluates marker content.
- The hook calls silo by its absolute path, so a relative `PATH` entry cannot run a repo's own `silo`. Re-source the hook after moving the binary.
- `silo use` only emits `unset` for environment names that are valid shell identifiers.

### Behaviour notes

- `silo which` prints two lines when a marker is ignored: an `(ignored)` line with the reason, then the profile that actually applies (env, default, or none).
- Leaving a trusted repo keeps the active profile (unchanged from 0.2.0). In bash the hook now re-checks only when the marker directory changes rather than on every prompt.
- Migration: re-source `silo hook`, then run `silo trust <repo>` for each repo you pinned.

## 0.2.0 - 2026-07-12

### Agent bootstrap

- `silo bootstrap`: init + Keychain doctor + login checklist
- `AGENTS.md` + `CLAUDE.md` for paste-into-agent setup
- `silo go <name>` alias for `run`
- `silo wrap install|uninstall|list` for `silo-<name>` launchers
- `silo auth status` and `doctor --checklist`
- Shell completions via `clap_complete`
- `SILO_HOME` / `SILO_BIN_DIR` overrides
- Clearer need-login signals; bootstrap keeps running when doctor warns
- Issue templates, CoC, `scripts/dogfood.sh`

## 0.1.1 - 2026-07-12

### Multi-silo

- Unlimited named silos (not only personal/work)
- `init --count N`, `--names`, multi-arg `profile create`
- Soft bulk guard 256; assets show 10 silos

## 0.1.0 - 2026-07-12

Initial release.
