# Changelog

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
