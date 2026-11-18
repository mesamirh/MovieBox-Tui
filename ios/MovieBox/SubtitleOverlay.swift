import Foundation
import SwiftUI

// MARK: - Model

struct SubtitleCue: Equatable, Sendable {
    let start: TimeInterval
    let end: TimeInterval
    let text: String
}

enum SubtitleError: LocalizedError {
    case badURL
    case http(Int)
    case encoding

    var errorDescription: String? {
        switch self {
        case .badURL:
            return "That subtitle link is malformed."
        case .http(let status):
            return "The subtitle server answered with HTTP \(status)."
        case .encoding:
            return "The subtitle file used an unsupported text encoding."
        }
    }
}

// MARK: - Parser

/// Parses SubRip (.srt) and WebVTT (.vtt) files into timed cues.
///
/// The parser is deliberately forgiving: provider caption files are often
/// slightly malformed, so anything it cannot understand is skipped instead of
/// failing the whole track.
enum SubtitleParser {
    static func parse(_ raw: String) -> [SubtitleCue] {
        var text = raw
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
        if text.hasPrefix("\u{FEFF}") {
            text.removeFirst()
        }

        let lines = text.components(separatedBy: "\n")
        var cues: [SubtitleCue] = []
        var block: [String] = []

        func flush() {
            defer { block = [] }
            guard let timingIndex = block.firstIndex(where: { $0.contains("-->") }) else {
                return
            }
            let timing = block[timingIndex]
            let halves = timing.components(separatedBy: "-->")
            guard halves.count == 2,
                  let start = timestamp(halves[0]),
                  let end = timestamp(halves[1].trimmingCharacters(in: .whitespaces))
            else {
                return
            }
            // A WebVTT end timestamp may be followed by cue settings.
            let endValue = end
            guard endValue > start else { return }

            let body = block[(timingIndex + 1)...]
                .map(cleaned)
                .filter { !$0.isEmpty }
                .joined(separator: "\n")
            guard !body.isEmpty else { return }
            cues.append(SubtitleCue(start: start, end: endValue, text: body))
        }

        for line in lines {
            if line.trimmingCharacters(in: .whitespaces).isEmpty {
                flush()
            } else {
                block.append(line)
                // Guard against pathological files.
                if cues.count >= 20_000 { break }
            }
        }
        flush()

        return cues.sorted { $0.start < $1.start }
    }

    /// Accepts `HH:MM:SS,mmm`, `HH:MM:SS.mmm`, `MM:SS.mmm` and WebVTT cue
    /// settings trailing the timestamp.
    private static func timestamp(_ raw: String) -> TimeInterval? {
        var candidate = raw.trimmingCharacters(in: .whitespaces)
        if candidate.contains(" ") {
            candidate = candidate.components(separatedBy: .whitespaces).first ?? candidate
        }
        candidate = candidate.replacingOccurrences(of: ",", with: ".")
        let pieces = candidate.split(separator: ":")
        guard pieces.count >= 2, pieces.count <= 3 else { return nil }

        var total: TimeInterval = 0
        for piece in pieces {
            guard let value = Double(piece), value >= 0 else { return nil }
            total = total * 60 + value
        }
        return total
    }

    private static func cleaned(_ line: String) -> String {
        var result = line
        // Strip ASS-style override blocks: {\an8}, {\pos(...)}
        while let open = result.range(of: "{"), let close = result.range(of: "}") {
            guard open.lowerBound < close.lowerBound else { break }
            result.removeSubrange(open.lowerBound...close.upperBound)
        }
        // Strip simple HTML tags: <i>, </b>, <font ...>
        while let open = result.range(of: "<"), let close = result.range(of: ">") {
            guard open.lowerBound < close.lowerBound else { break }
            result.removeSubrange(open.lowerBound...close.upperBound)
        }
        result = result
            .replacingOccurrences(of: "&amp;", with: "&")
            .replacingOccurrences(of: "&lt;", with: "<")
            .replacingOccurrences(of: "&gt;", with: ">")
            .replacingOccurrences(of: "&nbsp;", with: " ")
            .replacingOccurrences(of: "&#39;", with: "'")
        return result.trimmingCharacters(in: .whitespaces)
    }

