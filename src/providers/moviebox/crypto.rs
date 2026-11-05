use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use md5::Md5;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const DEFAULT_SECRET_BYTES: &[u8] = b"\xef\xa8\x91\x97\x4e\xec\xd3\x14\x8d\xf6\x3a\xa6\x11\x60\x2d\xef\xd1\x01\x25\x9b\xa5\x21\x02\x2c\x57\xae\x05\x66\xbd\x8e";
const SIGNATURE_BODY_MAX_BYTES: usize = 102_400;

type HmacMd5 = Hmac<Md5>;

const STOCK_CERT_SHA256: &str = "03d4b741632d136bc23e2de4c6adcdb7712f1980b619c183221801730bbd0410";
const BUILD_TAG_SALT: &str = "oneroom";
const BUILD_TAG_HEADER: reqwest::header::HeaderName =
    reqwest::header::HeaderName::from_static("x-client-build");
const BUILD_TAG_INFO_KEY: &str = "X-Client-Build";

fn build_tag_stamp(ts_ms: u64, rand_part: u32) -> String {
    format!("{}{:03}000{}", ts_ms / 1000, ts_ms % 1000, rand_part)
}

fn build_tag_for_cert(ts_ms: u64, rand_part: u32, cert_sha256_hex: &str) -> String {
    let stamp = build_tag_stamp(ts_ms, rand_part);
    let hash = crate::cache::md5_hex_bytes(
        format!("{}{}{}", cert_sha256_hex, BUILD_TAG_SALT, stamp).as_bytes(),
    );
    format!("{}.{}", stamp, hash)
}

pub fn generate_x_client_build(ts_ms: u64) -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    build_tag_for_cert(ts_ms, rng.random_range(0..1000), STOCK_CERT_SHA256)
}

fn client_info_with_build_tag(client_info: &str, tag: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(client_info) else {
        return client_info.to_string();
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            BUILD_TAG_INFO_KEY.to_string(),
            serde_json::Value::String(tag.to_string()),
        );
    }
    serde_json::to_string(&value).unwrap_or_else(|_| client_info.to_string())
}

fn insert_header(
    headers: &mut reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
    value: &str,
) {
    match value.parse() {
        Ok(parsed) => {
            headers.insert(name, parsed);
        }
        Err(error) => {
            log::warn!("moviebox signing header skipped: {error}");
        }
    }
}

