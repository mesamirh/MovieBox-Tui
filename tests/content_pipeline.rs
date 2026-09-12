use moviebox_tui::{
    models::SearchResult,
    providers::models::{
        CatalogItem, Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId,
        RequestContext, Season,
    },
    tui::{
        action::Action,
        app::App,
        state::{AppMode, InputMode, Screen},
    },
};

#[tokio::test]
async fn test_stale_details_response_protection() {
    let mut app = App::new();
    app.state_mut().update_available = None;
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().active_screen = Screen::Details;

    let context_a = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };
    app.state_mut().active_details_request = 1;
    app.state_mut().active_subject_id = Some("movie_a".to_string());
    app.state_mut().selected_details = Some(MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "movie_a".to_string(),
        },
        title: "Movie A".to_string(),
        media_type: MediaType::Movie,
        year: None,
        description: None,
        tagline: None,
        imdb_rating: None,
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons: vec![],
        dubs: vec![],
    });

    let context_b = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };
    app.state_mut().active_details_request = 2;
    app.state_mut().active_subject_id = Some("movie_b".to_string());
    app.state_mut().selected_details = Some(MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "movie_b".to_string(),
        },
        title: "Movie B Draft".to_string(),
        media_type: MediaType::Movie,
        year: None,
        description: None,
        tagline: None,
        imdb_rating: None,
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons: vec![],
        dubs: vec![],
    });

    let stale_payload = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "movie_a".to_string(),
        },
        title: "Movie A Full Metadata".to_string(),
        media_type: MediaType::Movie,
        year: None,
        description: Some("Stale synopsis from Movie A".to_string()),
        tagline: None,
        imdb_rating: None,
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons: vec![],
        dubs: vec![],
    };
    app.handle_action(Action::DetailsSuccess(
        context_a,
        1,
        "movie_a".to_string(),
        stale_payload,
    ))
    .await;

    assert_eq!(app.state().active_subject_id.as_deref(), Some("movie_b"));
    assert_eq!(
        app.state().selected_details.as_ref().unwrap().title,
        "Movie B Draft"
    );

    let valid_payload = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "movie_b".to_string(),
        },
        title: "Movie B Full Metadata".to_string(),
        media_type: MediaType::Movie,
        year: None,
        description: Some("Correct synopsis for Movie B".to_string()),
        tagline: None,
        imdb_rating: None,
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons: vec![],
        dubs: vec![],
    };
    app.handle_action(Action::DetailsSuccess(
        context_b,
        2,
        "movie_b".to_string(),
        valid_payload,
    ))
    .await;

    assert_eq!(app.state().active_subject_id.as_deref(), Some("movie_b"));
    assert_eq!(
        app.state().selected_details.as_ref().unwrap().title,
        "Movie B Full Metadata"
    );
    assert_eq!(
        app.state()
            .selected_details
            .as_ref()
            .unwrap()
            .description
            .as_deref(),
        Some("Correct synopsis for Movie B")
    );
}

#[test]
fn test_cache_key_isolation_across_providers_queries_and_dimensions() {
    use moviebox_tui::cache::{
        get_provider_details_path, get_provider_search_path, get_provider_stream_path,
    };

    let search_mb_batman_p1 = get_provider_search_path(ProviderKind::MovieBox, "batman", 1);
    let search_mb_batman_p2 = get_provider_search_path(ProviderKind::MovieBox, "batman", 2);
    let search_mb_superman_p1 = get_provider_search_path(ProviderKind::MovieBox, "superman", 1);
    let search_addon_batman_p1 = get_provider_search_path(ProviderKind::Addons, "batman", 1);

    assert_ne!(search_mb_batman_p1, search_mb_batman_p2);
    assert_ne!(search_mb_batman_p1, search_mb_superman_p1);
    assert_ne!(search_mb_batman_p1, search_addon_batman_p1);

    let details_mb_1 = get_provider_details_path(ProviderKind::MovieBox, "1001");
    let details_mb_2 = get_provider_details_path(ProviderKind::MovieBox, "1002");
    let details_4k_1 = get_provider_details_path(ProviderKind::FourKHdHub, "1001");

    assert_ne!(details_mb_1, details_mb_2);
    assert_ne!(details_mb_1, details_4k_1);

    let stream_s1_e1 = get_provider_stream_path(ProviderKind::MovieBox, "series_1", 1, 1);
    let stream_s1_e2 = get_provider_stream_path(ProviderKind::MovieBox, "series_1", 1, 2);
    let stream_s2_e1 = get_provider_stream_path(ProviderKind::MovieBox, "series_1", 2, 1);

    assert_ne!(stream_s1_e1, stream_s1_e2);
    assert_ne!(stream_s1_e1, stream_s2_e1);
}

