# Players

Playback is handed to an external player. `player.rs` detects available players and
builds the exact command; `tui/app/playback.rs` spawns it.

## Detection

`player::detect()` (powered by a centralized `probe_player_executable` engine) returns players in priority order, resolving paths dynamically with non-negative caching:

- macOS: IINA (if present), then mpv, then VLC (probing `/Applications`, `~/Applications`, Homebrew `/opt/homebrew/bin`, MacPorts `/opt/local/bin`, Nix profiles `~/.nix-profile/bin` and `/run/current-system/sw/bin`, standard `/bin`, and embedded app bundles `/Applications/IINA.app/Contents/MacOS/iina-cli` and `~/Applications/IINA.app/Contents/MacOS/iina-cli`).
- Linux: mpv, then VLC (probing native `$PATH`, user `.local/bin`, Flathub/Flatpak user & system exports `org.videolan.VLC` / `io.mpv.Mpv`, Snap `/snap/bin/*`, Nix profiles, standard `/bin`, and `flatpak run`).
- Windows: MPC-HC (if present), then mpv, then VLC (probing executable-adjacent binaries, WinGet Links & Packages directory `%LOCALAPPDATA%\Microsoft\WinGet\Packages`, `%USERPROFILE%\Downloads` and `%USERPROFILE%\Desktop` extractions, `Program Files` including `MPC-HC`, `K-Lite Codec Pack\MPC-HC64`, `mpv`, `mpv-player`, `mpv.net`, and `VideoLAN\VLC`, `LOCALAPPDATA\Programs`, portable drive roots `C:\mpc-hc`, `C:\mpv`, `C:\vlc`, `C:\tools`, Scoop shims & apps, Chocolatey, and Windows Registry `App Paths` and `Environment\Path`; zero-byte Windows Store App Execution Aliases in `WindowsApps` are intentionally excluded). MPC-HC is also found via `MOVIEBOX_MPC_PATH` or `PATH` (`mpc-hc64.exe`, `mpc-hc.exe`).
- Android/Termux: Android intent chooser (`termux-open`, `termux-open-url`, or `termux-am`), prioritized at index 0 ahead of CLI `mpv` and `vlc` to avoid selecting headless players lacking display servers. Streams play via external Android video players.

Resolution caches detected paths across runs while allowing newly installed players to be discovered dynamically when opening or navigating the Settings Hub (`/settings`), without requiring an application restart. A preferred player can be forced via `MOVIEBOX_PLAYER` env or `default_player` in config (e.g. `mpv`, `iina`, `vlc`, `mpc`, `android`), which reorders the list. The media player picker in the Settings Hub lists every detected player on your system and saves your selection to `config.json`. Playback launches directly using the preferred compatible player without intermediate modal dialogs.

## Command construction

| Player          | Invocation                                                                                                                                | Notes                                                                                                  |
| :--- | :--- | :--- |
| mpv             | `mpv --autofit=WxH --geometry=50%:50% --idle=no --keep-open=no --ytdl-format=bestvideo+bestaudio/best --hls-bitrate=max [--start=..] [--script=..] [--script-opts=..] [--http-header-fields=..] [--sub-file=..] <url>` | Window sized to the terminal. Injects `moviebox_tracker.lua` for position tracking and resume. Sets maximum adaptive stream bitrate and best video/audio format selection. Each non-UA, non-Referer header is passed as a separate `--http-header-fields` argument. Flatpak mpv is launched via `flatpak run` with `@@` file forwarding markers for local files. |
| VLC             | `vlc --width=W --height=H --play-and-exit --adaptive-logic=highest [--start-time=..] [--http-referrer=..] [--http-user-agent=..] [--sub-file=..] <url>` | Sets adaptive logic to highest representation. Supports start time resume via `--start-time`. For CloudFront cookie-gated streams (MovieBox DASH), the URL is replaced by a loopback sidecar proxy address (`http://127.0.0.1:<port>/https/<host>/path`) that injects all auth headers server-side before forwarding chunks to VLC. Flatpak VLC wraps local files with `@@` markers. |
| MPC-HC          | `mpc-hc64.exe [/sub <path>] <url>` | Windows only. Never receives `/ref` or `/ua` (unrecognized by K-Lite/clsid builds, which open help instead of playing) and never receives an empty `/sub ""`. Always plays through the loopback sidecar proxy, which injects headers upstream. |
| IINA            | `iina-cli --keep-running --no-stdin --mpv-autofit=.. [--mpv-start=..] [--mpv-http-header-fields=..] [--mpv-sub-files=..] <url>`               | Uses the installed IINA `iina-cli`; falls back to `open -a IINA <url>` with an informational status warning if the CLI is absent. |
| Android / Termux | `termux-open --chooser --content-type video/* <url>` (or `termux-open-url` / `termux-am`) | Opens an app chooser on the device, delegating playback to external apps (VLC, MX Player, Just Player, MPV Android). Requires `pkg install -y termux-tools termux-am`. Forwards `User-Agent`, `Referer`, and subtitles when using `termux-am`. |

