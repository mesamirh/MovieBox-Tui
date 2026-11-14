//! Built-in torrent engine.
//!
//! Plays the same role as Stremio's bundled enginefs server: a torrent client
//! (`librqbit`) plus a tiny local HTTP server that serves one file of a
//! torrent with Range support, so mpv/VLC can stream and seek while pieces
//! are still downloading. It answers `/stream?link=<magnet>&index=N` URLs on
//! loopback only, so every player and download path works unchanged.

use crate::providers::addons::torrent::{
    TorrentFile, guess_file_id_with_hint, magnet_display_name,
};
use librqbit::{
    AddTorrent, AddTorrentOptions, ListenerMode, ListenerOptions, ManagedTorrent, Session,
    SessionOptions,
};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, OnceCell};

/// Time allowed to find peers and fetch the torrent metadata. Kept below the
/// playback timeout in `tui/app/playback.rs` so the user sees a real error.
const METADATA_TIMEOUT: Duration = Duration::from_secs(90);
const DEFAULT_LISTEN_PORT: u16 = 4240;
const LISTEN_PORT_TRIES: u16 = 20;
const MAX_REQUEST_HEAD: usize = 16 * 1024;
const COPY_BUF: usize = 64 * 1024;
/// Bytes the player must have read before playback counts as started. Players
/// first probe the container header and index (a few hundred KB), so 2 MiB
/// means real video data is flowing, not just the probe.
const PLAYING_AFTER_BYTES: u64 = 2 * 1024 * 1024;
/// Bytes pre-buffered from the start of the selected file before launching the
/// player, plus the container tail (MP4 `moov` atom / MKV `Cues`).
const PREWARM_HEAD_BYTES: u64 = 2 * 1024 * 1024;
const PREWARM_TAIL_BYTES: u64 = 256 * 1024;
const PREWARM_TIMEOUT: Duration = Duration::from_secs(25);

struct Entry {
    handle: Arc<ManagedTorrent>,
    paused: AtomicBool,
}

pub struct Engine {
    session: Arc<Session>,
    torrents: Mutex<HashMap<String, Arc<OnceCell<Entry>>>>,
}

static ENGINE: OnceCell<Arc<Engine>> = OnceCell::const_new();

/// Starts the engine (once per process): torrent session + local HTTP server.
pub async fn ensure_started() -> Result<Arc<Engine>, String> {
    ENGINE
        .get_or_try_init(|| async {
            let dir = crate::config::cache_dir().join("torrents");
            let _ = tokio::fs::remove_dir_all(&dir).await;
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| format!("Cannot create torrent cache dir: {e}"))?;

            let session = new_session(dir).await?;

            let base_port = crate::config::builtin_torrent_port();
            let max_port = base_port.saturating_add(crate::config::BUILTIN_TORRENT_PORT_TRIES);
            let mut bound = None;
            let mut last_err = None;
            for candidate in base_port..=max_port {
                match TcpListener::bind(("127.0.0.1", candidate)).await {
                    Ok(l) => {
                        bound = Some((candidate, l));
                        break;
                    }
                    Err(e) => {
                        log::warn!("torrent engine: port 127.0.0.1:{candidate} busy: {e}");
                        last_err = Some((candidate, e));
                    }
                }
            }
            let (port, listener) = bound.ok_or_else(|| {
                let (p, e) = last_err
                    .map(|(p, e)| (p, e.to_string()))
                    .unwrap_or((base_port, "no free port".to_string()));
                format!("Cannot listen on 127.0.0.1:{p}: {e}")
            })?;
            crate::config::set_bound_torrent_port(port);

            let engine = Arc::new(Engine {
                session,
                torrents: Mutex::new(HashMap::new()),
            });
            tokio::spawn(serve(listener, engine.clone()));
            log::info!("built-in torrent engine listening on 127.0.0.1:{port}");
            Ok::<_, String>(engine)
        })
        .await
        .cloned()
}

