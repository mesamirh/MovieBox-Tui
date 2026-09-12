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
    pub moviebox_enabled: bool,
    pub fourkhdhub_enabled: bool,
    pub bdix_circleftp_enabled: bool,
    pub bdix_dhakaflix_enabled: bool,
    pub bdix_probed: bool,
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
            moviebox_enabled: true,
            fourkhdhub_enabled: true,
            bdix_circleftp_enabled: false,
            bdix_dhakaflix_enabled: false,
            bdix_probed: false,
            streaming_enabled: true,
            tv_enabled: true,
            addons_enabled: false,
            default_player: None,
            download_dir: None,
        }
    }
}

pub const APP_NAME: &str = "moviebox-tui";

pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MOVIEBOX_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = dirs::config_dir() {
        return Some(dir.join(APP_NAME));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return Some(p.join(APP_NAME));
        }
    }
    if let Ok(prefix) = std::env::var("PREFIX") {
        let p = PathBuf::from(prefix).join("etc").join(APP_NAME);
        if p.exists() {
            return Some(p);
        }
    }
    dirs::home_dir().map(|h| h.join(".config").join(APP_NAME))
}

pub fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MOVIEBOX_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = dirs::data_dir() {
        return Some(dir.join(APP_NAME));
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return Some(p.join(APP_NAME));
        }
    }
    if let Ok(prefix) = std::env::var("PREFIX") {
        let p = PathBuf::from(prefix).join("var").join("lib").join(APP_NAME);
        if p.exists() {
            return Some(p);
        }
    }
    dirs::home_dir().map(|h| h.join(".local").join("share").join(APP_NAME))
}

pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MOVIEBOX_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = dirs::cache_dir() {
        return dir.join(APP_NAME);
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return p.join(APP_NAME);
        }
    }
    if let Ok(prefix) = std::env::var("PREFIX") {
        let p = PathBuf::from(prefix)
            .join("var")
            .join("cache")
            .join(APP_NAME);
        if p.exists() {
            return p;
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".cache").join(APP_NAME))
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
    let Some(path) = config_path() else {
        return Config::default();
    };
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mut val: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
            let old_bdix = val
                .get("bdix_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if let Some(obj) = val.as_object_mut() {
                obj.remove("bdix_enabled");
            }
            if let Ok(mut config) = serde_json::from_value::<Config>(val) {
                if old_bdix {
                    config.bdix_circleftp_enabled = true;
                    config.bdix_dhakaflix_enabled = true;
                }
                return config;
            }
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let corrupt_path = path.with_extension(format!("corrupt.{stamp}"));
        log::error!(
            "failed to parse config from {}, rotating to {}",
            crate::logging::sanitize_path(&path),
            crate::logging::sanitize_path(&corrupt_path)
        );
        let _ = std::fs::rename(&path, corrupt_path);
    }
    Config::default()
}

pub fn save(config: &Config) {
    let Some(path) = config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Err(error) = crate::cache::atomic_write_file(&path, json.as_bytes()) {
            log::warn!("failed to write config: {error}");
        }
    }
}

pub fn load_addons() -> Vec<InstalledAddon> {
    let mut list = if let Some(path) = addons_path() {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Vec<InstalledAddon>>(&content) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let corrupt_path = path.with_extension(format!("corrupt.{stamp}"));
                        log::error!(
                            "failed to parse addons config from {} ({e}), rotating to {}",
                            crate::logging::sanitize_path(&path),
                            crate::logging::sanitize_path(&corrupt_path)
                        );
                        let _ = std::fs::rename(&path, corrupt_path);
                        Vec::new()
                    }
                },
                Err(e) => {
                    log::warn!(
                        "failed to read addons config from {}: {e}",
                        crate::logging::sanitize_path(&path)
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_and_serde() {
        let config = Config::default();
        assert!(config.auto_update);
        assert_eq!(config.active_provider, ProviderKind::MovieBox);

        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.active_mode, config.active_mode);
    }
}
