use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const MAX_LINE_BYTES: usize = 8 * 1024;
const MAX_HEADERS: usize = 64;
const MAX_MANIFEST_BYTES: usize = 10 * 1024 * 1024;
const CHUNK_IDLE_TIMEOUT_SECS: u64 = 60;
const WATCHDOG_IDLE_SECS: u64 = 600;
const DASH_RANGE_CHUNK_BYTES: usize = 95 * 1024;
const MAX_CACHED_SEGMENTS: usize = 24;
const MAX_SEGMENT_BYTES: usize = 16 * 1024 * 1024;
const PREFETCH_LOOKAHEAD: u32 = 3;

type CachedSegment = (String, String, Arc<[u8]>);
type CachedManifest = (String, Arc<[u8]>);

#[derive(Default)]
struct SegmentCache {
    completed: Mutex<std::collections::VecDeque<CachedSegment>>,
    in_flight: Mutex<std::collections::HashMap<String, Arc<tokio::sync::Notify>>>,
}

impl SegmentCache {
    fn get(&self, url: &str) -> Option<(String, Arc<[u8]>)> {
        let lock = self.completed.lock().ok()?;
        lock.iter()
            .find(|(u, _, _)| u == url)
            .map(|(_, ct, b)| (ct.clone(), b.clone()))
    }

    fn contains_or_in_flight(&self, url: &str) -> bool {
        if let Ok(lock) = self.completed.lock() {
            if lock.iter().any(|(u, _, _)| u == url) {
                return true;
            }
        }
        if let Ok(lock) = self.in_flight.lock() {
            if lock.contains_key(url) {
                return true;
            }
        }
        false
    }

    fn insert(&self, url: String, content_type: String, data: Arc<[u8]>) {
        if let Ok(mut lock) = self.completed.lock() {
            if lock.iter().any(|(u, _, _)| u == &url) {
                return;
            }
            if lock.len() >= MAX_CACHED_SEGMENTS {
                lock.pop_front();
            }
            lock.push_back((url, content_type, data));
        }
    }
}

struct InFlightGuard {
    cache: Arc<SegmentCache>,
    url: String,
    notify: Arc<tokio::sync::Notify>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut lock) = self.cache.in_flight.lock() {
            lock.remove(&self.url);
        }
        self.notify.notify_waiters();
    }
}

#[derive(Clone)]
struct ProxyContext {
    proxy_port: u16,
    client: reqwest::Client,
    auth_headers: Arc<Vec<(String, String)>>,
    target_host: Option<String>,
    subtitle_url: Option<String>,
    max_height: Option<u64>,
    segment_cache: Arc<SegmentCache>,
    manifest_cache: Arc<Mutex<Option<CachedManifest>>>,
}
struct ConnectionGuard {
    conns: Arc<AtomicUsize>,
    activity: Arc<Mutex<Instant>>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.conns.fetch_sub(1, Ordering::Relaxed);
        if let Ok(mut lock) = self.activity.lock() {
            *lock = Instant::now();
        }
    }
}

pub fn spawn_sidecar(
    target_url: &str,
    headers: &[(String, String)],
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
) -> Result<(String, std::process::Child), String> {
    let exe = std::env::current_exe()
        .ok()
        .or_else(|| std::env::args().next().map(PathBuf::from))
        .ok_or_else(|| "unable to locate current executable".to_string())?;

    let headers_json = serde_json::to_string(headers).unwrap_or_else(|_| "[]".to_string());
    let sub_arg = subtitle_url.unwrap_or("");
    let height_arg = max_height.map(|h| h.to_string()).unwrap_or_default();

    let mut cmd = Command::new(exe);
    cmd.args([
        "--proxy-for-vlc",
        target_url,
        &headers_json,
        sub_arg,
        &height_arg,
    ]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    crate::player::configure_detached_process(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn proxy sidecar: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture sidecar stdout".to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });

    let line = rx.recv_timeout(Duration::from_secs(8)).map_err(|_| {
        let _ = child.kill();
        let _ = child.wait();
        "proxy sidecar timed out waiting for PORT line".to_string()
    })?;

    let port: u16 = line
        .trim()
        .strip_prefix("PORT ")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            format!("proxy sidecar returned unexpected output: {line:?}")
        })?;

    let proxy_path = if let Some(rest) = target_url.strip_prefix("https://") {
        format!("/https/{rest}")
    } else if let Some(rest) = target_url.strip_prefix("http://") {
        format!("/http/{rest}")
    } else {
        format!("/https/{target_url}")
    };

    Ok((format!("http://127.0.0.1:{port}{proxy_path}"), child))
}

