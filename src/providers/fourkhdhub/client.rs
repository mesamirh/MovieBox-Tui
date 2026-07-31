use super::{hubcloud, parser};
use crate::providers::models::{CatalogItem, MediaDetails, PlaybackSource, ProviderKind, Release};
use reqwest::Url;

const DEFAULT_BASE_URL: &str = "https://4khdhub.one/";

#[derive(thiserror::Error, Debug)]
pub enum FourKHdHubError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid provider URL: {0}")]
    InvalidUrl(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("no playable mirror resolved")]
    NoPlayableMirror,
}

#[derive(Clone)]
pub struct FourKHdHubClient {
    client: reqwest::Client,
    base_url: Url,
}

impl Default for FourKHdHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FourKHdHubClient {
    pub fn new() -> Self {
        let base = std::env::var("MOVIEBOX_FOURKHDHUB_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::with_base_url(&base).unwrap_or_else(|_| Self {
            client: build_client(),
            base_url: Url::parse(DEFAULT_BASE_URL).expect("valid default 4KHDHub URL"),
        })
    }

    pub fn with_base_url(base: &str) -> Result<Self, FourKHdHubError> {
        let base_url =
            Url::parse(base).map_err(|_| FourKHdHubError::InvalidUrl(base.to_string()))?;
        if base_url.scheme() != "https" {
            return Err(FourKHdHubError::InvalidUrl(base.to_string()));
        }
        Ok(Self {
            client: build_client(),
            base_url,
        })
    }

    pub async fn health_check(&self) -> Result<(), FourKHdHubError> {
        let response = self.client.get(self.base_url.clone()).send().await?;
        if !response.status().is_success() {
            return Err(FourKHdHubError::Parse(format!(
                "health check returned {}",
                response.status()
            )));
        }
        Ok(())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<CatalogItem>, FourKHdHubError> {
        // Try several query shapes — the site's WordPress search is picky and
        // often returns zero (or unrelated) hits for multi-word titles.
        let mut by_id: std::collections::HashMap<String, CatalogItem> =
            std::collections::HashMap::new();
        let mut last_err: Option<FourKHdHubError> = None;

        for variant in search_query_variants(query) {
            match self.search_once(&variant).await {
                Ok(items) => {
                    for item in items {
                        by_id.entry(item.id.value.clone()).or_insert(item);
                    }
                }
                Err(e) => last_err = Some(e),
            }
        }

        if by_id.is_empty() {
            if let Some(err) = last_err {
                return Err(err);
            }
            return Ok(Vec::new());
        }
        Ok(by_id.into_values().collect())
    }

    async fn search_once(&self, query: &str) -> Result<Vec<CatalogItem>, FourKHdHubError> {
        let mut url = self.base_url.clone();
        url.query_pairs_mut().clear().append_pair("s", query);
        let html = self.fetch_text(url).await?;
        parser::parse_search(&self.base_url, &html)
    }

    pub async fn details(&self, id: &str) -> Result<MediaDetails, FourKHdHubError> {
        let url = self.provider_url(id)?;
        let html = self.fetch_text(url).await?;
        parser::parse_details(id, &html)
    }

    pub async fn releases(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<Release>, FourKHdHubError> {
        let url = self.provider_url(id)?;
        let html = self.fetch_text(url).await?;
        parser::parse_releases(&html, season, episode)
    }

    pub async fn resolve_release(
        &self,
        release: &Release,
    ) -> Result<PlaybackSource, FourKHdHubError> {
        if release.provider != ProviderKind::FourKHdHub {
            return Err(FourKHdHubError::Parse(
                "release belongs to another provider".into(),
            ));
        }
        for mirror in &release.mirrors {
            let candidates = if mirror.resolver_url.contains("hubcloud.") {
                hubcloud::resolve(&self.client, &mirror.resolver_url).await
            } else if mirror.resolver_url.contains("hubdrive.") {
                hubcloud::resolve_hubdrive(&self.client, &mirror.resolver_url).await
            } else {
                hubcloud::validate_playback_url(&mirror.resolver_url)
                    .map(|url| vec![(url, mirror.label.clone(), mirror.headers.clone())])
            };
            if let Ok(candidates) = candidates {
                for (url, label, headers) in candidates {
                    if let Ok(playable_url) = self.preflight(&url, &headers).await {
                        return Ok(PlaybackSource {
                            provider: ProviderKind::FourKHdHub,
                            url: playable_url,
                            headers,
                            subtitle: None,
                            source_label: label,
                        });
                    }
                }
            }
        }
        Err(FourKHdHubError::NoPlayableMirror)
    }

    async fn preflight(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<String, FourKHdHubError> {
        hubcloud::validate_playback_url(url)?;
        let mut request = self
            .client
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send().await?.error_for_status()?;
        let mut final_url = response.url().clone();
        hubcloud::validate_playback_url(final_url.as_str())?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if content_type.contains("text/html")
            || content_type.contains("application/zip")
            || content_type.contains("text/plain")
        {
            let wrapped = final_url
                .query_pairs()
                .find(|(name, _)| name == "link")
                .map(|(_, value)| value.into_owned())
                .filter(|value| value.starts_with("https://"))
                .ok_or_else(|| {
                    FourKHdHubError::Parse(format!("invalid media content type: {content_type}"))
                })?;
            hubcloud::validate_playback_url(&wrapped)?;
            let mut wrapped_request = self
                .client
                .get(&wrapped)
                .header(reqwest::header::RANGE, "bytes=0-0");
            for (name, value) in headers {
                wrapped_request = wrapped_request.header(name, value);
            }
            let wrapped_response = wrapped_request.send().await?.error_for_status()?;
            final_url = wrapped_response.url().clone();
            hubcloud::validate_playback_url(final_url.as_str())?;
            let wrapped_type = wrapped_response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if wrapped_type.contains("text/html")
                || wrapped_type.contains("application/zip")
                || wrapped_type.contains("text/plain")
            {
                return Err(FourKHdHubError::Parse(format!(
                    "invalid wrapped media content type: {wrapped_type}"
                )));
            }
        }
        Ok(final_url.to_string())
    }

    async fn fetch_text(&self, url: Url) -> Result<String, FourKHdHubError> {
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.text().await?)
    }

    fn provider_url(&self, id: &str) -> Result<Url, FourKHdHubError> {
        let url = self
            .base_url
            .join(id.trim_start_matches('/'))
            .map_err(|_| FourKHdHubError::InvalidUrl(id.to_string()))?;
        if url.host_str() != self.base_url.host_str() {
            return Err(FourKHdHubError::InvalidUrl(id.to_string()));
        }
        Ok(url)
    }
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(5))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        )
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_default()
}

/// Alternate strings to try against WordPress `?s=` search.
fn search_query_variants(query: &str) -> Vec<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<String> = Vec::new();
    let mut push = |s: String| {
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if !s.is_empty()
            && !out
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&s))
        {
            out.push(s);
        }
    };

    push(trimmed.to_string());

    // Drop very common words that dilute WP search.
    let stop = ["the", "a", "an", "of", "and", "or", "to", "in", "on"];
    let significant: Vec<&str> = trimmed
        .split_whitespace()
        .filter(|w| {
            let lower = w.to_ascii_lowercase();
            !stop.contains(&lower.as_str()) && w.len() >= 2
        })
        .collect();
    if !significant.is_empty() {
        push(significant.join(" "));
    }

    // Hyphenated slug form.
    push(trimmed.to_ascii_lowercase().replace(' ', "-"));

    // Quoted-ish exact: site sometimes ranks better without punctuation.
    let alnum_spaces: String = trimmed
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    push(alnum_spaces);

    // Longest single token as last resort (recall boost; relevance filter culls noise).
    if let Some(longest) = significant.iter().max_by_key(|w| w.len()) {
        if longest.len() >= 4 {
            push((*longest).to_string());
        }
    }

    out.truncate(5);
    out
}
