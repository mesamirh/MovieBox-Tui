use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::models::BrowseMetrics;
use crate::providers::Provider;
use crate::providers::bdix::circleftp::CircleFtpClient;
use crate::providers::bdix::dhakaflix::client::DhakaFlixClient;
use crate::providers::fourkhdhub::FourKHdHubClient;
use crate::providers::models::{CatalogItem, MediaDetails, ProviderError, ProviderKind};
use crate::providers::moviebox::client::MovieBoxClient;

#[derive(Clone)]
pub struct MovieBoxService {
    pub client: MovieBoxClient,
    pub fourk_client: Option<FourKHdHubClient>,
    pub circleftp_client: CircleFtpClient,
    pub dhakaflix_client: DhakaFlixClient,
    pub addon_client: crate::providers::addons::AddonClient,
    pub dramachi_client: crate::providers::dramachi::DramachiClient,
    pub opensubtitles: crate::providers::opensubtitles::OpenSubtitlesClient,
    pub http_client: reqwest::Client,
}

impl Default for MovieBoxService {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieBoxService {
    pub fn new() -> Self {
        let http_client = crate::net::http_client_builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self {
            client: MovieBoxClient::new(),
            fourk_client: FourKHdHubClient::new().ok(),
            circleftp_client: CircleFtpClient::new(),
            dhakaflix_client: DhakaFlixClient::new(),
            addon_client: crate::providers::addons::AddonClient::new(),
            dramachi_client: crate::providers::dramachi::DramachiClient::new(),
            opensubtitles: crate::providers::opensubtitles::OpenSubtitlesClient::new(),
            http_client,
        }
    }

    pub fn capabilities(&self, provider: ProviderKind) -> crate::providers::ProviderCapabilities {
        match provider {
            ProviderKind::MovieBox => Provider::capabilities(&self.client),
            ProviderKind::FourKHdHub => self
                .fourk_client
                .as_ref()
                .map(Provider::capabilities)
                .unwrap_or_default(),
            ProviderKind::BdixCircleFtp => Provider::capabilities(&self.circleftp_client),
            ProviderKind::BdixDhakaFlix => Provider::capabilities(&self.dhakaflix_client),
            ProviderKind::Addons => Provider::capabilities(&self.addon_client),
            ProviderKind::Dramachi => Provider::capabilities(&self.dramachi_client),
        }
    }

    pub async fn suggest(&self, query: &str) -> Result<Vec<String>, String> {
        let payload = self
            .client
            .suggest(query)
            .await
            .map_err(|e| e.to_string())?;
        let suggestions =
            crate::providers::moviebox::adapt::moviebox_suggest_json_to_strings(&payload);
        Ok(crate::language_filter::hindi_suggestions(
            suggestions,
            crate::config::load().hide_hindi,
        ))
    }

    /// Spec S6b: Hindi catalog entries leave every listing page.
    ///
    /// The rule runs here, in the core funnel both clients call, so every
    /// provider and both front ends are covered by one place instead of a
    /// per-client copy. The page-size guard inside
    /// [`crate::language_filter::hindi_catalog_items`] keeps an all-Hindi page
    /// non-empty, so a client that ends pagination on an empty page still asks
    /// for the next one.
    fn hide_hindi_catalog(&self, items: Vec<CatalogItem>) -> Vec<CatalogItem> {
        crate::language_filter::hindi_catalog_items(items, crate::config::load().hide_hindi)
    }

    pub async fn search_typed(
        &self,
        provider: ProviderKind,
        query: &str,
        page: usize,
    ) -> Result<Vec<CatalogItem>, ProviderError> {
        let items = match provider {
            ProviderKind::MovieBox => Provider::search(&self.client, query, page).await,
            ProviderKind::FourKHdHub => {
                let fourk = self.fourk_client.as_ref().ok_or_else(|| {
                    ProviderError::Unavailable("4KHDHub is unavailable".to_string())
                })?;
                Provider::search(fourk, query, page).await
            }
            ProviderKind::BdixCircleFtp => {
                Provider::search(&self.circleftp_client, query, page).await
            }
            ProviderKind::BdixDhakaFlix => {
                Provider::search(&self.dhakaflix_client, query, page).await
            }
            ProviderKind::Addons => Provider::search(&self.addon_client, query, page).await,
            ProviderKind::Dramachi => Provider::search(&self.dramachi_client, query, page).await,
        }?;
        Ok(self.hide_hindi_catalog(items))
    }

