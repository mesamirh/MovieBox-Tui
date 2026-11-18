#!/usr/bin/env bash
# Boot a simulator, install the built app and launch it.
#
# Usage: ./ios/scripts/run-simulator.sh [device-name]
#   device-name defaults to "iPhone 17 Pro"; any booted iOS runtime works.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEVICE_NAME="${1:-iPhone 17 Pro}"
BUNDLE_ID="com.moviebox.ios"
APP="$ROOT/target/ios/arm64/MovieBox.app"

if [[ ! -d "$APP" ]]; then
    echo "error: $APP is missing. Run ios/scripts/build-app.sh first." >&2
    exit 1
fi

echo "==> resolving simulator: $DEVICE_NAME"
UDID="$(xcrun simctl list devices available \
    | grep -F "$DEVICE_NAME (" \
    | head -1 \
    | sed -E 's/.*\(([0-9A-Fa-f-]{36})\).*/\1/')"

if [[ -z "$UDID" ]]; then
    echo "available devices:" >&2
    xcrun simctl list devices available >&2
    exit 1
fi

echo "==> booting $UDID"
xcrun simctl boot "$UDID" 2>/dev/null || true
open -a Simulator --args -CurrentDeviceUDID "$UDID" || true
xcrun simctl bootstatus "$UDID" -b >/dev/null

echo "==> installing"
xcrun simctl uninstall "$UDID" "$BUNDLE_ID" 2>/dev/null || true
xcrun simctl install "$UDID" "$APP"

echo "==> launching $BUNDLE_ID"
xcrun simctl launch --console-pty "$UDID" "$BUNDLE_ID"
