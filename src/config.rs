use crate::providers::addons::models::InstalledAddon;
use crate::providers::models::ProviderKind;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auto_update: bool,
    pub last_update_check: u64,
    pub active_mode: String,
    pub active_provider: ProviderKind,
    pub active_theme: String,
    pub bdix_enabled: bool,
    pub streaming_enabled: bool,
    pub tv_enabled: bool,
    pub addons_enabled: bool,
    pub default_player: Option<String>,
    pub download_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_update: true,
            last_update_check: 0,
            active_mode: "streaming".to_string(),
            active_provider: ProviderKind::MovieBox,
            active_theme: String::new(),
            bdix_enabled: false,
            streaming_enabled: true,
            tv_enabled: true,
            addons_enabled: false,
            default_player: None,
            download_dir: None,
        }
    }
}

pub const APP_NAME: &str = "moviebox-tui";

/// Test-only escape hatch: several tests exercise `HistoryManager`/
/// `FavoritesManager`/`Config` save paths directly, which write to whatever
/// these functions return with no other isolation. Without this override
/// those saves land on the real, shared config/data directories — on a
/// machine where the app is also installed for actual use, that silently
/// overwrites real user data with test fixtures. Set `MOVIEBOX_CONFIG_DIR`
/// / `MOVIEBOX_DATA_DIR` to a throwaway directory before running `cargo
/// test` to avoid this; production behavior (unset) is unchanged.
pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MOVIEBOX_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::config_dir().map(|dir| dir.join(APP_NAME))
}

pub fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MOVIEBOX_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::data_dir().map(|dir| dir.join(APP_NAME))
}

pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MOVIEBOX_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .map(|dir| dir.join(APP_NAME))
        .unwrap_or_else(|| std::env::temp_dir().join(APP_NAME))
}

pub fn logs_dir() -> PathBuf {
    data_dir()
        .map(|dir| dir.join("logs"))
        .unwrap_or_else(|| std::env::temp_dir().join(APP_NAME).join("logs"))
}

pub fn scripts_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("scripts"))
}

pub fn playback_state_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("playback"))
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("config.json"))
}

pub fn addons_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("addons_config.json"))
}

pub fn tv_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("tv_config.json"))
}

pub fn history_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("history.json"))
}

pub fn favorites_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("favorites.json"))
}

pub fn load() -> Config {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| serde_json::from_str::<Config>(&c).ok())
        .unwrap_or_default()
}

pub fn save(config: &Config) {
    let Some(path) = config_path() else {
        return;
    };
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Err(error) = crate::cache::atomic_write_file(&path, json.as_bytes()) {
            log::warn!("failed to write config: {error}");
        }
    }
}

pub fn load_addons() -> Vec<InstalledAddon> {
    let mut list = addons_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<Vec<InstalledAddon>>(&content).ok())
        .unwrap_or_default();

    if !list.iter().any(|a| a.is_core()) {
        list.insert(0, InstalledAddon::cinemeta_default());
        save_addons(&list);
    } else {
        for a in &mut list {
            if a.is_core() {
                a.enabled = true;
            }
        }
    }
    list
}

pub fn save_addons(addons: &[InstalledAddon]) {
    let Some(path) = addons_path() else {
        return;
    };
    if let Some(app_dir) = path.parent()
        && std::fs::create_dir_all(app_dir).is_err()
    {
        return;
    }
    let Ok(json) = serde_json::to_string_pretty(addons) else {
        return;
    };
    if let Err(error) = crate::cache::atomic_write_file(&path, json.as_bytes()) {
        log::warn!("failed to write addons config: {error}");
    }
}
