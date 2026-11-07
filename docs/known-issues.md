# Known issues and limitations

Tracked here so future work and issue reports reference the same facts.

## Latent / by-design

- **`supports_headers` routes authenticated streams via loopback proxy.** Sources that
  carry authentication headers (e.g. MovieBox CloudFront signed cookies) automatically
  route through the local detached HTTP loopback proxy (`src/proxy.rs`) for both VLC and
  Android Intent playback, streaming video chunks over localhost without requiring player-side
  cookie support.
- **MovieBox request signing** hardcodes an API secret and spoofs a device identity in
  `crypto.rs`. This is inherent to the scraper; treat the module as one unit.
- **Android intent playback** forwards `User-Agent`, `Referer`, and subtitles (`subtitles_location`/`subs`) when using `am` or `termux-am`. Chooser-based launches via `termux-open` open the raw video URL directly.

## Environment-dependent

- **4KHDHub mirrors rotate and can be expired on upstream hosts.** Direct streams are resolved concurrently across all available release mirrors using prioritized scoring (Cloudflare R2 / S3 / Seekable Streams → Storage → PixelDrain API → Google UserContent / Direct Attachments) and bounded concurrency (`select_ok` in chunks of 3). When all upstream mirrors for an older release are dead/expired (e.g. 404 or expired tokens), the resolver fails fast (<4.5s) and guides the user to select another release.
- **Termux playback depends on Android intent dispatchers**: Verify `termux-open` / `am` availability and the Android player chooser behavior on target devices.
- **Windows 11 Smart App Control (SAC) restricts untrusted web downloads.** Unsigned binaries downloaded via web browsers are tagged with `ZoneId=3` (Mark of the Web), causing Windows 11 SAC to block launch without a standard "Run anyway" button. The official installer (`install.ps1`) strips this tag automatically using `Unblock-File`. For manual ZIP downloads, users must unblock via file properties or `Unblock-File moviebox-tui.exe`.

## Verification

- Automated testing is enforced via `cargo test --all-features --locked` covering 568 unit and integration tests across 12 test targets (the library and 11 integration-test binaries; see [`docs/testing.md`](testing.md)). The count is updated when tests change; it does not replace real-player and real-device verification.
- Static correctness is enforced by strict compiler type checking, the lint gate (`cargo clippy --all-targets --all-features --locked -- -D warnings`), formatting (`cargo fmt --check`), dependency vulnerability scanning (`cargo audit`), and packaging verification (`cargo package --locked`).
- Runtime and platform-specific behavior (terminal resize, focus handling, external player launch, and Termux chooser) are verified through the release checklist in [`release-checklist.md`](release-checklist.md).
