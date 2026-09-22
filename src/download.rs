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
use tokio::io::AsyncWriteExt;

const MAX_ATTEMPTS: usize = 4;
const SEGMENT_THRESHOLD: u64 = 32 * 1024 * 1024;
const MAX_SEGMENTS: usize = 8;
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
    /// Short, actionable text for the failure notification. The `Display`
    /// output carries the request URL and library internals, which belong in
    /// the log rather than on screen.
    pub fn user_message(&self) -> String {
        match self {
            Self::Http(status) => match *status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    format!("Server refused the download ({}).", status.as_u16())
                }
                StatusCode::NOT_FOUND | StatusCode::GONE => {
                    "File is no longer available on the server.".to_string()
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    "Server is rate limiting downloads. Try again later.".to_string()
                }
                other if other.is_server_error() => {
                    format!("Server error ({}). Try again later.", other.as_u16())
                }
                other => format!("Server returned HTTP {}.", other.as_u16()),
            },
            Self::Network(error) => {
                if error.is_timeout() {
                    "Connection timed out.".to_string()
                } else if error.is_connect() {
                    "Cannot reach the server.".to_string()
                } else {
                    "Connection to the server was lost.".to_string()
                }
            }
            Self::File(error) => format!("Cannot write the file: {}.", error.kind()),
            Self::InvalidRange(_) => "Server sent an invalid partial response.".to_string(),
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
        if cancel.load(Ordering::Relaxed) {
            return Ok(DownloadOutcome::Paused {
                bytes: file_len(&partial).await,
            });
        }

        let mut offset = file_len(&partial).await;
        let mut request = client.get(url);
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
            if let Some(validator) = metadata.etag.as_ref().or(metadata.last_modified.as_ref()) {
                request = request.header(IF_RANGE, validator);
            }
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
            && offset == 0
            && response.status() == StatusCode::OK
            // Many CDNs honour Range without advertising Accept-Ranges, so try
            // segmenting unless the server explicitly refuses; a worker that
            // gets a plain 200 falls back to the single-stream path below.
            && response
                .headers()
                .get(ACCEPT_RANGES)
                .and_then(|value| value.to_str().ok())
                .is_none_or(|value| !value.eq_ignore_ascii_case("none"))
            && response
                .content_length()
                .is_some_and(|total| total >= SEGMENT_THRESHOLD)
        {
            let total = response.content_length().unwrap_or_default();
            let segments = segment_count(total);
            let current_metadata = ResumeMetadata {
                etag: header_string(&response, ETAG),
                last_modified: header_string(&response, LAST_MODIFIED),
                total: Some(total),
                segments: Some(segments),
            };
            if !metadata_matches(&metadata, &current_metadata) {
                remove_segment_files(destination).await;
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
                Err(DownloadError::InvalidRange(_)) => {
                    remove_segment_files(destination).await;
                    metadata = ResumeMetadata::default();
                    write_metadata(&metadata_path, &metadata).await?;
                    segmented_disabled = true;
                    continue;
                }
                result => return result,
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

        metadata.etag = header_string(&response, ETAG).or(metadata.etag);
        metadata.last_modified = header_string(&response, LAST_MODIFIED).or(metadata.last_modified);
        metadata.total = response_total.or(metadata.total);
        metadata.segments = None;
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
    metadata: ResumeMetadata,
    cancel: Arc<AtomicBool>,
    report: &mut F,
) -> Result<DownloadOutcome, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    let total = metadata
        .total
        .ok_or_else(|| DownloadError::InvalidRange("segment total missing".into()))?;
    let segments = metadata.segments.unwrap_or_else(|| segment_count(total));
    let ranges = segment_ranges(total, segments);
    let mut initial = 0;

    for (index, (start, end)) in ranges.iter().copied().enumerate() {
        let path = segment_path(destination, index);
        let expected = end - start + 1;
        let length = file_len(&path).await;
        if length > expected {
            truncate(&path).await?;
        } else {
            initial += length;
        }
    }

    let downloaded = Arc::new(AtomicU64::new(initial));
    let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::channel::<(u64, usize)>(64);
    let mut tasks = tokio::task::JoinSet::new();
    let validator = metadata.etag.clone().or(metadata.last_modified.clone());

    for (index, (start, end)) in ranges.iter().copied().enumerate() {
        let client = client.clone();
        let url = url.to_string();
        let path = segment_path(destination, index);
        let cancel = cancel.clone();
        let progress_sender = progress_sender.clone();
        let validator = validator.clone();
        tasks.spawn(async move {
            download_segment(
                &client,
                &url,
                &path,
                start,
                end,
                total,
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
                }
            }
            result = tasks.join_next() => {
                match result {
                    Some(Ok(Ok(()))) => finished += 1,
                    Some(Ok(Err(DownloadError::Paused))) => {
                        tasks.abort_all();
                        return Ok(DownloadOutcome::Paused {
                            bytes: downloaded.load(Ordering::Relaxed),
                        });
                    }
                    Some(Ok(Err(error))) => {
                        tasks.abort_all();
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
        return Ok(DownloadOutcome::Paused {
            bytes: downloaded.load(Ordering::Relaxed),
        });
    }

    let assembly = sidecar_path(destination, "assembling");
    let mut output = tokio::fs::File::create(&assembly).await?;
    let mut copy_buffer = vec![0u8; 256 * 1024];
    for index in 0..segments {
        let path = segment_path(destination, index);
        let mut part = tokio::fs::File::open(&path).await?;
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut part, &mut copy_buffer).await?;
            if n == 0 {
                break;
            }
            tokio::io::AsyncWriteExt::write_all(&mut output, &copy_buffer[..n]).await?;
        }
    }
    output.flush().await?;
    output.sync_data().await?;
    drop(output);
    if file_len(&assembly).await != total {
        return Err(DownloadError::Incomplete {
            downloaded: file_len(&assembly).await,
            expected: total,
        });
    }
    if destination.exists() {
        let _ = tokio::fs::remove_file(destination).await;
    }
    if let Err(e) = tokio::fs::rename(&assembly, destination).await {
        let _ = tokio::fs::remove_file(destination).await;
        tokio::fs::rename(&assembly, destination)
            .await
            .map_err(|e2| {
                DownloadError::File(std::io::Error::other(format!(
                    "failed to move assembled file to destination: {e} ({e2})"
                )))
            })?;
    }
    for index in 0..segments {
        let _ = tokio::fs::remove_file(segment_path(destination, index)).await;
    }
    let _ = tokio::fs::remove_file(metadata_path).await;
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
        let existing = file_len(path).await.min(expected);
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
                // Range header ignored outright: retrying cannot help.
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

        let raw_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        let mut file = tokio::io::BufWriter::with_capacity(256 * 1024, raw_file);
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
                return Err(DownloadError::Paused);
            }
            match tokio::time::timeout(Duration::from_secs(30), response.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    let remaining = expected - written;
                    let bytes = chunk.len().min(remaining as usize);
                    file.write_all(&chunk[..bytes]).await?;
                    written += bytes as u64;
                    unbatched_bytes += bytes as u64;
                    if unbatched_bytes >= 256 * 1024
                        || last_progress_send.elapsed() >= Duration::from_millis(100)
                    {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                        unbatched_bytes = 0;
                        last_progress_send = Instant::now();
                    }
                    if written == expected {
                        if unbatched_bytes > 0 {
                            let _ = progress.send((unbatched_bytes, attempt)).await;
                        }
                        file.flush().await?;
                        file.get_mut().sync_data().await?;
                        return Ok(());
                    }
                }
                Ok(Ok(None)) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
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
                    last_error = Some(DownloadError::Network(error));
                    break;
                }
                Err(_) => {
                    if unbatched_bytes > 0 {
                        let _ = progress.send((unbatched_bytes, attempt)).await;
                    }
                    file.flush().await?;
                    last_error = Some(DownloadError::InvalidRange("read timeout".into()));
                    break;
                }
            }
        }
        retry_delay(attempt).await;
    }

    Err(last_error.unwrap_or(DownloadError::Incomplete {
        downloaded: file_len(path).await,
        expected,
    }))
}

