# Android 5.x Legacy Install (Oppo A37fw and similar)

This guide covers **Android 5.0–5.1.1 (API 21–22)** Termux installs, e.g. Oppo A37fw (`aarch64 / arm64-v8a / sdk 22`).
Official `MovieBox_Android_arm64.tar.gz` before the API-21 fix fails here with `CANNOT LINK EXECUTABLE: empty/missing DT_HASH`. Use the API-21 build below.

> **Scope:** Android 5.0+ only. Android 4.4 and below (API ≤ 20) are **not supported** — current Rust (1.90) and NDK (r26d) minimum is API 21. This guide is `aarch64` only; `armv7l` 32-bit devices need a separate `armv7-linux-androideabi21` build.

## 1. Preflight

On the phone (Termux or `ssh -p 8022`):

```bash
uname -m; getprop ro.product.cpu.abi; getprop ro.build.version.sdk
# Oppo A37fw expected:
# aarch64
# arm64-v8a
# 22
```

- `sdk 21/22` → this guide applies.
- `aarch64/arm64` → use the `arm64_api21` tarball.
- `armv7l/armeabi-v7a` → stop, you need an armv7 API-21 build (see §7).
- `sdk <= 20` → not supported.

Termux on Android 5/6 must be **Termux 0.118 APK** (F-Droid archive). Play Store builds require Android 7+. Expect on launch:

```
You are using legacy Termux environment.
Packages are unmaintained and will not receive any updates.
```

That warning is normal.

## 2. Install Termux deps

```bash
pkg install -y curl tar termux-tools termux-am
termux-setup-storage
```

For playback install an Android 5.1-compatible player (VLC 3.6 from F-Droid archive, MX Player, or Just Player). Streams launch via Android intent, not inside the terminal.

## 3. Download the API-21 binary

Do **not** use `/tmp` — it is not writable on legacy Termux (`curl: (23) Failed writing body`). Use `$HOME`.

After this PR merges, the normal installer works:

```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

Until then (v0.1.26 and earlier), install the legacy asset directly:

```bash
rm -f ~/mb.tar.gz
curl -fsSL https://github.com/pdev-labs/MovieBox-Tui/releases/download/api21-v0.1.26/MovieBox_Android_arm64_api21.tar.gz -o ~/mb.tar.gz
mkdir -p ~/mb && tar -xzf ~/mb.tar.gz -C ~/mb
cp ~/mb/moviebox-tui $PREFIX/bin/
chmod 755 $PREFIX/bin/moviebox-tui
moviebox-tui --version
```

Expected (exit 0):

```
WARNING: linker: Unsupported flags DT_FLAGS_1=0x8000001
WARNING: linker: moviebox-tui: unused DT entry: type 0x6ffffef5 arg 0x1188
...
moviebox-tui 0.1.26
```

Warnings are the old linker ignoring GNU extensions; it uses the SYSV `DT_HASH` this build restores. The old official build instead ended with `CANNOT LINK EXECUTABLE` + exit 1.

SHA256: `45478a8595e2e7840a76197bb7c65a0f1526ced928979f8b6b03f1a4303071b0`

## 4. Run

```bash
moviebox-tui
```

Over SSH (`ssh -p 8022 u0_xxx@<phone-ip>` from PC) the TUI works, and playback intents still fire on the phone itself.

## 5. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `curl` stuck with no output | Slow link / IPv6 on old stack, 4.6 MB looks hung | `curl -4 -L --progress-bar <url> -o ~/mb.tar.gz` |
| `curl: (23) Failed writing body` | `/tmp` not writable on Termux | Use `~/mb.tar.gz`, never `/tmp/mb.tar.gz` |
| `CANNOT LINK EXECUTABLE: empty/missing DT_HASH` | Pre-API-21-fix official asset | Use API-21 tarball above |
| `Unsupported flags / unused DT entry` + version prints | Harmless, old linker ignoring GNU_HASH | Ignore |
| `curl: SSL / certificate` | Stale CA bundle | `pkg reinstall ca-certificates`, retry |
| Download blocked (`objects.githubusercontent.com`) | Network filter | Download on PC via `gh release download api21-v0.1.26 --repo pdev-labs/MovieBox-Tui`, then `adb push moviebox-tui /sdcard/` → `cp /sdcard/moviebox-tui $PREFIX/bin/` |
| `Unsupported Termux architecture (armv7l)` from `install.sh` | 32-bit userspace | Needs armv7 API-21 build, not covered here |

## 6. Rebuilding yourself (GitHub Actions)

No PC toolchain needed. Fork → the fixed `.github/workflows/release.yml` builds `aarch64-linux-android21-clang` + `-Wl,--hash-style=both`. Manual legacy workflow used for verification (`aarch64-linux-android`, NDK r26d, `readelf -dW | grep "(HASH)"` gate) is the reference. Download the `MovieBox_Android_arm64_api21` artifact and install per §3.

## 7. Note for armv7 / Android ≤ 4.4

- `armv7l` devices: same flags but target `armv7-linux-androideabi` with `armv7a-linux-androideabi21-clang`. No hosted asset in this PR.
- API ≤ 20: unsupported by Rust 1.90 / NDK r26d. Would require downgrading toolchain and dependencies; out of scope.
