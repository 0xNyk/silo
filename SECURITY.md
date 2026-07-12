# Security

## What silo is

silo launches Claude Code under isolated `CLAUDE_CONFIG_DIR` profiles. It is a process launcher and doctor, not a credential vault.

## Non-negotiables

- Claude Code owns OAuth secrets (Keychain on macOS; `.credentials.json` on Linux/Windows).
- silo never prints access tokens or refresh tokens.
- silo never runs `security … -g` (no Keychain secret dump).
- silo never exports multi-account credential packs by default.
- Profile directories are created `0700`; config is `0600` where supported.
- Skill/command/agent sharing is **opt-in**. History, projects, and credentials are never shared by default.

## macOS Keychain

On some Claude Code builds, OAuth still lands in a **single shared** Keychain service (`Claude Code-credentials`). When `silo doctor --keychain` reports `shared`:

- Do not run two OAuth profiles concurrently.
- Prefer sequential sessions, or `setup-token` / API key for a second concurrent process.

## Reporting

Open a private security advisory on GitHub if you find a vulnerability:

https://github.com/0xNyk/silo/security

Do not open a public issue with secret material.
