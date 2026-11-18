import Foundation
import SwiftUI

/// Coarse state of one async load, shared by every screen so they can render
/// consistent loading, error and empty states.
enum LoadPhase: Equatable {
    case idle
    case loading
    case loaded
    case failed(String)

    var errorMessage: String? {
        if case .failed(let message) = self { return message }
        return nil
    }
}

/// Paginated catalog feed for one home tab.
struct HomeFeed {
    var items: [MobileCatalogItem] = []
    var phase: LoadPhase = .idle
    /// Last page that was successfully loaded (0 = nothing yet).
    var page: UInt32 = 0
    var canLoadMore = true
    var isLoadingMore = false
}

/// A request to resolve playable sources, shown as the source picker sheet.
struct SourceRequest: Identifiable, Hashable {
    let id = UUID()
    let subjectId: String
    let provider: String
    let title: String
    let year: String
    let posterUrl: String?
    let isSeries: Bool
    let season: UInt32
    let episode: UInt32
    let episodeTitle: String?
    /// Resume position in seconds (0 for a fresh start).
    let startAt: Double
    /// Dub `subject_id`s, forwarded to the caption lookup so every dub's
    /// captions reach the picker the way they already do in the terminal.
    var siblingIds: [String] = []

    /// "S1 E3" or "Movie", used by the source picker header.
    var contextLabel: String {
        if isSeries {
            var label = "Season \(season) · Episode \(episode)"
            if let episodeTitle, !episodeTitle.isEmpty {
                label += " · \(episodeTitle)"
            }
            return label
        }
        return "Movie"
    }
}

/// A resolved, directly playable stream handed to the player.
struct PlaybackRequest: Identifiable, Hashable {
    let id = UUID()
    let url: URL
    let headers: [MobileHeader]
    let title: String
    let contextLabel: String
    let posterUrl: String?
    let provider: String
    let subjectId: String
    let isSeries: Bool
    let year: String
    let season: UInt32
    let episode: UInt32
    let resourceId: String?
    let startAt: Double
    /// Quality / source label for the player HUD, e.g. "1080p · Direct".
    let releaseLabel: String
    /// Dub `subject_id`s, forwarded to the caption lookup.
    var siblingIds: [String] = []
    /// Ordered candidates for the same title still untried, in the order
    /// `Format.playableDirectFiles` produced. The player walks them when the
    /// current stream fails; they are not part of the request's identity.
    var alternatives: [MobileRelease] = []
}

// MARK: - Building attempts

extension PlaybackRequest {
    /// A request for one more candidate of the title `base` already describes.
    ///
    /// `startAt` is the resume position to carry into the new attempt and
    /// `alternatives` are the candidates left after this one.
    static func attempt(
        for release: MobileRelease,
        basedOn base: PlaybackRequest,
        startAt: Double,
        alternatives: [MobileRelease]
    ) -> PlaybackRequest? {
        guard let raw = release.url, let url = URL(string: raw) else { return nil }
        return PlaybackRequest(
            url: url,
            headers: release.headers,
            title: base.title,
            contextLabel: base.contextLabel,
            posterUrl: base.posterUrl,
            provider: base.provider,
            subjectId: base.subjectId,
            isSeries: base.isSeries,
            year: base.year,
            season: base.season,
            episode: base.episode,
            resourceId: release.resourceId,
            startAt: startAt,
            releaseLabel: Format.releaseLabel(release),
            siblingIds: base.siblingIds,
            alternatives: alternatives
        )
    }
}

@MainActor
final class AppModel: ObservableObject {
    // MARK: Home

    @Published private(set) var feeds: [String: HomeFeed] = [:]

    // MARK: Search

    @Published private(set) var searchPhase: LoadPhase = .idle
    @Published private(set) var searchResults: [MobileCatalogItem] = []
    @Published private(set) var suggestions: [String] = []
    @Published private(set) var isLoadingMoreSearch = false

    // MARK: Library

    @Published private(set) var favoritesPhase: LoadPhase = .idle
    @Published private(set) var favoriteItems: [MobileCatalogItem] = []
    @Published private(set) var historyPhase: LoadPhase = .idle
    @Published private(set) var historyItems: [MobileHistoryItem] = []

    // MARK: Playback coordination

    /// Source picker sheet.
    @Published var sourceRequest: SourceRequest?
    /// Full-screen player.
    @Published var playing: PlaybackRequest?
    /// Last non-fatal message, shown as a small banner.
    @Published var banner: String?

    private var searchQuery = ""
    private var searchPage: UInt32 = 0
    private var searchCanLoadMore = true
    private var searchToken = 0
    private var pendingPlayback: PlaybackRequest?
    private var historyDirty = false

