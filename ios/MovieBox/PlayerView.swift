import AVFoundation
import AVKit
import SwiftUI
import UIKit

// MARK: - Engine

/// Why one attempt failed.
///
/// `headline` is the sentence the overlay leads with; `detail` carries the error
/// domain, code and underlying reason, which is what distinguishes a container
/// rejection from a 403 or a timeout (S2).
struct PlaybackFailure: Equatable {
    /// What kind of problem this is, so an exhausted chain can stop repeating an
    /// AVFoundation description once the reason is "no picture" (S7).
    enum Kind: Equatable {
        /// The item itself failed; `detail` carries its real error.
        case systemError
        /// The asset had no decodable video track, so it was never played (S6a).
        case noVideoTrack
        /// The item reached `readyToPlay` with no video surface (S6b).
        case audioOnly
    }

    let headline: String
    let detail: String
    var kind: Kind = .systemError
}

/// What the pre-play check could establish about one candidate (S6a).
enum VideoValidation {
    /// The asset answered: the verdict is final for this candidate.
    case checked(Format.VideoVerdict)
    /// The asset never answered (offline, timeout, cancelled). Nothing was
    /// learned, so the candidate is played unchecked and the post-ready guard
    /// stays off for it (S6b).
    case notChecked
}

/// Owns the `AVPlayer` for one playback session: header-authenticated asset,
/// status observation, resume seek and clock publishing. A failed attempt swaps
/// in another candidate through `load(url:headers:startAt:)` instead of giving
/// the player up.
final class PlayerEngine: ObservableObject {
    let player = AVPlayer()

    @Published private(set) var isReady = false
    @Published private(set) var failure: PlaybackFailure?
    /// True while the next candidate's video tracks are being checked, so the
    /// loading overlay can say what the player is waiting for (S6a).
    @Published private(set) var isValidatingSource = false
    @Published private(set) var currentTime: Double = 0
    @Published private(set) var duration: Double = 0
    @Published private(set) var isPlaying = false
    @Published private(set) var didReachEnd = false

    private var startAt: Double
    private var didInitialSeek = false
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var observations: [NSKeyValueObservation] = []
    private var validationTask: Task<Void, Never>?
    /// Only a candidate whose asset reported a decodable video track is held to
    /// the audio-only guard, so a genuine audio-only title whose check was
    /// skipped still plays (S6b).
    private var passedVideoCheck = false
    private var stopped = false

    init(url: URL, headers: [MobileHeader], startAt: Double) {
        self.startAt = startAt

        // Playback category so the stream keeps its audio with the ringer switch off.
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .moviePlayback)
        try? session.setActive(true)

