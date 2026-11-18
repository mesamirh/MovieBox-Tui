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
