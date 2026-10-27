#!/usr/bin/env bash
# silo - one-line install
#
#   curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash
#   curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/0xNyk/silo/main/install.sh | bash -s -- --version v0.2.0
#
# Flags:
#   --version <tag>     release to install (default: the pinned DEFAULT_VERSION below; "latest" allowed)
#   --install-dir <dir> default: ~/.local/bin
#   --dry-run           print what would be downloaded and verified, then exit
#   -h, --help
#
# Options (env):
#   SILO_VERSION=v0.2.0           same as --version (a generic VERSION= is still read, with a warning)
#   INSTALL_DIR=...               same as --install-dir
#   SILO_FORCE_CARGO=1            skip binary download; cargo install from the release tag
#   SILO_VERIFY_ATTESTATION=auto  auto | require | skip  (GitHub artifact attestation via `gh`)
#
# Every prebuilt archive is verified against a SHA-256 before it is unpacked:
# the digest pinned in this script for older releases, otherwise the SHA256SUMS
# file published with the same release. A missing or mismatched checksum aborts.
# With SILO_VERIFY_ATTESTATION=require, source (cargo) builds are refused.
set -euo pipefail

REPO="0xNyk/silo"
# Bump together with each release (and drop it from pinned_sha256 once that release ships SHA256SUMS).
DEFAULT_VERSION="v0.2.0"
SIGNER_WORKFLOW="${REPO}/.github/workflows/release.yml"

INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
SILO_FORCE_CARGO="${SILO_FORCE_CARGO:-0}"
ATTEST_MODE="${SILO_VERIFY_ATTESTATION:-auto}"
DRY_RUN=0
requested=""
tag=""
tmp=""

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
  cat <<'USAGE'
usage: install.sh [--version <tag>|latest] [--install-dir <dir>] [--dry-run]

env: SILO_VERSION, INSTALL_DIR, SILO_FORCE_CARGO=1,
     SILO_VERIFY_ATTESTATION=auto|require|skip
USAGE
}

need() { command -v "$1" >/dev/null 2>&1 || fail "need \`$1\` on PATH"; }

curl_https() {
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL --retry 3 "$@"
}

# Download $1 to $2 and print the final HTTP status. Transport errors (DNS, TLS,
# a redirect to a non-https URL) make this return non-zero.
fetch_status() {
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 -sSL --retry 3 \
    -o "$2" -w '%{http_code}' "$1"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    fail "need sha256sum or shasum to verify the download"
  fi
}

# Releases published before the workflow emitted SHA256SUMS and attestations.
# Digests recorded from the published GitHub release assets.
pinned_sha256() {
  case "$1" in
    silo-0.2.0-aarch64-apple-darwin.tar.gz) echo "66294f58fcb6be10055cd270d828177f7e549e51ace00fffd842e126ec41cddd" ;;
    *) return 1 ;;
  esac
}

# Print the digest listed for $2 in checksum file $1 (exact filename match).
checksum_entry() {
  awk -v name="$2" '($2 == name || $2 == "*" name) { print tolower($1); exit }' "$1"
}

is_sha256() {
  [[ "$1" =~ ^[0-9a-f]{64}$ ]]
}

normalize_tag() {
  case "$1" in
    latest) printf 'latest' ;;
    v*) printf '%s' "$1" ;;
    *) printf 'v%s' "$1" ;;
  esac
}

