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
) -> Result<String, String> {
    let exe = std::env::current_exe()
        .ok()
        .or_else(|| std::env::args().next().map(PathBuf::from))
        .ok_or_else(|| "unable to locate current executable".to_string())?;

    let headers_json = serde_json::to_string(headers).unwrap_or_else(|_| "[]".to_string());
    let sub_arg = subtitle_url.unwrap_or("");

    let mut cmd = Command::new(exe);
    cmd.args(["--proxy-for-vlc", target_url, &headers_json, sub_arg]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

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

    Ok(format!("http://127.0.0.1:{port}{proxy_path}"))
}

pub async fn run_sidecar(
    target_url: String,
    headers: Vec<(String, String)>,
    subtitle_url: Option<String>,
) {
    let client = crate::net::http_client_builder()
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
            let elapsed = {
                let lock = watchdog_activity.lock().unwrap();
                lock.elapsed()
            };
            if conns == 0 && elapsed > Duration::from_secs(WATCHDOG_IDLE_SECS) {
                std::process::exit(0);
            }
        }
    });

    let target_host = extract_host_authority(&target_url);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(val) => val,
            Err(err) => {
                log::warn!("transient proxy accept error: {err}");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let client = client.clone();
        let headers = headers.clone();
        let target_host = target_host.clone();
        let active_conns = Arc::clone(&active_connections);
        let activity = Arc::clone(&last_activity);

        active_conns.fetch_add(1, Ordering::Relaxed);
        {
            if let Ok(mut lock) = activity.lock() {
                *lock = Instant::now();
            }
        }

        let sub_opt = subtitle_url.clone();
        tokio::spawn(async move {
            let _guard = ConnectionGuard {
                conns: active_conns,
                activity,
            };
            let _ = handle_connection(
                stream,
                port,
                &client,
                &headers,
                target_host.as_deref(),
                sub_opt.as_deref(),
            )
            .await;
        });
    }
}

fn extract_host_authority(url: &str) -> Option<String> {
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

async fn handle_connection(
    stream: TcpStream,
    proxy_port: u16,
    client: &reqwest::Client,
    auth_headers: &[(String, String)],
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let mut request_line = String::new();
    let n = buf_reader.read_line(&mut request_line).await?;
    if n == 0 {
        return Ok(());
    }
    if request_line.len() > MAX_LINE_BYTES {
        writer
            .write_all(b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await?;
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path_and_query = parts.next().unwrap_or("/");

    let mut range_header = None;
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
            }
        }
    }

    let target_url = match extract_target_url(path_and_query) {
        Some(url) => url,
        None => {
            let response =
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            writer.write_all(response.as_bytes()).await?;
            return Ok(());
        }
    };

    let mut req = match method {
        "HEAD" => client.head(&target_url),
        _ => client.get(&target_url),
    };

    for (name, val) in auth_headers {
        req = req.header(name.as_str(), val.as_str());
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
            return Ok(());
        }
    };

    let status = upstream_res.status();
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK")
    );
    writer.write_all(status_line.as_bytes()).await?;

    let content_length = upstream_res
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    let is_dash_manifest = target_url.ends_with(".mpd")
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
                        "Content-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await?;
            return Ok(());
        }
        let manifest_str = String::from_utf8_lossy(&manifest_bytes);
        let rewritten = rewrite_dash_manifest(&manifest_str, proxy_port, target_host, subtitle_url);
        let rewritten_bytes = rewritten.as_bytes();

        let headers_out = format!(
            "Content-Type: application/dash+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            rewritten_bytes.len()
        );
        writer.write_all(headers_out.as_bytes()).await?;
        writer.write_all(rewritten_bytes).await?;
        writer.flush().await?;
        return Ok(());
    }

    for (header_name, header_val) in upstream_res.headers() {
        let name_str = header_name.as_str();
        if target_url.ends_with(".srt") || target_url.ends_with(".vtt") {
            if name_str.eq_ignore_ascii_case("content-type") {
                continue;
            }
        }
        if name_str.eq_ignore_ascii_case("content-type")
            || name_str.eq_ignore_ascii_case("content-length")
            || name_str.eq_ignore_ascii_case("content-range")
            || name_str.eq_ignore_ascii_case("accept-ranges")
        {
            if let Ok(val_str) = header_val.to_str() {
                writer
                    .write_all(format!("{name_str}: {val_str}\r\n").as_bytes())
                    .await?;
            }
        }
    }
    writer
        .write_all(b"Access-Control-Allow-Origin: *\r\n")
        .await?;
    if target_url.ends_with(".srt") {
        writer
            .write_all(b"Content-Type: application/x-subrip\r\n")
            .await?;
    } else if target_url.ends_with(".vtt") {
        writer.write_all(b"Content-Type: text/vtt\r\n").await?;
    }
    writer.write_all(b"Connection: close\r\n\r\n").await?;

    let mut stream = upstream_res.bytes_stream();
    loop {
        let chunk_result =
            tokio::time::timeout(Duration::from_secs(CHUNK_IDLE_TIMEOUT_SECS), stream.next()).await;
        match chunk_result {
            Ok(Some(Ok(chunk))) => writer.write_all(&chunk).await?,
            Ok(Some(Err(e))) => return Err(Box::new(e)),
            Ok(None) => break,
            Err(_elapsed) => break,
        }
    }
    writer.flush().await?;

    Ok(())
}

fn extract_target_url(path_and_query: &str) -> Option<String> {
    let raw = path_and_query.strip_prefix('/')?;
    if let Some(rest) = raw.strip_prefix("sub/") {
        if let Ok(decoded) = percent_encoding::percent_decode_str(rest).decode_utf8() {
            return Some(decoded.into_owned());
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
                        .map(|s| s.into_owned());
                }
            }
        }
        None
    } else {
        None
    }
}

fn rewrite_dash_manifest(
    manifest: &str,
    proxy_port: u16,
    target_host: Option<&str>,
    subtitle_url: Option<&str>,
) -> String {
    let Some(host) = target_host else {
        return manifest.to_string();
    };

    let https_prefix = format!("https://{host}/");
    let http_prefix = format!("http://{host}/");

    let proxy_https = format!("http://127.0.0.1:{proxy_port}/https/{host}/");
    let proxy_http = format!("http://127.0.0.1:{proxy_port}/http/{host}/");

    let mut rewritten = manifest
        .replace(&https_prefix, &proxy_https)
        .replace(&http_prefix, &proxy_http);

    if let Some(sub) = subtitle_url {
        if !sub.is_empty() {
            let encoded_sub =
                percent_encoding::utf8_percent_encode(sub, percent_encoding::NON_ALPHANUMERIC);
            let sub_proxy_url = format!("http://127.0.0.1:{proxy_port}/sub/{encoded_sub}");
            let sub_adaptation_set = format!(
                r#"<AdaptationSet contentType="text" mimeType="text/vtt" lang="en">
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

        let rewritten = rewrite_dash_manifest(manifest, 8888, Some("sacdn.example.com"), None);

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
        let rewritten = rewrite_dash_manifest(manifest, 9999, Some("cdn.example.com:8080"), None);
        assert!(
            rewritten.contains("http://127.0.0.1:9999/https/cdn.example.com:8080/dash/seg.mp4"),
            "Port must be preserved in proxy route"
        );
    }
}
