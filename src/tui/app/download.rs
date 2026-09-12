use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::{action::Action, overlay::NotificationKind, state::Screen};

impl App {
    pub(super) fn resolve_download_base_dir(&self) -> std::path::PathBuf {
        crate::service::resolve_download_dir(self.state.download_dir.as_deref())
    }

    pub(super) fn start_resilient_download(
        &mut self,
        subtitle_url: Option<String>,
        link: Option<String>,
        headers: Vec<(String, String)>,
    ) {
        if self.state.download_progress.is_some() || self.state.active_screen != Screen::Details {
            return;
        }
        let Some(link) = link else {
            if self.state.is_fetching_streams {
                self.state.is_waiting_for_download_stream = true;
                self.state.notify(
                    NotificationKind::Info,
                    "Preparing download",
                    "Waiting for stream details.",
                );
            } else {
                self.state.notify(
                    NotificationKind::Warning,
                    "Download unavailable",
                    "Select a downloadable stream first.",
                );
            }
            return;
        };

        let raw_title = self
            .state
            .selected_details
            .as_ref()
            .map(|details| details.title.as_str())
            .unwrap_or(crate::download::DEFAULT_STREAM_NAME);
        let clean_title = crate::providers::moviebox::clean_moviebox_title(raw_title);
        let is_series = self
            .state
            .selected_details
            .as_ref()
            .is_some_and(|d| d.is_series())
            || !self.state.available_seasons.is_empty();
        let season = self.state.selected_season;
        let episode = self.state.selected_episode;
        let safe_title = crate::download::safe_file_stem(&clean_title);

        let extension = link
            .split('?')
            .next()
            .and_then(|path| path.rsplit('.').next())
            .filter(|ext| {
                let lower = ext.to_ascii_lowercase();
                matches!(lower.as_str(), "mp4" | "mkv" | "webm" | "ts")
            })
            .unwrap_or("mp4")
            .to_ascii_lowercase();

        let base_dir = self.resolve_download_base_dir();
        let (target_dir, base_name) = if is_series {
            (
                base_dir
                    .join("Series")
                    .join(&safe_title)
                    .join(format!("Season {season}")),
                format!("{safe_title} - S{season:02}E{episode:02}"),
            )
        } else {
            (
                base_dir.join("Movies").join(&safe_title),
                safe_title.clone(),
            )
        };
        let destination = target_dir.join(format!("{base_name}.{extension}"));
        if is_media_already_downloaded(&target_dir, &base_name) {
            self.state.is_waiting_for_download_stream = false;
            self.state.notify(
                NotificationKind::Warning,
                "Already downloaded",
                format!("{base_name} already exists on disk."),
            );
            return;
        }

        let sub_lang = self
            .state
            .last_download_subtitle_language
            .take()
            .or_else(|| self.state.season_subtitle_preference.clone().flatten());

        self.state.is_waiting_for_download_stream = false;
        self.state.download_title = Some(base_name.clone());
        self.state.download_status = Some("Preparing download...".into());
        self.state.download_progress = Some(0.0);
        self.state
            .cancel_download
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.state.notify(
            NotificationKind::Info,
            "Download Started",
            format!("Downloading {base_name}.{extension} (resumable)."),
        );

        let cancel = self.state.cancel_download.clone();
        let sender = self.action_sender.clone();
        let user_agent = self.service.client.user_agent().to_string();

        let mut client_builder = crate::net::http_client_builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .tcp_keepalive(std::time::Duration::from_secs(30));

        let mut has_custom_ua = false;
        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in &headers {
            if k.eq_ignore_ascii_case("user-agent") {
                has_custom_ua = true;
                client_builder = client_builder.user_agent(v);
            } else if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                header_map.insert(name, val);
            }
        }
        if !has_custom_ua {
            client_builder = client_builder.user_agent(user_agent);
        }
        let client = client_builder
            .default_headers(header_map)
            .build()
            .unwrap_or_else(|err| {
                log::warn!(
                    "failed to build custom download client ({err}), falling back to default"
                );
                self.service.http_client().clone()
            });

        let is_dash = link.ends_with(".mpd") || link.contains("/dash/");

