#!/usr/bin/env bash
# Build the Rust core for iOS and (re)generate the UniFFI Swift bindings.
#
# Produces:
#   ios/Generated/moviebox_tui.swift   -- Swift bindings
#   ios/Generated/moviebox_tuiFFI.h    -- C header
#   ios/Generated/bridging.h           -- header included by swiftc
#   target/MovieBox.xcframework        -- device + simulator static libraries
#
# The device build uses `--lib` on purpose: the `moviebox-tui` binary is a
# terminal application and cannot link for iOS.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEVICE_TARGET="aarch64-apple-ios"
SIM_TARGET="aarch64-apple-ios-sim"
OUT_DIR="$ROOT/ios/Generated"
XCFRAMEWORK="$ROOT/target/MovieBox.xcframework"

FEATURES="mobile"

echo "==> ensure Rust targets are installed"
rustup target add "$DEVICE_TARGET" "$SIM_TARGET" >/dev/null

echo "==> build device static library"
cargo build --lib --release --features "$FEATURES" --target "$DEVICE_TARGET"

echo "==> build simulator static library"
cargo build --lib --release --features "$FEATURES" --target "$SIM_TARGET"

echo "==> generate Swift bindings"
mkdir -p "$OUT_DIR"
cargo run --quiet --features "$FEATURES" --bin uniffi-bindgen -- generate \
    --library "$ROOT/target/$SIM_TARGET/release/libmoviebox_tui.a" \
    --language swift \
    --out-dir "$OUT_DIR"

cat > "$OUT_DIR/bridging.h" <<'HEADER'
#include "moviebox_tuiFFI.h"
HEADER

echo "==> assemble XCFramework"
rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
    -library "$ROOT/target/$DEVICE_TARGET/release/libmoviebox_tui.a" \
    -headers "$OUT_DIR" \
    -library "$ROOT/target/$SIM_TARGET/release/libmoviebox_tui.a" \
    -headers "$OUT_DIR" \
    -output "$XCFRAMEWORK" >/dev/null

echo "done:"
echo "  bindings   -> $OUT_DIR"
echo "  xcframework-> $XCFRAMEWORK"
