use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

#[tokio::test]
#[ignore = "live network test; run with cargo test --test download_throughput -- --ignored --nocapture"]
async fn measure_download_throughput() {
    let url = std::env::var("MOVIEBOX_SPEEDTEST_URL")
        .unwrap_or_else(|_| "http://ipv4.download.thinkbroadband.com/100MB.zip".to_string());
    let dir = std::env::temp_dir().join("moviebox-throughput");
    tokio::fs::create_dir_all(&dir).await.expect("scratch dir");
    let destination = dir.join("sample.bin");
    let _ = tokio::fs::remove_file(&destination).await;

    let client = moviebox_tui::net::streaming_client_builder()
        .user_agent(moviebox_tui::net::DEFAULT_BROWSER_USER_AGENT)
        .build()
        .expect("client");

    let mut peak_workers = 0usize;
    let mut last_progress = None;
    let started = Instant::now();
    let outcome = moviebox_tui::download::download(
        &client,
        &url,
        &destination,
        Arc::new(AtomicBool::new(false)),
        |progress| {
            peak_workers = peak_workers.max(progress.workers);
            // Ignore the final report: it is emitted after segment assembly.
            if progress
                .total
                .is_none_or(|total| progress.downloaded < total)
            {
                last_progress = Some(Instant::now());
            }
        },
    )
    .await
    .expect("download succeeds");
    let elapsed = started.elapsed().as_secs_f64();

    let bytes = match outcome {
        moviebox_tui::download::DownloadOutcome::Completed { bytes } => bytes,
        other => panic!("unexpected outcome: {other:?}"),
    };
    let transfer = last_progress
        .map(|at| at.duration_since(started).as_secs_f64())
        .unwrap_or(elapsed);
    println!(
        "downloaded {:.1} MB in {:.2}s total = {:.2} MB/s using {} worker(s); \
         transfer {:.2}s ({:.2} MB/s), post-transfer assembly {:.2}s",
        bytes as f64 / 1_048_576.0,
        elapsed,
        bytes as f64 / 1_048_576.0 / elapsed,
        peak_workers,
        transfer,
        bytes as f64 / 1_048_576.0 / transfer,
        elapsed - transfer
    );
    assert_eq!(
        bytes,
        tokio::fs::metadata(&destination).await.unwrap().len()
    );
    let _ = tokio::fs::remove_file(&destination).await;
}
