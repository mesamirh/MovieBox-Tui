# Players

Playback is handed to an external player. `player.rs` detects available players and
builds the exact command; `tui/app/playback.rs` spawns it.

## Detection

`player::detect()` (powered by a centralized `probe_player_executable` engine) returns players in priority order, resolving paths dynamically with non-negative caching:

- macOS: IINA (if present), then mpv, then VLC (probing `/Applications`, `~/Applications`, Homebrew `/opt/homebrew/bin`, MacPorts `/opt/local/bin`, Nix profiles `~/.nix-profile/bin` and `/run/current-system/sw/bin`, and standard `/bin`).
- Linux: mpv, then VLC (probing native `$PATH`, user `.local/bin`, Flathub/Flatpak user & system exports `org.videolan.VLC` / `io.mpv.Mpv`, Snap `/snap/bin/*`, Nix profiles, standard `/bin`, and `flatpak run`).
- Windows: mpv, then VLC (probing executable-adjacent binaries, WinGet Links & Packages directory `%LOCALAPPDATA%\Microsoft\WinGet\Packages`, `%USERPROFILE%\Downloads` and `%USERPROFILE%\Desktop` extractions, `Program Files` including `mpv`, `mpv-player`, `mpv.net`, and `VideoLAN\VLC`, `LOCALAPPDATA\Programs`, portable drive roots `C:\mpv`, `C:\vlc`, `C:\tools`, Scoop shims & apps, Chocolatey, and Windows Registry `App Paths` and `Environment\Path`).
- Android/Termux: Android intent chooser (`termux-open`, `termux-open-url`, or `termux-am`). Streams play via external Android video players (VLC for Android, Just Player, MX Player, MPV Android APK).

Resolution caches detected paths across runs while allowing newly installed players to be discovered dynamically when opening or navigating the Settings Hub (`/settings`), without requiring an application restart. A preferred player can be forced via `MOVIEBOX_PLAYER` env or `default_player` in config (e.g. `mpv`, `iina`, `vlc`, `android`), which reorders the list. The media player picker in the Settings Hub lists every detected player on your system and saves your selection to `config.json`. Playback launches directly using the preferred compatible player without intermediate modal dialogs.

## Command construction

| Player          | Invocation                                                                                                                                | Notes                                                                                                  |
| :--- | :--- | :--- |
| mpv             | `mpv --autofit=WxH --geometry=50%:50% --idle=no --keep-open=no [--start=..] [--script=..] [--script-opts=..] [--http-header-fields=..] [--sub-file=..] <url>` | Window sized to the terminal. Injects `moviebox_tracker.lua` for position tracking and resume. Flatpak mpv is launched via `flatpak run`. |
| VLC             | `vlc --width=W --height=H --play-and-exit [--start-time=..] [--http-referrer=..] [--http-user-agent=..] [--sub-file=..] <url>`            | Supports start time resume via `--start-time`.                                                         |
| IINA            | `iina-cli --keep-running --no-stdin --mpv-autofit=.. [--mpv-start=..] --mpv-http-header-fields=.. --mpv-sub-files=.. <url>`               | Uses the installed IINA `iina-cli`; falls back to `open -a IINA <url>` only if the CLI is absent.      |
| Android / Termux | `termux-open --chooser --content-type video/* <url>` (or `termux-open-url` / `termux-am`) | Opens an app chooser on the device, delegating playback to external apps (VLC, MX Player, Just Player, MPV Android). Requires `pkg install -y termux-tools termux-am`. Forwards `User-Agent`, `Referer`, and subtitles when using `termux-am`. |

Window size is derived from the live terminal size times the font cell size reported
by the image picker, then clamped to a fixed range.

## Headers

