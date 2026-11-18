//! Torrent helpers for addon streams: smart file selection, peer-source hints
//! and lazy preparation of the built-in torrent engine (`crate::torrent_engine`).

use super::adapter::{parse_season_episode, torrent_gateway_url};
use super::models::StreamItem;

/// Well-known public trackers added to every magnet so peer discovery starts
/// quickly even when the addon only supplies a few (or no) trackers.
pub const DEFAULT_TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.stealth.si:80/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://tracker.tiny-vps.com:6969/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "udp://open.tracker.cl:1337/announce",
    "udp://explodie.org:6969/announce",
    "udp://tracker.dler.org:6969/announce",
    "udp://opentracker.i2p.rocks:6969/announce",
    "udp://tracker.bittor.pw:1337/announce",
    "udp://tracker.filemail.com:6969/announce",
    "https://tracker.tamersunion.org:443/announce",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "avi", "mp4", "m4v", "wmv", "mov", "mpg", "mpeg", "ts", "webm",
];

/// Seeder count shown in a stream's text (`👤 12` in Torrentio, `Seeders: 12`
/// elsewhere). `None` when the addon does not say.
pub fn stream_seeders(stream: &StreamItem) -> Option<u32> {
    let text = format!(
        "{} {} {}",
        stream.name.as_deref().unwrap_or_default(),
        stream.title.as_deref().unwrap_or_default(),
        stream.description.as_deref().unwrap_or_default()
    );
    parse_seeders(&text)
}

pub fn parse_seeders(text: &str) -> Option<u32> {
    fn leading_number(rest: &str) -> Option<u32> {
        let digits: String = rest
            .trim_start_matches([':', '=', ' ', '\t'])
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    }
    if let Some((_, rest)) = text.split_once('👤') {
        return leading_number(rest);
    }
    let lower = text.to_ascii_lowercase();
    for key in ["seeders", "seeder", "seeds"] {
        if let Some((_, rest)) = lower.split_once(key) {
            return leading_number(rest);
        }
    }
    None
}

/// Screen recordings and other pre-release rips (CAM, telesync, screener…).
/// They carry a "1080p" label now and then but look far worse than a normal
/// release, so they are ranked below everything else.
pub fn is_low_quality_source(name: &str) -> bool {
    const MARKERS: &[&str] = &[
        "cam",
        "hdcam",
        "camrip",
        "hdts",
        "telesync",
        "tc",
        "hdtc",
        "telecine",
        "scr",
        "screener",
        "workprint",
    ];
    name.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| MARKERS.contains(&token))
}

/// A file inside a torrent. `id` is the one-based file number (the `index`
/// query parameter of engine stream URLs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFile {
    pub id: usize,
    pub path: String,
    pub length: u64,
}

fn is_video_file(path: &str) -> bool {
    path.rsplit_once('.')
        .map(|(_, ext)| VIDEO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Port of enginefs `guessFileIdx`: among video files, prefer the one that
/// matches the requested season/episode, otherwise take the largest file.
/// Sample clips are ignored when anything else is available.
pub fn guess_file_id(
    files: &[TorrentFile],
    season: Option<usize>,
    episode: Option<usize>,
) -> Option<usize> {
    let mut media: Vec<&TorrentFile> = files.iter().filter(|f| is_video_file(&f.path)).collect();
    if media.is_empty() {
        return None;
    }
    let without_samples: Vec<&TorrentFile> = media
        .iter()
        .copied()
        .filter(|f| !f.path.to_ascii_lowercase().contains("sample"))
        .collect();
    if !without_samples.is_empty() {
        media = without_samples;
    }

    let matching: Vec<&TorrentFile> = match (season, episode) {
        (Some(s), Some(e)) if s > 0 && e > 0 => media
            .iter()
            .copied()
            .filter(|f| parse_season_episode(file_name(&f.path)) == Some((s, e)))
            .collect(),
        _ => Vec::new(),
    };

    let pool = if matching.is_empty() {
        &media
    } else {
        &matching
    };
    let mut best: Option<&TorrentFile> = None;
    for f in pool.iter().copied() {
        if best.is_none_or(|b| f.length > b.length) {
            best = Some(f);
        }
    }
    best.map(|f| f.id)
}

/// Like [`guess_file_id`], but first honours a filename hint (the addon's
/// `behaviorHints.filename`, carried in the magnet's `dn`): an exact path or
/// exact file-name match wins over every heuristic. Falls back to the
/// heuristics when the hint matches nothing (e.g. `dn` is only a title).
pub fn guess_file_id_with_hint(
    files: &[TorrentFile],
    hint: Option<&str>,
    season: Option<usize>,
    episode: Option<usize>,
) -> Option<usize> {
    if let Some(hint) = hint.map(str::trim).filter(|h| !h.is_empty()) {
        let hint = hint.replace('\\', "/").to_ascii_lowercase();
        let by_path = files
            .iter()
            .find(|f| f.path.replace('\\', "/").to_ascii_lowercase() == hint);
        let by_name = || {
            files
                .iter()
                .find(|f| file_name(&f.path).to_ascii_lowercase() == file_name(&hint))
        };
        if let Some(f) = by_path.or_else(by_name) {
            return Some(f.id);
        }
    }
    guess_file_id(files, season, episode)
}

/// Display name (`dn`) of a magnet link, if any.
pub fn magnet_display_name(magnet: &str) -> Option<String> {
    let parsed = url::Url::parse(magnet).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == "dn")
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.trim().is_empty())
}

