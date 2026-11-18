use super::models::{
    AddonManifest, AddonResource, CatalogDefinition, CatalogExtra, MetaDetail, MetaItem, StreamItem,
};
use reqwest::Client;
use std::time::Duration;

/// Stream addons (Torrentio and friends) can need well over the 12s default
/// that suits catalogs, particularly with large provider lists.
const STREAM_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug)]
pub struct AddonClient {
    http: Client,
}

impl Default for AddonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AddonClient {
    pub fn new() -> Self {
        // Some addon hosts reject unknown clients; MOVIEBOX_ADDON_USER_AGENT
        // lets the user present a different one.
        let user_agent = std::env::var("MOVIEBOX_ADDON_USER_AGENT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "MovieBox-Tui/1.0 (Addon-Client)".to_string());
        let http = crate::net::http_client_builder()
            .timeout(Duration::from_secs(12))
            .connect_timeout(Duration::from_secs(8))
            .user_agent(user_agent)
            .build()
            .unwrap_or_default();
        Self { http }
    }

    pub fn normalize_manifest_url(raw: &str) -> String {
        let mut url = raw
            .trim()
            .trim_matches(|c: char| matches!(c, '"' | '\'' | '<' | '>'))
            .trim()
            .to_string();
        if let Some(rest) = url.strip_prefix("stremio://") {
            url = format!("https://{rest}");
        } else if !crate::net::is_http_url(&url) {
            url = format!("https://{url}");
        }
        let (base, suffix) = match url.find(['?', '#']) {
            Some(idx) => (&url[..idx], &url[idx..]),
            None => (url.as_str(), ""),
        };
        let base = base.trim_end_matches('/');
        // The "configure" page of an addon is not its manifest; Stremio's
        // install links point at /manifest.json beside it.
        let base = base.strip_suffix("/configure").unwrap_or(base);
        if base.ends_with("/manifest.json") {
            format!("{base}{suffix}")
        } else {
            format!("{base}/manifest.json{suffix}")
        }
    }

    pub fn base_addon_url(manifest_url: &str) -> String {
        let normalized = Self::normalize_manifest_url(manifest_url);
        if let Some(pos) = normalized.rfind("/manifest.json") {
            normalized[..pos].to_string()
        } else {
            normalized.trim_end_matches('/').to_string()
        }
    }

    pub async fn fetch_manifest(&self, manifest_url: &str) -> Result<AddonManifest, String> {
        let url = Self::normalize_manifest_url(manifest_url);
        let url_clone = url.clone();
        if let Ok(Some(manifest)) = tokio::task::spawn_blocking(move || {
            crate::cache::get_addon_manifest_cache_typed(&url_clone)
        })
        .await
        {
            if !manifest.name.trim().is_empty() {
                return Ok(manifest);
            }
        }

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach manifest: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Manifest returned HTTP {}", resp.status()));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;
        let manifest = parse_manifest_lenient(&raw)?;

        if manifest.name.trim().is_empty() {
            return Err("Addon manifest missing valid name".to_string());
        }

        let url_clone = url.clone();
        let manifest_clone = manifest.clone();
        tokio::task::spawn_blocking(move || {
            crate::cache::set_addon_manifest_cache_typed(&url_clone, &manifest_clone);
        });

        Ok(manifest)
    }

    pub async fn fetch_catalog_search(
        &self,
        base_url: &str,
        r#type: &str,
        catalog_id: &str,
        query: &str,
    ) -> Result<Vec<MetaItem>, String> {
        let encoded_query =
            percent_encoding::utf8_percent_encode(query.trim(), percent_encoding::NON_ALPHANUMERIC)
                .to_string();

        let endpoint =
            format!("{base_url}/catalog/{type}/{catalog_id}/search={encoded_query}.json");
        let resp = self
            .http
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| format!("Catalog search request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Catalog returned HTTP {}", resp.status()));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid catalog response JSON: {e}"))?;

        parse_catalog_metas(raw)
    }

    pub async fn fetch_catalog(
        &self,
        base_url: &str,
        r#type: &str,
        catalog_id: &str,
        extra: Option<&str>,
    ) -> Result<Vec<MetaItem>, String> {
        let endpoint = if let Some(ext) = extra.filter(|s| !s.trim().is_empty()) {
            format!("{base_url}/catalog/{type}/{catalog_id}/{ext}.json")
        } else {
            format!("{base_url}/catalog/{type}/{catalog_id}.json")
        };
        let resp = self
            .http
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| format!("Catalog request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Catalog returned HTTP {}", resp.status()));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid catalog response JSON: {e}"))?;

