use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::{action::Action, event::EventHandler, state::Screen};
use ratatui::Frame;
use std::time::Duration;

impl App {
    pub async fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> std::io::Result<()>
    where
        std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
    {
        if self.state.image_picker.is_none() && self.state.image_supported {
            match ratatui_image::picker::Picker::from_query_stdio() {
                Ok(picker) => {
                    let cell_h = picker.font_size().height;
                    if cell_h > 0 {
                        self.state.poster_rows = (96_u16.div_ceil(cell_h)).max(3);
                    }
                    self.state.image_picker = Some(picker);
                }
                Err(_) => {
                    self.state.image_supported = false;
                }
            }
        }

        let mut events = EventHandler::new(Duration::from_millis(100));

        if self.state.active_provider == ProviderKind::MovieBox {
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.init().await;
            });
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.state.auto_update && now.saturating_sub(self.state.last_update_check) > 3600 {
            self.state.last_update_check = now;
            self.state.manual_update_check = false;
            self.persist_config();
            self.action_sender.send(Action::CheckForUpdates).ok();
        }
        self.state.active_screen = Screen::Home;

        self.state.available_players = crate::tui::player::detect();
        let preferred = std::env::var("MOVIEBOX_PLAYER")
            .ok()
            .and_then(|value| crate::tui::state::PlayerKind::parse(&value))
            .or_else(|| {
                self.state
                    .default_player
                    .as_deref()
                    .and_then(crate::tui::state::PlayerKind::parse)
            });
        if let Some(preferred) = preferred
            && let Some(index) = self
                .state
                .available_players
                .iter()
                .position(|&k| k == preferred)
        {
            let kind = self.state.available_players.remove(index);
            self.state.available_players.insert(0, kind);
        }

