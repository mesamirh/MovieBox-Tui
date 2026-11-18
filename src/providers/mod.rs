pub mod addons;
pub mod bdix;
pub mod dramachi;
pub mod fourkhdhub;
pub mod models;
pub mod moviebox;
pub mod opensubtitles;
pub mod tv;

pub use tv as m3u;

pub use models::{
    AudioTrackOption, CatalogItem, Episode, MediaDetails, MediaType, PlaybackSource, ProviderError,
    ProviderKind, ProviderMediaId, Release, ResolutionIntent, Season, SourceMirror, SubtitleOption,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_search: bool,
    pub supports_pagination: bool,
    pub supports_series: bool,
    pub supports_subtitles: bool,
    pub supports_homepage: bool,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            supports_search: true,
            supports_pagination: false,
            supports_series: true,
            supports_subtitles: true,
            supports_homepage: false,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderKind;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn search(&self, query: &str, page: usize) -> Result<Vec<CatalogItem>, ProviderError>;
    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError>;
}

#[allow(async_fn_in_trait)]
pub trait ReleaseProvider: Send + Sync {
    async fn episode_streams(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<Release>, ProviderError>;
}
