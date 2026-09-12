use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::{
    action::Action,
    event::EventHandler,
    state::{InputMode, Screen},
};
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

enum ForcedProtocol {
    None,
    Type(ratatui_image::picker::ProtocolType),
}

impl App {
    pub async fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> std::io::Result<()>
    where
        std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
    {
        let should_probe = self.state.image_supported;
        if self.state.image_picker.is_none() && should_probe {
            self.probe_terminal().await;
        }

        let mut events = EventHandler::new(Duration::from_millis(100));

        if self.state.active_provider == ProviderKind::MovieBox {
            let client = self.service.client.clone();
            tokio::spawn(async move {
                let _ = client.init().await;
            });
        }

        if !self.state.bdix_probed {
            self.action_sender.send(Action::CheckBdixNetwork).ok();
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.state.auto_update && now.saturating_sub(self.state.last_update_check) > 3600 {
            self.state.manual_update_check = false;
            self.action_sender.send(Action::CheckForUpdates).ok();
        }
        self.state.active_screen = Screen::Home;

        self.state.available_players = crate::tui::player::detect();
        if (self.state.default_player.is_none()
            || self.state.default_player.as_deref() == Some("auto"))
            && let Some(first) = self.state.available_players.first()
        {
            self.state.default_player = Some(first.config_key().to_string());
        }
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
        let mut last_window_title = String::new();

        loop {
            let want_beam =
                self.state.input_mode == InputMode::Editing && !self.state.basic_terminal;
            if want_beam != self.state.cursor_beam {
                use crossterm::cursor::SetCursorStyle;
                let style = if want_beam {
                    SetCursorStyle::SteadyBar
                } else {
                    SetCursorStyle::DefaultUserShape
                };
                let _ = crossterm::execute!(std::io::stdout(), style);
                self.state.cursor_beam = want_beam;
            }
            let title = self.contextual_title();
            if title != last_window_title {
                let _ = crate::tui::terminal::set_window_title(&title);
                last_window_title = title;
            }
            self.state.normalize_result_view();
            if self.state.clear_terminal_before_draw {
                if let Err(err) = terminal.clear() {
                    log::debug!("terminal clear warning: {err}");
                    let _ = terminal.backend_mut().clear();
                }
                self.state.clear_poster_protocols();
                self.state.clear_terminal_before_draw = false;
                self.state.dirty = true;
            }
            if self.state.dirty {
                if let Err(err) = terminal.draw(|frame| self.draw(frame)) {
                    log::warn!("transient draw warning: {err}");
                }
                self.state.dirty = false;
            }

            tokio::select! {
                Some(action) = events.next() => {
                    if let Some(quit) = self.handle_action(action).await {
                        return Ok(quit);
                    }
                    while let Ok(action) = events.try_recv() {
                        if let Some(quit) = self.handle_action(action).await {
                            return Ok(quit);
                        }
                    }
                }
                Some(action) = self.action_receiver.recv() => {
                    if let Some(quit) = self.handle_action(action).await {
                        return Ok(quit);
                    }
                    while let Ok(action) = self.action_receiver.try_recv() {
                        if let Some(quit) = self.handle_action(action).await {
                            return Ok(quit);
                        }
                    }
                }
            }
        }
    }

    pub(super) async fn probe_terminal(&mut self) {
        use ratatui_image::picker::{Capability, ProtocolType};

        match Self::forced_protocol() {
            Some(ForcedProtocol::None) => {
                self.state.image_supported = false;
                self.state.image_picker = None;
                return;
            }
            Some(ForcedProtocol::Type(ProtocolType::Halfblocks)) => {
                self.state.image_supported = false;
                self.state.image_picker = None;
                return;
            }
            Some(ForcedProtocol::Type(protocol)) => {
                let font_size = Self::cell_size_override().unwrap_or(ratatui_image::FontSize {
                    width: 10,
                    height: 20,
                });
                #[allow(deprecated)]
                let mut picker = ratatui_image::picker::Picker::from_fontsize(font_size);
                picker.set_protocol_type(protocol);
                self.accept_picker(picker);
                return;
            }
            None => {}
        }
        let picker = self.query_picker().await;

        if self.state.theme_is_auto
            && let Some(background) = picker.as_ref().and_then(|p| {
                p.capabilities()
                    .iter()
                    .find_map(|capability| match capability {
                        Capability::Background(red, green, blue) => Some((*red, *green, *blue)),
                        _ => None,
                    })
            })
        {
            let luminance = 0.2126 * f32::from(background.0)
                + 0.7152 * f32::from(background.1)
                + 0.0722 * f32::from(background.2);
            let is_light = luminance > 128.0;
            self.theme = crate::tui::theme::Theme::detect_with_light(Some(is_light));
            self.state.active_theme_kind = if is_light { "Latte" } else { "Mocha" }.to_string();
        }

        let mut picker = match (picker, Self::cell_size_override()) {
            (Some(picker), None) => picker,
            (probed, cell_size) => {
                let font_size = cell_size.unwrap_or(ratatui_image::FontSize {
                    width: 10,
                    height: 20,
                });
                #[allow(deprecated)]
                let mut rebuilt = ratatui_image::picker::Picker::from_fontsize(font_size);
                if let Some(probed) = probed {
                    rebuilt.set_protocol_type(probed.protocol_type());
                }
                if let Some(ForcedProtocol::Type(protocol)) = Self::forced_protocol() {
                    rebuilt.set_protocol_type(protocol);
                }
                rebuilt
            }
        };

        let capabilities_empty = picker.capabilities().is_empty();
        if matches!(picker.protocol_type(), ProtocolType::Halfblocks) && !capabilities_empty {
            let caps = picker.capabilities();
            let salvaged = if caps.iter().any(|c| matches!(c, Capability::Kitty)) {
                Some(ProtocolType::Kitty)
            } else if caps.iter().any(|c| matches!(c, Capability::Sixel)) {
                Some(ProtocolType::Sixel)
            } else {
                None
            };
            if let Some(protocol) = salvaged {
                log::info!(
                    "salvaging graphics-capable terminal that answered as halfblocks: {:?}",
                    protocol
                );
                picker.set_protocol_type(protocol);
            }
        }

        if matches!(picker.protocol_type(), ProtocolType::Halfblocks) {
            self.state.image_supported = false;
            self.state.image_picker = None;
            return;
        }
        self.accept_picker(picker);
    }

    fn accept_picker(&mut self, picker: ratatui_image::picker::Picker) {
        let cell_h = picker.font_size().height;
        if cell_h > 0 {
            self.state.poster_rows = (96_u16.div_ceil(cell_h)).max(3);
        }

        self.state.image_picker = Some(picker);
        self.state.image_supported = true;
    }

    async fn query_picker(&self) -> Option<ratatui_image::picker::Picker> {
        if !crate::tui::terminal::should_query_images() {
            return None;
        }
        tokio::task::spawn_blocking(|| {
            let options = ratatui_image::picker::cap_parser::QueryStdioOptions {
                timeout: Duration::from_millis(400),
                terminal_background_color_osc: true,
                ..Default::default()
            };
            ratatui_image::picker::Picker::from_query_stdio_with_options(options).ok()
        })
        .await
        .unwrap_or_default()
    }

    fn forced_protocol() -> Option<ForcedProtocol> {
        match std::env::var("MOVIEBOX_IMAGE_PROTOCOL")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "none" | "off" | "false" => Some(ForcedProtocol::None),

            "sixel" => Some(ForcedProtocol::Type(
                ratatui_image::picker::ProtocolType::Sixel,
            )),
            "kitty" => Some(ForcedProtocol::Type(
                ratatui_image::picker::ProtocolType::Kitty,
            )),
            "iterm2" => Some(ForcedProtocol::Type(
                ratatui_image::picker::ProtocolType::Iterm2,
            )),
            _ => None,
        }
    }

