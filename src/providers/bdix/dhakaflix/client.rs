use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::providers::models::{
    CatalogItem, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release,
};

#[derive(Debug, Error)]
pub enum DhakaFlixError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

fn parse_title_and_year(raw_title: &str) -> (String, Option<String>) {
    let mut title = raw_title.to_string();
    let mut year = None;

    if let Some(start) = title.rfind('(') {
        if let Some(rel_end) = title.get(start..).and_then(|s| s.find(')')) {
            let end = start + rel_end;
            if let (Some(year_slice), Some(title_slice)) =
                (title.get(start + 1..end), title.get(..start))
            {
                let extracted = crate::providers::models::extract_4digit_year(year_slice);
                if extracted.len() == 4 && year_slice.trim().len() == 4 {
                    year = Some(extracted);
                    title = title_slice.trim().to_string();
                    return (title, year);
                }
            }
        }
    }

    let lower = title.to_lowercase();
    let qualities = [
        " 1080p", " 720p", " 480p", " 2160p", " 4k", " hd", " hdrip", " webrip", " hdcam",
    ];
    for q in qualities {
        if lower.ends_with(q) {
            let trim_len = title.len().saturating_sub(q.len());
            if let Some(prefix) = title.get(..trim_len) {
                title = prefix.trim().to_string();
            }
            break;
        }
    }

    (title, year)
}

fn quality_score(name: &str) -> u8 {
    let lower = name.to_lowercase();
    if lower.contains("2160p") || lower.contains("4k") {
        3
    } else if lower.contains("1080p") {
        2
    } else if lower.contains("720p") {
        1
    } else {
        0
    }
}

pub const SERVERS: &[(&str, &str)] = &[
    ("http://172.16.50.7", "/DHAKA-FLIX-7/"),
    ("http://172.16.50.14", "/DHAKA-FLIX-14/"),
    ("http://172.16.50.12", "/DHAKA-FLIX-12/"),
    ("http://172.16.50.9", "/DHAKA-FLIX-9/"),
];

const DEAD_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct DhakaFlixClient {
    client: Client,
    recent_fails: Arc<RwLock<HashMap<String, Instant>>>,
}

