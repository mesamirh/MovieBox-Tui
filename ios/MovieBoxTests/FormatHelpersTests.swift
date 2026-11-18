import AVFoundation
import XCTest
@testable import MovieBox

/// `Format` is the only presentation logic that can be exercised without a
/// network round trip, so it is pinned here rather than through the views.
final class FormatHelpersTests: XCTestCase {

    // MARK: - clock

    func testClockFormatsUnderAnHour() {
        XCTAssertEqual(Format.clock(1), "0:01")
        XCTAssertEqual(Format.clock(9), "0:09")
        XCTAssertEqual(Format.clock(59.4), "0:59")
        XCTAssertEqual(Format.clock(60), "1:00")
        XCTAssertEqual(Format.clock(125), "2:05")
        XCTAssertEqual(Format.clock(3599), "59:59")
    }

    func testClockRoundsToTheNearestSecond() {
        XCTAssertEqual(Format.clock(59.5), "1:00")
        XCTAssertEqual(Format.clock(59.49), "0:59")
    }

    func testClockFormatsAnHourOrMore() {
        XCTAssertEqual(Format.clock(3600), "1:00:00")
        XCTAssertEqual(Format.clock(3661), "1:01:01")
        XCTAssertEqual(Format.clock(7325), "2:02:05")
    }

    func testClockRoundsUpAcrossTheHourBoundary() {
        XCTAssertEqual(Format.clock(3599.6), "1:00:00")
    }

    func testClockHandlesLargeValues() {
        XCTAssertEqual(Format.clock(360_000), "100:00:00")
        XCTAssertEqual(Format.clock(359_999.6), "100:00:00")
    }

    func testClockFallsBackToZeroForNonPositiveAndNonFiniteInput() {
        XCTAssertEqual(Format.clock(0), "0:00")
        XCTAssertEqual(Format.clock(-1), "0:00")
        XCTAssertEqual(Format.clock(-0.5), "0:00")
        XCTAssertEqual(Format.clock(Double.nan), "0:00")
        XCTAssertEqual(Format.clock(Double.infinity), "0:00")
        XCTAssertEqual(Format.clock(-Double.infinity), "0:00")
    }

    // MARK: - bytes

    func testBytesIsNilWithoutAValueOrForZero() {
        XCTAssertNil(Format.bytes(nil))
        XCTAssertNil(Format.bytes(0))
    }

    func testBytesFormatsPositiveValues() {
        let small = Format.bytes(1)
        let large = Format.bytes(5_000_000_000)
        XCTAssertNotNil(small)
        XCTAssertNotNil(large)
        XCTAssertFalse(small?.isEmpty ?? true)
        XCTAssertFalse(large?.isEmpty ?? true)
        // Same input, same output: no hidden state in the formatter.
        XCTAssertEqual(Format.bytes(1_500_000), Format.bytes(1_500_000))
    }

    // MARK: - episodeLabel

    func testEpisodeLabel() {
        XCTAssertEqual(Format.episodeLabel(season: 1, episode: 3), "S1 E3")
        XCTAssertEqual(Format.episodeLabel(season: 12, episode: 104), "S12 E104")
        XCTAssertEqual(Format.episodeLabel(season: 0, episode: 0), "S0 E0")
    }

    // MARK: - qualityRank

    func testQualityRankOrdersKnownQualities() {
        XCTAssertEqual(Format.qualityRank("480p"), 480)
        XCTAssertEqual(Format.qualityRank("720p"), 720)
        XCTAssertEqual(Format.qualityRank("1080p"), 1080)
        XCTAssertEqual(Format.qualityRank("2160p"), 2160)
        XCTAssertTrue(Format.qualityRank("1080p") > Format.qualityRank("720p"))
        XCTAssertTrue(Format.qualityRank("4K") > Format.qualityRank("1080p"))
    }

    func testQualityRankTreats4KAsTheTopQuality() {
        XCTAssertEqual(Format.qualityRank("4K"), 2160)
        XCTAssertEqual(Format.qualityRank("4k"), 2160)
        XCTAssertEqual(Format.qualityRank("4K HDR"), 2160)
        XCTAssertEqual(Format.qualityRank("1080p"), 1080)
    }

