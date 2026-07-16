pub mod client;
pub mod crypto;

use client::{ScraperError, MovieBoxClient};
use serde_json::{Value, json};

impl MovieBoxClient {
    pub async fn get_homepage(&self, page: usize) -> Result<Value, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/tab-operating?page={}&tabId=0&version=",
            page
        );
        self.get(&path).await
    }

    pub async fn search(&self, query: &str, page: usize) -> Result<Value, ScraperError> {
        let payload = json!({
            "keyword": query,
            "page": page,
            "perPage": 20,
            "subjectType": "All",
            "tabId": "All"
        });
        self.post("/wefeed-mobile-bff/subject-api/search/v2", &payload)
            .await
    }

    pub async fn get_details(&self, subject_id: &str) -> Result<Value, ScraperError> {
        let path = format!(
            "/wefeed-mobile-bff/subject-api/get?subjectId={}",
            subject_id
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
                "/wefeed-mobile-bff/subject-api/resource?subjectId={}&page={}&perPage=20{}",
                subject_id, page, res_param
            )
        } else {
            format!(
                "/wefeed-mobile-bff/subject-api/resource?subjectId={}&se={}&ep={}&page={}&perPage=20{}",
                subject_id, season, episode, page, res_param
            )
        };
        self.get(&path).await
    }

    pub async fn get_play_info(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Value, ScraperError> {
        let path = if season == 0 && episode == 0 {
            format!(
                "/wefeed-mobile-bff/subject-api/play-info?subjectId={}",
                subject_id
            )
        } else {
            format!(
                "/wefeed-mobile-bff/subject-api/play-info?subjectId={}&se={}&ep={}",
                subject_id, season, episode
            )
        };
        self.get(&path).await
    }

    pub async fn get_all_resources(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
    ) -> Result<Value, ScraperError> {
        let resolutions = ["1080", "720", "480", "360", ""];
        let mut handles = Vec::new();

        for res in resolutions {
            let c = self.clone();
            let sid = subject_id.to_string();
            let r = res.to_string();
            handles.push(tokio::spawn(async move {
                c.get_resources(&sid, season, episode, 1, Some(&r)).await
            }));
        }

        let mut all_list = Vec::new();
        for h in handles {
            if let Ok(Ok(res)) = h.await
                && let Some(list) = res.get("list").and_then(|l| l.as_array())
            {
                all_list.extend(list.clone());
            }
        }

        if all_list.is_empty() {
            Err(ScraperError::ApiStatus(404))
        } else {
            let mut combined = serde_json::Map::new();
            combined.insert("list".to_string(), serde_json::Value::Array(all_list));
            Ok(serde_json::Value::Object(combined))
        }
    }
}
