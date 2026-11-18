import CoreGraphics
import SwiftUI

// MARK: - Theme

enum Theme {
    static let background = Color(red: 0.04, green: 0.04, blue: 0.05)
    static let surface = Color(red: 0.11, green: 0.11, blue: 0.13)
    static let surfaceRaised = Color(red: 0.16, green: 0.16, blue: 0.19)
    static let accent = Color(red: 0.96, green: 0.28, blue: 0.32)
    static let subtleText = Color.white.opacity(0.62)
}

// MARK: - Formatting

enum Format {
    static func bytes(_ value: UInt64?) -> String? {
        guard let value, value > 0 else { return nil }
        return ByteCountFormatter.string(fromByteCount: Int64(value), countStyle: .file)
    }

    static func clock(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds > 0 else { return "0:00" }
        let total = Int(seconds.rounded())
        let hours = total / 3600
        let minutes = (total % 3600) / 60
        let secs = total % 60
        if hours > 0 {
            return String(format: "%d:%02d:%02d", hours, minutes, secs)
        }
        return String(format: "%d:%02d", minutes, secs)
    }

    static func progress(_ entry: MobileHistoryItem) -> Double {
        guard let duration = entry.durationSeconds, duration > 0 else {
            return entry.completed ? 1 : 0
        }
        return min(1, Double(entry.progressSeconds) / Double(duration))
    }

    static func episodeLabel(season: UInt32, episode: UInt32) -> String {
        "S\(season) E\(episode)"
    }

    /// Rough quality ordering, so 1080p sorts above 720p and 4K above both.
    static func qualityRank(_ quality: String?) -> Int {
        guard let quality, !quality.isEmpty else { return 0 }
        let lowered = quality.lowercased()
        if lowered.contains("4k") || lowered.contains("2160") { return 2160 }
        let digits = quality.compactMap { $0.isNumber ? $0 : nil }
        if digits.isEmpty { return 0 }
        return Int(String(digits)) ?? 0
    }

    /// How reliably the system player decodes a codec: lower opens first.
    ///
    /// `AVPlayer` decodes H.264 on every device it runs on, while MovieBox's 4K
    /// entries are usually 10-bit HEVC or Dolby Vision and can play their audio
    /// with no picture at all. AV1/VP9 have no hardware path here, so they rank
    /// last. An unlabelled codec sits in the middle: unknown, not promoted.
    static func codecRank(_ codec: String?) -> Int {
        guard let codec else { return 1 }
        let normalized = codec.lowercased().filter { $0.isLetter || $0.isNumber }
        guard !normalized.isEmpty else { return 1 }
        if normalized.contains("avc") || normalized.contains("h264") || normalized.contains("x264") {
            return 0
        }
        if normalized.contains("av1") || normalized.contains("vp9") {
            return 2
        }
        return 1
    }

    /// Resolution preference for playback, lower first: 1080p, 720p, then
    /// everything else, with 4K last because the 4K candidates are the ones
    /// that arrive as 10-bit HEVC or Dolby Vision.
    static func playbackQualityRank(_ quality: String?) -> Int {
        let rank = qualityRank(quality)
        if rank >= 2160 { return 3 }
        if rank >= 1080 { return 0 }
        if rank >= 720 { return 1 }
        return 2
    }

    /// Direct files the system player can open, best candidate first.
    ///
    /// Codec wins over resolution, resolution breaks the codec tie, and the
    /// provider order breaks everything left so the order stays deterministic.
    static func playableDirectFiles(_ releases: [MobileRelease]) -> [MobileRelease] {
        releases
            .filter { $0.isDirectFile && $0.url?.isEmpty == false }
            .enumerated()
            .sorted { left, right in
                let leftRank = codecRank(left.element.codec) * 4
                    + playbackQualityRank(left.element.quality)
                let rightRank = codecRank(right.element.codec) * 4
                    + playbackQualityRank(right.element.quality)
                if leftRank != rightRank { return leftRank < rightRank }
                return left.offset < right.offset
            }
            .map(\.element)
    }

    /// HUD label for one candidate, e.g. "1080p · Direct".
    static func releaseLabel(_ release: MobileRelease) -> String {
        let label = [release.quality, release.sourceLabel]
            .compactMap { $0 }
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
        return label.isEmpty ? release.sourceLabel : label
    }

    // MARK: - Playback fallback

    /// Extra sources tried after the first one fails. Bounded so a broken title
    /// cannot walk through every candidate it has (S3/S4).
    static let maxFallbackAttempts = 2