pub async fn run_sidecar(
    target_url: String,
    headers: Vec<(String, String)>,
    subtitle_url: Option<String>,
    max_height: Option<u64>,
) {
    let client = crate::net::http_client_builder_base()
        .http1_only()
        .pool_max_idle_per_host(16)
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(_) => return,
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => return,
    };

    println!("PORT {port}");
    use std::io::Write;
    let _ = std::io::stdout().flush();

    let active_connections = Arc::new(AtomicUsize::new(0));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    let watchdog_conns = Arc::clone(&active_connections);
    let watchdog_activity = Arc::clone(&last_activity);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let conns = watchdog_conns.load(Ordering::Relaxed);
            let elapsed = match watchdog_activity.lock() {
                Ok(lock) => lock.elapsed(),
                Err(poisoned) => poisoned.into_inner().elapsed(),
            };
            if conns == 0 && elapsed > Duration::from_secs(WATCHDOG_IDLE_SECS) {
                std::process::exit(0);
            }
        }
    });

    let ctx = ProxyContext {
        proxy_port: port,
        client,
        auth_headers: Arc::new(headers),
        target_host: extract_host_authority(&target_url),
        subtitle_url,
        max_height,
        segment_cache: Arc::new(SegmentCache::default()),
        manifest_cache: Arc::new(Mutex::new(None)),
    };

    if crate::player::is_dash_url(&target_url) {
        let warm_ctx = ctx.clone();
        let warm_url = target_url.clone();
        tokio::spawn(async move {
            warmup_dash_sidecar(&warm_ctx, &warm_url).await;
        });
    }
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => {
                let _ = conn.0.set_nodelay(true);
                conn
            }
            Err(err) => {
                log::warn!("transient proxy accept error: {err}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let conn_ctx = ctx.clone();
        let active_conns = Arc::clone(&active_connections);
        let activity = Arc::clone(&last_activity);

        active_conns.fetch_add(1, Ordering::Relaxed);
        {
            if let Ok(mut lock) = activity.lock() {
                *lock = Instant::now();
            }
        }

        tokio::spawn(async move {
            let _guard = ConnectionGuard {
                conns: active_conns,
                activity,
            };
            let _ = handle_connection(stream, &conn_ctx).await;
        });
    }
}

pub(crate) fn extract_host_authority(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = after_scheme.split('/').next()?;
    if authority.is_empty() {
        None
    } else {
        Some(authority.to_string())
    }
}

pub(crate) fn should_forward_header(forward_all: bool, name: &str) -> bool {
    forward_all || name.eq_ignore_ascii_case("user-agent") || name.eq_ignore_ascii_case("referer")
}

