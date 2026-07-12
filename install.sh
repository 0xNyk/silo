#!/usr/bin/env bash
# silo — one-line install
#
#   curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
#
# Options (env):
#   VERSION=v0.1.1     pin a release tag (default: latest)
#   INSTALL_DIR=...    default: ~/.local/bin
#   SILO_FORCE_CARGO=1 skip binary download; cargo install from git
#
set -euo pipefail

REPO="0xNyk/silo"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
SILO_FORCE_CARGO="${SILO_FORCE_CARGO:-0}"

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || fail "need \`$1\` on PATH"; }

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    fail "need shasum or sha256sum"
  fi
}

install_from_cargo() {
  need cargo
  info "installing from source via cargo…"
  cargo install --git "https://github.com/${REPO}" --locked --force
  if command -v silo >/dev/null 2>&1; then
    bold "installed $(command -v silo)"
    silo -V
  else
    local cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
    info "binary: ${cargo_bin}/silo  (add that dir to PATH if needed)"
    "${cargo_bin}/silo" -V || true
  fi
  info "next: silo init --count 10"
}

ensure_path_hint() {
  case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      info "note: ${INSTALL_DIR} is not on PATH"
      info "      add:  export PATH=\"${INSTALL_DIR}:\$PATH\""
      ;;
  esac
}

# --- cargo-only path ---
if [ "$SILO_FORCE_CARGO" = "1" ]; then
  install_from_cargo
  exit 0
fi

need curl
need tar
need uname

os=$(uname -s)
arch=$(uname -m)
case "$os/$arch" in
  Darwin/arm64|Darwin/aarch64) triple="aarch64-apple-darwin" ;;
  Darwin/x86_64)               triple="x86_64-apple-darwin" ;;
  Linux/x86_64|Linux/amd64)    triple="x86_64-unknown-linux-gnu" ;;
  Linux/aarch64|Linux/arm64)   triple="aarch64-unknown-linux-gnu" ;;
  *)
    info "no prebuilt binary for $os/$arch — falling back to cargo"
    install_from_cargo
    exit 0
    ;;
esac

if [ "$VERSION" = "latest" ]; then
  tag=$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest")
  tag="${tag##*/}"
  [ -n "$tag" ] && [ "$tag" != "latest" ] || fail "could not resolve latest release"
else
  tag="$VERSION"
  case "$tag" in
    v*) ;;
    *) tag="v${tag}" ;;
  esac
fi
ver="${tag#v}"
asset="silo-${ver}-${triple}.tar.gz"
url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
sum_url="https://github.com/${REPO}/releases/download/${tag}/checksums.txt"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

bold "silo install"
info "repo:    ${REPO}"
info "version: ${tag}"
info "target:  ${triple}"
info "dest:    ${INSTALL_DIR}/silo"

if ! curl -fsSL "$url" -o "${tmp}/${asset}"; then
  info "prebuilt ${asset} not found — falling back to cargo"
  install_from_cargo
  exit 0
fi

if curl -fsSL "$sum_url" -o "${tmp}/checksums.txt" 2>/dev/null; then
  expect=$(grep -E "  ${asset}\$| ${asset}\$" "${tmp}/checksums.txt" | awk '{print $1}' | head -1 || true)
  if [ -n "${expect:-}" ]; then
    got=$(sha256_file "${tmp}/${asset}")
    [ "$got" = "$expect" ] || fail "checksum mismatch for ${asset}"
    info "checksum: ok"
  else
    info "checksum: no entry for ${asset} (skipped)"
  fi
else
  info "checksum: checksums.txt missing (skipped)"
fi

tar -xzf "${tmp}/${asset}" -C "$tmp"
bin=$(find "$tmp" -type f -name silo | head -1)
[ -n "${bin:-}" ] || fail "silo binary not found in archive"

mkdir -p "$INSTALL_DIR"
install -m 755 "$bin" "${INSTALL_DIR}/silo"

bold "installed ${INSTALL_DIR}/silo"
"${INSTALL_DIR}/silo" -V || true
ensure_path_hint
info "next: silo init --count 10"
info "      silo doctor --keychain"
