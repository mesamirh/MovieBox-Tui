use super::App;
use crate::tui::{
    action::Action,
    overlay::NotificationKind,
    state::{BrowsePreset, DetailsPane, InputMode, Screen},
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

impl App {
    pub(super) fn handle_mouse(&mut self, col: u16, row: u16) -> Option<Action> {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let cols = if cols == 0 { 80 } else { cols };
        let rows = if rows == 0 { 24 } else { rows };
        let area = Rect::new(0, 0, cols, rows);

        if self.handle_overlay_mouse(col, row, area) {
            return None;
        }

        match self.state.active_screen {
            Screen::Home => self.handle_home_mouse(col, row, area),
            Screen::Details => self.handle_details_mouse(col, row, area),
        }
    }

    fn handle_overlay_mouse(&mut self, col: u16, row: u16, area: Rect) -> bool {
        if self.state.is_updating {
            return true;
        }

        if !self.state.notifications.is_empty() {
            let rects = crate::tui::overlay::notification_rects(
                area,
                &self.state.notifications,
                self.state.basic_terminal,
                self.state.download_progress.is_some(),
            );
            for (idx, rect) in rects {
                if rect.contains(ratatui::layout::Position::new(col, row)) {
                    self.state.notifications.remove(idx);
                    return true;
                }
            }
        }

        if self.state.download_progress.is_some() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(area);
            let dl_area = chunks[1];
            if dl_area.contains(ratatui::layout::Position::new(col, row)) {
                let cancel_rect = Rect {
                    x: dl_area.right().saturating_sub(14),
                    y: dl_area.y,
                    width: 14.min(dl_area.width),
                    height: 1,
                };
                if cancel_rect.contains(ratatui::layout::Position::new(col, row)) {
                    self.action_sender.send(Action::CancelDownload).ok();
                }
                return true;
            }
        }
        if self.state.player_picker_popup {
            let items = self
                .state
                .available_players
                .iter()
                .map(|k| k.label().to_string())
                .collect::<Vec<_>>();
            let confirm_label = "Select";
            match click_in_picker(
                crate::tui::overlay::picker_layout(area, &items, confirm_label, 10),
                col,
                row,
                &self.state.player_picker_state,
                items.len(),
                area,
            ) {
                Some(Some(clicked_idx)) => {
                    self.state.player_picker_state.select(Some(clicked_idx));
                    self.action_sender.send(Action::Submit).ok();
                }
                Some(None) => {}
                None => {
                    self.state.player_picker_popup = false;
                    self.state.settings_player_picker = false;
                }
            }
            return true;
        }
        if self.state.show_sources_popup {
            let items = crate::providers::models::ProviderKind::ENABLED
                .iter()
                .map(|p| format!(" [✓] {} ", p.label()))
                .collect::<Vec<_>>();
            match click_in_picker(
                crate::tui::overlay::picker_layout(area, &items, "Toggle", 20),
                col,
                row,
                &self.state.sources_list_state,
                items.len(),
                area,
            ) {
                Some(Some(clicked_idx)) => {
                    self.state.sources_list_state.select(Some(clicked_idx));
                    if let Some(&provider) =
                        crate::providers::models::ProviderKind::ENABLED.get(clicked_idx)
                    {
                        self.action_sender
                            .send(Action::ToggleProvider(provider))
                            .ok();
                    }
                }
                Some(None) => {}
                None => {
                    self.state.show_sources_popup = false;
                }
            }
            return true;
        }

        if self.state.show_theme_popup {
            let theme_names = crate::tui::theme::AVAILABLE_THEMES;
            let longest_name = theme_names
                .iter()
                .map(|name| crate::tui::text::width(name))
                .max()
                .unwrap_or(10);
            let items: Vec<String> = theme_names
                .iter()
                .map(|name| {
                    if self.state.basic_terminal {
                        format!("  {name:<pad$}   * * *  ", pad = longest_name)
                    } else {
                        format!("  {name:<pad$}   ■ ■ ■  ", pad = longest_name)
                    }
                })
                .collect();
            match click_in_picker(
                crate::tui::overlay::picker_layout(area, &items, "Apply", 16),
                col,
                row,
                &self.state.theme_list_state,
                items.len(),
                area,
            ) {
                Some(Some(clicked_idx)) => {
                    self.state.theme_list_state.select(Some(clicked_idx));
                    if let Some(&theme_name) = theme_names.get(clicked_idx) {
                        self.action_sender
                            .send(Action::SelectTheme(theme_name.to_string()))
                            .ok();
                        self.state.show_theme_popup = false;
                        self.state.theme_list_state.select(None);
                        self.state
                            .set_status_default(format!("{theme_name} theme applied."));
                    }
                }
                Some(None) => {}
                None => {
                    self.state.show_theme_popup = false;
                    self.state.theme_list_state.select(None);
                }
            }
            return true;
        }
        if self.state.show_settings_popup {
            let popup =
                crate::tui::overlay::settings_modal_layout(area, self.state.settings_category);
            if !popup.contains(ratatui::layout::Position::new(col, row)) {
                self.state.show_settings_popup = false;
                self.state.settings_download_dir_input = None;
                self.persist_config();
                return true;
            }

            if let Some(cat) = crate::tui::widgets::settings::settings_category_tab_at(
                popup,
                col,
                row,
                self.state.basic_terminal,
                self.state.settings_category,
            ) {
                self.state.settings_select_category(cat);
                return true;
            }

            if let Some(clicked_row) = crate::tui::widgets::settings::settings_row_at(
                popup,
                self.state.settings_category,
                col,
                row,
            ) {
                self.state.settings_selected_row = clicked_row;
                self.action_sender.send(Action::SettingsActivateRow).ok();
                return true;
            }

            return true;
        }

        if self.state.show_browse_popup {
            let is_addon =
                self.state.active_provider == crate::providers::models::ProviderKind::Addons;
            let raw_labels: Vec<String> = if is_addon {
                crate::providers::addons::models::curated_catalog_presets(
                    &self.state.installed_addons,
                )
                .into_iter()
                .map(|target| target.label)
                .collect()
            } else {
                BrowsePreset::ALL
                    .iter()
                    .map(|preset| preset.label().to_string())
                    .collect()
            };
            let browse_items: Vec<String> = raw_labels
                .iter()
                .map(|label| {
                    let badge_str = if label.to_ascii_lowercase().contains("movie")
                        || label.to_ascii_lowercase().contains("top rated (all-time)")
                        || label.to_ascii_lowercase().contains("top rated (recent")
                    {
                        "[MOVIES]   "
                    } else if label.to_ascii_lowercase().contains("series")
                        || label.to_ascii_lowercase().contains("airing")
                        || label.to_ascii_lowercase().contains("show")
                        || label.to_ascii_lowercase().contains("tv")
                    {
                        "[SERIES]   "
                    } else {
                        "[DISCOVER] "
                    };
                    format!("{badge_str}{label}")
                })
                .collect();
            match click_in_picker(
                crate::tui::overlay::picker_layout(area, &browse_items, "Open", 36),
                col,
                row,
                &self.state.browse_list_state,
                browse_items.len(),
                area,
            ) {
                Some(Some(clicked_idx)) => {
                    self.state.browse_list_state.select(Some(clicked_idx));
                    self.state.show_browse_popup = false;
                    self.state.browse_list_state.select(None);
                    if is_addon {
                        let targets = crate::providers::addons::models::curated_catalog_presets(
                            &self.state.installed_addons,
                        );
                        if let Some(target) = targets.get(clicked_idx).cloned() {
                            self.action_sender
                                .send(Action::SelectAddonCatalog(target))
                                .ok();
                        }
                    } else if let Some(preset) = BrowsePreset::ALL.get(clicked_idx).copied() {
                        self.action_sender.send(Action::SelectBrowse(preset)).ok();
                    }
                }
                Some(None) => {}
                None => {
                    self.state.show_browse_popup = false;
                    self.state.browse_list_state.select(None);
                }
            }
            return true;
        }

        if self.state.show_help {
            self.state.show_help = false;
            return true;
        }

        if let Some((ver, notes)) = &self.state.update_available {
            let layout = crate::tui::overlay::update_modal_layout(area, notes);
            if layout
                .popup_area
                .contains(ratatui::layout::Position::new(col, row))
            {
                let is_homebrew = std::env::current_exe()
                    .map(|p| crate::updater::apply::is_homebrew_managed(&p))
                    .unwrap_or(false);

                if row == layout.button_row_y {
                    if col < layout.update_btn_end_x {
                        if is_homebrew {
                            self.state
                                .set_status_short("Run: brew upgrade moviebox-tui");
                            self.state.notify(
                                NotificationKind::Info,
                                "Homebrew Upgrade",
                                "Run: brew upgrade moviebox-tui",
                            );
                        } else {
                            self.action_sender.send(Action::StartSelfUpdate).ok();
                        }
                    } else if col < layout.open_btn_end_x {
                        let url =
                            format!("https://github.com/mesamirh/MovieBox-Tui/releases/tag/v{ver}");
                        let _ = open::that(&url);
                    }
                    self.state.update_available = None;
                }
            } else {
                self.state.update_available = None;
            }
            return true;
        }

        if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
            let items = self
                .state
                .subtitle_list
                .iter()
                .map(|(name, _)| crate::tui::text::format_subtitle_label(name))
                .collect::<Vec<_>>();
            let confirm_label = if self.state.is_download_subtitle_popup {
                "Download"
            } else {
                "Use"
            };
            match click_in_picker(
                crate::tui::overlay::picker_layout(area, &items, confirm_label, 20),
                col,
                row,
                &self.state.subtitle_list_state,
                items.len(),
                area,
            ) {
                Some(Some(clicked_idx)) => {
                    self.state.subtitle_list_state.select(Some(clicked_idx));
                    self.action_sender.send(Action::Submit).ok();
                }
                Some(None) => {}
                None => {
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
                }
            }
            return true;
        }

        if self.state.show_season_download_confirm {
            let summary = crate::tui::screens::details::season_confirm_summary(&self.state);
            let longest = summary
                .iter()
                .map(|line| crate::tui::text::width(line))
                .max()
                .unwrap_or(36);
            let popup = crate::tui::overlay::download_confirm_layout(area, summary.len(), longest);
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                let action_y =
                    crate::tui::overlay::download_confirm_action_row(popup, summary.len());
                if row == action_y {
                    let mid_x = popup.x + popup.width / 2;
                    if col < mid_x {
                        self.action_sender.send(Action::ConfirmDownloadSeason).ok();
                    } else {
                        self.state.show_season_download_confirm = false;
                    }
                }
            } else {
                self.state.show_season_download_confirm = false;
            }
            return true;
        }

        if self.state.show_episode_download_confirm {
            let summary = crate::tui::screens::details::episode_confirm_summary(&self.state);
            let longest = summary
                .iter()
                .map(|line| crate::tui::text::width(line))
                .max()
                .unwrap_or(36);
            let popup = crate::tui::overlay::download_confirm_layout(area, summary.len(), longest);
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                let action_y =
                    crate::tui::overlay::download_confirm_action_row(popup, summary.len());
                if row == action_y {
                    let mid_x = popup.x + popup.width / 2;
                    if col < mid_x {
                        self.action_sender.send(Action::ConfirmDownloadEpisode).ok();
                    } else {
                        self.state.show_episode_download_confirm = false;
                    }
                }
            } else {
                self.state.show_episode_download_confirm = false;
            }
            return true;
        }

        if self.state.tv_config_popup {
            let rows = self.state.tv_manager_rows();
            let total_rows = rows.len();
            let longest_source_width = self
                .state
                .tv_playlists
                .iter()
                .map(|source| crate::tui::text::width(source))
                .max()
                .unwrap_or(28);
            let popup = crate::tui::overlay::tv_config_layout(
                area,
                longest_source_width,
                total_rows,
                self.state.tv_input_active,
            );
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                if !self.state.tv_input_active {
                    let item_start_y = popup.y + 1;
                    if row >= item_start_y && (row - item_start_y) < total_rows as u16 {
                        let clicked_idx = (row - item_start_y) as usize;
                        self.state.tv_manager_selected = clicked_idx;
                        if let Some(r) = rows.get(clicked_idx) {
                            match r {
                                crate::tui::state::TvManagerRow::AddUrl => {
                                    self.action_sender.send(Action::TvInputToggle(false)).ok();
                                }
                                crate::tui::state::TvManagerRow::AddFile => {
                                    self.action_sender.send(Action::TvInputToggle(true)).ok();
                                }
                                crate::tui::state::TvManagerRow::Reload => {
                                    self.action_sender.send(Action::TvReloadPlaylists).ok();
                                }
                                crate::tui::state::TvManagerRow::Done => {
                                    self.state.tv_config_popup = false;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                self.state.tv_config_popup = false;
                self.state.tv_input_active = false;
            }
            return true;
        }

        if self.state.addon_manager_popup {
            let addons_count = self.state.installed_addons.len();
            let popup = crate::tui::overlay::addon_manager_layout(
                area,
                addons_count,
                self.state.addon_input_active,
            );
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                if !self.state.addon_input_active {
                    let list_start_y = popup.y + 1;
                    let button_y = list_start_y + addons_count as u16 + 1;
                    if row > list_start_y && row < button_y {
                        let clicked_addon_idx = (row - list_start_y - 1) as usize;
                        if clicked_addon_idx < addons_count {
                            self.state.addon_manager_selected = clicked_addon_idx + 1;
                            self.addon_manager_activate();
                        }
                    } else if row == button_y {
                        self.state.addon_manager_selected = addons_count + 1;
                        self.addon_manager_activate();
                    }
                }
            } else {
                self.state.addon_manager_popup = false;
                self.state.addon_input_active = false;
            }
            return true;
        }

        false
    }

    fn handle_home_mouse(&mut self, col: u16, row: u16, area: Rect) -> Option<Action> {
        if self.state.is_loading && self.state.search_results.is_empty() {
            return None;
        }

        let is_landing = self.state.search_results.is_empty()
            && (self.state.search_query.trim().is_empty()
                || self.state.input_mode == InputMode::Editing);

        let landing_layout = if is_landing {
            Some(crate::tui::screens::home::landing_split(
                area,
                self.state.is_tv_mode,
                self.state.basic_terminal,
                self.state.landing_deck_visible(),
            ))
        } else {
            None
        };

        if let Some((_tier, ref rows)) = landing_layout {
            let card_width = crate::tui::screens::home::search_deck_width(area, &self.state, true);
            let card_x = area.x + area.width.saturating_sub(card_width) / 2;
            let search_y = rows.rects[rows.search].y;
            let search_card_area = Rect {
                x: card_x,
                y: search_y,
                width: card_width,
                height: rows.rects[rows.search].height,
            };
            if self.state.show_provider_popup {
                let available = self.state.available_providers();
                let (_container_area, inner_area) =
                    crate::tui::screens::home::provider_popup_bounds(
                        area,
                        search_card_area,
                        available.len(),
                    );

                if col >= inner_area.left()
                    && col < inner_area.right()
                    && row >= inner_area.top()
                    && row < inner_area.bottom()
                {
                    let clicked_idx = (row - inner_area.top()) as usize;
                    if let Some(&provider) = available.get(clicked_idx) {
                        self.switch_provider(provider);
                    }
                }

                self.state.show_provider_popup = false;
                self.state.provider_list_state.select(None);
                return None;
            }

            if row == rows.rects[rows.mode_row].y {
                self.handle_home_bottom_bar_click(col, area.width);
                return None;
            }

            if self.state.input_mode == InputMode::Editing
                && !self.state.search_suggestions.is_empty()
            {
                let visible_count = self.state.search_suggestions.len().min(6);
                let selected_index = self.state.suggest_index.unwrap_or(0);
                let suggestion_offset = selected_index
                    .saturating_add(1)
                    .saturating_sub(visible_count)
                    .min(
                        self.state
                            .search_suggestions
                            .len()
                            .saturating_sub(visible_count),
                    );
                let visible_slice_len = self
                    .state
                    .search_suggestions
                    .len()
                    .saturating_sub(suggestion_offset)
                    .min(visible_count);

                let (container_area, inner_area) =
                    crate::tui::screens::home::search_suggestions_bounds(
                        area,
                        search_card_area,
                        visible_slice_len,
                    );

                if col >= inner_area.left()
                    && col < inner_area.right()
                    && row >= inner_area.top()
                    && row < inner_area.bottom()
                {
                    let clicked_idx = suggestion_offset + (row - inner_area.top()) as usize;
                    if let Some(query) = self.state.search_suggestions.get(clicked_idx).cloned() {
                        self.action_sender
                            .send(Action::SelectSuggestion { query })
                            .ok();
                    }
                    return None;
                }

                if col >= container_area.left()
                    && col < container_area.right()
                    && row >= container_area.top()
                    && row < container_area.bottom()
                {
                    return None;
                }
            }

            if col >= search_card_area.left()
                && col < search_card_area.right()
                && row >= search_card_area.top()
                && row < search_card_area.bottom()
            {
                let is_query_empty = self.state.search_query.is_empty();
                if is_query_empty {
                    let pill_rect = crate::tui::screens::home::search_bar_provider_pill_rect(
                        search_card_area,
                        &self.state,
                    );
                    if col >= pill_rect.left()
                        && col < pill_rect.right()
                        && row >= pill_rect.top()
                        && row < pill_rect.bottom()
                    {
                        if self.state.mode() == crate::tui::state::AppMode::Streaming {
                            let available = self.state.available_providers();
                            let current_idx = available
                                .iter()
                                .position(|p| *p == self.state.active_provider)
                                .unwrap_or(0);
                            self.state.show_provider_popup = true;
                            self.state.provider_list_state.select(Some(current_idx));
                            self.state.input_mode = InputMode::Normal;
                        } else if self.state.mode() == crate::tui::state::AppMode::Tv {
                            self.action_sender.send(Action::ToggleTvMode).ok();
                        }
                        return None;
                    }
                }
                self.state.input_mode = InputMode::Editing;
                self.state.favorites_focus = false;
                self.state.favorites_landing_state.select(None);
                return None;
            }

            if self.state.landing_deck_visible() {
                let deck_y = rows.rects[rows.favorites].y;
                let item_count = self.state.landing_deck_items_count() as u16;
                let total_count = self.state.landing_deck_total_items_count();
                let overflow = total_count.saturating_sub(item_count as usize);
                let overflow_row = u16::from(overflow > 0);
                let deck_height =
                    (item_count + overflow_row + 2).min(rows.rects[rows.favorites].height);

                let deck_card_area = Rect {
                    x: card_x,
                    y: deck_y,
                    width: card_width,
                    height: deck_height,
                };

                if col >= deck_card_area.left()
                    && col < deck_card_area.right()
                    && row >= deck_card_area.top()
                    && row < deck_card_area.bottom()
                {
                    let rel_row = row - deck_card_area.top();
                    if rel_row == 0 {
                        self.state.cycle_home_deck_tab();
                        self.state.favorites_focus = true;
                        self.state.input_mode = InputMode::Normal;
                    } else if rel_row >= 1 && rel_row <= item_count {
                        let idx = (rel_row - 1) as usize;
                        let prev_selected = if self.state.favorites_focus {
                            self.state.favorites_landing_state.selected()
                        } else {
                            None
                        };
                        self.state.favorites_focus = true;
                        self.state.input_mode = InputMode::Normal;
                        self.state.favorites_landing_state.select(Some(idx));
                        if prev_selected == Some(idx) {
                            match self.state.effective_home_deck_tab() {
                                crate::tui::state::HomeDeckTab::ContinueWatching => {
                                    self.action_sender
                                        .send(Action::OpenContinueWatching(idx))
                                        .ok();
                                }
                                crate::tui::state::HomeDeckTab::Favorites => {
                                    self.action_sender.send(Action::OpenFavorite(idx)).ok();
                                }
                            }
                        }
                    } else if overflow > 0 && rel_row == item_count + 1 {
                        match self.state.effective_home_deck_tab() {
                            crate::tui::state::HomeDeckTab::ContinueWatching => {
                                self.action_sender
                                    .send(Action::Search {
                                        query: "/history".to_string(),
                                        force_refresh: false,
                                    })
                                    .ok();
                            }
                            crate::tui::state::HomeDeckTab::Favorites => {
                                self.action_sender.send(Action::ShowFavorites).ok();
                            }
                        }
                    } else {
                        self.state.favorites_focus = true;
                        self.state.input_mode = InputMode::Normal;
                    }
                    return None;
                }
            } else if !self.state.is_tv_mode && area.height >= 26 {
                let suggestions_open = self.state.input_mode == InputMode::Editing
                    && !self.state.search_suggestions.is_empty();
                if !suggestions_open {
                    let discover_y = rows.rects[rows.favorites].y;
                    let discover_height = 6;
                    let discover_card_area = Rect {
                        x: card_x,
                        y: discover_y,
                        width: card_width,
                        height: discover_height,
                    };

                    if col >= discover_card_area.left()
                        && col < discover_card_area.right()
                        && row >= discover_card_area.top()
                        && row < discover_card_area.bottom()
                    {
                        if self.state.active_provider
                            == crate::providers::models::ProviderKind::Addons
                        {
                            self.action_sender.send(Action::ShowBrowseMenu).ok();
                        } else {
                            let rel_row = row - discover_card_area.top();
                            match rel_row {
                                1 => self
                                    .action_sender
                                    .send(Action::SelectBrowse(BrowsePreset::Trending))
                                    .ok(),
                                2 => self
                                    .action_sender
                                    .send(Action::SelectBrowse(BrowsePreset::TopRatedAllTime))
                                    .ok(),
                                3 => self
                                    .action_sender
                                    .send(Action::SelectBrowse(BrowsePreset::TopRatedRecent))
                                    .ok(),
                                4 => self
                                    .action_sender
                                    .send(Action::SelectBrowse(BrowsePreset::MostWatched))
                                    .ok(),
                                _ => self.action_sender.send(Action::ShowBrowseMenu).ok(),
                            };
                        }
                        return None;
                    }
                }
            }

            return None;
        }

        let (search_bar_area, results_chunk) =
            crate::tui::screens::home::search_results_layout(area);

        if self.state.input_mode == InputMode::Editing && !self.state.search_suggestions.is_empty()
        {
            let visible_count = self.state.search_suggestions.len().min(6);
            let selected_index = self.state.suggest_index.unwrap_or(0);
            let suggestion_offset = selected_index
                .saturating_add(1)
                .saturating_sub(visible_count)
                .min(
                    self.state
                        .search_suggestions
                        .len()
                        .saturating_sub(visible_count),
                );

            let visible_slice_len = self
                .state
                .search_suggestions
                .len()
                .saturating_sub(suggestion_offset)
                .min(visible_count);

            let (container_area, inner_area) = crate::tui::screens::home::search_suggestions_bounds(
                area,
                search_bar_area,
                visible_slice_len,
            );

            if col >= inner_area.left()
                && col < inner_area.right()
                && row >= inner_area.top()
                && row < inner_area.bottom()
            {
                let clicked_idx = suggestion_offset + (row - inner_area.top()) as usize;
                if let Some(query) = self.state.search_suggestions.get(clicked_idx).cloned() {
                    self.action_sender
                        .send(Action::SelectSuggestion { query })
                        .ok();
                }
                return None;
            }

            if col >= container_area.left()
                && col < container_area.right()
                && row >= container_area.top()
                && row < container_area.bottom()
            {
                return None;
            }
        }

        if row >= search_bar_area.y && row < search_bar_area.y + search_bar_area.height {
            self.state.input_mode = InputMode::Editing;
            self.state.favorites_focus = false;
            self.state.favorites_landing_state.select(None);
            return None;
        }

        if row >= results_chunk.y && row < area.height {
            if self.state.search_results.is_empty() {
                let next_label = self.state.next_provider().label();
                let ctrl_p = crate::tui::text::CTRL_P_STR;
                let (btn1, btn2) = crate::tui::screens::home::no_results_button_hitboxes(
                    results_chunk,
                    next_label,
                    ctrl_p,
                    self.state.is_tv_mode,
                    self.state.active_provider == crate::providers::models::ProviderKind::Addons,
                );
                let pos = ratatui::layout::Position::new(col, row);
                if btn1.contains(pos) {
                    if self.state.is_tv_mode {
                        self.action_sender.send(Action::TvReloadPlaylists).ok();
                    } else if self.state.active_provider
                        == crate::providers::models::ProviderKind::Addons
                    {
                        self.action_sender.send(Action::ShowAddonManager).ok();
                    } else {
                        self.cycle_provider();
                    }
                    return None;
                }
                if btn2.contains(pos) {
                    self.state.clear_search_state();
                    self.state.input_mode = InputMode::Normal;
                    self.state.set_status_default("");
                    return None;
                }
                return None;
            }
            if col < results_chunk.x || col >= results_chunk.right() {
                return None;
            }
            let metrics = self
                .state
                .result_metrics(results_chunk.height.saturating_sub(1), results_chunk.width);
            let row_height = metrics.row_height;
            let clicked_relative_row = row.saturating_sub(results_chunk.y);
            let visual_row = (clicked_relative_row / row_height) as usize;
            let col_step = (metrics.col_width + 1).max(1);
            let clicked_column = (((col.saturating_sub(results_chunk.x)) / col_step) as usize)
                .min(metrics.columns.saturating_sub(1) as usize);
            let page_start = self.state.result_scroll;

            let target_idx = page_start + visual_row * metrics.columns as usize + clicked_column;
            if target_idx < self.state.search_results.len() {
                let prev_selected = self.state.search_list_state.selected();
                self.state.search_list_state.select(Some(target_idx));

                if prev_selected == Some(target_idx) {
                    self.action_sender.send(Action::Submit).ok();
                } else if let Some(res) = self.state.search_results.get(target_idx) {
                    self.action_sender
                        .send(Action::FetchPreview(res.id.clone()))
                        .ok();
                    self.prefetch_visible_posters();
                }
            }
            return None;
        }

        None
    }

    fn handle_home_bottom_bar_click(&mut self, col: u16, width: u16) {
        let compact = width < 76;
        let ultra_compact = width < 58;
        let ctrl_s = if ultra_compact || compact {
            "S"
        } else {
            crate::tui::text::CTRL_S_STR
        };
        let ctrl_t = if ultra_compact || compact {
            "T"
        } else {
            crate::tui::text::CTRL_T_STR
        };

        enum BottomBtn {
            Stream,
            Tv,
        }

        let current_mode = self.state.mode();
        let mut buttons: Vec<(BottomBtn, u16)> = Vec::new();

        if self.state.streaming_enabled && current_mode != crate::tui::state::AppMode::Streaming {
            let len = (3 + ctrl_s.len() + 6) as u16;
            buttons.push((BottomBtn::Stream, len));
        }
        if self.state.tv_enabled && current_mode != crate::tui::state::AppMode::Tv {
            let len = (3 + ctrl_t.len() + 2) as u16;
            buttons.push((BottomBtn::Tv, len));
        }

        let mode_count = buttons.len();
        let sep_len = if compact { 3 } else { 5 };
        let modes_total_w: u16 = if mode_count > 0 {
            buttons.iter().map(|(_, w)| *w).sum::<u16>()
                + (mode_count.saturating_sub(1) as u16) * sep_len
        } else {
            0
        };

        let util_gap = if modes_total_w > 0 {
            if compact { 4 } else { 7 }
        } else {
            0
        };
        let help_w = if ultra_compact { 3 } else { 8 };
        let quit_w = if ultra_compact { 3 } else { 8 };
        let util_sep = 2;

        let total_w = modes_total_w + util_gap + help_w + util_sep + quit_w;
        let start_x = width.saturating_sub(total_w) / 2;

        let mut curr_x = start_x;
        for (btn, w) in buttons {
            if col >= curr_x && col < curr_x + w {
                match btn {
                    BottomBtn::Stream => {
                        if self.state.mode() == crate::tui::state::AppMode::Streaming {
                            self.cycle_provider();
                        } else {
                            self.action_sender.send(Action::SwitchToStreamingMode).ok();
                        }
                    }
                    BottomBtn::Tv => {
                        if self.state.mode() != crate::tui::state::AppMode::Tv {
                            self.action_sender.send(Action::ToggleTvMode).ok();
                        }
                    }
                }
                return;
            }
            curr_x += w + sep_len;
        }

        let help_start = start_x + modes_total_w + util_gap;
        if col >= help_start && col < help_start + help_w {
            self.action_sender.send(Action::ToggleHelp).ok();
            return;
        }

        let quit_start = help_start + help_w + util_sep;
        if col >= quit_start && col < quit_start + quit_w {
            self.action_sender.send(Action::Quit).ok();
        }
    }

    fn handle_details_mouse(&mut self, col: u16, row: u16, area: Rect) -> Option<Action> {
        let details = self.state.selected_details.as_ref()?.clone();

        let has_languages = details.has_languages();
        let is_series = details.is_series() && !self.state.available_seasons.is_empty();

        let mut available_panes = Vec::new();
        if has_languages {
            available_panes.push(DetailsPane::Languages);
        }
        if is_series {
            available_panes.push(DetailsPane::Seasons);
            available_panes.push(DetailsPane::Episodes);
        }

        let layout = crate::tui::screens::details::details_screen_layout(
            area,
            self.state.selected_details.as_ref(),
        );
        let _tier = layout.tier;
        let workflow_area = layout.workflow_area;
        let bottom_area = layout.bottom_area;
        let footer_area = layout.footer_area;
        if row >= footer_area.y && row < footer_area.bottom() {
            self.handle_details_footer_click(col, row - footer_area.y, area.width);
            return None;
        }

        if workflow_area.height > 0 && row == workflow_area.y {
            let count = available_panes.len() + 1;
            let section_w = area.width / count as u16;
            let pane_idx = (col / section_w.max(1)) as usize;
            if pane_idx < available_panes.len() {
                self.state.details_pane = available_panes[pane_idx];
            } else {
                self.state.details_pane = DetailsPane::Streams;
            }
            return None;
        }

        let visible_selector_panes = crate::tui::screens::details::visible_selector_panes(
            &available_panes,
            self.state.details_pane,
            area.width,
        );

        let selector_height = if visible_selector_panes.is_empty() {
            0
        } else {
            let episode_count = self
                .state
                .available_episode_numbers
                .get(self.state.season_list_state.selected().unwrap_or(0))
                .map_or(0, Vec::len);
            let language_count = details.dubs.len();
            language_count
                .max(self.state.available_seasons.len())
                .max(episode_count)
                .min((bottom_area.height / 3).clamp(4, 10) as usize) as u16
                + 2
        };

        let lower_chunks =
            Layout::vertical([Constraint::Length(selector_height), Constraint::Min(3)])
                .split(bottom_area);

        let selector_area = lower_chunks[0];
        let streams_area = lower_chunks[1];

        if !visible_selector_panes.is_empty()
            && selector_area.contains(ratatui::layout::Position::new(col, row))
        {
            let selector_constraints = crate::tui::screens::details::selector_pane_constraints(
                &visible_selector_panes,
                selector_area.width,
            );
            let selector_chunks = Layout::horizontal(selector_constraints).split(selector_area);
            for (pane, pane_rect) in visible_selector_panes
                .into_iter()
                .zip(selector_chunks.iter())
            {
                if pane_rect.contains(ratatui::layout::Position::new(col, row)) {
                    let clicked_row = row.saturating_sub(pane_rect.y + 1) as usize;
                    match pane {
                        DetailsPane::Languages => {
                            self.state.details_pane = DetailsPane::Languages;
                            if clicked_row < details.dubs.len() {
                                self.action_sender
                                    .send(Action::SelectLanguage(clicked_row))
                                    .ok();
                            }
                        }
                        DetailsPane::Seasons => {
                            self.state.details_pane = DetailsPane::Seasons;
                            if clicked_row < self.state.available_seasons.len() {
                                self.state.season_list_state.select(Some(clicked_row));
                                self.state.selected_season = self
                                    .state
                                    .available_seasons
                                    .get(clicked_row)
                                    .map(|s| s.number)
                                    .unwrap_or(1);
                                self.state.episode_list_state.select(Some(0));
                                self.trigger_episode_fetch();
                            }
                        }
                        DetailsPane::Episodes => {
                            self.state.details_pane = DetailsPane::Episodes;
                            let season_idx = self.state.season_list_state.selected().unwrap_or(0);
                            if let Some(ep_numbers) =
                                self.state.available_episode_numbers.get(season_idx)
                            {
                                if clicked_row < ep_numbers.len() {
                                    self.state.episode_list_state.select(Some(clicked_row));
                                    self.state.selected_episode = ep_numbers[clicked_row];
                                    self.trigger_episode_fetch();
                                }
                            }
                        }
                        DetailsPane::Streams => {}
                    }
                    return None;
                }
            }
        }

        if streams_area.contains(ratatui::layout::Position::new(col, row)) {
            self.state.details_pane = DetailsPane::Streams;
            let streams_count = self.state.selected_resources.len();

            if streams_count > 0 {
                let list_start_y = if streams_area.height >= 4 {
                    streams_area.y.saturating_add(2)
                } else {
                    streams_area.y.saturating_add(1)
                };
                let list_end_y = streams_area.bottom().saturating_sub(1);
                if row >= list_start_y && row < list_end_y {
                    let relative_row = (row - list_start_y) as usize;
                    let clicked_stream_idx = self
                        .state
                        .resource_list_state
                        .offset()
                        .saturating_add(relative_row);

                    if clicked_stream_idx < streams_count {
                        self.state
                            .resource_list_state
                            .select(Some(clicked_stream_idx));
                        if self.state.is_playing {
                            self.state.notify(
                                NotificationKind::Warning,
                                "Playback already active",
                                "Stop the current player before starting another.",
                            );
                        } else if !self.state.is_resolving_playback
                            && self.state.last_playback_launch.elapsed().as_millis() >= 500
                        {
                            self.action_sender.send(Action::PlayStream).ok();
                        }
                    }
                }
            }
            return None;
        }

        None
    }

    fn handle_details_footer_click(&mut self, col: u16, line_idx: u16, width: u16) {
        let is_streams = self.state.details_pane == DetailsPane::Streams;
        let is_seasons = self.state.details_pane == DetailsPane::Seasons;
        let is_episodes = self.state.details_pane == DetailsPane::Episodes;
        let is_languages = self.state.details_pane == DetailsPane::Languages;
        let compact = width < crate::tui::screens::details::DETAILS_FOOTER_SPLIT_THRESHOLD;

        let is_favorited = self.state.is_selected_details_favorited();
        let fav_label_len = if is_favorited { 10 } else { 8 };

        enum FooterAction {
            PlaySelect,
            Download,
            Favorite,
            StreamsTab,
            Back,
        }

        let mut primary: Vec<(FooterAction, u16)> = Vec::new();
        let mut secondary: Vec<(FooterAction, u16)> = Vec::new();

        if is_streams {
            primary.push((FooterAction::PlaySelect, 7 + 1 + 4));
            let d_label_len = if compact { 4 } else { 8 };
            primary.push((FooterAction::Download, 3 + 1 + d_label_len));
            secondary.push((FooterAction::Favorite, 3 + 1 + fav_label_len));
            secondary.push((FooterAction::Back, 5 + 1 + 4));
        } else if is_languages {
            primary.push((FooterAction::PlaySelect, 7 + 1 + 6));
            primary.push((FooterAction::Favorite, 3 + 1 + fav_label_len));
            secondary.push((FooterAction::StreamsTab, 5 + 1 + 7));
            secondary.push((FooterAction::Back, 5 + 1 + 4));
        } else if is_seasons {
            primary.push((FooterAction::PlaySelect, 7 + 1 + 6));
            let d_label_len = if compact { 8 } else { 15 };
            primary.push((FooterAction::Download, 3 + 1 + d_label_len));
            primary.push((FooterAction::Favorite, 3 + 1 + fav_label_len));
            secondary.push((FooterAction::StreamsTab, 5 + 1 + 7));
            secondary.push((FooterAction::Back, 5 + 1 + 4));
        } else if is_episodes {
            primary.push((FooterAction::PlaySelect, 7 + 1 + 6));
            let d_label_len = if compact { 8 } else { 16 };
            primary.push((FooterAction::Download, 3 + 1 + d_label_len));
            primary.push((FooterAction::Favorite, 3 + 1 + fav_label_len));
            secondary.push((FooterAction::StreamsTab, 5 + 1 + 7));
            secondary.push((FooterAction::Back, 5 + 1 + 4));
        } else {
            primary.push((FooterAction::PlaySelect, 7 + 1 + 6));
            primary.push((FooterAction::Favorite, 3 + 1 + fav_label_len));
            secondary.push((FooterAction::StreamsTab, 5 + 1 + 7));
            secondary.push((FooterAction::Back, 5 + 1 + 4));
        }
        let active_buttons =
            if width >= crate::tui::screens::details::DETAILS_FOOTER_SPLIT_THRESHOLD {
                if line_idx > 0 {
                    return;
                }
                let mut combined = primary;
                combined.extend(secondary);
                combined
            } else if line_idx == 0 {
                primary
            } else {
                secondary
            };

        let sep = 3_u16;
        let total_w: u16 = active_buttons.iter().map(|(_, w)| *w).sum::<u16>()
            + (active_buttons.len().saturating_sub(1) as u16) * sep;
        let mut curr_x = width.saturating_sub(total_w) / 2;

        for (action, w) in active_buttons {
            let start = curr_x;
            let end = start + w;
            if col >= start && col < end + sep {
                match action {
                    FooterAction::PlaySelect => {
                        if is_streams {
                            self.action_sender.send(Action::PlayStream).ok();
                        } else {
                            self.action_sender.send(Action::Submit).ok();
                        }
                    }
                    FooterAction::Download => {
                        if is_seasons {
                            self.action_sender.send(Action::PromptDownloadSeason).ok();
                        } else {
                            self.action_sender.send(Action::PromptDownloadEpisode).ok();
                        }
                    }
                    FooterAction::Favorite => {
                        self.action_sender.send(Action::ToggleFavorite).ok();
                    }
                    FooterAction::StreamsTab => {
                        self.action_sender.send(Action::TabPane).ok();
                    }
                    FooterAction::Back => {
                        self.action_sender.send(Action::GoBack).ok();
                    }
                }
                return;
            }
            curr_x += w + sep;
        }
    }
}

fn click_in_picker(
    popup: Rect,
    col: u16,
    row: u16,
    state: &ratatui::widgets::ListState,
    total_items: usize,
    area: Rect,
) -> Option<Option<usize>> {
    if !popup.contains(ratatui::layout::Position::new(col, row)) {
        return None;
    }
    let visible_rows = total_items.clamp(1, crate::tui::overlay::max_picker_rows(area));
    let offset = state.offset();
    let item_y = popup.y.saturating_add(1);
    if row >= item_y && (row - item_y) < visible_rows as u16 {
        Some(Some(offset + (row - item_y) as usize))
    } else {
        Some(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AudioTrackOption, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release,
    };
    use crate::tui::action::Action;
    use crate::tui::state::DetailsPane;
    use ratatui::layout::Rect;

    #[test]
    fn test_details_streams_panel_click_empty_space_does_not_open_stream() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Details;
        app.state.details_pane = DetailsPane::Languages;
        app.state.selected_details = Some(MediaDetails {
            id: ProviderMediaId {
                provider: ProviderKind::MovieBox,
                value: "test_movie".to_string(),
            },
            title: "Test Movie".to_string(),
            media_type: MediaType::Movie,
            year: Some("2025".to_string()),
            description: Some("Short description".to_string()),
            tagline: None,
            imdb_rating: Some("4.6".to_string()),
            director: None,
            stars: None,
            prints: None,
            audios: None,
            poster_url: Some("https://example.com/poster.jpg".to_string()),
            duration: Some("2h".to_string()),
            genres: vec![],
            seasons: vec![],
            dubs: vec![
                AudioTrackOption {
                    subject_id: "1".to_string(),
                    language: "Original".to_string(),
                    label: "Original".to_string(),
                },
                AudioTrackOption {
                    subject_id: "2".to_string(),
                    language: "Hindi".to_string(),
                    label: "Hindi".to_string(),
                },
            ],
        });
        app.state.selected_resources = vec![Release {
            provider: ProviderKind::MovieBox,
            filename: "Movie.1080p.mkv".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("HEVC".to_string()),
            language: None,
            size_bytes: Some(1024 * 1024 * 1024),
            season: None,
            episode: None,
            mirrors: vec![],
            resource_id: None,
        }];
        app.state.resource_list_state.select(Some(0));

        let area = Rect::new(0, 0, 120, 30);
        let layout = crate::tui::screens::details::details_screen_layout(
            area,
            app.state.selected_details.as_ref(),
        );
        let bottom_area = layout.bottom_area;
        let lower_chunks =
            Layout::vertical([Constraint::Length(4), Constraint::Min(3)]).split(bottom_area);
        let streams_area = lower_chunks[1];

        let empty_space_row = streams_area.bottom().saturating_sub(3);
        let action = app.handle_details_mouse(streams_area.x + 10, empty_space_row, area);
        assert!(action.is_none());
        assert_eq!(app.state.details_pane, DetailsPane::Streams);
        assert_eq!(app.state.resource_list_state.selected(), Some(0));

        let mut received_play = false;
        while let Ok(act) = app.action_receiver.try_recv() {
            if matches!(act, Action::PlayStream) {
                received_play = true;
            }
        }
        assert!(!received_play);

        let stream_item_row = if streams_area.height >= 4 {
            streams_area.y + 2
        } else {
            streams_area.y + 1
        };
        let action_stream = app.handle_details_mouse(streams_area.x + 10, stream_item_row, area);
        assert!(action_stream.is_none());
        assert_eq!(app.state.details_pane, DetailsPane::Streams);

        let mut stream_played = false;
        while let Ok(act) = app.action_receiver.try_recv() {
            if matches!(act, Action::PlayStream) {
                stream_played = true;
            }
        }
        assert!(stream_played);
    }

    #[tokio::test]
    async fn test_home_no_results_mouse_click_actions() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = crate::tui::state::InputMode::Normal;
        app.state.active_provider = ProviderKind::FourKHdHub;
        app.state.search_query.set_content("deewaniyat");
        app.state.search_results = vec![];

        let area = Rect::new(0, 0, 100, 30);
        let (_search_bar_area, results_chunk) =
            crate::tui::screens::home::search_results_layout(area);
        let next_label = app.state.next_provider().label();
        let ctrl_p = crate::tui::text::CTRL_P_STR;
        let (btn1, btn2) = crate::tui::screens::home::no_results_button_hitboxes(
            results_chunk,
            next_label,
            ctrl_p,
            app.state.is_tv_mode,
            app.state.active_provider == crate::providers::models::ProviderKind::Addons,
        );

        app.handle_home_mouse(btn2.x + 1, btn2.y, area);
        assert!(app.state.search_query.is_empty());

        app.state.search_query.set_content("deewaniyat");
        app.handle_home_mouse(btn1.x + 1, btn1.y, area);
        assert_ne!(app.state.active_provider, ProviderKind::FourKHdHub);
    }
    #[tokio::test]
    async fn test_home_provider_pill_mouse_click_opens_popup() {
        let original_config = crate::config::load();
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = crate::tui::state::InputMode::Normal;
        app.state.active_provider = ProviderKind::MovieBox;
        app.state.search_query.clear();

        let area = Rect::new(0, 0, 100, 30);
        let card_width = crate::tui::screens::home::search_deck_width(area, &app.state, true);
        let card_x = area.x + area.width.saturating_sub(card_width) / 2;
        let (_tier, rows) = crate::tui::screens::home::landing_split(
            area,
            app.state.is_tv_mode,
            app.state.basic_terminal,
            app.state.landing_deck_visible(),
        );
        let search_card_area = Rect {
            x: card_x,
            y: rows.rects[rows.search].y,
            width: card_width,
            height: rows.rects[rows.search].height,
        };

        let pill_rect =
            crate::tui::screens::home::search_bar_provider_pill_rect(search_card_area, &app.state);

        app.handle_home_mouse(pill_rect.x + 1, pill_rect.y, area);
        assert!(app.state.show_provider_popup);
        assert_eq!(app.state.provider_list_state.selected(), Some(0));

        let available = app.state.available_providers();
        let (_container, inner) = crate::tui::screens::home::provider_popup_bounds(
            area,
            search_card_area,
            available.len(),
        );

        let fourk_idx = available
            .iter()
            .position(|p| *p == ProviderKind::FourKHdHub)
            .unwrap();
        let target_y = inner.y + fourk_idx as u16;
        app.handle_home_mouse(inner.x + 1, target_y, area);
        assert!(!app.state.show_provider_popup);
        assert_eq!(app.state.active_provider, ProviderKind::FourKHdHub);

        app.handle_home_mouse(pill_rect.x + 1, pill_rect.y, area);
        assert!(app.state.show_provider_popup);
        let last_idx = available.len() - 1;
        let last_provider = available[last_idx];
        let last_target_y = inner.y + last_idx as u16;
        app.handle_home_mouse(inner.x + 1, last_target_y, area);
        assert!(!app.state.show_provider_popup);
        assert_eq!(app.state.active_provider, last_provider);

        let pill_rect =
            crate::tui::screens::home::search_bar_provider_pill_rect(search_card_area, &app.state);
        app.handle_home_mouse(pill_rect.x + 1, pill_rect.y, area);
        assert!(app.state.show_provider_popup);
        app.handle_home_mouse(area.x, area.y, area);
        assert!(!app.state.show_provider_popup);
        assert_eq!(app.state.active_provider, last_provider);
        crate::config::save(&original_config);
    }

    #[tokio::test]
    async fn test_download_cancel_mouse_hitbox() {
        let mut app = App::new();
        let area = Rect::new(0, 0, 80, 24);
        app.state.download_progress = Some(50.0);
        app.state
            .cancel_download
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let handled = app.handle_overlay_mouse(20, 22, area);
        assert!(handled);
        assert!(app.action_receiver.try_recv().is_err());

        let handled_cancel = app.handle_overlay_mouse(72, 21, area);
        assert!(handled_cancel);
        assert!(matches!(
            app.action_receiver.try_recv().ok(),
            Some(Action::CancelDownload)
        ));
    }

    #[tokio::test]
    async fn test_subtitle_popup_outside_click_emits_cancellation() {
        let mut app = App::new();
        let area = Rect::new(0, 0, 80, 24);
        app.state.subtitle_popup = true;
        app.state.subtitle_list = vec![("English".to_string(), "https://sub.url".to_string())];

        let handled = app.handle_overlay_mouse(0, 0, area);
        assert!(handled);
        assert!(!app.state.subtitle_popup);
        let notif = app.state.notifications.back().expect("notification posted");
        assert_eq!(notif.title, "Playback Cancelled");
    }
}
