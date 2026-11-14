//! Nuvio plugin support.
//!
//! A Nuvio plugin repository is a `manifest.json` with a `scrapers` list; each
//! scraper is a JavaScript file exposing `getStreams(tmdbId, mediaType, season,
//! episode)`. Scrapers are self-contained bundles that only need a browser-like
//! `fetch`, which Node.js provides, so each one is run by a small Node.js
//! script (`nuvio_runner.cjs`) that prints the streams as JSON.
//!
//! Every scraper of an installed repository becomes one entry of the addon
//! list (off by default, like in Nuvio), so the existing addon manager can
//! enable, disable and remove them. Such an entry is recognised by its
//! `manifest_url`: `nuvio:<script filename or URL>@<repository base URL>`.

use super::client::AddonClient;
use super::models::{InstalledAddon, StreamBehaviorHints, StreamItem};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// The text `AddonClient::fetch_manifest` fails with for a repository
/// manifest, so the installer can switch to the Nuvio path.
pub const REPO_MANIFEST_ERROR: &str =
    "This is a Nuvio plugin repository (JavaScript scrapers), not a Stremio addon.";

const URL_PREFIX: &str = "nuvio:";
/// Longest one scraper may take, start of Node.js included.
pub const SCRAPER_DEADLINE: Duration = Duration::from_secs(40);
/// A downloaded scraper is reused for this long before it is fetched again.
const SCRIPT_MAX_AGE: Duration = Duration::from_secs(6 * 60 * 60);
const RUNNER_JS: &str = include_str!("nuvio_runner.cjs");
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/124.0 Safari/537.36";

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    crate::net::http_client_builder_base()
        .timeout(Duration::from_secs(20))
        .user_agent(BROWSER_UA)
        .build()
        .unwrap_or_default()
});

/// IMDb id -> TMDB id, for this run of the program.
static TMDB_IDS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NuvioScraper {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    /// Script path relative to the repository, or a full URL.
    pub filename: String,
    /// Stremio type names this scraper supports (`movie`, `series`).
    pub types: Vec<String>,
}

/// Reads the `scrapers` list of a repository manifest, skipping entries that
/// have no script.
pub fn parse_repo(raw: &serde_json::Value) -> Result<Vec<NuvioScraper>, String> {
    let items = raw
        .get("scrapers")
        .and_then(|v| v.as_array())
        .ok_or("The manifest has no scrapers list")?;
    let mut scrapers = Vec::new();
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let str_field = |key: &str| {
            obj.get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let Some(filename) = str_field("filename") else {
            continue;
        };
        let name = str_field("name")
            .or_else(|| str_field("id"))
            .unwrap_or_else(|| filename.clone());
        let mut types: Vec<String> = Vec::new();
        for kind in obj
            .get("supportedTypes")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            let mapped = match kind.to_ascii_lowercase().as_str() {
                "movie" => "movie",
                "tv" | "series" | "anime" => "series",
                _ => continue,
            };
            if !types.iter().any(|t| t == mapped) {
                types.push(mapped.to_string());
            }
        }
        scrapers.push(NuvioScraper {
            name,
            description: str_field("description"),
            version: str_field("version"),
            filename,
            types,
        });
    }
    Ok(scrapers)
}

/// True when `raw` points directly at a standalone `.js` or `.mjs` Nuvio
/// scraper script rather than a repository or Stremio `manifest.json`.
pub fn is_direct_script_url(raw: &str) -> bool {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '<' | '>'))
        .trim();
    let path = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
    let lower = path.to_ascii_lowercase();
    (lower.ends_with(".js") || lower.ends_with(".mjs") || lower.ends_with(".cjs"))
        && !lower.ends_with("/manifest.json")
}

