# Changelog

## [Unreleased]

### Changed
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
  - Removed redundant `Poster Graphics` toggle from Settings Hub (`/settings` $\to$ Appearance) and `config.json`, delegating terminal graphics strictly to automatic native GPU protocol detection (Kitty, Sixel, iTerm2).

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
  - Removed standalone `/theme` slash command, parser routing, and auto-suggestions; theme selection and visual palette swatches are managed directly within the interactive Settings Hub (`/settings` $\to$ Appearance $\to$ Theme).
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