#[tokio::test]
async fn test_search_failure_vs_empty_result_distinction() {
    let mut app = App::new();
    app.state_mut().update_available = None;
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().input_mode = InputMode::Normal;
    app.state_mut().search_query.set_content("xyznonexistent");

    let context = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };

    app.handle_action(Action::SearchSuccess {
        context,
        request_id: app.state().active_search_request,
        query: "xyznonexistent".to_string(),
        page: 1,
        items: vec![],
    })
    .await;

    assert!(app.state().search_results.is_empty());
    assert!(app.state().search_error.is_none());
    assert!(!app.state().is_loading);

    app.handle_action(Action::SearchFailure(
        context,
        app.state().active_search_request,
        1,
        "Network connection reset by peer".to_string(),
    ))
    .await;

    assert!(app.state().search_results.is_empty());
    assert_eq!(
        app.state().search_error.as_deref(),
        Some("Network connection reset by peer")
    );
    assert!(!app.state().is_loading);
}

#[tokio::test]
async fn test_mode_switch_stale_response_protection() {
    let mut app = App::new();
    app.state_mut().update_available = None;
    app.state_mut().set_mode(AppMode::Streaming);

    let streaming_generation = app.state().provider_generation;
    let streaming_context = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: streaming_generation,
    };
    app.state_mut().active_search_request = 1;
    app.state_mut().search_query.set_content("avatar");

    app.handle_action(Action::SwitchProvider(ProviderKind::Addons))
        .await;
    assert_eq!(app.state().active_provider, ProviderKind::Addons);
    assert_ne!(app.state().provider_generation, streaming_generation);

    let moviebox_items = vec![CatalogItem {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "mb_12345".to_string(),
        },
        title: "Avatar".to_string(),
        media_type: MediaType::Movie,
        year: Some("2009".to_string()),
        poster_url: Some("https://example.com/avatar.jpg".to_string()),
        season_count: None,
    }];

    app.handle_action(Action::SearchSuccess {
        context: streaming_context,
        request_id: 1,
        query: "avatar".to_string(),
        page: 1,
        items: moviebox_items,
    })
    .await;

    assert!(app.state().search_results.is_empty());
}

#[tokio::test]
async fn test_poster_identity_isolation() {
    let mut app = App::new();
    app.state_mut().update_available = None;
    app.state_mut().active_screen = Screen::Details;
    app.state_mut().active_subject_id = Some("target_movie_id".to_string());
    app.state_mut().poster_image = None;

    let dynamic_img = std::sync::Arc::new(image::DynamicImage::new_rgb8(10, 10));

    app.handle_action(Action::PosterSuccess(
        "other_movie_id".to_string(),
        dynamic_img.clone(),
    ))
    .await;

    assert!(app.state().poster_image.is_none());

    app.handle_action(Action::PosterSuccess(
        "target_movie_id".to_string(),
        dynamic_img.clone(),
    ))
    .await;

    assert!(app.state().poster_image.is_some());
}

#[tokio::test]
async fn test_search_preview_and_details_metadata_isolation() {
    let mut app = App::new();
    app.state_mut().update_available = None;

    let old_preview = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::Addons,
            value: "tt_old_movie".to_string(),
        },
        title: "Old Movie Title".to_string(),
        media_type: MediaType::Movie,
        year: Some("2020".to_string()),
        description: Some("Old Movie Description".to_string()),
        tagline: None,
        imdb_rating: Some("8.5".to_string()),
        director: Some("Old Director".to_string()),
        stars: Some("Old Star".to_string()),
        prints: None,
        audios: None,
        poster_url: Some("https://example.com/old.jpg".to_string()),
        duration: Some("120 min".to_string()),
        genres: vec!["Action".to_string()],
        seasons: vec![],
        dubs: vec![],
    };

    let new_search_result = SearchResult {
        id: "tt_new_movie".to_string(),
        title: "New Movie Title".to_string(),
        stype: 1,
        release_year: "2025".to_string(),
        cover_url: Some("https://example.com/new.jpg".to_string()),
        season: 0,
        episode: 0,
        provider: ProviderKind::Addons,
    };

    // Mismatched preview should NOT leak description/director/stars/rating into fallback
    let fallback = MediaDetails::from_search_result(&new_search_result, Some(&old_preview));
    assert_eq!(fallback.id.value, "tt_new_movie");
    assert_eq!(fallback.title, "New Movie Title");
    assert_eq!(fallback.year.as_deref(), Some("2025"));
    assert!(fallback.description.is_none());
    assert!(fallback.director.is_none());
    assert!(fallback.stars.is_none());
    assert!(fallback.imdb_rating.is_none());

    // Matching preview SHOULD preserve preview details
    let matching_preview = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::Addons,
            value: "tt_new_movie".to_string(),
        },
        title: "New Movie Title".to_string(),
        media_type: MediaType::Movie,
        year: Some("2025".to_string()),
        description: Some("New Movie Description".to_string()),
        tagline: None,
        imdb_rating: Some("7.9".to_string()),
        director: Some("New Director".to_string()),
        stars: Some("New Star".to_string()),
        prints: None,
        audios: None,
        poster_url: Some("https://example.com/new.jpg".to_string()),
        duration: Some("140 min".to_string()),
        genres: vec!["Drama".to_string()],
        seasons: vec![],
        dubs: vec![],
    };
    let matching_fallback =
        MediaDetails::from_search_result(&new_search_result, Some(&matching_preview));
    assert_eq!(matching_fallback.id.value, "tt_new_movie");
    assert_eq!(
        matching_fallback.description.as_deref(),
        Some("New Movie Description")
    );
    assert_eq!(matching_fallback.director.as_deref(), Some("New Director"));
}
#[tokio::test]
async fn test_series_details_defaults_to_season_one_when_no_history() {
    let mut app = App::new();
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().active_screen = Screen::Details;
    app.state_mut().active_subject_id = Some("bb_series".to_string());
    app.state_mut().selected_season = 1;
    app.state_mut().selected_episode = 1;
    app.state_mut().active_details_request = 1;

    let context = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };

    let seasons = (1..=5)
        .map(|s| Season {
            number: s,
            episodes: (1..=10)
                .map(|e| Episode {
                    season: s,
                    number: e,
                    title: None,
                })
                .collect(),
        })
        .collect();

    let details = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "bb_series".to_string(),
        },
        title: "Breaking Bad".to_string(),
        media_type: MediaType::Series,
        year: Some("2008".to_string()),
        description: None,
        tagline: None,
        imdb_rating: Some("9.5".to_string()),
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons,
        dubs: vec![],
    };

    app.handle_action(Action::DetailsSuccess(
        context,
        1,
        "bb_series".to_string(),
        details,
    ))
    .await;

    assert_eq!(app.state().selected_season, 1);
    assert_eq!(app.state().selected_episode, 1);
    assert_eq!(app.state().season_list_state.selected(), Some(0));
    assert_eq!(app.state().episode_list_state.selected(), Some(0));
}