Playback sources (for example 4KHD and MovieBox) may carry `Referer`/`User-Agent` headers.
MovieBox DASH streams additionally carry signed `Cookie` headers for CloudFront authentication.
mpv/IINA send them via `http-header-fields` (`--http-header-fields=...` or `--mpv-http-header-fields=...`),
while VLC maps them to `--http-referrer` / `--http-user-agent`. Android intent playback forwards `Referer`/`User-Agent` extras when using `am`/`termux-am`, and supports unauthenticated streams (CircleFTP, DhakaFlix, IPTV) and CDN streams natively.
The `supports_headers` gate in `app/playback.rs` validates whether a player can satisfy required stream headers. MovieBox-TUI strictly respects the user's configured default player: if the chosen player cannot satisfy a stream's headers (such as VLC or Android Player attempting to play signed MovieBox DASH manifests with CloudFront cookie requirements), playback will not silently fall back to an alternative player. Instead, an explicit warning notification informs the user of the lack of header support, prompting them to switch to a supported player or install one for their operating system (`mpv` and `IINA` on macOS; `mpv` on Linux and Windows).
## Subtitles

- mpv receives the remote subtitle URL directly (`--sub-file=<url>`); mpv fetches it
  with the stream headers applied.
- VLC and IINA download the subtitle to a temp file first, preserving the URL's
  extension (srt/vtt/ass/…), and pass the local path. The download applies the source
  headers. On failure a status is shown and playback continues without subtitles.
- Android intent playback passes subtitle paths to `am`/`termux-am` via `subtitles_location` and `subs` intent extras.
- Temp files are cleaned up after the player exits and purged at startup if stale.

## Playback Tracking & Resume

When launching media with in-progress watch history, the player command automatically includes the starting position (`--start` / `--mpv-start` / `--start-time`).

- **Immediate Start Registration**: Media sessions are pre-registered into watch history (`record_start`) upon launch, ensuring that fast-exiting intent dispatchers (Android `termux-open` / `am start`, macOS `open -a IINA`) and unexpected terminal terminations retain history immediately.
- **Pre-Seeded State Files**: Before launching mpv or IINA, MovieBox-TUI creates a pending state file populated with metadata (`title`, `cover_url`, `stype`, `release_year`). If the application exits abruptly during playback, `reconcile_from_dir` self-heals and inserts new items into watch history without data loss.
- **Latched Lua Tracker (`moviebox_tracker.lua`)**: Observes `time-pos` and `duration` every 5 seconds. State file writes are atomic (`.tmp` file flushed and renamed with destination removal for Windows compatibility). Video completion at ≥ 90% or EOF is permanently latched, preventing player shutdown events from resetting completion status. Missing or live stream durations write JSON `null` to prevent invalid duration calculations.
- **Isolated Tracker vs Fallback Reconciliation**: Players with active Lua trackers (`mpv`, `iina-cli`) rely strictly on state file reconciliation, eliminating wall-clock progress overwrite races during pauses or seeks. Process elapsed time is used strictly as a guarded fallback for players without tracker scripts (e.g. VLC).
## Spawning

`launch_player` spawns the player with null stdin/stdout, piped stderr, and its own
process group (Unix) or no-console flag (Windows). A blocking task reads stderr and
reports every non-zero process exit as a player error, including failures with no
diagnostic output. Watch progress is reconciled only after a successful exit.

## Android / Termux Playback Architecture

On Android (Termux), terminal sessions do not have access to an X11 or Wayland display server by default:

- **Command-Line `mpv` (`pkg install mpv`)**: Operates in headless/audio-only mode. Video output cannot be rendered to the terminal screen and will fail or produce audio without video.
- **External Player Intent Dispatch**: Video playback is designed to open in dedicated Android video player applications (**VLC for Android**, **Just Player**, **MX Player**, or **MPV Android APK**).
- **Prerequisites**: Termux requires `pkg install -y termux-tools termux-am`.
  - `termux-open` (from `termux-tools`) broadcasts an `android.intent.action.VIEW` intent to `TermuxOpenReceiver`, presenting Android's native app chooser.
  - `termux-am` (from `termux-am`) connects directly to `termux-app`'s local Unix domain socket (`am.sock`), allowing intent parameter passing (including `User-Agent`, `Referer`, and subtitles).
  - **SELinux & Exit Code 126 Protection**: On Android 10+, executing `/system/bin/am` directly from an unrooted Termux environment causes Android's system shell to call `cmd activity`, which is blocked by SELinux when executing Termux app data binaries (`Permission denied`, exit code 126). MovieBox-TUI detects Termux environments, prevents illegal system `am` invocations, preserves `LD_PRELOAD` for Termux applet compatibility, and surfaces actionable remediation notifications.
