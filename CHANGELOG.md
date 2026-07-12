# Changelog

## 0.1.1 — 2026-07-12

### Multi-silo first

- Document and UX: **unlimited named silos** (not a two-account product)
- `silo init --count N` creates `s01..sNN` (custom `--prefix`)
- `silo init --names a,b,c,…`
- `silo profile create` accepts **many names** in one command
- Soft bulk-create guard at 256 (typo protection, not a product cap)
- Brand assets: field of 10 silos; blueprint shows 10+ examples

## 0.1.0 — 2026-07-12

Initial release.

- Profiles under `~/.silo/profiles/<name>`
- `init`, `profile`, `auth login`, `run`, `use`, `default`, `link` / `unlink` / `which`
- Opt-in `share` for skills / commands / agents only
- `doctor --keychain` classifies macOS Keychain as shared | isolated | unknown
- Shell hook for `.claude-profile` project bind
- No credential vault swap, no auto-rotate
