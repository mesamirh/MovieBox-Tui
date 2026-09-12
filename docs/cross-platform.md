# Cross-platform

Build targets: macOS, Linux, Windows, and Android (Termux via the Linux ARM64
binary). The codebase uses
`crossterm` (all platforms), `ratatui`, and a couple of small per-OS branches.

"Build target" does not mean every runtime integration is verified on every
device. Desktop builds are covered by CI; external players and Termux require
the release checks in [`release-checklist.md`](release-checklist.md).

## OS notes

| Platform         | Notes                                                                                                                                                                                                                                                                                                |
| :--- | :--- |
| macOS            | IINA preferred player (via the installed IINA `iina-cli`); VLC/mpv `.app` paths detected. `process_group(0)` on spawn.                                                                                                                                                                                |
| Windows          | mpv/VLC detected via Program Files + `%LOCALAPPDATA%`; path-safe file stems; static MSVC CRT linking (`+crt-static`) for zero-dependency standalone binaries; handle-safe atomic download replacement (dropping file handles before renames); background process spawning (registry queries, yt-dlp downloader, and update helper) hardened with canonical `CREATE_NO_WINDOW` (`0x08000000`). |
| Linux            | Flatpak mpv/VLC supported (`flatpak run …`); xdg data/cache dirs.                                                                                                                                                                                                                                    |
| Android (Termux) | Targets `aarch64-linux-android` linked against the Android NDK (Clang API 24+) and system Bionic `libc.so`, producing a native ELF executable with a valid `PT_PHDR` program header table and `/system/bin/linker64` dynamic loader. This eliminates `Could not find a PHDR: broken executable?` crashes and executes natively without on-device compilation. Includes a pure-Rust DNS resolver that reads system configuration and falls back to public resolvers (Cloudflare, Google, Quad9) for zero-config name resolution. Android playback delegates to external video players (VLC for Android, Just Player, MX Player, MPV Android) via Termux intent dispatchers (`termux-open`, `termux-open-url`, `termux-am`), requiring `pkg install -y termux-tools termux-am`. To protect against Android 10+ SELinux execute restrictions (exit code 126), system `/system/bin/am` fallbacks are strictly avoided in Termux; `LD_PRELOAD` is preserved for Termux applets while `PATH` is sanitized for system calls. Android intent playback supports unauthenticated streams (CircleFTP, DhakaFlix, IPTV) and standard CDN streams (4KHDHub). Real-device chooser behavior is a release prerequisite. |

## Terminal capabilities

The app probes the terminal at startup via `ratatui_image` (400ms cap,
off the UI thread). Non-graphics terminals (e.g. macOS `Apple_Terminal`,
legacy Windows `conhost`, and `TERM=dumb/linux/cygwin`) are skipped to
prevent escape sequence probe leakage (`Gi=31...`):

- **Poster rendering**: Sixel, Kitty, and iTerm2 protocols where the probe
  detects them. Terminals that report kitty/sixel capability but no cell
  size (Windows Terminal sixel, iTerm2 over SSH) are salvaged with default
  cell metrics. On terminals lacking graphics support (e.g. standard Windows
  PowerShell, legacy console host, macOS Terminal.app), search result cards
  and details screens display standardized bordered containers with centered `No Art`
  indicators, ensuring consistent layout geometry without confusing broken blocks.
  automatically gated behind `has_active_modal()` to prevent graphic bleed
  through modal popups and overlays. `MOVIEBOX_NO_IMAGE=1` disables queries;
  `MOVIEBOX_IMAGE_PROTOCOL` forces a protocol (`kitty`, `sixel`, `iterm2`);
  `MOVIEBOX_CELL_SIZE=WxH` overrides metrics.
- **Colors & Themes**: With no explicit theme, `NO_COLOR` wins, then truecolor RGB
  (auto-detected across Ghostty, Kitty, WezTerm, iTerm2, Alacritty, Foot, Windows Terminal,
  Hyper, Tabby, Warp, VSCode, Konsole, xfce4-terminal, GNOME Terminal, and Tilix; enabled by
  default on Windows Terminal via `WT_SESSION`), quantized 256-color palettes for strict terminals,
  fallback palette (`Theme::fallback`) using crisp cyan accents; an OSC 11 background query
  picks light/dark variants. Light mode themes (including Catppuccin Latte) are tuned for
  WCAG AA compliance, ensuring high-contrast readability across light terminal backgrounds.
  Modal pickers feature clean transparent border backdrops, minimum 7-row breathing room, and background
  selection suppression to isolate dialog focus. An explicit `MOVIEBOX_THEME` or saved theme
  always wins over autodetection.
