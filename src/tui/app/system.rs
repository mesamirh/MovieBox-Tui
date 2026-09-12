use super::App;
use crate::tui::{action::Action, overlay::NotificationKind, state::Screen};

impl App {
    pub(super) async fn handle_system(&mut self, action: Action) -> Option<()> {
        match action {
            Action::Tick => {
                let mut needs_redraw = (self.state.is_loading && self.state.tick_count % 5 == 0)
                    || self.state.tick_count < 15
                    || (self.state.active_screen == Screen::Home
                        && self.state.search_results.is_empty()
                        && self.state.search_query.is_empty()
                        && self.state.tick_count % 40 == 0);
                self.state.tick_count = self.state.tick_count.wrapping_add(1);
                if !self.state.notifications.is_empty() {
                    needs_redraw = true;
                    self.state.expire_notifications();
                }
                if self.state.status_timer > 0 {
                    needs_redraw = true;
                    self.state.status_timer -= 1;
                    if self.state.status_timer == 0 {
                        self.state.status_message.clear();
                    }
                }
                if let Some((time, _, _)) = self.state.last_resize_time {
                    needs_redraw = true;
                    if time.elapsed() >= std::time::Duration::from_millis(300) {
                        self.state.last_resize_time = None;
                        self.state.clear_terminal_before_draw = true;
                        self.state.clear_poster_protocols();
                    }
                }
                if needs_redraw {
                    self.state.dirty = true;
                }

                let query_trimmed = self.state.search_query.as_str().trim();
                if query_trimmed != self.state.last_suggest_query.as_str()
                    && self.state.last_search_edit.elapsed()
                        >= std::time::Duration::from_millis(350)
                {
                    self.state.last_suggest_query.clear();
                    self.state.last_suggest_query.push_str(query_trimmed);
                    if !query_trimmed.is_empty() {
                        if self.state.is_tv_mode && !query_trimmed.starts_with('/') {
                            let q = query_trimmed.to_lowercase();
                            self.state.search_suggestions = self
                                .state
                                .tv_channels
                                .iter()
                                .filter(|c| c.name.to_lowercase().contains(&q))
                                .take(10)
                                .map(|c| c.name.clone())
                                .collect();
                            self.state.dirty = true;
                        } else {
                            self.action_sender
                                .send(Action::Suggest(query_trimmed.to_string()))
                                .ok();
                        }
                    } else {
                        self.state.search_suggestions.clear();
                        self.state.dirty = true;
                    }
                }

                if self.state.pending_episode_fetch.is_some()
                    && self.state.last_episode_nav.elapsed()
                        >= std::time::Duration::from_millis(300)
                {
                    if let Some((subject_id, se, ep)) = self.state.pending_episode_fetch.take() {
                        let mut found_cached = false;
                        if let Some(pool) = self.state.stream_pool.get(&subject_id) {
                            if let Some(cached) = pool.episode_index.get(&(se, ep)) {
                                found_cached = true;
                                let count = cached.len();
                                self.state.selected_resources = cached.clone();
                                self.state.is_loading = false;
                                self.state.resource_list_state.select(if count > 0 {
                                    Some(0)
                                } else {
                                    None
                                });
                                self.state.set_status_default(format!(
                                    "Resolved {} direct stream sources (cached).",
                                    count
                                ));
                            }
                        }

                        if !found_cached {
                            self.action_sender
                                .send(Action::FetchEpisodeStreams {
                                    subject_id,
                                    season: se,
                                    episode: ep,
                                    force_refresh: false,
                                })
                                .ok();
                        }
                    }
                }
            }

            Action::FocusChange => {
                self.prepare_image_soft_refresh();
            }

            Action::Resize(w, h) => {
                self.state.last_resize_time = Some((std::time::Instant::now(), w, h));
                self.state.clear_poster_protocols();
                self.state.dirty = true;
            }

            Action::SwitchProvider(provider) => self.switch_provider(provider),

            Action::ToggleHelp => {
                if matches!(self.state.active_screen, Screen::Home | Screen::Details) {
                    self.state.show_help = !self.state.show_help;
                    if self.state.show_help {
                        self.state.help_scroll = 0;
                    }
                    if self.state.show_help {
                        self.state.show_theme_popup = false;
                        self.state.show_browse_popup = false;
                        self.state.tv_config_popup = false;
                        self.state.player_picker_popup = false;
                        self.state.subtitle_popup = false;
                        self.state.is_download_subtitle_popup = false;
                        self.state.show_season_download_confirm = false;
                        self.state.show_episode_download_confirm = false;
                    }
                }
            }

            Action::Refresh => match self.state.active_screen {
                Screen::Home => {
                    let query = self.state.search_query.trim().to_string();
                    if self.state.is_tv_mode {
                        self.state.set_status_default("Reloading TV playlists...");
                        self.reload_tv_playlists();
                    } else if let Some(preset) = self.state.active_browse_preset {
                        self.state.is_loading = true;
                        self.state
                            .set_status_default(format!("Reloading {}...", preset.label()));
                        let tab_id = if self.state.current_tab_id.is_empty() {
                            "2".to_string()
                        } else {
                            self.state.current_tab_id.clone()
                        };
                        self.action_sender
                            .send(Action::FetchHomepage { tab_id, page: 1 })
                            .ok();
                    } else if let Some(catalog) = self.state.active_addon_catalog.clone() {
                        self.action_sender
                            .send(Action::SelectAddonCatalog(catalog))
                            .ok();
                    } else if !query.is_empty() {
                        self.action_sender
                            .send(Action::Search {
                                query,
                                force_refresh: true,
                            })
                            .ok();
                    }
                }
                Screen::Details => {
                    if let Some(id) = self.state.active_subject_id.clone() {
                        let se = if self.state.available_seasons.is_empty() {
                            0
                        } else {
                            self.state.selected_season
                        };
                        let ep = if self.state.available_seasons.is_empty() {
                            0
                        } else {
                            self.state.selected_episode
                        };
                        let id_clone = id.clone();
                        let id_clone_2 = id.clone();
                        let provider = self.provider_for_subject(&id);
                        tokio::task::spawn_blocking(move || {
                            crate::cache::invalidate_provider_stream_cache(
                                provider, &id_clone, se, ep,
                            );
                            crate::cache::invalidate_provider_details_cache(provider, &id_clone_2);
                        });
                        self.state.selected_season = se;
                        self.state.selected_episode = ep;

                        self.action_sender
                            .send(Action::FetchDetails(id.clone(), true))
                            .ok();

                        self.action_sender
                            .send(Action::FetchEpisodeStreams {
                                subject_id: id,
                                season: se,
                                episode: ep,
                                force_refresh: true,
                            })
                            .ok();
                    }
                }
            },

            Action::ClearCache => {
                self.request_tasks.cancel_all();
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(crate::cache::clear_all_cache)
                        .await
                        .map_err(|error| format!("cache clear task failed: {error}"))
                        .and_then(|result| result);
                    sender.send(Action::CacheCleared(result)).ok();
                });
                self.state
                    .fetch_cancel
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                self.state.fetch_cancel =
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                self.state.provider_generation = self.state.provider_generation.wrapping_add(1);
                self.state.active_preview_request =
                    self.state.active_preview_request.wrapping_add(1);
                self.state.active_search_request = self.state.active_search_request.wrapping_add(1);
                self.state.active_details_request =
                    self.state.active_details_request.wrapping_add(1);
                self.state.active_resource_request =
                    self.state.active_resource_request.wrapping_add(1);
                self.reset_transient_overlays();
                self.state.active_screen = Screen::Home;
                self.state.input_mode = crate::tui::state::InputMode::Normal;
                self.state.is_loading = false;
                self.state.clear_search_state();
                self.state.clear_details_state();
                self.state.stream_pool.clear();
                self.state.image_cache.clear();
                self.state.preview_cache.clear();
                self.state.suggest_cache.clear();
                self.state.homepage_cache.clear();
                if self.state.is_tv_mode {
                    self.state.tv_channels.clear();
                }
                self.prepare_image_soft_refresh();
                self.state.set_status_default("Clearing cache...");
                self.state.dirty = true;
            }

