use crate::models::{
    BrowseMetrics, CatalogItem, MediaDetails, ProviderKind, Release, SubtitleOption,
};
use crate::providers::addons::models::AddonManifest;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_EXPIRY_SECS: u64 = 24 * 60 * 60;
const STREAM_CACHE_EXPIRY_SECS: u64 = 2 * 60 * 60;
const HOMEPAGE_CACHE_EXPIRY_SECS: u64 = 60 * 60;
pub const CACHE_MAGIC: [u8; 4] = *b"MBC1";

#[derive(Serialize, Deserialize)]
pub struct CacheEnvelope<T> {
    pub version: u32,
    pub expires_at: u64,
    pub data: T,
}

pub fn get_typed_cache<T: DeserializeOwned + Serialize>(
    path: &Path,
    expiry_secs: u64,
) -> Option<T> {
    use std::io::Read;
    let mut file = fs::File::open(path).ok()?;
    let mut file_len = 0;
    if let Ok(metadata) = file.metadata() {
        if let Some(elapsed) = metadata.modified().ok().and_then(|m| m.elapsed().ok()) {
            if elapsed.as_secs() > expiry_secs {
                let _ = fs::remove_file(path);
                return None;
            }
        }
        file_len = metadata.len() as usize;
    }
    let mut bytes = Vec::with_capacity(file_len);
    if file.read_to_end(&mut bytes).is_err() {
        return None;
    }
    if bytes.len() >= 4 && bytes[0..4] == CACHE_MAGIC {
        if let Ok(envelope) = rmp_serde::from_slice::<CacheEnvelope<T>>(&bytes[4..]) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            if now < envelope.expires_at {
                return Some(envelope.data);
            } else {
                let _ = fs::remove_file(path);
                return None;
            }
        }
    }
    if let Ok(content) = String::from_utf8(bytes) {
        if let Ok(val) = serde_json::from_str::<T>(&content) {
            let _ = fs::remove_file(path);
            set_typed_cache(path, expiry_secs, &val);
            return Some(val);
        }
    }
    let _ = fs::remove_file(path);
    None
}

pub fn set_typed_cache<T: Serialize + ?Sized>(path: &Path, expiry_secs: u64, data: &T) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let envelope = CacheEnvelope {
        version: 1,
        expires_at: now + expiry_secs,
        data,
    };
    let mut file_bytes = Vec::with_capacity(512);
    file_bytes.extend_from_slice(&CACHE_MAGIC);
    if rmp_serde::encode::write(&mut file_bytes, &envelope).is_ok() {
        if let Err(error) = atomic_write_cache_file(path, &file_bytes) {
            log::warn!(
                "failed to write cache file {}: {error}",
                crate::logging::sanitize_path(path)
            );
        }
    } else {
        log::warn!(
            "failed to serialize cache envelope for {}",
            crate::logging::sanitize_path(path)
        );
    }
}

pub fn md5_hex_bytes(bytes: &[u8]) -> String {
    use md5::{Digest, Md5};
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = Md5::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut safe_query = String::with_capacity(32);
    for &b in &result {
        safe_query.push(HEX_CHARS[(b >> 4) as usize] as char);
        safe_query.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    safe_query
}

pub fn md5_hex(value: &str) -> String {
    md5_hex_bytes(value.as_bytes())
}

pub fn atomic_write_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("tmp-{}-{stamp}", std::process::id()));
    write_durable(&temporary, bytes)?;
    match durable_replace(&temporary, path, &format!("{stamp}-f")) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

pub fn atomic_write_cache_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("tmp-{}-{stamp}", std::process::id()));
    fs::write(&temporary, bytes)?;
    if fs::rename(&temporary, path).is_ok() {
        return Ok(());
    }
    let _ = fs::remove_file(path);
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

