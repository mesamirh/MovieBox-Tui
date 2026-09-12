use moviebox_tui::providers::moviebox::client::{MovieBoxClient, ScraperError};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use std::{collections::HashMap, env, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;

#[derive(Clone)]
struct AppState {
    client: MovieBoxClient,
    api_key: Option<Arc<str>>,
    allowed_origin: Arc<str>,
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    body: Value,
}

impl HttpResponse {
    fn ok(body: Value) -> Self {
        Self { status: 200, reason: "OK", body }
    }

    fn error(status: u16, reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            reason,
            body: json!({ "ok": false, "error": message.into() }),
        }
    }
}

fn upstream_error(err: ScraperError) -> HttpResponse {
    match &err {
        ScraperError::ApiStatus(404) => HttpResponse::error(404, "Not Found", err.to_string()),
        ScraperError::ApiStatus(429) => {
            HttpResponse::error(429, "Too Many Requests", err.to_string())
        }
        _ => HttpResponse::error(502, "Bad Gateway", err.to_string()),
    }
}

fn authorized(headers: &HashMap<String, String>, state: &AppState) -> bool {
    let Some(expected) = &state.api_key else {
        return true;
    };

    headers
        .get("x-api-key")
        .map(|value| value.as_str())
        == Some(expected.as_ref())
}

fn parse_headers(lines: impl Iterator<Item = String>) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    headers
}

fn safe_origin() -> Arc<str> {
    let origin = env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "*".to_string());
    let origin = origin.trim();

    if origin.is_empty() || origin.contains('\r') || origin.contains('\n') {
        Arc::<str>::from("*")
    } else {
        Arc::<str>::from(origin.to_string())
    }
}

async fn route_request(
    method: &str,
    target: &str,
    headers: &HashMap<String, String>,
    state: &AppState,
) -> HttpResponse {
    if method == "OPTIONS" {
        return HttpResponse {
            status: 204,
            reason: "No Content",
            body: Value::Null,
        };
    }

    if method != "GET" {
        return HttpResponse::error(405, "Method Not Allowed", "Only GET requests are supported");
    }

    let parsed = if target.starts_with("http://") || target.starts_with("https://") {
        Url::parse(target)
    } else {
        Url::parse(&format!("http://localhost{target}"))
    };

    let parsed = match parsed {
        Ok(url) => url,
        Err(_) => return HttpResponse::error(400, "Bad Request", "Invalid request URL"),
    };

    let path = parsed.path();

    if path == "/" {
        return HttpResponse::ok(json!({
            "name": "MovieBox metadata API",
            "status": "ok",
            "endpoints": [
                "GET /health",
                "GET /api/search?q=<query>&page=1",
                "GET /api/details/<subject_id>"
            ]
        }));
    }

    if path == "/health" {
        return HttpResponse::ok(json!({ "ok": true, "status": "healthy" }));
    }

    if !authorized(headers, state) {
        return HttpResponse::error(401, "Unauthorized", "Invalid or missing API key");
    }

    if path == "/api/search" {
        let params: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
        let query = params.get("q").map(|value| value.trim()).unwrap_or("");

        if query.is_empty() {
            return HttpResponse::error(400, "Bad Request", "q cannot be empty");
        }

        let page = params
            .get("page")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .clamp(1, 100);

        return match state.client.search(query, page).await {
            Ok(data) => HttpResponse::ok(json!({
                "ok": true,
                "query": query,
                "page": page,
                "data": data,
            })),
            Err(err) => upstream_error(err),
        };
    }

    if let Some(encoded_id) = path.strip_prefix("/api/details/") {
        let subject_id = percent_decode_str(encoded_id).decode_utf8_lossy();
        let subject_id = subject_id.trim();

        if subject_id.is_empty() || subject_id.len() > 128 || subject_id.contains('/') {
            return HttpResponse::error(400, "Bad Request", "Invalid subject id");
        }

        return match state.client.get_details(subject_id).await {
            Ok(data) => HttpResponse::ok(json!({ "ok": true, "data": data })),
            Err(err) => upstream_error(err),
        };
    }

    HttpResponse::error(404, "Not Found", "Endpoint not found")
}

async fn write_response(
    socket: &mut TcpStream,
    response: HttpResponse,
    allowed_origin: &str,
) -> std::io::Result<()> {
    let body = if response.status == 204 {
        String::new()
    } else {
        serde_json::to_string(&response.body).unwrap_or_else(|_| {
            "{\"ok\":false,\"error\":\"Failed to serialize response\"}".to_string()
        })
    };

    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, X-API-Key\r\nVary: Origin\r\n\r\n",
        response.status,
        response.reason,
        body.len(),
        allowed_origin,
    );

    socket.write_all(headers.as_bytes()).await?;
    if !body.is_empty() {
        socket.write_all(body.as_bytes()).await?;
    }
    socket.shutdown().await
}

async fn handle_connection(mut socket: TcpStream, state: AppState) -> std::io::Result<()> {
    const MAX_HEADER_BYTES: usize = 16 * 1024;
    let mut request = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 2048];

    loop {
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            return Ok(());
        }

        request.extend_from_slice(&chunk[..read]);

        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }

        if request.len() > MAX_HEADER_BYTES {
            let response = HttpResponse::error(
                431,
                "Request Header Fields Too Large",
                "Request headers are too large",
            );
            return write_response(&mut socket, response, state.allowed_origin.as_ref()).await;
        }
    }

    let request_text = String::from_utf8_lossy(&request);
    let mut lines = request_text.split("\r\n").map(str::to_string);
    let Some(request_line) = lines.next() else {
        return Ok(());
    };

    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("");
    let target = request_parts.next().unwrap_or("");

    if method.is_empty() || target.is_empty() {
        let response = HttpResponse::error(400, "Bad Request", "Malformed HTTP request");
        return write_response(&mut socket, response, state.allowed_origin.as_ref()).await;
    }

    let headers = parse_headers(lines.take_while(|line| !line.is_empty()));
    let response = route_request(method, target, &headers, &state).await;
    write_response(&mut socket, response, state.allowed_origin.as_ref()).await
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
        allowed_origin: safe_origin(),
    };

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(7860);

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind HTTP listener");

    println!("MovieBox API listening on 0.0.0.0:{port}");

    loop {
        let (socket, _) = match listener.accept().await {
            Ok(connection) => connection,
            Err(err) => {
                eprintln!("failed to accept connection: {err}");
                continue;
            }
        };

        let state = state.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket, state).await {
                eprintln!("request failed: {err}");
            }
        });
    }
}
