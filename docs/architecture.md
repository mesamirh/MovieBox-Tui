# Architecture

MovieBox-Tui is a terminal client (ratatui + crossterm + tokio) for streaming movies,
series and TV channels from multiple providers. This document describes the shape of the
code and how data flows through it.

## Module map

```
src/
  main.rs                       entry point: logging init, panic hook, raw mode,
                                alternate screen, App::new + App::run
  lib.rs                        crate root, module declarations
  cache.rs                      disk cache: provider-namespaced, TTL'd, atomic writes
  config.rs                     Config load/save (config.json)
  download.rs                   download engine (resume, ranges, segments, retry)
  favorites.rs                  starred-titles persistence (favorites.json)
  history.rs                    watch history persistence
  logging.rs                    file logging (rotation, sanitization)
  models.rs                     shared domain models (SearchResult, BrowseMetrics, StreamPool,
                                Notification, SubjectIdentity)
  player.rs                     player detection (OnceLock) and command construction (mpv/VLC/IINA/Android)
  providers/
    mod.rs                      provider module tree
    models.rs                   shared typed models (ProviderKind, CatalogItem,
                                MediaDetails, Release, PlaybackSource, …)
    moviebox/                   primary provider (client + request signing)
    fourkhdhub/                 4KHDHub provider (client, hubcloud resolver, parser)
    bdix/circleftp/             BDIX CircleFTP provider
    bdix/dhakaflix/             BDIX DhakaFlix provider
    addons/                     Community HTTP addons provider
    tv/                         Live TV / IPTV provider (M3U parser and models)
  service.rs                    MovieBoxService headless engine (search, details, streams, captions)
  updater/                      GitHub release update check (mod, check, download, verify,
                                extract, apply, artifact)
  tui/
    app/                        the application object and all behavior
      mod.rs                    App struct, App::new, helpers
      run.rs                    terminal event loop (run), rendering (draw),
                                and handle_action dispatcher (thin routing table)
      network.rs                poster fetch + provider dispatch helpers
      search.rs                 search command routing, search request setup,
                                provider search dispatch, poster prefetch
      playback.rs               player launching + playback actions
      download.rs               download orchestration actions
      favorites.rs              favorite toggle/open actions, /favorites virtual list
      requests.rs               suggest/history/homepage/details/preview/
                                episode-stream actions
      navigation.rs             list navigation, submit actions, provider helpers
      tv.rs                     TV mode: playlist manager + playback
      addons.rs                 Addon mode: addon manager + HTTP addon actions
      keyboard.rs               raw key-event handling
      mouse.rs                  mouse click handling and hitbox routing
      system.rs                 help, refresh, cache, theme, updates, focus, resize
    state.rs                    AppState: all UI state + in-memory LRU caches
    action.rs                   the Action enum (event/message model)
    commands.rs                 Slash command registry, parsing, and suggestions
    event.rs                    EventHandler: input events + tick → Action channel
    overlay.rs                  popups, pickers, notifications
    screens/                    render-only modules (home, details, help)
    terminal.rs                 terminal capability probes
    theme.rs                    color themes
    text.rs                     grapheme-safe text helpers
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

## Data flow — a typical search

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
   subtitle to a temp file, and spawns the player with null stdin/stdout and piped
   stderr; a blocking task waits and reports crashes.
4. Playback is handed to mpv / VLC / IINA / Android intent per the active player.

## Configuration and persistence

- `config.json` — settings (mode persistence, mode toggles, theme, provider, auto-update, `default_player`, download directory, BDIX) in the config dir.
- `addons_config.json` — installed HTTP community addons in the config dir.
- `tv_config.json` — user M3U playlist sources (URLs or file paths) in the config dir.
- `history.json` — watch history in the system data dir.
- `favorites.json`: starred titles in the system data dir, independent of `history.json`.
- `playback/` — temporary playback states for session crash/kill reconciliation in the system data dir.
- `scripts/` — bundled player scripts (`moviebox_tracker.lua`) in the system data dir.
- Cache lives under the system cache dir, keyed per provider.
- Logs live under the system data dir with rotation.

See `config.md` and `logging.md` for exact locations and formats.
