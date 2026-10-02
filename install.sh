#!/bin/bash
set -euo pipefail

APP_NAME="anesis"
REPO="anesis-dev/anesis-cli"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)     OS="linux";;
    Darwin*)    OS="macos";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

case "$ARCH" in
    x86_64)     ARCH="x86_64";;
    arm64|aarch64) ARCH="aarch64";;
    *)          echo "Unsupported architecture: $ARCH"; exit 1;;
esac

LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "Failed to fetch latest version"
    exit 1
fi

echo "Installing $APP_NAME $LATEST_VERSION..."

ASSET="${APP_NAME}-${OS}-${ARCH}.tar.gz"
RELEASE_BASE_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION"
DOWNLOAD_URL="$RELEASE_BASE_URL/$ASSET"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"

echo "Downloading from $DOWNLOAD_URL..."
curl -fsSL -o "$ASSET" "$DOWNLOAD_URL"

if command -v sha256sum >/dev/null 2>&1; then
    sha256_of() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    echo "Neither sha256sum nor shasum is available; cannot verify the download." >&2
    exit 1
fi

echo "Verifying checksum..."
curl -fsSL -o SHA256SUMS "$RELEASE_BASE_URL/SHA256SUMS"

EXPECTED=$(grep " \+$ASSET\$" SHA256SUMS | head -n1 | cut -d' ' -f1)
if [ -z "$EXPECTED" ]; then
    echo "No checksum for $ASSET in SHA256SUMS" >&2
    exit 1
fi

ACTUAL=$(sha256_of "$ASSET")
if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "Checksum mismatch for $ASSET" >&2
    echo "  expected: $EXPECTED" >&2
    echo "  actual:   $ACTUAL" >&2
    exit 1
fi
echo "✓ Checksum verified"

tar xzf "$ASSET"

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
mv "$APP_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$APP_NAME"

cd -

echo "✓ $APP_NAME installed successfully to $INSTALL_DIR/$APP_NAME"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""

# Auto-install shell completions
SHELL_NAME=$(basename "${SHELL:-}" 2>/dev/null || true)
case "$SHELL_NAME" in
    bash|zsh|fish)
        "$INSTALL_DIR/$APP_NAME" completions "$SHELL_NAME" 2>/dev/null || true
        ;;
esac
