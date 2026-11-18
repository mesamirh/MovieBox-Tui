# iOS Client

`ios/` contains a native SwiftUI app that exposes the MovieBox catalog on iPhone and
iPad. It is not a fourth implementation of the wire contract: the Swift layer never
talks to MovieBox, it drives the same Rust provider engine the terminal client uses,
through UniFFI bindings generated from this crate.

This page documents what exists in the repository today: the `mobile` feature boundary,
the FFI surface, the build pipeline, deep links and the known limitations.

## Why the core is shared

`providers/`, `net.rs` and `models.rs` are ratatui-free, so they can be reused as-is by a
non-terminal front end. Everything expensive or fragile about this project already lives
in those modules:

- request signing (HMAC-MD5 signature, client token, spoofed device identity),
- CloudFront cookie handling and mirror resolution,
- the typed provider models and the aggregation logic in `src/service.rs`.

Reimplementing that in Swift would mean maintaining the same anti-bot and provider quirks
twice, and any provider change would have to be ported by hand. Instead the iOS app links
`libmoviebox_tui.a` and calls `MobileClient` (`src/ffi.rs`), so both clients break and get
fixed together.

The terminal client is unaffected: the default build is unchanged, the `moviebox-tui`
binary never links the bridge, and the TUI behaves exactly as before (`S3`).

## The `mobile` feature boundary

`Cargo.toml` gains three things, all inert unless the `mobile` feature is enabled:

```toml
[lib]
crate-type = ["rlib", "staticlib"]

[features]
mobile = ["dep:uniffi"]

[[bin]]
name = "uniffi-bindgen"
path = "src/bin/uniffi-bindgen.rs"
required-features = ["mobile"]
```

- `src/ffi.rs` is declared `#[cfg(feature = "mobile")]` in `src/lib.rs`, so a normal
  `cargo build` neither compiles nor links UniFFI.
- The `staticlib` crate type is what iOS consumes; `rlib` is kept for the binary, tests
  and the existing consumers.
- `uniffi-bindgen` is a feature-gated binary used only to regenerate Swift/C bindings.

One refactor made the boundary possible: the TUI-free helpers
`sanitize_language_label`, `format_subtitle_label`, `format_duration` and
`parse_duration_seconds` moved from `src/tui/text.rs` to `src/text_util.rs`
(`src/tui/text.rs` re-exports them, so TUI call sites are unchanged). Without that move,
`src/service.rs` and `src/history.rs` would pull `crossterm` into the FFI dependency
chain.

The device build uses `--lib` on purpose — the `moviebox-tui` binary is a terminal
application and cannot link for iOS.

## FFI surface

Everything lives in `src/ffi.rs`. The Swift side sees one object, its records and one
error type; nothing terminal-specific is exposed.

| Type | Kind | Contents |
| --- | --- | --- |
| `MobileClient` | object | Catalog, details, streams, captions, favorites, history. |
| `MobileError` | error (`flat_error`) | A single `message` string. |
| `MobileCatalogItem` | record | `id`, `provider`, `title`, `isSeries`, `year`, `posterUrl`. |
| `MobileDetails` | record | Poster/title/year/rating/runtime/genres/credits/overview plus `seasons`. |
| `MobileSeason` | record | `number`, `episodes`. |
| `MobileEpisode` | record | `season`, `number`, `title`, `overview`. |
| `MobileRelease` | record | `filename`, quality/codec/language/size, `url`, `sourceLabel`, `headers`, `resourceId`, `isDirectFile`. |
| `MobileHeader` | record | `name`, `value` — forwarded to the player. |
| `MobileSubtitle` | record | `name`, `url`. |
| `MobileHistoryItem` | record | Provider/subject identity, title, poster, `season`, `episode`, `progressSeconds`, `durationSeconds`, `completed`. |

`MobileClient` methods:

| Method | Purpose |
| --- | --- |
| `new(configDir:cacheDir:dataDir:)` | Builds the client against explicit stores, so iOS keeps everything inside its app sandbox. |
| `search(query:page:)` | Catalog search results. |
| `suggest(query:)` | Search-as-you-type suggestions. |
| `home(tabId:page:)` | Home rows. Tab ids match the Roku channel: `"1"` Home, `"2"` Movies, `"5"` TV Shows, `"8"` Anime. |
| `details(subjectId:)` | Full metadata, including the season/episode tree. |
| `releases(subjectId:season:episode:)` | Stream candidates. Movies pass `season = 0, episode = 0`, the same convention as the terminal client. |
| `captions(subjectId:resourceId:season:episode:)` | Subtitle tracks for a release whose `resourceId` is known. |
| `favorites()` / `isFavorite(...)` / `toggleFavorite(...)` / `removeFavorite(...)` | On-disk favorites store. |
| `history()` / `recordProgress(...)` / `removeHistoryEntry(...)` / `clearHistory()` | On-disk watch history, including completed markers. |

