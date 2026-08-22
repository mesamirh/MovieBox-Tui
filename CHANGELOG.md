# Changelog

## [Unreleased]

### Added
- **Favorites**:
  - Added a Favorites feature for starring whole movies and series (`src/favorites.rs`, `favorites.json`), independent of watch history and unaffected by `/clear-cache`.
  - Added `*` on the Home screen and `f` / `F` on the Details screen to toggle a title's favorite status, with a `★` indicator on favorited rows and a `[f] Favorite` / `[f] Unfavorite` hint on the Details screen.
  - Added an arrow-navigable Favorites row on the landing screen (Streaming and Addon modes) showing up to 5 recently-starred titles, with a `+N more • /favorites` overflow link; `Down` from the search bar focuses the row, `Enter` opens the selected title, `Esc` releases focus.
  - Added the `/favorites` slash command, mirroring `/history`, to load the full starred list into the results view; `*` unstars the selected row there.
  - Added mouse support for the landing Favorites row (select/open rows, open the full list via the overflow line).
  - Extracted cross-provider title-identity matching into `SubjectIdentity` (`src/models.rs`), now shared by watch history and Favorites so remakes, cross-provider duplicates, and movie/series title collisions are deduplicated identically.

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
  - Introduced a 132-test automated suite covering critical algorithmic boundaries, end-to-end user journeys, watch history reconciliation & precision progress tracking, cross-mode history audit, in-app self-update lifecycle, real-world release artifact downloads, live SHA-256 verification, genuine version upgrade execution, content & metadata loading pipelines, stale request isolation, active player session lifecycle & duplicate launch protection, dynamic slash command autocomplete (`/download-dir reset`), search/command draft cancellation via `Esc`, error handling, addon manifest validation, mouse interactions, modal dismissals, TUI rendering across terminal size matrices, state reconciliation, crypto HMAC signing, download chunk arithmetic, and URL/stem security.
  - Added structured integration tests in `tests/` (`content_pipeline.rs`, `error_handling.rs`, `tui_acceptance.rs`, `history_reconciliation.rs`, `history_audit.rs`, `update_lifecycle.rs`, `real_acceptance.rs`, `version_upgrade_e2e.rs`, `cache_lifecycle.rs`, `player_integration.rs`, `m3u_integration.rs`, `addons_manifest.rs`, `download_integration.rs`, `url_security.rs`) and test fixtures (`tests/fixtures/`).
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