            Action::CacheCleared(result) => match result {
                Ok(()) => {
                    if self.state.is_tv_mode && !self.state.tv_playlists.is_empty() {
                        self.state.notify(
                            NotificationKind::Success,
                            "Cache Cleared",
                            "Cache cleared. Reloading playlists...",
                        );
                        self.reload_tv_playlists();
                    } else {
                        self.state.notify(
                            NotificationKind::Success,
                            "Cache Cleared",
                            "Temporary cache files cleared.",
                        );
                    }
                }
                Err(error) => {
                    log::error!("cache clear failed: {error}");
                    self.state
                        .notify(NotificationKind::Error, "Cache Clear Failed", error);
                }
            },
            Action::ToggleSettingsPopup => {
                let open = !self.state.show_settings_popup;
                if open {
                    self.reset_transient_overlays();
                    for player in crate::tui::player::detect() {
                        if !self.state.available_players.contains(&player) {
                            self.state.available_players.push(player);
                        }
                    }
                    self.state.ensure_default_player();
                    self.state.show_settings_popup = true;
                    self.state.settings_category = crate::tui::state::SettingsCategory::General;
                    self.state.settings_selected_row = 0;
                    self.state.settings_download_dir_input = None;
                    self.state.settings_player_picker = false;
                    self.state.show_sources_popup = false;
                    self.state.input_mode = crate::tui::state::InputMode::Normal;
                } else {
                    self.state.show_settings_popup = false;
                    self.state.settings_download_dir_input = None;
                    self.state.settings_player_picker = false;
                    self.state.show_sources_popup = false;
                    self.persist_config();
                }
            }

