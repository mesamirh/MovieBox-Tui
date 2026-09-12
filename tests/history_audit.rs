use moviebox_tui::history::{HistoryManager, PendingPlaybackState, WatchHistoryItem};
use moviebox_tui::providers::models::ProviderKind;
use moviebox_tui::tui::action::Action;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::state::Screen;

#[allow(clippy::too_many_arguments)]
fn dummy_history_item(
    provider: &str,
    subject_id: &str,
    title: &str,
    stype: i64,
    release_year: &str,
    season: usize,
    episode: usize,
    duration: Option<u64>,
    progress: u64,
    completed: bool,
) -> WatchHistoryItem {
    WatchHistoryItem {
        provider: provider.to_string(),
        subject_id: subject_id.to_string(),
        title: title.to_string(),
        cover_url: Some(format!("https://img.example.com/{subject_id}.jpg")),
        stype,
        release_year: release_year.to_string(),
        season,
        episode,
        timestamp: 1000,
        duration_seconds: duration,
        progress_seconds: progress,
        completed,
    }
}

#[test]
fn test_streaming_mode_movie_partial_progress_and_resume() {
    let mut manager = HistoryManager::default();
    let movie = dummy_history_item(
        "moviebox",
        "mb_movie_100",
        "Interstellar",
        1,
        "2014",
        0,
        0,
        Some(10140),
        3600,
        false,
    );

    manager.update_progress(movie.clone(), 3600, Some(10140), false);

    assert_eq!(manager.recent.len(), 1);
    let item = manager.recent.first().unwrap();
    assert_eq!(item.provider, "moviebox");
    assert_eq!(item.subject_id, "mb_movie_100");
    assert_eq!(item.progress_seconds, 3600);
    assert_eq!(item.duration_seconds, Some(10140));
    assert!(item.is_in_progress());
    assert!(!item.completed);
    assert_eq!(item.progress_percentage(), Some(35.50296));

    let fetched = manager.get_item("moviebox", "mb_movie_100", 0, 0, Some("Interstellar"));
    assert!(fetched.is_some());
    let fetched_item = fetched.unwrap();
    assert_eq!(fetched_item.progress_seconds, 3600);
    assert!(fetched_item.is_in_progress());
}

#[test]
fn test_addon_mode_movie_partial_progress_and_resume() {
    let mut manager = HistoryManager::default();
    let addon_movie = dummy_history_item(
        "addons",
        "tt1160419",
        "Dune",
        1,
        "2021",
        0,
        0,
        Some(9300),
        4650,
        false,
    );

    manager.update_progress(addon_movie.clone(), 4650, Some(9300), false);

    assert_eq!(manager.recent.len(), 1);
    let item = manager.recent.first().unwrap();
    assert_eq!(item.provider, "addons");
    assert_eq!(item.subject_id, "tt1160419");
    assert_eq!(item.progress_seconds, 4650);
    assert_eq!(item.progress_percentage(), Some(50.0));
    assert!(item.is_in_progress());

    let fetched = manager.get_item("addons", "tt1160419", 0, 0, Some("Dune"));
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().progress_seconds, 4650);
}

#[test]
fn test_cross_mode_isolation_between_moviebox_and_addons() {
    let mut manager = HistoryManager::default();

    let mb_dune = dummy_history_item(
        "moviebox",
        "mb_dune_id",
        "Dune",
        1,
        "2021",
        0,
        0,
        Some(9300),
        2000,
        false,
    );
    let addon_dune = dummy_history_item(
        "addons",
        "tt1160419",
        "Dune",
        1,
        "2021",
        0,
        0,
        Some(9300),
        6000,
        false,
    );

    manager.update_progress(mb_dune.clone(), 2000, Some(9300), false);
    manager.update_progress(addon_dune.clone(), 6000, Some(9300), false);

    assert_eq!(manager.recent.len(), 2);
    assert!(!HistoryManager::is_same_show(&mb_dune, &addon_dune));

    let mb_res = manager
        .get_item("moviebox", "mb_dune_id", 0, 0, None)
        .unwrap();
    assert_eq!(mb_res.provider, "moviebox");
    assert_eq!(mb_res.progress_seconds, 2000);

    let addon_res = manager.get_item("addons", "tt1160419", 0, 0, None).unwrap();
    assert_eq!(addon_res.provider, "addons");
    assert_eq!(addon_res.progress_seconds, 6000);
}

