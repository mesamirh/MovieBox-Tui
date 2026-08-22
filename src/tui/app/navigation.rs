use super::App;
use crate::providers::models::{ProviderKind, Release};
use crate::tui::{action::Action, state::Screen};

impl App {
    fn remember_player_preference(&mut self, player: crate::tui::state::PlayerKind) {
        self.state.default_player = Some(player.config_key().to_string());
        if let Some(index) = self
            .state
            .available_players
            .iter()
            .position(|&kind| kind == player)
        {
            let preferred = self.state.available_players.remove(index);
            self.state.available_players.insert(0, preferred);
        }
        self.persist_config();
    }

    pub(super) fn switch_provider(&mut self, provider: ProviderKind) {
        if self.state.is_tv_mode {
            return;
        }
        if provider == self.state.active_provider {
            return;
        }
        self.prepare_image_soft_refresh();
        self.reset_mode_state();
        self.state.active_provider = provider;
        self.state.active_screen = Screen::Home;
        self.state.details_pane = crate::tui::state::DetailsPane::default();
        self.state.selected_season = 1;
        self.state.selected_episode = 1;
        self.state.language_chosen = false;
        self.state.stream_pool.clear();
        self.state
            .cancel_download
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.state.download_progress = None;
        self.state.download_status = None;
        self.state.download_queue.clear();
        self.state.download_queue_total = 0;
        self.state.is_waiting_for_download_stream = false;
        self.state.search_posters.clear();
        self.state.failed_posters.clear();
        self.state.search_poster_protocols.clear();
        self.state.in_flight_posters.clear();
        self.state.image_cache.clear();
        self.state.preview_cache.clear();
        self.state.poster_image = None;
        self.state.poster_protocol = None;
        self.state.search_list_state.select(None);
        self.state.resource_list_state.select(None);
        self.state.dirty = true;
        self.state.set_status(
            format!(
                "{} selected. Search uses only this provider.",
                provider.label()
            ),
            180,
        );
        self.persist_config();
        if provider == ProviderKind::MovieBox {
            let client = self.service.client.clone();
            tokio::spawn(async move {
                let _ = client.init().await;
            });
        }
        let trimmed = self.state.search_query.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('/') {
            let query = trimmed.to_string();
            let context = self.prepare_search_request(&query);
            self.run_search_request(query, false, context);
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
        if self.state.image_picker.is_some() {
            self.state.clear_terminal_before_draw = true;
        }
    }

    pub(super) fn prepare_image_soft_refresh(&mut self) {
        self.state.poster_protocol = None;
        self.state.search_poster_protocols.clear();
        self.state.dirty = true;
    }

    pub(super) fn provider_for_subject(&self, subject_id: &str) -> ProviderKind {
        self.state
            .search_results
            .iter()
            .find(|result| result.id == subject_id)
            .map(|result| result.provider)
            .or_else(|| {
                self.get_selected_release()
                    .as_ref()
                    .map(|release| release.provider)
            })
            .unwrap_or(self.state.active_provider)
    }

    pub(super) fn current_subject_provider(&self) -> ProviderKind {
        self.state
            .active_subject_id
            .as_deref()
            .map(|subject_id| self.provider_for_subject(subject_id))
            .unwrap_or(self.state.active_provider)
    }

    pub(super) fn trigger_next_page_if_needed(&mut self) {
        if self.state.is_tv_mode
            || self.state.is_loading
            || self.state.search_results.is_empty()
            || self.state.active_browse_preset.is_some()
            || self.state.search_query.trim().starts_with('/')
        {
            return;
        }
        let total = self.state.search_results.len();
        let selected = self.state.search_list_state.selected().unwrap_or(0);
        let offset = self.state.search_list_state.offset();
        let visible = self.state.visible_items.max(6);

        if selected + 8 >= total || offset + visible + 4 >= total {
            let next_page = self.state.current_page + 1;
            if self.state.is_homepage_mode {
                self.action_sender
                    .send(Action::FetchHomepage {
                        tab_id: self.state.current_tab_id.clone(),
                        page: next_page,
                    })
                    .ok();
            } else {
                let query = self.state.search_query.clone();
                let service = self.service.clone();
                let sender = self.action_sender.clone();
                let context = self.request_context();
                let request_id = self.state.active_search_request;
                self.state.is_loading = true;
                tokio::spawn(async move {
                    let q = query.clone();
                    let provider = context.provider;
                    if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                        crate::cache::get_provider_search_cache(provider, &q, next_page)
                    })
                    .await
                    {
                        sender
                            .send(Action::SearchSuccess {
                                context,
                                request_id,
                                query,
                                page: next_page,
                                payload: cached,
                            })
                            .ok();
                        return;
                    }

                    let result = service.search(context.provider, &query, next_page).await;
                    match result {
                        Ok(res) => {
                            let q = query.clone();
                            let provider = context.provider;
                            let cached = res.clone();
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_search_cache(
                                    provider, &q, next_page, &cached,
                                );
                            });
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    request_id,
                                    query,
                                    page: next_page,
                                    payload: res,
                                })
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::SearchFailure(context, request_id, next_page, e))
                                .ok();
                        }
                    }
                });
            }
        }
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
        let item = self
            .state
            .selected_resources
            .as_ref()?
            .get("list")?
            .as_array()?
            .get(self.state.resource_list_state.selected().unwrap_or(0))?;

        item.get("_addon_release")
            .or_else(|| item.get("_fourk_release"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
}

impl App {
    pub(super) async fn handle_navigation(&mut self, action: Action) -> Option<()> {
        match action {
            Action::GoBack => {
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    self.state.player_picker_link = None;
                    self.state.player_picker_subtitle = None;
                    self.state.player_picker_playback = None;
                    self.state.player_picker_state.select(None);
                    return None;
                }
                if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.pending_play_link = None;
                    self.state.pending_open_with = false;
                    self.state.subtitle_list.clear();
                    self.state.subtitle_list_state.select(None);
                    return None;
                }
                if self.state.show_help {
                    self.state.show_help = false;
                    return None;
                }
                if self.state.show_browse_popup {
                    self.state.show_browse_popup = false;
                    self.state.browse_list_state.select(None);
                    return None;
                }
                if self.state.favorites_focus {
                    self.state.favorites_focus = false;
                    self.state.favorites_landing_state.select(None);
                    return None;
                }
                match self.state.active_screen {
                    Screen::Home => {
                        self.state.is_loading = false;
                        self.state.active_search_request =
                            self.state.active_search_request.wrapping_add(1);
                        self.state.active_homepage_request =
                            self.state.active_homepage_request.wrapping_add(1);
                        self.state.active_preview_request =
                            self.state.active_preview_request.wrapping_add(1);
                        self.state.search_poster_protocols.clear();
                        self.state.search_results.clear();
                        self.state.active_browse_preset = None;
                        self.state.active_addon_catalog = None;
                        self.state.browse_metrics.clear();
                        self.state.search_error = None;
                        self.state.search_query.clear();
                        self.state.search_suggestions.clear();
                        self.state.suggest_index = None;
                        self.state.search_preview = None;
                        self.state.preview_loading = false;
                        self.state.poster_image = None;
                        self.state.poster_protocol = None;
                        self.state.search_list_state.select(None);
                        self.state.set_status("Search cleared.".to_string(), 150);
                    }
                    Screen::Details => {
                        self.state
                            .fetch_cancel
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        self.state.active_preview_request =
                            self.state.active_preview_request.wrapping_add(1);
                        self.state.active_details_request =
                            self.state.active_details_request.wrapping_add(1);
                        self.state.active_resource_request =
                            self.state.active_resource_request.wrapping_add(1);
                        self.state.stream_pool.clear();
                        self.state.pending_episode_fetch = None;
                        self.state.selected_details = None;
                        self.state.selected_resources = None;
                        self.state.active_subject_id = None;
                        self.state.available_seasons.clear();
                        self.state.available_episode_numbers.clear();
                        self.state.is_fetching_streams = false;
                        self.state.stream_error = None;
                        self.state.poster_image = None;
                        self.state.poster_protocol = None;
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
                    && let Some(id) = dub.get("subjectId").and_then(crate::tui::state::subject_id)
                {
                    let next_id = id.to_string();
                    self.state.selected_resources = None;
                    self.state.resource_list_state.select(None);
                    self.state.language_chosen = true;
                    self.state.language_list_state.select(Some(idx));
                    if self.state.active_subject_id.as_deref() == Some(&next_id) {
                        if !self.state.stream_pool.contains_key(&next_id) {
                            self.state.set_status("Loading streams...".to_string(), 150);
                            self.action_sender
                                .send(Action::InitStreamPool(next_id))
                                .ok();
                        } else {
                            self.trigger_episode_fetch();
                        }
                    } else {
                        self.state
                            .set_status("Switching language...".to_string(), 150);
                        self.action_sender
                            .send(Action::FetchDetails(next_id, false))
                            .ok();
                    }
                }
            }

            Action::MoveUp => {
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
                    Screen::Home => {
                        if self.state.favorites_focus {
                            let current =
                                self.state.favorites_landing_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.favorites_landing_state.select(Some(current - 1));
                            }
                            return None;
                        }
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            self.state.search_list_state.select(Some(current - 1));
                            if let Some(res) = self.state.search_results.get(current - 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        }
                        self.prefetch_visible_posters();
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
                    Screen::Home => {
                        if self.state.favorites_focus {
                            let total = self.state.favorites_landing_items().len();
                            let current =
                                self.state.favorites_landing_state.selected().unwrap_or(0);
                            if current + 1 < total {
                                self.state.favorites_landing_state.select(Some(current + 1));
                            }
                            return None;
                        }
                        if self.state.search_results.is_empty()
                            && self.state.search_query.trim().is_empty()
                            && self.state.favorites_landing_visible()
                        {
                            self.state.favorites_focus = true;
                            self.state.favorites_landing_state.select(Some(0));
                            return None;
                        }
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current + 1 < self.state.search_results.len() {
                            self.state.search_list_state.select(Some(current + 1));
                            if let Some(res) = self.state.search_results.get(current + 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                            self.prefetch_visible_posters();
                        }
                        self.trigger_next_page_if_needed();
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
                            }
                        }
                    },
                }
            }

            Action::MoveLeft => {
                if self.state.active_screen == Screen::Home {
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
                    self.prefetch_visible_posters();
                }
            }

            Action::MoveRight => {
                if self.state.active_screen == Screen::Home {
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
                    self.prefetch_visible_posters();
                    self.trigger_next_page_if_needed();
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
                    let idx = self.state.player_picker_state.selected().unwrap_or(0);
                    if let Some(player) = self.state.available_players.get(idx).copied() {
                        if let Some(source) = self.state.player_picker_playback.clone() {
                            if !crate::tui::player::supports_headers(player, &source.headers) {
                                self.state.set_status(
                                    format!(
                                        "{} cannot play this source; choose mpv or IINA.",
                                        player.label()
                                    ),
                                    180,
                                );
                                return None;
                            }
                        }
                        self.state.player_picker_popup = false;
                        self.remember_player_preference(player);
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

                    if let Some(name) = sub_name.filter(|n| n != "None") {
                        self.state.last_download_subtitle_language = Some(name.clone());
                        if self.state.download_queue_total > 0 {
                            self.state.season_subtitle_preference = Some(name);
                        }
                    }

                    self.action_sender
                        .send(Action::DownloadStream(sub_url_final))
                        .ok();
                    return None;
                }
                if self.state.favorites_focus {
                    if let Some(idx) = self.state.favorites_landing_state.selected() {
                        self.action_sender.send(Action::OpenFavorite(idx)).ok();
                    }
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
                        self.state.active_subject_id = Some(item.id.clone());
                        let mut fallback_details = serde_json::json!({
                            "id": item.id,
                            "subjectId": item.id,
                            "title": item.title,
                            "subjectType": item.stype,
                            "releaseDate": item.release_year,
                            "cover": { "url": item.cover_url },
                        });
                        if let Some(preview) = &self.state.search_preview {
                            if let Some(preview_obj) = preview.as_object() {
                                if let Some(fallback_obj) = fallback_details.as_object_mut() {
                                    for (k, v) in preview_obj {
                                        if !fallback_obj.contains_key(k)
                                            || fallback_obj[k].is_null()
                                        {
                                            fallback_obj.insert(k.clone(), v.clone());
                                        }
                                    }
                                }
                            }
                        }
                        self.state.selected_details = Some(fallback_details);
                        self.state.selected_resources = None;
                        self.state.is_loading = true;
                        self.state.is_fetching_streams = false;
                        self.state.stream_error = None;
                        self.state.resource_list_state.select(None);
                        self.state.language_list_state.select(Some(0));
                        self.state.season_list_state.select(Some(0));
                        self.state.episode_list_state.select(Some(0));
                        self.state.language_chosen = false;
                        if let Some(cached) = self
                            .state
                            .image_cache
                            .get(&item.id)
                            .or_else(|| self.state.search_posters.get(&item.id))
                        {
                            self.state.poster_image = Some((**cached).clone());
                        } else {
                            self.state.poster_image = None;
                        }
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