        tokio::spawn(async move {
            if let Err(error) = tokio::fs::create_dir_all(&target_dir).await {
                sender
                    .send(Action::DownloadFailed(format!(
                        "Cannot create download directory: {error}"
                    )))
                    .ok();
                return;
            }

            if let Some(subtitle_url) = subtitle_url {
                let subtitle_extension = subtitle_url
                    .rsplit('.')
                    .next()
                    .map(|extension| extension.to_ascii_lowercase())
                    .filter(|extension| {
                        matches!(extension.as_str(), "srt" | "vtt" | "ass" | "ssa" | "sub")
                    })
                    .unwrap_or_else(|| "srt".to_string());

                let lang_code = sub_lang
                    .as_deref()
                    .and_then(crate::providers::moviebox::title::language_to_code)
                    .or_else(|| subtitle_language_from_url(&subtitle_url));

                let final_ext = if let Some(code) = lang_code {
                    format!("{code}.{subtitle_extension}")
                } else {
                    subtitle_extension
                };

                let subtitle_path = destination.with_extension(final_ext);
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.get(subtitle_url).send(),
                )
                .await;
                match result {
                    Ok(Ok(response)) => match response.error_for_status() {
                        Ok(response) => match response.bytes().await {
                            Ok(bytes) => {
                                if let Err(error) = tokio::fs::write(subtitle_path, bytes).await {
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Error: subtitle write failed: {error}"
                                        )))
                                        .ok();
                                }
                            }
                            Err(error) => {
                                sender
                                    .send(Action::SetStatus(format!(
                                        "Error: subtitle download failed: {error}"
                                    )))
                                    .ok();
                            }
                        },
                        Err(error) => {
                            sender
                                .send(Action::SetStatus(format!(
                                    "Error: subtitle download failed: {error}"
                                )))
                                .ok();
                        }
                    },
                    Ok(Err(error)) => {
                        sender
                            .send(Action::SetStatus(format!(
                                "Error: subtitle download failed: {error}"
                            )))
                            .ok();
                    }
                    Err(_) => {
                        sender
                            .send(Action::SetStatus(
                                "Error: subtitle download timed out".to_string(),
                            ))
                            .ok();
                    }
                }
            }

            if is_dash {
                let Some(ytdlp_bin) = crate::player::find_in_path("yt-dlp") else {
                    sender
                        .send(Action::DownloadFailed(yt_dlp_missing_guidance()))
                        .ok();
                    return;
                };

                let mut cmd = tokio::process::Command::new(ytdlp_bin);
                for (k, v) in &headers {
                    if k.eq_ignore_ascii_case("user-agent") {
                        cmd.arg("--user-agent").arg(v);
                    } else {
                        cmd.arg("--add-header").arg(format!("{k}: {v}"));
                    }
                }
                cmd.arg("-f")
                    .arg("bestvideo+bestaudio/best")
                    .arg("--newline")
                    .arg("--part")
                    .arg("-o")
                    .arg(&destination)
                    .arg("--force-overwrites")
                    .arg(&link);
                #[cfg(target_os = "windows")]
                {
                    cmd.creation_flags(crate::player::CREATE_NO_WINDOW);
                }

                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());

                let mut child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(err) => {
                        sender
                            .send(Action::DownloadFailed(format!(
                                "Failed to start yt-dlp: {err}"
                            )))
                            .ok();
                        return;
                    }
                };

                if let Some(stderr) = child.stderr.take() {
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let mut reader = tokio::io::BufReader::new(stderr).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            log::debug!("yt-dlp stderr: {line}");
                        }
                    });
                }

                if let Some(stdout) = child.stdout.take() {
                    use tokio::io::AsyncBufReadExt;
                    let mut reader = tokio::io::BufReader::new(stdout).lines();
                    let mut stream_index: usize = 0;
                    let mut max_progress: f64 = 0.0;
                    let mut last_send =
                        std::time::Instant::now() - std::time::Duration::from_secs(1);

                    loop {
                        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                            let _ = child.kill().await;
                            sender
                                .send(Action::DownloadPaused(
                                    destination.to_string_lossy().into_owned(),
                                ))
                                .ok();
                            return;
                        }
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {
                                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                                    let _ = child.kill().await;
                                    sender
                                        .send(Action::DownloadPaused(
                                            destination.to_string_lossy().into_owned(),
                                        ))
                                        .ok();
                                    return;
                                }
                            }
                            line_res = reader.next_line() => {
                                match line_res {
                                    Ok(Some(line)) => {
                                        if line.contains("[download] Destination:") {
                                            stream_index = stream_index.saturating_add(1);
                                        } else if let Some((raw_pct, status)) = parse_ytdlp_progress(&line) {
                                            let current_stream = stream_index.max(1);
                                            let (normalized_pct, display_status) = if current_stream <= 1 {
                                                (raw_pct * 0.90, status)
                                            } else {
                                                (90.0 + (raw_pct * 0.08), format!("Audio | {status}"))
                                            };
                                            max_progress = max_progress.max(normalized_pct);
                                            if last_send.elapsed() >= std::time::Duration::from_millis(250) || raw_pct >= 99.9 {
                                                sender
                                                    .send(Action::UpdateDownload(Some(max_progress), Some(display_status)))
                                                    .ok();
                                                last_send = std::time::Instant::now();
                                            }
                                        } else if line.contains("[Merger]") || line.contains("[ffmpeg]") {
                                            max_progress = max_progress.max(99.0);
                                            sender
                                                .send(Action::UpdateDownload(
                                                    Some(max_progress),
                                                    Some("Merging audio & video...".to_string()),
                                                ))
                                                .ok();
                                            last_send = std::time::Instant::now();
                                        }
                                    }
                                    Ok(None) => break,
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }

                let status = child.wait().await;
                match status {
                    Ok(s) if s.success() => {
                        sender
                            .send(Action::DownloadCompleted(
                                destination.to_string_lossy().into_owned(),
                            ))
                            .ok();
                    }
                    Ok(s) => {
                        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                            sender
                                .send(Action::DownloadPaused(
                                    destination.to_string_lossy().into_owned(),
                                ))
                                .ok();
                        } else {
                            sender
                                .send(Action::DownloadFailed(format!(
                                    "yt-dlp exited with status {s}"
                                )))
                                .ok();
                        }
                    }
                    Err(err) => {
                        sender
                            .send(Action::DownloadFailed(format!(
                                "Failed to wait for yt-dlp: {err}"
                            )))
                            .ok();
                    }
                }
            } else {
                let progress_sender = sender.clone();
                let result = crate::download::download(
                    &client,
                    &link,
                    &destination,
                    cancel,
                    move |progress| {
                        let total = progress.total.unwrap_or_default();
                        let percentage = if total > 0 {
                            progress.downloaded as f64 / total as f64 * 100.0
                        } else {
                            0.0
                        };
                        let speed = progress.bytes_per_second / 1024.0 / 1024.0;
                        let eta = if total > progress.downloaded && progress.bytes_per_second > 0.0
                        {
                            (total - progress.downloaded) as f64 / progress.bytes_per_second
                        } else {
                            0.0
                        };
                        let status = if total > 0 {
                            format!(
                                "{:.1}/{:.1} MB | {:.1} MB/s | ETA {:.0}s | {}x | attempt {}",
                                progress.downloaded as f64 / 1024.0 / 1024.0,
                                total as f64 / 1024.0 / 1024.0,
                                speed,
                                eta,
                                progress.workers,
                                progress.attempt
                            )
                        } else {
                            format!(
                                "{:.1} MB | {:.1} MB/s | {}x | attempt {}",
                                progress.downloaded as f64 / 1024.0 / 1024.0,
                                speed,
                                progress.workers,
                                progress.attempt
                            )
                        };
                        progress_sender
                            .send(Action::UpdateDownload(Some(percentage), Some(status)))
                            .ok();
                    },
                )
                .await;

                match result {
                    Ok(crate::download::DownloadOutcome::Completed { .. }) => {
                        sender
                            .send(Action::DownloadCompleted(
                                destination.to_string_lossy().into_owned(),
                            ))
                            .ok();
                    }
                    Ok(crate::download::DownloadOutcome::Paused { .. }) => {
                        sender
                            .send(Action::DownloadPaused(
                                destination.to_string_lossy().into_owned(),
                            ))
                            .ok();
                    }
                    Err(error) => {
                        sender.send(Action::DownloadFailed(error.to_string())).ok();
                    }
                }
            }
        });
    }
}

