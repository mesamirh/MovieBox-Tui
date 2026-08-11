pub mod bdix;
pub mod browse;
pub mod fourkhdhub;
pub mod m3u;
pub mod models;
pub mod moviebox;

use models::Release;

pub(crate) trait Provider {
    async fn search(&self, query: &str, page: usize) -> Result<serde_json::Value, String>;
    async fn details(&self, id: &str) -> Result<serde_json::Value, String>;
}

pub(crate) trait ReleaseProvider {
    async fn episode_streams(
        &self,
        id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Vec<Release>, String>;
}