    pub async fn fetch_addon_catalog(
        &self,
        manifest_url: &str,
        r#type: &str,
        catalog_id: &str,
    ) -> Result<Vec<CatalogItem>, String> {
        let manifest_clone = manifest_url.to_string();
        let type_clone = r#type.to_string();
        let cat_id_clone = catalog_id.to_string();
        if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
            crate::cache::get_addon_catalog_cache_typed(&manifest_clone, &type_clone, &cat_id_clone)
        })
        .await
        {
            return Ok(cached);
        }

        let base_url = crate::providers::addons::AddonClient::base_addon_url(manifest_url);
        let metas = self
            .addon_client
            .fetch_catalog(&base_url, r#type, catalog_id, None)
            .await
            .map_err(|e| e.to_string())?;

        let items: Vec<CatalogItem> = metas
            .iter()
            .map(crate::providers::addons::adapter::meta_to_catalog_item)
            .collect();

        let manifest_clone = manifest_url.to_string();
        let type_clone = r#type.to_string();
        let cat_id_clone = catalog_id.to_string();
        let items_clone = items.clone();
        tokio::task::spawn_blocking(move || {
            crate::cache::set_addon_catalog_cache_typed(
                &manifest_clone,
                &type_clone,
                &cat_id_clone,
                &items_clone,
            );
        });
        Ok(items)
    }

    pub async fn details_typed(
        &self,
        provider: ProviderKind,
        subject_id: &str,
    ) -> Result<MediaDetails, ProviderError> {
        match provider {
            ProviderKind::MovieBox => Provider::details(&self.client, subject_id).await,
            ProviderKind::FourKHdHub => {
                let fourk = self.fourk_client.as_ref().ok_or_else(|| {
                    ProviderError::Unavailable("4KHDHub is unavailable".to_string())
                })?;
                Provider::details(fourk, subject_id).await
            }
            ProviderKind::BdixCircleFtp => {
                Provider::details(&self.circleftp_client, subject_id).await
            }
            ProviderKind::BdixDhakaFlix => {
                Provider::details(&self.dhakaflix_client, subject_id).await
            }
            ProviderKind::Addons => Provider::details(&self.addon_client, subject_id).await,
            ProviderKind::Dramachi => Provider::details(&self.dramachi_client, subject_id).await,
        }
    }

    pub async fn homepage(
        &self,
        tab_id: &str,
        page: usize,
    ) -> Result<
        (
            Vec<CatalogItem>,
            std::collections::HashMap<String, BrowseMetrics>,
        ),
        String,
    > {
        let payload = self
            .client
            .get_homepage(tab_id, page)
            .await
            .map_err(|e| e.to_string())?;
        let (items, metrics) =
            crate::providers::moviebox::adapt::moviebox_homepage_json_to_catalog(&payload);
        Ok((self.hide_hindi_catalog(items), metrics))
    }

