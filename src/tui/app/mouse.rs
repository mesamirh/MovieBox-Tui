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
        if self.state.show_theme_popup {
            let items = crate::tui::theme::AVAILABLE_THEMES;
            match hit_test_centered_picker(area, col, row, 40, items.len(), 30, 60) {
                Some(Some(clicked_idx)) => {
                    self.state.theme_list_state.select(Some(clicked_idx));
                    if let Some(theme_name) = items.get(clicked_idx) {
                        self.action_sender
                            .send(Action::SelectTheme(theme_name.to_string()))
                            .ok();
                        self.state.show_theme_popup = false;
                        self.state.theme_list_state.select(None);
                        self.state
                            .set_status(format!("{theme_name} theme applied."), 150);
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

        if self.state.show_browse_popup {
            let is_addon = self.state.mode() == crate::tui::state::AppMode::Addon;
            let total_count = if is_addon {
                crate::providers::addons::models::curated_catalog_presets(
                    &self.state.installed_addons,
                )
                .len()
            } else {
                BrowsePreset::ALL.len()
            };
            match hit_test_centered_picker(area, col, row, 40, total_count, 30, 60) {
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
                if row == layout.button_row_y {
                    if col < layout.update_btn_end_x {
                        self.action_sender.send(Action::StartSelfUpdate).ok();
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

        if self.state.player_picker_popup {
            let count = self.state.available_players.len();
            match hit_test_centered_picker(area, col, row, 40, count, 24, 60) {
                Some(Some(clicked_idx)) => {
                    self.state.player_picker_state.select(Some(clicked_idx));
                    self.action_sender.send(Action::Submit).ok();
                }
                Some(None) => {}
                None => {
                    self.state.player_picker_popup = false;
                }
            }
            return true;
        }

        if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
            let count = self.state.subtitle_list.len().min(8);
            match hit_test_centered_picker(area, col, row, 44, count, 30, 60) {
                Some(Some(clicked_idx)) => {
                    self.state.subtitle_list_state.select(Some(clicked_idx));
                    self.action_sender.send(Action::Submit).ok();
                }
                Some(None) => {}
                None => {
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                }
            }
            return true;
        }

        if self.state.show_season_download_confirm {
            let popup = crate::tui::overlay::centered(area, 40, 6, 36, 64);
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                let action_y = popup.y + 3;
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
            let popup = crate::tui::overlay::centered(area, 40, 6, 36, 64);
            if popup.contains(ratatui::layout::Position::new(col, row)) {
                let action_y = popup.y + 3;
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
            let content_width = self
                .state
                .tv_playlists
                .iter()
                .map(|source| crate::tui::text::width(source))
                .max()
                .unwrap_or(28)
                .max(40)
                .max(crate::tui::text::width(
                    "[ Add URL ] [ Add file ] [ Reload ] [ Done ]",
                ));
            let (popup_width, popup_height) = if self.state.tv_input_active {
                (content_width.saturating_add(6) as u16, 6)
            } else {
                (
                    content_width.saturating_add(6) as u16,
                    total_rows.min(10).saturating_add(5) as u16,
                )
            };
            let popup = crate::tui::overlay::centered(area, popup_width, popup_height, 36, 70);
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
            let popup_width = 76u16.min(area.width.saturating_sub(4)).max(56);
            let popup_height = if self.state.addon_input_active {
                7u16
            } else {
                (addons_count as u16)
                    .saturating_add(6)
                    .min(area.height.saturating_sub(4))
                    .max(7)
            };
            let popup = crate::tui::overlay::centered(area, popup_width, popup_height, 36, 80);
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
                        let rel_x = col.saturating_sub(popup.x + 1);
                        if rel_x <= 25 {
                            self.state.addon_manager_selected = addons_count + 1;
                        } else {
                            self.state.addon_manager_selected = addons_count + 2;
                        }
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
        let logo_height = if self.state.basic_terminal { 2 } else { 6 };
        let search_width =
            crate::tui::screens::home::search_deck_width(area, &self.state, is_landing);

        let search_bar_area = if is_landing {
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(16),
                    Constraint::Length(logo_height),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

            if row == vertical_chunks[7].y {
                self.handle_home_mode_click(col, area.width);
                return None;
            }
            if row == vertical_chunks[8].y {
                self.handle_home_util_click(col, area.width);
                return None;
            }
            if self.state.favorites_landing_visible()
                && self.handle_favorites_landing_click(col, row, vertical_chunks[6])
            {
                return None;
            }

            centered_width_rect(vertical_chunks[4], search_width)
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);

            Rect {
                x: chunks[0].x + 2,
                y: chunks[0].y,
                width: chunks[0].width.saturating_sub(4),
                height: chunks[0].height,
            }
        };

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

            let start_y = search_bar_area.bottom();
            if row >= start_y && (row - start_y) < visible_count as u16 {
                let clicked_idx = suggestion_offset + (row - start_y) as usize;
                if let Some(query) = self.state.search_suggestions.get(clicked_idx).cloned() {
                    self.action_sender
                        .send(Action::SelectSuggestion { query })
                        .ok();
                }
                return None;
            }
        }

        if row >= search_bar_area.y && row < search_bar_area.y + search_bar_area.height {
            self.state.input_mode = InputMode::Editing;
            self.state.favorites_focus = false;
            self.state.favorites_landing_state.select(None);
            return None;
        }

        if !is_landing {
            let results_y = 2;
            if row >= results_y && row < area.height.saturating_sub(1) {
                let row_height = self.state.poster_rows.max(3) + 1;
                let clicked_relative_row = row.saturating_sub(results_y);
                let clicked_idx = (clicked_relative_row / row_height) as usize;

                let table_offset = self.state.search_list_state.selected().unwrap_or(0);
                let visible_items =
                    ((area.height.saturating_sub(results_y)) / row_height).max(1) as usize;
                let page_start = if table_offset >= visible_items {
                    table_offset.saturating_sub(visible_items - 1)
                } else {
                    0
                };

                let target_idx = page_start + clicked_idx;
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
        }

        None
    }

    /// Mirrors the layout math in `screens::home::render_favorites_landing`
    /// so hitboxes stay in sync with what's drawn. Returns `true` if the
    /// click landed inside the favorites card (handled either way).
    fn handle_favorites_landing_click(&mut self, col: u16, row: u16, area: Rect) -> bool {
        let row_count = self.state.favorites_landing_items().len() as u16;
        if row_count == 0 || area.height < 2 || area.width < 20 {
            return false;
        }
        let overflow = self
            .state
            .favorites
            .items
            .len()
            .saturating_sub(row_count as usize);
        let overflow_row = u16::from(overflow > 0);
        let card_width = area.width.clamp(20, 56);
        let content_height = (1 + row_count + overflow_row).min(area.height);
        let card = Rect {
            x: area.x + area.width.saturating_sub(card_width) / 2,
            y: area.y,
            width: card_width,
            height: content_height,
        };
        if !card.contains(ratatui::layout::Position::new(col, row)) {
            return false;
        }

        let rel_row = row - card.y;
        if rel_row == 0 {
            // Header row; no action.
        } else if rel_row <= row_count {
            let idx = (rel_row - 1) as usize;
            let prev_selected = if self.state.favorites_focus {
                self.state.favorites_landing_state.selected()
            } else {
                None
            };
            self.state.favorites_focus = true;
            self.state.favorites_landing_state.select(Some(idx));
            if prev_selected == Some(idx) {
                self.action_sender.send(Action::OpenFavorite(idx)).ok();
            }
        } else if overflow > 0 && rel_row == row_count + 1 {
            self.action_sender.send(Action::ShowFavorites).ok();
        }
        true
    }

    fn handle_home_mode_click(&mut self, col: u16, width: u16) {
        enum ModeBtn {
            Streaming,
            Tv,
            Addon,
        }

        let ctrl_s = crate::tui::text::ctrl_key("S");
        let ctrl_t = crate::tui::text::ctrl_key("T");
        let ctrl_a = crate::tui::text::ctrl_key("A");
        let ctrl_p = crate::tui::text::ctrl_key("P");

        let mut buttons: Vec<(ModeBtn, u16)> = Vec::new();
        let sep = 5_u16;

        let current_mode = self.state.mode();
        let is_streaming = current_mode == crate::tui::state::AppMode::Streaming;
        if self.state.streaming_enabled {
            let b1_len = if is_streaming {
                (1 + ctrl_p.len() + 2 + self.state.active_provider.label().chars().count()) as u16
            } else {
                (1 + ctrl_s.len() + 2 + 9) as u16
            };
            buttons.push((ModeBtn::Streaming, b1_len));
        }

        if self.state.tv_enabled {
            let b2_len = if current_mode == crate::tui::state::AppMode::Tv {
                6_u16
            } else {
                (1 + ctrl_t.len() + 2 + 2) as u16
            };
            buttons.push((ModeBtn::Tv, b2_len));
        }

        if self.state.addons_enabled {
            let b3_len = if current_mode == crate::tui::state::AppMode::Addon {
                10_u16
            } else {
                (1 + ctrl_a.len() + 2 + 6) as u16
            };
            buttons.push((ModeBtn::Addon, b3_len));
        }

        let total_w: u16 = buttons.iter().map(|(_, w)| *w).sum::<u16>()
            + (buttons.len().saturating_sub(1) as u16) * sep;
        let mut curr_x = width.saturating_sub(total_w) / 2;

        for (btn, w) in buttons {
            let start = curr_x;
            let end = start + w;
            if col >= start && col <= end + 1 {
                match btn {
                    ModeBtn::Streaming => {
                        if is_streaming {
                            self.cycle_provider();
                        } else {
                            self.action_sender.send(Action::SwitchToStreamingMode).ok();
                        }
                    }
                    ModeBtn::Tv => {
                        if current_mode != crate::tui::state::AppMode::Tv {
                            self.action_sender.send(Action::ToggleTvMode).ok();
                        }
                    }
                    ModeBtn::Addon => {
                        if current_mode != crate::tui::state::AppMode::Addon {
                            self.action_sender.send(Action::ToggleAddonMode).ok();
                        }
                    }
                }
                return;
            }
            curr_x += w + sep;
        }
    }

    fn handle_home_util_click(&mut self, col: u16, width: u16) {
        enum UtilBtn {
            Help,
            Quit,
        }

        let buttons: [(UtilBtn, u16); 2] = [(UtilBtn::Help, 8), (UtilBtn::Quit, 8)];
        let sep = 5_u16;

        let total_w: u16 = buttons.iter().map(|(_, w)| *w).sum::<u16>()
            + (buttons.len().saturating_sub(1) as u16) * sep;
        let mut curr_x = width.saturating_sub(total_w) / 2;

        for (btn, w) in buttons {
            let start = curr_x;
            let end = start + w;
            if col >= start && col <= end + 1 {
                match btn {
                    UtilBtn::Help => {
                        self.action_sender.send(Action::ToggleHelp).ok();
                    }
                    UtilBtn::Quit => {
                        self.action_sender.send(Action::Quit).ok();
                    }
                }
                return;
            }
            curr_x += w + sep;
        }
    }

    fn handle_details_mouse(&mut self, col: u16, row: u16, area: Rect) -> Option<Action> {
        let details_json = self.state.selected_details.as_ref()?.clone();

        let type_val = crate::tui::state::stype(&details_json);
        let has_languages = details_json
            .get("dubs")
            .and_then(|d| d.as_array())
            .is_some_and(|d| d.len() > 1);
        let is_series = type_val == 2 && !self.state.available_seasons.is_empty();

        let mut available_panes = Vec::new();
        if has_languages {
            available_panes.push(DetailsPane::Languages);
        }
        if is_series {
            available_panes.push(DetailsPane::Seasons);
            available_panes.push(DetailsPane::Episodes);
        }

        let tier = crate::tui::screens::details::DetailsLayoutTier::for_area(area);
        let header_height = tier.header_height(area, self.state.selected_details.as_ref());
        let footer_height = tier.footer_height(area.width);

        let chunks = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(footer_height),
        ])
        .split(area);

        let workflow_area = chunks[1];
        let bottom_area = chunks[2];
        let footer_area = chunks[3];

        if row >= footer_area.y && row < footer_area.bottom() {
            self.handle_details_footer_click(col, row - footer_area.y, area.width);
            return None;
        }

        if row == workflow_area.y {
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

        let visible_selector_panes = if matches!(
            tier,
            crate::tui::screens::details::DetailsLayoutTier::Narrow
                | crate::tui::screens::details::DetailsLayoutTier::Tiny
        ) {
            available_panes
                .iter()
                .copied()
                .filter(|pane| *pane == self.state.details_pane)
                .collect::<Vec<_>>()
        } else {
            available_panes
        };

        let selector_height = if visible_selector_panes.is_empty() {
            0
        } else {
            let episode_count = self
                .state
                .available_episode_numbers
                .get(self.state.season_list_state.selected().unwrap_or(0))
                .map_or(0, Vec::len);
            let language_count = details_json
                .get("dubs")
                .and_then(|dubs| dubs.as_array())
                .map_or(0, Vec::len);
            language_count
                .max(self.state.available_seasons.len())
                .max(episode_count)
                .min(4) as u16
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
            let selector_chunks = Layout::horizontal(vec![
                Constraint::Ratio(
                    1,
                    visible_selector_panes.len() as u32
                );
                visible_selector_panes.len()
            ])
            .split(selector_area);

            for (pane, pane_rect) in visible_selector_panes
                .into_iter()
                .zip(selector_chunks.iter())
            {
                if pane_rect.contains(ratatui::layout::Position::new(col, row)) {
                    let clicked_row = row.saturating_sub(pane_rect.y + 1) as usize;
                    match pane {
                        DetailsPane::Languages => {
                            self.state.details_pane = DetailsPane::Languages;
                            if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array())
                            {
                                if clicked_row < dubs.len() {
                                    self.action_sender
                                        .send(Action::SelectLanguage(clicked_row))
                                        .ok();
                                }
                            }
                        }
                        DetailsPane::Seasons => {
                            self.state.details_pane = DetailsPane::Seasons;
                            if clicked_row < self.state.available_seasons.len() {
                                self.state.season_list_state.select(Some(clicked_row));
                                self.state.selected_season =
                                    self.state
                                        .available_seasons
                                        .get(clicked_row)
                                        .and_then(|s| s.get("se"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(1) as usize;
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
            let streams_count = self
                .state
                .selected_resources
                .as_ref()
                .and_then(|r| r.get("list"))
                .and_then(|l| l.as_array())
                .map_or(0, Vec::len);

            if streams_count > 0 {
                let list = self
                    .state
                    .selected_resources
                    .as_ref()
                    .and_then(|r| r.get("list"))
                    .and_then(|l| l.as_array())
                    .cloned()
                    .unwrap_or_default();

                let clicked_stream_row = row.saturating_sub(streams_area.y + 1);
                let mut line_offset = 0_u16;
                let mut prev_resolution = None;
                let mut matched_idx = None;

                for (i, file) in list.iter().enumerate() {
                    let resolution = file.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                    if prev_resolution != Some(resolution) {
                        if i > 0 {
                            line_offset += 1;
                        }
                        line_offset += 1;
                        prev_resolution = Some(resolution);
                    }
                    if clicked_stream_row == line_offset {
                        matched_idx = Some(i);
                        break;
                    }
                    line_offset += 1;
                }

                let target_idx = matched_idx.unwrap_or_else(|| {
                    (clicked_stream_row as usize / 2).min(streams_count.saturating_sub(1))
                });

                let prev_selected = self.state.resource_list_state.selected();
                self.state.resource_list_state.select(Some(target_idx));

                if prev_selected == Some(target_idx) {
                    if self.state.is_playing {
                        self.state.notify(
                            NotificationKind::Warning,
                            "Playback already active",
                            "Stop the current player before starting another.",
                        );
                    } else if !self.state.is_resolving_playback
                        && self.state.last_playback_launch.elapsed().as_millis() >= 500
                    {
                        self.action_sender.send(Action::PlayStream(false)).ok();
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
        let is_languages = self.state.details_pane == DetailsPane::Languages;
        let compact = width < 80;
        let very_compact = width < 45;

        enum FooterAction {
            Tab,
            Move,
            PlaySelect,
            OpenWith,
            Download,
            Refresh,
            Back,
        }

        let mut primary: Vec<(FooterAction, u16)> = Vec::new();
        let tab_label_len = if compact { 4 } else { 9 };
        primary.push((FooterAction::Tab, 5 + 1 + tab_label_len));
        primary.push((FooterAction::Move, 4 + 1 + 4));

        if !very_compact {
            let enter_label_len = if is_streams { 4 } else { 6 };
            primary.push((FooterAction::PlaySelect, 7 + 1 + enter_label_len));
        }

        let mut secondary: Vec<(FooterAction, u16)> = Vec::new();
        if very_compact {
            let enter_label_len = if is_streams { 4 } else { 6 };
            secondary.push((FooterAction::PlaySelect, 7 + 1 + enter_label_len));
        } else {
            if is_streams {
                let o_label_len = if compact { 4 } else { 9 };
                secondary.push((FooterAction::OpenWith, 3 + 1 + o_label_len));
            }
            if !is_languages {
                let d_label_len = if compact { 4 } else { 8 };
                secondary.push((FooterAction::Download, 3 + 1 + d_label_len));
            }
            if !very_compact {
                let r_label_len = if compact { 5 } else { 7 };
                secondary.push((FooterAction::Refresh, 3 + 1 + r_label_len));
            }
        }
        secondary.push((FooterAction::Back, 5 + 1 + 4));

        let active_buttons = if width >= 70 {
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
                    FooterAction::Tab => {
                        self.action_sender.send(Action::TabPane).ok();
                    }
                    FooterAction::Move => {
                        self.action_sender.send(Action::MoveDown).ok();
                    }
                    FooterAction::PlaySelect => {
                        if is_streams {
                            self.action_sender.send(Action::PlayStream(false)).ok();
                        } else {
                            self.action_sender.send(Action::Submit).ok();
                        }
                    }
                    FooterAction::OpenWith => {
                        self.action_sender.send(Action::PlayStream(true)).ok();
                    }
                    FooterAction::Download => {
                        if is_seasons {
                            self.action_sender.send(Action::PromptDownloadSeason).ok();
                        } else {
                            self.action_sender.send(Action::PromptDownloadEpisode).ok();
                        }
                    }
                    FooterAction::Refresh => {
                        self.action_sender.send(Action::Refresh).ok();
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

fn centered_width_rect(area: Rect, width: u16) -> Rect {
    let w = width.min(area.width);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y,
        width: w,
        height: area.height,
    }
}

fn hit_test_centered_picker(
    area: Rect,
    col: u16,
    row: u16,
    width_pct: u16,
    total_items: usize,
    min_w: u16,
    max_w: u16,
) -> Option<Option<usize>> {
    let popup =
        crate::tui::overlay::centered(area, width_pct, total_items as u16 + 4, min_w, max_w);
    if popup.contains(ratatui::layout::Position::new(col, row)) {
        let item_y = popup.y.saturating_add(1);
        if row >= item_y && (row - item_y) < total_items as u16 {
            Some(Some((row - item_y) as usize))
        } else {
            Some(None)
        }
    } else {
        None
    }
}