fn write_durable(target: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = fs::File::create(target)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn durable_replace(
    temporary: &std::path::Path,
    path: &std::path::Path,
    suffix: &str,
) -> std::io::Result<()> {
    if fs::rename(temporary, path).is_ok() {
        sync_parent_dir(path);
        return Ok(());
    }
    let _ = fs::remove_file(path);
    if fs::rename(temporary, path).is_ok() {
        sync_parent_dir(path);
        return Ok(());
    }
    let copy_target = path.with_extension(format!("tmp-{}-{suffix}", std::process::id()));
    let outcome = fs::copy(temporary, &copy_target)
        .and_then(|_| {
            let file = fs::File::open(&copy_target)?;
            file.sync_all()
        })
        .and_then(|_| {
            let _ = fs::remove_file(path);
            fs::rename(&copy_target, path)
        });
    let _ = fs::remove_file(&copy_target);
    match outcome {
        Ok(()) => {
            sync_parent_dir(path);
            Ok(())
        }
        Err(_) => Err(std::io::Error::other(format!(
            "atomic replace failed for {}",
            crate::logging::sanitize_path(path)
        ))),
    }
}

#[cfg(unix)]
fn sync_parent_dir(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &std::path::Path) {}

pub async fn atomic_write_file_async(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let path_buf = path.to_path_buf();
    let payload = bytes.to_vec();
    tokio::task::spawn_blocking(move || atomic_write_file(&path_buf, &payload))
        .await
        .map_err(|err| std::io::Error::other(format!("atomic write task failed: {err}")))?
}

pub fn get_provider_cache_dir(provider: ProviderKind, subdir: &str) -> PathBuf {
    crate::config::cache_dir()
        .join(provider.cache_key())
        .join(subdir)
}

pub fn get_provider_stream_path(
    provider: ProviderKind,
    subject_id: &str,
    season: usize,
    episode: usize,
) -> PathBuf {
    let mut path = get_provider_cache_dir(provider, "streams");
    let schema = match provider {
        ProviderKind::FourKHdHub => "v4_",
        // Discard streams cached with the DASH referer on recovered MP4 files.
        ProviderKind::MovieBox => "v2_",
        _ => "",
    };
    let hashed_id = md5_hex(subject_id);
    path.push(format!("{schema}{hashed_id}_{season}_{episode}.cache"));
    path
}

pub fn get_provider_stream_cache_typed(
    provider: ProviderKind,
    subject_id: &str,
    season: usize,
    episode: usize,
) -> Option<Vec<Release>> {
    let path = get_provider_stream_path(provider, subject_id, season, episode);
    let releases: Vec<Release> = get_typed_cache(&path, STREAM_CACHE_EXPIRY_SECS)?;
    if releases.is_empty() {
        return None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(min_exp) = min_stream_cookie_expiry(&releases) {
        if min_exp <= now.saturating_add(60) {
            let _ = fs::remove_file(&path);
            return None;
        }
    }
    Some(releases)
}

pub fn set_provider_stream_cache_typed(
    provider: ProviderKind,
    subject_id: &str,
    season: usize,
    episode: usize,
    releases: &[Release],
) {
    if releases.is_empty() {
        return;
    }
    let path = get_provider_stream_path(provider, subject_id, season, episode);
    let ttl = stream_cache_ttl_secs(releases);
    set_typed_cache(&path, ttl, releases);
}

fn min_stream_cookie_expiry(releases: &[Release]) -> Option<u64> {
    releases
        .iter()
        .flat_map(|r| &r.mirrors)
        .flat_map(|mirror| &mirror.headers)
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .flat_map(|(_, val)| val.split(';'))
        .filter_map(|part| {
            let trimmed = part.trim();
            if let Some(policy_raw) = trimmed.strip_prefix("CloudFront-Policy=") {
                cf_policy_date_less_than(policy_raw)
            } else if let Some(edge_cookie) = trimmed.strip_prefix("Edge-Cache-Cookie=") {
                edge_cache_cookie_expiry(edge_cookie)
            } else {
                None
            }
        })
        .min()
}

fn edge_cache_cookie_expiry(cookie_val: &str) -> Option<u64> {
    for field in cookie_val.split(':') {
        let trimmed = field.trim();
        if let Some(raw_ts) = trimmed.strip_prefix("t=") {
            if let Ok(ts) = raw_ts.parse::<u64>() {
                return Some(ts);
            }
        }
    }
    None
}

fn stream_cache_ttl_secs(releases: &[Release]) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let min_expiry = min_stream_cookie_expiry(releases);
    let remaining = min_expiry
        .and_then(|exp| exp.checked_sub(now))
        .unwrap_or(STREAM_CACHE_EXPIRY_SECS);
    remaining.clamp(60, STREAM_CACHE_EXPIRY_SECS)
}

fn cf_policy_date_less_than(policy_raw: &str) -> Option<u64> {
    use base64::Engine as _;
    let mut normalized: String = policy_raw
        .trim()
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '=',
            '~' => '/',
            other => other,
        })
        .collect();
    let padding = (4 - normalized.len() % 4) % 4;
    if padding > 0 {
        normalized.push_str(&"=".repeat(padding));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(normalized.as_bytes())
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json.get("Statement")?
        .as_array()?
        .first()?
        .get("Condition")?
        .get("DateLessThan")?
        .get("AWS:EpochTime")?
        .as_u64()
}

