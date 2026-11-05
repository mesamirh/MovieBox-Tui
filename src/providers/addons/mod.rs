pub mod adapter;
pub mod aggregator;
pub mod client;
pub mod models;

pub use adapter::{
    meta_detail_to_media_details, meta_to_catalog_item, release_to_playback_source,
    stream_item_to_release,
};
pub use aggregator::aggregate_streams;
pub use client::AddonClient;
pub use models::{AddonManifest, InstalledAddon, MetaDetail, MetaItem, StreamItem};

use crate::providers::models::{CatalogItem, MediaDetails, ProviderError, ProviderKind};
use crate::providers::{Provider, ProviderCapabilities};

impl Provider for AddonClient {
    fn id(&self) -> ProviderKind {
        ProviderKind::Addons
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_search: true,
            supports_pagination: false,
            supports_series: true,
            supports_subtitles: true,
            supports_homepage: false,
        }
    }

    async fn search(&self, query: &str, _page: usize) -> Result<Vec<CatalogItem>, ProviderError> {
        let addons = tokio::task::spawn_blocking(crate::config::load_addons)
            .await
            .unwrap_or_default();
        let catalog_addons: Vec<_> = addons
            .iter()
            .filter(|a| a.enabled && (a.provides_meta || a.provides_catalog))
            .collect();

        if catalog_addons.is_empty() {
            return Err(ProviderError::Unavailable(
                "No catalog/metadata addon enabled. Open /config to configure one.".to_string(),
            ));
        }

        let mut combined = Vec::new();
        for addon in catalog_addons {
            let base_url = Self::base_addon_url(&addon.manifest_url);
            let (movies_res, series_res) = tokio::join!(
                self.fetch_catalog_search(&base_url, "movie", "top", query),
                self.fetch_catalog_search(&base_url, "series", "top", query),
            );
            if let Ok(movies) = movies_res {
                combined.extend(movies);
            }
            if let Ok(series) = series_res {
                combined.extend(series);
            }
            if !combined.is_empty() {
                break;
            }
        }

        let mut seen = std::collections::HashSet::new();
        Ok(combined
            .into_iter()
            .filter(|m| seen.insert(m.id.clone()))
            .map(|m| adapter::meta_to_catalog_item(&m))
            .collect())
    }

    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError> {
        let (type_hint, clean_id) = match id.split_once(':') {
            Some((prefix, rest))
                if ["movie", "series", "tv", "anime", "other"]
                    .contains(&prefix.to_ascii_lowercase().as_str()) =>
            {
                (Some(prefix.to_ascii_lowercase()), rest)
            }
            _ => (None, id),
        };

        let addons = tokio::task::spawn_blocking(crate::config::load_addons)
            .await
            .unwrap_or_default();
        let mut target_addons: Vec<_> = addons
            .iter()
            .filter(|a| a.enabled && a.provides_meta)
            .collect();

        for addon in &addons {
            if addon.enabled
                && !target_addons
                    .iter()
                    .any(|m| m.manifest_url == addon.manifest_url)
            {
                target_addons.push(addon);
            }
        }

        let types_to_try: Vec<&str> = if let Some(hint) = &type_hint {
            let mut types = vec![hint.as_str()];
            for t in ["series", "movie", "tv", "anime", "other"] {
                if !types.contains(&t) {
                    types.push(t);
                }
            }
            types
        } else {
            vec!["series", "movie", "tv", "anime", "other"]
        };

        let mut best_detail: Option<models::MetaDetail> = None;

        for addon in &target_addons {
            let base_url = Self::base_addon_url(&addon.manifest_url);
            for t in &types_to_try {
                if let Ok(d) = self.fetch_meta(&base_url, t, clean_id).await {
                    let has_valid_id = d.id == clean_id;
                    let has_title = !d.name.trim().is_empty()
                        || d.title.as_deref().is_some_and(|t| !t.trim().is_empty());
                    if has_valid_id && has_title {
                        if !d.videos.is_empty()
                            || d.r#type.eq_ignore_ascii_case("series")
                            || d.r#type.eq_ignore_ascii_case("tv")
                        {
                            let mut details = adapter::meta_detail_to_media_details(&d);
                            if let Some(hint) = &type_hint {
                                details.id.value = format!("{hint}:{clean_id}");
                            }
                            return Ok(details);
                        }
                        if type_hint.as_deref() == Some("movie")
                            && d.r#type.eq_ignore_ascii_case("movie")
                        {
                            let mut details = adapter::meta_detail_to_media_details(&d);
                            details.id.value = format!("movie:{clean_id}");
                            return Ok(details);
                        }
                        if best_detail.is_none() {
                            best_detail = Some(d);
                        }
                    }
                }
            }
        }

        if let Some(d) = best_detail {
            let mut details = adapter::meta_detail_to_media_details(&d);
            if let Some(hint) = &type_hint {
                details.id.value = format!("{hint}:{clean_id}");
            }
            return Ok(details);
        }

        Err(ProviderError::NotFound)
    }
}
