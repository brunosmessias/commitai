#!/bin/sh
# commitai installer
# https://github.com/brunosmessias/commitai
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/brunosmessias/commitai/main/install.sh | sh
#   curl -fsSL ... | sh -s -- --to /usr/local/bin
#   curl -fsSL ... | sh -s -- --version v0.1.0
#
# Detects your OS/arch, downloads the matching binary from the latest GitHub
# Release of commitai, and installs it to ~/.local/bin (or the path you pass
# via --to, or /usr/local/bin if you run as root).

set -eu

REPO="brunosmessias/commitai"
BINARY="commitai"
DEFAULT_VERSION="latest"

print()  { printf '%s\n' "$*"; }
err()    { printf 'error: %s\n' "$*" >&2; }
have()   { command -v "$1" >/dev/null 2>&1; }

usage() {
  cat <<EOF
commitai installer

Usage:
  install.sh [options]

Options:
  --to <DIR>        Install directory (default: ~/.local/bin, or /usr/local/bin if running as root)
  --version <VER>   Install a specific version (default: latest release)
  --repo <OWNER/REPO>  GitHub repo to install from (default: $REPO)
  -h, --help        Show this help
EOF
}

# --- arg parsing -------------------------------------------------------------

INSTALL_TO=""
VERSION="$DEFAULT_VERSION"
while [ $# -gt 0 ]; do
  case "$1" in
    --to)       INSTALL_TO="$2"; shift 2 ;;
    --version)  VERSION="$2"; shift 2 ;;
    --repo)     REPO="$2"; shift 2 ;;
    -h|--help)  usage; exit 0 ;;
    *) err "unknown option: $1"; usage; exit 1 ;;
  esac
done

# --- preflight ---------------------------------------------------------------

if ! have curl && ! have wget; then
  err "need curl or wget to download the binary"
  exit 1
fi
if ! have tar && ! have unzip; then
  err "need tar (and unzip on Windows) to extract the archive"
  exit 1
fi

# --- pick install dir --------------------------------------------------------

if [ -z "$INSTALL_TO" ]; then
  if [ "$(id -u 2>/dev/null || echo 1000)" -eq 0 ]; then
    INSTALL_TO="/usr/local/bin"
  else
    INSTALL_TO="$HOME/.local/bin"
    mkdir -p "$INSTALL_TO"
  fi
fi

# --- detect target -----------------------------------------------------------

# Map uname output to a Rust target triple. This intentionally covers only the
# targets our release workflow builds; we fail loud on anything else rather
# than guess and ship the wrong binary.
TARGET=""
case "$(uname -s)" in
  Linux)
    case "$(uname -m)" in
      x86_64|amd64)   TARGET="x86_64-unknown-linux-musl" ;;
      aarch64|arm64)  TARGET="aarch64-unknown-linux-musl" ;;
      *) err "unsupported Linux arch: $(uname -m)"; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$(uname -m)" in
      x86_64)         TARGET="x86_64-apple-darwin" ;;
      arm64|aarch64)  TARGET="aarch64-apple-darwin" ;;
      *) err "unsupported macOS arch: $(uname -m)"; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    TARGET="x86_64-pc-windows-msvc"
    ;;
  *)
    err "unsupported OS: $(uname -s). Use the prebuilt binaries from the Releases page manually."
    exit 1
    ;;
esac

EXT="tar.gz"
[ "$TARGET" = "x86_64-pc-windows-msvc" ] && EXT="zip"
BIN_EXT=""
[ "$TARGET" = "x86_64-pc-windows-msvc" ] && BIN_EXT=".exe"

# --- resolve version ---------------------------------------------------------

if [ "$VERSION" = "latest" ]; then
  RELEASE_URL="https://api.github.com/repos/$REPO/releases/latest"
  print "› fetching latest release metadata…"
  if have curl; then
    TAG=$(curl -fsSL "$RELEASE_URL" \
      | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p')
  else
    TAG=$(wget -qO- "$RELEASE_URL" \
      | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p')
  fi
  if [ -z "$TAG" ]; then
    err "could not determine latest release from $RELEASE_URL"
    err "if this is a brand-new repo, push a v* tag and wait for the release workflow to finish"
    exit 1
  fi
else
  TAG="$VERSION"
fi

VERSION_NUM="${TAG#v}"
ASSET="${BINARY}-v${VERSION_NUM}-${TARGET}${BIN_EXT}.${EXT}"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"

# --- download ----------------------------------------------------------------

TMPDIR=$(mktemp -d 2>/dev/null || mktemp -d -t commitai)
trap 'rm -rf "$TMPDIR"' EXIT INT TERM

print "› downloading $ASSET"
if have curl; then
  if ! curl -fSL --progress-bar "$DOWNLOAD_URL" -o "$TMPDIR/$ASSET"; then
    err "download failed: $DOWNLOAD_URL"
    err "check that release $TAG exists and has an asset for $TARGET"
    exit 1
  fi
else
  if ! wget -q --show-progress "$DOWNLOAD_URL" -O "$TMPDIR/$ASSET"; then
    err "download failed: $DOWNLOAD_URL"
    exit 1
  fi
fi

# --- extract -----------------------------------------------------------------

print "› extracting"
cd "$TMPDIR"
if [ "$EXT" = "zip" ]; then
  unzip -q "$ASSET"
else
  tar -xzf "$ASSET"
fi
EXTRACTED="$TMPDIR/${BINARY}${BIN_EXT}"
if [ ! -f "$EXTRACTED" ]; then
  err "expected binary $EXTRACTED not found in archive"
  exit 1
fi
chmod +x "$EXTRACTED"

# --- install -----------------------------------------------------------------

print "› installing to $INSTALL_TO/$BINARY"
if [ -w "$INSTALL_TO" ]; then
  mv "$EXTRACTED" "$INSTALL_TO/$BINARY"
else
  err "$INSTALL_TO is not writable; rerun with sudo or pass --to <writable dir>"
  exit 1
fi

# --- done --------------------------------------------------------------------

# Path hint, only when the user might not have it on PATH.
case ":$PATH:" in
  *":$INSTALL_TO:"*) ;;
  *)
    print ""
    print "  note: $INSTALL_TO is not on your \$PATH."
    print "  add it with:  export PATH=\"$INSTALL_TO:\$PATH\""
    print ""
    ;;
esac

print "✓ $BINARY $TAG installed. try: $BINARY --version"
