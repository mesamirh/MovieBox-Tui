use moviebox_tui::models::MediaType;
use moviebox_tui::providers::addons::adapter::{
    meta_detail_to_media_details, parse_audio_tracks, parse_codec, parse_quality,
    parse_season_episode, stream_item_to_release,
};
use moviebox_tui::providers::addons::models::{
    AddonManifest, InstalledAddon, MetaDetail, StreamBehaviorHints, StreamItem,
};
use moviebox_tui::providers::models::parse_size_bytes;
use std::collections::HashMap;

#[test]
fn test_addon_manifest_fixture_deserialization() {
    let fixture_content = include_str!("fixtures/addons/manifest.json");
    let manifest: AddonManifest = serde_json::from_str(fixture_content)
        .expect("failed to deserialize Cinemeta addon manifest");

    assert_eq!(manifest.id, "org.stremio.cinemeta");
    assert_eq!(manifest.name, "Cinemeta");
    assert_eq!(manifest.version.as_deref(), Some("3.0.12"));
    assert_eq!(manifest.resources.len(), 3);
    assert_eq!(manifest.resources[0].name(), "catalog");
    assert_eq!(manifest.resources[1].name(), "meta");
    assert_eq!(manifest.resources[2].name(), "stream");

    assert_eq!(manifest.catalogs.len(), 2);
    assert_eq!(manifest.catalogs[0].r#type, "movie");
    assert_eq!(manifest.catalogs[0].id, "top");
    assert_eq!(manifest.catalogs[0].name.as_deref(), Some("Popular Movies"));
    assert_eq!(manifest.catalogs[0].extra.len(), 1);
    assert_eq!(manifest.catalogs[0].extra[0].name, "genre");

    assert!(manifest.provides_catalog());
    assert!(manifest.provides_meta());
    assert!(manifest.provides_stream());
}

#[test]
fn test_addon_series_classification_and_default_season_structure() {
    let off_campus_series = MetaDetail {
        id: "tt32034988".to_string(),
        r#type: "series".to_string(),
        name: "Off Campus".to_string(),
        title: None,
        poster: Some("https://images.metahub.space/poster/medium/tt32034988/img.jpg".to_string()),
        cover: None,
        background: None,
        logo: None,
        description: Some("College drama series".to_string()),
        overview: None,
        synopsis: None,
        release_info: Some("2026".to_string()),
        year: Some("2026".to_string()),
        released: None,
        imdb_rating: Some("7.8".to_string()),
        rating: None,
        genres: vec!["Drama".to_string(), "Romance".to_string()],
        genre: vec![],
        runtime: Some("45 min".to_string()),
        cast: vec![],
        stars: vec![],
        director: vec![],
        directors: vec![],
        writer: vec![],
        writers: vec![],
        videos: vec![],
    };

    let details = meta_detail_to_media_details(&off_campus_series);
    assert_eq!(details.id.value, "series:tt32034988");
    assert_eq!(details.title, "Off Campus");
    assert_eq!(details.media_type, MediaType::Series);
    assert_eq!(details.year.as_deref(), Some("2026"));

    assert!(details.seasons.is_empty());
}

#[test]
fn test_addon_season_episode_token_parsing() {
    assert_eq!(
        parse_season_episode("Off.Campus.S01E08.1080p.mkv"),
        Some((1, 8))
    );
    assert_eq!(
        parse_season_episode("Off.Campus.s1e8.720p.mkv"),
        Some((1, 8))
    );
    assert_eq!(
        parse_season_episode("Off.Campus.S01.E06.1080p.mkv"),
        Some((1, 6))
    );
    assert_eq!(
        parse_season_episode("Off_Campus_S02_E05_HDR.mkv"),
        Some((2, 5))
    );
    assert_eq!(
        parse_season_episode("Breaking.Bad.1x02.720p.mkv"),
        Some((1, 2))
    );
    assert_eq!(
        parse_season_episode("Stranger Things Season 3 Episode 4"),
        Some((3, 4))
    );
    assert_eq!(
        parse_season_episode("Attack on Titan Episode 15 1080p"),
        Some((1, 15))
    );
    assert_eq!(parse_season_episode("Inception 2010 1080p BluRay"), None);
}

#[test]
fn test_addon_episode_stream_filtering_and_isolation() {
    let raw_streams = [
        StreamItem {
            name: Some("HdHub\n1080p".to_string()),
            title: Some("Off.Campus.S01E08.1080p.BluRay.x265.mkv\n1.2 GB".to_string()),
            description: None,
            url: Some("https://cdn.example.com/s01e08_1080p.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("HdHub\n1080p".to_string()),
            title: Some("Off.Campus.S01E06.1080p.BluRay.x265.mkv\n1.1 GB".to_string()),
            description: None,
            url: Some("https://cdn.example.com/s01e06_1080p.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("HdHub\n720p".to_string()),
            title: Some("Off.Campus.S01E08.720p.HD.mkv\n700 MB".to_string()),
            description: None,
            url: Some("https://cdn.example.com/s01e08_720p.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("HdHub\n720p".to_string()),
            title: Some("Off.Campus.S01E07.720p.HD.mkv\n680 MB".to_string()),
            description: None,
            url: Some("https://cdn.example.com/s01e07_720p.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
    ];

    let s01e08_releases: Vec<_> = raw_streams
        .iter()
        .filter_map(|s| stream_item_to_release("HdHub", s, 1, 8))
        .collect();
    assert_eq!(s01e08_releases.len(), 2);
    assert_eq!(
        s01e08_releases[0].mirrors[0].resolver_url,
        "https://cdn.example.com/s01e08_1080p.mp4"
    );
    assert_eq!(
        s01e08_releases[1].mirrors[0].resolver_url,
        "https://cdn.example.com/s01e08_720p.mp4"
    );

    let s01e06_releases: Vec<_> = raw_streams
        .iter()
        .filter_map(|s| stream_item_to_release("HdHub", s, 1, 6))
        .collect();
    assert_eq!(s01e06_releases.len(), 1);
    assert_eq!(
        s01e06_releases[0].mirrors[0].resolver_url,
        "https://cdn.example.com/s01e06_1080p.mp4"
    );

    let s01e07_releases: Vec<_> = raw_streams
        .iter()
        .filter_map(|s| stream_item_to_release("HdHub", s, 1, 7))
        .collect();
    assert_eq!(s01e07_releases.len(), 1);
    assert_eq!(
        s01e07_releases[0].mirrors[0].resolver_url,
        "https://cdn.example.com/s01e07_720p.mp4"
    );
}

#[test]
fn test_addon_movie_stream_resolution_remains_unaffected() {
    let movie_stream = StreamItem {
        name: Some("Cinemeta\n1080p".to_string()),
        title: Some("Inception.2010.1080p.BluRay.x264.mkv\n2.4 GB".to_string()),
        description: None,
        url: Some("https://cdn.example.com/inception_1080p.mp4".to_string()),
        behavior_hints: None,
        ..Default::default()
    };

    let release = stream_item_to_release("Cinemeta", &movie_stream, 0, 0)
        .expect("movie release should parse");
    assert_eq!(release.quality.as_deref(), Some("1080p"));
    assert_eq!(release.season, None);
    assert_eq!(release.episode, None);
    assert_eq!(
        release.mirrors[0].resolver_url,
        "https://cdn.example.com/inception_1080p.mp4"
    );
}

#[test]
fn test_addon_stream_parsing_and_release_mapping() {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Custom)".to_string());
    headers.insert("Referer".to_string(), "https://hubcloud.club/".to_string());

    let stream = StreamItem {
        name: Some("HdHub\n1080p".to_string()),
        title: Some(
            "Breaking.Bad.S01E01.1080p.BluRay.x265.HEVC.Hindi.English-FSL.mkv\n1.15 GB".to_string(),
        ),
        description: None,
        url: Some("https://cdn.hubcloud.club/stream/breakingbad_s01e01.mp4".to_string()),
        behavior_hints: Some(StreamBehaviorHints {
            not_web_ready: false,
            headers: Some(headers),
            video_size: Some(1_234_567_890),
            filename: Some("Breaking.Bad.S01E01.1080p.mkv".to_string()),
        }),
        ..Default::default()
    };

    let release = stream_item_to_release("HdHub", &stream, 1, 1).expect("valid release");
    assert_eq!(release.quality.as_deref(), Some("1080p"));
    assert_eq!(release.codec.as_deref(), Some("HEVC/x265"));
    assert_eq!(release.language.as_deref(), Some("Hindi + English"));
    assert_eq!(release.size_bytes, Some(1_234_567_890));
    assert_eq!(release.season, Some(1));
    assert_eq!(release.episode, Some(1));

    let mirror = &release.mirrors[0];
    assert_eq!(
        mirror.resolver_url,
        "https://cdn.hubcloud.club/stream/breakingbad_s01e01.mp4"
    );
    assert_eq!(mirror.headers.len(), 2);
    assert!(
        mirror
            .headers
            .iter()
            .any(|(k, v)| k == "Referer" && v == "https://hubcloud.club/")
    );
}

#[test]
fn test_addon_token_and_audio_parsers() {
    assert_eq!(
        parse_quality("Movie.2024.2160p.UHD.HDR"),
        Some("2160p".to_string())
    );
    assert_eq!(parse_quality("Movie.1080p.FHD"), Some("1080p".to_string()));
    assert_eq!(parse_quality("Movie.720p.HD"), Some("720p".to_string()));
    assert_eq!(parse_quality("Movie.480p.SD"), Some("480p".to_string()));

    assert_eq!(
        parse_codec("x265 HEVC 10bit"),
        Some("HEVC/x265".to_string())
    );
    assert_eq!(parse_codec("H.264 AVC"), Some("AVC/x264".to_string()));
    assert_eq!(parse_codec("Movie AV1 HDR"), Some("AV1".to_string()));

    assert_eq!(parse_size_bytes("Size: 1.15 GB"), Some(1_234_803_097));
    assert_eq!(parse_size_bytes("File size: 850 MB"), Some(891_289_600));

    assert_eq!(
        parse_audio_tracks("Hindi + English Dual Audio"),
        Some("Hindi + English + Dual Audio".to_string())
    );
    assert_eq!(
        parse_audio_tracks("Japanese Audio [Eng Sub]"),
        Some("English + Japanese".to_string())
    );
}

#[test]
fn test_installed_addon_cinemeta_protection() {
    let cinemeta = InstalledAddon::cinemeta_default();
    assert!(cinemeta.is_core());
    assert!(cinemeta.enabled);
    assert!(cinemeta.provides_catalog);
    assert!(cinemeta.provides_meta);
    assert!(!cinemeta.provides_stream);

    let community_addon = InstalledAddon {
        manifest_url: "https://addon.example.com/manifest.json".to_string(),
        name: "Community Streams".to_string(),
        version: Some("1.0.0".to_string()),
        description: Some("Direct HTTP Streams".to_string()),
        enabled: true,
        provides_catalog: false,
        provides_meta: false,
        provides_stream: true,
        id_prefixes: vec!["tt".to_string()],
        types: vec!["movie".to_string(), "series".to_string()],
        catalogs: vec![],
    };
    assert!(!community_addon.is_core());
}

#[test]
fn test_curated_catalog_presets_includes_installed_addon_catalogs_and_skips_search_only() {
    use moviebox_tui::providers::addons::models::{
        AddonManifest, CatalogDefinition, CatalogExtra, curated_catalog_presets,
        find_catalog_preset,
    };

    let raw_manifest = r#"{
        "id": "org.kitsu.anime",
        "name": "Kitsu Anime",
        "version": "1.0.0",
        "resources": ["catalog", "meta"],
        "types": ["anime", "series"],
        "catalogs": [
            {"type": "anime", "id": "kitsu-anime-trending", "name": "Trending"},
            {"type": "anime", "id": "kitsu-anime-popular", "name": "Popular"},
            {"type": "anime", "id": "kitsu-anime-search", "name": "Search", "extra": [{"name": "search", "isRequired": true}]}
        ]
    }"#;
    let manifest: AddonManifest = serde_json::from_str(raw_manifest).unwrap();
    let kitsu = InstalledAddon::from_manifest(
        "https://kitsu.example.com/manifest.json".to_string(),
        &manifest,
    );
    assert_eq!(kitsu.catalogs.len(), 3);

    let addons = vec![InstalledAddon::cinemeta_default(), kitsu];
    let presets = curated_catalog_presets(&addons);
    assert!(
        presets
            .iter()
            .any(|p| p.catalog_id == "kitsu-anime-trending")
    );
    assert!(
        presets
            .iter()
            .any(|p| p.catalog_id == "kitsu-anime-popular")
    );
    assert!(!presets.iter().any(|p| p.catalog_id == "kitsu-anime-search"));

    let matched = find_catalog_preset(&addons, "trending").expect("trending preset");
    assert_eq!(matched.catalog_id, "kitsu-anime-trending");
    assert_eq!(matched.addon_name, "Kitsu Anime");

    let _ = (CatalogDefinition {
        r#type: "movie".to_string(),
        id: "top".to_string(),
        name: None,
        extra: vec![CatalogExtra {
            name: "genre".to_string(),
            is_required: false,
            options: None,
        }],
    },);
}

#[tokio::test]
async fn test_invalid_addon_manifest_url_shows_actionable_error_and_preserves_state() {
    let client = moviebox_tui::providers::addons::client::AddonClient::new();
    let err = client
        .fetch_manifest("https://invalid-nonexistent-domain.test/manifest.json")
        .await
        .unwrap_err();
    assert!(err.contains("Failed to reach manifest") || err.contains("Manifest returned HTTP"));

    let mut app = moviebox_tui::tui::app::App::new();
    let initial_addons_count = app.state().installed_addons.len();

    app.handle_action(moviebox_tui::tui::action::Action::SetStatus(format!(
        "Error: Addon install failed: {err}"
    )))
    .await;

    assert!(!app.state().notifications.is_empty());
    let notif = app.state().notifications.back().unwrap();
    assert_eq!(notif.kind, moviebox_tui::models::NotificationKind::Error);
    assert_eq!(app.state().installed_addons.len(), initial_addons_count);
}

#[test]
fn test_addon_mixed_stream_filtering_and_magnet_mapping() {
    let raw_streams = vec![
        StreamItem {
            name: Some("HTTP 1080p Stream".to_string()),
            title: Some("Direct Video Stream".to_string()),
            description: None,
            url: Some("https://example.test/stream1.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("HTTP 720p Stream".to_string()),
            title: Some("Direct Video Stream".to_string()),
            description: None,
            url: Some("http://example.test/stream2.mp4".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("Magnet Link".to_string()),
            title: Some("Torrent Stream".to_string()),
            description: None,
            url: Some("magnet:?xt=urn:btih:d08244124e9f0863014f56947ab51404ec102770".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("Local File".to_string()),
            title: Some("File Protocol".to_string()),
            description: None,
            url: Some("file:///etc/passwd".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("FTP Stream".to_string()),
            title: Some("FTP Protocol".to_string()),
            description: None,
            url: Some("ftp://example.test/video.mkv".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("Invalid".to_string()),
            title: Some("Invalid URL".to_string()),
            description: None,
            url: Some("not-a-valid-url-string".to_string()),
            behavior_hints: None,
            ..Default::default()
        },
        StreamItem {
            name: Some("Empty".to_string()),
            title: Some("Empty URL".to_string()),
            description: None,
            url: None,
            behavior_hints: None,
            ..Default::default()
        },
    ];

    let mut accepted_releases = Vec::new();
    for stream in &raw_streams {
        if let Some(release) = stream_item_to_release("Torrentio", stream, 0, 0) {
            accepted_releases.push(release);
        }
    }

    // Two HTTP streams plus the magnet, which now plays through the built-in
    // torrent engine. file://, ftp:// and junk URLs are still rejected.
    assert_eq!(accepted_releases.len(), 3);
    assert_eq!(
        accepted_releases[0].mirrors[0].resolver_url,
        "https://example.test/stream1.mp4"
    );
    assert_eq!(
        accepted_releases[1].mirrors[0].resolver_url,
        "http://example.test/stream2.mp4"
    );
    let engine_prefix = format!(
        "{}/stream?link=magnet",
        moviebox_tui::config::torrent_engine_base()
    );
    assert!(
        accepted_releases[2].mirrors[0]
            .resolver_url
            .starts_with(&engine_prefix)
    );
}

#[tokio::test]
async fn test_addon_enable_disable_and_removal_lifecycle() {
    let mut app = moviebox_tui::tui::app::App::new();
    let initial_count = app.state().installed_addons.len();

    let custom_manifest = serde_json::json!({
        "id": "community.streams.test",
        "version": "1.0.0",
        "name": "Community Direct Streams",
        "description": "Test HTTP streams",
        "resources": ["stream"],
        "types": ["movie"],
        "catalogs": []
    });
    let manifest: AddonManifest = serde_json::from_value(custom_manifest).unwrap();
    let installed =
        InstalledAddon::from_manifest("https://example.test/manifest.json".to_string(), &manifest);

    app.state_mut().installed_addons.push(installed);
    assert_eq!(app.state().installed_addons.len(), initial_count + 1);

    let idx = initial_count;
    app.handle_action(moviebox_tui::tui::action::Action::AddonToggleEnabled(idx))
        .await;
    assert!(!app.state().installed_addons[idx].enabled);

    app.handle_action(moviebox_tui::tui::action::Action::AddonToggleEnabled(idx))
        .await;
    assert!(app.state().installed_addons[idx].enabled);

    app.handle_action(moviebox_tui::tui::action::Action::AddonRemove(idx))
        .await;
    assert_eq!(app.state().installed_addons.len(), initial_count);
}
