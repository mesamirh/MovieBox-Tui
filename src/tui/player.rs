#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::state::PlayerKind;

/// Resolve an installed media player executable.
pub fn resolve_player(kind: PlayerKind) -> Option<PathBuf> {
    match kind {
        PlayerKind::Mpv => resolve_mpv(),
        PlayerKind::Iina => resolve_iina(),
        PlayerKind::Vlc => resolve_vlc(),
    }
}

pub fn detect_available_players() -> Vec<PlayerKind> {
    let mut players = Vec::new();
    if resolve_mpv().is_some() {
        players.push(PlayerKind::Mpv);
    }
    if resolve_iina().is_some() {
        players.push(PlayerKind::Iina);
    }
    if resolve_vlc().is_some() {
        players.push(PlayerKind::Vlc);
    }
    players
}

pub fn mpv_log_path() -> PathBuf {
    std::env::temp_dir().join("moviebox-tui-mpv.log")
}

fn resolve_mpv() -> Option<PathBuf> {
    if let Some(path) = find_on_path("mpv") {
        return Some(path);
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\MPV Player\mpv.exe",
            r"C:\Program Files\mpv\mpv.exe",
            r"C:\Program Files (x86)\MPV Player\mpv.exe",
            r"C:\Program Files (x86)\mpv\mpv.exe",
        ];
        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return Some(path);
            }
        }

        if let Some(home) = dirs::home_dir() {
            let scoop = home.join(r"scoop\apps\mpv\current\mpv.exe");
            if scoop.is_file() {
                return Some(scoop);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let app = PathBuf::from("/Applications/mpv.app/Contents/MacOS/mpv");
        if app.is_file() {
            return Some(app);
        }
    }

    None
}

fn resolve_iina() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if Path::new("/Applications/IINA.app").exists() {
            return Some(PathBuf::from("open"));
        }
    }

    find_on_path("iina")
}

fn resolve_vlc() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let app = PathBuf::from("/Applications/VLC.app/Contents/MacOS/VLC");
        if app.is_file() {
            return Some(app);
        }
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\VideoLAN\VLC\vlc.exe",
            r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        ];
        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    find_on_path("vlc")
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let output = Command::new("where")
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = stdout.lines().next()?.trim();
        if first.is_empty() {
            return None;
        }
        let path = PathBuf::from(first);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("which")
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = stdout.lines().next()?.trim();
        if first.is_empty() {
            return None;
        }
        let path = PathBuf::from(first);
        if path.is_file() || path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

/// Build a process command for streaming a URL (or local path) in the given player.
pub fn build_player_command(
    kind: PlayerKind,
    link: &str,
    subtitle: Option<&str>,
) -> Option<Command> {
    let exe = resolve_player(kind)?;
    let mut cmd = match kind {
        PlayerKind::Mpv => {
            let mut c = Command::new(&exe);
            // Direct stream — mpv pulls the URL itself (no pre-download).
            c.arg("--no-terminal");
            c.arg("--ytdl=no");
            c.arg("--force-window=immediate");
            c.arg("--network-timeout=60");
            c.arg(format!("--log-file={}", mpv_log_path().display()));

            #[cfg(windows)]
            {
                c.arg("--taskbar-progress=no");
            }

            if let Some(sub) = subtitle.filter(|s| !s.is_empty()) {
                c.arg(format!("--sub-file={sub}"));
            }
            c.arg("--");
            c.arg(link);
            c
        }
        PlayerKind::Iina => {
            let mut c = Command::new(&exe);
            if exe.file_name().and_then(|n| n.to_str()) == Some("open") {
                c.arg("-a").arg("IINA").arg(link);
            } else {
                c.arg(link);
            }
            c
        }
        PlayerKind::Vlc => {
            let mut c = Command::new(&exe);
            c.arg(link);
            if let Some(sub) = subtitle.filter(|s| !s.is_empty()) {
                c.arg("--sub-file").arg(sub);
            }
            c
        }
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    Some(cmd)
}

/// Stream CDN → localhost → mpv without saving a file.
/// Uses the same HTTP client as download (which works), but pipes live.
pub async fn start_stream_proxy(
    http: reqwest::Client,
    upstream: String,
) -> Result<u16, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Proxy bind failed: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Proxy addr failed: {e}"))?
        .port();

    tokio::spawn(async move {
        // Handle a few connections (mpv may open Range probes).
        for _ in 0..6 {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let http = http.clone();
            let upstream = upstream.clone();
            tokio::spawn(async move {
                let mut req_buf = vec![0u8; 16384];
                let Ok(n) = socket.read(&mut req_buf).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&req_buf[..n]);
                let range = req
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                    .map(|l| l[6..].trim().to_string());

                let mut builder = http
                    .get(&upstream)
                    .timeout(std::time::Duration::from_secs(1800));
                if let Some(r) = range {
                    builder = builder.header("Range", r);
                }

                let upstream_resp = match builder.send().await {
                    Ok(r) => r,
                    Err(e) => {
                        let body = format!("proxy upstream error: {e}");
                        let resp = format!(
                            "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                        return;
                    }
                };

                let code = upstream_resp.status().as_u16();
                if !matches!(code, 200 | 206) {
                    let body = format!("upstream HTTP {code}");
                    let resp = format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    return;
                }

                let mut headers = format!("HTTP/1.1 {code}\r\n");
                for (k, v) in upstream_resp.headers().iter() {
                    let name = k.as_str();
                    if matches!(
                        name,
                        "transfer-encoding" | "connection" | "keep-alive" | "proxy-connection"
                    ) {
                        continue;
                    }
                    if let Ok(val) = v.to_str() {
                        headers.push_str(&format!("{name}: {val}\r\n"));
                    }
                }
                headers.push_str("Connection: close\r\n\r\n");
                if socket.write_all(headers.as_bytes()).await.is_err() {
                    return;
                }

                let mut body = upstream_resp;
                while let Ok(Some(chunk)) = body.chunk().await {
                    if socket.write_all(&chunk).await.is_err() {
                        break;
                    }
                }
            });
        }
    });

    Ok(port)
}