#[tokio::test]
async fn test_series_details_resumes_watch_history() {
    let mut app = App::new();
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().active_screen = Screen::Details;
    app.state_mut().selected_season = 1;
    app.state_mut().selected_episode = 1;
    app.state_mut()
        .history
        .recent
        .push(moviebox_tui::history::WatchHistoryItem {
            provider: "moviebox".to_string(),
            subject_id: "bb_series".to_string(),
            title: "Breaking Bad".to_string(),
            stype: 2,
            season: 3,
            episode: 7,
            duration_seconds: Some(3000),
            progress_seconds: 500,
            completed: false,
            timestamp: 1000,
            release_year: "2008".to_string(),
            cover_url: None,
        });
    app.state_mut().active_details_request = 1;

    let context = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };

    let seasons = (1..=5)
        .map(|s| Season {
            number: s,
            episodes: (1..=10)
                .map(|e| Episode {
                    season: s,
                    number: e,
                    title: None,
                })
                .collect(),
        })
        .collect();

    let details = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "bb_series".to_string(),
        },
        title: "Breaking Bad".to_string(),
        media_type: MediaType::Series,
        year: Some("2008".to_string()),
        description: None,
        tagline: None,
        imdb_rating: Some("9.5".to_string()),
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons,
        dubs: vec![],
    };

    app.handle_action(Action::DetailsSuccess(
        context,
        1,
        "bb_series".to_string(),
        details,
    ))
    .await;

    assert_eq!(app.state().selected_season, 3);
    assert_eq!(app.state().selected_episode, 7);
    assert_eq!(app.state().season_list_state.selected(), Some(2));
    assert_eq!(app.state().episode_list_state.selected(), Some(6));
}

#[tokio::test]
async fn test_series_details_preserves_season_and_episode_on_language_switch() {
    let mut app = App::new();
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().active_screen = Screen::Details;
    app.state_mut().active_subject_id = Some("bb_series".to_string());
    app.state_mut().language_chosen = true;
    app.state_mut().selected_season = 4;
    app.state_mut().selected_episode = 2;

    app.state_mut().active_details_request = 1;

    let context = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };

    let seasons = (1..=5)
        .map(|s| Season {
            number: s,
            episodes: (1..=10)
                .map(|e| Episode {
                    season: s,
                    number: e,
                    title: None,
                })
                .collect(),
        })
        .collect();

    let details = MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: "bb_series".to_string(),
        },
        title: "Breaking Bad".to_string(),
        media_type: MediaType::Series,
        year: Some("2008".to_string()),
        description: None,
        tagline: None,
        imdb_rating: Some("9.5".to_string()),
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: None,
        duration: None,
        genres: vec![],
        seasons,
        dubs: vec![],
    };

    app.handle_action(Action::DetailsSuccess(
        context,
        1,
        "bb_series".to_string(),
        details,
    ))
    .await;

    assert_eq!(app.state().selected_season, 4);
    assert_eq!(app.state().selected_episode, 2);
    assert_eq!(app.state().season_list_state.selected(), Some(3));
    assert_eq!(app.state().episode_list_state.selected(), Some(1));
}
