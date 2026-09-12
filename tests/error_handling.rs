use moviebox_tui::models::NotificationKind;
use moviebox_tui::providers::models::RequestContext;
use moviebox_tui::tui::action::Action;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::state::PlayerKind;
use moviebox_tui::tui::text::is_http_url;

#[tokio::test]
async fn test_search_failure_clears_loading_and_sets_error_state() {
    let mut app = App::new();
    app.state_mut().is_loading = true;
    app.state_mut().active_search_request = 42;
    app.state_mut().search_query.set_content("Inception");

    let context = RequestContext {
        provider: app.state().active_provider,
        generation: app.state().provider_generation,
    };

    app.handle_action(Action::SearchFailure(
        context,
        42,
        1,
        "Network connection timed out after 10s".to_string(),
    ))
    .await;

    assert!(!app.state().is_loading);
    assert_eq!(
        app.state().search_error.as_deref(),
        Some("Network connection timed out after 10s")
    );
    assert!(app.state().search_results.is_empty());
}

#[tokio::test]
async fn test_stream_resolve_failure_resets_resolving_flag_and_notifies_user() {
    let mut app = App::new();
    app.state_mut().is_resolving_playback = true;

    app.handle_action(Action::SetStatus(
        "Error: 4KHDHub: Mirrors for this release are dead or expired on 4KHDHub. Select another release (e.g. 1080p) or press Ctrl+P for MovieBox.".to_string(),
    ))
    .await;

    assert!(!app.state().is_resolving_playback);
    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Error);
    assert_eq!(notif.title, "4KHDHub Stream Unavailable");
    assert!(
        notif
            .message
            .contains("Mirrors for this release are dead or expired")
    );
}

#[tokio::test]
async fn test_download_resolve_failure_sets_error_status_and_notifies() {
    let mut app = App::new();

    app.handle_action(Action::SetStatus(
        "Error: Resolve failed: Stream mirror link expired".to_string(),
    ))
    .await;

    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Error);
    assert!(notif.message.contains("Stream mirror link expired"));
}

#[tokio::test]
async fn test_invalid_url_schemes_rejected_by_security_filter() {
    assert!(!is_http_url("file:///etc/passwd"));
    assert!(!is_http_url("ftp://server.local/file"));
    assert!(!is_http_url("javascript:alert(1)"));
    assert!(!is_http_url("data:text/html;base64,PHNjcmlwdD4="));
    assert!(!is_http_url(""));
    assert!(is_http_url("http://example.com"));
    assert!(is_http_url("https://example.com/manifest.json"));
}

#[tokio::test]
async fn test_rapid_playback_invocations_debounced_and_single_flight() {
    let mut app = App::new();
    app.state_mut().last_playback_launch = std::time::Instant::now();
    app.state_mut().is_resolving_playback = false;

    let res = app.handle_action(Action::PlayStream).await;
    assert_eq!(res, None);
    assert!(!app.state().is_resolving_playback);
}

#[tokio::test]
async fn test_active_player_session_blocks_duplicate_playback_and_recovers_on_exit() {
    let mut app = App::new();
    app.state_mut().is_playing = true;

    let res = app.handle_action(Action::PlayStream).await;
    assert_eq!(res, None);
    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Warning);
    assert_eq!(notif.title, "Playback active");
    app.handle_action(Action::PlayerExited).await;
    assert!(!app.state().is_playing);
    assert!(!app.state().is_resolving_playback);
}
#[tokio::test]
async fn test_authoritative_launch_player_blocks_bypass_attempts() {
    let mut app = App::new();
    app.state_mut().update_available = None;
    app.state_mut().is_playing = false;

    app.handle_action(Action::LaunchPlayer(
        PlayerKind::Mpv,
        "magnet:?xt=urn:btih:d08244124e9f0863014f56947ab51404ec102770".to_string(),
        None,
    ))
    .await;

    assert!(!app.state().is_playing);
    let notif = app
        .state()
        .notifications
        .back()
        .expect("Notification must be present");
    assert_eq!(notif.kind, NotificationKind::Error);
    assert_eq!(notif.title, "Unsupported stream");

    app.handle_action(Action::LaunchPlayer(
        PlayerKind::Mpv,
        "file:///etc/shadow".to_string(),
        None,
    ))
    .await;

    assert!(!app.state().is_playing);
    let notif = app
        .state()
        .notifications
        .back()
        .expect("Notification must be present");
    assert_eq!(notif.kind, NotificationKind::Error);
    assert_eq!(notif.title, "Unsupported stream");
}
