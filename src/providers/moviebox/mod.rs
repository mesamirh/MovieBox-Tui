pub mod adapt;
pub mod client;
pub mod crypto;
pub mod session;
pub mod title;

pub use title::clean_moviebox_title;

pub const STREAM_REFERER: &str = "https://sportslive.wine";
use crate::providers::models::{CatalogItem, MediaDetails, ProviderError, ProviderKind};
use crate::providers::{Provider, ProviderCapabilities};

impl From<ScraperError> for ProviderError {
    fn from(err: ScraperError) -> Self {
        match err {
            ScraperError::Reqwest(e) => ProviderError::Network(e.to_string()),
            ScraperError::ApiStatus(429) => ProviderError::RateLimited(None),
            ScraperError::ApiStatus(404) => ProviderError::NotFound,
            ScraperError::ApiStatus(s) => ProviderError::Unavailable(format!("HTTP status {s}")),
            ScraperError::HostsExhausted => {
                ProviderError::Unavailable("All hosts exhausted".to_string())
            }
            ScraperError::Json(e) => ProviderError::Parsing(e.to_string()),
            ScraperError::MissingToken => ProviderError::Unavailable("Missing token".to_string()),
        }
    }
}

impl Provider for client::MovieBoxClient {
    fn id(&self) -> ProviderKind {
        ProviderKind::MovieBox
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_search: true,
            supports_pagination: true,
            supports_series: true,
            supports_subtitles: true,
            supports_homepage: true,
        }
    }

    async fn search(&self, query: &str, page: usize) -> Result<Vec<CatalogItem>, ProviderError> {
        let json = self
            .search(query, page)
            .await
            .map_err(ProviderError::from)?;
        Ok(adapt::moviebox_search_json_to_catalog(&json))
    }

    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError> {
        let json = self.get_details(id).await.map_err(ProviderError::from)?;
        adapt::moviebox_details_json_to_media_details(&json)
    }
}

impl crate::providers::ReleaseProvider for client::MovieBoxClient {
    async fn episode_streams(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<crate::providers::models::Release>, ProviderError> {
        let resource_path = episode_resource_path(id, season, episode);
        let (play_info_res, resources_res) = tokio::join!(
            self.get_play_info(id, season, episode),
            self.get(&resource_path)
        );

        let mut releases = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        if let Ok(json) = play_info_res {
            for rel in adapt::moviebox_play_info_json_to_releases(
                &json,
                season,
                episode,
                self.user_agent(),
            ) {
                if let Some(url) = rel.direct_url() {
                    let base = url.split('?').next().unwrap_or(url).to_string();
                    if !base.is_empty() {
                        seen_urls.insert(base);
                    }
                }
                releases.push(rel);
            }
        }

        let items = match resources_res {
            Ok(payload) => payload
                .get("list")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            Err(_) if releases.is_empty() => {
                let page = episode.saturating_sub(1) / 20 + 1;
                self.fetch_resource_page(id, season, episode, 0, page)
                    .await
                    .map_err(ProviderError::from)?
                    .0
            }
            Err(_) => Vec::new(),
        };
        for item in items {
            let mut rel = adapt::moviebox_resource_item_to_release(&item);
            if (season != 0 || episode != 0)
                && (rel.season != Some(season) || rel.episode != Some(episode))
            {
                continue;
            }
            if rel.mirrors.is_empty() {
                // Both playback and resource APIs can return an upgrade clip.
                // Ask the resource failover API for the original file instead.
                let origin = item
                    .get("resourceLink")
                    .or_else(|| item.get("url"))
                    .and_then(Value::as_str)
                    .filter(|url| adapt::is_deprecation_notice_url(url));
                let (Some(resource_id), Some(origin)) = (rel.resource_id.as_deref(), origin) else {
                    continue;
                };
                let payload = match self.get_resource_position(id, resource_id, origin).await {
                    Ok(payload) => payload,
                    Err(error) => {
                        log::warn!("moviebox direct file recovery failed: {error}");
                        continue;
                    }
                };
                let Some(recovered) = adapt::moviebox_resource_position_json_to_release(
                    &payload,
                    &rel,
                    self.user_agent(),
                ) else {
                    continue;
                };
                rel = recovered;
            }
            if let Some(url) = rel.direct_url() {
                let base = url.split('?').next().unwrap_or(url).to_string();
                if !base.is_empty() && !seen_urls.insert(base) {
                    continue;
                }
            }
            releases.push(rel);
        }

        if releases.is_empty() {
            return Err(ProviderError::Unavailable(
                "No stream sources available".to_string(),
            ));
        }

        releases.sort_by(|left, right| {
            right
                .resolution_u64()
                .cmp(&left.resolution_u64())
                .then_with(|| right.size_bytes.cmp(&left.size_bytes))
        });

        Ok(releases)
    }
}

use client::{MovieBoxClient, ScraperError};
use serde_json::{Value, json};

impl MovieBoxClient {
    pub async fn get_resource_position(
        &self,
        subject_id: &str,
        resource_id: &str,
        failed_url: &str,
    ) -> Result<Value, ScraperError> {
        self.get(&resource_position_path(subject_id, resource_id, failed_url))
            .await
    }

