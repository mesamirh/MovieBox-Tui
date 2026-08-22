use super::App;
use crate::tui::{
    action::Action,
    state::{InputMode, Screen},
};
use crossterm::event::KeyEvent;

impl App {
    pub(super) async fn handle_key(&mut self, key: KeyEvent) -> Option<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key.code {
                self.action_sender.send(Action::Quit).ok();
                return None;
            }
            if let KeyCode::Char('t') = key.code {
                if self.state.tv_enabled {
                    self.action_sender.send(Action::ToggleTvMode).ok();
                } else {
                    self.state.set_status(
                        "TV Mode is disabled. Use /enable-tv to enable.".to_string(),
                        120,
                    );
                }
                return None;
            }
            if let KeyCode::Char('a') = key.code {
                if self.state.addons_enabled {
                    self.action_sender.send(Action::ToggleAddonMode).ok();
                } else {
                    self.state.set_status(
                        "Addon Mode is disabled. Use /enable-addons to enable.".to_string(),
                        120,
                    );
                }
                return None;
            }
            if let KeyCode::Char('s') = key.code {
                if !self.state.streaming_enabled {
                    self.state.set_status(
                        "Streaming Mode is disabled. Use /enable-streaming to enable.".to_string(),
                        120,
                    );
                } else if !self.state.is_tv_mode && !self.state.is_addon_mode {
                    self.state
                        .set_status("Already in Streaming Mode.".to_string(), 120);
                } else {
                    self.action_sender.send(Action::SwitchToStreamingMode).ok();
                }
                return None;
            }
            if let KeyCode::Char('p') = key.code {
                if self.state.is_tv_mode {
                    self.state.notify(
                        crate::tui::overlay::NotificationKind::Info,
                        "TV Mode",
                        "Provider cycling is only available in Streaming Mode.",
                    );
                } else if self.state.is_addon_mode {
                    self.action_sender.send(Action::ShowAddonManager).ok();
                } else {
                    self.cycle_provider();
                }
                return None;
            }
        }

        if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
            && self.state.download_progress.is_some()
            && self.state.input_mode != InputMode::Editing
            && !self.state.tv_input_active
        {
            self.action_sender.send(Action::CancelDownload).ok();
            return None;
        }

        if let Some((version, _)) = &self.state.update_available {
            match key.code {
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    self.action_sender.send(Action::StartSelfUpdate).ok();
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    let url = format!(
                        "https://github.com/mesamirh/MovieBox-Tui/releases/tag/v{}",
                        version
                    );
                    let _ = open::that(&url);
                    self.state.update_available = None;
                }
                KeyCode::Esc => {
                    self.state.update_available = None;
                }
                _ => {}
            }
            return None;
        }

        if self.state.show_browse_popup {
            let is_addon = self.state.mode() == crate::tui::state::AppMode::Addon;
            let total_count = if is_addon {
                crate::providers::addons::models::curated_catalog_presets(
                    &self.state.installed_addons,
                )
                .len()
            } else {
                crate::tui::state::BrowsePreset::ALL.len()
            };

            match key.code {
                KeyCode::Esc => {
                    self.state.show_browse_popup = false;
                    self.state.browse_list_state.select(None);
                }
                KeyCode::Up => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        false,
                    );
                }
                KeyCode::Down => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        true,
                    );
                }
                KeyCode::Enter => {
                    let index = self.state.browse_list_state.selected().unwrap_or(0);
                    if is_addon {
                        let targets = crate::providers::addons::models::curated_catalog_presets(
                            &self.state.installed_addons,
                        );
                        if let Some(target) = targets.get(index).cloned() {
                            self.action_sender
                                .send(Action::SelectAddonCatalog(target))
                                .ok();
                        }
                    } else if let Some(preset) =
                        crate::tui::state::BrowsePreset::ALL.get(index).copied()
                    {
                        self.action_sender.send(Action::SelectBrowse(preset)).ok();
                    }
                }
                _ => {}
            }
            return None;
        }

        if self.state.show_theme_popup {
            match key.code {
                KeyCode::Esc => {
                    self.state.show_theme_popup = false;
                    if let Some(orig) = self.state.original_theme_kind.take() {
                        self.action_sender.send(Action::SelectTheme(orig)).ok();
                    }
                }
                KeyCode::Up => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.theme_list_state,
                        crate::tui::theme::AVAILABLE_THEMES.len(),
                        false,
                    );
                    if let Some(i) = self.state.theme_list_state.selected() {
                        let selected_theme = crate::tui::theme::AVAILABLE_THEMES[i].to_string();
                        self.action_sender
                            .send(Action::SelectTheme(selected_theme))
                            .ok();
                    }
                }
                KeyCode::Down => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.theme_list_state,
                        crate::tui::theme::AVAILABLE_THEMES.len(),
                        true,
                    );
                    if let Some(i) = self.state.theme_list_state.selected() {
                        let selected_theme = crate::tui::theme::AVAILABLE_THEMES[i].to_string();
                        self.action_sender
                            .send(Action::SelectTheme(selected_theme))
                            .ok();
                    }
                }
                KeyCode::Enter => {
                    self.state.show_theme_popup = false;
                    self.state.original_theme_kind = None;
                    self.persist_config();
                }
                _ => {}
            }
            return None;
        }

        match self.state.input_mode {
            InputMode::Editing => match key.code {
                KeyCode::Esc => {
                    self.state.input_mode = InputMode::Normal;
                    self.state.suggest_index = None;
                    self.state.search_suggestions.clear();
                    if self.state.search_query.starts_with('/')
                        || self.state.search_results.is_empty()
                    {
                        self.state.search_query.clear();
                    }
                    self.state.set_status(String::new(), 150);
                }
                KeyCode::Enter => {
                    let selected_opt = self
                        .state
                        .suggest_index
                        .and_then(|idx| self.state.search_suggestions.get(idx).cloned());

                    let mut query = if let Some(sug) = selected_opt {
                        sug
                    } else {
                        self.state.search_query.trim().to_string()
                    };

                    if query.starts_with('/') {
                        let suggestions =
                            crate::tui::commands::SlashCommand::suggest(&self.state, &query);
                        if suggestions.len() == 1 {
                            query = suggestions[0].clone();
                        }
                    }

                    if !query.is_empty() {
                        if query.trim().eq_ignore_ascii_case("/history") {
                            self.state.search_query = "/history".to_string();
                        } else if query.starts_with('/') {
                            self.state.search_query.clear();
                        } else {
                            self.state.search_query = query.clone();
                        }
                        self.state.input_mode = InputMode::Normal;
                        self.state.search_suggestions.clear();
                        self.state.suggest_index = None;
                        self.state.search_list_state.select(None);
                        self.state.last_search_edit = std::time::Instant::now();
                        self.action_sender
                            .send(Action::Search {
                                query,
                                force_refresh: false,
                            })
                            .ok();
                    }
                }
                KeyCode::Tab => {
                    let trimmed = self.state.search_query.trim();
                    if trimmed.starts_with('/') {
                        let selected_or_first = self
                            .state
                            .suggest_index
                            .and_then(|idx| self.state.search_suggestions.get(idx).cloned())
                            .or_else(|| {
                                let suggestions = crate::tui::commands::SlashCommand::suggest(
                                    &self.state,
                                    trimmed,
                                );
                                suggestions.first().cloned()
                            });
                        if let Some(sug) = selected_or_first {
                            self.state.search_query = sug;
                            self.state.last_search_edit = std::time::Instant::now();
                        }
                    }
                }
                KeyCode::Backspace => {
                    crate::tui::text::remove_last_grapheme(&mut self.state.search_query);
                    self.state.suggest_index = None;
                    self.state.last_search_edit = std::time::Instant::now();
                }
                KeyCode::Char(c) => {
                    self.state.search_query.push(c);
                    self.state.suggest_index = None;
                    self.state.last_search_edit = std::time::Instant::now();
                }
                KeyCode::Up if !self.state.search_suggestions.is_empty() => {
                    let max_idx = self.state.search_suggestions.len() - 1;
                    let next_idx = match self.state.suggest_index {
                        Some(0) | None => max_idx,
                        Some(i) => i - 1,
                    };
                    self.state.suggest_index = Some(next_idx);
                }
                KeyCode::Down if !self.state.search_suggestions.is_empty() => {
                    let max_idx = self.state.search_suggestions.len() - 1;
                    let next_idx = match self.state.suggest_index {
                        None => 0,
                        Some(i) if i == max_idx => 0,
                        Some(i) => i + 1,
                    };
                    self.state.suggest_index = Some(next_idx);
                }
                _ => {}
            },
            InputMode::Normal => match self.state.active_screen {
                Screen::Home => {
                    if self.state.addon_manager_popup {
                        if self.state.addon_input_active {
                            match key.code {
                                KeyCode::Esc => {
                                    self.state.addon_input_active = false;
                                    self.state.addon_input_buffer.clear();
                                    self.state.addon_input_cursor = 0;
                                }
                                KeyCode::Enter => {
                                    let buffer = self.state.addon_input_buffer.trim().to_string();
                                    self.state.addon_input_active = false;
                                    self.state.addon_input_buffer.clear();
                                    self.state.addon_input_cursor = 0;
                                    if !buffer.is_empty() {
                                        self.action_sender
                                            .send(Action::AddonAddManifest(buffer))
                                            .ok();
                                    }
                                }
                                KeyCode::Left => {
                                    self.state.addon_input_cursor =
                                        self.state.addon_input_cursor.saturating_sub(1);
                                }
                                KeyCode::Right => {
                                    if self.state.addon_input_cursor
                                        < self.state.addon_input_buffer.chars().count()
                                    {
                                        self.state.addon_input_cursor += 1;
                                    }
                                }
                                KeyCode::Backspace => {
                                    if self.state.addon_input_cursor > 0 {
                                        let mut chars: Vec<char> =
                                            self.state.addon_input_buffer.chars().collect();
                                        chars.remove(self.state.addon_input_cursor - 1);
                                        self.state.addon_input_buffer = chars.into_iter().collect();
                                        self.state.addon_input_cursor -= 1;
                                    }
                                }
                                KeyCode::Delete => {
                                    let mut chars: Vec<char> =
                                        self.state.addon_input_buffer.chars().collect();
                                    if self.state.addon_input_cursor < chars.len() {
                                        chars.remove(self.state.addon_input_cursor);
                                        self.state.addon_input_buffer = chars.into_iter().collect();
                                    }
                                }
                                KeyCode::Char(c) if !c.is_control() => {
                                    let mut chars: Vec<char> =
                                        self.state.addon_input_buffer.chars().collect();
                                    chars.insert(self.state.addon_input_cursor, c);
                                    self.state.addon_input_buffer = chars.into_iter().collect();
                                    self.state.addon_input_cursor += 1;
                                }
                                _ => {}
                            }
                            return None;
                        }
                        match key.code {
                            KeyCode::Esc => {
                                self.reset_transient_overlays();
                                self.state.addon_manager_popup = false;
                            }
                            KeyCode::Up => {
                                use crate::tui::state::AddonManagerRow;
                                let rows = self.state.addon_manager_rows();
                                let total = rows.len();
                                let mut next = if self.state.addon_manager_selected == 0 {
                                    total.saturating_sub(1)
                                } else {
                                    self.state.addon_manager_selected - 1
                                };
                                while next != self.state.addon_manager_selected
                                    && matches!(rows.get(next), Some(AddonManagerRow::Header(_)))
                                {
                                    next = if next == 0 {
                                        total.saturating_sub(1)
                                    } else {
                                        next - 1
                                    };
                                }
                                self.state.addon_manager_selected = next;
                            }
                            KeyCode::Down => {
                                use crate::tui::state::AddonManagerRow;
                                let rows = self.state.addon_manager_rows();
                                let total = rows.len();
                                let mut next = if self.state.addon_manager_selected + 1 >= total {
                                    0
                                } else {
                                    self.state.addon_manager_selected + 1
                                };
                                while next != self.state.addon_manager_selected
                                    && matches!(rows.get(next), Some(AddonManagerRow::Header(_)))
                                {
                                    next = if next + 1 >= total { 0 } else { next + 1 };
                                }
                                self.state.addon_manager_selected = next;
                            }
                            KeyCode::Char('d') | KeyCode::Delete => {
                                use crate::tui::state::AddonManagerRow;
                                if let Some(AddonManagerRow::Addon(index)) = self
                                    .state
                                    .addon_manager_rows()
                                    .get(self.state.addon_manager_selected)
                                    .copied()
                                {
                                    self.action_sender.send(Action::AddonRemove(index)).ok();
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                self.addon_manager_activate();
                            }
                            _ => {}
                        }
                        return None;
                    }

                    if self.state.tv_config_popup {
                        if self.state.tv_input_active {
                            match key.code {
                                KeyCode::Esc => {
                                    self.state.tv_input_active = false;
                                    self.state.tv_input_buffer.clear();
                                }
                                KeyCode::Enter => {
                                    let buffer = self.state.tv_input_buffer.trim().to_string();
                                    self.state.tv_input_active = false;
                                    self.state.tv_input_buffer.clear();
                                    if !buffer.is_empty() {
                                        self.action_sender.send(Action::TvPlaylistAdd(buffer)).ok();
                                    }
                                }
                                KeyCode::Backspace => {
                                    crate::tui::text::remove_last_grapheme(
                                        &mut self.state.tv_input_buffer,
                                    );
                                }
                                KeyCode::Char(c) if !c.is_control() => {
                                    self.state.tv_input_buffer.push(c);
                                }
                                _ => {}
                            }
                            return None;
                        }
                        match key.code {
                            KeyCode::Esc => {
                                self.reset_transient_overlays();
                                self.state.tv_config_popup = false;
                            }
                            KeyCode::Up => {
                                use crate::tui::state::TvManagerRow;
                                let rows = self.state.tv_manager_rows();
                                let total = rows.len();
                                let mut next = if self.state.tv_manager_selected == 0 {
                                    total.saturating_sub(1)
                                } else {
                                    self.state.tv_manager_selected - 1
                                };
                                while next != self.state.tv_manager_selected
                                    && matches!(rows.get(next), Some(TvManagerRow::Header(_)))
                                {
                                    next = if next == 0 {
                                        total.saturating_sub(1)
                                    } else {
                                        next - 1
                                    };
                                }
                                self.state.tv_manager_selected = next;
                            }
                            KeyCode::Down => {
                                use crate::tui::state::TvManagerRow;
                                let rows = self.state.tv_manager_rows();
                                let total = rows.len();
                                let mut next = if self.state.tv_manager_selected + 1 >= total {
                                    0
                                } else {
                                    self.state.tv_manager_selected + 1
                                };
                                while next != self.state.tv_manager_selected
                                    && matches!(rows.get(next), Some(TvManagerRow::Header(_)))
                                {
                                    next = if next + 1 >= total { 0 } else { next + 1 };
                                }
                                self.state.tv_manager_selected = next;
                            }
                            KeyCode::Char('d') => {
                                use crate::tui::state::TvManagerRow;
                                if let Some(TvManagerRow::Playlist(index)) = self
                                    .state
                                    .tv_manager_rows()
                                    .get(self.state.tv_manager_selected)
                                    .copied()
                                {
                                    self.action_sender
                                        .send(Action::TvPlaylistRemove(index))
                                        .ok();
                                }
                            }
                            KeyCode::Enter => {
                                self.tv_manager_activate();
                            }
                            _ => {}
                        }
                        return None;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.action_sender.send(Action::GoBack).ok();
                        }
                        KeyCode::Up => {
                            self.action_sender.send(Action::MoveUp).ok();
                        }
                        KeyCode::Down => {
                            self.action_sender.send(Action::MoveDown).ok();
                        }
                        KeyCode::Left => {
                            self.action_sender.send(Action::MoveLeft).ok();
                        }
                        KeyCode::Right => {
                            self.action_sender.send(Action::MoveRight).ok();
                        }
                        KeyCode::Enter => {
                            if self.state.search_results.is_empty()
                                && !self.state.search_query.trim().is_empty()
                            {
                                self.action_sender
                                    .send(Action::Search {
                                        query: self.state.search_query.trim().to_string(),
                                        force_refresh: true,
                                    })
                                    .ok();
                            } else {
                                self.action_sender.send(Action::Submit).ok();
                            }
                        }
                        KeyCode::Char('?') => {
                            self.action_sender.send(Action::ToggleHelp).ok();
                        }
                        KeyCode::Char('q') => {
                            self.action_sender.send(Action::Quit).ok();
                        }
                        KeyCode::Char('r') => {
                            if self.state.is_tv_mode {
                                self.action_sender.send(Action::TvReloadPlaylists).ok();
                            } else {
                                self.action_sender.send(Action::Refresh).ok();
                            }
                        }
                        KeyCode::Char('o') | KeyCode::Char('O')
                            if self.state.input_mode == InputMode::Normal
                                && self.state.is_tv_mode =>
                        {
                            let idx_opt = self.state.search_list_state.selected();
                            if let Some(idx) = idx_opt {
                                if let Some(item) = self.state.search_results.get(idx) {
                                    self.action_sender
                                        .send(Action::ShowPlayerPicker(item.id.clone(), None))
                                        .ok();
                                }
                            }
                        }
                        KeyCode::Char('*') if self.state.favorites_available() => {
                            self.action_sender.send(Action::ToggleFavorite).ok();
                        }
                        KeyCode::Char(c)
                            if (key.modifiers.is_empty()
                                || key.modifiers == KeyModifiers::SHIFT) =>
                        {
                            self.state.input_mode = InputMode::Editing;
                            self.state.favorites_focus = false;
                            self.state.favorites_landing_state.select(None);
                            if c == '/' {
                                self.state.search_query.clear();
                            }
                            self.state.search_query.push(c);

                            self.state.search_suggestions.clear();
                            self.state.suggest_index = None;
                            self.state.set_status(String::new(), 150);
                            self.state.last_search_edit = std::time::Instant::now();
                        }
                        _ => {}
                    }
                }
                Screen::Details => match key.code {
                    KeyCode::Tab => {
                        self.action_sender.send(Action::TabPane).ok();
                    }
                    KeyCode::BackTab => {
                        self.action_sender.send(Action::BackTabPane).ok();
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if self.state.show_season_download_confirm {
                            self.action_sender.send(Action::ConfirmDownloadSeason).ok();
                        } else if self.state.show_episode_download_confirm {
                            self.action_sender.send(Action::ConfirmDownloadEpisode).ok();
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        if self.state.show_season_download_confirm {
                            self.state.show_season_download_confirm = false;
                        } else if self.state.show_episode_download_confirm {
                            self.state.show_episode_download_confirm = false;
                        }
                    }
                    KeyCode::Esc => {
                        if self.state.show_season_download_confirm {
                            self.state.show_season_download_confirm = false;
                        } else if self.state.show_episode_download_confirm {
                            self.state.show_episode_download_confirm = false;
                        } else {
                            self.action_sender.send(Action::GoBack).ok();
                        }
                    }
                    KeyCode::Char('q') => {
                        self.action_sender.send(Action::Quit).ok();
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        if !self.state.subtitle_popup && !self.state.player_picker_popup {
                            if let crate::tui::state::DetailsPane::Streams = self.state.details_pane
                            {
                                self.action_sender.send(Action::PlayStream(true)).ok();
                            }
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if !self.state.subtitle_popup && !self.state.player_picker_popup {
                            if let crate::tui::state::DetailsPane::Seasons = self.state.details_pane
                            {
                                if !self.state.available_seasons.is_empty() {
                                    self.action_sender.send(Action::PromptDownloadSeason).ok();
                                }
                            } else {
                                self.action_sender.send(Action::PromptDownloadEpisode).ok();
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        self.action_sender.send(Action::Refresh).ok();
                    }
                    KeyCode::Char('?') => {
                        self.action_sender.send(Action::ToggleHelp).ok();
                    }
                    KeyCode::Char('b') => {
                        self.action_sender.send(Action::GoBack).ok();
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        if !self.state.subtitle_popup
                            && !self.state.player_picker_popup
                            && !self.state.show_season_download_confirm
                            && !self.state.show_episode_download_confirm
                            && self.state.favorites_available()
                        {
                            self.action_sender.send(Action::ToggleFavorite).ok();
                        }
                    }

                    KeyCode::Up => {
                        self.action_sender.send(Action::MoveUp).ok();
                    }
                    KeyCode::Down => {
                        self.action_sender.send(Action::MoveDown).ok();
                    }
                    KeyCode::Left => {
                        if self.state.show_season_download_confirm {
                            self.state.season_download_confirm_yes_selected = true;
                        } else if self.state.show_episode_download_confirm {
                            self.state.episode_download_confirm_yes_selected = true;
                        }
                    }
                    KeyCode::Right => {
                        if self.state.show_season_download_confirm {
                            self.state.season_download_confirm_yes_selected = false;
                        } else if self.state.show_episode_download_confirm {
                            self.state.episode_download_confirm_yes_selected = false;
                        }
                    }
                    KeyCode::Enter => {
                        let open_with = key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::SHIFT);
                        if self.state.show_season_download_confirm {
                            if self.state.season_download_confirm_yes_selected {
                                self.action_sender.send(Action::ConfirmDownloadSeason).ok();
                            } else {
                                self.state.show_season_download_confirm = false;
                            }
                        } else if self.state.show_episode_download_confirm {
                            if self.state.episode_download_confirm_yes_selected {
                                self.action_sender.send(Action::ConfirmDownloadEpisode).ok();
                            } else {
                                self.state.show_episode_download_confirm = false;
                            }
                        } else if self.state.subtitle_popup
                            || self.state.player_picker_popup
                            || self.state.is_download_subtitle_popup
                        {
                            self.action_sender.send(Action::Submit).ok();
                        } else {
                            match self.state.details_pane {
                                crate::tui::state::DetailsPane::Streams => {
                                    self.action_sender.send(Action::PlayStream(open_with)).ok();
                                }
                                crate::tui::state::DetailsPane::Seasons => {
                                    self.trigger_episode_fetch();
                                }
                                crate::tui::state::DetailsPane::Episodes => {
                                    self.trigger_episode_fetch();
                                }
                                crate::tui::state::DetailsPane::Languages => {
                                    let idx =
                                        self.state.language_list_state.selected().unwrap_or(0);

                                    self.action_sender.send(Action::SelectLanguage(idx)).ok();
                                }
                            }
                        }
                    }
                    _ => {}
                },
            },
        }
        None
    }
}