    func testQualityRankIsCaseInsensitiveAndReadsEmbeddedDigits() {
        XCTAssertEqual(Format.qualityRank("1080P"), 1080)
        XCTAssertEqual(Format.qualityRank(" 1080p "), 1080)
        XCTAssertEqual(Format.qualityRank("1920x1080"), 19201080)
    }

    func testQualityRankIsZeroWhenUnknown() {
        XCTAssertEqual(Format.qualityRank(nil), 0)
        XCTAssertEqual(Format.qualityRank(""), 0)
        XCTAssertEqual(Format.qualityRank("HD"), 0)
        XCTAssertEqual(Format.qualityRank("unknown"), 0)
    }

    // MARK: - progress

    func testProgressIsZeroOrOneWithoutADuration() {
        XCTAssertEqual(Format.progress(historyItem(progress: 30, duration: nil, completed: false)), 0)
        XCTAssertEqual(Format.progress(historyItem(progress: 30, duration: nil, completed: true)), 1)
        XCTAssertEqual(Format.progress(historyItem(progress: 0, duration: 0, completed: false)), 0)
        XCTAssertEqual(Format.progress(historyItem(progress: 0, duration: 0, completed: true)), 1)
    }

    func testProgressIsTheRatioOfWatchedToDuration() {
        XCTAssertEqual(Format.progress(historyItem(progress: 25, duration: 100)), 0.25, accuracy: 0.000_1)
        XCTAssertEqual(Format.progress(historyItem(progress: 0, duration: 100)), 0)
        XCTAssertEqual(Format.progress(historyItem(progress: 100, duration: 100, completed: true)), 1)
    }

    func testProgressIsClampedToAtMostOne() {
        XCTAssertEqual(Format.progress(historyItem(progress: 200, duration: 100)), 1)
        XCTAssertEqual(Format.progress(historyItem(progress: UInt64.max, duration: 1)), 1)
    }

    private func release(
        _ filename: String,
        quality: String? = "1080p",
        codec: String? = "H264",
        url: String? = "https://cdn.example.com/stream.mp4",
        isDirectFile: Bool = true
    ) -> MobileRelease {
        MobileRelease(
            filename: filename,
            quality: quality,
            codec: codec,
            language: nil,
            sizeBytes: nil,
            season: nil,
            episode: nil,
            url: url,
            sourceLabel: "Direct",
            headers: [],
            resourceId: nil,
            isDirectFile: isDirectFile
        )
    }

    private func historyItem(
        progress: UInt64,
        duration: UInt64?,
        completed: Bool = false
    ) -> MobileHistoryItem {
        MobileHistoryItem(
            provider: "test",
            subjectId: "subject",
            title: "Title",
            posterUrl: nil,
            isSeries: false,
            year: "2024",
            season: 0,
            episode: 0,
            progressSeconds: progress,
            durationSeconds: duration,
            completed: completed
        )
    }
}

/// Playback source selection. `AVPlayer` decodes H.264 everywhere it runs, while
/// MovieBox's 4K entries arrive as 10-bit HEVC or Dolby Vision and play their
/// audio with no picture, so the first candidate has to be a decodable one
/// instead of whatever the provider listed first.
final class PlaybackSelectionTests: XCTestCase {

    // MARK: - codecRank

    func testH264OutranksHevcAndUnsupportedCodecs() {
        XCTAssertEqual(Format.codecRank("H264"), 0)
        XCTAssertEqual(Format.codecRank("h.264"), 0)
        XCTAssertEqual(Format.codecRank("AVC"), 0)
        XCTAssertEqual(Format.codecRank("HEVC"), 1)
        XCTAssertEqual(Format.codecRank("H265"), 1)
        XCTAssertEqual(Format.codecRank("Dolby Vision"), 1)
        XCTAssertEqual(Format.codecRank("AV1"), 2)
        XCTAssertEqual(Format.codecRank("VP9"), 2)
    }

    func testAnUnknownCodecRanksBetweenH264AndAV1() {
        XCTAssertEqual(Format.codecRank(nil), 1)
        XCTAssertEqual(Format.codecRank(""), 1)
        XCTAssertTrue(Format.codecRank(nil) > Format.codecRank("H264"))
        XCTAssertTrue(Format.codecRank(nil) < Format.codecRank("AV1"))
    }

