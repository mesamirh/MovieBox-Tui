use super::App;
use crate::providers::models::{MediaDetails, ProviderKind, Release};
use crate::tui::{
    action::Action,
    state::{InputMode, Screen},
};

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
        let previous_query = if !self.state.search_query.trim().is_empty() {
            Some(self.state.search_query.trim().to_string())
        } else {
            self.state
                .selected_details
                .as_ref()
                .map(|details| details.title.trim().to_string())
        };
        self.prepare_image_soft_refresh();
        self.reset_mode_state();
        self.state.active_provider = provider;
        self.state.active_screen = Screen::Home;
        self.state.details_pane = crate::tui::state::DetailsPane::default();
        self.state.selected_season = 1;
        self.state.selected_episode = 1;
        self.state.language_chosen = false;
        self.state.stream_pool.clear();
        self.state.has_streams_settled = false;
        self.state.has_search_settled = false;
        self.state.is_waiting_for_download_stream = false;
        self.state.clear_poster_cache();
        self.state.image_cache.clear();
        self.state.preview_cache.clear();
        self.state.search_list_state.select(None);
        self.state.resource_list_state.select(None);
        self.state.dirty = true;
        self.state
            .set_status_default(format!("Provider: {}", provider.label()));
        self.persist_config();
        if provider == ProviderKind::MovieBox {
            let client = self.service.client.clone();
            tokio::spawn(async move {
                let _ = client.init().await;
            });
        }
        if let Some(query) = previous_query.filter(|q| !q.is_empty() && !q.starts_with('/')) {
            self.state.search_query.set_content(&query);
            self.state.is_loading = true;
            self.state.has_search_settled = false;
            let context = self.prepare_search_request(&query);
            self.run_search_request(query, false, context);
        }
    }

    pub(super) fn cycle_provider(&mut self) {
        let available_providers = self.state.available_providers();

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
        self.state.clear_poster_protocols();
        self.state.dirty = true;
    }

    pub(super) fn provider_for_subject(&self, subject_id: &str) -> ProviderKind {
        self.state.provider_for_subject(subject_id)
    }

    pub(super) fn current_subject_provider(&self) -> ProviderKind {
        self.state.current_subject_provider()
    }

    fn result_grid_columns(&self) -> usize {
        self.state
            .last_result_metrics
            .map(|metrics| metrics.columns as usize)
            .unwrap_or(1)
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
        let offset = self.state.result_scroll;
        let visible = self.state.effective_visible_items().max(6);

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
                let query = self.state.search_query.to_string();
                let service = self.service.clone();
                let sender = self.action_sender.clone();
                let context = self.request_context();
                let request_id = self.state.active_search_request;
                self.state.is_loading = true;
                tokio::spawn(async move {
                    let q = query.clone();
                    let provider = context.provider;
                    if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                        crate::cache::get_provider_search_cache_typed(provider, &q, next_page)
                    })
                    .await
                    {
                        sender
                            .send(Action::SearchSuccess {
                                context,
                                request_id,
                                query,
                                page: next_page,
                                items: cached,
                            })
                            .ok();
                        return;
                    }

                    let result = service
                        .search_typed(context.provider, &query, next_page)
                        .await;
                    match result {
                        Ok(items) => {
                            let q = query.clone();
                            let provider = context.provider;
                            let cached = items.clone();
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_search_cache_typed(
                                    provider, &q, next_page, &cached,
                                );
                            });
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    request_id,
                                    query,
                                    page: next_page,
                                    items,
                                })
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::SearchFailure(
                                    context,
                                    request_id,
                                    next_page,
                                    e.user_message(context.provider),
                                ))
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
            .is_some_and(|details| details.has_languages());
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
            let is_series = self
                .state
                .selected_details
                .as_ref()
                .is_some_and(|d| d.is_series());

            let (se, ep) = if is_series {
                let se_idx = self.state.season_list_state.selected().unwrap_or(0);
                let ep_idx = self.state.episode_list_state.selected().unwrap_or(0);

                let season_num = self
                    .state
                    .available_seasons
                    .get(se_idx)
                    .map(|s| s.number)
                    .unwrap_or(1);
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
            self.state.has_streams_settled = false;
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
                self.state.selected_resources.clear();
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.set_status_short("Loading streams...");
                self.state.pending_episode_fetch = None;
                let sender = self.action_sender.clone();
                let context = self.request_context();
                let request_id = self.state.active_resource_request;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    sender
                        .send(Action::EpisodeStreamsReady(
                            context, request_id, id, se, ep, streams,
                        ))
                        .ok();
                });
            } else {
                self.state.selected_resources.clear();
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.set_status_short("Loading streams...");

                self.state.pending_episode_fetch = Some((id.clone(), se, ep));
                self.state.last_episode_nav = std::time::Instant::now();
            }
        }
    }

    pub(super) fn get_selected_link(&self) -> Option<String> {
        let release = self.get_selected_release()?;
        release.direct_url().map(|s| s.to_string())
    }

    pub(super) fn get_selected_resource_id(&self) -> Option<String> {
        self.get_selected_release().and_then(|r| r.resource_id)
    }

    pub(super) fn get_selected_release(&self) -> Option<Release> {
        let idx = self.state.resource_list_state.selected()?;
        self.state.selected_resources.get(idx).cloned()
    }
}

