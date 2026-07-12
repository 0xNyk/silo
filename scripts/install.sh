#!/usr/bin/env bash
# Back-compat wrapper - prefer the root one-liner:
#   curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
set -euo pipefail
ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
exec bash "${ROOT}/install.sh" "$@"