async fn handle_connection(
    stream: TcpStream,
    ctx: &ProxyContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proxy_port = ctx.proxy_port;
    let client = &ctx.client;
    let auth_headers = ctx.auth_headers.as_slice();
    let target_host = ctx.target_host.as_deref();
    let subtitle_url = ctx.subtitle_url.as_deref();
    let max_height = ctx.max_height;
    let (reader, writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut writer = tokio::io::BufWriter::with_capacity(128 * 1024, writer);

    loop {
        let mut request_line = String::new();
        let n = buf_reader.read_line(&mut request_line).await?;
        if n == 0 {
            return Ok(());
        }
        if request_line.len() > MAX_LINE_BYTES {
            writer
                .write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await?;
            writer.flush().await?;
            return Ok(());
        }

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let path_and_query = parts.next().unwrap_or("/");

        let mut range_header = None;
        let mut client_close = false;
        let mut header_count = 0usize;
        loop {
            let mut header_line = String::new();
            if buf_reader.read_line(&mut header_line).await? == 0 {
                break;
            }
            let trimmed = header_line.trim();
            if trimmed.is_empty() {
                break;
            }
            if header_line.len() > MAX_LINE_BYTES {
                break;
            }
            header_count += 1;
            if header_count > MAX_HEADERS {
                break;
            }
            if let Some((name, val)) = trimmed.split_once(':') {
                if name.trim().eq_ignore_ascii_case("range") {
                    range_header = Some(val.trim().to_string());
                } else if name.trim().eq_ignore_ascii_case("connection") {
                    if val.trim().eq_ignore_ascii_case("close") {
                        client_close = true;
                    }
                }
            }
        }

        let target_url = match extract_target_url(path_and_query) {
            Some(url) => url,
            None => {
                let response =
                    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                writer.write_all(response.as_bytes()).await?;
                writer.flush().await?;
                return Ok(());
            }
        };
        let extracted_host = extract_host_authority(&target_url);
        let sub_host = subtitle_url.and_then(extract_host_authority);
        let is_allowed = match (target_host, extracted_host.as_deref()) {
            (Some(allowed), Some(extracted)) => {
                extracted == allowed || (sub_host.is_some() && extracted_host == sub_host)
            }
            _ => false,
        };
        if !is_allowed {
            let response =
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;
            return Ok(());
        }

        let forward_all_headers = extracted_host.as_deref() == target_host;

        if method == "GET" && crate::player::is_dash_url(&target_url) {
            let cached_mpd = ctx.manifest_cache.lock().ok().and_then(|g| {
                g.as_ref()
                    .filter(|(u, _)| u == &target_url)
                    .map(|(_, b)| Arc::clone(b))
            });
            if let Some(rewritten_bytes) = cached_mpd {
                let conn_header = if client_close {
                    "Connection: close\r\n"
                } else {
                    "Connection: keep-alive\r\n"
                };
                let headers_out = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/dash+xml\r\nContent-Length: {}\r\n{conn_header}\r\n",
                    rewritten_bytes.len()
                );
                writer.write_all(headers_out.as_bytes()).await?;
                writer.write_all(&rewritten_bytes).await?;
                writer.flush().await?;
                if client_close {
                    return Ok(());
                }
                continue;
            }
        }

        if method == "GET" && is_m4s_segment(&target_url) {
            for next_url in next_dash_segment_urls(&target_url, PREFETCH_LOOKAHEAD) {
                if !ctx.segment_cache.contains_or_in_flight(&next_url) {
                    let cache_clone = Arc::clone(&ctx.segment_cache);
                    let client_clone = client.clone();
                    let headers_clone = Arc::clone(&ctx.auth_headers);
                    tokio::spawn(async move {
                        let _ = fetch_or_get_segment(
                            &cache_clone,
                            &client_clone,
                            &next_url,
                            &headers_clone,
                            forward_all_headers,
                        )
                        .await;
                    });
                }
            }

            match fetch_or_get_segment(
                &ctx.segment_cache,
                client,
                &target_url,
                auth_headers,
                forward_all_headers,
            )
            .await
            {
                Ok((seg_status, content_type, seg_bytes)) => {
                    let conn_hdr = if client_close { "close" } else { "keep-alive" };
                    let total = seg_bytes.len();
                    let parsed_range = range_header
                        .as_deref()
                        .and_then(|r| parse_byte_range_request(r, total));
                    let (out_status, content_range_hdr, body_slice) = if seg_status.is_success()
                        && let Some((start, end)) = parsed_range
                    {
                        (
                            reqwest::StatusCode::PARTIAL_CONTENT,
                            format!("Content-Range: bytes {start}-{end}/{total}\r\n"),
                            &seg_bytes[start..=end],
                        )
                    } else {
                        (seg_status, String::new(), seg_bytes.as_ref())
                    };
                    let response_hdr = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{content_range_hdr}Accept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nConnection: {conn_hdr}\r\n\r\n",
                        out_status.as_u16(),
                        out_status.canonical_reason().unwrap_or("OK"),
                        body_slice.len()
                    );
                    writer.write_all(response_hdr.as_bytes()).await?;
                    writer.write_all(body_slice).await?;
                    writer.flush().await?;
                    if client_close {
                        return Ok(());
                    }
                    continue;
                }
                Err(e) => {
                    let body = format!("Gateway Error: {e}");
                    let response = format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    writer.write_all(response.as_bytes()).await?;
                    writer.flush().await?;
                    return Ok(());
                }
            }
        }

        let mut req = match method {
            "HEAD" => client.head(&target_url),
            _ => client.get(&target_url),
        };

        for (name, val) in auth_headers {
            if should_forward_header(forward_all_headers, name) {
                req = req.header(name.as_str(), val.as_str());
            }
        }
        if let Some(range) = range_header {
            req = req.header("Range", range);
        }
        let upstream_res = match req.send().await {
            Ok(res) => res,
            Err(e) => {
                let body = format!("Gateway Error: {e}");
                let response = format!(
                    "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                writer.write_all(response.as_bytes()).await?;
                writer.flush().await?;
                return Ok(());
            }
        };

        let status = upstream_res.status();

        let content_length = upstream_res
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        let is_dash_manifest = crate::player::is_dash_url(&target_url)
            || upstream_res
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|ct| ct.contains("dash+xml") || ct.contains("xml"))
                .unwrap_or(false);

        let within_manifest_limit = content_length.is_none_or(|len| len <= MAX_MANIFEST_BYTES);

        if is_dash_manifest && status.is_success() && within_manifest_limit {
            let manifest_bytes = upstream_res.bytes().await?;
            if manifest_bytes.len() > MAX_MANIFEST_BYTES {
                let body = "Manifest too large";
                writer
                .write_all(
                    format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await?;
                writer.flush().await?;
                return Ok(());
            }
            let manifest_str = String::from_utf8_lossy(&manifest_bytes);
            let rewritten = rewrite_dash_manifest(
                &manifest_str,
                proxy_port,
                target_host,
                subtitle_url,
                max_height,
            );
            let rewritten_bytes = rewritten.as_bytes();
            if let Ok(mut lock) = ctx.manifest_cache.lock() {
                *lock = Some((target_url.clone(), Arc::from(rewritten_bytes)));
            }

            let conn_header = if client_close {
                "Connection: close\r\n"
            } else {
                "Connection: keep-alive\r\n"
            };
            let headers_out = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/dash+xml\r\nContent-Length: {}\r\n{conn_header}\r\n",
                rewritten_bytes.len()
            );
            writer.write_all(headers_out.as_bytes()).await?;
            writer.write_all(rewritten_bytes).await?;
            writer.flush().await?;
            if client_close {
                return Ok(());
            }
            continue;
        }
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            status.as_u16(),
            status.canonical_reason().unwrap_or("OK")
        );
        writer.write_all(status_line.as_bytes()).await?;
        let keep_alive = !client_close && content_length.is_some();
        let headers_bytes =
            format_proxy_response_headers(upstream_res.headers(), &target_url, keep_alive);
        writer.write_all(&headers_bytes).await?;
        writer.flush().await?;
        let mut stream = upstream_res.bytes_stream();
        loop {
            let chunk_result =
                tokio::time::timeout(Duration::from_secs(CHUNK_IDLE_TIMEOUT_SECS), stream.next())
                    .await;
            match chunk_result {
                Ok(Some(Ok(chunk))) => {
                    writer.write_all(&chunk).await?;
                    writer.flush().await?;
                }
                Ok(Some(Err(e))) => return Err(Box::new(e)),
                Ok(None) => break,
                Err(_elapsed) => break,
            }
        }
        writer.flush().await?;

        if !keep_alive {
            return Ok(());
        }
    }
}

fn is_m4s_segment(url: &str) -> bool {
    let clean = url
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    clean.to_ascii_lowercase().ends_with(".m4s")
}

async fn warmup_dash_sidecar(ctx: &ProxyContext, mpd_url: &str) {
    let mut req = ctx.client.get(mpd_url);
    for (name, val) in ctx.auth_headers.iter() {
        req = req.header(name.as_str(), val.as_str());
    }
    let Ok(res) = req.send().await else {
        return;
    };
    if !res.status().is_success() {
        return;
    }
    let Ok(manifest_bytes) = res.bytes().await else {
        return;
    };
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return;
    }
    let manifest_str = String::from_utf8_lossy(&manifest_bytes);
    let rewritten = rewrite_dash_manifest(
        &manifest_str,
        ctx.proxy_port,
        ctx.target_host.as_deref(),
        ctx.subtitle_url.as_deref(),
        ctx.max_height,
    );
    if let Ok(mut lock) = ctx.manifest_cache.lock() {
        *lock = Some((mpd_url.to_string(), Arc::from(rewritten.as_bytes())));
    }
    let Some((base_dir, _)) = mpd_url.rsplit_once('/') else {
        return;
    };
    let vid_id = if rewritten.contains("id=\"0\"") {
        "0"
    } else if rewritten.contains("id=\"1\"") {
        "1"
    } else {
        "2"
    };
    for rel in [
        format!("init-stream{vid_id}.m4s"),
        "init-stream3.m4s".to_string(),
        format!("chunk-stream{vid_id}-00001.m4s"),
        "chunk-stream3-00001.m4s".to_string(),
    ] {
        let seg_url = format!("{base_dir}/{rel}");
        if !ctx.segment_cache.contains_or_in_flight(&seg_url) {
            let cache_clone = Arc::clone(&ctx.segment_cache);
            let client_clone = ctx.client.clone();
            let headers_clone = Arc::clone(&ctx.auth_headers);
            tokio::spawn(async move {
                let _ = fetch_or_get_segment(
                    &cache_clone,
                    &client_clone,
                    &seg_url,
                    &headers_clone,
                    true,
                )
                .await;
            });
        }
    }
}

