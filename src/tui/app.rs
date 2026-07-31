use ratatui::Frame;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::providers::{
    fourkhdhub::{
        FourKHdHubClient, details_to_moviebox_json, releases_to_moviebox_json,
        search_to_moviebox_json,
    },
    models::{ProviderKind, Release, RequestContext, SearchScope},
    moviebox::client::MovieBoxClient,
};
use crate::tui::{
    action::Action,
    event::EventHandler,
    overlay::NotificationKind,
    state::{AppState, InputMode, Screen, SearchResult},
    theme::Theme,
};

pub fn clean_moviebox_title(raw_title: &str) -> String {
    let mut end = raw_title.len();

    if let Some(start) = raw_title[..end].find(" [") {
        end = start;
    }
    if let Some(start) = raw_title[..end].find(" (") {
        let inside = &raw_title[start..end].to_lowercase();
        if inside.contains("dub") || inside.contains("hindi") {
            end = start;
        }
    }

    if let Some(s_idx) = raw_title[..end].rfind(" S") {
        let suffix = &raw_title[s_idx + 2..end];
        let is_season = suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == 'S');
        if is_season && suffix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            end = s_idx;
        }
    }
    raw_title[..end].trim_end().to_string()
}

/// Score how well `title` matches `query`. Higher is better.
/// Below 400 is treated as noise and dropped.
fn search_relevance(title: &str, query: &str) -> i32 {
    let q = normalize_search_key(query);
    if q.is_empty() {
        return 0;
    }
    let t = normalize_search_key(title);
    if t.is_empty() {
        return 0;
    }

    let title_lower = title.to_lowercase();
    let query_lower = query.trim().to_lowercase();

    // Exact (ignoring punctuation/case)
    if t == q {
        return 1000;
    }
    // Exact on display string
    if title_lower.trim() == query_lower {
        return 980;
    }
    // Starts with full query (normalized)
    if t.starts_with(&q) {
        return 900;
    }
    // Title contains full query as contiguous key
    if t.contains(&q) {
        // Prefer shorter titles (closer match) slightly
        let length_penalty = (t.len().saturating_sub(q.len()) as i32).min(80);
        return 750 - length_penalty / 4;
    }
    // Query contains full title (short titles like "Dune")
    if q.contains(&t) && t.len() >= 3 {
        return 700;
    }

    // Token match: all significant query tokens must appear in title.
    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(normalize_search_key)
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return 0;
    }
    let matched = tokens.iter().filter(|tok| t.contains(tok.as_str())).count();
    if matched == tokens.len() {
        // All tokens present — solid hit
        let mut score = 550 + (matched as i32 * 20);
        // Penalty when title has many extra tokens (looser match)
        let title_tokens = title
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 2)
            .count();
        score -= (title_tokens.saturating_sub(tokens.len()) as i32 * 15).min(120);
        // Prefer non-dub when query didn't ask for it
        let q_has_lang = query_lower.contains("hindi")
            || query_lower.contains("tamil")
            || query_lower.contains("telugu");
        if !q_has_lang
            && (title_lower.contains("[hindi]")
                || title_lower.contains("[tamil]")
                || title_lower.contains("[telugu]"))
        {
            score -= 80;
        }
        return score.max(400);
    }
    // Majority token match for longer queries only (avoid noisy 1/2 hits)
    if tokens.len() >= 3 && matched * 100 / tokens.len() >= 70 {
        return 420;
    }
    0
}

fn normalize_search_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn provider_from_subject(item: &serde_json::Value, fallback: ProviderKind) -> ProviderKind {
    match item
        .get("_provider")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
    {
        "fourkhdhub" => ProviderKind::FourKHdHub,
        "moviebox" => ProviderKind::MovieBox,
        _ => fallback,
    }
}

fn tag_subjects_with_provider(payload: serde_json::Value, provider: ProviderKind) -> serde_json::Value {
    let mut payload = payload;
    if let Some(results) = payload
        .get_mut("results")
        .and_then(|r| r.as_array_mut())
        .and_then(|arr| arr.first_mut())
        .and_then(|first| first.get_mut("subjects"))
        .and_then(|s| s.as_array_mut())
    {
        for subject in results.iter_mut() {
            if let Some(obj) = subject.as_object_mut() {
                obj.entry("_provider")
                    .or_insert_with(|| serde_json::json!(provider.cache_key()));
            }
        }
    }
    payload
}

fn merge_search_payloads(parts: Vec<(ProviderKind, serde_json::Value)>) -> serde_json::Value {
    let mut subjects = Vec::new();
    for (provider, payload) in parts {
        let tagged = tag_subjects_with_provider(payload, provider);
        if let Some(list) = tagged
            .get("results")
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("subjects"))
            .and_then(|s| s.as_array())
        {
            subjects.extend(list.iter().cloned());
        }
    }
    serde_json::json!({ "results": [{ "subjects": subjects }], "multi_provider": true })
}

