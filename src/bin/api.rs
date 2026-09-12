use axum::{
    extract::{Path, Query, State},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use moviebox_tui::providers::moviebox::client::{MovieBoxClient, ScraperError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{env, sync::Arc};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    client: MovieBoxClient,
    api_key: Option<Arc<str>>,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into() }
    }

    fn unauthorized() -> Self {
        Self { status: StatusCode::UNAUTHORIZED, message: "Invalid or missing API key".to_string() }
    }

    fn upstream(err: ScraperError) -> Self {
        let status = match err {
            ScraperError::ApiStatus(404) => StatusCode::NOT_FOUND,
            ScraperError::ApiStatus(429) => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::BAD_GATEWAY,
        };
        Self { status, message: err.to_string() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(json!({
                "ok": false,
                "error": self.message,
            })),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    page: Option<usize>,
}

fn authorize(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
    let Some(expected) = &state.api_key else {
        return Ok(());
    };

    let supplied = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());

    if supplied == Some(expected.as_ref()) {
        Ok(())
    } else {
        Err(ApiError::unauthorized())
    }
}

async fn root() -> Json<Value> {
    Json(json!({
        "name": "MovieBox metadata API",
        "status": "ok",
        "endpoints": [
            "GET /health",
            "GET /api/search?q=<query>&page=1",
            "GET /api/details/<subject_id>"
        ]
    }))
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true, "status": "healthy" }))
}

async fn search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, ApiError> {
    authorize(&headers, &state)?;

    let query = params.q.trim();
    if query.is_empty() {
        return Err(ApiError::bad_request("q cannot be empty"));
    }

    let page = params.page.unwrap_or(1).clamp(1, 100);
    let data = state
        .client
        .search(query, page)
        .await
        .map_err(ApiError::upstream)?;

    Ok(Json(json!({
        "ok": true,
        "query": query,
        "page": page,
        "data": data,
    })))
}

async fn details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(subject_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    authorize(&headers, &state)?;

    let subject_id = subject_id.trim();
    if subject_id.is_empty() || subject_id.len() > 128 {
        return Err(ApiError::bad_request("invalid subject id"));
    }

    let data = state
        .client
        .get_details(subject_id)
        .await
        .map_err(ApiError::upstream)?;

    Ok(Json(json!({
        "ok": true,
        "data": data,
    })))
}

fn cors_layer() -> CorsLayer {
    let api_key_header = HeaderName::from_static("x-api-key");

    match env::var("ALLOWED_ORIGIN") {
        Ok(origin) if !origin.trim().is_empty() && origin.trim() != "*" => {
            let origin = origin
                .parse::<HeaderValue>()
                .expect("ALLOWED_ORIGIN must be a valid HTTP origin");
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET])
                .allow_headers([CONTENT_TYPE, api_key_header])
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET])
            .allow_headers(Any),
    }
}

#[tokio::main]
async fn main() {
    let api_key = env::var("MOVIEBOX_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(Arc::<str>::from);

    let state = AppState {
        client: MovieBoxClient::new(),
        api_key,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/search", get(search))
        .route("/api/details/{subject_id}", get(details))
        .with_state(state)
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http());

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(7860);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind HTTP listener");

    println!("MovieBox API listening on 0.0.0.0:{port}");

    axum::serve(listener, app)
        .await
        .expect("HTTP server terminated unexpectedly");
}