fn next_dash_segment_urls(url: &str, count: u32) -> Vec<String> {
    let (base, query_suffix) = match url.find('?') {
        Some(idx) => (&url[..idx], &url[idx..]),
        None => (url, ""),
    };
    if !base.to_ascii_lowercase().ends_with(".m4s") {
        return Vec::new();
    }
    let without_ext = &base[..base.len() - 4];
    let file_stem = without_ext.rsplit('/').next().unwrap_or(without_ext);
    if let Some(init_id) = file_stem.strip_prefix("init-stream") {
        let dir_prefix = &without_ext[..without_ext.len() - file_stem.len()];
        return if init_id == "3" {
            vec![format!("{dir_prefix}init-stream0.m4s{query_suffix}")]
        } else {
            vec![format!("{dir_prefix}init-stream3.m4s{query_suffix}")]
        };
    }
    if file_stem.to_ascii_lowercase().starts_with("init") {
        return Vec::new();
    }
    let digit_count = without_ext
        .bytes()
        .rev()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digit_count == 0 || digit_count > 9 {
        return Vec::new();
    }
    let split_pos = without_ext.len() - digit_count;
    let prefix = &without_ext[..split_pos];
    if !prefix.ends_with(['-', '_', '/']) {
        return Vec::new();
    }
    let digits = &without_ext[split_pos..];
    let Ok(num) = digits.parse::<u64>() else {
        return Vec::new();
    };
    let mut out: Vec<String> = (1..=u64::from(count))
        .map(|step| {
            let next_num = num + step;
            format!("{prefix}{next_num:0digit_count$}.m4s{query_suffix}")
        })
        .collect();

    for vid_tag in ["chunk-stream0-", "chunk-stream1-", "chunk-stream2-"] {
        if let Some(dir_prefix) = prefix.strip_suffix(vid_tag) {
            out.push(format!(
                "{dir_prefix}chunk-stream3-{num:0digit_count$}.m4s{query_suffix}"
            ));
            let next_num = num + 1;
            out.push(format!(
                "{dir_prefix}chunk-stream3-{next_num:0digit_count$}.m4s{query_suffix}"
            ));
            break;
        }
    }
    out
}

fn parse_content_range_total(header_val: &str) -> Option<usize> {
    let (_, total_str) = header_val.trim().split_once('/')?;
    total_str.trim().parse::<usize>().ok()
}

fn parse_byte_range_request(header_val: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 {
        return None;
    }
    let spec = header_val.trim().strip_prefix("bytes=")?;
    let (start_str, end_str) = spec.split_once('-')?;
    let start = start_str.trim().parse::<usize>().ok()?;
    if start >= total {
        return None;
    }
    let end = if end_str.trim().is_empty() {
        total - 1
    } else {
        end_str.trim().parse::<usize>().ok()?.min(total - 1)
    };
    if start <= end {
        Some((start, end))
    } else {
        None
    }
}

async fn fetch_or_get_segment(
    cache: &Arc<SegmentCache>,
    client: &reqwest::Client,
    url: &str,
    auth_headers: &[(String, String)],
    forward_all_headers: bool,
) -> Result<(reqwest::StatusCode, String, Arc<[u8]>), reqwest::Error> {
    if let Some((ct, data)) = cache.get(url) {
        return Ok((reqwest::StatusCode::OK, ct, data));
    }

    let (is_owner, notify, notified_fut) = match cache.in_flight.lock() {
        Ok(mut lock) => {
            if let Some(existing) = lock.get(url) {
                let n = Arc::clone(existing);
                (false, Arc::clone(&n), Some(n.notified_owned()))
            } else {
                let n = Arc::new(tokio::sync::Notify::new());
                lock.insert(url.to_string(), Arc::clone(&n));
                (true, n, None)
            }
        }
        Err(_) => (true, Arc::new(tokio::sync::Notify::new()), None),
    };

    if let Some(fut) = notified_fut {
        let _ = tokio::time::timeout(Duration::from_secs(15), fut).await;
        if let Some((ct, data)) = cache.get(url) {
            return Ok((reqwest::StatusCode::OK, ct, data));
        }
    }

    let _guard = if is_owner {
        Some(InFlightGuard {
            cache: Arc::clone(cache),
            url: url.to_string(),
            notify,
        })
    } else {
        None
    };

    let (status, content_type, data) =
        fetch_m4s_chunked(client, url, auth_headers, forward_all_headers).await?;
    if status.is_success() && !data.is_empty() && data.len() <= MAX_SEGMENT_BYTES {
        cache.insert(url.to_string(), content_type.clone(), Arc::clone(&data));
    }
    Ok((status, content_type, data))
}