    pub async fn get_play_info(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Value, ScraperError> {
        let path = if season == 0 && episode == 0 {
            format!(
                "/wefeed-mobile-bff/subject-api/play-info/v2?subjectId={}",
                subject_id
            )
        } else {
            format!(
                "/wefeed-mobile-bff/subject-api/play-info/v2?subjectId={}&se={}&ep={}",
                subject_id, season, episode
            )
        };
        self.get(&path).await
    }
    pub async fn search(&self, query: &str, page: usize) -> Result<Value, ScraperError> {
        let payload = json!({
            "keyword": query,
            "page": page,
            "perPage": 15,
            "subjectType": 0
        });
        self.post("/wefeed-mobile-bff/subject-api/search/v2", &payload)
            .await
    }

    pub async fn suggest(&self, query: &str) -> Result<Value, ScraperError> {
        self.search(query, 1).await
    }

    pub async fn get_details(&self, subject_id: &str) -> Result<Value, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/subject-api/get?subjectId={}",
            subject_id
        );
        let mut details = self.get(&path).await?;

        let stype = details
            .get("subjectType")
            .and_then(|s| s.as_i64())
            .or_else(|| details.get("stype").and_then(|s| s.as_i64()))
            .unwrap_or(1);

        if stype == 2 {
            let season_path = format!(
                "/wefeed-mobile-bff/subject-api/season-info?subjectId={}",
                subject_id
            );
            if let Ok(season_info) = self.get(&season_path).await {
                if let Value::Object(ref mut map) = details {
                    map.insert("seasons".to_string(), season_info);
                }
            }
        }

        Ok(details)
    }

    pub async fn get_homepage(&self, tab_id: &str, page: usize) -> Result<Value, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/tab-operating?page={}&tabId={}&version=",
            page, tab_id
        );
        self.get(&path).await
    }

    pub async fn get_resources(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
        page: usize,
        resolution: Option<&str>,
        per_page: usize,
    ) -> Result<Value, ScraperError> {
        let res_param = if let Some(r) = resolution {
            if r.is_empty() {
                String::new()
            } else {
                format!("&resolution={}", r)
            }
        } else {
            String::new()
        };

        let path = if season == 0 && episode == 0 {
            format!(
                "/wefeed-mobile-bff/subject-api/resource?subjectId={}&page={}&perPage={}{}",
                subject_id, page, per_page, res_param
            )
        } else {
            format!(
                "/wefeed-mobile-bff/subject-api/resource?subjectId={}&se={}&ep={}&page={}&perPage={}{}",
                subject_id, season, episode, page, per_page, res_param
            )
        };
        self.get(&path).await
    }

    pub async fn fetch_resource_page(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
        resolution: u32,
        page: usize,
    ) -> Result<(Vec<Value>, Value), ScraperError> {
        let path = resource_page_path(subject_id, season, episode, resolution, page);

        let res = self.get(&path).await?;

        let items = res
            .get("list")
            .and_then(|l| l.as_array())
            .cloned()
            .unwrap_or_default();

        let pager = res.get("pager").cloned().unwrap_or_else(|| json!({}));

        Ok((items, pager))
    }

    pub async fn fetch_collection_resolutions(
        &self,
        subject_id: &str,
    ) -> Result<Vec<u32>, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/subject-api/resource?subjectId={}&page=1&perPage=20",
            subject_id
        );
        let res = self.get(&path).await?;

        let mut resolutions = Vec::new();
        if let Some(cols) = res.get("collectionResolutions").and_then(|c| c.as_array()) {
            for col in cols {
                if let Some(r) = col.get("resolution").and_then(|v| v.as_u64()) {
                    if r > 0 {
                        resolutions.push(r as u32);
                    }
                }
            }
        }

        if resolutions.is_empty() {
            if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                for item in list {
                    if let Some(r) = item.get("resolution").and_then(|v| v.as_u64()) {
                        let res_u32 = r as u32;
                        if res_u32 > 0 && !resolutions.contains(&res_u32) {
                            resolutions.push(res_u32);
                        }
                    }
                }
            }
        }

        resolutions.sort_by(|a, b| b.cmp(a));

        if resolutions.is_empty() {
            resolutions = vec![1080, 720, 480, 360];
        }
        Ok(resolutions)
    }

    pub async fn get_ext_captions(
        &self,
        subject_id: &str,
        resource_id: &str,
    ) -> Result<Value, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/subject-api/get-ext-captions?subjectId={}&resourceId={}",
            subject_id, resource_id
        );
        self.get(&path).await
    }
}