    // MARK: - playableDirectFiles

    func testPlayableDirectFilesPrefersH264OverAnEarlier4KHevcEntry() {
        let releases = [
            release("4k-hevc.mp4", quality: "4K", codec: "HEVC"),
            release("1080-h264.mp4", quality: "1080p", codec: "H264"),
        ]

        let playable = Format.playableDirectFiles(releases)

        XCTAssertEqual(playable.count, 2)
        XCTAssertEqual(playable.first?.filename, "1080-h264.mp4")
        XCTAssertEqual(playable.last?.filename, "4k-hevc.mp4")
    }

    func testPlayableDirectFilesPrefers1080pOver4KWithinOneCodec() {
        let releases = [
            release("4k-h264.mp4", quality: "4K", codec: "H264"),
            release("1080-h264.mp4", quality: "1080p", codec: "H264"),
            release("720-h264.mp4", quality: "720p", codec: "H264"),
        ]

        XCTAssertEqual(
            Format.playableDirectFiles(releases).map(\.filename),
            ["1080-h264.mp4", "720-h264.mp4", "4k-h264.mp4"]
        )
    }

    func testPlayableDirectFilesKeepsHevcWhenItIsTheOnlyCandidate() {
        let releases = [
            release("4k-hevc.mp4", quality: "4K", codec: "HEVC"),
            release("manifest.mpd", codec: "HEVC", isDirectFile: false),
        ]

        let playable = Format.playableDirectFiles(releases)

        XCTAssertEqual(playable.map(\.filename), ["4k-hevc.mp4"])
    }

    func testPlayableDirectFilesDropsManifestsAndUrlLessEntries() {
        let releases = [
            release("manifest.m3u8", isDirectFile: false),
            release("no-url.mp4", url: nil),
            release("empty-url.mp4", url: ""),
        ]

        XCTAssertTrue(Format.playableDirectFiles(releases).isEmpty)
    }

    func testPlayableDirectFilesKeepsProviderOrderBetweenEqualCandidates() {
        let releases = [
            release("first.mp4"),
            release("second.mp4"),
            release("third.mp4"),
        ]

        XCTAssertEqual(
            Format.playableDirectFiles(releases).map(\.filename),
            ["first.mp4", "second.mp4", "third.mp4"]
        )
    }

    // MARK: helpers

    private func release(
        _ filename: String,
        quality: String? = "1080p",
        codec: String? = "H264",
        url: String? = "https://cdn.example.com/stream.mp4",
        isDirectFile: Bool = true
    ) -> MobileRelease {
        MobileRelease(
            filename: filename,
            quality: quality,
            codec: codec,
            language: nil,
            sizeBytes: nil,
            season: nil,
            episode: nil,
            url: url,
            sourceLabel: "Direct",
            headers: [],
            resourceId: nil,
            isDirectFile: isDirectFile
        )
    }
}

/// A failed attempt has to move on to another candidate of the same title, but
/// only a bounded number of times and never back to a URL it already tried.
/// The failure overlay also has to name what actually went wrong.
final class PlaybackFallbackTests: XCTestCase {

    // MARK: - nextFallback

    func testAFailureWithARemainingCandidateReturnsThatCandidate() {
        let next = Format.nextFallback(
            alternatives: [
                release("second.mp4", url: "https://cdn.example.com/second.mp4"),
                release("third.mp4", url: "https://cdn.example.com/third.mp4"),
            ],
            attemptedURLs: [url("https://cdn.example.com/first.mp4")],
            extraAttempts: 0
        )

        XCTAssertEqual(next?.filename, "second.mp4")
    }

    func testAFailureWithNoRemainingCandidateReturnsNil() {
        XCTAssertNil(
            Format.nextFallback(alternatives: [], attemptedURLs: [], extraAttempts: 0)
        )
        // A candidate without a usable URL can never be attempted.
        XCTAssertNil(
            Format.nextFallback(
                alternatives: [release("no-url.mp4", url: nil)],
                attemptedURLs: [url("https://cdn.example.com/first.mp4")],
                extraAttempts: 0
            )
        )
    }