            Action::ShowSettingsPopup => {
                for player in crate::tui::player::detect() {
                    if !self.state.available_players.contains(&player) {
                        self.state.available_players.push(player);
                    }
                }
                self.state.ensure_default_player();
                self.state.show_settings_popup = true;
                self.state.settings_category = crate::tui::state::SettingsCategory::General;
                self.state.settings_selected_row = 0;
                self.state.settings_download_dir_input = None;
                self.state.settings_player_picker = false;
                self.state.show_sources_popup = false;
                self.state.input_mode = crate::tui::state::InputMode::Normal;
            }

            Action::CloseSettingsPopup => {
                self.state.show_settings_popup = false;
                self.state.settings_download_dir_input = None;
                self.state.settings_player_picker = false;
                self.state.show_sources_popup = false;
                self.persist_config();
            }

            Action::SelectSettingsCategory(cat) => {
                self.state.settings_select_category(cat);
            }

            Action::SettingsResetDownloadDir => {
                self.state.download_dir = None;
                self.state.settings_download_dir_input = None;
                self.persist_config();
                let default_dir = crate::logging::sanitize_path(self.resolve_download_base_dir());
                self.state.notify(
                    NotificationKind::Success,
                    "Download Folder",
                    format!("Reset download folder to default ({default_dir})"),
                );
            }

