# Changelog

## 0.2.0 — 2026-07-12

### Agent-ready bootstrap

- `silo bootstrap` — one-shot init + Keychain doctor + login checklist
- `AGENTS.md` + `CLAUDE.md` playbooks for “paste repo into agent”
- `silo go <name>` alias for `run`
- `silo wrap install|uninstall|list` → `silo-<name>` launchers
- `silo auth status` + `doctor --checklist`
- Real shell completions via `clap_complete`
- `SILO_HOME` / `SILO_BIN_DIR` overrides (tests + portable)
- Stronger need-login signals; doctor does not kill bootstrap on warnings
- Issue templates, CoC, `scripts/dogfood.sh`

## 0.1.1 — 2026-07-12

### Multi-silo first

- Unlimited named silos messaging
- `init --count N`, `--names`, multi-arg `profile create`
- Soft bulk guard 256; brand assets with 10 silos

## 0.1.0 — 2026-07-12

Initial release.
