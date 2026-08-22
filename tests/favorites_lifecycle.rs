use moviebox_tui::favorites::FavoriteItem;
use moviebox_tui::providers::models::ProviderKind;
use moviebox_tui::tui::action::Action;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::state::Screen;

#[allow(clippy::too_many_arguments)]
fn dummy_favorite(
    provider: &str,
    subject_id: &str,
    title: &str,
    stype: i64,
    release_year: &str,
    added_at: u64,
) -> FavoriteItem {
    FavoriteItem {
        provider: provider.to_string(),
        subject_id: subject_id.to_string(),
        title: title.to_string(),
        cover_url: Some(format!("https://img.example.com/{subject_id}.jpg")),
        stype,
        release_year: release_year.to_string(),
        added_at,
    }
}

#[test]
fn test_favorites_path_is_outside_clear_cache_scope() {
    // /clear-cache (crate::cache::clear_all_cache) only removes cache_dir()
    // and data_dir()/iptv_cache. favorites.json must live outside both.
    let favorites_path = moviebox_tui::config::favorites_path().expect("favorites path");
    let cache_dir = moviebox_tui::config::cache_dir();
    let data_dir = moviebox_tui::config::data_dir().expect("data dir");
    let iptv_cache = data_dir.join("iptv_cache");

    assert!(!favorites_path.starts_with(&cache_dir));
    assert_ne!(favorites_path.parent().unwrap(), iptv_cache);
    assert!(favorites_path.starts_with(&data_dir));
}

#[test]
fn test_clearing_watch_history_leaves_favorites_intact() {
    let mut app = App::new();
    app.state_mut().favorites.clear();
    app.state_mut()
        .favorites
        .items
        .push(dummy_favorite("moviebox", "mb_1", "Dune", 1, "2021", 1000));
    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox", "mb_2", "Arrival", 1, "2016", 2000,
    ));

    app.state_mut().history.clear();

    assert_eq!(app.state().favorites.items.len(), 2);

    app.state_mut().favorites.clear();
}

#[tokio::test]
async fn test_favorites_slash_command_populates_search_results_accurately() {
    let mut app = App::new();
    app.state_mut().favorites.clear();

    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox",
        "mb_101",
        "Gladiator",
        1,
        "2000",
        1000,
    ));
    app.state_mut().favorites.items.push(dummy_favorite(
        "addons",
        "tt0111161",
        "The Shawshank Redemption",
        1,
        "1994",
        2000,
    ));

    app.state_mut().active_screen = Screen::Home;
    app.handle_action(Action::Search {
        query: "/favorites".to_string(),
        force_refresh: false,
    })
    .await;

    assert_eq!(app.state().search_query, "/favorites");
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

    app.state_mut().favorites.clear();
}

#[tokio::test]
async fn test_favorites_virtual_list_orders_newest_first() {
    let mut app = App::new();
    app.state_mut().favorites.clear();

    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox",
        "mb_old",
        "Oldest Pick",
        1,
        "2001",
        100,
    ));
    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox",
        "mb_new",
        "Newest Pick",
        1,
        "2020",
        3000,
    ));
    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox",
        "mb_mid",
        "Middle Pick",
        1,
        "2010",
        2000,
    ));

    app.state_mut().active_screen = Screen::Home;
    app.handle_action(Action::Search {
        query: "/favorites".to_string(),
        force_refresh: false,
    })
    .await;

    let titles: Vec<_> = app
        .state()
        .search_results
        .iter()
        .map(|r| r.title.clone())
        .collect();
    assert_eq!(
        titles,
        vec![
            "Newest Pick".to_string(),
            "Middle Pick".to_string(),
            "Oldest Pick".to_string(),
        ]
    );

    app.state_mut().favorites.clear();
}

#[tokio::test]
async fn test_toggle_favorite_action_on_home_selected_result() {
    let mut app = App::new();
    app.state_mut().favorites.clear();

    app.state_mut().active_screen = Screen::Home;
    app.state_mut().search_results = vec![moviebox_tui::tui::state::SearchResult {
        id: "mb_toggle".to_string(),
        title: "Interstellar".to_string(),
        stype: 1,
        release_year: "2014".to_string(),
        cover_url: None,
        season: 0,
        episode: 0,
        provider: ProviderKind::MovieBox,
    }];
    app.state_mut().search_list_state.select(Some(0));

    app.handle_action(Action::ToggleFavorite).await;
    assert_eq!(app.state().favorites.items.len(), 1);
    assert_eq!(app.state().favorites.items[0].subject_id, "mb_toggle");

    app.handle_action(Action::ToggleFavorite).await;
    assert!(app.state().favorites.items.is_empty());
}

#[tokio::test]
async fn test_open_favorite_navigates_to_details_screen() {
    let mut app = App::new();
    app.state_mut().favorites.clear();
    app.state_mut().favorites.items.push(dummy_favorite(
        "moviebox", "mb_open", "Arrival", 1, "2016", 5000,
    ));

    app.state_mut().active_screen = Screen::Home;
    app.handle_action(Action::OpenFavorite(0)).await;

    assert_eq!(app.state().active_screen, Screen::Details);
    assert_eq!(app.state().active_subject_id.as_deref(), Some("mb_open"));

    app.state_mut().favorites.clear();
}