impl App {
    pub(super) async fn handle_download(&mut self, action: Action) -> Option<()> {
        match action {
            Action::DownloadStream(subtitle_url) => {
                if self.state.is_resolving_playback {
                    return None;
                }
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
                                "Download unavailable",
                                "No downloadable mirrors were found for this release.",
                            );
                            return None;
                        };
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing download",
                            format!(
                                "Resolving {} from {}...",
                                first_mirror.label,
                                release.provider.label()
                            ),
                        );
                        let client = if release.provider == ProviderKind::Addons
                            || release.provider == ProviderKind::BdixCircleFtp
                            || release.provider == ProviderKind::BdixDhakaFlix
                        {
                            let sender_clone = self.action_sender.clone();
                            sender_clone
                                .send(Action::StartDownload(
                                    subtitle_url,
                                    Some(first_mirror.resolver_url.clone()),
                                    first_mirror.headers.clone(),
                                ))
                                .ok();
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
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(18),
                                client.resolve_release(&release),
                            )
                            .await;
                            match result {
                                Ok(Ok(source)) => {
                                    sender
                                        .send(Action::StartDownload(
                                            subtitle_url,
                                            Some(source.url),
                                            source.headers,
                                        ))
                                        .ok();
                                }
                                Ok(Err(error)) => {
                                    log::error!("4KHDHub download resolve failed: {error}");
                                    sender
                                        .send(Action::SetStatus(format!("Error: 4KHDHub: {error}")))
                                        .ok();
                                }
                                Err(_) => {
                                    log::error!("4KHDHub download resolve timed out");
                                    sender
                                        .send(Action::SetStatus(
                                            "Error: 4KHDHub download resolution timed out. Select another release (e.g. 1080p) or press Ctrl+P for MovieBox.".to_string(),
                                        ))
                                        .ok();
                                }
                            }
                        });
                    } else {
                        self.action_sender
                            .send(Action::StartDownload(subtitle_url, None, Vec::new()))
                            .ok();
                    }
                } else {
                    let release = self.get_selected_release();
                    let link = self.get_selected_link();
                    let headers = release
                        .as_ref()
                        .and_then(|r| r.mirrors.first())
                        .map(|m| m.headers.clone())
                        .unwrap_or_default();
                    self.action_sender
                        .send(Action::StartDownload(subtitle_url, link, headers))
                        .ok();
                }
                return None;
            }
            Action::StartDownload(subtitle_url, link, headers) => {
                self.state.is_resolving_playback = false;
                self.start_resilient_download(subtitle_url, link, headers);
                return None;
            }
            Action::PromptDownloadEpisode => {
                self.state.show_episode_download_confirm = true;
                self.state.episode_download_confirm_yes_selected = false;
            }

            Action::ConfirmDownloadEpisode => {
                self.state.show_episode_download_confirm = false;

                let subject_id = self.state.active_subject_id.clone().unwrap_or_default();
                let resource_id = self.get_selected_resource_id();

                if let Some(rid) = resource_id {
                    self.state.notify(
                        NotificationKind::Info,
                        "Preparing download",
                        "Resolving episode stream...",
                    );
                    let service = self.service.clone();
                    let sender = self.action_sender.clone();
                    let sibling_ids: Vec<String> = self
                        .state
                        .selected_details
                        .as_ref()
                        .map(|d| {
                            let mut ids = vec![d.id.value.clone()];
                            ids.extend(d.dubs.iter().map(|dub| dub.subject_id.clone()));
                            ids.retain(|s| !s.is_empty());
                            ids.sort();
                            ids.dedup();
                            ids
                        })
                        .unwrap_or_default();
                    let season = self.state.selected_season;
                    let episode = self.state.selected_episode;
                    tokio::spawn(async move {
                        if let Ok(res) = service
                            .get_ext_captions(&subject_id, &rid, &sibling_ids, season, episode)
                            .await
                        {
                            sender.send(Action::ShowDownloadSubtitlePopup(res)).ok();
                        } else {
                            sender.send(Action::DownloadStream(None)).ok();
                        }
                    });
                } else {
                    self.action_sender.send(Action::DownloadStream(None)).ok();
                }
            }

            Action::PromptDownloadSeason => {
                self.state.show_season_download_confirm = true;
                self.state.season_download_confirm_yes_selected = false;
            }

            Action::ConfirmDownloadSeason => {
                self.state.show_season_download_confirm = false;
                self.state.season_subtitle_preference = None;
                let season_num = self.state.selected_season;

                let season_array_idx = self
                    .state
                    .available_seasons
                    .iter()
                    .position(|s| s.number == season_num);

                if let Some(idx) = season_array_idx {
                    if idx < self.state.available_episode_numbers.len() {
                        let ep_numbers = self.state.available_episode_numbers[idx].clone();
                        self.state.download_queue.clear();

                        for ep in ep_numbers {
                            self.state.download_queue.push_back((season_num, ep));
                        }
                        self.state.download_queue_total = self.state.download_queue.len();
                        self.action_sender.send(Action::ProcessDownloadQueue).ok();
                    }
                }
            }

            Action::ProcessDownloadQueue => {
                if self.state.download_progress.is_some() {
                    return None;
                }

                if let Some((season, episode)) = self.state.download_queue.pop_front() {
                    self.state.selected_season = season;
                    self.state.selected_episode = episode;
                    let remaining = self.state.download_queue.len();
                    let total = self.state.download_queue_total;
                    let num = total - remaining;

                    let raw_title = self
                        .state
                        .selected_details
                        .as_ref()
                        .map(|details| details.title.as_str())
                        .unwrap_or(crate::download::DEFAULT_STREAM_NAME);
                    let clean_title = crate::providers::moviebox::clean_moviebox_title(raw_title);
                    let safe_title = crate::download::safe_file_stem(&clean_title);

                    let base_dir = self.resolve_download_base_dir();
                    let target_dir = base_dir
                        .join("Series")
                        .join(&safe_title)
                        .join(format!("Season {season}"));
                    let base_name = format!("{safe_title} - S{season:02}E{episode:02}");

                    if is_media_already_downloaded(&target_dir, &base_name) {
                        self.state.notify(
                            NotificationKind::Info,
                            "Skipping episode",
                            format!(
                                "S{season:02}E{episode:02} already downloaded ({num}/{total})."
                            ),
                        );
                        self.action_sender.send(Action::ProcessDownloadQueue).ok();
                        return None;
                    }

                    self.state.notify(
                        NotificationKind::Info,
                        "Preparing episode",
                        format!("S{season:02}E{episode:02} · {num}/{total}"),
                    );

                    let subject_id = self.state.active_subject_id.clone().unwrap_or_default();

                    self.state.selected_resources.clear();
                    self.state.is_fetching_streams = true;

                    self.action_sender
                        .send(Action::FetchEpisodeStreams {
                            subject_id,
                            season,
                            episode,
                            force_refresh: false,
                        })
                        .ok();

                    self.action_sender.send(Action::DownloadStream(None)).ok();
                } else if self.state.download_queue_total > 0 {
                    self.state.notify(
                        NotificationKind::Success,
                        "Season downloaded",
                        format!("{} files completed.", self.state.download_queue_total),
                    );
                    self.state.download_queue_total = 0;
                }
            }

            Action::UpdateDownload(prog, stat) => {
                if self.state.download_progress != prog || self.state.download_status != stat {
                    self.state.download_progress = prog;
                    self.state.download_status = stat;
                    self.state.dirty = true;
                }
            }
            Action::DownloadCompleted(path) => {
                self.state.download_progress = Some(100.0);
                self.state.download_status = Some("Completed".into());
                self.state.notify(
                    NotificationKind::Success,
                    "Download complete",
                    format!("Saved to {path}"),
                );
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    sender.send(Action::ClearDownload).ok();
                });
            }
            Action::DownloadFailed(error) => {
                self.state.download_progress = None;
                self.state.download_status = None;
                self.state.download_title = None;
                if self.state.download_queue_total > 0 {
                    let total = self.state.download_queue_total;
                    let remaining = self.state.download_queue.len();
                    let completed = total.saturating_sub(remaining + 1);
                    self.state.notify(
                        NotificationKind::Error,
                        "Season download halted",
                        format!("{completed}/{total} files finished before error: {error}"),
                    );
                } else {
                    self.state.notify(
                        NotificationKind::Error,
                        "Download failed",
                        format!("Partial file preserved. {error}"),
                    );
                }
                self.state.download_queue.clear();
                self.state.download_queue_total = 0;
            }
            Action::DownloadPaused(path) => {
                self.state.download_progress = None;
                self.state.download_status = None;
                self.state.download_title = None;
                if self.state.download_queue_total > 0 {
                    let total = self.state.download_queue_total;
                    let remaining = self.state.download_queue.len();
                    let completed = total.saturating_sub(remaining + 1);
                    self.state.notify(
                        NotificationKind::Warning,
                        "Season download paused",
                        format!("{completed}/{total} files finished. Resume with {path}.part"),
                    );
                } else {
                    self.state.notify(
                        NotificationKind::Warning,
                        "Download paused",
                        format!("Start again to resume {path}.part"),
                    );
                }
                self.state.download_queue.clear();
                self.state.download_queue_total = 0;
            }
            Action::ClearDownload => {
                self.state.download_progress = None;
                self.state.download_status = None;
                self.state.download_title = None;
                if !self.state.download_queue.is_empty() {
                    self.action_sender.send(Action::ProcessDownloadQueue).ok();
                } else if self.state.download_queue_total > 0 {
                    self.state.notify(
                        NotificationKind::Success,
                        "Season downloaded",
                        format!("{} files completed.", self.state.download_queue_total),
                    );
                    self.state.download_queue_total = 0;
                }
            }
            Action::CancelDownload => {
                self.state
                    .cancel_download
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                self.state.download_status = Some("Cancelling...".to_string());
                self.state.notify(
                    NotificationKind::Warning,
                    "Cancelling download",
                    "Partial data will be preserved.",
                );
            }
            _ => return None,
        }
        None
    }
}

