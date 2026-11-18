import XCTest
@testable import MovieBox

/// Characterisation tests for the WebVTT half of `SubtitleParser`. Provider
/// caption tracks come in both flavours, so both must keep working.
final class SubtitleParserWebVTTTests: XCTestCase {

    func testHeaderAndDotMillisecondTimings() {
        let file = [
            "WEBVTT",
            "",
            "00:00:01.000 --> 00:00:02.500",
            "First cue",
            "",
            "00:00:03.000 --> 00:00:04.250",
            "Second cue",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 2.5, text: "First cue"))
        XCTAssertEqual(cues[1], SubtitleCue(start: 3.0, end: 4.25, text: "Second cue"))
    }

    /// A VTT end timestamp may be followed by cue settings; only the timestamp
    /// belongs to `end`, and the settings never reach the rendered text.
    func testCueSettingsAfterTheEndTimestampAreIgnored() {
        let file = [
            "WEBVTT",
            "",
            "00:00:01.000 --> 00:00:02.500 align:middle line:90%",
            "Centred",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 2.5, text: "Centred"))
    }

    /// VTT allows an identifier line between the blank line and the timing line.
    func testCueIdentifierLineIsNotPartOfTheText() {
        let file = [
            "WEBVTT",
            "",
            "intro",
            "00:00:01.000 --> 00:00:02.000",
            "Identified cue",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "Identified cue")
    }

    /// A file whose header is not followed by a blank line keeps the header in
    /// the same block as the first cue; it must still parse.
    func testHeaderImmediatelyFollowedByACueStillParses() {
        let file = "WEBVTT\n00:00:01.000 --> 00:00:02.000\nNo blank line after header\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "No blank line after header")
    }

    func testShortMinutesSecondsDotMillisecondsForm() {
        let file = "WEBVTT\n\n00:15.000 --> 00:18.500\nShort\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].start, 15.0, accuracy: 0.000_5)
        XCTAssertEqual(cues[0].end, 18.5, accuracy: 0.000_5)
    }

    func testMixedSRTAndVTTBlocksBothParse() {
        let file = [
            "WEBVTT",
            "",
            "00:00:01.000 --> 00:00:02.000",
            "vtt style",
            "",
            "2",
            "00:00:03,000 --> 00:00:04,000",
            "srt style",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues.map { $0.text }, ["vtt style", "srt style"])
    }

    /// VTT `NOTE`/`STYLE` blocks carry no timing and are skipped, while the
    /// cues around them survive.
    func testMetadataBlocksWithoutTimingAreSkipped() {
        let file = [
            "WEBVTT",
            "",
            "NOTE this comment has no timing",
            "",
            "00:00:01.000 --> 00:00:02.000",
            "Kept",
            "",
            "STYLE",
            "::cue { color: yellow }"
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "Kept")
    }
}