Window size is derived from the live terminal size times the font cell size reported
by the image picker, then clamped to a fixed range.

## Headers

Playback sources (for example 4KHD and MovieBox) may carry `Referer`/`User-Agent` headers.
MovieBox DASH streams additionally carry signed `Cookie` headers for CloudFront authentication.
- mpv and IINA send headers via native CLI flags (`--user-agent`, `--referrer`, and `--http-header-fields`), and forward custom auth cookies via `--ytdl-raw-options-append` to yt-dlp hooks for DASH manifests.
while VLC maps them to `--http-referrer` / `--http-user-agent`. Android intent playback forwards `Referer`/`User-Agent` extras when using `am`/`termux-am`, and supports unauthenticated streams (CircleFTP, DhakaFlix, IPTV) and CDN streams natively.
The `supports_headers` gate in `app/playback.rs` validates whether a player can satisfy required stream headers. For VLC, Android Intent, and MPC-HC, signed CloudFront `Cookie` streams (such as MovieBox DASH manifests) are automatically supported via the detached loopback sidecar proxy (`--proxy-for-vlc`). The proxy injects the signed CloudFront cookies and auth headers server-side and streams chunks directly over localhost.
## Subtitles

- mpv receives the remote subtitle URL directly (`--sub-file=<url>`); mpv fetches it
  with the stream headers applied.
- VLC and IINA download the subtitle to a temp file first, preserving the URL's
  extension (srt/vtt/ass/…), and pass the local path. The download applies the source
  headers. On failure a status is shown and playback continues without subtitles.
- Android intent playback downloads subtitles to shared storage (`~/storage/downloads/moviebox_subs` or `/sdcard/Download/moviebox_subs`) named clearly after the media title (e.g. `<Title> - S<N:02>E<E:02>.<ext>` or `<Title>.<ext>`), and passes local file paths to `am`/`termux-am` via `subtitles_location` and `subs` intent extras.
  - Players supporting automatic subtitle intent binding (e.g. MX Player) load the file automatically.
  - For players that do not auto-load external file paths from remote stream intents (e.g. VLC for Android, mpv-android), the file is immediately available in the player's subtitle file picker under `Download/moviebox_subs/`.
  - Subtitles are retained during playback without premature deletion on intent command exit, and purged on a 24-hour retention cycle or via `/settings` → Clear Disk Cache.
- Desktop temp files are cleaned up after the player process exits and purged at startup if stale.

## Playback Tracking & Resume

When launching media with in-progress watch history, the player command automatically includes the starting position (`--start` / `--mpv-start` / `--start-time`).