        load(url: url, headers: headers, startAt: startAt)
    }

    deinit {
        stop()
    }

    /// Swaps in another candidate for the same session.
    ///
    /// Every observer and the previous item are released first, so an attempt
    /// that failed leaves no KVO observation or notification behind, and the
    /// same `AVPlayer` keeps driving the already-mounted view controller. The
    /// new item is only installed after its asset reports a video track this
    /// device can decode (S6a).
    func load(url: URL, headers: [MobileHeader], startAt: Double) {
        guard !stopped else { return }
        releaseObservers()
        validationTask?.cancel()
        validationTask = nil

        self.startAt = startAt
        didInitialSeek = false
        isReady = false
        failure = nil
        currentTime = 0
        duration = 0
        isPlaying = false
        didReachEnd = false
        passedVideoCheck = false
        isValidatingSource = true

        let asset = AVURLAsset(url: url, options: Self.options(for: headers))
        // Nothing is mounted while the asset is checked, so AVKit can never show
        // the placeholder for a candidate that would render nothing. The check
        // itself runs off the main actor; only its verdict comes back here.
        validationTask = Task { @MainActor [weak self] in
            let validation = await Self.validate(asset)
            guard !Task.isCancelled, let self else { return }
            self.install(asset: asset, validation: validation)
        }
    }

    /// Stops playback and releases every observer. Safe to call more than once.
    func stop() {
        guard !stopped else { return }
        stopped = true
        validationTask?.cancel()
        validationTask = nil
        isValidatingSource = false
        player.pause()
        releaseObservers()
        player.replaceCurrentItem(with: nil)
        try? AVAudioSession.sharedInstance().setActive(
            false,
            options: .notifyOthersOnDeactivation
        )
    }

    private func releaseObservers() {
        if let timeObserver {
            player.removeTimeObserver(timeObserver)
            self.timeObserver = nil
        }
        if let endObserver {
            NotificationCenter.default.removeObserver(endObserver)
            self.endObserver = nil
        }
        for observation in observations {
            observation.invalidate()
        }
        observations.removeAll()
    }

    private static func options(for headers: [MobileHeader]) -> [String: Any] {
        guard !headers.isEmpty else { return [:] }
        var fields: [String: String] = [:]
        for header in headers {
            fields[header.name] = header.value
        }
        // AVFoundation still reads this key by name on iOS.
        return ["AVURLAssetHTTPHeaderFieldsKey": fields]
    }

    // MARK: Validation

    /// Asks AVFoundation whether the candidate has a video track this device can
    /// decode, off the main thread and without ever throwing into the view.
    private static func validate(_ asset: AVURLAsset) async -> VideoValidation {
        let tracks: [AVAssetTrack]
        do {
            tracks = try await asset.loadTracks(withMediaType: .video)
        } catch {
            // The asset never answered: skip the check rather than reject a
            // candidate for a signal that was never received.
            return .notChecked
        }
        guard !tracks.isEmpty else {
            return .checked(Format.videoVerdict(hasVideoTrack: false, anyDecodable: false))
        }
        var unanswered = 0
        for track in tracks {
            do {
                if try await track.load(.isDecodable) {
                    return .checked(Format.videoVerdict(hasVideoTrack: true, anyDecodable: true))
                }
            } catch {
                unanswered += 1
            }
        }
        // No track said "decodable" but at least one refused to answer: the
        // evidence is incomplete, so treat the check as skipped.
        guard unanswered < tracks.count else { return .notChecked }
        return .checked(Format.videoVerdict(hasVideoTrack: true, anyDecodable: false))
    }

    /// Mounts the checked candidate, or reports an unusable one so the caller
    /// advances through the same bounded chain as a failure (S6a).
    private func install(asset: AVURLAsset, validation: VideoValidation) {
        guard !stopped else { return }
        validationTask = nil
        isValidatingSource = false

        if case .checked(let verdict) = validation, !verdict.isUsable {
            isReady = false
            player.pause()
            failure = PlaybackFailure(
                headline: verdict.reason,
                detail: "AVAssetTrack.isDecodable reported no playable video track for this file.",
                kind: .noVideoTrack
            )
            return
        }
        if case .checked(.usable) = validation {
            passedVideoCheck = true
        }

        let item = AVPlayerItem(asset: asset)
        player.automaticallyWaitsToMinimizeStalling = true
        player.replaceCurrentItem(with: item)
        observe(item: item)
        player.play()
    }

    // MARK: Observation

    private func observe(item: AVPlayerItem) {
        observations.append(
            item.observe(\.status, options: [.new, .initial]) { [weak self] item, _ in
                DispatchQueue.main.async { self?.apply(itemStatus: item.status, item: item) }
            }
        )

        observations.append(
            player.observe(\.timeControlStatus, options: [.new]) { [weak self] player, _ in
                let playing = player.timeControlStatus == .playing
                DispatchQueue.main.async { self?.isPlaying = playing }
            }
        )

        timeObserver = player.addPeriodicTimeObserver(
            forInterval: CMTime(seconds: 0.25, preferredTimescale: 600),
            queue: .main
        ) { [weak self] time in
            self?.handleTick(time)
        }

        endObserver = NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: item,
            queue: .main
        ) { [weak self] _ in
            self?.didReachEnd = true
            self?.isPlaying = false
        }
    }

    private func apply(itemStatus: AVPlayerItem.Status, item: AVPlayerItem) {
        switch itemStatus {
        case .readyToPlay:
            // S6b: a ready item with no video surface renders the AVKit
            // placeholder forever. Only a candidate that passed the pre-play
            // check is held to this, so an uncheckable audio-only title is never
            // interrupted.
            if passedVideoCheck,
               Format.isAudioOnly(presentationHeight: item.presentationSize.height) {
                isReady = false
                player.pause()
                failure = PlaybackFailure(
                    headline: "This source has sound but no picture to show.",
                    detail: "AVPlayerItem.presentationSize has no height after readyToPlay.",
                    kind: .audioOnly
                )
                return
            }
            failure = nil
            isReady = true
            let itemDuration = item.duration.seconds
            if itemDuration.isFinite, itemDuration > 0 {
                duration = itemDuration
            }
            if !didInitialSeek {
                didInitialSeek = true
                if startAt > 1 {
                    let ceiling = duration > 5 ? duration - 5 : duration
                    let target = max(0, min(startAt, ceiling))
                    player.seek(
                        to: CMTime(seconds: target, preferredTimescale: 600),
                        toleranceBefore: .zero,
                        toleranceAfter: .zero
                    )
                }
            }
            player.play()
        case .failed:
            isReady = false
            failure = PlaybackFailure(
                headline: item.error?.localizedDescription
                    ?? "The stream could not be opened. The source may be offline or need different headers.",
                detail: Format.failureDetail(item.error)
            )
        case .unknown:
            break
        @unknown default:
            break
        }
    }

    private func handleTick(_ time: CMTime) {
        let seconds = time.seconds
        if seconds.isFinite, seconds >= 0 {
            currentTime = seconds
        }
        let itemDuration = player.currentItem?.duration.seconds ?? 0
        if itemDuration.isFinite, itemDuration > 0, abs(itemDuration - duration) > 0.5 {
            duration = itemDuration
        }
    }
}