pub struct App {
    state: AppState,
    theme: Theme,
    client: MovieBoxClient,
    fourk_client: FourKHdHubClient,
    action_sender: mpsc::UnboundedSender<Action>,
    action_receiver: mpsc::UnboundedReceiver<Action>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let (action_sender, action_receiver) = mpsc::unbounded_channel();
        let mut state = AppState::default();

        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("moviebox-tui").join("config.json");
            if let Ok(config_str) = std::fs::read_to_string(config_path) {
                if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
                    if let Some(auto_update) =
                        config_json.get("auto_update").and_then(|v| v.as_bool())
                    {
                        state.auto_update = auto_update;
                    }
                    if let Some(last_check) = config_json
                        .get("last_update_check")
                        .and_then(|v| v.as_u64())
                    {
                        state.last_update_check = last_check;
                    }
                    if config_json.get("active_provider").and_then(|v| v.as_str())
                        == Some(ProviderKind::FourKHdHub.cache_key())
                    {
                        state.active_provider = ProviderKind::FourKHdHub;
                    }
                    if let Some(scope_key) =
                        config_json.get("search_scope").and_then(|v| v.as_str())
                    {
                        state.search_scope = SearchScope::from_cache_key(scope_key);
                    }
                }
            }
        }

        Self {
            state,
            theme: Theme::new(),
            client: MovieBoxClient::new(),
            fourk_client: FourKHdHubClient::new(),
            action_sender,
            action_receiver,
        }
    }

    fn request_context(&self) -> RequestContext {
        RequestContext {
            provider: self.state.active_provider,
            generation: self.state.provider_generation,
        }
    }

    fn context_is_current(&self, context: RequestContext) -> bool {
        context.provider == self.state.active_provider
            && context.generation == self.state.provider_generation
    }

    fn persist_config(&self) {
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("moviebox-tui");
            let _ = std::fs::create_dir_all(&app_dir);
            let config = serde_json::json!({
                "auto_update": self.state.auto_update,
                "last_update_check": self.state.last_update_check,
                "active_provider": self.state.active_provider.cache_key(),
                "search_scope": self.state.search_scope.cache_key(),
            });
            let _ = std::fs::write(app_dir.join("config.json"), config.to_string());
        }
    }

    fn open_provider_picker(&mut self) {
        if self.state.is_tv_mode {
            return;
        }
        self.state.show_help = false;
        self.state.tv_config_popup = false;
        self.state.player_picker_popup = false;
        self.state.subtitle_popup = false;
        self.state.provider_picker_popup = true;
        let options = SearchScope::menu_options();
        let selected = options
            .iter()
            .position(|scope| *scope == self.state.search_scope)
            .unwrap_or(0);
        self.state.provider_picker_state.select(Some(selected));
        self.state.input_mode = InputMode::Normal;
    }

    fn set_search_scope(&mut self, scope: SearchScope) {
        self.state.provider_picker_popup = false;
        if self.state.search_scope == scope {
            self.state.status_message = format!("Search scope already: {}", scope.label());
            self.state.status_timer = 120;
            return;
        }
        self.state.search_scope = scope;
        // Keep active_provider aligned when user pins a single source.
        if let SearchScope::Only(provider) = scope {
            self.state.active_provider = provider;
            if provider == ProviderKind::MovieBox {
                let client = self.client.clone();
                tokio::spawn(async move {
                    let _ = client.init_resilient().await;
                });
            }
        }
        self.persist_config();
        self.state.status_message = format!(
            "Search scope: {}. Next search uses {}.",
            scope.label(),
            match scope {
                SearchScope::All => "MovieBox + 4KHDHub",
                SearchScope::Only(p) => p.label(),
            }
        );
        self.state.status_timer = 200;
        self.state.notify(
            NotificationKind::Info,
            "Search sources",
            scope.label(),
        );
    }

    fn switch_provider(&mut self, provider: ProviderKind) {
        // Soft pin: used when opening a multi-search hit from a specific catalog.
        if provider == self.state.active_provider {
            return;
        }
        self.state.active_provider = provider;
        self.persist_config();
    }

    fn prepare_sixel_redraw(&mut self) {
        if self.state.image_picker.as_ref().is_some_and(|picker| {
            matches!(
                picker.protocol_type(),
                ratatui_image::picker::ProtocolType::Sixel
            )
        }) {
            self.state.clear_terminal_before_draw = true;
        }
    }

    fn cycle_details_pane(&mut self, forward: bool) {
        use crate::tui::state::DetailsPane;

        if self.state.active_screen != Screen::Details {
            return;
        }

        let has_languages = self
            .state
            .selected_details
            .as_ref()
            .and_then(|details| details.get("dubs"))
            .and_then(|dubs| dubs.as_array())
            .is_some_and(|dubs| dubs.len() > 1);
        let is_series = !self.state.available_seasons.is_empty();
        let mut panes = Vec::new();
        if has_languages {
            panes.push(DetailsPane::Languages);
        }
        if is_series {
            panes.push(DetailsPane::Seasons);
            panes.push(DetailsPane::Episodes);
        }
        panes.push(DetailsPane::Streams);

        let current = panes
            .iter()
            .position(|pane| *pane == self.state.details_pane)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % panes.len()
        } else if current == 0 {
            panes.len() - 1
        } else {
            current - 1
        };
        self.state.details_pane = panes[next];
    }

    fn trigger_episode_fetch(&mut self) {
        if let Some(id) = self.state.active_subject_id.clone() {
            let stype = self
                .state
                .selected_details
                .as_ref()
                .and_then(|d| d.get("subjectType").or_else(|| d.get("stype")))
                .and_then(|s| s.as_i64())
                .unwrap_or(1);

            let (se, ep) = if stype == 2 {
                let se_idx = self.state.season_list_state.selected().unwrap_or(0);
                let ep_idx = self.state.episode_list_state.selected().unwrap_or(0);

                let season_num = self
                    .state
                    .available_seasons
                    .get(se_idx)
                    .and_then(|s| s.get("se"))
                    .and_then(|s| s.as_i64())
                    .unwrap_or(1) as usize;

                let ep_num =
                    if let Some(ep_numbers) = self.state.available_episode_numbers.get(se_idx) {
                        ep_numbers.get(ep_idx).copied().unwrap_or(ep_idx + 1)
                    } else {
                        ep_idx + 1
                    };
                (season_num, ep_num)
            } else {
                (0, 0)
            };

            self.state.selected_season = se;
            self.state.selected_episode = ep;
            self.state.resource_list_state.select(None);
            self.state.stream_error = None;
            self.state.active_resource_request = self.state.active_resource_request.wrapping_add(1);

            let memory_cached = self
                .state
                .stream_pool
                .get(&id)
                .and_then(|pool| pool.episode_index.get(&(se, ep)))
                .filter(|streams| !streams.is_empty())
                .cloned();
            let disk_cached = memory_cached.is_none().then(|| {
                crate::cache::get_provider_stream_cache(self.state.active_provider, &id, se, ep)
                    .and_then(|value| value.as_array().cloned())
            });
            let cached = memory_cached.or_else(|| disk_cached.flatten());

            if let Some(streams) = cached {
                if let Some(pool) = self.state.stream_pool.get_mut(&id) {
                    pool.episode_index.insert((se, ep), streams.clone());
                }
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.status_message = "Loading streams...".to_string();
                self.state.status_timer = 90;
                self.state.pending_episode_fetch = None;
                let sender = self.action_sender.clone();
                let context = self.request_context();
                let request_id = self.state.active_resource_request;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    sender
                        .send(Action::EpisodeStreamsReady(
                            context,
                            request_id,
                            id,
                            se,
                            ep,
                            serde_json::Value::Array(streams),
                        ))
                        .ok();
                });
            } else {
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.status_message = "Loading streams...".to_string();
                self.state.status_timer = 90;

                self.state.pending_episode_fetch = Some((id.clone(), se, ep));
                self.state.last_episode_nav = std::time::Instant::now();
            }
        }
    }

    fn get_selected_link(&self) -> Option<String> {
        self.state
            .selected_resources
            .as_ref()
            .and_then(|res| res.get("list"))
            .and_then(|l| l.as_array())
            .and_then(|list| {
                let idx = self.state.resource_list_state.selected().unwrap_or(0);
                list.get(idx)
            })
            .and_then(|file| file.get("resourceLink"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
    }

    fn get_selected_resource_id(&self) -> Option<String> {
        self.state
            .selected_resources
            .as_ref()
            .and_then(|res| res.get("list"))
            .and_then(|l| l.as_array())
            .and_then(|list| {
                let idx = self.state.resource_list_state.selected().unwrap_or(0);
                list.get(idx)
            })
            .and_then(|file| file.get("resourceId"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
    }

    fn get_selected_release(&self) -> Option<Release> {
        self.state
            .selected_resources
            .as_ref()?
            .get("list")?
            .as_array()?
            .get(self.state.resource_list_state.selected().unwrap_or(0))?
            .get("_fourk_release")
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    fn open_library(&mut self) {
        self.prepare_sixel_redraw();
        self.state.show_help = false;
        self.state.tv_config_popup = false;
        self.state.player_picker_popup = false;
        self.state.subtitle_popup = false;
        self.state.is_download_subtitle_popup = false;
        self.state.input_mode = InputMode::Normal;
        self.state.active_screen = Screen::Library;
        self.refresh_library(false);
    }

    fn refresh_library(&mut self, notify: bool) {
        self.state.library_items = crate::tui::library::scan_library();
        if self.state.library_items.is_empty() {
            self.state.library_list_state.select(None);
        } else {
            let sel = self
                .state
                .library_list_state
                .selected()
                .unwrap_or(0)
                .min(self.state.library_items.len() - 1);
            self.state.library_list_state.select(Some(sel));
        }
        let count = self.state.library_items.len();
        self.state.status_message = if count == 0 {
            "Library empty — download something with [d] first.".into()
        } else {
            format!(
                "Library: {count} download{}.",
                if count == 1 { "" } else { "s" }
            )
        };
        self.state.status_timer = 150;
        if notify {
            self.state.notify(
                NotificationKind::Info,
                "Library refreshed",
                format!("{count} item{}", if count == 1 { "" } else { "s" }),
            );
        }
    }

    fn play_selected_library_item(&mut self, open_with: bool) {
        let _ = open_with; // always show player choice for local files
        let Some(idx) = self.state.library_list_state.selected() else {
            self.state.notify(
                NotificationKind::Warning,
                "Nothing selected",
                "Pick a download first.",
            );
            return;
        };
        let Some(item) = self.state.library_items.get(idx).cloned() else {
            return;
        };
        if !item.path.is_file() {
            self.state.notify(
                NotificationKind::Error,
                "File missing",
                "This download was moved or deleted. Press r to refresh.",
            );
            return;
        }
        // Ensure player list is populated (detection can finish late).
        if self.state.available_players.is_empty() {
            self.state.available_players = crate::tui::player::detect();
        }
        if self.state.available_players.is_empty() {
            self.state.notify(
                NotificationKind::Error,
                "Player unavailable",
                "Install mpv, IINA, or VLC.",
            );
            return;
        }
        let path = item.play_path();
        let sub = item.subtitle_path();
        self.state.status_message = format!("Open \"{}\" with…", item.title);
        self.state.status_timer = 120;
        // Always show the player menu from the library (even with one player).
        self.action_sender
            .send(Action::ShowPlayerPicker(path, sub))
            .ok();
    }

    fn start_resilient_download(&mut self, subtitle_url: Option<String>, link: Option<String>) {
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

        let title = self
            .state
            .selected_details
            .as_ref()
            .and_then(|details| details.get("title"))
            .and_then(|title| title.as_str())
            .unwrap_or("MovieBox-Tui_Stream");
        let media_type = self
            .state
            .selected_details
            .as_ref()
            .and_then(|details| details.get("stype").or_else(|| details.get("subjectType")))
            .and_then(|value| value.as_i64())
            .unwrap_or(1);
        let season = self.state.selected_season;
        let episode = self.state.selected_episode;
        let clean_title = crate::tui::app::clean_moviebox_title(title);
        let safe_title = crate::download::safe_file_stem(&clean_title);

        let extension = self
            .state
            .selected_resources
            .as_ref()
            .and_then(|resources| resources.get("list"))
            .and_then(|list| list.as_array())
            .and_then(|list| list.get(self.state.resource_list_state.selected().unwrap_or(0)))
            .and_then(|resource| {
                resource
                    .get("fileName")
                    .or_else(|| resource.get("title"))
                    .and_then(|name| name.as_str())
            })
            .and_then(|name| std::path::Path::new(name).extension())
            .and_then(|extension| extension.to_str())
            .filter(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v"
                )
            })
            .unwrap_or("mp4")
            .to_ascii_lowercase();

        let base_dir = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("MovieBox-TUI");
        let (target_dir, base_name) = if media_type == 2 {
            (
                base_dir
                    .join("Series")
                    .join(&safe_title)
                    .join(format!("Season {season}")),
                format!("{safe_title}_S{season:02}E{episode:02}"),
            )
        } else {
            (base_dir.join("Movies"), safe_title)
        };
        let mut destination = target_dir.join(format!("{base_name}.{extension}"));
        let mut counter = 2;
        while destination.exists() {
            destination = target_dir.join(format!("{base_name}_{counter}.{extension}"));
            counter += 1;
        }

        self.state.is_waiting_for_download_stream = false;
        self.state.download_status = Some("Preparing download...".into());
        self.state.download_progress = Some(0.0);
        self.state
            .cancel_download
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.state.notify(
            NotificationKind::Info,
            "Download started",
            "Partial data will be preserved.",
        );

        let cancel = self.state.cancel_download.clone();
        let sender = self.action_sender.clone();
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| self.client.http_client().clone());

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
                let subtitle_path = destination.with_extension("srt");
                let subtitle_client = client.clone();
                tokio::spawn(async move {
                    if let Ok(response) = subtitle_client.get(subtitle_url).send().await
                        && response.status().is_success()
                        && let Ok(bytes) = response.bytes().await
                    {
                        let _ = tokio::fs::write(subtitle_path, bytes).await;
                    }
                });
            }

            let progress_sender = sender.clone();
            let result =
                crate::download::download(&client, &link, &destination, cancel, move |progress| {
                    let total = progress.total.unwrap_or_default();
                    let percentage = if total > 0 {
                        progress.downloaded as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    let speed = progress.bytes_per_second / 1024.0 / 1024.0;
                    let eta = if total > progress.downloaded && progress.bytes_per_second > 0.0 {
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
                })
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
        });
    }

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
                    if matches!(
                        picker.protocol_type(),
                        ratatui_image::picker::ProtocolType::Halfblocks
                    ) {
                        self.state.image_supported = false;
                    } else {
                        let cell_h = picker.font_size().height;
                        if cell_h > 0 {
                            self.state.poster_rows = (96_u16.div_ceil(cell_h)).max(3);
                        }
                        self.state.image_picker = Some(picker);
                    }
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
                let _ = client.init_resilient().await;
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
        } else {
            self.state.active_screen = Screen::Home;
        }

        let player_sender = self.action_sender.clone();
        tokio::task::spawn_blocking(move || {
            player_sender
                .send(Action::PlayersDetected(crate::tui::player::detect()))
                .ok();
        });

        loop {
            if self.state.clear_terminal_before_draw {
                terminal.clear()?;
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
            Action::Tick => {
                let mut needs_redraw = (self.state.is_loading && self.state.tick_count % 5 == 0)
                    || self.state.tick_count < 15;
                self.state.tick_count = self.state.tick_count.wrapping_add(1);
                if !self.state.notifications.is_empty() {
                    needs_redraw = true;
                    self.state.expire_notifications();
                }
                if self.state.status_timer > 0 {
                    needs_redraw = true;
                    self.state.status_timer -= 1;
                    if self.state.status_timer == 0 {
                        self.state.status_message.clear();
                    }
                }
                if needs_redraw {
                    self.state.dirty = true;
                }

                let current_query = self.state.search_query.trim().to_string();
                if current_query != self.state.last_suggest_query
                    && self.state.last_search_edit.elapsed()
                        >= std::time::Duration::from_millis(350)
                {
                    self.state.last_suggest_query = current_query.clone();
                    if !current_query.is_empty() {
                        if self.state.is_tv_mode {
                            let q = current_query.to_lowercase();
                            self.state.search_suggestions = self
                                .state
                                .tv_channels
                                .iter()
                                .filter(|c| c.name.to_lowercase().contains(&q))
                                .take(10)
                                .map(|c| c.name.clone())
                                .collect();
                        } else {
                            self.action_sender.send(Action::Suggest(current_query)).ok();
                        }
                    } else {
                        self.state.search_suggestions.clear();
                    }
                }

                if self.state.pending_episode_fetch.is_some()
                    && self.state.last_episode_nav.elapsed()
                        >= std::time::Duration::from_millis(300)
                {
                    if let Some((subject_id, se, ep)) = self.state.pending_episode_fetch.take() {
                        let mut found_cached = false;
                        if let Some(pool) = self.state.stream_pool.get(&subject_id) {
                            if let Some(cached) = pool.episode_index.get(&(se, ep)) {
                                found_cached = true;
                                let count = cached.len();
                                let mut result = serde_json::Map::new();
                                result.insert(
                                    "list".to_string(),
                                    serde_json::Value::Array(cached.clone()),
                                );
                                self.state.selected_resources =
                                    Some(serde_json::Value::Object(result));
                                self.state.is_loading = false;
                                self.state.resource_list_state.select(if count > 0 {
                                    Some(0)
                                } else {
                                    None
                                });
                                self.state.status_message =
                                    format!("Resolved {} direct stream sources (cached).", count);
                                self.state.status_timer = 150;
                            }
                        }

                        if !found_cached {
                            self.action_sender
                                .send(Action::FetchEpisodeStreams {
                                    subject_id,
                                    season: se,
                                    episode: ep,
                                    force_refresh: false,
                                })
                                .ok();
                        }
                    }
                }
            }
            Action::Quit => {
                return Some(());
            }
            Action::FocusChange => {
                self.prepare_sixel_redraw();
                self.state.poster_protocol = None;
                self.state.search_poster_protocols.clear();
                if self.state.image_picker.is_some() {}
            }
            Action::Resize(_w, _h) => {
                self.prepare_sixel_redraw();
                self.state.poster_protocol = None;
                self.state.search_poster_protocols.clear();
                if self.state.image_picker.is_some() {}
            }
            Action::SwitchProvider(provider) => self.switch_provider(provider),
            Action::OpenProviderPicker => {
                self.open_provider_picker();
            }
            Action::SetSearchScope(scope) => {
                self.set_search_scope(scope);
            }
            Action::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};

                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::SUPER)
                {
                    if let KeyCode::Char('c') = key.code
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.action_sender.send(Action::Quit).ok();
                        return Some(());
                    }
                    if let KeyCode::Char('t') = key.code
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.action_sender.send(Action::ToggleTvMode).ok();
                        return None;
                    }
                    if let KeyCode::Char('p') = key.code
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.action_sender.send(Action::OpenProviderPicker).ok();
                        return None;
                    }
                    // Library: Ctrl+L or Cmd+L (does not steal bare "l" while typing).
                    if let KeyCode::Char('l') | KeyCode::Char('L') = key.code {
                        if matches!(self.state.active_screen, Screen::Library) {
                            self.action_sender.send(Action::GoBack).ok();
                        } else if matches!(
                            self.state.active_screen,
                            Screen::Home | Screen::Details
                        ) {
                            self.action_sender.send(Action::OpenLibrary).ok();
                        }
                        return None;
                    }
                }

                if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
                    && self.state.download_progress.is_some()
                {
                    self.action_sender.send(Action::CancelDownload).ok();
                    return None;
                }

                if key.code == KeyCode::F(1) {
                    self.action_sender.send(Action::ToggleHelp).ok();
                    return None;
                }

                match self.state.input_mode {
                    InputMode::Editing => match key.code {
                        KeyCode::Esc => {
                            self.state.input_mode = InputMode::Normal;
                            self.state.status_message = String::new();
                            self.state.status_timer = 150;
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
                                self.state.last_suggest_query =
                                    self.state.search_query.trim().to_string();
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
                                self.state.last_suggest_query =
                                    self.state.search_query.trim().to_string();
                            }
                        }
                        _ => {}
                    },
                    InputMode::Normal => match self.state.active_screen {
                        Screen::Startup => {}
                        Screen::Library => match key.code {
                            KeyCode::Esc | KeyCode::Char('b') => {
                                self.action_sender.send(Action::GoBack).ok();
                            }
                            KeyCode::Up => {
                                self.action_sender.send(Action::MoveUp).ok();
                            }
                            KeyCode::Down => {
                                self.action_sender.send(Action::MoveDown).ok();
                            }
                            KeyCode::Enter => {
                                // When the player menu is open, confirm the chosen player.
                                if self.state.player_picker_popup {
                                    self.action_sender.send(Action::Submit).ok();
                                } else {
                                    self.action_sender
                                        .send(Action::PlayLibraryItem { open_with: true })
                                        .ok();
                                }
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                if !self.state.player_picker_popup {
                                    self.action_sender
                                        .send(Action::PlayLibraryItem { open_with: true })
                                        .ok();
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                self.action_sender.send(Action::RefreshLibrary).ok();
                            }
                            KeyCode::Char('?') => {
                                self.action_sender.send(Action::ToggleHelp).ok();
                            }
                            KeyCode::Char('q') => {
                                self.action_sender.send(Action::Quit).ok();
                            }
                            _ => {}
                        },
                        Screen::Home => {
                            if self.state.tv_config_popup {
                                match key.code {
                                    KeyCode::Esc => {
                                        if self.state.tv_wizard_step == 1 {
                                            self.state.tv_wizard_step = 0;
                                            self.state.tv_wizard_selected_idx = 0;
                                            self.state.tv_wizard_options = vec![
                                                "Grouped by category".to_string(),
                                                "Grouped by language".to_string(),
                                                "Grouped by broadcast area".to_string(),
                                            ];
                                        } else {
                                            self.state.tv_config_popup = false;
                                        }
                                    }
                                    KeyCode::Up => {
                                        if self.state.tv_wizard_selected_idx > 0 {
                                            self.state.tv_wizard_selected_idx -= 1;
                                        } else {
                                            self.state.tv_wizard_selected_idx = self
                                                .state
                                                .tv_wizard_options
                                                .len()
                                                .saturating_sub(1);
                                        }
                                    }
                                    KeyCode::Down => {
                                        if self.state.tv_wizard_selected_idx
                                            < self.state.tv_wizard_options.len().saturating_sub(1)
                                        {
                                            self.state.tv_wizard_selected_idx += 1;
                                        } else {
                                            self.state.tv_wizard_selected_idx = 0;
                                        }
                                    }
                                    KeyCode::Char(' ') => {
                                        if self.state.tv_wizard_step == 1 {
                                            if let Some(opt) = self
                                                .state
                                                .tv_wizard_options
                                                .get(self.state.tv_wizard_selected_idx)
                                                .cloned()
                                            {
                                                if self.state.tv_wizard_selections.contains(&opt) {
                                                    self.state.tv_wizard_selections.remove(&opt);
                                                } else {
                                                    self.state.tv_wizard_selections.insert(opt);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Enter => {
                                        if self.state.tv_wizard_step == 0 {
                                            if let Some(selected_group) = self
                                                .state
                                                .tv_wizard_options
                                                .get(self.state.tv_wizard_selected_idx)
                                                .cloned()
                                            {
                                                self.state.tv_wizard_step = 1;
                                                self.state.tv_wizard_selected_idx = 0;
                                                if selected_group == "Grouped by category" {
                                                    self.state.tv_wizard_options =
                                                        crate::tui::iptv_data::CATEGORIES
                                                            .iter()
                                                            .map(|s| s.to_string())
                                                            .collect();
                                                } else if selected_group == "Grouped by language" {
                                                    self.state.tv_wizard_options =
                                                        crate::tui::iptv_data::LANGUAGES
                                                            .iter()
                                                            .map(|(n, _)| n.to_string())
                                                            .collect();
                                                } else {
                                                    self.state.tv_wizard_options =
                                                        crate::tui::iptv_data::COUNTRIES
                                                            .iter()
                                                            .map(|(n, _)| n.to_string())
                                                            .collect();
                                                }
                                            }
                                        } else {
                                            self.state.tv_config_popup = false;

                                            self.state.is_loading = true;
                                            self.state.status_message =
                                                "Fetching TV channels...".to_string();
                                            self.state.status_timer = 150;

                                            let mut urls_to_fetch = Vec::new();
                                            for sel in &self.state.tv_wizard_selections {
                                                if crate::tui::iptv_data::CATEGORIES
                                                    .contains(&sel.as_str())
                                                {
                                                    urls_to_fetch.push(format!("https://iptv-org.github.io/iptv/categories/{}.m3u", sel.to_lowercase()));
                                                } else if let Some((_, code)) =
                                                    crate::tui::iptv_data::LANGUAGES
                                                        .iter()
                                                        .find(|(n, _)| n == sel)
                                                {
                                                    urls_to_fetch.push(format!("https://iptv-org.github.io/iptv/languages/{}.m3u", code));
                                                } else if let Some((_, code)) =
                                                    crate::tui::iptv_data::COUNTRIES
                                                        .iter()
                                                        .find(|(n, _)| n == sel)
                                                {
                                                    urls_to_fetch.push(format!("https://iptv-org.github.io/iptv/countries/{}.m3u", code));
                                                }
                                            }

                                            let sender = self.action_sender.clone();
                                            tokio::spawn(async move {
                                                let mut config_path = dirs::config_dir()
                                                    .unwrap_or_else(|| {
                                                        std::path::PathBuf::from(".")
                                                    });
                                                config_path.push("moviebox-tui");
                                                std::fs::create_dir_all(&config_path).ok();
                                                config_path.push("tv_config.json");
                                                if let Ok(json) =
                                                    serde_json::to_string(&urls_to_fetch)
                                                {
                                                    std::fs::write(&config_path, json).ok();
                                                }

                                                let parser =
                                                    crate::providers::iptv_org::m3u::M3UParser::new(
                                                    );
                                                let mut all_channels = Vec::new();
                                                for url in urls_to_fetch {
                                                    let filename = url
                                                        .split('/')
                                                        .next_back()
                                                        .unwrap_or("playlist.m3u");
                                                    if let Ok(channels) =
                                                        parser.fetch_playlist(&url, filename).await
                                                    {
                                                        all_channels.extend(channels);
                                                    }
                                                }
                                                sender
                                                    .send(Action::TvChannelsLoaded(all_channels))
                                                    .ok();
                                            });
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
                                KeyCode::Enter => {
                                    if self.state.search_results.is_empty()
                                        && !self.state.search_query.trim().is_empty()
                                        && (self
                                            .state
                                            .status_message
                                            .to_ascii_lowercase()
                                            .starts_with("no matches")
                                            || self
                                                .state
                                                .status_message
                                                .to_ascii_lowercase()
                                                .contains("search failed"))
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
                                                .send(Action::ShowPlayerPicker(
                                                    item.id.clone(),
                                                    None,
                                                ))
                                                .ok();
                                        }
                                    }
                                }
                                KeyCode::Char(c)
                                    if key.modifiers.is_empty()
                                        || key.modifiers == KeyModifiers::SHIFT =>
                                {
                                    self.state.input_mode = InputMode::Editing;
                                    self.state.search_query.push(c);

                                    self.state.search_suggestions.clear();
                                    self.state.suggest_index = None;
                                    self.state.status_message = String::new();
                                    self.state.status_timer = 150;
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
                                    if let crate::tui::state::DetailsPane::Streams =
                                        self.state.details_pane
                                    {
                                        self.action_sender.send(Action::PlayStream(true)).ok();
                                    }
                                }
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                if !self.state.subtitle_popup && !self.state.player_picker_popup {
                                    if let crate::tui::state::DetailsPane::Seasons =
                                        self.state.details_pane
                                    {
                                        if !self.state.available_seasons.is_empty() {
                                            self.action_sender
                                                .send(Action::PromptDownloadSeason)
                                                .ok();
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
                                        self.action_sender
                                            .send(Action::ConfirmDownloadEpisode)
                                            .ok();
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
                                            self.action_sender
                                                .send(Action::PlayStream(open_with))
                                                .ok();
                                        }
                                        crate::tui::state::DetailsPane::Seasons => {
                                            // Confirm season → jump to episodes so navigation continues.
                                            self.state.episode_list_state.select(Some(0));
                                            self.trigger_episode_fetch();
                                            self.state.details_pane =
                                                crate::tui::state::DetailsPane::Episodes;
                                            self.state.status_message =
                                                "Season selected. Pick an episode.".into();
                                            self.state.status_timer = 120;
                                        }
                                        crate::tui::state::DetailsPane::Episodes => {
                                            // Confirm episode → jump to streams list.
                                            self.trigger_episode_fetch();
                                            self.state.details_pane =
                                                crate::tui::state::DetailsPane::Streams;
                                            self.state.status_message =
                                                "Episode selected. Pick a stream / Enter to play."
                                                    .into();
                                            self.state.status_timer = 120;
                                        }
                                        crate::tui::state::DetailsPane::Languages => {
                                            let idx = self
                                                .state
                                                .language_list_state
                                                .selected()
                                                .unwrap_or(0);

                                            self.action_sender
                                                .send(Action::SelectLanguage(idx))
                                                .ok();
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                    },
                }
            }

            Action::ToggleHelp => {
                if matches!(
                    self.state.active_screen,
                    Screen::Home | Screen::Details | Screen::Library
                ) {
                    self.state.show_help = !self.state.show_help;
                    if self.state.show_help {
                        self.state.tv_config_popup = false;
                        self.state.player_picker_popup = false;
                        self.state.subtitle_popup = false;
                        self.state.is_download_subtitle_popup = false;
                        self.state.show_season_download_confirm = false;
                        self.state.show_episode_download_confirm = false;
                    }
                }
            }
            Action::ToggleTvMode => {
                self.state.is_tv_mode = !self.state.is_tv_mode;
                self.state.tick_count = 0; // Reset animation
                if self.state.is_tv_mode {
                    self.state.tv_config_popup = false;
                    self.state.search_query.clear();
                    self.state.search_results.clear();
                    self.state.status_message = "Initializing Moviebox TV Mode...".to_string();
                    self.state.status_timer = 200;

                    let sender = self.action_sender.clone();
                    tokio::spawn(async move {
                        let mut config_path =
                            dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                        config_path.push("moviebox-tui");
                        config_path.push("tv_config.json");

                        let mut loaded_urls = Vec::new();
                        if let Ok(content) = std::fs::read_to_string(&config_path) {
                            if let Ok(urls) = serde_json::from_str::<Vec<String>>(&content) {
                                if !urls.is_empty() {
                                    loaded_urls = urls;
                                }
                            }
                        }

                        if !loaded_urls.is_empty() {
                            let parser = crate::providers::iptv_org::m3u::M3UParser::new();
                            let mut all_channels = Vec::new();
                            for url in loaded_urls {
                                let filename = url.split('/').next_back().unwrap_or("playlist.m3u");
                                if let Ok(channels) = parser.fetch_playlist(&url, filename).await {
                                    all_channels.extend(channels);
                                }
                            }
                            sender.send(Action::TvChannelsLoaded(all_channels)).ok();
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                            sender.send(Action::ShowTvWizard).ok();
                        }
                    });
                } else {
                    self.state.tv_config_popup = false;
                    self.state.search_query.clear();
                    self.state.search_results.clear();
                }
            }
            Action::ShowTvWizard => {
                if self.state.is_tv_mode {
                    self.state.show_help = false;
                    self.state.player_picker_popup = false;
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.tv_config_popup = true;
                    self.state.input_mode = crate::tui::state::InputMode::Normal;
                }
            }
            Action::TvChannelsLoaded(channels) => {
                self.state.tv_channels = channels;
                self.state.is_loading = false;
                self.state.status_message =
                    format!("Loaded {} TV channels.", self.state.tv_channels.len());
                self.state.status_timer = 150;
            }
            Action::GoBack => {
                self.prepare_sixel_redraw();
                if self.state.provider_picker_popup {
                    self.state.provider_picker_popup = false;
                    return None;
                }
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    self.state.player_picker_link = None;
                    self.state.player_picker_subtitle = None;
                    return None;
                }
                if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    self.state.subtitle_popup = false;
                    self.state.is_download_subtitle_popup = false;
                    self.state.pending_play_link = None;
                    return None;
                }
                if self.state.show_help {
                    self.state.show_help = false;
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Home => {
                        if !self.state.search_results.is_empty()
                            || !self.state.search_query.is_empty()
                        {
                            self.state.search_poster_protocols.clear();
                            self.state.search_results.clear();
                            self.state.search_query.clear();
                            self.state.search_preview = None;
                            self.state.status_message = "Search cleared.".to_string();
                            self.state.status_timer = 150;
                        }
                    }
                    Screen::Details => {
                        self.state
                            .fetch_cancel
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        self.state.stream_pool.clear();
                        self.state.pending_episode_fetch = None;
                        self.state.selected_resources = None;
                        self.state.active_screen = Screen::Home;
                        self.state.is_loading = false;
                        self.state.language_chosen = false;
                        self.state.status_message =
                            "Select a movie/series and press Enter".to_string();
                        self.state.status_timer = 150;
                    }
                    Screen::Library => {
                        self.state.active_screen = Screen::Home;
                        self.state.status_message = "Back to search.".to_string();
                        self.state.status_timer = 100;
                    }
                }
            }
            Action::Refresh => match self.state.active_screen {
                Screen::Library => {
                    self.refresh_library(true);
                }
                Screen::Home => {
                    let query = self.state.search_query.trim().to_string();
                    if self.state.is_tv_mode {
                        if query.is_empty() {
                            self.state.status_message =
                                "TV Mode channels are loaded from local config.".to_string();
                            self.state.status_timer = 150;
                        } else {
                            self.action_sender
                                .send(Action::Search {
                                    query,
                                    force_refresh: true,
                                })
                                .ok();
                        }
                    } else if !query.is_empty() {
                        self.action_sender
                            .send(Action::Search {
                                query,
                                force_refresh: true,
                            })
                            .ok();
                    }
                }
                Screen::Details => {
                    if let Some(id) = self.state.active_subject_id.clone() {
                        let se = if self.state.available_seasons.is_empty() {
                            0
                        } else {
                            self.state.selected_season
                        };
                        let ep = if self.state.available_seasons.is_empty() {
                            0
                        } else {
                            self.state.selected_episode
                        };
                        let id_clone = id.clone();
                        let provider = self.state.active_provider;
                        tokio::task::spawn_blocking(move || {
                            crate::cache::invalidate_provider_stream_cache(
                                provider, &id_clone, se, ep,
                            );
                        });
                        self.state.selected_season = se;
                        self.state.selected_episode = ep;
                        self.action_sender
                            .send(Action::FetchEpisodeStreams {
                                subject_id: id,
                                season: se,
                                episode: ep,
                                force_refresh: true,
                            })
                            .ok();
                    }
                }
                _ => {}
            },
            Action::ClearCache => {
                crate::cache::clear_all_cache();
                self.state.status_message = "Cache cleared completely.".to_string();
                self.state.status_timer = 150;
            }
            Action::SelectLanguage(idx) => {
                // Confirm audio only on explicit Enter (not arrow navigation).
                if let Some(details) = &self.state.selected_details
                    && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                    && let Some(dub) = dubs.get(idx)
                    && let Some(id) = dub.get("subjectId").and_then(|i| i.as_str())
                {
                    let next_id = id.to_string();
                    self.state.language_list_state.select(Some(idx));
                    self.state.language_chosen = true;
                    self.state.selected_resources = None;
                    self.state.resource_list_state.select(None);

                    // Same dub already loaded — just advance; don't refetch.
                    if self.state.active_subject_id.as_deref() == Some(next_id.as_str()) {
                        let is_series = !self.state.available_seasons.is_empty();
                        self.state.details_pane = if is_series {
                            crate::tui::state::DetailsPane::Seasons
                        } else {
                            crate::tui::state::DetailsPane::Streams
                        };
                        self.state.status_message = if is_series {
                            "Audio set. Pick a season, then episode.".into()
                        } else {
                            "Audio set.".into()
                        };
                        self.state.status_timer = 150;
                        if !self.state.stream_pool.contains_key(&next_id) {
                            self.action_sender
                                .send(Action::InitStreamPool(next_id))
                                .ok();
                        } else if is_series {
                            self.trigger_episode_fetch();
                        }
                        return None;
                    }

                    self.state.status_message = "Switching audio...".to_string();
                    self.state.status_timer = 150;
                    self.action_sender
                        .send(Action::FetchDetails(next_id, false))
                        .ok();
                }
            }
            Action::Suggest(query) => {
                if query.starts_with('/') {
                    let mut commands = vec!["/clear-cache", "/update", "/toggle-update", "/github"];
                    if self.state.is_tv_mode {
                        commands.push("/list");
                        commands.push("/config");
                    } else {
                        commands.extend(vec![
                            "/discover",
                            "/home",
                            "/movies",
                            "/shows",
                            "/tvshows",
                            "/anime",
                        ]);
                    }
                    let mut suggestions = vec![];
                    for cmd in commands {
                        if cmd.starts_with(&query) {
                            suggestions.push(serde_json::json!({ "title": cmd }));
                        }
                    }
                    if !suggestions.is_empty() {
                        let fake_payload = serde_json::json!({
                            "results": [{
                                "subjects": suggestions
                            }]
                        });
                        self.action_sender
                            .send(Action::SuggestSuccess(query, fake_payload))
                            .ok();
                    }
                    return None;
                }

                if self.state.is_tv_mode {
                    return None;
                }
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state.search_suggestions.clear();
                    return None;
                }

                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let query_clone = query.clone();
                tokio::spawn(async move {
                    if let Ok(res) = client.suggest(&query_clone).await {
                        sender.send(Action::SuggestSuccess(query_clone, res)).ok();
                    }
                });
            }
            Action::SuggestSuccess(query, payload) => {
                if self.state.suggest_index.is_some() {
                    return None;
                }

                let matches = query == self.state.search_query.trim();
                if !matches {
                    return None;
                }

                self.state.search_suggestions.clear();

                let subjects_opt = payload
                    .get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("subjects"))
                    .and_then(|s| s.as_array());

                if let Some(subjects) = subjects_opt {
                    for item in subjects.iter().take(8) {
                        let raw_title = item
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let clean_title = raw_title
                            .split('[')
                            .next()
                            .unwrap_or(&raw_title)
                            .trim()
                            .to_string();

                        let normalized_query = query
                            .to_lowercase()
                            .replace(|c: char| !c.is_alphanumeric(), "");
                        let normalized_title = clean_title
                            .to_lowercase()
                            .replace(|c: char| !c.is_alphanumeric(), "");
                        if !normalized_title.contains(&normalized_query)
                            && !normalized_query.is_empty()
                        {
                            continue;
                        }

                        if !self.state.search_suggestions.contains(&clean_title) {
                            self.state.search_suggestions.push(clean_title);
                        }
                    }
                }
            }
            Action::SelectSuggestion { query } => {
                self.action_sender
                    .send(Action::Search {
                        query,
                        force_refresh: false,
                    })
                    .ok();
            }
            Action::Search {
                query,
                force_refresh,
            } => {
                let lower_query = query.trim().to_lowercase();

                if lower_query == "/clear-cache" {
                    self.action_sender.send(Action::ClearCache).ok();
                    self.state.search_query.clear();
                    return None;
                }

                if lower_query == "/github" {
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "https://github.com/mesamirh/MovieBox-Tui"])
                        .spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg("https://github.com/mesamirh/MovieBox-Tui")
                        .spawn();
                    #[cfg(all(target_os = "linux", not(target_os = "android")))]
                    let _ = std::process::Command::new("xdg-open")
                        .arg("https://github.com/mesamirh/MovieBox-Tui")
                        .spawn();
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    return None;
                }

                if lower_query == "/update" {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.active_screen = Screen::Startup;
                    self.state.update_available = None;
                    self.state.manual_update_check = true;
                    self.action_sender.send(Action::CheckForUpdates).ok();
                    return None;
                }
                if lower_query == "/toggle-update" {
                    self.state.auto_update = !self.state.auto_update;
                    self.persist_config();
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "Automatic updates",
                        if self.state.auto_update {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                    );
                    return None;
                }

                if self.state.is_tv_mode {
                    if lower_query == "/config" {
                        self.action_sender.send(Action::ShowTvWizard).ok();
                        self.state.search_query.clear();
                        return None;
                    }
                    if matches!(
                        lower_query.as_str(),
                        "/home" | "/discover" | "/movies" | "/shows" | "/tvshows" | "/anime"
                    ) {
                        self.state.status_message =
                            "Switch to streaming mode to use this command".to_string();
                        self.state.status_timer = 150;
                        self.state.search_query.clear();
                        return None;
                    }

                    let q = lower_query.clone();
                    self.state.search_results = self
                        .state
                        .tv_channels
                        .iter()
                        .filter(|c| {
                            q == "/list"
                                || c.name.to_lowercase().contains(&q)
                                || c.group.to_lowercase().contains(&q)
                        })
                        .map(|c| SearchResult {
                            id: c.stream_url.clone(),
                            title: c.name.clone(),
                            stype: 3,
                            release_year: c.group.clone(),
                            cover_url: Some(c.logo.clone()),
                            season: 1,
                            provider: ProviderKind::MovieBox,
                            relevance: 500,
                        })
                        .collect();
                    self.state.is_loading = false;
                    self.state
                        .search_list_state
                        .select(if self.state.search_results.is_empty() {
                            None
                        } else {
                            Some(0)
                        });

                    if !self.state.search_results.is_empty() {
                        let results_to_fetch = self
                            .state
                            .search_results
                            .iter()
                            .take(15)
                            .map(|r| (r.id.clone(), r.stype, r.cover_url.clone()))
                            .collect::<Vec<_>>();
                        let sender = self.action_sender.clone();
                        let req_client = self.client.http_client().clone();
                        tokio::spawn(async move {
                            let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
                            for (id, _stype, cover_url) in results_to_fetch {
                                if let Some(url) = cover_url {
                                    if url.is_empty() {
                                        continue;
                                    }
                                    let permit = sem.clone().acquire_owned().await.ok();
                                    let tx = sender.clone();
                                    let client = req_client.clone();
                                    tokio::spawn(async move {
                                        let _permit = permit;
                                        if let Ok(resp) = client
                                            .get(&url)
                                            .header("User-Agent", "MovieBox-Tui/1.0")
                                            .send()
                                            .await
                                        {
                                            if let Ok(bytes) = resp.bytes().await {
                                                let bytes_clone = bytes.clone();
                                                if let Ok(Ok(img)) =
                                                    tokio::task::spawn_blocking(move || {
                                                        image::load_from_memory(&bytes_clone)
                                                    })
                                                    .await
                                                {
                                                    tx.send(Action::SearchPosterLoaded(
                                                        id,
                                                        Some(std::sync::Arc::new(img)),
                                                    ))
                                                    .ok();
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    }
                    self.state.status_message = if self.state.search_results.is_empty() {
                        format!("No matches for '{}'.", query)
                    } else {
                        format!("Found {} channels.", self.state.search_results.len())
                    };
                    self.state.status_timer = 150;
                    return None;
                }

                let tab_id = match lower_query.as_str() {
                    "/home" | "/discover" => Some("0"),
                    "/movies" => Some("2"),
                    "/shows" | "/tvshows" => Some("5"),
                    "/anime" => Some("8"),
                    _ => None,
                };

                if let Some(tid) = tab_id {
                    if self.state.active_provider != ProviderKind::MovieBox {
                        self.state.status_message =
                            "4KHDHub has no discover feed; enter a title to search.".into();
                        self.state.status_timer = 180;
                        return None;
                    }
                    self.action_sender
                        .send(Action::FetchHomepage {
                            tab_id: tid.to_string(),
                            page: 1,
                        })
                        .ok();
                    return None;
                }

                self.state.is_homepage_mode = false;
                self.state.current_page = 1;
                self.state.active_screen = Screen::Home;
                self.state.selected_details = None;
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.search_list_state.select(Some(0));
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state.search_preview = None;
                let scope = self.state.search_scope;
                self.state.status_message = format!(
                    "Searching {} for '{}'...",
                    scope.short_label(),
                    query
                );
                self.state.status_timer = 150;

                let query_clone = query.clone();
                let sender = self.action_sender.clone();
                let client = self.client.clone();
                let fourk_client = self.fourk_client.clone();
                let context = self.request_context();
                let cache_key = format!("{}::{}", scope.cache_key(), query_clone);
                tokio::spawn(async move {
                    if !force_refresh {
                        if let Some(cached) = crate::cache::get_provider_search_cache(
                            ProviderKind::MovieBox,
                            &cache_key,
                        ) {
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    query: query_clone.clone(),
                                    payload: cached,
                                })
                                .ok();
                            return;
                        }
                    }

                    let want_mb = scope.includes(ProviderKind::MovieBox);
                    let want_fourk = scope.includes(ProviderKind::FourKHdHub);

                    let mb_task = if want_mb {
                        let mb_query = query_clone.clone();
                        let mb_client = client.clone();
                        Some(tokio::spawn(async move {
                            mb_client
                                .search(&mb_query, 1)
                                .await
                                .map(|v| tag_subjects_with_provider(v, ProviderKind::MovieBox))
                                .map_err(|e| e.to_string())
                        }))
                    } else {
                        None
                    };
                    let fourk_task = if want_fourk {
                        let fourk_query = query_clone.clone();
                        let fk_client = fourk_client.clone();
                        Some(tokio::spawn(async move {
                            fk_client
                                .search(&fourk_query)
                                .await
                                .map(|items| search_to_moviebox_json(&items))
                                .map_err(|e| e.to_string())
                        }))
                    } else {
                        None
                    };

                    let mut parts = Vec::new();
                    let mut errors = Vec::new();
                    let mut empty_notes = Vec::new();

                    if let Some(task) = mb_task {
                        match task.await {
                            Ok(Ok(payload)) => {
                                let n = payload
                                    .get("results")
                                    .and_then(|r| r.as_array())
                                    .and_then(|a| a.first())
                                    .and_then(|f| f.get("subjects"))
                                    .and_then(|s| s.as_array())
                                    .map(|s| s.len())
                                    .unwrap_or(0);
                                if n == 0 {
                                    empty_notes.push("MovieBox: 0 catalog hits");
                                }
                                parts.push((ProviderKind::MovieBox, payload));
                            }
                            Ok(Err(e)) => errors.push(format!("MovieBox: {e}")),
                            Err(e) => errors.push(format!("MovieBox task: {e}")),
                        }
                    }
                    if let Some(task) = fourk_task {
                        match task.await {
                            Ok(Ok(payload)) => {
                                let n = payload
                                    .get("results")
                                    .and_then(|r| r.as_array())
                                    .and_then(|a| a.first())
                                    .and_then(|f| f.get("subjects"))
                                    .and_then(|s| s.as_array())
                                    .map(|s| s.len())
                                    .unwrap_or(0);
                                if n == 0 {
                                    empty_notes
                                        .push("4KHDHub: not in catalog (or site returned nothing)");
                                }
                                parts.push((ProviderKind::FourKHdHub, payload));
                            }
                            Ok(Err(e)) => errors.push(format!("4KHDHub: {e}")),
                            Err(e) => errors.push(format!("4KHDHub task: {e}")),
                        }
                    }

                    if parts.is_empty() {
                        let msg = if errors.is_empty() {
                            "No providers selected".into()
                        } else {
                            errors.join(" · ")
                        };
                        sender.send(Action::SearchFailure(context, msg)).ok();
                        return;
                    }

                    let mut merged = merge_search_payloads(parts);
                    if let Some(obj) = merged.as_object_mut() {
                        obj.insert(
                            "search_notes".into(),
                            serde_json::json!(empty_notes
                                .into_iter()
                                .chain(errors.iter().map(|s| s.as_str()))
                                .collect::<Vec<_>>()),
                        );
                    }
                    crate::cache::set_provider_search_cache(
                        ProviderKind::MovieBox,
                        &cache_key,
                        &merged,
                    );
                    sender
                        .send(Action::SearchSuccess {
                            context,
                            query: query_clone,
                            payload: merged,
                        })
                        .ok();
                });
            }
            Action::FetchHomepage { tab_id, page } => {
                if self.state.is_tv_mode {
                    return None;
                }
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state.is_loading = false;
                    self.state.status_message =
                        "This provider exposes search, not a shared MovieBox homepage.".into();
                    self.state.status_timer = 180;
                    return None;
                }
                self.state.is_homepage_mode = true;
                self.state.current_tab_id = tab_id.clone();
                self.state.current_page = page;
                self.state.active_screen = Screen::Home;
                self.state.selected_details = None;
                self.state.selected_resources = None;
                self.state.is_loading = true;
                if page == 1 {
                    self.state.search_results.clear();
                    self.state.search_list_state.select(Some(0));
                }
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state.status_message = "Loading discover tab...".to_string();
                self.state.status_timer = 150;

                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let force_refresh = false;

                if !force_refresh {
                    if let Some(cached) = crate::cache::get_homepage_cache(&tab_id, page) {
                        sender
                            .send(Action::HomepageSuccess {
                                tab_id: tab_id.clone(),
                                page,
                                payload: cached,
                            })
                            .ok();
                    }
                }

                tokio::spawn(async move {
                    match client.get_homepage(&tab_id, page).await {
                        Ok(res) => {
                            let r_clone = res.clone();
                            let t_clone = tab_id.clone();
                            let p_clone = page;
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_homepage_cache(&t_clone, p_clone, &r_clone);
                            });
                            sender
                                .send(Action::HomepageSuccess {
                                    tab_id,
                                    page,
                                    payload: res,
                                })
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::HomepageFailure(format!("{:?}", e)))
                                .ok();
                        }
                    }
                });
            }
            Action::SearchSuccess {
                context,
                query,
                payload,
            } => {
                if !self.context_is_current(context) || query != self.state.search_query.trim() {
                    return None;
                }
                self.state.is_loading = false;
                if self.state.current_page <= 1 {
                    self.state.search_results.clear();
                }
                let is_multi = payload
                    .get("multi_provider")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let subjects_opt = payload
                    .get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("subjects"))
                    .and_then(|s| s.as_array());

                let mut provider_counts: std::collections::HashMap<ProviderKind, usize> =
                    std::collections::HashMap::new();

                if let Some(subjects) = subjects_opt {
                    for item in subjects {
                        let id = item
                            .get("subjectId")
                            .and_then(|si| si.as_str())
                            .unwrap_or("")
                            .to_string();
                        if id.is_empty() {
                            continue;
                        }
                        let raw_title = item
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown")
                            .to_string();

                        let clean_title = crate::tui::app::clean_moviebox_title(&raw_title);
                        let provider = provider_from_subject(item, context.provider);

                        // Accuracy-first: score both raw and cleaned titles, keep best.
                        let relevance = search_relevance(&raw_title, &query)
                            .max(search_relevance(&clean_title, &query));
                        if relevance < 400 {
                            continue;
                        }

                        let stype = item
                            .get("subjectType")
                            .and_then(|s| s.as_i64())
                            .unwrap_or(0);
                        let release_year = item
                            .get("releaseDate")
                            .and_then(|rd| rd.as_str())
                            .unwrap_or("N/A")
                            .to_string();

                        let cover_url = item
                            .get("poster")
                            .or_else(|| item.get("cover"))
                            .or_else(|| item.get("pic"))
                            .and_then(|c| {
                                c.as_str().or_else(|| c.get("url").and_then(|u| u.as_str()))
                            })
                            .map(|s| s.to_string());

                        let season =
                            item.get("season").and_then(|s| s.as_u64()).unwrap_or(0) as usize;

                        // Same id + same provider only (providers may reuse id shapes).
                        if let Some(existing) = self
                            .state
                            .search_results
                            .iter_mut()
                            .find(|r| r.id == id && r.provider == provider)
                        {
                            if season > existing.season || relevance > existing.relevance {
                                existing.season = season.max(existing.season);
                                existing.title = clean_title;
                                existing.stype = stype;
                                existing.release_year = release_year;
                                existing.cover_url = cover_url;
                                existing.relevance = existing.relevance.max(relevance);
                            }
                            continue;
                        }

                        let raw_lower = raw_title.to_lowercase();
                        let is_dub = raw_lower.contains("[hindi]")
                            || raw_lower.contains("[tamil]")
                            || raw_lower.contains("[telugu]")
                            || raw_lower.contains("[english]");

                        // Collapse dub variants within the *same* provider only.
                        if is_dub
                            && self.state.search_results.iter().any(|r| {
                                r.provider == provider
                                    && r.title == clean_title
                                    && r.stype == stype
                            })
                        {
                            continue;
                        }

                        // Keep cross-provider duplicates (same title on MovieBox + 4KHDHub)
                        // so the user can pick whichever catalog has streams.
                        if self.state.search_results.iter().any(|r| {
                            r.provider == provider
                                && r.title == clean_title
                                && r.release_year == release_year
                                && r.stype == stype
                        }) {
                            continue;
                        }

                        *provider_counts.entry(provider).or_insert(0) += 1;
                        self.state.search_results.push(SearchResult {
                            id,
                            title: clean_title,
                            stype,
                            release_year,
                            cover_url,
                            season,
                            provider,
                            relevance,
                        });
                    }
                    self.state.search_results.sort_by(|a, b| {
                        b.relevance
                            .cmp(&a.relevance)
                            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                            .then_with(|| b.release_year.cmp(&a.release_year))
                            .then_with(|| a.provider.label().cmp(b.provider.label()))
                    });
                    // Keep the list readable — top accurate hits only.
                    const MAX_RESULTS: usize = 40;
                    if self.state.search_results.len() > MAX_RESULTS {
                        self.state.search_results.truncate(MAX_RESULTS);
                    }
                }

                if !self.state.search_results.is_empty() {
                    let results_to_fetch = self
                        .state
                        .search_results
                        .iter()
                        .take(15)
                        .map(|r| (r.id.clone(), r.stype, r.cover_url.clone()))
                        .collect::<Vec<_>>();

                    let sender = self.action_sender.clone();
                    let req_client = self.client.http_client().clone();
                    tokio::spawn(async move {
                        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
                        for (id, _stype, cover_url) in results_to_fetch {
                            if let Some(url) = cover_url {
                                let permit = sem.clone().acquire_owned().await.ok();
                                let tx = sender.clone();
                                let client = req_client.clone();
                                tokio::spawn(async move {
                                    let _permit = permit;
                                    if let Ok(resp) = client
                                        .get(&url)
                                        .header("User-Agent", "MovieBox-Tui/1.0")
                                        .send()
                                        .await
                                    {
                                        if let Ok(bytes) = resp.bytes().await {
                                            let bytes_clone = bytes.clone();
                                            if let Ok(Ok(img)) =
                                                tokio::task::spawn_blocking(move || {
                                                    image::load_from_memory(&bytes_clone)
                                                })
                                                .await
                                            {
                                                tx.send(Action::SearchPosterLoaded(
                                                    id,
                                                    Some(std::sync::Arc::new(img)),
                                                ))
                                                .ok();
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    });
                }

                let notes = payload
                    .get("search_notes")
                    .and_then(|n| n.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" · ")
                    })
                    .unwrap_or_default();

                self.state.status_message = if self.state.search_results.is_empty() {
                    if notes.is_empty() {
                        format!(
                            "No accurate matches for '{}'. Try a shorter title, or Ctrl+P to change sources.",
                            query
                        )
                    } else {
                        format!(
                            "No accurate matches for '{}'. ({notes})",
                            query
                        )
                    }
                } else if is_multi || provider_counts.len() > 1 {
                    let breakdown = ProviderKind::ENABLED
                        .iter()
                        .filter_map(|p| {
                            let n = provider_counts.get(p).copied().unwrap_or(0);
                            (n > 0).then(|| format!("{}:{}", p.label(), n))
                        })
                        .collect::<Vec<_>>()
                        .join(" · ");
                    if notes.is_empty() {
                        format!(
                            "Found {} results ({breakdown}). Best matches first.",
                            self.state.search_results.len()
                        )
                    } else {
                        format!(
                            "Found {} ({breakdown}). Note: {notes}",
                            self.state.search_results.len()
                        )
                    }
                } else {
                    format!(
                        "Found {} results on {}.",
                        self.state.search_results.len(),
                        match self.state.search_scope {
                            SearchScope::Only(p) => p.label().to_string(),
                            SearchScope::All => context.provider.label().to_string(),
                        }
                    )
                };
                self.state.status_timer = 200;
                if self.state.current_page <= 1 {
                    if let Some(res) = self.state.search_results.first() {
                        self.state.search_list_state.select(Some(0));
                        self.action_sender
                            .send(Action::FetchPreview(res.id.clone()))
                            .ok();
                    } else {
                        self.state.search_list_state.select(None);
                    }
                }
            }

            Action::SearchFailure(context, err) => {
                if !self.context_is_current(context) {
                    return None;
                }
                self.state.is_loading = false;
                self.state.status_message = format!("Search failed: {}", err);
                self.state.status_timer = 150;
            }
            Action::HomepageSuccess {
                tab_id,
                page,
                payload,
            } => {
                if !self.state.is_homepage_mode || self.state.current_tab_id != tab_id {
                    return None;
                }
                self.state.is_loading = false;
                if page == 1 {
                    self.state.search_results.clear();
                }

                let mut extracted_subjects = Vec::new();
                if let Some(items) = payload.get("items").and_then(|i| i.as_array()) {
                    for item in items {
                        if let Some(banner) = item
                            .get("banner")
                            .and_then(|b| b.get("banners"))
                            .and_then(|b| b.as_array())
                        {
                            for b in banner {
                                if let Some(subject) = b.get("subject") {
                                    extracted_subjects.push(subject.clone());
                                }
                            }
                        }
                        if let Some(custom_data) = item
                            .get("customData")
                            .and_then(|c| c.get("items"))
                            .and_then(|i| i.as_array())
                        {
                            for c in custom_data {
                                if let Some(subject) = c.get("subject") {
                                    extracted_subjects.push(subject.clone());
                                }
                            }
                        }
                        if let Some(subjects) = item.get("subjects").and_then(|s| s.as_array()) {
                            for subject in subjects {
                                extracted_subjects.push(subject.clone());
                            }
                        }
                    }
                }

                let mut count = 0;
                for item in extracted_subjects {
                    let id = item
                        .get("subjectId")
                        .and_then(|si| si.as_str())
                        .unwrap_or("")
                        .to_string();
                    let raw_title = item
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let clean_title = crate::tui::app::clean_moviebox_title(&raw_title);
                    let stype = item
                        .get("subjectType")
                        .and_then(|st| st.as_i64())
                        .unwrap_or(0);
                    let release_year = item
                        .get("releaseDate")
                        .and_then(|rd| rd.as_str())
                        .unwrap_or("")
                        .split('-')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let cover_url = item
                        .get("cover")
                        .and_then(|c| c.get("url"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());

                    let season = item.get("season").and_then(|s| s.as_u64()).unwrap_or(0) as usize;

                    if let Some(existing) =
                        self.state.search_results.iter_mut().find(|r| r.id == id)
                    {
                        if season > existing.season {
                            existing.season = season;
                            existing.title = clean_title;
                            existing.stype = stype;
                            existing.release_year = release_year;
                            existing.cover_url = cover_url;
                        }
                        continue;
                    }

                    let raw_lower = raw_title.to_lowercase();
                    let is_dub = raw_lower.contains("[hindi]")
                        || raw_lower.contains("[tamil]")
                        || raw_lower.contains("[telugu]")
                        || raw_lower.contains("[english]");

                    if is_dub
                        && self
                            .state
                            .search_results
                            .iter()
                            .any(|r| r.title == clean_title && r.stype == stype)
                    {
                        continue;
                    }

                    if self.state.search_results.iter().any(|r| {
                        r.title == clean_title && r.release_year == release_year && r.stype == stype
                    }) {
                        continue;
                    }

                    if !id.is_empty() {
                        self.state.search_results.push(SearchResult {
                            id,
                            title: clean_title,
                            stype,
                            release_year,
                            cover_url,
                            season,
                            provider: ProviderKind::MovieBox,
                            relevance: 500,
                        });
                        count += 1;
                    }
                }

                if count > 0 {
                    let results_to_fetch = self
                        .state
                        .search_results
                        .iter()
                        .skip(if page == 1 { 0 } else { (page - 1) * 20 })
                        .take(20)
                        .map(|r| (r.id.clone(), r.stype, r.cover_url.clone()))
                        .collect::<Vec<_>>();

                    let sender = self.action_sender.clone();
                    let req_client = self.client.http_client().clone();
                    tokio::spawn(async move {
                        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
                        for (id, _stype, cover_url) in results_to_fetch {
                            if let Some(url) = cover_url {
                                let permit = sem.clone().acquire_owned().await.ok();
                                let tx = sender.clone();
                                let client = req_client.clone();
                                tokio::spawn(async move {
                                    let _permit = permit;
                                    if let Ok(resp) = client
                                        .get(&url)
                                        .header("User-Agent", "MovieBox-Tui/1.0")
                                        .send()
                                        .await
                                    {
                                        if let Ok(bytes) = resp.bytes().await {
                                            let bytes_clone = bytes.clone();
                                            if let Ok(Ok(img)) =
                                                tokio::task::spawn_blocking(move || {
                                                    image::load_from_memory(&bytes_clone)
                                                })
                                                .await
                                            {
                                                tx.send(Action::SearchPosterLoaded(
                                                    id,
                                                    Some(std::sync::Arc::new(img)),
                                                ))
                                                .ok();
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    });
                }

                if count > 0 && self.state.current_page <= 1 {
                    self.state.search_list_state.select(Some(0));
                    if let Some(first) = self.state.search_results.first() {
                        self.action_sender
                            .send(Action::FetchPreview(first.id.clone()))
                            .ok();
                    }
                } else if count == 0 && self.state.current_page <= 1 {
                    self.state.search_list_state.select(None);
                }

                self.state.status_message =
                    format!("Found {} discover items", self.state.search_results.len());
                self.state.status_timer = 150;
            }
            Action::HomepageFailure(err) => {
                self.state.is_loading = false;
                self.state.status_message = format!("Discover failed: {}", err);
                self.state.status_timer = 150;
            }
            Action::MoveUp => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_sixel_redraw();
                }
                if self.state.provider_picker_popup {
                    let options = SearchScope::menu_options();
                    let i = match self.state.provider_picker_state.selected() {
                        Some(0) | None => options.len().saturating_sub(1),
                        Some(i) => i - 1,
                    };
                    self.state.provider_picker_state.select(Some(i));
                    return None;
                }
                if self.state.player_picker_popup {
                    let i = match self.state.player_picker_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.state.available_players.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.state.player_picker_state.select(Some(i));
                    return None;
                } else if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current > 0 {
                        self.state.subtitle_list_state.select(Some(current - 1));
                    }
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Library => {
                        let current = self.state.library_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            self.state.library_list_state.select(Some(current - 1));
                        }
                    }
                    Screen::Home => {
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            self.state.search_list_state.select(Some(current - 1));
                            if let Some(res) = self.state.search_results.get(current - 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        }
                    }
                    Screen::Details => match self.state.details_pane {
                        crate::tui::state::DetailsPane::Streams => {
                            let current = self.state.resource_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.resource_list_state.select(Some(current - 1));
                            }
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            let current = self.state.season_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.season_list_state.select(Some(current - 1));
                                self.state.episode_list_state.select(Some(0));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            let current = self.state.episode_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.episode_list_state.select(Some(current - 1));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Languages => {
                            // Browse only — confirm with Enter (do not auto-apply).
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.language_list_state.select(Some(current - 1));
                            }
                        }
                    },
                }
            }
            Action::TabPane => {
                self.cycle_details_pane(true);
            }
            Action::BackTabPane => {
                self.cycle_details_pane(false);
            }
            Action::MoveDown => {
                if self.state.active_screen == Screen::Home {
                    self.prepare_sixel_redraw();
                }
                if self.state.provider_picker_popup {
                    let options = SearchScope::menu_options();
                    let i = match self.state.provider_picker_state.selected() {
                        Some(i) if i + 1 < options.len() => i + 1,
                        _ => 0,
                    };
                    self.state.provider_picker_state.select(Some(i));
                    return None;
                }
                if self.state.player_picker_popup {
                    let i = match self.state.player_picker_state.selected() {
                        Some(i) => {
                            if i >= self.state.available_players.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.state.player_picker_state.select(Some(i));
                    return None;
                } else if self.state.subtitle_popup || self.state.is_download_subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current + 1 < self.state.subtitle_list.len() {
                        self.state.subtitle_list_state.select(Some(current + 1));
                    }
                    return None;
                }
                match self.state.active_screen {
                    Screen::Startup => {}
                    Screen::Library => {
                        let current = self.state.library_list_state.selected().unwrap_or(0);
                        if current + 1 < self.state.library_items.len() {
                            self.state.library_list_state.select(Some(current + 1));
                        }
                    }
                    Screen::Home => {
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current + 1 < self.state.search_results.len() {
                            self.state.search_list_state.select(Some(current + 1));
                            if let Some(res) = self.state.search_results.get(current + 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        } else if !self.state.is_tv_mode
                            && !self.state.is_loading
                            && !self.state.search_results.is_empty()
                        {
                            let next_page = self.state.current_page + 1;
                            if self.state.is_homepage_mode {
                                self.action_sender
                                    .send(Action::FetchHomepage {
                                        tab_id: self.state.current_tab_id.clone(),
                                        page: next_page,
                                    })
                                    .ok();
                            } else {
                                self.state.current_page = next_page;
                                let query = self.state.search_query.clone();
                                let client = self.client.clone();
                                let fourk_client = self.fourk_client.clone();
                                let sender = self.action_sender.clone();
                                let context = self.request_context();
                                self.state.is_loading = true;
                                self.state.status_message =
                                    format!("Loading page {}...", next_page);
                                tokio::spawn(async move {
                                    let result = match context.provider {
                                        ProviderKind::MovieBox => client
                                            .search(&query, next_page)
                                            .await
                                            .map_err(|error| format!("{error:?}")),
                                        ProviderKind::FourKHdHub => fourk_client
                                            .search(&query)
                                            .await
                                            .map(|items| search_to_moviebox_json(&items))
                                            .map_err(|error| error.to_string()),
                                    };
                                    match result {
                                        Ok(res) => {
                                            sender
                                                .send(Action::SearchSuccess {
                                                    context,
                                                    query,
                                                    payload: res,
                                                })
                                                .ok();
                                        }
                                        Err(e) => {
                                            sender.send(Action::SearchFailure(context, e)).ok();
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Screen::Details => match self.state.details_pane {
                        crate::tui::state::DetailsPane::Streams => {
                            let res_opt = &self.state.selected_resources;
                            let list_opt = res_opt
                                .as_ref()
                                .and_then(|r| r.get("list"))
                                .and_then(|l| l.as_array());
                            if let Some(list) = list_opt {
                                let current =
                                    self.state.resource_list_state.selected().unwrap_or(0);
                                if current + 1 < list.len() {
                                    self.state.resource_list_state.select(Some(current + 1));
                                }
                            }
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            let current = self.state.season_list_state.selected().unwrap_or(0);
                            if current + 1 < self.state.available_seasons.len() {
                                self.state.season_list_state.select(Some(current + 1));
                                self.state.episode_list_state.select(Some(0));
                                self.trigger_episode_fetch();
                            }
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            let current = self.state.episode_list_state.selected().unwrap_or(0);
                            if let Some(season_idx) = self.state.season_list_state.selected() {
                                if let Some(ep_numbers) =
                                    self.state.available_episode_numbers.get(season_idx)
                                {
                                    if current + 1 < ep_numbers.len() {
                                        self.state.episode_list_state.select(Some(current + 1));
                                        self.trigger_episode_fetch();
                                    }
                                }
                            }
                        }
                        crate::tui::state::DetailsPane::Languages => {
                            // Browse only — confirm with Enter (do not auto-apply).
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if let Some(details) = &self.state.selected_details
                                && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                                && current + 1 < dubs.len()
                            {
                                self.state.language_list_state.select(Some(current + 1));
                            }
                        }
                    },
                }
            }
            Action::MoveLeft => {
                if self.state.active_screen == Screen::Home {
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = self.state.visible_items.max(1);
                    if current > jump {
                        self.state.search_list_state.select(Some(current - jump));
                    } else {
                        self.state.search_list_state.select(Some(0));
                    }
                    if let Some(res) = self
                        .state
                        .search_results
                        .get(self.state.search_list_state.selected().unwrap_or(0))
                    {
                        self.action_sender
                            .send(Action::FetchPreview(res.id.clone()))
                            .ok();
                    }
                }
            }
            Action::MoveRight => {
                if self.state.active_screen == Screen::Home {
                    let current = self.state.search_list_state.selected().unwrap_or(0);
                    let jump = self.state.visible_items.max(1);
                    let total = self.state.search_results.len();
                    if current + jump < total {
                        self.state.search_list_state.select(Some(current + jump));
                    } else if total > 0 {
                        self.state.search_list_state.select(Some(total - 1));
                    }
                    if let Some(res) = self
                        .state
                        .search_results
                        .get(self.state.search_list_state.selected().unwrap_or(0))
                    {
                        self.action_sender
                            .send(Action::FetchPreview(res.id.clone()))
                            .ok();
                    }
                }
            }
            Action::Submit => {
                if self.state.is_loading
                    && !self.state.provider_picker_popup
                    && !self.state.player_picker_popup
                {
                    return None;
                }
                if self.state.last_search_edit.elapsed().as_millis() < 500
                    && !self.state.provider_picker_popup
                    && !self.state.player_picker_popup
                    && !self.state.subtitle_popup
                {
                    return None;
                }
                if self.state.provider_picker_popup {
                    let options = SearchScope::menu_options();
                    let idx = self
                        .state
                        .provider_picker_state
                        .selected()
                        .unwrap_or(0)
                        .min(options.len().saturating_sub(1));
                    if let Some(scope) = options.get(idx).copied() {
                        self.action_sender
                            .send(Action::SetSearchScope(scope))
                            .ok();
                    }
                    return None;
                }
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    let idx = self.state.player_picker_state.selected().unwrap_or(0);
                    if let Some(player) = self.state.available_players.get(idx).copied() {
                        if let Some(source) = self.state.player_picker_playback.take() {
                            self.action_sender
                                .send(Action::LaunchPlayback(player, source))
                                .ok();
                        } else if let Some(link) = self.state.player_picker_link.take() {
                            let sub = self.state.player_picker_subtitle.take();
                            self.action_sender
                                .send(Action::LaunchPlayer(player, link, sub))
                                .ok();
                        }
                    }
                    return None;
                }
                if self.state.subtitle_popup {
                    self.state.subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_url = self
                        .state
                        .subtitle_list
                        .get(idx)
                        .map(|(_, u)| u.clone())
                        .filter(|s| !s.is_empty());
                    if let Some(link) = self.state.pending_play_link.take() {
                        let open_with = self.state.pending_open_with;
                        // Always offer a player choice when more than one is installed.
                        if open_with || self.state.available_players.len() > 1 {
                            self.action_sender
                                .send(Action::ShowPlayerPicker(link, sub_url))
                                .ok();
                        } else {
                            self.action_sender
                                .send(Action::LaunchMpv(link, sub_url))
                                .ok();
                        }
                    }
                    return None;
                } else if self.state.is_download_subtitle_popup {
                    self.state.is_download_subtitle_popup = false;
                    let idx = self.state.subtitle_list_state.selected().unwrap_or(0);
                    let sub_name = self.state.subtitle_list.get(idx).map(|(n, _)| n.clone());
                    let sub_url = self.state.subtitle_list.get(idx).map(|(_, u)| u.clone());
                    let sub_url_final = sub_url.filter(|s| !s.is_empty());

                    if self.state.download_queue_total > 0 {
                        self.state.season_subtitle_preference = sub_name.filter(|n| n != "None");
                    }

                    self.action_sender
                        .send(Action::DownloadStream(sub_url_final))
                        .ok();
                    return None;
                }
                if self.state.active_screen == Screen::Home {
                    let idx_opt = self.state.search_list_state.selected();
                    let item_opt =
                        idx_opt.and_then(|idx| self.state.search_results.get(idx).cloned());
                    if let Some(item) = item_opt {
                        if self.state.is_tv_mode || item.stype == 3 {
                            self.action_sender
                                .send(Action::LaunchMpv(item.id.clone(), None))
                                .ok();
                            return None;
                        }
                        // Route details/streams to the provider that returned this hit.
                        // Don't bump generation / wipe search — soft provider pin only.
                        if self.state.active_provider != item.provider {
                            self.state.active_provider = item.provider;
                            self.persist_config();
                        }
                        self.state.active_screen = Screen::Details;
                        self.state.selected_details = None;
                        self.state.selected_resources = None;
                        self.state.is_loading = true;
                        self.state.is_fetching_streams = false;
                        self.state.stream_error = None;
                        self.state.resource_list_state.select(None);
                        self.state.language_list_state.select(Some(0));
                        self.state.season_list_state.select(Some(0));
                        self.state.episode_list_state.select(Some(0));
                        self.state.language_chosen = false;
                        self.state.poster_image = None;
                        self.state.available_seasons.clear();
                        self.state.status_message = format!(
                            "Loading details for {} ({})...",
                            item.title,
                            item.provider.label()
                        );
                        self.state.status_timer = 150;

                        let sender = self.action_sender.clone();
                        sender
                            .send(Action::FetchDetails(item.id.clone(), false))
                            .ok();
                    }
                }
            }
            Action::FetchDetails(id, force_refresh) => {
                self.state.poster_protocol = None;
                self.state.is_loading = true;
                self.state
                    .fetch_cancel
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.state.status_message = "Fetching details...".to_string();
                self.state.stream_pool.clear();
                let client = self.client.clone();
                let fourk_client = self.fourk_client.clone();
                let sender = self.action_sender.clone();
                let id_clone = id.clone();
                let context = self.request_context();
                tokio::spawn(async move {
                    if !force_refresh {
                        let id_for_cache = id_clone.clone();
                        if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                            crate::cache::get_provider_details_cache(
                                context.provider,
                                &id_for_cache,
                            )
                        })
                        .await
                        {
                            sender
                                .send(Action::DetailsSuccess(context, id_clone.clone(), cached))
                                .ok();
                            return;
                        }
                    }
                    let result = match context.provider {
                        ProviderKind::MovieBox => client
                            .get_details(&id_clone)
                            .await
                            .map_err(|error| format!("{error:?}")),
                        ProviderKind::FourKHdHub => fourk_client
                            .details(&id_clone)
                            .await
                            .map(|details| details_to_moviebox_json(&details))
                            .map_err(|error| error.to_string()),
                    };
                    match result {
                        Ok(details) => {
                            let id_for_cache = id_clone.clone();
                            let details_for_cache = details.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_details_cache(
                                    context.provider,
                                    &id_for_cache,
                                    &details_for_cache,
                                )
                            })
                            .await;
                            sender
                                .send(Action::DetailsSuccess(context, id_clone, details))
                                .ok();
                        }
                        Err(e) => {
                            sender.send(Action::DetailsFailure(context, e)).ok();
                        }
                    }
                });
            }
            Action::FetchPreview(id) => {
                if self.state.is_tv_mode {
                    self.state.preview_loading = false;
                    if !self.state.image_cache.contains(&id) {
                        if let Some(channel) =
                            self.state.tv_channels.iter().find(|c| c.stream_url == id)
                        {
                            let cover_url = channel.logo.clone();
                            if !cover_url.is_empty() {
                                let tx = self.action_sender.clone();
                                let client = self.client.http_client().clone();
                                let id2 = id.clone();
                                tokio::spawn(async move {
                                    if let Ok(resp) = client
                                        .get(&cover_url)
                                        .header("User-Agent", "MovieBox-Tui/1.0")
                                        .send()
                                        .await
                                    {
                                        if let Ok(bytes) = resp.bytes().await {
                                            if let Ok(Ok(img)) =
                                                tokio::task::spawn_blocking(move || {
                                                    image::load_from_memory(&bytes)
                                                })
                                                .await
                                            {
                                                tx.send(Action::SearchPosterLoaded(
                                                    id2,
                                                    Some(std::sync::Arc::new(img)),
                                                ))
                                                .ok();
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    return None;
                }
                // Preview uses the provider that returned this search hit.
                let item_provider = self
                    .state
                    .search_results
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| r.provider)
                    .unwrap_or(self.state.active_provider);

                if item_provider == ProviderKind::FourKHdHub {
                    // Use poster/year already on the search row; no lightweight preview API.
                    self.state.preview_loading = false;
                    if let Some(hit) = self.state.search_results.iter().find(|r| r.id == id) {
                        self.state.search_preview = Some(serde_json::json!({
                            "title": hit.title,
                            "releaseDate": hit.release_year,
                            "subjectType": hit.stype,
                            "cover": { "url": hit.cover_url },
                            "_provider": "fourkhdhub",
                        }));
                    } else {
                        self.state.search_preview = None;
                    }
                    return None;
                }
                if let Some(cached) = self.state.preview_cache.get(&id).cloned() {
                    self.state.preview_loading = false;
                    self.state.search_preview = Some(cached.clone());
                    self.state.poster_image = None;
                    self.state.poster_protocol = None;
                    if let Some(img) = self.state.image_cache.get(&id) {
                        self.state.poster_image = Some((**img).clone());
                    } else if let Some(url) = cached
                        .get("cover")
                        .and_then(|c| c.get("url"))
                        .and_then(|u| u.as_str())
                    {
                        let url = url.to_string();
                        let tx = self.action_sender.clone();
                        let id2 = id.clone();
                        let client = self.client.http_client().clone();
                        tokio::spawn(async move {
                            if let Ok(resp) = client
                                .get(&url)
                                .header("User-Agent", "MovieBox-Tui/1.0")
                                .send()
                                .await
                            {
                                if let Ok(bytes) = resp.bytes().await {
                                    if let Ok(Ok(img)) = tokio::task::spawn_blocking(move || {
                                        image::load_from_memory(&bytes)
                                    })
                                    .await
                                    {
                                        tx.send(Action::PosterSuccess(
                                            id2,
                                            std::sync::Arc::new(img),
                                        ))
                                        .ok();
                                    }
                                }
                            }
                        });
                    }
                    return None;
                }
                self.state.preview_loading = true;
                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    match client.get_details(&id_clone).await {
                        Ok(details) => {
                            sender.send(Action::PreviewSuccess(id_clone, details)).ok();
                        }
                        Err(e) => {
                            sender.send(Action::PreviewFailure(format!("{:?}", e))).ok();
                        }
                    }
                });
            }
            Action::PreviewSuccess(id, json) => {
                let current_id = if self.state.active_screen == Screen::Details {
                    self.state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("id"))
                        .and_then(|i| {
                            i.as_i64()
                                .map(|n| n.to_string())
                                .or_else(|| i.as_str().map(|s| s.to_string()))
                        })
                } else {
                    self.state
                        .search_list_state
                        .selected()
                        .and_then(|idx| self.state.search_results.get(idx))
                        .map(|res| res.id.clone())
                };

                if current_id.as_deref() != Some(id.as_str()) {
                    return None;
                }

                self.state.preview_loading = false;

                self.state.preview_cache.put(id.clone(), json.clone());
                self.state.search_preview = Some(json.clone());
                self.state.poster_image = None;
                self.state.poster_protocol = None;
                if let Some(cached_img) = self.state.image_cache.get(&id) {
                    self.state.poster_image = Some((**cached_img).clone());
                } else if let Some(cover_val) = json.get("cover")
                    && let Some(url) = cover_val.get("url").and_then(|u| u.as_str())
                {
                    let url_clone = url.to_string();
                    let action_tx = self.action_sender.clone();
                    let id_clone = id.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(5))
                            .build()
                            .unwrap_or_default();
                        if let Ok(resp) = client
                            .get(&url_clone)
                            .header("User-Agent", "MovieBox-Tui/1.0")
                            .send()
                            .await
                        {
                            if let Ok(bytes) = resp.bytes().await {
                                if let Ok(Ok(img)) = tokio::task::spawn_blocking(move || {
                                    image::load_from_memory(&bytes)
                                })
                                .await
                                {
                                    let _ = action_tx.send(Action::PosterSuccess(
                                        id_clone,
                                        std::sync::Arc::new(img),
                                    ));
                                }
                            }
                        }
                    });
                }
            }
            Action::PosterSuccess(id, img) => {
                self.state.image_cache.put(id.clone(), img.clone());

                let current_id = self
                    .state
                    .search_list_state
                    .selected()
                    .and_then(|idx| self.state.search_results.get(idx))
                    .map(|res| res.id.clone());

                if current_id.as_deref() == Some(id.as_str()) {
                    self.state.poster_image = Some((*img).clone());
                    self.state.poster_protocol = None;
                }
            }
            Action::SearchPosterLoaded(id, img_opt) => {
                if let Some(img) = img_opt {
                    self.state.search_posters.put(id, img);
                }
            }
            Action::PreviewFailure(err) => {
                self.state.preview_loading = false;
                self.state.status_message = format!("Preview failed: {}", err);
                self.state.status_timer = 150;
            }

            Action::PlayStream(open_with) => {
                if self.state.active_provider == ProviderKind::FourKHdHub {
                    if let Some(release) = self.get_selected_release() {
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing playback",
                            "Resolving the selected mirror.",
                        );
                        let client = self.fourk_client.clone();
                        let sender = self.action_sender.clone();
                        // open_with is forced true when multiple players exist so the
                        // async path always shows the picker after resolve.
                        let force_picker = open_with || self.state.available_players.len() > 1;
                        let default_player = self
                            .state
                            .available_players
                            .first()
                            .copied()
                            .unwrap_or(crate::tui::state::PlayerKind::Mpv);
                        tokio::spawn(async move {
                            match client.resolve_release(&release).await {
                                Ok(source) if force_picker => {
                                    sender.send(Action::ShowPlaybackPicker(source)).ok();
                                }
                                Ok(source) => {
                                    sender
                                        .send(Action::LaunchPlayback(default_player, source))
                                        .ok();
                                }
                                Err(error) => {
                                    sender
                                        .send(Action::SetStatus(format!(
                                            "Error: 4KHDHub source failed: {error}"
                                        )))
                                        .ok();
                                }
                            }
                        });
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
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let resource_id = self.get_selected_resource_id();

                    if let Some(rid) = resource_id {
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing playback",
                            "Fetching subtitles.",
                        );
                        let client = self.client.clone();
                        let sender = self.action_sender.clone();
                        let link_clone = link.clone();
                        tokio::spawn(async move {
                            if let Ok(res) = client.get_ext_captions(&subject_id, &rid).await {
                                sender
                                    .send(Action::ShowSubtitlePopup(link_clone, res, open_with))
                                    .ok();
                            } else {
                                if open_with {
                                    sender.send(Action::ShowPlayerPicker(link_clone, None)).ok();
                                } else {
                                    sender.send(Action::LaunchMpv(link_clone, None)).ok();
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
                }
            }
            Action::ShowSubtitlePopup(link, ext_captions, open_with) => {
                let mut options = vec![("None".to_string(), "".to_string())];

                if let Some(captions_list) =
                    ext_captions.get("extCaptions").and_then(|c| c.as_array())
                {
                    for cap in captions_list {
                        let name = cap
                            .get("lanName")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let url = cap
                            .get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !url.is_empty() {
                            options.push((name, url));
                        }
                    }
                }

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
                let mut options = vec![("None".to_string(), "".to_string())];

                if let Some(captions_list) =
                    ext_captions.get("extCaptions").and_then(|c| c.as_array())
                {
                    for cap in captions_list {
                        let name = cap
                            .get("lanName")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let url = cap
                            .get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !url.is_empty() {
                            options.push((name, url));
                        }
                    }
                }

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
                // If several players are installed, always let the user pick
                // (IINA is often detected first but fails on some stream URLs).
                if self.state.available_players.len() > 1 {
                    self.action_sender
                        .send(Action::ShowPlayerPicker(link, subtitle_url))
                        .ok();
                    return None;
                }
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
            Action::DownloadStream(subtitle_url) => {
                if self.state.active_provider == ProviderKind::FourKHdHub {
                    if let Some(release) = self.get_selected_release() {
                        self.state.notify(
                            NotificationKind::Info,
                            "Preparing download",
                            "Resolving the selected mirror.",
                        );
                        let client = self.fourk_client.clone();
                        let sender = self.action_sender.clone();
                        tokio::spawn(async move {
                            match client.resolve_release(&release).await {
                                Ok(source) => {
                                    sender
                                        .send(Action::StartDownload(subtitle_url, Some(source.url)))
                                        .ok();
                                }
                                Err(error) => {
                                    sender
                                        .send(Action::SetStatus(format!("Resolve failed: {error}")))
                                        .ok();
                                }
                            }
                        });
                    } else {
                        self.action_sender
                            .send(Action::StartDownload(subtitle_url, None))
                            .ok();
                    }
                } else {
                    self.action_sender
                        .send(Action::StartDownload(
                            subtitle_url,
                            self.get_selected_link(),
                        ))
                        .ok();
                }
                return None;
            }
            Action::StartDownload(subtitle_url, link) => {
                self.start_resilient_download(subtitle_url, link);
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
                        "Fetching subtitles.",
                    );
                    let client = self.client.clone();
                    let sender = self.action_sender.clone();
                    tokio::spawn(async move {
                        if let Ok(res) = client.get_ext_captions(&subject_id, &rid).await {
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

                let season_array_idx = self.state.available_seasons.iter().position(|s| {
                    s.get("se").and_then(|v| v.as_i64()).unwrap_or(0) as usize == season_num
                });

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
                    let sender = self.action_sender.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        sender.send(Action::ProcessDownloadQueue).ok();
                    });
                    return None;
                }

                if let Some((season, episode)) = self.state.download_queue.pop_front() {
                    self.state.selected_season = season;
                    self.state.selected_episode = episode;
                    let remaining = self.state.download_queue.len();
                    let total = self.state.download_queue_total;
                    let num = total - remaining;

                    self.state.notify(
                        NotificationKind::Info,
                        "Preparing episode",
                        format!("S{season:02}E{episode:02} · {num}/{total}"),
                    );

                    let subject_id = self.state.active_subject_id.clone().unwrap_or_default();

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

            Action::DetailsSuccess(context, id, payload) => {
                if !self.context_is_current(context) || self.state.active_screen != Screen::Details
                {
                    return None;
                }
                self.state.is_loading = false;
                let mut final_payload = payload.clone();
                if self.state.language_chosen {
                    if let Some(existing) = &self.state.selected_details {
                        if let Some(final_obj) = final_payload.as_object_mut() {
                            if let Some(existing_obj) = existing.as_object() {
                                let preserve_keys = [
                                    "title",
                                    "synopsis",
                                    "cover",
                                    "year",
                                    "releaseDate",
                                    "duration",
                                    "countryName",
                                    "genre",
                                    "imdbRatingValue",
                                    "intro",
                                    "description",
                                    "dubs",
                                ];
                                for key in preserve_keys {
                                    if let Some(v) = existing_obj.get(key) {
                                        final_obj.insert(key.to_string(), v.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                self.state.active_subject_id = Some(id.clone());
                self.state.selected_details = Some(final_payload.clone());
                let payload = final_payload;

                if self.state.poster_image.is_none() {
                    if let Some(cached_img) = self.state.image_cache.get(&id) {
                        self.state.poster_image = Some((**cached_img).clone());
                    } else if let Some(cover_val) = payload.get("cover")
                        && let Some(url) = cover_val.get("url").and_then(|u| u.as_str())
                    {
                        let url_clone = url.to_string();
                        let action_tx = self.action_sender.clone();
                        let id_clone = id.clone();
                        tokio::spawn(async move {
                            let client = reqwest::Client::new();
                            if let Ok(resp) = client
                                .get(&url_clone)
                                .header("User-Agent", "MovieBox-Tui/1.0")
                                .send()
                                .await
                            {
                                if let Ok(bytes) = resp.bytes().await {
                                    if let Ok(Ok(img)) = tokio::task::spawn_blocking(move || {
                                        image::load_from_memory(&bytes)
                                    })
                                    .await
                                    {
                                        let _ = action_tx.send(Action::PosterSuccess(
                                            id_clone,
                                            std::sync::Arc::new(img),
                                        ));
                                    }
                                }
                            }
                        });
                    }
                }

                let stype = payload
                    .get("subjectType")
                    .and_then(|s| s.as_i64())
                    .or_else(|| payload.get("stype").and_then(|s| s.as_i64()))
                    .unwrap_or(1);

                if let Some(seasons_arr) = payload
                    .get("seasons")
                    .and_then(|s| s.get("seasons"))
                    .and_then(|s| s.as_array())
                {
                    self.state.available_seasons = seasons_arr.clone();
                } else if stype == 2 {
                    let max_ep = payload
                        .get("resourceDetectors")
                        .and_then(|r| r.as_array())
                        .and_then(|a| a.first())
                        .and_then(|r| r.get("totalEpisode"))
                        .and_then(|t| t.as_i64())
                        .unwrap_or(1);

                    self.state.available_seasons = vec![serde_json::json!({
                        "se": 1,
                        "maxEp": max_ep,
                        "allEp": ""
                    })];
                } else {
                    self.state.available_seasons.clear();
                }

                self.state.available_episode_numbers.clear();
                for season in &self.state.available_seasons {
                    let all_ep_str = season.get("allEp").and_then(|v| v.as_str()).unwrap_or("");
                    let ep_numbers: Vec<usize> = if !all_ep_str.is_empty() {
                        all_ep_str
                            .split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect()
                    } else {
                        let max_ep =
                            season.get("maxEp").and_then(|m| m.as_i64()).unwrap_or(1) as usize;
                        (1..=max_ep).collect()
                    };
                    self.state.available_episode_numbers.push(ep_numbers);
                }

                self.state.season_list_state.select(Some(0));
                self.state.episode_list_state.select(Some(0));

                if let Some(dubs) = payload.get("dubs").and_then(|d| d.as_array()) {
                    let mut current_idx = 0;
                    for (i, dub) in dubs.iter().enumerate() {
                        let dub_id = dub.get("subjectId").and_then(|i| {
                            i.as_i64()
                                .map(|n| n.to_string())
                                .or_else(|| i.as_str().map(|s| s.to_string()))
                        });
                        if dub_id == Some(id.clone()) {
                            current_idx = i;
                        }
                    }
                    self.state.language_list_state.select(Some(current_idx));
                } else {
                    self.state.language_list_state.select(Some(0));
                }

                if !self.state.language_chosen {
                    self.state.selected_season = 1;
                    self.state.selected_episode = 1;
                }

                let has_multiple_dubs = payload
                    .get("dubs")
                    .and_then(|d| d.as_array())
                    .is_some_and(|a| a.len() > 1);

                if has_multiple_dubs && !self.state.language_chosen {
                    self.state.details_pane = crate::tui::state::DetailsPane::Languages;
                    self.state.is_loading = false;
                    self.state.status_message =
                        "Pick audio with ↑↓, then press Enter to confirm.".to_string();
                    self.state.status_timer = 180;
                } else {
                    if stype == 2 && !self.state.available_seasons.is_empty() {
                        // After confirming audio, land on seasons so the user can continue.
                        self.state.details_pane = crate::tui::state::DetailsPane::Seasons;
                        self.state.status_message =
                            "Audio set. ↑↓ season, Enter → episodes.".to_string();
                        self.state.status_timer = 150;
                    } else {
                        self.state.details_pane = crate::tui::state::DetailsPane::Streams;
                    }

                    self.state.is_loading = true;
                    self.state
                        .fetch_cancel
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    self.action_sender.send(Action::InitStreamPool(id)).ok();
                }
            }
            Action::DetailsFailure(context, err) => {
                if !self.context_is_current(context) {
                    return None;
                }
                self.state.is_loading = false;
                self.state.status_message = format!("Details fetch failed: {}", err);
                self.state.status_timer = 150;
            }
            Action::SetStatus(msg) => {
                if msg.starts_with("Error:") {
                    self.state.notify(
                        NotificationKind::Error,
                        "Operation failed",
                        msg.trim_start_matches("Error:").trim(),
                    );
                } else {
                    self.state.status_message = msg;
                    self.state.status_timer = 150;
                }
            }
            Action::InitStreamPool(subject_id) => {
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state
                        .stream_pool
                        .insert(subject_id.clone(), Default::default());
                    self.trigger_episode_fetch();
                    return None;
                }
                let client = self.client.clone();
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let resolutions = client
                        .fetch_collection_resolutions(&subject_id)
                        .await
                        .unwrap_or_default();
                    sender
                        .send(Action::StreamPoolInitialized(subject_id, resolutions))
                        .ok();
                });
            }
            Action::StreamPoolInitialized(subject_id, resolutions) => {
                if Some(&subject_id) != self.state.active_subject_id.as_ref() {
                    return None;
                }
                let pool = crate::tui::state::SubjectStreamPool {
                    available_resolutions: resolutions,
                    ..Default::default()
                };
                self.state.stream_pool.insert(subject_id.clone(), pool);

                let (se, ep) = if let Some(details) = &self.state.selected_details {
                    let stype = details
                        .get("subjectType")
                        .and_then(|s| s.as_i64())
                        .or_else(|| details.get("stype").and_then(|s| s.as_i64()))
                        .unwrap_or(1);
                    if stype == 2 {
                        let se = if self.state.selected_season > 0 {
                            self.state.selected_season
                        } else {
                            1
                        };
                        let ep = if self.state.selected_episode > 0 {
                            self.state.selected_episode
                        } else {
                            1
                        };
                        (se, ep)
                    } else {
                        (0usize, 0usize)
                    }
                } else {
                    let se = if self.state.selected_season > 0 {
                        self.state.selected_season
                    } else {
                        1
                    };
                    let ep = if self.state.selected_episode > 0 {
                        self.state.selected_episode
                    } else {
                        1
                    };
                    (se, ep)
                };
                let _ = (se, ep);

                self.state.selected_season = se;
                self.state.selected_episode = ep;

                let already_loaded = self
                    .state
                    .selected_resources
                    .as_ref()
                    .and_then(|resources| resources.get("list"))
                    .and_then(|list| list.as_array())
                    .is_some_and(|list| !list.is_empty());
                if already_loaded {
                    if let Some(streams) = self
                        .state
                        .selected_resources
                        .as_ref()
                        .and_then(|resources| resources.get("list"))
                        .and_then(|list| list.as_array())
                        .cloned()
                        && let Some(pool) = self.state.stream_pool.get_mut(&subject_id)
                    {
                        pool.episode_index.insert((se, ep), streams);
                    }
                    self.state.is_loading = false;
                    self.state.is_fetching_streams = false;
                    return None;
                }

                self.action_sender
                    .send(Action::FetchEpisodeStreams {
                        subject_id,
                        season: se,
                        episode: ep,
                        force_refresh: false,
                    })
                    .ok();
            }
            Action::FetchEpisodeStreams {
                subject_id,
                season,
                episode,
                force_refresh,
            } => {
                self.state.active_resource_request =
                    self.state.active_resource_request.wrapping_add(1);
                let request_id = self.state.active_resource_request;
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.selected_resources = None;
                self.state.stream_error = None;

                if force_refresh {
                    if let Some(pool) = self.state.stream_pool.get_mut(&subject_id) {
                        pool.episode_index.remove(&(season, episode));
                    }
                }

                let context = self.request_context();

                if context.provider == ProviderKind::FourKHdHub {
                    let sender = self.action_sender.clone();
                    let client = self.fourk_client.clone();
                    let id = subject_id.clone();
                    tokio::spawn(async move {
                        match client.releases(&id, season, episode).await {
                            Ok(releases) if !releases.is_empty() => {
                                sender
                                    .send(Action::EpisodeStreamsReady(
                                        context,
                                        request_id,
                                        id,
                                        season,
                                        episode,
                                        releases_to_moviebox_json(&releases),
                                    ))
                                    .ok();
                            }
                            Ok(_) => {
                                sender
                                    .send(Action::EpisodeStreamsFailed(
                                        context,
                                        request_id,
                                        id,
                                        season,
                                        episode,
                                        "No exact release found".into(),
                                    ))
                                    .ok();
                            }
                            Err(error) => {
                                sender
                                    .send(Action::EpisodeStreamsFailed(
                                        context,
                                        request_id,
                                        id,
                                        season,
                                        episode,
                                        error.to_string(),
                                    ))
                                    .ok();
                            }
                        }
                    });
                    return None;
                }

                if let Some(pool) = self.state.stream_pool.get_mut(&subject_id) {
                    if !force_refresh {
                        if let Some(cached) = pool.episode_index.get(&(season, episode)) {
                            let sender = self.action_sender.clone();
                            let cached = cached.clone();
                            let cached_subject_id = subject_id.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                                sender
                                    .send(Action::EpisodeStreamsReady(
                                        context,
                                        request_id,
                                        cached_subject_id,
                                        season,
                                        episode,
                                        serde_json::Value::Array(cached),
                                    ))
                                    .ok();
                            });
                            return None;
                        }
                    }

                    let mut absolute_episode = 0;
                    for s_val in &self.state.available_seasons {
                        let se = s_val.get("se").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
                        if se < season {
                            absolute_episode +=
                                s_val.get("maxEp").and_then(|m| m.as_i64()).unwrap_or(1) as usize;
                        }
                    }
                    absolute_episode += episode.saturating_sub(1);
                    let estimated_page = (absolute_episode / 20) + 1;

                    let client = self.client.clone();
                    let sender = self.action_sender.clone();
                    let cancel_token = self.state.fetch_cancel.clone();
                    let id_clone = subject_id.clone();
                    let resolutions = pool.available_resolutions.clone();
                    let is_movie = season == 0 && episode == 0;

                    tokio::spawn(async move {
                        if !force_refresh {
                            let id_for_cache = id_clone.clone();
                            if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                                crate::cache::get_provider_stream_cache(
                                    context.provider,
                                    &id_for_cache,
                                    season,
                                    episode,
                                )
                            })
                            .await
                            {
                                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                                sender
                                    .send(Action::SetStatus("Loaded from cache.".to_string()))
                                    .ok();
                                sender
                                    .send(Action::EpisodeStreamsReady(
                                        context,
                                        request_id,
                                        subject_id.clone(),
                                        season,
                                        episode,
                                        cached,
                                    ))
                                    .ok();
                                return;
                            }
                        }

                        sender
                            .send(Action::SetStatus("Fetching streams...".to_string()))
                            .ok();

                        let mut all_items: Vec<serde_json::Value> = Vec::new();
                        let mut found_target = false;
                        let mut last_err: Option<String> = None;
                        let mut saw_success = false;

                        // One retry with identity rotate if the first pass is empty.
                        for attempt in 0..2 {
                            if attempt == 1 {
                                sender
                                    .send(Action::SetStatus(
                                        "Retrying streams with a fresh session...".into(),
                                    ))
                                    .ok();
                                client.refresh_identity();
                                let _ = client.init().await;
                                tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                                all_items.clear();
                                found_target = false;
                            }

                            if is_movie {
                                let mut page = 1usize;
                                loop {
                                    if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                                        break;
                                    }
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(18),
                                        client.fetch_resource_page(&id_clone, 0, page),
                                    )
                                    .await
                                    {
                                        Ok(Ok((items, pager))) => {
                                            saw_success = true;
                                            let has_more = pager
                                                .get("hasMore")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false);
                                            all_items.extend(items);
                                            if !has_more {
                                                break;
                                            }
                                            page += 1;
                                            if page > 10 {
                                                break;
                                            }
                                        }
                                        Ok(Err(e)) => {
                                            last_err = Some(e.to_string());
                                            break;
                                        }
                                        Err(_) => {
                                            last_err = Some("Stream request timed out".into());
                                            break;
                                        }
                                    }
                                }
                            } else {
                                let mut page = estimated_page;
                                'outer: loop {
                                    if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                                        break 'outer;
                                    }
                                    let mut page_handles = Vec::new();

                                    let res_to_fetch = if resolutions.is_empty() {
                                        vec![0]
                                    } else {
                                        resolutions.clone()
                                    };

                                    for &res in &res_to_fetch {
                                        let c = client.clone();
                                        let id = id_clone.clone();
                                        let ct = cancel_token.clone();
                                        page_handles.push(tokio::spawn(async move {
                                            if ct.load(std::sync::atomic::Ordering::Relaxed) {
                                                return Err("cancelled".to_string());
                                            }
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(18),
                                                c.fetch_resource_page(&id, res, page),
                                            )
                                            .await
                                            {
                                                Ok(Ok((items, pager))) => Ok((items, pager)),
                                                Ok(Err(e)) => Err(e.to_string()),
                                                Err(_) => Err("timeout".into()),
                                            }
                                        }));
                                    }

                                    let mut page_empty = true;
                                    let mut has_more = false;
                                    for handle in page_handles {
                                        match handle.await {
                                            Ok(Ok((items, pager))) => {
                                                saw_success = true;
                                                if !items.is_empty() {
                                                    page_empty = false;
                                                }
                                                if pager
                                                    .get("hasMore")
                                                    .and_then(|v| v.as_bool())
                                                    .unwrap_or(false)
                                                {
                                                    has_more = true;
                                                }
                                                for item in &items {
                                                    let se = item
                                                        .get("se")
                                                        .and_then(|v| v.as_i64())
                                                        .unwrap_or(0)
                                                        as usize;
                                                    let ep = item
                                                        .get("ep")
                                                        .and_then(|v| v.as_i64())
                                                        .unwrap_or(0)
                                                        as usize;
                                                    if se == season && ep == episode {
                                                        found_target = true;
                                                    }
                                                }
                                                all_items.extend(items);
                                            }
                                            Ok(Err(e)) => {
                                                last_err = Some(e);
                                            }
                                            Err(e) => {
                                                last_err = Some(e.to_string());
                                            }
                                        }
                                    }

                                    if found_target || page_empty || !has_more {
                                        break 'outer;
                                    }
                                    page += 1;
                                    if page > 60 {
                                        break;
                                    }
                                }
                            }

                            let target_ok = if is_movie {
                                !all_items.is_empty()
                            } else {
                                found_target
                            };
                            if target_ok && !all_items.is_empty() {
                                break;
                            }
                        }

                        let target_ok = if is_movie {
                            !all_items.is_empty()
                        } else {
                            found_target
                        };

                        if !target_ok || all_items.is_empty() {
                            let detail = last_err.unwrap_or_else(|| {
                                if saw_success {
                                    "No playable streams returned for this title".into()
                                } else {
                                    "Could not reach MovieBox stream servers".into()
                                }
                            });
                            // Keep "limit" wording when the upstream actually said so,
                            // otherwise avoid the old misleading blanket "Rate Limit".
                            let msg = if detail.to_ascii_lowercase().contains("limit")
                                || detail.to_ascii_lowercase().contains("rate")
                                || detail.to_ascii_lowercase().contains("quota")
                            {
                                format!(
                                    "{detail}. Wait ~30s, press r to retry, or Ctrl+P for another provider."
                                )
                            } else {
                                format!(
                                    "{detail}. Press r to retry or Ctrl+P to switch provider."
                                )
                            };
                            sender
                                .send(Action::EpisodeStreamsFailed(
                                    context,
                                    request_id,
                                    id_clone,
                                    season,
                                    episode,
                                    msg,
                                ))
                                .ok();
                        } else {
                            sender
                                .send(Action::EpisodeStreamsReady(
                                    context,
                                    request_id,
                                    id_clone,
                                    season,
                                    episode,
                                    serde_json::Value::Array(all_items),
                                ))
                                .ok();
                        }
                    });
                }
            }
            Action::EpisodeStreamsReady(
                context,
                request_id,
                subject_id,
                target_se,
                target_ep,
                payload,
            ) => {
                if request_id != self.state.active_resource_request {
                    return None;
                }
                if !self.context_is_current(context)
                    || Some(&subject_id) != self.state.active_subject_id.as_ref()
                {
                    return None;
                }
                if target_se != self.state.selected_season
                    || target_ep != self.state.selected_episode
                {
                    return None;
                }

                let mut raw_list = payload.as_array().cloned().unwrap_or_default();

                if let Some(subject_id) = &self.state.active_subject_id {
                    let id = subject_id.clone();
                    if let Some(pool) = self.state.stream_pool.get_mut(&id) {
                        let mut actual_resolutions = std::collections::HashSet::new();

                        for item in raw_list.clone() {
                            if let Some(r) = item.get("resolution").and_then(|r| r.as_u64()) {
                                actual_resolutions.insert(r as u32);
                            }

                            let mut se = item
                                .get("se")
                                .and_then(|v| {
                                    v.as_i64()
                                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                                })
                                .unwrap_or(0) as usize;
                            let mut ep = item
                                .get("ep")
                                .and_then(|v| {
                                    v.as_i64()
                                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                                })
                                .unwrap_or(0) as usize;

                            if target_se == 0 && target_ep == 0 {
                                se = 0;
                                ep = 0;
                            }

                            let entry = pool.episode_index.entry((se, ep)).or_insert_with(Vec::new);
                            let link = item
                                .get("resourceLink")
                                .and_then(|l| l.as_str())
                                .unwrap_or("");
                            if !entry.iter().any(|i| {
                                i.get("resourceLink").and_then(|l| l.as_str()).unwrap_or("") == link
                            }) {
                                entry.push(item);
                            }
                        }

                        if !actual_resolutions.is_empty() {
                            let mut existing: std::collections::HashSet<u32> =
                                pool.available_resolutions.iter().cloned().collect();
                            existing.extend(actual_resolutions);
                            let mut res_vec: Vec<u32> = existing.into_iter().collect();
                            res_vec.sort_unstable_by(|a, b| b.cmp(a));

                            pool.available_resolutions = res_vec;
                        }

                        if let Some(target_streams) =
                            pool.episode_index.get(&(target_se, target_ep))
                        {
                            raw_list = target_streams.clone();
                        } else {
                            raw_list.clear();
                        }
                    }
                }

                let mut filtered = raw_list;

                filtered.sort_by(|a, b| {
                    let res_a = a.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                    let res_b = b.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                    res_b.cmp(&res_a)
                });

                let count = filtered.len();
                let array_payload = serde_json::Value::Array(filtered.clone());
                if count > 0 {
                    if let Some(subject_id) = &self.state.active_subject_id {
                        let id_clone = subject_id.clone();
                        let payload_clone = array_payload.clone();
                        tokio::task::spawn_blocking(move || {
                            crate::cache::set_provider_stream_cache(
                                context.provider,
                                &id_clone,
                                target_se,
                                target_ep,
                                &payload_clone,
                            );
                        });
                    }
                }

                let mut result = serde_json::Map::new();
                result.insert("list".to_string(), array_payload);
                self.state.selected_resources = Some(serde_json::Value::Object(result));
                self.state.is_loading = false;
                self.state.is_fetching_streams = false;
                self.state.stream_error = None;
                self.state
                    .resource_list_state
                    .select(if count > 0 { Some(0) } else { None });
                self.state.status_message = format!("{} streams available.", count);
                self.state.status_timer = 150;

                if self.state.is_waiting_for_download_stream {
                    self.state.is_waiting_for_download_stream = false;

                    let is_season_queue = self.state.download_queue_total > 0;
                    if is_season_queue {
                        let subject_id = self.state.active_subject_id.clone().unwrap_or_default();
                        if let Some(rid) = self.get_selected_resource_id() {
                            let client = self.client.clone();
                            let sender = self.action_sender.clone();
                            let pref = self.state.season_subtitle_preference.clone();
                            let no_pref = pref.is_none();

                            tokio::spawn(async move {
                                if let Ok(res) = client.get_ext_captions(&subject_id, &rid).await {
                                    if no_pref {
                                        sender.send(Action::ShowDownloadSubtitlePopup(res)).ok();
                                    } else if let Some(pref_lang) = pref {
                                        let mut sub_url = None;
                                        if let Some(list) = res.as_array() {
                                            for sub in list {
                                                if let (Some(lang), Some(url)) = (
                                                    sub.get("language").and_then(|l| l.as_str()),
                                                    sub.get("url").and_then(|u| u.as_str()),
                                                ) {
                                                    if lang == pref_lang {
                                                        sub_url = Some(url.to_string());
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        sender.send(Action::DownloadStream(sub_url)).ok();
                                    }
                                } else {
                                    sender.send(Action::DownloadStream(None)).ok();
                                }
                            });
                            return None;
                        }
                    }

                    self.action_sender.send(Action::DownloadStream(None)).ok();
                }
            }
            Action::EpisodeStreamsFailed(
                context,
                request_id,
                subject_id,
                target_se,
                target_ep,
                err,
            ) => {
                if request_id != self.state.active_resource_request {
                    return None;
                }
                if !self.context_is_current(context)
                    || Some(&subject_id) != self.state.active_subject_id.as_ref()
                {
                    return None;
                }
                if target_se != self.state.selected_season
                    || target_ep != self.state.selected_episode
                {
                    return None;
                }
                self.state.is_loading = false;
                self.state.is_fetching_streams = false;
                self.state.selected_resources = None;
                self.state.stream_error = Some(err.clone());
                self.state.status_message = format!("Error: {}", err);
                self.state.status_timer = 150;
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
                self.state.download_queue.clear();
                self.state.download_queue_total = 0;
                self.state.notify(
                    NotificationKind::Error,
                    "Download failed",
                    format!("Partial file preserved. {error}"),
                );
            }
            Action::DownloadPaused(path) => {
                self.state.download_progress = None;
                self.state.download_status = None;
                self.state.download_queue.clear();
                self.state.download_queue_total = 0;
                self.state.notify(
                    NotificationKind::Warning,
                    "Download paused",
                    format!("Start again to resume {path}.part"),
                );
            }
            Action::ClearDownload => {
                self.state.download_progress = None;
                self.state.download_status = None;
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

            Action::PlayersDetected(players) => {
                self.state.available_players = players;
            }
            Action::ShowPlaybackPicker(source) => {
                if self.state.available_players.is_empty() {
                    self.state.status_message =
                        "No media player found. Install mpv, IINA, or VLC.".into();
                    self.state.status_timer = 150;
                    return None;
                }
                self.state.show_help = false;
                self.state.tv_config_popup = false;
                self.state.provider_picker_popup = false;
                self.state.player_picker_popup = true;
                self.state.player_picker_playback = Some(source);
                self.state.player_picker_link = None;
                self.state.player_picker_subtitle = None;
                let idx = crate::tui::player::preferred_index(&self.state.available_players);
                self.state.player_picker_state.select(Some(idx));
                self.state.subtitle_popup = false;
                self.state.status_message =
                    "Choose a player (mpv recommended for streams).".into();
                self.state.status_timer = 150;
            }
            Action::ShowPlayerPicker(link, subtitle) => {
                if self.state.available_players.is_empty() {
                    self.state.available_players = crate::tui::player::detect();
                }
                if self.state.available_players.is_empty() {
                    self.state.notify(
                        NotificationKind::Error,
                        "Player unavailable",
                        "Install mpv, IINA, or VLC.",
                    );
                    return None;
                }
                // Always show the picker so the user can confirm which player to use
                // (library + multi-player stream playback rely on this).
                self.state.show_help = false;
                self.state.tv_config_popup = false;
                self.state.provider_picker_popup = false;
                self.state.player_picker_popup = true;
                self.state.player_picker_playback = None;
                self.state.player_picker_link = Some(link);
                self.state.player_picker_subtitle = subtitle;
                let idx = crate::tui::player::preferred_index(&self.state.available_players);
                self.state.player_picker_state.select(Some(idx));
                self.state.subtitle_popup = false;
                self.state.status_message = "Choose a player (↑↓ then Enter).".into();
                self.state.status_timer = 150;
            }
            Action::OpenLibrary => {
                self.open_library();
            }
            Action::RefreshLibrary => {
                if self.state.active_screen == Screen::Library {
                    self.refresh_library(true);
                }
            }
            Action::PlayLibraryItem { open_with } => {
                self.play_selected_library_item(open_with);
            }
            Action::LaunchPlayer(kind, link, sub) => {
                self.state.player_picker_popup = false;
                let player_name = match kind {
                    crate::tui::state::PlayerKind::Mpv => "mpv",
                    crate::tui::state::PlayerKind::Iina => "IINA",
                    crate::tui::state::PlayerKind::Vlc => "VLC",
                };
                self.state.notify(
                    NotificationKind::Info,
                    "Opening player",
                    format!("Launching {player_name}…"),
                );
                tokio::spawn(async move {
                    let mut local_sub = sub.clone();
                    let mut sub_temp_path = None;
                    if let Some(s_url) = sub {
                        // Local subtitle (library) — use as-is for every player.
                        if std::path::Path::new(&s_url).is_file() {
                            local_sub = Some(s_url);
                        } else if kind == crate::tui::state::PlayerKind::Vlc
                            || kind == crate::tui::state::PlayerKind::Iina
                        {
                            if let Ok(resp) = reqwest::get(&s_url).await {
                                if let Ok(bytes) = resp.bytes().await {
                                    let temp_path = std::env::temp_dir().join(format!(
                                        "moviebox_sub_{}.srt",
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                    ));
                                    if tokio::fs::write(&temp_path, bytes).await.is_ok() {
                                        local_sub = Some(temp_path.to_string_lossy().to_string());
                                        sub_temp_path = Some(temp_path);
                                    }
                                }
                            }
                        } else {
                            // mpv can load remote sub URLs via --sub-file=
                            local_sub = Some(s_url);
                        }
                    }

                    let mut cmd =
                        crate::tui::player::command(kind, &link, local_sub.as_deref(), &[]);
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());

                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        cmd.process_group(0);
                    }

                    if let Err(e) = cmd.spawn() {
                        eprintln!("failed to launch player: {e}");
                    }

                    if let Some(path) = sub_temp_path {
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            let _ = tokio::fs::remove_file(path).await;
                        });
                    }
                });
            }
            Action::LaunchPlayback(kind, source) => {
                self.state.player_picker_popup = false;
                if !crate::tui::player::supports_headers(kind, &source.headers) {
                    self.state.status_message =
                        "This source needs headers VLC cannot provide; use mpv or IINA.".into();
                    self.state.status_timer = 180;
                    return None;
                }
                tokio::spawn(async move {
                    let mut local_sub = source.subtitle.clone();
                    let mut sub_temp_path = None;
                    if kind == crate::tui::state::PlayerKind::Vlc
                        || kind == crate::tui::state::PlayerKind::Iina
                    {
                        if let Some(s_url) = source.subtitle.as_ref() {
                            if let Ok(resp) = reqwest::get(s_url).await {
                                if let Ok(bytes) = resp.bytes().await {
                                    let temp_path = std::env::temp_dir().join(format!(
                                        "moviebox_sub_{}.srt",
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                    ));
                                    if tokio::fs::write(&temp_path, bytes).await.is_ok() {
                                        local_sub = Some(temp_path.to_string_lossy().to_string());
                                        sub_temp_path = Some(temp_path);
                                    }
                                }
                            }
                        }
                    }

                    let mut cmd = crate::tui::player::command(
                        kind,
                        &source.url,
                        local_sub.as_deref(),
                        &source.headers,
                    );
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        cmd.process_group(0);
                    }
                    let _ = cmd.spawn();

                    if let Some(path) = sub_temp_path {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let _ = tokio::fs::remove_file(path).await;
                    }
                });
            }
            Action::CheckForUpdates => {
                let update_sender = self.action_sender.clone();
                tokio::task::spawn_blocking(move || {
                    let start = std::time::Instant::now();
                    let result = crate::tui::updater::check(env!("CARGO_PKG_VERSION"));

                    let elapsed = start.elapsed();
                    if elapsed.as_millis() < 1500 {
                        std::thread::sleep(std::time::Duration::from_millis(1500) - elapsed);
                    }

                    match result {
                        Ok(Some(version)) => {
                            update_sender.send(Action::UpdateAvailable(version)).ok();
                        }
                        Ok(None) => {
                            update_sender
                                .send(Action::UpdateAvailable("none".into()))
                                .ok();
                        }
                        Err(error) => {
                            update_sender
                                .send(Action::UpdateAvailable(format!("error:{}", error)))
                                .ok();
                        }
                    }
                });
            }
            Action::UpdateAvailable(version) => {
                if self.state.active_screen == Screen::Startup {
                    self.state.active_screen = Screen::Home;
                }

                if version == "none" {
                    if self.state.manual_update_check {
                        self.state.notify(
                            NotificationKind::Success,
                            "Up to date",
                            "You are using the latest version.",
                        );
                    }
                    self.state.manual_update_check = false;
                } else if version.starts_with("error:") {
                    let err = version.trim_start_matches("error:");
                    if self.state.manual_update_check {
                        self.state.notify(
                            NotificationKind::Error,
                            "Update check failed",
                            err.to_string(),
                        );
                    }
                    self.state.manual_update_check = false;
                } else {
                    self.state.manual_update_check = false;
                    self.state.update_available = Some(version.clone());
                    self.state.notify(
                        NotificationKind::Info,
                        "Update Available",
                        format!("Version v{} is available! Download at github.com/mesamirh/MovieBox-Tui", version),
                    );
                }
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
                super::screens::startup::draw(frame, main_area, &mut self.state, &self.theme);
            }
            Screen::Home => {
                super::screens::home::draw(frame, main_area, &mut self.state, &self.theme);
            }
            Screen::Details => {
                super::screens::details::draw(frame, main_area, &mut self.state, &self.theme);
            }
            Screen::Library => {
                super::screens::library::draw(frame, main_area, &mut self.state, &self.theme);
            }
        }

        // Global overlays — must work on Library too (not only Home/Details).
        if self.state.player_picker_popup {
            let items = self
                .state
                .available_players
                .iter()
                .map(|k| {
                    match k {
                        crate::tui::state::PlayerKind::Mpv => "mpv",
                        crate::tui::state::PlayerKind::Iina => "IINA",
                        crate::tui::state::PlayerKind::Vlc => "VLC",
                    }
                    .to_string()
                })
                .collect::<Vec<_>>();
            crate::tui::overlay::picker(
                frame,
                main_area,
                &items,
                &mut self.state.player_picker_state,
                crate::tui::overlay::PickerSpec {
                    title: "Open with",
                    confirm_label: "Open",
                    minimum_width: 24,
                },
                &self.theme,
                self.state.basic_terminal,
            );
        }

        if self.state.show_help {
            super::screens::help::draw(frame, main_area, &self.state, &self.theme);
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

        crate::tui::overlay::notifications(
            frame,
            area,
            &self.state.notifications,
            &self.theme,
            self.state.basic_terminal,
        );
    }
}
