# Contributing

## Scope

silo stays thin:

- `CLAUDE_CONFIG_DIR` profiles
- clean env launch
- project bind (`.claude-profile`)
- opt-in skill share
- Keychain-aware doctor

Out of scope unless discussed first:

- multi-Max auto-rotation
- Keychain vault dump/restore
- OAuth proxies / provider marketplaces
- multi-CLI mega-orchestration (Codex/Gemini)

## Dev

```bash
cargo build
cargo build --release
cargo test
./target/release/silo doctor --keychain
```

## Style

- Prefer official Claude Code docs over reverse-engineered token formats.
- Never log secrets.
- Keep the CLI boring and predictable.