fn subtitle_language_from_url(url: &str) -> Option<&'static str> {
    let lower = url.to_lowercase();
    if lower.contains(".en.")
        || lower.contains("_en.")
        || lower.contains("/en/")
        || lower.contains("english")
    {
        Some("en")
    } else if lower.contains(".es.")
        || lower.contains("_es.")
        || lower.contains("/es/")
        || lower.contains("spanish")
    {
        Some("es")
    } else if lower.contains(".hi.")
        || lower.contains("_hi.")
        || lower.contains("/hi/")
        || lower.contains("hindi")
    {
        Some("hi")
    } else if lower.contains(".fr.")
        || lower.contains("_fr.")
        || lower.contains("/fr/")
        || lower.contains("french")
    {
        Some("fr")
    } else if lower.contains(".de.")
        || lower.contains("_de.")
        || lower.contains("/de/")
        || lower.contains("german")
    {
        Some("de")
    } else if lower.contains(".ar.")
        || lower.contains("_ar.")
        || lower.contains("/ar/")
        || lower.contains("arabic")
    {
        Some("ar")
    } else if lower.contains(".pt.")
        || lower.contains("_pt.")
        || lower.contains("/pt/")
        || lower.contains("portuguese")
    {
        Some("pt")
    } else if lower.contains(".ru.")
        || lower.contains("_ru.")
        || lower.contains("/ru/")
        || lower.contains("russian")
    {
        Some("ru")
    } else if lower.contains(".ja.")
        || lower.contains("_ja.")
        || lower.contains("/ja/")
        || lower.contains("japanese")
    {
        Some("ja")
    } else if lower.contains(".ko.")
        || lower.contains("_ko.")
        || lower.contains("/ko/")
        || lower.contains("korean")
    {
        Some("ko")
    } else if lower.contains(".zh.")
        || lower.contains("_zh.")
        || lower.contains("/zh/")
        || lower.contains("chinese")
    {
        Some("zh")
    } else if lower.contains(".it.")
        || lower.contains("_it.")
        || lower.contains("/it/")
        || lower.contains("italian")
    {
        Some("it")
    } else if lower.contains(".bn.")
        || lower.contains("_bn.")
        || lower.contains("/bn/")
        || lower.contains("bengali")
    {
        Some("bn")
    } else {
        None
    }
}

