use reqwest::{
    Client, StatusCode,
    header::{ACCEPT_RANGES, CONTENT_RANGE, ETAG, IF_RANGE, LAST_MODIFIED, RANGE},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

const MAX_ATTEMPTS: usize = 4;
const SEGMENT_THRESHOLD: u64 = 8 * 1024 * 1024;
const MAX_SEGMENTS: usize = 16;
pub const DEFAULT_STREAM_NAME: &str = "MovieBox-Tui_Stream";

pub fn safe_file_stem(value: &str) -> String {
    let mut stem = value
        .chars()
        .take(120)
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    stem = stem.trim_matches(['.', ' ', '_']).to_string();
    if stem.is_empty() {
        return DEFAULT_STREAM_NAME.into();
    }
    let upper = stem.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|number| {
                number.len() == 1 && number.bytes().all(|byte| matches!(byte, b'1'..=b'9'))
            });
    if reserved {
        stem.push('_');
    }
    stem
}

pub fn resolve_media_target(
    base_dir: &Path,
    safe_title: &str,
    is_series: bool,
    season: usize,
    episode: usize,
) -> (PathBuf, String) {
    if is_series {
        (
            base_dir
                .join("Series")
                .join(safe_title)
                .join(format!("Season {season}")),
            format!("{safe_title} - S{season:02}E{episode:02}"),
        )
    } else {
        (
            base_dir.join("Movies").join(safe_title),
            safe_title.to_string(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub bytes_per_second: f64,
    pub attempt: usize,
    pub workers: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    Completed { bytes: u64 },
    Paused { bytes: u64 },
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("server returned HTTP {0}")]
    Http(StatusCode),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("file error: {0}")]
    File(#[from] std::io::Error),
    #[error("invalid partial response: {0}")]
    InvalidRange(String),
    #[error("download ended at {downloaded} of {expected} bytes")]
    Incomplete { downloaded: u64, expected: u64 },
    #[error("download paused")]
    Paused,
}

impl DownloadError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Http(status) => match *status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    format!("Server refused download (HTTP {}).", status.as_u16())
                }
                StatusCode::NOT_FOUND | StatusCode::GONE => {
                    "File is no longer available on server.".to_string()
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    "Server rate limit exceeded. Try again later.".to_string()
                }
                other if other.is_server_error() => {
                    format!("Server error (HTTP {}). Try again later.", other.as_u16())
                }
                other => format!("Server returned HTTP {}.", other.as_u16()),
            },
            Self::Network(error) => {
                if error.is_timeout() {
                    "Connection timed out.".to_string()
                } else if error.is_connect() {
                    "Cannot reach download server.".to_string()
                } else {
                    "Connection to server was lost.".to_string()
                }
            }
            Self::File(error) => format!("File write error: {}.", error.kind()),
            Self::InvalidRange(reason) if reason.contains("timeout") => {
                "Connection timed out.".to_string()
            }
            Self::InvalidRange(_) => "Server returned invalid partial response.".to_string(),
            Self::Incomplete {
                downloaded,
                expected,
            } => format!(
                "Download stopped at {:.1} of {:.1} MB.",
                *downloaded as f64 / 1_048_576.0,
                *expected as f64 / 1_048_576.0
            ),
            Self::Paused => "Download paused.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ResumeMetadata {
    etag: Option<String>,
    last_modified: Option<String>,
    total: Option<u64>,
    segments: Option<usize>,
    #[serde(default)]
    segment_progress: Vec<u64>,
}