Each exported method is synchronous on the foreign side and runs its async work on a
dedicated multi-thread Tokio runtime. That keeps UniFFI's async plumbing out of Swift,
but it also means no call may run on the main actor — `ios/MovieBox/Bridge.swift` holds
one `MobileClient` per process behind a serial queue and wraps every call in a
`withCheckedContinuation`.

## Build pipeline

The build is script-driven; no Xcode project is committed.

| Script | What it does | Output |
| --- | --- | --- |
| `ios/scripts/build-rust.sh` | Installs the iOS targets, builds the static library for `aarch64-apple-ios` (device) and `aarch64-apple-ios-sim` (simulator) with `--features mobile --lib`, regenerates the Swift bindings, then assembles the XCFramework. | `ios/Generated/` (`moviebox_tui.swift`, `moviebox_tuiFFI.h`, `bridging.h`) and `target/MovieBox.xcframework` |
| `ios/scripts/build-app.sh` | Compiles every `ios/MovieBox/**/*.swift` file with `swiftc` against the iOS Simulator SDK, links the simulator static library and the generated bindings, then writes the bundle. | `target/ios/arm64/MovieBox.app` |
| `ios/scripts/run-simulator.sh` | Resolves and boots a simulator device (default `iPhone 17 Pro`), installs and launches `com.moviebox.ios` with `--console-pty` so the Rust logs stream to the terminal. | running app |

```bash
./ios/scripts/build-rust.sh
./ios/scripts/build-app.sh
./ios/scripts/run-simulator.sh
```

`ios/Generated/` is UniFFI output: never edit it by hand, `build-rust.sh` rewrites it.
`build-app.sh` re-type-checks the Swift sources, so it must be re-run after any Swift
change.

## Deep links and automation

The bundle registers the `moviebox` URL scheme (`CFBundleURLTypes` in
`ios/MovieBox/Info.plist`):

```text
moviebox://play/<subjectId>?season=<n>&episode=<n>&autoplay=1
```

- Without `autoplay`, the details screen loads and the source picker is presented.
- With `autoplay` (`1`, `true` or `yes`), the first direct-file release is resolved and
  played in one step. If nothing is directly playable the picker opens with an explicit
  banner instead of failing silently.
- `season`/`episode` default to the first season and episode of a series; movies ignore
  them.

The same handler also runs at launch, which is how automation and screenshots reach
playback without tapping:

```bash
SIMCTL_CHILD_MOVIEBOX_OPEN_URL="moviebox://play/<id>?autoplay=1" \
  xcrun simctl launch <udid> com.moviebox.ios
```

`-MovieBoxOpenURL <url>` is accepted as a launch argument too. Both entry points live in
`LaunchOptions` (`ios/MovieBox/MovieBoxApp.swift`).

## What the app does

| Screen | Behaviour |
| --- | --- |
| Browse | Poster grid for the four catalog tabs, paginated and pull-to-refresh. |
| Search | Debounced search with suggestions when nothing matches, plus empty and error states. |
| Details | Metadata and credits, favorite toggle, season picker and episode list, Play button. |
| Sources | Every `releases(...)` candidate with quality, codec, language, size and source label. Direct files are listed first and are the only tappable ones. |
| Player | `AVPlayerViewController` on the selected release URL, with provider headers forwarded through `AVURLAssetHTTPHeaderFieldsKey`. Failures offer "Choose another source". |
| Subtitles | Caption tracks from `captions(...)`, fetched over HTTP and parsed as SRT or WebVTT into a timed overlay with a CC toggle. |
| Library | Continue Watching (progress bars, completed markers, resume, single-entry removal, clear-all) and Favorites, both from the on-disk Rust stores. |
| Progress | `recordProgress(...)` every ten seconds while playing and once on close, marking the item completed near the end. |

## Known limitations

- **DASH and HLS manifests are unplayable.** AVPlayer cannot open an arbitrary `.mpd` or
  HLS manifest on its own, so releases with `isDirectFile == false` are listed but
  disabled in the picker. Titles that only offer manifests stay unplayable until a
  remuxing path exists.
- **Subtitles are an overlay, not embedded tracks.** AVPlayer cannot attach an arbitrary
  remote subtitle file to a stream, so cues are parsed and drawn above the video against
  the player clock.
- **App Transport Security is relaxed.** `NSAllowsArbitraryLoads` is enabled because
  stream CDNs are served over plain HTTP with provider-specific headers.
- **Sideload/simulator oriented.** There is no App Store distribution path, no code
  signing configuration and no Xcode project in the repository; the build is entirely
  script-driven. Device static libraries link, but the packaged bundle targets the
  simulator.

## See also

- `ios/README.md` — app-level build, run and troubleshooting notes.
- [architecture.md](architecture.md) — where the `mobile` feature attaches to the data flow.
- [modules.md](modules.md) — `ffi.rs` and `text_util.rs` in the module tree.
- [cross-platform.md](cross-platform.md) — platform matrix.
