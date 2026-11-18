#!/usr/bin/env bash
# Compile the SwiftUI app for the iOS Simulator and assemble a runnable .app
# bundle without requiring an Xcode project.
#
# Usage: ./ios/scripts/build-app.sh [simulator-arch] [deployment-target]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ARCH="${1:-arm64}"
DEPLOYMENT="${2:-17.0}"
SIM_TARGET="${ARCH}-apple-ios-sim"

SDK_PATH="$(xcrun --sdk iphonesimulator --show-sdk-path)"
RUST_LIB="$ROOT/target/aarch64-apple-ios-sim/release/libmoviebox_tui.a"
GENERATED="$ROOT/ios/Generated"
APP_SRC="$ROOT/ios/MovieBox"
BUNDLE="$ROOT/target/ios/$ARCH/MovieBox.app"

if [[ ! -f "$RUST_LIB" ]]; then
    echo "error: $RUST_LIB is missing. Run ios/scripts/build-rust.sh first." >&2
    exit 1
fi
if [[ ! -f "$GENERATED/moviebox_tui.swift" ]]; then
    echo "error: Swift bindings are missing. Run ios/scripts/build-rust.sh first." >&2
    exit 1
fi

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE"

echo "==> compiling Swift for $ARCH (iOS $DEPLOYMENT, simulator)"
# shellcheck disable=SC2046
swiftc \
    -sdk "$SDK_PATH" \
    -target "${ARCH}-apple-ios${DEPLOYMENT}-simulator" \
    -swift-version 5 \
    -O \
    -import-objc-header "$GENERATED/bridging.h" \
    -o "$BUNDLE/MovieBox" \
    $(find "$APP_SRC" -name '*.swift' | sort) \
    "$GENERATED/moviebox_tui.swift" \
    "$RUST_LIB" \
    -framework Foundation \
    -framework UIKit \
    -framework SwiftUI \
    -framework AVKit \
    -framework AVFoundation \
    -framework CoreGraphics \
    -framework CoreMedia \
    -framework Security \
    -framework SystemConfiguration \
    -framework Network \
    -framework CoreFoundation \
    -liconv \
    -lresolv \
    -lc++ \
    -lz

cp "$APP_SRC/Info.plist" "$BUNDLE/Info.plist"
if [[ -d "$APP_SRC/Assets.xcassets" ]]; then
    xcrun actool --compile "$BUNDLE" \
        --platform iphonesimulator \
        --minimum-deployment-target "$DEPLOYMENT" \
        --app-icon AppIcon \
        --output-partial-info-plist "$BUNDLE/assetcatalog.plist" \
        "$APP_SRC/Assets.xcassets" >/dev/null 2>&1 || true
fi

echo "==> built $BUNDLE"