/// Normalizes a user-supplied Nuvio URL: converts bare GitHub repository links
/// (`https://github.com/<owner>/<repo>` or `.../tree/<branch>[/<dir>]`) or
/// `/blob/<branch>/...` links to `raw.githubusercontent.com` URLs before
/// normalizing to `/manifest.json`.
pub fn normalize_nuvio_url(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '<' | '>'))
        .trim()
        .trim_end_matches('/');
    if let Some(rest) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
            let owner = parts[0];
            let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
            let branch = parts[3];
            let tail = parts[4..].join("/");
            let raw_base = if tail.is_empty() {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}")
            } else {
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{tail}")
            };
            if is_direct_script_url(&raw_base) {
                return raw_base;
            }
            return AddonClient::normalize_manifest_url(&raw_base);
        } else if parts.len() == 2 {
            let owner = parts[0];
            let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
            return format!("https://raw.githubusercontent.com/{owner}/{repo}/main/manifest.json");
        }
    }
    if is_direct_script_url(trimmed) {
        if crate::net::is_http_url(trimmed) {
            return trimmed.to_string();
        }
        return format!("https://{trimmed}");
    }
    AddonClient::normalize_manifest_url(trimmed)
}

/// Downloads and validates a standalone `.js` Nuvio scraper URL, returning an
/// enabled [`InstalledAddon`] entry ready to run.
pub async fn fetch_single_scraper(raw_url: &str) -> Result<InstalledAddon, String> {
    let url = normalize_nuvio_url(raw_url);
    let resp = HTTP
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach scraper script: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Scraper script returned HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read scraper script: {e}"))?;
    if !body.contains("getStreams") {
        return Err("Script does not define getStreams(...)".to_string());
    }
    let clean_path = url.split(['?', '#']).next().unwrap_or(&url);
    let (repo_base, filename) = clean_path
        .rsplit_once('/')
        .map(|(b, f)| (b.to_string(), f.to_string()))
        .unwrap_or_else(|| (url.clone(), "scraper.js".to_string()));
    let stem = filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&filename)
        .replace(['-', '_'], " ");
    let title = stem
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let scraper = NuvioScraper {
        name: if title.is_empty() {
            filename.clone()
        } else {
            title
        },
        description: Some("Standalone Nuvio scraper".to_string()),
        version: Some("1.0.0".to_string()),
        filename: url.clone(),
        types: vec!["movie".to_string(), "series".to_string()],
    };
    let mut entry = installed_from_scraper(&repo_base, &scraper);
    entry.enabled = true;
    Ok(entry)
}

/// Downloads a repository manifest. Returns the repository base URL (the
/// manifest URL without `/manifest.json`) and its scrapers.
pub async fn fetch_repo(manifest_url: &str) -> Result<(String, Vec<NuvioScraper>), String> {
    let url = normalize_nuvio_url(manifest_url);
    let resp = HTTP
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach repository: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Repository returned HTTP {}", resp.status()));
    }
    let raw: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid repository JSON: {e}"))?;
    let scrapers = parse_repo(&raw)?;
    if scrapers.is_empty() {
        return Err("The repository lists no usable scrapers".to_string());
    }
    Ok((AddonClient::base_addon_url(&url), scrapers))
}

pub fn is_nuvio_addon(addon: &InstalledAddon) -> bool {
    addon.manifest_url.starts_with(URL_PREFIX)
}

/// One addon-list entry for a scraper. Off by default.
pub fn installed_from_scraper(repo_base: &str, scraper: &NuvioScraper) -> InstalledAddon {
    InstalledAddon {
        // '@' separates the script from the repository, so one inside the
        // script path (jsDelivr's `repo@branch`) is escaped.
        manifest_url: format!(
            "{URL_PREFIX}{}@{repo_base}",
            scraper.filename.replace('@', "%40")
        ),
        name: format!("{} [Nuvio]", scraper.name),
        version: scraper.version.clone(),
        description: scraper
            .description
            .clone()
            .or_else(|| Some("Nuvio scraper".to_string())),
        enabled: false,
        provides_catalog: false,
        provides_meta: false,
        provides_stream: true,
        id_prefixes: vec!["tt".to_string()],
        types: scraper.types.clone(),
        catalogs: Vec::new(),
    }
}

/// Adds a repository's scrapers to the addon list. Re-adding a repository
/// keeps the on/off choice the user already made for each scraper.
pub fn merge_repo(addons: &mut Vec<InstalledAddon>, repo_base: &str, scrapers: &[NuvioScraper]) {
    for scraper in scrapers {
        let mut entry = installed_from_scraper(repo_base, scraper);
        if let Some(pos) = addons
            .iter()
            .position(|a| a.manifest_url == entry.manifest_url)
        {
            entry.enabled = addons[pos].enabled;
            addons.remove(pos);
        }
        addons.push(entry);
    }
}

