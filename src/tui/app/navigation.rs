use super::{App, network};
use crate::providers::models::{ProviderKind, Release};
use crate::tui::{action::Action, state::Screen};

impl App {
    pub(super) fn switch_provider(&mut self, provider: ProviderKind) {
        if self.state.is_tv_mode {
            return;
        }
        if provider == self.state.active_provider {
            return;
        }
        self.prepare_image_refresh();
        self.state
            .fetch_cancel
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.state.fetch_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.state.provider_generation = self.state.provider_generation.wrapping_add(1);
        self.state.active_provider = provider;
        self.state.active_screen = Screen::Home;
        self.state.is_homepage_mode = false;
        self.state.browse_view = None;
        self.state.browse_menu_open = false;
        self.state.is_tv_mode = false;
        self.state.is_loading = false;
        self.state.is_fetching_streams = false;
        self.state.stream_error = None;
        self.state.search_results.clear();
        self.state.search_suggestions.clear();
        self.state.search_preview = None;
        self.state.preview_loading = false;
        self.state.selected_details = None;
        self.state.selected_resources = None;
        self.state.active_subject_id = None;
        self.state.available_seasons.clear();
        self.state.available_episode_numbers.clear();
        self.state.stream_pool.clear();
        self.state.is_resolving_playback = false;
        self.state.search_posters.clear();
        self.state.search_poster_protocols.clear();
        self.state.image_cache.clear();
        self.state.preview_cache.clear();
        self.state.poster_image = None;
        self.state.poster_protocol = None;
        self.state.search_poster_protocols.clear();
        self.state.search_list_state.select(None);
        self.state.resource_list_state.select(None);
        self.state.set_status(
            format!(
                "{} selected. Search uses only this provider.",
                provider.label()
            ),
            180,
        );
        self.persist_config();
        if provider == ProviderKind::MovieBox {
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.init().await;
            });
        }
    }

    pub(super) fn cycle_provider(&mut self) {
        let available_providers: Vec<ProviderKind> = ProviderKind::ENABLED
            .into_iter()
            .filter(|p| !p.is_bdix() || self.state.bdix_enabled)
            .collect();

        if available_providers.is_empty() {
            return;
        }

        let current = available_providers
            .iter()
            .position(|provider| *provider == self.state.active_provider)
            .unwrap_or(0);
        let next = available_providers[(current + 1) % available_providers.len()];
        self.switch_provider(next);
    }

    pub(super) fn prepare_image_refresh(&mut self) {
        if self.state.image_picker.as_ref().is_some_and(|picker| {
            !matches!(
                picker.protocol_type(),
                ratatui_image::picker::ProtocolType::Halfblocks
            )
        }) {
            self.state.clear_terminal_before_draw = true;
        }
    }

    pub(super) fn prepare_image_soft_refresh(&mut self) {
        self.state.poster_protocol = None;
        self.state.search_poster_protocols.clear();
        self.state.dirty = true;
    }

    pub(super) fn cycle_details_pane(&mut self, forward: bool) {
        use crate::tui::state::DetailsPane;

        if self.state.active_screen != Screen::Details {
            return;
        }

        let has_languages = self
            .state
            .selected_details
            .as_ref()
            .and_then(|details| details.get("dubs"))
            .and_then(|dubs| dubs.as_array())
            .is_some_and(|dubs| dubs.len() > 1);
        let is_series = !self.state.available_seasons.is_empty();
        let mut panes = Vec::new();
        if has_languages {
            panes.push(DetailsPane::Languages);
        }
        if is_series {
            panes.push(DetailsPane::Seasons);
            panes.push(DetailsPane::Episodes);
        }
        panes.push(DetailsPane::Streams);

        let current = panes
            .iter()
            .position(|pane| *pane == self.state.details_pane)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % panes.len()
        } else if current == 0 {
            panes.len() - 1
        } else {
            current - 1
        };
        self.state.details_pane = panes[next];
    }

    pub(super) fn trigger_episode_fetch(&mut self) {
        if let Some(id) = self.state.active_subject_id.clone() {
            let stype = self
                .state
                .selected_details
                .as_ref()
                .map(crate::tui::state::stype)
                .unwrap_or(1);

            let (se, ep) = if stype == 2 {
                let se_idx = self.state.season_list_state.selected().unwrap_or(0);
                let ep_idx = self.state.episode_list_state.selected().unwrap_or(0);

                let season_num = self
                    .state
                    .available_seasons
                    .get(se_idx)
                    .and_then(|s| s.get("se"))
                    .and_then(|s| s.as_i64())
                    .unwrap_or(1) as usize;

                let ep_num =
                    if let Some(ep_numbers) = self.state.available_episode_numbers.get(se_idx) {
                        ep_numbers.get(ep_idx).copied().unwrap_or(ep_idx + 1)
                    } else {
                        ep_idx + 1
                    };
                (season_num, ep_num)
            } else {
                (0, 0)
            };

            self.state.selected_season = se;
            self.state.selected_episode = ep;
            self.state.resource_list_state.select(None);
            self.state.stream_error = None;
            self.state.active_resource_request = self.state.active_resource_request.wrapping_add(1);

            let memory_cached = self
                .state
                .stream_pool
                .get(&id)
                .and_then(|pool| pool.episode_index.get(&(se, ep)))
                .filter(|streams| !streams.is_empty())
                .cloned();

            if let Some(streams) = memory_cached {
                if let Some(pool) = self.state.stream_pool.get_mut(&id) {
                    pool.episode_index.insert((se, ep), streams.clone());
                }
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.set_status("Loading streams...".to_string(), 90);
                self.state.pending_episode_fetch = None;
                let sender = self.action_sender.clone();
                let context = self.request_context();
                let request_id = self.state.active_resource_request;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    sender
                        .send(Action::EpisodeStreamsReady(
                            context,
                            request_id,
                            id,
                            se,
                            ep,
                            serde_json::Value::Array(streams),
                        ))
                        .ok();
                });
            } else {
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.set_status("Loading streams...".to_string(), 90);

                self.state.pending_episode_fetch = Some((id.clone(), se, ep));
                self.state.last_episode_nav = std::time::Instant::now();
            }
        }
    }

    pub(super) fn get_selected_link(&self) -> Option<String> {
        self.state
            .selected_resources
            .as_ref()
            .and_then(|res| res.get("list"))
            .and_then(|l| l.as_array())
            .and_then(|list| {
                let idx = self.state.resource_list_state.selected().unwrap_or(0);
                list.get(idx)
            })
            .and_then(|file| file.get("resourceLink"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
    }

    pub(super) fn get_selected_resource_id(&self) -> Option<String> {
        self.state
            .selected_resources
            .as_ref()
            .and_then(|res| res.get("list"))
            .and_then(|l| l.as_array())
            .and_then(|list| {
                let idx = self.state.resource_list_state.selected().unwrap_or(0);
                list.get(idx)
            })
            .and_then(|file| file.get("resourceId"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
    }

    pub(super) fn get_selected_release(&self) -> Option<Release> {
        self.state
            .selected_resources
            .as_ref()?
            .get("list")?
            .as_array()?
            .get(self.state.resource_list_state.selected().unwrap_or(0))?
            .get("_fourk_release")
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

impl App {
    pub(super) async fn handle_navigation(&mut self, action: Action) -> Option<()> {
        match action {
            Action::GoBack => {
                self.prepare_image_refresh();
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    self.state.player_picker_link = None;
                    self.state.player_picker_subtitle = None;
                    return None;
                }
                if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.pending_play_link = None;
                    return None;
                }
                if self.state.show_help {
                    self.state.show_help = false;
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Home => {
                        if self.state.browse_menu_open {
                            self.state.browse_menu_open = false;
                            return None;
                        }
                        if self.state.browse_view.is_some() {
                            self.state.browse_view = None;
                            self.state.browse_menu_open = false;
                            self.state.search_poster_protocols.clear();
                            self.state.search_results.clear();
                            self.state.search_error = None;
                            self.state.search_query.clear();
                            self.state.search_preview = None;
                            self.state.set_status("Browse exited.".to_string(), 150);
                            return None;
                        }
                        if !self.state.search_results.is_empty()
                            || !self.state.search_query.is_empty()
                        {
                            self.state.search_poster_protocols.clear();
                            self.state.search_results.clear();
                            self.state.search_error = None;
                            self.state.search_query.clear();
                            self.state.search_preview = None;
                            self.state.set_status("Search cleared.".to_string(), 150);
                        }
                    }
                    Screen::Details => {
                        self.state
                            .fetch_cancel
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        self.state.stream_pool.clear();
                        self.state.pending_episode_fetch = None;
                        self.state.selected_resources = None;
                        self.state.active_screen = Screen::Home;
                        self.state.is_loading = false;
                        self.state.language_chosen = false;
                        self.state
                            .set_status("Select a movie/series and press Enter".to_string(), 150);
                    }
                }
            }

            Action::SelectLanguage(idx) => {
                if let Some(details) = &self.state.selected_details
                    && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                    && let Some(dub) = dubs.get(idx)
                    && let Some(id) = dub.get("subjectId").and_then(|i| i.as_str())
                {
                    let next_id = id.to_string();
                    self.state.selected_resources = None;
                    self.state.resource_list_state.select(None);
                    self.state.language_chosen = true;
                    self.state
                        .set_status("Switching language...".to_string(), 150);
                    self.action_sender
                        .send(Action::FetchDetails(next_id, false))
                        .ok();
                }
            }

            Action::MoveUp => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_image_refresh();
                }
                if self.state.player_picker_popup {
                    let i = match self.state.player_picker_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.state.available_players.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.state.player_picker_state.select(Some(i));
                    return None;
                } else if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current > 0 {
                        self.state.subtitle_list_state.select(Some(current - 1));
                    }
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Home => {
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            self.state.search_list_state.select(Some(current - 1));
                            if let Some(res) = self.state.search_results.get(current - 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        }
                    }
                    Screen::Details => match self.state.details_pane {
                        crate::tui::state::DetailsPane::Streams => {
                            let current = self.state.resource_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.resource_list_state.select(Some(current - 1));
                            }
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            let current = self.state.season_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.season_list_state.select(Some(current - 1));
                                self.state.episode_list_state.select(Some(0));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            let current = self.state.episode_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.episode_list_state.select(Some(current - 1));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Languages => {
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.language_list_state.select(Some(current - 1));
                                self.action_sender
                                    .send(Action::SelectLanguage(current - 1))
                                    .ok();
                            }
                        }
                    },
                }
            }

            Action::TabPane => {
                self.cycle_details_pane(true);
            }

            Action::BackTabPane => {
                self.cycle_details_pane(false);
            }

            Action::MoveDown => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_image_refresh();
                }
                if self.state.player_picker_popup {
                    let i = match self.state.player_picker_state.selected() {
                        Some(i) => {
                            if i >= self.state.available_players.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.state.player_picker_state.select(Some(i));
                    return None;
                } else if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current + 1 < self.state.subtitle_list.len() {
                        self.state.subtitle_list_state.select(Some(current + 1));
                    }
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Home => {
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current + 1 < self.state.search_results.len() {
                            self.state.search_list_state.select(Some(current + 1));
                            if let Some(res) = self.state.search_results.get(current + 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        } else if !self.state.is_tv_mode
                            && !self.state.is_loading
                            && !self.state.search_results.is_empty()
                            && self.state.browse_view.is_none()
                        {
                            let next_page = self.state.current_page + 1;
                            if self.state.is_homepage_mode {
                                self.action_sender
                                    .send(Action::FetchHomepage {
                                        tab_id: self.state.current_tab_id.clone(),
                                        page: next_page,
                                    })
                                    .ok();
                            } else {
                                self.state.current_page = next_page;
                                let query = self.state.search_query.clone();
                                let client = self.client.clone();
                                let fourk_client = self.fourk_client.clone();
                                let circleftp_client = self.circleftp_client.clone();
                                let dhakaflix_client = self.dhakaflix_client.clone();
                                let sender = self.action_sender.clone();
                                let context = self.request_context();
                                self.state.is_loading = true;
                                self.state
                                    .set_status(format!("Loading page {}...", next_page), 150);
                                tokio::spawn(async move {
                                    let result = network::provider_search(
                                        &client,
                                        &fourk_client,
                                        &circleftp_client,
                                        &dhakaflix_client,
                                        context.provider,
                                        &query,
                                        next_page,
                                    )
                                    .await;
                                    match result {
                                        Ok(res) => {
                                            sender
                                                .send(Action::SearchSuccess {
                                                    context,
                                                    query,
                                                    payload: res,
                                                })
                                                .ok();
                                        }
                                        Err(e) => {
                                            sender.send(Action::SearchFailure(context, e)).ok();
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Screen::Details => match self.state.details_pane {
                        crate::tui::state::DetailsPane::Streams => {
                            let res_opt = &self.state.selected_resources;
                            let list_opt = res_opt
                                .as_ref()
                                .and_then(|r| r.get("list"))
                                .and_then(|l| l.as_array());
                            if let Some(list) = list_opt {
                                let current =
                                    self.state.resource_list_state.selected().unwrap_or(0);
                                if current + 1 < list.len() {
                                    self.state.resource_list_state.select(Some(current + 1));
                                }
                            }
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            let current = self.state.season_list_state.selected().unwrap_or(0);
                            if current + 1 < self.state.available_seasons.len() {
                                self.state.season_list_state.select(Some(current + 1));
                                self.state.episode_list_state.select(Some(0));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            let current = self.state.episode_list_state.selected().unwrap_or(0);
                            if let Some(season_idx) = self.state.season_list_state.selected() {
                                if let Some(ep_numbers) =
                                    self.state.available_episode_numbers.get(season_idx)
                                {
                                    if current + 1 < ep_numbers.len() {
                                        self.state.episode_list_state.select(Some(current + 1));
                                        self.trigger_episode_fetch();
                                    }
                                }
                            }
                        }
                        crate::tui::state::DetailsPane::Languages => {
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if let Some(details) = &self.state.selected_details
                                && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                                && current + 1 < dubs.len()
                            {
                                self.state.language_list_state.select(Some(current + 1));
                                self.action_sender
                                    .send(Action::SelectLanguage(current + 1))
                                    .ok();
                            }
                        }
                    },
                }
            }

            Action::MoveLeft => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_image_refresh();
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = self.state.visible_items.max(1);
                    if current > jump {
                        self.state.search_list_state.select(Some(current - jump));
                    } else {
                        self.state.search_list_state.select(Some(0));
                    }
                    if let Some(res) = self
                        .state
                        .search_results
                        .get(self.state.search_list_state.selected().unwrap_or(0))
                    {
                        self.action_sender
                            .send(Action::FetchPreview(res.id.clone()))
                            .ok();
                    }
                }
            }

            Action::MoveRight => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_image_refresh();
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = self.state.visible_items.max(1);
                    let total = self.state.search_results.len();
                    if current + jump < total {
                        self.state.search_list_state.select(Some(current + jump));
                    } else if total > 0 {
                        self.state.search_list_state.select(Some(total - 1));
                    }
                    if let Some(res) = self
                        .state
                        .search_results
                        .get(self.state.search_list_state.selected().unwrap_or(0))
                    {
                        self.action_sender
                            .send(Action::FetchPreview(res.id.clone()))
                            .ok();
                    }
                }
            }

            Action::Submit => {
                if self.state.is_loading {
                    return None;
                }
                if self.state.last_search_edit.elapsed().as_millis() < 500 {
                    return None;
                }
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    let idx = self.state.player_picker_state.selected().unwrap_or(0);
                    if let Some(player) = self.state.available_players.get(idx).copied() {
                        if let Some(source) = self.state.player_picker_playback.take() {
                            self.action_sender
                                .send(Action::LaunchPlayback(player, source))
                                .ok();
                        } else if let Some(link) = self.state.player_picker_link.take() {
                            let sub = self.state.player_picker_subtitle.take();
                            self.action_sender
                                .send(Action::LaunchPlayer(player, link, sub))
                                .ok();
                        }
                    }
                    return None;
                }
                if self.state.subtitle_popup {
                    self.state.subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_url = self.state.subtitle_list.get(idx).map(|(_, u)| u.clone());
                    if let Some(link) = self.state.pending_play_link.take() {
                        let open_with = self.state.pending_open_with;
                        if open_with {
                            self.action_sender
                                .send(Action::ShowPlayerPicker(link, sub_url))
                                .ok();
                        } else {
                            self.action_sender
                                .send(Action::LaunchMpv(link, sub_url))
                                .ok();
                        }
                    }
                    return None;
                } else if self.state.is_download_subtitle_popup {
                    self.state.is_download_subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_name = self.state.subtitle_list.get(idx).map(|(n, _)| n.clone());
                    let sub_url = self.state.subtitle_list.get(idx).map(|(_, u)| u.clone());
                    let sub_url_final = sub_url.filter(|s| !s.is_empty());

                    if self.state.download_queue_total > 0 {
                        self.state.season_subtitle_preference = sub_name.filter(|n| n != "None");
                    }

                    self.action_sender
                        .send(Action::DownloadStream(sub_url_final))
                        .ok();
                    return None;
                }
                if self.state.active_screen == Screen::Home {
                    let idx_opt = self.state.search_list_state.selected();
                    let item_opt =
                        idx_opt.and_then(|idx| self.state.search_results.get(idx).cloned());
                    if let Some(item) = item_opt {
                        if self.state.is_tv_mode || item.stype == 3 {
                            self.action_sender
                                .send(Action::LaunchMpv(item.id.clone(), None))
                                .ok();
                            return None;
                        }
                        self.state.active_screen = Screen::Details;
                        self.state.selected_details = None;
                        self.state.selected_resources = None;
                        self.state.is_loading = true;
                        self.state.is_fetching_streams = false;
                        self.state.stream_error = None;
                        self.state.resource_list_state.select(None);
                        self.state.language_list_state.select(Some(0));
                        self.state.season_list_state.select(Some(0));
                        self.state.episode_list_state.select(Some(0));
                        self.state.language_chosen = false;
                        self.state.poster_image = None;
                        self.state.available_seasons.clear();
                        self.state
                            .set_status(format!("Loading details for {}...", item.title), 150);

                        let sender = self.action_sender.clone();
                        sender
                            .send(Action::FetchDetails(item.id.clone(), false))
                            .ok();
                    }
                }
            }
            _ => return None,
        }
        None
    }
}