pub fn invalidate_provider_stream_cache(
    provider: ProviderKind,
    subject_id: &str,
    season: usize,
    episode: usize,
) {
    let path = get_provider_stream_path(provider, subject_id, season, episode);
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
}

pub fn get_provider_details_path(provider: ProviderKind, subject_id: &str) -> PathBuf {
    let mut path = get_provider_cache_dir(provider, "details");
    let schema = if provider == ProviderKind::FourKHdHub {
        "v2_"
    } else {
        ""
    };
    let hashed_id = md5_hex(subject_id);
    path.push(format!("details_{schema}{hashed_id}.cache"));
    path
}

pub fn get_provider_details_cache_typed(
    provider: ProviderKind,
    subject_id: &str,
) -> Option<MediaDetails> {
    let path = get_provider_details_path(provider, subject_id);
    get_typed_cache(&path, CACHE_EXPIRY_SECS)
}

pub fn set_provider_details_cache_typed(
    provider: ProviderKind,
    subject_id: &str,
    details: &MediaDetails,
) {
    let path = get_provider_details_path(provider, subject_id);
    set_typed_cache(&path, CACHE_EXPIRY_SECS, details);
}

pub fn invalidate_provider_details_cache(provider: ProviderKind, subject_id: &str) {
    let path = get_provider_details_path(provider, subject_id);
    let _ = fs::remove_file(path);
}

pub fn get_provider_search_path(provider: ProviderKind, query: &str, page: usize) -> PathBuf {
    let mut path = get_provider_cache_dir(provider, "search");
    let hashed = md5_hex(query);
    path.push(format!("{hashed}_{page}.cache"));
    path
}

pub fn get_provider_search_cache_typed(
    provider: ProviderKind,
    query: &str,
    page: usize,
) -> Option<Vec<CatalogItem>> {
    let path = get_provider_search_path(provider, query, page);
    let items: Vec<CatalogItem> = get_typed_cache(&path, CACHE_EXPIRY_SECS)?;
    (!items.is_empty()).then_some(items)
}

pub fn set_provider_search_cache_typed(
    provider: ProviderKind,
    query: &str,
    page: usize,
    items: &[CatalogItem],
) {
    if items.is_empty() {
        return;
    }
    let path = get_provider_search_path(provider, query, page);
    set_typed_cache(&path, CACHE_EXPIRY_SECS, items);
}

pub fn get_homepage_path(tab_id: &str, page: usize) -> PathBuf {
    let mut path = get_provider_cache_dir(ProviderKind::MovieBox, "homepage");
    path.push(format!("home_{tab_id}_{page}.cache"));
    path
}

pub fn get_homepage_cache_typed(
    tab_id: &str,
    page: usize,
) -> Option<(Vec<CatalogItem>, HashMap<String, BrowseMetrics>)> {
    let path = get_homepage_path(tab_id, page);
    get_typed_cache(&path, HOMEPAGE_CACHE_EXPIRY_SECS)
}

pub fn set_homepage_cache_typed(
    tab_id: &str,
    page: usize,
    data: &(Vec<CatalogItem>, HashMap<String, BrowseMetrics>),
) {
    let path = get_homepage_path(tab_id, page);
    set_typed_cache(&path, HOMEPAGE_CACHE_EXPIRY_SECS, data);
}

pub fn get_addon_catalog_path(manifest_url: &str, r_type: &str, catalog_id: &str) -> PathBuf {
    let mut path = get_provider_cache_dir(ProviderKind::Addons, "catalogs");
    let hashed = md5_hex(&format!("{manifest_url}_{r_type}_{catalog_id}"));
    path.push(format!("catalog_{hashed}.cache"));
    path
}