fn is_media_already_downloaded(target_dir: &std::path::Path, base_name: &str) -> bool {
    let extensions = ["mp4", "mkv", "webm", "ts"];
    for ext in extensions {
        let final_file = target_dir.join(format!("{base_name}.{ext}"));
        if final_file.exists() {
            let part_json = target_dir.join(format!("{base_name}.{ext}.part.json"));
            let part_file = target_dir.join(format!("{base_name}.{ext}.part"));
            let part_0 = target_dir.join(format!("{base_name}.{ext}.part.0"));
            if !part_json.exists() && !part_file.exists() && !part_0.exists() {
                if let Ok(metadata) = std::fs::metadata(&final_file) {
                    if metadata.len() > 1024 * 1024 {
                        return true;
                    }
                }
            }
        }
    }
    false
}
pub(crate) fn yt_dlp_missing_guidance() -> String {
    if crate::updater::artifact::is_termux_environment() {
        "MovieBox DASH streams require yt-dlp. Please install yt-dlp and ffmpeg on your device (e.g. 'pkg install yt-dlp ffmpeg') to download these streams.".to_string()
    } else if cfg!(target_os = "macos") {
        "MovieBox DASH streams require yt-dlp. Please install yt-dlp and ffmpeg on your Mac (e.g. 'brew install yt-dlp ffmpeg') to download these streams.".to_string()
    } else if cfg!(target_os = "windows") {
        "MovieBox DASH streams require yt-dlp. Please install yt-dlp and ffmpeg on your system (e.g. 'winget install yt-dlp.yt-dlp Gyan.FFmpeg') to download these streams.".to_string()
    } else if cfg!(target_os = "linux") {
        "MovieBox DASH streams require yt-dlp. Please install yt-dlp and ffmpeg via your system package manager to download these streams.".to_string()
    } else {
        "MovieBox DASH streams require yt-dlp. Please install yt-dlp and ffmpeg on your system to download these streams.".to_string()
    }
}

