use tokio::sync::mpsc;

use crate::providers::models::RequestContext;
use crate::tui::{action::Action, state::AppState, theme::Theme};

mod addons;
mod download;
mod favorites;
mod keyboard;
mod mouse;
mod navigation;
mod network;
mod playback;
mod requests;
mod run;
mod search;
mod system;
mod tv;

pub struct App {
    state: AppState,
    theme: Theme,
    service: std::sync::Arc<crate::service::MovieBoxService>,
    action_sender: mpsc::UnboundedSender<Action>,
    action_receiver: mpsc::UnboundedReceiver<Action>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (action_sender, action_receiver) = mpsc::unbounded_channel();
        let mut state = AppState::default();

        let config = crate::tui::config::load();
        state.auto_update = config.auto_update;
        state.last_update_check = config.last_update_check;
        state.bdix_enabled = config.bdix_enabled;
        state.streaming_enabled = config.streaming_enabled;
        state.tv_enabled = config.tv_enabled;
        state.addons_enabled = config.addons_enabled;
        if !state.streaming_enabled && !state.tv_enabled && !state.addons_enabled {
            state.streaming_enabled = true;
        }
        let provider_was_sanitized = !state.bdix_enabled && config.active_provider.is_bdix();
        state.active_provider = if provider_was_sanitized {
            crate::providers::models::ProviderKind::MovieBox
        } else {
            config.active_provider
        };

        match config.active_mode.as_str() {
            "tv" if state.tv_enabled => {
                state.set_mode(crate::tui::state::AppMode::Tv);
            }
            "addon" if state.addons_enabled => {
                state.set_mode(crate::tui::state::AppMode::Addon);
                state.active_provider = crate::providers::models::ProviderKind::Addons;
            }
            _ => {
                if state.streaming_enabled {
                    state.set_mode(crate::tui::state::AppMode::Streaming);
                } else if state.tv_enabled {
                    state.set_mode(crate::tui::state::AppMode::Tv);
                } else if state.addons_enabled {
                    state.set_mode(crate::tui::state::AppMode::Addon);
                    state.active_provider = crate::providers::models::ProviderKind::Addons;
                } else {
                    state.set_mode(crate::tui::state::AppMode::Streaming);
                }
            }
        }
        state.active_theme_kind = config.active_theme;
        state.default_player = config.default_player;
        state.download_dir = config.download_dir.map(std::path::PathBuf::from);
        state.installed_addons = crate::config::load_addons();

        let theme_str = std::env::var("MOVIEBOX_THEME").unwrap_or_else(|_| {
            if state.active_theme_kind.is_empty() {
                "Mocha".to_string()
            } else {
                state.active_theme_kind.clone()
            }
        });
        let theme_kind = crate::tui::theme::ThemeKind::parse(&theme_str);
        state.active_theme_kind = theme_kind.as_str().to_string();
        let theme = crate::tui::theme::Theme::from_kind(theme_kind);

        let service = std::sync::Arc::new(crate::service::MovieBoxService::new());
        if service.fourk_client.is_none() {
            log::warn!("4KHDHub client unavailable; provider will be disabled");
        }

        let mut app = Self {
            theme,
            state,
            service,
            action_sender,
            action_receiver,
        };
        if app.state.is_tv_mode {
            app.load_tv_playlists_from_config();
            app.reload_tv_playlists();
        } else if app.state.is_addon_mode {
            app.load_installed_addons_from_config();
        }
        if provider_was_sanitized {
            app.persist_config();
        }
        app
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn request_context(&self) -> RequestContext {
        RequestContext {
            provider: self.state.active_provider,
            generation: self.state.provider_generation,
        }
    }

    fn context_is_current(&self, context: RequestContext) -> bool {
        context.provider == self.state.active_provider
            && context.generation == self.state.provider_generation
    }

