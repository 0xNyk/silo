#!/usr/bin/env bash
# silo installer — downloads a GitHub release binary with SHA-256 verify
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/scripts/install.sh | bash
#   VERSION=v0.1.0 INSTALL_DIR=~/.local/bin bash install.sh
set -euo pipefail

REPO="0xNyk/silo"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"

info() { printf '  %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || fail "need $1"; }
need curl
need tar
need shasum

os=$(uname -s)
arch=$(uname -m)
case "$os/$arch" in
  Darwin/arm64)  triple="aarch64-apple-darwin" ;;
  Darwin/x86_64) triple="x86_64-apple-darwin" ;;
  Linux/x86_64)  triple="x86_64-unknown-linux-gnu" ;;
  Linux/aarch64) fail "linux aarch64 builds not published yet — install from source: cargo install --git https://github.com/${REPO}" ;;
  *) fail "unsupported platform: $os $arch" ;;
esac

if [ "$VERSION" = "latest" ]; then
  tag=$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest")
  tag="${tag##*/}"
  [ -n "$tag" ] && [ "$tag" != "latest" ] || fail "could not resolve latest release"
else
  tag="$VERSION"
fi
ver="${tag#v}"
asset="silo-${ver}-${triple}.tar.gz"
url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
sum_url="https://github.com/${REPO}/releases/download/${tag}/checksums.txt"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

info "repo:    ${REPO}"
info "version: ${tag}"
info "target:  ${triple}"
info "install: ${INSTALL_DIR}/silo"

curl -fsSL "$url" -o "${tmp}/${asset}"
if curl -fsSL "$sum_url" -o "${tmp}/checksums.txt"; then
  expect=$(grep " ${asset}\$" "${tmp}/checksums.txt" | awk '{print $1}' || true)
  if [ -z "$expect" ]; then
    # try asset.sha256 sibling style
    if curl -fsSL "${url}.sha256" -o "${tmp}/one.sha256" 2>/dev/null; then
      expect=$(awk '{print $1}' "${tmp}/one.sha256")
    fi
  fi
  if [ -n "$expect" ]; then
    got=$(shasum -a 256 "${tmp}/${asset}" | awk '{print $1}')
    [ "$got" = "$expect" ] || fail "checksum mismatch for ${asset}"
    info "checksum: ok"
  else
    info "checksum: (no matching line — skipped)"
  fi
else
  info "checksum: (checksums.txt missing — skipped)"
fi

tar -xzf "${tmp}/${asset}" -C "$tmp"
bin=$(find "$tmp" -type f -name silo | head -1)
[ -n "$bin" ] || fail "silo binary not found in archive"
mkdir -p "$INSTALL_DIR"
install -m 755 "$bin" "${INSTALL_DIR}/silo"
info "installed ${INSTALL_DIR}/silo"
"${INSTALL_DIR}/silo" -V || true
info "next: silo init --with-defaults"
