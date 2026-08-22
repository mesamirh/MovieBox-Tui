use super::{App, network};
use crate::providers::models::{ProviderKind, RequestContext};
use crate::service::extract_browse_metrics;
use crate::tui::{
    action::Action,
    overlay::NotificationKind,
    state::{BrowsePreset, InputMode, Screen, SearchResult},
};

fn browse_group_matches(title: &str, preset: BrowsePreset) -> bool {
    let title = title.to_lowercase();
    match preset {
        BrowsePreset::Trending => title.contains("trending") || title.contains("hot"),
        BrowsePreset::TopRatedAllTime => {
            title.contains("top") || title.contains("rated") || title.contains("favorite")
        }
        BrowsePreset::TopRatedRecent => {
            title.contains("new")
                || title.contains("release")
                || title.contains("recent")
                || title.contains("latest")
        }
        BrowsePreset::MostWatched => {
            title.contains("popular")
                || title.contains("popluar")
                || title.contains("most")
                || title.contains("watched")
                || title.contains("box office")
                || title.contains("action")
                || title.contains("adventure")
                || title.contains("super hero")
                || title.contains("stars")
        }
    }
}

fn compare_browse_values(
    left: Option<f64>,
    right: Option<f64>,
    descending: bool,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            let order = left
                .partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal);
            if descending { order.reverse() } else { order }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

impl App {
    pub(super) fn apply_tv_search_results(&mut self, query: &str, lower_query: &str) {
        self.state.search_results = self
            .state
            .tv_channels
            .iter()
            .filter(|channel| {
                lower_query == "/list"
                    || channel.name.to_lowercase().contains(lower_query)
                    || channel.group.to_lowercase().contains(lower_query)
            })
            .map(|channel| SearchResult {
                id: channel.stream_url.clone(),
                title: channel.name.clone(),
                stype: 3,
                release_year: channel.group.clone(),
                cover_url: Some(channel.logo.clone()),
                season: 1,
                episode: 1,
                provider: ProviderKind::MovieBox,
            })
            .collect();
        self.state.is_loading = false;
        self.state
            .search_list_state
            .select(if self.state.search_results.is_empty() {
                None
            } else {
                Some(0)
            });
        if !self.state.search_results.is_empty() {
            self.prefetch_visible_posters();
        }
        self.state.set_status(
            if self.state.search_results.is_empty() {
                format!("No matches for '{}'.", query)
            } else {
                format!("Found {} channels.", self.state.search_results.len())
            },
            150,
        );
    }

    pub(super) fn handle_search_command(&mut self, query: &str, lower_query: &str) -> Option<bool> {
        let trimmed = query.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        if trimmed == "/" {
            self.state.search_query.clear();
            self.state.input_mode = InputMode::Normal;
            return Some(true);
        }

        let parsed = match crate::tui::commands::SlashCommand::parse(trimmed) {
            Some(p) => p,
            None => {
                let cmd_name = trimmed.split_whitespace().next().unwrap_or(trimmed);
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Warning,
                    "Unknown Command",
                    format!("Command '{cmd_name}' is not recognized. Type '/' to view available commands."),
                );
                return Some(true);
            }
        };

        let current_mode = self.state.mode();
        let ctrl_s = crate::tui::text::ctrl_key("S");
        let ctrl_t = crate::tui::text::ctrl_key("T");
        let ctrl_a = crate::tui::text::ctrl_key("A");