            Action::SettingsAdjustValue(forward) => match self.state.settings_category {
                crate::tui::state::SettingsCategory::General => {
                    match self.state.settings_selected_row {
                        0 => {
                            self.state.auto_update = !self.state.auto_update;
                            self.persist_config();
                        }
                        1 => {
                            self.state.cycle_settings_player(forward);
                            self.persist_config();
                        }
                        _ => {}
                    }
                }
                crate::tui::state::SettingsCategory::ContentModes => {
                    match self.state.settings_selected_row {
                        0 => {
                            let enable_req = !self.state.streaming_enabled;
                            if !enable_req && !self.state.can_disable_streaming_mode() {
                                self.state.notify(
                                    NotificationKind::Warning,
                                    "Streaming Mode",
                                    "At least one mode must remain active.",
                                );
                            } else {
                                self.state.streaming_enabled = enable_req;
                                self.persist_config();
                                if !self.state.streaming_enabled && !self.state.is_tv_mode {
                                    if self.state.tv_enabled {
                                        self.state.set_mode(crate::tui::state::AppMode::Tv);
                                    }
                                }
                            }
                        }
                        1 => {
                            self.state.show_sources_popup = true;
                            if self.state.sources_list_state.selected().is_none() {
                                self.state.sources_list_state.select(Some(0));
                            }
                        }
                        2 => {
                            let enable_req = !self.state.tv_enabled;
                            if !enable_req && !self.state.can_disable_tv_mode() {
                                self.state.notify(
                                    NotificationKind::Warning,
                                    "Live TV Mode",
                                    "At least one mode must remain active.",
                                );
                            } else {
                                self.state.tv_enabled = enable_req;
                                self.persist_config();
                                if !self.state.tv_enabled && self.state.is_tv_mode {
                                    if self.state.streaming_enabled {
                                        self.state.set_mode(crate::tui::state::AppMode::Streaming);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                crate::tui::state::SettingsCategory::Appearance => {
                    if self.state.settings_selected_row == 0 {
                        let next_theme = self.state.cycle_settings_theme(forward);
                        let kind = crate::tui::theme::ThemeKind::parse(&next_theme);
                        self.theme = crate::tui::theme::Theme::from_kind(kind);
                        self.persist_config();
                        self.state.dirty = true;
                    }
                }
                crate::tui::state::SettingsCategory::StorageInfo => {}
            },

            Action::SettingsActivateRow => match self.state.settings_category {
                crate::tui::state::SettingsCategory::General => {
                    match self.state.settings_selected_row {
                        0 => {
                            self.state.auto_update = !self.state.auto_update;
                            self.persist_config();
                        }
                        1 => {
                            if self.state.available_players.is_empty() {
                                let detected = crate::tui::player::detect();
                                if !detected.is_empty() {
                                    self.state.available_players = detected;
                                }
                                self.state.ensure_default_player();
                            }
                            if !self.state.available_players.is_empty() {
                                self.state.settings_player_picker = true;
                                self.state.player_picker_popup = true;
                                let selected_idx =
                                    self.state
                                        .default_player
                                        .as_deref()
                                        .and_then(|k| {
                                            self.state.available_players.iter().position(|p| {
                                                p.config_key().eq_ignore_ascii_case(k)
                                            })
                                        })
                                        .unwrap_or(0);
                                self.state.player_picker_state.select(Some(selected_idx));
                            }
                        }
                        2 => {
                            if let Some(input) = self.state.settings_download_dir_input.take() {
                                let new_path = input.as_str().trim();
                                if let Some(pb) =
                                    crate::tui::state::AppState::expand_download_path(new_path)
                                {
                                    self.state.download_dir = Some(pb.clone());
                                    self.persist_config();
                                    self.state.notify(
                                        NotificationKind::Success,
                                        "Download Folder",
                                        format!("Download folder set to {}", pb.display()),
                                    );
                                } else {
                                    self.state.download_dir = None;
                                    self.persist_config();
                                    let default_dir = crate::logging::sanitize_path(
                                        self.resolve_download_base_dir(),
                                    );
                                    self.state.notify(
                                        NotificationKind::Success,
                                        "Download Folder",
                                        format!("Reset download folder to default ({default_dir})"),
                                    );
                                }
                            } else {
                                let current = self
                                    .state
                                    .download_dir
                                    .as_ref()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                self.state.settings_download_dir_input =
                                    Some(crate::tui::text::TextInputBuffer::from_str(&current));
                            }
                        }
                        _ => {}
                    }
                }
                crate::tui::state::SettingsCategory::ContentModes => {
                    match self.state.settings_selected_row {
                        0 => {
                            let enable_req = !self.state.streaming_enabled;
                            if !enable_req && !self.state.can_disable_streaming_mode() {
                                self.state.notify(
                                    NotificationKind::Warning,
                                    "Streaming Mode",
                                    "At least one mode must remain active.",
                                );
                            } else {
                                self.state.streaming_enabled = enable_req;
                                self.persist_config();
                                if !self.state.streaming_enabled && !self.state.is_tv_mode {
                                    if self.state.tv_enabled {
                                        self.state.set_mode(crate::tui::state::AppMode::Tv);
                                    }
                                }
                            }
                        }
                        1 => {
                            self.state.show_sources_popup = true;
                            if self.state.sources_list_state.selected().is_none() {
                                self.state.sources_list_state.select(Some(0));
                            }
                        }
                        2 => {
                            let enable_req = !self.state.tv_enabled;
                            if !enable_req && !self.state.can_disable_tv_mode() {
                                self.state.notify(
                                    NotificationKind::Warning,
                                    "Live TV Mode",
                                    "At least one mode must remain active.",
                                );
                            } else {
                                self.state.tv_enabled = enable_req;
                                self.persist_config();
                                if !self.state.tv_enabled && self.state.is_tv_mode {
                                    if self.state.streaming_enabled {
                                        self.state.set_mode(crate::tui::state::AppMode::Streaming);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                crate::tui::state::SettingsCategory::Appearance => {
                    if self.state.settings_selected_row == 0 {
                        self.state.original_theme_kind = Some(self.state.active_theme_kind.clone());
                        self.state.show_theme_popup = true;
                        if let Some(idx) = crate::tui::theme::AVAILABLE_THEMES
                            .iter()
                            .position(|&t| t.eq_ignore_ascii_case(&self.state.active_theme_kind))
                        {
                            self.state.theme_list_state.select(Some(idx));
                        } else {
                            self.state.theme_list_state.select(Some(0));
                        }
                    }
                }
                crate::tui::state::SettingsCategory::StorageInfo => {
                    match self.state.settings_selected_row {
                        0 => {
                            self.state.notify(
                                NotificationKind::Info,
                                "Clearing Cache",
                                "Clearing temporary disk cache...",
                            );
                            self.action_sender.send(Action::ClearCache).ok();
                        }
                        1 => {
                            self.state.history.clear();
                            self.state.homepage_cache.clear();
                            if self
                                .state
                                .search_query
                                .trim()
                                .eq_ignore_ascii_case("/history")
                            {
                                self.state.search_results.clear();
                                self.state.search_list_state.select(None);
                            }
                            self.state.notify(
                                NotificationKind::Success,
                                "History",
                                "Watch history cleared",
                            );
                        }
                        2 => {
                            self.state.manual_update_check = true;
                            self.state.notify(
                                NotificationKind::Info,
                                "Checking for updates",
                                "Checking GitHub releases...",
                            );
                            self.action_sender.send(Action::CheckForUpdates).ok();
                        }
                        3 => {
                            const REPO_URL: &str = "https://github.com/mesamirh/MovieBox-Tui";
                            match open::that(REPO_URL) {
                                Ok(()) => {
                                    self.state.notify(
                                        NotificationKind::Info,
                                        "GitHub",
                                        "Opening repository in browser...",
                                    );
                                }
                                Err(error) => {
                                    log::warn!("failed to open web browser: {error}");
                                    self.state.notify(
                                        NotificationKind::Warning,
                                        "Browser Launch Failed",
                                        format!("Could not open browser: {error}"),
                                    );
                                }
                            }
                        }
                        4 => {
                            self.state.bdix_probed = false;
                            self.action_sender.send(Action::CheckBdixNetwork).ok();
                            self.state.notify(
                                NotificationKind::Info,
                                "BDIX Check",
                                "Probing local network mirrors...",
                            );
                        }
                        _ => {}
                    }
                }
            },

            Action::ToggleThemePopup => {
                let open = !self.state.show_theme_popup;
                if open {
                    self.reset_transient_overlays();
                    self.state.tv_config_popup = false;
                    self.state.original_theme_kind = Some(self.state.active_theme_kind.clone());
                    self.state.show_theme_popup = true;
                    if let Some(idx) = crate::tui::theme::AVAILABLE_THEMES
                        .iter()
                        .position(|&t| t.eq_ignore_ascii_case(&self.state.active_theme_kind))
                    {
                        self.state.theme_list_state.select(Some(idx));
                    } else {
                        self.state.theme_list_state.select(Some(0));
                    }
                } else {
                    self.state.show_theme_popup = false;
                }
            }

            Action::ShowBrowseMenu => {
                let current_mode = self.state.mode();
                if current_mode == crate::tui::state::AppMode::Tv {
                    let ctrl_s = crate::tui::text::CTRL_S_STR;
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        format!("Command /browse is available in Streaming Mode ({ctrl_s})."),
                    );
                } else if current_mode == crate::tui::state::AppMode::Streaming
                    && self.state.active_provider
                        != crate::providers::models::ProviderKind::MovieBox
                {
                    self.state
                        .set_status_long("Browse is available only with the MovieBox provider.");
                } else {
                    self.reset_transient_overlays();
                    self.state.show_browse_popup = true;
                    self.state.browse_list_state.select(Some(0));
                    self.state.input_mode = crate::tui::state::InputMode::Normal;
                }
            }

            Action::SelectTheme(theme_name) => {
                if theme_name.is_empty() && self.state.theme_is_auto {
                    self.theme = crate::tui::theme::Theme::detect();
                } else {
                    let kind = crate::tui::theme::ThemeKind::parse(&theme_name);
                    self.state.active_theme_kind = kind.as_str().to_string();
                    self.state.theme_is_auto = false;
                    self.theme = crate::tui::theme::Theme::from_kind(kind);
                }
                if !self.state.show_theme_popup {
                    self.persist_config();
                }
                self.state.dirty = true;
            }

            Action::SetStatus(msg) => {
                self.state.is_resolving_playback = false;
                if msg.starts_with("Error:") {
                    log::error!("{msg}");
                    let body = msg.trim_start_matches("Error:").trim();
                    let title = if body.starts_with("4KHDHub") {
                        "4KHDHub Stream Unavailable"
                    } else {
                        "Operation failed"
                    };
                    self.state.notify(NotificationKind::Error, title, body);
                } else {
                    self.state.set_status_default(msg);
                }
            }

            Action::CheckForUpdates => {
                if self.state.is_checking_updates {
                    return None;
                }
                self.state.is_checking_updates = true;
                let update_sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let task =
                        tokio::spawn(crate::updater::check_release(env!("CARGO_PKG_VERSION")));
                    let result = match task.await {
                        Ok(res) => res,
                        Err(join_err) => Err(format!("update check task error: {join_err}")),
                    };
                    update_sender.send(Action::UpdateAvailable(result)).ok();
                });
            }

            Action::UpdateAvailable(result) => {
                self.state.is_checking_updates = false;
                self.state.last_update_check = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                self.persist_config();

                match result {
                    Ok(None) => {
                        if self.state.manual_update_check {
                            self.state.set_status_long(format!(
                                "MovieBox-Tui is up to date (v{}).",
                                env!("CARGO_PKG_VERSION")
                            ));
                            self.state.notify(
                                NotificationKind::Success,
                                "Up to date",
                                format!(
                                    "MovieBox-Tui v{} is the latest version.",
                                    env!("CARGO_PKG_VERSION")
                                ),
                            );
                        }
                        self.state.manual_update_check = false;
                    }
                    Err(err) => {
                        if self.state.manual_update_check {
                            self.state
                                .set_status_long(format!("Update check failed: {err}"));
                            self.state
                                .notify(NotificationKind::Error, "Update check failed", err);
                        }
                        self.state.manual_update_check = false;
                    }
                    Ok(Some(release)) => {
                        self.state.manual_update_check = false;
                        let version = release.version.clone();
                        let notes = release.notes.clone();
                        self.state.update_release = Some(release);
                        if self.state.input_mode == crate::tui::state::InputMode::Editing {
                            self.state.update_available = Some((version.clone(), notes));
                            self.state.notify(
                                NotificationKind::Info,
                                "Update Available",
                                format!(
                                    "MovieBox-Tui v{version} is available. Exit search to view."
                                ),
                            );
                        } else if self.state.is_playing || self.state.download_progress.is_some() {
                            self.state.update_available = Some((version.clone(), notes));
                            self.state
                                .set_status_short(format!("Update v{version} available."));
                        } else {
                            self.reset_transient_overlays();
                            self.state.update_available = Some((version, notes));
                        }
                    }
                }
            }

            Action::StartSelfUpdate => {
                if self.state.is_updating {
                    return None;
                }
                if self.state.download_progress.is_some() {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Update Deferred",
                        "Cannot perform in-app update while a download is active.",
                    );
                    return None;
                }
                if self.state.is_playing {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Update Deferred",
                        "Cannot perform in-app update while playback is active.",
                    );
                    return None;
                }

                self.state.is_updating = true;
                self.state.update_available = None;
                self.state.update_progress_msg = Some("Starting self-update...".to_string());
                self.state.set_status_long("Starting self-update...");
                self.state.notify(
                    NotificationKind::Info,
                    "Self-Update",
                    "Downloading release artifact and verifying checksum...",
                );

                let cached_release = self.state.update_release.clone();
                let update_sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let release = match cached_release {
                        Some(r) => r,
                        None => {
                            match crate::updater::check_release(env!("CARGO_PKG_VERSION")).await {
                                Ok(Some(r)) => r,
                                Ok(None) => {
                                    update_sender
                                        .send(Action::SelfUpdateComplete(Err(
                                            "Already on the latest version.".to_string(),
                                        )))
                                        .ok();
                                    return;
                                }
                                Err(e) => {
                                    update_sender.send(Action::SelfUpdateComplete(Err(e))).ok();
                                    return;
                                }
                            }
                        }
                    };

                    let (progress_tx, mut progress_rx) =
                        tokio::sync::mpsc::unbounded_channel::<String>();
                    let fwd_sender = update_sender.clone();
                    tokio::spawn(async move {
                        while let Some(msg) = progress_rx.recv().await {
                            fwd_sender.send(Action::SelfUpdateProgress(msg)).ok();
                        }
                    });

                    let outcome =
                        crate::updater::perform_self_update(&release, Some(&progress_tx)).await;
                    update_sender.send(Action::SelfUpdateComplete(outcome)).ok();
                });
            }

            Action::SelfUpdateProgress(msg) => {
                self.state.update_progress_msg = Some(msg.clone());
                self.state.set_status_long(msg);
            }

            Action::SelfUpdateComplete(result) => {
                self.state.is_updating = false;
                self.state.update_progress_msg = None;
                match result {
                    Ok(crate::updater::SelfUpdateOutcome::Success) => {
                        self.state
                            .set_status_long("Update successful! Restarting...");
                        self.state.notify(
                            NotificationKind::Success,
                            "Update Installed",
                            "MovieBox-Tui was updated successfully. Restarting process...",
                        );

                        crossterm::terminal::disable_raw_mode().ok();
                        crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::LeaveAlternateScreen,
                            crossterm::event::DisableMouseCapture,
                            crossterm::cursor::Show
                        )
                        .ok();

                        if let Ok(exe_path) = std::env::current_exe() {
                            if let Err(e) = crate::updater::restart_process(&exe_path) {
                                log::error!("failed to restart process after update: {e}");
                            }
                        }
                        std::process::exit(0);
                    }
                    Ok(crate::updater::SelfUpdateOutcome::RequiresManualUpgrade(msg)) => {
                        self.state.set_status(msg.clone(), 300);
                        self.state
                            .notify(NotificationKind::Warning, "Manual Update Required", msg);
                    }
                    Err(err) => {
                        self.state.set_status_long(format!("Update failed: {err}"));
                        self.state
                            .notify(NotificationKind::Error, "Update Failed", err);
                    }
                }
            }

            Action::ToggleProvider(provider) => {
                self.toggle_provider(provider);
            }

            Action::CheckBdixNetwork => {
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let timeout = std::time::Duration::from_secs(3);
                    let circleftp =
                        crate::net::probe_url("http://new.circleftp.net:5000/api", timeout).await;
                    let dhakaflix = crate::net::probe_url("http://172.16.50.7/", timeout).await;
                    sender
                        .send(Action::BdixProbeResult {
                            circleftp,
                            dhakaflix,
                        })
                        .ok();
                });
            }

            Action::BdixProbeResult {
                circleftp,
                dhakaflix,
            } => {
                self.state.bdix_circleftp_enabled = circleftp;
                self.state.bdix_dhakaflix_enabled = dhakaflix;
                self.state.bdix_probed = true;
                if !self.state.provider_enabled(self.state.active_provider) {
                    let next = self
                        .state
                        .available_providers()
                        .into_iter()
                        .next()
                        .unwrap_or(crate::providers::models::ProviderKind::MovieBox);
                    self.switch_provider(next);
                }
                self.persist_config();
                if circleftp || dhakaflix {
                    let mut found = Vec::new();
                    if circleftp {
                        found.push("CircleFTP");
                    }
                    if dhakaflix {
                        found.push("DhakaFlix");
                    }
                    self.state.notify(
                        NotificationKind::Info,
                        "BDIX Network Detected",
                        format!("Automatically enabled: {}", found.join(", ")),
                    );
                }
            }
            _ => return None,
        }
        None
    }

    fn toggle_provider(&mut self, provider: crate::providers::models::ProviderKind) {
        let currently_enabled = self.state.provider_enabled(provider);
        if currently_enabled {
            let would_remain: Vec<crate::providers::models::ProviderKind> =
                crate::providers::models::ProviderKind::ENABLED
                    .into_iter()
                    .filter(|p| *p != provider && self.state.provider_enabled(*p))
                    .collect();
            if would_remain.is_empty() {
                self.state.notify(
                    NotificationKind::Warning,
                    provider.label(),
                    "Cannot disable: at least one streaming provider must remain active.",
                );
                return;
            }
        }
        self.state
            .set_provider_enabled(provider, !currently_enabled);
        if currently_enabled && self.state.active_provider == provider {
            let next = self
                .state
                .available_providers()
                .into_iter()
                .next()
                .unwrap_or(crate::providers::models::ProviderKind::MovieBox);
            self.switch_provider(next);
        }
        self.persist_config();
    }
}
