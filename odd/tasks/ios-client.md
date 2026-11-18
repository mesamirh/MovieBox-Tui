# iOS client (feat/ios-client)

Native iOS application for MovieBox, sharing the Rust provider layer as the single source of truth.

## Specs

**S1.** The repository must produce a native iOS app that runs on iPhone/iPad and offers the MovieBox catalog: search, home/trending browse, title details, seasons and episodes, source and resolution selection, subtitles, favorites and watch history — full functional parity with the existing `roku/` channel.

**S2.** The iOS client must share the existing Rust provider / API / crypto implementation through UniFFI bindings, instead of reimplementing the MovieBox wire contract a third time in Swift.

**S3.** The existing terminal TUI must keep working unchanged: identical behavior, identical default build, no new runtime requirement.

**S4.** "Analiza e itera todas las veces necesarias parea que esto funcione" — the deliverable must be verified by actually building and running in the iOS simulator, iterating until playback works, not by inspection alone.

## Tasks

| ID | Task | Spec | Status | Commit |
| --- | --- | --- | --- | --- |
| T1 | Spike: prove the crate links as an iOS staticlib | S2, S3 | done — device and simulator both link | a483b05 |
| T2 | Extract TUI-free helpers to `src/text_util.rs`; unlink `service.rs`/`history.rs` from `crate::tui::text` | S2, S3 | done — 428 lib tests pass | a483b05 |
| T3 | Feature-gate `tui` out of the mobile build | S2, S3 | dropped — see L5 | — |
| T4 | UniFFI surface `src/ffi/`: records + catalog/details/streams/captions client | S1, S2 | done | 779426a |
| T5 | FFI favorites + watch history | S1 | done | 779426a |
| T6 | `ios/scripts/` build-rust, build-app, run-simulator | S2 | done | e00d04e |
| T6b | Smoke test: staticlib links and runs in the simulator | S4 | done — real search + 4 playable releases | — |
| T7 | SwiftUI app `ios/MovieBox/`: home, search, details, episodes, player | S1, S4 | done | e00d04e |
| T8 | SwiftUI: subtitles, resolution picker, favorites, history | S1 | done | e00d04e |
| T9 | Verify: boot simulator, iterate until a stream plays | S4 | done — video playing, see L7 | e00d04e |
| T10 | Docs + CI job for the iOS build | S3 | done | c44e3f7 |

## Contract notes for T7/T8

Home tab ids, taken from the Roku channel's navigation so both clients agree:
`"1"` Home, `"2"` Movies, `"5"` TV Shows, `"8"` Anime.

Playback: `MobileRelease.isDirectFile == true` means the URL is a real media file.
`false` means a DASH/HLS manifest that AVPlayer cannot open as-is, so the UI must
prefer direct files and surface manifests only as a fallback.

## Log

**L1** (user, verbatim): "quiero que me hagas la version para IOS. Analiza e itera todas las veces necesarias parea que esto funcione"

**L2** (user decision — architecture): Rust core + UniFFI + SwiftUI shell. Rationale accepted by the user: `providers/`, `net.rs`, `models.rs` are ratatui-free and reusable as-is; requires feature-gating `tui`, `crate-type = ["staticlib"]`, removing the `crate::tui::text` coupling in `service.rs` and `history.rs`, adding UniFFI and the `aarch64-apple-ios` target. Trade-off accepted: this touches shared code the TUI consumes (high-risk item 3), so differential checks on the TUI are required.

**L3** (user decision — scope): Full functional parity with the Roku channel: search, home/trending, details, seasons/episodes, resolution and subtitle selection, favorites and on-disk history.

**L4** (evidence, exploration): `roku/` is an independent BrightScript reimplementation with zero shared code (branch `feat/roku-channel`, not on `main`). `src/` is byte-identical between `main` and that branch, so `feat/ios-client` branches from `main`. Toolchain confirmed present: Xcode 27.2, iOS SDK 27.2, simulator runtime iOS 26.5, Swift 6.4.2.

**L5** (evidence — T3 dropped): the unmodified crate already compiles and links for `aarch64-apple-ios` and `aarch64-apple-ios-sim`; `crossterm` and `ratatui` build on iOS because it is a unix target. Feature-gating `tui` would add risk to shared code the TUI consumes with no proven benefit, because the linker drops unreferenced TUI objects. Only the `moviebox-tui` **binary** cannot link for device, which the build script avoids with `--lib`. T2 is still required: without it `service.rs`/`history.rs` reference `crate::tui::text`, pulling `crossterm` into the FFI dependency chain.

**L6** (evidence — bridge proven): the smoke app ran in the iOS 26.5 simulator and returned 10 live search results plus 4 real stream candidates (1080p/720p/480p) for `Avatar`, proving Rust staticlib → UniFFI → Swift → network works end to end. Observation that shaped the FFI: one candidate was a DASH `.mpd`, which AVPlayer cannot play, so `MobileRelease.is_direct_file` was added and `release_from` now prefers a direct-file mirror.

**L7** (verification, S4): playback confirmed visually in the iOS 26.5 simulator. A deep link with `autoplay=1` resolved four direct-file sources and AVPlayer rendered video (HUD: `Avatar [Hindi] · Movie · 1080p · Ayuti Ye Dire Konjo`). Finding fixed on the way: `AppModel.play` parked the request in `pendingPlayback` waiting for the source sheet's `onDismiss` to promote it, so any caller with no sheet on screen stalled forever; it now presents the player directly when no sheet is up.

**L8** (delivery note — slice boundary): the feature is 4410 authored lines across four commits and cannot be split further without breaking compilability, because the app depends on the bridge. If it goes to review, split it as two chained PRs at this boundary: (1) `a483b05` + `779426a` — text_util decoupling and the UniFFI bridge, reviewable and testable in Rust alone; (2) `e00d04e` + `c44e3f7` — the SwiftUI app, scripts, docs and CI.

**L9** (user decision, 2nd request): "Convierte en proyecto IOS de xcode" — replace the script-only build with a real Xcode project. Done in `181a1e3`. Three things had to be right: per-SDK `LIBRARY_SEARCH_PATHS` (a single list links the device archive into the simulator build), `EXCLUDED_ARCHS[sdk=iphonesimulator*] = x86_64` (the Rust archive has no x86_64 simulator slice), and committing `ios/Generated/` (Xcode resolves source references while planning, before the Build Rust core phase could generate them). Verified with `xcodebuild` for the simulator and `generic/platform=iOS`, and the Xcode-built bundle plays video.

**L10** (delivery note — revised slice boundary). Five commits, 7760 authored lines. Two chained PRs at this boundary: (1) `a483b05` text_util decoupling + `779426a` UniFFI bridge — reviewable and testable in Rust alone; (2) `e00d04e` SwiftUI app and scripts + `c44e3f7` docs and CI + `181a1e3` Xcode project — the app delivery. Untracked `roku/` (leftover from `feat/roku-channel`) and `odd/` are intentionally not committed and currently block a native review assess, which requires an explicit untracked declaration.