pub async fn download<F>(
    client: &Client,
    url: &str,
    destination: &Path,
    cancel: Arc<AtomicBool>,
    mut report: F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let partial = sidecar_path(destination, "part");
    let metadata_path = sidecar_path(destination, "part.json");
    let mut metadata = read_metadata(&metadata_path).await;
    let started = Instant::now();
    let mut last_report = Instant::now() - Duration::from_secs(1);
    let mut last_error = None;
    let mut segmented_disabled = false;

    for attempt in 1..=MAX_ATTEMPTS {
        let is_segmented_state = metadata.segments.is_some();
        if cancel.load(Ordering::Relaxed) {
            let paused_bytes = if is_segmented_state {
                metadata.segment_progress.iter().sum()
            } else {
                file_len(&partial).await
            };
            return Ok(DownloadOutcome::Paused {
                bytes: paused_bytes,
            });
        }

        let mut offset = if is_segmented_state {
            0
        } else {
            file_len(&partial).await
        };
        let mut request = client.get(url);
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
            if let Some(validator) = metadata.etag.as_ref().or(metadata.last_modified.as_ref()) {
                request = request.header(IF_RANGE, validator);
            }
        } else if !segmented_disabled {
            request = request.header(RANGE, "bytes=0-0");
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(DownloadError::Network(error));
                retry_delay(attempt).await;
                continue;
            }
        };

        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && metadata.total == Some(offset)
        {
            finalize(&partial, &metadata_path, destination).await?;
            return Ok(DownloadOutcome::Completed { bytes: offset });
        }
        if !response.status().is_success() {
            last_error = Some(DownloadError::Http(response.status()));
            retry_delay(attempt).await;
            continue;
        }

        if !segmented_disabled
            && (offset == 0 || !metadata.segment_progress.is_empty())
            && (response.status() == StatusCode::OK
                || response.status() == StatusCode::PARTIAL_CONTENT)
            && response
                .headers()
                .get(ACCEPT_RANGES)
                .and_then(|value| value.to_str().ok())
                .is_none_or(|value| !value.eq_ignore_ascii_case("none"))
        {
            let total_opt = if response.status() == StatusCode::PARTIAL_CONTENT {
                response
                    .headers()
                    .get(CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|cr| parse_content_range(cr).ok())
                    .and_then(|(_, t)| t)
            } else {
                response.content_length()
            };
            if let Some(total) = total_opt.filter(|&t| t >= SEGMENT_THRESHOLD) {
                let segments = segment_count(total);
                let mut current_metadata = ResumeMetadata {
                    etag: header_string(&response, ETAG),
                    last_modified: header_string(&response, LAST_MODIFIED),
                    total: Some(total),
                    segments: Some(segments),
                    segment_progress: vec![0; segments],
                };
                if metadata_matches(&metadata, &current_metadata)
                    && metadata.segment_progress.len() == segments
                    && file_len(&partial).await == total
                {
                    current_metadata.segment_progress = metadata.segment_progress.clone();
                }
                write_metadata(&metadata_path, &current_metadata).await?;
                drop(response);
                match download_segmented(
                    client,
                    url,
                    destination,
                    &metadata_path,
                    current_metadata,
                    cancel.clone(),
                    &mut report,
                )
                .await
                {
                    Err(DownloadError::InvalidRange(_))
                    | Err(DownloadError::Http(_))
                    | Err(DownloadError::Network(_)) => {
                        let _ = tokio::fs::remove_file(&partial).await;
                        metadata = ResumeMetadata::default();
                        write_metadata(&metadata_path, &metadata).await?;
                        segmented_disabled = true;
                        continue;
                    }
                    result => return result,
                }
            }
            if offset == 0 && response.status() == StatusCode::PARTIAL_CONTENT {
                drop(response);
                segmented_disabled = true;
                continue;
            }
        }
        let response_total = if response.status() == StatusCode::PARTIAL_CONTENT {
            let content_range = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| DownloadError::InvalidRange("Content-Range missing".into()))?;
            let (start, total) = parse_content_range(content_range)?;
            if start != offset {
                return Err(DownloadError::InvalidRange(format!(
                    "requested byte {offset}, received {start}"
                )));
            }
            total
        } else {
            if offset > 0 {
                truncate(&partial).await?;
                offset = 0;
            }
            response.content_length()
        };

        if offset > 0
            && let (Some(previous), Some(current)) = (metadata.total, response_total)
            && previous != current
        {
            truncate(&partial).await?;
            metadata = ResumeMetadata::default();
            write_metadata(&metadata_path, &metadata).await?;
            last_error = Some(DownloadError::InvalidRange(
                "remote file size changed; partial reset".into(),
            ));
            retry_delay(attempt).await;
            continue;
        }

        if offset == 0 && file_len(&partial).await > 0 {
            truncate(&partial).await?;
        }
        metadata.etag = header_string(&response, ETAG).or(metadata.etag);
        metadata.last_modified = header_string(&response, LAST_MODIFIED).or(metadata.last_modified);
        metadata.total = response_total.or(metadata.total);
        metadata.segments = None;
        metadata.segment_progress.clear();
        write_metadata(&metadata_path, &metadata).await?;
        let raw_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&partial)
            .await?;
        let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, raw_file);
        let mut response = response;
        let mut downloaded = offset;
        let transfer_started = Instant::now();

        loop {
            if cancel.load(Ordering::Relaxed) {
                file.flush().await?;
                return Ok(DownloadOutcome::Paused { bytes: downloaded });
            }

            match tokio::time::timeout(Duration::from_secs(30), response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    file.write_all(&chunk).await?;
                    downloaded += chunk.len() as u64;
                    if last_report.elapsed() >= Duration::from_millis(200) {
                        let elapsed = started.elapsed().as_secs_f64();
                        report(DownloadProgress {
                            downloaded,
                            total: metadata.total,
                            bytes_per_second: if elapsed > 0.0 {
                                downloaded.saturating_sub(offset) as f64 / elapsed
                            } else {
                                0.0
                            },
                            attempt,
                            workers: 1,
                        });
                        last_report = Instant::now();
                    }
                }
                Ok(Ok(None)) => {
                    file.flush().await?;
                    file.get_mut().sync_data().await?;
                    if let Some(expected) = metadata.total
                        && downloaded != expected
                    {
                        last_error = Some(DownloadError::Incomplete {
                            downloaded,
                            expected,
                        });
                        break;
                    }
                    report(DownloadProgress {
                        downloaded,
                        total: metadata.total.or(Some(downloaded)),
                        bytes_per_second: if transfer_started.elapsed().as_secs_f64() > 0.0 {
                            downloaded.saturating_sub(offset) as f64
                                / transfer_started.elapsed().as_secs_f64()
                        } else {
                            0.0
                        },
                        attempt,
                        workers: 1,
                    });
                    finalize(&partial, &metadata_path, destination).await?;
                    return Ok(DownloadOutcome::Completed { bytes: downloaded });
                }
                Ok(Err(error)) => {
                    file.flush().await?;
                    last_error = Some(DownloadError::Network(error));
                    break;
                }
                Err(_) => {
                    file.flush().await?;
                    last_error = Some(DownloadError::InvalidRange("read timeout".into()));
                    break;
                }
            }
        }

        retry_delay(attempt).await;
    }

    Err(last_error.unwrap_or(DownloadError::InvalidRange(
        "download failed without a response".into(),
    )))
}

