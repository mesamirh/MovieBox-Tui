import SwiftUI

struct LibraryView: View {
    @EnvironmentObject private var model: AppModel

    enum Section: String, CaseIterable, Identifiable {
        case continueWatching
        case favorites

        var id: String { rawValue }

        var title: String {
            switch self {
            case .continueWatching: return "Continue"
            case .favorites: return "Favorites"
            }
        }
    }

    @State private var section: Section = .continueWatching
    @State private var confirmClear = false

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    Picker("Library section", selection: $section) {
                        ForEach(Section.allCases) { entry in
                            Text(entry.title).tag(entry)
                        }
                    }
                    .pickerStyle(.segmented)
                    .padding(.horizontal, 16)

                    content
                }
                .padding(.top, 8)
                .padding(.bottom, 24)
            }
            .background(Theme.background.ignoresSafeArea())
            .navigationTitle("Library")
            .refreshable {
                await model.refreshFavorites()
                await model.refreshHistory()
            }
            .task {
                await model.refreshFavorites()
                await model.refreshHistory()
            }
            .toolbar {
                if section == .continueWatching, !model.historyItems.isEmpty {
                    ToolbarItem(placement: .topBarTrailing) {
                        Menu {
                            Button(role: .destructive) {
                                confirmClear = true
                            } label: {
                                Label("Clear watch history", systemImage: "trash")
                            }
                        } label: {
                            Image(systemName: "ellipsis.circle")
                        }
                        .accessibilityLabel("History options")
                    }
                }
            }
            .confirmationDialog(
                "Clear watch history?",
                isPresented: $confirmClear,
                titleVisibility: .visible
            ) {
                Button("Clear history", role: .destructive) {
                    Task { await model.clearHistory() }
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This removes every saved progress entry. Favorites are kept.")
            }
            .navigationDestination(for: MobileCatalogItem.self) { item in
                DetailsView(item: item)
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        switch section {
        case .continueWatching:
            historyContent
        case .favorites:
            favoritesContent
        }
    }

    // MARK: Favorites

    @ViewBuilder
    private var favoritesContent: some View {
        if model.favoriteItems.isEmpty {
            switch model.favoritesPhase {
            case .idle, .loading:
                LoadingBlock(message: "Loading favorites…")
            default:
                EmptyHint(
                    icon: "heart",
                    title: "No favorites yet",
                    message: "Open a title and tap the heart to keep it here for later."
                )
            }
        } else {
            PosterGrid(items: model.favoriteItems, onRemove: { item in
                Task { await model.removeFavorite(item) }
            })
        }
    }

    // MARK: History

    @ViewBuilder
    private var historyContent: some View {
        if model.historyItems.isEmpty {
            switch model.historyPhase {
            case .idle, .loading:
                LoadingBlock(message: "Loading watch history…")
            default:
                EmptyHint(
                    icon: "clock.arrow.circlepath",
                    title: "Nothing watched yet",
                    message: "Start a movie or an episode and your progress will show up here."
                )
            }
        } else {
            VStack(spacing: 10) {
                ForEach(model.historyItems, id: \.self) { entry in
                    historyRow(entry)
                }
            }
            .padding(.horizontal, 16)

            Text("Tap an entry to resume. Progress is saved every few seconds while you watch.")
                .font(.caption2)
                .foregroundStyle(Theme.subtleText)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)
        }
    }

    private func historyRow(_ entry: MobileHistoryItem) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Button {
                Task { await model.resume(entry) }
            } label: {
                HStack(alignment: .top, spacing: 12) {
                    PosterImage(url: entry.posterUrl, title: entry.title)
                        .frame(width: 54, height: 81)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))

                    VStack(alignment: .leading, spacing: 6) {
                        Text(entry.title)
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.white)
                            .lineLimit(2)
                            .multilineTextAlignment(.leading)

                        HStack(spacing: 6) {
                            if entry.isSeries {
                                Text(Format.episodeLabel(season: entry.season, episode: entry.episode))
                            }
                            if !entry.year.isEmpty {
                                Text(entry.year)
                            }
                            if entry.completed {
                                Label("Watched", systemImage: "checkmark")
                                    .labelStyle(.titleAndIcon)
                                    .font(.system(size: 9, weight: .bold))
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(Theme.accent.opacity(0.22), in: Capsule())
                                    .foregroundStyle(Theme.accent)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(Theme.subtleText)

                        progressLine(entry)
                    }

                    Spacer(minLength: 0)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(accessibilityLabel(entry))

            Menu {
                Button(role: .destructive) {
                    Task { await model.removeHistory(entry) }
                } label: {
                    Label("Remove from history", systemImage: "trash")
                }
            } label: {
                Image(systemName: "ellipsis")
                    .font(.footnote.weight(.bold))
                    .foregroundStyle(Theme.subtleText)
                    .padding(8)
            }
            .accessibilityLabel("Options for \(entry.title)")
        }
        .padding(10)
        .background(Theme.surface, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    @ViewBuilder
    private func progressLine(_ entry: MobileHistoryItem) -> some View {
        if let duration = entry.durationSeconds, duration > 0 {
            ProgressView(value: Format.progress(entry))
                .tint(Theme.accent)
                .scaleEffect(x: 1, y: 0.7, anchor: .center)
            Text("\(Format.clock(Double(entry.progressSeconds))) of \(Format.clock(Double(duration)))")
                .font(.caption2)
                .foregroundStyle(Theme.subtleText)
        } else if entry.progressSeconds > 0 {
            Text("\(Format.clock(Double(entry.progressSeconds))) watched")
                .font(.caption2)
                .foregroundStyle(Theme.subtleText)
        }
    }

    private func accessibilityLabel(_ entry: MobileHistoryItem) -> String {
        var parts = [entry.title]
        if entry.isSeries {
            parts.append(Format.episodeLabel(season: entry.season, episode: entry.episode))
        }
        if entry.completed {
            parts.append("watched")
        } else if let duration = entry.durationSeconds, duration > 0 {
            let percent = Int(Format.progress(entry) * 100)
            parts.append("\(percent) percent watched")
        } else {
            parts.append("\(Format.clock(Double(entry.progressSeconds))) watched")
        }
        return "Resume " + parts.joined(separator: ", ")
    }
}