/// Converts the `btih` of a magnet link to 40-char lowercase hex. Accepts the
/// hex form and the 32-char base32 form some addons emit.
pub fn btih_to_hex(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() == 40 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(raw.to_ascii_lowercase());
    }
    if raw.len() != 32 {
        return None;
    }
    let mut bits: u64 = 0;
    let mut nbits = 0u32;
    let mut out: Vec<u8> = Vec::with_capacity(20);
    for c in raw.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a',
            b'2'..=b'7' => c - b'2' + 26,
            _ => return None,
        } as u64;
        bits = (bits << 5) | v;
        nbits += 5;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
            bits &= (1u64 << nbits) - 1;
        }
    }
    (out.len() == 20).then(|| out.iter().map(|b| format!("{b:02x}")).collect())
}

/// Rebuilds a `magnet:` URI that an addon put in a stream's `url` field so it
/// can be played by the built-in engine. The hash is normalised to hex, the
/// magnet's own trackers are kept, the addon's `sources` and the default
/// trackers are added. Name priority: addon filename hint, the magnet's `dn`,
/// then `title_fallback`.
pub fn normalize_magnet(
    raw: &str,
    filename_hint: Option<&str>,
    title_fallback: Option<&str>,
    extra_sources: &[String],
) -> Option<String> {
    let parsed = url::Url::parse(raw.trim()).ok()?;
    if parsed.scheme() != "magnet" {
        return None;
    }
    let mut hash = None;
    let mut dn = None;
    let mut sources: Vec<String> = Vec::new();
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "xt" if hash.is_none() => {
                let lower = v.to_ascii_lowercase();
                if let Some(h) = lower.strip_prefix("urn:btih:") {
                    hash = btih_to_hex(h);
                }
            }
            "dn" => dn = Some(v.into_owned()),
            "tr" => sources.push(v.into_owned()),
            _ => {}
        }
    }
    let hash = hash?;
    sources.extend(extra_sources.iter().cloned());
    let name = filename_hint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or(dn.filter(|s| !s.trim().is_empty()))
        .or(title_fallback
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string));
    Some(super::adapter::build_magnet_uri(
        &hash,
        name.as_deref(),
        &sources,
    ))
}

/// The parts of an engine stream URL needed to prepare playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayLink {
    pub magnet: String,
    /// Zero-based file index, if the URL already selects a file.
    pub file_idx: Option<usize>,
    /// Optional season/episode hint carried for season-pack downloads.
    pub season: Option<usize>,
    pub episode: Option<usize>,
}

/// Parses a stream URL of the built-in engine. Anything else (including a
/// look-alike URL on another host) is not a torrent stream.
pub fn parse_gateway_url(raw: &str) -> Option<GatewayLink> {
    let parsed = url::Url::parse(raw).ok()?;
    let is_local_engine = parsed.scheme() == "http"
        && parsed.host_str() == Some("127.0.0.1")
        && parsed
            .port()
            .is_some_and(crate::config::is_local_torrent_port);
    if !is_local_engine || parsed.path() != "/stream" {
        return None;
    }
    let mut magnet = None;
    let mut index = None;
    let mut season = None;
    let mut episode = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "link" => magnet = Some(v.into_owned()),
            "index" => index = v.parse::<usize>().ok(),
            "season" => season = v.parse::<usize>().ok().filter(|&n| n > 0),
            "episode" => episode = v.parse::<usize>().ok().filter(|&n| n > 0),
            _ => {}
        }
    }
    let magnet = magnet.filter(|m| m.starts_with("magnet:"))?;
    Some(GatewayLink {
        magnet,
        file_idx: index.map(|i| i.saturating_sub(1)),
        season,
        episode,
    })
}