    /// The candidates to try after `release` fails, in the order
    /// `playableDirectFiles` produced: only the ones that come after it, and
    /// never the URL already playing.
    static func fallbackCandidates(
        after release: MobileRelease,
        in candidates: [MobileRelease]
    ) -> [MobileRelease] {
        let remaining: ArraySlice<MobileRelease>
        if let index = candidates.firstIndex(of: release) {
            remaining = candidates[(index + 1)...]
        } else {
            // The chosen release is not from this list: keep all of it, the URL
            // filter below still drops the one already playing.
            remaining = candidates[...]
        }
        return remaining.filter { $0.url != release.url }
    }

    /// The next source to try after a failed attempt, or `nil` when the chain
    /// must stop.
    ///
    /// Stops when the attempt budget is spent, when no candidate is left, and
    /// for any candidate whose URL this session already tried.
    static func nextFallback(
        alternatives: [MobileRelease],
        attemptedURLs: Set<URL>,
        extraAttempts: Int
    ) -> MobileRelease? {
        guard extraAttempts < maxFallbackAttempts else { return nil }
        return alternatives.first { release in
            guard let raw = release.url, let url = URL(string: raw) else { return false }
            return !attemptedURLs.contains(url)
        }
    }

    // MARK: - Video suitability

    /// What the asset says about its own picture, before anything is played
    /// (S6a). Apart from `usable`, the case itself is the reason the candidate
    /// is replaced through the fallback chain.
    enum VideoVerdict: Equatable {
        /// At least one video track reports `isDecodable`.
        case usable
        /// The asset carries no video track at all.
        case noVideoTrack
        /// Video tracks exist, but the system reports none of them decodable.
        case undecodable

        var isUsable: Bool { self == .usable }

        /// Sentence the overlay leads with when this candidate is rejected.
        var reason: String {
            switch self {
            case .usable:
                return "This source has a decodable video track."
            case .noVideoTrack:
                return "This source has no video track."
            case .undecodable:
                return "This device cannot decode this source’s video."
            }
        }
    }

    /// Decide whether a candidate can put a picture on screen from what
    /// AVFoundation reported about its tracks.
    ///
    /// The signal is `AVAssetTrack.isDecodable`, never a list of fourCCs, so
    /// `hev1`, `av01` and `vp09` all reach the same rule.
    static func videoVerdict(hasVideoTrack: Bool, anyDecodable: Bool) -> VideoVerdict {
        guard hasVideoTrack else { return .noVideoTrack }
        return anyDecodable ? .usable : .undecodable
    }

    /// A mounted item whose presentation size has no height is delivering audio
    /// only, which AVKit renders as a bare placeholder (S6b).
    static func isAudioOnly(presentationHeight: CGFloat) -> Bool {
        !(presentationHeight > 0)
    }

    /// Headline for an exhausted chain that ran out of candidates with a
    /// playable video track, or `nil` when the chain ended on real errors and
    /// the last error has to stay on screen (S7).
    static func videoExhaustedNotice(triedCount: Int, hadVideoRejection: Bool) -> String? {
        guard hadVideoRejection else { return nil }
        let sources = triedCount == 1 ? "1 source" : "\(triedCount) sources"
        return "No source of this title has a playable video track — \(sources) tried."
    }

    /// Technical one-liner for a failed playback attempt: the error domain and
    /// code plus the underlying description, which is what tells a container
    /// rejection apart from a 403 or a timeout (S2).
    static func failureDetail(_ error: Error?) -> String {
        guard let error else { return "No error detail reported by the system player." }
        let failure = error as NSError
        var parts = ["\(failure.domain) \(failure.code)"]
        var described = false
        if let underlying = failure.userInfo[NSUnderlyingErrorKey] as? NSError {
            if underlying.domain != failure.domain {
                parts.append("\(underlying.domain) \(underlying.code)")
            }
            let description = underlying.localizedDescription
            if !description.isEmpty {
                parts.append(description)
                described = true
            }
        }
        if let reason = failure.localizedFailureReason, !reason.isEmpty {
            parts.append(reason)
            described = true
        }
        if !described, !failure.localizedDescription.isEmpty {
            // No underlying reason to report: the item's own sentence is all
            // the system gave us, and the overlay must never be detail-less.
            parts.append(failure.localizedDescription)
        }
        return parts.joined(separator: " · ")
    }
}

// MARK: - Poster image

struct PosterImage: View {
    let url: String?
    let title: String

