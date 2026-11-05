use std::time::Duration;
use thiserror::Error;

use crate::providers::models::{
    CatalogItem, Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release, Season,
    SourceMirror,
};

use super::parser::{CircleFtpSearchResponse, circleftp_search_to_catalog};

#[derive(Debug, Error)]
pub enum CircleFtpError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

type PostCache =
    std::sync::Arc<std::sync::Mutex<Option<(String, std::time::Instant, serde_json::Value)>>>;

#[derive(Clone)]
pub struct CircleFtpClient {
    client: reqwest::Client,
    base_url: String,
    post_cache: PostCache,
}

impl Default for CircleFtpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub const BASE_URL: &str = "http://new.circleftp.net:5000";
pub const API_URL: &str = "http://new.circleftp.net:5000/api";
pub const POSTS_URL: &str = "http://new.circleftp.net:5000/api/posts";
pub const UPLOADS_URL: &str = "http://new.circleftp.net:5000/uploads/";

impl CircleFtpClient {
    pub fn new() -> Self {
        Self {
            client: build_client(),
            base_url: API_URL.to_string(),
            post_cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    async fn fetch_post(&self, id: &str) -> Result<serde_json::Value, CircleFtpError> {
        if let Ok(lock) = self.post_cache.lock()
            && let Some((cached_id, ts, val)) = lock.as_ref()
            && cached_id == id
            && ts.elapsed() < Duration::from_secs(60)
        {
            return Ok(val.clone());
        }
        let url = format!("{}/posts/{}", self.base_url, id);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let json: serde_json::Value = resp.json().await?;
        if let Ok(mut lock) = self.post_cache.lock() {
            *lock = Some((id.to_string(), std::time::Instant::now(), json.clone()));
        }
        Ok(json)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<CatalogItem>, CircleFtpError> {
        let mut url = reqwest::Url::parse(&format!("{}/posts", self.base_url))
            .map_err(|e| CircleFtpError::Parse(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("searchTerm", query)
            .append_pair("order", "desc");

        let resp = self.client.get(url).send().await?.error_for_status()?;

        let search_resp: CircleFtpSearchResponse = resp.json().await?;
        Ok(circleftp_search_to_catalog(&search_resp))
    }

    pub async fn details(&self, id: &str) -> Result<MediaDetails, CircleFtpError> {
        let json = self.fetch_post(id).await?;

        let title = json
            .get("title")
            .or(json.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let r#type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let media_type = if r#type == "series" {
            MediaType::Series
        } else {
            MediaType::Movie
        };
        let year = json.get("year").and_then(|v| {
            if v.is_number() {
                Some(v.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        });

        let poster_url = json
            .get("image")
            .or(json.get("imageSm"))
            .and_then(|v| v.as_str())
            .map(|s| format!("{UPLOADS_URL}{}", s));

        let mut seasons = Vec::new();
        if media_type == MediaType::Series {
            if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                for (s_idx, s_val) in content.iter().enumerate() {
                    let mut episodes = Vec::new();
                    if let Some(ep_arr) = s_val.get("episodes").and_then(|v| v.as_array()) {
                        for (e_idx, e_val) in ep_arr.iter().enumerate() {
                            let ep_title = e_val
                                .get("title")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            episodes.push(Episode {
                                season: s_idx + 1,
                                number: e_idx + 1,
                                title: ep_title,
                                overview: None,
                            });
                        }
                    }
                    seasons.push(Season {
                        number: s_idx + 1,
                        episodes,
                    });
                }
            }
        }

        let description = json
            .get("metaData")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let duration = json
            .get("watchTime")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut genres = Vec::new();
        if let Some(cats) = json.get("categories").and_then(|v| v.as_array()) {
            for cat in cats {
                if let Some(name) = cat.get("name").and_then(|v| v.as_str()) {
                    genres.push(name.to_string());
                }
            }
        }

        let title_full = json.get("title").and_then(|v| v.as_str()).unwrap_or("");

        let t_lower = title_full.to_lowercase();

        let audios = [
            ("hindi", "Hindi"),
            ("bengali", "Bengali"),
            ("bangla", "Bengali"),
            ("english", "English"),
            ("tamil", "Tamil"),
            ("telugu", "Telugu"),
            ("malayalam", "Malayalam"),
            ("korean", "Korean"),
            ("dual audio", "Dual Audio"),
            ("multi audio", "Multi Audio"),
        ]
        .iter()
        .find(|(k, _)| t_lower.contains(k))
        .map(|(_, v)| v.to_string());

        let prints = [
            ("cam", "CAM"),
            ("hdcam", "CAM"),
            ("hdtc", "HDTC"),
            ("tc", "HDTC"),
            ("hdrip", "HDRip"),
            ("hd rip", "HDRip"),
            ("webrip", "WEBRip"),
            ("web-rip", "WEBRip"),
            ("webdl", "WEB-DL"),
            ("web-dl", "WEB-DL"),
            ("bluray", "BluRay"),
            ("brrip", "BluRay"),
        ]
        .iter()
        .find(|(k, _)| t_lower.contains(k))
        .map(|(_, v)| v.to_string());

        Ok(MediaDetails {
            id: ProviderMediaId {
                provider: ProviderKind::BdixCircleFtp,
                value: id.to_string(),
            },
            title,
            media_type,
            year,
            description,
            tagline: None,
            imdb_rating: None,
            director: None,
            stars: None,
            prints,
            audios,
            poster_url,
            duration,
            genres,
            seasons,
            dubs: vec![],
        })
    }

    async fn fetch_size(&self, link: &str) -> Option<u64> {
        tokio::time::timeout(Duration::from_millis(1200), self.client.head(link).send())
            .await
            .ok()?
            .ok()?
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)?
            .to_str()
            .ok()?
            .parse::<u64>()
            .ok()
    }

    pub async fn releases(
        &self,
        id: &str,
        season: Option<usize>,
        episode: Option<usize>,
    ) -> Result<Vec<Release>, CircleFtpError> {
        let json = self.fetch_post(id).await?;

        let mut releases = Vec::new();
        let r#type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

        let quality_str = json.get("quality").and_then(|v| v.as_str()).unwrap_or("HD");
        let quality = crate::providers::bdix::common::detect_resolution(quality_str)
            .or_else(|| Some(quality_str.to_string()));

        let title_str = json.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let codec = crate::providers::bdix::common::detect_codec(title_str);
        let language = crate::providers::bdix::common::detect_audio_language(title_str);

        if r#type == "series" {
            if let (Some(target_s), Some(target_e)) = (season, episode) {
                if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
                    if let Some(s_val) = content.get(target_s.saturating_sub(1)) {
                        if let Some(ep_arr) = s_val.get("episodes").and_then(|v| v.as_array()) {
                            if let Some(e_val) = ep_arr.get(target_e.saturating_sub(1)) {
                                if let Some(link) = e_val.get("link").and_then(|v| v.as_str()) {
                                    let filename = link
                                        .split('/')
                                        .next_back()
                                        .unwrap_or("Video File")
                                        .replace("%20", " ")
                                        .replace("%28", "(")
                                        .replace("%29", ")")
                                        .replace("%5B", "[")
                                        .replace("%5D", "]");
                                    let size_bytes = self.fetch_size(link).await;

                                    releases.push(Release {
                                        provider: ProviderKind::BdixCircleFtp,
                                        filename,
                                        quality: quality.clone(),
                                        codec: codec.clone(),
                                        language: language.clone(),
                                        size_bytes,
                                        season: Some(target_s),
                                        episode: Some(target_e),
                                        mirrors: vec![SourceMirror {
                                            label: "CircleFTP".to_string(),
                                            resolver_url: link.to_string(),
                                            headers: Vec::new(),
                                            direct_file: true,
                                        }],
                                        resource_id: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else {
            if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
                let filename = content
                    .split('/')
                    .next_back()
                    .unwrap_or("Video File")
                    .replace("%20", " ")
                    .replace("%28", "(")
                    .replace("%29", ")")
                    .replace("%5B", "[")
                    .replace("%5D", "]");
                let size_bytes = self.fetch_size(content).await;

                releases.push(Release {
                    provider: ProviderKind::BdixCircleFtp,
                    filename,
                    quality: quality.clone(),
                    codec: codec.clone(),
                    language: language.clone(),
                    size_bytes,
                    season: None,
                    episode: None,
                    mirrors: vec![SourceMirror {
                        label: "CircleFTP".to_string(),
                        resolver_url: content.to_string(),
                        headers: Vec::new(),
                        direct_file: true,
                    }],
                    resource_id: None,
                });
            }
        }

        Ok(releases)
    }
}

fn build_client() -> reqwest::Client {
    crate::net::http_client_builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default()
}