async fn fetch_m4s_chunked(
    client: &reqwest::Client,
    url: &str,
    auth_headers: &[(String, String)],
    forward_all_headers: bool,
) -> Result<(reqwest::StatusCode, String, Arc<[u8]>), reqwest::Error> {
    let first_end = DASH_RANGE_CHUNK_BYTES - 1;
    let mut req = client
        .get(url)
        .header("Range", format!("bytes=0-{first_end}"));
    for (name, val) in auth_headers {
        if should_forward_header(forward_all_headers, name) {
            req = req.header(name.as_str(), val.as_str());
        }
    }

    let res = req.send().await?;
    let status = res.status();
    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    if !status.is_success() {
        let body = res.bytes().await.unwrap_or_default();
        return Ok((status, content_type, Arc::from(body.as_ref())));
    }

    if status == reqwest::StatusCode::OK {
        let body = res.bytes().await?;
        return Ok((
            reqwest::StatusCode::OK,
            content_type,
            Arc::from(body.as_ref()),
        ));
    }

    let total_opt = res
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range_total);
    let first_chunk = res.bytes().await?;

    let Some(total) = total_opt.filter(|&t| t > first_chunk.len() && t <= MAX_SEGMENT_BYTES) else {
        return Ok((
            reqwest::StatusCode::OK,
            content_type,
            Arc::from(first_chunk.as_ref()),
        ));
    };

    let mut ranges = Vec::new();
    let mut pos = first_chunk.len();
    while pos < total {
        let end = (pos + DASH_RANGE_CHUNK_BYTES - 1).min(total - 1);
        ranges.push((pos, end));
        pos = end + 1;
    }

    let mut assembled = Vec::with_capacity(total);
    assembled.extend_from_slice(&first_chunk);
    for batch in ranges.chunks(16) {
        let chunk_futs = batch.iter().copied().map(|(start, end)| async move {
            let mut sub_req = client
                .get(url)
                .header("Range", format!("bytes={start}-{end}"));
            for (name, val) in auth_headers {
                if should_forward_header(forward_all_headers, name) {
                    sub_req = sub_req.header(name.as_str(), val.as_str());
                }
            }
            let sub_res = sub_req.send().await?.error_for_status()?;
            sub_res.bytes().await
        });
        let batch_chunks = futures::future::try_join_all(chunk_futs).await?;
        for chunk in batch_chunks {
            assembled.extend_from_slice(&chunk);
        }
    }

    Ok((reqwest::StatusCode::OK, content_type, Arc::from(assembled)))
}
fn format_proxy_response_headers(
    headers: &reqwest::header::HeaderMap,
    target_url: &str,
    keep_alive: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    let clean_path = target_url
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_srt = clean_path.ends_with(".srt");
    let is_vtt = clean_path.ends_with(".vtt");

    for (header_name, header_val) in headers {
        let name_str = header_name.as_str();
        if (is_srt || is_vtt) && name_str.eq_ignore_ascii_case("content-type") {
            continue;
        }
        if name_str.eq_ignore_ascii_case("content-type")
            || name_str.eq_ignore_ascii_case("content-length")
            || name_str.eq_ignore_ascii_case("content-range")
            || name_str.eq_ignore_ascii_case("accept-ranges")
        {
            if let Ok(val_str) = header_val.to_str() {
                out.extend_from_slice(format!("{name_str}: {val_str}\r\n").as_bytes());
            }
        }
    }

    out.extend_from_slice(b"Access-Control-Allow-Origin: *\r\n");
    if is_srt {
        out.extend_from_slice(b"Content-Type: application/x-subrip\r\n");
    } else if is_vtt {
        out.extend_from_slice(b"Content-Type: text/vtt\r\n");
    }
    if keep_alive {
        out.extend_from_slice(b"Connection: keep-alive\r\n\r\n");
    } else {
        out.extend_from_slice(b"Connection: close\r\n\r\n");
    }
    out
}

fn extract_target_url(path_and_query: &str) -> Option<String> {
    let raw = path_and_query.strip_prefix('/')?;
    if let Some(rest) = raw.strip_prefix("sub/") {
        if let Ok(decoded) = percent_encoding::percent_decode_str(rest).decode_utf8() {
            let s = decoded.into_owned();
            if s.starts_with("http://") || s.starts_with("https://") {
                return Some(s);
            }
        }
    }
    if let Some(rest) = raw.strip_prefix("https/") {
        Some(format!("https://{rest}"))
    } else if let Some(rest) = raw.strip_prefix("http/") {
        Some(format!("http://{rest}"))
    } else if raw.starts_with("proxy?") || raw.contains("&url=") || raw.starts_with("proxy?url=") {
        let query_start = raw.find('?')?;
        let query = &raw[query_start + 1..];
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "url" {
                    return percent_encoding::percent_decode_str(v)
                        .decode_utf8()
                        .ok()
                        .map(|s| s.into_owned())
                        .filter(|s| s.starts_with("http://") || s.starts_with("https://"));
                }
            }
        }
        None
    } else {
        None
    }
}