        parse_catalog_metas(raw)
    }

    pub async fn fetch_meta(
        &self,
        base_url: &str,
        r#type: &str,
        id: &str,
    ) -> Result<MetaDetail, String> {
        let endpoint = format!("{base_url}/meta/{type}/{id}.json");
        let resp = self
            .http
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| format!("Metadata request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Metadata returned HTTP {}", resp.status()));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid metadata response JSON: {e}"))?;

        let meta_target = if let Some(m) = raw.get("meta").filter(|m| m.is_object()) {
            m.clone()
        } else {
            raw
        };

        let detail: MetaDetail = serde_json::from_value(meta_target)
            .map_err(|e| format!("Invalid metadata detail format: {e}"))?;

        Ok(detail)
    }

    pub async fn fetch_streams(
        &self,
        base_url: &str,
        r#type: &str,
        id: &str,
    ) -> Result<Vec<StreamItem>, String> {
        let endpoint = format!("{base_url}/stream/{type}/{id}.json");
        let resp = self
            .http
            .get(&endpoint)
            .timeout(STREAM_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| format!("Streams request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Streams returned HTTP {}", resp.status()));
        }

        let mut raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid streams response JSON: {e}"))?;

        if let Some(val) = raw.get_mut("streams").map(serde_json::Value::take) {
            parse_stream_items(val)
        } else if raw.is_array() {
            parse_stream_items(raw)
        } else {
            Ok(Vec::new())
        }
    }
}

fn json_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn parse_manifest_resource(value: &serde_json::Value) -> Option<AddonResource> {
    match value {
        serde_json::Value::String(name) => Some(AddonResource::Simple(name.clone())),
        serde_json::Value::Object(obj) => {
            let name = obj.get("name")?.as_str()?.to_string();
            let list = |key: &str| {
                obj.get(key)
                    .map(|v| json_string_list(Some(v)))
                    .filter(|l| !l.is_empty())
            };
            Some(AddonResource::Detailed {
                name,
                types: list("types"),
                id_prefixes: list("idPrefixes"),
            })
        }
        _ => None,
    }
}

fn parse_manifest_catalog(value: &serde_json::Value) -> Option<CatalogDefinition> {
    let obj = value.as_object()?;
    let r#type = obj.get("type")?.as_str()?.to_string();
    let id = obj.get("id")?.as_str()?.to_string();
    let name = obj.get("name").and_then(|n| n.as_str()).map(str::to_string);
    let mut extra = Vec::new();
    if let Some(items) = obj.get("extra").and_then(|e| e.as_array()) {
        for item in items {
            if let Some(extra_obj) = item.as_object()
                && let Some(extra_name) = extra_obj.get("name").and_then(|n| n.as_str())
            {
                let options = extra_obj
                    .get("options")
                    .map(|v| json_string_list(Some(v)))
                    .filter(|o| !o.is_empty());
                extra.push(CatalogExtra {
                    name: extra_name.to_string(),
                    is_required: extra_obj
                        .get("isRequired")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(false),
                    options,
                });
            }
        }
    } else {
        // Older manifests list extras as plain names.
        let required = json_string_list(obj.get("extraRequired"));
        for extra_name in json_string_list(obj.get("extraSupported")) {
            let is_required = required.contains(&extra_name);
            extra.push(CatalogExtra {
                name: extra_name,
                is_required,
                options: None,
            });
        }
    }
    Some(CatalogDefinition {
        r#type,
        id,
        name,
        extra,
    })
}

/// Reads an addon manifest field by field. Real-world manifests carry
/// odd shapes (a numeric version, a catalog without an id, legacy extras);
/// one bad entry must not make the whole addon impossible to install.
pub(crate) fn parse_manifest_lenient(raw: &serde_json::Value) -> Result<AddonManifest, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| "Manifest is not a JSON object".to_string())?;
    // A Nuvio plugin repository lists JavaScript scrapers, not Stremio
    // resources. The installer recognises this error and installs the
    // scrapers instead (see `nuvio`).
    if obj.get("scrapers").is_some_and(|v| v.is_array()) && !obj.contains_key("resources") {
        return Err(super::nuvio::REPO_MANIFEST_ERROR.to_string());
    }
    let text = |key: &str| -> Option<String> {
        match obj.get(key)? {
            serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };
    let name = text("name")
        .or_else(|| text("id"))
        .ok_or_else(|| "Addon manifest missing valid name".to_string())?;
    let id = text("id").unwrap_or_else(|| name.clone());
    let resources: Vec<AddonResource> = obj
        .get("resources")
        .and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(parse_manifest_resource).collect())
        .unwrap_or_default();
    let catalogs: Vec<CatalogDefinition> = obj
        .get("catalogs")
        .and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(parse_manifest_catalog).collect())
        .unwrap_or_default();
    Ok(AddonManifest {
        id,
        name,
        version: text("version"),
        description: text("description"),
        resources,
        types: json_string_list(obj.get("types")),
        catalogs,
        id_prefixes: json_string_list(obj.get("idPrefixes")),
        logo: text("logo"),
        background: text("background"),
    })
}

/// Parses a `streams` array one entry at a time, so a single entry with an
/// unexpected shape (a number where a string is expected, say) is skipped
/// instead of discarding every stream the addon returned.
fn parse_stream_items(val: serde_json::Value) -> Result<Vec<StreamItem>, String> {
    let entries = match val {
        serde_json::Value::Array(entries) => entries,
        serde_json::Value::Null => return Ok(Vec::new()),
        _ => return Err("Failed to parse streams: expected an array".to_string()),
    };
    let total = entries.len();
    let mut items = Vec::with_capacity(total);
    let mut first_error = None;
    for entry in entries {
        match serde_json::from_value::<StreamItem>(entry) {
            Ok(item) => items.push(item),
            Err(e) => {
                first_error.get_or_insert_with(|| e.to_string());
            }
        }
    }
    if items.is_empty()
        && let Some(e) = first_error
    {
        return Err(format!("Failed to parse streams: {e}"));
    }
    if items.len() < total {
        log::warn!("skipped {} unreadable stream entries", total - items.len());
    }
    Ok(items)
}