pub fn generate_x_client_token(ts: u64) -> String {
    let ts_str = ts.to_string();
    let reversed_ts: String = ts_str.chars().rev().collect();
    let hash_val = crate::cache::md5_hex_bytes(reversed_ts.as_bytes());
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
    let canonical_url = if let Ok(parsed) = Url::parse(url) {
        let path = parsed.path();
        let query = sorted_query_string(url);
        if query.is_empty() {
            path.to_string()
        } else {
            format!("{}?{}", path, query)
        }
    } else {
        url.to_string()
    };

    let body_bytes = body.map(|b| b.as_bytes());
    let (body_hash, body_length) = if let Some(bytes) = body_bytes {
        let len = bytes.len();
        let truncated = if len > SIGNATURE_BODY_MAX_BYTES {
            &bytes[..SIGNATURE_BODY_MAX_BYTES]
        } else {
            bytes
        };
        (crate::cache::md5_hex_bytes(truncated), len.to_string())
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
    let Ok(mut mac) = HmacMd5::new_from_slice(DEFAULT_SECRET_BYTES) else {
        log::warn!("moviebox signature fallback: invalid HMAC key material");
        return format!("{}|2|", timestamp_ms);
    };
    mac.update(canonical.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

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
        .unwrap_or_default()
        .as_millis() as u64;

    let accept = "application/json";
    let content_type = "application/json";

    let client_token = generate_x_client_token(ts);
    let signature =
        generate_x_tr_signature(method, Some(accept), Some(content_type), url, body, ts);
    let build_tag = generate_x_client_build(ts);
    let client_info = client_info_with_build_tag(client_info, &build_tag);

    let mut headers = reqwest::header::HeaderMap::new();

    insert_header(&mut headers, reqwest::header::USER_AGENT, user_agent);
    insert_header(&mut headers, reqwest::header::ACCEPT, accept);
    insert_header(&mut headers, reqwest::header::CONTENT_TYPE, content_type);
    insert_header(&mut headers, reqwest::header::CONNECTION, "keep-alive");
    insert_header(
        &mut headers,
        reqwest::header::HeaderName::from_static("x-client-token"),
        &client_token,
    );
    insert_header(
        &mut headers,
        reqwest::header::HeaderName::from_static("x-tr-signature"),
        &signature,
    );
    insert_header(&mut headers, BUILD_TAG_HEADER, &build_tag);
    insert_header(
        &mut headers,
        reqwest::header::HeaderName::from_static("x-client-info"),
        &client_info,
    );
    insert_header(
        &mut headers,
        reqwest::header::HeaderName::from_static("x-client-status"),
        "0",
    );
    insert_header(
        &mut headers,
        reqwest::header::HeaderName::from_static("x-forwarded-for"),
        spoofed_ip,
    );

    if let Some(token) = auth_token {
        let bearer = format!("Bearer {}", token);
        insert_header(&mut headers, reqwest::header::AUTHORIZATION, &bearer);
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
    let version_codes = [50020117, 50020118, 50020119, 50020120, 50020121];
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
        r#"{{"package_name":"com.community.oneroom","version_name":"4.0.01.0813.03","version_code":{},"os":"android","os_version":"{}","install_ch":"ps","device_id":"{}","install_store":"ps","gaid":"{}","brand":"{}","model":"{}","system_language":"en","net":"{}","region":"US","timezone":"{}","sp_code":"40401","X-Play-Mode":"2"}}"#,
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

    let prefixes: &[&str] = &[
        "103.241", "49.36", "117.195", "106.198", "122.162", "157.32", "182.70", "103.58", "27.60",
        "59.90",
    ];
    let prefix = prefixes[rng.random_range(0..prefixes.len())];
    let c: u8 = rng.random_range(1..254);
    let d: u8 = rng.random_range(1..254);
    format!("{}.{}.{}", prefix, c, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorted_query_string() {
        let url = "https://api.example.com/endpoint?b=2&a=1&c=3";
        assert_eq!(sorted_query_string(url), "a=1&b=2&c=3");

        let no_query = "https://api.example.com/endpoint";
        assert_eq!(sorted_query_string(no_query), "");
    }

    #[test]
    fn test_generate_x_client_token() {
        let token = generate_x_client_token(1700000000000);
        let parts: Vec<&str> = token.split(',').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "1700000000000");
        assert_eq!(parts[1].len(), 32);
    }

    #[test]
    fn test_generate_x_tr_signature() {
        let sig = generate_x_tr_signature(
            "GET",
            Some("application/json"),
            Some("application/json"),
            "https://api.example.com/path?tab=1&page=2",
            None,
            1700000000000,
        );
        let parts: Vec<&str> = sig.split('|').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "1700000000000");
        assert_eq!(parts[1], "2");
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn test_generate_client_info_and_ua() {
        let (ua, client_info) = generate_client_info_and_ua();
        assert!(ua.contains("com.community.oneroom/500201"));
        assert!(ua.contains("Cronet/135.0.7012.3"));

        let parsed: serde_json::Value =
            serde_json::from_str(&client_info).expect("valid JSON client_info");
        assert_eq!(parsed["version_name"], "4.0.01.0813.03");
        assert_eq!(parsed["package_name"], "com.community.oneroom");
        let code = parsed["version_code"]
            .as_i64()
            .expect("version_code is number");
        assert!((50020117..=50020121).contains(&code));
        assert_eq!(parsed["sp_code"], "40401");
        assert_eq!(parsed["X-Play-Mode"], "2");
    }

    fn test_cert_hex() -> String {
        use sha2::Digest;
        sha2::Sha256::digest(b"stock-certificate")
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    #[test]
    fn test_build_tag_matches_published_vectors() {
        assert_eq!(
            build_tag_for_cert(1700000000123, 45, &test_cert_hex()),
            "170000000012300045.1a8f8b991f95799fba169a8a7f889ebc"
        );
        assert_eq!(
            build_tag_for_cert(1700000000007, 7, &test_cert_hex()),
            "17000000000070007.d1c4e3525f7a8bf894ffdb6fcda377b2"
        );
    }

    #[test]
    fn test_generate_x_client_build_format() {
        let tag = generate_x_client_build(1700000000123);
        let (stamp, hash) = tag.split_once('.').expect("tag has stamp.hash shape");
        assert!((17..=19).contains(&stamp.len()));
        assert!(stamp.chars().all(|c| c.is_ascii_digit()));
        assert!(stamp.starts_with("1700000000123"));
        assert_eq!(hash.len(), 32);
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn test_client_info_with_build_tag_inserts_and_replaces() {
        let tagged = client_info_with_build_tag(r#"{"device_id":"d1"}"#, "stamp.hash");
        let parsed: serde_json::Value = serde_json::from_str(&tagged).expect("valid JSON");
        assert_eq!(parsed["device_id"], "d1");
        assert_eq!(parsed["X-Client-Build"], "stamp.hash");

        let replaced = client_info_with_build_tag(&tagged, "new.tag");
        let parsed: serde_json::Value = serde_json::from_str(&replaced).expect("valid JSON");
        assert_eq!(parsed["X-Client-Build"], "new.tag");

        assert_eq!(
            client_info_with_build_tag("not json", "stamp.hash"),
            "not json"
        );
    }

    #[test]
    fn test_build_signed_headers_include_build_tag() {
        let (_, client_info) = generate_client_info_and_ua();
        let headers = build_signed_headers(
            "GET",
            "https://api.example.com/path",
            None,
            None,
            "ua",
            &client_info,
            "1.2.3.4",
        );
        let tag = headers
            .get("x-client-build")
            .and_then(|v| v.to_str().ok())
            .expect("x-client-build header present");
        let (stamp, hash) = tag.split_once('.').expect("tag shape");
        assert!((17..=19).contains(&stamp.len()));
        assert_eq!(hash.len(), 32);

        let info = headers
            .get("x-client-info")
            .and_then(|v| v.to_str().ok())
            .expect("x-client-info header present");
        let parsed: serde_json::Value = serde_json::from_str(info).expect("valid JSON");
        assert_eq!(parsed["X-Client-Build"], tag);
    }
}