resolve_latest() {
  local effective
  need curl
  effective=$(curl_https -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest") \
    || fail "could not resolve latest release"
  printf '%s' "${effective##*/}"
}

install_from_cargo() {
  if [ "$ATTEST_MODE" = "require" ]; then
    fail "SILO_VERIFY_ATTESTATION=require refuses a cargo source build (no attestation); $1"
  fi
  info "$1 - falling back to cargo"
  if [ "$DRY_RUN" = "1" ]; then
    info "dry-run: would run cargo install --git https://github.com/${REPO} --tag ${tag} --locked --force"
    exit 0
  fi
  need cargo
  info "installing ${tag} from source via cargo…"
  cargo install --git "https://github.com/${REPO}" --tag "$tag" --locked --force
  if command -v silo >/dev/null 2>&1; then
    bold "installed $(command -v silo)"
    silo -V
  else
    local cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
    info "binary: ${cargo_bin}/silo  (add that dir to PATH if needed)"
    "${cargo_bin}/silo" -V || true
  fi
  info "next: silo init --count 10"
  exit 0
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

main() {
  if [ -n "${SILO_VERSION:-}" ]; then
    requested="$SILO_VERSION"
  elif [ -n "${VERSION:-}" ]; then
    warn "using VERSION=${VERSION} from the environment; prefer SILO_VERSION or --version"
    requested="$VERSION"
  else
    requested="$DEFAULT_VERSION"
  fi

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --version) [ "$#" -ge 2 ] || fail "--version needs a value"; requested="$2"; shift 2 ;;
      --version=*) requested="${1#--version=}"; shift ;;
      --install-dir) [ "$#" -ge 2 ] || fail "--install-dir needs a value"; INSTALL_DIR="$2"; shift 2 ;;
      --install-dir=*) INSTALL_DIR="${1#--install-dir=}"; shift ;;
      --dry-run) DRY_RUN=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) fail "unknown argument: $1 (try --help)" ;;
    esac
  done

  case "$ATTEST_MODE" in
    auto|require|skip) ;;
    *) fail "SILO_VERIFY_ATTESTATION must be auto, require, or skip" ;;
  esac

  tag=$(normalize_tag "$requested")
  if [ "$tag" = "latest" ]; then
    tag=$(resolve_latest)
  fi
  [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] \
    || fail "invalid release tag '${tag}' (expected vMAJOR.MINOR.PATCH or latest)"
  local ver="${tag#v}"

  # --- cargo-only path ---
  if [ "$SILO_FORCE_CARGO" = "1" ]; then
    install_from_cargo "SILO_FORCE_CARGO=1"
  fi

  need uname

  local os arch triple
  os=$(uname -s)
  arch=$(uname -m)
  case "$os/$arch" in
    Darwin/arm64|Darwin/aarch64) triple="aarch64-apple-darwin" ;;
    Darwin/x86_64)               triple="x86_64-apple-darwin" ;;
    Linux/x86_64|Linux/amd64)    triple="x86_64-unknown-linux-gnu" ;;
    Linux/aarch64|Linux/arm64)   triple="aarch64-unknown-linux-gnu" ;;
    *) install_from_cargo "no prebuilt binary for $os/$arch" ;;
  esac

  local asset="silo-${ver}-${triple}.tar.gz"
  local base_url="https://github.com/${REPO}/releases/download/${tag}"
  local pinned=""
  if pinned=$(pinned_sha256 "$asset"); then :; else pinned=""; fi
  if [ -n "$pinned" ] && [ "$ATTEST_MODE" = "require" ]; then
    fail "${tag} predates release attestations; pick a newer --version"
  fi

  bold "silo install"
  info "repo:    ${REPO}"
  info "version: ${tag}"
  info "target:  ${triple}"
  info "dest:    ${INSTALL_DIR}/silo"

  if [ "$DRY_RUN" = "1" ]; then
    info "dry-run: archive   ${base_url}/${asset}"
    if [ -n "$pinned" ]; then
      info "dry-run: sha256    ${pinned} (pinned in installer)"
    else
      info "dry-run: checksums ${base_url}/SHA256SUMS (required)"
    fi
    info "dry-run: attestation mode ${ATTEST_MODE} (signer ${SIGNER_WORKFLOW})"
    exit 0
  fi

  need curl
  need tar
  need awk

  tmp=$(mktemp -d "${TMPDIR:-/tmp}/silo-install.XXXXXX")
  trap 'rm -rf "$tmp"' EXIT

  local status
  status=$(fetch_status "${base_url}/${asset}" "${tmp}/${asset}") \
    || fail "download of ${asset} failed (network, TLS, or non-https redirect)"
  case "$status" in
    200) ;;
    404) install_from_cargo "release ${tag} has no prebuilt ${asset}" ;;
    *) fail "download of ${asset} returned HTTP ${status}" ;;
  esac

  # --- checksum (fail closed) ---
  local listed="" sums_name="" candidate
  for candidate in SHA256SUMS checksums.txt; do
    status=$(fetch_status "${base_url}/${candidate}" "${tmp}/${candidate}") \
      || fail "download of ${candidate} failed (network, TLS, or non-https redirect)"
    case "$status" in
      200)
        sums_name="$candidate"
        listed=$(checksum_entry "${tmp}/${candidate}" "$asset")
        break
        ;;
      404) ;;
      *) fail "download of ${candidate} returned HTTP ${status}" ;;
    esac
  done

  local expect source_label
  if [ -n "$pinned" ]; then
    if [ -n "$listed" ] && [ "$listed" != "$pinned" ]; then
      fail "${sums_name} disagrees with the pinned digest for ${asset}; refusing to install"
    fi
    expect="$pinned"
    source_label="pinned digest"
  elif [ -n "$listed" ]; then
    expect="$listed"
    source_label="${sums_name}"
  elif [ -n "$sums_name" ]; then
    fail "${sums_name} for ${tag} has no entry for ${asset}; refusing to install"
  else
    fail "release ${tag} publishes no SHA256SUMS; refusing to install an unverified binary (use SILO_FORCE_CARGO=1 to build from source)"
  fi
  is_sha256 "$expect" || fail "malformed checksum for ${asset}"

  local got
  got=$(sha256_file "${tmp}/${asset}")
  [ "$got" = "$expect" ] || fail "checksum mismatch for ${asset} (expected ${expect}, got ${got})"
  info "checksum: ok (${source_label})"

  # --- provenance (optional unless SILO_VERIFY_ATTESTATION=require) ---
  if [ "$ATTEST_MODE" = "skip" ]; then
    info "attestation: skipped (SILO_VERIFY_ATTESTATION=skip)"
  elif [ -n "$pinned" ]; then
    info "attestation: none for ${tag} (predates attestations; pinned digest checked)"
  elif command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    gh attestation verify "${tmp}/${asset}" \
      --repo "$REPO" \
      --signer-workflow "$SIGNER_WORKFLOW" \
      --source-ref "refs/tags/${tag}" \
      --deny-self-hosted-runners >/dev/null \
      || fail "GitHub attestation verification failed for ${asset}"
    info "attestation: ok (${SIGNER_WORKFLOW} @ ${tag})"
  elif [ "$ATTEST_MODE" = "require" ]; then
    fail "SILO_VERIFY_ATTESTATION=require needs an authenticated \`gh\` CLI"
  else
    info "attestation: skipped (install and log in to \`gh\` to verify provenance)"
  fi

  mkdir -p "${tmp}/extract"
  tar -xzf "${tmp}/${asset}" -C "${tmp}/extract"
  local bin="${tmp}/extract/silo-${ver}-${triple}/silo"
  if [ ! -f "$bin" ]; then
    # Older hand-packed archives may use a different top-level layout.
    bin=$(find "${tmp}/extract" -type f -name silo | head -1)
  fi
  [ -n "$bin" ] && [ -f "$bin" ] || fail "silo binary not found in archive"

  mkdir -p "$INSTALL_DIR"
  install -m 755 "$bin" "${INSTALL_DIR}/silo"

  bold "installed ${INSTALL_DIR}/silo"
  "${INSTALL_DIR}/silo" -V || true
  ensure_path_hint
  info "next: silo bootstrap --wrap --hook"
  info "      silo auth login <name>   # per profile, browser OAuth"
  info "      silo run <name>          # daily switch"
}

main "$@"