fn filter_dash_representations(manifest: &str, max_height: u64) -> String {
    let mut reps = Vec::new();
    let mut cursor = 0;
    while let Some(start_rel) = manifest[cursor..].find("<Representation") {
        let start = cursor + start_rel;
        let rest = &manifest[start..];
        if let Some(tag_end_rel) = rest.find('>') {
            let tag_content = &rest[..=tag_end_rel];
            if tag_content.ends_with("/>") {
                let end = start + tag_end_rel + 1;
                reps.push((start, end));
                cursor = end;
            } else if let Some(close_rel) = rest.find("</Representation>") {
                let end = start + close_rel + "</Representation>".len();
                reps.push((start, end));
                cursor = end;
            } else {
                cursor = start + tag_end_rel + 1;
            }
        } else {
            break;
        }
    }

    if reps.is_empty() {
        return manifest.to_string();
    }

    let parse_height = |block: &str| -> Option<u64> {
        let mut search = block;
        while let Some(pos) = search.find("height=") {
            let after = &search[pos + "height=".len()..];
            let quote = after.chars().next()?;
            if quote == '"' || quote == '\'' {
                let digits: String = after[1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(h) = digits.parse::<u64>() {
                    return Some(h);
                }
            }
            search = after;
        }
        None
    };

    let heights: Vec<Option<u64>> = reps
        .iter()
        .map(|&(s, e)| parse_height(&manifest[s..e]))
        .collect();
    let video_heights: Vec<u64> = heights.iter().filter_map(|&h| h).collect();
    if video_heights.is_empty() {
        return manifest.to_string();
    }

    let target_ceiling = if video_heights.iter().any(|&h| h <= max_height) {
        max_height
    } else {
        *video_heights.iter().min().unwrap_or(&max_height)
    };

    let mut out = String::with_capacity(manifest.len());
    let mut last = 0;
    for (&(s, e), h_opt) in reps.iter().zip(heights.iter()) {
        if let Some(h) = h_opt {
            if *h > target_ceiling {
                out.push_str(&manifest[last..s]);
                last = e;
            }
        }
    }
    out.push_str(&manifest[last..]);
    out
}

fn rewrite_dash_manifest(
    manifest: &str,
    proxy_port: u16,
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
    max_height: Option<u64>,
) -> String {
    let Some(host) = target_host else {
        return manifest.to_string();
    };

    let filtered_manifest = if let Some(limit) = max_height.filter(|&h| h > 0) {
        filter_dash_representations(manifest, limit)
    } else {
        manifest.to_string()
    };

    let https_prefix = format!("https://{host}/");
    let http_prefix = format!("http://{host}/");

    let proxy_https = format!("http://127.0.0.1:{proxy_port}/https/{host}/");
    let proxy_http = format!("http://127.0.0.1:{proxy_port}/http/{host}/");

    let mut rewritten = filtered_manifest
        .replace(&https_prefix, &proxy_https)
        .replace(&http_prefix, &proxy_http);

    if let Some(sub) = subtitle_url {
        if !sub.is_empty() {
            let encoded_sub =
                percent_encoding::utf8_percent_encode(sub, percent_encoding::NON_ALPHANUMERIC);
            let sub_proxy_url = format!("http://127.0.0.1:{proxy_port}/sub/{encoded_sub}");
            let sub_mime = match crate::service::subtitle_extension_from_url(sub) {
                "vtt" => "text/vtt",
                "ass" | "ssa" => "text/x-ssa",
                _ => "application/x-subrip",
            };
            let sub_adaptation_set = format!(
                r#"<AdaptationSet contentType="text" mimeType="{sub_mime}" lang="en">
    <Role schemeIdUri="urn:mpeg:dash:role:2011" value="subtitle"/>
    <Representation id="sub_en" bandwidth="1000">
      <BaseURL>{sub_proxy_url}</BaseURL>
    </Representation>
  </AdaptationSet>
</Period>"#
            );
            if rewritten.contains("</Period>") {
                rewritten = rewritten.replacen("</Period>", &sub_adaptation_set, 1);
            }
        }
    }

    rewritten
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_target_url() {
        assert_eq!(
            extract_target_url("/https/sacdn.example.com/dash/index.mpd").as_deref(),
            Some("https://sacdn.example.com/dash/index.mpd")
        );
        assert_eq!(
            extract_target_url("/http/example.com/video.mp4").as_deref(),
            Some("http://example.com/video.mp4")
        );
        assert_eq!(
            extract_target_url("/https/example.com/seg.m4s?token=abc&exp=123").as_deref(),
            Some("https://example.com/seg.m4s?token=abc&exp=123")
        );
        assert_eq!(
            extract_target_url("/proxy?url=https%3A%2F%2Fexample.com%2Ffallback.mpd").as_deref(),
            Some("https://example.com/fallback.mpd")
        );
        assert_eq!(extract_target_url("/invalid/path"), None);
    }
    #[test]
    fn test_extract_target_url_sub_rejects_non_http() {
        assert_eq!(
            extract_target_url("/sub/file%3A%2F%2F%2Fetc%2Fpasswd"),
            None
        );
        assert_eq!(extract_target_url("/sub/data%3Atext%2Fhtml%2Chello"), None);
        assert!(
            extract_target_url("/sub/https%3A%2F%2Fcdn.example.com%2Fsub.vtt")
                .as_deref()
                .unwrap_or("")
                .starts_with("https://")
        );
    }

    #[test]
    fn test_extract_host_authority_strips_path_preserves_port() {
        assert_eq!(
            extract_host_authority("https://cdn.example.com/dash/index.mpd").as_deref(),
            Some("cdn.example.com")
        );
        assert_eq!(
            extract_host_authority("http://cdn.example.com:8080/dash/index.mpd").as_deref(),
            Some("cdn.example.com:8080")
        );
        assert_eq!(extract_host_authority("not-a-url"), None);
    }
    #[test]
    fn test_host_whitelist_allows_target_and_subtitle_hosts() {
        let target_host = "video.example.com";
        let subtitle_url = "https://captions.example.com/sub.srt";
        let sub_host = extract_host_authority(subtitle_url);

        let is_allowed = |url: &str| -> bool {
            let host = extract_host_authority(url);
            host.as_deref() == Some(target_host) || (sub_host.is_some() && host == sub_host)
        };

        assert!(is_allowed("https://video.example.com/chunk.m4s"));
        assert!(is_allowed("https://captions.example.com/sub.srt"));
        assert!(!is_allowed("https://evil.example.com/steal"));
        assert!(!is_allowed("https://sub.evil.com/fake"));
    }
    #[test]
    fn test_host_whitelist_rejects_missing_target_host() {
        let target_host: Option<&str> = None;
        let subtitle_url: Option<&str> = None;
        let sub_host = subtitle_url.and_then(extract_host_authority);

        let is_allowed = |url: &str| -> bool {
            let extracted_host = extract_host_authority(url);
            match (target_host, extracted_host.as_deref()) {
                (Some(allowed), Some(extracted)) => {
                    extracted == allowed || (sub_host.is_some() && extracted_host == sub_host)
                }
                _ => false,
            }
        };

        assert!(!is_allowed("https://video.example.com/chunk.m4s"));
        assert!(!is_allowed("http://127.0.0.1:8080"));
    }

    #[test]
    fn test_rewrite_dash_manifest_scoped_to_target_host() {
        let manifest = r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" xsi:schemaLocation="urn:mpeg:dash:schema:mpd:2011 https://standards.iso.org/schema.xsd">
<Period>
  <AdaptationSet>
    <Representation id="1080p">
      <BaseURL>https://sacdn.example.com/dash/123/1080.mp4</BaseURL>
      <SegmentTemplate initialization="https://sacdn.example.com/dash/123/init.m4s" media="seg_$Number$.m4s" />
    </Representation>
  </AdaptationSet>
</Period>
</MPD>"#;

        let rewritten =
            rewrite_dash_manifest(manifest, 8888, Some("sacdn.example.com"), None, None);
        assert!(
            rewritten.contains("https://standards.iso.org/schema.xsd"),
            "XML namespace schema must not be corrupted"
        );

        assert!(
            rewritten.contains("http://127.0.0.1:8888/https/sacdn.example.com/dash/123/1080.mp4"),
            "Matching host BaseURL must be rewritten to proxy route"
        );

        assert!(
            rewritten.contains("http://127.0.0.1:8888/https/sacdn.example.com/dash/123/init.m4s"),
            "Matching host initialization URL must be rewritten to proxy route"
        );

        assert!(
            rewritten.contains("media=\"seg_$Number$.m4s\""),
            "Relative media template must remain relative"
        );
    }

    #[test]
    fn test_rewrite_dash_manifest_with_explicit_port() {
        let manifest = r#"<MPD><Period><BaseURL>https://cdn.example.com:8080/dash/seg.mp4</BaseURL></Period></MPD>"#;
        let rewritten =
            rewrite_dash_manifest(manifest, 9999, Some("cdn.example.com:8080"), None, None);
        assert!(
            rewritten.contains("http://127.0.0.1:9999/https/cdn.example.com:8080/dash/seg.mp4"),
            "Port must be preserved in proxy route"
        );
    }
    #[test]
    fn test_format_proxy_response_headers_for_srt_with_query_and_case() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "text/plain".parse().unwrap());
        headers.insert("content-length", "1234".parse().unwrap());
        headers.insert("accept-ranges", "bytes".parse().unwrap());
        headers.insert("server", "cloudflare".parse().unwrap());

        let url = "https://cdn.example.com/subs.SRT?token=abc123&expires=999#top";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url, false)).unwrap();
        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert_eq!(out.matches("Content-Type: application/x-subrip").count(), 1);
        assert_eq!(out.matches("Connection: close").count(), 1);
        assert!(!out.contains("text/plain"));
        assert!(out.contains("content-length: 1234"));
        assert!(out.contains("accept-ranges: bytes"));
    }

    #[test]
    fn test_format_proxy_response_headers_for_vtt() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "application/octet-stream".parse().unwrap());
        headers.insert("content-length", "500".parse().unwrap());

        let url = "https://cdn.example.com/subs.vtt";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url, false)).unwrap();
        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert_eq!(out.matches("Content-Type: text/vtt").count(), 1);
        assert!(!out.contains("application/octet-stream"));
        assert!(out.contains("content-length: 500"));
    }

    #[test]
    fn test_format_proxy_response_headers_for_media_stream() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "video/mp4".parse().unwrap());
        headers.insert("content-length", "10000000".parse().unwrap());

        let url = "https://cdn.example.com/video.mp4";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url, true)).unwrap();
        assert_eq!(out.matches("Access-Control-Allow-Origin: *").count(), 1);
        assert!(out.contains("content-type: video/mp4"));
        assert!(!out.contains("application/x-subrip"));
        assert!(!out.contains("text/vtt"));
    }

    #[test]
    fn test_format_proxy_response_headers_keep_alive() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "video/mp4".parse().unwrap());
        headers.insert("content-length", "1000".parse().unwrap());

        let url = "https://cdn.example.com/segment.m4s";
        let out = String::from_utf8(format_proxy_response_headers(&headers, url, true)).unwrap();
        assert!(out.contains("Connection: keep-alive"));
        assert!(!out.contains("Connection: close"));
    }

    #[test]
    fn test_filter_dash_representations_caps_height() {
        let manifest = r#"<MPD>
<Period>
  <AdaptationSet>
    <Representation id="1080" height="1080" bandwidth="800000"><BaseURL>1080.m4s</BaseURL></Representation>
    <Representation id="720" height="720" bandwidth="400000"><BaseURL>720.m4s</BaseURL></Representation>
    <Representation id="480" height="480" bandwidth="200000"><BaseURL>480.m4s</BaseURL></Representation>
  </AdaptationSet>
</Period>
</MPD>"#;
        let filtered = filter_dash_representations(manifest, 720);
        assert!(!filtered.contains("height=\"1080\""));
        assert!(filtered.contains("height=\"720\""));
        assert!(filtered.contains("height=\"480\""));

        let filtered_480 = filter_dash_representations(manifest, 480);
        assert!(!filtered_480.contains("height=\"1080\""));
        assert!(!filtered_480.contains("height=\"720\""));
        assert!(filtered_480.contains("height=\"480\""));
    }

    #[test]
    fn test_next_dash_segment_urls_preserves_zero_padding_and_prefetches_audio() {
        let next = next_dash_segment_urls(
            "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream0-00721.m4s?token=abc",
            3,
        );
        assert_eq!(
            next,
            vec![
                "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream0-00722.m4s?token=abc",
                "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream0-00723.m4s?token=abc",
                "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream0-00724.m4s?token=abc",
                "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream3-00721.m4s?token=abc",
                "https://sbcdn3.hakunaymatata.com/dash/123/chunk-stream3-00722.m4s?token=abc",
            ]
        );
        assert_eq!(
            next_dash_segment_urls(
                "https://sbcdn3.hakunaymatata.com/dash/123/init-stream0.m4s",
                3
            ),
            vec!["https://sbcdn3.hakunaymatata.com/dash/123/init-stream3.m4s"]
        );
        assert!(
            next_dash_segment_urls("https://sbcdn3.hakunaymatata.com/dash/123/init.m4s", 3)
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_fetch_m4s_chunked_splits_ranges_and_caches() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let total_size = 240 * 1024;
        let payload: Arc<Vec<u8>> = Arc::new((0..total_size).map(|i| (i % 253) as u8).collect());
        let request_count = Arc::new(AtomicUsize::new(0));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/dash/chunk-stream0-00001.m4s");

        let srv_payload = Arc::clone(&payload);
        let srv_reqs = Arc::clone(&request_count);
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let data = Arc::clone(&srv_payload);
                let reqs = Arc::clone(&srv_reqs);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    reqs.fetch_add(1, Ordering::Relaxed);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let mut range_val = None;
                    for line in req.lines() {
                        if let Some(v) = line
                            .strip_prefix("Range: bytes=")
                            .or_else(|| line.strip_prefix("range: bytes="))
                        {
                            range_val = Some(v.trim().to_string());
                        }
                    }
                    let total = data.len();
                    let (start, end) = range_val
                        .and_then(|r| {
                            let (s, e) = r.split_once('-')?;
                            Some((
                                s.parse::<usize>().ok()?,
                                e.parse::<usize>().ok()?.min(total - 1),
                            ))
                        })
                        .unwrap_or((0, total - 1));
                    let slice = &data[start..=end];
                    let hdr = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/iso.segment\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        slice.len()
                    );
                    let _ = socket.write_all(hdr.as_bytes()).await;
                    let _ = socket.write_all(slice).await;
                });
            }
        });

        let client = crate::net::http_client_builder_base().build().unwrap();
        let cache = Arc::new(SegmentCache::default());
        let (status, ct, assembled) = fetch_or_get_segment(&cache, &client, &url, &[], true)
            .await
            .unwrap();
        assert_eq!(status, reqwest::StatusCode::OK);
        assert_eq!(ct, "video/iso.segment");
        assert_eq!(assembled.as_ref(), payload.as_slice());
        assert_eq!(request_count.load(Ordering::Relaxed), 3);

        let (_, _, cached) = fetch_or_get_segment(&cache, &client, &url, &[], true)
            .await
            .unwrap();
        assert_eq!(cached.as_ref(), payload.as_slice());
        assert_eq!(request_count.load(Ordering::Relaxed), 3);
        server.abort();
    }

    #[tokio::test]
    async fn test_warmup_dash_sidecar_populates_manifest_and_opening_segments() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.contains("/dash/index.mpd") {
                        let mpd = r#"<MPD><Period><AdaptationSet contentType="video"><Representation id="0" height="720" bandwidth="1000"><SegmentTemplate initialization="init-stream$RepresentationID$.m4s" media="chunk-stream$RepresentationID$-$Number%05d$.m4s"/></Representation></AdaptationSet></Period></MPD>"#;
                        let hdr = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/dash+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            mpd.len(),
                            mpd
                        );
                        let _ = socket.write_all(hdr.as_bytes()).await;
                    } else {
                        let seg = b"SEGMENT_BYTES";
                        let hdr = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: video/iso.segment\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            seg.len()
                        );
                        let _ = socket.write_all(hdr.as_bytes()).await;
                        let _ = socket.write_all(seg).await;
                    }
                });
            }
        });

        let target_url = format!("http://{addr}/dash/index.mpd");
        let ctx = ProxyContext {
            proxy_port: addr.port(),
            client: crate::net::http_client_builder_base().build().unwrap(),
            auth_headers: Arc::new(Vec::new()),
            target_host: Some(addr.to_string()),
            subtitle_url: None,
            max_height: Some(720),
            segment_cache: Arc::new(SegmentCache::default()),
            manifest_cache: Arc::new(Mutex::new(None)),
        };

        warmup_dash_sidecar(&ctx, &target_url).await;
        for _ in 0..20 {
            if ctx
                .segment_cache
                .get(&format!("http://{addr}/dash/chunk-stream0-00001.m4s"))
                .is_some()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        assert!(ctx.manifest_cache.lock().unwrap().is_some());
        assert!(
            ctx.segment_cache
                .get(&format!("http://{addr}/dash/init-stream0.m4s"))
                .is_some()
        );
        assert!(
            ctx.segment_cache
                .get(&format!("http://{addr}/dash/init-stream3.m4s"))
                .is_some()
        );
        assert!(
            ctx.segment_cache
                .get(&format!("http://{addr}/dash/chunk-stream0-00001.m4s"))
                .is_some()
        );
        server.abort();
    }
}
