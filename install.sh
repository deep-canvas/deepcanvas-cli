#!/usr/bin/env sh
set -eu

REPO="deep-canvas/deepcanvas-cli"
BIN_NAME="deep"

detect_target() {
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "$os" in
        darwin) os="apple-darwin" ;;
        linux)  os="unknown-linux-musl" ;;
        *) echo "unsupported OS: $os" >&2; exit 1 ;;
    esac
    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) echo "unsupported arch: $arch" >&2; exit 1 ;;
    esac
    echo "${arch}-${os}"
}

detect_install_dir() {
    if [ -w /usr/local/bin ] 2>/dev/null; then
        echo "/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/'
}

TARGET="$(detect_target)"
INSTALL_DIR="$(detect_install_dir)"
VERSION="${DEEP_VERSION:-$(get_latest_version)}"

if [ -z "$VERSION" ]; then
    echo "could not determine latest version" >&2
    exit 1
fi

ARCHIVE="deep-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"
CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/deep-${VERSION}-checksums.txt"

echo "→ Installing deep ${VERSION} (${TARGET}) to ${INSTALL_DIR}/${BIN_NAME}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$ARCHIVE"
curl -fsSL "$CHECKSUMS_URL" -o "$TMP/checksums.txt" 2>/dev/null || true

# Verify checksum
if [ -f "$TMP/checksums.txt" ]; then
    cd "$TMP"
    if command -v sha256sum >/dev/null; then
        grep "  $ARCHIVE\$" checksums.txt | sha256sum -c -
    elif command -v shasum >/dev/null; then
        grep "  $ARCHIVE\$" checksums.txt | shasum -a 256 -c -
    fi
    cd - >/dev/null
fi

tar -xzf "$TMP/$ARCHIVE" -C "$TMP"
EXTRACTED="${TMP}/deep-${VERSION}-${TARGET}"
mv "${EXTRACTED}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo
echo "✓ Installed: ${INSTALL_DIR}/${BIN_NAME}"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*)
        echo "  Run: deep --help"
        ;;
    *)
        echo
        echo "⚠ ${INSTALL_DIR} is not in your PATH."
        echo "  Add to shell config:"
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac
