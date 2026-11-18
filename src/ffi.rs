//! UniFFI bridge for the native mobile clients (iOS today).
//!
//! This module is compiled only with the `mobile` feature, so the terminal build
//! and the `moviebox-tui` binary never link it. It exposes exactly what a native
//! front end needs — catalog browsing, title details, stream releases, captions,
//! favorites and watch history — and nothing that assumes a terminal.
//!
//! Every exported method is synchronous from the foreign side and runs its async
//! work on a dedicated multi-thread Tokio runtime. That keeps the Swift side free
//! of UniFFI's async-runtime plumbing while still moving all I/O off the caller's
//! thread; the SwiftUI layer calls these from detached tasks.

use std::sync::{Arc, Mutex};

use crate::providers::models::{
    CatalogItem, MediaDetails, MediaType, ProviderKind, Release, SubtitleOption,
};
use crate::providers::ReleaseProvider;

/// Error surfaced to Swift. `flat_error` keeps the generated Swift type simple: a
/// single `MobileError` with a message.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum MobileError {
    #[error("{message}")]
    Failure { message: String },
}

impl MobileError {
    fn new(message: impl Into<String>) -> Self {
        Self::Failure {
            message: message.into(),
        }
    }
}

type MobileResult<T> = Result<T, MobileError>;

/// A catalog entry (search result, home row item, favorite, history entry).
#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileCatalogItem {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub is_series: bool,
    pub year: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileEpisode {
    pub season: u32,
    pub number: u32,
    pub title: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileSeason {
    pub number: u32,
    pub episodes: Vec<MobileEpisode>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileDetails {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub is_series: bool,
    pub year: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub rating: Option<String>,
    pub director: Option<String>,
    pub stars: Option<String>,
    pub runtime: Option<String>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub seasons: Vec<MobileSeason>,
    /// Dub `subject_id`s sibling to this subject. Passing them to `captions`
    /// widens the caption set the way the terminal client already does.
    pub sibling_subject_ids: Vec<String>,
}

/// An HTTP header the player must send with the stream request.
#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileHeader {
    pub name: String,
    pub value: String,
}

/// One playable stream candidate.
#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileRelease {
    pub filename: String,
    pub quality: Option<String>,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub size_bytes: Option<u64>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    /// Resolved stream URL, when the provider already produced one.
    pub url: Option<String>,
    /// Human label for the source (e.g. `Direct`, `4KHDHub`).
    pub source_label: String,
    pub headers: Vec<MobileHeader>,
    /// Needed to request captions for this release.
    pub resource_id: Option<String>,
    /// `true` when `url` points straight at a media file the system player can
    /// open. `false` means it is a manifest (DASH `.mpd` or HLS `.m3u8`) that
    /// needs remuxing, so the UI should prefer direct files when available.
    pub is_direct_file: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileSubtitle {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MobileHistoryItem {
    pub provider: String,
    pub subject_id: String,
    pub title: String,
    pub poster_url: Option<String>,
    pub is_series: bool,
    pub year: String,
    pub season: u32,
    pub episode: u32,
    pub progress_seconds: u64,
    pub duration_seconds: Option<u64>,
    pub completed: bool,
}

/// The client handed to Swift. One instance per app process is expected.
#[derive(uniffi::Object)]
pub struct MobileClient {
    service: Arc<crate::service::MovieBoxService>,
    runtime: tokio::runtime::Runtime,
    favorites: Mutex<crate::favorites::FavoritesManager>,
    history: Mutex<crate::history::HistoryManager>,
}

#[uniffi::export]
impl MobileClient {
    /// Build the client and point every on-disk store at the app sandbox.
    ///
    /// iOS hands us its own writable directories; `dirs` cannot be trusted to
    /// resolve them inside a sandbox, so the paths are injected here.
    #[uniffi::constructor]
    pub fn new(config_dir: String, cache_dir: String, data_dir: String) -> Arc<Self> {
        // SAFETY: called once, before any other thread touches these variables or
        // the runtime below is created. The TUI sets them from the shell instead.
        unsafe {
            std::env::set_var("MOVIEBOX_CONFIG_DIR", &config_dir);
            std::env::set_var("MOVIEBOX_CACHE_DIR", &cache_dir);
            std::env::set_var("MOVIEBOX_DATA_DIR", &data_dir);
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("moviebox-mobile")
            .build()
            .expect("failed to build the mobile Tokio runtime");

        Arc::new(Self {
            service: Arc::new(crate::service::MovieBoxService::new()),
            runtime,
            favorites: Mutex::new(crate::favorites::FavoritesManager::new()),
            history: Mutex::new(crate::history::HistoryManager::new()),
        })
    }

    /// Free-text search against the default provider.
    pub fn search(&self, query: String, page: u32) -> MobileResult<Vec<MobileCatalogItem>> {
        let provider = ProviderKind::MovieBox;
        let page = page.max(1) as usize;
        let items = self
            .runtime
            .block_on(self.service.search_typed(provider, &query, page))
            .map_err(|error| MobileError::new(error.to_string()))?;
        Ok(items.into_iter().map(catalog_item_from).collect())
    }

    /// Type-ahead suggestions for the search field.
    pub fn suggest(&self, query: String) -> MobileResult<Vec<String>> {
        self.runtime
            .block_on(self.service.suggest(&query))
            .map_err(MobileError::new)
    }

    /// One home/trending tab. Tab ids match the terminal client: `1`, `2`, `5`, `8`.
    pub fn home(&self, tab_id: String, page: u32) -> MobileResult<Vec<MobileCatalogItem>> {
        let page = page.max(1) as usize;
        let (items, _metrics) = self
            .runtime
            .block_on(self.service.homepage(&tab_id, page))
            .map_err(MobileError::new)?;
        Ok(items.into_iter().map(catalog_item_from).collect())
    }

    /// Full details, including the season/episode tree for series.
    pub fn details(&self, subject_id: String) -> MobileResult<MobileDetails> {
        let provider = ProviderKind::MovieBox;
        let details = self
            .runtime
            .block_on(self.service.details_typed(provider, &subject_id))
            .map_err(|error| MobileError::new(error.to_string()))?;
        Ok(details_from(&details))
    }

    /// Stream candidates for a title. Movies use `season = 0, episode = 0`, which
    /// is the same convention the terminal client uses.
    pub fn releases(
        &self,
        subject_id: String,
        season: u32,
        episode: u32,
    ) -> MobileResult<Vec<MobileRelease>> {
        let season = season as usize;
        let episode = episode as usize;
        let releases = self.runtime.block_on(async {
            ReleaseProvider::episode_streams(&self.service.client, &subject_id, season, episode)
                .await
        });
        let releases = releases.map_err(|error| MobileError::new(error.to_string()))?;
        // Spec S2/S3: the Hindi filter lives in the core, so the iOS picker and
        // autoplay see exactly what the terminal client sees.
        let releases = if crate::config::load().hide_hindi {
            crate::language_filter::filter_releases(releases)
        } else {
            releases
        };
        Ok(releases.into_iter().map(release_from).collect())
    }

    /// Subtitle tracks for a release whose `resource_id` is known.
    ///
    /// `sibling_ids` are the dub subject ids from `MobileDetails`, which is how
    /// the terminal client reaches the captions of every available dub.
    pub fn captions(
        &self,
        subject_id: String,
        resource_id: String,
        sibling_ids: Vec<String>,
        season: u32,
        episode: u32,
    ) -> MobileResult<Vec<MobileSubtitle>> {
        let captions = self
            .runtime
            .block_on(self.service.get_ext_captions(
                &subject_id,
                &resource_id,
                &sibling_ids,
                season as usize,
                episode as usize,
            ))
            .map_err(MobileError::new)?;
        Ok(captions.into_iter().map(subtitle_from).collect())
    }

    /// Online subtitles for a title, searched by the content being watched.
    ///
    /// Returns an empty list instead of failing: the picker still shows the
    /// provider captions, so a missing internet connection never blocks playback.
    pub fn search_online_subtitles(
        &self,
        title: String,
        year: Option<String>,
        is_series: bool,
        season: u32,
        episode: u32,
    ) -> Vec<MobileSubtitle> {
        self.runtime
            .block_on(self.service.search_online_subtitles(
                &title,
                year.as_deref(),
                is_series,
                season as usize,
                episode as usize,
            ))
            .into_iter()
            .map(subtitle_from)
            .collect()
    }

    /// Favorites, newest last, mirroring the terminal client's ordering.
    pub fn favorites(&self) -> Vec<MobileCatalogItem> {
        let manager = self.favorites.lock().expect("favorites lock");
        manager.items.iter().map(favorite_item_from).collect()
    }

    pub fn is_favorite(&self, subject_id: String, provider: String) -> bool {
        let manager = self.favorites.lock().expect("favorites lock");
        manager.items.iter().any(|item| {
            item.subject_id == subject_id && same_provider(&item.provider, &provider)
        })
    }

    /// Adds the item to favorites, or removes it if already present. Returns
    /// `true` when the item ended up favorited.
    pub fn toggle_favorite(&self, item: MobileCatalogItem) -> bool {
        let details = details_for_storage(&item);
        let favorite = crate::favorites::FavoriteItem::from_details(&item.provider, &details);
        let mut manager = self.favorites.lock().expect("favorites lock");
        let now_favorited = manager.toggle(favorite);
        manager.save();
        now_favorited
    }

    pub fn remove_favorite(&self, subject_id: String, provider: String) {
        let mut manager = self.favorites.lock().expect("favorites lock");
        manager.remove(&crate::models::SubjectIdentity {
            provider: &provider,
            subject_id: &subject_id,
            title: "",
            stype: 0,
            release_year: "",
        });
        manager.save();
    }

    /// Watch history, most recent last.
    pub fn history(&self) -> Vec<MobileHistoryItem> {
        let manager = self.history.lock().expect("history lock");
        manager.recent.iter().map(history_item_from).collect()
    }

    /// Persist playback progress. `completed` marks the episode watched.
    pub fn record_progress(
        &self,
        item: MobileHistoryItem,
        progress_seconds: u64,
        duration_seconds: Option<u64>,
        completed: bool,
    ) {
        let entry = history_item_to_storage(&item);
        let mut manager = self.history.lock().expect("history lock");
        manager.update_progress(entry, progress_seconds, duration_seconds, completed);
        manager.save();
    }

    pub fn remove_history_entry(
        &self,
        provider: String,
        subject_id: String,
        season: u32,
        episode: u32,
    ) {
        let mut manager = self.history.lock().expect("history lock");
        manager.remove(&provider, &subject_id, season as usize, episode as usize);
        manager.save();
    }

    pub fn clear_history(&self) {
        let mut manager = self.history.lock().expect("history lock");
        manager.clear();
        manager.save();
    }
}

fn same_provider(a: &str, b: &str) -> bool {
    match (ProviderKind::parse(a.trim()), ProviderKind::parse(b.trim())) {
        (Some(left), Some(right)) => left == right,
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}

fn catalog_item_from(item: CatalogItem) -> MobileCatalogItem {
    // Spec S6a: the raw title keeps the provider's dub marker for detection, and
    // the record Swift renders carries the cleaned form instead, so the marker
    // never reaches an iOS listing.
    let title = crate::providers::moviebox::clean_moviebox_title(&item.title).to_string();
    MobileCatalogItem {
        provider: item.id.provider.label().to_string(),
        id: item.id.value,
        title,
        is_series: item.media_type == MediaType::Series,
        year: item.year,
        poster_url: item.poster_url,
    }
}

fn favorite_item_from(item: &crate::favorites::FavoriteItem) -> MobileCatalogItem {
    MobileCatalogItem {
        id: item.subject_id.clone(),
        provider: item.provider.clone(),
        // Spec S6a: favorites stored before this change still hold the marked
        // title, so the marker is stripped on the way out as well.
        title: crate::providers::moviebox::clean_moviebox_title(&item.title).to_string(),
        is_series: item.stype == 2,
        year: if item.release_year.trim().is_empty() {
            None
        } else {
            Some(item.release_year.clone())
        },
        poster_url: item.cover_url.clone(),
    }
}

fn history_item_from(item: &crate::history::WatchHistoryItem) -> MobileHistoryItem {
    MobileHistoryItem {
        provider: item.provider.clone(),
        subject_id: item.subject_id.clone(),
        // Spec S6a: same as favorites, history rows recorded before this change
        // are shown without the marker.
        title: crate::providers::moviebox::clean_moviebox_title(&item.title).to_string(),
        poster_url: item.cover_url.clone(),
        is_series: item.stype == 2,
        year: item.release_year.clone(),
        season: item.season as u32,
        episode: item.episode as u32,
        progress_seconds: item.progress_seconds,
        duration_seconds: item.duration_seconds,
        completed: item.completed,
    }
}

fn history_item_to_storage(item: &MobileHistoryItem) -> crate::history::WatchHistoryItem {
    crate::history::WatchHistoryItem {
        provider: item.provider.clone(),
        subject_id: item.subject_id.clone(),
        title: item.title.clone(),
        cover_url: item.poster_url.clone(),
        stype: if item.is_series { 2 } else { 1 },
        release_year: item.year.clone(),
        season: item.season as usize,
        episode: item.episode as usize,
        timestamp: 0,
        duration_seconds: item.duration_seconds,
        progress_seconds: item.progress_seconds,
        completed: item.completed,
        stream_filename: None,
    }
}

/// Rebuild the `MediaDetails` shape that favorites/history normalisation expects,
/// so the FFI stores exactly what the terminal client would store.
fn details_for_storage(item: &MobileCatalogItem) -> MediaDetails {
    MediaDetails {
        id: crate::providers::models::ProviderMediaId {
            provider: ProviderKind::parse(&item.provider).unwrap_or(ProviderKind::MovieBox),
            value: item.id.clone(),
        },
        title: item.title.clone(),
        media_type: if item.is_series {
            MediaType::Series
        } else {
            MediaType::Movie
        },
        year: item.year.clone(),
        description: None,
        tagline: None,
        imdb_rating: None,
        director: None,
        stars: None,
        prints: None,
        audios: None,
        poster_url: item.poster_url.clone(),
        duration: None,
        genres: Vec::new(),
        seasons: Vec::new(),
        dubs: Vec::new(),
    }
}

fn details_from(details: &MediaDetails) -> MobileDetails {
    // Spec S2/S3: Hindi dubs are dropped here, so `sibling_subject_ids` (and the
    // caption search that walks it) never reaches a Hindi dub either.
    let dubs = if crate::config::load().hide_hindi {
        crate::language_filter::filter_dubs(details.dubs.clone())
    } else {
        details.dubs.clone()
    };
    MobileDetails {
        id: details.id.value.clone(),
        provider: details.id.provider.label().to_string(),
        // S8: the player HUD and the online subtitle search both read this title, so
        // the dub marker of a MovieBox subject title never reaches the viewer.
        title: crate::providers::moviebox::title::clean_subtitle_title(&details.title),
        is_series: details.is_series(),
        year: details.year.clone(),
        overview: details.description.clone(),
        tagline: details.tagline.clone(),
        rating: details.imdb_rating.clone(),
        director: details.director.clone(),
        stars: details.stars.clone(),
        runtime: details.duration.clone(),
        genres: details.genres.clone(),
        poster_url: details.poster_url.clone(),
        sibling_subject_ids: dubs
            .iter()
            .map(|dub| dub.subject_id.clone())
            .filter(|id| !id.is_empty() && *id != details.id.value)
            .collect(),
        seasons: details
            .seasons
            .iter()
            .map(|season| MobileSeason {
                number: season.number as u32,
                episodes: season
                    .episodes
                    .iter()
                    .map(|episode| MobileEpisode {
                        season: episode.season as u32,
                        number: episode.number as u32,
                        title: episode.title.clone(),
                        overview: episode.overview.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn release_from(release: Release) -> MobileRelease {
    // Prefer a mirror that points at a real file: AVPlayer cannot play DASH
    // manifests, so a direct MP4 is worth more than an earlier manifest entry.
    let mirror = release
        .mirrors
        .iter()
        .find(|mirror| mirror.direct_file)
        .or_else(|| release.mirrors.first());

    let headers = mirror
        .map(|mirror| {
            mirror
                .headers
                .iter()
                .map(|(name, value)| MobileHeader {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    MobileRelease {
        filename: release.filename.clone(),
        quality: release.quality.clone(),
        codec: release.codec.clone(),
        language: release.language.clone(),
        size_bytes: release.size_bytes,
        season: release.season.map(|value| value as u32),
        episode: release.episode.map(|value| value as u32),
        url: mirror.map(|mirror| mirror.resolver_url.clone()),
        source_label: mirror
            .map(|mirror| mirror.label.clone())
            .unwrap_or_else(|| release.source_label().to_string()),
        headers,
        resource_id: release.resource_id.clone(),
        is_direct_file: mirror.map(|mirror| mirror.direct_file).unwrap_or(false),
    }
}

fn subtitle_from(subtitle: SubtitleOption) -> MobileSubtitle {
    MobileSubtitle {
        name: subtitle.name,
        url: subtitle.url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_provider_accepts_display_and_key_forms() {
        assert!(same_provider("MovieBox", "moviebox"));
        assert!(!same_provider("MovieBox", "Dramachi"));
    }

    #[test]
    fn catalog_item_from_exposes_a_cleaned_listing_title() {
        // Spec S6a: iOS renders the record title verbatim, so the dub marker
        // never reaches it even though the raw catalog title still carries it.
        let item = CatalogItem {
            id: crate::providers::models::ProviderMediaId {
                provider: ProviderKind::MovieBox,
                value: "123".to_string(),
            },
            title: "[Hindi] Ek Deewane Ki Deewaniyat".to_string(),
            media_type: MediaType::Movie,
            year: Some("2025".to_string()),
            poster_url: None,
            season_count: None,
        };

        let mobile = catalog_item_from(item);

        assert_eq!(mobile.title, "Ek Deewane Ki Deewaniyat");
        assert_eq!(mobile.id, "123");
        assert_eq!(mobile.year.as_deref(), Some("2025"));
    }

    #[test]
    fn history_round_trips_through_storage_shape() {
        let item = MobileHistoryItem {
            provider: "MovieBox".to_string(),
            subject_id: "123".to_string(),
            title: "Example".to_string(),
            poster_url: Some("https://example.test/poster.jpg".to_string()),
            is_series: true,
            year: "2024".to_string(),
            season: 2,
            episode: 7,
            progress_seconds: 90,
            duration_seconds: Some(3600),
            completed: false,
        };

        let stored = history_item_to_storage(&item);
        assert_eq!(stored.stype, 2);
        assert_eq!(stored.season, 2);
        assert_eq!(stored.episode, 7);

        let back = history_item_from(&stored);
        assert_eq!(back.subject_id, item.subject_id);
        assert_eq!(back.season, 2);
        assert_eq!(back.duration_seconds, Some(3600));
    }
}
