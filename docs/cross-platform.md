# Cross-Platform Support

MovieBox-TUI runs on **macOS**, **Linux**, **Windows**, and **Android (Termux)**.

## Supported Operating Systems

### macOS
- **Media Players**: `IINA` (CLI + app bundle), `mpv`, and `VLC`.
- **Binaries**: Shipped as universal binaries supporting both Apple Silicon (`arm64`) and Intel (`x86_64`).

### Linux
- **Media Players**: Native `mpv`, `VLC`, or Flatpaks (`flatpak run --file-forwarding --filesystem=xdg-cache/moviebox-tui:ro --filesystem=xdg-data/moviebox-tui --filesystem=/tmp:ro <app-id>`).
- **Binaries**: Static Musl executables compatible with glibc and musl systems across x86_64 and aarch64.

### Windows
- **Media Players**: Discovers `mpv.exe` and `vlc.exe` via WinGet, Scoop, Program Files, and the Windows Registry.
- **Process Safety**: Processes launch with `CREATE_NO_WINDOW` and independent process groups so terminal signals do not terminate playback or downloads.
- **Path Compatibility**: Supports Windows drive letters (`C:\...`), backslashes (`\`), forward slashes (`/`), and UNC paths.

### Android (Termux)
- **Media Players**: Uses Android intent dispatchers (`termux-open` / `am start`) to stream directly to external video players (VLC for Android, Just Player, MX Player, MPV Android) in headless CLI environments.
- **Graphical Desktops (Termux:X11, uDroid, PRoot)**: When an active `$DISPLAY` or `$WAYLAND_DISPLAY` is present, native desktop players (`mpv`, `VLC`) take priority over Android intent dispatchers.
- **Prerequisites**: Run `pkg install -y termux-tools` in Termux.
- **Subtitles**: Subtitles are saved to shared storage (`~/storage/downloads/moviebox_subs`) or served over the local `StreamRelay` loopback HTTP proxy when Termux shared storage is not mounted.
- **DNS**: Built-in DNS resolver queries public resolvers (Cloudflare, Google, Quad9) without requiring root or Android JNI.

### iOS (native app)
- **Not a terminal client**: `ios/` is a SwiftUI app that links the same Rust core as a static library through UniFFI, so no terminal, shell or external player is involved.
- **Targets**: Deployment target iOS 17.0, arm64 device and arm64 simulator static libraries, packaged as `target/MovieBox.xcframework`.
- **Playback**: `AVPlayerViewController` with provider headers forwarded per request; subtitles render as a timed overlay because AVPlayer cannot attach arbitrary remote subtitle files.
- **Build**: Script-driven (`ios/scripts/`), no Xcode project, no App Store distribution path.

See [iOS Client](ios-client.md) for the FFI surface, build pipeline and limitations.
## Terminal Compatibility

MovieBox-TUI automatically adapts to your terminal emulator:

- **Images & Posters**: Automatically detects Kitty graphics, Sixel, and iTerm2 protocols (Ghostty, Kitty, WezTerm, iTerm2, foot, Alacritty, and Windows Terminal v1.22+). In Windows Terminal v1.22+, enable Sixel under **Settings → Profiles → Defaults → Advanced → Enable Sixel graphics** (or `"experimental.sixelSupport": true` in `settings.json`). Terminals without image support display clean text containers (`No Art`). Disable with `MOVIEBOX_NO_IMAGE=1` or force with `MOVIEBOX_IMAGE_PROTOCOL=sixel`.
- **Colors & Themes**: Auto-detects 24-bit TrueColor, 256-color palettes, and high-contrast monochrome mode (`NO_COLOR=1`).
- **Keyboard Navigation**: Uses the Kitty keyboard protocol on supported emulators. Mobile Termux keyboards use standard ANSI input to avoid character garbage.
- **Window Titles**: Automatically updates terminal window titles based on current title and screen mode.