#[test]
fn test_series_episode_advancement_and_completion_tracking() {
    let mut manager = HistoryManager::default();

    let ep1 = dummy_history_item(
        "moviebox",
        "mb_show_99",
        "Severance",
        2,
        "2022",
        1,
        1,
        Some(3600),
        3400,
        true,
    );
    manager.update_progress(ep1.clone(), 3400, Some(3600), true);

    assert!(manager.is_watched("moviebox", "mb_show_99", 1, 1));
    assert!(!manager.is_watched("moviebox", "mb_show_99", 1, 2));
    assert_eq!(manager.recent.len(), 1);
    assert_eq!(manager.recent.first().unwrap().episode, 1);
    assert!(manager.recent.first().unwrap().completed);

    let ep2 = dummy_history_item(
        "moviebox",
        "mb_show_99",
        "Severance",
        2,
        "2022",
        1,
        2,
        Some(3600),
        1200,
        false,
    );
    manager.update_progress(ep2.clone(), 1200, Some(3600), false);

    assert_eq!(manager.recent.len(), 1);
    let current = manager.recent.first().unwrap();
    assert_eq!(current.season, 1);
    assert_eq!(current.episode, 2);
    assert_eq!(current.progress_seconds, 1200);
    assert!(current.is_in_progress());
    assert!(!current.completed);

    assert!(manager.is_watched("moviebox", "mb_show_99", 1, 1));
    assert!(!manager.is_watched("moviebox", "mb_show_99", 1, 2));

    let ep1_reopen = dummy_history_item(
        "moviebox",
        "mb_show_99",
        "Severance",
        2,
        "2022",
        1,
        1,
        Some(3600),
        3600,
        true,
    );
    manager.mark_watched(ep1_reopen);

    assert!(manager.is_watched("moviebox", "mb_show_99", 1, 1));
}

#[test]
fn test_threshold_boundaries_for_is_in_progress() {
    let mut item = dummy_history_item(
        "moviebox",
        "m_test",
        "Test Film",
        1,
        "2023",
        0,
        0,
        Some(1000),
        0,
        false,
    );

    item.progress_seconds = 0;
    assert!(!item.is_in_progress());

    item.progress_seconds = 29;
    assert!(!item.is_in_progress());

    item.progress_seconds = 30;
    assert!(item.is_in_progress());

    item.progress_seconds = 500;
    assert!(item.is_in_progress());

    item.progress_seconds = 899;
    assert!(item.is_in_progress());

    item.progress_seconds = 900;
    assert!(!item.is_in_progress());

    item.progress_seconds = 999;
    assert!(!item.is_in_progress());

    item.completed = true;
    item.progress_seconds = 500;
    assert!(!item.is_in_progress());
}

#[test]
fn test_history_disk_persistence_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let history_file = temp_dir.path().join("history.json");

    let mut manager = HistoryManager::default();
    let item1 = dummy_history_item(
        "moviebox",
        "id1",
        "Movie 1",
        1,
        "2020",
        0,
        0,
        Some(5000),
        2500,
        false,
    );
    let item2 = dummy_history_item(
        "addons",
        "tt999999",
        "Series 1",
        2,
        "2021",
        1,
        3,
        Some(3000),
        3000,
        true,
    );

    manager.update_progress(item1, 2500, Some(5000), false);
    manager.update_progress(item2, 3000, Some(3000), true);

    let serialized = serde_json::to_string(&manager).unwrap();
    std::fs::write(&history_file, serialized.as_bytes()).unwrap();

    let read_content = std::fs::read_to_string(&history_file).unwrap();
    let loaded: HistoryManager = serde_json::from_str(&read_content).unwrap();

    assert_eq!(loaded.recent.len(), 2);
    assert!(loaded.is_watched("addons", "tt999999", 1, 3));
    assert!(!loaded.is_watched("moviebox", "id1", 0, 0));
}

