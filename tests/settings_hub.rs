use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use moviebox_tui::{
    player::PlayerKind,
    tui::{
        action::Action,
        app::App,
        commands::SlashCommand,
        state::{AppState, SettingsCategory, settings_player_label},
        widgets::settings::{
            category_tab_rects, settings_category_tab_at, settings_row_at, settings_row_rects,
        },
    },
};
use ratatui::layout::Rect;

#[test]
fn test_settings_command_parsing_and_aliases() {
    let state = AppState::default();
    assert!(SlashCommand::Settings.is_available(&state));
    assert_eq!(SlashCommand::Settings.name(), "/settings");
    assert_eq!(
        SlashCommand::parse("/settings"),
        Some(SlashCommand::Settings)
    );
    assert_eq!(SlashCommand::parse("/pref"), Some(SlashCommand::Settings));
    assert_eq!(
        SlashCommand::parse("/preferences"),
        Some(SlashCommand::Settings)
    );
    assert_eq!(
        SlashCommand::parse("/options"),
        Some(SlashCommand::Settings)
    );
    assert_eq!(SlashCommand::parse("/config"), Some(SlashCommand::Settings));
    assert_eq!(SlashCommand::parse("/theme"), None);
    assert_eq!(SlashCommand::parse("/themes"), None);
    assert_eq!(SlashCommand::parse("/help"), Some(SlashCommand::Help));
    assert_eq!(SlashCommand::parse("/?"), Some(SlashCommand::Help));

    let suggestions = SlashCommand::suggest(&state, "/");
    assert_eq!(
        suggestions,
        vec![
            "/settings".to_string(),
            "/browse".to_string(),
            "/history".to_string(),
            "/favorites".to_string(),
            "/clear".to_string(),
            "/help".to_string(),
        ]
    );

    let s_sug = SlashCommand::suggest(&state, "/s");
    assert_eq!(s_sug, vec!["/settings".to_string()]);

    let c_sug = SlashCommand::suggest(&state, "/c");
    assert_eq!(c_sug, vec!["/clear".to_string()]);

    let p_sug = SlashCommand::suggest(&state, "/p");
    assert!(p_sug.is_empty());

    let toggle_suggestions = SlashCommand::suggest(&state, "/toggle");
    assert!(toggle_suggestions.is_empty());
    assert!(SlashCommand::suggest(&state, "/t").is_empty());
}

#[test]
fn test_settings_modal_has_active_modal() {
    let mut state = AppState::default();
    assert!(!state.has_active_modal());

    state.show_settings_popup = true;
    assert!(state.has_active_modal());

    state.show_settings_popup = false;
    assert!(!state.has_active_modal());
}

#[test]
fn test_settings_toggle_modes_safety_guard() {
    let mut state = AppState {
        streaming_enabled: true,
        tv_enabled: false,
        addons_enabled: false,
        ..Default::default()
    };
    assert!(!state.can_disable_streaming_mode());
    assert!(state.can_disable_tv_mode());

    state.tv_enabled = true;
    assert!(state.can_disable_streaming_mode());
    assert!(state.can_disable_tv_mode());
}

#[test]
fn test_settings_player_choices_and_labels() {
    let mut state = AppState::default();
    assert_eq!(state.settings_player_choices(), Vec::<&str>::new());

    state.available_players = vec![PlayerKind::Mpv, PlayerKind::Vlc, PlayerKind::Iina];
    assert_eq!(state.settings_player_choices(), vec!["mpv", "vlc", "iina"]);

    state.default_player = Some("custom_player".to_string());
    assert_eq!(state.settings_player_choices(), vec!["mpv", "vlc", "iina"]);

    assert_eq!(settings_player_label(None), "None");
    assert_eq!(settings_player_label(Some("auto")), "None");
    assert_eq!(settings_player_label(Some("mpv")), "mpv");
    assert_eq!(settings_player_label(Some("vlc")), "VLC");
    assert_eq!(settings_player_label(Some("iina")), "IINA");
    assert_eq!(settings_player_label(Some("android")), "Android Player");
    assert_eq!(settings_player_label(Some("custom_player")), "Custom");
}

#[test]
fn test_settings_player_and_theme_cycling() {
    let mut state = AppState {
        available_players: vec![PlayerKind::Mpv, PlayerKind::Vlc],
        ..Default::default()
    };
    assert_eq!(state.default_player, None);

    state.cycle_settings_player(true);
    assert_eq!(state.default_player, Some("vlc".to_string()));

    state.cycle_settings_player(true);
    assert_eq!(state.default_player, Some("mpv".to_string()));

    state.cycle_settings_player(false);
    assert_eq!(state.default_player, Some("vlc".to_string()));

    state.cycle_settings_player(false);
    assert_eq!(state.default_player, Some("mpv".to_string()));

    let original_theme = state.active_theme_kind.clone();
    let next_theme = state.cycle_settings_theme(true);
    assert_ne!(original_theme, next_theme);
    assert_eq!(state.active_theme_kind, next_theme);
}