pub fn get_addon_catalog_cache_typed(
    manifest_url: &str,
    r_type: &str,
    catalog_id: &str,
) -> Option<Vec<CatalogItem>> {
    let path = get_addon_catalog_path(manifest_url, r_type, catalog_id);
    get_typed_cache(&path, HOMEPAGE_CACHE_EXPIRY_SECS)
}

pub fn set_addon_catalog_cache_typed(
    manifest_url: &str,
    r_type: &str,
    catalog_id: &str,
    items: &[CatalogItem],
) {
    let path = get_addon_catalog_path(manifest_url, r_type, catalog_id);
    set_typed_cache(&path, HOMEPAGE_CACHE_EXPIRY_SECS, items);
}

pub fn get_addon_manifest_path(manifest_url: &str) -> PathBuf {
    let mut path = get_provider_cache_dir(ProviderKind::Addons, "manifests");
    let hashed = md5_hex(manifest_url);
    path.push(format!("manifest_{hashed}.cache"));
    path
}

pub fn get_addon_manifest_cache_typed(manifest_url: &str) -> Option<AddonManifest> {
    let path = get_addon_manifest_path(manifest_url);
    get_typed_cache(&path, CACHE_EXPIRY_SECS)
}

pub fn set_addon_manifest_cache_typed(manifest_url: &str, manifest: &AddonManifest) {
    let path = get_addon_manifest_path(manifest_url);
    set_typed_cache(&path, CACHE_EXPIRY_SECS, manifest);
}

fn get_namespaced_image_path(namespace: &str, id: &str) -> PathBuf {
    let mut path = crate::config::cache_dir();
    path.push(namespace);
    path.push("images");
    let safe_name = md5_hex(id);
    path.push(format!("{safe_name}.img"));
    path
}

const IMAGE_CACHE_EXPIRY_SECS: u64 = 30 * 24 * 60 * 60;

fn check_image_file(path: &PathBuf) -> Option<Vec<u8>> {
    if path.exists() {
        if let Ok(metadata) = std::fs::metadata(path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = modified.elapsed()
            && elapsed.as_secs() > IMAGE_CACHE_EXPIRY_SECS
        {
            let _ = std::fs::remove_file(path);
            return None;
        }
        return std::fs::read(path).ok();
    }
    None
}

pub fn get_namespaced_image_cache(namespace: &str, id: &str) -> Option<Vec<u8>> {
    let path = get_namespaced_image_path(namespace, id);
    check_image_file(&path)
}

pub fn set_namespaced_image_cache(namespace: &str, id: &str, bytes: &[u8]) {
    let path = get_namespaced_image_path(namespace, id);
    if let Err(error) = atomic_write_cache_file(&path, bytes) {
        log::warn!(
            "failed to commit image cache to {}: {error}",
            crate::logging::sanitize_path(&path)
        );
    }
}

pub fn get_captions_path(subject_id: &str, resource_id: &str) -> PathBuf {
    let mut path = get_provider_cache_dir(ProviderKind::MovieBox, "captions");
    let hashed_id = md5_hex(&format!("{}_{}", subject_id, resource_id));
    path.push(format!("captions_{}.cache", hashed_id));
    path
}

pub fn get_captions_cache_typed(
    subject_id: &str,
    resource_id: &str,
) -> Option<Vec<SubtitleOption>> {
    let path = get_captions_path(subject_id, resource_id);
    get_typed_cache(&path, CACHE_EXPIRY_SECS)
}

pub fn set_captions_cache_typed(subject_id: &str, resource_id: &str, captions: &[SubtitleOption]) {
    let path = get_captions_path(subject_id, resource_id);
    set_typed_cache(&path, CACHE_EXPIRY_SECS, captions);
}

#[cfg_attr(windows, allow(clippy::permissions_set_readonly_false))]
fn resilient_remove_file(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(windows)]
            if let Ok(metadata) = fs::metadata(path) {
                let mut permissions = metadata.permissions();
                if permissions.readonly() {
                    permissions.set_readonly(false);
                    if fs::set_permissions(path, permissions).is_ok() {
                        return fs::remove_file(path);
                    }
                }
            }
            Err(err)
        }
    }
}