async fn download_segmented<F>(
    client: &Client,
    url: &str,
    destination: &Path,
    metadata_path: &Path,
    mut metadata: ResumeMetadata,
    cancel: Arc<AtomicBool>,
    report: &mut F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let partial_buf = sidecar_path(destination, "part");
    let partial_path = partial_buf.as_path();
    let total = metadata
        .total
        .ok_or_else(|| DownloadError::InvalidRange("segment total missing".into()))?;
    let segments = metadata.segments.unwrap_or_else(|| segment_count(total));
    let ranges = segment_ranges(total, segments);
    if metadata.segment_progress.len() != segments {
        metadata.segment_progress = vec![0; segments];
    }

    let raw_partial = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(partial_path)
        .await?;
    if raw_partial.metadata().await.map(|m| m.len()).unwrap_or(0) != total {
        raw_partial.set_len(total).await?;
        metadata.segment_progress.fill(0);
    }
    drop(raw_partial);

    let mut initial = 0u64;
    let mut segment_counters = Vec::with_capacity(segments);
    for (index, &(start, end)) in ranges.iter().enumerate() {
        let expected = end - start + 1;
        let written = metadata.segment_progress[index].min(expected);
        metadata.segment_progress[index] = written;
        initial += written;
        segment_counters.push(Arc::new(AtomicU64::new(written)));
    }
    write_metadata(metadata_path, &metadata).await?;

    let downloaded = Arc::new(AtomicU64::new(initial));
    let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::channel::<(u64, usize)>(128);
    let mut tasks = tokio::task::JoinSet::new();
    let validator = metadata.etag.clone().or(metadata.last_modified.clone());

    for (index, (start, end)) in ranges.iter().copied().enumerate() {
        let client = client.clone();
        let url = url.to_string();
        let path = partial_path.to_path_buf();
        let cancel = cancel.clone();
        let progress_sender = progress_sender.clone();
        let validator = validator.clone();
        let seg_counter = Arc::clone(&segment_counters[index]);
        tasks.spawn(async move {
            download_segment(
                &client,
                &url,
                &path,
                start,
                end,
                total,
                seg_counter,
                validator,
                cancel,
                progress_sender,
            )
            .await
        });
    }
    drop(progress_sender);

    let started = Instant::now();
    let mut last_report = Instant::now() - Duration::from_secs(1);
    let mut last_meta_flush = Instant::now();
    let mut finished = 0;
    while finished < segments {
        tokio::select! {
            progress = progress_receiver.recv() => {
                if let Some((bytes, attempt)) = progress {
                    let current = downloaded.fetch_add(bytes, Ordering::Relaxed) + bytes;
                    if last_report.elapsed() >= Duration::from_millis(200) {
                        let elapsed = started.elapsed().as_secs_f64();
                        report(DownloadProgress {
                            downloaded: current,
                            total: Some(total),
                            bytes_per_second: if elapsed > 0.0 {
                                current.saturating_sub(initial) as f64 / elapsed
                            } else {
                                0.0
                            },
                            attempt,
                            workers: segments,
                        });
                        last_report = Instant::now();
                    }
                    if last_meta_flush.elapsed() >= Duration::from_secs(2) {
                        for (idx, counter) in segment_counters.iter().enumerate() {
                            metadata.segment_progress[idx] = counter.load(Ordering::Relaxed);
                        }
                        let _ = write_metadata(metadata_path, &metadata).await;
                        last_meta_flush = Instant::now();
                    }
                }
            }
            result = tasks.join_next() => {
                match result {
                    Some(Ok(Ok(()))) => finished += 1,
                    Some(Ok(Err(DownloadError::Paused))) => {
                        tasks.abort_all();
                        for (idx, counter) in segment_counters.iter().enumerate() {
                            metadata.segment_progress[idx] = counter.load(Ordering::Relaxed);
                        }
                        let _ = write_metadata(metadata_path, &metadata).await;
                        return Ok(DownloadOutcome::Paused {
                            bytes: downloaded.load(Ordering::Relaxed),
                        });
                    }
                    Some(Ok(Err(error))) => {
                        tasks.abort_all();
                        for (idx, counter) in segment_counters.iter().enumerate() {
                            metadata.segment_progress[idx] = counter.load(Ordering::Relaxed);
                        }
                        let _ = write_metadata(metadata_path, &metadata).await;
                        return Err(error);
                    }
                    Some(Err(error)) => {
                        tasks.abort_all();
                        return Err(DownloadError::InvalidRange(format!(
                            "download worker stopped: {error}"
                        )));
                    }
                    None => break,
                }
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        for (idx, counter) in segment_counters.iter().enumerate() {
            metadata.segment_progress[idx] = counter.load(Ordering::Relaxed);
        }
        let _ = write_metadata(metadata_path, &metadata).await;
        return Ok(DownloadOutcome::Paused {
            bytes: downloaded.load(Ordering::Relaxed),
        });
    }

    let final_file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(partial_path)
        .await?;
    final_file.sync_data().await?;
    drop(final_file);
    if file_len(partial_path).await != total {
        return Err(DownloadError::Incomplete {
            downloaded: file_len(partial_path).await,
            expected: total,
        });
    }
    finalize(partial_path, metadata_path, destination).await?;
    report(DownloadProgress {
        downloaded: total,
        total: Some(total),
        bytes_per_second: if started.elapsed().as_secs_f64() > 0.0 {
            total.saturating_sub(initial) as f64 / started.elapsed().as_secs_f64()
        } else {
            0.0
        },
        attempt: 1,
        workers: segments,
    });
    Ok(DownloadOutcome::Completed { bytes: total })
}

#[allow(clippy::too_many_arguments)]
async fn download_segment(
    client: &Client,
    url: &str,
    path: &Path,
    start: u64,
    end: u64,
    total: u64,
    segment_written: Arc<AtomicU64>,
    validator: Option<String>,
    cancel: Arc<AtomicBool>,
    progress: tokio::sync::mpsc::Sender<(u64, usize)>,
) -> Result<(), DownloadError> {
    let expected = end - start + 1;
    let mut last_error = None;

    for attempt in 1..=MAX_ATTEMPTS {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Paused);
        }
        let existing = segment_written.load(Ordering::Relaxed).min(expected);
        if existing == expected {
            return Ok(());
        }
        let requested_start = start + existing;
        let mut request = client
            .get(url)
            .header(RANGE, format!("bytes={requested_start}-{end}"));
        if let Some(validator) = &validator {
            request = request.header(IF_RANGE, validator);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(DownloadError::Network(error));
                retry_delay(attempt).await;
                continue;
            }
        };
        if response.status() != StatusCode::PARTIAL_CONTENT {
            let error = DownloadError::InvalidRange(format!(
                "worker expected HTTP 206, received {}",
                response.status()
            ));
            if response.status() == StatusCode::OK {
                return Err(error);
            }
            last_error = Some(error);
            retry_delay(attempt).await;
            continue;
        }
        let content_range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| DownloadError::InvalidRange("Content-Range missing".into()))?;
        let (received_start, received_total) = parse_content_range(content_range)?;
        if received_start != requested_start || received_total != Some(total) {
            return Err(DownloadError::InvalidRange(format!(
                "worker requested {requested_start}-{end}/{total}, received {content_range}"
            )));
        }

        let mut raw_file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .await?;
        raw_file
            .seek(std::io::SeekFrom::Start(requested_start))
            .await?;
        let mut file = tokio::io::BufWriter::with_capacity(1024 * 1024, raw_file);
        let mut response = response;
        let mut written = existing;
        let mut unbatched_bytes = 0u64;
        let mut last_progress_send = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                if unbatched_bytes > 0 {
                    let _ = progress.send((unbatched_bytes, attempt)).await;
                }
                file.flush().await?;
                segment_written.store(written, Ordering::Relaxed);
                return Err(DownloadError::Paused);
            }
            match tokio::time::timeout(Duration::from_secs(30), response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let remaining = expected - written;
                    let bytes = chunk.len().min(remaining as usize);
                    file.write_all(&chunk[..bytes]).await?;
                    written += bytes as u64;
                    // Workers are aborted without a flush when a sibling pauses or
                    // fails, so resume progress must exclude bytes still buffered.
                    segment_written.store(written - file.buffer().len() as u64, Ordering::Relaxed);
                    unbatched_bytes += bytes as u64;
                    if unbatched_bytes >= 512 * 1024
                        || last_progress_send.elapsed() >= Duration::from_millis(120)
                    {
                        let _ = progress.try_send((unbatched_bytes, attempt));
                        unbatched_bytes = 0;
                        last_progress_send = Instant::now();
                    }
                    if written == expected {
                        if unbatched_bytes > 0 {
                            let _ = progress.send((unbatched_bytes, attempt)).await;
                        }
                        file.flush().await?;
                        return Ok(());
                    }
                }
                Ok(Ok(None)) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    segment_written.store(written, Ordering::Relaxed);
                    last_error = Some(DownloadError::Incomplete {
                        downloaded: written,
                        expected,
                    });
                    break;
                }
                Ok(Err(error)) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    segment_written.store(written, Ordering::Relaxed);
                    last_error = Some(DownloadError::Network(error));
                    break;
                }
                Err(_) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    segment_written.store(written, Ordering::Relaxed);
                    last_error = Some(DownloadError::InvalidRange("read timeout".into()));
                    break;
                }
            }
        }
        retry_delay(attempt).await;
    }

    Err(last_error.unwrap_or(DownloadError::Incomplete {
        downloaded: segment_written.load(Ordering::Relaxed),
        expected,
    }))
}