// MARK: - AVPlayerViewController host

private struct PlayerViewControllerHost: UIViewControllerRepresentable {
    let player: AVPlayer

    func makeUIViewController(context: Context) -> AVPlayerViewController {
        let controller = AVPlayerViewController()
        controller.player = player
        controller.showsPlaybackControls = true
        controller.videoGravity = .resizeAspect
        controller.updatesNowPlayingInfoCenter = false
        controller.entersFullScreenWhenPlaybackBegins = false
        return controller
    }

    func updateUIViewController(_ controller: AVPlayerViewController, context: Context) {
        if controller.player !== player {
            controller.player = player
        }
    }
}

// MARK: - Player screen

struct PlayerView: View {
    let request: PlaybackRequest

    @EnvironmentObject private var model: AppModel
    @StateObject private var engine: PlayerEngine
    @StateObject private var subtitles = SubtitleController()
    /// The attempt on screen: `request` first, then each automatic fallback.
    /// The HUD, subtitle lookup and recorded progress all read from here.
    @State private var attempt: PlaybackRequest
    /// Every URL this session tried, so no candidate is attempted twice (S4).
    @State private var attemptedURLs: Set<URL>
    /// Automatic attempts made after the first failure; capped by
    /// `Format.maxFallbackAttempts`.
    @State private var extraAttempts = 0
    /// Set by a close or a "Choose another source" tap: the chain stops (S4).
    @State private var chainCancelled = false
    /// Attempts rejected because the candidate had no playable video track, so
    /// an exhausted chain can say why it ended (S7).
    @State private var videoRejections = 0
    /// Set when the chain stopped without a candidate left, which is what turns
    /// the failure overlay into the exhausted message (S7).
    @State private var chainExhausted = false
    @State private var lastRecordedTime: Double = -1
    @State private var captionNotice: String?
    @State private var onlineTracks: [MobileSubtitle] = []
    @State private var isSearchingOnline = false

