# iOS playback failure diagnostics and source fallback

Playback can fail on a specific candidate while another candidate of the same
title would play. Today the player shows a bare "Cannot Open" and stops, so the
viewer cannot tell a broken file from a network problem, and the app cannot
recover on its own.

## Specs

**S1 (user evidence, verbatim)** — screenshot of `The Boss Baby`, HUD
`Movie · 1080p · 1080p hevc`, overlay `Playback failed` / `Cannot Open`.

The 1080p HEVC candidate is chosen (correctly: no H.264 was offered) and
AVFoundation refuses the container. The next candidate must be reachable without
the viewer guessing.

**S2 — the failure must be legible.** The overlay reports the underlying
`AVError` domain and code plus the item's error description, next to the release
label. `Cannot Open` alone cannot distinguish a format rejection from a 403 or a
timeout.

**S3 — automatic fallback.** When an item fails to open, the player tries the
next candidate of the same title, in the order `Format.playableDirectFiles`
already produced, up to two extra attempts. The viewer sees which source is being
tried. When every candidate fails, the overlay shows the last real error and the
count of sources tried.

**S4 — the fallback is bounded and never loops.** No candidate is retried twice,
no attempt happens without a remaining candidate, and a user-initiated close or a
"Choose another source" tap cancels the chain.

**S5 — progress and subtitles survive the switch.** The resume position of the
failed attempt is carried into the next one, recorded progress still comes from
the candidate the viewer actually watched, and the subtitle tracks are reloaded
for the new release because `resource_id` differs per release.

**S6 (user evidence, 2026-11-18, second screenshot)** — "ahora se quedo asi": after the
fallback advanced from `1080p hevc` to the next candidate (`Carla Bastos`), AVKit rendered
its video placeholder and nothing else. So a candidate can be `readyToPlay`, show no video,
and never fail. Waiting for a `.failed` status is not enough.

**S6a — validate the candidate's video before playing it.** Before installing an item, load
the asset's video tracks. A candidate with no video track, or with no video track the system
reports as decodable (`AVAssetTrack.isDecodable`), is unusable and must be replaced through
the existing bounded fallback chain instead of being played. Use the AVFoundation signal, not
a hardcoded fourCC list: `hev1`/`av01`/`vp09` all surface through it without a table to
maintain.

**S6b — a played item with no video surface still counts as unusable.** After `readyToPlay`,
an item whose `presentationSize` has no height is delivering audio only; it must advance like
a failure instead of sitting on the placeholder.

**S7 — the exhausted message says what happened.** When every candidate is unusable, the
overlay says the title has no source with a playable video track and how many candidates were
tried, rather than the last opaque AVFoundation description.

## Log

- **L1 (user, verbatim)** — "/Users/jesustorres/Downloads/Screenshot 2026-11-18 at 2.53.34 PM.png me pasa esto" — the screenshot shows `The Boss Baby`, `Movie · 1080p · 1080p hevc`, `Playback failed / Cannot Open`.
- **L2 (evidence)** — `PlayerEngine.apply(itemStatus:)` stores only `item.error?.localizedDescription` (`ios/MovieBox/PlayerView.swift`), and `PlaybackRequest` carries one URL with no candidate list, so nothing can advance after a failure. `Format.playableDirectFiles` already returns the ordered candidates in both construction sites (`ios/MovieBox/AppModel.swift`, `ios/MovieBox/SourcePickerView.swift`).
- **L4 (evidence)** — `AVAssetTrack.isDecodable` is the supported answer to "will this track play", and `AVPlayerItem.presentationSize` is zero-height for an audio-only item. Both are async loads on iOS 17 (`load(_:)`), matching the SDK the project builds against.
- **L3 (decision)** — automatic fallback rather than a manual-only retry: the viewer asked for playback, and a container rejection is not something they can diagnose. Bounded at two extra attempts per S4.