    fn cell_size_override() -> Option<ratatui_image::FontSize> {
        let raw = std::env::var("MOVIEBOX_CELL_SIZE").ok()?;
        let raw = raw.trim().to_ascii_lowercase();
        let (width, height) = raw.split_once(['x', '*'])?;
        let width: u16 = width.trim().parse().ok()?;
        let height: u16 = height.trim().parse().ok()?;
        if width == 0 || height == 0 {
            return None;
        }
        Some(ratatui_image::FontSize { width, height })
    }
    pub fn contextual_title(&self) -> String {
        match self.state.active_screen {
            Screen::Home => match self.state.mode() {
                crate::tui::state::AppMode::Streaming => {
                    if self.state.active_provider == crate::providers::models::ProviderKind::Addons
                    {
                        "MovieBox-Tui — Addons".to_string()
                    } else {
                        "MovieBox-Tui — Streaming".to_string()
                    }
                }
                crate::tui::state::AppMode::Tv => "MovieBox-Tui — Live TV".to_string(),
            },
            Screen::Details => {
                if let Some(details) = &self.state.selected_details {
                    if !details.title.is_empty() {
                        return format!("MovieBox-Tui — {}", details.title);
                    }
                }
                "MovieBox-Tui — Details".to_string()
            }
        }
    }

