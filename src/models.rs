pub use crate::providers::models::{
    AudioTrackOption, CatalogItem, Episode, MediaDetails, MediaType, PlaybackSource, ProviderError,
    ProviderKind, ProviderMediaId, Release, Season, SourceMirror, SubtitleOption,
};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct SubjectIdentity<'a> {
    pub provider: &'a str,
    pub subject_id: &'a str,
    pub title: &'a str,
    pub stype: i64,
    pub release_year: &'a str,
}

impl<'a> SubjectIdentity<'a> {
    pub fn matches(&self, other: &SubjectIdentity<'_>) -> bool {
        if self.stype != other.stype {
            return false;
        }

        let prov_a = crate::providers::models::ProviderKind::parse(self.provider);
        let prov_b = crate::providers::models::ProviderKind::parse(other.provider);
        let same_provider = match (prov_a, prov_b) {
            (Some(pa), Some(pb)) => pa == pb,
            _ => self
                .provider
                .trim()
                .eq_ignore_ascii_case(other.provider.trim()),
        };

        if !same_provider {
            return false;
        }

        if !self.subject_id.is_empty() && !other.subject_id.is_empty() {
            return self.subject_id == other.subject_id;
        }

        let clean_a = crate::providers::moviebox::clean_moviebox_title(self.title);
        let clean_b = crate::providers::moviebox::clean_moviebox_title(other.title);
        if !clean_a.is_empty() && clean_a.eq_ignore_ascii_case(clean_b) {
            let year_a = self.release_year.trim();
            let year_b = other.release_year.trim();
            if !year_a.is_empty() && !year_b.is_empty() {
                return year_a == year_b;
            }
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub stype: i64,
    pub release_year: String,
    pub cover_url: Option<String>,
    pub season: usize,
    pub episode: usize,
    pub provider: ProviderKind,
}

impl SearchResult {
    pub fn from_catalog_item(item: CatalogItem) -> Self {
        Self {
            id: item.id.value,
            title: item.title,
            stype: if item.media_type == MediaType::Series {
                2
            } else {
                1
            },
            release_year: item.year.unwrap_or_default(),
            cover_url: item.poster_url,
            season: 0,
            episode: 1,
            provider: item.id.provider,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowseMetric {
    Trending,
    Rating,
    RecentRating,
    Popularity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrowsePreset {
    Trending,
    TopRatedAllTime,
    TopRatedRecent,
    MostWatched,
}

impl BrowsePreset {
    pub const ALL: [Self; 4] = [
        Self::Trending,
        Self::TopRatedAllTime,
        Self::TopRatedRecent,
        Self::MostWatched,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Trending => "Trending Now",
            Self::TopRatedAllTime => "Top Rated (All-Time)",
            Self::TopRatedRecent => "Top Rated (Recent Releases)",
            Self::MostWatched => "Most Watched",
        }
    }

    pub fn metric(self) -> BrowseMetric {
        match self {
            Self::Trending => BrowseMetric::Trending,
            Self::TopRatedAllTime => BrowseMetric::Rating,
            Self::TopRatedRecent => BrowseMetric::RecentRating,
            Self::MostWatched => BrowseMetric::Popularity,
        }
    }

    pub fn descending(self) -> bool {
        true
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct BrowseMetrics {
    pub trending: Option<f64>,
    pub rating: Option<f64>,
    pub recent_rating: Option<f64>,
    pub popularity: Option<f64>,
}

impl BrowseMetrics {
    pub fn value(self, metric: BrowseMetric) -> Option<f64> {
        match metric {
            BrowseMetric::Trending => self.trending,
            BrowseMetric::Rating => self.rating,
            BrowseMetric::RecentRating => self.recent_rating,
            BrowseMetric::Popularity => self.popularity,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SubjectStreamPool {
    pub episode_index: HashMap<(usize, usize), Vec<Release>>,
    pub fetched_pages: HashMap<u32, HashSet<usize>>,
    pub total_pages: HashMap<u32, usize>,
    pub available_resolutions: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationKind {
    pub fn total_duration(&self) -> Duration {
        match self {
            NotificationKind::Info => Duration::from_secs(4),
            NotificationKind::Success => Duration::from_secs(5),
            NotificationKind::Warning => Duration::from_secs(7),
            NotificationKind::Error => Duration::from_secs(10),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub title: String,
    pub message: String,
    pub expires_at: Instant,
    /// A sticky notification never expires on its own: it stays on screen until
    /// the code that created it dismisses it (used for "torrent running").
    pub sticky: bool,
}

impl Notification {
    pub fn new(
        kind: NotificationKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let duration = kind.total_duration();
        Self {
            kind,
            title: title.into(),
            message: message.into(),
            expires_at: Instant::now() + duration,
            sticky: false,
        }
    }

    /// A notification that stays until it is explicitly dismissed.
    pub fn new_sticky(
        kind: NotificationKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            sticky: true,
            ..Self::new(kind, title, message)
        }
    }

    pub fn expired(&self) -> bool {
        !self.sticky && Instant::now() >= self.expires_at
    }
}