- **Keyboard & Cursor**: The kitty keyboard protocol (disambiguated escapes, event
  types) is requested at start and popped on exit; unsupported terminals
  ignore it. Real input cursor styling (`SetCursorStyle::SteadyBar`) activates
  during text editing modes on supported terminals.
- **Loading & Progress Indicators**: Unicode Braille loading spinners (`⠋ ⠙ ⠹ ...`)
  render during search, metadata discovery, and stream fetching, falling back to
  clean text indicators (`..`) on basic/dumb terminals.
- **Window Titles**: Contextual terminal emulator window titles are emitted dynamically
  reflecting active navigation mode and title (`MovieBox-Tui — Streaming`,
  `MovieBox-Tui — Live TV`, `MovieBox-Tui — Addons`, `MovieBox-Tui — {Title}`).
- **Terminal classification**: `TERM=dumb`/`linux` fall back to a basic UI.
- Focus events re-render in place without clearing; results render in two
  columns from 110 columns wide (three from 160).
- **Responsive Installers**: Both Unix (`install.sh`) and Windows (`install.ps1`) installation scripts query terminal width dynamically (`tput cols` / `stty size` / `$Host.UI.RawUI.WindowSize.Width`), adapting the header across wide (72-column block art), compact (31-column 2-line half-block art for mobile/Termux portrait mode), and minimal (text-only) tiers with dynamic horizontal centering to prevent line wrapping.
## Network & TLS portability

- **TLS Engine**: Uses `rustls` with embedded Mozilla roots (`webpki-roots`) across all targets (macOS, Linux, Windows, Android/Termux). The release binary has no OpenSSL runtime dependency; the `ring` cryptography backend is compiled into the binary by the platform build toolchain.
- **DNS Resolution**: Pure-Rust resolver (hickory) on all platforms: it reads the OS
  configuration first (`/etc/resolv.conf`, registry on Windows) and falls back to
  embedded public resolvers (Cloudflare `1.1.1.1`, Google `8.8.8.8`, Quad9 `9.9.9.9`)
  when no system configuration exists — as on Android/Termux or minimal containers.
  No JNI or `ndk-context` required.
## In-App Self-Update Engine

MovieBox-TUI embeds an in-app binary upgrade engine (`src/updater/`) with cross-platform environment detection and deterministic fallback:

- **Direct Replacement**: Supported on Linux (`x86_64`, `aarch64`), macOS (Universal binary), and Windows (`x64`, `arm64`). Downloaded release archives are validated against SHA-256 checksums, unpacked to temporary staging files, and swapped atomically with rollback on failure.
- **Windows Helper Script**: On Windows, because running executables are locked by the OS, an external transient batch helper (`moviebox_update_helper.bat`) waits for the parent process PID to terminate, attempts atomic binary replacement with a 5-iteration retry loop (to tolerate antivirus/SmartScreen file locks), restarts the application, and self-deletes.
- **Homebrew Managed Environments**: Automatically detected via path markers (`/Cellar/`, `/opt/homebrew/`, `/usr/local/Cellar/`, `/home/linuxbrew/`). In-app binary overwrites are disabled to protect package manager integrity; the update modal displays Homebrew instructions (`brew upgrade moviebox-tui`) with a dedicated `[b]` shortcut.
- **Android / Termux**: Protects Termux environments from overwriting bionic libc binaries with incompatible Linux glibc binaries. Instructs users to re-run the universal installer script (`curl -fsSL ... | bash`).
- **Deterministic GitHub Asset Fallback**: When GitHub API requests are unauthenticated or rate-limited (status 403), tag resolution falls back to HTTP redirects, and asset download URLs are computed deterministically from release tags rather than failing update operations.
- **Input Isolation & Event Locking**: Update notifications defer blocking modal presentation while typing in search mode (`InputMode::Editing`) to prevent keystroke hijacking. During in-flight update execution (`is_updating`), all keyboard and mouse events are locked while displaying an active Braille progress spinner.

## Things to verify per release

- Player launch on each OS (mpv/VLC/IINA/Android intent) — window sizing, subtitles,
  headers.
- Poster rendering across Sixel (Windows Terminal, foot), Kitty, iTerm2, and basic
  non-graphics terminals.
- TV mode with a sample M3U playlist (URL and local file).
- Stremio Addons provider with sample HTTP addon manifests (Cinemeta, torrent/stream addons).
- Termux: on-device check that Play opens the Android chooser and the stream plays.
