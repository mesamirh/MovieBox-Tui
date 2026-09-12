use moviebox_tui::providers::ReleaseProvider;
use moviebox_tui::providers::moviebox::client::MovieBoxClient;

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_movie_stream_real_urls() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    // Ek Deewane Ki Deewaniyat subject_id: 4179386086617137184
    let releases = client
        .episode_streams("4179386086617137184", 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let notice_hash = "1c7de0bd3393702d9191801f15f88f8d";
    let mut real_streams_found = 0;

    for release in &releases {
        if let Some(link) = release.direct_url() {
            println!("Movie stream link: {}", link);
            assert!(
                !link.contains(notice_hash),
                "Stream URL should NOT be the legacy upgrade notice video: {}",
                link
            );
            assert!(
                link.contains("sacdn.hakunaymatata.com") && link.ends_with("/index.mpd"),
                "Stream URL should be a valid MPEG-DASH manifest: {}",
                link
            );
            real_streams_found += 1;
        }
    }

    assert!(
        real_streams_found > 0,
        "Should find at least one real movie stream"
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_series_resolutions_and_streams() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    // Search for a series dynamically
    let search_res = client
        .search("Breaking Bad", 1)
        .await
        .expect("search series");
    let catalog =
        moviebox_tui::providers::moviebox::adapt::moviebox_search_json_to_catalog(&search_res);
    assert!(!catalog.is_empty(), "search should return catalog items");

    let series = catalog
        .iter()
        .find(|item| item.media_type == moviebox_tui::providers::models::MediaType::Series)
        .unwrap_or(&catalog[0]);

    println!(
        "Testing live series: {} ({})",
        series.title, series.id.value
    );

    let releases_ep1 = client
        .episode_streams(&series.id.value, 1, 1)
        .await
        .expect("fetch episode 1 streams");
    assert!(!releases_ep1.is_empty(), "should find episode 1 streams");
    let ep1_url = releases_ep1[0].direct_url().expect("valid direct url");
    println!("Resolved S01E01 direct URL: {}", ep1_url);
    assert!(ep1_url.contains("_1_1_"));

    let releases_ep2 = client
        .episode_streams(&series.id.value, 1, 2)
        .await
        .expect("fetch episode 2 streams");
    assert!(!releases_ep2.is_empty(), "should find episode 2 streams");
    let ep2_url = releases_ep2[0].direct_url().expect("valid direct url");
    println!("Resolved S01E02 direct URL: {}", ep2_url);
    assert!(ep2_url.contains("_1_2_"));
    assert_ne!(
        ep1_url, ep2_url,
        "different episodes must have different manifests"
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_multiple_seasons_and_episodes_return_distinct_streams() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    for (subject_id, series_title, test_cases) in [
        (
            "6207982430134357800",
            "Breaking Bad",
            vec![(1, 1), (1, 2), (2, 1), (2, 2)],
        ),
        (
            "4585605580068379856",
            "One Piece S1-S2",
            vec![(1, 1), (1, 2), (2, 1), (2, 2)],
        ),
    ] {
        println!("\n=== Verifying {} ({}) ===", series_title, subject_id);
        let mut urls = Vec::new();

        for (season, episode) in test_cases {
            let releases = client
                .episode_streams(subject_id, season, episode)
                .await
                .expect("fetch streams");
            assert!(
                !releases.is_empty(),
                "expected streams for {} S{:02}E{:02}",
                series_title,
                season,
                episode
            );
            let url = releases[0]
                .direct_url()
                .expect("must have direct url")
                .to_string();
            println!(
                "{} S{:02}E{:02} -> filename: {}, url: {}",
                series_title, season, episode, releases[0].filename, url
            );
            assert!(
                releases[0]
                    .filename
                    .contains(&format!("S{:02}E{:02}", season, episode)),
                "filename must match season and episode"
            );
            urls.push(((season, episode), url));
        }

        for i in 0..urls.len() {
            for j in (i + 1)..urls.len() {
                assert_ne!(
                    urls[i].1, urls[j].1,
                    "Streams for {} S{:02}E{:02} and S{:02}E{:02} must not be the same! Got identical URL: {}",
                    series_title, urls[i].0.0, urls[i].0.1, urls[j].0.0, urls[j].0.1, urls[i].1
                );
            }
        }
    }
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_inspect_live_mpd_manifest() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    let releases = client
        .episode_streams("4179386086617137184", 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let mirror = &releases[0].mirrors[0];
    println!("Fetching MPD Manifest from: {}", mirror.resolver_url);

    let mut req = client.http_client().get(&mirror.resolver_url);
    for (name, val) in &mirror.headers {
        req = req.header(name, val);
    }
    let resp = req.send().await.expect("send mpd request");
    let xml = resp.text().await.expect("read mpd xml");
    println!(
        "=== RAW MPD MANIFEST ===\n{}\n========================",
        xml
    );

    // Test quality switching with mpv
    for (target_label, height_constraint, expected_res) in [
        (
            "1080p",
            "bestvideo[height<=1080]+bestaudio/best",
            "1920x1080",
        ),
        ("720p", "bestvideo[height<=720]+bestaudio/best", "1280x720"),
        ("480p", "bestvideo[height<=480]+bestaudio/best", "854x480"),
    ] {
        let mut cmd = moviebox_tui::player::command(
            moviebox_tui::player::PlayerKind::Mpv,
            &mirror.resolver_url,
            None,
            &mirror.headers,
            None,
            None,
            None,
        );
        cmd.arg("--vo=null")
            .arg("--ao=null")
            .arg("--frames=15")
            .arg(format!("--ytdl-format={height_constraint}"));

        let output = cmd.output().expect("run mpv with quality format");
        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{out}\n{err}");
        println!("Quality [{target_label}] output:\n{}", combined);
        assert!(
            combined.contains(expected_res),
            "mpv should select resolution {} for quality {}",
            expected_res,
            target_label
        );
    }
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_mpv_end_to_end_playback() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    let releases = client
        .episode_streams("4179386086617137184", 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let release = &releases[0];
    let mirror = &release.mirrors[0];

    println!(
        "Testing mpv playback on live manifest: {}",
        mirror.resolver_url
    );

    let mut cmd = moviebox_tui::player::command(
        moviebox_tui::player::PlayerKind::Mpv,
        &mirror.resolver_url,
        None,
        &mirror.headers,
        None,
        None,
        None,
    );

    cmd.arg("--vo=null").arg("--ao=null").arg("--frames=20");

    let output = cmd.output().expect("execute mpv command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    println!("MPV execution output:\n{}", combined);
    assert!(
        combined.contains("Video") || combined.contains("hevc"),
        "mpv should detect video stream"
    );
    assert!(
        combined.contains("Audio") || combined.contains("aac"),
        "mpv should detect audio stream"
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_dynamic_movie_mpv_playback() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    let search_res = client.search("Avengers", 1).await.expect("search Avengers");
    let catalog =
        moviebox_tui::providers::moviebox::adapt::moviebox_search_json_to_catalog(&search_res);
    assert!(!catalog.is_empty(), "search should return results");

    let movie = catalog
        .iter()
        .find(|item| item.media_type == moviebox_tui::providers::models::MediaType::Movie)
        .unwrap_or(&catalog[0]);

    println!(
        "Testing live dynamic movie: {} ({})",
        movie.title, movie.id.value
    );

    let releases = client
        .episode_streams(&movie.id.value, 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let mirror = &releases[0].mirrors[0];
    println!("Dynamic movie direct URL: {}", mirror.resolver_url);

    let mut cmd = moviebox_tui::player::command(
        moviebox_tui::player::PlayerKind::Mpv,
        &mirror.resolver_url,
        None,
        &mirror.headers,
        None,
        None,
        None,
    );

    cmd.arg("--vo=null").arg("--ao=null").arg("--frames=20");

    let output = cmd.output().expect("execute mpv command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    println!("MPV execution output:\n{}", combined);
    assert!(
        combined.contains("Video") || combined.contains("Audio"),
        "mpv should successfully decode stream"
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_dash_download_stream_with_headers() {
    let client = MovieBoxClient::new();
    client.init().await.expect("client init successful");

    let releases = client
        .episode_streams("4179386086617137184", 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let mirror = &releases[0].mirrors[0];
    assert!(
        !mirror.headers.is_empty(),
        "mirror should contain auth headers"
    );

    let dest = std::env::temp_dir().join("test_live_moviebox_download.mp4");
    let dest_str = dest.to_string_lossy().into_owned();
    let part_dest = std::env::temp_dir().join("test_live_moviebox_download.mp4.part");
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&part_dest);

    let mut cmd = std::process::Command::new("yt-dlp");
    for (k, v) in &mirror.headers {
        if k.eq_ignore_ascii_case("user-agent") {
            cmd.arg("--user-agent").arg(v);
        } else {
            cmd.arg("--add-header").arg(format!("{k}: {v}"));
        }
    }
    cmd.arg("-f")
        .arg("bestvideo+bestaudio/best")
        .arg("--newline")
        .arg("--part")
        .arg("-o")
        .arg(&dest_str)
        .arg("--force-overwrites")
        .arg(&mirror.resolver_url);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().expect("spawn yt-dlp");
    let stdout = child.stdout.take().expect("capture stdout");

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(stdout);
    let mut saw_progress = false;

    for l in reader.lines().map_while(Result::ok) {
        println!("yt-dlp output: {l}");
        if l.contains("[download]") && l.contains('%') {
            saw_progress = true;
            break;
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&part_dest);
    assert!(
        saw_progress,
        "yt-dlp must start downloading MovieBox DASH fragments without 403 Forbidden"
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_session_persistence_and_reuse() {
    let client1 = MovieBoxClient::new();
    let token1 = client1.ensure_session().await.expect("ensure session 1");
    assert!(!token1.is_empty(), "token1 should not be empty");

    // Client 2 without explicit init should load persisted session
    let client2 = MovieBoxClient::new();
    let token2 = client2.ensure_session().await.expect("ensure session 2");
    assert_eq!(
        token1, token2,
        "client2 must reuse the persisted valid session token"
    );

    // Perform search with client 2
    let search_res = client2
        .search("Inception", 1)
        .await
        .expect("search with client2");
    assert!(!search_res.is_null());

    // Invalidation test
    client2.invalidate_session();
    let token3 = client2
        .ensure_session()
        .await
        .expect("ensure session 3 after invalidation");
    assert!(!token3.is_empty(), "token3 should be acquired");
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_fourkhdhub_movie_resolution() {
    let client = moviebox_tui::providers::fourkhdhub::FourKHdHubClient::new()
        .expect("fourkhdhub client creation");
    let items = client.search("Inception").await.expect("search Inception");
    assert!(!items.is_empty(), "Inception search should return results");

    let target = &items[0];
    println!("Found item: {} ({:?})", target.title, target.id);
    let releases = client
        .releases(&target.id.value, 0, 0)
        .await
        .expect("fetch movie releases");
    assert!(!releases.is_empty(), "releases should not be empty");

    for release in &releases {
        println!("Attempting resolve for: {}", release.filename);
        let start = std::time::Instant::now();
        match client.resolve_release(release).await {
            Ok(source) => {
                println!(
                    "Resolved in {:?}: {} [{}]",
                    start.elapsed(),
                    source.url,
                    source.source_label
                );
                assert!(source.url.starts_with("https://"));
                break;
            }
            Err(e) => {
                println!("Failed fast in {:?}: {e}", start.elapsed());
                assert!(start.elapsed() < std::time::Duration::from_secs(12));
            }
        }
    }
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_fourkhdhub_game_of_thrones_resolution() {
    let client = moviebox_tui::providers::fourkhdhub::FourKHdHubClient::new()
        .expect("fourkhdhub client creation");
    let items = client
        .search("Game of Thrones")
        .await
        .expect("search Game of Thrones");
    assert!(!items.is_empty(), "Game of Thrones search results");

    let target = items
        .iter()
        .find(|item| item.title.to_lowercase().contains("thrones"))
        .expect("Game of Thrones entry");
    println!("Found series: {} ({:?})", target.title, target.id);

    let releases = client
        .releases(&target.id.value, 1, 1)
        .await
        .expect("releases for S01E01");
    assert!(!releases.is_empty(), "S01E01 releases found");

    for release in &releases {
        println!(
            "Attempting resolve for S01E01 release: {}",
            release.filename
        );
        let start = std::time::Instant::now();
        match client.resolve_release(release).await {
            Ok(source) => {
                println!(
                    "SUCCESS in {:?}: {} [{}]",
                    start.elapsed(),
                    source.url,
                    source.source_label
                );
                assert!(source.url.starts_with("https://"));
                break;
            }
            Err(e) => {
                println!("Failed fast in {:?}: {e}", start.elapsed());
                assert!(start.elapsed() < std::time::Duration::from_secs(12));
            }
        }
    }
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_captions_end_to_end() {
    let service = moviebox_tui::service::MovieBoxService::new();
    service.client.init().await.expect("client init successful");

    let releases = service
        .client
        .episode_streams("4179386086617137184", 0, 0)
        .await
        .expect("fetch movie streams");
    assert!(!releases.is_empty(), "releases should not be empty");

    let release = &releases[0];
    let resource_id = release
        .resource_id
        .as_deref()
        .expect("moviebox release must include resource_id");
    assert!(!resource_id.is_empty());

    let sibling_ids = vec!["3264772588333157424".to_string()];
    let captions = service
        .get_ext_captions("4179386086617137184", resource_id, &sibling_ids, 0, 0)
        .await
        .expect("fetch captions");
    assert!(
        captions.len() >= 5,
        "should return aggregated subtitles across dub siblings, got {}",
        captions.len()
    );
    assert!(
        captions
            .iter()
            .all(|c| !c.url.is_empty() && !c.url.contains("aa348f2541d13ffe"))
    );
    assert!(
        captions
            .iter()
            .any(|c| c.name.eq_ignore_ascii_case("English"))
    );
    assert!(
        captions
            .iter()
            .any(|c| c.name.contains("বাংলা") || c.name.eq_ignore_ascii_case("Bengali"))
    );
}

#[tokio::test]
#[ignore = "live network test; run with cargo test --test live_stream_verification -- --ignored"]
async fn test_live_moviebox_breaking_bad_series_captions_latency() {
    let service = moviebox_tui::service::MovieBoxService::new();
    service.client.init().await.expect("client init successful");

    let subject_id = "6207982430134357800";
    let releases = service
        .client
        .episode_streams(subject_id, 1, 1)
        .await
        .expect("fetch episode streams");
    assert!(!releases.is_empty());

    let release = &releases[0];
    let resource_id = release
        .resource_id
        .as_deref()
        .expect("must have resource_id");

    let sibling_ids = vec![
        "1382465867459478920".to_string(),
        "9185019261486531816".to_string(),
    ];

    let t0 = std::time::Instant::now();
    let captions = service
        .get_ext_captions(subject_id, resource_id, &sibling_ids, 1, 1)
        .await
        .expect("fetch captions");
    let elapsed = t0.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "captions resolution should complete within 3 seconds, took {:.2?}",
        elapsed
    );
    assert!(
        captions.len() >= 10,
        "should return >= 10 languages for Breaking Bad S1E1, got {}",
        captions.len()
    );
    assert!(
        captions
            .iter()
            .any(|c| c.name.eq_ignore_ascii_case("English")),
        "must include English"
    );
}