pub fn is_gateway_stream_url(raw: &str) -> bool {
    parse_gateway_url(raw).is_some()
}

/// Attaches `season` and `episode` query parameters to a gateway stream URL
/// that lacks an explicit `index`, so `download()` selects the requested
/// episode from a season pack rather than falling back to the largest file.
pub fn with_episode_hint(raw: &str, season: Option<usize>, episode: Option<usize>) -> String {
    let (Some(s), Some(e)) = (season.filter(|&n| n > 0), episode.filter(|&n| n > 0)) else {
        return raw.to_string();
    };
    let Some(link) = parse_gateway_url(raw) else {
        return raw.to_string();
    };
    if link.file_idx.is_some() || (link.season.is_some() && link.episode.is_some()) {
        return raw.to_string();
    }
    if let Some(stripped) = raw.strip_suffix("&play") {
        format!("{stripped}&season={s}&episode={e}&play")
    } else {
        format!("{raw}&season={s}&episode={e}")
    }
}

/// Prepares an engine stream URL for playback: the torrent is added, its
/// metadata resolved, and the file's head + container tail pre-warmed so the
/// player starts against warm pieces. When the URL names no file, the right
/// one is picked (filename hint, then the requested episode, then the largest
/// video) and the URL is rewritten to the active engine port.
pub async fn prepare_playback_url(
    url: &str,
    season: Option<usize>,
    episode: Option<usize>,
) -> Result<String, String> {
    let Some(link) = parse_gateway_url(url) else {
        return Ok(url.to_string());
    };
    log::info!("torrent: preparing playback");

    let season = season.or(link.season);
    let episode = episode.or(link.episode);
    let files = crate::torrent_engine::prepare(&link.magnet, link.file_idx).await?;
    let target_idx = match link.file_idx {
        Some(idx) => idx,
        None => {
            let hint = magnet_display_name(&link.magnet);
            let id = guess_file_id_with_hint(&files, hint.as_deref(), season, episode)
                .ok_or_else(|| "No video file found in this torrent".to_string())?;
            id.saturating_sub(1)
        }
    };
    crate::torrent_engine::prewarm_file(&link.magnet, target_idx).await;
    torrent_gateway_url(
        &crate::config::torrent_engine_base(),
        &link.magnet,
        Some(target_idx),
    )
    .ok_or_else(|| "Could not build torrent stream URL".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(id: usize, path: &str, length: u64) -> TorrentFile {
        TorrentFile {
            id,
            path: path.to_string(),
            length,
        }
    }

    #[test]
    fn parses_seeder_counts() {
        assert_eq!(
            parse_seeders("Movie.2024.1080p\n👤 128 💾 2.1 GB ⚙️ YTS"),
            Some(128)
        );
        assert_eq!(parse_seeders("👤 0 💾 1 GB"), Some(0));
        assert_eq!(parse_seeders("Seeders: 7 | Size: 1 GB"), Some(7));
        assert_eq!(parse_seeders("Movie.2024.1080p 2.1 GB"), None);
    }

    #[test]
    fn flags_cam_and_telesync_releases() {
        assert!(is_low_quality_source("Movie.2024.1080p.HDCAM.x264"));
        assert!(is_low_quality_source("Movie 2024 TELESYNC 720p"));
        assert!(is_low_quality_source("Movie.2024.CAM.mkv"));
        assert!(!is_low_quality_source("Movie.2024.1080p.BluRay.x265.mkv"));
        // plain words that merely contain the letters must not match
        assert!(!is_low_quality_source(
            "Cameron.Documentary.2024.1080p.WEB-DL"
        ));
        // `.ts` is a container extension, not a telesync
        assert!(!is_low_quality_source("Show.S01E01.1080p.ts"));
    }

    #[test]
    fn picks_requested_episode_from_season_pack() {
        let files = vec![
            f(1, "Show.S01/Show.S01E01.1080p.mkv", 2_000),
            f(2, "Show.S01/Show.S01E02.1080p.mkv", 1_000),
            f(3, "Show.S01/Show.S01E03.1080p.mkv", 3_000),
            f(4, "Show.S01/info.nfo", 10),
        ];
        assert_eq!(guess_file_id(&files, Some(1), Some(2)), Some(2));
    }

    #[test]
    fn falls_back_to_largest_video_and_skips_samples() {
        let files = vec![
            f(1, "Movie/Sample/sample.mkv", 9_000),
            f(2, "Movie/Movie.2024.1080p.mkv", 4_000),
            f(3, "Movie/Movie.2024.720p.mkv", 2_000),
            f(4, "Movie/readme.txt", 99_999),
        ];
        assert_eq!(guess_file_id(&files, None, None), Some(2));
        // an episode that is not in the pack falls back to the largest video
        assert_eq!(guess_file_id(&files, Some(1), Some(5)), Some(2));
    }

    #[test]
    fn magnet_includes_default_trackers() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        let magnet = super::super::adapter::build_magnet_uri(hash, None, &[]);
        assert!(magnet.contains("opentrackr"));
        assert_eq!(magnet.matches("&tr=").count(), DEFAULT_TRACKERS.len());
    }

    #[test]
    fn filename_hint_beats_heuristics() {
        let files = vec![
            f(1, "Pack/Show.S01E01.mkv", 9_000),
            f(2, "Pack/Show.S01E02.mkv", 1_000),
        ];
        // exact file name (case-insensitive) wins over "largest"
        assert_eq!(
            guess_file_id_with_hint(&files, Some("show.s01e02.MKV"), None, None),
            Some(2)
        );
        // a title-only hint matches nothing -> heuristics
        assert_eq!(
            guess_file_id_with_hint(&files, Some("Show Season 1"), None, None),
            Some(1)
        );
    }

    #[test]
    fn reads_magnet_display_name() {
        let m = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=Show%20S01E02.mkv";
        assert_eq!(magnet_display_name(m).as_deref(), Some("Show S01E02.mkv"));
        assert_eq!(magnet_display_name("magnet:?xt=urn:btih:abc"), None);
    }

    #[test]
    fn no_video_files_returns_none() {
        let files = vec![f(1, "a.txt", 5), f(2, "b.nfo", 6)];
        assert_eq!(guess_file_id(&files, None, None), None);
    }

    #[test]
    fn parses_and_rebuilds_engine_urls() {
        let base = crate::config::torrent_engine_base();
        let magnet = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
        let url = torrent_gateway_url(&base, magnet, None).unwrap();
        let link = parse_gateway_url(&url).expect("engine url");
        assert_eq!(link.magnet, magnet);
        assert_eq!(link.file_idx, None);

        let with_idx = torrent_gateway_url(&base, magnet, Some(2)).unwrap();
        assert_eq!(parse_gateway_url(&with_idx).unwrap().file_idx, Some(2));

        let hinted = with_episode_hint(&url, Some(1), Some(4));
        let parsed_hint = parse_gateway_url(&hinted).expect("hinted engine url");
        assert_eq!(parsed_hint.season, Some(1));
        assert_eq!(parsed_hint.episode, Some(4));

        assert!(!is_gateway_stream_url("https://cdn.example.com/video.mp4"));
    }

    #[test]
    fn ignores_lookalike_urls_on_other_hosts() {
        let magnet = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
        let remote = torrent_gateway_url("http://evil.example:11490", magnet, None).unwrap();
        assert!(!is_gateway_stream_url(&remote));
    }

    #[test]
    fn btih_accepts_hex_and_base32() {
        let hex = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(btih_to_hex(hex).as_deref(), Some(hex));
        assert_eq!(btih_to_hex(&hex.to_uppercase()).as_deref(), Some(hex));
        assert_eq!(
            btih_to_hex("AERUKZ4JVPG66AJDIVTYTK6N54ASGRLH").as_deref(),
            Some(hex)
        );
        assert_eq!(btih_to_hex("not-a-hash"), None);
    }

    #[test]
    fn normalizes_magnet_from_url_field() {
        let raw = "magnet:?xt=urn:btih:AERUKZ4JVPG66AJDIVTYTK6N54ASGRLH&dn=Show%20S01E02&tr=udp%3A%2F%2Ftracker.example%3A1337%2Fannounce&so=1";
        let m = normalize_magnet(raw, None, Some("Title"), &[]).expect("magnet");
        assert!(m.starts_with("magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"));
        assert!(m.contains("dn=Show%20S01E02"));
        assert!(m.contains("tracker%2Eexample"));
        assert!(m.contains("opentrackr"));
        // filename hint wins over dn
        let m = normalize_magnet(raw, Some("Show.S01E02.mkv"), None, &[]).unwrap();
        assert!(m.contains("dn=Show%2ES01E02%2Emkv"));
        assert!(normalize_magnet("magnet:?dn=x", None, None, &[]).is_none());
        assert!(normalize_magnet("https://x.test/a", None, None, &[]).is_none());
    }
}
