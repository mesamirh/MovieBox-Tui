import XCTest
@testable import MovieBox

/// The value types in `AppModel.swift` are pure; only the observable state they
/// describe can be exercised without a `Bridge` call, so that is all that is
/// tested here.
final class AppModelValueTypeTests: XCTestCase {

    // MARK: - LoadPhase

    func testErrorPhaseExposesItsMessage() {
        XCTAssertEqual(LoadPhase.failed("boom").errorMessage, "boom")
        XCTAssertEqual(LoadPhase.failed("").errorMessage, "")
    }

    func testNonFailedPhasesHaveNoErrorMessage() {
        XCTAssertNil(LoadPhase.idle.errorMessage)
        XCTAssertNil(LoadPhase.loading.errorMessage)
        XCTAssertNil(LoadPhase.loaded.errorMessage)
    }

    func testPhasesWithTheSameStatusAreEqual() {
        XCTAssertEqual(LoadPhase.idle, LoadPhase.idle)
        XCTAssertEqual(LoadPhase.failed("x"), LoadPhase.failed("x"))
        XCTAssertNotEqual(LoadPhase.failed("x"), LoadPhase.failed("y"))
        XCTAssertNotEqual(LoadPhase.loading, LoadPhase.loaded)
    }

    // MARK: - HomeFeed

    func testHomeFeedStartsEmptyAndLoadable() {
        let feed = HomeFeed()

        XCTAssertTrue(feed.items.isEmpty)
        XCTAssertEqual(feed.phase, .idle)
        XCTAssertEqual(feed.page, 0)
        XCTAssertTrue(feed.canLoadMore)
        XCTAssertFalse(feed.isLoadingMore)
    }

    // MARK: - SourceRequest contextLabel

    func testSeriesContextLabelCarriesSeasonEpisodeAndTitle() {
        let request = SourceRequest(
            subjectId: "s1",
            provider: "p",
            title: "Show",
            year: "2024",
            posterUrl: nil,
            isSeries: true,
            season: 1,
            episode: 3,
            episodeTitle: "Pilot",
            startAt: 0
        )

        XCTAssertEqual(request.contextLabel, "Season 1 · Episode 3 · Pilot")
    }

    func testSeriesContextLabelOmitsAnEmptyEpisodeTitle() {
        let withoutTitle = SourceRequest(
            subjectId: "s1",
            provider: "p",
            title: "Show",
            year: "2024",
            posterUrl: nil,
            isSeries: true,
            season: 2,
            episode: 5,
            episodeTitle: nil,
            startAt: 0
        )
        let withEmptyTitle = SourceRequest(
            subjectId: "s1",
            provider: "p",
            title: "Show",
            year: "2024",
            posterUrl: nil,
            isSeries: true,
            season: 2,
            episode: 5,
            episodeTitle: "",
            startAt: 0
        )

        XCTAssertEqual(withoutTitle.contextLabel, "Season 2 · Episode 5")
        XCTAssertEqual(withEmptyTitle.contextLabel, "Season 2 · Episode 5")
    }

    func testMovieContextLabel() {
        let request = SourceRequest(
            subjectId: "s1",
            provider: "p",
            title: "Movie",
            year: "2024",
            posterUrl: nil,
            isSeries: false,
            season: 0,
            episode: 0,
            episodeTitle: nil,
            startAt: 0
        )

        XCTAssertEqual(request.contextLabel, "Movie")
    }

    // MARK: - SubtitleCue model

    func testCuesCompareByStartEndAndText() {
        XCTAssertEqual(SubtitleCue(start: 1, end: 2, text: "a"), SubtitleCue(start: 1, end: 2, text: "a"))
        XCTAssertNotEqual(SubtitleCue(start: 1, end: 2, text: "a"), SubtitleCue(start: 1, end: 2, text: "b"))
        XCTAssertNotEqual(SubtitleCue(start: 1, end: 2, text: "a"), SubtitleCue(start: 1, end: 3, text: "a"))
    }

    // MARK: - SubtitleError copy

    /// The controller surfaces these strings to the viewer, so they are part of
    /// the user-visible contract.
    func testSubtitleErrorMessages() {
        XCTAssertEqual(SubtitleError.badURL.errorDescription, "That subtitle link is malformed.")
        XCTAssertEqual(
            SubtitleError.http(503).errorDescription,
            "The subtitle server answered with HTTP 503."
        )
        XCTAssertEqual(
            SubtitleError.encoding.errorDescription,
            "The subtitle file used an unsupported text encoding."
        )
    }
}
