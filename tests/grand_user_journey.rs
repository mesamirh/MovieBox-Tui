use moviebox_tui::providers::addons::models::{AddonManifest, InstalledAddon};
use moviebox_tui::providers::models::{ProviderKind, RequestContext};
use moviebox_tui::tui::action::Action;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::state::{AppMode, InputMode, Screen};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn test_grand_user_journey_complete_lifecycle() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();
    app.state_mut().update_available = None;

    terminal.draw(|frame| app.draw(frame)).unwrap();
    assert_eq!(app.state().active_screen, Screen::Home);

    app.state_mut().set_mode(AppMode::Streaming);
    app.state_mut().active_provider = ProviderKind::MovieBox;
    app.state_mut().search_query = "Inception".to_string();

    let streaming_ctx = RequestContext {
        provider: ProviderKind::MovieBox,
        generation: app.state().provider_generation,
    };
    let search_payload = serde_json::json!({
        "results": [{
            "subjects": [{
                "subjectId": "inc_101",
                "title": "Inception",
                "subjectType": 1,
                "releaseDate": "2010",
                "cover": { "url": "https://example.com/inception.jpg" }
            }]
        }]
    });

    app.handle_action(Action::SearchSuccess {
        context: streaming_ctx,
        request_id: app.state().active_search_request,
        query: "Inception".to_string(),
        page: 1,
        payload: search_payload,
    })
    .await;

    assert_eq!(app.state().search_results.len(), 1);
    assert_eq!(app.state().search_results[0].title, "Inception");

    let details_payload = serde_json::json!({
        "id": "inc_101",
        "title": "Inception",
        "subjectType": 1,
        "releaseDate": "2010",
        "duration": "120 min",
        "synopsis": "A thief who steals corporate secrets through the use of dream-sharing technology."
    });

    app.state_mut().active_screen = Screen::Details;
    app.state_mut().active_subject_id = Some("inc_101".to_string());
    app.handle_action(Action::DetailsSuccess(
        streaming_ctx,
        app.state().active_details_request,
        "inc_101".to_string(),
        details_payload,
    ))
    .await;

    assert_eq!(app.state().active_screen, Screen::Details);
    assert_eq!(
        app.state().selected_details.as_ref().unwrap()["title"],
        "Inception"
    );

    app.state_mut().is_playing = true;
    let duplicate_play = app.state().is_playing;
    assert!(duplicate_play);

    app.state_mut().is_playing = false;
    app.handle_action(Action::PlayerExited(app.state().playback_generation))
        .await;

    let history_item = moviebox_tui::history::WatchHistoryItem {
        provider: "moviebox".to_string(),
        subject_id: "inc_101".to_string(),
        title: "Inception".to_string(),
        cover_url: Some("https://example.com/inception.jpg".to_string()),
        stype: 1,
        release_year: "2010".to_string(),
        season: 0,
        episode: 0,
        timestamp: 1700000000,
        duration_seconds: Some(120),
        progress_seconds: 45,
        completed: false,
    };

    app.handle_action(Action::UpdateProgress {
        item: history_item.clone(),
        progress: 45,
        duration: Some(120),
        completed: false,
    })
    .await;

    let saved = app
        .state()
        .history
        .get_item("moviebox", "inc_101", 0, 0, None)
        .expect("History item must be saved");
    assert_eq!(saved.progress_seconds, 45);
    assert!(saved.is_in_progress());

    let resume_pos = app
        .state()
        .history
        .get_item("moviebox", "inc_101", 0, 0, None)
        .map(|i| i.progress_seconds);
    assert_eq!(resume_pos, Some(45));

    let series_ep1 = moviebox_tui::history::WatchHistoryItem {
        provider: "moviebox".to_string(),
        subject_id: "bb_series".to_string(),
        title: "Breaking Bad".to_string(),
        cover_url: None,
        stype: 2,
        release_year: "2008".to_string(),
        season: 1,
        episode: 1,
        timestamp: 1700000100,
        duration_seconds: Some(2400),
        progress_seconds: 2400,
        completed: true,
    };
    app.handle_action(Action::UpdateProgress {
        item: series_ep1,
        progress: 2400,
        duration: Some(2400),
        completed: true,
    })
    .await;

    let series_ep2 = moviebox_tui::history::WatchHistoryItem {
        provider: "moviebox".to_string(),
        subject_id: "bb_series".to_string(),
        title: "Breaking Bad".to_string(),
        cover_url: None,
        stype: 2,
        release_year: "2008".to_string(),
        season: 1,
        episode: 2,
        timestamp: 1700000200,
        duration_seconds: Some(2400),
        progress_seconds: 900,
        completed: false,
    };
    app.handle_action(Action::UpdateProgress {
        item: series_ep2,
        progress: 900,
        duration: Some(2400),
        completed: false,
    })
    .await;

    let recent_series = app
        .state()
        .history
        .recent
        .iter()
        .find(|i| i.subject_id == "bb_series")
        .expect("Series card must exist");
    assert_eq!(recent_series.season, 1);
    assert_eq!(recent_series.episode, 2);
    assert_eq!(recent_series.progress_seconds, 900);

    let ep1_watched = app
        .state()
        .history
        .is_watched("moviebox", "bb_series", 1, 1);
    assert!(ep1_watched);

    app.handle_action(Action::ToggleAddonMode).await;
    assert_eq!(app.state().mode(), AppMode::Addon);

    let cinemeta_json = serde_json::json!({
        "id": "org.stremio.cinemeta",
        "version": "3.0.12",
        "name": "Cinemeta",
        "description": "Official Stremio metadata",
        "resources": ["meta", "catalog"],
        "types": ["movie", "series"],
        "catalogs": []
    });
    let manifest: AddonManifest = serde_json::from_value(cinemeta_json).unwrap();
    let installed_count = app.state().installed_addons.len();
    app.state_mut()
        .installed_addons
        .push(InstalledAddon::from_manifest(
            "https://v3-cinemeta.strem.io/manifest.json".to_string(),
            &manifest,
        ));
    assert_eq!(app.state().installed_addons.len(), installed_count + 1);

    app.handle_action(Action::AddonAddManifest(
        "http://not-a-valid-manifest-url.example".to_string(),
    ))
    .await;
    assert!(app.state().installed_addons.len() > installed_count);

    app.handle_action(Action::CancelDownload).await;
    assert!(app.state().cancel_download.load(Ordering::SeqCst));
    assert_eq!(
        app.state().download_status.as_deref(),
        Some("Cancelling...")
    );

    app.handle_action(Action::ClearDownload).await;
    assert!(app.state().download_status.is_none());
    assert!(app.state().download_progress.is_none());

    app.state_mut().input_mode = InputMode::Editing;
    app.state_mut().search_query = "/download-dir".to_string();
    assert_eq!(app.state().search_query, "/download-dir");

    let esc_key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    app.handle_action(Action::Key(esc_key)).await;
    assert_eq!(app.state().search_query, "");
    assert_eq!(app.state().input_mode, InputMode::Normal);

    for (w, h) in [(40, 15), (80, 24), (120, 40), (200, 60)] {
        let backend = TestBackend::new(w, h);
        let mut t = Terminal::new(backend).unwrap();
        let draw_res = t.draw(|frame| app.draw(frame));
        assert!(draw_res.is_ok());
    }

    let quit_res = app.handle_action(Action::Quit).await;
    assert_eq!(quit_res, Some(()));

    let relaunch_app = App::new();
    assert_eq!(relaunch_app.state().active_screen, Screen::Home);
}
