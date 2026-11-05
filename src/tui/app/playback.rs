use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::text::parse_duration_seconds;
use crate::tui::{action::Action, overlay::NotificationKind, state::Screen};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlaybackResolution {
    Available(crate::tui::state::PlayerKind),
    ExplicitPlayerIncompatible {
        chosen: crate::tui::state::PlayerKind,
        compatible_alternatives: Vec<crate::tui::state::PlayerKind>,
    },
    NoCompatiblePlayer {
        available: Vec<crate::tui::state::PlayerKind>,
    },
    NoPlayersInstalled,
}

impl App {
    pub(super) fn resolve_playback_player(
        &self,
        source: &crate::providers::models::PlaybackSource,
    ) -> PlaybackResolution {
        if self.state.available_players.is_empty() {
            return PlaybackResolution::NoPlayersInstalled;
        }

        let preferred = std::env::var(crate::player::ENV_MOVIEBOX_PLAYER)
            .ok()
            .and_then(|value| crate::tui::state::PlayerKind::parse(&value))
            .or_else(|| {
                let configured = self
                    .state
                    .default_player
                    .as_deref()
                    .filter(|p| !p.eq_ignore_ascii_case("auto"))
                    .and_then(crate::tui::state::PlayerKind::parse);
                if crate::player::has_graphical_display()
                    && configured == Some(crate::tui::state::PlayerKind::AndroidIntent)
                    && self
                        .state
                        .available_players
                        .iter()
                        .any(|&p| p != crate::tui::state::PlayerKind::AndroidIntent)
                {
                    None
                } else {
                    configured
                }
            });

        if let Some(chosen) = preferred {
            if self.state.available_players.contains(&chosen) {
                if crate::tui::player::supports_headers(chosen, &source.headers) {
                    return PlaybackResolution::Available(chosen);
                }
                let compatible_alternatives = self
                    .state
                    .available_players
                    .iter()
                    .copied()
                    .filter(|&k| {
                        k != chosen && crate::tui::player::supports_headers(k, &source.headers)
                    })
                    .collect::<Vec<_>>();
                return PlaybackResolution::ExplicitPlayerIncompatible {
                    chosen,
                    compatible_alternatives,
                };
            }
        }

        if let Some(player) = self
            .state
            .available_players
            .iter()
            .copied()
            .find(|kind| crate::tui::player::supports_headers(*kind, &source.headers))
        {
            PlaybackResolution::Available(player)
        } else {
            PlaybackResolution::NoCompatiblePlayer {
                available: self.state.available_players.clone(),
            }
        }
    }

    pub(super) fn dispatch_playback_or_notify(
        &mut self,
        source: crate::providers::models::PlaybackSource,
    ) {
        match self.resolve_playback_player(&source) {
            PlaybackResolution::Available(player) => {
                self.action_sender
                    .send(Action::LaunchPlayback(player, source))
                    .ok();
            }
            PlaybackResolution::ExplicitPlayerIncompatible {
                chosen,
                compatible_alternatives,
            } => {
                self.state.is_resolving_playback = false;
                self.state.pending_playback_source = None;
                let chosen_name = chosen.label();
                let body = if let Some(first) = compatible_alternatives.first() {
                    format!(
                        "{chosen_name} lacks header support. Switch to {} in /settings.",
                        first.label()
                    )
                } else {
                    format!("{chosen_name} lacks custom header support.")
                };
                self.state.notify(
                    NotificationKind::Warning,
                    format!("{chosen_name} Incompatible"),
                    body,
                );
            }
            PlaybackResolution::NoCompatiblePlayer { available: _ } => {
                self.state.is_resolving_playback = false;
                self.state.pending_playback_source = None;
                self.state.notify(
                    NotificationKind::Error,
                    "Incompatible Player",
                    "Installed player lacks custom stream header support.".to_string(),
                );
            }
            PlaybackResolution::NoPlayersInstalled => {
                self.state.is_resolving_playback = false;
                self.state.pending_playback_source = None;
                let custom_path = match self.state.default_player.as_deref() {
                    Some("vlc") => self.state.vlc_path.as_deref(),
                    Some("mpv") => self.state.mpv_path.as_deref(),
                    Some("iina") => self.state.iina_path.as_deref(),
                    _ => None,
                };
                let (title, message) = if let Some(bad_path) = custom_path {
                    (
                        "Invalid Player Path",
                        format!("Player path not found: {bad_path}"),
                    )
                } else if crate::config::is_termux_environment() {
                    (
                        "No Media Player",
                        "No Android video player or termux-tools detected.".to_string(),
                    )
                } else {
                    (
                        "No Media Player",
                        "No supported media player (mpv, IINA, VLC) found.".to_string(),
                    )
                };
                self.state.notify(NotificationKind::Error, title, message);
            }
        }
    }