    var body: some View {
        ZStack {
            Rectangle().fill(Theme.surface)
            if let raw = url,
               !raw.isEmpty,
               let parsed = URL(string: raw) {
                AsyncImage(url: parsed, transaction: Transaction(animation: .easeInOut(duration: 0.15))) { phase in
                    switch phase {
                    case .success(let image):
                        image
                            .resizable()
                            .scaledToFill()
                    case .failure:
                        placeholder
                    case .empty:
                        ProgressView()
                            .tint(.white.opacity(0.55))
                    @unknown default:
                        placeholder
                    }
                }
            } else {
                placeholder
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipped()
        .accessibilityHidden(true)
        .accessibilityLabel("Poster for \(title)")
    }

    private var placeholder: some View {
        Image(systemName: "film")
            .font(.title3)
            .foregroundStyle(.white.opacity(0.3))
    }
}

// MARK: - Poster card + grid

struct PosterCard: View {
    let item: MobileCatalogItem

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            PosterImage(url: item.posterUrl, title: item.title)
                .aspectRatio(2.0 / 3.0, contentMode: .fit)
                .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .strokeBorder(.white.opacity(0.08), lineWidth: 0.5)
                )

            Text(item.title)
                .font(.footnote.weight(.semibold))
                .foregroundStyle(.primary)
                .lineLimit(2)
                .multilineTextAlignment(.leading)

            HStack(spacing: 6) {
                if let year = item.year, !year.isEmpty {
                    Text(year)
                }
                if item.isSeries {
                    Text("SERIES")
                        .font(.system(size: 9, weight: .bold))
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .background(Theme.surfaceRaised, in: Capsule())
                }
            }
            .font(.caption2)
            .foregroundStyle(Theme.subtleText)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAddTraits(.isButton)
    }

    private var accessibilityLabel: String {
        var parts = [item.title]
        if item.isSeries {
            parts.append("series")
        } else {
            parts.append("movie")
        }
        if let year = item.year, !year.isEmpty {
            parts.append(year)
        }
        return parts.joined(separator: ", ")
    }
}

struct PosterGrid: View {
    let items: [MobileCatalogItem]
    var onReachEnd: (() -> Void)?
    var onRemove: ((MobileCatalogItem) -> Void)?

    private let columns = [
        GridItem(.adaptive(minimum: 104, maximum: 168), spacing: 14, alignment: .top)
    ]

    var body: some View {
        LazyVGrid(columns: columns, alignment: .leading, spacing: 18) {
            ForEach(items, id: \.self) { item in
                cell(for: item)
                    .onAppear {
                        if item == items.last {
                            onReachEnd?()
                        }
                    }
            }
        }
        .padding(.horizontal, 16)
    }

    @ViewBuilder
    private func cell(for item: MobileCatalogItem) -> some View {
        let link = NavigationLink(value: item) {
            PosterCard(item: item)
        }
        .buttonStyle(.plain)

        if let onRemove {
            link.contextMenu {
                Button(role: .destructive) {
                    onRemove(item)
                } label: {
                    Label("Remove from favorites", systemImage: "heart.slash")
                }
            }
        } else {
            link
        }
    }
}

// MARK: - State blocks

struct LoadingBlock: View {
    var message: String = "Loading…"

    var body: some View {
        VStack(spacing: 12) {
            ProgressView()
                .controlSize(.large)
                .tint(.white)
            Text(message)
                .font(.footnote)
                .foregroundStyle(Theme.subtleText)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 48)
    }
}

struct ErrorBlock: View {
    let message: String
    var retry: (() -> Void)?

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "wifi.exclamationmark")
                .font(.system(size: 30))
                .foregroundStyle(Theme.accent)
            Text("Couldn’t load this")
                .font(.headline)
            Text(message)
                .font(.footnote)
                .foregroundStyle(Theme.subtleText)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            if let retry {
                Button("Try again", action: retry)
                    .buttonStyle(.borderedProminent)
                    .tint(Theme.accent)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 32)
        .padding(.vertical, 48)
    }
}

struct EmptyHint: View {
    let icon: String
    let title: String
    let message: String

    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 32))
                .foregroundStyle(Theme.subtleText)
            Text(title)
                .font(.headline)
            Text(message)
                .font(.footnote)
                .foregroundStyle(Theme.subtleText)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 32)
        .padding(.vertical, 48)
    }
}

// MARK: - Small pieces

struct TagChip: View {
    let text: String
    var tint: Color = Theme.surfaceRaised

    var body: some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .lineLimit(1)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(tint, in: Capsule())
    }
}

struct BannerView: View {
    let message: String
    let onDismiss: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Text(message)
                .font(.footnote)
                .foregroundStyle(.white)
                .lineLimit(2)
            Spacer(minLength: 0)
            Button(action: onDismiss) {
                Image(systemName: "xmark")
                    .font(.caption.weight(.bold))
            }
            .buttonStyle(.plain)
            .foregroundStyle(.white.opacity(0.7))
            .accessibilityLabel("Dismiss message")
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        .padding(.horizontal, 16)
    }
}