fn segment_count(total: u64) -> usize {
    // ponytail: fixed tiers, make it a setting if servers start throttling
    // per-connection counts.
    if total < 256 * 1024 * 1024 {
        4
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

fn segment_path(destination: &Path, index: usize) -> PathBuf {
    sidecar_path(destination, &format!("part.{index}"))
}

fn metadata_matches(previous: &ResumeMetadata, current: &ResumeMetadata) -> bool {
    if previous.total != current.total || previous.segments != current.segments {
        return false;
    }

    if let (Some(a), Some(b)) = (&previous.etag, &current.etag) {
        return a == b;
    }

    match (&previous.last_modified, &current.last_modified) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

async fn remove_segment_files(destination: &Path) {
    for index in 0..MAX_SEGMENTS {
        let _ = tokio::fs::remove_file(segment_path(destination, index)).await;
    }
    let _ = tokio::fs::remove_file(sidecar_path(destination, "assembling")).await;
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
    fn test_user_message_is_concise_and_hides_internals() {
        assert_eq!(
            DownloadError::Http(StatusCode::FORBIDDEN).user_message(),
            "Server refused the download (403)."
        );
        assert_eq!(
            DownloadError::Http(StatusCode::NOT_FOUND).user_message(),
            "File is no longer available on the server."
        );
        assert_eq!(
            DownloadError::Http(StatusCode::BAD_GATEWAY).user_message(),
            "Server error (502). Try again later."
        );
        assert_eq!(
            DownloadError::Incomplete {
                downloaded: 52 * 1024 * 1024,
                expected: 104 * 1024 * 1024,
            }
            .user_message(),
            "Download stopped at 52.0 of 104.0 MB."
        );

        // Internal detail stays in the log, not on screen.
        let invalid = DownloadError::InvalidRange(
            "worker requested 0-1023/4096, received bytes 0-99/4096".into(),
        );
        assert_eq!(
            invalid.user_message(),
            "Server sent an invalid partial response."
        );
        assert!(!invalid.user_message().contains("worker"));
    }

    #[tokio::test]
    async fn test_user_message_omits_url_for_network_errors() {
        // Port 1 is never listening, so this fails without leaving the host.
        let error = reqwest::Client::new()
            .get("http://127.0.0.1:1/secret-token-in-url")
            .send()
            .await
            .expect_err("connection refused");
        let message = DownloadError::Network(error).user_message();
        assert_eq!(message, "Cannot reach the server.");
        assert!(!message.contains("127.0.0.1"));
        assert!(!message.contains("secret-token-in-url"));
    }

    #[test]
    fn test_segment_count_tiers() {
        assert_eq!(segment_count(SEGMENT_THRESHOLD), 4);
        assert_eq!(segment_count(255 * 1024 * 1024), 4);
        assert_eq!(segment_count(256 * 1024 * 1024), MAX_SEGMENTS);
        assert_eq!(segment_count(8 * 1024 * 1024 * 1024), MAX_SEGMENTS);
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
}