    pub async fn fetch_collection_resolutions(&self, subject_id: &str) -> Result<Vec<u32>, String> {
        self.client
            .fetch_collection_resolutions(subject_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_ext_captions(
        &self,
        subject_id: &str,
        resource_id: &str,
        sibling_ids: &[String],
        season: usize,
        episode: usize,
    ) -> Result<Vec<crate::providers::models::SubtitleOption>, String> {
        if !subject_id.is_empty() && !resource_id.is_empty() {
            let sid = subject_id.to_string();
            let rid = resource_id.to_string();
            if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                crate::cache::get_captions_cache_typed(&sid, &rid)
            })
            .await
            {
                if !cached.is_empty() || sibling_ids.is_empty() {
                    return Ok(cached);
                }
            }
        }

        let mut all_captions = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        let primary_fut = self.client.get_ext_captions(subject_id, resource_id);
        let mut sibling_futs = Vec::new();
        for sib in sibling_ids.iter().take(3) {
            if !sib.is_empty() && sib != subject_id {
                let client = &self.client;
                sibling_futs.push(async move {
                    tokio::time::timeout(std::time::Duration::from_secs(6), async move {
                        let page = if episode > 0 { (episode - 1) / 20 + 1 } else { 1 };
                        if let Ok((items, _)) =
                            client.fetch_resource_page(sib, season, episode, 0, page).await
                        {
                            let matched_item = find_matching_resource_item(&items, season, episode);
                            if let Some(item) = matched_item {
                                let item_rid = item
                                    .get("resourceId")
                                    .or_else(|| item.get("id"))
                                    .and_then(|v| {
                                        if let Some(n) = v.as_i64() {
                                            Some(n.to_string())
                                        } else if let Some(n) = v.as_u64() {
                                            Some(n.to_string())
                                        } else {
                                            v.as_str().map(|s| s.to_string())
                                        }
                                    });
                                if let Some(rid) = item_rid {
                                    if let Ok(res_payload) = client.get_ext_captions(sib, &rid).await
                                    {
                                        return crate::providers::moviebox::adapt::captions_json_to_options(
                                            &res_payload,
                                        );
                                    }
                                }
                            }
                        }
                        Vec::new()
                    })
                    .await
                    .unwrap_or_default()
                });
            }
        }

        let (primary_res, sibling_results) =
            tokio::join!(primary_fut, futures::future::join_all(sibling_futs));
        if let Ok(payload) = primary_res {
            Self::append_unique_captions(
                &mut all_captions,
                &mut seen_urls,
                crate::providers::moviebox::adapt::captions_json_to_options(&payload),
            );
        }
        for res in sibling_results {
            Self::append_unique_captions(&mut all_captions, &mut seen_urls, res);
        }
        let mut deduplicated: Vec<crate::providers::models::SubtitleOption> = Vec::new();
        let mut seen_languages = std::collections::HashSet::new();
        for sub in all_captions {
            let sanitized_lang = crate::text_util::sanitize_language_label(&sub.name);
            if seen_languages.insert(sanitized_lang) {
                deduplicated.push(sub);
            }
        }

        deduplicated.sort_by(|a, b| {
            let clean_a = crate::text_util::sanitize_language_label(&a.name);
            let clean_b = crate::text_util::sanitize_language_label(&b.name);
            if clean_a.eq_ignore_ascii_case("english") {
                std::cmp::Ordering::Less
            } else if clean_b.eq_ignore_ascii_case("english") {
                std::cmp::Ordering::Greater
            } else {
                clean_a.cmp(&clean_b)
            }
        });
        if !subject_id.is_empty() && !resource_id.is_empty() && !deduplicated.is_empty() {
            let sid = subject_id.to_string();
            let rid = resource_id.to_string();
            let to_cache = deduplicated.clone();
            tokio::task::spawn_blocking(move || {
                crate::cache::set_captions_cache_typed(&sid, &rid, &to_cache);
            });
        }

        Ok(deduplicated)
    }

    /// Online subtitles for a title, keyed by its resolved IMDb id.
    ///
    /// S4 orders Spanish first, English second and the rest after. S6 means every
    /// failure path returns an empty list: an unreachable addon or an IMDb title
    /// with no confident match must never break playback or drop the provider
    /// captions the caller already has.
    ///
    /// S8: the incoming title is a MovieBox subject title and can carry a dub marker
    /// (`[Hindi] Animal`, `Animal - Hindi`) or a trailing release year, none of which
    /// the strict normalized-title match in `pick_best_candidate` accepts. It is
    /// cleaned once, and that same cleaned form is both the IMDb query and the
    /// cache key, so a tagged title and its clean form share one lookup.
    pub async fn search_online_subtitles(
        &self,
        title: &str,
        year: Option<&str>,
        is_series: bool,
        season: usize,
        episode: usize,
    ) -> Vec<crate::providers::models::SubtitleOption> {
        let query_title = crate::providers::moviebox::title::clean_subtitle_title(title);
        if query_title.is_empty() {
            return Vec::new();
        }
        let year_number = year.and_then(|value| value.trim().parse::<i32>().ok());
        let cache_key = imdb_cache_key(&query_title, year_number, is_series);

        let cached = crate::cache::get_imdb_id_cache(&cache_key)
            .filter(|value| crate::providers::opensubtitles::is_imdb_id(value));
        let imdb_id = match cached {
            Some(id) => id,
            None => {
                match self
                    .opensubtitles
                    .resolve_imdb_id(&query_title, year_number, is_series)
                    .await
                {
                    Ok(Some(id)) => {
                        crate::cache::set_imdb_id_cache(&cache_key, &id);
                        id
                    }
                    Ok(None) => return Vec::new(),
                    Err(error) => {
                        log::debug!("online subtitle imdb lookup failed: {}", error.user_message());
                        return Vec::new();
                    }
                }
            }
        };

        let season_episode = if is_series && season > 0 {
            Some((season, episode))
        } else {
            None
        };
        match self
            .opensubtitles
            .fetch_subtitles(&imdb_id, season_episode)
            .await
        {
            Ok(entries) => Self::online_subtitle_options(entries),
            Err(error) => {
                log::debug!("online subtitle search failed: {}", error.user_message());
                Vec::new()
            }
        }
    }