impl App {
    pub(super) async fn handle_navigation(&mut self, action: Action) -> Option<()> {
        match action {
            Action::GoBack => {
                if self.state.player_picker_popup {
                    self.state.is_resolving_playback = false;
                    self.state.player_picker_popup = false;
                    self.state.player_picker_state.select(None);
                    self.state.settings_player_picker = false;
                    return None;
                }
                if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    let is_dl = self.state.is_download_subtitle_popup;
                    self.state.is_resolving_playback = false;
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.pending_play_link = None;
                    self.state.pending_playback_source = None;
                    self.state.subtitle_list.clear();
                    self.state.subtitle_list_state.select(None);
                    if is_dl {
                        self.state
                            .set_status_default("Download subtitle selection cancelled.");
                    } else {
                        self.state.notify(
                            crate::tui::overlay::NotificationKind::Info,
                            "Playback Cancelled",
                            "Stream launch cancelled.",
                        );
                    }
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
                if self.state.show_provider_popup {
                    self.state.show_provider_popup = false;
                    self.state.provider_list_state.select(None);
                    return None;
                }
                if !self.state.notifications.is_empty() {
                    self.state.notifications.clear();
                    return None;
                }
                if self.state.favorites_focus {
                    self.state.favorites_focus = false;
                    self.state.favorites_landing_state.select(None);
                    return None;
                }
                match self.state.active_screen {
                    Screen::Home => {
                        if self.state.active_browse_preset.is_some()
                            || self.state.active_addon_catalog.is_some()
                            || self.state.is_homepage_mode
                        {
                            self.state.active_browse_preset = None;
                            self.state.active_addon_catalog = None;
                            self.state.is_homepage_mode = false;
                            self.state.clear_search_state();
                            self.state.set_status_default("");
                            return None;
                        }
                        if !self.state.search_results.is_empty() {
                            self.state.input_mode = InputMode::Editing;
                            self.state.favorites_focus = false;
                            self.state.favorites_landing_state.select(None);
                            self.state.last_search_edit = std::time::Instant::now();
                            return None;
                        }
                        let had_search = !self.state.search_query.trim().is_empty()
                            || !self.state.search_results.is_empty();
                        self.state.is_loading = false;
                        self.state.active_search_request =
                            self.state.active_search_request.wrapping_add(1);
                        self.state.active_homepage_request =
                            self.state.active_homepage_request.wrapping_add(1);
                        self.state.active_preview_request =
                            self.state.active_preview_request.wrapping_add(1);
                        self.state.clear_search_state();
                        if had_search {
                            self.state.set_status_default("Search cleared.");
                        }
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
                        self.state.selected_resources.clear();
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
                            .set_status_default("Select a movie/series and press Enter");
                    }
                }
            }

            Action::SelectLanguage(idx) => {
                if let Some(details) = &self.state.selected_details
                    && let Some(dub) = details.dubs.get(idx)
                    && !dub.subject_id.is_empty()
                {
                    let next_id = dub.subject_id.clone();
                    self.state.selected_resources.clear();
                    self.state.resource_list_state.select(None);
                    self.state.language_chosen = true;
                    self.state.language_list_state.select(Some(idx));
                    self.state.is_loading = true;
                    self.state.is_fetching_streams = true;
                    self.state.has_streams_settled = false;
                    self.state.stream_error = None;
                    if self.state.active_subject_id.as_deref() == Some(&next_id) {
                        if !self.state.stream_pool.contains_key(&next_id) {
                            self.state.set_status_default("Loading streams...");
                            self.action_sender
                                .send(Action::InitStreamPool(next_id))
                                .ok();
                        } else {
                            self.trigger_episode_fetch();
                        }
                    } else {
                        self.state.set_status_default("Switching language...");
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
                            } else {
                                self.state.favorites_focus = false;
                                self.state.favorites_landing_state.select(None);
                            }
                            return None;
                        }
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        let up_step = self.result_grid_columns();
                        let next = current.saturating_sub(up_step);
                        if next != current {
                            self.state.search_list_state.select(Some(next));
                            if let Some(res) = self.state.search_results.get(next) {
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
                            let total = self.state.landing_deck_items_count();
                            let current =
                                self.state.favorites_landing_state.selected().unwrap_or(0);
                            if current + 1 < total {
                                self.state.favorites_landing_state.select(Some(current + 1));
                            }
                            return None;
                        }
                        if self.state.search_results.is_empty()
                            && self.state.search_query.trim().is_empty()
                            && self.state.landing_deck_visible()
                        {
                            self.state.favorites_focus = true;
                            self.state.favorites_landing_state.select(Some(0));
                            return None;
                        }
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        let down_step = self.result_grid_columns();
                        let next = (current + down_step).min(self.state.search_results.len() - 1);
                        if next != current && !self.state.search_results.is_empty() {
                            self.state.search_list_state.select(Some(next));
                            if let Some(res) = self.state.search_results.get(next) {
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
                            let current = self.state.resource_list_state.selected().unwrap_or(0);
                            if current + 1 < self.state.selected_resources.len() {
                                self.state.resource_list_state.select(Some(current + 1));
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
                                && current + 1 < details.dubs.len()
                            {
                                self.state.language_list_state.select(Some(current + 1));
                            }
                        }
                    },
                }
            }

            Action::MoveLeft => {
                if self.state.active_screen == Screen::Home {
                    if self.state.favorites_focus {
                        self.state.cycle_home_deck_tab();
                        return None;
                    }
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = 1;
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
                    if self.state.favorites_focus {
                        self.state.cycle_home_deck_tab();
                        return None;
                    }
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = 1;
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
                if self.state.player_picker_popup {
                    let idx = self.state.player_picker_state.selected().unwrap_or(0);
                    if let Some(player) = self.state.available_players.get(idx).copied() {
                        self.state.player_picker_popup = false;
                        self.remember_player_preference(player);
                        self.state.settings_player_picker = false;
                    }
                    return None;
                }
                if self.state.subtitle_popup {
                    self.state.subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_url = self
                        .state
                        .subtitle_list
                        .get(idx)
                        .map(|(_, u)| u.clone())
                        .filter(|s| !s.is_empty());
                    if let Some(mut source) = self.state.pending_playback_source.take() {
                        source.subtitle = sub_url;
                        self.dispatch_playback_or_notify(source);
                    } else if let Some(link) = self.state.pending_play_link.take() {
                        self.action_sender
                            .send(Action::LaunchMpv(link, sub_url))
                            .ok();
                    }
                    return None;
                } else if self.state.is_download_subtitle_popup {
                    self.state.is_download_subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_name = self.state.subtitle_list.get(idx).map(|(n, _)| n.clone());
                    let sub_url = self.state.subtitle_list.get(idx).map(|(_, u)| u.clone());
                    let sub_url_final = sub_url.filter(|s| !s.is_empty());

                    let selected_language = sub_name.filter(|n| n != "None");
                    self.state.last_download_subtitle_language = selected_language.clone();
                    if self.state.download_queue_total > 0 {
                        self.state.season_subtitle_preference = Some(selected_language);
                    }

                    self.action_sender
                        .send(Action::DownloadStream(sub_url_final))
                        .ok();
                    return None;
                }
                if self.state.favorites_focus {
                    if let Some(idx) = self.state.favorites_landing_state.selected() {
                        match self.state.effective_home_deck_tab() {
                            crate::tui::state::HomeDeckTab::ContinueWatching => {
                                self.open_continue_watching(idx);
                            }
                            crate::tui::state::HomeDeckTab::Favorites => {
                                self.open_favorite(idx);
                            }
                        }
                    }
                    return None;
                }
                if self.state.active_screen == Screen::Home {
                    if self.state.last_search_edit.elapsed().as_millis() < 500 {
                        return None;
                    }
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
                        let fallback_details = MediaDetails::from_search_result(
                            &item,
                            self.state.search_preview.as_ref(),
                        );
                        self.state.selected_details = Some(fallback_details);
                        self.state.selected_resources.clear();
                        self.state.is_loading = true;
                        self.state.is_fetching_streams = false;
                        self.state.stream_error = None;
                        self.state.resource_list_state.select(None);
                        self.state.language_list_state.select(Some(0));

                        let is_history = self
                            .state
                            .search_query
                            .trim()
                            .eq_ignore_ascii_case("/history");
                        let se = if is_history && item.season > 0 {
                            item.season
                        } else {
                            1
                        };
                        let ep = if is_history && item.episode > 0 {
                            item.episode
                        } else {
                            1
                        };
                        self.state.selected_season = se;
                        self.state.selected_episode = ep;
                        self.state
                            .season_list_state
                            .select(Some(se.saturating_sub(1)));
                        self.state
                            .episode_list_state
                            .select(Some(ep.saturating_sub(1)));
                        self.state.language_chosen = false;
                        if let Some(cached) = self
                            .state
                            .image_cache
                            .get(&item.id)
                            .or_else(|| self.state.search_posters.get(&item.id))
                        {
                            self.state.poster_image = Some(std::sync::Arc::clone(cached));
                        } else {
                            self.state.poster_image = None;
                        }
                        self.state.available_seasons.clear();
                        self.state
                            .set_status_default(format!("Loading details for {}...", item.title));

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

    pub(super) fn resume_history_playback(&mut self) {
        if self.state.is_loading {
            return;
        }
        if self.state.last_search_edit.elapsed().as_millis() < 500 {
            return;
        }
        let idx_opt = self.state.search_list_state.selected();
        let item_opt = idx_opt.and_then(|idx| self.state.search_results.get(idx).cloned());
        if let Some(item) = item_opt {
            if self.state.is_tv_mode || item.stype == 3 {
                self.action_sender
                    .send(Action::LaunchMpv(item.id.clone(), None))
                    .ok();
                return;
            }
            self.state.active_screen = Screen::Details;
            self.state.active_subject_id = Some(item.id.clone());
            let fallback_details =
                MediaDetails::from_search_result(&item, self.state.search_preview.as_ref());
            self.state.selected_details = Some(fallback_details);
            self.state.selected_resources.clear();
            self.state.is_loading = true;
            self.state.is_fetching_streams = false;
            self.state.stream_error = None;
            self.state.resource_list_state.select(None);
            self.state.language_list_state.select(Some(0));

            let se = if item.season > 0 { item.season } else { 1 };
            let mut ep = if item.episode > 0 { item.episode } else { 1 };
            let provider_key = self.state.active_provider.cache_key();
            if let Some(hist) =
                self.state
                    .history
                    .get_item(provider_key, &item.id, se, ep, Some(&item.title))
            {
                if hist.completed && item.stype == 2 {
                    ep = ep.saturating_add(1);
                }
            }
            self.state.selected_season = se;
            self.state.selected_episode = ep;
            self.state
                .season_list_state
                .select(Some(se.saturating_sub(1)));
            self.state
                .episode_list_state
                .select(Some(ep.saturating_sub(1)));

            self.state.auto_play_on_ready = true;
            self.state.language_chosen = false;
            if let Some(cached) = self
                .state
                .image_cache
                .get(&item.id)
                .or_else(|| self.state.search_posters.get(&item.id))
            {
                self.state.poster_image = Some(std::sync::Arc::clone(cached));
            } else {
                self.state.poster_image = None;
            }
            self.state.available_seasons.clear();
            self.state
                .set_status_default(format!("Resuming playback for {}...", item.title));

            let sender = self.action_sender.clone();
            sender
                .send(Action::FetchDetails(item.id.clone(), false))
                .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::providers::models::ProviderKind;
    use crate::tui::action::Action;
    use crate::tui::app::App;
    use crate::tui::overlay::NotificationKind;

    #[tokio::test]
    async fn test_switch_provider_preserves_active_download() {
        let mut app = App::new();
        app.state.download_progress = Some(45.0);
        app.state.download_status = Some("45MB".to_string());
        app.state.download_title = Some("Test Movie".to_string());
        app.state
            .cancel_download
            .store(false, std::sync::atomic::Ordering::SeqCst);

        app.switch_provider(ProviderKind::FourKHdHub);

        assert_eq!(app.state.active_provider, ProviderKind::FourKHdHub);
        assert_eq!(app.state.download_progress, Some(45.0));
        assert_eq!(app.state.download_status.as_deref(), Some("45MB"));
        assert_eq!(app.state.download_title.as_deref(), Some("Test Movie"));
        assert!(
            !app.state
                .cancel_download
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn test_subtitle_popup_cancellation_feedback() {
        let mut app = App::new();
        app.state.subtitle_popup = true;
        app.state.pending_play_link = Some("https://example.com/video.mp4".to_string());

        app.handle_action(Action::GoBack).await;

        assert!(!app.state.subtitle_popup);
        assert!(app.state.pending_play_link.is_none());
        let notif = app
            .state
            .notifications
            .back()
            .expect("notification emitted");
        assert_eq!(notif.kind, NotificationKind::Info);
        assert_eq!(notif.title, "Playback Cancelled");
    }

    #[tokio::test]
    async fn test_escape_dismisses_active_notifications() {
        let mut app = App::new();
        app.state
            .notify(NotificationKind::Info, "Test Notification", "Some message");
        assert_eq!(app.state.notifications.len(), 1);

        app.handle_action(Action::GoBack).await;
        assert!(app.state.notifications.is_empty());
    }
}
