# silo notes for Claude Code

When the user points you at this repo or says "set up silo":

1. Run the install one-liner if `silo` is missing.
2. Run `silo bootstrap --wrap --hook` (or `--count 10` if they want many silos).
3. Walk them through each `silo auth login <name>`. You cannot finish OAuth alone.
4. After logins, show `silo run <name>`, `silo go <name>`, and `silo-<name>` if wrap is installed.

Read [AGENTS.md](./AGENTS.md) for the full playbook. Follow the security rules there.