#[test]
fn test_reconciliation_from_lua_tracker_state_files() {
    let temp_dir = tempfile::tempdir().unwrap();

    let state1 = PendingPlaybackState {
        provider: "moviebox".to_string(),
        subject_id: "show_alpha".to_string(),
        season: 1,
        episode: 1,
        progress_seconds: 3600,
        duration_seconds: Some(3600),
        completed: true,
        timestamp: 1100,
        title: None,
        cover_url: None,
        stype: None,
        release_year: None,
    };
    let state_file_1 = temp_dir.path().join("moviebox_show_alpha_1_1.json");
    std::fs::write(&state_file_1, serde_json::to_string(&state1).unwrap()).unwrap();

    let state2 = PendingPlaybackState {
        provider: "moviebox".to_string(),
        subject_id: "show_alpha".to_string(),
        season: 1,
        episode: 2,
        progress_seconds: 1500,
        duration_seconds: Some(3600),
        completed: false,
        timestamp: 1200,
        title: None,
        cover_url: None,
        stype: None,
        release_year: None,
    };
    let state_file_2 = temp_dir.path().join("moviebox_show_alpha_1_2.json");
    std::fs::write(&state_file_2, serde_json::to_string(&state2).unwrap()).unwrap();

    let mut manager = HistoryManager::default();
    manager.recent.push(dummy_history_item(
        "moviebox",
        "show_alpha",
        "Alpha Show",
        2,
        "2023",
        1,
        1,
        Some(3600),
        3600,
        true,
    ));

    let modified = manager.reconcile_from_dir(temp_dir.path());
    assert!(modified);

    assert!(manager.is_watched("moviebox", "show_alpha", 1, 1));
    assert!(!manager.is_watched("moviebox", "show_alpha", 1, 2));

    let current = manager.recent.first().unwrap();
    assert_eq!(current.season, 1);
    assert_eq!(current.episode, 2);
    assert_eq!(current.progress_seconds, 1500);

    assert!(!state_file_1.exists());
    assert!(!state_file_2.exists());
}

#[tokio::test]
async fn test_history_slash_command_populates_search_results_accurately() {
    let mut app = App::new();
    app.state_mut().history.clear();

    let item_mb = dummy_history_item(
        "moviebox",
        "mb_101",
        "Gladiator",
        1,
        "2000",
        0,
        0,
        Some(9000),
        4500,
        false,
    );
    let item_addon = dummy_history_item(
        "addons",
        "tt0111161",
        "The Shawshank Redemption",
        1,
        "1994",
        0,
        0,
        Some(8500),
        4250,
        false,
    );

    app.state_mut()
        .history
        .update_progress(item_mb, 4500, Some(9000), false);
    app.state_mut()
        .history
        .update_progress(item_addon, 4250, Some(8500), false);

    app.state_mut().active_screen = Screen::Home;
    app.handle_action(Action::Search {
        query: "/history".to_string(),
        force_refresh: false,
    })
    .await;

    assert_eq!(app.state().search_results.len(), 2);
    let titles: Vec<_> = app
        .state()
        .search_results
        .iter()
        .map(|r| r.title.as_str())
        .collect();
    assert!(titles.contains(&"Gladiator"));
    assert!(titles.contains(&"The Shawshank Redemption"));

    let providers: Vec<_> = app
        .state()
        .search_results
        .iter()
        .map(|r| r.provider)
        .collect();
    assert!(providers.contains(&ProviderKind::MovieBox));
    assert!(providers.contains(&ProviderKind::Addons));
}

#[tokio::test]
async fn test_empty_history_slash_command_settles_search_state() {
    let mut app = App::new();
    app.state_mut().history.clear();
    app.state_mut().active_screen = Screen::Home;
    app.handle_action(Action::Search {
        query: "/history".to_string(),
        force_refresh: false,
    })
    .await;

    assert!(app.state().search_results.is_empty());
    assert!(!app.state().is_loading);
    assert!(app.state().has_search_settled);
}
#[test]
fn test_update_progress_precision_preservation() {
    let mut manager = HistoryManager::default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut item = dummy_history_item(
        "moviebox",
        "mb_m2",
        "Interstellar",
        1,
        "2014",
        0,
        0,
        Some(10140),
        5000,
        false,
    );
    item.timestamp = now;

    manager.recent.push(item.clone());

    let lower_item = item.clone();
    manager.update_progress(lower_item, 3000, Some(10140), false);

    assert_eq!(manager.recent.first().unwrap().progress_seconds, 5000);
}

