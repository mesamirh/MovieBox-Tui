import SwiftUI

@main
struct MovieBoxApp: App {
    @StateObject private var model = AppModel()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(model)
                .preferredColorScheme(.dark)
                .tint(Theme.accent)
        }
    }
}

/// Launch-time entry points. `simctl launch … -MovieBoxOpenURL <url>` and
/// Shortcuts can hand the app a deep link before any UI is presented; routing it
/// through the same handler a real `moviebox://` open uses keeps both paths
/// identical.
enum LaunchOptions {
    static var deepLinkURL: URL? {
        // Preferred form for automation: `SIMCTL_CHILD_MOVIEBOX_OPEN_URL=… simctl launch`.
        if let raw = ProcessInfo.processInfo.environment["MOVIEBOX_OPEN_URL"],
           let url = URL(string: raw) {
            return url
        }
        let arguments = ProcessInfo.processInfo.arguments
        guard let flag = arguments.firstIndex(of: "-MovieBoxOpenURL"),
              arguments.count > flag + 1,
              let url = URL(string: arguments[flag + 1])
        else {
            return nil
        }
        return url
    }
}

/// Tab shell plus the two presentation surfaces both Browse and Library share:
/// the source picker sheet and the full-screen player.
struct RootView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        TabView {
            HomeView()
                .tabItem {
                    Label("Browse", systemImage: "square.grid.2x2.fill")
                }
            SearchView()
                .tabItem {
                    Label("Search", systemImage: "magnifyingglass")
                }
            LibraryView()
                .tabItem {
                    Label("Library", systemImage: "heart.fill")
                }
        }
        .tint(Theme.accent)
        .onOpenURL { url in
            Task { await model.handleDeepLink(url) }
        }
        .task {
            if let url = LaunchOptions.deepLinkURL {
                await model.handleDeepLink(url)
            }
        }
        .overlay(alignment: .bottom) {
            if let banner = model.banner {
                BannerView(message: banner) {
                    model.dismissBanner()
                }
                .padding(.bottom, 76)
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .task(id: banner) {
                    try? await Task.sleep(nanoseconds: 3_000_000_000)
                    if !Task.isCancelled {
                        model.dismissBanner()
                    }
                }
            }
        }
        .animation(.easeInOut(duration: 0.2), value: model.banner)
        .sheet(
            item: $model.sourceRequest,
            onDismiss: { model.promotePendingPlayback() }
        ) { request in
            SourcePickerView(request: request)
                .environmentObject(model)
        }
        .fullScreenCover(
            item: $model.playing,
            onDismiss: { model.dismissPlayer() }
        ) { playback in
            PlayerView(request: playback)
                .environmentObject(model)
        }
    }
}
