use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::text::parse_duration_seconds;
use crate::tui::{action::Action, overlay::NotificationKind, state::Screen};

pub(crate) use crate::service::extract_cover_url;

impl App {
    fn preferred_playback_player(
        &self,
        source: &crate::providers::models::PlaybackSource,
    ) -> Option<crate::tui::state::PlayerKind> {
        self.state
            .available_players
            .iter()
            .copied()
            .find(|kind| crate::tui::player::supports_headers(*kind, &source.headers))
    }

    fn build_watch_history_item(&self) -> Option<crate::history::WatchHistoryItem> {
        let subject_id = self.state.active_subject_id.as_ref()?;
        let provider = self.provider_for_subject(subject_id).cache_key();
        let season = self.state.selected_season;
        let episode = self.state.selected_episode;
        let mut title = "Unknown".to_string();
        let mut cover_url = None;
        let mut stype = 1;
        let mut release_year = "Unknown".to_string();
        let mut duration_seconds = None;

        if let Some(details) = &self.state.selected_details {
            if let Some(t) = details.get("title").and_then(|t| t.as_str()) {
                title = crate::providers::moviebox::clean_moviebox_title(t);
            }
            cover_url = extract_cover_url(details);
            stype = crate::tui::state::stype(details);
            if let Some(y) = details
                .get("year")
                .or_else(|| details.get("releaseYear"))
                .and_then(|y| y.as_str())
            {
                release_year = y.to_string();
            } else if let Some(y) = details.get("year").and_then(|y| y.as_i64()) {
                release_year = y.to_string();
            }
            if let Some(d) = details.get("duration").and_then(|v| v.as_str()) {
                duration_seconds = parse_duration_seconds(d);
            }
        }

        if cover_url.is_none() {
            cover_url = self
                .state
                .search_results
                .iter()
                .find(|r| r.id == *subject_id)
                .and_then(|r| r.cover_url.clone());
        }

        if cover_url.is_none() {
            if let Some(preview) = &self.state.search_preview {
                cover_url = extract_cover_url(preview);
                if duration_seconds.is_none() {
                    if let Some(d) = preview.get("duration").and_then(|v| v.as_str()) {
                        duration_seconds = parse_duration_seconds(d);
                    }
                }
            }
        }

        if title == "Unknown" {
            if let Some(res) = self
                .state
                .search_results
                .iter()
                .find(|r| r.id == *subject_id)
            {
                title = res.title.clone();
                stype = res.stype;
                if release_year == "Unknown" {
                    release_year = res.release_year.clone();
                }
            }
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Some(crate::history::WatchHistoryItem {
            provider: provider.to_string(),
            subject_id: subject_id.clone(),
            title,
            cover_url,
            stype,
            release_year,
            season,
            episode,
            timestamp,
            duration_seconds,
            progress_seconds: 0,
            completed: false,
        })
    }

    pub(super) fn launch_player(
        &mut self,
        kind: crate::tui::state::PlayerKind,
        link: String,
        subtitle: Option<String>,
        headers: Vec<(String, String)>,
    ) {
        if !crate::tui::text::is_http_url(&link) {
            self.state.is_playing = false;
            self.state.is_resolving_playback = false;
            self.state.notify(
                NotificationKind::Error,
                "Unsupported stream",
                "Only HTTP and HTTPS stream protocols are supported for playback.",
            );
            return;
        }

        let history_item = self.build_watch_history_item();
        let resume_seconds = if let Some(item) = &history_item {
            self.state
                .history
                .get_item(
                    &item.provider,
                    &item.subject_id,
                    item.season,
                    item.episode,
                    Some(&item.title),
                )
                .filter(|existing| existing.is_in_progress())
                .map(|existing| existing.progress_seconds)
        } else {
            None
        };

        self.state.is_playing = true;
        self.state.is_resolving_playback = false;

        let tracker_opts = history_item.as_ref().map(|item| {
            (
                item.provider.clone(),
                item.subject_id.clone(),
                item.season,
                item.episode,
            )
        });

        let sender = self.action_sender.clone();
        let cell_size = self
            .state
            .image_picker
            .as_ref()
            .map(|picker| picker.font_size());
        let window = crossterm::terminal::size().ok().map(|(cols, rows)| {
            let (cell_width, cell_height) = cell_size
                .filter(|size| size.width > 0 && size.height > 0)
                .map(|size| (size.width as u32, size.height as u32))
                .unwrap_or((8, 16));
            (
                (cols as u32 * cell_width).clamp(320, 1920),
                (rows as u32 * cell_height).clamp(180, 1080),
            )
        });
        tokio::spawn(async move {
            let mut local_subtitle = subtitle.clone();
            let mut temporary_subtitle = None;
            if matches!(
                kind,
                crate::tui::state::PlayerKind::Vlc | crate::tui::state::PlayerKind::Iina
            ) && let Some(url) = subtitle
            {
                let download_res = crate::service::MovieBoxService::new()
                    .download_subtitle_file(&url, &headers)
                    .await;
                match download_res {
                    Ok(path) => {
                        local_subtitle = Some(path.to_string_lossy().into_owned());
                        temporary_subtitle = Some(path);
                    }
                    Err(_) => {
                        log::warn!(
                            "subtitle download failed for {:?} player, playing without subtitles (url was {})",
                            kind,
                            crate::logging::sanitize_url(&url)
                        );
                        let _ = sender.send(Action::SetStatus(
                            "Subtitles unavailable; playing without subtitles.".to_string(),
                        ));
                    }
                }
            }

            let tracker_ref = tracker_opts
                .as_ref()
                .map(|(p, s, se, ep)| (p.as_str(), s.as_str(), *se, *ep));

            let mut command = crate::tui::player::command(
                kind,
                &link,
                local_subtitle.as_deref(),
                &headers,
                window,
                resume_seconds,
                tracker_ref,
            );
            command.stdin(std::process::Stdio::null());
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::piped());

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                command.process_group(0);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }

            match command.spawn() {
                Ok(mut child) => {
                    let start_time = std::time::Instant::now();
                    let stderr_stream = child.stderr.take();

                    tokio::task::spawn_blocking(move || {
                        let mut error_output = String::new();
                        if let Some(mut stderr) = stderr_stream {
                            use std::io::Read;
                            let _ = stderr.read_to_string(&mut error_output);
                        }

                        let result = child.wait();

                        if let Ok(status) = result {
                            let clean_error = error_output.trim().to_string();
                            if !status.success() {
                                let message = if clean_error.is_empty() {
                                    #[cfg(unix)]
                                    {
                                        use std::os::unix::process::ExitStatusExt;
                                        match status.signal() {
                                            Some(sig) => format!(
                                                "Player terminated by signal {sig} (no output captured)."
                                            ),
                                            None => {
                                                "Player exited with no output captured.".to_string()
                                            }
                                        }
                                    }
                                    #[cfg(not(unix))]
                                    {
                                        "Player exited with no output captured.".to_string()
                                    }
                                } else {
                                    clean_error
                                };
                                sender
                                    .send(Action::PlayerCrashed(status.code(), message))
                                    .ok();
                            } else {
                                sender.send(Action::ReconcileHistory).ok();

                                if let Some(item) = history_item {
                                    let elapsed = start_time.elapsed().as_secs();
                                    let duration = item.duration_seconds;
                                    // Without a known duration we can't tell playback time
                                    // from time spent paused/buffering, so `start + elapsed`
                                    // isn't trustworthy; skip saving rather than risk the
                                    // resume point drifting past the end of the file.
                                    if elapsed >= 30
                                        && let Some(d) = duration
                                    {
                                        let start_pos = resume_seconds.unwrap_or(0);
                                        let progress = start_pos.saturating_add(elapsed).min(d);
                                        let completed =
                                            d > 0 && progress >= (d as f64 * 0.90) as u64;
                                        sender
                                            .send(Action::UpdateProgress {
                                                item,
                                                progress,
                                                duration,
                                                completed,
                                            })
                                            .ok();
                                    }
                                }
                            }
                        }

                        if let Some(path) = temporary_subtitle {
                            let _ = std::fs::remove_file(path);
                        }
                        sender.send(Action::PlayerExited).ok();
                    });
                }
                Err(error) => {
                    log::error!(
                        "failed to spawn player {:?} for {}: {error}",
                        kind,
                        crate::logging::sanitize_url(&link)
                    );
                    if let Some(path) = temporary_subtitle {
                        let _ = tokio::fs::remove_file(path).await;
                    }
                    sender
                        .send(Action::PlayerCrashed(
                            None,
                            format!("Failed to spawn player executable: {error}"),
                        ))
                        .ok();
                    sender.send(Action::PlayerExited).ok();
                }
            }
        });
    }
}

