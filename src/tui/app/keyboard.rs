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
                return Some(());
            }
            if let KeyCode::Char('t') = key.code {
                self.action_sender.send(Action::ToggleTvMode).ok();
                return None;
            }
            if let KeyCode::Char('p') = key.code {
                self.cycle_provider();
                return None;
            }
        }

        if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
            && self.state.download_progress.is_some()
        {
            self.action_sender.send(Action::CancelDownload).ok();
            return None;
        }

        if let Some((version, _)) = &self.state.update_available {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.state.update_available = None;
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    let url = format!(
                        "https://github.com/mesamirh/MovieBox-Tui/releases/tag/v{}",
                        version
                    );
                    let _ = open::that(&url);
                    self.state.update_available = None;
                }
                _ => {}
            }
            return None;
        }

        if key.code == KeyCode::F(1) {
            self.action_sender.send(Action::ToggleHelp).ok();
            return None;
        }

        if self.state.show_theme_popup {
            match key.code {
                KeyCode::Esc => {
                    self.state.show_theme_popup = false;
                }
                KeyCode::Up => {
                    let max = crate::tui::theme::AVAILABLE_THEMES.len().saturating_sub(1);
                    let i = match self.state.theme_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                max
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.state.theme_list_state.select(Some(i));
                    let selected_theme = crate::tui::theme::AVAILABLE_THEMES[i].to_string();
                    self.action_sender
                        .send(Action::SelectTheme(selected_theme))
                        .ok();
                }
                KeyCode::Down => {
                    let max = crate::tui::theme::AVAILABLE_THEMES.len().saturating_sub(1);
                    let i = match self.state.theme_list_state.selected() {
                        Some(i) => {
                            if i >= max {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.state.theme_list_state.select(Some(i));
                    let selected_theme = crate::tui::theme::AVAILABLE_THEMES[i].to_string();
                    self.action_sender
                        .send(Action::SelectTheme(selected_theme))
                        .ok();
                }
                KeyCode::Enter => {
                    self.state.show_theme_popup = false;
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
                    self.state.set_status(String::new(), 150);
                }
                KeyCode::Enter => {
                    let query = self.state.search_query.trim().to_string();
                    if !query.is_empty() {
                        let selected_suggestion = self.state.suggest_index.is_some();
                        self.state.input_mode = InputMode::Normal;
                        self.state.search_suggestions.clear();
                        self.state.suggest_index = None;
                        self.state.search_list_state.select(None);
                        self.state.last_search_edit = std::time::Instant::now();
                        let action = if selected_suggestion {
                            Action::SelectSuggestion { query }
                        } else {
                            Action::Search {
                                query,
                                force_refresh: false,
                            }
                        };
                        self.action_sender.send(action).ok();
                    }
                }
                KeyCode::Backspace => {
                    crate::tui::text::remove_last_grapheme(&mut self.state.search_query);
                    self.state.suggest_index = None;
                    self.state.last_search_edit = std::time::Instant::now();
                }
                KeyCode::Char('q') if self.state.search_query.is_empty() => {
                    self.action_sender.send(Action::Quit).ok();
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
                    if let Some(sug) = self.state.search_suggestions.get(next_idx) {
                        self.state.search_query = sug.clone();
                        self.state.last_suggest_query = self.state.search_query.trim().to_string();
                    }
                }
                KeyCode::Down if !self.state.search_suggestions.is_empty() => {
                    let max_idx = self.state.search_suggestions.len() - 1;
                    let next_idx = match self.state.suggest_index {
                        None => 0,
                        Some(i) if i == max_idx => 0,
                        Some(i) => i + 1,
                    };
                    self.state.suggest_index = Some(next_idx);
                    if let Some(sug) = self.state.search_suggestions.get(next_idx) {
                        self.state.search_query = sug.clone();
                        self.state.last_suggest_query = self.state.search_query.trim().to_string();
                    }
                }
                _ => {}
            },
            InputMode::Normal => match self.state.active_screen {
                Screen::Startup => {}
                Screen::Home => {
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
                    if self.state.browse_menu_open {
                        match key.code {
                            KeyCode::Esc => {
                                self.state.browse_menu_open = false;
                            }
                            KeyCode::Up => {
                                let max = crate::providers::browse::BrowseView::ALL.len() - 1;
                                let i = match self.state.browse_list_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            max
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.state.browse_list_state.select(Some(i));
                            }
                            KeyCode::Down => {
                                let max = crate::providers::browse::BrowseView::ALL.len() - 1;
                                let i = match self.state.browse_list_state.selected() {
                                    Some(i) => {
                                        if i >= max {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.state.browse_list_state.select(Some(i));
                            }
                            KeyCode::Enter => {
                                if let Some(idx) = self.state.browse_list_state.selected() {
                                    if let Some(view) =
                                        crate::providers::browse::BrowseView::ALL.get(idx).copied()
                                    {
                                        self.action_sender
                                            .send(Action::SelectBrowseView(view))
                                            .ok();
                                    }
                                }
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
                        KeyCode::Char('b') | KeyCode::Char('B')
                            if key.modifiers.contains(KeyModifiers::ALT)
                                && !self.state.is_tv_mode =>
                        {
                            self.action_sender.send(Action::ToggleBrowseMenu).ok();
                        }
                        KeyCode::Char('s') | KeyCode::Char('S')
                            if self.state.browse_view.is_some() && !self.state.is_tv_mode =>
                        {
                            self.action_sender.send(Action::ToggleBrowseSort).ok();
                        }
                        KeyCode::Enter => {
                            if self.state.search_results.is_empty()
                                && !self.state.search_query.trim().is_empty()
                                && (self.state.search_error.is_some()
                                    || self
                                        .state
                                        .status_message
                                        .to_ascii_lowercase()
                                        .starts_with("no matches"))
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
                            self.action_sender.send(Action::Refresh).ok();
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
                        KeyCode::Char(c)
                            if (key.modifiers.is_empty()
                                || key.modifiers == KeyModifiers::SHIFT) =>
                        {
                            self.state.input_mode = InputMode::Editing;
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
