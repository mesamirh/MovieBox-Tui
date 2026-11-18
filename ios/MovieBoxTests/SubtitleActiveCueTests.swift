import XCTest
@testable import MovieBox

/// `SubtitleParser.text(at:in:)` decides what appears on screen for a given
/// playback position, so its boundaries are user-visible behaviour.
final class SubtitleActiveCueTests: XCTestCase {

    private let cues = [
        SubtitleCue(start: 1, end: 3, text: "A"),
        SubtitleCue(start: 5, end: 8, text: "B"),
        SubtitleCue(start: 10, end: 12, text: "C")
    ]

    func testNoCuesMeansNoText() {
        XCTAssertNil(SubtitleParser.text(at: 0, in: []))
        XCTAssertNil(SubtitleParser.text(at: 5, in: []))
    }

    func testBeforeTheFirstCueMeansNoText() {
        XCTAssertNil(SubtitleParser.text(at: 0, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 0.999, in: cues))
    }

    func testNegativeTimeMeansNoText() {
        XCTAssertNil(SubtitleParser.text(at: -1, in: cues))
        XCTAssertNil(SubtitleParser.text(at: -0.001, in: cues))
    }

    func testExactlyAtACueStartTheCueIsActive() {
        XCTAssertEqual(SubtitleParser.text(at: 1, in: cues), "A")
        XCTAssertEqual(SubtitleParser.text(at: 5, in: cues), "B")
        XCTAssertEqual(SubtitleParser.text(at: 10, in: cues), "C")
    }

    func testJustAfterACueStartTheCueIsActive() {
        XCTAssertEqual(SubtitleParser.text(at: 1.001, in: cues), "A")
    }

    func testExactlyAtACueEndTheCueIsNoLongerActive() {
        XCTAssertNil(SubtitleParser.text(at: 3, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 8, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 12, in: cues))
    }

    func testJustBeforeACueEndTheCueIsStillActive() {
        XCTAssertEqual(SubtitleParser.text(at: 2.999, in: cues), "A")
    }

    func testBetweenCuesMeansNoText() {
        XCTAssertNil(SubtitleParser.text(at: 4, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 9, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 3.5, in: cues))
    }

    func testAfterTheLastCueMeansNoText() {
        XCTAssertNil(SubtitleParser.text(at: 12.001, in: cues))
        XCTAssertNil(SubtitleParser.text(at: 10_000, in: cues))
    }

    func testOverlappingCuesAreShownInChronologicalOrder() {
        let overlapping = [
            SubtitleCue(start: 0, end: 5, text: "A"),
            SubtitleCue(start: 2, end: 6, text: "B")
        ]

        XCTAssertEqual(SubtitleParser.text(at: 3, in: overlapping), "A\nB")
        // At 0 only the first cue is running.
        XCTAssertEqual(SubtitleParser.text(at: 0, in: overlapping), "A")
        // At 5 the first cue has ended.
        XCTAssertEqual(SubtitleParser.text(at: 5, in: overlapping), "B")
    }

    func testTextFromAParsedFileMatchesPlaybackPositions() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:03,500",
            "Hello there",
            "",
            "2",
            "00:00:04,000 --> 00:00:06,250",
            "Second line",
            ""
        ].joined(separator: "\n")
        let parsed = SubtitleParser.parse(file)

        XCTAssertNil(SubtitleParser.text(at: 0.5, in: parsed))
        XCTAssertEqual(SubtitleParser.text(at: 2, in: parsed), "Hello there")
        XCTAssertNil(SubtitleParser.text(at: 3.75, in: parsed))
        XCTAssertEqual(SubtitleParser.text(at: 6.0, in: parsed), "Second line")
        XCTAssertNil(SubtitleParser.text(at: 7, in: parsed))
    }

    func testCueTextIsReturnedVerbatimIncludingLineBreaks() {
        let multi = [SubtitleCue(start: 0, end: 10, text: "first\nsecond")]

        XCTAssertEqual(SubtitleParser.text(at: 1, in: multi), "first\nsecond")
    }
}
