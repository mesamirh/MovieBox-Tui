use super::App;
use crate::tui::action::Action;

impl App {
    pub(super) async fn handle_tv(&mut self, action: Action) -> Option<()> {
        match action {
            Action::ToggleTvMode => {
                self.state.is_tv_mode = !self.state.is_tv_mode;
                self.state.tick_count = 0;
                self.state.browse_view = None;
                self.state.browse_menu_open = false;
                if self.state.is_tv_mode {
                    self.state.tv_config_popup = false;
                    self.state.search_query.clear();
                    self.state.search_results.clear();
                    self.state
                        .set_status("Loading TV playlists...".to_string(), 200);
                    self.load_tv_playlists_from_config();
                    self.reload_tv_playlists();
                    if self.state.tv_playlists.is_empty() {
                        self.action_sender.send(Action::ShowTvConfig).ok();
                    }
                } else {
                    self.state.tv_config_popup = false;
                    self.state.search_query.clear();
                    self.state.search_results.clear();
                }
            }

            Action::ShowTvConfig => {
                if self.state.is_tv_mode {
                    self.state.show_help = false;
                    self.state.player_picker_popup = false;
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
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
                self.state
                    .set_status("Reloading TV playlists...".to_string(), 150);
                self.reload_tv_playlists();
            }

            Action::TvInputToggle(is_file) => {
                self.state.tv_input_active = true;
                self.state.tv_input_is_file = is_file;
                self.state.tv_input_buffer.clear();
            }

            Action::TvChannelsLoaded(channels) => {
                let mut seen = std::collections::HashSet::new();
                self.state.tv_channels = channels
                    .into_iter()
                    .filter(|channel| {
                        !channel.stream_url.is_empty() && seen.insert(channel.stream_url.clone())
                    })
                    .collect();
                self.state.is_loading = false;
                if self.state.tv_channels.is_empty() {
                    self.state.set_status(
                        "No TV channels found. Add a playlist (/config).".to_string(),
                        200,
                    );
                } else {
                    self.state.set_status(
                        format!(
                            "{} TV channels imported from {} playlist(s).",
                            self.state.tv_channels.len(),
                            self.state.tv_playlists.len().max(1)
                        ),
                        200,
                    );
                }
            }
            _ => return None,
        }
        None
    }
}