    /// Text of the cue(s) active at `time`, if any.
    static func text(at time: TimeInterval, in cues: [SubtitleCue]) -> String? {
        guard !cues.isEmpty, time >= 0 else { return nil }

        var low = 0
        var high = cues.count - 1
        var index = -1
        while low <= high {
            let middle = (low + high) / 2
            if cues[middle].start <= time {
                index = middle
                low = middle + 1
            } else {
                high = middle - 1
            }
        }
        guard index >= 0 else { return nil }

        var pieces: [String] = []
        var cursor = index
        while cursor >= 0, pieces.count < 4 {
            let cue = cues[cursor]
            if cue.end > time {
                pieces.insert(cue.text, at: 0)
            } else if time - cue.end > 5 {
                break
            }
            cursor -= 1
        }
        return pieces.isEmpty ? nil : pieces.joined(separator: "\n")
    }
}

// MARK: - Controller

/// Loads a caption track over HTTP and keeps the cue that matches playback time.
///
/// AVPlayer cannot attach an arbitrary remote SRT to a stream, so subtitles are
/// rendered as a timed overlay driven by the player's clock.
final class SubtitleController: ObservableObject {
    @Published private(set) var tracks: [MobileSubtitle] = []
    @Published private(set) var selectedTrack: MobileSubtitle?
    @Published private(set) var activeText: String?
    @Published private(set) var isLoading = false
    @Published private(set) var statusMessage: String?

    private var cues: [SubtitleCue] = []
    private var lastTime: TimeInterval = -1

    var isEnabled: Bool { selectedTrack != nil && !cues.isEmpty }

    func setTracks(_ newTracks: [MobileSubtitle]) {
        tracks = newTracks
        if newTracks.isEmpty {
            statusMessage = "No subtitles for this release."
        }
    }

    func markUnavailable(_ message: String = "No subtitles for this release.") {
        tracks = []
        statusMessage = message
    }

    func select(_ track: MobileSubtitle) async {
        isLoading = true
        statusMessage = nil
        do {
            let loaded = try await Self.fetchCues(track.url)
            guard !loaded.isEmpty else {
                cues = []
                selectedTrack = nil
                activeText = nil
                isLoading = false
                statusMessage = "That subtitle file had no readable cues."
                return
            }
            cues = loaded
            selectedTrack = track
            lastTime = -1
            isLoading = false
        } catch {
            cues = []
            selectedTrack = nil
            activeText = nil
            isLoading = false
            let reason = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            statusMessage = "Could not load \(track.name): \(reason)"
        }
    }

    func disable() {
        selectedTrack = nil
        activeText = nil
        cues = []
        lastTime = -1
    }

    func update(time: TimeInterval) {
        guard isEnabled else { return }
        if abs(time - lastTime) < 0.05 { return }
        lastTime = time
        activeText = SubtitleParser.text(at: time, in: cues)
    }

    private static func fetchCues(_ urlString: String) async throws -> [SubtitleCue] {
        guard let url = URL(string: urlString) else { throw SubtitleError.badURL }
        var request = URLRequest(url: url, timeoutInterval: 20)
        request.setValue("text/vtt, text/plain, */*", forHTTPHeaderField: "Accept")
        let (data, response) = try await URLSession.shared.data(for: request)
        if let http = response as? HTTPURLResponse,
           !(200...299).contains(http.statusCode) {
            throw SubtitleError.http(http.statusCode)
        }
        guard let text = decode(data) else { throw SubtitleError.encoding }
        return await Task.detached(priority: .userInitiated) {
            SubtitleParser.parse(text)
        }.value
    }

    private static func decode(_ data: Data) -> String? {
        let encodings: [String.Encoding] = [
            .utf8, .utf16, .windowsCP1252, .isoLatin1, .ascii
        ]
        for encoding in encodings {
            if let text = String(data: data, encoding: encoding) {
                return text
            }
        }
        return nil
    }
}

// MARK: - Overlay

struct SubtitleOverlay: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 20, weight: .semibold, design: .rounded))
            .foregroundStyle(.white)
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .background(
                Color.black.opacity(0.62),
                in: RoundedRectangle(cornerRadius: 8, style: .continuous)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .strokeBorder(.white.opacity(0.12), lineWidth: 0.5)
            )
            .shadow(color: .black.opacity(0.8), radius: 2, y: 1)
            .padding(.horizontal, 24)
            .allowsHitTesting(false)
            .accessibilityLabel("Subtitles: \(text)")
    }
}