#[tokio::test]
async fn test_settings_keyboard_navigation_and_actions() {
    let mut app = App::new();

    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);
    assert_eq!(app.state().settings_category, SettingsCategory::General);
    assert_eq!(app.state().settings_selected_row, 0);

    app.handle_action(Action::Key(KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::empty(),
    )))
    .await;
    assert_eq!(
        app.state().settings_category,
        SettingsCategory::ContentModes
    );

    app.handle_action(Action::Key(KeyEvent::new(
        KeyCode::BackTab,
        KeyModifiers::empty(),
    )))
    .await;
    assert_eq!(app.state().settings_category, SettingsCategory::General);

    app.handle_action(Action::Key(KeyEvent::new(
        KeyCode::Down,
        KeyModifiers::empty(),
    )))
    .await;
    assert_eq!(app.state().settings_selected_row, 1);

    app.state_mut().available_players = vec![PlayerKind::Mpv, PlayerKind::Vlc];
    app.state_mut().default_player = None;
    app.handle_action(Action::SettingsAdjustValue(true)).await;
    assert_eq!(app.state().default_player, Some("vlc".to_string()));

    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().player_picker_popup);
    assert!(app.state().settings_player_picker);

    app.handle_action(Action::MoveDown).await;
    assert_eq!(app.state().player_picker_state.selected(), Some(0));

    app.handle_action(Action::Submit).await;
    assert!(!app.state().player_picker_popup);
    assert!(!app.state().settings_player_picker);
    assert!(app.state().show_settings_popup);
    assert_eq!(app.state().default_player, Some("mpv".to_string()));
    app.handle_action(Action::Key(KeyEvent::new(
        KeyCode::Esc,
        KeyModifiers::empty(),
    )))
    .await;
    assert!(!app.state().show_settings_popup);
}

#[tokio::test]
async fn test_settings_modes_toggle_keeps_popup_open() {
    let mut app = App::new();
    app.state_mut().streaming_enabled = true;
    app.state_mut().tv_enabled = true;
    app.state_mut().addons_enabled = true;
    app.state_mut()
        .set_mode(moviebox_tui::tui::state::AppMode::Streaming);

    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);

    app.handle_action(Action::SelectSettingsCategory(
        SettingsCategory::ContentModes,
    ))
    .await;
    assert!(app.state().show_settings_popup);
    assert_eq!(
        app.state().settings_category,
        SettingsCategory::ContentModes
    );

    app.state_mut().settings_selected_row = 1;
    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().show_settings_popup);
    assert!(app.state().show_sources_popup);
    app.state_mut().show_sources_popup = false;

    app.state_mut().settings_selected_row = 2;
    app.state_mut().tv_enabled = false;
    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().show_settings_popup);
    assert!(app.state().tv_enabled);
    app.state_mut().settings_selected_row = 0;
    app.state_mut().streaming_enabled = true;
    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().show_settings_popup);
    assert!(!app.state().streaming_enabled);
    assert!(app.state().is_tv_mode);

    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().show_settings_popup);

    app.state_mut()
        .set_mode(moviebox_tui::tui::state::AppMode::Streaming);
    app.state_mut().streaming_enabled = true;
    app.state_mut().tv_enabled = true;
    app.state_mut().addons_enabled = true;
    app.handle_action(Action::CloseSettingsPopup).await;
    assert!(!app.state().show_settings_popup);
}

#[tokio::test]
async fn test_settings_appearance_theme_cycle_and_popup() {
    let mut app = App::new();
    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);

    app.handle_action(Action::SelectSettingsCategory(SettingsCategory::Appearance))
        .await;
    assert!(app.state().show_settings_popup);
    assert_eq!(app.state().settings_category, SettingsCategory::Appearance);

    let initial_theme = app.state().active_theme_kind.clone();
    app.handle_action(Action::SettingsAdjustValue(true)).await;
    assert!(app.state().show_settings_popup);
    assert!(!app.state().show_theme_popup);
    assert_ne!(app.state().active_theme_kind, initial_theme);

    app.handle_action(Action::SettingsActivateRow).await;
    assert!(app.state().show_settings_popup);
    assert!(app.state().show_theme_popup);

    app.handle_action(Action::Key(KeyEvent::new(
        KeyCode::Esc,
        KeyModifiers::empty(),
    )))
    .await;
    assert!(app.state().show_settings_popup);
    assert!(!app.state().show_theme_popup);
    app.handle_action(Action::CloseSettingsPopup).await;
    assert!(!app.state().show_settings_popup);
}

