import AVFoundation
import AVKit
import SwiftUI
import UIKit

// MARK: - Engine

/// Owns the `AVPlayer` for one playback session: header-authenticated asset,
/// status observation, resume seek and clock publishing.
final class PlayerEngine: ObservableObject {
    let player = AVPlayer()

    @Published private(set) var isReady = false
    @Published private(set) var failure: String?
    @Published private(set) var currentTime: Double = 0
    @Published private(set) var duration: Double = 0
    @Published private(set) var isPlaying = false
    @Published private(set) var didReachEnd = false

    private let startAt: Double
    private var didInitialSeek = false
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var observations: [NSKeyValueObservation] = []
    private var stopped = false

    init(url: URL, headers: [MobileHeader], startAt: Double) {
        self.startAt = startAt

        var options: [String: Any] = [:]
        if !headers.isEmpty {
            var fields: [String: String] = [:]
            for header in headers {
                fields[header.name] = header.value
            }
            // AVFoundation still reads this key by name on iOS.
            options["AVURLAssetHTTPHeaderFieldsKey"] = fields
        }

        // Playback category so the stream keeps its audio with the ringer switch off.
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .moviePlayback)
        try? session.setActive(true)

        let asset = AVURLAsset(url: url, options: options)
        let item = AVPlayerItem(asset: asset)
        player.automaticallyWaitsToMinimizeStalling = true
        player.replaceCurrentItem(with: item)
        observe(item: item)
        player.play()
    }

    deinit {
        stop()
    }

    /// Stops playback and releases every observer. Safe to call more than once.
    func stop() {
        guard !stopped else { return }
        stopped = true
        player.pause()
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
        player.replaceCurrentItem(with: nil)
        try? AVAudioSession.sharedInstance().setActive(
            false,
            options: .notifyOthersOnDeactivation
        )
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
            failure = item.error?.localizedDescription
                ?? "The stream could not be opened. The source may be offline or need different headers."
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
    @State private var lastRecordedTime: Double = -1
    @State private var captionNotice: String?
    @State private var onlineTracks: [MobileSubtitle] = []
    @State private var isSearchingOnline = false

    init(request: PlaybackRequest) {
        self.request = request
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
        .task { await loadSubtitles() }
        .onChange(of: engine.currentTime) { _, time in
            subtitles.update(time: time)
            recordProgressIfNeeded(time)
        }
        .onChange(of: engine.failure) { _, failure in
            if failure != nil {
                subtitles.disable()
            }
        }
        .onDisappear {
            let time = engine.currentTime
            let duration = engine.duration > 0 ? engine.duration : nil
            let completed = isCompleted(time: time, duration: duration)
            engine.stop()
            guard time > 1 else { return }
            Task {
                await model.recordProgress(
                    for: request,
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
                    model.dismissPlayer()
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
                    Text("\(request.contextLabel) · \(request.releaseLabel)")
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
            Text("Opening stream…")
                .font(.footnote)
                .foregroundStyle(.white.opacity(0.75))
            Text(request.releaseLabel)
                .font(.caption2)
                .foregroundStyle(.white.opacity(0.5))
        }
        .padding(24)
        .background(.black.opacity(0.45), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }

    private func failureOverlay(_ message: String) -> some View {
        VStack(spacing: 14) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 34))
                .foregroundStyle(.yellow)
            Text("Playback failed")
                .font(.headline)
            Text(message)
                .font(.footnote)
                .foregroundStyle(.white.opacity(0.78))
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 12) {
                Button("Choose another source") {
                    chooseAnotherSource()
                }
                .buttonStyle(.borderedProminent)
                .tint(Theme.accent)

                Button("Close") {
                    model.dismissPlayer()
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

    // MARK: Data

    private func loadSubtitles() async {
        guard let resourceId = request.resourceId, !resourceId.isEmpty else {
            subtitles.markUnavailable()
            return
        }
        do {
            let tracks = try await model.loadCaptions(
                subjectId: request.subjectId,
                resourceId: resourceId,
                siblingIds: request.siblingIds,
                season: request.season,
                episode: request.episode
            )
            subtitles.setTracks(tracks)
            if let first = tracks.first {
                await pick(first)
            }
        } catch {
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
                for: request,
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
        engine.stop()
        model.dismissPlayer()
        Task {
            // Let the full-screen cover finish dismissing before the sheet opens.
            try? await Task.sleep(nanoseconds: 400_000_000)
            model.showSources(newRequest)
        }
    }
}
