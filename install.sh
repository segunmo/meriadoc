#!/usr/bin/env sh
set -e

REPO="segunmo/meriadoc"
BINARY="meriadoc"
INSTALL_DIR="${HOME}/.local/bin"

# --- Detect OS and architecture ---

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Linux)
    # Detect musl libc (Alpine and similar) vs glibc
    LIBC="gnu"
    if ldd /bin/sh 2>&1 | grep -q musl; then
      LIBC="musl"
    fi
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-${LIBC}" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)       echo "error: unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    EXT="tar.gz"
    ;;
  Darwin)
    case "$ARCH" in
      x86_64) TARGET="x86_64-apple-darwin" ;;
      arm64)  TARGET="aarch64-apple-darwin" ;;
      *)      echo "error: unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    EXT="tar.gz"
    ;;
  *)
    echo "error: unsupported OS: $OS" >&2
    echo "Download manually from https://github.com/${REPO}/releases" >&2
    exit 1
    ;;
esac

# --- Resolve version ---

if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
fi

if [ -z "$VERSION" ]; then
  echo "error: could not determine latest version" >&2
  exit 1
fi

# --- Download and install ---

ARCHIVE="${BINARY}-${VERSION}-${TARGET}.${EXT}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

echo "Installing ${BINARY} ${VERSION} (${TARGET})..."

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$ARCHIVE"
tar xzf "$TMP/$ARCHIVE" -C "$TMP"

mkdir -p "$INSTALL_DIR"
mv "$TMP/${BINARY}-${VERSION}-${TARGET}/${BINARY}" "$INSTALL_DIR/${BINARY}"
chmod +x "$INSTALL_DIR/${BINARY}"

echo ""
echo "Installed ${BINARY} ${VERSION} to ${INSTALL_DIR}/${BINARY}"

# --- PATH reminder ---

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "Note: ${INSTALL_DIR} is not in your PATH."
    echo "Add this to your shell profile (~/.zshrc, ~/.bashrc, etc.):"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac

# --- Completions hint ---

echo ""
echo "To install shell completions:"
echo "  bash:  ${BINARY} completions bash > ~/.local/share/bash-completion/completions/${BINARY}"
echo "  zsh:   ${BINARY} completions zsh > ~/.zfunc/_${BINARY}"
echo "  fish:  ${BINARY} completions fish > ~/.config/fish/completions/${BINARY}.fish"
echo ""