impl App {
    pub(super) async fn handle_playback(&mut self, action: Action) -> Option<()> {
        match action {
            Action::PlayStream(open_with) => {
                if self.state.is_playing {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Playback already active",
                        "Stop the current player before starting another.",
                    );
                    return None;
                }
                if self.state.is_resolving_playback
                    || self.state.last_playback_launch.elapsed().as_millis() < 500
                {
                    return None;
                }
                self.state.last_playback_launch = std::time::Instant::now();
                self.state.is_resolving_playback = true;
                if self.current_subject_provider() == ProviderKind::FourKHdHub
                    || self.current_subject_provider() == ProviderKind::Addons
                    || self.current_subject_provider().is_bdix()
                {
                    if let Some(release) = self.get_selected_release() {
                        let Some(first_mirror) = release.mirrors.first().cloned() else {
                            self.state.is_resolving_playback = false;
                            self.state.notify(
                                NotificationKind::Error,
                                "Playback unavailable",
                                "No playable mirrors were found for this release.",
                            );
                            return None;
                        };
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing playback",
                            "Resolving the selected mirror.",
                        );
                        let direct_source = crate::providers::models::PlaybackSource {
                            provider: release.provider,
                            url: first_mirror.resolver_url.clone(),
                            headers: first_mirror.headers.clone(),
                            subtitle: None,
                            source_label: first_mirror.label.clone(),
                        };
                        let default_player = self.preferred_playback_player(&direct_source);
                        let available_players = self.state.available_players.clone();
                        let client = if release.provider == ProviderKind::Addons
                            || release.provider == ProviderKind::BdixCircleFtp
                            || release.provider == ProviderKind::BdixDhakaFlix
                        {
                            let sender_clone = self.action_sender.clone();
                            if open_with || default_player.is_none() {
                                sender_clone
                                    .send(Action::ShowPlaybackPicker(direct_source))
                                    .ok();
                            } else if let Some(player) = default_player {
                                sender_clone
                                    .send(Action::LaunchPlayback(player, direct_source))
                                    .ok();
                            }
                            return None;
                        } else {
                            match self.service.fourk_client.clone() {
                                Some(client) => client,
                                None => {
                                    self.state.is_resolving_playback = false;
                                    self.action_sender
                                        .send(Action::SetStatus(
                                            "Error: 4KHDHub provider is unavailable".to_string(),
                                        ))
                                        .ok();
                                    return None;
                                }
                            }
                        };
                        let sender = self.action_sender.clone();
                        tokio::spawn(async move {
                            match client.resolve_release(&release).await {
                                Ok(source) => {
                                    let default_player =
                                        available_players.iter().copied().find(|kind| {
                                            crate::tui::player::supports_headers(
                                                *kind,
                                                &source.headers,
                                            )
                                        });
                                    if open_with || default_player.is_none() {
                                        sender.send(Action::ShowPlaybackPicker(source)).ok();
                                    } else if let Some(player) = default_player {
                                        sender.send(Action::LaunchPlayback(player, source)).ok();
                                    }
                                }
                                Err(error) => {
                                    log::error!("4KHDHub resolve failed: {error}");
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Error: 4KHDHub source failed: {error}"
                                        )))
                                        .ok();
                                }
                            }
                        });
                    } else {
                        self.state.is_resolving_playback = false;
                    }
                    return None;
                }
                if self.state.active_screen == Screen::Details
                    && let Some(link) = self.get_selected_link()
                {
                    let subject_id = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("id"))
                        .and_then(crate::tui::state::subject_id)
                        .unwrap_or_default();
                    let resource_id = self.get_selected_resource_id();

                    if let Some(rid) = resource_id {
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing playback",
                            "Fetching subtitles.",
                        );
                        let client = self.service.client.clone();
                        let sender = self.action_sender.clone();
                        let link_clone = link.clone();
                        tokio::spawn(async move {
                            let cached = tokio::task::spawn_blocking({
                                let subject_id = subject_id.clone();
                                let rid = rid.clone();
                                move || crate::cache::get_captions_cache(&subject_id, &rid)
                            })
                            .await
                            .ok()
                            .flatten();
                            if let Some(res) = cached {
                                sender
                                    .send(Action::ShowSubtitlePopup(
                                        link_clone.clone(),
                                        res,
                                        open_with,
                                    ))
                                    .ok();
                                return;
                            }
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(15),
                                client.get_ext_captions(&subject_id, &rid),
                            )
                            .await;
                            match result {
                                Ok(Ok(res)) => {
                                    let subject_id = subject_id.clone();
                                    let rid = rid.clone();
                                    let res_for_cache = res.clone();
                                    tokio::task::spawn_blocking(move || {
                                        crate::cache::set_captions_cache(
                                            &subject_id,
                                            &rid,
                                            &res_for_cache,
                                        );
                                    });
                                    sender
                                        .send(Action::ShowSubtitlePopup(link_clone, res, open_with))
                                        .ok();
                                }
                                _ => {
                                    if open_with {
                                        sender
                                            .send(Action::ShowPlayerPicker(link_clone, None))
                                            .ok();
                                    } else {
                                        sender.send(Action::LaunchMpv(link_clone, None)).ok();
                                    }
                                }
                            }
                        });
                    } else {
                        if open_with {
                            self.action_sender
                                .send(Action::ShowPlayerPicker(link, None))
                                .ok();
                        } else {
                            self.action_sender.send(Action::LaunchMpv(link, None)).ok();
                        }
                    }
                } else {
                    self.state.is_resolving_playback = false;
                }
            }
            Action::ShowSubtitlePopup(link, ext_captions, open_with) => {
                self.state.is_resolving_playback = false;
                let options = crate::tui::state::caption_options(&ext_captions);

                if options.len() > 1 {
                    self.state.show_help = false;
                    self.state.player_picker_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.subtitle_popup = true;
                    self.state.subtitle_list = options;
                    self.state.subtitle_list_state.select(Some(0));
                    self.state.pending_play_link = Some(link);
                    self.state.pending_open_with = open_with;
                } else {
                    if open_with {
                        self.action_sender
                            .send(Action::ShowPlayerPicker(link, None))
                            .ok();
                    } else {
                        self.action_sender.send(Action::LaunchMpv(link, None)).ok();
                    }
                }
            }
            Action::ShowDownloadSubtitlePopup(ext_captions) => {
                self.state.is_resolving_playback = false;
                let options = crate::tui::state::caption_options(&ext_captions);

                if options.len() > 1 {
                    self.state.show_help = false;
                    self.state.player_picker_popup = false;
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = true;
                    self.state.subtitle_list = options;
                    self.state.subtitle_list_state.select(Some(0));
                } else {
                    self.action_sender.send(Action::DownloadStream(None)).ok();
                }
            }
            Action::LaunchMpv(link, subtitle_url) => {
                if self.state.is_playing {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Playback already active",
                        "Stop the current player before starting another.",
                    );
                    return None;
                }
                if self.state.last_playback_launch.elapsed().as_millis() < 500 {
                    return None;
                }
                self.state.last_playback_launch = std::time::Instant::now();
                self.state.is_resolving_playback = false;
                let player = self.state.available_players.first().cloned();
                match player {
                    None => {
                        self.state.notify(
                            NotificationKind::Error,
                            "Player unavailable",
                            "Install mpv, IINA, or VLC.",
                        );
                    }
                    Some(kind) => {
                        let player_name = match kind {
                            crate::tui::state::PlayerKind::Mpv => "MPV",
                            crate::tui::state::PlayerKind::Iina => "IINA",
                            crate::tui::state::PlayerKind::Vlc => "VLC",
                            crate::tui::state::PlayerKind::AndroidIntent => "Android Player",
                        };
                        self.state.notify(
                            NotificationKind::Info,
                            "Opening player",
                            format!("Launching {player_name}."),
                        );

                        self.action_sender
                            .send(Action::LaunchPlayer(kind, link, subtitle_url))
                            .ok();
                    }
                }
            }

            Action::ShowPlaybackPicker(source) => {
                self.state.is_resolving_playback = false;
                if self.state.available_players.is_empty() {
                    self.state.set_status(
                        "No media player found. Install mpv, IINA, VLC, or use Android Player.",
                        150,
                    );
                    return None;
                }
                self.state.show_help = false;
                self.state.tv_config_popup = false;
                self.state.player_picker_popup = true;
                self.state.player_picker_playback = Some(source);
                self.state.player_picker_link = None;
                self.state.player_picker_subtitle = None;
                self.state.player_picker_state.select(Some(0));
                self.state.subtitle_popup = false;
            }
            Action::ShowPlayerPicker(link, subtitle) => {
                self.state.is_resolving_playback = false;
                if self.state.available_players.is_empty() {
                    self.state.notify(
                        NotificationKind::Error,
                        "Player unavailable",
                        "Install mpv, IINA, VLC, or use Android Player.",
                    );
                    return None;
                }
                self.state.show_help = false;
                self.state.tv_config_popup = false;
                self.state.player_picker_popup = true;
                self.state.player_picker_playback = None;
                self.state.player_picker_link = Some(link);
                self.state.player_picker_subtitle = subtitle;
                self.state.player_picker_state.select(Some(0));
                self.state.subtitle_popup = false;
            }
            Action::LaunchPlayer(kind, link, sub) => {
                self.state.is_resolving_playback = false;
                self.state.player_picker_popup = false;
                self.state.last_playback_launch = std::time::Instant::now();
                self.launch_player(kind, link, sub, Vec::new());
            }
            Action::LaunchPlayback(kind, source) => {
                self.state.is_resolving_playback = false;
                self.state.player_picker_popup = false;
                self.state.last_playback_launch = std::time::Instant::now();
                if !crate::tui::player::supports_headers(kind, &source.headers) {
                    self.state.set_status(
                        format!(
                            "This source needs headers {} cannot provide; use mpv or IINA.",
                            kind.label()
                        ),
                        180,
                    );
                    return None;
                }
                self.launch_player(kind, source.url, source.subtitle, source.headers);
            }
            Action::MarkWatched(item) => {
                self.state.history.mark_watched(item);
                let history = self.state.history.clone();
                tokio::task::spawn_blocking(move || history.save());
            }
            Action::UpdateProgress {
                item,
                progress,
                duration,
                completed,
            } => {
                self.state
                    .history
                    .update_progress(item, progress, duration, completed);
            }
            Action::ReconcileHistory => {
                self.state.history.reconcile_pending_playback_states();
            }
            Action::PlayerExited => {
                self.state.is_playing = false;
                self.state.is_resolving_playback = false;
            }
            Action::PlayerCrashed(code, error_msg) => {
                self.state.is_playing = false;
                self.state.is_resolving_playback = false;
                let code_str = code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into());
                log::error!("player crashed (code {code_str}): {error_msg}");

                let display_err = if error_msg.is_empty() {
                    "No error output provided by player.".to_string()
                } else {
                    error_msg.lines().last().unwrap_or(&error_msg).to_string()
                };

                self.state.set_status(
                    format!("Player crashed (code {code_str}): {display_err}"),
                    300,
                );

                self.state.notify(
                    NotificationKind::Error,
                    "Player Error",
                    format!("Crash code: {code_str}\n{display_err}"),
                );
            }
            _ => return None,
        }
        None
    }
}