        loop {
            if self.state.clear_terminal_before_draw {
                terminal.current_buffer_mut().reset();
                terminal.backend_mut().clear()?;
                terminal.swap_buffers();
                self.state.clear_terminal_before_draw = false;
                self.state.dirty = true;
            }
            if self.state.dirty {
                terminal.draw(|frame| self.draw(frame))?;
                self.state.dirty = false;
            }

            tokio::select! {
                Some(action) = events.next() => {
                    if let Some(quit) = self.handle_action(action).await {
                        return Ok(quit);
                    }
                }
                Some(action) = self.action_receiver.recv() => {
                    if let Some(quit) = self.handle_action(action).await {
                        return Ok(quit);
                    }
                }
            }
        }
    }

    async fn handle_action(&mut self, action: Action) -> Option<()> {
        if !matches!(action, Action::Tick | Action::UpdateDownload(..)) {
            self.state.dirty = true;
        }
        match action {
            Action::Quit => {
                return Some(());
            }

            Action::Key(key) => {
                self.handle_key(key).await;
            }

            Action::Tick
            | Action::FocusChange
            | Action::Resize(..)
            | Action::SwitchProvider(..)
            | Action::ToggleHelp
            | Action::Refresh
            | Action::ClearCache
            | Action::ToggleThemePopup
            | Action::SelectTheme(..)
            | Action::SetStatus(..)
            | Action::CheckForUpdates
            | Action::UpdateAvailable(..) => {
                self.handle_system(action).await;
            }

            Action::ToggleTvMode
            | Action::ShowTvConfig
            | Action::TvPlaylistAdd(..)
            | Action::TvPlaylistRemove(..)
            | Action::TvReloadPlaylists
            | Action::TvInputToggle(..)
            | Action::TvChannelsLoaded(..) => {
                self.handle_tv(action).await;
            }

            Action::MoveUp
            | Action::MoveDown
            | Action::MoveLeft
            | Action::MoveRight
            | Action::Submit
            | Action::GoBack
            | Action::TabPane
            | Action::BackTabPane
            | Action::SelectLanguage(..) => {
                self.handle_navigation(action).await;
            }

            Action::Suggest(..)
            | Action::SuggestSuccess(..)
            | Action::SelectSuggestion { .. }
            | Action::Search { .. }
            | Action::FetchHomepage { .. }
            | Action::SearchSuccess { .. }
            | Action::SearchFailure(..)
            | Action::HomepageSuccess { .. }
            | Action::HomepageFailure(..)
            | Action::ToggleBrowseMenu
            | Action::SelectBrowseView(..)
            | Action::ToggleBrowseSort
            | Action::FetchBrowse { .. }
            | Action::BrowseSuccess { .. }
            | Action::BrowseFailure(..)
            | Action::FetchDetails(..)
            | Action::DetailsSuccess(..)
            | Action::DetailsFailure(..)
            | Action::FetchPreview(..)
            | Action::PreviewSuccess(..)
            | Action::PreviewFailure(..)
            | Action::FetchEpisodeStreams { .. }
            | Action::EpisodeStreamsReady(..)
            | Action::EpisodeStreamsFailed(..)
            | Action::InitStreamPool(..)
            | Action::StreamPoolInitialized(..)
            | Action::PosterSuccess(..)
            | Action::SearchPosterLoaded(..) => {
                self.handle_requests(action).await;
            }

            Action::PlayStream(..)
            | Action::ShowSubtitlePopup(..)
            | Action::ShowDownloadSubtitlePopup(..)
            | Action::ShowPlaybackPicker(..)
            | Action::ShowPlayerPicker(..)
            | Action::LaunchMpv(..)
            | Action::LaunchPlayback(..)
            | Action::LaunchPlayer(..)
            | Action::PlayerCrashed(..) => {
                self.handle_playback(action).await;
            }

            Action::DownloadStream(..)
            | Action::StartDownload(..)
            | Action::PromptDownloadEpisode
            | Action::ConfirmDownloadEpisode
            | Action::PromptDownloadSeason
            | Action::ConfirmDownloadSeason
            | Action::ProcessDownloadQueue
            | Action::UpdateDownload(..)
            | Action::DownloadCompleted(..)
            | Action::DownloadFailed(..)
            | Action::DownloadPaused(..)
            | Action::ClearDownload
            | Action::CancelDownload => {
                self.handle_download(action).await;
            }
        }
        None
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if area.width < 85 || area.height < 24 {
            use ratatui::layout::Alignment;
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let msg_lines = vec![
                Line::from(format!(
                    "Terminal too small ({}x{}).",
                    area.width, area.height
                )),
                Line::from("Minimum required size: 85x24"),
                Line::from("Please enlarge your terminal window."),
            ];

            let padding_top = area.height.saturating_sub(2).saturating_sub(3) / 2;
            let mut msg = Vec::new();
            for _ in 0..padding_top {
                msg.push(Line::from(""));
            }
            msg.extend(msg_lines);

            let p = Paragraph::new(msg)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(self.theme.border),
                )
                .alignment(Alignment::Center);

            frame.render_widget(p, area);
            return;
        }

        let mut main_area = frame.area();
        let mut download_area = None;

        if self.state.download_progress.is_some() {
            use ratatui::layout::{Constraint, Direction, Layout};
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(main_area);

            main_area = chunks[0];
            download_area = Some(chunks[1]);
        }

        match self.state.active_screen {
            Screen::Startup => {
                crate::tui::screens::startup::draw(frame, main_area, &mut self.state, &self.theme);
            }
            Screen::Home => {
                crate::tui::screens::home::draw(frame, main_area, &mut self.state, &self.theme);
            }
            Screen::Details => {
                crate::tui::screens::details::draw(frame, main_area, &mut self.state, &self.theme);
            }
        }

        if self.state.show_help {
            crate::tui::screens::help::draw(frame, main_area, &self.state, &self.theme);
        }
        if let Some(prog) = self.state.download_progress {
            if let Some(dl_area) = download_area {
                use ratatui::widgets::{Block, Borders, Gauge};

                let status = self
                    .state
                    .download_status
                    .as_deref()
                    .unwrap_or("Downloading...");

                let title_text = if self.state.download_queue_total > 0 {
                    let total = self.state.download_queue_total;
                    let remaining = self.state.download_queue.len();
                    let current = total - remaining;
                    format!(
                        " Download: S{:02}E{:02} ({}/{}) | {} [X] Cancel ",
                        self.state.selected_season,
                        self.state.selected_episode,
                        current,
                        total,
                        status
                    )
                } else {
                    format!(" Download: {} [X] Cancel ", status)
                };

                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(title_text))
                    .gauge_style(self.theme.accent)
                    .ratio((prog / 100.0).clamp(0.0, 1.0));

                crate::tui::clear_area(frame, dl_area, &self.theme);
                frame.render_widget(gauge, dl_area);
            }
        }

        if self.state.show_theme_popup {
            let items: Vec<String> = crate::tui::theme::AVAILABLE_THEMES
                .iter()
                .map(|s| s.to_string())
                .collect();
            crate::tui::overlay::picker(
                frame,
                area,
                &items,
                &mut self.state.theme_list_state,
                crate::tui::overlay::PickerSpec {
                    title: "Select Theme",
                    confirm_label: "Apply",
                    minimum_width: 32,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }

        if let Some((version, notes)) = &self.state.update_available {
            use ratatui::layout::{Alignment, Constraint, Direction, Layout};
            use ratatui::text::{Line, Span};
            use ratatui::widgets::{Block, Borders, Clear, Paragraph};

            let popup_width = 65;
            let popup_height = 20;

            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(popup_width),
                    Constraint::Min(0),
                ])
                .split(area);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(popup_height),
                    Constraint::Min(0),
                ])
                .split(h_chunks[1]);

            let popup_area = v_chunks[1];
            frame.render_widget(Clear, popup_area);

            let mut text = vec![
                Line::from(Span::styled("Update Available!", self.theme.accent))
                    .alignment(Alignment::Center),
                Line::from(""),
                Line::from("A new version of MovieBox-Tui is available."),
                Line::from(""),
                Line::from(format!("Current: v{}", env!("CARGO_PKG_VERSION"))),
                Line::from(format!("Latest:  v{}", version)),
                Line::from(""),
                Line::from(Span::styled("Release Notes:", self.theme.highlight)),
            ];

            let note_lines = notes
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(3)
                .collect::<Vec<_>>();
            for line in note_lines {
                text.push(Line::from(crate::tui::text::truncate_width(line, 55)));
            }
            if notes.lines().count() > 3 {
                text.push(Line::from(Span::styled(
                    "... (read more on GitHub)",
                    self.theme.text_dim,
                )));
            }

            text.push(Line::from(""));
            text.push(
                Line::from(Span::styled(
                    "[Enter] Close popup   [o] Open in Browser",
                    self.theme.accent,
                ))
                .alignment(Alignment::Center),
            );

            let popup = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border),
            );

            frame.render_widget(popup, popup_area);
        }

        crate::tui::overlay::notifications(
            frame,
            area,
            &self.state.notifications,
            &self.theme,
            self.state.basic_terminal,
        );
    }
}