    /// Turns addon entries into picker rows: labelled by language, deduped by URL
    /// and ordered by the S4 preference.
    fn online_subtitle_options(
        entries: Vec<crate::providers::opensubtitles::OnlineSubtitle>,
    ) -> Vec<crate::providers::models::SubtitleOption> {
        use crate::providers::opensubtitles::{language_label, language_rank};

        let mut seen_urls = std::collections::HashSet::new();
        let mut ranked: Vec<(u8, String, String)> = entries
            .into_iter()
            .filter(|entry| seen_urls.insert(entry.url.clone()))
            .map(|entry| {
                let language = language_label(&entry.lang);
                let release = entry.label.trim();
                let name = if release.is_empty() {
                    language
                } else {
                    format!("{language} · {release}")
                };
                (language_rank(&entry.lang), name, entry.url)
            })
            .collect();
        ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        ranked
            .into_iter()
            .map(|(_, name, url)| crate::providers::models::SubtitleOption { name, url })
            .collect()
    }

    fn append_unique_captions(
        target: &mut Vec<crate::providers::models::SubtitleOption>,
        seen_urls: &mut std::collections::HashSet<String>,
        opts: Vec<crate::providers::models::SubtitleOption>,
    ) {
        for opt in opts {
            if seen_urls.insert(opt.url.clone()) {
                target.push(opt);
            }
        }
    }
    pub async fn fetch_poster_bytes(&self, url: &str) -> Option<Vec<u8>> {
        const MAX_POSTER_BYTES: usize = 5 * 1024 * 1024;
        let mut response = self
            .http_client
            .get(url)
            .header("User-Agent", crate::net::APP_HTTP_USER_AGENT)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let content_len = response.content_length();
        if content_len.is_some_and(|len| len > MAX_POSTER_BYTES as u64) {
            log::warn!(
                "poster at {} exceeds size limit ({} bytes), skipping",
                crate::logging::sanitize_url(url),
                content_len.unwrap_or(0)
            );
            return None;
        }
        let mut buf = Vec::with_capacity(
            content_len
                .map(|l| (l as usize).min(MAX_POSTER_BYTES))
                .unwrap_or(64 * 1024),
        );
        while let Ok(Some(chunk)) = response.chunk().await {
            if buf.len().saturating_add(chunk.len()) > MAX_POSTER_BYTES {
                return None;
            }
            buf.extend_from_slice(&chunk);
        }
        if buf.is_empty() { None } else { Some(buf) }
    }

    pub async fn fetch_and_decode_cached_poster(
        &self,
        namespace: &str,
        id: &str,
        url: &str,
    ) -> Option<Arc<image::DynamicImage>> {
        let ns = namespace.to_string();
        let id_owned = id.to_string();
        if let Ok(Some(bytes)) = tokio::task::spawn_blocking(move || {
            crate::cache::get_namespaced_image_cache(&ns, &id_owned)
        })
        .await
            && let Some(img) = decode_poster(bytes).await
        {
            return Some(img);
        }
        let bytes = Arc::<[u8]>::from(self.fetch_poster_bytes(url).await?);
        let ns = namespace.to_string();
        let id_owned = id.to_string();
        let bytes_for_cache = Arc::clone(&bytes);
        tokio::task::spawn_blocking(move || {
            crate::cache::set_namespaced_image_cache(&ns, &id_owned, &bytes_for_cache);
        });
        decode_poster_arc(bytes).await
    }

    pub async fn download_subtitle_file(
        &self,
        url: &str,
        headers: &[(String, String)],
        preferred_filename: Option<&str>,
    ) -> Result<PathBuf, String> {
        let mut request = self.http_client.get(url);
        let sub_host = crate::proxy::extract_host_authority(url);
        let referer_host = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("referer"))
            .and_then(|(_, val)| crate::proxy::extract_host_authority(val));
        let same_host = sub_host.is_some() && sub_host == referer_host;
        for (name, value) in headers {
            if crate::proxy::should_forward_header(same_host, name) {
                request = request.header(name.as_str(), value.as_str());
            }
        }