        match parsed {
            crate::tui::commands::ParsedCommand::ClearCache => {
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.action_sender.send(Action::ClearCache).ok();
                Some(true)
            }
            crate::tui::commands::ParsedCommand::Github => {
                let _ = open::that("https://github.com/mesamirh/MovieBox-Tui");
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                Some(true)
            }
            crate::tui::commands::ParsedCommand::Update => {
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                if !self.state.is_checking_updates {
                    self.state.update_available = None;
                    self.state.manual_update_check = true;
                    self.state
                        .set_status("Checking GitHub for updates...".to_string(), 180);
                    self.action_sender.send(Action::CheckForUpdates).ok();
                } else {
                    self.state
                        .set_status("Checking GitHub for updates...".to_string(), 180);
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::ToggleUpdate => {
                self.state.auto_update = !self.state.auto_update;
                self.persist_config();
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Info,
                    "Auto Update Check",
                    if self.state.auto_update {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
                Some(true)
            }
            crate::tui::commands::ParsedCommand::Theme => {
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.action_sender.send(Action::ToggleThemePopup).ok();
                Some(true)
            }
            crate::tui::commands::ParsedCommand::Browse => {
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                if current_mode == crate::tui::state::AppMode::Tv {
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        format!("Command /browse is available in Streaming Mode ({ctrl_s}) or Addon Mode ({ctrl_a})."),
                    );
                } else {
                    self.action_sender.send(Action::ShowBrowseMenu).ok();
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::History => {
                if current_mode == crate::tui::state::AppMode::Tv {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        format!("Command /history is available in Streaming Mode ({ctrl_s}) or Addon Mode ({ctrl_a})."),
                    );
                    Some(true)
                } else {
                    None
                }
            }
            crate::tui::commands::ParsedCommand::Favorites => {
                if current_mode == crate::tui::state::AppMode::Tv {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        format!("Command /favorites is available in Streaming Mode ({ctrl_s}) or Addon Mode ({ctrl_a})."),
                    );
                    Some(true)
                } else {
                    None
                }
            }
            crate::tui::commands::ParsedCommand::List => {
                if current_mode == crate::tui::state::AppMode::Tv {
                    self.apply_tv_search_results(query, lower_query);
                } else {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        format!(
                            "Command /list is only available in TV Mode. Switch with {ctrl_t}."
                        ),
                    );
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::Config => {
                let current_mode = self.state.mode();
                if current_mode == crate::tui::state::AppMode::Tv {
                    self.action_sender.send(Action::ShowTvConfig).ok();
                } else if current_mode == crate::tui::state::AppMode::Addon {
                    self.action_sender.send(Action::ShowAddonManager).ok();
                } else {
                    let ctrl_t = crate::tui::text::ctrl_key("T");
                    let ctrl_a = crate::tui::text::ctrl_key("A");
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "Configuration",
                        format!("Command /config is available in TV Mode ({ctrl_t}) or Addon Mode ({ctrl_a})."),
                    );
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::DownloadDir(raw_arg) => {
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;

                if raw_arg.is_empty() {
                    let current = crate::logging::sanitize_path(self.resolve_download_base_dir());
                    self.state
                        .notify(NotificationKind::Info, "Download Directory", current);
                    return Some(true);
                }

                if raw_arg.eq_ignore_ascii_case("reset") || raw_arg.eq_ignore_ascii_case("default")
                {
                    if self.state.download_dir.is_none() {
                        self.state.notify(
                            NotificationKind::Info,
                            "Download Directory",
                            "Already using system default (~/Downloads/MovieBox-TUI)",
                        );
                    } else {
                        self.state.download_dir = None;
                        self.persist_config();
                        self.state.notify(
                            NotificationKind::Success,
                            "Download Directory",
                            "Reset to default (~/Downloads/MovieBox-TUI)",
                        );
                    }
                    return Some(true);
                }

                let clean_arg = raw_arg.trim_matches(|c| c == '\'' || c == '"').trim();
                if clean_arg == "<path>"
                    || clean_arg == "path"
                    || clean_arg == "<dir>"
                    || clean_arg == "dir"
                {
                    self.state.notify(
                        NotificationKind::Info,
                        "Download Directory",
                        "Usage: /download-dir <folder_path>\nExample: /download-dir ~/Movies",
                    );
                    return Some(true);
                }
                let expanded_path = if let Some(stripped) = clean_arg
                    .strip_prefix("~/")
                    .or_else(|| clean_arg.strip_prefix("~\\"))
                {
                    if let Some(home) = dirs::home_dir() {
                        home.join(stripped)
                    } else {
                        std::path::PathBuf::from(clean_arg)
                    }
                } else if clean_arg == "~" {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(clean_arg))
                } else {
                    std::path::PathBuf::from(clean_arg)
                };

                match std::fs::create_dir_all(&expanded_path) {
                    Ok(_) => {
                        let test_file =
                            expanded_path.join(format!(".mb_probe_{}", std::process::id()));
                        match std::fs::write(&test_file, b"ok") {
                            Ok(_) => {
                                let _ = std::fs::remove_file(&test_file);
                                let canonical =
                                    std::fs::canonicalize(&expanded_path).unwrap_or(expanded_path);
                                let clean_path = {
                                    let s = canonical.to_string_lossy();
                                    if let Some(stripped) = s.strip_prefix(r"\\?\") {
                                        std::path::PathBuf::from(stripped)
                                    } else {
                                        canonical
                                    }
                                };
                                self.state.download_dir = Some(clean_path.clone());
                                self.persist_config();
                                let effective =
                                    crate::logging::sanitize_path(self.resolve_download_base_dir());
                                self.state.notify(
                                    NotificationKind::Success,
                                    "Download Directory",
                                    format!("Saved: {effective}"),
                                );
                            }
                            Err(err) => {
                                self.state.notify(
                                    NotificationKind::Error,
                                    "Permission Denied",
                                    format!(
                                        "Cannot write to '{}': {}",
                                        expanded_path.display(),
                                        err
                                    ),
                                );
                            }
                        }
                    }
                    Err(err) => {
                        self.state.notify(
                            NotificationKind::Error,
                            "Invalid Directory",
                            format!("Cannot create '{}': {}", expanded_path.display(), err),
                        );
                    }
                }

                Some(true)
            }
            crate::tui::commands::ParsedCommand::EnableBdix
            | crate::tui::commands::ParsedCommand::DisableBdix => {
                let enable_req = parsed == crate::tui::commands::ParsedCommand::EnableBdix;
                if current_mode != crate::tui::state::AppMode::Streaming {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "BDIX Sources",
                        format!("BDIX FTP sources are only available in Streaming Mode. Switch with {ctrl_s}."),
                    );
                    return Some(true);
                }
                if self.state.bdix_enabled == enable_req {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "BDIX Providers",
                        if enable_req {
                            "Already Enabled"
                        } else {
                            "Already Disabled"
                        },
                    );
                    return Some(true);
                }

                self.state.bdix_enabled = enable_req;
                self.persist_config();
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Info,
                    "BDIX Providers",
                    if self.state.bdix_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
                if !self.state.bdix_enabled && self.state.active_provider.is_bdix() {
                    let new_provider = ProviderKind::ENABLED
                        .iter()
                        .copied()
                        .find(|provider| !provider.is_bdix())
                        .unwrap_or(ProviderKind::MovieBox);
                    self.action_sender
                        .send(Action::SwitchProvider(new_provider))
                        .ok();
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::EnableStreaming
            | crate::tui::commands::ParsedCommand::DisableStreaming => {
                let enable_req = parsed == crate::tui::commands::ParsedCommand::EnableStreaming;
                if self.state.streaming_enabled == enable_req {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "Streaming Mode",
                        if enable_req {
                            "Already Enabled"
                        } else {
                            "Already Disabled"
                        },
                    );
                    return Some(true);
                }
                if !enable_req && !self.state.tv_enabled && !self.state.addons_enabled {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Warning,
                        "Streaming Mode",
                        "Cannot disable: at least one mode must remain active.",
                    );
                    return Some(true);
                }

                self.state.streaming_enabled = enable_req;
                self.persist_config();
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Info,
                    "Streaming Mode",
                    if self.state.streaming_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
                if !self.state.streaming_enabled
                    && !self.state.is_tv_mode
                    && !self.state.is_addon_mode
                {
                    if self.state.tv_enabled {
                        self.action_sender.send(Action::ToggleTvMode).ok();
                    } else if self.state.addons_enabled {
                        self.action_sender.send(Action::ToggleAddonMode).ok();
                    }
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::EnableTv
            | crate::tui::commands::ParsedCommand::DisableTv => {
                let enable_req = parsed == crate::tui::commands::ParsedCommand::EnableTv;
                if self.state.tv_enabled == enable_req {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "TV Mode",
                        if enable_req {
                            "Already Enabled"
                        } else {
                            "Already Disabled"
                        },
                    );
                    return Some(true);
                }
                if !enable_req && !self.state.streaming_enabled && !self.state.addons_enabled {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Warning,
                        "TV Mode",
                        "Cannot disable: at least one mode must remain active.",
                    );
                    return Some(true);
                }

                self.state.tv_enabled = enable_req;
                self.persist_config();
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Info,
                    "TV Mode",
                    if self.state.tv_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
                if !self.state.tv_enabled && self.state.is_tv_mode {
                    if self.state.streaming_enabled {
                        self.action_sender.send(Action::SwitchToStreamingMode).ok();
                    } else if self.state.addons_enabled {
                        self.action_sender.send(Action::ToggleAddonMode).ok();
                    }
                }
                Some(true)
            }
            crate::tui::commands::ParsedCommand::EnableAddons
            | crate::tui::commands::ParsedCommand::DisableAddons => {
                let enable_req = parsed == crate::tui::commands::ParsedCommand::EnableAddons;
                if self.state.addons_enabled == enable_req {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "Addon Mode",
                        if enable_req {
                            "Already Enabled"
                        } else {
                            "Already Disabled"
                        },
                    );
                    return Some(true);
                }
                if !enable_req && !self.state.streaming_enabled && !self.state.tv_enabled {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Warning,
                        "Addon Mode",
                        "Cannot disable: at least one mode must remain active.",
                    );
                    return Some(true);
                }

                self.state.addons_enabled = enable_req;
                self.persist_config();
                self.state.search_query.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.notify(
                    NotificationKind::Info,
                    "Addon Mode",
                    if self.state.addons_enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    },
                );
                if !self.state.addons_enabled && self.state.is_addon_mode {
                    if self.state.streaming_enabled {
                        self.action_sender.send(Action::SwitchToStreamingMode).ok();
                    } else if self.state.tv_enabled {
                        self.action_sender.send(Action::ToggleTvMode).ok();
                    }
                }
                Some(true)
            }
        }
    }

    pub(super) fn prepare_search_request(&mut self, query: &str) -> RequestContext {
        self.state.active_search_request = self.state.active_search_request.wrapping_add(1);
        self.state.active_preview_request = self.state.active_preview_request.wrapping_add(1);
        self.state.is_homepage_mode = false;
        self.state.active_browse_preset = None;
        self.state.active_addon_catalog = None;
        self.state.browse_metrics.clear();
        self.state.current_page = 1;
        self.state.active_screen = Screen::Home;
        self.state.active_subject_id = None;
        self.state.selected_details = None;
        self.state.selected_resources = None;
        self.state.is_loading = true;
        self.state.search_error = None;
        self.state.search_list_state.select(Some(0));
        self.state.search_suggestions.clear();
        self.state.suggest_index = None;
        self.state.search_preview = None;
        self.state.preview_loading = false;
        self.state.poster_image = None;
        self.state.poster_protocol = None;
        self.state
            .set_status(format!("Searching for '{}'...", query), 150);
        self.request_context()
    }

    pub(super) fn prepare_addon_catalog_request(
        &mut self,
        target: &crate::providers::addons::models::AddonCatalogTarget,
    ) -> RequestContext {
        self.state.active_search_request = self.state.active_search_request.wrapping_add(1);
        self.state.active_preview_request = self.state.active_preview_request.wrapping_add(1);
        self.state.is_homepage_mode = false;
        self.state.active_browse_preset = None;
        self.state.active_addon_catalog = Some(target.clone());
        self.state.browse_metrics.clear();
        self.state.current_page = 1;
        self.state.active_screen = Screen::Home;
        self.state.active_subject_id = None;
        self.state.selected_details = None;
        self.state.selected_resources = None;
        self.state.is_loading = true;
        self.state.search_error = None;
        self.state.search_list_state.select(Some(0));
        self.state.search_suggestions.clear();
        self.state.suggest_index = None;
        self.state.search_preview = None;
        self.state.preview_loading = false;
        self.state.poster_image = None;
        self.state.poster_protocol = None;
        self.state.failed_posters.clear();
        self.state.in_flight_posters.clear();
        self.state.search_results.clear();
        self.state.search_query.clear();
        self.request_context()
    }

    pub(super) fn run_search_request(
        &self,
        query: String,
        force_refresh: bool,
        context: RequestContext,
    ) {
        let request_id = self.state.active_search_request;
        let page = 1;
        let sender = self.action_sender.clone();
        let service = self.service.clone();
        tokio::spawn(async move {
            if !force_refresh {
                let q = query.clone();
                let provider = context.provider;
                if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                    crate::cache::get_provider_search_cache(provider, &q, page)
                })
                .await
                {
                    sender
                        .send(Action::SearchSuccess {
                            context,
                            request_id,
                            query: query.clone(),
                            page,
                            payload: cached,
                        })
                        .ok();
                    return;
                }
            }

            let result = service.search(context.provider, &query, page).await;
            match result {
                Ok(res) => {
                    let q = query.clone();
                    let provider = context.provider;
                    let cached = res.clone();
                    tokio::task::spawn_blocking(move || {
                        crate::cache::set_provider_search_cache(provider, &q, page, &cached);
                    });
                    sender
                        .send(Action::SearchSuccess {
                            context,
                            request_id,
                            query,
                            page,
                            payload: res,
                        })
                        .ok();
                }
                Err(error) => {
                    sender
                        .send(Action::SearchFailure(context, request_id, page, error))
                        .ok();
                }
            }
        });
    }

    pub(super) fn prepare_homepage_request(&mut self, tab_id: &str, page: usize) {
        self.state.active_homepage_request = self.state.active_homepage_request.wrapping_add(1);
        self.state.is_homepage_mode = true;
        self.state.current_tab_id = tab_id.to_string();
        self.state.current_page = page;
        self.state.active_screen = Screen::Home;
        self.state.is_loading = true;
        self.state.search_error = None;
        if page == 1 {
            self.state.active_preview_request = self.state.active_preview_request.wrapping_add(1);
            self.state.active_subject_id = None;
            self.state.selected_details = None;
            self.state.selected_resources = None;
            self.state.search_results.clear();
            self.state.browse_metrics.clear();
            self.state.search_list_state.select(Some(0));
            self.state.search_suggestions.clear();
            self.state.suggest_index = None;
            self.state.search_preview = None;
            self.state.preview_loading = false;
            self.state.poster_image = None;
            self.state.poster_protocol = None;
            self.state
                .set_status("Loading discover tab...".to_string(), 150);
        }
    }

    pub(super) fn prepare_details_request(&mut self, id: &str) -> RequestContext {
        self.state.active_subject_id = Some(id.to_string());
        self.state.poster_protocol = None;
        self.state.is_loading = true;
        self.state.active_details_request = self.state.active_details_request.wrapping_add(1);
        self.state
            .fetch_cancel
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.state
            .set_status("Fetching details...".to_string(), 150);
        self.state.stream_pool.clear();

        let provider = self.provider_for_subject(id);
        let mut context = self.request_context();
        context.provider = provider;
        context
    }

    pub(super) fn run_details_request(
        &self,
        id: String,
        force_refresh: bool,
        context: RequestContext,
    ) {
        let request_id = self.state.active_details_request;
        let service = self.service.clone();
        let sender = self.action_sender.clone();
        tokio::spawn(async move {
            if !force_refresh {
                let id_for_cache = id.clone();
                if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                    crate::cache::get_provider_details_cache(context.provider, &id_for_cache)
                })
                .await
                {
                    sender
                        .send(Action::DetailsSuccess(
                            context,
                            request_id,
                            id.clone(),
                            cached,
                        ))
                        .ok();
                    return;
                }
            }

            let result = service.details(context.provider, &id).await;
            match result {
                Ok(details) => {
                    let id_for_cache = id.clone();
                    let details_for_cache = details.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        crate::cache::set_provider_details_cache(
                            context.provider,
                            &id_for_cache,
                            &details_for_cache,
                        )
                    })
                    .await;
                    sender
                        .send(Action::DetailsSuccess(context, request_id, id, details))
                        .ok();
                }
                Err(error) => {
                    sender
                        .send(Action::DetailsFailure(context, request_id, error))
                        .ok();
                }
            }
        });
    }

    pub(super) fn run_homepage_request(&self, tab_id: String, page: usize) {
        let request_id = self.state.active_homepage_request;
        let service = self.service.clone();
        let sender = self.action_sender.clone();
        tokio::spawn(async move {
            let t_clone = tab_id.clone();
            if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                crate::cache::get_homepage_cache(&t_clone, page)
            })
            .await
            {
                sender
                    .send(Action::HomepageSuccess {
                        request_id,
                        tab_id: tab_id.clone(),
                        page,
                        payload: cached,
                    })
                    .ok();
                return;
            }

            match service.homepage(&tab_id, page).await {
                Ok(res) => {
                    let r_clone = res.clone();
                    let t_clone = tab_id.clone();
                    tokio::task::spawn_blocking(move || {
                        crate::cache::set_homepage_cache(&t_clone, page, &r_clone);
                    });
                    sender
                        .send(Action::HomepageSuccess {
                            request_id,
                            tab_id,
                            page,
                            payload: res,
                        })
                        .ok();
                }
                Err(error) => {
                    sender
                        .send(Action::HomepageFailure(request_id, format!("{:?}", error)))
                        .ok();
                }
            }
        });
    }

    pub(super) fn extract_homepage_subjects(payload: &serde_json::Value) -> Vec<serde_json::Value> {
        let mut extracted_subjects = Vec::new();
        if let Some(items) = payload.get("items").and_then(|i| i.as_array()) {
            for item in items {
                if let Some(banner) = item
                    .get("banner")
                    .and_then(|b| b.get("banners"))
                    .and_then(|b| b.as_array())
                {
                    for banner_item in banner {
                        if let Some(subject) = banner_item.get("subject") {
                            extracted_subjects.push(subject.clone());
                        }
                    }
                }
                if let Some(custom_data) = item
                    .get("customData")
                    .and_then(|c| c.get("items"))
                    .and_then(|i| i.as_array())
                {
                    for custom_item in custom_data {
                        if let Some(subject) = custom_item.get("subject") {
                            extracted_subjects.push(subject.clone());
                        }
                    }
                }
                if let Some(subjects) = item.get("subjects").and_then(|s| s.as_array()) {
                    for subject in subjects {
                        extracted_subjects.push(subject.clone());
                    }
                }
            }
        }
        extracted_subjects
    }

    pub(super) fn extract_browse_subjects(
        payload: &serde_json::Value,
        preset: BrowsePreset,
    ) -> Vec<serde_json::Value> {
        let Some(items) = payload.get("items").and_then(|items| items.as_array()) else {
            return Vec::new();
        };

        let matching_items: Vec<_> = items
            .iter()
            .filter(|item| {
                item.get("title")
                    .and_then(|title| title.as_str())
                    .is_some_and(|title| browse_group_matches(title, preset))
            })
            .collect();

        let groups = if matching_items.is_empty() {
            items.iter().collect()
        } else {
            matching_items
        };

        let mut subjects = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        let rank_metric = preset.metric() == crate::tui::state::BrowseMetric::Trending;

        for group in groups {
            let Some(group_subjects) = group.get("subjects").and_then(|s| s.as_array()) else {
                continue;
            };
            for (index, subject) in group_subjects.iter().enumerate() {
                let mut subject = subject.clone();
                let id_opt = subject.get("subjectId").and_then(|i| i.as_str());
                if let Some(id) = id_opt {
                    if seen_ids.contains(id) {
                        continue;
                    }
                    seen_ids.insert(id.to_string());
                }
                if rank_metric && let Some(subject_object) = subject.as_object_mut() {
                    subject_object.insert(
                        "__browse_rank".to_string(),
                        serde_json::json!((group_subjects.len() - index) as f64),
                    );
                }
                subjects.push(subject);
            }
        }

        subjects
    }

    pub(super) fn append_homepage_subjects(&mut self, subjects: Vec<serde_json::Value>) -> usize {
        let mut count = 0;
        for item in subjects {
            let id = item
                .get("subjectId")
                .and_then(|si| si.as_str())
                .unwrap_or("")
                .to_string();
            let raw_title = item
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let clean_title = crate::providers::moviebox::clean_moviebox_title(&raw_title);
            let stype = item
                .get("subjectType")
                .and_then(|st| st.as_i64())
                .unwrap_or(0);
            let release_year = item
                .get("releaseDate")
                .and_then(|rd| rd.as_str())
                .unwrap_or("")
                .split('-')
                .next()
                .unwrap_or("")
                .to_string();
            let cover_url = item
                .get("cover")
                .and_then(|c| c.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());
            let season = item.get("season").and_then(|s| s.as_u64()).unwrap_or(0) as usize;
            let metrics = extract_browse_metrics(&item);

            if let Some(existing) = self.state.search_results.iter_mut().find(|r| r.id == id) {
                let stored_metrics = self.state.browse_metrics.entry(id.clone()).or_default();
                stored_metrics.trending = stored_metrics.trending.or(metrics.trending);
                stored_metrics.rating = stored_metrics.rating.or(metrics.rating);
                stored_metrics.recent_rating =
                    stored_metrics.recent_rating.or(metrics.recent_rating);
                stored_metrics.popularity = stored_metrics.popularity.or(metrics.popularity);
                if season > existing.season {
                    existing.season = season;
                    existing.title = clean_title;
                    existing.stype = stype;
                    existing.release_year = release_year;
                    existing.cover_url = cover_url;
                }
                continue;
            }

            let raw_lower = raw_title.to_lowercase();
            let is_dub = raw_lower.contains("[hindi]")
                || raw_lower.contains("[tamil]")
                || raw_lower.contains("[telugu]")
                || raw_lower.contains("[english]");

            if is_dub
                && self
                    .state
                    .search_results
                    .iter()
                    .any(|r| r.title == clean_title && r.stype == stype)
            {
                continue;
            }

            if self.state.search_results.iter().any(|r| {
                r.title == clean_title && r.release_year == release_year && r.stype == stype
            }) {
                continue;
            }

            if !id.is_empty() {
                self.state.browse_metrics.insert(id.clone(), metrics);
                self.state.search_results.push(SearchResult {
                    id,
                    title: clean_title,
                    stype,
                    release_year,
                    cover_url,
                    season,
                    episode: 1,
                    provider: ProviderKind::MovieBox,
                });
                count += 1;
            }
        }
        count
    }

    pub(super) fn sort_browse_results(&mut self) {
        let Some(preset) = self.state.active_browse_preset else {
            return;
        };
        let metrics = self.state.browse_metrics.clone();
        let metric = preset.metric();
        let descending = preset.descending();
        self.state.search_results.sort_by(|left, right| {
            let left_value = metrics
                .get(&left.id)
                .and_then(|values| values.value(metric));
            let right_value = metrics
                .get(&right.id)
                .and_then(|values| values.value(metric));
            let metric_order = compare_browse_values(left_value, right_value, descending);
            if left_value.is_none() && right_value.is_none() {
                metric_order
            } else {
                metric_order
                    .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
            }
        });
    }

    pub(super) fn prefetch_visible_posters(&mut self) {
        if !self.state.image_supported || self.state.search_results.is_empty() {
            return;
        }
        let total = self.state.search_results.len();
        let selected = self.state.search_list_state.selected().unwrap_or(0);
        let offset = self.state.search_list_state.offset();
        let visible = self.state.visible_items.max(8);

        let base_start = offset.min(selected);
        let start = base_start.saturating_sub(6);
        let end = (offset + visible + 14).min(total);

        if start < end {
            let slice: Vec<(String, Option<String>, ProviderKind)> = self.state.search_results
                [start..end]
                .iter()
                .map(|r| (r.id.clone(), r.cover_url.clone(), r.provider))
                .collect();
            self.spawn_search_posters(slice);
        }
    }

    pub(super) fn spawn_search_posters(
        &mut self,
        results: Vec<(String, Option<String>, ProviderKind)>,
    ) {
        if !self.state.image_supported {
            return;
        }

        let mut to_fetch = Vec::new();
        for (id, cover_url, provider) in results {
            if self.state.search_posters.contains(&id) || self.state.failed_posters.contains(&id) {
                continue;
            }
            if !self.state.in_flight_posters.insert(id.clone()) {
                continue;
            }
            to_fetch.push((id, cover_url, provider));
        }

        if to_fetch.is_empty() {
            return;
        }

        let sender = self.action_sender.clone();
        let service = self.service.clone();

        tokio::spawn(async move {
            let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
            for (id, cover_url, provider) in to_fetch {
                let permit = sem.clone().acquire_owned().await.ok();
                let tx = sender.clone();
                let service = service.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let id_clone = id.clone();
                    if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                        let id_c = id_clone.clone();
                        move || crate::cache::get_namespaced_image_cache("posters", &id_c)
                    })
                    .await
                    {
                        if let Some(img) = network::decode_poster(bytes).await {
                            tx.send(Action::SearchPosterLoaded(id_clone, Some(img)))
                                .ok();
                            return;
                        }
                    }

                    let mut resolved_url = cover_url;
                    if resolved_url.is_none() {
                        if let Ok(details) = service.details(provider, &id).await {
                            resolved_url = crate::tui::app::playback::extract_cover_url(&details);
                        }
                    }

                    if let Some(url) = resolved_url {
                        if !url.is_empty() {
                            if let Some(bytes) = service.fetch_poster_bytes(&url).await {
                                let bytes_clone = bytes.clone();
                                let id_c = id.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    crate::cache::set_namespaced_image_cache(
                                        "posters",
                                        &id_c,
                                        &bytes_clone,
                                    );
                                })
                                .await;

                                if let Some(img) = network::decode_poster(bytes).await {
                                    tx.send(Action::SearchPosterLoaded(id, Some(img))).ok();
                                    return;
                                }
                            }
                        }
                    }

                    tx.send(Action::SearchPosterLoaded(id, None)).ok();
                });
            }
        });
    }
}
