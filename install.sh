#!/bin/bash
set -e

echo "Installing MovieBox-Tui..."

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Darwin" ]; then
    FILE="MovieBox_macOS_Universal.tar.gz"
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        FILE="MovieBox_Linux_x64.tar.gz"
    else
        echo "Error: Unsupported Linux architecture: $ARCH. Only x86_64 is supported currently."
        exit 1
    fi
else
    echo "Error: Unsupported OS: $OS"
    exit 1
fi

URL="https://github.com/mesamirh/MovieBox-Tui/releases/latest/download/$FILE"
TMP_DIR=$(mktemp -d)

echo "Downloading latest release for $OS..."
curl -fsSL "$URL" -o "$TMP_DIR/$FILE"

echo "Extracting..."
tar -xzf "$TMP_DIR/$FILE" -C "$TMP_DIR"

INSTALL_DIR="/usr/local/bin"
echo "Installing to $INSTALL_DIR (may require sudo password)..."
sudo mv "$TMP_DIR/moviebox" "$INSTALL_DIR/moviebox-tui"
sudo chmod +x "$INSTALL_DIR/moviebox-tui"

rm -rf "$TMP_DIR"

echo "Success! You can now run 'moviebox-tui' from anywhere in your terminal."
