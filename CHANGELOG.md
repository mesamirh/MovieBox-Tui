# Changelog

## [Unreleased]

## [0.1.29] - 2026-11-05

### Fixed
- **Providers & Stream Resolution**:
  - Restored MovieBox TV series streaming by generating and attaching the official `X-Client-Build` APK certificate stamp with signed requests.
  - Preserved multi colon and namespaced media identifiers (such as Kitsu anime IDs) in Stremio addon stream aggregation.
  - Replaced hard assertions on HTTP client builders with safe fallbacks in MovieBox, CircleFTP, and DhakaFlix clients.
- **Networking & DNS**:
  - Added runtime fallback to public DNS (Cloudflare, Google, Quad9) in `FallbackResolver` when primary system DNS lookup fails or is blocked by an ISP.
- **Media Playback & Sidecar Proxy**:
  - Handled poisoned watchdog activity locks gracefully in the stream proxy sidecar loop to prevent unwrap crashes.
  - Standardized cross-platform temporary subtitle cleanup path joins to eliminate forward-slash path issues on Windows.
- **Test Infrastructure**:
  - Synchronized environment dependent player tests with `ENV_MUTEX` and probed Android intent openers dynamically to eliminate parallel test race conditions.

### Changed
- **Dependencies & Build Infrastructure**:
  - Upgraded `unicode-width` to 0.2.3, `log` to 0.4.35, and `zip` to 9.2.1 (#234).

## [0.1.28] - 2026-10-29

### Fixed
- **Providers & Stream Resolution**:
  - Extracted signed CDN MP4 streams from MovieBox subject detector links (`/subject-api/get`), restoring movie streaming and multi-resolution playback after upstream restricted play-info with deprecation clips.
  - Reported explicit error causes when MovieBox serves placeholder clips instead of generic "no streams available", and surfaced upstream HTTP failure statuses directly to the user.
- **Download Engine & Batch Queue**:
  - Recorded only flushed bytes to `.part.json` segment progress during downloads, preventing aborted or paused workers from leaving zero-byte gaps and file corruption on resume.

### Changed
- **Dependencies & Build Infrastructure**:
  - Upgraded `tokio` to 1.54.1, `scraper` to 0.29.0, `lru` to 0.19.0, `zip` to 9.1.0, `sha2` to 0.11.1, `open` to 5.4.5, `libc` to 0.2.191, and `serde_json` to 1.0.154.

## [0.1.27] - 2026-10-23

### Fixed
- **Media Playback & Subtitle Engine**:
  - Removed `--play-and-exit` from VLC player invocations so the player stays active across adaptive and HLS chunk transitions.
  - Reset legacy Android player defaults in graphical X11 and Wayland sessions so native Linux players take precedence over external Android intent dialogs.
  - Prioritized native Linux media player paths over foreign container fallbacks on Linux systems and preserved dynamic auto-player resolution for unconfigured sessions.
- **Providers & Stream Resolution**:
  - Recovered playable signed MP4 streams via `/resource-position/v2` when MovieBox returns deprecation notice clips, querying exact seasons and episodes and reporting accurate file sizes.
  - Stripped the DASH `Referer` header on direct MP4 streams to prevent CDN HTTP 429 throttling.
  - Bumped the MovieBox stream cache schema to `v2_` to discard stale cached deprecation notices.
  - Filtered out unhosted MovieBox `/hls/` signed cookie targets in manifest resolution, which stops unplayable H.264 stream entries from failing with HTTP 404 during downloads or media playback.
- **Download Engine & Batch Queue**:
  - Fixed season batch downloads by arming `is_waiting_for_download_stream` during queue steps, eliminating immediate empty-list download calls that aborted multi-episode queues.
  - Reset the download resolution state lock on 4KHDHub resolver timeouts and errors so failed mirror resolutions do not block subsequent download attempts.
  - Added sequential single-connection fallback to the internal HTTP downloader when multi-worker segmented range requests receive HTTP 403, 429, or network disconnects.
### Added
- **In-App Updater Lifecycle**:
  - Tracked target executable path before staging updates and resolved deleted inode suffixes (` (deleted)`) on Linux to restart the new binary automatically after popup self-update.
  - Scoped Snap package detection to `SNAP_NAME` matching `moviebox-tui` to prevent false positive Snap environment detection when launched inside unrelated host sandboxes.

### Changed
- **Dependencies & Build Infrastructure**:
  - Upgraded `zip` to 9.0.5, `rustls` to 0.23.46, `quinn-proto` to 0.11.19, `rustls-webpki` to 0.103.16, and `webpki-roots` to 1.0.10, resolving RustSec security advisories.
  - Upgraded `sha2` to 0.11 with chunked streaming hash updates, preserving non-blocking checksum verification for release archives.
  - Upgraded `dirs` to 7.0, `futures` to 0.3.34, and `zip` to 8.6 across platform config paths, async tasks, and updater archive extraction.

## [0.1.26] - 2026-10-01

### Fixed
- **Media Playback & Subtitle Engine**:
  - Preserved native Windows `\` path separators for `--sub-file` in VLC (`C:\...`) and passed `--no-one-instance` on Windows and Linux so running VLC windows keep subtitle, header, and resume flags.
  - Stripped query strings and fragments before parsing subtitle extensions (`.srt`, `.vtt`, `.ass`, `.ssa`, `.sub`) so signed CloudFront and OSS URLs keep their true format and DASH MIME type.
  - Pre-downloaded subtitles locally for `mpv` with remote URL fallback, parallelized subtitle downloads with `StreamRelay` sidecar startup, appended an 8-char URL hash to subtitle filenames, and sanitized commas in `--script-opts`.
  - Mounted `xdg-cache/moviebox-tui:ro`, `xdg-data/moviebox-tui`, and `/tmp:ro` in `flatpak run` while keeping custom user Flatpak flags so sandboxed `mpv` and `VLC` can read subtitles and write watch progress.
  - Served Termux private-storage subtitles over the local `StreamRelay` `/sub/` loopback endpoint when Android shared storage is unavailable, and restricted startup subtitle cleanup to internal cache and temp folders.
  - Parsed player `stderr` and exit codes to report specific failure reasons (`HTTP 403`, `HTTP 404`, `HTTP 410`, timeout, connection refused), and added deterministic cross-platform contract integration tests (`tests/player_contract.rs`) asserting VLC Win32/UNC paths, IINA CLI option schemas, and header support across OSes.
  - Unified IINA command construction with standard `--mpv-*` argument schemas across platforms, and restricted macOS IINA header disqualification to GUI app fallbacks missing command line tools.
  - Standardized player display labels to uppercase acronyms (MPV, IINA, VLC) across player selection popups, settings, and notifications.
- **StreamRelay Proxy & Downloader**:
  - Forwarded `User-Agent` and `Referer` while isolating stream cookies (`Cookie: Edge-Cache-Cookie=...`) to same-host requests across `StreamRelay`, subtitle pre-downloads, and media downloads.
  - Capped `.m4s` segment chunk fetching in `StreamRelay` to batches of 16 concurrent `Range` requests, registered in-flight segment wait futures under lock, and required `http://` or `https://` schemes on `/proxy?url=` routes.
  - Increased MovieBox DASH `yt-dlp` `--concurrent-fragments` from `8` to `32` (`3.77 MiB/s -> 11.13 MiB/s`, `+195%` live CDN throughput) and scaled the direct HTTP multi-segment engine to 4 to 16 workers with `Range: bytes=0-0` size discovery, 1 MiB write buffers, and a single end-of-file `sync_data` flush.
  - Reported specific subtitle and DASH download errors (HTTP status, timeout, missing `yt-dlp` vs `ffmpeg`) and matched `"at"` after `"of"` in `yt-dlp` progress lines.
- **Providers & Disk Cache**:
  - Removed the broad `"notice"` substring filter in MovieBox stream adaptation while keeping `/notice.mp4` and known deprecation hashes, which restores streams for titles like *Red Notice* and *Burn Notice*.
  - Advanced the active MovieBox host index immediately on connection or HTTP retry errors, removed the duplicate `/subject-api/resource` preflight round-trip in `InitStreamPool`, and removed the 120ms delay on cached episode switches.
  - Streamed 4KHDHub mirror resolution directly into preflight probes via `FuturesUnordered` without waiting for slow mirrors, cached fetched detail pages across `details` and `releases` in 4KHDHub, Dramachi, and CircleFTP, and bounded CircleFTP `HEAD` size probes to 1.2 seconds.
  - Required a leading `'S'` prefix in 4KHDHub season count parsing so release years after `"Score"` do not parse as season counts, and cached static HTML selectors in 4KHDHub parsers.
  - Matched Dramachi episode numbers across Unicode boundaries and quality suffixes, parallelized Stremio addon movie/series catalog searches and M3U playlist loads, and inserted `/manifest.json` before query parameters in addon URLs.
  - Evicted cached poster images only when file age exceeds the 30-day TTL so valid cache files survive minor clock skew.
  - Replaced synchronous `fsync` barriers on ephemeral cache writes with atomic temp-file renames, cached default directory paths via `LazyLock`, reused a static HTTP client in `probe_url`, and shared `Arc<[u8]>` poster buffers across disk caching and decoding.
  - Centralized Termux platform detection in core configuration and deduplicated base64 manifest policy parsing.
- **TUI Navigation, Themes, Modals & Mouse**:
  - Validated `Details` and stream responses against each item's own provider so cross-provider items opened from Continue Watching, Favorites, `/history`, or `/favorites` load without hanging.
  - Aborted in-flight caption and 4KHDHub mirror resolution tasks on `Esc` or `GoBack` so stale subtitle popups or players do not launch after leaving `Details`.
  - Processed active Picker and Subtitle popups before the background loading guard on `Enter`, cleared queued season downloads when cancelling the download subtitle prompt or on stream failure, and reset TV input focus on overlay dismissal.
  - Enabled `/browse` for Stremio Addons, reused in-memory addon state during stream resolution, skipped search pagination on active addon catalogs, and clamped initial cursor selection in empty TV and Addon manager modals.
  - Synchronized `Home` and `Details` mouse hitboxes with the 3-row download gauge, narrow `< 85` col selector panes, compact `< 76` col bottom bar buttons, non-Stream footer `[Enter] Select` actions, and outside-click theme preview revert.
  - Downgraded named themes in `Theme::from_kind` on 16-color (`Basic`) and `NO_COLOR` terminals, improved `Latte` light-mode contrast on selected badges and modal rows, and added `ALACRITTY_SOCKET`, `KONSOLE_VERSION`, `TERMUX_VERSION`, `WT_SESSION`, and `TERM_PROGRAM` graphics/TrueColor detection.
  - Shortened toast notifications to show filenames instead of full paths and removed duplicate status-bar and toast alerts on compact terminals.
  - Pruned conversational filler from terminal size warnings, language selection prompts, and package manager upgrade notices.

## [0.1.25] - 2026-09-27

### Fixed
- **Search Navigation & Pagination Engine**:
  - Gated background search pagination in `src/tui/app/navigation.rs` so arrow navigation on result sets smaller than the visible viewport never triggers background page fetches.
  - Added `search_exhausted` tracking in `src/tui/state.rs` and `src/tui/app/requests.rs`, stopping pagination crawls when provider search returns fewer than 15 items or zero query-matching results.
  - Preserved stable card slot positions during search pagination in `src/tui/app/requests.rs` by appending subsequent page items to the list rather than re-sorting the entire array out from under the cursor.
  - Removed redundant "Provider: {name}" status message override from the search bar on provider switch in `src/tui/app/navigation.rs`, preserving the clean search placeholder while the active provider pill displays on the right.
- **Media Playback & Buffering Engine**:
  - Prioritized native desktop media players (`mpv`, `VLC`) over Android intent openers in `src/player.rs` when an active X11/Wayland display server (`$DISPLAY`, `$WAYLAND_DISPLAY`) is detected, preventing Ubuntu Xfce (udroid/PRoot) and Termux:X11 desktop sessions from dispatching playback to external Android app choosers.
  - Routed DASH `.mpd` streams through the local `StreamRelay` proxy (`src/proxy.rs`, `src/tui/app/playback.rs`) across all media players (`mpv`, `IINA`, `VLC`, `AndroidIntent`), caching rewritten `.mpd` manifests in RAM, warm-prefetching initialization and opening segments on sidecar startup, fetching `.m4s` segments via concurrent `95 KB` HTTP `Range` sub-requests (`0.286s` per segment vs `3.120s`), and prefetching both the next 3 video segments (`N+1..N+3`) and synchronized audio segments (`chunk-stream3-N..N+1`).
  - Tuned `mpv` and `IINA` playback flags (`--hwdec=auto-safe`, `--stream-buffer-size=4M`, `--cache-secs=120`, `--cache-pause-wait=3`, `--cache-pause-initial=no`, and `--ytdl=no` on non-DASH direct streams) in `src/player.rs`, reducing 1080p cold-seek latency at `01:00:00` from `11.96s` to `3.87s` and boosting read-ahead buffer fill rate from `1 MB` (`9.3s`) to `7 MB` (`53s`).
  - Replaced aggressive VLC buffering flags (`--network-caching=10000`, `--adaptive-logic=nearoptimal`) with `--network-caching=3000`, `--file-caching=3000`, `--http-reconnect`, and `--adaptive-logic=predictive` in `src/player.rs` to stop demuxer stream restarts and reconnect cleanly on dropped HTTP streams.
  - Enforced `max_height` resolution limits in the loopback stream proxy manifest rewriter (`src/proxy.rs`, `src/main.rs`, `src/tui/app/playback.rs`), pruning representations exceeding the chosen ceiling so Android players and VLC respect 480p and 720p selections.
  - Parsed signed expiration timestamps (`:t=`) from MovieBox `Edge-Cache-Cookie` headers in `src/cache.rs` alongside CloudFront policies, automatically invalidating stale stream cache entries within 60 seconds of expiration to prevent HTTP 403 playback crashes.
  - Persisted selected stream filenames (`stream_filename`) in watch history (`src/history.rs`, `src/player/tracker.rs`, `src/tui/app/playback.rs`) and auto-focused matching stream rows on load (`src/tui/app/requests.rs`, `src/tui/app/system.rs`), restoring the user's previously chosen quality mirror when resuming multi-stream releases.
  - Enabled TCP nodelay (`stream.set_nodelay(true)`) and per-chunk immediate flushes on the local `StreamRelay` proxy in `src/proxy.rs`, eliminating 100–300ms time-to-first-frame buffering delays on loopback sockets.
  - Implemented negative caching (`Option<Option<String>>`) for media player detection (`MPV_CACHED`, `VLC_CACHED`, `IINA_CACHED`) in `src/player.rs`, eliminating redundant filesystem scans and registry lookups when players are absent.
  - Throttled Lua tracker periodic disk sync in `src/player/tracker.rs` when playback is paused or positional change is sub-second, cutting SSD writes by up to 90% during playback pauses.
  - Removed duplicate redundant `history.save()` calls in `src/tui/app/playback.rs` upon marking items watched.
  - Defined `local now = os.time()` in `src/player/tracker.rs` before formatting playback state JSON in `moviebox_tracker.lua` and scoped initial `PendingPlaybackState` seed writes in `src/tui/app/playback.rs` strictly to `PlayerKind::Mpv`, preventing `VLC`, `IINA`, and `AndroidIntent` watch progress from resetting to `0` on player exit.
  - Detached spawned media players and the `StreamRelay` sidecar into independent OS sessions via `configure_detached_process` (`libc::setsid()` + `SIGHUP` ignore on Unix, `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` on Windows) in `src/player.rs`, `src/proxy.rs`, and `src/tui/app/playback.rs`, redirected player `stderr` to temporary log files instead of parent pipes to prevent `SIGPIPE` crashes when the terminal closes, and reaped the sidecar immediately when desktop players exit while the TUI is open.
- **Download Engine & Subprocess Diagnostics**:
  - Added preflight verification for `ffmpeg` alongside `yt-dlp` in `src/tui/app/download.rs`, failing fast with installation guidance when media muxing tools are missing.
  - Captured child process stderr lines during DASH transfers to surface concrete `yt-dlp` error diagnostics instead of bare exit codes.
  - Accelerated MovieBox DASH stream downloads from ~0.92 MiB/s to ~10.29 MiB/s by passing `--http-chunk-size 95K` and `--concurrent-fragments 8` to `yt-dlp` with resilient retry policies (`--fragment-retries 10`, `--retries 5`, `--socket-timeout 30`) in `src/tui/app/download.rs`, resetting Alibaba Cloud Tengine's per-request `limit_rate_after 96k` rate-limiter across persistent Keep-Alive sockets.
  - Refactored the direct HTTP multi-segment downloader in `src/download.rs` to pre-allocate the `.part` file and write segments in-place via positional `SeekFrom::Start` offsets (scaling concurrency to 4–12 workers down to 16 MB files), eliminating temporary `.part.N` files and the post-download `.assembling` full-file disk copy.
- **TUI Rendering & Event Loop Performance**:
  - Expanded terminal standard output `BufWriter` capacity from 64 KB to 256 KB in `src/main.rs`, enabling atomic frame flushes and eliminating visual tearing during graphical Sixel and Kitty poster transfers.
  - Replaced allocating `to_string()` hash set checks in `src/favorites.rs` with zero-allocation borrowed string comparisons in `is_favorite`, eliminating 400+ heap allocations per second during search grid rendering.
  - Eliminated redundant 100ms idle tick redraws on static home screens in `src/tui/app/system.rs`, dropping idle CPU consumption to zero.
  - Replaced per-card `Layout::split` invocations in `src/tui/screens/home.rs` with direct row arithmetic calculations, avoiding repeated layout solver overhead across search result slots.
  - Rendered stream media tags (HDR, ATMOS, 5.1, HEVC, AV1, WEB-DL) in `src/tui/screens/details.rs` using styled surface pill spans and unified background modal dimming to `theme.muted` across `render_media_tag_spans`, `resolution_badge_spans` (`src/tui/widgets/badge.rs`), `render_poster_placeholder` (`src/tui/widgets/poster.rs`), Details headers (`src/tui/screens/details.rs`), and Home search result bars, cards, ratings, and history progress rows (`src/tui/screens/home.rs`).
  - Replaced linear index iteration with exact O(1) `HashSet` lookups in `FavoritesManager::is_favorite` (`src/favorites.rs`).
  - Guarded `PopKeyboardEnhancementFlags` in `restore_terminal` (`src/main.rs`) against Termux environments (`!is_termux_environment()`), preventing stray ANSI escape sequences from leaking into the Android Termux shell on exit.
  - Routed `Warning:` and non-Home `Action::SetStatus` messages through toast notifications in `src/tui/app/system.rs`, `src/tui/app/playback.rs`, and `src/tui/app/requests.rs`, ensuring subtitle fallback notices, missing `iina-cli` warnings, and addon installation statuses remain visible on the Details screen and modal overlays.
  - Unified notification card geometry and line-wrapping calculations in `compute_notification_cards` (`src/tui/overlay.rs`), eliminating duplicate layout passes between frame rendering and mouse click hit-testing.
- **Provider Scraping & Concurrency Engine**:
  - Replaced chunked mirror resolution barriers in 4KHDHub (`src/providers/fourkhdhub/client.rs`, `src/providers/fourkhdhub/hubcloud.rs`) with `HTTP 206 Partial Content` seekability verification (`preflight_with_seekable`) and score-prioritized early termination (`is_seekable && score <= 1`), ensuring byte-range-seekable 10Gbps/R2/FSL mirrors are selected over unseekable `HTTP 200` mirrors or quota-exhausted `workers.dev` links.
  - Centralized disk-cached caption lookup and storage in `MovieBoxService::get_ext_captions` (`src/service.rs`) and parallelized primary and sibling dub caption fetches via `tokio::join!`, eliminating duplicate cache wrappers in `src/tui/app/playback.rs` and `src/tui/app/requests.rs`.
  - Bound individual Stremio addon stream fetches in `src/providers/addons/aggregator.rs` with a 5-second timeout, preventing unresponsive third-party addons from blocking stream aggregation.
  - Capped initial search poster scraping in DhakaFlix (`src/providers/bdix/dhakaflix/client.rs`) to the first 20 items, eliminating dozens of blocking HTTP POST requests on large catalog searches.
  - Centralized `MediaDetails::sibling_ids`, human-readable file size parsing (`parse_size_bytes`), DASH URL detection (`is_dash_url`), `yt-dlp` format selectors (`ytdlp_format_selector`), download target path resolution (`resolve_media_target`), and cached poster fetching (`MovieBoxService::fetch_and_decode_cached_poster`) in their owning domain modules (`src/providers/models.rs`, `src/player.rs`, `src/download.rs`, `src/service.rs`), and removed `src/tui/app/network.rs`.
  - Streamed poster HTTP responses via bounded chunk accumulation (`5 MiB` cap) in `MovieBoxService::fetch_poster_bytes` (`src/service.rs`) and added `FallbackResolver` DNS + `Range: bytes=0-0` bounds to `probe_url` (`src/net.rs`).
- **Storage & Disk Cache Engine**:
  - Optimized typed cache serialization in `src/cache.rs` using direct in-place encoding (`rmp_serde::encode::write`) into pre-seeded magic header buffers, eliminating intermediate vector allocations and duplicate memory copies.
  - Pre-allocated file buffer capacity from file metadata in `get_typed_cache` in `src/cache.rs`, eliminating repetitive vector reallocations during cache reads.
  - Offloaded `atomic_write_file_async` in `src/cache.rs` to `tokio::task::spawn_blocking`, switched `src/logging.rs` to `WriteMode::BufferAndFlush` with a `LazyLock` cached home directory path, centralized corrupt JSON file rotation (`rotate_corrupt_file`) in `src/config.rs`, and enabled cross-platform stale update artifact cleanup in `src/updater/apply.rs`.
  - Isolated all unit and integration test runs into a per-process temporary sandbox (`TEST_SANDBOX_DIR` in `src/config.rs`) and guarded external browser launches (`open_external_url` in `src/net.rs`), preventing tests from modifying real user configuration, watch history, favorites, or disk cache files.
  - Added unit and mock-server test coverage for `4KHDHub` HTTP 206 seekable mirror prioritization, `CircleFTP` catalog mapping, `DhakaFlix` stream extraction, `Addons` concurrent stream aggregation and torrent filtering, `TV` UTF-8 BOM / HLS M3U parsing, `IINA` command construction, `StreamRelay` DASH sidecar warmup, non-`mpv` watch progress retention, and `Action::SetStatus` warning/modal toast routing.
### Changed
- **TUI & Ergonomics**:
  - Remapped `q` / `Q` on `Screen::Details` in `src/tui/app/keyboard.rs` from application exit to `Action::GoBack`, preserving active search results and returning to the home screen without unexpected process termination.
  - Standardized modal and popup dismissal across Settings, Providers, Browse Presets, Sources, Themes, Addons, and TV Config overlays to dismiss on both `Esc` and `q`.
  - Reordered episode list label formatting in `src/tui/screens/details.rs` to compute prefix and progress badge widths before truncating episode titles, strictly preventing list item text overflow and jagged multi-line wrapping.
  - Replaced ambiguous `" ... [i]"` plot synopsis truncation with an explicit `" ... [i: More]"` indicator in `src/tui/screens/details.rs`.
  - Optimized stream table column constraints for ultra-compact terminals (<65 columns) in `src/tui/screens/details.rs`, compacting `SIZE` to 7 characters and reclaiming columns for release title readability on mobile viewports.
  - Rendered cached IMDb rating badges across all visible search cards in `src/tui/screens/home.rs` by peeking `preview_cache`, enabling at-a-glance quality comparison across the grid without manual card-by-card selection.
  - Collapsed empty `[No Art]` poster placeholder columns when `image_supported` is false in `src/tui/screens/home.rs`, expanding search card text and metadata across the full available column width on non-graphical terminals.
- **Documentation & Cross-Platform Terminal Compatibility**:
  - Documented native Windows Terminal Sixel image support requirements (`v1.22+` via Microsoft Store) in `docs/cross-platform.md` and `README.md`, clarifying that no third-party terminal or configuration is required on Windows.
## [0.1.24] - 2026-09-24

### Added
- **Scoop Package Manager Distribution**:
  - Added native Scoop manifest in `bucket/moviebox-tui.json` supporting Windows x64 and arm64 architectures with automated hash verification.
  - Automated Scoop manifest synchronization alongside Homebrew in `.github/workflows/manifests.yml` from release `SHA256SUMS`.
  - Added `InstallationEnvironment::Scoop` detection in `src/updater/apply.rs` with update prompt shortcuts (`[s]`).
- **Adaptive Stream Resolution Picker**:
  - Decomposed multi-bitrate DASH streams in `src/providers/moviebox/adapt.rs` into selectable quality rows (`1080p`, `720p`, `480p`).
  - Threaded quality constraints to player builders, injecting `--ytdl-format` for `mpv`/`IINA` and `--adaptive-maxheight` for `VLC`.
- **Custom Player Executable Path Configuration**:
  - Added `vlc_path`, `mpv_path`, and `iina_path` fields to `Config` in `src/config.rs` and `src/tui/state.rs`.
  - Supported persisted custom player paths with automatic static path cache invalidation upon configuration updates.


### Changed
- **Streamlined Download Failure Notifications**:
  - Replaced verbose, truncated DASH download failure guidance with terse, actionable commands (`Run: winget install yt-dlp.yt-dlp Gyan.FFmpeg`).
  - Removed misleading "Partial file preserved" prefix on pre-download failures in `src/tui/app/download.rs`.
  - Scaled notification toast card width to the widest line across multi-line messages in `src/tui/overlay.rs`.
  - Threaded selected stream resolution constraint (`bestvideo[height<={h}]...`) to `yt-dlp` in `src/tui/app/download.rs`, ensuring DASH downloads fetch the highlighted quality rather than forcing 1080p.
### Fixed
- **Streaming Proxy & Sidecar Lifecycle**:
  - Enforced fail-closed target host verification in `src/proxy.rs`, rejecting requests with `403 Forbidden` if `target_host` is missing or unverified.
  - Detached proxy sidecar child handles via `std::mem::forget`, preventing defunct zombie process leaks on Unix.
  - Removed 60s request deadline on upstream proxy client and added 128KB `tokio::io::BufWriter` for sustained chunk throughput.
  - Deferred status line emission until manifest sizing is verified, returning `502 Bad Gateway` on oversized payloads.
- **Download Engine & Subprocess Safety**:
  - Re-canonicalized created download directories within background tasks in `src/tui/app/download.rs`, verifying real paths against download roots to prevent symlink traversal.
  - Sanitized provider HTTP header keys and values passed to yt-dlp, stripping CRLF and control bytes to prevent header splitting.
  - Removed global 60s transfer timeout and buffered single-connection file writes with 256KB `tokio::io::BufWriter`.
  - Added fast-fail fallback to single-stream downloads when segment workers receive `200 OK` responses.
  - Sanitized download failure notices via `DownloadError::user_message()`, replacing raw error dumps with clean user guidance.
- **Media Playback & Progress Tracking**:
  - Configured high-throughput stream buffering in `src/player.rs` for `mpv` and `IINA` (8s cushion, 120s readahead, 256MB demuxer cache) to eliminate rebuffering loops.
  - Set 10s network cache floor and `nearoptimal` adaptive logic for `VLC`, preventing playback stalls on DASH jitter.
  - Recorded elapsed watch progress for VLC code 1 exits and routed IINA through wall-clock progress tracking.
- **TUI Layout & Modal Ergonomics**:
  - Deduplicated movie title repetition in Details screen streams table (`src/tui/screens/details.rs`), displaying source/CDN instead of identical media titles across rows.
  - Streamlined download progress dock in `src/tui/app/run.rs` to a sleek, minimal header showing queue position and metrics without redundant title repetition.
  - Isolated TV mode search dispatch in `src/tui/app/requests.rs` and balanced landing search input width to 48 columns.
  - Auto-sized `/settings` modal height dynamically to category row counts and balanced modal width to 60 columns.
  - Dimmed background deck items, suggestions, and provider pills when modal popups are active in `src/tui/screens/home.rs`.
  - Reorganized Details screen footer action hierarchy, lowered split threshold to 80 columns, and synchronized mouse hitboxes.
  - Factored `ListState::offset()` into Details pane row hitboxes for scrolled season and episode lists.
  - Unified streams table highlight symbol under modals in `src/tui/screens/details.rs`, preventing table header and column horizontal shifts.
  - Reset `show_overview_modal` on mode transitions in `src/tui/app/tv.rs` and top-aligned metadata paragraphs in `src/tui/screens/details.rs`.
  - Streamlined TV playlist management popup to match Addons mode design with dynamic sizing and clean status indicators.
  - Replaced `Action::ToggleTvMode` with idempotent `Action::SwitchToTvMode` and protected `Ctrl+T`/`Ctrl+S` against unintended flips.
- **Network, Upstream & Concurrency**:
  - Bound parallel sibling provider queries in `get_ext_captions` (`src/service.rs`) with individual 8s timeouts to prevent stalls.
  - Tracked in-pool cached stream tasks in `request_tasks.streams` in `src/tui/app/requests.rs`, ensuring active tasks cancel on navigation.
  - Removed 30s deadline and added 3-attempt exponential backoff chunk retry for release binary downloads in `src/updater/download.rs`.
  - Added 5 MiB ceiling to `fetch_poster_bytes` in `src/service.rs` and referenced dynamic `CARGO_PKG_VERSION` in `APP_HTTP_USER_AGENT`.

### Changed
- **Direct Download Triggering**:
  - Streamlined download initiation by starting stream and season downloads immediately upon pressing `d` on the details screen.
  - Pruned intermediate confirmation dialogs, transient state flags (`show_episode_download_confirm`), and confirmation render helpers.
- **Deep Module Architecture & Task Encapsulation**:
  - Encapsulated async task lifecycle in `RequestTaskHandles` (`src/tui/app/mod.rs`), consolidating cancellation, task spawning, and abort handle tracking behind dedicated dispatch methods.
  - Encapsulated download destination directory creation, UNC normalization, and sandbox boundary checks into `prepare_target_dir` in `src/tui/app/download.rs`.
  - Added `render_modal_frame` in `src/tui/overlay.rs` combining area clearing with `Clear`, inner rectangle calculation, and modal frame border rendering.
- **Terminal Image Picker & Opener Caching**:
  - Consolidated fallback and custom cell size image picker creation in `src/tui/app/run.rs` through `create_picker_with_font_size`.
  - Cached Android opener detection via `std::sync::LazyLock` in `src/player.rs`.
- **CI/CD Quality Gates & Tooling**:
  - Added Linux musl target compilation verification in `.github/workflows/ci.yml`.
  - Migrated CI test matrix to parallel, process-isolated `cargo-nextest` execution.
- **Documentation Overhaul**:
  - Streamlined all user and technical guides in `docs/` (`players.md`, `controls.md`, `tv-mode.md`, `config.md`, `addons-mode.md`, `providers.md`, `cross-platform.md`, `cache.md`, `downloads.md`), cutting redundant tables and dense text walls.
  - Documented environment variables (`MOVIEBOX_LOG`, `MOVIEBOX_IMAGE_PROTOCOL`, `MOVIEBOX_CELL_SIZE`, `NO_COLOR`).
## [0.1.23] - 2026-09-21

### Fixed
- **MPV Player Header & Cookie Flag Construction**:
  - Replaced comma-joined `--ytdl-raw-options` header strings in `src/player.rs` with dedicated `--ytdl-raw-options-append` arguments for non-UA/referer headers (e.g. CloudFront `Cookie`).
  - Preserved native `--user-agent` and `--referrer` flags, eliminating `mpv` option parsing aborts (code 1) from commas inside browser User-Agent strings while ensuring DASH manifest requests receive authentication cookies (preventing code 2 playback errors).
- **Details Header Synopsis Click Hitbox**:
  - Constrained the overview modal mouse trigger to the actual rendered synopsis text lines (`synopsis_area`) rather than the entire top header box, preventing accidental popup triggers when clicking posters, title text, metadata badges, or empty border padding.
- **Details Screen Modal Backdrop Dimming**:
  - Dimmed empty stream placeholder and status messages (e.g. *"Choose an audio track to load streams."*) to `theme.muted` when any modal or synopsis overlay is open, matching the dimmed backdrop styling of the rest of the details screen.

- **Concise Actionable Playback Error Notifications**:
  - Mapped player exit codes to short, direct notices in `src/tui/app/playback.rs` (`Stream Dead: Link expired or unreachable.` for code 2, `Player Error: Check player configuration.` for code 1), replacing cryptic raw process exit strings (`Player exited (code 2).`).
## [0.1.22] - 2026-09-21
### Added
- **Dramachi Native Streaming Provider**:
  - Integrated Dramachi as a native streaming provider for Asian dramas, K-dramas, C-dramas, anime, and movies via `https://api.nodeobjects.com/`.
  - Added direct HTTP byte-range video streaming support (`direct_file: true`) bypassing CloudFront proxies and authentication overhead.
  - Added multi-dub and language rip resolution supporting original, English, and localized dubs with composite subject ID routing (`{title_id}::{rip}`).
  - Added `ProviderKind::Dramachi` support across `MovieBoxService`, TUI search, details, stream resolution, downloads, settings, and badges (`[Dramachi]`).
### Fixed
- **Human-Readable Subtitle Filenames & Intent Storage**:
  - Formatted subtitle filenames with media titles and season/episode tags (e.g. `<Title> - S<N:02>E<E:02>.<ext>`) in `src/service.rs` and `src/tui/app/playback.rs`, replacing random process/timestamp IDs for easy identification in external player file pickers (`moviebox_subs/`).
- **Concise Stream Failure Messages**:
  - Simplified verbose stream failure messages (`Provider is temporarily unavailable: No stream sources available`) across the details screen (`src/tui/screens/details.rs`) and notification bar (`src/tui/app/requests.rs`) to concise, direct notices (`No streams available on <Provider>.`).
- **Stream Proxy Subtitle Whitelist & Scope Isolation**:
  - Permitted external subtitle CDN hosts in `src/proxy.rs` proxy connection validation, preventing HTTP 403 errors when media players fetch external subtitle tracks alongside DASH stream manifests.
  - Scoped authentication headers exclusively to target video hosts, preventing credential spillage to subtitle CDNs.
- **Stremio Addon Mode Series Metadata Resolution & Episode Picker**:
  - Encoded media type hints into Addon catalog and search results (`series:{id}` and `movie:{id}`) in `src/providers/addons/adapter.rs`, preserving search intent across preview prefetching and details resolution.
  - Hardened metadata probing in `AddonClient::details` (`src/providers/addons/mod.rs`) to prioritize `series` queries and check for episode videos (`!videos.is_empty()`) before falling back to `movie`, resolving an issue where series (e.g. Breaking Bad) fell back to corrupted movie metadata ("Mirror") lacking episode lists.
  - Normalized subject IDs in `src/providers/addons/aggregator.rs` before constructing stream query IDs (`{clean_id}:{season}:{episode}` for series, `{clean_id}` for movies), including support for Season 0 (Specials).
  - Fixed Season 0 episode count and index calculation in `season_confirm_summary` (`src/tui/screens/details.rs`).
  - Replaced verbose stream failure and missing addon messages with concise status notices in `src/tui/app/requests.rs`.
  - Aligned Addon Mode empty search results with Streaming Mode by returning `Ok(vec![])` on zero-result queries instead of triggering a boxed `Search Request Error`, restoring the standard no-results view with `[ Try on MovieBox (^P) ]` and `[ Clear Search (c) ]` pills.
  - Returned `Ok(vec![])` on empty addon catalogs in `MovieBoxService::fetch_addon_catalog` (`src/service.rs`), preventing empty catalog views from raising false search failure errors.
  - Reset `active_screen` to `Screen::Home` in `reset_mode_state` (`src/tui/app/tv.rs`), preventing orphaned Details screen states when switching between TV and Streaming modes.
- **Logging Engine, Crash Diagnostics & Backtrace Capture**:
  - Added stack backtrace capture (`std::backtrace::Backtrace::capture()`) and immediate log buffer flushing (`moviebox_tui::logging::flush()`) to `std::panic::set_hook` in `src/main.rs`, ensuring crash locations and backtraces are committed to disk before terminal restoration and process termination.
  - Standardized default file log level to `info` across all builds in `src/logging.rs`, ensuring session startup metadata, player lifecycle commands, and exit durations are consistently captured without requiring manual `MOVIEBOX_LOG` configuration.
  - Retained `LoggerHandle` globally in `src/logging.rs` with graceful fallback to default logging levels if `MOVIEBOX_LOG` contains an invalid level specification.
  - Re-routed Windows logs directory (`crate::config::logs_dir()`) to Local AppData (`%LOCALAPPDATA%\moviebox-tui\logs`) to align with documentation and avoid roaming enterprise profile sync issues.
  - Hardened `sanitize_url` in `src/logging.rs` to redact embedded user/password credentials (`user:pass@host`), preserve original URI schemes, and retain port numbers for localhost and custom IPTV ports.
  - Sanitized and bounded player crash error messages (`clean_player_error`) in `src/tui/app/playback.rs`, preventing unbounded stderr streams and CDN auth tokens from leaking into error logs.
  - Added desktop player launch banners and clean exit logs with duration tracking to `src/tui/app/playback.rs`.
  - Added HTTP status code validation (`error_for_status()`) in `DramachiClient` and `CircleFtpClient`, preventing HTTP 403/502/Cloudflare errors from masquerading as misleading JSON parse failures.
  - Added diagnostic logging for system DNS fallback to public resolvers and probe failure statuses in `src/net.rs`.
  - Added warning logs for disk write and serialization failures in `src/cache.rs::set_typed_cache`.
- **BDIX DhakaFlix Configuration Persistence**:
  - Loaded `bdix_dhakaflix_enabled` from saved configuration in `App::new` (`src/tui/app/mod.rs`), preventing DhakaFlix provider toggle from resetting to disabled upon restarting the application.
- **M3U Playlist Parser Async Cache Probing**:
  - Replaced blocking synchronous `file_path.exists()` check in `M3UParser::fetch_playlist` with non-blocking `tokio::fs::try_exists` in `src/providers/tv/parser.rs`, eliminating async event loop stalls during remote playlist caching.
- **HTTP Client Builder Timeout Floors**:
  - Enforced baseline connection (`15s`) and request (`60s`) timeout floors in `http_client_builder` (`src/net.rs`) to prevent stalled HTTP connections across external providers without explicit timeouts.
- **Pruned Dead BDIX Release Stubs**:
  - Removed unused `resolve_release` identity functions from `CircleFtpClient` and `DhakaFlixClient` (`src/providers/bdix/`).
- **Hardened Issue Templates & Automated Triage**:
  - Added `Android (Termux)` option, mandatory pre-flight checklist, and sanitized terminal log output requirement to `.github/ISSUE_TEMPLATE/bug_report.yml`.
  - Added direct links to GitHub Discussions and Termux setup documentation in `.github/ISSUE_TEMPLATE/config.yml`.
  - Added automated `incomplete-issue.yml` workflow to flag and comment on vague issue reports lacking diagnostic context.
  - Added automated `stale.yml` workflow to close unresponsive `needs-info` issues after 7 days.
- **Security & Terminal Signal Hardening**:
  - Enforced host authority whitelist validation against `target_host` in local proxy sidecar `handle_connection` (`src/proxy.rs`), returning HTTP 403 Forbidden on host mismatch to prevent Server-Side Request Forgery and unauthorized auth header leakage.
  - Enforced HTTP and HTTPS scheme validation on percent-decoded subtitle URLs in proxy `extract_target_url` (`src/proxy.rs`), preventing arbitrary URL scheme forwarding.
  - Registered Unix `SIGTERM` and `SIGHUP` signal handlers in `EventHandler::new` (`src/tui/event.rs`) routing to `Action::Quit`, ensuring proper terminal de-initialization and alternate screen restoration via `TerminalGuard` on process termination.
  - Replaced silent `reqwest::Client::new` fallbacks with explicit builder `expect` assertions in `MovieBoxClient`, `DhakaFlixClient`, and `CircleFtpClient`, preventing silent networking degradation without shared DNS and connection pooling configurations.
  - Added canonical path containment checks in `start_resilient_download` (`src/tui/app/download.rs`) ensuring download target paths resolve within the configured download directory tree.
- **Cross-Platform Compatibility & Player Integration**:
  - Stripped Windows extended-length verbatim prefix (`\\?\`) in `start_resilient_download` (`src/tui/app/download.rs`), preventing false-positive download containment failures when comparing canonicalized base directories with un-canonicalized target paths.
  - Added environment variable expansion for `REG_EXPAND_SZ` values in Windows registry queries (`src/player.rs`), allowing automatic discovery of player binaries configured with `%USERPROFILE%`, `%SystemRoot%`, or `%LOCALAPPDATA%` paths.
  - Retained downloaded local subtitle files for Android intent launches in `src/tui/app/playback.rs` without premature process-exit deletion, avoiding loopback proxy URLs (`http://127.0.0.1:<port>/sub/...`) and allowing external players (VLC, MX Player) to load subtitles from shared storage before background cleanup.
  - Gated Kitty keyboard enhancement protocol flags (`PushKeyboardEnhancementFlags`) behind `!is_termux_environment()` in `src/main.rs`, preventing unsupported escape sequence noise on mobile touch terminals.
  - Restricted Termux shared storage subtitle directory probing (`~/storage/downloads/moviebox_subs`) to verified Termux environments in `src/main.rs` and `src/cache.rs`.
  - Added Flatpak `@@` file forwarding markers for local file and `file://` URLs in `mpv_command` and `vlc_command` (`src/player.rs`), ensuring Flatpak Document portal exports media and subtitle files into sandboxed player containers.
- **Audio Track Switch Metadata & Synopsis Preservation**:
  - Preserved rich metadata (duration, synopsis, genres, cast, crew, and clean title) when switching audio dubs on the Details screen in `src/tui/app/requests.rs`.
  - Resolved an issue where selecting an audio track (e.g. Hindi dub) caused duration (`2h 20m`) to disappear and replaced the multi-line synopsis with a title placeholder (`Ek Deewane Ki Deewaniyat`), breaking visual balance against the poster.
  - Sanitized window title in `contextual_title` (`src/tui/app/run.rs`) using `clean_moviebox_title` to prevent raw dub tags (`[Hindi]`) from leaking into the terminal window title.
- **MovieBox Edge-Cache CDN Stream Manifest Resolution**:
  - Added `Edge-Cache-Cookie` `urlprefix` Base64 decoding in `resolve_dash_manifest_from_policy` (`src/providers/moviebox/adapt.rs`).
  - Resolves active multi-quality MPEG-DASH manifests (`https://sbcdn*.hakunaymatata.com/dash/.../index.mpd`) generated under MovieBox's updated CDN token structure.
  - Fixes playback and stream resolution failure (`No stream sources available`) across movies and episodic series where previous parser only checked for `CloudFront-Policy`.
- **Settings Sub-Popups & Picker Dialog Anchoring**:
  - Unified sub-popup positioning (`Streaming Sources`, `Default Media Player`, `Theme`, `Browse`) to anchor directly inside Settings & Preferences (`settings_picker_layout` in `src/tui/overlay.rs`) rather than floating into empty screen space on tall terminal windows.
  - Stabilized Settings & Preferences modal height across all category tabs (`General`, `Content Modes`, `Appearance`, `Maintenance`) to 9 rows, eliminating dialog jitter when cycling tabs.
  - Synchronized mouse click detection in `src/tui/app/mouse.rs` with `settings_picker_layout` and `browse_picker_layout`.
- **Update Modal Minimalist Redesign & Single-Surface Card**:
  - Redesigned update popup from a multi-compartment divided box into a clean, single-surface dialog card (`src/tui/overlay.rs` and `src/tui/app/run.rs`).
  - Embedded version directly in the top title frame (`Update Available: vX.Y.Z`) and docked primary actions directly into the bottom border (`[u] Update ──── [o] GitHub`), eliminating all internal divider lines and wasted vertical padding.
  - Unified modal positioning by anchoring both update notification and in-progress update modals directly at the landing search bar position (`home_search_y`), keeping the dimmed ASCII logo visible above while focusing full attention on update progress.
  - Integrated category badges inline with release highlights (`[Added]`, `[Fixed]`, `[Changed]`, `[Perf]`) while retaining full Markdown compatibility with GitHub release notes.
  - Streamlined in-progress update modal into a compact 42-column status pill (`Updating: vX.Y.Z`) displaying active mechanical stage (`Downloading release`, `Verifying checksum`, `Installing binary`) without noisy warning labels or type mixing.
- **Details Poster Geometry Stability Across Dub Synopsis Lengths**:
  - Locked `content_rows` to 6 in `DetailsLayoutTier::header_height` (`src/tui/screens/details.rs`) when `show_poster` is active, maintaining consistent 8-row header height and 6-row poster container dimensions.
  - Prevented poster image shrinking and header layout shifts caused by variable synopsis text lengths across audio dubs and releases.
- **Single-Line Input Windowing & Overflow Prevention**:
  - Fixed text overflow in `render_single_line_input` (`src/tui/widgets/input.rs`) where appending ellipsis (`...`) without sufficient budget caused line length to exceed modal width and trigger word wrapping onto a second line.
  - Reserved 3 columns for trailing ellipsis when characters remain past the cursor and removed `Wrap` on single-line input widgets, keeping prompt symbols and text strictly single-line across all cursor movements.
- **Addons Manager Search Bar Anchor & Overlay Isolation**:
  - Anchored Addons Manager popup directly in place of the landing search bar using unified `home_search_y` geometry in `src/tui/overlay.rs`.
  - Gated the landing search bar, search suggestions, provider pill popup, and discovery decks in `src/tui/screens/home.rs` during active Addon Manager sessions, preventing background search bar leakage.
- **MovieBox Deprecation Notice Stream Filtering**:
  - Filtered deprecation notice video URLs (`macdn.aoneroom.com/other/`, notice video hash `b164fbfb4347792950bdfbfb563d39d9`) from community resource releases in `src/providers/moviebox/adapt.rs`.
  - Prevented 21-second upgrade announcement video placeholders from leaking into stream selection as playable releases when episodes lack official DASH streams.
- **Audio Dub Label Sanitization & Stream Quality Deduplication**:
  - Prevented blank audio track gap in episode details by mapping standalone `"Dub"` version tags to `"English Dub"` in `clean_language_name` (`src/tui/screens/details.rs`).
  - Added native `540p` resolution badge mapping in `src/tui/widgets/badge.rs` to distinguish true `540p` media encodes from `480p`.
  - Deduplicated episode entries in `src/providers/dramachi/client.rs` across multi-resolution manifests, preventing duplicate episode rows while sorting available streams in descending resolution priority.
- **4KHDHub Resolution Detection & Numeric Sorting**:
  - Expanded `detect_quality` in `src/providers/fourkhdhub/parser.rs` to detect `4K`, `UHD`, `2160`, `1080`, `FHD`, `720`, `HD`, and `480` tokens.
  - Replaced ASCII string sorting with numeric resolution sorting (`resolution_u64()` descending, then `size_bytes` descending) in `parse_releases`, ensuring 4K UHD BluRay REMUXes prioritize ahead of lower-resolution streams.
- **MovieBox High-Bitrate Server Catalog Merging**:
  - Merged mobile DASH manifests (`/subject-api/play-info/v2`) and full-bitrate server files (`/subject-api/resource`) concurrently in `MovieBoxClient::episode_streams` (`src/providers/moviebox/mod.rs`), deduplicating stream links and sorting releases by numeric resolution and size.
- **External Player Adaptive Quality Optimization**:
  - Added `--ytdl-format=bestvideo+bestaudio/best` and `--hls-bitrate=max` to `mpv` command invocation in `src/player.rs` to prevent adaptive demuxers from locking into low-bitrate streams.
  - Added `--adaptive-logic=highest` to VLC invocation in `src/player.rs`.
### Changed
- **Minimalist Addon Manager Modal Redesign & Balanced Geometry**:
  - Streamlined Addon Manager popup into a content-fitted modal matching the provider popup design language, eliminating empty right-side letterboxing and arbitrary vertical blank gaps.
  - Replaced noisy capability bracket badges (`[Core]`, `[Meta]`, `[Streams]`, `[Catalog]`), button bracket wrappers (`[ Add Manifest URL ]`), and arbitrary gap rows with continuous list rhythm, clean checkmarks (`✓`) for active addons, symmetric 2-cell horizontal padding, and full-width background highlight bars (`highlight_symbol("")`).
  - Aligned mouse click hitboxes and keyboard navigation with simplified two-variant row indexing (`Addon(usize)` and `AddUrl`).
- **Contextual `/config` Command Separation & Slash Command Cleanup**:
  - Promoted `/config` to a dedicated slash command rather than an alias for `/settings`.
  - Restricted `/config` suggestion visibility to contexts where a configuration modal exists: TV mode (playlist manager) and Addons provider (manifest manager).
  - Added a contextual guidance notification when `/config` is entered while MovieBox or 4KHDHub is active, prompting users to use `/settings` for preferences or switch to Addons (`Ctrl+P`/`^P`) to configure addons.
  - Restored missing `/browse` description in `SlashCommand::description_for` to display proper annotations in search suggestions.
  - Added contextual `/config` command row to in-app help overlay (`?`) under TV Mode and Addons provider.
  - Pruned redundant and confusing slash command aliases (`/pref`, `/preferences`, `/options`, and `/fav`), establishing `/settings` and `/favorites` as single canonical commands while retaining universal terminal conventions (`/?` for `/help`, `/q`/`/quit` for `/exit`).
### Refactored
- **Streamlined MovieBox Episode Stream Resolution**:
  - Replaced speculative parallel `get_resources` and secondary `fetch_resource_page` fallbacks in `providers/moviebox/mod.rs` with direct `play-info/v2` resolution.
  - Eliminated sprawling 60-page background resource pagination loop in `src/tui/app/requests.rs`, cutting redundant network requests and preventing dead community upload parsing.

## [0.1.21] - 2026-09-19

### Added
- **Automated Issue Quality, Version Validation, and Duplicate Management**:
  - Modernized GitHub issue templates (`.github/ISSUE_TEMPLATE/`) to link directly to official mdBook documentation site guides (`mesamirh.github.io/MovieBox-Tui/`).
  - Added `.github/workflows/incomplete-issue.yml` with semver release validation checking user versions against latest GitHub releases and printing platform upgrade commands.
  - Implemented automated OS labeling (`os: android`, `os: linux`, `os: macos`, `os: windows`), title sanity validation, and Termux player setup guidance.
  - Added automatic removal of `needs-info` and `stale` labels when the issue author provides responses.
  - Added `.github/workflows/duplicate-detector.yml` using `actions-cool/issues-similarity` to flag and cross-reference duplicate issue submissions.
  - Added `.github/workflows/stale.yml` to automatically close abandoned `needs-info` issues after 7 days of inactivity.

### Security
- **Dependency Advisory Remediation**:
  - Updated `rustls` to `0.23.45` and `rustls-webpki` to `0.103.15`, resolving security advisory `RUSTSEC-2026-0285` (TLS 1.3 handshake boundary handling).

### Performance
- **TUI Landing Frame Latency & Allocation Pruning**:
  - Reduced landing frame draw latency from 28.95 µs to 23.03 µs (-20.4%) by eliminating per-frame heap allocations (`logo_text: &'static str`, zero-copy `&rows.rects` borrowing) and caching search result metrics across unscrollable viewports.
- **Zero-Copy Title Normalization (`clean_moviebox_title`)**:
  - Converted `clean_moviebox_title` from an allocating `String` generator to a pure zero-copy slice parser (`&str -> &str`), replacing allocating `.to_lowercase()` substring searches with in-place ASCII case-insensitive matching (`rfind_ignore_ascii_case`).
  - Achieved sub-microsecond parsing latency: 191.43 ns/op over 10,000 operations with zero heap allocations.
### Changed
- **Windows Installer Terminal Lifecycle and Environment Propagation**:
  - Replaced process-terminating `exit` statements with scoped `return` in `install.ps1`, preventing host terminal windows and tabs from abruptly closing during piped `iex` execution or on preflight warnings.
  - Added Win32 `WM_SETTINGCHANGE` environment broadcast via `SendMessageTimeout`, forcing Windows Explorer and newly launched terminals to immediately refresh `PATH` without requiring a system sign-out.
  - Removed buffer-wiping `[Console]::Clear()` from installer header initialization to preserve user terminal diagnostic history.
  - Stripped Unix `$` shell prompt prefix from streaming launch guidance and added direct binary execution fallback path (`& "$ExePath"`) for existing terminal windows.
- **Concise Error and Status Messaging**:
  - Streamlined `ProviderError::user_message` to output compact, high-signal status messages under 40 characters for mobile and compact viewports.
  - Replaced sprawling raw socket errors and leaked endpoint URLs with clear failure reasons (`CircleFTP unreachable: requires BDIX network.`, `MovieBox timed out.`, `Cannot reach 4KHDHub.`, `No results found.`).
  - Shortened 4KHDHub playback and download resolution timeout messages to fit single-row status lines (`4KHDHub timed out.`).
  - Sanitized subtitle download/write error strings to prevent raw filesystem/network error leaks.
  - Compacted addon torrent stream warning to `Blocked {} torrent streams. HTTP only.`.
  - Streamlined player crash fallback diagnostic to `Player exited (code {code}).`.
### Fixed
- **In-Flight Notification Replacement & Compact Stream Errors**:
  - Generalized notification category matching in `AppState::notify` across stream and playback domains, ensuring stream resolution errors replace in-flight "Preparing playback" toasts in-place instead of creating visual overlapping.
  - Compacted 4KHDHub stream error messages from 135-character redundant text blocks to concise messages under 30 characters (`Mirrors dead or expired.`).
- **4KHDHub Mediator Redirector Resolution**:
  - Implemented automatic mediator unpacker in `src/providers/fourkhdhub/hubcloud.rs` supporting `greenmotors.club` and `greenmountmotors.` intermediate redirector domains.
  - Implemented multi-stage decoding pipeline (double Base64, ROT13, JSON extraction) to transparently recover downstream HubCloud and HubDrive mirror endpoints without external browser dependencies.
  - Excluded mediator domains from direct file classification in `parser.rs` and added mediator domain rejection to `validate_playback_url`.
- **Playback vs Download Mirror Resolution Separation**:
  - Introduced `ResolutionIntent` (`Playback` vs `Download`) in `src/providers/models.rs` and wired through `resolve_release`.
  - For playback, prioritized seekable multi-connection video CDNs (`pixel.hubcloud.` -> Google Video CDN, Cloudflare R2, PixelDrain API) while deprioritizing single-use download workers (`workers.dev`).
  - Added `downloadQuotaExceeded` and `Access Denied` error body detection in `FourKHdHubClient::preflight` to instantly reject exhausted worker links and prevent IINA HTTP authentication dialogs.
- **4KHDHub Multi-Stream Deduplication**:
  - Fixed an issue where 4KHDHub stream releases were prematurely collapsed into a single item by scoping query-string-insensitive URL deduplication strictly to MovieBox CDN streams.
  - Upgraded 4KHDHub stream cache schema to `v4_` to invalidate stale single-stream caches.
- **Terminal Color Support Environment Isolation**:
  - Scoped process environment lookups (`ALACRITTY_WINDOW_ID`, `WEZTERM_EXECUTABLE`, `TILIX_ID`, `VTE_VERSION`) strictly to `ColorSupport::current()` rather than the pure `classify_terminal` helper.
  - Eliminated host environment variable leakage where running tests inside Alacritty or WezTerm falsely forced 256-color and basic terminals to report Truecolor support.
  - Extracted pure `is_vte_version_truecolor` validator, eliminating non-thread-safe `std::env::set_var` test mutations.
- **Android Termux Intent Opener Resiliency & Socket Fallback**:
  - Implemented multi-opener resolution in `src/player.rs` returning ordered candidate commands (`termux-am` → `termux-open` → `termux-open-url`).
  - Added automatic in-flight fallback in `src/tui/app/playback.rs`: if `termux-am` exits with `am.sock` / socket connection failure on Android 12+, MovieBox-TUI automatically retries with `termux-open` without halting playback.
  - Replaced long and inaccurate `"Run: pkg install -y termux-am"` notifications with concise mobile-formatted messages under 32 characters (`Termux Setup: Run 'pkg install termux-tools'.`, `No Player: Install a video player.`, `CLI mpv: Switch to Android Player in /settings.`).
- **tmux Poster Image Passthrough**:
  - Replaced the hard-coded `$TMUX` detection block in `src/tui/terminal.rs` with an outer terminal graphics capability probe (`GHOSTTY_RESOURCES_DIR`, `KITTY_WINDOW_ID`, `WEZTERM_EXECUTABLE`, `ITERM_SESSION_ID`, `ALACRITTY_LOG`, `ALACRITTY_WINDOW_ID`, `foot`).
  - Enabled automatic poster graphics queries and DCS passthrough inside `tmux` sessions running within Ghostty, Kitty, WezTerm, iTerm2, foot, and Alacritty, eliminating empty "No Art" placeholders.
- **Legacy Windows Console Compatibility (Windows 8.1 & conhost)**:
  - Eliminated unsupported `underline-color` control codes from ratatui crossterm backend, restoring full TUI rendering on Windows 8.1 and legacy Windows console hosts where `SetUnderlineColor` causes draw frame errors.
  - Pruned unused `all-widgets`, `widget-calendar`, and `macros` feature dependencies from ratatui build graph.
- **Update Modal Geometry & Symmetrical Border Padding**:
  - Eliminated unnecessary dead vertical gap below action buttons in "Update Available" dialog by calculating exact rendered line heights (`update_modal_layout_with_env`) accounting for installation environment notices.
  - Symmetrized vertical padding with balanced 1-row margins above the version header and below the action button row, eliminating bottom-heavy content displacement.
  - Aligned inner horizontal margins to a uniform 2-column padding on both left and right borders, preventing premature right-side text truncation.
  - Synchronized mouse click hitbox (`button_row_y`) with rendered action buttons across all package environments (DirectReplace, Homebrew, Termux, Flatpak, Snap).

## [0.1.20] - 2026-09-14

### Added
- **Overview and Synopsis Modal with Stremio Episode Summary Support**:
  - Implemented a centered, scrollable Overview/Synopsis dialog (`draw_overview_modal` in `src/tui/overlay.rs`) for inspecting full, unwrapped descriptions of movies, TV shows, and episodes without layout distortion.
  - Added keyboard shortcut `i` / `I` and footer action `[i] Info` in the Details screen to toggle the overview dialog, with `↑`/`↓`/`j`/`k`/`PageUp`/`PageDown` content scrolling and `Esc`/`q`/`Enter`/`i` dismissal.
  - Added mouse interaction opening the overview modal when clicking the header metadata card, dismiss-on-outside-click, and mouse wheel scrolling.
  - Appended `... [i]` truncation indicator in header synopsis rendering when text exceeds the bounded header row budget.
  - Extended canonical `Episode` data model (`src/providers/models.rs`) with `pub overview: Option<String>` and updated the Stremio Addons metadata adapter (`src/providers/addons/`) to deserialize and forward episode descriptions and titles from `MetaVideo`.

- **Modal Picker Navigation & Page Scrolling Unification**:
  - Added `j` and `k` vim key navigation across Browse Categories, TV Playlist Manager, and Stremio Addon Manager popups, unifying list navigation bindings across all modals.
  - Added `PageUp` and `PageDown` 5-item stepping to Provider menu and Player picker popups, and `Home` / `End` boundary jumping to Player picker.
  - Fixed `PageUp` and `PageDown` in Help overlay to scroll by dynamic visible terminal page height instead of single-line stepping.
  - Directly confirmed or canceled download confirmation dialogs via `y`/`Y` and `n`/`N` only when active, preventing dead branches on the Details screen.

- **Named Constants & Path Centralization**:
  - Extracted canonical `TERMUX_PREFIX_USR` constant in `src/updater/artifact.rs` and refactored scattered Termux binaries and library path probes across `src/player.rs`.
  - Extracted `STANDARD_UNIX_BIN_DIRS` in `src/player.rs` consolidating repeated `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, and nix profiles.
  - Extracted `MAX_PLAYLIST_BYTES` (15 MiB limit) in `src/providers/tv/parser.rs` replacing 4 duplicate inline checks.
  - Added `total_duration()` on `NotificationKind` in `src/models.rs`, eliminating duplicated duration matching in `src/tui/overlay.rs`.
  - Extracted `ENV_MOVIEBOX_PLAYER`, `STREAM_REFERER`, `SESSION_CACHE_FILE`, `APP_HTTP_USER_AGENT`, `release_tag_url`, and `CIRCLEFTP_BASE_URL` into canonical module constants.

### Changed
- **Modal Geometry and Margin Symmetrization**:
  - Balanced four-sided padding in `draw_updating_modal` ("Self-Update in Progress"), standardizing outer dimensions to 50x7 columns with equal 1-row top and bottom margins and symmetrical side margins.
  - Symmetrized "Update Available" modal layout in `src/tui/overlay.rs` by calculating exact rendered line heights instead of over-allocating fixed header and footer rows, eliminating empty vertical gaps above the button row.
  - Removed premature text truncation in update modal bullet items, allowing release notes to span the full available width of the inner frame.
  - Synchronized mouse click hitbox for update modal action buttons (`button_row_y`) with rendered button coordinates.
  - Stripped redundant `"Press [o] to read full changelog on GitHub"` line from release note bodies to avoid duplicating the bottom `[o] Open Release Page` action button.

### Fixed
- **Codebase Production-Readiness & Reliability Hardening**:
  - Fixed UTF-8 byte boundary slicing panic in download path input rendering (`src/tui/widgets/settings.rs`) by computing grapheme-to-byte offsets via `input.cursor_byte_offset()`.
  - Prevented playback lockups when stream resolution fails or times out in background resolution tasks (`src/tui/app/playback.rs`) by dispatching `Action::PlayerExited` to reset `is_resolving_playback` and `is_playing`.
  - Resolved permanent Flatpak player cache miss loop (`src/player.rs`) by checking command string prefixes (`"flatpak run "`) alongside filesystem path probes.
  - Prevented GNU Automake `/usr/bin/am` on desktop Linux from being falsely detected as the Android Activity Manager (`src/player.rs`) by strictly gating system `am` probes behind `target_os = "android"`.
  - Isolated Details screen keyboard handling from background pane cycling and conflicting modal actions when `is_download_subtitle_popup` is active (`src/tui/app/keyboard.rs`).
  - Cleared stale modal overlay flags on provider switches and view resets (`src/tui/app/navigation.rs` and `src/tui/state.rs`).
  - Detached Windows self-updater helper process (`src/updater/apply.rs`) using `DETACHED_PROCESS` and `CREATE_NEW_PROCESS_GROUP`, preventing console termination on exit from aborting in-flight binary replacement.
  - Isolated background `yt-dlp` downloads on Windows (`src/tui/app/download.rs`) in a separate process group (`CREATE_NEW_PROCESS_GROUP`), preventing terminal `Ctrl+C` interrupt signals from prematurely killing active downloads.
  - Restricted Android `~/storage/downloads` directory path overrides (`src/service.rs`) strictly to verified Termux environments (`is_termux_environment()`), preserving standard desktop Downloads paths on Linux and macOS.
  - Exempted live TV streams and media lacking duration (`duration_seconds: None`) from the Continue Watching shelf (`src/history.rs`), preventing infinite in-progress shelf entries.
  - Enforced 15MB file size limits (`MAX_PLAYLIST_BYTES`) on local M3U playlist file reads (`src/providers/tv/parser.rs`), preventing memory exhaustion when reading oversized local files.
  - Suppressed background list scrollbars on Details screen (`src/tui/screens/details.rs`) and Home search results (`src/tui/screens/home.rs`) when modal overlays (such as synopsis description, download confirmation, or settings) are active, preventing bright purple scrollbar tracks from bleeding onto dimmed backdrop panes.
  - Fixed subtitle proxy header duplication and conflicting MIME type bug (`src/proxy.rs`) by writing `Access-Control-Allow-Origin: *` and subtitle `Content-Type` outside the upstream header loop, stripping query parameters and case-normalizing URLs (`.srt`/`.vtt`) to override upstream generic content types and ensure clean single-header responses across media players.
  - Corrected BDIX network probing in `src/tui/app/system.rs` by querying CircleFTP's active API route (`/api/posts`) instead of unrouted 404 `/api`, concurrently evaluating all DhakaFlix mirror endpoints (`172.16.50.7/14/12/9`), and preserving existing user-enabled mirror states across startup probes.

- **External Player Launch Reliability & Crash Normalization**:
  - Filtered false `PlayerCrashed` notifications when VLC exits with status code `1` and empty stderr (VLC normal exit / end of stream with `--play-and-exit`), and treated Unix `SIGTERM` signal termination as a clean user/OS-initiated quit.
  - Eliminated process pipe deadlock and blocking thread leaks when media players spawn persistent background subprocesses by reading stderr concurrently with `child.wait()` and applying a 2-second drain timeout.
  - Removed Windows Store App Execution Alias (`Microsoft\WindowsApps\vlc.exe`) from VLC candidate paths, preventing 0-byte reparse points from hijacking detection and failing launch.
  - Emitted separate, independent `--http-header-fields` CLI arguments for mpv and IINA instead of comma-joining them, preventing header values with commas (cookies, MIME types) from being parsed into corrupted fragments.
  - Added `~/Applications/IINA.app/Contents/MacOS/iina-cli` candidate path probe and surfaced diagnostic status warning when IINA falls back to `open -a IINA` without CLI argument support.

- **Text Input Widget Cursor Truncation & Ellipsis Normalization**:
  - Fixed double-ellipsis rendering (`"......"`) in single-line text inputs when text exceeds boundaries by retaining single-ellipsis trailing truncation and implementing reverse cursor-adjacent prefix truncation from the cursor backwards.

- **Help Screen Scroll Indicator Math & Home Multi-Column Thumb Tracking**:
  - Corrected denominator in single-column scroll position display in `src/tui/screens/help.rs` from `scroll + 1 / max_scroll` to `scroll + 1 / max_scroll + 1`, eliminating `11/10` display anomalies at bottom scroll boundaries.
  - Synchronized search results scrollbar thumb position and viewport bounds in `src/tui/screens/home.rs` with calculated row coordinates (`div_ceil(columns)`) rather than raw linear item counts.

- **Provider Quality Parsing & DhakaFlix ID Port Robustness**:
  - Handled `"4k"` and `"2160p"` explicitly in `Release::resolution_u64` in `src/providers/models.rs`, preventing BDIX 4K releases from falling back to 1080p sort priority.
  - Replaced colon-splitting in DhakaFlix ID parsing with `rfind(":/")` delimiter slicing in `src/providers/bdix/dhakaflix/client.rs`, preventing base URLs with explicit ports (`:8080`) from corrupting API path requests.

- **Windows Process Group Isolation & Unix Signal Reporting**:
  - Configured `CREATE_NEW_PROCESS_GROUP` on Windows in external player spawning (`playback.rs`), preventing terminal interrupt signals (Ctrl+C / Ctrl+Break) from terminating active external players.
  - Inspected Unix exit status signals in `clean_player_error` when exit code is `None`, reporting informative diagnostics (`Player terminated by signal {sig}.`) instead of generic failure messages.
  - Added `AppState::reset_details_view()` to reset selector panes, season/episode selections, and modal state when leaving Details screen, and aborted pending stream pool and episode prefetch tasks.

### Performance
- **Action Enum Footprint Reduction (73.6% Memory Reduction)**:
  - Boxed `MediaDetails` in `Action::PreviewSuccess` and `Action::DetailsSuccess`, and boxed `WatchHistoryItem` in `Action::MarkWatched` and `Action::UpdateProgress`, reducing `Action` enum memory footprint from 424 bytes to 112 bytes across all event queues and channel messages.

- **Zero-Allocation Addons Catalog Parsing & Stream Deserialization**:
  - Centralized fallback catalog parsing in `parse_catalog_metas` in `src/providers/addons/client.rs` using `Value::take`, eliminating intermediate array cloning during catalog and stream deserialization.

- **Theme ColorSupport Environment Probe Memoization**:
  - Cached `ColorSupport::current()` via `std::sync::LazyLock` in `src/tui/theme.rs`, eliminating repeated inspection of terminal environment variables on every theme load and swatch render.

- **Overview Modal Text Wrapping Deduplication**:
  - Consolidated layout calculation and line wrapping in `draw_overview_modal` into a single pass, eliminating redundant duplicate text wrapping passes per frame.

- **Browse Preset In-Place Sorting & Mouse Event Allocation Pruning**:
  - Replaced full clone of `AppState::browse_metrics` during search result sorting with disjoint field borrows, eliminating heap allocations during browse ranking.
  - Replaced deep clone of `MediaDetails` on every mouse event in the Details screen with borrowed field inspection.

### Removed
- **Dead Code and Obsolete Action Variants**:
  - Removed unused `Action::LaunchPlayer` (consolidated into `Action::LaunchPlayback`), removed dead `selection_symbol` in Details screen, removed obsolete `_fourk_release` legacy check in MovieBox adapter, and replaced runtime `b64_decode` in MovieBox signing with a pre-computed byte literal.
  - Consolidated duplicate text buffer keyboard navigation branches across download dir, addon URL, and TV playlist inputs into `TextInputBuffer::handle_key`.

- **README Walkthrough Media Asset and Donate Anchor Focus**:
  - Updated the WebM walkthrough video attachment link to the latest asset URL across all localized READMEs and documentation.
  - Moved the `#optional-support` anchor target inside the collapsible `<details>` container with `tabindex="-1"`, enabling native browser ancestor-revealing and keyboard focus navigation to automatically expand the crypto donation section when clicking the Donate badge.

- **Termux Android Player Prioritization and Compact Error Diagnostics**:
  - Prioritized `PlayerKind::AndroidIntent` at index 0 in `player::detect()` when running inside Termux, preventing auto-selection of headless CLI `mpv` or `vlc` when CLI packages are installed without an active X11/Wayland display server.
  - Piped stdout in addition to stderr for `AndroidIntent` process spawning in `src/tui/app/playback.rs`, capturing intent dispatcher errors (`ActivityNotFoundException`, `no activity found to handle Intent`, `am.sock` socket connection failures) previously discarded by `Stdio::null()`.
  - Replaced generic `Crash code: 1` modals with compact, player-neutral diagnostics for missing video player apps (`No Video Player: Install a video player on Android.`), unconfigured Termux intent tools (`Termux Setup Needed: Run: pkg install -y termux-am`), and headless mpv execution in Termux (`CLI mpv Unsupported: Switch to Android Player in /settings.`).
  - Tuned mobile toast message height constraints in `src/tui/overlay.rs` (`max_msg_lines = 2` when `area.height >= 20` and `area.width < 65`), ensuring two-line diagnostics are rendered without premature truncation on mobile portrait screens.
## [0.1.19] - 2026-09-13

### Added
- **VLC MovieBox DASH and Signed Cookie Streaming Compatibility**:
  - Added a detached loopback HTTP sidecar proxy (`--proxy-for-vlc`) enabling VLC playback for MovieBox MPEG-DASH and MP4 streams protected by AWS CloudFront signed cookies (`CloudFront-Policy`, `CloudFront-Signature`, `CloudFront-Key-Pair-Id`).
  - Implemented hierarchical path proxy routing (`/https/<host>/path` and `/http/<host>/path`) in `src/proxy.rs`; VLC's adaptive demuxer resolves relative segment URLs against the proxy path, eliminating query-string stripping that caused 400 Bad Request on DASH segment fetches.
  - Added in-flight DASH manifest rewriter targeting origin CDN hosts to rewrite relative and absolute segment URLs to route through the local proxy.
  - Added zero-copy chunked streaming using `reqwest::Response::bytes_stream()` for constant-memory chunk forwarding without buffering full segments in RAM.
  - Configured platform process detachment via `process_group(0)` on Unix and `DETACHED_PROCESS` with `CREATE_NEW_PROCESS_GROUP` on Windows, ensuring streaming survives terminal window closure.
  - Updated `supports_headers` and `header_capable_players` in `src/player.rs` to allow VLC as a supported player for all streaming sources.


- **Android Intent MovieBox Playback via Loopback Proxy**:
  - Extended loopback HTTP sidecar proxy (`src/proxy.rs`) to support `PlayerKind::AndroidIntent` for streams requiring CloudFront signed cookies.
  - Enabled `AndroidIntent` in `supports_headers` and `header_capable_players`, allowing external players on Android (VLC, MX Player, Just Player) to play MovieBox streams without header compatibility errors.
  - Added proxy route `/sub/<encoded_url>` with CORS headers and proper MIME typing (`application/x-subrip`, `text/vtt`), injecting WebVTT adaptation sets into MPEG-DASH manifests.
### Fixed
- **Crash Hardening & Bounds Safety**:
  - Fixed potential `usize` arithmetic underflow on empty search results in navigation handler (`.saturating_sub(1)`).
  - Fixed potential `usize` arithmetic underflow on empty player picker navigation.
  - Guarded JWT token split indexing with safe `.get(1)` pattern matching in MovieBox session parser.
  - Hardened string slicing against non-ASCII UTF-8 character boundary hazards in DhakaFlix title/year parsing.
  - Bounded help modal line-window slicing index to prevent out-of-bounds panics under constrained terminal heights.
  - Clamped confirmation dialog action-row coordinate calculation to modal bounds on small screens.
- **UI Layout & Mouse Synchronization**:
  - Extracted shared `split_main_and_download` layout helper, eliminating constraint divergence between mouse hit testing and rendering passes.
  - Synchronized details screen workflow step mouse hit-detection with dynamic text layout bounds.
  - Replaced byte length count with display Unicode width for notification toast badge column derivations.
  - Fixed addon manager badge column calculation to derive from visual Unicode width.
  - Fixed provider selection popup list scroll state retention in Home screen rendering.
- **Concurrency, Architecture & Network Safety**:
  - Resolved architectural layer violation where provider modules directly imported text utilities from TUI presentation layer.
  - Tracked pagination search, stream pool initialization, and episode prefetch tasks in `RequestTaskHandles` for prompt cancellation on navigation.
  - Offloaded synchronous filesystem writes and directory creations in playback, download, and startup paths to background threads.
  - Throttled high-frequency download progress channel event dispatch to 100ms intervals.
  - Filtered forwarded Stremio addon stream HTTP headers against strict allowlist to prevent SSRF or credential leakage.
  - Hardened network URL reachability probe to enforce HTTP 2xx/3xx status verification.
  - Enforced response body size limits on remote IPTV M3U playlists (15 MB).
  - Propagated DhakaFlix network errors instead of silently swallowing empty streams.
  - Fixed `next_provider` navigation to handle currently disabled active providers correctly.
  - Consolidated `/exit` slash command in autocomplete candidate suggestions.
  - Centralized BDIX codec, resolution, and audio language heuristics into shared module.
  - Pruned unused structs and dead code in CircleFTP and Addon models.
- **VLC Sidecar Proxy Correctness**:
  - Replaced `reqwest::Client::timeout(30s)` (a hard deadline over the entire response body) with `connect_timeout(15s)` only; per-chunk idle timeout (60s) now signals stalled transfers without terminating long-running video streams mid-playback.
  - Introduced a `ConnectionGuard` RAII wrapper ensuring `active_connections` is atomically decremented even when a connection handler task panics, eliminating a counter leak that kept the sidecar alive indefinitely under 24/7 operation.
  - Set watchdog idle threshold to 10 minutes (600s) with a 15-second polling interval; removed parent-PID liveness polling — the sidecar is fully detached and exits cleanly after 10 minutes of inactivity with no active connections.
  - Fixed host-authority extraction to preserve explicit port numbers (e.g. `cdn.example.com:8080`) in DASH manifest segment URL rewriting; previously the port was stripped, routing manifest-relative segments to the wrong CDN endpoint and causing 400/403 errors.
  - Capped HTTP request-line and header reads at 8 KiB per line and 64 headers per request, preventing unbounded `read_line` memory growth from malformed or adversarial clients.
  - Added `child.wait()` after `child.kill()` on proxy spawn failure to reap the child process and avoid zombie accumulation.
- **Search Poster Loader**: Cleared `in_flight_posters` when navigating back from the Details screen (`GoBack` → `Screen::Details` path) so that poster fetch tasks cancelled mid-flight no longer leave permanent "Loading..." placeholders on the next visit.
- **Stream Cache TTL**: `set_provider_stream_cache_typed` now parses `DateLessThan` / `AWS:EpochTime` from the `CloudFront-Policy` Base64 payload embedded in release `Cookie` headers and caps the on-disk cache TTL to `min(2h, cf_expiry_remaining)` (floor 60s), preventing stale cached stream URLs from being served after CloudFront signed-cookie expiry.
- **Process & Task Lifecycle Resilience**:
  - Configured `kill_on_drop(true)` on `tokio::process::Command` when spawning `yt-dlp` in `start_resilient_download`, preventing orphaned download worker processes from continuing in the background if tasks are cancelled.
  - Added `JoinHandle` tracking for active download workers in `RequestTaskHandles`, pairing it with an asynchronous watcher that catches background worker panics and dispatches `Action::DownloadFailed` to prevent permanent state lockup.
  - Hardened loopback proxy sidecar accept loop to catch transient I/O and network errors (`EMFILE`, `ECONNABORTED`, `EINTR`) with a 50ms backoff sleep, preventing premature daemon exit under socket pressure.
  - Ensured `fetch_cancel` Arc is rotated to a fresh instance and active stream/details fetch tasks are explicitly aborted upon navigating back from the Details screen (`GoBack`), preventing subsequent searches from being poisoned by aborted states.
  - Wrapped `Action::CheckForUpdates` in a strict 15-second timeout, preventing indefinite UI loading states during transient network drops or GitHub API stalls.
  - Threaded explicit task cancellation across `RequestTaskHandles`, download cancellation tokens, and search fetch tokens upon receiving `Action::Quit`, ensuring all background I/O operations terminate before the terminal exits.
  - Replaced detached spawned tasks in `AddonClient::fetch_addon_streams` with direct futures via `futures::future::join_all`, ensuring that cancelled stream lookups immediately drop pending addon network requests.
  - Added sandbox environment detection for Flatpak (`FLATPAK_ID`, `/.flatpak-info`) and Snap (`SNAP`) in `src/updater`, preventing corrupt in-app self-update writes on read-only mountpoints and guiding users to their package managers.
  - Staged update binaries in the directory adjacent to `current_exe` rather than a temporary filesystem mount, ensuring atomic replacement across distinct storage partitions (`EXDEV`).
  - Fixed Windows update helper double-spawn race condition by ensuring the parent process terminates immediately without invoking redundant exec/spawn wrappers.
  - Added root uid check (`libc::getuid() == 0`) before probing `/system/bin/am` on Android outside Termux, preventing SELinux exit code 126 crashes on modern Android releases.
  - Gated Android `/storage/downloads` subtitle path probing with `is_termux_environment()`, preventing non-Android systems with `~/storage` from misrouting subtitle files.
  - Updated `config_dir()` and `data_dir()` to fall back to `std::env::temp_dir()` with an explicit warning when user directories cannot be resolved.
  - Updated `install.sh` to provide clear guidance when invoked on Windows MINGW/MSYS/Cygwin environments, and hardened SHA256 checksum parsing against leading asterisk formatting.
- **TUI & Render Hot-Path Efficiency**:
  - Added `id_index` (`HashSet<(String, String, i64)>`) to `FavoritesManager`, eliminating $O(N)$ linear scans across visible result cards on every render frame.
  - Bound window title recomputations in `contextual_title()` to dirty frame updates or initial launches, eliminating per-tick allocations.
  - Added active screen and editing mode guards to search suggestion debounce in `Action::Tick`, skipping redundant string operations when not actively typing queries.
  - Consolidated duplicate theme picker keyboard navigation actions (`Up`, `Down`, `Home`, `End`, `PageUp`, `PageDown`) into a single state change handler.
  - Pruned unused scaffolding structs (`UiState`, `CatalogState`, `PlaybackState`, `DownloadState`) from `src/tui/state.rs`.
  - Pruned obsolete and unused action variants (`ProbeTerminal`, `ShowSettingsPopup`, `CloseSettingsPopup`) from `src/tui/action.rs` and consolidated settings handlers.
  - Consolidated all video playback launches through canonical `LaunchPlayback` actions, removing misleading and redundant `LaunchMpv` aliases.
  - Prioritized `termux-am` ahead of `termux-open` in Android opener probing to preserve HTTP request headers and subtitle arguments.
### Changed
- **Symmetrical Popup and Picker Margin Alignment**:
  - Eliminated lopsided right-side dead space across all floating popup pickers (Theme Picker, Streaming Sources, Media Player, Subtitles, Catalog Browse, and Provider Popup), aligning borders to provide equal horizontal padding on both sides (`│  content  │`).
  - Corrected `picker_layout` width calculation to `content_width.saturating_add(2)` (accounting for left and right border glyphs) rather than over-allocating `+ 6`, eliminating 4 phantom trailing empty columns.
  - Standardized symmetrical 2-space padding across all popup picker lines and hit-testing rectangles in `src/tui/app/mouse.rs`.
  - Updated `overlay::picker` to automatically apply uniform 2-space left and right margins to dynamic items.
- **Linux ARM64 Production Portability & Installer Hardening**:
  - Removed Android Bionic TLS assembly hacks (`__bionic_tls_align_anchor`) and Python `PT_TLS` byte-patching from the generic Linux `aarch64-unknown-linux-musl` target, isolating Android Bionic requirements to the dedicated `build-android` (`aarch64-linux-android`) NDK pipeline.
  - Configured `-C link-arg=-Wl,-z,max-page-size=65536` on `aarch64-unknown-linux-musl`, aligning ELF `PT_LOAD` segments to 64KB to guarantee execution compatibility across 4KB kernels (Raspberry Pi 4B), 16KB kernels (Raspberry Pi 5 / BCM2712), and 64KB kernels (AWS Graviton, enterprise Linux ARM64).
  - Added a QEMU (`qemu-user-static`) execution smoke test gate in the release CI pipeline for `aarch64-unknown-linux-musl`, actively booting the compiled AArch64 Linux binary and verifying `--version` before packaging.
  - Enhanced `install.sh` to proactively detect 32-bit Raspbian userland (`getconf LONG_BIT == 32`) on 64-bit ARM hardware, providing an actionable native `cargo install` path rather than attempting incompatible 64-bit binary execution.
  - Improved `install.sh` smoke test failure diagnostics to capture and print explicit non-zero exit codes when binary execution fails without output.
- **Modal and Overlay Declutter — Keyhint Footer Removal**:
  - Removed redundant boilerplate footer keyhint bars (`render_modal_footer` / `render_footer`) and top divider borders across all modal dialogs and overlay pickers (Settings, Streaming Sources, Themes, Players, Subtitles, TV Config, Addon Manager, Provider popup).
  - Removed duplicate and truncated popup title headers on option-selection pickers (Media Player, Themes, Sources) so they cleanly present options without repeating row labels.
  - Compacted Settings modal vertical geometry to snap directly to the search bar row position and eliminated dead spacer gaps between category tabs and content rows.
  - Added background dimming overlays behind all modal pickers to prevent background text bleed-through.
  - Streamlined Settings rows into single-line entries with aligned label and value spans, removing redundant explanatory subtexts.
  - Expanded Settings → Content Modes from a coarse global toggle to individual provider controls covering MovieBox, 4KHDHub, CircleFTP (BDIX), and DhakaFlix (BDIX) alongside Streaming and Live TV mode toggles.
  - Added an active provider guard ensuring at least one streaming provider remains enabled at all times and automatically falling back to the next available source if the currently active provider is disabled.
  - Preserved backward compatibility with legacy `bdix_enabled` configs by automatically migrating enabled BDIX state to both CircleFTP and DhakaFlix flags on startup.
- **Automatic BDIX Network Availability Probing**:
  - Added non-blocking HTTP network probes (`probe_url`) on first application startup with a 3-second timeout against CircleFTP and DhakaFlix endpoints, automatically enabling accessible optical mirrors on local networks while disabling unreachable ones.
  - Added a dedicated "Re-check BDIX Network" action in Settings → Maintenance allowing users to manually re-probe and refresh BDIX provider availability at any time.
- **Watch History Clearing and Item Management**:
  - Added a dedicated "Clear Watch History" action row in Settings → Maintenance, allowing users to wipe all watch history records and saved playback positions in one action.
  - Added individual item deletion via `d` and `Delete` keybindings when browsing `/history` results or the Home screen Resume deck, updating disk storage and notifying immediately.
- **Unified Streaming and Stremio Addons Engine**:
  - Merged separate Addon Mode into standard Streaming Mode; Stremio Addons (`ProviderKind::Addons`) is now a first-class streaming provider selectable directly via `Ctrl+P`.
  - Simplified application state model by removing `AppMode::Addon` and `AppState.is_addon_mode`, making navigation two-mode (`Streaming` and `Live TV`).
  - Routed Addon Manager dialog access through `/config` when the active provider is `Addons`.

- **Theme Color and Contrast Hardening**:
  - Added typed background and surface color helper methods on `Theme` (`surface0_color`, `surface1_color`, `surface2_color`, `crust_color`, `mantle_color`) extracting underlying foreground colors with graceful `base` fallback.
  - Replaced hardcoded Catppuccin Mocha RGB values across badge, home screen pill, confirmation modal, and settings input backgrounds with dynamic theme color helpers, ensuring correct contrast across all dark and light themes.
  - Fixed resolution badge text on active rows to dynamically derive contrast foreground from `theme.crust_color()` rather than hardcoded Mocha crust RGB (`17, 17, 27`).
  - Fixed inactive modal badge text style from `theme.muted` to `theme.overlay1`, eliminating low-contrast illegibility on `surface1` backgrounds.
  - Hardened Catppuccin Latte light theme tokens (`sapphire`, `lavender`, `highlight`, `success`, `shortcut`, `rating`, `teal`), achieving WCAG AA contrast (≥4.5:1 on base, ≥3.0:1 on surface1) without sacrificing hue recognition.

- **Terminal Color Quantization and Standard Detection Fixes**:
  - Fixed `rgb_to_xterm256` color quantization to compute squared Euclidean distance to both the nearest 6×6×6 cube entry and the nearest 24-step grayscale ramp, preventing dark and low-saturation palette backgrounds (such as Mocha base, Nord base, and Dracula base) from crushing to pure black or wrong-hue cube cells on 256-color terminals.
  - Aligned `NO_COLOR` environment variable checks with the no-color.org specification by checking non-empty values (`is_ok_and(|v| !v.is_empty())`), ensuring empty strings do not disable color support.
  - Constrained `VTE_VERSION` truecolor capability detection to VTE versions ≥ 3600, correctly identifying older VTE terminals as 256-color rather than truecolor.
  - Replaced hardcoded fallback colors (`Color::Rgb(69, 71, 90)` and `Color::Rgb(116, 199, 236)` on landing overflow pill, and `Color::White` on details metadata) with dynamic theme palette colors (`theme.base`).
- **Notification System Overhaul and Adaptive Layout Polish**:
  - Added instant `Esc` key dismissal for active notification toasts across home and details screens when no popup modals or text input fields are active.
  - Replaced fixed 3-card stack with terminal dimension-adaptive budgets: compact/Termux displays ($H < 20$ or $W < 65$) show at most 1 toast capped at 42 columns and 1 line of message; standard displays ($H < 30$) show up to 2 toasts capped at 2 lines; large displays show up to 3 toasts.
  - Added in-place category lifecycle replacement and de-duplication in `state.notify()`: notifications in the same category (e.g. `Playback`, `Download`, `Updates`, `Cache`) or matching title supersede prior in-flight stages and reset the countdown timer rather than spawning redundant stacked cards.
  - Compressed notification copy into terse, high-density phrasing, ensuring status transitions (e.g. playback prep, favorite toggle, cache clear, updates) fit cleanly into single-line messages without multi-row wrapping.
- **OS-Aware Header-Capable Player Feedback**:
  - Replaced hardcoded multi-line player incompatibility warning paragraphs with concise single-line notifications dynamically listing supported header-capable players for the user's OS (`mpv` and `IINA` on macOS; `mpv` on Linux and Windows).
  - Eliminated speculative hardcoded provider hints in playback rejection notifications.
- **Help Modal Alignment and Shortcut Declutter**:
  - Added `help_modal_layout` in `overlay.rs` snapping the Help modal directly to the search bar vertical position (`search_y`), eliminating vertical misalignments where the search bar previously peaked through above or behind the dialog.
  - Added render suppression in `home.rs` ensuring the search bar, landing deck, and suggestions are completely hidden while Help is active.
  - Migrated Help dialog rendering to `ModalFrame` with standard backdrop clearing and `theme.lavender` borders, removing redundant manual block construction.
  - Lowered two-column layout threshold from 102 to 78 columns, enabling clean dual-column layouts on standard 80×24 terminals without scrolling.
  - Streamlined shortcut descriptions: removed non-existent `[s]` subtitles hotkey, corrected `Ctrl+W` label, eliminated duplicate `/list` command in TV mode, and stripped noisy brackets.
  - Pruned dead `Theme.bg` field and obsolete `SlashCommand::PRIMARY` array.
- **Landing Deck Minimalist Declutter (Resume & Favorites)**:
  - Replaced wordy "Continue Watching" label with concise "Resume" (reducing header width from 34 to 24 characters) and restyled tab navigation with discrete keyhint syntax (`[Tab]`).
  - Streamlined in-progress watch metadata from redundant triplets (`S01E01 · 5% · 42m left`) to clean time-to-finish tags (`S01E01 · 42m left`), falling back to percentage only when stream duration is absent.
- **Details Screen UX Polish and Layout Space Reclamation**:
  - Dynamically capped `selector_height` to the item count of visible selector panes, eliminating excessive empty vertical rows when selector panes have few items.
  - Dynamically capped `streams_area` height to the exact settled stream count (`streams_count + 3`), eliminating the massive blank dead space below the stream table on titles with few streams.
  - Formatted Season list items as full descriptive `Season N` labels and Episode list items as `Episode 01 · Title` (or `Episode 01` fallback), ensuring clear context.
  - Left-aligned data cells in the `SIZE` column to start at index 0, aligning the size value (`1.4GB`) directly beneath the `SIZE` column header.
  - Added symmetric 1-character horizontal padding (`Padding::horizontal(1)`) to the streams panel container block, preventing text and resolution badges from touching the outer border lines.
  - Replaced heavy filled `●` focus markers in pane titles with a minimal `›` (single chevron) glyph.
  - Removed noisy pane position counters (`1/4`) from focused pane titles in favor of clean item count badges (`› Audio (4)`).
  - Deduplicated `Audio:` metadata badge from the top header line when dedicated dub/audio selector panes are visible below.
  - Replaced legacy middot (`·`) separators in the header metadata line with clean two-space spacing.
  - Suppressed missing/N/A release years in the metadata badge row.
  - Dropped redundant no-op `highlight_style`/`highlight_symbol` calls from selector `List` widgets.
- **Search Results Layout Top Margin and Metadata Alignment**:
  - Added dedicated top breathing margin (`search_results_layout`) above the search bar on non-compact viewports (`area.height >= 14`), moving the search bar down from row 0 to row 1 to prevent it hugging the top window border.
  - Synchronized mouse click geometry in `handle_home_mouse` with `search_results_layout`, keeping search bar and results list hitboxes aligned.
  - Balanced search result card layout across three vertical rows: Row 1 displays title and resolution badge, Row 2 displays rating, release year, and media type, and Row 3 displays provider badge and genres/loading state.
  - Aligned all 3 rows of text with the 3 rows of the poster placeholder (`╭──────╮` on Row 1, `│ No Art │` on Row 2, `╰──────╯` on Row 3), eliminating the hanging bottom border and empty-line visual disconnect.
### Fixed
- **History and Favorites Search Settle State**:
  - Set `has_search_settled` to `true` when querying `/history`, `/favorites`, and TV `/list`, preventing an empty watch history or favorites list from being permanently trapped in a `Searching for “...”` loading spinner.
  - Suppressed search action pills entirely on empty `/history` and `/favorites` views, cleanly presenting the empty state message without clutter.
  - Shortened and streamlined notification copy across commands (`Unknown command '{cmd}'. Type '/' for list.`), mode switches, playback guards, and settings actions to fit cleanly into compact notification cards.
- **Details Screen Selector Panes Tight Label Highlighting**:
  - Replaced wide full-width block highlight bars across Audio, Seasons, and Episodes selector panes with compact, text-bounded highlight pills, eliminating awkward empty horizontal background strips across the lists.
- **Stream Table Migration to Native Ratatui Table Architecture**:
  - Replaced legacy dual-widget hack (`Paragraph` header + `List` items) with native `ratatui::widgets::Table` and `Row`/`Cell` primitives, guaranteeing 100% mathematical column synchronization between table headers (`RES`, `SIZE`, `MEDIA TAGS`, `SOURCE`, `RELEASE`) and data cells across every terminal dimension tier.
  - Replaced wide string space hacks with declarative `Constraint` column widths (`Length(8)`, `Length(9)`, `Length(18)`, `Length(16)`, `Min(24)`), eliminating column drift and text overlap.
  - Added active-selection awareness to `resolution_badge_spans`: badges render with subtle dark surfaces and theme accent text when unselected, cleanly switching to bold high-contrast foreground badges when the stream row is highlighted. Added automated cross-theme test verifying contrast and legibility across all 9 built-in themes.
  - Filtered out redundant codec and resolution labels in the `SOURCE` column (e.g. `Multi-Res hevc` or `1080p`), falling back to clean provider origin names (`MovieBox CDN`) when no distinct third-party upload source exists.
- **Subtitle Episode Mismatch on MovieBox Series**:
  - `fetch_resource_page` now accepts `season` and `episode` parameters and appends `se=`/`ep=` to the API URL.
  - Added robust string and integer parsing for `se` and `ep` payload values and matched resource items in `episode_streams` and `service.rs` sibling fallback explicitly by `item.se == season && item.ep == episode` rather than unconditionally taking `.first()`, preventing episode 1 resource IDs from being assigned to later episodes.
  - Fixed `playback.rs` using `selected_details.id.value` (root series ID) instead of `state.active_subject_id` (dub-aware active ID) as the subject key for caption requests; for dubbed variants the two diverge, causing a cache miss and wrong-subject subtitle fetch.
  - Updated all call sites (`download.rs`, `requests.rs` prefetch and season-queue spawns, `playback.rs`) to capture and forward the current `selected_season`/`selected_episode` before spawning async subtitle tasks.
  - Users with pre-existing stale subtitle cache entries can purge old entries via Settings → Maintenance → Purge Cache.
- **Search Results Horizontal Margin Alignment**:
  - Aligned search results grid horizontally with `search_bar_area` (`x: chunks[2].x + 2, width: chunks[2].width - 4`), eliminating the awkward 2-character left overhang where result cards previously started flush against the terminal's 0-column border while the search input prompt was indented.
  - Synchronized mouse hitboxes and column detection in `handle_home_mouse` to match the padded results bounds and ignore out-of-bounds clicks in the margin.
- **Subtitle Selection Picker Padding and Geometry Optimization**:
  - Added symmetric 1-character horizontal padding (`Span::raw(" ")`) around raw picker list item spans in `crate::tui::overlay::picker`, ensuring text and active selection pills no longer hug the outer modal borders.
  - Reduced subtitle picker `minimum_width` from `26` to `20` and unified modal titles to `Subtitles`, eliminating the excessive 10-column dead space gap on the right-hand side while cleanly fitting the `"Subtitles · 1/12"` title without truncation.
  - Synchronized `minimum_width` constraints across rendering and mouse hitboxes in `mouse.rs` for player (`10`), theme (`16`), and subtitle (`20`) pickers.
- **Provider Selection Popup UX and Layout Unification**:
  - Replaced manual paragraph iteration in `render_provider_popup` with `ratatui::widgets::List` and canonical `selection_style`, eliminating competing dual-indicator visuals (left bar and background highlight) into a single cohesive selection cue.
  - Added persistent `✓` active provider prefix glyph in `theme.success`, ensuring the currently active source remains visually distinguishable while navigating through options.
  - Added compact key hints footer (`[↑↓] [↵] [Esc]`) anchored to the bottom of the popup with automatic geometry expansion in `provider_popup_bounds`.
  - Styled popup block title with `theme.title` matching modal frame standards and restored unselected item labels to `theme.text`.
- **TUI Declutter and OS/Terminal Native-Feel Adaptation**:
  - Standardized `key_hint` with split styling (`[`/`]` in `theme.overlay0`, key in `theme.shortcut`, label in `theme.subtext1`), improving shortcut readability across all modals and popups.
  - Stripped instructional "Press X" prose from stream failure messages, details error cards, and the update modal.
  - Streamlined home screen bottom bar on compact terminals by dropping redundant mode word labels and keeping clean shortcut glyphs.
  - Trimmed dynamic rotating search placeholder hints to three focused prompts per mode.
  - Added Truecolor detection support for `Konsole` and `xfce4-terminal` via `TERM_PROGRAM`.
  - Added `BACKGROUND` environment variable fallback for light/dark terminal detection.
  - Updated Help modal title from "Help · Streaming Mode" to clean "Help · Streaming".
- **Universal Solid Highlight Bar Selection Overhaul**:
  - Replaced floating cursor and indicator glyphs (`▌`, `>`, `▸`) across all application screens with full-width solid inverted highlight bars (`Modifier::REVERSED` with `theme.highlight` and dark bold text), unifying selection styling across the entire TUI to match the provider selection standard.
  - Upgraded search results cards by eliminating the floating vertical `▌` indicator paragraph and styling the selected card's title with a tight inverted highlight pill (`" " + title + " "` in `Modifier::REVERSED`), stopping exactly where the title text ends rather than stretching across empty card space, preserving rich metadata badge colors on transparent background underneath, and recovering 2 horizontal columns in `item_slot_rects`.
  - Refactored Continue Watching and Favorites landing deck lists to use empty highlight symbols (`""`) with clean 2-space padding, ensuring the active row renders as an uninterrupted solid highlight bar.
  - Converted search autocomplete suggestions to use empty highlight symbols (`""`) with 1-space leading padding, rendering active suggestions as solid highlight bars.
  - Updated all four Media Details panes (Audio, Seasons, Episodes, Streams) to use canonical `overlay::selection_style` with `Modifier::REVERSED` and empty highlight symbols (`""`), eliminating the `▌` glyph and horizontal layout shifts when alternating focus between panes.
  - Unified Provider popup, TV playlist manager, Addons manager, and overlay modal pickers (themes, players, subtitles, browse presets) by removing leading `▌` symbols and rendering clean solid highlight bars.
- **Terminal Color Support and Capability Detection Precision**:
  - Fixed a critical precedence bug in `classify_terminal` where `term == "xterm"` degraded truecolor-capable terminals to 16-color Basic even when `COLORTERM=truecolor` was set.
  - Made `term_program` matching case-insensitive across all checks (`iterm.app`, `hyper`, `tabby`, `wezterm`, `warpterminal`, `warp`, `vscode`, `ghostty`, `konsole`, `xfce4-terminal`, `apple_terminal`).
  - Added truecolor recognition for VTE-based terminals via `VTE_VERSION` (GNOME Terminal, Tilix), Warp via `warp` / `WARP_IS_LOCAL_SHELL_SESSION`, and Alacritty / WezTerm socket and executable environment variables.
  - Corrected Windows fallback in `classify_terminal` to `ColorSupport::Color256` instead of `Truecolor`, preventing legacy Windows ConHost sessions from receiving garbled truecolor escapes while preserving automatic `Truecolor` for Windows Terminal via `WT_SESSION`.
  - Connected `uses_basic_ui` directly to `ColorSupport::Basic`, ensuring all basic and 16-color terminals automatically receive plain borders and simplified ASCII controls.
### Removed
- **Legacy Addon Mode Routing and Keybindings**:
  - Removed `Ctrl+A` shortcut and the bottom bar Addon Mode button.
  - Removed obsolete `Action::ToggleAddonMode`.


### Fixed
- **Notification Toast Title Text Coloring**:
  - Styled notification toast title spans with `badge_style` rather than the default foreground text style, ensuring that Error (red), Warning (yellow), Success (green), and Info (blue) toasts render consistent title colors matching their borders and badges.
## [0.1.18] - 2026-09-06

### Added
- **Minimal Update Available Modal & Indented Release Notes**:
  - Formatted release notes into a clean, scannable indented hierarchy under `Release Notes:` with category badges (`[Added]`, `[Fixed]`) and bullet points, displaying concise feature titles without decorative tree glyphs, block prefixes (`▌`), or multi-line duplicate text walls.
  - Stripped decorative star icons and redundant prompt sentences, keeping the header focused on a clean, high-contrast version diff (`Installed: vX.Y.Z → Latest: vX.Y.Z`).
  - Separated dialog sections with subtle horizontal card divider lines (`─`) and dedicated action button pills (`[u] Update Now`, `[o] Open Release Page`, `[Esc] Dismiss`).
- **Calm, Minimal Self-Updating Progress Dialog**:
  - Redesigned `draw_updating_modal` into a calm, focused card with top and bottom border clearances, eliminating visual crowding against the title and frame.
  - Streamlined the in-flight display into a unified active status line with spinner (`⠋ Downloading MovieBox-Tui v...` / `⠋ Installing MovieBox-Tui v...`) and a cross-platform safety warning (`⚠ Please wait • do not close terminal` / `[!] Please wait - do not close terminal`).
### Fixed
- **Landing Screen ASCII Banner Bleed-Through**:
- **Modal Frame Corner Backdrops & Transparent Halos**:
  - Removed block-level background color overrides from `ModalFrame`, eliminating dark rectangular halos and square pixel spillage around rounded corner glyphs (`╭`, `╮`, `╰`, `╯`) across transparent and custom terminal themes.
- **Update Modal Geometry & Border Clearances**:
  - Widened the modal from 72 to 76 columns to align with standard dialog geometry and provide a generous 3-column safety margin, preventing text lines and bullet points from crowding the outer borders.
- **Search Bar Synthetic Cursor Artifacts**:
  - Removed artificial block glyphs (`▎` / `█`) drawn directly into search input paragraphs, relying on native terminal cursor positioning without visual cursor duplication or blinking redraw churn.
- **README Walkthrough Media Attachment**:
  - Formatted the WebM walkthrough attachment link in `README.md` and `docs/README.md` for native inline playback.
- **CI Performance Benchmark Runner Tolerance**:
  - Bound wall-clock timing assertion tolerances during unoptimized debug test execution in `tests/performance_audit.rs`, eliminating flaky runner noise failures across virtualized CI runners while maintaining release regression bounds.
### Performance
- **Zero-Allocation Text Truncation Pipeline (`src/tui/text.rs`)**:
  - Implemented SIMD `is_ascii()` fast path in `width` and migrated `truncate_width` to `Cow<'a, str>`, eliminating 100% of heap allocations on fitting titles and text spans.
  - Reduced 10,000-operation truncation latency from `5,243.5µs` to `246.2µs` (21.3x faster, `524.3ns -> 24.6ns/op`) under release compiler profile on Apple Silicon.
  - Streamlined `truncate_middle_width` to construct output into a single preallocated buffer, eliminating intermediate `Vec<&str>`, reversal, and concatenation allocations.
- **Table-Lookup Hex Encoding (`src/cache.rs`)**:
  - Replaced 16 dynamic `core::fmt::write` dispatches per MD5 digest with direct 16-byte static lookup table indexing in `md5_hex`.
  - Reduced 10,000-digest hashing latency from `4,597.9µs` to `1,829.8µs` (2.51x faster, `459.8ns -> 183.0ns/op`).
- **Preallocated IPTV M3U Playlist Parser (`src/providers/tv/parser.rs`)**:
  - Added newline-count capacity preallocation to `M3UParser::parse_m3u`, eliminating repeated dynamic vector reallocations during large playlist loads and achieving `352.0µs` parse duration for 500-channel playlists.
- **Buffered Downloader Chunk Write Aggregation (`src/download.rs`)**:
  - Wrapped segment file descriptors in `tokio::io::BufWriter::with_capacity(256 * 1024)`, aggregating incoming 8KB–16KB HTTP response chunks into sequential 256KB disk blocks and eliminating up to 96.8% of unbuffered filesystem write syscalls.
- **TUI Draw Loop Allocation Pruning (`src/tui/screens/home.rs`, `src/tui/screens/details.rs`)**:
  - Eliminated redundant `display_title.clone()` and duplicate Unicode width calculations in search result and landing deck render loops, delivering headless draw latencies of `32.3µs/frame` (80×24), `43.4µs/frame` (120×30), and `67.3µs/frame` (160×40).
## [0.1.17] - 2026-09-06

### Added
- **Anchored Provider Selection Menu**:
  - Replaced immediate mouse cycling on the landing search bar provider badge (`[MovieBox · ^P]`) with a styled popup menu anchored directly beneath the provider badge.
  - Added dedicated keyboard navigation (`↑`/`↓`/`k`/`j`, `Home`/`End`, `Enter`/`Space`, `Esc`) for the provider menu while preserving direct `Ctrl+P` sequential provider cycling across all screens.
  - Implemented shared geometry (`search_bar_provider_pill_rect` and `provider_popup_bounds`) between renderers and mouse hitboxes, ensuring exact alignment and zero geometry divergence.
- **Redesigned High-Contrast Download Bar & Responsive Layout**:
  - Replaced the solid rectangular download gauge with a sleek proportional track (`[━━━━━────]` on modern terminals and `[=====>----]` on basic terminals) with filled accent contrast and dimmed surface rail.
  - Added prominent media title indicators on the top border (`⬇ Downloading: <Title>` or `⬇ S<N>E<N> (<current>/<total>): <Title>`) with bold styling and automatic terminal width truncation.
  - Replaced ambiguous whole-area click cancellation with an isolated `[x] Cancel` button hitbox on the top-right border, preventing accidental download interruptions while keeping the bar mouse-safe.
  - Formatted transfer statistics into clean badges (`<Size> | <Speed> | ETA <Time>`) with zero unclosed parenthesis artifacts and zero floating dots.
- **Details Screen UX & UI Deduplication**:
  - Replaced repetitive audio language strings in the metadata header with a concise summary badge (`N Audio Tracks`), reserving line space for genre tags and IMDb ratings.
  - Eliminated the redundant workflow breadcrumb bar on wide/desktop layouts where selector columns are already visible side-by-side, reclaiming a vertical display row to show more episodes without scrolling.
  - Standardized selector list typography and cursor alignment, eliminating irregular bullet padding and double-space indentation across Audio, Season, and Episode items.
  - Deduplicated stream table columns by showing clean CDN/provider origins under `SOURCE` and stripping redundant resolution/codec suffixes from the `RELEASE` column.
  - Added count context to pane titles (`Audio (N)`, `Seasons (N)`, `Episodes (N)`), providing immediate visibility into available content quantities.
- **Native Android ARM64 Release Target & Pipeline**:
  - Added native `aarch64-linux-android` build target to the release workflow (`.github/workflows/release.yml`) using Android NDK r26d and Clang (API 24+).
  - Configured automated packaging of `MovieBox_Android_arm64.tar.gz` with native Bionic dynamic linking (`libc.so`), valid ELF `PT_PHDR` program header table, and `/system/bin/linker64` dynamic loader.
  - Added Android ELF header validation in CI to verify `PT_PHDR` and `/system/bin/linker64` presence, preventing `Could not find a PHDR: broken executable?` aborts on Android devices.
  - Updated universal installer script (`install.sh`) to detect Android Termux on 64-bit ARM and automatically fetch `MovieBox_Android_arm64.tar.gz` with verified SHA256 checksums, enabling 1-second native installation without on-device compilation.
- **Native mdBook & GitHub Pages Documentation Architecture**:
  - Integrated `mdBook` documentation engine reading directly from canonical `docs/*.md` guides with zero duplicated markdown files and zero third-party web frameworks.
  - Added `docs/SUMMARY.md` defining table-of-contents chapter navigation across all guides, architecture diagrams, and operational workflows.
  - Added `docs/installation.md` detailing complete installation instructions for macOS (curl script and Homebrew tap), Linux, Windows (PowerShell), Android (Termux), Cargo, and source builds.
  - Formatted `docs/README.md` as the book's introductory landing page featuring core capabilities, prerequisites player matrix, and the live terminal demonstration video streamed from the GitHub CDN.
  - Added minimal root `book.toml` with `navy` dark theme, collapsible sidebar navigation, and client-side full-text search.
  - Added automated GitHub Actions deployment workflow (`.github/workflows/pages.yml`) publishing the documentation site to GitHub Pages on every push to `main`.
  - Added documentation build integrity validation step in CI hygiene pipeline (`.github/workflows/ci.yml`).


### Changed
- **Codebase Cleanups, Dead Code Removal & Shared Logic Centralization**:
  - Removed obsolete compatibility forwarders and dead code (`render_favorites_landing`, `remove_last_grapheme`, `pad_to_width`, `ctrl_key`, and unused shortcut constants).
  - Centralized `theme_color` in `src/tui/theme.rs`, eliminating duplicate definitions across widgets and screens.
  - Centralized favorite status lookups for active details into `AppState::is_selected_details_favorited`, unifying 30-line duplicate calculations in details screen rendering and mouse hitbox detection.
  - Unified subtitle picker label formatting (`"None"` -> `"No subtitles"`, language sanitization) into `format_subtitle_label`.
  - Consolidated details pane border, title, and selection styling through `pane_styles`, eliminating over 50 lines of duplicate style branching.
  - Streamlined details footer action definitions into declarative primary/secondary group builders, cutting redundant code blocks while preserving exact keybindings and layout.
  - Hardened cross-platform process spawning by unifying the Windows `CREATE_NO_WINDOW` (`0x08000000`) flag into a canonical constant in `player.rs`.
  - Decoupled network URL validation by moving `is_http_url` to `crate::net`, eliminating backend network provider dependencies on the TUI text formatting module.

### Fixed
- **Global Modal Background Unfocus & Dimming**:
  - Automatically unfocused and dimmed all background components across Details, Home, and Runner views whenever any modal or popup dialog is active, replacing bright active borders, focus bullets, and selection highlight rectangles with dimmed styling (`theme.muted`).
  - Dimmed background resolution badges (`Multi`, `4K`, `1080p`, etc.) and provider origin tags to muted styling (`theme.muted` on surface backgrounds) during active modal popups, preventing neon badge colors from competing with foreground dialogs.
  - Dimmed unselected list items across Audio dubs, Seasons, Episodes, and Search Results to `theme.muted`, eliminating bright white text bleeds in the background.
  - Dimmed background footer shortcuts and download bar elements during open dialogs, ensuring visual focus remains strictly on the active foreground popup.
  - Suppressed terminal graphics protocol rendering for background posters when a modal dialog is open, preventing image pixels from bleeding over foreground confirmation dialogs.
- **Stream Table Source Column Resolution**:
  - Prioritized specific mirror and uploader labels (`file.source_label()`) in the stream table `SOURCE` column before falling back to generic provider names, displaying actual source tags (`Pahe.in`, `PSA`, `NF`, `GalaxyRG`, etc.) when present while retaining provider fallbacks for direct CDN streams.
- **MovieBox DASH Progress Normalization, Throttling & Background Continuity**:
  - Normalized multi-stream MPEG-DASH download percentages across video (0–90%), audio (90–98%), and merger (99–100%) stages, eliminating progress resets back to 0% when the video stream finishes and the audio stream begins.
  - Enforced monotonic progress tracking and throttled progress event emissions to 250ms intervals, eliminating terminal text jitter and channel saturation.
  - Preserved active downloads and queue processing across content provider switching (`Ctrl+P`) and mode toggling (`Ctrl+T`, `Ctrl+A`), eliminating premature download pauses and false cancellation warnings when navigating the TUI.
  - Added explicit cancellation feedback notifications when dismissing the subtitle selection popup via `Esc` or outside mouse click, preventing silent stream launch cancellations.
- **Empty Search Clear Guarding on Landing Screen**:
  - Guarded search-cleared status notifications in `Action::GoBack`, `c`/`C`, and `Ctrl+U` to only fire when an active search query or loaded results actually existed, eliminating spurious "Search cleared." status messages when navigating on an already-empty landing page.
  - Allowed pressing Enter on an empty search input in editing mode to cleanly switch back to normal mode without triggering unneeded state resets.
- **MovieBox DASH Stream Download Engine & Header Authentication**:
  - Forwarded mirror authentication headers (`Cookie` containing CloudFront signed policy and `Referer`) through `Action::StartDownload` and `start_resilient_download`, eliminating `HTTP 403 Forbidden` errors on MovieBox CDN downloads.
  - Added dedicated MPEG-DASH stream engine utilizing `yt-dlp` to assemble multi-track audio/video manifests (`index.mpd`) into `.mp4`, maintaining full feature parity with progressive single-file downloads.
  - Integrated real-time child process progress parsing (`parse_ytdlp_progress`), reporting live percentage, speed, and ETA metrics to the TUI status bar.
  - Implemented dynamic, OS-tailored installation guidance for `yt-dlp` and `ffmpeg` when missing from the host system (Homebrew on macOS, package managers on Linux, WinGet on Windows, and Termux `pkg install yt-dlp ffmpeg` on Android).
  - Hardened Windows background process spawning with `CREATE_NO_WINDOW` and added cross-platform path resolution fallbacks for macOS Homebrew, Nix, and Termux environments.
  - Preserved external subtitle sidecar retrieval alongside DASH video downloads, maintaining clean `<base_dir>/Movies/<Title>/` and `<base_dir>/Series/<Title>/Season <N>/` directory hierarchy.
- **Series Season Default & Episode List Rendering Normalization**:
  - Initialized unselected series search results with `season: 0`, preventing catalog season counts from masquerading as watch history progress and erroneously defaulting multi-season shows (e.g. *Breaking Bad*) to their final season on initial selection.
  - Guarded history pre-seeding in search submission strictly to `/history` queries and active continue-watching items, ensuring search results always open at Season 1 Episode 1 for new series while resuming at the user's progress for previously watched shows.
  - Authoritatively resolved target season and episode in `DetailsSuccess`, preserving in-view season and episode positions across audio dub switches while defaulting fresh series navigation to Season 1 Episode 1.
  - Replaced `"·  "` unwatched episode status prefix with clean whitespace padding, eliminating double middots (`· ·  EP 01`) and stray dots before unselected episodes in the details pane.
  - Guarded duration formatting against empty strings in `MediaDetails`, eliminating duplicate bullet dividers (`·  ·`) in the metadata header for series without runtime durations.
- **Accurate Playback & Download Preparation Notifications**:
  - Replaced misleading "Fetching subtitles" toast notifications during stream and episode download preparation with accurate stream preparation notices (`Preparing <filename>...` and `Resolving episode stream...`), eliminating false subtitle retrieval messages when streams rely on embedded subtitles or contain no external captions.
  - Clarified external subtitle download failure status to `External subtitle unavailable; playing stream directly.` to prevent ambiguity when video containers carry built-in subtitles.
  - Standardized download mirror error notice to `No downloadable mirrors were found for this release.` and normalized notification title casing.
- **MovieBox Subtitle Resolution & High-Speed Multi-Tier Aggregation**:
  - Implemented multi-tier subtitle resolution in `MovieBoxService::get_ext_captions` with immediate early-exit (< 250ms) when the active stream provides rich subtitles (≥ 5 tracks), eliminating 15–20 superfluous sibling network round-trips that previously triggered 15-second playback resolution timeouts.
  - Replaced sequential nested loops for sibling dub crawls with concurrent parallel dispatch via `futures::future::join_all`, bounding multi-dub subtitle aggregation (e.g. *Ek Deewane Ki Deewaniyat*) to < 1.2s.
  - Added asynchronous background subtitle cache pre-warming in `Action::EpisodeStreamsReady`, fetching and caching subtitle options as soon as streams are received so that pressing Enter on a stream yields an instant < 1ms cache hit.
  - Resolved full multi-language subtitle availability (English, Bengali, Arabic, Chinese, Filipino, French, Hindi, Indonesian, Malay, Portuguese, Punjabi, Russian, Urdu) on MovieBox by linking genuine upload `resourceId` identifiers to releases instead of internal CDN transcoding stream IDs.
  - Filtered out 34-byte dummy placeholder caption files returned by transcoding endpoints and deduplicated subtitle tracks by language and download URL.
  - Added automatic fallback to subject resources in `MovieBoxService::get_ext_captions` when a stream ID does not directly attach captions.
  - Added `in_id` language code mapping to `sanitize_language_label` for localized Indonesian subtitle display.
  - Registered `draw_subtitle_picker` in `App::draw`, restoring the visual "Subtitles" modal picker overlay during stream playback and download preparation when external captions exist.
  - Tightened modal picker vertical height calculation in `picker_layout`, eliminating blank gap lines between the last list item and the bottom divider for short lists.
  - Hardened `Action::PlayStream` error handling with structured diagnostic warnings on subtitle timeout or resolution failure before dispatching direct playback.
- **Termux Android Player Exit Code 126 & Intent Bridge Resolution**:
  - Eliminated `Player Error: Crash code: 126 (/system/bin/am[11]: /data/data/com.termux/files/usr/bin/cmd: Permission denied)` crash in Termux on Android 10+ by prioritizing native Termux openers (`termux-open`, `termux-open-url`, `termux-am`) and strictly avoiding unprivileged `/system/bin/am` shell script calls.
  - Preserved `LD_PRELOAD` for Termux applet compatibility while prepending system paths (`/system/bin:/system/xbin`) for system command invocations.
  - Added actionable diagnostic notifications when player execution fails or when no player is detected in Termux, directing users to install `termux-tools termux-am` and verify an external Android player (VLC, Just Player, MX Player).
  - Updated universal installer (`install.sh`), README, and documentation guides with `termux-am` prerequisites and architecture details explaining external Android video player integration versus headless CLI `mpv`.
- **Android Intent Player Stream Compatibility**:
  - Eliminated blanket player incompatibility errors on Android: unauthenticated streams (CircleFTP, DhakaFlix, IPTV, direct streams) and streams carrying standard `Referer`/`User-Agent` headers (4KHDHub) now dispatch directly to Android video players via `termux-open` or `am start`.
  - Added empty-header guard to `supports_headers`, ensuring streams without authentication requirements are never falsely rejected as incompatible.
  - Forwarded `User-Agent`, `Referer`, and subtitles (`subtitles_location` and `subs`) as intent extras when dispatching playback via `am` or `termux-am`.
  - Prevented circular provider switch prompts when playing 4KHDHub streams by tailoring notification hints based on the active provider.
- **Documentation Mobile Layout & Typesetting Normalization**:
  - Replaced unparsed LaTeX syntax (`$\to$`, `$\ge$`, `$\mu\text{s}$`, `$N\times$`) across all documentation guides with standard Unicode characters (`→`, `≥`, `µs`, `N×`), eliminating raw unrendered markup in mdBook output.
  - Added responsive documentation stylesheet (`docs/custom.css`) integrated via `book.toml`, enabling smooth touch horizontal scrolling, compact cell padding, visible scrollbars, and dynamic code text wrapping across mobile and small screen viewports.
  - Formatted provider matrix table in `docs/providers.md` with explicit column alignments and bold provider labels to optimize scannability on narrow screens.

### Changed
- **Documentation Readability & Typography Polish**:
  - Expanded reading container width (`--content-max-width: 860px`) and relaxed line-height (`1.62em`) with vertical list spacing (`0.45em`) in `docs/custom.css`.
  - Added theme-adaptive inline code badge containers (`:not(pre) > code`) with bordered backgrounds, and framed `<pre>` code blocks with rounded corners and drop shadows.
  - Added table row hover transitions and container borders across configuration, platform, and player documentation tables.
  - Implemented responsive mobile header title scaling (`.menu-title`) and smooth anchor navigation scrolling (`scroll-behavior: smooth`).
## [0.1.16] - 2026-09-05

### Added
- **Home Landing Deck Continue Watching & Multi-Tab Navigation**:
  - Implemented interactive multi-tab landing deck on the Home screen supporting both `Continue Watching` and `Favorites`.
  - Added seamless `Tab` and `Shift+Tab` keyboard cycling between Continue Watching and Favorites tabs with instant row focus retention.
  - Added one-click/key direct resume (`Enter`, `Space`, or `P`) on Continue Watching items, automatically configuring episode advancement, season positioning, and auto-play in Details view.
  - Formatted Continue Watching rows with title truncation, series episode badge (`S01E03`), progress percentage, and remaining duration (`45% · 24m left`).
  - Added mouse support for clicking the landing deck header bar to switch tabs, and double-clicking items to play/open.
- **Windows TrueColor & High-Contrast Terminal Theming**:
  - Enabled 24-bit TrueColor auto-detection by default on Windows 10/11 (`conhost.exe`, Windows Terminal, PowerShell, CMD), ensuring Windows users receive rich Catppuccin themes out of the box.
  - Overhauled 16-color ANSI dark fallback palette (`Theme::fallback`), replacing low-contrast dark blue and magenta with high-contrast cyan accents for borders, titles, headers, and highlights.
  - Added background row selection suppression in Settings Hub when modal popups (media player picker, theme picker, download directory input) are active, directing 100% of user focus to floating dialogs.
  - Clamped popup picker minimum height to 7 rows, eliminating visual crowding on single-item selections.
  - Elevated active selection surface styling to `theme.surface1` with accent-highlighted cursor indicators (`▌ ` / `▸ `) across lists and settings rows.
- **In-App Self-Update Engine Hardening**:
  - Added deterministic fallback download URL generation for GitHub release assets and `SHA256SUMS` when unauthenticated API requests encounter HTTP 403 rate limits.
  - Added active in-flight self-update progress modal (`draw_updating_modal`) featuring animated Braille spinners, version upgrade indicators (`v{old} → v{new}`), and real-time status steps.
  - Added environment-aware update modal actions: displays Homebrew upgrade instructions (`brew upgrade moviebox-tui`) with `[b]` shortcut on Homebrew installations, Termux installer guidance on Android, and package manager notifications on read-only installations.
  - Added 5-iteration retry loop with bounded 1-second backoff in the Windows update helper script (`moviebox_update_helper.bat`) to tolerate transient file locks from antivirus or Windows search indexers during binary replacement.
  - Cached full `Release` metadata in `AppState` and action pipeline, eliminating duplicate network queries between release checking and self-update invocation.
- **Empirical Performance Standards & Benchmark Testing Guidelines**:
  - Added strict performance verification standards requiring empirical before-and-after measurements (runtime latency, allocations, frame render latency, I/O syscalls, binary footprint) across hot paths.
  - Documented standardized performance benchmark reporting format in `docs/testing.md` for reproducible optimization audits.
- **Centralized Poster Placeholder & UI Animation Widgets**:
  - Extracted reusable `render_poster_placeholder` widget to `src/tui/widgets/poster.rs`, standardizing placeholder containers, loading dots, and geometry clamping across Home and Details screens.
  - Centralized `loading_spinner` in `src/tui/widgets`, providing uniform ASCII fallback (`..`, `...`) on basic terminals and animated Braille frames on modern terminals.
  - Added zero-allocation cursor helpers (`cursor_prefix_str`, `cursor_column_offset`, `cursor_split_parts`) to `TextInputBuffer`, replacing dynamic vector and string allocations with zero-copy slices during typing and cursor blinking.
  - Added `step_list_selection` to `src/tui/state.rs`, centralizing bounds-safe list stepping for PageUp and PageDown navigation across browse, theme, and favorites lists.
  - Added `clear_poster_cache` and `clear_poster_protocols` to `AppState`, guaranteeing consistent flushing of in-flight requests, LRU image handles, and terminal protocols across provider switches and search resets.
### Removed
- **Poster Graphics Configuration & Halfblocks Engine**:
  - Removed Unicode Halfblocks poster engine (`▀`/`▄`), eliminating low-resolution cell distortion, font scanlines, and terminal redraw lag during list scrolling.
  - Removed redundant `Poster Graphics` toggle from Settings Hub (`/settings` → Appearance) and `config.json`, delegating terminal graphics strictly to automatic native GPU protocol detection (Kitty, Sixel, iTerm2).

### Fixed
- **Cross-Platform Handle Safety & Silent Failure Elimination**:
  - Fixed Windows file sharing violation (`ERROR_SHARING_VIOLATION`) in multi-segment download assembly by explicitly flushing, syncing, and dropping the file write handle before executing destination renames.
  - Added overwrite handling on destination collisions during download finalization on Windows, preventing failed renames on re-downloaded media.
  - Guarded `FavoritesManager::load_from_path` against premature corrupt file rotation on transient read errors, matching history and configuration persistence invariants.
  - Hardened Lua tracker script and state file directory initialization to return `None` on directory creation or write failures rather than passing non-existent paths to media players.
  - Handled web browser launch failures in Settings Hub (`open::that`), logging warnings and displaying the repository URL on headless or restricted environments.
  - Added `CREATE_NO_WINDOW` flag (`0x08000000`) to the Windows update helper process spawn to eliminate console window flashes during in-app updates.
- **Resilient Cross-Platform Cache Clearing & In-Flight Task Isolation**:
  - Hardened `clear_all_cache` with recursive directory contents deletion, leaving the root directory node intact to prevent `ERROR_ACCESS_DENIED` and `ERROR_SHARING_VIOLATION` failures when Windows processes or shells hold folder handles.
  - Added Windows read-only attribute clearing before unlinking locked files.
  - Included external Android subtitle cache directory (`~/storage/downloads/moviebox_subs`) and temporary system subtitle caches in the cache purge sequence.
  - Connected `Action::ClearCache` directly to `self.request_tasks.cancel_all()` and added cancellation guards in `spawn_search_posters`, preventing in-flight background requests from writing stale responses or posters immediately after cache clearance.
  - Propagated concrete filesystem `Result<(), String>` to `Action::CacheCleared`, replacing hardcoded success notifications with real error reporting.
- **Unified Search Result Selection Background**:
  - Unified search result card selection highlight across the entire item slot (`item_area`), eliminating fragmented background rendering between cursor indicators, posters, and text columns.
  - Removed redundant poster sub-area buffer clearing before image rendering, preventing selection background clipping and black gutter artifacts on the right edge of posters.
- **High-Precision Playback Tracking & Race Elimination**:
  - Eliminated wall-clock race condition where process elapsed time overwrote exact seek/pause positions from `mpv` and `iina-cli` Lua trackers.
  - Hardened `moviebox_tracker.lua` with latched completion: reaching $\ge 90\%$ playback or EOF permanently latches completion, preventing shutdown events from reverting completed status.
  - Implemented atomic state file persistence in Lua using temporary files (`.tmp`) and clean destination replacement for Windows and Unix platforms.
  - Handled unknown stream durations by writing JSON `null`, preventing zero-duration calculations.
  - Pre-registered media playback on launch (`record_start`), ensuring immediate watch history persistence for Android intent dispatchers (`termux-open`, `am start`) and app fallbacks.
  - Added self-healing recovery in `reconcile_from_dir`: pre-seeded pending state files carry metadata (`title`, `cover_url`, `stype`, `release_year`), restoring new items into watch history even after sudden terminal exits or reboots.
  - Implemented smart series episode advancement: completing an episode automatically cues the next episode (`episode + 1` or next season) on resume (`Space`/`P`) and in the Details view.
- **Details View Empty Stream Source Label Geometry**:
  - Replaced verbose empty stream message with a compact string (`No stream sources found on {provider} (Ctrl+P to switch provider, r to retry)`), preventing awkward multi-line text wrapping on standard 80-column terminals.
- **Standardized 'No Art' Poster Containers Across Terminals**:
  - Replaced robot eyes and broken infinite loading spinners with clean, static, centered `No Art` bordered blocks across search results and details screens on terminals without graphics support.
  - Preserved full-fidelity native GPU graphics rendering on supported terminals while standardizing card geometry and poster container boundaries across all platforms.
  - Gated background poster network requests and CPU image decoding strictly behind `image_supported`, eliminating redundant network bandwidth and CPU cycles on standard terminals.

### Changed
- **Provider Switching Shortcut**:
  - Scoped `Ctrl+P` strictly to Streaming Mode for provider cycling, eliminating redundant `Ctrl+P` handling in TV and Addon modes.
  - Streamlined `/config` as a direct alias for `/settings`.

- **Pruned Redundant Theme Slash Command**:
  - Removed standalone `/theme` slash command, parser routing, and auto-suggestions; theme selection and visual palette swatches are managed directly within the interactive Settings Hub (`/settings` → Appearance → Theme).
- **Discover Categories Landing Card UX**:
- **Clean Segmented Landing Deck Header Styling**:
  - Replaced crowded decorative star (`★`) and bracket (`[ ]`) glyphs with a clean, segmented tab bar header (`Continue Watching │ Favorites (Tab)`).
  - Streamlined overflow row formatting to centered minimalist pill indicators (`+N more · /history` and `+N more · /favorites`).
  - Streamlined `Discover & Quick Categories` card: elevated `/browse` command to a right-aligned header badge (`[ /browse ]`), eliminated redundant `/browse ·` row prefixes, and adapted category rows dynamically between Streaming and Addon modes.
  - Added direct mouse click navigation to discover categories, routing clicks to preset browse queries or the addon catalog menu.
- **Command Dispatch & Cache Lookup Optimization**:
  - Unified `ParsedCommand` and `SlashCommand` into a single canonical enum, eliminating duplicate type definitions across command dispatch and testing.
  - Streamlined image disk cache lookups (`get_namespaced_image_cache`), eliminating 6-iteration fallback namespace scans across unrelated provider directories on cache misses.
  - Replaced duplicate `is_termux_env` in player module with `crate::updater::artifact::is_termux_environment`.
- **Automation Workflows & Installer Hardening**:
  - Added bounded execution timeouts (`timeout-minutes`) across all CI and release pipeline jobs to prevent runner hangs.
  - Accelerated release preflight by eliminating redundant cross-compilation target toolchain downloads during source packaging.
  - Streamlined Homebrew formula updater (`homebrew.yml`) by parsing the release's attested `SHA256SUMS` manifest directly with strict 64-char hex validation, eliminating redundant ~50MB archive downloads.
  - Added on-device binary execution smoke tests to `install.ps1` and enhanced `install.sh` error diagnostics with API version resolution fallbacks.
- **Documentation & User Guide Streamlining**:
  - Overhauled root `README.md` into a developer-focused technical guide, pruning marketing copy, redundant comparison tables, and promotional buzzwords.
  - Added structured media player setup guide with package manager commands (`brew`, `apt`, `winget`).
  - Updated macOS Homebrew installation with explicit `brew trust` step required by Homebrew 6.0+ for third-party taps.
  - Streamlined quickstart section to reference in-app interactive help (`?`) and `docs/controls.md`, preventing documentation drift.
- **Streamlined Test Architecture & High-Signal Test Suite**:
  - Consolidated unit assertions for file stem sanitization, MD5 hashing, atomic writing, and badge rendering directly into their respective modules (`src/download.rs`, `src/cache.rs`, `src/tui/widgets/badge.rs`).
  - Pruned 10 redundant, weightless, and duplicate test suites (`player_integration.rs`, `url_security.rs`, `download_integration.rs`, `m3u_integration.rs`, `cache_lifecycle.rs`, `real_acceptance.rs`, `grand_user_journey.rs`, `history_reconciliation.rs`, `version_upgrade_e2e.rs`), reducing integration test files from 20 to 9 focused suites while preserving complete regression coverage.
  - Hardened `.omp/AGENTS.md` and `docs/testing.md` with strict engineering rules rejecting weightless tests, duplicate test layers, and monolithic multi-phase journey tests.
  - Aligned update modal keyboard tests with modal input isolation, verifying keystrokes do not fall through to background search results while the dialog is active.
### Fixed
- **Responsive Text Sizing & Layout Truncation**:
  - Raised Details footer split threshold (`DETAILS_FOOTER_SPLIT_THRESHOLD`) to 106 columns, ensuring shortcuts use a clean 2-row layout on terminals between 80 and 105 columns without clipping.
  - Omitted `[Ctrl+P] Provider` hint from the Details footer when in Addon mode or viewing addon streams, recovering 18 columns of footer space.
  - Added line-width budgeting to Details metadata: audios and extra metadata (Genre, Director, Cast) now dynamically truncate to available row width without wrapping beyond their allocated lines.
  - Synchronized Settings tab hit-testing (`category_tab_rects`) with compact rendering (`popup_area.width < 58`), resolving mouse click target divergence on compact terminals.
  - Added dynamic subtext and label truncation in Settings rows, preventing 2-row Paragraph wrapping and off-screen line displacement.
  - Replaced hardcoded path truncation in Settings download directory with dynamic middle truncation (`truncate_middle_width`), maximizing displayed path length while fitting compact rows.
  - Added dynamic description truncation to Home discover categories card, preventing overflow on narrow (50–54 col) screens.
  - Added responsive compact labels (`[Try Provider (P)]` and `[Clear (c)]`) and narrowed separation to No-Results buttons when terminal width is under 56 columns.
  - Removed obsolete `Ctrl+P` hint from search bar mode pill in Addon mode, replacing it with `[Addon Mode]`.
  - Added available width clamping to non-landing search bar placeholders and status messages to prevent 1-row Paragraph wrapping.
  - Omitted redundant media type separator in search results metadata when terminal width is under 36 columns and release year is present, keeping provider badges intact.
  - Budgeted list entry lengths in TV Playlists and Addon Manager popups to fit within inner popup bounds regardless of installed badge counts.
  - Raised Help menu two-column threshold to 102 columns to prevent keybinding description truncation.
  - Added compact title formatting to download gauge when width is under 60 columns.
  - Added compact button formatting (`[u] Update  [o] Web  [Esc] Back`) to Update Modal on terminals under 60 columns.
  - Added compact header formatting to Updating progress modal when inner width is under 42 columns.
  - Added title boundary protection to `ModalFrame`, truncating overly long titles to preserve border integrity.
  - Eliminated dark rectangular halo artifact around modal popups by removing block-level background color overrides from `ModalFrame`, allowing rounded borders to cleanly render against transparent and custom terminal backgrounds without pixel spillage.
- **Resilient Configuration & Accurate Metadata**:
  - Safeguarded user TV playlists (`tv_config.json`) and HTTP addons (`addons_config.json`): replaced destructive error-swallowing deletion with timestamped `.corrupt.{timestamp}` file rotation and sanitized logging on JSON parse failures.
  - Eliminated fabricated S01E01 fallback in addon metadata adapter, accurately reporting empty series episodes when an upstream catalog entry lacks episode records instead of injecting unplayable dummy data.
- **Discover Card Layout & Truncation**:
  - Fixed horizontal text clipping on discover card category descriptions by adjusting `margins_len` to account for visual pointer and margin cell budgets.
  - Suppressed discover card rendering while search suggestions dropdown is open, preventing visual overlap.
- **Update Modal Input Isolation & Event Guards**:
  - Prevented keystroke hijacking: deferred blocking update modal presentation while the user is actively typing in the search bar (`InputMode::Editing`), ensuring keys (`u`, `o`, `Esc`) never trigger unintended update actions.
  - Added input lock during in-flight updates (`is_updating`), consuming all keyboard and mouse events to prevent mid-upgrade process termination or disk corruption.
- **Comprehensive Multi-Platform Player Detection & Dynamic Settings Refresh**:
  - Expanded Windows MPV and VLC candidate discovery across executable-adjacent directories (`.\mpv.exe`, `.\vlc.exe`), WinGet Packages (`%LOCALAPPDATA%\Microsoft\WinGet\Packages`), user `Downloads` and `Desktop` extractions, `mpv.net` (`mpvnet.exe`, `mpv.net`), `mpv.com`, Scoop apps and shims, Chocolatey, portable drive roots (`C:\mpv`, `C:\vlc`, `C:\tools`), and Windows Registry `App Paths` and `Environment\Path`.
  - Expanded macOS and Linux discovery across Nix profiles (`~/.nix-profile/bin`, `/run/current-system/sw/bin`), Homebrew, MacPorts, user `.local/bin`, and user/system Flatpak exports.
  - Replaced permanent negative caching (`OnceLock<Option<String>>`) with non-negative path caching across MPV, VLC, IINA, and Android Intent openers, ensuring players installed after cold start are discovered immediately.
  - Added non-destructive dynamic player detection merging to Settings Hub (`ToggleSettingsPopup`, `ShowSettingsPopup`), player selection activation, and value cycling, refreshing `available_players` in real time without requiring an app restart.
- **Direct Playback & Header Compatibility**:
  - Eliminated vestigial in-stream "Open with" popup that blocked playback when only VLC was installed, routing playback directly to the preferred compatible player.
  - Prevented silent player overrides: when a user explicitly selects a default player (e.g. VLC) that cannot satisfy stream authentication headers (e.g. MovieBox signed DASH manifests), the app now halts playback and warns the user with actionable detected alternatives instead of silently launching an unselected player.
  - Added structured `PlaybackResolution` engine with dynamic context-aware notifications across playback resolution and download operations.
- **Terminal Graphics Probe Leak**:
  - Prevented raw Kitty APC escape sequence leak (`Gi=31...`) on macOS `Terminal.app` and legacy non-graphics consoles by skipping graphics stdio probes.
  - Removed unsafe mid-session stdio graphics re-probing on `FocusChange` events.

- **High-Performance Player Detection Engine (`src/player.rs`)**:
  - Centralized OS executable probing (`mpv`, `vlc`, `IINA`) into a single `probe_player_executable` engine, stripping ~150 lines of duplicate path traversal.
  - Added static caching (`OnceLock`) to `IINA` resolution and Android Termux Intent detection, eliminating repeated expensive filesystem IO and PATH lookups during playback launches.
  - Expanded candidate resolution: added macOS MacPorts (`/opt/local/bin/*`) and standard `/bin/*` locations.
  - Aligned installer (`install.sh`, `install.ps1`) player detection with the app engine, explicitly probing standard `/Applications/*.app` and `C:\Program Files` deployments so GUI installations are correctly discovered immediately post-install.
- **Responsive Mobile Installer Headers (`install.sh` & `install.ps1`)**:
  - Implemented dynamic terminal column detection (`tput cols`, `stty size`, and `$COLUMNS` in `install.sh`; `$Host.UI.RawUI.WindowSize.Width` in `install.ps1`) with automatic multi-tier banner sizing.
  - Eliminated ASCII art banner wrapping and visual corruption on narrow mobile viewports (e.g. Android Termux portrait mode at 40–55 columns) by rendering an adaptive 31-column compact half-block banner (`█▀▄▀█...`) and dynamic horizontal centering.
- **Android Termux Static-PIE & TLS Alignment**:
  - Linked `aarch64-unknown-linux-musl` target as static-PIE (`-C relocation-model=pic -C link-arg=-pie`) to produce `ET_DYN` (ELF `e_type: 0x0003`) binaries accepted by Android Bionic's `/system/bin/linker64`, resolving runtime failure (`unexpected e_type: 2`).
  - Added 64-byte `PT_TLS` alignment anchor in `src/main.rs` and post-build ELF program header alignment in `.github/workflows/release.yml` to satisfy Android Bionic's ARM64 TLS segment minimum alignment validation.
  - Added automated ELF `e_type` and `PT_TLS` validation checks to `.github/workflows/release.yml` and a post-installation execution smoke test to `install.sh`.
### Removed
- Pruned unused legacy type aliases (`SeasonInfo`, `EpisodeInfo`, `StreamResource`, `StreamMirror`) in `src/models.rs`.
- Removed dead `util_row` struct field from `LandingRows` in `src/tui/screens/home.rs`.
- Removed vestigial `ShowPlaybackPicker` and `ShowPlayerPicker` action variants and playback picker state fields in favor of direct compatible player dispatch.
- Removed unreferenced static screenshot assets (`assets/`), reducing repository clone size by ~1.3MB.
- **Redundant Slash Commands**:
  - Pruned 10+ legacy slash commands (`/download-dir`, `/clear-cache`, `/update`, `/github`, `/probe`, `/toggle-update`, `/toggle-bdix`, `/toggle-streaming`, `/toggle-tv`, `/toggle-addons`, `/enable-*`, `/disable-*`) superseded by the interactive Settings Hub.
  - Removed 300+ lines of redundant command execution and file write probing in `src/tui/app/search.rs`.
  - Removed duplicate unreachable `Ctrl+U` key handling in `src/tui/app/keyboard.rs`.
## [0.1.15] - 2026-09-03

### Added
- **MovieBox MPEG-DASH Stream Playback**:
  - Implemented visitor login authentication with JWT session tracking and atomic disk persistence (`~/.cache/moviebox-tui/moviebox_session.bin`).
  - Added CloudFront DASH manifest resolution (`index.mpd`) labeled with `[Multi]` badge in stream listings for multi-resolution playback (`1080p`, `720p`, `480p`).
  - Forwarded authentication headers and mobile `User-Agent` to `mpv`, `IINA`, and `VLC` for authenticated CloudFront DASH demuxing.
- **Seekable 4KHDHub Stream Resolution**:
  - Prioritized seekable Cloudflare R2, S3, and FSL stream mirrors (`HTTP 206 Partial Content`) over attachment downloads.
  - Added automatic unwrapping of base64 `watch-online` redirect URLs and expanded HubCloud button selectors.
  - Concurrently probe resolver mirror candidates in chunks of 3 with a 4-second timeout, filtering expired or broken links.
- **Interactive Settings Hub (`/settings`)**:
  - Added a unified 4-tab modal (`General`, `Content Modes`, `Appearance`, `Maintenance`) for media player selection, download directory configuration, live theme palette preview, and provider toggles.
- **TUI Layout & Ergonomics**:
  - Added `/exit` slash command (with `/quit` and `/q` aliases) to exit and restore the terminal directly from the search prompt.
  - Added direct watch history resume on `Space` / `P` for the recorded season and episode.
  - In-place provider switching (`Ctrl+P`) on the Details screen to re-query titles without returning to the landing screen.
  - Multi-`Esc` navigation: first `Esc` returns focus to the search bar; second `Esc` clears the query and returns to the home landing.
  - Added fluid 10-frame Unicode Braille loading spinners (`⠋⠙⠹...`) with ASCII fallbacks.
  - Grapheme-cluster-safe text input (`TextInputBuffer`) across search, TV playlist, and addon inputs with `Ctrl+W`, `Ctrl+U`, `Delete`, and `Home`/`End` support.
  - Added 4-tier responsive stream table layout and dynamic single-column fallback for narrow terminals (<85 cols).
  - Added Kitty keyboard protocol support (`DISAMBIGUATE_ESCAPE_CODES`, `REPORT_EVENT_TYPES`) for lower latency input on supported terminals.
  - Added terminal theme autodetection using OSC 11 background luminance queries, with `NO_COLOR` override support.

### Changed
- **Unified Favorite Keybinding**: Standardized favorite toggling across all screens exclusively on `f` / `F`.
- **Search & Landing UX**:
  - Anchored search bar at row 0 across empty, loading, and zero-results states to eliminate layout shifting.
  - Structured zero-results and query error screens with actionable guidance shortcuts (`[Ctrl+P] Switch Provider`, `[r] Retry`, `[Esc] Back`).
  - Enclosed landing favorites list in an aligned bordered card.
- **Streams Table Display**:
  - Replaced non-existent stream duration column with source and uploader columns, expanding room for codec and media tags.
  - Grouped secondary stream audio/video codec tags (`DV`, `ATMOS`, `HEVC`) with subtle separator points.
  - Isolated stream row selection highlight so background styling applies exclusively to the active stream row rather than table headers.
- **MovieBox Client Spoofing**: Updated client identity headers to APK `v4.0.01` to ensure backend compatibility and avoid upgrade notice videos.
- **Search Prefetching**: Bounded initial search prefetching to visible viewport bounds to reduce initial network requests.

### Fixed
- **Stream Playback & Resolver State**:
  - Fixed an issue where timed-out or failed stream resolutions left the playback state locked, preventing subsequent playback attempts.
  - Fixed empty stream panel click focusing the pane without inadvertently triggering playback of index 0.
  - Prevented transient "No stream sources found" flash while metadata or streams are loading.
- **Metadata & UI Isolation**:
  - Guarded search preview metadata and details merging with strict provider and item ID checks, preventing stale metadata leakage when navigating quickly.
  - Suppressed Sixel, Kitty, and iTerm2 poster graphics while modal dialogs or search dropdowns are open to prevent graphic bleed-through.
  - Fixed poster placeholder widget incorrectly drawing over loaded images on the results screen.
  - Fixed text wrapping panic on multibyte / CJK characters during title year extraction.
- **Cross-Platform & Installation**:
  - Windows: Fixed self-update batch script losing staged binary on process exit, preserved leading backslashes on UNC paths, and sanitized NTFS forbidden characters in tracker state filenames.
  - Android (Termux): Added fallback directory resolution for config, cache, and data paths, and added architecture guard preventing incompatible glibc Linux ARM64 binaries from overwriting Termux installations.
  - DNS: Replaced hardcoded `/etc/resolv.conf` requirement with custom resolver querying OS DNS first and falling back to public DNS (Cloudflare, Google, Quad9) on zero-config platforms.
  - Installers: Hardened path quoting, signal handling, and TLS 1.2 negotiation in `install.sh` and `install.ps1`.

### Performance
- **MessagePack Binary Disk Cache (`src/cache.rs`)**: Replaced raw JSON disk caching with binary MessagePack serialization (`rmp-serde`) with magic signature `MBC1`, versioned envelopes, and automatic migration from legacy JSON caches.
- **Zero-Copy IPTV M3U Parser (`src/providers/tv/parser.rs`)**: Converted playlist attribute parsing to single-pass slice scanning, eliminating bulk heap allocations on large playlists.
- **Scraper Efficiency (`src/providers/fourkhdhub/`)**: Precompiled static CSS selectors using `LazyLock<Selector>` across scraping paths.
- **Connection Pooling & DNS (`src/net.rs`)**: Shared static DNS resolver cache and tuned HTTP connection pool settings (`tcp_nodelay`, keepalive, idle timeouts).

### Removed
- Removed deprecated JSON conversion adapters and untyped `serde_json::Value` service endpoints in favor of strongly typed domain structs.
- Removed redundant `[o] Open With` and `[s] Subtitles` Details shortcuts in favor of `/settings` media player selection and automatic subtitle loading.
- Removed obsolete `*` favorite shortcut in favor of `f` / `F`.

## [0.1.14] - 2026-08-26

### Added
- **Favorites**:
  - Added a Favorites feature for starring whole movies and series (`src/favorites.rs`, `favorites.json`), independent of watch history and unaffected by `/clear-cache`.
  - Added `*` on the Home screen and `f` / `F` on the Details screen to toggle a title's favorite status, with a `★` indicator on favorited rows and a `[f] Favorite` / `[f] Unfavorite` hint on the Details screen.
  - Added an arrow-navigable Favorites row on the landing screen (Streaming and Addon modes) showing up to 5 recently-starred titles, with a `+N more • /favorites` overflow link; `Down` from the search bar focuses the row, `Enter` opens the selected title, `Esc` releases focus.
  - Added the `/favorites` slash command, mirroring `/history`, to load the full starred list into the results view; `*` unstars the selected row there.
  - Added mouse support for the landing Favorites row (select/open rows, open the full list via the overflow line).
  - Extracted cross-provider title-identity matching into `SubjectIdentity` (`src/models.rs`), now shared by watch history and Favorites so remakes, cross-provider duplicates, and movie/series title collisions are deduplicated identically.

### Fixed
- **External player failure reporting**: Treat every non-zero player exit, including exits after several seconds or without stderr output, as a playback error instead of reconciling false watch progress.
- **Search focus**: Restore Backspace on the Home screen as a reliable way to focus the search input from results or Favorites.
- **Season subtitles**: Remember an explicit subtitle or no-subtitle choice for every episode in a season download.
- **Playback state safety**: Sanitize provider and subject identifiers before using them in tracker state filenames.
- **Cross-platform release validation**: Run all-feature locked builds/tests, binary startup smoke tests, Unix and Windows installer syntax checks, and release-target builds in CI.
- **Release artifact smoke tests**: Execute each native release target's produced binary on its CI runner before archiving.
- **Documentation accuracy**: Document Android intent limitations, the actual Termux binary model, macOS-only IINA support, native runtime verification requirements, and the current automated test count.
- **Windows VLC & player spawn fix**: Removed erroneous `CREATE_NO_WINDOW` flag that suppressed GUI window creation and caused VLC to crash on Windows; normalized Windows backslash subtitle paths in `--sub-file` and gracefully handle subtitle download failures without breaking stream playback.
- **Termux player opener resolution**: Resolve `termux-open`, `termux-open-url`, and `termux-am` directly in `$PREFIX/bin` and static Termux paths; remove broken `/system/bin/am` fallback in unrooted Termux that crashed with Permission Denied (exit code 126).
- **Termux dependency path**: Remove the Android platform-verifier dependency path that caused the v0.1.12 startup panic; real-device confirmation remains required.

## [0.1.13] - 2026-08-21

### Added
- **Production-Grade In-App Self-Update Engine**:
  - Implemented modular self-update architecture (`src/updater/` with `check.rs`, `artifact.rs`, `download.rs`, `verify.rs`, `extract.rs`, and `apply.rs`).
  - Added streaming SHA-256 integrity verification validating exact hash matching against release `SHA256SUMS`.
  - Added hardened archive extraction for `.tar.gz` and `.zip` with strict path traversal protection against `..` components and absolute root paths.
  - Added multi-platform installation strategies: atomic binary replacement with `.old` backup and automatic rollback on Unix/Linux/macOS/Termux, detached helper process on Windows, and Homebrew prefix detection guiding users to `brew upgrade moviebox-tui`.
  - Added active work protection deferring self-update when active video playback or background downloads are running.
  - Connected `[u]` shortcut and `[u] Update Now` button in the existing Update Available modal, preserving visual styling, animations, and dismissal model.
  - Added safe terminal state restoration (`disable_raw_mode`, `LeaveAlternateScreen`, `DisableMouseCapture`, `ShowCursor`) before process exec/restart.
- **Update System Concurrency & Platform Compatibility Architecture**:
  - Added single-flight guard (`is_checking_updates`) ensuring manual (`/update`) and automatic startup checks never spawn duplicate concurrent network requests.
  - Added shared geometry calculation (`UpdateModalLayout`, `update_modal_layout`) guaranteeing 1:1 synchronization between popup rendering and mouse hit testing.
  - Added release asset data modeling (`Release`, `ReleaseAsset`, `TargetPlatform`) with deterministic platform compatibility detection across macOS Universal, Linux x64/arm64, Windows x64/arm64, and Android Termux ARM64.
  - Added dedicated integration test suite `tests/update_lifecycle.rs` testing update single-flighting, error recovery, mouse hit testing, and asset filtering.
- **Comprehensive QA & Regression Test Architecture**:
  - Introduced the initial 132-test automated suite covering critical algorithmic boundaries, end-to-end user journeys, watch history reconciliation & precision progress tracking, cross-mode history audit, in-app self-update lifecycle, real-world release artifact downloads, live SHA-256 verification, genuine version upgrade execution, content & metadata loading pipelines, stale request isolation, active player session lifecycle & duplicate launch protection, dynamic slash command autocomplete (`/download-dir reset`), search/command draft cancellation via `Esc`, error handling, addon manifest validation, mouse interactions, modal dismissals, TUI rendering across terminal size matrices, state reconciliation, crypto HMAC signing, download chunk arithmetic, and URL/stem security.
  - Added structured integration tests in `tests/` (including content, error, TUI, history, Favorites, update, release, cache, player, TV playlist, addon, download, and URL-security suites) and test fixtures (`tests/fixtures/`).
  - Added [`docs/testing.md`](docs/testing.md) detailing test architecture, command references, and manual QA procedures.
- **Playback Tracking & Watch History Progress**:
  - Added real-time playback position tracking for `mpv` with injected tracker script (`moviebox_tracker.lua`) and 5-second periodic state auto-save to disk.
  - Added automatic startup state reconciliation (`reconcile_pending_playback_states`) ensuring watched progress is preserved even when closing the terminal or killing tmux mid-playback.
  - Added two-tone smooth scrub line progress bars (`━─────── 1% (2h 18m left) • Watched 11h ago`) and completion status badges (`[✓ Completed]`, `[✓ Watched]`) in `/history` and Details screens.
  - Added cross-provider title-based history deduplication and auto-resume from the last watched position.
- **Addon Mode Watch History Parity**:
  - Added full watch history support (`/history`) in Addon Mode matching Streaming Mode, enabling seamless watch progress tracking, scrub bars, and completion badges for community HTTP addon content.
- **Pluggable Provider Trait & Capability Architecture**:
  - Formalized the public `Provider` and `ReleaseProvider` traits across all built-in scrapers (`MovieBox`, `4KHDHub`, `CircleFTP`, `DhakaFlix`, and `Addons`).
  - Added `ProviderCapabilities` (`supports_search`, `supports_pagination`, `supports_series`, `supports_subtitles`, `supports_homepage`) and `MovieBoxService::capabilities()` for dynamic capability reporting.
  - Added structured `ProviderError` boundaries (`Network`, `RateLimited`, `NotFound`, `Parsing`, `Unavailable`) with `.user_message()` for consistent error notifications.
- **Theme System Expansion & Official Color Calibration**:
  - Added official **Dracula**, **Gruvbox**, and **Rosé Pine** themes to the `/theme` picker alongside Catppuccin and Nord.
  - Added alias parsing support for `"dracula"`, `"gruvbox"`, `"rose-pine"`, and `"catppuccin"`.
  - Guaranteed 100% transparent terminal compatibility across all themes with zero background opacity overrides.
  - Fixed modal backdrop rendering by removing fullscreen screen clearing when opening `/theme`.
  - Optimized live preview navigation to eliminate unnecessary disk I/O on arrow key navigation.
- **Universal Multi-OS Player Detection & Flathub/Snap Compatibility**:
  - Added sub-millisecond, filesystem-backed player probing across Linux (Flathub, Flatpak exports, Snap, and Native), macOS (Homebrew, MacPorts, App Bundles), Windows (Program Files, WinApps, Scoop, Chocolatey, WinGet), and Android (Termux).
  - Fixed Flathub/Flatpak VLC detection failure by adding direct probes for `~/.local/share/flatpak/exports/bin/org.videolan.VLC` and `/var/lib/flatpak/exports/bin/org.videolan.VLC`.
  - Added full Flathub/Flatpak and Snap compatibility for MPV (`io.mpv.Mpv`, `/snap/bin/mpv`).
  - Centralized player process construction (`build_player_process_command`) and standardized subtitle flag arguments (`--sub-file=<path>`) across all platforms.
- **Codebase Optimization & Comprehensive Caching Architecture**:
  - Centralized application paths (`config_dir`, `data_dir`, `cache_dir`, `logs_dir`, `scripts_dir`, `playback_state_dir`) in `src/config.rs`.
  - Added dedicated disk caching for Addon Mode stream aggregation (`2h` TTL), catalog `/browse` presets (`1h` TTL), and verified manifests (`24h` TTL).
  - Added search pagination caching (`search_{hash}_{page}.json`) preventing redundant API calls when navigating multi-page search results.
  - Eliminated redundant `reqwest::Client` allocations in background poster pipelines in favor of the shared `service.http_client()`.
  - Streamlined `MovieBoxService` usage across background tasks and removed redundant `addon_client` field from `AppState`.
  - Centralized formatting utilities (`format_file_size`, `format_duration`) in `src/tui/text.rs`.
  - Modernized `Config` loading and persistence with safe, standard Serde derives.
- **Addon Mode (Community HTTP Addons)**:
  - Added full support for community HTTP addon manifests (`/manifest.json`, `/catalog`, `/meta`, `/stream`) with dedicated `Ctrl+A` mode switching.
  - Pre-installed and locked Cinemeta out-of-the-box as the default core metadata provider with zero API keys required.
  - Added interactive Addon Manager dialog (`/addons`, `Ctrl+P` in Addon Mode) with one-click enabling, removal, and manifest URL adding.
  - Added concurrent multi-addon stream resolution aggregating playable releases from all enabled stream addons.
  - Added a smart runtime torrent detector that automatically detects if an addon's streams are 100% blocked raw torrents (e.g., Torrentio without Debrid) and flashes a UI warning toast that only HTTP streams are supported.
- **Addon Mode `/browse` & Curated Catalog Exploration**:
  - Added `/browse` support in Addon Mode with a minimal, organized 4-preset catalog picker (`Top Movies`, `Top Series`, `Top Rated Movies`, `Top Rated Series`).
  - Added direct catalog fetching (`/catalog/{type}/{id}.json`) with poster hydration, details navigation, stream resolution, and `/reload` support.
- **Strict Slash Command Guarding & Guidance**:
  - Intercepted all `/` slash commands to guarantee zero remote catalog network requests.
  - Added warning toast notifications for unrecognized slash commands (`"Command '/xyz' is not recognized. Type '/' to view available commands."`).
  - Added platform-aware mode-guidance toasts (`^T` / `^S` / `^A` on macOS, `Ctrl+T` / `Ctrl+S` / `Ctrl+A` on Linux/Windows) for mode-restricted commands.
- **Active Mode & Provider State Persistence**:
  - Added `active_mode` configuration field in `config.json` automatically persisting and restoring the last active mode (`streaming`, `tv`, `addon`) and active provider across app restarts.
- **Configurable Mode Navigation**:
  - Added `/enable-streaming`, `/disable-streaming`, `/enable-tv`, and `/disable-tv` slash commands alongside `/enable-addons` and `/disable-addons`.
  - Enforced safety validation ensuring at least one mode remains active and gracefully migrating focus when disabling the current mode.
- **Dynamic Multi-Source Host & Resolver Resolution**:
  - Added 100% dynamic domain-based host extractor (`extract_domain_label`) and stream tag parser (`detect_stream_host`) identifying and formatting direct hosts (Pixeldrain, Hubcloud, Fast Download, Google Drive, Mega, etc.) and debrid resolvers without hardcoded tables.
- **Full Emoji & Symbol Sanitization**:
  - Added `strip_emojis` and `clean_stream_text` sanitizing all raw stream titles, release names, source labels, and languages from community addons for clean terminal alignment without broken characters.
  - Standardized checkbox representations to clean ASCII `[x] / [ ]`.
- **Complete Mouse Navigation**:
  - Added dynamic footer hitboxes for `[Ctrl+S] Streaming`, `[Ctrl+T] TV`, `[Ctrl+A] Addons`, `[Ctrl+P] {Provider}`, `[?] Help`, `[q] Quit`.
  - Added complete mouse click support for Addon Manager modal and browse popups.

### Fixed
- **Windows MSVC Static CRT Linking (`+crt-static`)**:
  - Configured `target-feature=+crt-static` in `.cargo/config.toml` for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, statically embedding the C runtime to eliminate external `VCRUNTIME140.dll` dependency and resolve `0xC0000135` (`STATUS_DLL_NOT_FOUND`) on clean Windows installations.
- **Cross-Platform Installer Polish & Windows In-Memory Execution**:
  - Replaced file-based execution commands in Windows documentation with the in-memory stream pipeline (`irm ... | iex`) to eliminate `PSSecurityException` execution policy blocks.
  - Added immediate active process `$env:PATH` update in `install.ps1` so the command is recognized in the current shell session without terminal restart.
  - Replaced rigid fixed-width boxed summary tables with responsive, borderless hero layouts across both `install.ps1` and `install.sh`, preventing broken box-drawing characters and layout overflow on narrow screens.
- **Pending History Reconciliation Order**:
  - Sorted pending Lua tracker state files chronologically during startup reconciliation to guarantee correct playback state replay order.
- **MovieBox Title Sanitization (DEF-02)**:
  - Fixed destructive title truncation where leading bracket tags (`[Dub]`, `[1080p]`, `[RAW]`) and titles starting with parentheses (e.g. `(500) Days of Summer`) were stripped down to empty strings.
  - Preserved release years in parentheses (`Inception (2010)`) and added a fallback safeguard returning the trimmed original title if sanitization ever results in an empty string.
- **Watch History Identity & Deduplication Collisions (DEF-03)**:
  - Enforced `stype` separation in `HistoryManager::is_same_show` so Movies and TV Series sharing identical titles (e.g. `Home`) never overwrite one another.
  - Enforced strict canonical identity (`provider + subject_id`), preventing cross-provider conflicts and ensuring remakes with differing release years remain distinct entries.
- **Background Episode Playback State Reconciliation (MISS-01)**:
  - Fixed a state loss bug in `reconcile_pending_playback_states` where a completed episode's watched status was discarded if the user had already advanced to a subsequent episode before the state file was processed.
- **Windows MPV Script Options Path Escaping (DEF-04)**:
  - Fixed path corruption in MPV's `--script-opts` on Windows by normalizing backslashes (`\`) to forward slashes (`/`), preventing MPV escape sequence parsing from corrupting `state_file` paths in `moviebox_tracker.lua`.
- **M3U Single-Quoted Attribute Support (DEF-07)**:
  - Extended `M3UParser` attribute extraction to support both single-quoted (`tvg-id='...'`) and double-quoted attributes, preserving channel IDs, logos, and groups across varied IPTV playlists.
- **Continuous OS-Level SIGINT Handling (DEF-05)**:
  - Wrapped `tokio::signal::ctrl_c()` in a continuous background loop to ensure repeated non-interactive OS signals are reliably handled.
- **Subtitle Prefetch Fallback (DEF-08)**:
  - Reduced subtitle download timeout from 30s to 8s to prevent unnecessary startup delays when launching external players if a subtitle mirror hangs.
- **Addon Stream Sorting & Rendering**:
  - Fixed addon streams randomly scrambling on UI hover when sizes are tied by adding a secondary stable sort based on the mirror label.
  - Fixed misleading `0MB` stream sizes for community addons that omit video sizes by cleanly rendering `--` instead.
- **Terminal Race Condition & Blank Screen on `/clear-cache`**:
  - Replaced physical terminal clear with a soft image refresh when executing `/clear-cache`, resolving a race condition with terminal emulators that swallowed the full Home screen render and caused the screen to go completely blank after a few seconds.
  - Added comprehensive state isolation preventing search queries, results, and details states from lingering after cache clears.
  - Sanitized slash command input handling to prevent visual query glitches.
  - Replaced standard status messages with elevated toast notifications for cache actions.
- **Atomic Mode Highlight & Single Active Selection**:
  - Added canonical `AppMode` enum (`Streaming`, `Tv`, `Addon`) and atomic state transitions guaranteeing that only one active mode is highlighted in the bottom dock at any time.
  - Hardened state isolation with automatic cleanup across mode switches.
- **Notification Readability & Word-Boundary Wrapping**:
  - Replaced horizontal middle-truncation with unicode display-width aware word wrapping (`wrap_text`).
  - Added adaptive width (up to 72 chars) and dynamic height scaling with guaranteed unbroken rounded borders.
- **Resilient Addon Metadata & Fallbacks**:
  - Added flexible visitors and serde aliases for `genres`, `cast`, `director`, `imdbRating`, `releaseInfo`, and `runtime` preventing deserialization failures across varied community addon JSON schemas.
  - Added multi-tier fallback resolution in the Details screen to guarantee titles, release years, synopsis, and posters are always preserved from search results and previews.
- **Android / Termux TLS Certificate Compatibility**:
  - Switched `reqwest` to use pure-Rust embedded `webpki-roots` certificate verification, resolving `rustls-platform-verifier` crashes and panics in non-JVM Android CLI environments like Termux.
- **Transparent Stream & Search Diagnostics**:
  - Replaced misleading generic `"No matches"` and `"Rate Limit"` errors with truthful, contextual diagnostics: `"No stream sources available on {provider}"`, `"Network connection failed to {provider}"`, `"Rate limited by {provider}"`, and `"Episode S{season}E{episode} is not listed on {provider}"`.
  - Added helpful actionable hints (`Press Ctrl+P to try another provider, or r to refresh`).
- **Rate-Limiting & Concurrency Hardening**:
  - Added HTTP 429 `Retry-After` header parsing with bounded exponential backoff in `MovieBoxClient`.
  - Added semaphore concurrency limiting (`Semaphore::new(2)`) during parallel episode page resolution to prevent burst requests from tripping provider rate limiters.
- **Addon Mode Series Hierarchy & Episode Stream Isolation**:
  - Fixed series misclassification as movies in Addon Mode when metadata omitted the `videos` array by ensuring canonical season structures and series-first metadata endpoint prioritization.
  - Added regex and token-based episode stream isolation (`parse_season_episode`) in `stream_item_to_release`, preventing cross-episode stream pollution (e.g. S01E06 streams appearing when viewing S01E08).
  - Added preservation of `episodeNumbers` arrays from addon metadata in the season list state.
- **Direct Addon & BDIX Playback & Download Dispatch**:
  - Fixed Addon and BDIX playback and download routing in `handle_playback` and `handle_download` to dispatch directly to external media players and the chunk downloader, preserving custom HTTP headers (`behaviorHints.headers`) and source labels without unnecessary Moviebox API subtitle timeouts.
- **Selector Tab Preservation in Standard Displays**:
  - Maintained visibility of Audio Languages, Seasons, and Episodes selector tabs side-by-side in standard ~80-column terminals when focusing Streams, preventing tabs from disappearing when 0 streams are available.

### Changed
- **Modular TV Provider Architecture**:
  - Reorganized Live TV / IPTV provider into a dedicated module directory (`src/providers/tv/`) with separated `models.rs` and `parser.rs`.
- **Core Infrastructure Consolidation**:
  - Centralized atomic file operations (`atomic_write_file`, `atomic_write_file_async`), MD5 digest formatting (`md5_hex`), and text extraction helpers in `cache.rs` and `service.rs`.
  - Centralized application paths, border type resolution, and mode status announcements across TUI modules.
- **Addon Manager UI Optimization**:
  - Implemented full cursor navigation (`Left`/`Right` keys) and inline editing (`Backspace`/`Delete`) for the Addon Manager input field.
  - Implemented a scrolling viewport renderer for the Addon Manager input, allowing editing of very long manifest URLs without wrapping or truncation.
  - Compacted the Addon Manager dialog with an aligned two-tier layout placing `[ Add Manifest URL ]` and `[ Done ]` action buttons side-by-side.
- **Multi-System Core Module Decoupling**:
  - Promoted `player.rs` (process management & detection), `config.rs` (shared configuration), and `updater.rs` (release checks) to core modules in `src/`, preparing the architecture for upcoming CLI and GUI frontends with full backward compatibility.

### Documentation
- **Streamlined README & Controls Guide**:
  - Transformed `README.md` into a focused landing page with measured `~5 MB RAM` benchmark data, defensible value propositions, and direct links to deep guides in `docs/`.
  - Created standalone `docs/controls.md` covering all keyboard shortcuts, mouse controls, and slash commands.
  - Added a 3-phase project roadmap: Terminal UI (TUI) -> Command-Line Interface (CLI) -> Desktop GUI Client.
  - Added a community-first feedback and support section with optional crypto donation options.

## [0.1.12] - 2026-08-15

### Added
- **CLI Help Flag**: Added `-h` / `--help` CLI flags printing formatted usage, available options, and environment variables.
- **Full Mouse Support**: Complete mouse navigation throughout the application:
  - Click search bar to edit; click suggestion items to search immediately.
  - Click search results to select/preview; click again or double click to enter Details.
  - Click Details panes (Audio Languages, Seasons, Episodes, and Streams) to select and launch playback.
  - Click centered footer toolbar buttons (`[Ctrl+P] Provider`, `[Ctrl+T] TV`, `[?] Help`, `[q] Quit`).
  - Full click support across all modal popups (Theme, Browse, Subtitles, Players, TV playlists & actions, and Download confirmation).
- **Contextual Downloads**:
  - Pressing `d` or clicking `[Download]` while on the **Seasons** pane prompts to download the whole season (all episodes).
  - Triggering download while on **Episodes** or **Streams** downloads that single episode.
- **Organized Downloads & Custom Directory**:
  - Structured Series downloads under `<base_dir>/Series/<Title>/Season <N>/<Title> - S<N:02>E<E:02>.<ext>` and Movies under `<base_dir>/Movies/<Title>/<Title>.<ext>`.
  - Added ISO 639-1 language code tagging to subtitle sidecars (e.g. `<BaseName>.en.srt`) for automatic track identification in media players and servers.
  - Added smart duplication prevention: completed episodes on disk are automatically skipped during season batch downloads.
  - Added `/download-dir <path>` slash command with directory creation and active write-probe validation.
  - Added `/download-dir reset` (contextually suggested only when custom path is configured) to revert to OS default.
  - Safe automatic fallback to default OS Downloads folder if custom path becomes inaccessible.
  - Configuration persistence across sessions in `config.json`.
- **Tree Branch Suggestions**:
  - Redesigned search and slash command autocomplete into a minimal, transparent tree-branch layout (`├─ ` / `└─ `) anchored directly under the search prompt.
  - Added aligned slash command descriptions (`browse`, `history`, `theme`, `config`, `update`, etc.) without duplicate leading slashes.
  - Clean typography-driven active selection with bold vibrant accent styling.
- **Multilingual Audio Track Detection**:
  - Expanded 4kHDHub release parser to detect 30+ regional and international languages (Hindi, Tamil, Telugu, Kannada, Malayalam, Bengali, Marathi, Punjabi, Gujarati, Urdu, Japanese, Korean, Chinese, Spanish, French, German, Italian, etc.) and abbreviations (`Tam`, `Tel`, `Kan`, etc.).
  - Responsive stream list formatting showing all available languages without crowding mirror counts.
- **Floating Pill HUD & Smooth Resize**:
  - Added floating terminal dimension HUD and event coalescing for smooth window resizing without blank screens.
- **Elevated Notification Badges**:
  - Redesigned notification popups into elevated, rounded bottom-right badge cards with clean typography.
- **Persistent Long-Term Poster Caching**:
  - Increased image cache retention to 30 days (`IMAGE_CACHE_EXPIRY_SECS`), serving previously fetched posters instantly from disk across sessions with zero redundant network requests.
  - Unified image caching under a shared namespace with automatic cross-namespace lookup across MovieBox, 4KHDHub, IPTV, CircleFTP, and DhakaFlix.
- **Streamlined Browse Views**:
  - Curated `/browse` views into 4 categorized shelves (Popular, Top Rated, Trending, Most Watched) with proper filtering.
- **Native Graphics & Single Standardized 'No Poster' Placeholder**:
  - Replaced redundant dual labels (`Poster unavailable` / `No Art`) and noisy halfblock mosaic fallback with a single clean, centered `No Poster` label across search results, details, and history on non-graphics terminals.
  - Eliminated ANSI block characters, yellow/white selection redraw bars, and unnecessary background image downloads on basic terminals.
  - Preserved full native high-resolution graphical rendering on Sixel, Kitty, and iTerm2 supported terminals.
  - Added `MOVIEBOX_NO_IMAGE=1` environment override to disable image probing on slow or headless sessions.
- **Next-Gen Multi-Tiered Animated Installers (`install.sh` & `install.ps1`)**:
  - Multi-tier progressive rendering with official MovieBox branding and Catppuccin Mocha aesthetic.
  - Live smooth Braille spinners (`⠋ ⠙ ⠹ ...`), SHA256 cryptographic verification against `SHA256SUMS`, and media player ecosystem detection.
  - 100% sudo-less user-level installation into `~/.local/bin` (or `%LOCALAPPDATA%\Programs\MovieBox-Tui\bin` on Windows) with automatic non-destructive shell PATH integration and zero password prompts.
  - Added full CLI flags: `--version <tag>`, `--dir <path>`, `--force`, `--dry-run`, and `--uninstall`.
- **Explicit Download Directory Autocomplete Hints**:
  - Added `/download-dir <path>` slash command suggestion with clear action descriptions (`Set custom folder (e.g. ~/Movies)` vs. `View current download folder`).
  - Added friendly guidance notification if a user inputs literal `<path>` placeholders.

### Fixed
- **Custom Download Directory Container Hierarchy**:
  - Ensured custom download directories always maintain the standardized `MovieBox-TUI` root container (`MovieBox-TUI/Movies/...` and `MovieBox-TUI/Series/...`) without duplicating if already named `MovieBox-TUI`.
- **Multiline Notification Toast Rendering**:
  - Upgraded notification toast layout to compute dynamic height and wrap multiline messages per line cleanly without horizontal middle-truncation across newlines.
  - Sanitized notification folder paths by substituting home directory with `~`.
- **Default Audio Track Prioritization (Original / English)**:
  - Fixed movie and series details defaulting to regional Hindi dubs on MovieBox by prioritizing `Original` and `English` audio tracks over localized search result subject IDs.
  - Preserved explicit user language selections when intentionally switching between dubs.
- **Home Landing Header & Footer Persistence**:
  - Fixed ASCII logo header and shortcut footer disappearing into a blank screen when clearing history or viewing empty search states by removing fragile tick-based animation gates.
  - Ensured the landing screen renders the logo, version, centered search bar, and footer shortcuts immediately on every frame.
- **Watch History Consolidation & Latest Progress Representation**:
  - Consolidated watched episodes of the same series into a single entry per show in `/history` displaying the latest watched season and episode.
  - Automatically deduplicated and migrated legacy history rows on startup while maintaining complete per-episode checkmark indexes in `self.watched`.
- **History Poster Auto-Hydration & In-Memory Cache Retention**:
  - Fixed "No Poster" placeholders in `/history` by automatically resolving missing cover URLs and decoding posters in the background.
  - Preserved in-memory decoded image caches when opening `/history` to eliminate unnecessary UI redraw latency.
  - Added multi-source fallback extraction for cover URLs across playback, preview, and search results.
- **Stream Pool Initialization on Audio Selection**: Fixed stream fetching hanging on "Loading streams..." when selecting non-default audio dubs by ensuring stream pool entries are initialized before episode fetch.
- **Title Sanitization & Preservation**: Enhanced `clean_moviebox_title` to sanitize international audio dubs, video quality tags, and format markers across downloads, folder organization, and watch history while preserving 4-digit release years.
- **Terminal Restoration & Signal Handling**: Added `Ctrl+C` keyboard handling and asynchronous `SIGINT` signal listener to guarantee raw mode and alternate screen are always cleanly restored.
- **Download Hierarchy & Numbering**: Fixed series media type detection and removed season/episode off-by-one addition.
- **Parser UTF-8 Safety**: Hardened language detection boundary checks for multibyte titles against panics.
- **Startup Screen Artifacts**: Removed early startup `eprintln!` to eliminate terminal screen artifacts before entering alternate screen mode.
- **Android / Termux Stability**: Removed `hickory-dns` from network dependencies to resolve NDK context panics and crashes on Android.
- **Screen Flickering & Blanking**:
  - Eliminated full terminal clear on list navigation and infinite scroll pagination.
  - Fixed screen blanking when pressing `Esc` or resizing windows.
  - Replaced terminal clear with direct backend clear to eliminate cursor read timeouts.
- **Search & Navigation**:
  - Fixed search bar auto-closing when switching providers.
  - Fixed provider switching delays and event stream drops.
  - Kept chosen audio dub selected and prevented unwanted pane jumping on details refresh.
  - Handled empty query loading states and preset failures gracefully.
- **Downloads & Playback**:
  - Resolved MovieBox movie stream key mismatches and hardened resilient download flows.
  - Protected active downloads from accidental cancellation when typing `x` in the search bar.
  - Fixed playback lock edge cases and subtitle picker clipping.
- **Theme & Configuration**:
  - Fixed theme cancellation reverting correctly without persisting unapplied themes.
  - Unified `/theme` command and removed obsolete `/discover`, `/tab`, and `/themes` aliases.

### Changed
- Removed startup screen delay for instant app launch.
- Modernized in-place update notifications and dialogs.
- Rendered details footer on a single clean line to balance bottom margins.
- Removed search bar underline clutter in favor of clean header spacing.

## [0.1.11] - 2026-08-11

### Added
- **User-Owned M3U Playlists**: Full custom playlist management in TV mode with remote URL and local file support.
- **Android Runtime Support**: Termux playback and shared-storage handling continue to be exercised on real devices, but release artifacts remain desktop-focused.

### Refactored
- **Domain Modularization**: Split the application monolith into cohesive domain modules (`network`, `playback`, `download`, `requests`, `navigation`, `tv`, `system`, `keyboard`).
- **State Decomposition**: Split the monolithic application state into specialized domain state structs.
- **Strict Verification Gates**: Enforced workspace lint checks, static analysis, and testing.