pub fn clear_directory_contents(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let mut errors = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(());
            }
            return Err(format!("failed to read directory {}: {err}", dir.display()));
        }
    };

    for entry in entries.flatten() {
        let child_path = entry.path();
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => {
                if let Err(err) = clear_directory_contents(&child_path) {
                    errors.push(err);
                }
                if let Err(err) = fs::remove_dir(&child_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        errors.push(format!(
                            "failed to remove directory {}: {err}",
                            child_path.display()
                        ));
                    }
                }
            }
            _ => {
                if let Err(err) = resilient_remove_file(&child_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        errors.push(format!(
                            "failed to remove file {}: {err}",
                            child_path.display()
                        ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn purge_subtitle_cache_files(dir: &Path, errors: &mut Vec<String>) {
    if !dir.exists() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!(
                    "failed to read subtitle directory {}: {err}",
                    dir.display()
                ));
            }
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let lower = ext.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "srt" | "vtt" | "sub" | "ass" | "ssa" | "tmp" | "log"
                ) {
                    if let Err(err) = resilient_remove_file(&path) {
                        if err.kind() != std::io::ErrorKind::NotFound {
                            errors.push(format!(
                                "failed to remove subtitle file {}: {err}",
                                path.display()
                            ));
                        }
                    }
                }
            }
        }
    }
}