fn segment_count(total: u64) -> usize {
    if total < 32 * 1024 * 1024 {
        4
    } else if total < 128 * 1024 * 1024 {
        8
    } else if total < 512 * 1024 * 1024 {
        12
    } else {
        MAX_SEGMENTS
    }
}

fn segment_ranges(total: u64, segments: usize) -> Vec<(u64, u64)> {
    let size = total / segments as u64;
    (0..segments)
        .map(|index| {
            let start = index as u64 * size;
            let end = if index + 1 == segments {
                total - 1
            } else {
                start + size - 1
            };
            (start, end)
        })
        .collect()
}

fn metadata_matches(previous: &ResumeMetadata, current: &ResumeMetadata) -> bool {
    if previous.total.is_none()
        || previous.total != current.total
        || previous.segments != current.segments
    {
        return false;
    }

    if let (Some(a), Some(b)) = (&previous.etag, &current.etag) {
        return a == b;
    }

    match (&previous.last_modified, &current.last_modified) {
        (Some(a), Some(b)) => a == b,
        (None, None) => previous.etag.is_none() && current.etag.is_none(),
        _ => false,
    }
}

fn parse_content_range(value: &str) -> Result<(u64, Option<u64>), DownloadError> {
    let value = value
        .strip_prefix("bytes ")
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let (range, total) = value
        .split_once('/')
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let (start, _) = range
        .split_once('-')
        .ok_or_else(|| DownloadError::InvalidRange(value.into()))?;
    let start = start
        .parse()
        .map_err(|_| DownloadError::InvalidRange(value.into()))?;
    let total = if total == "*" {
        None
    } else {
        Some(
            total
                .parse()
                .map_err(|_| DownloadError::InvalidRange(value.into()))?,
        )
    };
    Ok((start, total))
}

