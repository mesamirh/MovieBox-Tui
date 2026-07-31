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
    #[error("API error ({code}): {message}")]
    ApiMessage { code: i64, message: String },
    #[error(
        "All hosts exhausted — MovieBox servers rejected or timed out every endpoint. Wait a minute, press r to retry, or Ctrl+P to switch provider."
    )]
    HostsExhausted,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing expected token")]
    MissingToken,
}

#[derive(Clone)]
struct ClientIdentity {
    user_agent: String,
    client_info: String,
    spoofed_ip: String,
}

impl ClientIdentity {
    fn fresh() -> Self {
        let (user_agent, client_info) =
            crate::providers::moviebox::crypto::generate_client_info_and_ua();
        let spoofed_ip = crate::providers::moviebox::crypto::random_spoofed_ip();
        Self {
            user_agent,
            client_info,
            spoofed_ip,
        }
    }
}

#[derive(Clone)]
pub struct MovieBoxClient {
    client: reqwest::Client,
    runtime_token: Arc<RwLock<Option<String>>>,
    active_base_idx: Arc<AtomicUsize>,
    identity: Arc<RwLock<ClientIdentity>>,
}

impl Default for MovieBoxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieBoxClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            runtime_token: Arc::new(RwLock::new(None)),
            active_base_idx: Arc::new(AtomicUsize::new(0)),
            identity: Arc::new(RwLock::new(ClientIdentity::fresh())),
        }
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Rotate device fingerprint / spoofed IP and drop the session token.
    pub fn refresh_identity(&self) {
        if let Ok(mut token) = self.runtime_token.write() {
            *token = None;
        }
        if let Ok(mut identity) = self.identity.write() {
            *identity = ClientIdentity::fresh();
        }
    }

    pub async fn init(&self) -> Result<(), ScraperError> {
        let path = "/wefeed-mobile-bff/tab-operating?page=1&tabId=0&version=";
        let _ = self.get(path).await?;

        let has_token = self.runtime_token.read().unwrap().is_some();
        if !has_token {
            return Err(ScraperError::MissingToken);
        }
        Ok(())
    }

    /// Init, rotating identity once if the first attempt fails.
    pub async fn init_resilient(&self) -> Result<(), ScraperError> {
        match self.init().await {
            Ok(()) => Ok(()),
            Err(_) => {
                self.refresh_identity();
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                self.init().await
            }
        }
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
            let mut write_token = self.runtime_token.write().unwrap();
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
        let start_idx = self.active_base_idx.load(Ordering::Relaxed);
        let mut last_error: Option<ScraperError> = None;

        // Two full host-pool passes: current identity, then rotated identity.
        for pass in 0..2 {
            if pass == 1 {
                self.refresh_identity();
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                // Best-effort token bootstrap after rotate.
                let _ = self
                    .request_once(
                        "GET",
                        "/wefeed-mobile-bff/tab-operating?page=1&tabId=0&version=",
                        None,
                        start_idx,
                    )
                    .await;
            }

            for i in 0..HOST_POOL.len() {
                if i > 0 {
                    let delay_ms = 80 + (i as u64 * 40);
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let idx = (start_idx + i) % HOST_POOL.len();
                match self
                    .request_once(method, path_and_query, body, idx)
                    .await
                {
                    Ok(val) => {
                        self.active_base_idx.store(idx, Ordering::Relaxed);
                        return Ok(val);
                    }
                    Err(err) => {
                        let should_retry = error_is_retryable(&err);
                        last_error = Some(err);
                        if !should_retry {
                            return Err(last_error.unwrap());
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or(ScraperError::HostsExhausted))
    }

    async fn request_once(
        &self,
        method: &str,
        path_and_query: &str,
        body: Option<&str>,
        host_idx: usize,
    ) -> Result<Value, ScraperError> {
        let base = HOST_POOL[host_idx % HOST_POOL.len()];
        let url = format!("{base}{path_and_query}");

        let token = self.runtime_token.read().unwrap().clone();
        let identity = self.identity.read().unwrap().clone();
        let headers = build_signed_headers(
            method,
            &url,
            body,
            token.as_deref(),
            &identity.user_agent,
            &identity.client_info,
            &identity.spoofed_ip,
        );

        let mut builder = match method {
            "POST" => self.client.post(&url),
            _ => self.client.get(&url),
        };

        builder = builder.headers(headers);
        if let Some(b) = body {
            builder = builder.body(b.to_string());
        }

        let resp = builder.send().await?;
        self.absorb_x_user(resp.headers()).await;
        let status = resp.status().as_u16();

        if RETRY_STATUS_CODES.contains(&status) {
            return Err(ScraperError::ApiStatus(status));
        }

        self.parse_response(resp).await
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
            match tokio::task::spawn_blocking(move || serde_json::from_str(&raw_text))
                .await
                .unwrap()
            {
                Ok(v) => v,
                Err(e) => return Err(ScraperError::Json(e)),
            };

        // Upstream wraps payloads as { code, message, data }.
        if let Some(code) = body_val.get("code").and_then(|c| c.as_i64()) {
            if code != 0 {
                let message = body_val
                    .get("message")
                    .or_else(|| body_val.get("msg"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown API error")
                    .to_string();
                return Err(ScraperError::ApiMessage { code, message });
            }
        }

        if let Some(data) = body_val.get("data") {
            if data.is_null() {
                return Err(ScraperError::ApiMessage {
                    code: body_val
                        .get("code")
                        .and_then(|c| c.as_i64())
                        .unwrap_or(-1),
                    message: body_val
                        .get("message")
                        .or_else(|| body_val.get("msg"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("empty response data")
                        .to_string(),
                });
            }
            Ok(data.clone())
        } else {
            Ok(body_val)
        }
    }
}

fn error_is_retryable(err: &ScraperError) -> bool {
    match err {
        ScraperError::ApiStatus(code) if RETRY_STATUS_CODES.contains(code) => true,
        ScraperError::ApiMessage { code, message } => {
            *code == 429
                || *code == 403
                || *code == -1
                || message_looks_like_rate_limit(message)
        }
        ScraperError::Reqwest(_)
        | ScraperError::Json(_)
        | ScraperError::MissingToken
        | ScraperError::HostsExhausted
        | ScraperError::ApiStatus(_) => true,
    }
}

fn message_looks_like_rate_limit(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("limit")
        || lower.contains("rate")
        || lower.contains("quota")
        || lower.contains("too many")
        || lower.contains("throttl")
        || lower.contains("frequen")
}