/// First port tried for incoming peers (`MOVIEBOX_TORRENT_LISTEN_PORT` overrides).
fn listen_base_port() -> u16 {
    std::env::var("MOVIEBOX_TORRENT_LISTEN_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .filter(|p| *p > 0)
        .unwrap_or(DEFAULT_LISTEN_PORT)
}

/// Starts the torrent session the way Stremio's engine runs: it accepts
/// incoming peers and asks the router for a UPnP port mapping. A bare
/// `Session::new` only dials out, which finds far fewer peers behind NAT or
/// CGNAT. If no port can be bound it falls back to outgoing-only.
/// `MOVIEBOX_TORRENT_NO_LISTEN=1` skips listening entirely and
/// `MOVIEBOX_TORRENT_NO_UTP=1` listens on TCP only.
async fn new_session(dir: std::path::PathBuf) -> Result<Arc<Session>, String> {
    if std::env::var_os("MOVIEBOX_TORRENT_NO_LISTEN").is_none() {
        let base = listen_base_port();
        for port in base..base.saturating_add(LISTEN_PORT_TRIES) {
            let opts = SessionOptions {
                listen: Some(ListenerOptions {
                    // uTP is off by default in librqbit; Stremio's engine uses
                    // it, and many peers can only be reached that way.
                    mode: if std::env::var_os("MOVIEBOX_TORRENT_NO_UTP").is_some() {
                        ListenerMode::TcpOnly
                    } else {
                        ListenerMode::TcpAndUtp
                    },
                    listen_addr: SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
                    enable_upnp_port_forwarding: true,
                    ..Default::default()
                }),
                ..Default::default()
            };
            match Session::new_with_opts(dir.clone(), opts).await {
                Ok(session) => {
                    log::info!("torrent engine: accepting peers on port {port} (UPnP requested)");
                    return Ok(session);
                }
                Err(e) => log::warn!("torrent engine: cannot listen on port {port}: {e}"),
            }
        }
        log::warn!("torrent engine: no free listen port, running outgoing-only");
    }
    Session::new(dir)
        .await
        .map_err(|e| format!("Cannot start torrent engine: {e}"))
}

/// Adds the torrent, waits for its metadata and returns the file list.
/// `file_idx` (zero-based) limits the download to that file when known.
pub async fn prepare(magnet: &str, file_idx: Option<usize>) -> Result<Vec<TorrentFile>, String> {
    let engine = ensure_started().await?;
    let handle = engine.handle_for(magnet, file_idx).await?;
    attach_handle(&handle);
    files_of(&handle)
}

/// Pre-warms the first [`PREWARM_HEAD_BYTES`] and container tail
/// ([`PREWARM_TAIL_BYTES`], where MP4 `moov` atoms and MKV `Cues` live) of
/// `file_idx` so the media player starts against warm pieces instead of cold
/// swarm requests. Bounded by [`PREWARM_TIMEOUT`] so slow swarms still launch.
pub async fn prewarm_file(magnet: &str, file_idx: usize) {
    let Ok(engine) = ensure_started().await else {
        return;
    };
    let Ok(handle) = engine.handle_for(magnet, Some(file_idx)).await else {
        return;
    };
    attach_handle(&handle);
    let Ok(files) = files_of(&handle) else {
        return;
    };
    let Some(file) = files.get(file_idx) else {
        return;
    };
    let total = file.length;
    if total == 0 {
        return;
    }
    let head_len = total.min(PREWARM_HEAD_BYTES);
    let tail_len = if total > PREWARM_HEAD_BYTES.saturating_add(PREWARM_TAIL_BYTES) {
        PREWARM_TAIL_BYTES
    } else {
        0
    };
    PREWARM_DONE_BYTES.store(0, Ordering::Relaxed);
    PREWARM_TARGET_BYTES.store(head_len.saturating_add(tail_len), Ordering::Relaxed);

    let warm = async {
        let mut stream = Box::pin(handle.clone().stream(file_idx).await.ok()?);
        let mut buf = vec![0u8; COPY_BUF];
        let mut rem = head_len;
        while rem > 0 {
            let want = rem.min(buf.len() as u64) as usize;
            let n = stream.read(&mut buf[..want]).await.ok()?;
            if n == 0 {
                break;
            }
            PREWARM_DONE_BYTES.fetch_add(n as u64, Ordering::Relaxed);
            rem -= n as u64;
        }
        if tail_len > 0 {
            let tail_start = total.saturating_sub(tail_len);
            if stream.seek(SeekFrom::Start(tail_start)).await.is_ok() {
                let mut tail_rem = tail_len;
                while tail_rem > 0 {
                    let want = tail_rem.min(buf.len() as u64) as usize;
                    let n = stream.read(&mut buf[..want]).await.ok()?;
                    if n == 0 {
                        break;
                    }
                    PREWARM_DONE_BYTES.fetch_add(n as u64, Ordering::Relaxed);
                    tail_rem -= n as u64;
                }
            }
        }
        Some(())
    };
    let _ = tokio::time::timeout(PREWARM_TIMEOUT, warm).await;
}

/// Pauses all managed torrents in the background when playback ends or is
/// cancelled via `Esc` and no download is active, stopping idle peer traffic
/// while keeping the current season pack ready if the next episode is opened.
pub fn pause_all_background() {
    let Some(engine) = ENGINE.get().cloned() else {
        return;
    };
    if let Ok(rt) = tokio::runtime::Handle::try_current() {
        rt.spawn(async move {
            engine.pause_all().await;
        });
    }
}

// ---------------------------------------------------------------------------
// Live status, for the "torrent running" notification in the UI.
// ---------------------------------------------------------------------------

/// Where a monitored torrent is on its way to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The torrent session is still starting.
    StartingEngine,
    /// Session is up; waiting for peers and the torrent metadata.
    FindingPeers,
    /// Metadata is in; pieces are downloading but the player has not read
    /// enough data yet.
    Buffering,
    /// The player is reading video data.
    Playing,
}

