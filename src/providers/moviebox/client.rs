use crate::providers::moviebox::crypto::build_signed_headers;
use reqwest::Response;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
const HOST_POOL: &[&str] = &[
    "https://api6.aoneroom.com",
    "https://api5.aoneroom.com",
    "https://api4.aoneroom.com",
    "https://api4sg.aoneroom.com",
    "https://api3.aoneroom.com",
    "https://api6sg.aoneroom.com",
    "https://api.inmoviebox.com",
];

const RETRY_STATUS_CODES: &[u16] = &[403, 406, 407, 429, 500, 502, 503, 504];

#[derive(thiserror::Error, Debug)]
pub enum ScraperError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("API error status: {0}")]
    ApiStatus(u16),
    #[error("All hosts exhausted")]
    HostsExhausted,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing expected token")]
    MissingToken,
}

#[derive(Clone)]
pub struct MovieBoxClient {
    client: reqwest::Client,
    runtime_token: Arc<RwLock<Option<String>>>,
    active_base_idx: Arc<AtomicUsize>,
    user_agent: String,
    client_info: String,
    spoofed_ip: String,
}

impl Default for MovieBoxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieBoxClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .connect_timeout(std::time::Duration::from_secs(3))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let (user_agent, client_info) =
            crate::providers::moviebox::crypto::generate_client_info_and_ua();
        let spoofed_ip = crate::providers::moviebox::crypto::random_spoofed_ip();

        Self {
            client,
            runtime_token: Arc::new(RwLock::new(None)),
            active_base_idx: Arc::new(AtomicUsize::new(0)),
            user_agent,
            client_info,
            spoofed_ip,
        }
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn init(&self) -> Result<(), ScraperError> {
        let path = "/wefeed-mobile-bff/tab-operating?page=1&tabId=0&version=";
        let _ = self.request_hosts("GET", path, None).await?;

        let has_token = self
            .runtime_token
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some();
        if !has_token {
            return Err(ScraperError::MissingToken);
        }
        Ok(())
    }

    async fn absorb_x_user(&self, headers: &reqwest::header::HeaderMap) {
        let Some(x_user_val) = headers.get("x-user") else {
            return;
        };
        let Ok(x_user_str) = x_user_val.to_str() else {
            return;
        };
        let Ok(json): Result<Value, _> = serde_json::from_str(x_user_str) else {
            return;
        };
        let Some(token) = json.get("token").and_then(|t| t.as_str()) else {
            return;
        };
        if !token.is_empty() {
            let mut write_token = self
                .runtime_token
                .write()
                .unwrap_or_else(|e| e.into_inner());
            *write_token = Some(token.to_string());
        }
    }

    pub async fn get(&self, path_and_query: &str) -> Result<Value, ScraperError> {
        self.request("GET", path_and_query, None).await
    }

    pub async fn post(&self, path_and_query: &str, body: &Value) -> Result<Value, ScraperError> {
        let body_str = serde_json::to_string(body)?;
        self.request("POST", path_and_query, Some(&body_str)).await
    }

    async fn request(
        &self,
        method: &str,
        path_and_query: &str,
        body: Option<&str>,
    ) -> Result<Value, ScraperError> {
        match self.request_hosts(method, path_and_query, body).await {
            Err(ScraperError::HostsExhausted) => {
                let has_token = self
                    .runtime_token
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some();
                if has_token {
                    return Err(ScraperError::HostsExhausted);
                }
                let _ = self.init().await;
                self.request_hosts(method, path_and_query, body).await
            }
            result => result,
        }
    }

    async fn request_hosts(
        &self,
        method: &str,
        path_and_query: &str,
        body: Option<&str>,
    ) -> Result<Value, ScraperError> {
        let start_idx = self.active_base_idx.load(Ordering::Relaxed);
        let mut backoff_ms: u64 = 50;

        for i in 0..HOST_POOL.len() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                backoff_ms = 50;
            }
            let idx = (start_idx + i) % HOST_POOL.len();
            let base = HOST_POOL[idx];
            let url = format!("{}{}", base, path_and_query);

            let token = self
                .runtime_token
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let headers = build_signed_headers(
                method,
                &url,
                body,
                token.as_deref(),
                &self.user_agent,
                &self.client_info,
                &self.spoofed_ip,
            );

            let mut builder = match method {
                "POST" => self.client.post(&url),
                _ => self.client.get(&url),
            };

            builder = builder.headers(headers);
            if let Some(b) = body {
                builder = builder.body(b.to_string());
            }

            match builder.send().await {
                Ok(resp) => {
                    self.absorb_x_user(resp.headers()).await;
                    let status = resp.status().as_u16();

                    if RETRY_STATUS_CODES.contains(&status) {
                        log::warn!(
                            "moviebox host {idx} returned retryable status {status}: {}",
                            crate::logging::sanitize_url(&url)
                        );
                        if status == 429 {
                            backoff_ms = resp
                                .headers()
                                .get(reqwest::header::RETRY_AFTER)
                                .and_then(|v| v.to_str().ok())
                                .and_then(|v| v.parse::<u64>().ok())
                                .map(|secs| secs.saturating_mul(1000).min(3000))
                                .unwrap_or(400);
                        }
                        continue;
                    }

                    self.active_base_idx.store(idx, Ordering::Relaxed);

                    match self.parse_response(resp).await {
                        Ok(val) => return Ok(val),
                        Err(error) => {
                            log::warn!(
                                "moviebox host {idx} parse failed: {error} [{}]",
                                crate::logging::sanitize_url(&url)
                            );
                            continue;
                        }
                    }
                }
                Err(error) => {
                    log::warn!(
                        "moviebox host {idx} request failed: {error} [{}]",
                        crate::logging::sanitize_url(&url)
                    );
                    continue;
                }
            }
        }

        log::error!("moviebox: all hosts exhausted for [redacted]");
        Err(ScraperError::HostsExhausted)
    }

    async fn parse_response(&self, resp: Response) -> Result<Value, ScraperError> {
        let status = resp.status();
        if !status.is_success() {
            return Err(ScraperError::ApiStatus(status.as_u16()));
        }

        let raw_text = match resp.text().await {
            Ok(t) => t,
            Err(e) => return Err(ScraperError::Reqwest(e)),
        };

        let body_val: Value =
            match tokio::task::spawn_blocking(move || serde_json::from_str(&raw_text)).await {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => return Err(ScraperError::Json(e)),
                Err(_) => {
                    return Err(ScraperError::HostsExhausted);
                }
            };

        if let Some(data) = body_val.get("data") {
            Ok(data.clone())
        } else {
            Ok(body_val)
        }
    }
}
