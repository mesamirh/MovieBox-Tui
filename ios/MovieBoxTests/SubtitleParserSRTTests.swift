import XCTest
@testable import MovieBox

/// Characterisation tests for the SubRip half of `SubtitleParser`.
///
/// These pin behaviour the subtitle overlay depends on while a stream plays, so
/// a silent change in the parser fails here instead of shipping.
final class SubtitleParserSRTTests: XCTestCase {

    // MARK: - Well-formed files

    func testTwoCueFileParsesTimingsAndText() {
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

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 3.5, text: "Hello there"))
        XCTAssertEqual(cues[1], SubtitleCue(start: 4.0, end: 6.25, text: "Second line"))
    }

    func testHoursMinutesSecondsMillisecondsConvertToSeconds() {
        let file = "1\n01:02:03,500 --> 01:02:05,000\nLater\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].start, 3723.5, accuracy: 0.000_5)
        XCTAssertEqual(cues[0].end, 3725.0, accuracy: 0.000_5)
    }

    func testMinutesSecondsFormWithoutHours() {
        let file = "1\n00:01,250 --> 00:03,750\nShort form\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].start, 1.25, accuracy: 0.000_5)
        XCTAssertEqual(cues[0].end, 3.75, accuracy: 0.000_5)
    }

    func testMultiLineCueBodyKeepsOneLineBreakPerLine() {
        let file = "1\n00:00:01,000 --> 00:00:04,000\nfirst line\nsecond line\nthird line\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "first line\nsecond line\nthird line")
    }

    /// A blank line is the block separator in SubRip, so a cue's text can never
    /// span one: everything after the blank line carries no timing of its own
    /// and is dropped rather than glued onto the previous cue.
    func testBlankLineInsideTextEndsTheCueAndDropsTheUntimedTail() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:03,000",
            "first paragraph",
            "",
            "second paragraph",
            "",
            "2",
            "00:00:05,000 --> 00:00:06,000",
            "next cue",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues[0].text, "first paragraph")
        XCTAssertEqual(cues[1].text, "next cue")
    }

    func testCuesAreSortedByStartTimeEvenWhenTheFileIsOutOfOrder() {
        let file = [
            "2",
            "00:00:05,000 --> 00:00:06,000",
            "Second",
            "",
            "1",
            "00:00:01,000 --> 00:00:02,000",
            "First",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues.map { $0.text }, ["First", "Second"])
        XCTAssertTrue(zip(cues, cues.dropFirst()).allSatisfy { $0.start <= $1.start })
    }

    // MARK: - Line endings and BOM

    func testCRLFLineEndingsParseLikeLF() {
        let lf = "1\n00:00:01,000 --> 00:00:02,000\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nWorld\n"
        let crlf = lf.replacingOccurrences(of: "\n", with: "\r\n")

        XCTAssertEqual(SubtitleParser.parse(crlf), SubtitleParser.parse(lf))
        XCTAssertEqual(SubtitleParser.parse(crlf).count, 2)
        // No stray carriage returns survive into the rendered text.
        XCTAssertFalse(SubtitleParser.parse(crlf).contains { $0.text.contains("\r") })
    }

    func testCROnlyLineEndingsParse() {
        let cr = "1\r00:00:01,000 --> 00:00:02,000\rHello\r"

        let cues = SubtitleParser.parse(cr)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 2.0, text: "Hello"))
    }

    /// The BOM sits in front of the timing line here, so a parser that left it
    /// in place would fail to read the start timestamp and drop the cue.
    func testLeadingBOMIsStrippedEvenWhenItPrecedesTheTimingLine() {
        let file = "\u{FEFF}00:00:01,000 --> 00:00:02,000\nHello\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0], SubtitleCue(start: 1.0, end: 2.0, text: "Hello"))
    }

    func testLeadingBOMWithCRLFAndIndexLineParses() {
        let file = "\u{FEFF}1\r\n00:00:01,000 --> 00:00:02,000\r\nHello\r\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "Hello")
    }

    // MARK: - Markup cleanup

    func testASSStyleOverridesAndHTMLTagsAreStripped() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:03,000",
            "{\\an8}<i>Top</i> line",
            "",
            "2",
            "00:00:04,000 --> 00:00:06,000",
            "<font color=\"#ffffff\">Coloured</font> &amp; &lt;escaped&gt; AT&#39;T",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 2)
        XCTAssertEqual(cues[0].text, "Top line")
        XCTAssertEqual(cues[1].text, "Coloured & <escaped> AT'T")
    }

    func testUnbalancedMarkupIsKeptWithoutHanging() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:02,000",
            "brace without close {",
            "",
            "2",
            "00:00:03,000 --> 00:00:04,000",
            "close before open } then { open",
            "",
            "3",
            "00:00:05,000 --> 00:00:06,000",
            "angle without close <",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 3)
        XCTAssertEqual(cues[0].text, "brace without close {")
        XCTAssertEqual(cues[1].text, "close before open } then { open")
        XCTAssertEqual(cues[2].text, "angle without close <")
    }

    /// Lines that are nothing but markup carry no readable text, so a cue whose
    /// whole body is markup is dropped instead of rendering an empty overlay.
    func testCueWithOnlyMarkupInItsBodyIsDropped() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:02,000",
            "{\\an8}<i></i>",
            "",
            "2",
            "00:00:03,000 --> 00:00:04,000",
            "real text",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "real text")
    }

    /// Regression: the stripper used to remove one character too many after
    /// every closing `}`/`>`, so text immediately after a markup block was
    /// truncated (`{\an8}Hello` rendered as `ello`).
    func testTextImmediatelyAfterMarkupSurvives() {
        let file = [
            "1",
            "00:00:01,000 --> 00:00:02,000",
            "{\\an8}Hello",
            "",
            "2",
            "00:00:03,000 --> 00:00:04,000",
            "{\\i1}x{\\i0}y",
            "",
            "3",
            "00:00:05,000 --> 00:00:06,000",
            "<b>Bold</b>tail",
            ""
        ].joined(separator: "\n")

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.map { $0.text }, ["Hello", "xy", "Boldtail"])
    }

    func testSurroundingWhitespaceIsTrimmedFromEachTextLine() {
        let file = "1\n00:00:01,000 --> 00:00:02,000\n   padded   \n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].text, "padded")
    }

    func testSRTPositionSettingsAfterTheEndTimestampAreIgnored() {
        let file = "1\n00:00:01,000 --> 00:00:02,000 X1:100 X2:200 Y1:10 Y2:20\nPositioned\n"

        let cues = SubtitleParser.parse(file)

        XCTAssertEqual(cues.count, 1)
        XCTAssertEqual(cues[0].end, 2.0, accuracy: 0.000_5)
        XCTAssertEqual(cues[0].text, "Positioned")
    }
}
