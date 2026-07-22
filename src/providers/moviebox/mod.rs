pub mod client;
pub mod crypto;

use client::{MovieBoxClient, ScraperError};
use serde_json::{Value, json};

impl MovieBoxClient {
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

    pub async fn suggest(&self, query: &str) -> Result<Value, ScraperError> {
        let payload = json!({
            "keyword": query,
            "page": 1,
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
                c.get_resources(&sid, season, episode, 1, Some(&r), 20)
                    .await
            }));
        }

        let mut all_list = Vec::new();
        let mut last_err = None;
        for h in handles {
            if let Ok(res_result) = h.await {
                match res_result {
                    Ok(res) => {
                        if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                            all_list.extend(list.clone());
                        }
                    }
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
        }

        if all_list.is_empty() {
            if let Some(e) = last_err {
                Err(e)
            } else {
                Err(ScraperError::ApiStatus(404))
            }
        } else {
            let mut combined = serde_json::Map::new();
            combined.insert("list".to_string(), serde_json::Value::Array(all_list));
            Ok(serde_json::Value::Object(combined))
        }
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