        let response = tokio::time::timeout(std::time::Duration::from_secs(8), request.send())
            .await
            .map_err(|_| "timed out".to_string())?
            .map_err(|e| {
                if e.is_timeout() {
                    "timed out".to_string()
                } else {
                    "connection failed".to_string()
                }
            })?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status().as_u16()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|_| "read error".to_string())?;

        let extension = subtitle_extension_from_url(url);
        let base_dir = resolve_subtitle_dir();
        let _ = tokio::fs::create_dir_all(&base_dir).await;

        let url_clean = url.split(['?', '#']).next().unwrap_or(url);
        let url_tag = &crate::cache::md5_hex(url_clean)[..8];
        let file_stem = if let Some(pref) = preferred_filename {
            let base = crate::download::safe_file_stem(pref);
            format!("{base}_{url_tag}")
        } else {
            format!(
                "{}_{}_{url_tag}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        };
        let path = base_dir.join(format!("{file_stem}.{extension}"));
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| format!("write error ({})", e.kind()))?;

        Ok(path)
    }
}

pub async fn decode_poster(bytes: Vec<u8>) -> Option<Arc<image::DynamicImage>> {
    tokio::task::spawn_blocking(move || {
        let img = image::load_from_memory(&bytes).ok()?;
        const MAX_DIM: u32 = 512;
        let downscaled = if img.width().max(img.height()) <= MAX_DIM {
            img
        } else {
            img.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Triangle)
        };
        Some(Arc::new(downscaled))
    })
    .await
    .ok()?
}

async fn decode_poster_arc(bytes: Arc<[u8]>) -> Option<Arc<image::DynamicImage>> {
    tokio::task::spawn_blocking(move || {
        let img = image::load_from_memory(&bytes).ok()?;
        const MAX_DIM: u32 = 512;
        let downscaled = if img.width().max(img.height()) <= MAX_DIM {
            img
        } else {
            img.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Triangle)
        };
        Some(Arc::new(downscaled))
    })
    .await
    .ok()?
}

pub fn metric_value(item: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    let mut containers = vec![item];
    if let Some(metadata) = item.get("metadata") {
        containers.push(metadata);
    }
    if let Some(meta) = item.get("meta") {
        containers.push(meta);
    }

    containers.into_iter().find_map(|container| {
        keys.iter().find_map(|key| {
            let value = container.get(*key)?;
            value
                .as_f64()
                .or_else(|| value.as_i64().map(|number| number as f64))
                .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        })
    })
}

pub fn extract_browse_metrics(item: &serde_json::Value) -> BrowseMetrics {
    BrowseMetrics {
        trending: metric_value(
            item,
            &["__browse_rank", "imdb_trending", "imdbTrending", "trending"],
        ),
        rating: metric_value(
            item,
            &["imdbRatingValue", "imdbRate", "imdb_rating", "imdbRating"],
        ),
        recent_rating: metric_value(
            item,
            &[
                "imdb_rating_30d",
                "imdbRating30Days",
                "imdbRatingLast30Days",
                "imdb_rating_recent",
                "imdbRatingValue",
                "imdbRate",
            ],
        ),
        popularity: metric_value(
            item,
            &[
                "__browse_rank",
                "imdb_popularity",
                "imdbPopularity",
                "popularity",
                "viewers",
            ],
        ),
    }
}

pub fn resolve_subtitle_dir() -> PathBuf {
    if crate::config::is_termux_environment() {
        if let Some(home) = dirs::home_dir() {
            let storage = home.join("storage/downloads/moviebox_subs");
            if home.join("storage/downloads").exists() {
                let _ = std::fs::create_dir_all(&storage);
                return storage;
            }
        }
    }
    crate::config::cache_dir().join("subs")
}

pub fn subtitle_extension_from_url(url: &str) -> &'static str {
    let clean = url.split(['?', '#']).next().unwrap_or(url);
    match clean.rsplit('.').next() {
        Some(ext) if ext.eq_ignore_ascii_case("srt") => "srt",
        Some(ext) if ext.eq_ignore_ascii_case("vtt") => "vtt",
        Some(ext) if ext.eq_ignore_ascii_case("ass") => "ass",
        Some(ext) if ext.eq_ignore_ascii_case("ssa") => "ssa",
        Some(ext) if ext.eq_ignore_ascii_case("sub") => "sub",
        _ => "srt",
    }
}

