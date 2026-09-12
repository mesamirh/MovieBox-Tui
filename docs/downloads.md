# Downloads

The download engine in `download.rs` streams a video URL to disk with resume, ranges,
and optional segmentation. Orchestration lives in `app/download.rs`.

## How it works

- Single-episode downloads write to `<dest>.part` plus a `<dest>.part.json` metadata
  sidecar (etag, last-modified, total size, segment count).
- **Resume**: on a retry, the engine checks what is already in the `.part` file and
  continues from there using `Range` requests.
- **Segmentation**: files above a size threshold can be downloaded in parallel
  segments (up to a capped count), then stitched.
- **I/O Aggregation**: Download segment writers are buffered with a 256KB `tokio::io::BufWriter`, aggregating incoming 8KB–16KB HTTP response chunks into sequential disk writes and reducing filesystem syscalls by up to 96.8%.
- **Retries**: a failed attempt is retried a limited number of times; 30s idle
- **Cancel**: an `AtomicBool` cancel flag pauses/resumes cleanly, preserving the
  partial file for a later resume.
- **User-Agent**: the download HTTP client inherits the active provider's mobile `User-Agent`
  to prevent CDN stream rejections when downloading media segments.
- **Stream Engines**:
  - **Progressive Streams** (CircleFTP, DhakaFlix, 4KHDHub, Addons): Handled directly by the native Rust multi-segment range downloader, splitting files into parallel chunks with `.part` state tracking.
  - **MPEG-DASH Streams** (MovieBox): Multi-track segmented audio/video streams (`index.mpd`) requiring CloudFront cookie authentication. Downloaded via `yt-dlp` with automatic authentication header forwarding (`Cookie`, `Referer`, `User-Agent`), real-time progress parsing, and track multiplexing into `.mp4`.

## External Tool Prerequisites

Downloading from **MovieBox** requires `yt-dlp` and `ffmpeg` on the host system to demux and merge MPEG-DASH audio and video streams:

- **macOS**: Install via Homebrew: `brew install yt-dlp ffmpeg`
- **Linux**: Install via your system package manager (e.g. `sudo apt install yt-dlp ffmpeg`, `sudo pacman -S yt-dlp ffmpeg`)
- **Windows**: Install via WinGet or Scoop: `winget install yt-dlp.yt-dlp Gyan.FFmpeg`
- **Android / Termux**: Install via Termux package manager: `pkg install yt-dlp ffmpeg`

If a MovieBox download is initiated without `yt-dlp` installed, MovieBox-TUI prevents execution and displays an OS-tailored notification with installation guidance. Progressive streams from other providers (CircleFTP, DhakaFlix, 4KHDHub, Addons) do not require `yt-dlp` or `ffmpeg`.
## File names and directories

`safe_file_stem` sanitizes titles for all platforms: control/whitespace/illegal
characters are replaced, Windows reserved names (`CON`, `COM1`-`COM9`, …) are avoided,
and length is capped.

- **Series downloads:** Saved under `<base_dir>/Series/<Title>/Season <N>/<Title> - S<N:02>E<E:02>.<ext>` (and subtitle `<Title> - S<N:02>E<E:02>.<lang>.<sub_ext>`).
- **Movie downloads:** Saved under `<base_dir>/Movies/<Title>/<Title>.<ext>` (and subtitle `<Title>.<lang>.<sub_ext>`).
- **Default path:** Files go to the user's OS download directory (`~/Downloads/MovieBox-TUI`). On Android-family environments the code prefers shared `storage/downloads` when present.
- **Custom path:** Users can set a custom download directory in `/settings` → General → Download Folder or reset to default. Target directories are validated with a write probe before saving, and the code creates the `MovieBox-TUI` subfolder hierarchy (`Movies/` and `Series/`). If custom storage becomes unavailable at runtime, the engine falls back to the default download location.

## Contextual triggers & Seasons

- **Contextual trigger**: Pressing `d` (or clicking `[Download]`) while focused on the **Seasons** pane prompts to download all episodes of the selected season. Triggering download while on the **Episodes** or **Streams** pane prompts to download only that single episode.
- **Duplication prevention**: When starting a download or processing a season batch queue, the engine checks if the target media file is already completed on disk. Existing completed episodes are skipped.
- A season download enqueues every episode (`download_queue`) and processes them one at a time, each resolving its stream and subtitle. Progress is reported through `Action::UpdateDownload` and the status bar; failures pause and preserve partial data.
- Season downloads ask for the subtitle policy once per batch. The selected language, including an explicit `None` choice, is reused for every queued episode.

Selected subtitle sidecars are saved next to the video using ISO 639-1 language codes (e.g. `.en.srt`, `.hi.srt`) and supported subtitle extensions (`.srt`, `.vtt`, `.ass`, `.ssa`, `.sub`).

## Download Bar & Progress Architecture

- **High-Contrast Responsive Bar**: Renders a proportional progress track (`[━━━━━────]` on modern terminals and `[=====>----]` on basic terminals) with prominent media title display (`⬇ Downloading: <Title>`), percentage badge, and separated transfer metrics (`<Size> | <Speed> | ETA <Time>`).
- **Monotonic DASH Normalization**: Multi-stream MPEG-DASH downloads via `yt-dlp` automatically normalize segmented tracks (video 0–90%, audio 90–98%, and track merger 99–100%) so that progress strictly increases and never resets backwards to 0% mid-download.
- **Throttled Updates**: Progress events are throttled to 250ms intervals, eliminating terminal flicker and event-channel flooding from high-frequency chunk streams.
- **Background Download Continuity**: Downloads continue running uninterrupted in the background when switching content providers (`Ctrl+P`) or toggling TV mode (`Ctrl+T`), keeping all active transfer workers and queues alive.
- **Isolated Cancellation Hitbox**: Cancellation is bound strictly to the `x` / `X` keyboard shortcut and the `[x] Cancel` button in the top-right corner of the download bar. Clicking anywhere else on the bar safely consumes the mouse event without interrupting active downloads.

## Outcomes

`DownloadCompleted` / `DownloadPaused` / `DownloadFailed` drive the UI status and
notifications. `ClearCache` and stale-file cleanup do not touch in-progress downloads.
