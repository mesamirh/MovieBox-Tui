pub mod client;
pub mod crypto;

use client::{MovieBoxClient, ScraperError};
use serde_json::{Value, json};

impl MovieBoxClient {
    pub async fn search(&self, query: &str, page: usize) -> Result<Value, ScraperError> {
        self.search_with_tab(query, page, "MovieTV").await
    }

    pub async fn suggest(&self, query: &str) -> Result<Value, ScraperError> {
        let payload = json!({
            "keyword": query,
            "page": 1,
            "perPage": 8,
            "subjectType": "All",
            "tabId": "All"
        });
        self.post("/wefeed-mobile-bff/subject-api/search/v2", &payload)
            .await
    }

    async fn search_with_tab(
        &self,
        query: &str,
        page: usize,
        tab_id: &str,
    ) -> Result<Value, ScraperError> {
        let payload = json!({
            "keyword": query,
            "page": page,
            "perPage": 20,
            "subjectType": "All",
            "tabId": tab_id
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
        _season: usize,
        _episode: usize,
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

        let path = format!(
            "/wefeed-mobile-bff/subject-api/resource?subjectId={}&page={}&perPage=20{}",
            subject_id, page, res_param
        );
        self.get(&path).await
    }

    pub async fn get_all_resources(
        &self,
        subject_id: &str,
        season: usize,
        episode: usize,
        absolute_episode_index: usize,
    ) -> Result<Value, ScraperError> {
        let resolutions = ["1080", "720", "480", "360", ""];
        let mut handles = Vec::new();

        let per_page = 20;
        let page = (absolute_episode_index / per_page) + 1;

        for res in resolutions {
            let c = self.clone();
            let sid = subject_id.to_string();
            let r = res.to_string();
            handles.push(tokio::spawn(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(4),
                    c.get_resources(&sid, season, episode, page, Some(&r)),
                )
                .await
                .unwrap_or(Err(ScraperError::ApiStatus(408)))
            }));
        }

        let mut all_list = Vec::new();
        let mut last_err = None;
        for h in handles {
            if let Ok(res_result) = h.await {
                match res_result {
                    Ok(res) => {
                        if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                            let target_ep = episode;
                            let mut found: Option<&Value> = None;
                            for stream in list.iter() {
                                let stream_ep =
                                    stream.get("ep").and_then(|e| e.as_u64()).unwrap_or(1) as usize;
                                let stream_se =
                                    stream.get("se").and_then(|s| s.as_u64()).unwrap_or(1) as usize;

                                if stream_ep == target_ep {
                                    if stream_se == season {
                                        found = Some(stream);
                                        break;
                                    } else if found.is_none() {
                                        found = Some(stream);
                                    }
                                }
                            }

                            if let Some(stream) = found {
                                all_list.push(stream.clone());
                            }
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