    pub async fn handle_action(&mut self, action: Action) -> Option<()> {
        if self.state.last_resize_time.is_some()
            || !matches!(action, Action::Tick | Action::UpdateDownload(..))
        {
            self.state.dirty = true;
        }
        match action {
            Action::Quit => {
                return Some(());
            }

            Action::Key(key) => {
                self.handle_key(key).await;
            }

            Action::MouseClick(col, row) => {
                self.handle_mouse(col, row);
            }

            Action::ProbeTerminal => {
                self.probe_terminal().await;
                let outcome = if self.state.image_supported {
                    let protocol = self
                        .state
                        .image_picker
                        .as_ref()
                        .map(|picker| format!("{:?}", picker.protocol_type()))
                        .unwrap_or_default();
                    format!("Graphics protocol: {protocol}")
                } else {
                    "No graphics protocol detected; using text placeholders".to_string()
                };
                self.state.notify(
                    crate::tui::overlay::NotificationKind::Info,
                    "Terminal probe",
                    outcome,
                );
            }

            Action::WheelScroll { up } => {
                if self.state.show_help {
                    let delta = 3 * if up { -1i32 } else { 1 };
                    self.state.help_scroll =
                        (self.state.help_scroll as i32 + delta).max(0) as usize;
                    return None;
                }
                let steps = if self.state.active_screen == Screen::Home
                    && !self.state.favorites_focus
                    && !self.state.search_results.is_empty()
                    && self.state.input_mode != InputMode::Editing
                {
                    self.state.effective_row_height().min(8)
                } else {
                    2
                };
                let step = if up { Action::MoveUp } else { Action::MoveDown };
                for _ in 0..steps {
                    let action = step.clone();
                    if let Some(quit) = self.handle_navigation(action).await {
                        return Some(quit);
                    }
                }
            }

            Action::Tick
            | Action::FocusChange
            | Action::Resize(..)
            | Action::SwitchProvider(..)
            | Action::ToggleHelp
            | Action::Refresh
            | Action::ClearCache
            | Action::CacheCleared(..)
            | Action::ToggleThemePopup
            | Action::SelectTheme(..)
            | Action::ShowBrowseMenu
            | Action::SetStatus(..)
            | Action::CheckForUpdates
            | Action::UpdateAvailable(..)
            | Action::StartSelfUpdate
            | Action::SelfUpdateProgress(..)
            | Action::SelfUpdateComplete(..)
            | Action::ToggleSettingsPopup
            | Action::ShowSettingsPopup
            | Action::CloseSettingsPopup
            | Action::SelectSettingsCategory(..)
            | Action::SettingsAdjustValue(..)
            | Action::SettingsActivateRow
            | Action::SettingsResetDownloadDir
            | Action::ToggleProvider(..)
            | Action::CheckBdixNetwork
            | Action::BdixProbeResult { .. } => {
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

            Action::SwitchToStreamingMode
            | Action::ShowAddonManager
            | Action::AddonAddManifest(..)
            | Action::AddonToggleEnabled(..)
            | Action::AddonRemove(..)
            | Action::AddonInputToggle(..) => {
                self.handle_addons(action).await;
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
            | Action::SelectBrowse(..)
            | Action::SelectAddonCatalog(..)
            | Action::SearchSuccess { .. }
            | Action::SearchFailure(..)
            | Action::HomepageSuccess { .. }
            | Action::HomepageFailure(..)
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

            Action::PlayStream
            | Action::ShowSubtitlePopup(..)
            | Action::ShowDownloadSubtitlePopup(..)
            | Action::LaunchMpv(..)
            | Action::LaunchPlayback(..)
            | Action::DispatchPlayback(..)
            | Action::LaunchPlayer(..)
            | Action::MarkWatched(..)
            | Action::UpdateProgress { .. }
            | Action::ReconcileHistory
            | Action::PlayerCrashed(..)
            | Action::PlayerExited => {
                self.handle_playback(action).await;
            }

            Action::ToggleFavorite
            | Action::ShowFavorites
            | Action::OpenFavorite(..)
            | Action::OpenContinueWatching(..) => {
                self.handle_favorites(action).await;
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

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if self.draw_resize_badge(frame, area) {
            return;
        }

        if self.draw_too_small_gate(frame, area) {
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

        self.draw_download_gauge(frame, download_area);
        self.draw_settings_modal(frame, area);
        self.draw_sources_picker(frame, area);
        self.draw_theme_picker(frame, area);
        self.draw_player_picker(frame, area);
        self.draw_subtitle_picker(frame, area);
        self.draw_update_modal(frame, area);
        self.draw_updating_modal(frame, area);
        crate::tui::overlay::notifications(
            frame,
            area,
            &self.state.notifications,
            &self.theme,
            self.state.basic_terminal,
            self.state.download_progress.is_some(),
        );
    }

    fn draw_resize_badge(&self, frame: &mut Frame, area: Rect) -> bool {
        if let Some((_, w, h)) = self.state.last_resize_time {
            let sep = if self.state.basic_terminal { "x" } else { "×" };
            let label = format!("{} {} {}", w, sep, h);
            let label_len = label.chars().count() as u16 + 4;
            let badge_w = label_len.min(area.width);
            let badge_h = 1_u16;
            let badge_x = area.x + (area.width.saturating_sub(badge_w)) / 2;
            let badge_y = area.y + (area.height.saturating_sub(badge_h)) / 2;
            let badge_area = ratatui::layout::Rect {
                x: badge_x,
                y: badge_y,
                width: badge_w,
                height: badge_h,
            };
            let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                label,
                self.theme.title,
            )]);
            frame.render_widget(
                ratatui::widgets::Paragraph::new(line)
                    .alignment(ratatui::layout::Alignment::Center),
                badge_area,
            );
            true
        } else {
            false
        }
    }

    fn draw_too_small_gate(&self, frame: &mut Frame, area: Rect) -> bool {
        if area.width < 50 || area.height < 14 {
            use ratatui::layout::Alignment;
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            if area.width < 4 || area.height < 2 {
                return true;
            }

            if area.width < 25 || area.height < 5 {
                let p = Paragraph::new(format!("{}×{} (min 50×14)", area.width, area.height))
                    .style(self.theme.lavender)
                    .alignment(Alignment::Center);
                frame.render_widget(p, area);
                return true;
            }

            let msg_lines = vec![
                Line::from(format!(
                    "Terminal too small ({}×{}).",
                    area.width, area.height
                )),
                Line::from("Minimum required size: 50×14"),
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
            true
        } else {
            false
        }
    }

    fn draw_download_gauge(&self, frame: &mut Frame, download_area: Option<Rect>) {
        let Some(prog) = self.state.download_progress else {
            return;
        };
        let Some(dl_area) = download_area else {
            return;
        };
        if dl_area.width < 16 || dl_area.height < 3 {
            return;
        }

        use ratatui::layout::Alignment;
        use ratatui::style::Modifier;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let is_compact = dl_area.width < 60;
        let basic = self.state.basic_terminal;
        let modal_active = self.state.has_active_modal();
        let raw_title = self
            .state
            .download_title
            .as_deref()
            .or_else(|| {
                self.state
                    .selected_details
                    .as_ref()
                    .map(|d| d.title.as_str())
            })
            .unwrap_or("Media");

        let cancel_label = if is_compact { "[x]" } else { "[x] Cancel" };
        let cancel_budget = (crate::tui::text::width(cancel_label) as u16).saturating_add(4);

        let mut left_title_spans = Vec::new();
        let prefix_style = if modal_active {
            self.theme.muted
        } else {
            self.theme.teal.add_modifier(Modifier::BOLD)
        };
        if basic {
            left_title_spans.push(Span::styled(" [DL] ", prefix_style));
        } else {
            left_title_spans.push(Span::styled(" ⬇ Downloading: ", prefix_style));
        }

        if self.state.download_queue_total > 0 {
            let total = self.state.download_queue_total;
            let remaining = self.state.download_queue.len();
            let current = total.saturating_sub(remaining);
            let queue_str = format!(
                "S{:02}E{:02} ({}/{}): ",
                self.state.selected_season, self.state.selected_episode, current, total
            );
            left_title_spans.push(Span::styled(
                queue_str,
                if modal_active {
                    self.theme.muted
                } else {
                    self.theme.sapphire
                },
            ));
        }

        let prefix_width = left_title_spans.iter().map(Span::width).sum::<usize>() as u16;
        let title_max_width = dl_area
            .width
            .saturating_sub(prefix_width.saturating_add(cancel_budget).saturating_add(4))
            as usize;
        let truncated_title = crate::tui::text::truncate_width(raw_title, title_max_width.max(6));
        left_title_spans.push(Span::styled(
            truncated_title,
            if modal_active {
                self.theme.muted
            } else {
                self.theme.title.add_modifier(Modifier::BOLD)
            },
        ));
        left_title_spans.push(Span::raw(" "));

        let left_title = Line::from(left_title_spans);
        let right_title = Line::from(vec![Span::styled(
            format!(" {cancel_label} "),
            if modal_active {
                self.theme.muted
            } else {
                self.theme.error.add_modifier(Modifier::BOLD)
            },
        )])
        .alignment(Alignment::Right);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(left_title)
            .title(right_title)
            .border_style(if modal_active {
                self.theme.muted
            } else {
                self.theme.lavender
            })
            .border_type(crate::tui::overlay::border_type(basic));

        let inner_area = block.inner(dl_area);
        crate::tui::clear_area(frame, dl_area, &self.theme);
        frame.render_widget(block, dl_area);

        if inner_area.height == 0 || inner_area.width < 10 {
            return;
        }

        let status_str = self
            .state
            .download_status
            .as_deref()
            .unwrap_or("Downloading...");

        let pct_val = prog.clamp(0.0, 100.0);
        let pct_badge = format!(" {:>5.1}% ", pct_val);
        let pct_width = 8usize;

        let available_for_rest = (inner_area.width as usize).saturating_sub(pct_width + 1);

        let bar_width = if available_for_rest < 20 {
            10usize.min(available_for_rest)
        } else {
            (available_for_rest / 2).clamp(12, 38)
        };

        let track_cells = bar_width.saturating_sub(2);
        let ratio = (pct_val / 100.0).clamp(0.0, 1.0);
        let filled_cells = ((track_cells as f64) * ratio).round() as usize;
        let unfilled_cells = track_cells.saturating_sub(filled_cells);

        let mut row_spans = Vec::new();
        row_spans.push(Span::styled(
            pct_badge,
            if modal_active {
                self.theme.muted
            } else {
                self.theme.sapphire.add_modifier(Modifier::BOLD)
            },
        ));

        row_spans.push(Span::styled(
            "[",
            if modal_active {
                self.theme.muted
            } else {
                self.theme.surface1
            },
        ));
        if basic {
            if filled_cells > 0 && unfilled_cells > 0 {
                row_spans.push(Span::styled(
                    "=".repeat(filled_cells.saturating_sub(1)),
                    if modal_active {
                        self.theme.muted
                    } else {
                        self.theme.accent.add_modifier(Modifier::BOLD)
                    },
                ));
                row_spans.push(Span::styled(
                    ">",
                    if modal_active {
                        self.theme.muted
                    } else {
                        self.theme.accent.add_modifier(Modifier::BOLD)
                    },
                ));
            } else {
                row_spans.push(Span::styled(
                    "=".repeat(filled_cells),
                    if modal_active {
                        self.theme.muted
                    } else {
                        self.theme.accent.add_modifier(Modifier::BOLD)
                    },
                ));
            }
            row_spans.push(Span::styled(
                "-".repeat(unfilled_cells),
                if modal_active {
                    self.theme.muted
                } else {
                    self.theme.surface1
                },
            ));
        } else {
            row_spans.push(Span::styled(
                "━".repeat(filled_cells),
                if modal_active {
                    self.theme.muted
                } else {
                    self.theme.teal.add_modifier(Modifier::BOLD)
                },
            ));
            row_spans.push(Span::styled(
                "─".repeat(unfilled_cells),
                if modal_active {
                    self.theme.muted
                } else {
                    self.theme.surface1
                },
            ));
        }
        row_spans.push(Span::styled(
            "]  ",
            if modal_active {
                self.theme.muted
            } else {
                self.theme.surface1
            },
        ));

        let status_budget = available_for_rest.saturating_sub(bar_width + 2);
        if status_budget >= 6 {
            let truncated_status = crate::tui::text::truncate_width(status_str, status_budget);
            if truncated_status.contains(" | ") {
                let parts: Vec<&str> = truncated_status.split(" | ").collect();
                for (idx, part) in parts.iter().enumerate() {
                    if idx > 0 {
                        row_spans.push(Span::styled(
                            " | ",
                            if modal_active {
                                self.theme.muted
                            } else {
                                self.theme.surface1
                            },
                        ));
                    }
                    if modal_active {
                        row_spans.push(Span::styled(part.to_string(), self.theme.muted));
                    } else if part.starts_with("ETA") {
                        row_spans.push(Span::styled(part.to_string(), self.theme.rating));
                    } else if part.contains("/s") {
                        row_spans.push(Span::styled(
                            part.to_string(),
                            self.theme.teal.add_modifier(Modifier::BOLD),
                        ));
                    } else if part.starts_with("Audio") {
                        row_spans.push(Span::styled(
                            part.to_string(),
                            self.theme.lavender.add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        row_spans.push(Span::styled(part.to_string(), self.theme.subtext1));
                    }
                }
            } else {
                row_spans.push(Span::styled(
                    truncated_status,
                    if modal_active {
                        self.theme.muted
                    } else {
                        self.theme.subtext1
                    },
                ));
            }
        }

        frame.render_widget(Paragraph::new(Line::from(row_spans)), inner_area);
    }

    fn draw_theme_picker(&mut self, frame: &mut Frame, area: Rect) {
        if self.state.show_theme_popup {
            use ratatui::text::{Line, Span};

            let theme_names = crate::tui::theme::AVAILABLE_THEMES;
            let longest_name = theme_names
                .iter()
                .map(|name| crate::tui::text::width(name))
                .max()
                .unwrap_or(10);
            let raw_items: Vec<String> = theme_names
                .iter()
                .map(|name| {
                    if self.state.basic_terminal {
                        format!("  {name:<pad$}  * * * ", pad = longest_name)
                    } else {
                        format!("  {name:<pad$}  ■ ■ ■ ", pad = longest_name)
                    }
                })
                .collect();

            let lines: Vec<Line<'static>> = theme_names
                .iter()
                .map(|name| {
                    let mut spans = vec![
                        Span::raw("  "),
                        Span::styled(format!("{name:<pad$}", pad = longest_name), self.theme.text),
                        Span::raw("  "),
                    ];
                    spans.extend(crate::tui::theme::Theme::palette_swatch_spans(
                        name,
                        self.state.basic_terminal,
                    ));
                    spans.push(Span::raw(" "));
                    Line::from(spans)
                })
                .collect();

            crate::tui::overlay::picker_with_lines(
                frame,
                area,
                &lines,
                &raw_items,
                &mut self.state.theme_list_state,
                crate::tui::overlay::PickerSpec {
                    title: "",
                    confirm_label: "Apply",
                    minimum_width: 16,
                    show_counter: false,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }
    }
    fn draw_settings_modal(&mut self, frame: &mut Frame, area: Rect) {
        if self.state.show_settings_popup {
            crate::tui::widgets::settings::draw(frame, area, &mut self.state, &self.theme);
        }
    }
    fn draw_sources_picker(&mut self, frame: &mut Frame, area: Rect) {
        if self.state.show_sources_popup {
            let providers = crate::providers::models::ProviderKind::ENABLED;
            let raw_items: Vec<String> = providers
                .iter()
                .map(|p| format!(" [✓] {} ", p.label()))
                .collect();
            let lines: Vec<ratatui::text::Line<'static>> = providers
                .iter()
                .map(|p| {
                    let is_enabled = self.state.provider_enabled(*p);
                    let (check, check_style) = if is_enabled {
                        if self.state.basic_terminal {
                            (
                                "[X] ",
                                self.theme
                                    .success
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            )
                        } else {
                            (
                                "[✓] ",
                                self.theme
                                    .success
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            )
                        }
                    } else {
                        ("[ ] ", self.theme.text_dim)
                    };
                    let label_style = if is_enabled {
                        self.theme.text
                    } else {
                        self.theme.text_dim
                    };
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::raw(" "),
                        ratatui::text::Span::styled(check, check_style),
                        ratatui::text::Span::styled(p.label(), label_style),
                        ratatui::text::Span::raw(" "),
                    ])
                })
                .collect();

            crate::tui::overlay::picker_with_lines(
                frame,
                area,
                &lines,
                &raw_items,
                &mut self.state.sources_list_state,
                crate::tui::overlay::PickerSpec {
                    title: "",
                    confirm_label: "Toggle",
                    minimum_width: 20,
                    show_counter: false,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }
    }
    fn draw_player_picker(&mut self, frame: &mut Frame, area: Rect) {
        if self.state.player_picker_popup {
            let raw_items = self
                .state
                .available_players
                .iter()
                .map(|k| format!("  {} ", k.label()))
                .collect::<Vec<_>>();
            crate::tui::overlay::picker(
                frame,
                area,
                &raw_items,
                &mut self.state.player_picker_state,
                crate::tui::overlay::PickerSpec {
                    title: "",
                    confirm_label: "Select",
                    minimum_width: 10,
                    show_counter: false,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }
    }
    fn draw_subtitle_picker(&mut self, frame: &mut Frame, area: Rect) {
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
            crate::tui::overlay::picker(
                frame,
                area,
                &items,
                &mut self.state.subtitle_list_state,
                crate::tui::overlay::PickerSpec {
                    title: if self.state.is_download_subtitle_popup {
                        "Download Subtitles"
                    } else {
                        "Subtitles"
                    },
                    confirm_label,
                    minimum_width: 32,
                    show_counter: true,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }
    }

    fn draw_update_modal(&self, frame: &mut Frame, area: Rect) {
        if self.state.input_mode == crate::tui::state::InputMode::Editing {
            return;
        }

        if let Some((version, notes)) = &self.state.update_available {
            use ratatui::layout::Alignment;
            use ratatui::text::{Line, Span};
            use ratatui::widgets::Paragraph;

            let env = std::env::current_exe()
                .ok()
                .map(|p| crate::updater::apply::detect_environment(&p))
                .unwrap_or(crate::updater::apply::InstallationEnvironment::DirectReplace);

            let layout = crate::tui::overlay::update_modal_layout(area, notes);
            let popup_area = layout.popup_area;
            let display_count = layout.display_count;
            let _ = layout.has_more;

            let note_lines: Vec<&str> = notes
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();

            let inner_area = crate::tui::widgets::ModalFrame::new(
                "Update Available",
                &self.theme,
                self.state.basic_terminal,
            )
            .render(frame, popup_area, area);
            let divider_char = if self.state.basic_terminal {
                "-"
            } else {
                "─"
            };
            let divider_str = divider_char.repeat(inner_area.width as usize);

            let mut text = vec![
                Line::from(vec![
                    Span::styled("Installed: ", self.theme.text_dim),
                    Span::styled(
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        self.theme
                            .subtext1
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::styled("   →   ", self.theme.accent),
                    Span::styled("Latest: ", self.theme.text_dim),
                    Span::styled(
                        format!("v{version}"),
                        self.theme.teal.add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                ])
                .alignment(Alignment::Center),
            ];

            match env {
                crate::updater::apply::InstallationEnvironment::Homebrew => {
                    text.push(
                        Line::from(vec![
                            Span::styled("Homebrew Managed • Run: ", self.theme.text_dim),
                            Span::styled(
                                "brew upgrade moviebox-tui",
                                self.theme
                                    .shortcut
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                        ])
                        .alignment(Alignment::Center),
                    );
                }
                crate::updater::apply::InstallationEnvironment::Termux => {
                    text.push(
                        Line::from(vec![Span::styled(
                            "Termux / Android • Re-run installer script to update",
                            self.theme.accent,
                        )])
                        .alignment(Alignment::Center),
                    );
                }
                crate::updater::apply::InstallationEnvironment::ReadOnly => {
                    text.push(
                        Line::from(vec![Span::styled(
                            "Binary is read-only • Update via your package manager",
                            self.theme.accent,
                        )])
                        .alignment(Alignment::Center),
                    );
                }
                crate::updater::apply::InstallationEnvironment::DirectReplace
                | crate::updater::apply::InstallationEnvironment::WindowsHelper => {}
            }

            text.push(Line::from(Span::styled(
                divider_str.clone(),
                self.theme.surface1,
            )));
            text.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Release Notes:",
                    self.theme
                        .highlight
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            ]));
            let line_width = (inner_area.width as usize).saturating_sub(6);
            let basic = self.state.basic_terminal;

            for line in note_lines.iter().take(display_count) {
                let trimmed = line.trim();
                let mut spans = Vec::new();

                if trimmed.starts_with("### ")
                    || trimmed.starts_with("## ")
                    || trimmed.starts_with("# ")
                {
                    let title = trimmed.trim_start_matches('#').trim();
                    spans.push(Span::raw("    "));
                    if title.eq_ignore_ascii_case("Added") {
                        spans.push(Span::styled(
                            "[Added]",
                            self.theme.teal.add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else if title.eq_ignore_ascii_case("Fixed") {
                        spans.push(Span::styled(
                            "[Fixed]",
                            self.theme
                                .rating
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else if title.eq_ignore_ascii_case("Changed") {
                        spans.push(Span::styled(
                            "[Changed]",
                            self.theme
                                .sapphire
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else if title.eq_ignore_ascii_case("Performance")
                        || title.eq_ignore_ascii_case("Perf")
                    {
                        spans.push(Span::styled(
                            "[Performance]",
                            self.theme
                                .accent
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else if title.eq_ignore_ascii_case("Security") {
                        spans.push(Span::styled(
                            "[Security]",
                            self.theme
                                .error
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(
                            title,
                            self.theme
                                .highlight
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    }
                } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    let bullet = trimmed[2..].trim();
                    spans.push(Span::raw("      "));
                    spans.push(Span::styled(
                        if basic { "- " } else { "• " },
                        self.theme.accent,
                    ));
                    if let Some(rest) = bullet.strip_prefix("**") {
                        if let Some(end) = rest.find("**") {
                            let title = &rest[..end];
                            let after = &rest[end + 2..];
                            let (colon, rest) = if let Some(stripped) = after.strip_prefix(':') {
                                (": ", stripped.trim())
                            } else {
                                ("", after.trim())
                            };
                            let clean_rest = rest.replace('`', "");
                            spans.push(Span::styled(
                                title,
                                self.theme.text.add_modifier(ratatui::style::Modifier::BOLD),
                            ));
                            if !colon.is_empty() {
                                spans.push(Span::styled(colon, self.theme.subtext1));
                            }
                            if !clean_rest.is_empty() {
                                let budget = line_width
                                    .saturating_sub(8)
                                    .saturating_sub(crate::tui::text::width(title))
                                    .saturating_sub(colon.len());
                                if budget > 0 {
                                    spans.push(Span::styled(
                                        crate::tui::text::truncate_width(&clean_rest, budget)
                                            .into_owned(),
                                        self.theme.text_dim,
                                    ));
                                }
                            }
                        } else {
                            let clean = bullet.replace('`', "");
                            spans.push(Span::styled(
                                crate::tui::text::truncate_width(
                                    &clean,
                                    line_width.saturating_sub(8),
                                )
                                .into_owned(),
                                self.theme.text,
                            ));
                        }
                    } else {
                        let clean = bullet.replace('`', "");
                        spans.push(Span::styled(
                            crate::tui::text::truncate_width(&clean, line_width.saturating_sub(8))
                                .into_owned(),
                            self.theme.text,
                        ));
                    }
                } else {
                    let clean = trimmed.replace('`', "");
                    spans.push(Span::raw("      "));
                    spans.push(Span::styled(
                        crate::tui::text::truncate_width(&clean, line_width.saturating_sub(6))
                            .into_owned(),
                        self.theme.text_dim,
                    ));
                }
                text.push(Line::from(spans));
            }

            text.push(Line::from(Span::styled(divider_str, self.theme.surface1)));

            let is_compact_modal = inner_area.width < 58;
            let buttons = match env {
                crate::updater::apply::InstallationEnvironment::Homebrew => {
                    if is_compact_modal {
                        vec![
                            Span::styled("[b]", self.theme.shortcut),
                            Span::styled(" Copy  ", self.theme.text),
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(" Web  ", self.theme.text),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Back", self.theme.text),
                        ]
                    } else {
                        vec![
                            Span::styled("[b]", self.theme.shortcut),
                            Span::styled(
                                " Copy Command    ",
                                self.theme.text.add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(" Open Release Page    ", self.theme.text),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Dismiss", self.theme.text),
                        ]
                    }
                }
                crate::updater::apply::InstallationEnvironment::Termux
                | crate::updater::apply::InstallationEnvironment::ReadOnly => {
                    if is_compact_modal {
                        vec![
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(" Web  ", self.theme.text),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Back", self.theme.text),
                        ]
                    } else {
                        vec![
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(
                                " Open Release Page    ",
                                self.theme.text.add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Dismiss", self.theme.text),
                        ]
                    }
                }
                crate::updater::apply::InstallationEnvironment::DirectReplace
                | crate::updater::apply::InstallationEnvironment::WindowsHelper => {
                    if is_compact_modal {
                        vec![
                            Span::styled("[u]", self.theme.shortcut),
                            Span::styled(" Update  ", self.theme.text),
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(" Web  ", self.theme.text),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Back", self.theme.text),
                        ]
                    } else {
                        vec![
                            Span::styled("[u]", self.theme.shortcut),
                            Span::styled(
                                " Update Now    ",
                                self.theme.text.add_modifier(ratatui::style::Modifier::BOLD),
                            ),
                            Span::styled("[o]", self.theme.shortcut),
                            Span::styled(" Open Release Page    ", self.theme.text),
                            Span::styled("[Esc]", self.theme.shortcut),
                            Span::styled(" Dismiss", self.theme.text),
                        ]
                    }
                }
            };

            text.push(Line::from(buttons).alignment(Alignment::Center));

            let popup = Paragraph::new(text);
            frame.render_widget(popup, inner_area);
        }
    }

    fn draw_updating_modal(&self, frame: &mut Frame, area: Rect) {
        if !self.state.is_updating {
            return;
        }

        use ratatui::layout::Alignment;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let width = 56.min(area.width.saturating_sub(4)).max(36);
        let height = 9.min(area.height.saturating_sub(2)).max(7);
        let popup_area = crate::tui::overlay::centered(area, width, height, 36, 56);

        let inner_area = crate::tui::widgets::ModalFrame::new(
            "Self-Update in Progress",
            &self.theme,
            self.state.basic_terminal,
        )
        .render(frame, popup_area, area);

        let spinner =
            crate::tui::widgets::loading_spinner(self.state.tick_count, self.state.basic_terminal);

        let target_version = self
            .state
            .update_release
            .as_ref()
            .map(|r| r.version.as_str())
            .unwrap_or("latest");

        let progress_msg = self
            .state
            .update_progress_msg
            .as_deref()
            .unwrap_or("Applying update...");

        let action_text = if progress_msg.contains("SHA256") || progress_msg.contains("checksum") {
            "Verifying release checksum...".to_string()
        } else if progress_msg.contains("Extracting") || progress_msg.contains("Applying") {
            format!("Installing MovieBox-Tui v{target_version}...")
        } else {
            format!("Downloading MovieBox-Tui v{target_version}...")
        };

        let (warn_icon, warn_style) = if self.state.basic_terminal {
            ("[!] ", self.theme.shortcut)
        } else {
            ("⚠ ", self.theme.rating)
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("{spinner} "),
                    self.theme
                        .accent
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(
                    action_text,
                    self.theme
                        .header
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled(warn_icon, warn_style),
                Span::styled(
                    if self.state.basic_terminal {
                        "Please wait - do not close terminal"
                    } else {
                        "Please wait • do not close terminal"
                    },
                    self.theme.text_dim,
                ),
            ])
            .alignment(Alignment::Center),
        ];

        let popup = Paragraph::new(text);
        frame.render_widget(popup, inner_area);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_download_bar_rendering_modern_and_basic() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.state.download_progress = Some(93.2);
        app.state.download_title = Some("Ek Deewane Ki Deewaniyat".to_string());
        app.state.download_status = Some("778.6 MB | 4.6 MB/s | ETA 00:12".to_string());
        app.state.basic_terminal = false;
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..100 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                rendered.push_str(cell.symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Downloading:"));
        assert!(rendered.contains("Ek Deewane Ki Deewaniyat"));
        assert!(rendered.contains("[x] Cancel"));
        assert!(rendered.contains("93.2%"));
        assert!(rendered.contains("778.6 MB"));
        assert!(rendered.contains("4.6 MB/s"));
        assert!(rendered.contains("ETA 00:12"));

        app.state.basic_terminal = true;
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        let mut basic_rendered = String::new();
        for y in 0..24 {
            for x in 0..100 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                basic_rendered.push_str(cell.symbol());
            }
            basic_rendered.push('\n');
        }

        assert!(basic_rendered.contains("[DL]"));
        assert!(basic_rendered.contains("===="));
        assert!(basic_rendered.contains("[x] Cancel"));

        let narrow_backend = TestBackend::new(45, 24);
        let mut narrow_terminal = Terminal::new(narrow_backend).unwrap();
        let res = narrow_terminal.draw(|f| {
            app.draw(f);
        });
        assert!(res.is_ok());
    }

    #[test]
    fn test_download_bar_dimmed_when_modal_active() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.state.download_progress = Some(50.0);
        app.state.download_title = Some("Test Movie".to_string());
        app.state.show_settings_popup = true;

        let res = terminal.draw(|f| {
            app.draw(f);
        });
        assert!(res.is_ok());
    }

    #[test]
    fn test_update_modal_rendering_and_markdown_formatting() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        let notes = "### Added\n- **Anchored Provider Selection Menu**: Replaced immediate mouse cycling\n- Added dedicated keyboard navigation\n### Fixed\n- **Stream Table Alignment**: Corrected column budget\n";
        app.state.update_available = Some(("0.1.18".to_string(), notes.to_string()));

        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..100 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                rendered.push_str(cell.symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains("Update Available"));
        assert!(rendered.contains("Installed:"));
        assert!(rendered.contains("Latest:"));
        assert!(rendered.contains("v0.1.18"));
        assert!(rendered.contains("Release Notes:"));
        assert!(!rendered.contains("▌"));
        assert!(rendered.contains("[Added]"));
        assert!(rendered.contains("[Fixed]"));
        assert!(rendered.contains("Anchored Provider Selection Menu:"));
        assert!(rendered.contains("Stream Table Alignment:"));
        assert!(!rendered.contains("**Anchored"));
        assert!(!rendered.contains("**Stream"));
        assert!(rendered.contains("[u]"));
        assert!(rendered.contains("[o]"));
        assert!(rendered.contains("[Esc]"));
    }

    #[test]
    fn test_updating_modal_pipeline_rendering_and_geometry() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.state.is_updating = true;
        app.state.update_progress_msg =
            Some("Downloading MovieBox_macOS_Universal.tar.gz...".to_string());
        app.state.basic_terminal = false;

        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..100 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                rendered.push_str(cell.symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Self-Update in Progress"));
        assert!(rendered.contains("Downloading MovieBox-Tui v"));
        assert!(rendered.contains("⚠"));
        assert!(rendered.contains("Please wait • do not close terminal"));

        app.state.basic_terminal = true;
        app.state.update_progress_msg = Some("Applying update...".to_string());
        terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();

        let mut basic_rendered = String::new();
        for y in 0..30 {
            for x in 0..100 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                basic_rendered.push_str(cell.symbol());
            }
            basic_rendered.push('\n');
        }
        assert!(basic_rendered.contains("Self-Update in Progress"));
        assert!(basic_rendered.contains("Installing MovieBox-Tui v"));
        assert!(basic_rendered.contains("[!]"));
        assert!(basic_rendered.contains("Please wait - do not close terminal"));

        let min_backend = TestBackend::new(50, 14);
        let mut min_terminal = Terminal::new(min_backend).unwrap();
        app.state.basic_terminal = false;
        app.state.update_progress_msg =
            Some("Downloading MovieBox_macOS_Universal.tar.gz...".to_string());
        min_terminal
            .draw(|f| {
                app.draw(f);
            })
            .unwrap();
        let mut min_rendered = String::new();
        for y in 0..14 {
            for x in 0..50 {
                let cell = min_terminal.backend().buffer().cell((x, y)).unwrap();
                min_rendered.push_str(cell.symbol());
            }
            min_rendered.push('\n');
        }
        assert!(min_rendered.contains("Self-Update in Progress"));
        assert!(min_rendered.contains("Downloading MovieBox-Tui v"));
        assert!(min_rendered.contains("Please wait • do not close terminal"));
    }
}