    func testTheFallbackStopsAfterTwoExtraAttempts() {
        let candidate = release("next.mp4", url: "https://cdn.example.com/next.mp4")

        XCTAssertEqual(
            Format.nextFallback(alternatives: [candidate], attemptedURLs: [], extraAttempts: 0)?.filename,
            "next.mp4"
        )
        XCTAssertEqual(
            Format.nextFallback(alternatives: [candidate], attemptedURLs: [], extraAttempts: 1)?.filename,
            "next.mp4"
        )
        XCTAssertNil(
            Format.nextFallback(alternatives: [candidate], attemptedURLs: [], extraAttempts: 2)
        )
        XCTAssertNil(
            Format.nextFallback(alternatives: [candidate], attemptedURLs: [], extraAttempts: 3)
        )
    }

    func testAURLAlreadyTriedIsNeverReturnedAgain() {
        let tried = release("first.mp4", url: "https://cdn.example.com/first.mp4")
        let fresh = release("second.mp4", url: "https://cdn.example.com/second.mp4")
        let attempted: Set<URL> = [url("https://cdn.example.com/first.mp4")]

        XCTAssertEqual(
            Format.nextFallback(
                alternatives: [tried, fresh],
                attemptedURLs: attempted,
                extraAttempts: 0
            )?.filename,
            "second.mp4"
        )
        XCTAssertNil(
            Format.nextFallback(alternatives: [tried], attemptedURLs: attempted, extraAttempts: 0)
        )
    }

    // MARK: - fallbackCandidates

    func testFallbackCandidatesStartAfterTheChosenSourceAndSkipItsURL() {
        let first = release("first.mp4", url: "https://cdn.example.com/first.mp4")
        let second = release("second.mp4", url: "https://cdn.example.com/second.mp4")
        let third = release("third.mp4", url: "https://cdn.example.com/third.mp4")
        let ordered = [first, second, third]

        XCTAssertEqual(
            Format.fallbackCandidates(after: first, in: ordered).map(\.filename),
            ["second.mp4", "third.mp4"]
        )
        XCTAssertEqual(
            Format.fallbackCandidates(after: second, in: ordered).map(\.filename),
            ["third.mp4"]
        )
        XCTAssertTrue(Format.fallbackCandidates(after: third, in: ordered).isEmpty)
        XCTAssertEqual(
            Format.fallbackCandidates(after: first, in: [first, release("dup.mp4", url: first.url)]).map(\.filename),
            []
        )
    }

    // MARK: - failureDetail

    func testFailureDetailReportsTheDomainCodeAndUnderlyingReason() {
        let underlying = NSError(
            domain: NSOSStatusErrorDomain,
            code: -12847,
            userInfo: [NSLocalizedDescriptionKey: "The media format is not supported."]
        )
        let error = NSError(
            domain: AVFoundationErrorDomain,
            code: AVError.Code.unknown.rawValue,
            userInfo: [NSUnderlyingErrorKey: underlying]
        )

        let detail = Format.failureDetail(error)

        XCTAssertFalse(detail.isEmpty)
        XCTAssertTrue(detail.contains(AVFoundationErrorDomain), detail)
        XCTAssertTrue(detail.contains("\(AVError.Code.unknown.rawValue)"), detail)
        XCTAssertTrue(detail.contains("-12847"), detail)
        XCTAssertTrue(detail.contains("The media format is not supported."), detail)
    }

    func testFailureDetailNeverLeavesAnErrorUndescribed() {
        let error = NSError(
            domain: NSURLErrorDomain,
            code: NSURLErrorNotConnectedToInternet,
            userInfo: [NSLocalizedDescriptionKey: "The network connection was lost."]
        )

        let detail = Format.failureDetail(error)

        XCTAssertFalse(detail.isEmpty)
        XCTAssertTrue(detail.contains(NSURLErrorDomain), detail)
        XCTAssertTrue(detail.contains("\(NSURLErrorNotConnectedToInternet)"), detail)
        XCTAssertTrue(detail.contains("The network connection was lost."), detail)
    }

    func testFailureDetailIsReadableWithoutAnError() {
        XCTAssertEqual(Format.failureDetail(nil), "No error detail reported by the system player.")
    }

