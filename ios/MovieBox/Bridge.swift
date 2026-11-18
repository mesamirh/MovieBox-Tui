import Foundation

/// Serial gateway between SwiftUI and the synchronous UniFFI client.
///
/// Every call into the Rust core blocks on network or disk, so nothing here may
/// run on the main actor. Work is dispatched onto one dedicated queue — which
/// also matches the Rust client's expectation of one serialized caller — and the
/// result is handed back through a checked continuation.
final class Bridge: @unchecked Sendable {
    static let shared = Bridge()

    private let queue = DispatchQueue(
        label: "com.moviebox.ios.bridge",
        qos: .userInitiated
    )
    private var client: MobileClient?

    private init() {}

    /// Runs a throwing client call off the main actor.
    func perform<T: Sendable>(
        _ body: @escaping @Sendable (MobileClient) throws -> T
    ) async throws -> T {
        try await withCheckedThrowingContinuation { continuation in
            queue.async {
                do {
                    continuation.resume(returning: try body(self.clientOnQueue()))
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Runs a non-throwing client call off the main actor (favorites, history,
    /// progress recording and the other local-store helpers).
    func performSync<T: Sendable>(
        _ body: @escaping @Sendable (MobileClient) -> T
    ) async -> T {
        await withCheckedContinuation { continuation in
            queue.async {
                continuation.resume(returning: body(self.clientOnQueue()))
            }
        }
    }

    // MARK: - Client bootstrap

    /// Builds the one client for the process, pointing every store at the app
    /// sandbox. Only ever called on `queue`.
    private func clientOnQueue() -> MobileClient {
        if let client { return client }

        let base = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        let config = base.appendingPathComponent("config", isDirectory: true)
        let cache = base.appendingPathComponent("cache", isDirectory: true)
        let data = base.appendingPathComponent("data", isDirectory: true)
        for directory in [config, cache, data] {
            try? FileManager.default.createDirectory(
                at: directory,
                withIntermediateDirectories: true
            )
        }

        let created = MobileClient(
            configDir: config.path,
            cacheDir: cache.path,
            dataDir: data.path
        )
        client = created
        return created
    }
}

enum BridgeError {
    /// Turns a UniFFI error into something a viewer can read.
    static func message(_ error: Error) -> String {
        if let mobile = error as? MobileError {
            switch mobile {
            case .Failure(let message):
                let trimmed = message.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty { return trimmed }
            }
        }
        let description = error.localizedDescription
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if !description.isEmpty { return description }
        return "Something went wrong while talking to MovieBox."
    }
}