pub(crate) fn parse_ytdlp_progress(line: &str) -> Option<(f64, String)> {
    if !line.contains("[download]") || line.contains("Destination:") {
        return None;
    }
    let trimmed = line.split("[download]").nth(1)?.trim();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    let pct_str = words.iter().find(|w| w.ends_with('%'))?;
    let pct: f64 = pct_str.trim_end_matches('%').parse().ok()?;

    if trimmed.contains(" in ") {
        let size = words
            .iter()
            .position(|&w| w == "of")
            .and_then(|idx| words.get(idx + 1))
            .copied()
            .unwrap_or("");
        let duration = words
            .iter()
            .position(|&w| w == "in")
            .and_then(|idx| words.get(idx + 1))
            .copied()
            .unwrap_or("");
        if !size.is_empty() && !duration.is_empty() {
            return Some((pct, format!("{size} in {duration}")));
        }
    }

    let mut size_parts = Vec::new();
    let mut speed = "";
    let mut eta = "";

    if let Some(of_idx) = words.iter().position(|&w| w == "of") {
        let at_idx = words.iter().position(|&w| w == "at").unwrap_or(words.len());
        for &w in &words[of_idx + 1..at_idx] {
            let clean = w.trim_matches('~').trim();
            if !clean.is_empty() {
                size_parts.push(clean);
            }
        }
    }

    if let Some(at_idx) = words.iter().position(|&w| w == "at") {
        if let Some(&s) = words.get(at_idx + 1) {
            speed = s;
        }
    }

    if let Some(eta_idx) = words.iter().position(|&w| w == "ETA") {
        if let Some(&e) = words.get(eta_idx + 1) {
            let clean_eta = e.trim_end_matches(['(', ')', ',']);
            if !clean_eta.eq_ignore_ascii_case("unknown") {
                eta = clean_eta;
            }
        }
    }

    let size = size_parts.join(" ");
    let mut parts = Vec::new();
    if !size.is_empty() {
        parts.push(size);
    }
    if !speed.is_empty() {
        parts.push(speed.to_string());
    }
    if !eta.is_empty() {
        parts.push(format!("ETA {eta}"));
    }

    let status = if parts.is_empty() {
        trimmed.to_string()
    } else {
        parts.join(" | ")
    };

    Some((pct, status))
}