    // MARK: - Home

    func feed(for tabId: String) -> HomeFeed {
        feeds[tabId] ?? HomeFeed()
    }

    func loadHome(tabId: String, refresh: Bool = false) async {
        var feed = feed(for: tabId)
        if feed.phase == .loading || feed.isLoadingMore { return }
        if !refresh, feed.phase == .loaded, !feed.items.isEmpty { return }

        feed.phase = .loading
        if refresh { feed.canLoadMore = true }
        feeds[tabId] = feed

        let nextPage: UInt32 = refresh || feed.items.isEmpty ? 1 : feed.page + 1
        do {
            let items = try await Bridge.shared.perform {
                try $0.home(tabId: tabId, page: nextPage)
            }
            var updated = feeds[tabId] ?? feed
            if refresh || updated.items.isEmpty {
                updated.items = items
            } else {
                updated.items = Self.appendingUnique(items, to: updated.items)
            }
            updated.page = nextPage
            updated.canLoadMore = !items.isEmpty
            updated.isLoadingMore = false
            updated.phase = .loaded
            feeds[tabId] = updated
        } catch {
            var updated = feeds[tabId] ?? feed
            updated.phase = .failed(BridgeError.message(error))
            updated.isLoadingMore = false
            feeds[tabId] = updated
        }
    }

    func loadMoreHome(tabId: String) async {
        var feed = feed(for: tabId)
        guard feed.phase == .loaded, feed.canLoadMore, !feed.isLoadingMore else {
            return
        }
        feed.isLoadingMore = true
        feeds[tabId] = feed

        let nextPage = feed.page + 1
        do {
            let items = try await Bridge.shared.perform {
                try $0.home(tabId: tabId, page: nextPage)
            }
            var updated = feeds[tabId] ?? feed
            updated.items = Self.appendingUnique(items, to: updated.items)
            updated.page = nextPage
            updated.isLoadingMore = false
            updated.canLoadMore = !items.isEmpty
            feeds[tabId] = updated
        } catch {
            var updated = feeds[tabId] ?? feed
            updated.isLoadingMore = false
            updated.canLoadMore = false
            feeds[tabId] = updated
            banner = BridgeError.message(error)
        }
    }

    // MARK: - Search

    func runSearch(query: String) async {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            resetSearch()
            return
        }

        searchToken &+= 1
        let token = searchToken
        searchQuery = trimmed
        searchPhase = .loading
        suggestions = []