fn split_addon_url(manifest_url: &str) -> Option<(String, &str)> {
    let (filename, repo_base) = manifest_url.strip_prefix(URL_PREFIX)?.split_once('@')?;
    Some((filename.replace("%40", "@"), repo_base))
}

fn script_url(filename: &str, repo_base: &str) -> String {
    if filename.starts_with("http://") || filename.starts_with("https://") {
        filename.to_string()
    } else {
        format!(
            "{}/{}",
            repo_base.trim_end_matches('/'),
            filename.trim_start_matches('/')
        )
    }
}

fn cache_dir() -> PathBuf {
    crate::config::cache_dir().join("nuvio")
}

/// Stable file name for a URL (FNV-1a).
fn short_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

async fn ensure_runner(dir: &Path) -> Result<PathBuf, String> {
    let path = dir.join("nuvio_runner.cjs");
    let current = tokio::fs::read(&path).await.ok();
    if current.as_deref() != Some(RUNNER_JS.as_bytes()) {
        crate::cache::atomic_write_file_async(&path, RUNNER_JS.as_bytes())
            .await
            .map_err(|e| format!("Cannot write the Nuvio runner: {e}"))?;
    }
    Ok(path)
}

/// Returns the local copy of a scraper, downloading it when it is missing or
/// older than [`SCRIPT_MAX_AGE`]. A stale copy is used if the download fails.
async fn ensure_script(dir: &Path, url: &str) -> Result<PathBuf, String> {
    let path = dir.join(format!("{}.js", short_hash(url)));
    let fresh = tokio::fs::metadata(&path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < SCRIPT_MAX_AGE);
    if fresh {
        return Ok(path);
    }
    let downloaded = async {
        let resp = HTTP.get(url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let body = resp.bytes().await.map_err(|e| e.to_string())?;
        if body.is_empty() {
            return Err("the script is empty".to_string());
        }
        crate::cache::atomic_write_file_async(&path, &body)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    }
    .await;
    match downloaded {
        Ok(()) => Ok(path),
        Err(e) if path.exists() => {
            log::warn!("nuvio: using the cached scraper, refresh failed: {e}");
            Ok(path)
        }
        Err(e) => Err(format!("Cannot download the scraper: {e}")),
    }
}

async fn lookup_cinemeta_tmdb_id(imdb_id: &str, media_type: &str) -> Option<String> {
    let base = InstalledAddon::CINEMETA_MANIFEST.trim_end_matches("/manifest.json");
    let url = format!("{base}/meta/{media_type}/{imdb_id}.json");
    let raw: serde_json::Value = HTTP.get(&url).send().await.ok()?.json().await.ok()?;
    let id = raw.get("meta")?.get("moviedb_id")?;
    id.as_u64()
        .map(|n| n.to_string())
        .or_else(|| id.as_str().map(str::to_string))
}

async fn lookup_tmdb_find(imdb_id: &str, media_type: &str) -> Option<String> {
    let key = std::env::var("MOVIEBOX_TMDB_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())?;
    let url = format!(
        "https://api.themoviedb.org/3/find/{imdb_id}?external_source=imdb_id&api_key={key}"
    );
    let raw: serde_json::Value = HTTP.get(&url).send().await.ok()?.json().await.ok()?;
    let field = if media_type == "movie" {
        "movie_results"
    } else {
        "tv_results"
    };
    raw.get(field)?
        .as_array()?
        .first()?
        .get("id")?
        .as_u64()
        .map(|n| n.to_string())
}

/// Scrapers are keyed by TMDB id. Cinemeta usually knows it; a TMDB API key in
/// `MOVIEBOX_TMDB_API_KEY` is the fallback. When neither answers, the IMDb id
/// is passed on, which many scrapers can resolve themselves.
async fn resolve_tmdb_id(imdb_id: &str, media_type: &str) -> String {
    if !imdb_id.starts_with("tt") {
        return imdb_id.to_string();
    }
    let cache_key = format!("{media_type}:{imdb_id}");
    let cached = TMDB_IDS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&cache_key)
        .cloned();
    if let Some(hit) = cached {
        return hit;
    }
    let mut found = lookup_cinemeta_tmdb_id(imdb_id, media_type).await;
    if found.is_none() {
        found = lookup_tmdb_find(imdb_id, media_type).await;
    }
    match found {
        Some(id) => {
            TMDB_IDS
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(cache_key, id.clone());
            id
        }
        None => {
            log::warn!("nuvio: no TMDB id found for {imdb_id}, passing the IMDb id");
            imdb_id.to_string()
        }
    }
}

async fn run_node(
    runner: &Path,
    script: &Path,
    id: &str,
    media_type: &str,
    season: &str,
    episode: &str,
) -> Result<String, String> {
    let node = std::env::var("MOVIEBOX_NODE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "node".to_string());
    let mut cmd = tokio::process::Command::new(node);
    cmd.arg(runner)
        .arg(script)
        .arg(id)
        .arg(media_type)
        .arg(season)
        .arg(episode)
        .env(
            "NUVIO_TIMEOUT_MS",
            SCRAPER_DEADLINE
                .as_millis()
                .saturating_sub(3000)
                .to_string(),
        )
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Node.js not found. Install Node.js 18+ (or set MOVIEBOX_NODE)".to_string()
        } else {
            format!("Cannot start Node.js: {e}")
        }
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("no output")
            .chars()
            .take(160)
            .collect();
        return Err(format!(
            "The scraper printed nothing ({}): {tail}",
            output.status
        ));
    }
    Ok(stdout)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawStream {
    name: String,
    title: String,
    url: String,
    quality: String,
    size: String,
    headers: HashMap<String, String>,
}

/// Reads the runner's last output line: `{"streams":[...]}` or `{"error":"..."}`.
fn parse_runner_output(stdout: &str) -> Result<Vec<RawStream>, String> {
    let line = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .ok_or("The scraper printed nothing")?;
    let value: serde_json::Value =
        serde_json::from_str(line.trim()).map_err(|e| format!("Unreadable scraper output: {e}"))?;
    if let Some(err) = value.get("error").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    let list = value
        .get("streams")
        .and_then(|s| s.as_array())
        .ok_or("The scraper output has no streams list")?;
    let text = |v: &serde_json::Value, key: &str| -> String {
        v.get(key)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let mut streams = Vec::new();
    for item in list {
        let url = text(item, "url");
        if url.is_empty() {
            continue;
        }
        let mut headers = HashMap::new();
        if let Some(map) = item.get("headers").and_then(|h| h.as_object()) {
            for (key, val) in map {
                if let Some(val) = val.as_str() {
                    headers.insert(key.clone(), val.to_string());
                }
            }
        }
        streams.push(RawStream {
            name: text(item, "name"),
            title: text(item, "title"),
            url,
            quality: text(item, "quality"),
            size: text(item, "size"),
            headers,
        });
    }
    Ok(streams)
}

/// Shapes a scraper's stream like a Stremio stream, so quality, size, headers
/// and ranking go through the same code as every other addon stream.
fn to_stream_item(addon_name: &str, stream: RawStream) -> StreamItem {
    let heading = if !stream.title.is_empty() {
        stream.title.clone()
    } else if !stream.name.is_empty() {
        stream.name.clone()
    } else {
        format!("{addon_name} Stream")
    };
    let mut details = Vec::new();
    if !stream.quality.is_empty() {
        details.push(stream.quality.clone());
    }
    if !stream.size.is_empty() {
        details.push(stream.size.clone());
    }
    let title = if details.is_empty() {
        heading
    } else {
        format!("{heading}\n{}", details.join(" "))
    };
    let behavior_hints = if stream.headers.is_empty() {
        None
    } else {
        Some(StreamBehaviorHints {
            headers: Some(stream.headers),
            ..Default::default()
        })
    };
    StreamItem {
        name: Some(if stream.name.is_empty() {
            addon_name.to_string()
        } else {
            stream.name
        }),
        title: Some(title),
        url: Some(stream.url),
        behavior_hints,
        ..Default::default()
    }
}

/// Runs one installed Nuvio scraper for a title. `media_type` is the Stremio
/// name (`movie` or `series`); `imdb_id` is the bare IMDb id.
pub async fn fetch_streams(
    addon: &InstalledAddon,
    imdb_id: &str,
    media_type: &str,
    season: usize,
    episode: usize,
) -> Result<Vec<StreamItem>, String> {
    if imdb_id.is_empty() {
        return Ok(Vec::new());
    }
    if !addon.types.is_empty() && !addon.types.iter().any(|t| t == media_type) {
        return Ok(Vec::new());
    }
    let (filename, repo_base) =
        split_addon_url(&addon.manifest_url).ok_or("Invalid Nuvio scraper entry")?;
    let dir = cache_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Cannot create the Nuvio cache folder: {e}"))?;
    let runner = ensure_runner(&dir).await?;
    let script = ensure_script(&dir, &script_url(&filename, repo_base)).await?;
    let tmdb_id = resolve_tmdb_id(imdb_id, media_type).await;
    let (nuvio_type, season_arg, episode_arg) = if media_type == "movie" {
        ("movie", "-".to_string(), "-".to_string())
    } else if season > 0 && episode > 0 {
        ("tv", season.to_string(), episode.to_string())
    } else {
        ("tv", "-".to_string(), "-".to_string())
    };
    let stdout = run_node(
        &runner,
        &script,
        &tmdb_id,
        nuvio_type,
        &season_arg,
        &episode_arg,
    )
    .await?;
    let streams = parse_runner_output(&stdout)?;
    Ok(streams
        .into_iter()
        .map(|s| to_stream_item(&addon.name, s))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_repo() -> serde_json::Value {
        serde_json::json!({
            "name": "All-in-One-Nuvio",
            "version": "1.0.0",
            "scrapers": [
                {"id": "vidrock", "name": "🪨 VidRock", "version": "1.0.0",
                 "supportedTypes": ["movie", "tv"], "filename": "providers/vidrock.js"},
                {"id": "movies4u", "supportedTypes": ["movie"], "filename": "providers/movies4u.js"},
                {"id": "anikoto", "name": "Anikoto", "supportedTypes": ["anime", "Movie", "odd"],
                 "filename": "https://example.test/anikoto.js"},
                {"id": "nofile", "name": "No file"},
                "junk"
            ]
        })
    }

    #[test]
    fn repo_scrapers_are_read_and_mapped() {
        let scrapers = parse_repo(&sample_repo()).unwrap();
        assert_eq!(scrapers.len(), 3);
        assert_eq!(scrapers[0].types, ["movie", "series"]);
        assert_eq!(scrapers[1].name, "movies4u");
        assert_eq!(scrapers[1].types, ["movie"]);
        assert_eq!(scrapers[2].types, ["series", "movie"]);
        assert!(parse_repo(&serde_json::json!({"name": "x"})).is_err());
    }

    #[test]
    fn scraper_entries_round_trip_and_start_off() {
        let scrapers = parse_repo(&sample_repo()).unwrap();
        let base = "https://raw.example.test/repo/main";
        let entry = installed_from_scraper(base, &scrapers[0]);
        assert!(!entry.enabled);
        assert!(entry.provides_stream && !entry.provides_catalog && !entry.provides_meta);
        assert!(is_nuvio_addon(&entry));
        let (filename, repo) = split_addon_url(&entry.manifest_url).unwrap();
        assert_eq!(filename, "providers/vidrock.js");
        assert_eq!(repo, base);
        assert_eq!(
            script_url(&filename, repo),
            "https://raw.example.test/repo/main/providers/vidrock.js"
        );
        assert_eq!(
            script_url("https://example.test/anikoto.js", base),
            "https://example.test/anikoto.js"
        );
        let plain = InstalledAddon::cinemeta_default();
        assert!(!is_nuvio_addon(&plain));
    }

    #[test]
    fn normalizes_github_and_direct_script_urls() {
        assert_eq!(
            normalize_nuvio_url("https://github.com/tapframe/nuvio-providers"),
            "https://raw.githubusercontent.com/tapframe/nuvio-providers/main/manifest.json"
        );
        assert_eq!(
            normalize_nuvio_url("https://github.com/user/repo/tree/dev/src"),
            "https://raw.githubusercontent.com/user/repo/dev/src/manifest.json"
        );
        assert_eq!(
            normalize_nuvio_url("https://github.com/user/repo/blob/main/providers/vidrock.js"),
            "https://raw.githubusercontent.com/user/repo/main/providers/vidrock.js"
        );
        assert!(is_direct_script_url(
            "https://example.test/providers/vidrock.js"
        ));
        assert!(!is_direct_script_url("https://example.test/manifest.json"));
    }

    #[test]
    fn at_sign_in_script_url_survives_the_round_trip() {
        let scraper = NuvioScraper {
            name: "cdn".to_string(),
            description: None,
            version: None,
            filename: "https://cdn.jsdelivr.net/gh/user/repo@main/p.js".to_string(),
            types: Vec::new(),
        };
        let base = "https://cdn.jsdelivr.net/gh/user/repo@main";
        let entry = installed_from_scraper(base, &scraper);
        let (filename, repo) = split_addon_url(&entry.manifest_url).unwrap();
        assert_eq!(filename, scraper.filename);
        assert_eq!(repo, base);
    }

    #[test]
    fn re_adding_a_repository_keeps_the_users_choice() {
        let scrapers = parse_repo(&sample_repo()).unwrap();
        let base = "https://raw.example.test/repo/main";
        let mut addons = vec![InstalledAddon::cinemeta_default()];
        merge_repo(&mut addons, base, &scrapers);
        assert_eq!(addons.len(), 4);
        let url = installed_from_scraper(base, &scrapers[0]).manifest_url;
        addons
            .iter_mut()
            .find(|a| a.manifest_url == url)
            .unwrap()
            .enabled = true;
        merge_repo(&mut addons, base, &scrapers);
        assert_eq!(addons.len(), 4);
        assert!(
            addons
                .iter()
                .find(|a| a.manifest_url == url)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn runner_output_is_parsed() {
        let out = "noise line\n{\"streams\":[{\"name\":\"V\",\"title\":\"Movie\",\"url\":\"https://a.test/v.m3u8\",\"quality\":\"1080p\",\"size\":\"2 GB\",\"headers\":{\"Referer\":\"https://a.test\",\"X\":1}},{\"url\":\"\"}]}\n";
        let streams = parse_runner_output(out).unwrap();
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].headers.len(), 1);
        let err = parse_runner_output("{\"error\":\"site is down\"}\n").unwrap_err();
        assert_eq!(err, "site is down");
        assert!(parse_runner_output("").is_err());
        assert!(parse_runner_output("not json").is_err());
    }

    #[test]
    fn scraper_streams_look_like_stremio_streams() {
        let raw = RawStream {
            name: "VidRock".to_string(),
            title: "Movie 2024".to_string(),
            url: "https://a.test/v.m3u8".to_string(),
            quality: "1080p".to_string(),
            size: "2.1 GB".to_string(),
            headers: HashMap::from([("Referer".to_string(), "https://a.test".to_string())]),
        };
        let item = to_stream_item("VidRock [Nuvio]", raw);
        let release = super::super::adapter::stream_item_to_release("VidRock [Nuvio]", &item, 0, 0)
            .expect("release");
        assert_eq!(release.quality.as_deref(), Some("1080p"));
        assert_eq!(release.mirrors[0].headers.len(), 1);
        assert!(release.filename.contains("Movie"));
    }

    #[tokio::test]
    async fn a_scraper_runs_end_to_end_when_node_is_installed() {
        if std::process::Command::new("node")
            .arg("-v")
            .output()
            .is_err()
        {
            return;
        }
        let dir = std::env::temp_dir().join(format!("moviebox_nuvio_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let runner = dir.join("nuvio_runner.cjs");
        std::fs::write(&runner, RUNNER_JS).unwrap();
        let script = dir.join("scraper.js");
        std::fs::write(
            &script,
            "module.exports = { getStreams: async (id, type, s, e) => [{ name: 'T', \
             title: id + ' ' + type + ' ' + s + ' ' + e, url: 'https://a.test/v.mp4', quality: '720p' }] };",
        )
        .unwrap();
        let out = run_node(&runner, &script, "603", "tv", "1", "2")
            .await
            .unwrap();
        let streams = parse_runner_output(&out).unwrap();
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].title, "603 tv 1 2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
