import XCTest
@testable import MovieBox

/// The parser exists because provider caption files are frequently broken. These
/// tests pin that broken input degrades instead of crashing the player, and that
/// every tolerated case has a well-defined result.
final class SubtitleParserMalformedTests: XCTestCase {

    func testEmptyInputYieldsNoCues() {
        XCTAssertEqual(SubtitleParser.parse(""), [])
    }

    func testWhitespaceOnlyInputYieldsNoCues() {
        XCTAssertEqual(SubtitleParser.parse("   \n\n\t\n   \n"), [])
    }

    func testInputWithoutAnyTimingLineYieldsNoCues() {
        let file = "just some prose\nwith no timing at all\n\nmore prose\nand more\n"

        XCTAssertEqual(SubtitleParser.parse(file), [])
    }

    func testReversedBoundariesDropsTheCueAndNothingElse() {
        let file = [
            "1",
            "00:00:05,000 --> 00:00:02,000",
            "Backwards",
            "",
            "2",
            "00:00:10,000 --> 00:00:11,000",
            "Forwards",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0], SubtitleCue(start: 10.0, end: 11.0, text: "Forwards"))
    }

    func testZeroLengthCueIsDropped() {
        let file = "1\n00:00:02,000 --> 00:00:02,000\nSame instant\n"

        XCTAssertEqual(SubtitleParser.parse(file), [])
    }

    func testUnparsableBoundariesAreDropped() {
        let cases = [
            "00:0a:xx,000 --> 00:00:05,000",
            "start --> end",
            "00:00:01,000 -->",
            "--> 00:00:05,000",
            "1:2:3:4 --> 1:2:3:5",
            "-01:00:05,000 --> 00:00:06,000",
            "00:00:01,000 --> 00:00:05,000 --> 00:00:06,000"
        ]

        for timing in cases {
            let file = "1\n\(timing)\nSome text\n"
            XCTAssertEqual(
                SubtitleParser.parse(file),
                [],
                "timing line \(timing.debugDescription) should yield no cue"
            )
        }
    }

    /// The last cue of a file that does not end in a newline must not be lost.
    func testUnterminatedFinalCueIsStillParsed() {
        let file = "1\n00:00:01,000 --> 00:00:02,000\nLast cue without trailing newline"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 2.0, text: "Last cue without trailing newline"))
    }

    func testUnterminatedTimingLineWithoutTextIsDropped() {
        XCTAssertEqual(SubtitleParser.parse("1\n00:00:01,000 --> 00:00:02,000"), [])
    }

    func testGarbageBetweenCuesDoesNotDiscardTheGoodOnes() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:02,000",
            "Good",
            "",
            "garbage without timing",
            "more garbage",
            "",
            "2",
            "00:00:03,000 --> 00:00:04,000",
            "Also good",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues.map { $0.text }, ["Good", "Also good"])
    }

    func testNonFiniteTimestampsAreDroppedRatherThanProducingInvalidCues() {
        let file = "1\nnan:nan --> inf:inf\nNaN timing\n"

        XCTAssertEqual(SubtitleParser.parse(file), [])
    }

    /// A pathological track must stay bounded instead of growing without limit.
    func testVeryLongFileStaysBoundedAndOrdered() {
        let cueCount = 25_000
        var file = ""
        file.reserveCapacity(cueCount * 48)
        for index in 0..<cueCount {
            let start = index * 2
            file += "\(index)\n"
            file += "\(timestamp(start)) --> \(timestamp(start + 1))\n"
            file += "Line \(index)\n\n"
        }

        let cues = SubtitleParser.parse(file)

        XCTAssertLessThanOrEqual(cues.count, 20_000)
        XCTAssertGreaterThan(cues.count, 0)
        XCTAssertTrue(zip(cues, cues.dropFirst()).allSatisfy { $0.start <= $1.start })
        XCTAssertTrue(cues.allSatisfy { $0.end > $0.start })
    }

    func testOverlappingCuesDoNotProduceUnboundedText() {
        // 40 cues stacked on the same instant.
        var file = ""
        for index in 0..<40 {
            file += "\(index)\n00:00:01,000 --> 00:00:30,000\nStacked \(index)\n\n"
        }

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 40)
        let text = SubtitleParser.text(at: 5, in: cues)
        let shown = text?.split(separator: "\n").count ?? 0
        XCTAssertGreaterThan(shown, 0)
        XCTAssertLessThanOrEqual(shown, 4)
    }

    private func timestamp(_ seconds: Int) -> String {
        let hours = seconds / 3600
        let minutes = (seconds % 3600) / 60
        let secs = seconds % 60
        return String(format: "%02d:%02d:%02d,000", hours, minutes, secs)
    }
}