        do {
            let items = try await Bridge.shared.perform {
                try $0.search(query: trimmed, page: 1)
            }
            guard token == searchToken else { return }
            searchResults = items
            searchPage = 1
            searchCanLoadMore = !items.isEmpty
            searchPhase = .loaded
            if items.isEmpty {
                suggestions = await suggestedTerms(for: trimmed)
            }
        } catch {
            guard token == searchToken else { return }
            searchPhase = .failed(BridgeError.message(error))
        }
    }

    func loadMoreSearch() async {
        guard searchPhase == .loaded, searchCanLoadMore, !isLoadingMoreSearch else {
            return
        }
        let token = searchToken
        let query = searchQuery
        guard !query.isEmpty else { return }
        isLoadingMoreSearch = true
        let nextPage = searchPage + 1
        do {
            let items = try await Bridge.shared.perform {
                try $0.search(query: query, page: nextPage)
            }
            guard token == searchToken else { return }
            searchResults = Self.appendingUnique(items, to: searchResults)
            searchPage = nextPage
            searchCanLoadMore = !items.isEmpty
        } catch {
            guard token == searchToken else { return }
            searchCanLoadMore = false
        }
        isLoadingMoreSearch = false
    }

    func resetSearch() {
        searchToken &+= 1
        searchQuery = ""
        searchResults = []
        suggestions = []
        searchPage = 0
        searchCanLoadMore = true
        searchPhase = .idle
    }

    func useSuggestion(_ term: String) async {
        await runSearch(query: term)
    }

    private func suggestedTerms(for query: String) async -> [String] {
        let terms = try? await Bridge.shared.perform { try $0.suggest(query: query) }
        return Array((terms ?? []).prefix(8))
    }

    // MARK: - Details / streams / captions

    func loadDetails(subjectId: String) async throws -> MobileDetails {
        try await Bridge.shared.perform { try $0.details(subjectId: subjectId) }
    }

    func loadReleases(
        subjectId: String,
        season: UInt32,
        episode: UInt32
    ) async throws -> [MobileRelease] {
        try await Bridge.shared.perform {
            try $0.releases(subjectId: subjectId, season: season, episode: episode)
        }
    }

    func loadCaptions(
        subjectId: String,
        resourceId: String,
        siblingIds: [String],
        season: UInt32,
        episode: UInt32
    ) async throws -> [MobileSubtitle] {
        try await Bridge.shared.perform {
            try $0.captions(
                subjectId: subjectId,
                resourceId: resourceId,
                siblingIds: siblingIds,
                season: season,
                episode: episode
            )
        }
    }

    /// Online subtitles for the title being watched, Spanish first.
    ///
    /// An empty result is a normal answer (no IMDb match or no network), not an
    /// error: the caller keeps the provider captions it already has.
    func searchOnlineSubtitles(
        title: String,
        year: String,
        isSeries: Bool,
        season: UInt32,
        episode: UInt32
    ) async -> [MobileSubtitle] {
        await Bridge.shared.performSync {
            $0.searchOnlineSubtitles(
                title: title,
                year: year.isEmpty ? nil : year,
                isSeries: isSeries,
                season: season,
                episode: episode
            )
        }
    }

    // MARK: - Favorites

    func refreshFavorites() async {
        favoritesPhase = .loading
        let items = await Bridge.shared.performSync { $0.favorites() }
        // The store keeps the newest entry last; the library shows it first.
        favoriteItems = Array(items.reversed())
        favoritesPhase = .loaded
    }

    func isFavorite(subjectId: String, provider: String) async -> Bool {
        await Bridge.shared.performSync {
            $0.isFavorite(subjectId: subjectId, provider: provider)
        }
    }

    @discardableResult
    func toggleFavorite(_ item: MobileCatalogItem) async -> Bool {
        let isNowFavorite = await Bridge.shared.performSync {
            $0.toggleFavorite(item: item)
        }
        await refreshFavorites()
        banner = isNowFavorite
            ? "Added “\(item.title)” to favorites"
            : "Removed “\(item.title)” from favorites"
        return isNowFavorite
    }

    func removeFavorite(_ item: MobileCatalogItem) async {
        await Bridge.shared.performSync {
            $0.removeFavorite(subjectId: item.id, provider: item.provider)
        }
        await refreshFavorites()
        banner = "Removed “\(item.title)” from favorites"
    }

    // MARK: - History

    func refreshHistory() async {
        historyPhase = .loading
        let items = await Bridge.shared.performSync { $0.history() }
        // Store order is oldest first; keep watching surfaces the newest.
        historyItems = Array(items.reversed())
        historyPhase = .loaded
    }

    func removeHistory(_ entry: MobileHistoryItem) async {
        await Bridge.shared.performSync {
            $0.removeHistoryEntry(
                provider: entry.provider,
                subjectId: entry.subjectId,
                season: entry.season,
                episode: entry.episode
            )
        }
        await refreshHistory()
    }

    func clearHistory() async {
        await Bridge.shared.performSync { $0.clearHistory() }
        historyItems = []
        historyPhase = .loaded
        banner = "Watch history cleared"
    }

    // MARK: - Playback

    /// Opens the source picker for a title or episode.
    func showSources(_ request: SourceRequest) {
        sourceRequest = request
    }

    /// Called by the source picker once the viewer picks a stream. When the sheet
    /// is on screen the player is presented after it dismisses, so the two
    /// presentations never fight; with no sheet to dismiss (deep links) the player
    /// is presented straight away.
    func play(_ playback: PlaybackRequest) {
        guard sourceRequest != nil else {
            pendingPlayback = nil
            playing = playback
            return
        }
        pendingPlayback = playback
        sourceRequest = nil
    }

    func promotePendingPlayback() {
        guard let pending = pendingPlayback else { return }
        pendingPlayback = nil
        playing = pending
    }

    func dismissPlayer() {
        playing = nil
        if historyDirty {
            historyDirty = false
            Task { await refreshHistory() }
        }
    }

    func dismissBanner() {
        banner = nil
    }

    /// Persists progress for the title currently playing.
    func recordProgress(
        for request: PlaybackRequest,
        seconds: Double,
        duration: Double?,
        completed: Bool
    ) async {
        let clampedSeconds = max(0, seconds)
        let clampedDuration = duration.flatMap { $0.isFinite && $0 > 0 ? $0 : nil }
        let item = MobileHistoryItem(
            provider: request.provider,
            subjectId: request.subjectId,
            title: request.title,
            posterUrl: request.posterUrl,
            isSeries: request.isSeries,
            year: request.year,
            season: request.season,
            episode: request.episode,
            progressSeconds: UInt64(clampedSeconds.rounded()),
            durationSeconds: clampedDuration.map { UInt64($0.rounded()) },
            completed: completed
        )
        await Bridge.shared.performSync {
            $0.recordProgress(
                item: item,
                progressSeconds: item.progressSeconds,
                durationSeconds: item.durationSeconds,
                completed: completed
            )
        }
        historyDirty = true
        // The player records its last position after it closes; refresh then so
        // Continue Watching is current the next time it is looked at.
        if playing == nil {
            historyDirty = false
            await refreshHistory()
        }
    }

    /// Builds the playback request for an episode the viewer tapped.
    func sourceRequest(
        from details: MobileDetails,
        season: UInt32,
        episode: UInt32,
        episodeTitle: String?,
        startAt: Double = 0
    ) -> SourceRequest {
        SourceRequest(
            subjectId: details.id,
            provider: details.provider,
            title: details.title,
            year: details.year ?? "",
            posterUrl: details.posterUrl,
            isSeries: details.isSeries,
            season: season,
            episode: episode,
            episodeTitle: episodeTitle,
            startAt: startAt,
            siblingIds: details.siblingSubjectIds
        )
    }

    // MARK: - Deep links

    /// Opens a title from a URL such as:
    ///
    ///     moviebox://play/<subjectId>?season=1&episode=2&autoplay=1
    ///
    /// Without `autoplay` the source picker is shown; with it, the first
    /// direct-file source is resolved and played in one step.
    func handleDeepLink(_ url: URL) async {
        guard url.scheme?.lowercased() == "moviebox" else { return }
        guard url.host?.lowercased() == "play" else { return }

        let subjectId = url.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        guard !subjectId.isEmpty else {
            banner = "That link is missing a title identifier."
            return
        }

        let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        func queryValue(_ name: String) -> UInt32? {
            components?.queryItems?
                .first { $0.name == name }?
                .value
                .flatMap(UInt32.init)
        }
        let autoplay = components?.queryItems?.contains {
            $0.name == "autoplay" && ["1", "true", "yes"].contains($0.value?.lowercased() ?? "")
        } ?? false

        do {
            let details = try await loadDetails(subjectId: subjectId)
            let season: UInt32
            let episode: UInt32
            if details.isSeries {
                season = queryValue("season") ?? details.seasons.first?.number ?? 1
                episode = queryValue("episode")
                    ?? details.seasons.first?.episodes.first?.number
                    ?? 1
            } else {
                season = 0
                episode = 0
            }

            let request = sourceRequest(
                from: details,
                season: season,
                episode: episode,
                episodeTitle: nil
            )

            guard autoplay else {
                showSources(request)
                return
            }

            let releases = try await loadReleases(
                subjectId: details.id,
                season: season,
                episode: episode
            )
            let candidates = Format.playableDirectFiles(releases)
            guard let release = candidates.first else {
                // Nothing directly playable: fall back to the picker so the
                // viewer sees why instead of a silent failure.
                banner = "No directly playable source for this title."
                showSources(request)
                return
            }
            guard let raw = release.url, let streamURL = URL(string: raw) else {
                banner = "That source returned an unusable URL."
                showSources(request)
                return
            }

            play(
                PlaybackRequest(
                    url: streamURL,
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
                    siblingIds: details.siblingSubjectIds,
                    alternatives: Format.fallbackCandidates(after: release, in: candidates)
                )
            )
        } catch {
            NSLog("[moviebox] deep link failed: \(error)")
            banner = BridgeError.message(error)
        }
    }

    /// Resumes a history entry: resolve its sources, then play.
    func resume(_ entry: MobileHistoryItem) async {
        let request = SourceRequest(
            subjectId: entry.subjectId,
            provider: entry.provider,
            title: entry.title,
            year: entry.year,
            posterUrl: entry.posterUrl,
            isSeries: entry.isSeries,
            season: entry.season,
            episode: entry.episode,
            episodeTitle: nil,
            startAt: entry.completed ? 0 : Double(entry.progressSeconds)
        )
        showSources(request)
    }

    // MARK: - Helpers

    private static func appendingUnique(
        _ newItems: [MobileCatalogItem],
        to existing: [MobileCatalogItem]
    ) -> [MobileCatalogItem] {
        guard !newItems.isEmpty else { return existing }
        var seen = Set(existing.map { "\($0.provider)|\($0.id)" })
        var merged = existing
        for item in newItems {
            let key = "\(item.provider)|\(item.id)"
            if seen.insert(key).inserted {
                merged.append(item)
            }
        }
        return merged
    }
}