/// Snapshot shown to the user while a torrent is starting up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub elapsed: Duration,
    /// Peers connected right now (seeders included).
    pub peers: u32,
    /// How many of the connected peers are seeders.
    pub seeders: u32,
    /// Peers still being dialled or queued.
    pub connecting: u32,
    pub download_bytes_per_sec: u64,
    pub downloaded_bytes: u64,
    /// Bytes the player has read from the engine so far.
    pub served_bytes: u64,
    /// Bytes pre-warmed (head + container tail) before launching the player.
    pub prewarm_done_bytes: u64,
    pub prewarm_target_bytes: u64,
}

struct Monitor {
    started: Instant,
    handle: Option<Arc<ManagedTorrent>>,
}

static MONITOR: std::sync::Mutex<Option<Monitor>> = std::sync::Mutex::new(None);
static MONITOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static SERVED_BYTES: AtomicU64 = AtomicU64::new(0);
static PREWARM_DONE_BYTES: AtomicU64 = AtomicU64::new(0);
static PREWARM_TARGET_BYTES: AtomicU64 = AtomicU64::new(0);

fn with_monitor<R>(f: impl FnOnce(&mut Option<Monitor>) -> R) -> R {
    let mut guard = MONITOR.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// Starts watching a torrent the user just asked to play. Call it right
/// before preparing the stream; [`end_session`] stops the watching again.
pub fn begin_session() {
    SERVED_BYTES.store(0, Ordering::Relaxed);
    PREWARM_DONE_BYTES.store(0, Ordering::Relaxed);
    PREWARM_TARGET_BYTES.store(0, Ordering::Relaxed);
    with_monitor(|m| {
        *m = Some(Monitor {
            started: Instant::now(),
            handle: None,
        });
    });
    MONITOR_ACTIVE.store(true, Ordering::Relaxed);
}

pub fn end_session() {
    MONITOR_ACTIVE.store(false, Ordering::Relaxed);
    PREWARM_DONE_BYTES.store(0, Ordering::Relaxed);
    PREWARM_TARGET_BYTES.store(0, Ordering::Relaxed);
    with_monitor(|m| *m = None);
}

fn attach_handle(handle: &Arc<ManagedTorrent>) {
    with_monitor(|m| {
        if let Some(monitor) = m.as_mut()
            && monitor.handle.is_none()
        {
            monitor.handle = Some(handle.clone());
        }
    });
}

fn note_served(bytes: u64) {
    if MONITOR_ACTIVE.load(Ordering::Relaxed) {
        SERVED_BYTES.fetch_add(bytes, Ordering::Relaxed);
    }
}

/// Current state of the monitored torrent, or `None` when nothing is watched.
pub fn status() -> Option<Status> {
    let (started, handle) = with_monitor(|m| m.as_ref().map(|m| (m.started, m.handle.clone())))?;
    let served_bytes = SERVED_BYTES.load(Ordering::Relaxed);

    let mut status = Status {
        phase: if ENGINE.get().is_none() {
            Phase::StartingEngine
        } else {
            Phase::FindingPeers
        },
        elapsed: started.elapsed(),
        peers: 0,
        seeders: 0,
        connecting: 0,
        download_bytes_per_sec: 0,
        downloaded_bytes: 0,
        served_bytes,
        prewarm_done_bytes: PREWARM_DONE_BYTES.load(Ordering::Relaxed),
        prewarm_target_bytes: PREWARM_TARGET_BYTES.load(Ordering::Relaxed),
    };

    if let Some(handle) = handle {
        status.phase = Phase::Buffering;
        let stats = handle.stats();
        status.downloaded_bytes = stats.progress_bytes;
        if let Some(live) = stats.live {
            let peers = &live.snapshot.peer_stats;
            status.peers = peers.live;
            status.seeders = peers.live_seeders;
            status.connecting = peers.connecting.saturating_add(peers.queued);
            status.download_bytes_per_sec = live.download_speed.as_bytes();
        }
    }
    if served_bytes >= PLAYING_AFTER_BYTES {
        status.phase = Phase::Playing;
    }
    Some(status)
}

/// Deletes downloaded pieces. Called on exit; the next start also clears them.
pub fn cleanup() {
    let _ = std::fs::remove_dir_all(crate::config::cache_dir().join("torrents"));
}

impl Engine {
    async fn handle_for(
        &self,
        magnet: &str,
        file_idx: Option<usize>,
    ) -> Result<Arc<ManagedTorrent>, String> {
        let key = info_hash_from_magnet(magnet).ok_or("Invalid magnet link")?;
        // Evict any previously added torrents with a different info-hash so
        // switching streams does not leave old swarms running in the session.
        let (cell, evicted) = {
            let mut map = self.torrents.lock().await;
            let mut old_cells = Vec::new();
            map.retain(|k, v| {
                if k == &key {
                    true
                } else {
                    old_cells.push(v.clone());
                    false
                }
            });
            let cell = map.entry(key).or_default().clone();
            (cell, old_cells)
        };
        for old_cell in evicted {
            if let Some(old_entry) = old_cell.get() {
                let _ = self
                    .session
                    .delete(old_entry.handle.id().into(), true)
                    .await;
            }
        }
        let entry = cell
            .get_or_try_init(|| async {
                let handle = self.add_and_init(magnet, file_idx).await?;
                Ok::<_, String>(Entry {
                    handle,
                    paused: AtomicBool::new(false),
                })
            })
            .await?;
        if entry.paused.swap(false, Ordering::Relaxed) {
            let _ = self.session.unpause(&entry.handle).await;
        }
        // The torrent is added for the first file requested. Another file of
        // the same torrent (the next episode of a season pack) needs no new
        // add: librqbit fetches the pieces of any file that is being streamed
        // with priority, whether or not it was selected up front.
        Ok(entry.handle.clone())
    }

    async fn pause_all(&self) {
        let cells: Vec<Arc<OnceCell<Entry>>> = {
            let map = self.torrents.lock().await;
            map.values().cloned().collect()
        };
        for cell in cells {
            if let Some(entry) = cell.get()
                && !entry.paused.swap(true, Ordering::Relaxed)
            {
                let _ = self.session.pause(&entry.handle).await;
            }
        }
    }

    async fn add_and_init(
        &self,
        magnet: &str,
        file_idx: Option<usize>,
    ) -> Result<Arc<ManagedTorrent>, String> {
        let opts = AddTorrentOptions {
            overwrite: true,
            only_files: file_idx.map(|i| vec![i]),
            ..Default::default()
        };
        log::info!("torrent engine: adding torrent, looking for peers");
        let started = std::time::Instant::now();
        // A single deadline for add + init, kept below the 100s playback timeout
        // so the user sees a real error instead of a generic one.
        let handle = tokio::time::timeout(METADATA_TIMEOUT, async {
            let added = self
                .session
                .add_torrent(AddTorrent::from_url(magnet.to_string()), Some(opts))
                .await
                .map_err(|e| format!("Cannot add torrent: {e}"))?;
            let handle = added
                .into_handle()
                .ok_or("Torrent engine returned no handle")?;
            handle
                .wait_until_initialized()
                .await
                .map_err(|e| format!("Torrent failed to start: {e}"))?;
            Ok::<_, String>(handle)
        })
        .await
        .map_err(|_| "Timed out finding peers for this torrent".to_string())??;
        log::info!(
            "torrent engine: torrent ready after {:?}",
            started.elapsed()
        );
        Ok(handle)
    }
}

fn files_of(handle: &Arc<ManagedTorrent>) -> Result<Vec<TorrentFile>, String> {
    handle
        .with_metadata(|m| {
            m.file_infos
                .iter()
                .enumerate()
                .map(|(i, fi)| TorrentFile {
                    id: i + 1,
                    path: fi.relative_filename.to_string_lossy().replace('\\', "/"),
                    length: fi.len,
                })
                .collect::<Vec<_>>()
        })
        .map_err(|e| format!("Torrent info unavailable: {e}"))
}

fn info_hash_from_magnet(magnet: &str) -> Option<String> {
    let rest = magnet.split("xt=urn:btih:").nth(1)?;
    let raw: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    crate::providers::addons::torrent::btih_to_hex(&raw)
}

fn content_type(path: &str) -> &'static str {
    match path
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("mkv") => "video/x-matroska",
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",
        Some("ts") => "video/mp2t",
        _ => "application/octet-stream",
    }
}