- **Immediate Start Registration**: Media sessions are pre-registered into watch history (`record_start`) upon launch, ensuring that fast-exiting intent dispatchers (Android `termux-open` / `am start`, macOS `open -a IINA`) and unexpected terminal terminations retain history immediately.
- **Pre-Seeded State Files**: Before launching mpv or IINA, MovieBox-TUI creates a pending state file populated with metadata (`title`, `cover_url`, `stype`, `release_year`). If the application exits abruptly during playback, `reconcile_from_dir` self-heals and inserts new items into watch history without data loss.
- **Latched Lua Tracker (`moviebox_tracker.lua`)**: Observes `time-pos` and `duration` every 5 seconds. State file writes are atomic (`.tmp` file flushed and renamed with destination removal for Windows compatibility). Video completion at ≥ 90% or EOF is permanently latched, preventing player shutdown events from resetting completion status. Missing or live stream durations write JSON `null` to prevent invalid duration calculations.
- **Isolated Tracker vs Fallback Reconciliation**: Players with active Lua trackers (`mpv`, `iina-cli`) rely strictly on state file reconciliation, eliminating wall-clock progress overwrite races during pauses or seeks. Process elapsed time is used strictly as a guarded fallback for players without tracker scripts (e.g. VLC).
## Spawning

`launch_player` spawns the player with null stdin, piped stderr (plus piped stdout for
`PlayerKind::AndroidIntent` to intercept intent dispatcher error text), and its own
process group (Unix `process_group(0)`) or no-console flag (Windows `CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP`).

- **Concurrent Stderr Drain & Pipe Deadlock Prevention**: Stderr is read in a background thread concurrently with `child.wait()`, bounded by a 2-second timeout after process exit. This breaks pipe deadlocks when players (such as VLC with audio or output plugins) fork background subprocesses that inherit the pipe.
- **Clean Exit Normalization**: Routine exit states are normalized to avoid false crash reports:
  - VLC exit code `1` with empty stderr (normal stream completion under `--play-and-exit`) is classified as clean exit (`PlayerExited`).
  - Unix `SIGTERM` (signal 15) termination when the user closes player windows via the OS is treated as clean exit.
  - Genuine non-zero failures with stderr diagnostics are surfaced directly as actionable player errors.
  - When players exit without stderr diagnostics, exit codes are translated into actionable notifications (e.g. exit code `2` indicates `Stream Dead: Link expired or unreachable.`, exit code `1` indicates `Player Error: Check player configuration.`). Watch progress is reconciled only after a successful exit.

## Android / Termux Playback Architecture

On Android (Termux), terminal sessions do not have access to an X11 or Wayland display server by default:

- **Command-Line `mpv` (`pkg install mpv`)**: Operates in headless/audio-only mode. Video output cannot be rendered to the terminal screen and will fail or produce audio without video.
- **External Player Intent Dispatch**: Video playback is designed to open in dedicated Android video player applications (**VLC for Android**, **Just Player**, **MX Player**, or **MPV Android APK**).
- **Prerequisites**: Termux requires `pkg install -y termux-tools`.
  - `termux-open` (from `termux-tools`) broadcasts an `android.intent.action.VIEW` intent to `TermuxOpenReceiver`, presenting Android's native app chooser without relying on background sockets.
  - `termux-am` (from `termux-am`) attempts to connect to `am.sock` to pass custom stream headers and subtitles. If `am.sock` is unavailable or socket connection fails, MovieBox-TUI automatically falls back to `termux-open` transparently.
- **Actionable Mobile Diagnostics**: MovieBox-TUI parses intent dispatcher output and failure status codes, surfacing compact diagnostics formatted for mobile viewports:
  - **Missing video player app**: Surfaces `No Player` (`Install a video player.`).
  - **Missing intent tools**: Surfaces `Termux Setup` (`Run 'pkg install termux-tools'.`).
  - **Headless CLI mpv**: Detects video output initialization failures and surfaces `CLI mpv` (`Switch to Android Player in /settings.`).
  - **SELinux & Exit Code 126 Protection**: On Android 10+, executing `/system/bin/am` directly from an unrooted Termux environment causes Android's system shell to call `cmd activity`, which is blocked by SELinux when executing Termux app data binaries (`Permission denied`, exit code 126). MovieBox-TUI detects Termux environments, prevents illegal system `am` invocations, preserves `LD_PRELOAD` for Termux applet compatibility, strictly gates system `am` checks behind `target_os = "android"` (preventing GNU Automake collisions on desktop Linux), and surfaces actionable remediation notifications.
