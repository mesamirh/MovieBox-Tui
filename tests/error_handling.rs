use moviebox_tui::models::NotificationKind;
use moviebox_tui::providers::addons::client::AddonClient;
use moviebox_tui::providers::models::RequestContext;
use moviebox_tui::providers::tv::parser::M3UParser;
use moviebox_tui::tui::action::Action;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::text::is_http_url;

#[tokio::test]
async fn test_search_failure_clears_loading_and_sets_error_state() {
    let mut app = App::new();
    app.state_mut().is_loading = true;
    app.state_mut().active_search_request = 42;
    app.state_mut().search_query = "Inception".to_string();

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
async fn test_addon_manifest_invalid_url_triggers_error_notification_and_preserves_state() {
    let client = AddonClient::new();
    let err = client
        .fetch_manifest("https://invalid-nonexistent-domain.test/manifest.json")
        .await
        .unwrap_err();
    assert!(err.contains("Failed to reach manifest") || err.contains("Manifest returned HTTP"));

    let mut app = App::new();
    let initial_addons = app.state().installed_addons.len();

    app.handle_action(Action::SetStatus(format!(
        "Error: Addon install failed: {err}"
    )))
    .await;

    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Error);
    assert!(notif.message.contains("Addon install failed"));
    assert_eq!(app.state().installed_addons.len(), initial_addons);
}

#[tokio::test]
async fn test_stream_resolve_failure_resets_resolving_flag_and_notifies_user() {
    let mut app = App::new();
    app.state_mut().is_resolving_playback = true;

    app.handle_action(Action::SetStatus(
        "Error: 4KHDHub source failed: Stream extraction returned HTTP 403 Forbidden".to_string(),
    ))
    .await;

    assert!(!app.state().is_resolving_playback);
    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Error);
    assert!(
        notif
            .message
            .contains("Stream extraction returned HTTP 403")
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
async fn test_malformed_m3u_playlist_recovers_without_panic() {
    let parser = M3UParser::new();
    let malformed_m3u = "#EXTM3U\n#EXTINF:-1 tvg-id=\"\"\n\n#EXTINF:broken attributes without url";
    let channels = parser.parse_m3u(malformed_m3u);
    assert!(channels.is_empty());

    let partial_valid_m3u = "#EXTM3U\n#EXTINF:-1 tvg-name=\"Channel 1\",Valid Channel\nhttps://stream.example.com/live.m3u8\n#BROKEN_LINE";
    let channels2 = parser.parse_m3u(partial_valid_m3u);
    assert_eq!(channels2.len(), 1);
    assert_eq!(channels2[0].name, "Valid Channel");
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

    let res = app.handle_action(Action::PlayStream(false)).await;
    assert_eq!(res, None);
    assert!(!app.state().is_resolving_playback);
}

#[tokio::test]
async fn test_new_playback_request_replaces_active_session_instead_of_blocking() {
    let mut app = App::new();
    app.state_mut().is_playing = true;
    let notifications_before = app.state().notifications.len();

    // A new playback request while one is already active must no longer be
    // refused with a "Playback already active" warning; it should be free to
    // proceed and hand off from the old session.
    let res = app.handle_action(Action::PlayStream(false)).await;
    assert_eq!(res, None);
    assert_eq!(app.state().notifications.len(), notifications_before);
    assert!(
        app.state()
            .notifications
            .iter()
            .all(|n| n.title != "Playback already active")
    );
}

#[tokio::test]
async fn test_stale_player_exit_does_not_clobber_a_newer_playback_session() {
    let mut app = App::new();

    // Simulate a second launch having already bumped the generation past the
    // exiting (replaced) session's generation.
    app.state_mut().playback_generation = 5;
    app.state_mut().is_playing = true;
    app.state_mut().is_resolving_playback = true;

    app.handle_action(Action::PlayerExited(3)).await;
    assert!(app.state().is_playing, "stale exit must not stop playback");
    assert!(app.state().is_resolving_playback);

    app.handle_action(Action::PlayerExited(5)).await;
    assert!(!app.state().is_playing);
    assert!(!app.state().is_resolving_playback);
}

#[tokio::test]
async fn test_stale_player_crash_is_suppressed_but_current_generation_crash_notifies() {
    let mut app = App::new();
    app.state_mut().playback_generation = 5;
    app.state_mut().is_playing = true;
    let notifications_before = app.state().notifications.len();

    app.handle_action(Action::PlayerCrashed(3, Some(1), "stale error".to_string()))
        .await;
    assert!(app.state().is_playing, "stale crash must not stop playback");
    assert_eq!(app.state().notifications.len(), notifications_before);

    app.handle_action(Action::PlayerCrashed(5, Some(1), "real error".to_string()))
        .await;
    assert!(!app.state().is_playing);
    assert_eq!(app.state().notifications.len(), notifications_before + 1);
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, NotificationKind::Error);
    assert_eq!(notif.title, "Player Error");
}
