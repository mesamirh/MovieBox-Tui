use super::App;
use crate::tui::{
    action::Action,
    state::{InputMode, Screen},
};
use crossterm::event::KeyEvent;

impl App {
    pub(super) async fn handle_key(&mut self, key: KeyEvent) -> Option<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.state.is_updating {
            return None;
        }
        if self.state.show_help {
            match key.code {
                KeyCode::Up => {
                    self.state.help_scroll = self.state.help_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.state.help_scroll = self.state.help_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    let rows = crossterm::terminal::size().map(|(_, r)| r).unwrap_or(24);
                    let page_size = rows.saturating_sub(6).max(4) as usize;
                    self.state.help_scroll = self.state.help_scroll.saturating_sub(page_size);
                }
                KeyCode::PageDown => {
                    let rows = crossterm::terminal::size().map(|(_, r)| r).unwrap_or(24);
                    let page_size = rows.saturating_sub(6).max(4) as usize;
                    self.state.help_scroll = self.state.help_scroll.saturating_add(page_size);
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                    self.state.show_help = false;
                    self.state.help_scroll = 0;
                }
                _ => {}
            }
            return None;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key.code {
                self.action_sender.send(Action::Quit).ok();
                return None;
            }
            if !self.state.has_active_modal() {
                if let KeyCode::Char('t') = key.code {
                    if !self.state.tv_enabled {
                        self.state
                            .set_status_short("TV Mode is disabled. Use /settings to enable.");
                    } else if self.state.is_tv_mode {
                        self.state.set_status_short("Already in TV Mode.");
                    } else {
                        self.action_sender.send(Action::SwitchToTvMode).ok();
                        self.state.set_status_short("Switched to TV Mode.");
                    }
                    return None;
                }
                if let KeyCode::Char('s') = key.code {
                    if !self.state.streaming_enabled {
                        self.state.set_status_short(
                            "Streaming Mode is disabled. Use /settings to enable.",
                        );
                    } else if !self.state.is_tv_mode
                        && self.state.active_provider
                            != crate::providers::models::ProviderKind::Addons
                    {
                        self.state.set_status_short("Already in Streaming Mode.");
                    } else {
                        self.action_sender.send(Action::SwitchToStreamingMode).ok();
                        self.state.set_status_short("Switched to Streaming Mode.");
                    }
                    return None;
                }
                if let KeyCode::Char('p') = key.code {
                    if !self.state.is_tv_mode {
                        self.cycle_provider();
                    }
                    return None;
                }
            }
        }

        // Esc closes a player that is still loading (a wrong file, an
        // unsupported 4K stream) so the next link can be tried at once.
        if key.code == KeyCode::Esc
            && self.state.is_playing
            && self.state.input_mode != InputMode::Editing
            && !self.state.tv_input_active
            && !self.state.addon_input_active
            && !self.state.has_active_modal()
        {
            match crate::player::request_playback_cancel() {
                crate::player::CancelRequest::Cancelled => {
                    self.state.notify(
                        crate::tui::overlay::NotificationKind::Info,
                        "Playback Cancelled",
                        "Closing the player.",
                    );
                    return None;
                }
                crate::player::CancelRequest::Armed => {
                    self.state
                        .set_status_short("Press Esc again to close the player.");
                    return None;
                }
                crate::player::CancelRequest::NoPlayer => {}
            }
        }

        if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
            && self.state.download_progress.is_some()
            && self.state.input_mode != InputMode::Editing
            && !self.state.tv_input_active
            && !self.state.addon_input_active
            && !self.state.has_active_modal()
        {
            self.action_sender.send(Action::CancelDownload).ok();
            return None;
        }