#[cfg(test)]
mod tests {
    use crate::providers::models::{ProviderKind, Release, SourceMirror};
    use crate::tui::action::Action;
    use crate::tui::app::App;
    use crate::tui::overlay::NotificationKind;
    use crate::tui::state::Screen;

    #[tokio::test]
    async fn test_confirm_download_episode_emits_resolving_stream_notification() {
        let mut app = App::new();
        app.state.active_screen = Screen::Details;
        app.state.selected_resources = vec![Release {
            provider: ProviderKind::MovieBox,
            filename: "Episode.mkv".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("hevc".to_string()),
            language: None,
            size_bytes: Some(1024),
            season: Some(1),
            episode: Some(1),
            mirrors: vec![SourceMirror {
                label: "Direct".to_string(),
                resolver_url: "https://example.com/stream.mp4".to_string(),
                headers: vec![],
                direct_file: true,
            }],
            resource_id: Some("98765".to_string()),
        }];
        app.state.resource_list_state.select(Some(0));

        app.handle_download(Action::ConfirmDownloadEpisode).await;

        let notif = app.state.notifications.back().expect("notification posted");
        assert_eq!(notif.kind, NotificationKind::Info);
        assert_eq!(notif.title, "Preparing download");
        assert_eq!(notif.message, "Resolving episode stream...");
    }

    #[tokio::test]
    async fn test_download_stream_missing_mirror_reports_downloadable_error() {
        let mut app = App::new();
        app.state.active_provider = ProviderKind::FourKHdHub;
        app.state.active_screen = Screen::Details;
        app.state.selected_resources = vec![Release {
            provider: ProviderKind::FourKHdHub,
            filename: "Unavailable.mkv".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("hevc".to_string()),
            language: None,
            size_bytes: Some(1024),
            season: None,
            episode: None,
            mirrors: vec![],
            resource_id: None,
        }];
        app.state.resource_list_state.select(Some(0));

        app.handle_download(Action::DownloadStream(None)).await;

        let notif = app.state.notifications.back().expect("notification posted");
        assert_eq!(notif.kind, NotificationKind::Error);
        assert_eq!(notif.title, "Download unavailable");
        assert_eq!(
            notif.message,
            "No downloadable mirrors were found for this release."
        );
    }
    #[test]
    fn test_parse_ytdlp_progress_variants() {
        let line1 = "[download]   0.0% of ~   5.19MiB at      0.00B/s ETA Unknown (frag 0/1676)";
        let res1 = super::parse_ytdlp_progress(line1);
        assert!(res1.is_some());
        let (pct1, status1) = res1.unwrap();
        assert!((pct1 - 0.0).abs() < f64::EPSILON);
        assert_eq!(status1, "5.19MiB | 0.00B/s");
        assert!(!status1.contains('~'));
        assert!(!status1.contains("Unknown"));
        assert!(!status1.contains("(frag"));

        let line2 = "[download]  45.2% of ~ 1.45GiB at 12.3MiB/s ETA 00:45 (frag 500/1676)";
        let res2 = super::parse_ytdlp_progress(line2);
        assert!(res2.is_some());
        let (pct2, status2) = res2.unwrap();
        assert!((pct2 - 45.2).abs() < f64::EPSILON);
        assert_eq!(status2, "1.45GiB | 12.3MiB/s | ETA 00:45");

        let line3 = "[download] 100% of 1.45GiB in 02:15";
        let res3 = super::parse_ytdlp_progress(line3);
        assert!(res3.is_some());
        let (pct3, status3) = res3.unwrap();
        assert!((pct3 - 100.0).abs() < f64::EPSILON);
        assert_eq!(status3, "1.45GiB in 02:15");

        let line_non = "[generic] Extracting URL: https://example.com/index.mpd";
        assert!(super::parse_ytdlp_progress(line_non).is_none());

        let line_dest = "[download] Destination: /tmp/test.mp4";
        assert!(super::parse_ytdlp_progress(line_dest).is_none());
    }