fn parse_catalog_metas(mut raw: serde_json::Value) -> Result<Vec<MetaItem>, String> {
    if let Some(val) = raw.get_mut("metas").map(serde_json::Value::take) {
        serde_json::from_value(val).map_err(|e| format!("Failed to parse metas: {e}"))
    } else if let Some(val) = raw.get_mut("items").map(serde_json::Value::take) {
        serde_json::from_value(val).map_err(|e| format!("Failed to parse items: {e}"))
    } else if let Some(val) = raw.get_mut("results").map(serde_json::Value::take) {
        serde_json::from_value(val).map_err(|e| format!("Failed to parse items: {e}"))
    } else if raw.is_array() {
        serde_json::from_value(raw).map_err(|e| format!("Failed to parse raw array metas: {e}"))
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_page_urls_become_manifest_urls() {
        assert_eq!(
            AddonClient::normalize_manifest_url(
                "https://torrentio.strem.fun/providers=yts|limit=4/configure"
            ),
            "https://torrentio.strem.fun/providers=yts|limit=4/manifest.json"
        );
        assert_eq!(
            AddonClient::normalize_manifest_url("stremio://x.test/a/manifest.json"),
            "https://x.test/a/manifest.json"
        );
        assert_eq!(
            AddonClient::normalize_manifest_url("  x.test/  "),
            "https://x.test/manifest.json"
        );
        assert_eq!(
            AddonClient::normalize_manifest_url("https://x.test/a/manifest.json?k=v"),
            "https://x.test/a/manifest.json?k=v"
        );
    }

    #[test]
    fn odd_manifests_still_install() {
        let raw = serde_json::json!({
            "id": "org.example.odd",
            "name": "Odd Addon",
            "version": 2,
            "resources": ["stream", {"name": "catalog", "types": ["movie"]}, 7],
            "types": ["movie", 5, "series"],
            "idPrefixes": ["tt"],
            "catalogs": [
                {"type": "movie", "id": "top", "extraSupported": ["search", "skip"], "extraRequired": ["search"]},
                {"type": "movie"},
                {"type": "series", "id": "new", "extra": [{"name": "genre", "options": ["a", 1]}, {"isRequired": true}]}
            ]
        });
        let manifest = parse_manifest_lenient(&raw).unwrap();
        assert_eq!(manifest.version.as_deref(), Some("2"));
        assert!(manifest.provides_stream());
        assert_eq!(manifest.types, ["movie", "series"]);
        assert_eq!(manifest.catalogs.len(), 2);
        assert!(manifest.catalogs[0].extra[0].is_required);
        assert!(!manifest.catalogs[0].extra[1].is_required);
        assert_eq!(manifest.catalogs[1].extra.len(), 1);
        assert!(parse_manifest_lenient(&serde_json::json!({"resources": []})).is_err());
        assert!(parse_manifest_lenient(&serde_json::json!([1])).is_err());
        let nuvio = serde_json::json!({
            "name": "All-in-One-Nuvio",
            "version": "1.0.0",
            "scrapers": [{"id": "torrentio", "filename": "providers/torrentio.js"}]
        });
        let err = parse_manifest_lenient(&nuvio).unwrap_err();
        assert!(err.contains("Nuvio"));
    }

    #[test]
    fn one_unreadable_stream_does_not_drop_the_rest() {
        let raw = serde_json::json!([
            {"name": "ok", "url": "https://x.test/a.mp4"},
            {"name": "bad", "url": {"not": "a string"}},
            {"infoHash": "0123456789abcdef0123456789abcdef01234567", "fileIdx": 1}
        ]);
        let items = parse_stream_items(raw).unwrap();
        assert_eq!(items.len(), 2);
        assert!(
            parse_stream_items(serde_json::Value::Null)
                .unwrap()
                .is_empty()
        );
        assert!(parse_stream_items(serde_json::json!([{"url": {"a": 1}}])).is_err());
        assert!(parse_stream_items(serde_json::json!("nope")).is_err());
    }

    #[test]
    fn video_size_accepts_floats_and_numeric_strings() {
        let items = parse_stream_items(serde_json::json!([
            {"url": "https://x.test/a.mp4", "behaviorHints": {"videoSize": "1234.5"}},
            {"url": "https://x.test/b.mp4", "behaviorHints": {"videoSize": 2048.0}},
            {"url": "https://x.test/c.mp4", "behaviorHints": {"videoSize": "n/a"}}
        ]))
        .unwrap();
        let size = |i: usize| items[i].behavior_hints.as_ref().unwrap().video_size;
        assert_eq!(size(0), Some(1234));
        assert_eq!(size(1), Some(2048));
        assert_eq!(size(2), None);
    }
}