    // MARK: helpers

    private func url(_ raw: String) -> URL {
        guard let parsed = URL(string: raw) else {
            XCTFail("not a URL: \(raw)")
            return URL(fileURLWithPath: "/dev/null")
        }
        return parsed
    }

    private func release(
        _ filename: String,
        quality: String? = "1080p",
        codec: String? = "H264",
        url: String? = "https://cdn.example.com/stream.mp4",
        isDirectFile: Bool = true
    ) -> MobileRelease {
        MobileRelease(
            filename: filename,
            quality: quality,
            codec: codec,
            language: nil,
            sizeBytes: nil,
            season: nil,
            episode: nil,
            url: url,
            sourceLabel: "Direct",
            headers: [],
            resourceId: nil,
            isDirectFile: isDirectFile
        )
    }
}

/// A candidate can be `readyToPlay` and still show nothing: the player has to
/// decide from the asset's own video tracks whether the file can put a picture
/// on screen (S6a), and from `presentationSize` whether the item it already
/// mounted has any video surface at all (S6b). The exhausted overlay then has to
/// say that no source had a playable video track instead of echoing AVFoundation
/// (S7).
final class PlaybackVideoCheckTests: XCTestCase {

    // MARK: - videoVerdict

    func testACandidateWithoutAVideoTrackIsUnusable() {
        let verdict = Format.videoVerdict(hasVideoTrack: false, anyDecodable: false)

        XCTAssertEqual(verdict, .noVideoTrack)
        XCTAssertFalse(verdict.isUsable)
        XCTAssertFalse(verdict.reason.isEmpty)
    }

    func testACandidateWithoutAVideoTrackIsUnusableEvenWhenATrackClaimsDecodable() {
        XCTAssertEqual(
            Format.videoVerdict(hasVideoTrack: false, anyDecodable: true),
            .noVideoTrack
        )
    }

    func testVideoTracksThatAreAllUndecodableAreUnusable() {
        let verdict = Format.videoVerdict(hasVideoTrack: true, anyDecodable: false)

        XCTAssertEqual(verdict, .undecodable)
        XCTAssertFalse(verdict.isUsable)
        XCTAssertFalse(verdict.reason.isEmpty)
        // A missing track and an undecodable one are distinct reasons.
        XCTAssertNotEqual(
            Format.videoVerdict(hasVideoTrack: false, anyDecodable: false).reason,
            verdict.reason
        )
    }

    func testOneDecodableVideoTrackIsUsable() {
        let verdict = Format.videoVerdict(hasVideoTrack: true, anyDecodable: true)

        XCTAssertEqual(verdict, .usable)
        XCTAssertTrue(verdict.isUsable)
    }

    // MARK: - isAudioOnly

    func testAZeroHeightPresentationSizeIsAudioOnly() {
        XCTAssertTrue(Format.isAudioOnly(presentationHeight: 0))
    }

    func testAPositiveHeightPresentationSizeHasAVideoSurface() {
        XCTAssertFalse(Format.isAudioOnly(presentationHeight: 1080))
        XCTAssertFalse(Format.isAudioOnly(presentationHeight: 0.5))
    }

    // MARK: - videoExhaustedNotice

    func testTheExhaustedNoticeNamesTheMissingVideoTrackAndTheCount() {
        XCTAssertEqual(
            Format.videoExhaustedNotice(triedCount: 1, hadVideoRejection: true),
            "No source of this title has a playable video track — 1 source tried."
        )
        XCTAssertEqual(
            Format.videoExhaustedNotice(triedCount: 3, hadVideoRejection: true),
            "No source of this title has a playable video track — 3 sources tried."
        )
    }

    // PRESERVE: a chain that ended on a real AVFoundation error keeps the
    // per-attempt headline and technical detail (S2), so no video notice appears.
    func testTheExhaustedNoticeStaysQuietWhenNoCandidateWasRejectedForVideo() {
        XCTAssertNil(Format.videoExhaustedNotice(triedCount: 3, hadVideoRejection: false))

        let failure = PlaybackFailure(
            headline: "Cannot Open",
            detail: "AVFoundationErrorDomain -11828"
        )
        XCTAssertEqual(failure.kind, .systemError)
    }
}