impl Default for DhakaFlixClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DhakaFlixClient {
    pub fn new() -> Self {
        Self {
            client: crate::net::http_client_builder()
                .timeout(Duration::from_secs(5))
                .user_agent(crate::net::DEFAULT_BROWSER_USER_AGENT)
                .build()
                .unwrap_or_default(),
            recent_fails: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn healthy_servers(&self) -> Vec<(&'static str, &'static str)> {
        let fails = self.recent_fails.read().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        for (base, path) in SERVERS {
            if fails
                .get(*base)
                .is_none_or(|failed| failed.elapsed() >= DEAD_TTL)
            {
                out.push((*base, *path));
            }
        }
        out
    }

    fn dedup_best_quality(results: Vec<(CatalogItem, u8)>) -> Vec<CatalogItem> {
        let mut best: HashMap<(String, Option<String>), (CatalogItem, u8)> = HashMap::new();
        for (item, quality) in results {
            let key = (item.title.to_lowercase(), item.year.clone());
            match best.get(&key) {
                Some((_, existing)) if *existing >= quality => {}
                _ => {
                    best.insert(key, (item, quality));
                }
            }
        }
        let mut out: Vec<CatalogItem> = best.into_values().map(|(item, _)| item).collect();
        out.sort_by_key(|item| item.title.to_lowercase());
        out
    }

    pub async fn search(&self, query: &str) -> Result<Vec<CatalogItem>, DhakaFlixError> {
        let servers = self.healthy_servers();
        let mut futures = Vec::new();
        for (base_url, href) in servers {
            let url = format!("{}{}", base_url, href);
            let body = json!({
                "action": "get",
                "search": {
                    "href": href,
                    "pattern": query,
                    "ignorecase": true
                }
            });
            let client = self.client.clone();
            let recent_fails = self.recent_fails.clone();

            futures.push(async move {
                let mut server_results: Vec<(CatalogItem, u8)> = Vec::new();
                match client.post(&url).json(&body).send().await {
                    Ok(resp) => {
                        if let Ok(text) = resp.text().await {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(search_arr) =
                                    json.get("search").and_then(|v| v.as_array())
                                {
                                    for item in search_arr {
                                        if let Some(item_href) =
                                            item.get("href").and_then(|v| v.as_str())
                                        {
                                            let is_dir = item_href.ends_with('/');
                                            let mut parts: Vec<&str> = item_href
                                                .split('/')
                                                .filter(|p| !p.is_empty())
                                                .collect();

                                            if !is_dir && parts.len() > 1 {
                                                parts.pop();
                                            }

                                            let folder_path = format!("/{}/", parts.join("/"));

                                            if let Some(name) = parts.last() {
                                                let name_decoded =
                                                    percent_encoding::percent_decode_str(name)
                                                        .decode_utf8_lossy()
                                                        .to_string();
                                                let quality = quality_score(&name_decoded);
                                                let (clean_title, year) =
                                                    parse_title_and_year(&name_decoded);

                                                server_results.push((
                                                    CatalogItem {
                                                        id: ProviderMediaId {
                                                            provider: ProviderKind::BdixDhakaFlix,
                                                            value: format!(
                                                                "{}:{}{}",
                                                                base_url,
                                                                href,
                                                                folder_path
                                                                    .trim_start_matches(href)
                                                            ),
                                                        },
                                                        title: clean_title,
                                                        year,
                                                        media_type: MediaType::Movie,
                                                        poster_url: None,
                                                        season_count: None,
                                                    },
                                                    quality,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        log::warn!(
                            "dhakaflix search request failed: {error} [{}]",
                            crate::logging::sanitize_url(base_url)
                        );
                        if let Ok(mut fails) = recent_fails.write() {
                            fails.insert(base_url.to_string(), Instant::now());
                        }
                    }
                }
                server_results
            });
        }

        let results_array = futures::future::join_all(futures).await;
        let all_results: Vec<(CatalogItem, u8)> = results_array.into_iter().flatten().collect();
        let mut all_results = Self::dedup_best_quality(all_results);

        let item_ids: Vec<(usize, String)> = all_results
            .iter()
            .enumerate()
            .take(20)
            .map(|(idx, item)| (idx, item.id.value.clone()))
            .collect();
        let poster_stream = futures::stream::iter(item_ids.into_iter().map(|(idx, id)| {
            let client = self.client.clone();

            async move {
                if let Some((base_url, path)) = parse_dhakaflix_id(&id) {
                    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
                    if !path_parts.is_empty() {
                        let api_href = format!("/{}/", path_parts[0]);
                        let api_url = format!("{}{}", base_url, api_href);

                        let body = json!({
                            "action": "get",
                            "items": {
                                "href": path,
                                "what": 1
                            }
                        });

                        if let Ok(resp) = client.post(&api_url).json(&body).send().await {
                            if let Ok(json) = resp.json::<serde_json::Value>().await {
                                if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
                                    for file in items {
                                        if let Some(href) =
                                            file.get("href").and_then(|v| v.as_str())
                                        {
                                            let href_lower = href.to_lowercase();
                                            if href_lower.ends_with(".jpg")
                                                || href_lower.ends_with(".jpeg")
                                                || href_lower.ends_with(".png")
                                            {
                                                return (
                                                    idx,
                                                    Some(format!("{}{}", base_url, href)),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                (idx, None)
            }
        }));

        let posters: Vec<(usize, Option<String>)> =
            poster_stream.buffer_unordered(6).collect().await;
        for (idx, poster) in posters {
            if let Some(item) = all_results.get_mut(idx) {
                item.poster_url = poster;
            }
        }
        Ok(all_results)
    }

    pub async fn details(&self, id: &str) -> Result<MediaDetails, DhakaFlixError> {
        let mut title = "Unknown".to_string();
        if let Some((_, path)) = parse_dhakaflix_id(id) {
            let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
            if let Some(name) = path_parts.last() {
                title = percent_encoding::percent_decode_str(name)
                    .decode_utf8_lossy()
                    .to_string();
            }
        }

        let (clean_title, year) = parse_title_and_year(&title);

        Ok(MediaDetails {
            id: ProviderMediaId {
                provider: ProviderKind::BdixDhakaFlix,
                value: id.to_string(),
            },
            title: clean_title,
            media_type: MediaType::Movie,
            year,
            description: None,
            tagline: None,
            imdb_rating: None,
            director: None,
            stars: None,
            prints: None,
            audios: None,
            poster_url: None,
            duration: None,
            genres: vec![],
            seasons: vec![],
            dubs: vec![],
        })
    }

    pub async fn streams(&self, id: &str) -> Result<Vec<Release>, DhakaFlixError> {
        let mut releases = Vec::new();

        let Some((base_url, path)) = parse_dhakaflix_id(id) else {
            return Ok(releases);
        };
        let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

        if path_parts.last().is_some() {
            let mut api_href = "/";
            for (s_base, s_href) in SERVERS {
                if base_url == *s_base {
                    api_href = *s_href;
                    break;
                }
            }

            let api_url = format!("{}{}", base_url, api_href);
            let body = json!({
                "action": "get",
                "items": {
                    "href": path,
                    "what": 1
                }
            });

            let resp = self
                .client
                .post(&api_url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(items_arr) = json.get("items").and_then(|v| v.as_array()) {
                    for item in items_arr {
                        if let (Some(item_href), Some(size)) = (
                            item.get("href").and_then(|v| v.as_str()),
                            item.get("size").and_then(|v| v.as_u64()),
                        ) {
                            let filename = item_href
                                .split('/')
                                .next_back()
                                .unwrap_or("Unknown")
                                .to_string();
                            let filename_decoded = percent_encoding::percent_decode_str(&filename)
                                .decode_utf8_lossy()
                                .to_string();

                            let f_lower = filename_decoded.to_lowercase();
                            if !f_lower.ends_with(".mkv")
                                && !f_lower.ends_with(".mp4")
                                && !f_lower.ends_with(".avi")
                                && !f_lower.ends_with(".webm")
                            {
                                continue;
                            }

                            let quality =
                                crate::providers::bdix::common::detect_resolution(&f_lower)
                                    .or_else(|| Some("HD".to_string()));
                            let language =
                                crate::providers::bdix::common::detect_audio_language(&f_lower);
                            let codec = crate::providers::bdix::common::detect_codec(&f_lower);

                            releases.push(Release {
                                provider: ProviderKind::BdixDhakaFlix,
                                filename: filename_decoded,
                                quality,
                                codec,
                                language,
                                size_bytes: Some(size),
                                season: None,
                                episode: None,
                                mirrors: vec![crate::providers::models::SourceMirror {
                                    label: "DhakaFlix".to_string(),
                                    resolver_url: format!("{}{}", base_url, item_href),
                                    headers: vec![],
                                    direct_file: true,
                                }],
                                resource_id: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(releases)
    }
}

fn parse_dhakaflix_id(id: &str) -> Option<(&str, &str)> {
    if let Some(pos) = id.rfind(":/") {
        Some((&id[..pos], &id[pos + 1..]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dhakaflix_id_with_port() {
        let (base, path) =
            parse_dhakaflix_id("http://172.16.50.4:8080:/movies/action/film.mkv").unwrap();
        assert_eq!(base, "http://172.16.50.4:8080");
        assert_eq!(path, "/movies/action/film.mkv");
    }

    #[test]
    fn test_parse_dhakaflix_id_without_port() {
        let (base, path) =
            parse_dhakaflix_id("http://172.16.50.4:/movies/action/film.mkv").unwrap();
        assert_eq!(base, "http://172.16.50.4");
        assert_eq!(path, "/movies/action/film.mkv");
    }

    #[tokio::test]
    async fn test_dhakaflix_streams_extracts_and_parses_video_releases() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"items":[{"href":"/Movies/Inception.2010.2160p.HEVC.Dual.mkv","size":4294967296},{"href":"/Movies/notes.txt","size":128}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let client = DhakaFlixClient::new();
        let id = format!("http://{addr}:/Movies/Inception/");
        let releases = client.streams(&id).await.expect("streams");
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].filename, "Inception.2010.2160p.HEVC.Dual.mkv");
        assert_eq!(releases[0].quality.as_deref(), Some("4K"));
        assert_eq!(releases[0].codec.as_deref(), Some("HEVC"));
        assert_eq!(releases[0].language.as_deref(), Some("Dual Audio"));
        assert_eq!(releases[0].size_bytes, Some(4_294_967_296));
    }
}
