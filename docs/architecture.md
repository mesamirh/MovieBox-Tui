# Architecture

MovieBox-Tui is a terminal client (ratatui + crossterm + tokio) for streaming movies,
series and TV channels from multiple providers. This document describes the shape of the
code and how data flows through it.

## System Architecture & Data Flow

```text
               ┌────────────────────────────────────────────────────────┐
               │              Terminal User Input & Events              │
               │   Crossterm KeyPress / MouseClick / WindowResize / Tick│
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │           EventHandler Channel (mpsc::channel)         │
               │          Dispatches serialized Action messages         │
               └───────────────────────────┬────────────────────────────┘
                                           │
                                           ▼
               ┌────────────────────────────────────────────────────────┐
               │      App Event Loop & State Machine (Single Thread)    │
               │    src/tui/app/run.rs ─── Mutates AppState atomically  │
               └──────────────┬───────────────────────────┬─────────────┘
                              │                           │
              UI Render Pass  │          Async IO Tasks   │  Background Workers
                              ▼                           ▼
        ┌───────────────────────────┐   ┌───────────────────────────────────┐
        │   Ratatui Terminal Frame  │   │      Tokio Multi-Thread Runtime   │
        │ Screens / Widgets / Toast │   │  tokio::spawn / spawn_blocking    │
        └───────────────────────────┘   └─┬───────────────┬───────────────┬─┘
                                          │               │               │
                                          ▼               ▼               ▼
                                 ┌────────────────┐┌─────────────┐┌─────────────────┐
                                 │ Scrapers & Net ││ Disk Cache  ││ Media Players & │
                                 │ Providers & TV ││ MessagePack ││ Loopback Proxy  │
                                 └────────────────┘└─────────────┘└─────────────────┘
```

### Non-terminal front ends (iOS)

The crate is not terminal-only. The same provider, networking and model modules are
reused by a native front end through UniFFI, and that path never touches ratatui or
crossterm:

```text
  SwiftUI screens (ios/MovieBox/*.swift)
              │  Bridge.swift: serial queue + checked continuation
              ▼
  Generated bindings (ios/Generated/moviebox_tui.swift)
              ▼
  src/ffi.rs  MobileClient        ← compiled only with the `mobile` feature
              ▼
  MovieBoxService (src/service.rs) ── providers/ · net.rs · models.rs · text_util.rs
```

The bridge is gated by the `mobile` Cargo feature (`src/lib.rs` declares
`#[cfg(feature = "mobile")] pub mod ffi;`), so the default build and the `moviebox-tui`
binary never compile or link it. See [ios-client.md](ios-client.md) for the FFI surface
and the build pipeline.

## Subsystem Breakdown & Module Tree

```text
src/
├── main.rs                        # Application entrypoint & CLI dispatcher
├── lib.rs                         # Crate root and module declarations
│
├── Core Engine & Networking
│   ├── models.rs                  # Domain entities (CatalogItem, MediaDetails, Release)
│   ├── service.rs                 # Central multi-provider facade & aggregation engine
│   ├── net.rs                     # Hickory DNS fallback resolver & HTTP client builders
│   ├── logging.rs                 # Rotating file logger with path/URL privacy sanitization
│   └── proxy.rs                   # Detached loopback HTTP proxy & DASH manifest rewriter
│
├── Storage & Persistence
│   ├── cache.rs                   # Binary MessagePack cache (MBC1 header) with atomic writes
│   ├── config.rs                  # User configuration schema & atomic persistence (config.json)
│   ├── favorites.rs               # Bookmark storage & deduplication (favorites.json)
│   ├── history.rs                 # Watch progress, latched resume, & state reconciliation
│   └── download.rs                # Chunked multi-segment downloader with HTTP range resume
│
├── Media Providers (`src/providers/`)
│   ├── models.rs                  # Provider traits (Provider, ReleaseProvider) & capabilities
│   ├── moviebox/                  # CloudFront signed requests, token generation, & scraper
│   ├── fourkhdhub/                # 4K releases, HTML parser, & HubCloud mirror resolver
│   ├── dramachi/                  # Asian dramas and series scraper
│   ├── bdix/                      # BDIX optical intranet scrapers (CircleFTP, DhakaFlix)
│   │   └── common.rs              # Centralized codec, resolution, & language heuristics
│   ├── addons/                    # Community Stremio HTTP addon manifest & stream aggregator
│   └── tv/                        # IPTV M3U playlist parser & stream normalizer
│
├── Player & Process Supervisor
│   ├── player.rs                  # Player detection (mpv, IINA, VLC, Android) & command builder
│   └── player/tracker.rs          # Embedded Lua tracker script (moviebox_tracker.lua)
│
├── Self-Updater (`src/updater/`)
│   ├── check.rs                   # GitHub release API version checker
│   ├── download.rs                # Streaming release archive downloader
│   ├── verify.rs                  # Cryptographic SHA-256 checksum validator
│   ├── extract.rs                 # Tar.gz and Zip extractor with path traversal guard
│   └── apply.rs                   # Atomic binary replacement, path resolution, & restart helper
│
└── Presentation & TUI (`src/tui/`)
    ├── action.rs                  # Unified Action enum message bus
    ├── event.rs                   # Crossterm terminal input event listener & tick driver
    ├── state.rs                   # Centralized AppState & LRU memory caches
    ├── commands.rs                # Slash command registry (/settings, /browse, /exit)
    ├── terminal.rs                # Terminal capability & graphics protocol detection
    ├── theme.rs                   # Theme registry (Catppuccin, TokyoNight, Nord, etc.)
    ├── text.rs                    # Grapheme-safe input buffer & Unicode measurement
    ├── overlay.rs                 # Toasts, modals, confirmation dialogues, & picker frames
    ├── screens/                   # Declarative renderers (home.rs, details.rs, help.rs)
    ├── widgets/                   # Modular widgets (badge, input, modal, poster, settings)
    └── app/                       # State machine action handlers & event loop
        ├── run.rs                 # Main event loop (App::run) & frame drawing
        ├── keyboard.rs            # Keyboard shortcuts & vim navigation router
        ├── mouse.rs               # Mouse click hit-testing & drag coordinates
        ├── navigation.rs          # Grid steps, pagination, & screen transitions
        ├── requests.rs            # Async metadata & stream request dispatchers
        ├── playback.rs            # Player process launching & crash supervision
        ├── download.rs            # Download queue manager & progress bar updates
        ├── search.rs              # Search input handler & poster prefetching
        ├── favorites.rs           # Favorite bookmarks handler
        ├── tv.rs                  # IPTV channel manager & playlist actions
        ├── addons.rs              # Stremio addon manager actions
        └── system.rs              # Terminal resize, theme switching, & self-update UI
```

