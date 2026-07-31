#!/bin/bash
set -e

log_info() { echo -e "\033[0;34m$1\033[0m"; }
log_success() { echo -e "\033[0;32m$1\033[0m"; }
log_warn() { echo -e "\033[1;33mWARNING: $1\033[0m"; }
log_err() { echo -e "\033[0;31mERROR: $1\033[0m"; exit 1; }

INSTALL_DIR="/usr/local/bin"
BIN_NAME="moviebox-tui"
APP_PATH="$INSTALL_DIR/$BIN_NAME"

command -v curl >/dev/null 2>&1 || log_err "curl is required but not installed. Please install it."
command -v tar >/dev/null 2>&1 || log_err "tar is required but not installed. Please install it."

log_info "Fetching latest version information..."
LATEST_RELEASE=$(curl -sI "https://github.com/mesamirh/MovieBox-Tui/releases/latest")
VERSION=$(echo "$LATEST_RELEASE" | grep -i '^location:' | awk -F '/' '{print $NF}' | tr -d '\r')
[ -z "$VERSION" ] && log_err "Failed to fetch latest version from GitHub API."

if command -v "$BIN_NAME" > /dev/null 2>&1; then
    if strings "$APP_PATH" 2>/dev/null | grep -q "\-\-version"; then
        CURRENT_VERSION=$($BIN_NAME --version 2>/dev/null | awk '{print $2}')
    else
        CURRENT_VERSION="unknown"
    fi

    if [ -z "$CURRENT_VERSION" ]; then
        CURRENT_VERSION="unknown"
    fi

    if [ "v$CURRENT_VERSION" = "$VERSION" ]; then
        log_success "You already have the latest version ($VERSION) installed."
        exit 0
    fi
    log_info "Updating MovieBox-TUI from v$CURRENT_VERSION to $VERSION..."
    IS_UPDATE=1
else
    log_info "Installing MovieBox-TUI $VERSION..."
    IS_UPDATE=0
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Darwin" ]; then
    FILE="MovieBox_macOS_Universal.tar.gz"
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        FILE="MovieBox_Linux_x64.tar.gz"
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
        FILE="MovieBox_Linux_arm64.tar.gz"
    else
        log_err "Unsupported Linux architecture ($ARCH). Only x86_64 and arm64 are supported."
    fi
else
    log_err "Unsupported OS ($OS)."
fi

URL="https://github.com/mesamirh/MovieBox-Tui/releases/download/$VERSION/$FILE"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

log_info "Downloading $FILE..."
if ! curl -fsSL --progress-bar "$URL" -o "$TMP_DIR/$FILE"; then
    log_err "Download failed. Please check your internet connection."
fi

log_info "Extracting files..."
tar -xzf "$TMP_DIR/$FILE" -C "$TMP_DIR"

if [ ! -f "$TMP_DIR/$BIN_NAME" ]; then
    log_err "Binary not found in archive. This is an unexpected packaging error."
fi

log_info "Moving binary to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP_DIR/$BIN_NAME" "$APP_PATH"
    chmod +x "$APP_PATH"
else
    log_info "Requires sudo privileges to write to $INSTALL_DIR..."
    sudo mv "$TMP_DIR/$BIN_NAME" "$APP_PATH"
    sudo chmod +x "$APP_PATH"
fi

if ! echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
    log_warn "$INSTALL_DIR is not in your PATH. You may need to add it to run $BIN_NAME easily."
fi

if [ "$IS_UPDATE" -eq 1 ]; then
    log_success "Update complete! Run '$BIN_NAME' to start."
else
    log_success "Installation complete! Run '$BIN_NAME' to start."
fi
