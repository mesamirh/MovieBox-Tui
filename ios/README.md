# MovieBox for iOS

A native SwiftUI client for the MovieBox catalog. It shares the Rust provider,
API and crypto layer with the terminal client through UniFFI instead of
reimplementing the wire contract: the Swift app never talks to MovieBox itself,
it only drives `MobileClient`.

Deployment target: iOS 17.0. Language mode: Swift 5. Languages: SwiftUI, AVKit,
AVFoundation, Foundation — no third-party dependencies.

## Prerequisites

- Xcode 16 or newer with the iOS 17+ SDK and an iOS simulator runtime.
- Rust with the iOS targets (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`).
- `cargo`, plus the `uniffi-bindgen` binary target that ships in this repository.

## Build and run in Xcode

Open the project and hit Run:

```bash
open ios/MovieBox.xcodeproj
```

Pick the `MovieBox` scheme and any iPhone simulator. The target is fully
configured: bundle id `com.moviebox.ios`, iOS 17.0 deployment target, Swift 5
language mode, device family iPhone + iPad, and the generated UniFFI sources are
already referenced.

The Rust side is built by the target's first build phase, `Build Rust core`. It
is a no-op while `target/aarch64-apple-ios{,-sim}/release/libmoviebox_tui.a`
exist, so ordinary UI builds stay fast. Delete those files (or run
`ios/scripts/build-rust.sh` yourself) after changing anything under `src/`.

**Running on a physical device**: select your team under *Signing &
Capabilities* — the project uses automatic signing with no team preset. The
`Info.plist` already declares the `moviebox` URL scheme, App Transport Security
for provider CDNs and the local-network usage string.

## Build from the command line

The same project builds headless, which is what CI uses:

```bash
xcodebuild -project ios/MovieBox.xcodeproj -scheme MovieBox \
  -configuration Debug -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' build
```

There is also a `swiftc` path that needs no Xcode project at all, useful for a
quick check or for environments without one:

```bash
./ios/scripts/build-rust.sh   # Rust core + bindings (device and simulator)
./ios/scripts/build-app.sh    # compiles to target/ios/arm64/MovieBox.app
./ios/scripts/run-simulator.sh  # installs and launches it
```

`build-app.sh` compiles every `ios/MovieBox/**/*.swift` file directly, links
`target/aarch64-apple-ios-sim/release/libmoviebox_tui.a`, and writes
`target/ios/arm64/MovieBox.app`. Re-run it after any Swift change.

To use a different simulator device, pass a name:

```bash
xcrun simctl list devices available          # find a device
./ios/scripts/run-simulator.sh "iPhone 17"
```

Or install manually:

```bash
xcrun simctl install booted target/ios/arm64/MovieBox.app
xcrun simctl launch --console-pty booted com.moviebox.ios
```

`--console-pty` streams the Rust core's logs, which is the fastest way to see
provider failures. The app writes its config, cache and data stores under its
Application Support directory inside the simulator sandbox.

## Tests

`MovieBoxTests` is a hosted XCTest bundle (`TEST_HOST` pointed at `MovieBox.app`)
so it can use `@testable import MovieBox` and exercise the app's real types. Run
it against a booted simulator:

```bash
xcodebuild test -project ios/MovieBox.xcodeproj -scheme MovieBox \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -derivedDataPath /tmp/mb-test-dd
```

The suite covers the deterministic logic only, because every network and Rust
call sits behind `Bridge` and needs a live provider:

- `SubtitleParser` — SRT and WebVTT cue parsing, timestamp conversion, BOM/CRLF
  tolerance, ASS/HTML/entity cleanup, malformed input and the 20 000-cue bound,
  plus the active-cue lookup at its exact boundaries.
- `Format` — clock, byte-size, episode label, quality rank and progress helpers.
- Pure `AppModel` value types — `LoadPhase`, `HomeFeed`, `SourceRequest`, and the
  `SubtitleError` messages shown to the viewer.

Views, `Bridge` and `AppModel`'s async loaders need a simulator run instead:
there is no offline double for the UniFFI client.

## What the app does

| Screen | Behaviour |
| --- | --- |
| Browse | Four catalog tabs matching the Roku channel — Home (`1`), Movies (`2`), TV Shows (`5`), Anime (`8`) — as a poster grid with title and year, paginated and pull-to-refresh. |
| Search | Debounced and submit-on-return search with a results grid, type-ahead suggestions when nothing matches, and explicit empty/error states. |
| Details | Poster, title, year, rating, runtime, genres, tagline, overview, credits; favorite toggle; season picker and episode list for series; a Play button for movies. |
| Sources | Every `releases(...)` candidate with quality, codec, language, size and source label. Direct files come first, ordered by what the system player decodes most reliably — H.264/1080p before 4K HEVC or Dolby Vision — and are the only ones you can tap; DASH/HLS manifests are shown disabled with an explanation. |
| Player | `AVPlayerViewController` driven by the selected release URL. Provider headers are forwarded through `AVURLAssetHTTPHeaderFieldsKey`. Failures show a message with "Choose another source" instead of a blank screen. |
| Subtitles | Caption tracks from `captions(...)` — including the dub siblings the details response exposes — plus an on-demand online search (`searchOnlineSubtitles(...)`) that queries OpenSubtitles for the title being watched and lists Spanish before English before the rest. Fetched over HTTP, parsed as SRT or WebVTT and rendered as a timed overlay above the video. Toggleable from the CC button; malformed files and empty searches degrade to a notice, never a crash. |
| Library | Continue Watching from the on-disk history with progress bars, completed markers, resume, single-entry removal and clear-all; Favorites from the on-disk store with removal and details navigation. |
| Progress | While playing, progress is written with `recordProgress(...)` every ten seconds and once more on close, marking the item completed near the end. |

## Architecture

The generated UniFFI client is synchronous and blocks on network or disk, so no
call may run on the main actor.

- `Bridge.swift` — one `MobileClient` per process and a serial dispatch queue.
  Every call is wrapped in a `withCheckedContinuation`, which keeps the UI
  responsive and matches the Rust client's one-caller-at-a-time expectation.
- `AppModel.swift` — `@MainActor` observable state: catalog feeds, search,
  favorites, history, the source-picker request, the playback request and
  progress recording.
- `MovieBoxApp.swift` — app entry, tab shell, source sheet and player
  presentation.
- `HomeView.swift`, `SearchView.swift`, `DetailsView.swift`,
  `SourcePickerView.swift`, `PlayerView.swift`, `SubtitleOverlay.swift`,
  `LibraryView.swift` — one screen per file.
- `Components.swift` — poster cards/grid, loading/error/empty blocks, chips,
  formatting helpers.
- `ios/Generated/` — UniFFI output; never edit by hand, `build-rust.sh` rewrites it.

## Notes and limitations

- **Manifests.** Providers sometimes only offer DASH `.mpd` or HLS `.m3u8`
  manifests. AVPlayer cannot open those as files, so the source picker marks them
  unusable instead of failing silently during playback.
- **Codec order.** Autoplay and the source picker share `Format.playableDirectFiles`,
  which ranks H.264 above HEVC above AV1/VP9 and prefers 1080p over 4K. MovieBox's
  4K entries are often 10-bit HEVC or Dolby Vision: AVPlayer reports
  `readyToPlay`, plays the audio and never draws a picture, so a decodable
  candidate has to win over a higher-numbered one. When HEVC is the only direct
  file it is still played.
- **Subtitles are an overlay, not an AVPlayer track.** AVPlayer cannot attach an
  arbitrary remote SRT to a stream, so cues are synced against the player clock
  and drawn above the video.
- **App Transport Security.** `Info.plist` allows arbitrary loads because stream
  CDNs are served over plain HTTP with provider-specific headers.
- **No offline mode.** Catalog, details and sources always come from the network;
  only favorites and history are persisted locally by the Rust core.
- The device (`aarch64-apple-ios`) static library builds and links, but this
  repository's scripts only package and launch the simulator app.

## Troubleshooting

- `error: .../libmoviebox_tui.a is missing` — run `./ios/scripts/build-rust.sh`.
- `error: Swift bindings are missing` — same fix; `build-rust.sh` regenerates
  `ios/Generated/moviebox_tui.swift`.
- A title lists only manifest sources — pick another title, or check
  `xcrun simctl launch --console-pty` for the provider's response.
- Playback fails immediately with an HTTP error — the signed stream URL may have
  expired; reopen the source picker to resolve fresh candidates.