#[test]
fn test_duplicate_history_prevention_on_repeated_play() {
    let mut manager = HistoryManager::default();
    let item = dummy_history_item(
        "moviebox",
        "mb_m3",
        "The Matrix",
        1,
        "1999",
        0,
        0,
        Some(8160),
        1200,
        false,
    );

    manager.update_progress(item.clone(), 1200, Some(8160), false);
    manager.update_progress(item.clone(), 2400, Some(8160), false);
    manager.update_progress(item.clone(), 3600, Some(8160), false);

    assert_eq!(manager.recent.len(), 1);
    assert_eq!(manager.recent.first().unwrap().progress_seconds, 3600);
}

#[test]
fn test_corrupted_history_deserialization_recovery() {
    let malformed_json = "{ \"watched\": [\"corrupt\"], \"recent\": \"invalid_shape\" }";
    let deserialized = serde_json::from_str::<HistoryManager>(malformed_json);
    assert!(deserialized.is_err());

    let empty_json = "{}";
    let empty_manager = serde_json::from_str::<HistoryManager>(empty_json).unwrap();
    assert!(empty_manager.recent.is_empty());
}
#[test]
fn test_reconciliation_self_heals_unseen_items_with_metadata() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state_file = temp_dir.path().join("moviebox_new_movie_0_0.json");
    let state = PendingPlaybackState {
        provider: "moviebox".to_string(),
        subject_id: "new_movie".to_string(),
        season: 0,
        episode: 0,
        progress_seconds: 4200,
        duration_seconds: Some(7200),
        completed: false,
        timestamp: 5000,
        title: Some("Inception".to_string()),
        cover_url: Some("https://example.com/inception.jpg".to_string()),
        stype: Some(1),
        release_year: Some("2010".to_string()),
    };
    std::fs::write(&state_file, serde_json::to_string(&state).unwrap()).unwrap();

    let mut manager = HistoryManager::default();
    assert!(manager.recent.is_empty());

    let modified = manager.reconcile_from_dir(temp_dir.path());
    assert!(modified);
    assert_eq!(manager.recent.len(), 1);

    let recovered = manager.recent.first().unwrap();
    assert_eq!(recovered.title, "Inception");
    assert_eq!(recovered.subject_id, "new_movie");
    assert_eq!(recovered.progress_seconds, 4200);
    assert_eq!(recovered.duration_seconds, Some(7200));
    assert_eq!(recovered.release_year, "2010");
    assert_eq!(recovered.stype, 1);
    assert!(!state_file.exists());
}

#[test]
fn test_record_start_registers_history_immediately() {
    let mut manager = HistoryManager::default();
    let item = dummy_history_item(
        "moviebox",
        "interstellar",
        "Interstellar",
        1,
        "2014",
        0,
        0,
        Some(10140),
        0,
        false,
    );

    manager.record_start(&item, 120);
    assert_eq!(manager.recent.len(), 1);
    let first = manager.recent.first().unwrap();
    assert_eq!(first.subject_id, "interstellar");
    assert_eq!(first.progress_seconds, 120);
    assert!(!first.completed);

    manager.record_start(&item, 60);
    let second = manager.recent.first().unwrap();
    assert_eq!(second.progress_seconds, 120);

    manager.record_start(&item, 500);
    let third = manager.recent.first().unwrap();
    assert_eq!(third.progress_seconds, 500);
}

#[test]
fn test_pending_playback_state_from_item_fidelity() {
    let item = dummy_history_item(
        "moviebox",
        "oppenheimer",
        "Oppenheimer",
        1,
        "2023",
        0,
        0,
        Some(10800),
        3600,
        false,
    );

    let pending = PendingPlaybackState::from_item(&item, 3600, Some(10800), false);
    assert_eq!(pending.provider, "moviebox");
    assert_eq!(pending.subject_id, "oppenheimer");
    assert_eq!(pending.title.as_deref(), Some("Oppenheimer"));
    assert_eq!(pending.release_year.as_deref(), Some("2023"));
    assert_eq!(pending.stype, Some(1));
    assert_eq!(pending.progress_seconds, 3600);
    assert_eq!(pending.duration_seconds, Some(10800));
    assert!(!pending.completed);
}