        if self.state.input_mode != InputMode::Editing {
            if let Some((version, _)) = &self.state.update_available {
                let env = std::env::current_exe()
                    .as_deref()
                    .map(crate::updater::apply::detect_environment)
                    .unwrap_or(crate::updater::apply::InstallationEnvironment::DirectReplace);

                match key.code {
                    KeyCode::Char('u') | KeyCode::Char('U')
                        if matches!(
                            env,
                            crate::updater::apply::InstallationEnvironment::DirectReplace
                                | crate::updater::apply::InstallationEnvironment::WindowsHelper
                        ) =>
                    {
                        self.action_sender.send(Action::StartSelfUpdate).ok();
                        return None;
                    }
                    KeyCode::Char('b') | KeyCode::Char('B')
                        if env == crate::updater::apply::InstallationEnvironment::Homebrew =>
                    {
                        self.state.notify(
                            crate::tui::overlay::NotificationKind::Info,
                            "Homebrew Update",
                            "Installed via Homebrew.",
                        );
                        self.state.update_available = None;
                        return None;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S')
                        if env == crate::updater::apply::InstallationEnvironment::Scoop =>
                    {
                        self.state.notify(
                            crate::tui::overlay::NotificationKind::Info,
                            "Scoop Update",
                            "Installed via Scoop.",
                        );
                        self.state.update_available = None;
                        return None;
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        let url = crate::updater::check::release_tag_url(version);
                        let _ = crate::net::open_external_url(&url);
                        self.state.update_available = None;
                        return None;
                    }
                    KeyCode::Esc => {
                        self.state.update_available = None;
                        return None;
                    }
                    _ => {}
                }
                return None;
            }
        }
        if self.state.show_provider_popup {
            let available = self.state.available_providers();
            let total_count = available.len();

            match key.code {
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state.show_provider_popup = false;
                    self.state.provider_list_state.select(None);
                    self.cycle_provider();
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.show_provider_popup = false;
                    self.state.provider_list_state.select(None);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.provider_list_state,
                        total_count,
                        false,
                    );
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.provider_list_state,
                        total_count,
                        true,
                    );
                }
                KeyCode::PageUp => {
                    let current = self.state.provider_list_state.selected().unwrap_or(0);
                    let next = if total_count == 0 {
                        0
                    } else if current < 5 {
                        total_count.saturating_sub(1)
                    } else {
                        current - 5
                    };
                    self.state.provider_list_state.select(Some(next));
                }
                KeyCode::PageDown => {
                    let current = self.state.provider_list_state.selected().unwrap_or(0);
                    let next = if total_count == 0 || current + 5 >= total_count {
                        0
                    } else {
                        current + 5
                    };
                    self.state.provider_list_state.select(Some(next));
                }
                KeyCode::Home => {
                    if total_count > 0 {
                        self.state.provider_list_state.select(Some(0));
                    }
                }
                KeyCode::End => {
                    if total_count > 0 {
                        self.state.provider_list_state.select(Some(total_count - 1));
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let index = self.state.provider_list_state.selected().unwrap_or(0);
                    if let Some(&provider) = available.get(index) {
                        self.switch_provider(provider);
                    }
                    self.state.show_provider_popup = false;
                    self.state.provider_list_state.select(None);
                }
                _ => {}
            }
            return None;
        }

        if self.state.show_browse_popup {
            let is_addon =
                self.state.active_provider == crate::providers::models::ProviderKind::Addons;
            let total_count = if is_addon {
                crate::providers::addons::models::curated_catalog_presets(
                    &self.state.installed_addons,
                )
                .len()
            } else {
                crate::tui::state::BrowsePreset::ALL.len()
            };

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.show_browse_popup = false;
                    self.state.browse_list_state.select(None);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        false,
                    );
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        true,
                    );
                }
                KeyCode::Home => {
                    if total_count > 0 {
                        self.state.browse_list_state.select(Some(0));
                    }
                }
                KeyCode::End => {
                    if total_count > 0 {
                        self.state.browse_list_state.select(Some(total_count - 1));
                    }
                }
                KeyCode::PageUp => {
                    crate::tui::state::step_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        -5,
                    );
                }
                KeyCode::PageDown => {
                    crate::tui::state::step_list_selection(
                        &mut self.state.browse_list_state,
                        total_count,
                        5,
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
        if self.state.player_picker_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.action_sender.send(Action::GoBack).ok();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.action_sender.send(Action::MoveUp).ok();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.action_sender.send(Action::MoveDown).ok();
                }
                KeyCode::Home => {
                    if !self.state.available_players.is_empty() {
                        self.state.player_picker_state.select(Some(0));
                    }
                }
                KeyCode::End => {
                    if !self.state.available_players.is_empty() {
                        let last = self.state.available_players.len().saturating_sub(1);
                        self.state.player_picker_state.select(Some(last));
                    }
                }
                KeyCode::PageUp => {
                    let len = self.state.available_players.len();
                    if len > 0 {
                        let current = self.state.player_picker_state.selected().unwrap_or(0);
                        let next = if current < 5 {
                            len.saturating_sub(1)
                        } else {
                            current - 5
                        };
                        self.state.player_picker_state.select(Some(next));
                    }
                }
                KeyCode::PageDown => {
                    let len = self.state.available_players.len();
                    if len > 0 {
                        let current = self.state.player_picker_state.selected().unwrap_or(0);
                        let next = if current + 5 >= len { 0 } else { current + 5 };
                        self.state.player_picker_state.select(Some(next));
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.action_sender.send(Action::Submit).ok();
                }
                _ => {}
            }
            return None;
        }

        if self.state.show_sources_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.show_sources_popup = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.sources_list_state,
                        crate::providers::models::ProviderKind::ENABLED.len(),
                        false,
                    );
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.sources_list_state,
                        crate::providers::models::ProviderKind::ENABLED.len(),
                        true,
                    );
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let idx = self.state.sources_list_state.selected().unwrap_or(0);
                    if let Some(&provider) =
                        crate::providers::models::ProviderKind::ENABLED.get(idx)
                    {
                        self.action_sender
                            .send(Action::ToggleProvider(provider))
                            .ok();
                    }
                }
                _ => {}
            }
            return None;
        }

        if self.state.show_theme_popup {
            let total = crate::tui::theme::AVAILABLE_THEMES.len();
            let mut changed = false;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.show_theme_popup = false;
                    if let Some(orig) = self.state.original_theme_kind.take() {
                        self.action_sender.send(Action::SelectTheme(orig)).ok();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.theme_list_state,
                        total,
                        false,
                    );
                    changed = true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    crate::tui::state::cycle_list_selection(
                        &mut self.state.theme_list_state,
                        total,
                        true,
                    );
                    changed = true;
                }
                KeyCode::Home => {
                    if total > 0 {
                        self.state.theme_list_state.select(Some(0));
                        changed = true;
                    }
                }
                KeyCode::End => {
                    if total > 0 {
                        self.state.theme_list_state.select(Some(total - 1));
                        changed = true;
                    }
                }
                KeyCode::PageUp => {
                    crate::tui::state::step_list_selection(
                        &mut self.state.theme_list_state,
                        total,
                        -5,
                    );
                    changed = true;
                }
                KeyCode::PageDown => {
                    crate::tui::state::step_list_selection(
                        &mut self.state.theme_list_state,
                        total,
                        5,
                    );
                    changed = true;
                }
                KeyCode::Enter => {
                    self.state.show_theme_popup = false;
                    self.state.original_theme_kind = None;
                    self.persist_config();
                }
                _ => {}
            }
            if changed {
                if let Some(i) = self.state.theme_list_state.selected() {
                    let selected_theme = crate::tui::theme::AVAILABLE_THEMES[i].to_string();
                    self.action_sender
                        .send(Action::SelectTheme(selected_theme))
                        .ok();
                }
            }
            return None;
        }
        if self.state.show_settings_popup {
            if let Some(input) = &mut self.state.settings_download_dir_input {
                match key.code {
                    KeyCode::Esc => {
                        self.state.settings_download_dir_input = None;
                    }
                    KeyCode::Enter => {
                        self.action_sender.send(Action::SettingsActivateRow).ok();
                    }
                    _ => {
                        input.handle_key(key);
                    }
                }
                return None;
            }

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.state.show_settings_popup = false;
                    self.state.settings_download_dir_input = None;
                    self.persist_config();
                }
                KeyCode::Tab => {
                    self.state.settings_next_category();
                }
                KeyCode::BackTab => {
                    self.state.settings_previous_category();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.state.settings_row_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.state.settings_row_down();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.action_sender
                        .send(Action::SettingsAdjustValue(false))
                        .ok();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.action_sender
                        .send(Action::SettingsAdjustValue(true))
                        .ok();
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.action_sender.send(Action::SettingsActivateRow).ok();
                }
                KeyCode::Char('d') | KeyCode::Char('D')
                    if self.state.settings_category
                        == crate::tui::state::SettingsCategory::General
                        && self.state.settings_selected_row == 2 =>
                {
                    self.action_sender
                        .send(Action::SettingsResetDownloadDir)
                        .ok();
                }
                _ => {}
            }
            return None;
        }

        if self.state.input_mode == InputMode::Normal
            && self.state.active_screen == Screen::Home
            && key.code == KeyCode::Backspace
            && !self.state.addon_manager_popup
            && !self.state.tv_config_popup
        {
            self.state.input_mode = InputMode::Editing;
            self.state.favorites_focus = false;
            self.state.favorites_landing_state.select(None);
            self.state.search_suggestions.clear();
            self.state.suggest_index = None;
            self.state.set_status_default("");
            self.state.last_search_edit = std::time::Instant::now();
            return None;
        }

        match self.state.input_mode {
            InputMode::Editing => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char('u') = key.code {
                        let had_search = !self.state.search_query.trim().is_empty()
                            || !self.state.search_results.is_empty();
                        self.state.clear_search_state();
                        self.state.input_mode = InputMode::Normal;
                        if had_search {
                            self.state.set_status_default("Search cleared.");
                        }
                        return None;
                    }
                    if let KeyCode::Char('w') = key.code {
                        self.state.search_query.delete_word_backwards();
                        self.state.suggest_index = None;
                        self.state.search_suggestions.clear();
                        self.state.last_search_edit = std::time::Instant::now();
                        return None;
                    }
                }
                match key.code {
                    KeyCode::Esc => {
                        self.state.input_mode = InputMode::Normal;
                        self.state.suggest_index = None;
                        self.state.search_suggestions.clear();
                        self.state.clear_search_state();
                        self.state.status_message.clear();
                        self.state.status_timer = 0;
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
                                self.state.search_query.set_content("/history");
                            } else if query.starts_with('/') {
                                self.state.search_query.clear();
                            } else {
                                self.state.search_query.set_content(&query);
                            }
                            self.state.input_mode = InputMode::Normal;
                            if !query.starts_with('/') {
                                self.state.is_loading = true;
                                self.state.has_search_settled = false;
                                self.state.search_error = None;
                            }
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
                        } else {
                            self.state.input_mode = InputMode::Normal;
                            self.state.search_suggestions.clear();
                            self.state.suggest_index = None;
                        }
                    }
                    KeyCode::Tab => {
                        let trimmed = self.state.search_query.trim();
                        let selected_or_first = if trimmed.starts_with('/') {
                            self.state
                                .suggest_index
                                .and_then(|idx| self.state.search_suggestions.get(idx).cloned())
                                .or_else(|| {
                                    let suggestions = crate::tui::commands::SlashCommand::suggest(
                                        &self.state,
                                        trimmed,
                                    );
                                    suggestions.first().cloned()
                                })
                        } else {
                            self.state
                                .suggest_index
                                .and_then(|idx| self.state.search_suggestions.get(idx).cloned())
                                .or_else(|| self.state.search_suggestions.first().cloned())
                        };
                        if let Some(sug) = selected_or_first {
                            self.state.search_query.set_content(&sug);
                            self.state.last_search_edit = std::time::Instant::now();
                        }
                    }
                    KeyCode::Left => {
                        self.state.search_query.move_left();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::Right => {
                        self.state.search_query.move_right();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::Home => {
                        self.state.search_query.move_home();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::End => {
                        self.state.search_query.move_end();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::Backspace => {
                        self.state.search_query.delete_backwards();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::Delete => {
                        self.state.search_query.delete_forwards();
                        self.state.suggest_index = None;
                        self.state.last_search_edit = std::time::Instant::now();
                    }
                    KeyCode::Char(c) => {
                        self.state.search_query.insert(c);
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
                }
            }
            InputMode::Normal => match self.state.active_screen {
                Screen::Home => {
                    if self.state.addon_manager_popup {
                        if self.state.addon_input_active {
                            match key.code {
                                KeyCode::Esc => {
                                    self.state.addon_input_active = false;
                                    self.state.addon_input_buffer.clear();
                                }
                                KeyCode::Enter => {
                                    let buffer =
                                        self.state.addon_input_buffer.as_str().trim().to_string();
                                    self.state.addon_input_active = false;
                                    self.state.addon_input_buffer.clear();
                                    if !buffer.is_empty() {
                                        self.action_sender
                                            .send(Action::AddonAddManifest(buffer))
                                            .ok();
                                    }
                                }
                                _ => {
                                    self.state.addon_input_buffer.handle_key(key);
                                }
                            }
                            return None;
                        }
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                self.reset_transient_overlays();
                                self.state.addon_manager_popup = false;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.state.step_addon_manager_selected(-1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.state.step_addon_manager_selected(1);
                            }
                            KeyCode::Home => {
                                self.state.first_addon_manager_selected();
                            }
                            KeyCode::End => {
                                self.state.last_addon_manager_selected();
                            }
                            KeyCode::PageUp => {
                                self.state.step_addon_manager_selected(-5);
                            }
                            KeyCode::PageDown => {
                                self.state.step_addon_manager_selected(5);
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
                                    let buffer =
                                        self.state.tv_input_buffer.as_str().trim().to_string();
                                    self.state.tv_input_active = false;
                                    self.state.tv_input_buffer.clear();
                                    if !buffer.is_empty() {
                                        self.action_sender.send(Action::TvPlaylistAdd(buffer)).ok();
                                    }
                                }
                                _ => {
                                    self.state.tv_input_buffer.handle_key(key);
                                }
                            }
                            return None;
                        }
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                self.reset_transient_overlays();
                                self.state.tv_config_popup = false;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.state.step_tv_manager_selected(-1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.state.step_tv_manager_selected(1);
                            }
                            KeyCode::Home => {
                                self.state.first_tv_manager_selected();
                            }
                            KeyCode::End => {
                                self.state.last_tv_manager_selected();
                            }
                            KeyCode::PageUp => {
                                self.state.step_tv_manager_selected(-5);
                            }
                            KeyCode::PageDown => {
                                self.state.step_tv_manager_selected(5);
                            }
                            KeyCode::Char('r') => {
                                self.action_sender.send(Action::TvReloadPlaylists).ok();
                            }
                            KeyCode::Char('d') | KeyCode::Delete => {
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
                            KeyCode::Enter | KeyCode::Char(' ') => {
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
                        KeyCode::Tab | KeyCode::BackTab => {
                            if self.state.landing_deck_visible() {
                                self.state.cycle_home_deck_tab();
                            }
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            if self.state.favorites_focus {
                                self.state.favorites_landing_state.select(Some(0));
                            } else if !self.state.search_results.is_empty() {
                                self.state.search_list_state.select(Some(0));
                                if let Some(res) = self.state.search_results.first() {
                                    self.action_sender
                                        .send(Action::FetchPreview(res.id.clone()))
                                        .ok();
                                }
                                self.prefetch_visible_posters();
                                self.state.normalize_result_view();
                            }
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            if self.state.favorites_focus {
                                let total = self.state.landing_deck_items_count();
                                if total > 0 {
                                    self.state.favorites_landing_state.select(Some(total - 1));
                                }
                            } else if !self.state.search_results.is_empty() {
                                let last = self.state.search_results.len().saturating_sub(1);
                                self.state.search_list_state.select(Some(last));
                                if let Some(res) = self.state.search_results.get(last) {
                                    self.action_sender
                                        .send(Action::FetchPreview(res.id.clone()))
                                        .ok();
                                }
                                self.prefetch_visible_posters();
                                self.state.normalize_result_view();
                                self.trigger_next_page_if_needed();
                            }
                        }
                        KeyCode::PageDown => {
                            if self.state.favorites_focus {
                                let total = self.state.landing_deck_items_count();
                                crate::tui::state::step_list_selection(
                                    &mut self.state.favorites_landing_state,
                                    total,
                                    5,
                                );
                            } else if !self.state.search_results.is_empty() {
                                let step = self
                                    .state
                                    .last_result_metrics
                                    .map(|m| m.visible_items)
                                    .unwrap_or(8);
                                let cur = self.state.search_list_state.selected().unwrap_or(0);
                                let target = (cur + step)
                                    .min(self.state.search_results.len().saturating_sub(1));
                                self.state.search_list_state.select(Some(target));
                                if let Some(res) = self.state.search_results.get(target) {
                                    self.action_sender
                                        .send(Action::FetchPreview(res.id.clone()))
                                        .ok();
                                }
                                self.prefetch_visible_posters();
                                self.state.normalize_result_view();
                                self.trigger_next_page_if_needed();
                            }
                        }
                        KeyCode::PageUp => {
                            if self.state.favorites_focus {
                                let total = self.state.landing_deck_items_count();
                                crate::tui::state::step_list_selection(
                                    &mut self.state.favorites_landing_state,
                                    total,
                                    -5,
                                );
                            } else if !self.state.search_results.is_empty() {
                                let step = self
                                    .state
                                    .last_result_metrics
                                    .map(|m| m.visible_items)
                                    .unwrap_or(8);
                                let cur = self.state.search_list_state.selected().unwrap_or(0);
                                let target = cur.saturating_sub(step);
                                self.state.search_list_state.select(Some(target));
                                if let Some(res) = self.state.search_results.get(target) {
                                    self.action_sender
                                        .send(Action::FetchPreview(res.id.clone()))
                                        .ok();
                                }
                                self.prefetch_visible_posters();
                                self.state.normalize_result_view();
                            }
                        }
                        KeyCode::Enter => {
                            if self.state.search_results.is_empty()
                                && !self.state.search_query.trim().is_empty()
                            {
                                self.state.is_loading = true;
                                self.state.has_search_settled = false;
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
                        KeyCode::Char('c') | KeyCode::Char('C')
                            if self.state.download_progress.is_none() =>
                        {
                            let had_search = !self.state.search_query.trim().is_empty()
                                || !self.state.search_results.is_empty();
                            self.state.clear_search_state();
                            self.state.input_mode = InputMode::Normal;
                            if had_search {
                                self.state.set_status_default("Search cleared.");
                            }
                        }
                        KeyCode::Char('f') | KeyCode::Char('F')
                            if self.state.favorites_available()
                                && (!self.state.search_results.is_empty()
                                    || self.state.favorites_focus) =>
                        {
                            self.action_sender.send(Action::ToggleFavorite).ok();
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete
                            if (self
                                .state
                                .search_query
                                .trim()
                                .eq_ignore_ascii_case("/history")
                                && !self.state.search_results.is_empty())
                                || (self.state.favorites_focus
                                    && self.state.effective_home_deck_tab()
                                        == crate::tui::state::HomeDeckTab::ContinueWatching) =>
                        {
                            if self.state.favorites_focus {
                                if let Some(idx) = self.state.favorites_landing_state.selected() {
                                    let target =
                                        self.state.continue_watching_items().get(idx).map(|item| {
                                            (
                                                item.title.clone(),
                                                item.provider.clone(),
                                                item.subject_id.clone(),
                                                item.season,
                                                item.episode,
                                            )
                                        });
                                    if let Some((title, provider, subject_id, season, episode)) =
                                        target
                                    {
                                        self.state.history.remove(
                                            &provider,
                                            &subject_id,
                                            season,
                                            episode,
                                        );
                                        self.state.homepage_cache.clear();
                                        let total = self.state.landing_deck_items_count();
                                        if total == 0 {
                                            self.state.favorites_focus = false;
                                            self.state.favorites_landing_state.select(None);
                                        } else if idx >= total {
                                            self.state
                                                .favorites_landing_state
                                                .select(Some(total.saturating_sub(1)));
                                        }
                                        self.state.notify(
                                            crate::tui::overlay::NotificationKind::Info,
                                            "History",
                                            format!("Removed: {title}"),
                                        );
                                    }
                                }
                            } else if let Some(idx) = self.state.search_list_state.selected() {
                                if idx < self.state.search_results.len() {
                                    let res = self.state.search_results.remove(idx);
                                    let provider_key = res.provider.cache_key();
                                    self.state.history.remove(
                                        provider_key,
                                        &res.id,
                                        res.season,
                                        res.episode,
                                    );
                                    self.state.history.recent.retain(|i| {
                                        !(i.subject_id == res.id
                                            && (crate::providers::models::ProviderKind::parse(
                                                &i.provider,
                                            ) == Some(res.provider)
                                                || i.provider.eq_ignore_ascii_case(provider_key)))
                                    });
                                    self.state.history.save();
                                    self.state.homepage_cache.clear();
                                    let total = self.state.search_results.len();
                                    if total == 0 {
                                        self.state.search_list_state.select(None);
                                    } else if idx >= total {
                                        self.state
                                            .search_list_state
                                            .select(Some(total.saturating_sub(1)));
                                    }
                                    self.state.notify(
                                        crate::tui::overlay::NotificationKind::Info,
                                        "History",
                                        format!("Removed: {}", res.title),
                                    );
                                }
                            }
                        }
                        KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P')
                            if (self
                                .state
                                .search_query
                                .trim()
                                .eq_ignore_ascii_case("/history")
                                && !self.state.search_results.is_empty())
                                || (self.state.favorites_focus
                                    && self.state.effective_home_deck_tab()
                                        == crate::tui::state::HomeDeckTab::ContinueWatching) =>
                        {
                            if self.state.favorites_focus {
                                if let Some(idx) = self.state.favorites_landing_state.selected() {
                                    self.action_sender
                                        .send(Action::OpenContinueWatching(idx))
                                        .ok();
                                }
                            } else {
                                self.resume_history_playback();
                            }
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
                            self.state.search_query.insert(c);

                            self.state.search_suggestions.clear();
                            self.state.suggest_index = None;
                            self.state.set_status_default("");
                            self.state.last_search_edit = std::time::Instant::now();
                        }
                        _ => {}
                    }
                }
                Screen::Details => {
                    if self.state.show_overview_modal {
                        match key.code {
                            KeyCode::Esc
                            | KeyCode::Char('q')
                            | KeyCode::Char('Q')
                            | KeyCode::Char('i')
                            | KeyCode::Char('I')
                            | KeyCode::Enter => {
                                self.state.close_overview_modal();
                            }
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                                self.state.overview_modal_scroll =
                                    self.state.overview_modal_scroll.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                                self.state.overview_modal_scroll =
                                    self.state.overview_modal_scroll.saturating_add(1);
                            }
                            KeyCode::PageUp => {
                                self.state.overview_modal_scroll =
                                    self.state.overview_modal_scroll.saturating_sub(5);
                            }
                            KeyCode::PageDown => {
                                self.state.overview_modal_scroll =
                                    self.state.overview_modal_scroll.saturating_add(5);
                            }
                            _ => {}
                        }
                        return None;
                    }
                    match key.code {
                        KeyCode::Tab => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::TabPane).ok();
                            }
                        }
                        KeyCode::BackTab => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::BackTabPane).ok();
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            self.action_sender.send(Action::GoBack).ok();
                        }
                        KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                match self.state.details_pane {
                                    crate::tui::state::DetailsPane::Streams => {
                                        self.action_sender.send(Action::PlayStream).ok();
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
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                if let crate::tui::state::DetailsPane::Seasons =
                                    self.state.details_pane
                                {
                                    if !self.state.available_seasons.is_empty() {
                                        self.action_sender.send(Action::DownloadSeason).ok();
                                    }
                                } else {
                                    self.action_sender.send(Action::DownloadEpisode).ok();
                                }
                            }
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                if let Some((title, content)) = self.state.active_overview() {
                                    self.state.open_overview_modal(title, content);
                                }
                            }
                        }
                        KeyCode::Char('r') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::Refresh).ok();
                            }
                        }
                        KeyCode::Char('?') => {
                            self.action_sender.send(Action::ToggleHelp).ok();
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                                && self.state.favorites_available()
                            {
                                self.action_sender.send(Action::ToggleFavorite).ok();
                            }
                        }

                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            self.action_sender.send(Action::MoveUp).ok();
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            self.action_sender.send(Action::MoveDown).ok();
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::BackTabPane).ok();
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                            if !self.state.subtitle_popup
                                && !self.state.player_picker_popup
                                && !self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::TabPane).ok();
                            }
                        }
                        KeyCode::Enter => {
                            if self.state.subtitle_popup
                                || self.state.player_picker_popup
                                || self.state.is_download_subtitle_popup
                            {
                                self.action_sender.send(Action::Submit).ok();
                            } else {
                                match self.state.details_pane {
                                    crate::tui::state::DetailsPane::Streams => {
                                        self.action_sender.send(Action::PlayStream).ok();
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
                    }
                }
            },
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[tokio::test]
    async fn test_normal_mode_jump_navigation() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.search_results = vec![
            crate::models::SearchResult {
                id: "1".to_string(),
                title: "Movie 1".to_string(),
                stype: 1,
                release_year: "2020".to_string(),
                provider: crate::models::ProviderKind::MovieBox,
                cover_url: None,
                season: 0,
                episode: 0,
            },
            crate::models::SearchResult {
                id: "2".to_string(),
                title: "Movie 2".to_string(),
                stype: 1,
                release_year: "2021".to_string(),
                provider: crate::models::ProviderKind::MovieBox,
                cover_url: None,
                season: 0,
                episode: 0,
            },
            crate::models::SearchResult {
                id: "3".to_string(),
                title: "Movie 3".to_string(),
                stype: 1,
                release_year: "2022".to_string(),
                provider: crate::models::ProviderKind::MovieBox,
                cover_url: None,
                season: 0,
                episode: 0,
            },
        ];
        app.state.search_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(2));

        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(2));
        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(0));
    }

    #[tokio::test]
    async fn test_delete_history_item_with_key() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.search_query.set_content("/history");
        app.state.history.recent.clear();
        let item = crate::history::WatchHistoryItem {
            provider: "moviebox".to_string(),
            subject_id: "hist-1".to_string(),
            title: "History Movie".to_string(),
            cover_url: None,
            stype: 1,
            release_year: "2024".to_string(),
            season: 0,
            episode: 0,
            progress_seconds: 50,
            duration_seconds: Some(500),
            completed: false,
            timestamp: 1000,
            stream_filename: None,
        };
        app.state.history.record_start(&item, 50);
        app.state.search_results.push(item.to_search_result());
        app.state.search_list_state.select(Some(0));
        assert_eq!(app.state.search_results.len(), 1);
        assert_eq!(app.state.history.recent.len(), 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()))
            .await;

        assert!(app.state.search_results.is_empty());
        assert!(app.state.history.recent.is_empty());
        let notif = app
            .state
            .notifications
            .back()
            .expect("notification emitted");
        assert_eq!(notif.title, "History");
        assert_eq!(notif.message, "Removed: History Movie");
    }

    #[tokio::test]
    async fn test_update_modal_keystroke_isolation() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.update_available = Some(("2.0.0".to_string(), "Notes".to_string()));
        app.state.search_results = vec![
            crate::models::SearchResult {
                id: "1".to_string(),
                title: "Movie 1".to_string(),
                stype: 1,
                release_year: "2020".to_string(),
                provider: crate::models::ProviderKind::MovieBox,
                cover_url: None,
                season: 0,
                episode: 0,
            },
            crate::models::SearchResult {
                id: "2".to_string(),
                title: "Movie 2".to_string(),
                stype: 1,
                release_year: "2021".to_string(),
                provider: crate::models::ProviderKind::MovieBox,
                cover_url: None,
                season: 0,
                episode: 0,
            },
        ];
        app.state.search_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(0));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        assert!(app.state.update_available.is_none());
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.search_list_state.selected(), Some(1));
    }
    #[tokio::test]
    async fn test_normal_mode_c_clears_search_and_results() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.search_query.set_content("Inception");
        app.state.search_results = vec![crate::models::SearchResult {
            id: "1".to_string(),
            title: "Inception".to_string(),
            stype: 1,
            release_year: "2010".to_string(),
            provider: crate::models::ProviderKind::MovieBox,
            cover_url: None,
            season: 0,
            episode: 0,
        }];
        app.state.search_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()))
            .await;

        assert!(app.state.search_query.is_empty());
        assert!(app.state.search_results.is_empty());
        assert_eq!(app.state.search_list_state.selected(), None);
    }

    #[tokio::test]
    async fn test_search_results_multiple_esc_returns_to_homepage() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.search_query.set_content("Deewaniyat");
        app.state.search_results = vec![crate::models::SearchResult {
            id: "1".to_string(),
            title: "Deewaniyat".to_string(),
            stype: 1,
            release_year: "2024".to_string(),
            provider: crate::models::ProviderKind::MovieBox,
            cover_url: None,
            season: 0,
            episode: 0,
        }];
        app.state.search_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        while let Ok(action) = app.action_receiver.try_recv() {
            app.handle_action(action).await;
        }
        assert_eq!(app.state.input_mode, InputMode::Editing);
        assert_eq!(app.state.search_query.as_str(), "Deewaniyat");
        assert!(!app.state.search_results.is_empty());

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        while let Ok(action) = app.action_receiver.try_recv() {
            app.handle_action(action).await;
        }
        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(app.state.search_query.is_empty());
        assert!(app.state.search_results.is_empty());
        assert_eq!(app.state.search_list_state.selected(), None);
    }

    #[tokio::test]
    async fn test_browse_esc_returns_to_homepage() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.active_browse_preset = Some(crate::models::BrowsePreset::Trending);
        app.state.search_results = vec![crate::models::SearchResult {
            id: "1".to_string(),
            title: "Awarapan 2".to_string(),
            stype: 1,
            release_year: "2026".to_string(),
            provider: crate::models::ProviderKind::MovieBox,
            cover_url: None,
            season: 0,
            episode: 0,
        }];
        app.state.search_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        while let Ok(action) = app.action_receiver.try_recv() {
            app.handle_action(action).await;
        }

        assert!(app.state.active_browse_preset.is_none());
        assert!(app.state.search_results.is_empty());
        assert_eq!(app.state.search_list_state.selected(), None);
        assert_eq!(app.state.input_mode, InputMode::Normal);
    }
    #[tokio::test]
    async fn test_provider_popup_keyboard_navigation() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.is_tv_mode = false;
        app.state.active_provider = crate::models::ProviderKind::MovieBox;
        app.state.show_provider_popup = true;
        app.state.provider_list_state.select(Some(0));

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.provider_list_state.selected(), Some(1));

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_provider_popup);
        let expected = app.state.available_providers()[1];
        assert_eq!(app.state.active_provider, expected);

        app.state.show_provider_popup = true;
        app.state.provider_list_state.select(Some(0));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_provider_popup);
        assert_eq!(app.state.provider_list_state.selected(), None);

        app.state.show_provider_popup = true;
        app.state.provider_list_state.select(Some(0));
        let before_provider = app.state.active_provider;
        app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL))
            .await;
        assert!(!app.state.show_provider_popup);
        assert_ne!(app.state.active_provider, before_provider);
    }

    #[tokio::test]
    async fn test_empty_search_clear_does_not_set_status() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Home;
        app.state.input_mode = InputMode::Normal;
        app.state.search_query.clear();
        app.state.search_results.clear();

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.status_message, "");

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        while let Ok(action) = app.action_receiver.try_recv() {
            app.handle_action(action).await;
        }
        assert_eq!(app.state.status_message, "");

        app.state.input_mode = InputMode::Editing;
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert_eq!(app.state.status_message, "");

        app.state.search_query.set_content("inception");
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.status_message, "Search cleared.");
    }

    #[tokio::test]
    async fn test_details_overview_modal_keyboard_toggle_and_scroll() {
        let mut app = App::new();
        app.state.active_screen = crate::tui::state::Screen::Details;
        app.state.input_mode = InputMode::Normal;
        app.state.selected_details = Some(crate::providers::models::MediaDetails {
            id: crate::providers::models::ProviderMediaId {
                provider: crate::providers::models::ProviderKind::MovieBox,
                value: "movie_1".to_string(),
            },
            title: "Interstellar".to_string(),
            media_type: crate::providers::models::MediaType::Movie,
            year: Some("2014".to_string()),
            description: Some("Space exploration epic.".to_string()),
            tagline: None,
            imdb_rating: None,
            director: None,
            stars: None,
            prints: None,
            audios: None,
            poster_url: None,
            duration: None,
            genres: vec![],
            seasons: vec![],
            dubs: vec![],
        });

        assert!(!app.state.show_overview_modal);

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()))
            .await;
        assert!(app.state.show_overview_modal);
        assert_eq!(app.state.overview_modal_title, "Interstellar · Synopsis");
        assert_eq!(app.state.overview_modal_content, "Space exploration epic.");
        assert_eq!(app.state.overview_modal_scroll, 0);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.overview_modal_scroll, 1);

        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.overview_modal_scroll, 6);

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()))
            .await;
        assert_eq!(app.state.overview_modal_scroll, 5);

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_overview_modal);

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()))
            .await;
        assert!(app.state.show_overview_modal);

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_overview_modal);
    }

    #[tokio::test]
    async fn test_details_download_key_triggers_download_directly() {
        let mut app = App::new();
        app.state.active_screen = Screen::Details;
        app.state.details_pane = crate::tui::state::DetailsPane::Streams;
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()))
            .await;
        assert!(matches!(
            app.action_receiver.try_recv().ok(),
            Some(Action::DownloadEpisode)
        ));

        app.state.details_pane = crate::tui::state::DetailsPane::Seasons;
        app.state.available_seasons = vec![crate::providers::models::Season {
            number: 1,
            episodes: vec![],
        }];
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()))
            .await;
        assert!(matches!(
            app.action_receiver.try_recv().ok(),
            Some(Action::DownloadSeason)
        ));
    }
    #[tokio::test]
    async fn test_details_q_triggers_go_back() {
        let mut app = App::new();
        app.state.active_screen = Screen::Details;
        app.state.details_pane = crate::tui::state::DetailsPane::Streams;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()))
            .await;
        assert!(matches!(
            app.action_receiver.try_recv().ok(),
            Some(Action::GoBack)
        ));
    }

    #[tokio::test]
    async fn test_popups_dismiss_on_q() {
        let mut app = App::new();
        app.state.show_provider_popup = true;
        app.state.provider_list_state.select(Some(0));
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_provider_popup);
        assert_eq!(app.state.provider_list_state.selected(), None);

        app.state.show_settings_popup = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()))
            .await;
        assert!(!app.state.show_settings_popup);
    }
}