## The event loop

`App::run` (in `app/run.rs`) owns the only loop:

1. If `clear_terminal_before_draw` is set, the terminal buffer is cleared.
2. If `state.dirty`, the screen is drawn (`App::draw`).
3. `tokio::select!` waits for either:
   - an `Action` from the `EventHandler` (keyboard/mouse/focus/resize/tick), or
   - an `Action` pushed by a background task (network results, downloads, posters).

`EventHandler` (`event.rs`) spawns one task that reads crossterm events and a `Tick`
interval, forwarding them into the action channel (capacity 128).

## Async model

- The tokio runtime is multi-threaded (`rt-multi-thread`).
- **State is single-threaded**: all mutations to `AppState` happen inside
  `handle_action`, which is driven by the single event-loop task. Actions are
  serialized through the channel, so there are no data races on UI state.
- **Network** uses async `reqwest` clients (one per provider) plus per-provider signing.
- **Blocking work** (disk cache reads/writes, image decoding, M3U parsing, watch-history
  save, log cleanup) runs on `tokio::task::spawn_blocking` so the event loop is never
  blocked.
- Background tasks send `Action` messages back (e.g. `SearchSuccess`,
  `EpisodeStreamsReady`, `DownloadCompleted`), which `handle_action` consumes.

## Data flow: a typical search

1. User types a query; the `Key` handler updates `search_query` and sends `Action::Search`.
2. `handle_action` resolves the active provider, dispatches to the provider client
   (async), and spawns the request in a background task.
3. On success the task sends `Action::SearchSuccess`; `handle_action` stores
   `search_results`, marks `dirty`, and writes the provider search cache.
4. Posters for result rows are fetched by background tasks and delivered via
   `SearchPosterLoaded`/`PosterSuccess`; image protocols are cached per terminal.
5. `App::draw` renders the results; `dirty` is cleared.

## Playback flow

1. User selects a result → `Action::PlayStream` (moviebox) or 4KHDHub/BDIX resolve.
2. The provider resolves a `PlaybackSource` (url + optional headers/subtitle).
3. `launch_player` builds the player command (`player.rs`), optionally downloads the
   subtitle to a temp file, and spawns the player in a detached OS session (`setsid` /
   `DETACHED_PROCESS`) with null stdin/stdout and file-backed stderr logging; a
   blocking task waits and reports crashes while the TUI remains open.

## Configuration and persistence

- `config.json`: settings (mode persistence, mode toggles, theme, provider, auto-update, `default_player`, download directory, BDIX) in the config dir.
- `addons_config.json`: installed HTTP community addons in the config dir.
- `tv_config.json`: user M3U playlist sources (URLs or file paths) in the config dir.
- `history.json`: watch history in the system data dir.
- `favorites.json`: starred titles in the system data dir, independent of `history.json`.
- `playback/`: temporary playback states for session crash/kill reconciliation in the system data dir.
- `scripts/`: bundled player scripts (`moviebox_tracker.lua`) in the system data dir.
- Cache lives under the system cache dir, keyed per provider.
- Logs live under the system data dir with rotation.

See `config.md` and `logging.md` for exact locations and formats.