fn resource_position_path(subject_id: &str, resource_id: &str, failed_url: &str) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("subjectId", subject_id)
        .append_pair("resourceId", resource_id)
        .append_pair("failUrl", failed_url)
        .append_pair("failCode", "404")
        .append_pair("resourceNum", "0")
        .append_pair("isVip", "true")
        .finish();
    format!("/wefeed-mobile-bff/subject-api/resource-position/v2?{query}")
}

fn episode_resource_path(subject_id: &str, season: usize, episode: usize) -> String {
    // The legacy resource listing ignores se/ep and paginates the entire series.
    format!(
        "/wefeed-mobile-bff/subject-api/resource/v2?subjectId={subject_id}&page=1&perPage=20&all=0&startPosition=1&endPosition=1&pagerMode=2&resolution=0&se={season}&epFrom={episode}&epTo={episode}&isVip=true"
    )
}

pub fn resource_page_path(
    subject_id: &str,
    season: usize,
    episode: usize,
    resolution: u32,
    page: usize,
) -> String {
    let res_param = if resolution == 0 {
        String::new()
    } else {
        format!("&resolution={}", resolution)
    };

    if season == 0 && episode == 0 {
        format!(
            "/wefeed-mobile-bff/subject-api/resource?subjectId={}&page={}&perPage=20{}",
            subject_id, page, res_param
        )
    } else {
        format!(
            "/wefeed-mobile-bff/subject-api/resource?subjectId={}&se={}&ep={}&page={}&perPage=20{}",
            subject_id, season, episode, page, res_param
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_resource_path_targets_exact_episode() {
        for (season, episode) in [(0, 0), (5, 1), (2, 25)] {
            let path = episode_resource_path("123", season, episode);
            let url = url::Url::parse(&format!("https://example.com{path}")).unwrap();
            let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
            assert_eq!(url.path(), "/wefeed-mobile-bff/subject-api/resource/v2");
            assert_eq!(params["subjectId"], "123");
            assert_eq!(params["se"], season.to_string());
            assert_eq!(params["epFrom"], episode.to_string());
            assert_eq!(params["epTo"], episode.to_string());
            assert_eq!(params["page"], "1");
        }
    }

    #[test]
    fn test_resource_position_path_encodes_failed_url() {
        let origin = "https://cdn.example.com/notice.mp4?sign=a+b&t=123";
        let path = resource_position_path("123", "456", origin);
        let url = url::Url::parse(&format!("https://example.com{path}")).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(
            url.path(),
            "/wefeed-mobile-bff/subject-api/resource-position/v2"
        );
        assert_eq!(params["subjectId"], "123");
        assert_eq!(params["resourceId"], "456");
        assert_eq!(params["failUrl"], origin);
        assert_eq!(params["failCode"], "404");
        assert_eq!(params["resourceNum"], "0");
        assert_eq!(params["isVip"], "true");
        assert_eq!(params.len(), 6);
    }

    #[test]
    fn test_resource_page_path_movie() {
        let path = resource_page_path("12345", 0, 0, 0, 1);
        assert_eq!(
            path,
            "/wefeed-mobile-bff/subject-api/resource?subjectId=12345&page=1&perPage=20"
        );
    }

    #[test]
    fn test_resource_page_path_series_with_se_and_ep() {
        let path = resource_page_path("67890", 2, 5, 0, 1);
        assert_eq!(
            path,
            "/wefeed-mobile-bff/subject-api/resource?subjectId=67890&se=2&ep=5&page=1&perPage=20"
        );
    }

    #[test]
    fn test_resource_page_path_with_resolution() {
        let path = resource_page_path("67890", 1, 10, 1080, 2);
        assert_eq!(
            path,
            "/wefeed-mobile-bff/subject-api/resource?subjectId=67890&se=1&ep=10&page=2&perPage=20&resolution=1080"
        );
    }
}
