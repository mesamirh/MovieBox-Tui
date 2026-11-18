import SwiftUI

struct SearchView: View {
    @EnvironmentObject private var model: AppModel
    @State private var query = ""

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 14) {
                    content
                }
                .padding(.top, 8)
                .padding(.bottom, 24)
            }
            .background(Theme.background.ignoresSafeArea())
            .navigationTitle("Search")
            .searchable(
                text: $query,
                placement: .navigationBarDrawer(displayMode: .always),
                prompt: "Movies, shows, anime"
            )
            .onSubmit(of: .search) {
                Task { await model.runSearch(query: query) }
            }
            .task(id: query) {
                await debouncedSearch()
            }
            .navigationDestination(for: MobileCatalogItem.self) { item in
                DetailsView(item: item)
            }
        }
    }

    private func debouncedSearch() async {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            model.resetSearch()
            return
        }
        guard trimmed.count >= 2 else { return }
        // Wait out fast typing; `task(id:)` cancels this when the query changes.
        try? await Task.sleep(nanoseconds: 450_000_000)
        if Task.isCancelled { return }
        await model.runSearch(query: trimmed)
    }

    @ViewBuilder
    private var content: some View {
        if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            EmptyHint(
                icon: "magnifyingglass",
                title: "Search MovieBox",
                message: "Type at least two characters to look across the catalog. Results appear as you type."
            )
        } else if model.searchResults.isEmpty {
            switch model.searchPhase {
            case .idle, .loading:
                LoadingBlock(message: "Searching…")
            case .failed(let message):
                ErrorBlock(message: message) {
                    Task { await model.runSearch(query: query) }
                }
            case .loaded:
                VStack(spacing: 16) {
                    EmptyHint(
                        icon: "text.magnifyingglass",
                        title: "No matches",
                        message: "Nothing came back for “\(query)”. Try a shorter title or a different spelling."
                    )
                    suggestionRow
                }
            }
        } else {
            HStack {
                Text("\(model.searchResults.count) result\(model.searchResults.count == 1 ? "" : "s")")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(Theme.subtleText)
                Spacer()
                if model.searchPhase == .loading {
                    ProgressView().controlSize(.small).tint(.white)
                }
            }
            .padding(.horizontal, 16)

            PosterGrid(items: model.searchResults) {
                Task { await model.loadMoreSearch() }
            }

            if model.isLoadingMoreSearch {
                ProgressView().tint(.white).padding(.vertical, 8)
            }

            suggestionRow
        }
    }

    @ViewBuilder
    private var suggestionRow: some View {
        if !model.suggestions.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text("Try instead")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(Theme.subtleText)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(model.suggestions, id: \.self) { term in
                            Button {
                                query = term
                                Task { await model.runSearch(query: term) }
                            } label: {
                                Text(term)
                                    .font(.caption.weight(.semibold))
                                    .padding(.horizontal, 12)
                                    .padding(.vertical, 7)
                                    .background(Theme.surfaceRaised, in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .foregroundStyle(.white)
                        }
                    }
                    .padding(.horizontal, 16)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
