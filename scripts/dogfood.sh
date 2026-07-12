#!/usr/bin/env bash
# Local dogfood checklist - run after real OAuth logins
set -euo pipefail
export PATH="${HOME}/.local/bin:${PATH}"

need() { command -v "$1" >/dev/null || { echo "missing $1"; exit 1; }; }
need silo
need claude

echo "== silo dogfood =="
silo -V
silo status
silo profile list
silo doctor --keychain --checklist || true
silo auth status || true

echo
echo "== launch version under default/first profile =="
first=$(silo profile list 2>/dev/null | awk 'NR==2{print $1; exit}' | tr -d '*')
if [ -n "${first:-}" ] && [ "$first" != "NAME" ]; then
  silo run "$first" -- --version
else
  echo "(no profile to run)"
fi

echo
echo "== wrap list =="
silo wrap list || true

echo
echo "Manual (human):"
echo "  1) silo auth login <each need-login profile>"
echo "  2) silo run personal -p 'say PERSONAL only'"
echo "  3) silo run work -p 'say WORK only'"
echo "  4) confirm doctor keychain class after dual login"
echo "Done smoke. Complete manual steps for full e2e."