    fn build_watch_history_item(&self) -> Option<crate::history::WatchHistoryItem> {
        let subject_id = self.state.active_subject_id.as_ref()?;
        let provider = self.provider_for_subject(subject_id).cache_key();
        let season = self.state.selected_season;
        let episode = self.state.selected_episode;
        let selected_stream_filename = self.get_selected_release().map(|r| r.filename.clone());

        if let Some(details) = &self.state.selected_details {
            let mut item =
                crate::history::WatchHistoryItem::from_details(provider, details, season, episode);
            item.stream_filename = selected_stream_filename;
            if item.cover_url.is_none() {
                item.cover_url = self
                    .state
                    .search_results
                    .iter()
                    .find(|r| r.id == *subject_id)
                    .and_then(|r| r.cover_url.clone())
                    .or_else(|| {
                        self.state
                            .search_preview
                            .as_ref()
                            .filter(|p| p.id.value == *subject_id)
                            .and_then(|p| p.cover_url().map(|s| s.to_string()))
                    });
            }
            if item.duration_seconds.is_none() {
                item.duration_seconds = self
                    .state
                    .search_preview
                    .as_ref()
                    .filter(|p| p.id.value == *subject_id)
                    .and_then(|p| p.duration.as_deref().and_then(parse_duration_seconds));
            }
            return Some(item);
        }

        let res = self
            .state
            .search_results
            .iter()
            .find(|r| r.id == *subject_id);
        let title = res
            .map(|r| r.title.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let cover_url = res.and_then(|r| r.cover_url.clone());
        let stype = res.map(|r| r.stype).unwrap_or(1);
        let release_year = res
            .map(|r| r.release_year.clone())
            .unwrap_or_else(|| "Unknown".to_string());
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
            duration_seconds: None,
            progress_seconds: 0,
            completed: false,
            stream_filename: selected_stream_filename,
        })
    }

    pub(super) fn launch_player(
        &mut self,
        kind: crate::tui::state::PlayerKind,
        link: String,
        subtitle: Option<String>,
        headers: Vec<(String, String)>,
        max_height: Option<u64>,
    ) {
        if !crate::tui::text::is_http_url(&link) {
            self.state.is_playing = false;
            self.state.is_resolving_playback = false;
            self.state.notify(
                NotificationKind::Error,
                "Unsupported stream",
                "Only HTTP/HTTPS streams supported.",
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
        if let Some(item) = &history_item {
            self.state
                .history
                .record_start(item, resume_seconds.unwrap_or(0));
        }

        if matches!(kind, crate::tui::state::PlayerKind::Mpv)
            && let (Some((p, s, se, ep)), Some(item)) = (&tracker_opts, &history_item)
        {
            if let Some(state_path) = crate::player::tracker::state_file_path(p, s, *se, *ep) {
                let initial_state = crate::history::PendingPlaybackState::from_item(
                    item,
                    resume_seconds.unwrap_or(0),
                    item.duration_seconds,
                    false,
                );
                if let Ok(serialized) = serde_json::to_string(&initial_state) {
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = std::fs::write(&state_path, serialized) {
                            log::warn!(
                                "failed to write initial playback state to {}: {e}",
                                crate::logging::sanitize_path(&state_path)
                            );
                        }
                    });
                }
            }
        }

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
        let preferred_sub_name = history_item.as_ref().map(|item| {
            if item.season > 0 || item.episode > 0 {
                format!(
                    "{} - S{:02}E{:02}",
                    item.title,
                    item.season.max(1),
                    item.episode.max(1)
                )
            } else {
                item.title.clone()
            }
        });
        let service = self.service.clone();
        tokio::spawn(async move {
            let is_android = matches!(kind, crate::tui::state::PlayerKind::AndroidIntent);
            let is_dash = crate::player::is_dash_url(&link);
            let has_extra_headers = headers.iter().any(|(name, _)| {
                !name.eq_ignore_ascii_case("referer") && !name.eq_ignore_ascii_case("user-agent")
            });
            let eager_proxy = is_dash
                || (matches!(
                    kind,
                    crate::tui::state::PlayerKind::Vlc
                        | crate::tui::state::PlayerKind::AndroidIntent
                ) && has_extra_headers);
            let sidecar_sub = if is_android {
                subtitle.as_deref()
            } else {
                None
            };

            let eager_sidecar_fut = async {
                if eager_proxy {
                    let link_c = link.clone();
                    let headers_c = headers.clone();
                    let sub_c = sidecar_sub.map(str::to_string);
                    tokio::task::spawn_blocking(move || {
                        crate::proxy::spawn_sidecar(
                            &link_c,
                            &headers_c,
                            sub_c.as_deref(),
                            max_height,
                        )
                    })
                    .await
                    .ok()
                } else {
                    None
                }
            };

            let sub_fut = async {
                if let Some(url) = &subtitle {
                    Some(
                        service
                            .download_subtitle_file(url, &headers, preferred_sub_name.as_deref())
                            .await,
                    )
                } else {
                    None
                }
            };

            let (eager_sidecar_res, sub_res) = tokio::join!(eager_sidecar_fut, sub_fut);

            let mut local_subtitle = subtitle.clone();
            let mut temporary_subtitle = None;
            if let (Some(url), Some(download_res)) = (&subtitle, sub_res) {
                match download_res {
                    Ok(path) => {
                        local_subtitle = Some(path.to_string_lossy().into_owned());
                        if !is_android {
                            temporary_subtitle = Some(path);
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "subtitle download failed for {:?} player ({err}, url was {})",
                            kind,
                            crate::logging::sanitize_url(url)
                        );
                        if !matches!(kind, crate::tui::state::PlayerKind::Mpv) {
                            local_subtitle = None;
                            let _ = sender.send(Action::SetStatus(format!(
                                "Warning: Subtitle unavailable ({err}); playing without subs."
                            )));
                        }
                    }
                }
            }

            let tracker_ref = tracker_opts
                .as_ref()
                .map(|(p, s, se, ep)| (p.as_str(), s.as_str(), *se, *ep));

            let android_sub_is_private = is_android
                && local_subtitle
                    .as_deref()
                    .is_some_and(|p| p.starts_with("/data/data/com.termux/"));
            let needs_proxy = eager_proxy
                || (is_android
                    && subtitle.is_some()
                    && (local_subtitle.is_none() || android_sub_is_private));

            let mut sidecar_child = None;
            let (effective_link, effective_subtitle) = if needs_proxy {
                let sidecar_outcome = match eager_sidecar_res {
                    Some(res) => res,
                    None => {
                        let link_c = link.clone();
                        let headers_c = headers.clone();
                        let sub_c = sidecar_sub.map(str::to_string);
                        tokio::task::spawn_blocking(move || {
                            crate::proxy::spawn_sidecar(
                                &link_c,
                                &headers_c,
                                sub_c.as_deref(),
                                max_height,
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("join error: {e}")))
                    }
                };
                match sidecar_outcome {
                    Ok((local_url, sc_child)) => {
                        sidecar_child = Some(sc_child);
                        let sub_url = if is_android {
                            if local_subtitle.is_some() && !android_sub_is_private {
                                local_subtitle.clone()
                            } else if let Some(remote_sub) = &subtitle {
                                if let Some(authority) = local_url
                                    .strip_prefix("http://")
                                    .and_then(|s| s.split('/').next())
                                {
                                    let encoded = percent_encoding::utf8_percent_encode(
                                        remote_sub,
                                        percent_encoding::NON_ALPHANUMERIC,
                                    );
                                    Some(format!("http://{authority}/sub/{encoded}"))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            local_subtitle.clone()
                        };
                        (local_url, sub_url)
                    }
                    Err(err) => {
                        if matches!(
                            kind,
                            crate::tui::state::PlayerKind::Mpv
                                | crate::tui::state::PlayerKind::Iina
                        ) {
                            log::warn!(
                                "Failed to spawn stream proxy sidecar ({err}), falling back to direct playback"
                            );
                            (link.clone(), local_subtitle.clone())
                        } else {
                            log::error!("Failed to spawn stream proxy sidecar: {err}");
                            let _ = sender.send(Action::PlayerExited);
                            let _ = sender.send(Action::SetStatus(format!(
                                "Error: Stream proxy failed ({err})"
                            )));
                            return;
                        }
                    }
                }
            } else {
                (link.clone(), local_subtitle.clone())
            };

            let log_dir = std::env::temp_dir().join("moviebox-tui").join("subs");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let log_path = log_dir.join(format!("player_{}_{log_stamp}.log", std::process::id()));

            let spawn_configured_command =
                |mut cmd: std::process::Command, capture_stdout: bool| {
                    cmd.stdin(std::process::Stdio::null());
                    let log_file = std::fs::File::create(&log_path).ok();
                    if capture_stdout {
                        if let Some(cloned) = log_file.as_ref().and_then(|f| f.try_clone().ok()) {
                            cmd.stdout(std::process::Stdio::from(cloned));
                        } else {
                            cmd.stdout(std::process::Stdio::null());
                        }
                    } else {
                        cmd.stdout(std::process::Stdio::null());
                    }
                    if let Some(file) = log_file {
                        cmd.stderr(std::process::Stdio::from(file));
                    } else {
                        cmd.stderr(std::process::Stdio::null());
                    }
                    crate::player::configure_detached_process(&mut cmd);
                    cmd.spawn()
                };
            let command = crate::tui::player::command(
                kind,
                &effective_link,
                effective_subtitle.as_deref(),
                &headers,
                window,
                resume_seconds,
                tracker_ref,
                max_height,
            );
            if kind == crate::tui::state::PlayerKind::Iina
                && crate::player::iina_is_app_fallback()
                && (!headers.is_empty() || subtitle.is_some())
            {
                let _ = sender.send(Action::SetStatus(
                    "Warning: IINA opened without iina-cli: headers and subtitles unavailable."
                        .to_string(),
                ));
            }

            let spawn_result = if is_android {
                let candidates = crate::player::android_intent_commands(
                    &effective_link,
                    effective_subtitle.as_deref(),
                    &headers,
                );
                let mut spawned = None;
                let mut last_err = None;
                for (opener, cmd) in candidates {
                    match spawn_configured_command(cmd, true) {
                        Ok(child) => {
                            log::info!("spawned android opener: {opener:?}");
                            spawned = Some(child);
                            break;
                        }
                        Err(err) => {
                            log::warn!("failed to spawn opener {opener:?}: {err}");
                            last_err = Some(err);
                        }
                    }
                }
                match spawned {
                    Some(child) => Ok(child),
                    None => Err(last_err.unwrap_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "No Android intent tools found",
                        )
                    })),
                }
            } else {
                log::info!("launching player: {kind:?}");
                spawn_configured_command(command, false)
            };

            match spawn_result {
                Ok(mut child) => {
                    let start_time = std::time::Instant::now();
                    let fallback_link = effective_link.clone();
                    let fallback_sub = effective_subtitle.clone();
                    let fallback_headers = headers.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = child.wait();
                        let error_output = std::fs::read_to_string(&log_path).unwrap_or_default();
                        let _ = std::fs::remove_file(&log_path);

                        match &result {
                            Ok(status)
                                if status.success()
                                    || is_vlc_normal_exit(
                                        kind,
                                        status.code(),
                                        error_output.trim(),
                                    )
                                    || is_user_quit(status) =>
                            {
                                log::info!(
                                    "player {kind:?} finished cleanly (code: {:?}, duration: {}s)",
                                    status.code(),
                                    start_time.elapsed().as_secs()
                                );
                                let has_tracker = tracker_opts.is_some()
                                    && matches!(kind, crate::tui::state::PlayerKind::Mpv);

                                if has_tracker {
                                    sender.send(Action::ReconcileHistory).ok();
                                } else if let Some(item) = history_item {
                                    let elapsed = start_time.elapsed().as_secs();
                                    if elapsed >= 30 {
                                        let duration = item.duration_seconds;
                                        let start_pos = resume_seconds.unwrap_or(0);
                                        let total_pos = start_pos.saturating_add(elapsed);
                                        let progress = if let Some(d) = duration {
                                            total_pos.min(d)
                                        } else {
                                            total_pos
                                        };
                                        let completed = duration.is_some_and(|d| {
                                            d > 0 && progress >= (d as f64 * 0.90) as u64
                                        });
                                        sender
                                            .send(Action::UpdateProgress {
                                                item: Box::new(item),
                                                progress,
                                                duration,
                                                completed,
                                            })
                                            .ok();
                                    }
                                }
                            }
                            Ok(status) => {
                                let is_termux_socket_err = is_android
                                    && (error_output.contains("am.sock")
                                        || error_output.contains("Could not connect to socket")
                                        || error_output.contains("termux-am"));

                                if is_termux_socket_err {
                                    let secondary =
                                        crate::player::android_openers()
                                            .iter()
                                            .find(|op| {
                                                matches!(
                                                op,
                                                crate::player::AndroidOpener::TermuxOpen(_)
                                                    | crate::player::AndroidOpener::TermuxOpenUrl(_)
                                            )
                                            })
                                            .cloned();
                                    if let Some(op) = secondary {
                                        log::warn!(
                                            "primary opener failed socket connection, retrying with fallback opener {op:?}"
                                        );
                                        let mut fallback_cmd =
                                            crate::player::android_intent_command_for_opener(
                                                &op,
                                                &fallback_link,
                                                fallback_sub.as_deref(),
                                                &fallback_headers,
                                            );
                                        fallback_cmd.stdin(std::process::Stdio::null());
                                        fallback_cmd.stdout(std::process::Stdio::null());
                                        fallback_cmd.stderr(std::process::Stdio::null());
                                        crate::player::configure_detached_process(
                                            &mut fallback_cmd,
                                        );
                                        if let Ok(mut retry_child) = fallback_cmd.spawn() {
                                            if let Ok(retry_status) = retry_child.wait() {
                                                if retry_status.success() {
                                                    log::info!(
                                                        "fallback android opener {op:?} succeeded"
                                                    );
                                                    if let Some(path) = temporary_subtitle {
                                                        let _ = std::fs::remove_file(path);
                                                    }
                                                    sender.send(Action::PlayerExited).ok();
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }

                                #[cfg(unix)]
                                let signal = {
                                    use std::os::unix::process::ExitStatusExt;
                                    status.signal()
                                };
                                #[cfg(not(unix))]
                                let signal = None;
                                let clean_error =
                                    clean_player_error(status.code(), signal, error_output.trim());
                                sender
                                    .send(Action::PlayerCrashed(status.code(), clean_error))
                                    .ok();
                            }
                            Err(error) => {
                                sender
                                    .send(Action::PlayerCrashed(
                                        None,
                                        format!("Failed waiting for player process: {error}"),
                                    ))
                                    .ok();
                            }
                        }
                        if !is_android
                            && (matches!(kind, crate::tui::state::PlayerKind::Mpv)
                                || start_time.elapsed() >= std::time::Duration::from_secs(3)
                                || result.as_ref().is_ok_and(|s| !s.success()))
                            && let Some(mut sc) = sidecar_child
                        {
                            let _ = sc.kill();
                            let _ = sc.wait();
                        }
                        if start_time.elapsed() >= std::time::Duration::from_secs(3)
                            && let Some(path) = temporary_subtitle
                        {
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
                    let _ = tokio::fs::remove_file(&log_path).await;
                    if let Some(mut sc) = sidecar_child {
                        let _ = sc.kill();
                        let _ = sc.wait();
                    }
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
fn is_vlc_normal_exit(
    kind: crate::tui::state::PlayerKind,
    code: Option<i32>,
    stderr: &str,
) -> bool {
    matches!(kind, crate::tui::state::PlayerKind::Vlc)
        && (code == Some(1) || code == Some(0))
        && stderr.is_empty()
}

fn is_user_quit(status: &std::process::ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal() == Some(15)
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        false
    }
}

fn clean_player_error(code: Option<i32>, signal: Option<i32>, stderr: &str) -> String {
    let trimmed = stderr.trim();
    if !trimmed.is_empty() {
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("403 forbidden") || lower.contains("http error 403") {
            return "Stream forbidden by server (HTTP 403).".to_string();
        }
        if lower.contains("404 not found") || lower.contains("http error 404") {
            return "Stream link not found (HTTP 404).".to_string();
        }
        if lower.contains("410 gone") || lower.contains("http error 410") {
            return "Stream link expired (HTTP 410).".to_string();
        }
        if lower.contains("connection refused") {
            return "Stream connection refused.".to_string();
        }
        if lower.contains("timed out") || lower.contains("timeout") {
            return "Stream connection timed out.".to_string();
        }
        if lower.contains("unrecognized option") || lower.contains("unknown option") {
            return "Unsupported player CLI option.".to_string();
        }
        let best_line = trimmed
            .lines()
            .rev()
            .map(str::trim)
            .find(|l| {
                !l.is_empty()
                    && (l.contains("error")
                        || l.contains("Error")
                        || l.contains("ERROR")
                        || l.contains("failed")
                        || l.contains("Failed"))
            })
            .or_else(|| trimmed.lines().rev().map(str::trim).find(|l| !l.is_empty()))
            .unwrap_or(trimmed);
        let bounded = if best_line.len() > 120 {
            let mut end = 120;
            while end > 0 && !best_line.is_char_boundary(end) {
                end -= 1;
            }
            &best_line[..end]
        } else {
            best_line
        };
        return bounded.to_string();
    }

    if let Some(value) = code {
        format!("Player exited with status code {value}.")
    } else if let Some(sig) = signal {
        format!("Player terminated by signal {sig}.")
    } else {
        "Player exited unsuccessfully without error output.".to_string()
    }
}

impl App {
    pub(super) async fn handle_playback(&mut self, action: Action) -> Option<()> {
        match action {
            Action::PlayStream => {
                if self.state.is_playing {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Playback active",
                        "Player is already running.",
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
                    || self.current_subject_provider() == ProviderKind::Dramachi
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
                            format!("Resolving {}...", first_mirror.label),
                        );
                        let max_height = Some(release.resolution_u64()).filter(|&h| h > 0);
                        let direct_source = crate::providers::models::PlaybackSource {
                            provider: release.provider,
                            url: first_mirror.resolver_url.clone(),
                            headers: first_mirror.headers.clone(),
                            subtitle: None,
                            source_label: first_mirror.label.clone(),
                            max_height,
                        };
                        let client = if release.provider == ProviderKind::Addons
                            || release.provider == ProviderKind::Dramachi
                            || release.provider == ProviderKind::BdixCircleFtp
                            || release.provider == ProviderKind::BdixDhakaFlix
                        {
                            self.dispatch_playback_or_notify(direct_source);
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
                        self.request_tasks.spawn_playback_resolve(async move {
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(18),
                                client.resolve_release(
                                    &release,
                                    crate::providers::ResolutionIntent::Playback,
                                ),
                            )
                            .await;
                            match result {
                                Ok(Ok(source)) => {
                                    sender.send(Action::DispatchPlayback(source)).ok();
                                }
                                Ok(Err(error)) => {
                                    log::error!("4KHDHub resolve failed: {error}");
                                    sender.send(Action::PlayerExited).ok();
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Error: 4KHDHub: {}",
                                            error.user_message()
                                        )))
                                        .ok();
                                }
                                Err(_) => {
                                    log::error!("4KHDHub resolve timed out");
                                    sender.send(Action::PlayerExited).ok();
                                    sender
                                        .send(Action::SetStatus(
                                            "Error: 4KHDHub: Timed out.".to_string(),
                                        ))
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
                    && let Some(release) = self.get_selected_release()
                {
                    let Some(first_mirror) = release.mirrors.first().cloned() else {
                        self.state.is_resolving_playback = false;
                        self.state.notify(
                            NotificationKind::Error,
                            "Playback unavailable",
                            "No playable mirrors were found for this release.",
                        );
                        return None;
                    };
                    let max_height = Some(release.resolution_u64()).filter(|&h| h > 0);
                    let direct_source = crate::providers::models::PlaybackSource {
                        provider: release.provider,
                        url: first_mirror.resolver_url.clone(),
                        headers: first_mirror.headers.clone(),
                        subtitle: None,
                        source_label: first_mirror.label.clone(),
                        max_height,
                    };
                    let subject_id = self.state.active_subject_id.clone().unwrap_or_default();
                    let resource_id = self.get_selected_resource_id();

                    if let Some(rid) = resource_id {
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing playback",
                            format!("Preparing {}...", release.filename),
                        );
                        self.state.pending_playback_source = Some(direct_source.clone());
                        let service = self.service.clone();
                        let sender = self.action_sender.clone();
                        let source_clone = direct_source.clone();
                        let sibling_ids: Vec<String> = self
                            .state
                            .selected_details
                            .as_ref()
                            .map(|d| d.sibling_ids())
                            .unwrap_or_default();
                        let season = self.state.selected_season;
                        let episode = self.state.selected_episode;
                        self.request_tasks.spawn_playback_resolve(async move {
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(15),
                                service.get_ext_captions(
                                    &subject_id,
                                    &rid,
                                    &sibling_ids,
                                    season,
                                    episode,
                                ),
                            )
                            .await;
                            match result {
                                Ok(Ok(res)) => {
                                    sender
                                        .send(Action::ShowSubtitlePopup(source_clone.url, res))
                                        .ok();
                                }
                                Err(_) => {
                                    log::warn!(
                                        "[playback] Subtitle resolution timed out after 15s for rid={rid}"
                                    );
                                    sender.send(Action::DispatchPlayback(source_clone)).ok();
                                }
                                Ok(Err(err)) => {
                                    log::warn!(
                                        "[playback] Subtitle resolution failed for rid={rid}: {err}"
                                    );
                                    sender.send(Action::DispatchPlayback(source_clone)).ok();
                                }
                            }
                        });
                    } else {
                        self.dispatch_playback_or_notify(direct_source);
                    }
                } else {
                    self.state.is_resolving_playback = false;
                }
            }
            Action::ShowSubtitlePopup(link, subtitles) => {
                self.state.is_resolving_playback = false;
                let mut options = vec![("None".to_string(), "".to_string())];
                options.extend(subtitles.into_iter().map(|s| (s.name, s.url)));
                if options.len() > 1 {
                    self.state.show_help = false;
                    self.state.show_overview_modal = false;
                    self.state.player_picker_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.subtitle_popup = true;
                    self.state.subtitle_list = options;
                    self.state.subtitle_list_state.select(Some(0));
                    self.state.pending_play_link = Some(link);
                } else {
                    if let Some(source) = self.state.pending_playback_source.take() {
                        self.dispatch_playback_or_notify(source);
                    } else {
                        let source = crate::providers::models::PlaybackSource {
                            provider: self.state.active_provider,
                            url: link,
                            headers: vec![(
                                "User-Agent".to_string(),
                                self.service.client.user_agent().to_string(),
                            )],
                            subtitle: None,
                            source_label: "Direct".to_string(),
                            max_height: None,
                        };
                        self.dispatch_playback_or_notify(source);
                    }
                }
            }
            Action::ShowDownloadSubtitlePopup(subtitles) => {
                self.state.is_resolving_playback = false;
                let mut options = vec![("None".to_string(), "".to_string())];
                options.extend(subtitles.into_iter().map(|s| (s.name, s.url)));
                if options.len() > 1 {
                    self.state.show_help = false;
                    self.state.show_overview_modal = false;
                    self.state.player_picker_popup = false;
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = true;
                    self.state.subtitle_list = options;
                    self.state.subtitle_list_state.select(Some(0));
                } else {
                    self.action_sender.send(Action::DownloadStream(None)).ok();
                }
            }

            Action::LaunchPlayback(kind, source) => {
                self.state.is_resolving_playback = false;
                self.state.player_picker_popup = false;
                self.state.last_playback_launch = std::time::Instant::now();
                if !crate::tui::player::supports_headers(kind, &source.headers) {
                    self.state.notify(
                        NotificationKind::Error,
                        format!("{} Incompatible", kind.label()),
                        format!("{} lacks stream header support.", kind.label()),
                    );
                    return None;
                }
                self.launch_player(
                    kind,
                    source.url,
                    source.subtitle,
                    source.headers,
                    source.max_height,
                );
            }
            Action::DispatchPlayback(source) => {
                self.dispatch_playback_or_notify(source);
            }
            Action::MarkWatched(item) => {
                self.state.history.mark_watched(*item);
            }
            Action::UpdateProgress {
                item,
                progress,
                duration,
                completed,
            } => {
                self.state
                    .history
                    .update_progress(*item, progress, duration, completed);
            }
            Action::ReconcileHistory => {
                self.state.history.reconcile_pending_playback_states();
            }
            Action::PlayerExited => {
                self.state.history.reconcile_pending_playback_states();
                self.state.is_playing = false;
                self.state.is_resolving_playback = false;
            }
            Action::PlayerCrashed(code, error_msg) => {
                self.state.history.reconcile_pending_playback_states();
                self.state.is_playing = false;
                self.state.is_resolving_playback = false;
                let code_str = code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into());
                log::error!("player crashed (code {code_str}): {error_msg}");

                let is_termux = crate::config::is_termux_environment();
                let error_lower = error_msg.to_ascii_lowercase();
                let is_missing_activity = is_termux
                    && (error_lower.contains("no activity found")
                        || error_lower.contains("activitynotfoundexception"));
                let is_termux_tool_crash = is_termux
                    && (code == Some(126)
                        || error_msg.contains("Permission denied")
                        || error_msg.contains("/system/bin/am")
                        || error_msg.contains("termux-open")
                        || error_msg.contains("termux-am")
                        || error_msg.contains("am.sock")
                        || error_msg.contains("Could not connect to socket")
                        || (code == Some(1)
                            && (error_msg.is_empty()
                                || error_msg.contains("status code 1")
                                || error_msg.contains("broadcast"))));

                let is_headless_mpv = is_termux
                    && (error_lower.contains("failed to open display")
                        || error_lower.contains("video_out")
                        || error_lower.contains("vo/gpu")
                        || error_lower.contains("vo=gpu"));
                let (title, message) = if is_missing_activity {
                    ("No Player", "Install a video player.".to_string())
                } else if is_headless_mpv {
                    (
                        "CLI mpv",
                        "Switch to Android Player in /settings.".to_string(),
                    )
                } else if is_termux_tool_crash {
                    (
                        "Termux Setup",
                        "Missing termux-tools package in Termux.".to_string(),
                    )
                } else {
                    let has_specific =
                        !error_msg.is_empty() && !error_msg.starts_with("Player exited");
                    match code {
                        Some(2) => (
                            "Stream Error",
                            if has_specific {
                                error_msg
                            } else {
                                "Player failed to open stream (exit code 2).".to_string()
                            },
                        ),
                        Some(1) => (
                            "Player Error",
                            if has_specific {
                                error_msg
                            } else {
                                "Player exited with error code 1.".to_string()
                            },
                        ),
                        Some(c) => (
                            "Playback Failed",
                            if has_specific {
                                error_msg
                            } else {
                                format!("Player exited ({c}).")
                            },
                        ),
                        None => (
                            "Playback Failed",
                            if has_specific {
                                error_msg
                            } else {
                                "Player terminated.".to_string()
                            },
                        ),
                    }
                };

                self.state.notify(NotificationKind::Error, title, message);
            }
            _ => return None,
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::clean_player_error;

    #[test]
    fn failed_player_with_stderr_keeps_diagnostic() {
        assert_eq!(
            clean_player_error(Some(1), None, "VLC failed to open the stream"),
            "VLC failed to open the stream"
        );
    }

    #[test]
    fn failed_player_without_stderr_still_reports_failure() {
        assert_eq!(
            clean_player_error(Some(1), None, ""),
            "Player exited with status code 1."
        );
        assert_eq!(
            clean_player_error(None, Some(9), ""),
            "Player terminated by signal 9."
        );
        assert_eq!(
            clean_player_error(None, None, ""),
            "Player exited unsuccessfully without error output."
        );
    }
    #[test]
    fn player_exit_code_interpretation() {
        assert_eq!(
            clean_player_error(Some(2), None, "ffmpeg/http: HTTP error 403 Forbidden"),
            "Stream forbidden by server (HTTP 403)."
        );
        assert_eq!(
            clean_player_error(Some(2), None, "Connection timed out"),
            "Stream connection timed out."
        );
    }

    #[test]
    fn vlc_exit_code_1_empty_stderr_is_normal_exit() {
        use crate::tui::state::PlayerKind;
        assert!(super::is_vlc_normal_exit(PlayerKind::Vlc, Some(1), ""));
        assert!(super::is_vlc_normal_exit(PlayerKind::Vlc, Some(0), ""));
        assert!(!super::is_vlc_normal_exit(
            PlayerKind::Vlc,
            Some(1),
            "Error opening stream"
        ));
        assert!(!super::is_vlc_normal_exit(PlayerKind::Mpv, Some(1), ""));
        assert!(!super::is_vlc_normal_exit(PlayerKind::Vlc, Some(2), ""));
    }

    #[tokio::test]
    async fn show_popup_when_subtitles_available() {
        let mut app = crate::tui::app::App::new();

        let ext_captions = vec![
            crate::providers::models::SubtitleOption {
                name: "Spanish".to_string(),
                url: "https://example.com/es.srt".to_string(),
            },
            crate::providers::models::SubtitleOption {
                name: "French".to_string(),
                url: "https://example.com/fr.srt".to_string(),
            },
        ];

        app.handle_playback(crate::tui::action::Action::ShowSubtitlePopup(
            "https://example.com/video.mp4".to_string(),
            ext_captions,
        ))
        .await;

        assert!(app.state.subtitle_popup);
        assert_eq!(app.state.subtitle_list.len(), 3);
    }

    #[tokio::test]
    async fn test_get_selected_resource_id_resolution() {
        let mut app = crate::tui::app::App::new();
        assert_eq!(app.get_selected_resource_id(), None);

        app.state.selected_resources = vec![
            crate::providers::models::Release {
                provider: crate::providers::models::ProviderKind::MovieBox,
                filename: "Movie.1080p.mkv".to_string(),
                quality: Some("1080p".to_string()),
                codec: Some("hevc".to_string()),
                language: None,
                size_bytes: Some(1024),
                season: None,
                episode: None,
                mirrors: vec![],
                resource_id: Some("167282974499786072".to_string()),
            },
            crate::providers::models::Release {
                provider: crate::providers::models::ProviderKind::MovieBox,
                filename: "Movie.720p.mkv".to_string(),
                quality: Some("720p".to_string()),
                codec: Some("h264".to_string()),
                language: None,
                size_bytes: Some(512),
                season: None,
                episode: None,
                mirrors: vec![],
                resource_id: None,
            },
        ];

        app.state.resource_list_state.select(Some(0));
        assert_eq!(
            app.get_selected_resource_id().as_deref(),
            Some("167282974499786072")
        );

        app.state.resource_list_state.select(Some(1));
        assert_eq!(app.get_selected_resource_id(), None);
    }

    #[tokio::test]
    async fn test_play_stream_notifies_preparing_playback_accurately() {
        let mut app = crate::tui::app::App::new();
        app.state.active_provider = crate::providers::models::ProviderKind::MovieBox;
        app.state.active_screen = crate::tui::state::Screen::Details;
        app.state.selected_resources = vec![crate::providers::models::Release {
            provider: crate::providers::models::ProviderKind::MovieBox,
            filename: "Movie.1080p.mkv".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("hevc".to_string()),
            language: None,
            size_bytes: Some(1024),
            season: None,
            episode: None,
            mirrors: vec![crate::providers::models::SourceMirror {
                label: "Direct".to_string(),
                resolver_url: "https://example.com/video.mp4".to_string(),
                headers: vec![],
                direct_file: true,
            }],
            resource_id: Some("12345".to_string()),
        }];
        app.state.resource_list_state.select(Some(0));

        app.handle_playback(crate::tui::action::Action::PlayStream)
            .await;

        let notif = app.state.notifications.back().expect("notification posted");
        assert_eq!(notif.title, "Preparing playback");
        assert_eq!(notif.message, "Preparing Movie.1080p.mkv...");
    }

    #[test]
    fn test_subtitle_popup_renders_in_app_draw() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut app = crate::tui::app::App::new();

        app.state.subtitle_popup = true;
        app.state.subtitle_list = vec![
            ("None".to_string(), String::new()),
            (
                "English".to_string(),
                "https://example.com/en.srt".to_string(),
            ),
        ];
        app.state.subtitle_list_state.select(Some(0));

        terminal.draw(|frame| app.draw(frame)).unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(content.contains("Subtitles"));
        assert!(content.contains("No subtitles"));
        assert!(content.contains("English"));
        let items = vec!["No subtitles".to_string(), "English".to_string()];
        let popup_layout = crate::tui::overlay::picker_layout(
            ratatui::layout::Rect::new(0, 0, 80, 24),
            &items,
            "Use",
            20,
        );
        assert_eq!(popup_layout.height, 4);
        assert_eq!(popup_layout.width, 20);
    }

    #[tokio::test]
    async fn test_playback_resolving_lock_resets_on_incompatible_player() {
        let mut app = crate::tui::app::App::new();
        app.state.is_resolving_playback = true;
        app.state.available_players = vec![crate::tui::state::PlayerKind::AndroidIntent];

        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![("Cookie".to_string(), "CloudFront-Policy=test".to_string())],
            subtitle: None,
            source_label: "Multi-Res".to_string(),
            max_height: None,
        };

        assert_eq!(
            app.resolve_playback_player(&source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );
    }
    #[tokio::test]
    async fn test_dispatch_playback_notifies_bad_player_path() {
        let mut app = crate::tui::app::App::new();
        app.state.available_players.clear();
        app.state.default_player = Some("vlc".to_string());
        app.state.vlc_path = Some("D:\\PortableApps\\VLC\\vlc.exe".to_string());
        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/video.mp4".to_string(),
            headers: vec![],
            subtitle: None,
            source_label: "Direct".to_string(),
            max_height: None,
        };

        app.dispatch_playback_or_notify(source);

        let notif = app.state.notifications.back().expect("notification pushed");
        assert_eq!(notif.title, "Invalid Player Path");
        assert_eq!(
            notif.message,
            "Player path not found: D:\\PortableApps\\VLC\\vlc.exe"
        );
    }
    #[tokio::test]
    async fn test_explicit_player_incompatible_does_not_launch_alternative() {
        let mut app = crate::tui::app::App::new();
        app.state.available_players = vec![
            crate::tui::state::PlayerKind::Mpv,
            crate::tui::state::PlayerKind::AndroidIntent,
        ];
        app.state.default_player = Some("android".to_string());

        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![("Cookie".to_string(), "CloudFront-Policy=test".to_string())],
            subtitle: None,
            source_label: "Multi-Res".to_string(),
            max_height: None,
        };

        let resolution = app.resolve_playback_player(&source);
        assert_eq!(
            resolution,
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );

        app.state.is_resolving_playback = true;
        app.dispatch_playback_or_notify(source);
        assert!(app.state.pending_playback_source.is_none());
    }

    #[tokio::test]
    async fn test_vlc_resolves_as_available_for_cookie_source() {
        let mut app = crate::tui::app::App::new();
        app.state.available_players = vec![
            crate::tui::state::PlayerKind::Mpv,
            crate::tui::state::PlayerKind::Vlc,
        ];
        app.state.default_player = Some("vlc".to_string());

        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![("Cookie".to_string(), "CloudFront-Policy=test".to_string())],
            subtitle: None,
            source_label: "Multi-Res".to_string(),
            max_height: None,
        };

        let resolution = app.resolve_playback_player(&source);
        assert_eq!(
            resolution,
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::Vlc)
        );
    }
    #[tokio::test]
    async fn test_playback_resolving_lock_resets_on_player_crash() {
        let mut app = crate::tui::app::App::new();
        app.state.is_resolving_playback = true;
        app.state.is_playing = true;

        app.handle_playback(crate::tui::action::Action::PlayerCrashed(
            Some(1),
            "failed".to_string(),
        ))
        .await;

        assert!(!app.state.is_resolving_playback);
        assert!(!app.state.is_playing);
    }
    #[tokio::test]
    async fn test_android_player_allows_unauthenticated_and_referer_streams() {
        let mut app = crate::tui::app::App::new();
        app.state.available_players = vec![crate::tui::state::PlayerKind::AndroidIntent];
        app.state.default_player = Some("android".to_string());

        let bdix_source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::BdixCircleFtp,
            url: "http://10.16.100.244/movies/film.mkv".to_string(),
            headers: vec![],
            subtitle: None,
            source_label: "CircleFTP".to_string(),
            max_height: None,
        };
        assert_eq!(
            app.resolve_playback_player(&bdix_source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );

        let fourk_source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::FourKHdHub,
            url: "https://r2.example.com/stream.mkv".to_string(),
            headers: vec![
                ("Referer".to_string(), "https://hubcloud.one/".to_string()),
                ("User-Agent".to_string(), "Mozilla/5.0".to_string()),
            ],
            subtitle: None,
            source_label: "1080p".to_string(),
            max_height: None,
        };
        assert_eq!(
            app.resolve_playback_player(&fourk_source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );

        let auth_source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![("Cookie".to_string(), "CloudFront-Policy=test".to_string())],
            subtitle: None,
            source_label: "Multi-Res".to_string(),
            max_height: None,
        };
        assert_eq!(
            app.resolve_playback_player(&auth_source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );
    }

    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    #[tokio::test]
    async fn test_graphical_session_overrides_stale_android_default_player() {
        let _guard = ENV_LOCK.lock().await;
        let orig_display = std::env::var("DISPLAY").ok();
        let orig_player = std::env::var(crate::player::ENV_MOVIEBOX_PLAYER).ok();
        unsafe {
            std::env::set_var("DISPLAY", ":0.0");
            std::env::remove_var(crate::player::ENV_MOVIEBOX_PLAYER);
        }
        let mut app = crate::tui::app::App::new();
        app.state.available_players = vec![
            crate::tui::state::PlayerKind::Mpv,
            crate::tui::state::PlayerKind::Vlc,
            crate::tui::state::PlayerKind::AndroidIntent,
        ];
        app.state.default_player = Some("android".to_string());
        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![],
            subtitle: None,
            source_label: "1080p".to_string(),
            max_height: None,
        };
        assert_eq!(
            app.resolve_playback_player(&source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::Mpv)
        );
        unsafe {
            if let Some(val) = orig_display {
                std::env::set_var("DISPLAY", val);
            } else {
                std::env::remove_var("DISPLAY");
            }
            if let Some(val) = orig_player {
                std::env::set_var(crate::player::ENV_MOVIEBOX_PLAYER, val);
            } else {
                std::env::remove_var(crate::player::ENV_MOVIEBOX_PLAYER);
            }
        }
    }

    #[tokio::test]
    async fn test_graphical_session_respects_explicit_moviebox_player_env() {
        let _guard = ENV_LOCK.lock().await;
        let orig_display = std::env::var("DISPLAY").ok();
        let orig_player = std::env::var(crate::player::ENV_MOVIEBOX_PLAYER).ok();
        unsafe {
            std::env::set_var("DISPLAY", ":0.0");
            std::env::set_var(crate::player::ENV_MOVIEBOX_PLAYER, "android");
        }
        let mut app = crate::tui::app::App::new();
        app.state.available_players = vec![
            crate::tui::state::PlayerKind::Mpv,
            crate::tui::state::PlayerKind::Vlc,
            crate::tui::state::PlayerKind::AndroidIntent,
        ];
        app.state.default_player = Some("mpv".to_string());
        let source = crate::providers::models::PlaybackSource {
            provider: crate::providers::models::ProviderKind::MovieBox,
            url: "https://example.com/index.mpd".to_string(),
            headers: vec![],
            subtitle: None,
            source_label: "1080p".to_string(),
            max_height: None,
        };
        assert_eq!(
            app.resolve_playback_player(&source),
            super::PlaybackResolution::Available(crate::tui::state::PlayerKind::AndroidIntent)
        );
        unsafe {
            if let Some(val) = orig_display {
                std::env::set_var("DISPLAY", val);
            } else {
                std::env::remove_var("DISPLAY");
            }
            if let Some(val) = orig_player {
                std::env::set_var(crate::player::ENV_MOVIEBOX_PLAYER, val);
            } else {
                std::env::remove_var(crate::player::ENV_MOVIEBOX_PLAYER);
            }
        }
    }

    #[tokio::test]
    async fn test_player_crashed_termux_actionable_notification() {
        let mut app = crate::tui::app::App::new();
        let _guard = ENV_LOCK.lock().await;
        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
        }
        app.handle_playback(super::Action::PlayerCrashed(
            Some(126),
            "/system/bin/am[11]: /data/data/com.termux/files/usr/bin/cmd: Permission denied"
                .to_string(),
        ))
        .await;
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
        }
        let last_notification = app
            .state
            .notifications
            .back()
            .expect("expected notification");
        assert_eq!(last_notification.title, "Termux Setup");
        assert_eq!(
            last_notification.message,
            "Missing termux-tools package in Termux."
        );
    }

    #[tokio::test]
    async fn test_player_crashed_termux_missing_activity() {
        let mut app = crate::tui::app::App::new();
        let _guard = ENV_LOCK.lock().await;
        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
        }
        app.handle_playback(super::Action::PlayerCrashed(
            Some(1),
            "Error: Activity not started, no activity found to handle Intent".to_string(),
        ))
        .await;
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
        }
        let last_notification = app
            .state
            .notifications
            .back()
            .expect("expected notification");
        assert_eq!(last_notification.title, "No Player");
        assert_eq!(last_notification.message, "Install a video player.");
    }

    #[tokio::test]
    async fn test_player_crashed_termux_headless_mpv() {
        let mut app = crate::tui::app::App::new();
        let _guard = ENV_LOCK.lock().await;
        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
        }
        app.handle_playback(super::Action::PlayerCrashed(
            Some(1),
            "Error opening/initializing the selected video_out (--vo) device.".to_string(),
        ))
        .await;
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
        }
        let last_notification = app
            .state
            .notifications
            .back()
            .expect("expected notification");
        assert_eq!(last_notification.title, "CLI mpv");
        assert_eq!(
            last_notification.message,
            "Switch to Android Player in /settings."
        );
    }

    #[tokio::test]
    async fn test_player_crashed_termux_exit_code_1_generic() {
        let mut app = crate::tui::app::App::new();
        let _guard = ENV_LOCK.lock().await;
        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
        }
        app.handle_playback(super::Action::PlayerCrashed(
            Some(1),
            "Player exited with status code 1.".to_string(),
        ))
        .await;
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
        }
        let last_notification = app
            .state
            .notifications
            .back()
            .expect("expected notification");
        assert_eq!(last_notification.title, "Termux Setup");
        assert_eq!(
            last_notification.message,
            "Missing termux-tools package in Termux."
        );
    }

    #[tokio::test]
    async fn test_playback_captures_active_subject_and_episode() {
        let mut app = crate::tui::app::App::new();
        app.state.active_screen = crate::tui::state::Screen::Details;
        app.state.selected_season = 2;
        app.state.selected_episode = 5;
        app.state.active_subject_id = Some("dub_subject_42".to_string());
        app.state.selected_details = Some(crate::providers::models::MediaDetails {
            id: crate::providers::models::ProviderMediaId {
                provider: crate::providers::models::ProviderKind::MovieBox,
                value: "root_subject_100".to_string(),
            },
            title: "Test Series".to_string(),
            media_type: crate::models::MediaType::Series,
            year: None,
            description: None,
            tagline: None,
            imdb_rating: None,
            director: None,
            stars: None,
            prints: None,
            audios: None,
            poster_url: None,
            duration: None,
            genres: Vec::new(),
            seasons: Vec::new(),
            dubs: Vec::new(),
        });
        let mirror = crate::providers::models::SourceMirror {
            label: "1080p".to_string(),
            resolver_url: "https://example.com/video.mp4".to_string(),
            headers: Vec::new(),
            direct_file: true,
        };
        app.state.selected_resources = vec![crate::providers::models::Release {
            provider: crate::providers::models::ProviderKind::MovieBox,
            filename: "Test S02E05 1080p".to_string(),
            quality: Some("1080p".to_string()),
            codec: None,
            language: None,
            size_bytes: None,
            season: Some(2),
            episode: Some(5),
            mirrors: vec![mirror],
            resource_id: Some("res_s2e5".to_string()),
        }];
        app.state.resource_list_state.select(Some(0));
        assert_eq!(
            app.state.active_subject_id.as_deref(),
            Some("dub_subject_42")
        );
        assert_eq!(app.get_selected_resource_id().as_deref(), Some("res_s2e5"));
        assert_eq!(app.state.selected_season, 2);
        assert_eq!(app.state.selected_episode, 5);
    }

    #[tokio::test]
    async fn test_non_mpv_update_progress_is_retained_across_player_exited() {
        let mut app = crate::tui::app::App::new();
        let item = crate::history::WatchHistoryItem {
            provider: "moviebox".to_string(),
            subject_id: "vlc_retain_test".to_string(),
            title: "VLC Retention Movie".to_string(),
            cover_url: None,
            stype: 1,
            release_year: "2026".to_string(),
            season: 0,
            episode: 0,
            progress_seconds: 0,
            duration_seconds: Some(3600),
            completed: false,
            timestamp: 1000,
            stream_filename: Some("Movie.1080p.mkv".to_string()),
        };
        app.state.history.record_start(&item, 0);
        app.handle_playback(crate::tui::action::Action::UpdateProgress {
            item: Box::new(item),
            progress: 420,
            duration: Some(3600),
            completed: false,
        })
        .await;
        app.handle_playback(crate::tui::action::Action::PlayerExited)
            .await;

        let saved = app
            .state
            .history
            .get_item("moviebox", "vlc_retain_test", 0, 0, None)
            .expect("history item exists");
        assert_eq!(saved.progress_seconds, 420);
    }
}