#[tokio::test]
async fn test_settings_mouse_interaction() {
    let mut app = App::new();
    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);

    app.handle_action(Action::MouseClick(0, 0)).await;
    assert!(!app.state().show_settings_popup);
}

#[tokio::test]
async fn test_settings_mouse_tab_and_row_clicks() {
    let mut app = App::new();
    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);
    assert_eq!(app.state().settings_category, SettingsCategory::General);

    let popup = Rect::new(4, 4, 76, 17);
    let cat = settings_category_tab_at(popup, 7, popup.y + 1, false, SettingsCategory::General);
    assert_eq!(cat, Some(SettingsCategory::General));

    let cat_modes =
        settings_category_tab_at(popup, 19, popup.y + 1, false, SettingsCategory::General);
    assert_eq!(cat_modes, Some(SettingsCategory::ContentModes));

    let cat_appearance =
        settings_category_tab_at(popup, 37, popup.y + 1, false, SettingsCategory::General);
    assert_eq!(cat_appearance, Some(SettingsCategory::Appearance));

    let cat_maint =
        settings_category_tab_at(popup, 52, popup.y + 1, false, SettingsCategory::General);
    assert_eq!(cat_maint, Some(SettingsCategory::StorageInfo));

    let rows = settings_row_rects(popup, SettingsCategory::General);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        settings_row_at(popup, SettingsCategory::General, rows[0].x + 2, rows[0].y),
        Some(0)
    );
    assert_eq!(
        settings_row_at(
            popup,
            SettingsCategory::General,
            rows[0].x + 2,
            rows[0].y + 1
        ),
        Some(1)
    );
    assert_eq!(
        settings_row_at(popup, SettingsCategory::General, rows[1].x + 2, rows[1].y),
        Some(1)
    );
    assert_eq!(
        settings_row_at(popup, SettingsCategory::General, rows[2].x + 2, rows[2].y),
        Some(2)
    );
}

#[test]
fn test_settings_compact_tabs_rects() {
    let popup = Rect::new(4, 2, 60, 20);
    let tabs_area = Rect::new(popup.x + 3, popup.y + 1, popup.width - 6, 2);
    let rects = category_tab_rects(tabs_area, false, SettingsCategory::General);
    assert!(!rects.is_empty());

    let hit = settings_category_tab_at(
        popup,
        rects[0].1.x,
        popup.y + 1,
        false,
        SettingsCategory::General,
    );
    assert_eq!(hit, Some(SettingsCategory::General));
}

#[test]
fn test_expand_download_path() {
    assert_eq!(AppState::expand_download_path(""), None);
    assert_eq!(AppState::expand_download_path("default"), None);
    assert_eq!(AppState::expand_download_path("reset"), None);

    let path = AppState::expand_download_path("/tmp/moviebox_downloads");
    assert_eq!(
        path,
        Some(std::path::PathBuf::from("/tmp/moviebox_downloads"))
    );
}

#[tokio::test]
async fn test_settings_hub_clear_cache_activation_and_notification() {
    let mut app = App::new();
    app.handle_action(Action::ShowSettingsPopup).await;
    assert!(app.state().show_settings_popup);

    app.handle_action(Action::SelectSettingsCategory(
        SettingsCategory::StorageInfo,
    ))
    .await;
    assert_eq!(app.state().settings_category, SettingsCategory::StorageInfo);
    assert_eq!(app.state().settings_selected_row, 0);

    app.handle_action(Action::SettingsActivateRow).await;
    let clearing = app.state().notifications.back();
    assert_eq!(clearing.map(|n| n.title.as_str()), Some("Clearing Cache"));

    app.handle_action(Action::ClearCache).await;
    assert!(!app.state().show_settings_popup);
    assert_eq!(
        app.state().active_screen,
        moviebox_tui::tui::state::Screen::Home
    );

    app.handle_action(Action::CacheCleared(Ok(()))).await;
    let notification = app.state().notifications.back();
    assert!(notification.is_some());
    assert_eq!(notification.unwrap().title, "Cache Cleared");

    app.handle_action(Action::CacheCleared(Err("filesystem error".to_string())))
        .await;
    let err_notification = app.state().notifications.back();
    assert!(err_notification.is_some());
    assert_eq!(err_notification.unwrap().title, "Cache Clear Failed");
}

#[tokio::test]
async fn test_settings_hub_browser_open_row_activation() {
    let mut app = App::new();
    app.handle_action(Action::ShowSettingsPopup).await;
    app.handle_action(Action::SelectSettingsCategory(
        SettingsCategory::StorageInfo,
    ))
    .await;
    app.state_mut().settings_selected_row = 2;
    app.handle_action(Action::SettingsActivateRow).await;
    let notification = app.state().notifications.back();
    assert!(notification.is_some());
    let title = &notification.unwrap().title;
    assert!(title == "GitHub" || title == "Browser Launch Failed");
}
