import SwiftUI

/// Resolution / source picker. Direct media files are listed first because the
/// system player can open them; manifests are shown disabled with an explanation.
struct SourcePickerView: View {
    let request: SourceRequest

    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss

    @State private var phase: LoadPhase = .loading
    @State private var releases: [MobileRelease] = []

    private var playable: [MobileRelease] {
        Format.playableDirectFiles(releases)
    }

    private var manifests: [MobileRelease] {
        releases.filter { !($0.isDirectFile && $0.url?.isEmpty == false) }
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    header

                    switch phase {
                    case .idle, .loading:
                        LoadingBlock(message: "Finding sources…")
                    case .failed(let message):
                        ErrorBlock(message: message) {
                            Task { await load(force: true) }
                        }
                    case .loaded:
                        if releases.isEmpty {
                            EmptyHint(
                                icon: "antenna.radiowaves.left.and.right.slash",
                                title: "No sources",
                                message: "The provider returned no stream candidates for this title. Try another episode or come back later."
                            )
                        } else {
                            if playable.isEmpty {
                                ErrorBlock(
                                    message: "Only manifest streams (DASH/HLS) were offered. This player can’t open those directly, so nothing here is playable."
                                )
                            } else {
                                sectionHeader("Playable")
                                VStack(spacing: 10) {
                                    ForEach(playable, id: \.self) { release in
                                        playableRow(release)
                                    }
                                }
                            }

                            if !manifests.isEmpty {
                                sectionHeader("Not playable in this player")
                                VStack(spacing: 10) {
                                    ForEach(manifests, id: \.self) { release in
                                        manifestRow(release)
                                    }
                                }
                                Text("These are DASH or HLS manifests. The system player needs a direct media file, so they are listed for reference only.")
                                    .font(.caption2)
                                    .foregroundStyle(Theme.subtleText)
                                    .padding(.top, 2)
                            }
                        }
                    }
                }
                .padding(.horizontal, 16)
                .padding(.top, 12)
                .padding(.bottom, 28)
            }
            .background(Theme.background.ignoresSafeArea())
            .navigationTitle("Sources")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { dismiss() }
                }
            }
            .task { await load() }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
    }

    private var header: some View {
        HStack(alignment: .top, spacing: 12) {
            PosterImage(url: request.posterUrl, title: request.title)
                .frame(width: 52, height: 78)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))

            VStack(alignment: .leading, spacing: 4) {
                Text(request.title)
                    .font(.headline)
                    .lineLimit(2)
                Text(request.contextLabel)
                    .font(.caption)
                    .foregroundStyle(Theme.subtleText)
                    .lineLimit(2)
                if request.startAt > 0 {
                    Text("Resuming at \(Format.clock(request.startAt))")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(Theme.accent)
                }
            }
            Spacer(minLength: 0)
        }
    }

    private func sectionHeader(_ text: String) -> some View {
        Text(text.uppercased())
            .font(.caption2.weight(.bold))
            .foregroundStyle(Theme.subtleText)
    }

    private func playableRow(_ release: MobileRelease) -> some View {
        Button {
            start(release)
        } label: {
            rowContent(release, enabled: true)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Play \(release.quality ?? "source") \(release.sourceLabel)")
    }

    private func manifestRow(_ release: MobileRelease) -> some View {
        rowContent(release, enabled: false)
            .opacity(0.55)
            .accessibilityLabel("Unavailable manifest source \(release.sourceLabel)")
    }

    private func rowContent(_ release: MobileRelease, enabled: Bool) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: enabled ? "play.circle.fill" : "xmark.circle")
                .font(.title2)
                .foregroundStyle(enabled ? Theme.accent : Theme.subtleText)
                .padding(.top, 2)

            VStack(alignment: .leading, spacing: 6) {
                Text(release.filename)
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(.white)
                    .lineLimit(2)
                    .truncationMode(.middle)
                    .multilineTextAlignment(.leading)

                HStack(spacing: 6) {
                    if let quality = release.quality, !quality.isEmpty {
                        TagChip(text: quality, tint: Theme.accent.opacity(0.25))
                    }
                    if let codec = release.codec, !codec.isEmpty {
                        TagChip(text: codec)
                    }
                    if let language = release.language, !language.isEmpty {
                        TagChip(text: language.uppercased())
                    }
                    if let size = Format.bytes(release.sizeBytes) {
                        TagChip(text: size)
                    }
                    if release.resourceId != nil {
                        TagChip(text: "CC")
                    }
                }

                HStack(spacing: 6) {
                    Text(release.sourceLabel)
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(Theme.subtleText)
                    if !enabled {
                        Text("· manifest, not openable by AVPlayer")
                            .font(.caption2)
                            .foregroundStyle(Theme.subtleText)
                    }
                }
            }

            Spacer(minLength: 0)
        }
        .padding(12)
        .background(Theme.surface, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .contentShape(Rectangle())
    }

    // MARK: - Data

    private func load(force: Bool = false) async {
        if !force, phase == .loaded { return }
        phase = .loading
        do {
            let found = try await model.loadReleases(
                subjectId: request.subjectId,
                season: request.season,
                episode: request.episode
            )
            releases = found
            phase = .loaded
        } catch {
            phase = .failed(BridgeError.message(error))
        }
    }

    private func start(_ release: MobileRelease) {
        guard let raw = release.url, let url = URL(string: raw) else { return }
        let playback = PlaybackRequest(
            url: url,
            headers: release.headers,
            title: request.title,
            contextLabel: request.contextLabel,
            posterUrl: request.posterUrl,
            provider: request.provider,
            subjectId: request.subjectId,
            isSeries: request.isSeries,
            year: request.year,
            season: request.season,
            episode: request.episode,
            resourceId: release.resourceId,
            startAt: request.startAt,
            releaseLabel: Format.releaseLabel(release),
            siblingIds: request.siblingIds,
            alternatives: Format.fallbackCandidates(after: release, in: playable)
        )
        model.play(playback)
    }
}