/// Parses a `Range` header into an inclusive `(start, end)`. `Ok(None)` means
/// "serve the whole file"; `Err(())` means unsatisfiable (HTTP 416).
fn parse_range(header: Option<&str>, total: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(spec) = header.and_then(|h| h.trim().strip_prefix("bytes=")) else {
        return Ok(None);
    };
    if total == 0 {
        return Err(());
    }
    let first = spec.split(',').next().unwrap_or("").trim();
    let (a, b) = first.split_once('-').ok_or(())?;
    let (start, end) = if a.is_empty() {
        let n: u64 = b.parse().map_err(|_| ())?;
        (total.saturating_sub(n), total - 1)
    } else {
        let start: u64 = a.parse().map_err(|_| ())?;
        let end = if b.is_empty() {
            total - 1
        } else {
            b.parse::<u64>().map_err(|_| ())?.min(total - 1)
        };
        (start, end)
    };
    if start >= total || start > end {
        return Err(());
    }
    Ok(Some((start, end)))
}

async fn serve(listener: TcpListener, engine: Arc<Engine>) {
    loop {
        match listener.accept().await {
            Ok((sock, _)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(sock, engine).await {
                        log::debug!("torrent stream connection ended: {e}");
                    }
                });
            }
            Err(e) => {
                log::warn!("torrent engine accept failed: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

async fn write_status(sock: &mut TcpStream, code: u16, text: &str) -> std::io::Result<()> {
    let body = format!("{code} {text}");
    let head = format!(
        "HTTP/1.1 {code} {text}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(head.as_bytes()).await
}

async fn handle_conn(mut sock: TcpStream, engine: Arc<Engine>) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();

    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        let n = sock.read(&mut chunk).await.map_err(io)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_REQUEST_HEAD {
            write_status(&mut sock, 431, "Request Header Fields Too Large")
                .await
                .map_err(io)?;
            return Ok(());
        }
    }

    let head = String::from_utf8_lossy(&buf).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let headers: Vec<(&str, &str)> = lines
        .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim(), v.trim())))
        .collect();
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| *v)
    };
    let range = header("range").map(str::to_string);
    // Reject requests aimed at another hostname (DNS-rebinding from a web
    // page): players always address this server as 127.0.0.1 or localhost.
    let port = crate::config::active_torrent_port();
    let host_ok = header("host").is_none_or(|h| {
        h == format!("127.0.0.1:{port}")
            || h == format!("localhost:{port}")
            || h.strip_prefix("127.0.0.1:")
                .or_else(|| h.strip_prefix("localhost:"))
                .and_then(|p| p.parse::<u16>().ok())
                .is_some_and(crate::config::is_local_torrent_port)
    });

    if method != "GET" && method != "HEAD" {
        write_status(&mut sock, 405, "Method Not Allowed")
            .await
            .map_err(io)?;
        return Ok(());
    }
    if !host_ok {
        write_status(&mut sock, 403, "Forbidden")
            .await
            .map_err(io)?;
        return Ok(());
    }
    let Ok(parsed) = url::Url::parse(&format!("http://localhost{target}")) else {
        write_status(&mut sock, 400, "Bad Request")
            .await
            .map_err(io)?;
        return Ok(());
    };
    if parsed.path() != "/stream" {
        write_status(&mut sock, 404, "Not Found")
            .await
            .map_err(io)?;
        return Ok(());
    }
    let mut magnet = None;
    let mut index = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "link" => magnet = Some(v.into_owned()),
            "index" => index = v.parse::<usize>().ok().map(|i| i.saturating_sub(1)),
            _ => {}
        }
    }
    let Some(magnet) = magnet.filter(|m| m.starts_with("magnet:")) else {
        write_status(&mut sock, 400, "Missing magnet link")
            .await
            .map_err(io)?;
        return Ok(());
    };

    log::info!("torrent engine: stream request received");
    let handle = match engine.handle_for(&magnet, index).await {
        Ok(h) => h,
        Err(e) => {
            log::warn!("torrent stream failed: {e}");
            write_status(&mut sock, 502, "Torrent unavailable")
                .await
                .map_err(io)?;
            return Ok(());
        }
    };
    attach_handle(&handle);
    let files = files_of(&handle)?;
    let file_idx = match index {
        Some(i) => i,
        None => {
            guess_file_id_with_hint(&files, magnet_display_name(&magnet).as_deref(), None, None)
                .map(|id| id - 1)
                .ok_or("No video file in torrent")?
        }
    };
    let Some(file) = files.get(file_idx) else {
        write_status(&mut sock, 404, "No such file in torrent")
            .await
            .map_err(io)?;
        return Ok(());
    };
    let total = file.length;
    log::info!("torrent engine: serving file {file_idx}, {total} bytes");

    let (start, end, partial) = match parse_range(range.as_deref(), total) {
        Ok(Some((s, e))) => (s, e, true),
        Ok(None) => (0, total.saturating_sub(1), false),
        Err(()) => {
            let head = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            sock.write_all(head.as_bytes()).await.map_err(io)?;
            return Ok(());
        }
    };
    let len = if total == 0 { 0 } else { end - start + 1 };

    let mut response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nAccept-Ranges: bytes\r\nContent-Length: {len}\r\nCache-Control: no-cache\r\nConnection: close\r\n",
        if partial {
            "206 Partial Content"
        } else {
            "200 OK"
        },
        content_type(&file.path),
    );
    if partial {
        response.push_str(&format!("Content-Range: bytes {start}-{end}/{total}\r\n"));
    }
    response.push_str("\r\n");
    sock.write_all(response.as_bytes()).await.map_err(io)?;
    if method == "HEAD" || len == 0 {
        return Ok(());
    }

    let stream = handle
        .clone()
        .stream(file_idx)
        .await
        .map_err(|e| format!("Cannot open torrent file: {e}"))?;
    let mut stream = Box::pin(stream);
    stream.seek(SeekFrom::Start(start)).await.map_err(io)?;

    let mut remaining = len;
    let mut buf = vec![0u8; COPY_BUF];
    while remaining > 0 {
        let want = remaining.min(buf.len() as u64) as usize;
        let n = stream.read(&mut buf[..want]).await.map_err(io)?;
        if n == 0 {
            break;
        }
        sock.write_all(&buf[..n]).await.map_err(io)?;
        note_served(n as u64);
        remaining -= n as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_parsing() {
        assert_eq!(parse_range(None, 100), Ok(None));
        assert_eq!(parse_range(Some("bytes=0-9"), 100), Ok(Some((0, 9))));
        assert_eq!(parse_range(Some("bytes=10-"), 100), Ok(Some((10, 99))));
        assert_eq!(parse_range(Some("bytes=-20"), 100), Ok(Some((80, 99))));
        assert_eq!(parse_range(Some("bytes=90-500"), 100), Ok(Some((90, 99))));
        assert_eq!(parse_range(Some("bytes=100-"), 100), Err(()));
        assert_eq!(parse_range(Some("bytes=abc"), 100), Err(()));
    }

    #[test]
    fn magnet_hash_extraction() {
        let m = "magnet:?xt=urn:btih:0123456789ABCDEF0123456789abcdef01234567&dn=x";
        assert_eq!(
            info_hash_from_magnet(m).as_deref(),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
        assert_eq!(info_hash_from_magnet("magnet:?dn=x"), None);
    }
}
