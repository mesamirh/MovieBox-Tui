import SwiftUI

/// Browse tabs mirrored from the Roku channel's navigation so both clients
/// request the same catalog sections.
struct HomeTab: Identifiable, Hashable {
    let id: String
    let title: String

    static let all: [HomeTab] = [
        HomeTab(id: "1", title: "Home"),
        HomeTab(id: "2", title: "Movies"),
        HomeTab(id: "5", title: "TV Shows"),
        HomeTab(id: "8", title: "Anime")
    ]
}

struct HomeView: View {
    @EnvironmentObject private var model: AppModel
    @State private var selectedTabId = HomeTab.all[0].id

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    Picker("Browse section", selection: $selectedTabId) {
                        ForEach(HomeTab.all) { tab in
                            Text(tab.title).tag(tab.id)
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
            .navigationTitle("MovieBox")
            .refreshable {
                await model.loadHome(tabId: selectedTabId, refresh: true)
            }
            .task(id: selectedTabId) {
                await model.loadHome(tabId: selectedTabId)
            }
            .navigationDestination(for: MobileCatalogItem.self) { item in
                DetailsView(item: item)
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        let feed = model.feed(for: selectedTabId)

        if feed.items.isEmpty {
            switch feed.phase {
            case .idle, .loading:
                LoadingBlock(message: "Fetching the catalog…")
            case .failed(let message):
                ErrorBlock(message: message) {
                    Task { await model.loadHome(tabId: selectedTabId, refresh: true) }
                }
            case .loaded:
                EmptyHint(
                    icon: "film.stack",
                    title: "Nothing here yet",
                    message: "This catalog section came back empty. Pull to refresh, or try another section."
                )
            }
        } else {
            PosterGrid(items: feed.items) {
                Task { await model.loadMoreHome(tabId: selectedTabId) }
            }

            if feed.isLoadingMore {
                ProgressView()
                    .tint(.white)
                    .padding(.vertical, 8)
            } else if !feed.canLoadMore {
                Text("End of catalog")
                    .font(.caption2)
                    .foregroundStyle(Theme.subtleText)
                    .padding(.top, 4)
            }

            if let message = feed.phase.errorMessage {
                ErrorBlock(message: message) {
                    Task { await model.loadHome(tabId: selectedTabId, refresh: true) }
                }
            }
        }
    }
}