    init(request: PlaybackRequest) {
        self.request = request
        _attempt = State(initialValue: request)
        _attemptedURLs = State(initialValue: [request.url])
        _engine = StateObject(
            wrappedValue: PlayerEngine(
                url: request.url,
                headers: request.headers,
                startAt: request.startAt
            )
        )
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            PlayerViewControllerHost(player: engine.player)
                .ignoresSafeArea()

            VStack(spacing: 0) {
                topBar
                Spacer(minLength: 0)
                if subtitles.isEnabled, let text = subtitles.activeText {
                    SubtitleOverlay(text: text)
                        .padding(.bottom, 88)
                        .transition(.opacity)
                }
            }

            if let failure = engine.failure {
                failureOverlay(failure)
            } else if !engine.isReady {
                loadingOverlay
            }
        }
        .preferredColorScheme(.dark)
        .statusBarHidden(true)
        // One lookup per attempt: `resource_id` differs per release, so a
        // fallback has to reload its own tracks (S5).
        .task(id: attempt.id) { await loadSubtitles() }
        .onChange(of: engine.currentTime) { _, time in
            subtitles.update(time: time)
            recordProgressIfNeeded(time)
        }
        .onChange(of: engine.failure) { _, failure in
            guard let failure else { return }
            advance(after: failure)
        }
        .onDisappear {
            chainCancelled = true
            let time = engine.currentTime
            let duration = engine.duration > 0 ? engine.duration : nil
            let completed = isCompleted(time: time, duration: duration)
            engine.stop()
            guard time > 1 else { return }
            Task {
                await model.recordProgress(
                    for: attempt,
                    seconds: time,
                    duration: duration,
                    completed: completed
                )
            }
        }
    }

    // MARK: Overlays

    private var topBar: some View {
        VStack(spacing: 8) {
            HStack(alignment: .top, spacing: 12) {
                Button {
                    closePlayer()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 14, weight: .bold))
                        .padding(10)
                        .background(.black.opacity(0.55), in: Circle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(.white)
                .accessibilityLabel("Close player")

                VStack(alignment: .leading, spacing: 2) {
                    Text(request.title)
                        .font(.caption.weight(.semibold))
                        .lineLimit(1)
                    Text("\(attempt.contextLabel) · \(attempt.releaseLabel)")
                        .font(.caption2)
                        .foregroundStyle(.white.opacity(0.7))
                        .lineLimit(1)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(.black.opacity(0.5), in: Capsule())

                Spacer(minLength: 0)

                captionButton
            }

            if let notice = captionNotice {
                Text(notice)
                    .font(.caption2)
                    .foregroundStyle(.white.opacity(0.85))
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(.black.opacity(0.55), in: Capsule())
                    .transition(.opacity)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 8)
    }

    private var captionButton: some View {
        Menu {
            if subtitles.isEnabled {
                Button {
                    subtitles.disable()
                    captionNotice = nil
                } label: {
                    Label("Turn subtitles off", systemImage: "captions.bubble")
                }
            }
            if subtitles.tracks.isEmpty {
                Text("No subtitles for this release")
            } else {
                Section("This release") {
                    ForEach(subtitles.tracks, id: \.self) { track in
                        trackButton(track)
                    }
                }
            }
            Section("OpenSubtitles") {
                if onlineTracks.isEmpty {
                    Button {
                        Task { await searchOnline() }
                    } label: {
                        Label(
                            isSearchingOnline ? "Searching…" : "Search subtitles online",
                            systemImage: "magnifyingglass"
                        )
                    }
                    .disabled(isSearchingOnline)
                } else {
                    ForEach(onlineTracks, id: \.self) { track in
                        trackButton(track)
                    }
                }
            }
        } label: {
            Group {
                if subtitles.isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .tint(.white)
                } else {
                    Image(systemName: subtitles.isEnabled ? "captions.bubble.fill" : "captions.bubble")
                        .font(.system(size: 15, weight: .semibold))
                }
            }
            .padding(10)
            .background(.black.opacity(0.55), in: Circle())
        }
        .foregroundStyle(.white)
        .accessibilityLabel("Subtitles")
    }

    private var loadingOverlay: some View {
        VStack(spacing: 12) {
            ProgressView()
                .controlSize(.large)
                .tint(.white)
            Text(loadingMessage)
                .font(.footnote)
                .foregroundStyle(.white.opacity(0.75))
            Text(attempt.releaseLabel)
                .font(.caption2)
                .foregroundStyle(.white.opacity(0.5))
        }
        .padding(24)
        .background(.black.opacity(0.45), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }

    /// Says which half of an attempt is running: the video check reads the
    /// asset's tracks before anything is mounted (S6a).
    private var loadingMessage: String {
        if engine.isValidatingSource {
            return attemptedURLs.count > 1 ? "Checking another source…" : "Checking the source…"
        }
        return attemptedURLs.count > 1 ? "Trying another source…" : "Opening stream…"
    }

    private func failureOverlay(_ failure: PlaybackFailure) -> some View {
        // S7: a chain that ended because no candidate had a playable video track
        // says so and reports the count, instead of repeating the last opaque
        // AVFoundation description. A chain that ended on real errors keeps the
        // per-attempt headline and its technical detail (S2).
        let exhaustedNotice = chainExhausted
            ? Format.videoExhaustedNotice(
                triedCount: attemptedURLs.count,
                hadVideoRejection: videoRejections > 0
            )
            : nil

        return VStack(spacing: 14) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 34))
                .foregroundStyle(.yellow)
            Text("Playback failed")
                .font(.headline)
            Text(attempt.releaseLabel)
                .font(.caption.weight(.semibold))
                .foregroundStyle(Theme.accent)
                .lineLimit(1)
            if let exhaustedNotice {
                Text(exhaustedNotice)
                    .font(.footnote)
                    .foregroundStyle(.white.opacity(0.78))
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
                // The last attempt failed for a real system error: keep its
                // technical line, it is the part worth reporting.
                if failure.kind == .systemError {
                    Text(failure.detail)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.white.opacity(0.5))
                        .multilineTextAlignment(.center)
                        .fixedSize(horizontal: false, vertical: true)
                }
            } else {
                Text(failure.headline)
                    .font(.footnote)
                    .foregroundStyle(.white.opacity(0.78))
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
                // Domain, code and the underlying reason: the part that separates a
                // format rejection from a 403 or a timeout (S2).
                Text(failure.detail)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.5))
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
                if attemptedURLs.count > 1 {
                    Text("\(attemptedURLs.count) sources tried")
                        .font(.caption2)
                        .foregroundStyle(.white.opacity(0.5))
                }
            }
            HStack(spacing: 12) {
                Button("Choose another source") {
                    chooseAnotherSource()
                }
                .buttonStyle(.borderedProminent)
                .tint(Theme.accent)

                Button("Close") {
                    closePlayer()
                }
                .buttonStyle(.bordered)
                .tint(.white)
            }
        }
        .padding(24)
        .frame(maxWidth: 420)
        .background(.black.opacity(0.82), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .padding(.horizontal, 24)
    }

    // MARK: Fallback

    /// Called once for every failed attempt: move on to the next untried
    /// candidate of this title, or leave the last real error on screen when the
    /// chain has to stop.
    private func advance(after failure: PlaybackFailure) {
        // A rejection for "no playable video" counts like any other failed
        // attempt: it already cost its URL and its budget slot (S6a).
        if failure.kind != .systemError {
            videoRejections += 1
        }
        guard !chainCancelled,
              let next = Format.nextFallback(
                  alternatives: attempt.alternatives,
                  attemptedURLs: attemptedURLs,
                  extraAttempts: extraAttempts
              ),
              let nextAttempt = PlaybackRequest.attempt(
                  for: next,
                  basedOn: attempt,
                  startAt: resumePosition(),
                  alternatives: Format.fallbackCandidates(after: next, in: attempt.alternatives)
              )
        else {
            // No candidate left or the budget is spent: stop watching the clock
            // and keep the failure the viewer needs to report.
            chainExhausted = true
            subtitles.disable()
            return
        }

        attemptedURLs.insert(nextAttempt.url)
        extraAttempts += 1
        lastRecordedTime = nextAttempt.startAt > 1 ? nextAttempt.startAt : -1
        attempt = nextAttempt
        engine.load(
            url: nextAttempt.url,
            headers: nextAttempt.headers,
            startAt: nextAttempt.startAt
        )
    }

    /// Resume position carried into the next attempt: where the failed one got
    /// to, or the position it was asked to resume at when nothing ever played.
    private func resumePosition() -> Double {
        let live = engine.currentTime
        return max(attempt.startAt, live.isFinite ? live : 0)
    }

    private func closePlayer() {
        chainCancelled = true
        model.dismissPlayer()
    }

    // MARK: Data

    private func loadSubtitles() async {
        // A fallback attempt is a different release: its own tracks replace the
        // previous attempt's cues instead of inheriting them (S5).
        subtitles.disable()
        captionNotice = nil
        guard let resourceId = attempt.resourceId, !resourceId.isEmpty else {
            subtitles.markUnavailable()
            return
        }
        do {
            let tracks = try await model.loadCaptions(
                subjectId: attempt.subjectId,
                resourceId: resourceId,
                siblingIds: attempt.siblingIds,
                season: attempt.season,
                episode: attempt.episode
            )
            // A fallback replaced this attempt while the lookup was running:
            // its answer belongs to a release that is no longer on screen.
            guard !Task.isCancelled else { return }
            subtitles.setTracks(tracks)
            if let first = tracks.first {
                await pick(first)
            }
        } catch {
            guard !Task.isCancelled else { return }
            subtitles.markUnavailable("Subtitles are unavailable for this release.")
        }
    }

    private func trackButton(_ track: MobileSubtitle) -> some View {
        Button {
            Task { await pick(track) }
        } label: {
            if subtitles.selectedTrack == track {
                Label(track.name, systemImage: "checkmark")
            } else {
                Text(track.name)
            }
        }
    }

    /// Searches OpenSubtitles for the title being watched. An empty answer is
    /// reported, never treated as an error: the provider captions stay usable.
    private func searchOnline() async {
        guard !isSearchingOnline else { return }
        isSearchingOnline = true
        defer { isSearchingOnline = false }
        let tracks = await model.searchOnlineSubtitles(
            title: request.title,
            year: request.year,
            isSeries: request.isSeries,
            season: request.season,
            episode: request.episode
        )
        onlineTracks = tracks
        captionNotice = tracks.isEmpty
            ? "No online subtitles found for this title."
            : "\(tracks.count) online subtitles found."
    }

    private func pick(_ track: MobileSubtitle) async {
        await subtitles.select(track)
        if let message = subtitles.statusMessage {
            captionNotice = message
        } else if subtitles.isEnabled {
            let notice = "Subtitles: \(track.name)"
            captionNotice = notice
            Task {
                try? await Task.sleep(nanoseconds: 4_000_000_000)
                if captionNotice == notice {
                    captionNotice = nil
                }
            }
        }
    }

    private func recordProgressIfNeeded(_ time: Double) {
        guard engine.isReady, time > 1 else { return }
        if lastRecordedTime < 0 {
            lastRecordedTime = time
            record(time)
            return
        }
        if abs(time - lastRecordedTime) >= 10 {
            lastRecordedTime = time
            record(time)
        }
    }

    private func record(_ time: Double) {
        let duration = engine.duration > 0 ? engine.duration : nil
        let completed = isCompleted(time: time, duration: duration)
        Task {
            await model.recordProgress(
                for: attempt,
                seconds: time,
                duration: duration,
                completed: completed
            )
        }
    }

    private func isCompleted(time: Double, duration: Double?) -> Bool {
        if engine.didReachEnd { return true }
        guard let duration, duration > 1 else { return false }
        if time >= duration - 20 { return true }
        return time / duration >= 0.95
    }

    private func chooseAnotherSource() {
        let newRequest = SourceRequest(
            subjectId: request.subjectId,
            provider: request.provider,
            title: request.title,
            year: request.year,
            posterUrl: request.posterUrl,
            isSeries: request.isSeries,
            season: request.season,
            episode: request.episode,
            episodeTitle: request.contextLabel,
            startAt: 0,
            siblingIds: request.siblingIds
        )
        // The chain stops here: the viewer is choosing a source themselves (S4).
        chainCancelled = true
        engine.stop()
        model.dismissPlayer()
        Task {
            // Let the full-screen cover finish dismissing before the sheet opens.
            try? await Task.sleep(nanoseconds: 400_000_000)
            model.showSources(newRequest)
        }
    }
}
