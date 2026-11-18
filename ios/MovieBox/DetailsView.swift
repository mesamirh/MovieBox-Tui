import SwiftUI

private struct CreditRow: Identifiable {
    let label: String
    let value: String
    var id: String { label }
}

struct DetailsView: View {
    let item: MobileCatalogItem

    @EnvironmentObject private var model: AppModel
    @State private var details: MobileDetails?
    @State private var phase: LoadPhase = .loading
    @State private var favorite = false
    @State private var favoriteBusy = false
    @State private var seasonNumber: UInt32 = 0

    var body: some View {
        ScrollView {
            if let details {
                loaded(details)
            } else {
                switch phase {
                case .failed(let message):
                    ErrorBlock(message: message) {
                        Task { await load(force: true) }
                    }
                default:
                    LoadingBlock(message: "Loading title…")
                }
            }
        }
        .background(Theme.background.ignoresSafeArea())
        .navigationTitle(item.title)
        .navigationBarTitleDisplayMode(.inline)
        .task { await load() }
        .onAppear {
            guard let details else { return }
            Task { await refreshFavorite(details) }
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private func loaded(_ details: MobileDetails) -> some View {
        VStack(alignment: .leading, spacing: 18) {
            header(details)
            actions(details)
            if !details.genres.isEmpty {
                genreRow(details.genres)
            }
            if let overview = details.overview, !overview.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Overview")
                        .font(.subheadline.weight(.semibold))
                    Text(overview)
                        .font(.footnote)
                        .foregroundStyle(.white.opacity(0.82))
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(.horizontal, 16)
            }
            credits(details)
            if details.isSeries {
                episodesSection(details)
            }
        }
        .padding(.top, 12)
        .padding(.bottom, 32)
    }

    private func header(_ details: MobileDetails) -> some View {
        HStack(alignment: .top, spacing: 14) {
            PosterImage(url: details.posterUrl ?? item.posterUrl, title: details.title)
                .frame(width: 120, height: 180)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .strokeBorder(.white.opacity(0.08), lineWidth: 0.5)
                )

            VStack(alignment: .leading, spacing: 8) {
                Text(details.title)
                    .font(.title3.weight(.bold))
                    .lineLimit(3)
                    .fixedSize(horizontal: false, vertical: true)

                if let tagline = details.tagline, !tagline.isEmpty {
                    Text(tagline)
                        .font(.caption.italic())
                        .foregroundStyle(Theme.subtleText)
                        .lineLimit(3)
                }

                HStack(spacing: 8) {
                    if let year = details.year, !year.isEmpty {
                        TagChip(text: year)
                    }
                    if details.isSeries {
                        TagChip(text: "Series")
                    } else {
                        TagChip(text: "Movie")
                    }
                    if let rating = details.rating, !rating.isEmpty {
                        HStack(spacing: 3) {
                            Image(systemName: "star.fill")
                                .font(.system(size: 9))
                                .foregroundStyle(.yellow)
                            Text(rating)
                        }
                        .font(.caption2.weight(.semibold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Theme.surfaceRaised, in: Capsule())
                    }
                }

                if let runtime = details.runtime, !runtime.isEmpty {
                    Text(runtime)
                        .font(.caption)
                        .foregroundStyle(Theme.subtleText)
                }

                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 16)
    }

    private func actions(_ details: MobileDetails) -> some View {
        HStack(spacing: 12) {
            if !details.isSeries {
                Button {
                    model.showSources(
                        model.sourceRequest(from: details, season: 0, episode: 0, episodeTitle: nil)
                    )
                } label: {
                    Label("Play", systemImage: "play.fill")
                        .font(.subheadline.weight(.semibold))
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 10)
                }
                .buttonStyle(.borderedProminent)
                .tint(Theme.accent)
            }

            Button {
                Task { await toggleFavorite(details) }
            } label: {
                Label(
                    favorite ? "In favorites" : "Favorite",
                    systemImage: favorite ? "heart.fill" : "heart"
                )
                .font(.subheadline.weight(.semibold))
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
            }
            .buttonStyle(.bordered)
            .tint(favorite ? Theme.accent : .white)
            .disabled(favoriteBusy)
            .accessibilityLabel(favorite ? "Remove from favorites" : "Add to favorites")
        }
        .padding(.horizontal, 16)
    }

    private func genreRow(_ genres: [String]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(genres, id: \.self) { genre in
                    TagChip(text: genre)
                }
            }
            .padding(.horizontal, 16)
        }
    }

    @ViewBuilder
    private func credits(_ details: MobileDetails) -> some View {
        let rows: [CreditRow] = [
            details.director.map { CreditRow(label: "Director", value: $0) },
            details.stars.map { CreditRow(label: "Stars", value: $0) }
        ]
        .compactMap { $0 }
        .filter { !$0.value.isEmpty }

        if !rows.isEmpty {
            VStack(alignment: .leading, spacing: 6) {
                ForEach(rows) { row in
                    HStack(alignment: .top, spacing: 8) {
                        Text(row.label)
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.subtleText)
                            .frame(width: 62, alignment: .leading)
                        Text(row.value)
                            .font(.caption)
                            .foregroundStyle(.white.opacity(0.85))
                    }
                }
            }
            .padding(.horizontal, 16)
        }
    }

    @ViewBuilder
    private func episodesSection(_ details: MobileDetails) -> some View {
        let season = details.seasons.first { $0.number == seasonNumber } ?? details.seasons.first

        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Episodes")
                    .font(.headline)
                Spacer()
                if details.seasons.count > 1 {
                    Picker("Season", selection: $seasonNumber) {
                        ForEach(details.seasons, id: \.number) { item in
                            Text("Season \(item.number)").tag(item.number)
                        }
                    }
                    .pickerStyle(.menu)
                    .tint(.white)
                    .accessibilityLabel("Choose season")
                } else if let season {
                    Text("Season \(season.number)")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(Theme.subtleText)
                }
            }

            if let season, !season.episodes.isEmpty {
                VStack(spacing: 0) {
                    ForEach(season.episodes, id: \.self) { episode in
                        episodeRow(episode, details: details)
                        if episode != season.episodes.last {
                            Divider().background(Color.white.opacity(0.08))
                        }
                    }
                }
                .background(Theme.surface, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            } else {
                Text("No episodes are listed for this season yet.")
                    .font(.footnote)
                    .foregroundStyle(Theme.subtleText)
                    .padding(.vertical, 8)
            }
        }
        .padding(.horizontal, 16)
    }

    private func episodeRow(_ episode: MobileEpisode, details: MobileDetails) -> some View {
        Button {
            model.showSources(
                model.sourceRequest(
                    from: details,
                    season: episode.season,
                    episode: episode.number,
                    episodeTitle: episode.title
                )
            )
        } label: {
            HStack(alignment: .top, spacing: 12) {
                Text("\(episode.number)")
                    .font(.footnote.weight(.bold))
                    .foregroundStyle(Theme.subtleText)
                    .frame(width: 26, alignment: .trailing)

                VStack(alignment: .leading, spacing: 3) {
                    Text(episodeLabel(episode))
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.white)
                        .lineLimit(2)
                    if let overview = episode.overview, !overview.isEmpty {
                        Text(overview)
                            .font(.caption)
                            .foregroundStyle(Theme.subtleText)
                            .lineLimit(2)
                    }
                }

                Spacer(minLength: 8)

                Image(systemName: "play.circle.fill")
                    .font(.title3)
                    .foregroundStyle(Theme.accent)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 12)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Play season \(episode.season) episode \(episode.number)")
    }

    // MARK: - Data

    private func load(force: Bool = false) async {
        if !force, details != nil { return }
        phase = .loading
        do {
            let loaded = try await model.loadDetails(subjectId: item.id)
            details = loaded
            if loaded.seasons.contains(where: { $0.number == seasonNumber }) == false {
                seasonNumber = loaded.seasons.first?.number ?? 0
            }
            await refreshFavorite(loaded)
            phase = .loaded
        } catch {
            phase = .failed(BridgeError.message(error))
        }
    }

    private func episodeLabel(_ episode: MobileEpisode) -> String {
        if let title = episode.title, !title.isEmpty {
            return title
        }
        return "Episode \(episode.number)"
    }

    private func refreshFavorite(_ details: MobileDetails) async {
        favorite = await model.isFavorite(
            subjectId: details.id,
            provider: details.provider
        )
    }

    private func toggleFavorite(_ details: MobileDetails) async {
        guard !favoriteBusy else { return }
        favoriteBusy = true
        let catalogItem = MobileCatalogItem(
            id: details.id,
            provider: details.provider,
            title: details.title,
            isSeries: details.isSeries,
            year: details.year,
            posterUrl: details.posterUrl
        )
        if favorite {
            await model.removeFavorite(catalogItem)
            favorite = false
        } else {
            favorite = await model.toggleFavorite(catalogItem)
        }
        favoriteBusy = false
    }
}