fn sidecar_path(destination: &Path, suffix: &str) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

async fn file_len(path: &Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map(|metadata| metadata.len())
        .unwrap_or_default()
}

async fn truncate(path: &Path) -> Result<(), std::io::Error> {
    tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .await
        .map(|_| ())
}

async fn finalize(
    partial: &Path,
    metadata: &Path,
    destination: &Path,
) -> Result<(), std::io::Error> {
    if tokio::fs::rename(partial, destination).await.is_err() {
        let _ = tokio::fs::remove_file(destination).await;
        tokio::fs::rename(partial, destination).await?;
    }
    let _ = tokio::fs::remove_file(metadata).await;
    Ok(())
}

async fn read_metadata(path: &Path) -> ResumeMetadata {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return ResumeMetadata::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

async fn write_metadata(path: &Path, metadata: &ResumeMetadata) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec(metadata).map_err(std::io::Error::other)?;
    tokio::fs::write(path, bytes).await
}

fn header_string(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn retry_delay(attempt: usize) {
    if attempt < MAX_ATTEMPTS {
        tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_range_valid() {
        let (start, total) = parse_content_range("bytes 0-1023/2048").unwrap();
        assert_eq!(start, 0);
        assert_eq!(total, Some(2048));

        let (start, total) = parse_content_range("bytes 500-999/*").unwrap();
        assert_eq!(start, 500);
        assert_eq!(total, None);
    }

    #[test]
    fn test_parse_content_range_invalid() {
        assert!(parse_content_range("invalid range").is_err());
        assert!(parse_content_range("bytes invalid").is_err());
    }

    #[test]
    fn test_segment_ranges_partitioning() {
        let total = 1000;
        let ranges = segment_ranges(total, 4);
        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges[0], (0, 249));
        assert_eq!(ranges[1], (250, 499));
        assert_eq!(ranges[2], (500, 749));
        assert_eq!(ranges[3], (750, 999));
    }

    #[test]
    fn test_segment_count_scaling() {
        assert_eq!(segment_count(16 * 1024 * 1024), 4);
        assert_eq!(segment_count(64 * 1024 * 1024), 8);
        assert_eq!(segment_count(256 * 1024 * 1024), 12);
        assert_eq!(segment_count(1024 * 1024 * 1024), 16);
    }

    #[tokio::test]
    async fn test_segmented_in_place_download_and_resume() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let total_size: usize = 16 * 1024 * 1024;
        let payload: Arc<Vec<u8>> = Arc::new((0..total_size).map(|i| (i % 251) as u8).collect());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/segmented_movie.mp4");

        let server_payload = Arc::clone(&payload);
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let data = Arc::clone(&server_payload);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let mut range_header = None;
                    for line in req.lines() {
                        if let Some(val) = line
                            .strip_prefix("Range: bytes=")
                            .or_else(|| line.strip_prefix("range: bytes="))
                        {
                            range_header = Some(val.trim().to_string());
                        }
                    }
                    let total = data.len();
                    if let Some(r) = range_header {
                        let (s_str, e_str) = r.split_once('-').unwrap();
                        let start: usize = s_str.parse().unwrap();
                        if start >= total {
                            let resp = format!(
                                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nConnection: close\r\n\r\n"
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                            return;
                        }
                        let end: usize = if e_str.is_empty() {
                            total - 1
                        } else {
                            e_str.parse().unwrap()
                        };
                        let slice = &data[start..=end];
                        let resp = format!(
                            "HTTP/1.1 206 Partial Content\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            slice.len()
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                        let _ = socket.write_all(slice).await;
                    } else {
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                    }
                });
            }
        });

        let temp_dir = std::env::temp_dir().join(format!("mbx_seg_inplace_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let dest_file = temp_dir.join("movie.mp4");
        let client = crate::net::http_client_builder_base().build().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut max_workers = 0usize;

        let part_file = sidecar_path(&dest_file, "part");
        let meta_file = sidecar_path(&dest_file, "part.json");
        let seg_len = (total_size / 4) as u64;
        let mut pre_buf = vec![0u8; total_size];
        pre_buf[..seg_len as usize].copy_from_slice(&payload[..seg_len as usize]);
        tokio::fs::write(&part_file, &pre_buf).await.unwrap();
        write_metadata(
            &meta_file,
            &ResumeMetadata {
                etag: None,
                last_modified: None,
                total: Some(total_size as u64),
                segments: Some(4),
                segment_progress: vec![seg_len, 0, 0, 0],
            },
        )
        .await
        .unwrap();
        let res = download(&client, &url, &dest_file, cancel, |prog| {
            max_workers = max_workers.max(prog.workers);
        })
        .await;

        server.abort();
        assert!(
            matches!(res, Ok(DownloadOutcome::Completed { bytes }) if bytes == total_size as u64)
        );
        assert_eq!(max_workers, 4);
        let saved = tokio::fs::read(&dest_file).await.unwrap();
        assert_eq!(saved.len(), total_size);
        assert_eq!(saved, *payload);
        assert!(!sidecar_path(&dest_file, "part").exists());
        assert!(!sidecar_path(&dest_file, "part.json").exists());

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_segmented_pause_records_only_bytes_on_disk() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let total_size: usize = 16 * 1024 * 1024;
        let payload: Arc<Vec<u8>> = Arc::new((0..total_size).map(|i| (i % 251) as u8).collect());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/slow_movie.mp4");

        // Serve ranges slowly so every worker still holds unflushed bytes when paused.
        let server_payload = Arc::clone(&payload);
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let data = Arc::clone(&server_payload);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let range = req.lines().find_map(|line| {
                        line.strip_prefix("Range: bytes=")
                            .or_else(|| line.strip_prefix("range: bytes="))
                            .map(|v| v.trim().to_string())
                    });
                    let total = data.len();
                    let Some(range) = range else {
                        return;
                    };
                    let (s_str, e_str) = range.split_once('-').unwrap();
                    let start: usize = s_str.parse().unwrap();
                    let end: usize = if e_str.is_empty() {
                        total - 1
                    } else {
                        e_str.parse().unwrap()
                    };
                    let resp = format!(
                        "HTTP/1.1 206 Partial Content\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        end - start + 1
                    );
                    if socket.write_all(resp.as_bytes()).await.is_err() {
                        return;
                    }
                    for chunk in data[start..=end].chunks(16 * 1024) {
                        if socket.write_all(chunk).await.is_err() {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                });
            }
        });

        let temp_dir = std::env::temp_dir().join(format!("mbx_seg_pause_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let dest_file = temp_dir.join("movie.mp4");
        let part_file = sidecar_path(&dest_file, "part");
        let meta_file = sidecar_path(&dest_file, "part.json");
        let client = crate::net::http_client_builder_base().build().unwrap();

        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::clone(&cancel);
        let res = download(&client, &url, &dest_file, cancel, move |prog| {
            if prog.downloaded >= 1024 * 1024 {
                pause.store(true, Ordering::Relaxed);
            }
        })
        .await;
        assert!(matches!(res, Ok(DownloadOutcome::Paused { .. })));

        let metadata = read_metadata(&meta_file).await;
        let partial = tokio::fs::read(&part_file).await.unwrap();
        let ranges = segment_ranges(total_size as u64, metadata.segments.unwrap());
        for (&(start, _), &written) in ranges.iter().zip(&metadata.segment_progress) {
            let (start, written) = (start as usize, written as usize);
            assert_eq!(
                partial[start..start + written],
                payload[start..start + written],
                "segment at {start} claims {written} bytes that are not on disk"
            );
        }

        let res = download(
            &client,
            &url,
            &dest_file,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await;
        server.abort();
        assert!(
            matches!(res, Ok(DownloadOutcome::Completed { bytes }) if bytes == total_size as u64)
        );
        let saved = tokio::fs::read(&dest_file).await.unwrap();
        assert!(saved == *payload, "resumed download must match the source");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[test]
    fn test_safe_file_stem_empty_fallback() {
        assert_eq!(safe_file_stem(""), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("   "), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("..."), DEFAULT_STREAM_NAME);
        assert_eq!(safe_file_stem("___"), DEFAULT_STREAM_NAME);
    }

    #[test]
    fn test_safe_file_stem_sanitization_and_reserved() {
        assert_eq!(safe_file_stem("../../../etc/passwd"), "etc_passwd");
        assert_eq!(
            safe_file_stem("C:\\Windows\\System32\\calc.exe"),
            "C__Windows_System32_calc.exe"
        );
        assert_eq!(safe_file_stem("CON"), "CON_");
        assert_eq!(safe_file_stem("con"), "con_");
        assert_eq!(safe_file_stem("PRN"), "PRN_");
        assert_eq!(safe_file_stem("AUX"), "AUX_");
        assert_eq!(safe_file_stem("NUL"), "NUL_");
        assert_eq!(safe_file_stem("COM1"), "COM1_");
        assert_eq!(safe_file_stem("LPT9"), "LPT9_");
        assert_eq!(
            safe_file_stem("Normal Title: Special"),
            "Normal Title_ Special"
        );
        assert_eq!(
            safe_file_stem("Movie: The <Ultimate> Edition *?|"),
            "Movie_ The _Ultimate_ Edition"
        );
    }

    #[tokio::test]
    async fn test_finalize_replaces_existing_destination_cleanly() {
        let dir = std::env::temp_dir().join(format!(
            "mbx_test_finalize_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let partial = dir.join("video.mp4.part");
        let metadata = dir.join("video.mp4.metadata");
        let destination = dir.join("video.mp4");

        tokio::fs::write(&destination, b"old version")
            .await
            .unwrap();
        tokio::fs::write(&partial, b"new version").await.unwrap();
        tokio::fs::write(&metadata, b"{}").await.unwrap();

        let res = finalize(&partial, &metadata, &destination).await;
        assert!(res.is_ok());
        assert!(!partial.exists());
        assert!(!metadata.exists());
        assert!(destination.exists());
        assert_eq!(
            tokio::fs::read_to_string(&destination).await.unwrap(),
            "new version"
        );
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
    #[test]
    fn test_download_error_user_message() {
        let err_403 = DownloadError::Http(StatusCode::FORBIDDEN);
        assert_eq!(
            err_403.user_message(),
            "Server refused download (HTTP 403)."
        );

        let err_404 = DownloadError::Http(StatusCode::NOT_FOUND);
        assert_eq!(
            err_404.user_message(),
            "File is no longer available on server."
        );

        let err_429 = DownloadError::Http(StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            err_429.user_message(),
            "Server rate limit exceeded. Try again later."
        );

        let err_503 = DownloadError::Http(StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            err_503.user_message(),
            "Server error (HTTP 503). Try again later."
        );

        let err_incomplete = DownloadError::Incomplete {
            downloaded: 10 * 1024 * 1024,
            expected: 20 * 1024 * 1024,
        };
        assert_eq!(
            err_incomplete.user_message(),
            "Download stopped at 10.0 of 20.0 MB."
        );

        let err_paused = DownloadError::Paused;
        assert_eq!(err_paused.user_message(), "Download paused.");
    }

    #[tokio::test]
    async fn test_download_forwards_custom_headers_and_content_to_local_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/video.mp4");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            let lower = req.to_lowercase();
            assert!(lower.contains("user-agent: moviebox-custom-ua"));
            assert!(lower.contains("referer: https://custom.referer.test/"));
            assert!(lower.contains("x-auth-token: secret-token-123"));
            let body = b"test video binary chunk stream";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: video/mp4\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
        });

        let temp_dir = std::env::temp_dir().join(format!("mbx_dl_test_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let dest_file = temp_dir.join("test_video.mp4");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "MovieBox-Custom-UA".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::REFERER,
            "https://custom.referer.test/".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-auth-token"),
            "secret-token-123".parse().unwrap(),
        );

        let client = crate::net::http_client_builder_base()
            .default_headers(headers)
            .build()
            .unwrap();

        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut progress_count = 0usize;

        let res = download(&client, &url, &dest_file, cancel, |_prog| {
            progress_count += 1;
        })
        .await;

        server.await.unwrap();
        assert!(matches!(res, Ok(DownloadOutcome::Completed { .. })));
        assert!(dest_file.exists());
        let saved = tokio::fs::read(&dest_file).await.unwrap();
        assert_eq!(saved, b"test video binary chunk stream");
        assert!(progress_count > 0);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_download_segmented_drops_fallback_to_sequential_download() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/stream.mp4");

        let server = tokio::spawn(async move {
            let total_size = 5 * 1024 * 1024;
            let mut request_num = 0;
            while request_num < 10 {
                let (mut socket, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };
                request_num += 1;
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);

                if req.contains("Range: bytes=0-0") {
                    let resp = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-0/{total_size}\r\nContent-Length: 1\r\nAccept-Ranges: bytes\r\n\r\n0"
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                } else if req.contains("Range: bytes=") && request_num <= 4 {
                    let resp = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                } else {
                    let chunk = vec![b'X'; total_size];
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {total_size}\r\nAccept-Ranges: none\r\nConnection: close\r\n\r\n"
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.write_all(&chunk).await;
                    break;
                }
            }
        });

        let temp_dir =
            std::env::temp_dir().join(format!("mbx_fallback_test_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let dest_file = temp_dir.join("fallback_stream.mp4");

        let client = reqwest::Client::new();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let res = download(&client, &url, &dest_file, cancel, |_| {}).await;
        let _ = server.await;

        assert!(matches!(res, Ok(DownloadOutcome::Completed { .. })));
        assert!(dest_file.exists());
        let meta = tokio::fs::metadata(&dest_file).await.unwrap();
        assert_eq!(meta.len(), 5 * 1024 * 1024);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