    fn persist_config(&self) {
        let active_mode = match self.state.mode() {
            crate::tui::state::AppMode::Tv => "tv",
            crate::tui::state::AppMode::Addon => "addon",
            crate::tui::state::AppMode::Streaming => "streaming",
        };
        let config = crate::tui::config::Config {
            auto_update: self.state.auto_update,
            last_update_check: self.state.last_update_check,
            active_mode: active_mode.to_string(),
            active_provider: self.state.active_provider,
            active_theme: self.state.active_theme_kind.clone(),
            bdix_enabled: self.state.bdix_enabled,
            streaming_enabled: self.state.streaming_enabled,
            tv_enabled: self.state.tv_enabled,
            addons_enabled: self.state.addons_enabled,
            default_player: self.state.default_player.clone(),
            download_dir: self
                .state
                .download_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        };
        crate::tui::config::save(&config);
    }

    fn save_installed_addons(&self) {
        crate::config::save_addons(&self.state.installed_addons);
    }

    fn load_installed_addons_from_config(&mut self) {
        self.state.installed_addons = crate::config::load_addons();
    }

    fn save_tv_playlists(&self) {
        let Some(path) = crate::config::tv_path() else {
            return;
        };
        if let Some(app_dir) = path.parent()
            && std::fs::create_dir_all(app_dir).is_err()
        {
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(&self.state.tv_playlists) else {
            return;
        };
        if let Err(error) = crate::cache::atomic_write_file(&path, json.as_bytes()) {
            log::warn!("failed to save tv playlists: {error}");
        }
    }

    fn load_tv_playlists_from_config(&mut self) {
        self.state.tv_playlists.clear();
        let Some(path) = crate::config::tv_path() else {
            return;
        };
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&content)
        {
            let mut seen = std::collections::HashSet::new();
            self.state.tv_playlists = list
                .into_iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty() && seen.insert(item.clone()))
                .collect();
            if self.state.tv_playlists.is_empty() {
                let _ = std::fs::remove_file(path);
            }
        } else if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    fn reload_tv_playlists(&self) {
        let playlists = self.state.tv_playlists.clone();
        let sender = self.action_sender.clone();
        tokio::spawn(async move {
            let parser = crate::providers::tv::M3UParser::new();
            let mut all_channels = Vec::new();
            let mut failed = 0usize;
            for source in &playlists {
                match parser.fetch_playlist(source).await {
                    Ok(channels) => all_channels.extend(channels),
                    Err(error) => {
                        failed += 1;
                        log::warn!("tv playlist failed ({source}): {error}");
                    }
                }
            }
            sender
                .send(Action::TvChannelsLoaded(all_channels, failed))
                .ok();
        });
    }

    fn tv_manager_activate(&mut self) {
        use crate::tui::state::TvManagerRow;
        let Some(row) = self
            .state
            .tv_manager_rows()
            .get(self.state.tv_manager_selected)
            .copied()
        else {
            return;
        };
        match row {
            TvManagerRow::Playlist(index) => {
                self.action_sender
                    .send(Action::TvPlaylistRemove(index))
                    .ok();
            }
            TvManagerRow::AddUrl => {
                self.action_sender.send(Action::TvInputToggle(false)).ok();
            }
            TvManagerRow::AddFile => {
                self.action_sender.send(Action::TvInputToggle(true)).ok();
            }
            TvManagerRow::Reload => {
                self.action_sender.send(Action::TvReloadPlaylists).ok();
            }
            TvManagerRow::Done => {
                self.reset_transient_overlays();
                self.state.tv_config_popup = false;
            }
            TvManagerRow::Header(_) => {}
        }
    }

    fn addon_manager_activate(&mut self) {
        use crate::tui::state::AddonManagerRow;
        let Some(row) = self
            .state
            .addon_manager_rows()
            .get(self.state.addon_manager_selected)
            .copied()
        else {
            return;
        };
        match row {
            AddonManagerRow::Addon(index) => {
                self.action_sender
                    .send(Action::AddonToggleEnabled(index))
                    .ok();
            }
            AddonManagerRow::AddUrl => {
                self.action_sender.send(Action::AddonInputToggle(true)).ok();
            }
            AddonManagerRow::Header(_) => {}
        }
    }
}