    #[test]
    fn test_yt_dlp_missing_guidance_contains_platform_hint() {
        let guidance = super::yt_dlp_missing_guidance();
        assert!(guidance.contains("MovieBox DASH streams require yt-dlp"));
        assert!(guidance.contains("install yt-dlp and ffmpeg"));
    }

    #[tokio::test]
    async fn test_download_stream_forwards_mirror_headers() {
        let mut app = App::new();
        app.state.active_provider = ProviderKind::MovieBox;
        app.state.active_screen = Screen::Details;
        app.state.selected_resources = vec![Release {
            provider: ProviderKind::MovieBox,
            filename: "Movie.mp4".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("hevc".to_string()),
            language: None,
            size_bytes: Some(1024),
            season: None,
            episode: None,
            mirrors: vec![SourceMirror {
                label: "H.265".to_string(),
                resolver_url: "https://example.com/dash/123/index.mpd".to_string(),
                headers: vec![
                    ("Cookie".to_string(), "CloudFront-Policy=xyz".to_string()),
                    ("Referer".to_string(), "https://sportslive.wine".to_string()),
                ],
                direct_file: true,
            }],
            resource_id: Some("12345".to_string()),
        }];
        app.state.resource_list_state.select(Some(0));

        app.handle_download(Action::DownloadStream(None)).await;

        let dispatched = app.action_receiver.try_recv().expect("action dispatched");
        match dispatched {
            Action::StartDownload(_, link, headers) => {
                assert_eq!(
                    link.as_deref(),
                    Some("https://example.com/dash/123/index.mpd")
                );
                assert_eq!(headers.len(), 2);
                assert_eq!(headers[0].0, "Cookie");
                assert_eq!(headers[0].1, "CloudFront-Policy=xyz");
                assert_eq!(headers[1].0, "Referer");
                assert_eq!(headers[1].1, "https://sportslive.wine");
            }
            other => panic!("expected StartDownload, got {:?}", other),
        }
    }
    #[test]
    fn test_download_directory_and_filename_conventions() {
        let base_dir = std::path::PathBuf::from("/tmp/MovieBox-TUI");

        let movie_title = "Ek Deewane Ki Deewaniyat";
        let movie_target = base_dir.join("Movies").join(movie_title);
        let movie_file = movie_target.join(format!("{movie_title}.mp4"));
        let movie_sub = movie_target.join(format!("{movie_title}.en.srt"));

        let expected_movie_file = base_dir
            .join("Movies")
            .join(movie_title)
            .join(format!("{movie_title}.mp4"));
        let expected_movie_sub = base_dir
            .join("Movies")
            .join(movie_title)
            .join(format!("{movie_title}.en.srt"));
        assert_eq!(movie_file, expected_movie_file);
        assert_eq!(movie_sub, expected_movie_sub);

        let series_title = "Breaking Bad";
        let season: usize = 1;
        let episode: usize = 1;
        let series_target = base_dir
            .join("Series")
            .join(series_title)
            .join(format!("Season {season}"));
        let base_name = format!("{series_title} - S{season:02}E{episode:02}");
        let series_file = series_target.join(format!("{base_name}.mp4"));
        let series_sub = series_target.join(format!("{base_name}.en.srt"));

        let expected_series_file = base_dir
            .join("Series")
            .join(series_title)
            .join(format!("Season {season}"))
            .join(format!("{base_name}.mp4"));
        let expected_series_sub = base_dir
            .join("Series")
            .join(series_title)
            .join(format!("Season {season}"))
            .join(format!("{base_name}.en.srt"));
        assert_eq!(series_file, expected_series_file);
        assert_eq!(series_sub, expected_series_sub);
    }
}