pub fn ensure_moviebox_subdir(path: &Path) -> PathBuf {
    let is_already_mb = path
        .file_name()
        .map(|name| {
            let s = name.to_string_lossy();
            s.eq_ignore_ascii_case("MovieBox-TUI") || s.eq_ignore_ascii_case("MovieBox")
        })
        .unwrap_or(false);

    if is_already_mb {
        path.to_path_buf()
    } else {
        path.join("MovieBox-TUI")
    }
}

pub fn resolve_download_dir(custom_dir: Option<&Path>) -> PathBuf {
    if let Some(custom) = custom_dir {
        if std::fs::create_dir_all(custom).is_ok() {
            let probe = custom.join(format!(".mb_probe_{}", std::process::id()));
            if std::fs::write(&probe, b"ok").is_ok() {
                let _ = std::fs::remove_file(&probe);
                return ensure_moviebox_subdir(custom);
            }
        }
    }

    let base_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."));

    if crate::config::is_termux_environment() {
        if let Some(home) = dirs::home_dir() {
            let android_storage = home.join("storage/downloads");
            if android_storage.exists() {
                return ensure_moviebox_subdir(&android_storage);
            }
        }
    }

    ensure_moviebox_subdir(&base_dir)
}

pub fn find_matching_resource_item(
    items: &[serde_json::Value],
    season: usize,
    episode: usize,
) -> Option<&serde_json::Value> {
    items.iter().find(|item| {
        let parse_num = |k: &str| -> Option<usize> {
            item.get(k).and_then(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .or_else(|| v.as_i64().map(|n| n as usize))
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
        };
        let se = parse_num("se");
        let ep_num = parse_num("ep");
        (season == 0 && episode == 0) || (se == Some(season) && ep_num == Some(episode))
    })
}

/// Cache key of the resolved IMDb id for one title, year and kind (S8).
///
/// `clean_title` must come from `providers::moviebox::title::clean_subtitle_title`, which
/// is what makes `[Hindi] Animal` and `Animal` share one on-disk lookup.
fn imdb_cache_key(clean_title: &str, year: Option<i32>, is_series: bool) -> String {
    format!(
        "{}|{}|{}",
        clean_title.to_lowercase(),
        year.map(|value| value.to_string()).unwrap_or_default(),
        is_series
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::moviebox::title::clean_subtitle_title;

    fn online(lang: &str, label: &str, url: &str) -> crate::providers::opensubtitles::OnlineSubtitle {
        crate::providers::opensubtitles::OnlineSubtitle {
            url: url.to_string(),
            lang: lang.to_string(),
            label: label.to_string(),
            format: None,
        }
    }

    #[test]
    fn online_subtitle_options_orders_spanish_then_english_then_the_rest() {
        let options = MovieBoxService::online_subtitle_options(vec![
            online("por", "Portuguese.BluRay", "https://cdn.example.com/por.srt"),
            online("eng", "English.WEB", "https://cdn.example.com/eng.srt"),
            online("spa", "Spanish.BluRay", "https://cdn.example.com/spa.srt"),
        ]);

        assert_eq!(
            options
                .iter()
                .map(|option| option.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Español · Spanish.BluRay",
                "English · English.WEB",
                "POR · Portuguese.BluRay"
            ]
        );
    }

    #[test]
    fn online_subtitle_options_orders_alphabetically_within_one_language() {
        let options = MovieBoxService::online_subtitle_options(vec![
            online("spa", "Zeta", "https://cdn.example.com/z.srt"),
            online("spa", "Alfa", "https://cdn.example.com/a.srt"),
        ]);

        assert_eq!(
            options
                .iter()
                .map(|option| option.url.as_str())
                .collect::<Vec<_>>(),
            ["https://cdn.example.com/a.srt", "https://cdn.example.com/z.srt"]
        );
    }

    #[test]
    fn online_subtitle_options_drops_a_repeated_url() {
        let options = MovieBoxService::online_subtitle_options(vec![
            online("spa", "First", "https://cdn.example.com/same.srt"),
            online("eng", "Second", "https://cdn.example.com/same.srt"),
        ]);

        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "Español · First");
    }

    #[test]
    fn online_subtitle_options_falls_back_to_the_language_when_the_release_is_unknown() {
        let options =
            MovieBoxService::online_subtitle_options(vec![online("eng", "  ", "https://cdn.example.com/e.srt")]);

        assert_eq!(options[0].name, "English");
    }

    #[tokio::test]
    async fn search_online_subtitles_needs_no_network_without_a_title() {
        let service = MovieBoxService::new();

        assert!(service.search_online_subtitles("  ", Some("2010"), false, 0, 0).await.is_empty());
    }

    #[test]
    fn imdb_cache_key_uses_the_cleaned_title() {
        // The method derives the key from the cleaned title, so a tagged subject title
        // and its clean form share one cached IMDb lookup (S8).
        let tagged = imdb_cache_key(&clean_subtitle_title("[Hindi] Animal"), Some(2023), false);
        let plain = imdb_cache_key(&clean_subtitle_title("Animal"), Some(2023), false);

        assert_eq!(tagged, "animal|2023|false");
        assert_eq!(tagged, plain);
    }

    #[test]
    fn imdb_cache_key_keeps_year_and_kind_apart() {
        // PRESERVE: the key still separates titles by year and by movie/series.
        assert_eq!(
            imdb_cache_key("Animal", Some(2023), false),
            "animal|2023|false"
        );
        assert_ne!(
            imdb_cache_key("Animal", Some(2023), false),
            imdb_cache_key("Animal", None, false)
        );
        assert_ne!(
            imdb_cache_key("Animal", Some(2023), false),
            imdb_cache_key("Animal", Some(2023), true)
        );
    }

    #[tokio::test]
    async fn test_decode_poster_caps_dimensions() {
        let mut buffer = std::io::Cursor::new(Vec::new());
        let img = image::DynamicImage::new_rgb8(800, 600);
        img.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
        let bytes = buffer.into_inner();

        let decoded = decode_poster(bytes).await.unwrap();
        assert!(decoded.width() <= 512);
        assert!(decoded.height() <= 512);
        assert_eq!(decoded.width(), 512);
        assert_eq!(decoded.height(), 384);
    }

    #[test]
    fn test_find_matching_resource_item_integer_and_string_values() {
        let items = vec![
            serde_json::json!({
                "id": "1001",
                "se": 1,
                "ep": 1
            }),
            serde_json::json!({
                "id": "1002",
                "se": "2",
                "ep": "5"
            }),
        ];

        let matched = find_matching_resource_item(&items, 2, 5);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().get("id").unwrap().as_str(), Some("1002"));
    }

    #[test]
    fn test_find_matching_resource_item_series_does_not_fall_back_to_episode_one() {
        let items = vec![
            serde_json::json!({
                "id": "1001",
                "se": 1,
                "ep": 1
            }),
            serde_json::json!({
                "id": "1002",
                "se": 1,
                "ep": 2
            }),
        ];

        let matched = find_matching_resource_item(&items, 2, 3);
        assert!(matched.is_none());
    }

    #[test]
    fn test_find_matching_resource_item_movie_matches_first() {
        let items = vec![
            serde_json::json!({
                "id": "movie_res_1"
            }),
            serde_json::json!({
                "id": "movie_res_2"
            }),
        ];

        let matched = find_matching_resource_item(&items, 0, 0);
        assert!(matched.is_some());
        assert_eq!(
            matched.unwrap().get("id").unwrap().as_str(),
            Some("movie_res_1")
        );
    }

    #[test]
    fn test_subtitle_extension_from_url_strips_signed_query_and_fragment() {
        assert_eq!(
            subtitle_extension_from_url(
                "https://cacdn.hakunaymatata.com/subtitle/5cc122.vtt?Policy=eyJ&Signature=abc&Key-Pair-Id=KMH"
            ),
            "vtt"
        );
        assert_eq!(
            subtitle_extension_from_url("https://cdn.example.com/subs/track.ASS?token=123#cue1"),
            "ass"
        );
        assert_eq!(
            subtitle_extension_from_url("https://cdn.example.com/subs/track.ssa"),
            "ssa"
        );
        assert_eq!(
            subtitle_extension_from_url("https://cdn.example.com/subs/track.sub?exp=999"),
            "sub"
        );
        assert_eq!(
            subtitle_extension_from_url("https://cdn.example.com/subs/unknown?format=text"),
            "srt"
        );
    }
}
