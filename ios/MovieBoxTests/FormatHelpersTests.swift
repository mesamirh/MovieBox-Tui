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