pub fn clear_all_cache() -> Result<(), String> {
    let mut errors = Vec::new();
    let path = crate::config::cache_dir();
    if path.exists() {
        if let Err(err) = clear_directory_contents(&path) {
            errors.push(err);
        }
    }
    if let Some(data_dir) = crate::config::data_dir() {
        let legacy = data_dir.join("iptv_cache");
        if legacy.exists() {
            if let Err(err) = clear_directory_contents(&legacy) {
                errors.push(err);
            }
            let _ = fs::remove_dir(&legacy);
        }
    }
    let sub_dir = crate::service::resolve_subtitle_dir();
    if sub_dir.exists() {
        purge_subtitle_cache_files(&sub_dir, &mut errors);
    }
    let temp_subs = std::env::temp_dir().join("moviebox-tui").join("subs");
    if temp_subs.exists() {
        purge_subtitle_cache_files(&temp_subs, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn clean_old_cache_background() {
    tokio::task::spawn_blocking(|| {
        let path = crate::config::cache_dir();
        if !path.exists() {
            return;
        }

        let max_age = 7 * 24 * 60 * 60;

        let mut dirs_to_check = vec![path];
        while let Some(dir) = dirs_to_check.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            dirs_to_check.push(entry.path());
                        } else if metadata.is_file() {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(elapsed) = modified.elapsed() {
                                    if elapsed.as_secs() > max_age {
                                        let _ = fs::remove_file(entry.path());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mbx_cache_{}_{}_{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn test_typed_binary_cache_envelope_roundtrip() {
        let dir = unique_dir("typed_binary");
        let cache_file = dir.join("test_item.cache");

        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct SampleData {
            id: String,
            score: u64,
        }

        let original = SampleData {
            id: "movie_123".to_string(),
            score: 999,
        };

        set_typed_cache(&cache_file, 3600, &original);
        let loaded: Option<SampleData> = get_typed_cache(&cache_file, 3600);

        assert_eq!(loaded, Some(original));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_legacy_json_cache_auto_migration_to_msgpack() {
        let dir = unique_dir("legacy_json");
        let cache_file = dir.join("legacy.cache");

        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct MediaSample {
            title: String,
            year: u32,
        }

        let json_text = r#"{"title":"Inception","year":2010}"#;
        fs::write(&cache_file, json_text).expect("write legacy json");

        let loaded: Option<MediaSample> = get_typed_cache(&cache_file, 3600);
        assert_eq!(
            loaded,
            Some(MediaSample {
                title: "Inception".to_string(),
                year: 2010,
            })
        );

        let binary_bytes = fs::read(&cache_file).expect("read migrated binary file");
        assert_eq!(binary_bytes[0..4], CACHE_MAGIC);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_typed_cache_expiration() {
        let dir = unique_dir("expired_binary");
        let cache_file = dir.join("expired.cache");

        set_typed_cache(&cache_file, 0, &"secret_data".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        let loaded: Option<String> = get_typed_cache(&cache_file, 0);

        assert_eq!(loaded, None);
        assert!(!cache_file.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrite_replaces_content_and_leaves_no_temporaries() {
        let dir = unique_dir("overwrite");
        let target = dir.join("state.cache");
        atomic_write_file(&target, b"v1").expect("first write");
        atomic_write_file(&target, b"v2").expect("second write");
        assert_eq!(fs::read_to_string(&target).unwrap(), "v2");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains("tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporaries left behind: {leftovers:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_replace_preserves_existing_destination() {
        let dir = unique_dir("preserve");
        let destination = dir.join("not-a-file-target-dir");
        fs::create_dir_all(&destination).expect("destination directory");
        atomic_write_file(&destination.parent().unwrap().join("seed"), b"seed").ok();
        let result = atomic_write_file(&destination, b"payload");
        assert!(result.is_err());
        assert!(destination.is_dir(), "existing destination was destroyed");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains("tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporaries left behind: {leftovers:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn async_overwrite_replaces_content() {
        let dir = unique_dir("async");
        let target = dir.join("state.cache");
        atomic_write_file_async(&target, b"one")
            .await
            .expect("write");
        atomic_write_file_async(&target, b"two")
            .await
            .expect("write");
        assert_eq!(tokio::fs::read_to_string(&target).await.unwrap(), "two");
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn test_md5_hex_determinism() {
        assert_eq!(
            md5_hex("https://example.com/stream.m3u8"),
            "6d61b1f81ddc272d47bef071abd229bd"
        );
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_ne!(md5_hex("query1"), md5_hex("query2"));
    }
    #[test]
    fn test_clear_directory_contents_removes_nested_entries_and_preserves_root() {
        let dir = unique_dir("clear_nested");
        let sub1 = dir.join("moviebox").join("details");
        let sub2 = dir.join("moviebox").join("images");
        let sub3 = dir.join("tv_playlists");
        fs::create_dir_all(&sub1).expect("create sub1");
        fs::create_dir_all(&sub2).expect("create sub2");
        fs::create_dir_all(&sub3).expect("create sub3");

        fs::write(sub1.join("detail_1.cache"), b"details").expect("write detail");
        fs::write(sub2.join("img_1.bin"), b"image").expect("write img");
        fs::write(sub3.join("channels.m3u"), b"m3u").expect("write m3u");
        fs::write(dir.join("root_file.tmp"), b"root").expect("write root file");

        let result = clear_directory_contents(&dir);
        assert!(result.is_ok());
        assert!(dir.exists());
        let remaining = fs::read_dir(&dir).expect("read dir").count();
        assert_eq!(remaining, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clear_directory_contents_handles_nonexistent_directory() {
        let dir = unique_dir("nonexistent");
        let missing = dir.join("missing_sub");
        let result = clear_directory_contents(&missing);
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clear_all_cache_returns_ok() {
        let result = clear_all_cache();
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_cache_cookie_expiry_parsing() {
        let cookie = "urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20v:sign=abc:t=1790486259";
        assert_eq!(edge_cache_cookie_expiry(cookie), Some(1790486259));

        let cookie_no_t = "urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20v:sign=abc";
        assert_eq!(edge_cache_cookie_expiry(cookie_no_t), None);

        let cookie_invalid_t =
            "urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20v:sign=abc:t=not_a_number";
        assert_eq!(edge_cache_cookie_expiry(cookie_invalid_t), None);
    }

    #[test]
    fn test_min_stream_cookie_expiry_with_edge_cache_cookie() {
        let release = Release {
            provider: ProviderKind::MovieBox,
            filename: "Test S01E01".to_string(),
            quality: Some("1080p".to_string()),
            codec: None,
            language: None,
            size_bytes: None,
            season: Some(1),
            episode: Some(1),
            mirrors: vec![crate::models::SourceMirror {
                label: "MovieBox CDN".to_string(),
                resolver_url: "https://example.com/index.mpd".to_string(),
                headers: vec![(
                    "Cookie".to_string(),
                    "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20v:sign=abc:t=1899999999".to_string(),
                )],
                direct_file: true,
            }],
            resource_id: Some("res1".to_string()),
        };
        assert_eq!(min_stream_cookie_expiry(&[release]), Some(1899999999));
    }
}
