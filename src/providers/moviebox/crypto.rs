use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use md5::{Digest, Md5};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const SECRET_KEY_DEFAULT: &str = "76iRl07s0xSN9jqmEWAt79EBJZulIQIsV64FZr2O";
const SIGNATURE_BODY_MAX_BYTES: usize = 102_400;

type HmacMd5 = Hmac<Md5>;

fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn b64_decode(val: &str) -> Vec<u8> {
    let mut padded = val.to_string();
    let padding = (4 - padded.len() % 4) % 4;
    padded.push_str(&"=".repeat(padding));
    base64::engine::general_purpose::STANDARD
        .decode(padded)
        .unwrap_or_default()
}

fn b64_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn generate_x_client_token(ts: u64) -> String {
    let ts_str = ts.to_string();
    let reversed_ts: String = ts_str.chars().rev().collect();
    let hash_val = md5_hex(reversed_ts.as_bytes());
    format!("{},{}", ts_str, hash_val)
}

fn sorted_query_string(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return String::new();
    };

    let mut params = BTreeMap::new();
    for (k, v) in parsed.query_pairs() {
        params
            .entry(k.into_owned())
            .or_insert_with(Vec::new)
            .push(v.into_owned());
    }

    if params.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for (key, values) in params {
        for val in values {
            parts.push(format!("{}={}", key, val));
        }
    }
    parts.join("&")
}

pub fn build_canonical_string(
    method: &str,
    accept: Option<&str>,
    content_type: Option<&str>,
    url: &str,
    body: Option<&str>,
    timestamp_ms: u64,
) -> String {
    let parsed = Url::parse(url).expect("URL is valid by construction from base and path");
    let path = parsed.path();
    let query = sorted_query_string(url);
    let canonical_url = if query.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, query)
    };

    let body_bytes = body.map(|b| b.as_bytes());
    let (body_hash, body_length) = if let Some(bytes) = body_bytes {
        let len = bytes.len();
        let truncated = if len > SIGNATURE_BODY_MAX_BYTES {
            &bytes[..SIGNATURE_BODY_MAX_BYTES]
        } else {
            bytes
        };
        (md5_hex(truncated), len.to_string())
    } else {
        (String::new(), String::new())
    };

    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        method.to_uppercase(),
        accept.unwrap_or(""),
        content_type.unwrap_or(""),
        body_length,
        timestamp_ms,
        body_hash,
        canonical_url
    )
}

pub fn generate_x_tr_signature(
    method: &str,
    accept: Option<&str>,
    content_type: Option<&str>,
    url: &str,
    body: Option<&str>,
    timestamp_ms: u64,
) -> String {
    let canonical = build_canonical_string(method, accept, content_type, url, body, timestamp_ms);
    let secret_bytes = b64_decode(SECRET_KEY_DEFAULT);

    let mut mac = HmacMd5::new_from_slice(&secret_bytes).expect("HMAC can take key of any size");
    mac.update(canonical.as_bytes());
    let sig_b64 = b64_encode(&mac.finalize().into_bytes());

    format!("{}|2|{}", timestamp_ms, sig_b64)
}

pub fn build_signed_headers(
    method: &str,
    url: &str,
    body: Option<&str>,
    auth_token: Option<&str>,
    user_agent: &str,
    client_info: &str,
    spoofed_ip: &str,
) -> reqwest::header::HeaderMap {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is after UNIX EPOCH")
        .as_millis() as u64;

    let accept = "application/json";
    let content_type = "application/json";

    let client_token = generate_x_client_token(ts);
    let signature =
        generate_x_tr_signature(method, Some(accept), Some(content_type), url, body, ts);

    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert(reqwest::header::USER_AGENT, user_agent.parse().expect("Valid ASCII"));
    headers.insert("Accept", accept.parse().expect("Valid ASCII"));
    headers.insert("Content-Type", content_type.parse().expect("Valid ASCII"));
    headers.insert("Connection", "keep-alive".parse().expect("Valid ASCII"));
    headers.insert("X-Client-Token", client_token.parse().expect("Valid ASCII"));
    headers.insert("x-tr-signature", signature.parse().expect("Valid ASCII"));
    headers.insert("X-Client-Info", client_info.parse().expect("Valid ASCII"));
    headers.insert("X-Client-Status", "0".parse().expect("Valid ASCII"));

    headers.insert("X-Forwarded-For", spoofed_ip.parse().expect("Valid ASCII"));

    if let Some(token) = auth_token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().expect("Valid ASCII"),
        );
    }

    headers
}

pub(crate) fn generate_client_info_and_ua() -> (String, String) {
    use rand::RngExt;
    let mut rng = rand::rng();

    let android_versions = [
        ("9", "PQ3A.190605.03081104"),
        ("10", "QP1A.191005.007.A3"),
        ("11", "RP1A.200720.011"),
        ("12", "S1B.220414.015"),
        ("13", "TQ2A.230405.003"),
    ];
    let redmi_devices = [
        ("23078RKD5C", "Redmi"),
        ("2201117TY", "Redmi"),
        ("2201117TG", "Redmi"),
        ("22101316G", "Redmi"),
        ("21121210G", "Redmi"),
        ("M2012K11AG", "Redmi"),
        ("M2007J20CG", "Redmi"),
    ];
    let version_codes = [50020042, 50020043, 50020044, 50020045, 50020046];
    let network_types = ["NETWORK_WIFI", "NETWORK_MOBILE"];
    let timezones = [
        "Asia/Kolkata",
        "Asia/Shanghai",
        "Asia/Tokyo",
        "America/New_York",
        "Europe/London",
    ];

    let android = android_versions[rng.random_range(0..android_versions.len())];
    let device = redmi_devices[rng.random_range(0..redmi_devices.len())];
    let version_code = version_codes[rng.random_range(0..version_codes.len())];
    let network = network_types[rng.random_range(0..network_types.len())];
    let timezone = timezones[rng.random_range(0..timezones.len())];
    let gaid = random_uuid();
    let device_id = random_hex(32);

    let user_agent = format!(
        "com.community.oneroom/{} (Linux; U; Android {}; en_US; {}; Build/{}; Cronet/135.0.7012.3)",
        version_code, android.0, device.0, android.1
    );

    let client_info = format!(
        r#"{{"package_name":"com.community.oneroom","version_name":"3.0.03.0529.03","version_code":{},"os":"android","os_version":"{}","install_ch":"ps","device_id":"{}","install_store":"ps","gaid":"{}","brand":"{}","model":"{}","system_language":"en","net":"{}","region":"US","timezone":"{}","sp_code":"40401","X-Play-Mode":"2"}}"#,
        version_code, android.0, device_id, gaid, device.1, device.0, network, timezone
    );

    (user_agent, client_info)
}

fn random_hex(len: usize) -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    (0..len)
        .map(|_| format!("{:x}", rng.random_range(0..16)))
        .collect()
}

fn random_uuid() -> String {
    format!(
        "{}-{}-{}-{}-{}",
        random_hex(8),
        random_hex(4),
        random_hex(4),
        random_hex(4),
        random_hex(12)
    )
}

pub(crate) fn random_spoofed_ip() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let host: u8 = rng.random_range(1..254);
    format!("103.241.224.{}", host)
}
