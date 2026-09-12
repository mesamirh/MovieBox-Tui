use super::App;
use crate::tui::action::Action;

impl App {
    pub(super) fn reset_transient_overlays(&mut self) {
        self.state.show_help = false;
        self.state.show_theme_popup = false;
        self.state.show_browse_popup = false;
        self.state.show_provider_popup = false;
        self.state.provider_list_state.select(None);
        self.state.show_settings_popup = false;
        self.state.settings_download_dir_input = None;
        self.state.player_picker_popup = false;
        self.state.settings_player_picker = false;

        self.state.subtitle_popup = false;
        self.state.is_download_subtitle_popup = false;
        self.state.pending_play_link = None;
        self.state.subtitle_list.clear();
        self.state.subtitle_list_state.select(None);
        self.state.show_season_download_confirm = false;
        self.state.show_episode_download_confirm = false;
        self.state.is_resolving_playback = false;
        self.state.tv_input_active = false;
        self.state.tv_input_buffer.clear();
        self.state.tv_input_is_file = false;
    }

    pub(super) fn reset_mode_state(&mut self) {
        self.state
            .fetch_cancel
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.state.fetch_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.state.provider_generation = self.state.provider_generation.wrapping_add(1);
        self.state.active_preview_request = self.state.active_preview_request.wrapping_add(1);
        self.state.active_search_request = self.state.active_search_request.wrapping_add(1);
        self.state.active_details_request = self.state.active_details_request.wrapping_add(1);
        self.state.active_resource_request = self.state.active_resource_request.wrapping_add(1);
        self.state.tick_count = 0;
        self.reset_transient_overlays();
        self.state.input_mode = crate::tui::state::InputMode::Normal;
        self.state.is_loading = false;
        self.state.clear_search_state();
        self.state.clear_details_state();
    }

    pub(super) fn announce_mode(&mut self) {
        match self.state.mode() {
            crate::tui::state::AppMode::Streaming => {
                self.state.set_status_default(format!(
                    "Streaming mode active ({}).",
                    self.state.active_provider.label()
                ));
            }
            crate::tui::state::AppMode::Tv => {
                self.state.set_status_long("Loading TV playlists...");
            }
        }
    }

    pub(super) async fn handle_tv(&mut self, action: Action) -> Option<()> {
        match action {
            Action::ToggleTvMode => {
                self.reset_mode_state();
                let will_be_tv = !self.state.is_tv_mode;
                if will_be_tv {
                    self.state.set_mode(crate::tui::state::AppMode::Tv);
                } else {
                    self.state.set_mode(crate::tui::state::AppMode::Streaming);
                }
                if self.state.is_tv_mode {
                    self.state.tv_config_popup = false;
                    self.state.tv_channels.clear();
                    self.announce_mode();
                    self.load_tv_playlists_from_config();
                    self.reload_tv_playlists();
                    if self.state.tv_playlists.is_empty() {
                        self.action_sender.send(Action::ShowTvConfig).ok();
                    }
                } else {
                    self.state.tv_config_popup = false;
                    self.state.search_query.clear();
                    self.state.search_results.clear();
                    if self.state.active_provider == crate::providers::models::ProviderKind::Addons
                    {
                        self.state.active_provider =
                            crate::providers::models::ProviderKind::MovieBox;
                    }
                    self.announce_mode();
                }
                self.persist_config();
            }

            Action::ShowTvConfig => {
                if self.state.is_tv_mode {
                    self.reset_transient_overlays();
                    self.state.tv_config_popup = true;
                    self.state.input_mode = crate::tui::state::InputMode::Normal;
                    self.state.tv_manager_selected = 1;
                    self.state.tv_input_active = false;
                    self.state.tv_input_buffer.clear();
                }
            }

            Action::TvPlaylistAdd(source) => {
                let source = source.trim().to_string();
                if !source.is_empty()
                    && !self
                        .state
                        .tv_playlists
                        .iter()
                        .any(|existing| existing == &source)
                {
                    self.state.tv_playlists.push(source);
                    self.save_tv_playlists();
                    self.reload_tv_playlists();
                }
            }

            Action::TvPlaylistRemove(index) => {
                if index < self.state.tv_playlists.len() {
                    self.state.tv_playlists.remove(index);
                    if self.state.tv_manager_selected > self.state.tv_playlists.len() {
                        self.state.tv_manager_selected = self.state.tv_playlists.len();
                    }
                    self.save_tv_playlists();
                    self.reload_tv_playlists();
                }
            }

            Action::TvReloadPlaylists => {
                self.state.set_status_default("Reloading TV playlists...");
                self.reload_tv_playlists();
            }

            Action::TvInputToggle(is_file) => {
                self.state.tv_input_active = true;
                self.state.tv_input_is_file = is_file;
                self.state.tv_input_buffer.clear();
            }

            Action::TvChannelsLoaded(channels, failed) => {
                let mut seen = std::collections::HashSet::new();
                self.state.tv_channels = channels
                    .into_iter()
                    .filter(|channel| {
                        !channel.stream_url.is_empty() && seen.insert(channel.stream_url.clone())
                    })
                    .collect();
                self.state.is_loading = false;
                let query = self.state.search_query.trim().to_string();
                let lower_query = query.to_lowercase();
                if !query.is_empty() {
                    self.apply_tv_search_results(&query, &lower_query);
                    return None;
                }
                self.state
                    .search_list_state
                    .select(if self.state.tv_channels.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                if self.state.tv_channels.is_empty() {
                    let status = if failed > 0 {
                        format!(
                            "No TV channels found. {failed} playlist(s) failed to load. Add a playlist (/config)."
                        )
                    } else {
                        "No TV channels found. Add a playlist (/config).".to_string()
                    };
                    self.state.set_status_long(status);
                } else {
                    let mut status = format!(
                        "{} TV channels imported from {} playlist(s).",
                        self.state.tv_channels.len(),
                        self.state.tv_playlists.len().max(1)
                    );
                    if failed > 0 {
                        status.push_str(&format!(" {failed} playlist(s) failed to load."));
                    }
                    self.state.set_status_long(status);
                }
            }
            _ => return None,
        }
        None
    }
}
