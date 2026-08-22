# Players

Playback is handed to an external player. `player.rs` detects available players and
builds the exact command; `tui/app/playback.rs` spawns it.

## Detection

`player::detect()` returns players in priority order:

- macOS: IINA (if present), then mpv, then VLC (probing `/Applications`, `~/Applications`, Homebrew `/opt/homebrew/bin`, and MacPorts).
- Linux: mpv, then VLC (probing Native `$PATH`, Flathub/Flatpak user & system exports `org.videolan.VLC` / `io.mpv.Mpv`, Snap `/snap/bin/*`, and `flatpak run`).
- Windows: mpv, then VLC (probing `Program Files`, `LOCALAPPDATA\Programs`, `Microsoft\WindowsApps`, Scoop `scoop\shims`, and Chocolatey).
- Android/Termux: the Android intent fallback is attempted last when `termux-open` or `am` is available.

Resolution runs once at startup and is cached (`OnceLock`). A preferred player can be
forced via `MOVIEBOX_PLAYER` env or `default_player` in config (e.g. `mpv`, `iina`,
`vlc`, `android`), which reorders the list. Player picker (`Open with`) lists every
detected player and saves the chosen player as the next default unless
`MOVIEBOX_PLAYER` is set.

## Command construction

| Player          | Invocation                                                                                                                                | Notes                                                                                                  |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| mpv             | `mpv --autofit=WxH --geometry=50%:50% --idle=no --keep-open=no [--start=..] [--script=..] [--script-opts=..] [--http-header-fields=..] [--sub-file=..] <url>` | Window sized to the terminal. Injects `moviebox_tracker.lua` for position tracking and resume. Flatpak mpv is launched via `flatpak run`. |
| VLC             | `vlc --width=W --height=H --play-and-exit [--start-time=..] [--http-referrer=..] [--http-user-agent=..] [--sub-file=..] <url>`            | Supports start time resume via `--start-time`.                                                         |
| IINA            | `iina-cli --keep-running --no-stdin --mpv-autofit=.. [--mpv-start=..] --mpv-http-header-fields=.. --mpv-sub-files=.. <url>`               | Uses the bundled `iina-cli`; falls back to `open -a IINA <url>` only if the CLI is absent.             |
| Android / Proot | `termux-open --chooser --content-type video/* <url>` (or absolute `/system/bin/am start` fallback, ensuring `.so` injections are dropped) | Opens an app chooser on the device. Device-specific chooser behavior should be confirmed for each release. |

Window size is derived from the live terminal size times the font cell size reported
by the image picker, then clamped to a fixed range.

## Headers

Playback sources (for example 4KHD) may carry `Referer`/`User-Agent` headers.
mpv/IINA send them via `http-header-fields` (`--http-header-fields=...` or
`--mpv-http-header-fields=...`), while VLC maps them to `--http-referrer` /
`--http-user-agent`. The `supports_headers` gate in `app/playback.rs` warns when a
player cannot satisfy a source's headers.

## Subtitles

- mpv receives the remote subtitle URL directly (`--sub-file=<url>`); mpv fetches it
  with the stream headers applied.
- VLC and IINA download the subtitle to a temp file first, preserving the URL's
  extension (srt/vtt/ass/…), and pass the local path. The download applies the source
  headers. On failure a status is shown and playback continues without subtitles.
- Temp files are cleaned up after the player exits and purged at startup if stale.

## Playback Tracking & Resume

When launching media with in-progress watch history, the player command automatically
includes the starting position (`--start` / `--mpv-start` / `--start-time`). For `mpv`,
`src/player/tracker.rs` writes a companion tracker script (`moviebox_tracker.lua`) that
observes `time-pos` and `duration`, periodically saving playback state every 5 seconds to
the local data directory. On startup or after player exit, pending playback states are
reconciled into `history.json`.

## Spawning

`launch_player` spawns the player with null stdin/stdout, piped stderr, and its own
process group (Unix) or no-console flag (Windows), using `tokio::process` so the
watcher task can wait on it without blocking a thread. The watcher reads stderr
concurrently and reports a crash if the process exits non-zero within a few seconds
with output.

## Replacing an active session

Starting new playback while a player is already running does **not** require closing
the old one first: `launch_player` bumps a `playback_generation` counter and signals
the previous session's watcher (via a `oneshot` channel) to kill its player and hand
off. The old session's watcher still reconciles watch history and saves progress
before exiting, exactly as it would on a normal exit — only its `PlayerCrashed` /
`PlayerExited` report is tagged with the now-superseded generation, so `handle_playback`
ignores it instead of clobbering the flags for the session that's actually playing.

A dropped-without-sending stop channel (e.g. the app quitting while playback is
active) is deliberately **not** treated as a stop request, so playback keeps running
after the TUI exits, matching the existing behavior.
