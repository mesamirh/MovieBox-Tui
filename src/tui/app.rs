use ratatui::Frame;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::providers::moviebox::client::MovieBoxClient;
use crate::tui::{
    action::Action,
    event::EventHandler,
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

pub struct App {
    state: AppState,
    theme: Theme,
    client: MovieBoxClient,
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
                }
            }
        }

        Self {
            state,
            theme: Theme::new(),
            client: MovieBoxClient::new(),
            action_sender,
            action_receiver,
        }
    }

    fn trigger_episode_fetch(&mut self) {
        if let Some(id) = &self.state.active_subject_id {
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

            let mut found_cached = false;
            if let Some(cached) = crate::cache::get_stream_cache(id, se, ep) {
                found_cached = true;
                if let Some(arr) = cached.as_array() {
                    let count = arr.len();
                    let mut result = serde_json::Map::new();
                    result.insert("list".to_string(), cached.clone());
                    self.state.selected_resources = Some(serde_json::Value::Object(result));
                    self.state.is_loading = false;
                    self.state.is_fetching_streams = false;
                    self.state
                        .resource_list_state
                        .select(if count > 0 { Some(0) } else { None });
                    self.state.status_message =
                        format!("Resolved {} direct stream sources (cached).", count);
                    self.state.status_timer = 150;
                }
            }

            if !found_cached {
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.status_message = "Resolving streams...".to_string();
                self.state.status_timer = 150;

                self.state.pending_episode_fetch = Some((id.clone(), se, ep));
                self.state.last_episode_nav = std::time::Instant::now();
            } else {
                self.state.pending_episode_fetch = None;
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

        let _init_sender = self.action_sender.clone();
        let client_clone = self.client.clone();
        tokio::spawn(async move {
            match client_clone.init().await {
                Ok(_) => {
                    let _ = _init_sender;
                }
                Err(_e) => {}
            }
        });

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.state.auto_update && now.saturating_sub(self.state.last_update_check) > 3600 {
            self.state.last_update_check = now;
            if let Some(config_dir) = dirs::config_dir() {
                let config_path = config_dir.join("moviebox-tui").join("config.json");
                let _ = std::fs::create_dir_all(config_dir.join("moviebox-tui"));
                let config_json = serde_json::json!({
                    "auto_update": self.state.auto_update,
                    "last_update_check": now
                });
                let _ = std::fs::write(config_path, config_json.to_string());
            }
            self.action_sender.send(Action::CheckForUpdates).ok();
        } else {
            self.state.active_screen = Screen::Home;
        }

        let player_sender = self.action_sender.clone();
        tokio::task::spawn_blocking(move || {
            let mut players = Vec::new();
            let which_cmd = if cfg!(target_os = "windows") {
                "where"
            } else {
                "which"
            };
            let check_player = |cmd: &str| -> bool {
                std::process::Command::new(which_cmd)
                    .arg(cmd)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            };

            #[cfg(target_os = "macos")]
            {
                if std::path::Path::new("/Applications/IINA.app").exists() || check_player("iina") {
                    players.push(crate::tui::state::PlayerKind::Iina);
                }
            }
            if std::path::Path::new("/Applications/mpv.app").exists()
                || std::path::Path::new("C:\\Program Files\\mpv\\mpv.exe").exists()
                || check_player("mpv")
            {
                players.push(crate::tui::state::PlayerKind::Mpv);
            }
            if std::path::Path::new("/Applications/VLC.app").exists()
                || std::path::Path::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe").exists()
                || std::path::Path::new("C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe").exists()
                || check_player("vlc")
            {
                players.push(crate::tui::state::PlayerKind::Vlc);
            }
            player_sender.send(Action::PlayersDetected(players)).ok();
        });

        loop {
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
                if self.state.toast_timer > 0 {
                    needs_redraw = true;
                    self.state.toast_timer -= 1;
                    if self.state.toast_timer == 0 {
                        self.state.toast_message = None;
                    }
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

                if self.state.search_query.is_empty()
                    && matches!(self.state.active_screen, crate::tui::state::Screen::Home)
                {
                    let greeting = format!("Welcome back, {}", self.state.username);

                    let prompts = if self.state.is_tv_mode {
                        vec![
                            "What live channel are you looking for?",
                            "Try 'BBC', 'CNN', or 'ESPN'...",
                            "Discover news, sports, or entertainment...",
                            "Search your selected broadcast areas...",
                            "Feeling bored? Find a live stream...",
                            "Search live broadcast channels...",
                        ]
                    } else {
                        vec![
                            "What are you in the mood to watch?",
                            "Try 'Oppenheimer', 'Titanic', or 'Interstellar'...",
                            "Discover your next binge-worthy masterpiece...",
                            "Search for blockbuster movies, hit shows, or anime...",
                            "Feeling nostalgic? Search timeless classics...",
                            "Explore trending titles for your movie night...",
                        ]
                    };
                    let type_speed = 3;
                    let del_speed = 1;
                    let pause1 = 90;
                    let pause2 = 15;

                    let greeting_cycle =
                        greeting.len() * type_speed + pause1 + greeting.len() * del_speed + pause2;
                    let mut total_ticks = 0;
                    for p in prompts.iter() {
                        total_ticks += p.len() * type_speed + pause1 + p.len() * del_speed + pause2;
                    }

                    let tick_u = self.state.tick_count as usize;
                    let mut animated_text = String::new();

                    if tick_u < greeting_cycle {
                        let t = tick_u;
                        let greeting_len = greeting.chars().count();
                        let t_type = greeting_len * type_speed;
                        let t_del = greeting_len * del_speed;
                        let display_len = if t < t_type {
                            t / type_speed
                        } else if t < t_type + pause1 {
                            greeting_len
                        } else if t < t_type + pause1 + t_del {
                            greeting_len.saturating_sub((t - (t_type + pause1)) / del_speed)
                        } else {
                            0
                        };
                        animated_text = greeting
                            .chars()
                            .take(display_len.min(greeting_len))
                            .collect::<String>();
                    } else {
                        let mut t = (tick_u - greeting_cycle) % total_ticks;
                        for p in prompts.iter() {
                            let p_len = p.chars().count();
                            let t_type = p_len * type_speed;
                            let t_del = p_len * del_speed;
                            let cycle = t_type + pause1 + t_del + pause2;
                            if t < cycle {
                                let display_len = if t < t_type {
                                    t / type_speed
                                } else if t < t_type + pause1 {
                                    p_len
                                } else if t < t_type + pause1 + t_del {
                                    p_len.saturating_sub((t - (t_type + pause1)) / del_speed)
                                } else {
                                    0
                                };
                                animated_text =
                                    p.chars().take(display_len.min(p_len)).collect::<String>();
                                break;
                            } else {
                                t -= cycle;
                            }
                        }
                    }
                    if self.state.cached_animated_text != animated_text {
                        self.state.cached_animated_text = animated_text;
                        self.state.dirty = true;
                    }
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
                self.state.poster_protocol = None;
                self.state.search_poster_protocols.clear();
                if self.state.image_picker.is_some() {}
            }
            Action::Resize(_w, _h) => {
                self.state.poster_protocol = None;
                self.state.search_poster_protocols.clear();
                if self.state.image_picker.is_some() {}
            }
            Action::Key(key) => {
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
                }

                if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
                    && self.state.download_progress.is_some()
                {
                    self.action_sender.send(Action::CancelDownload).ok();
                    return None;
                }

                if key.code == KeyCode::F(1) {
                    self.action_sender.send(Action::ToggleHelp).ok();
                    return Some(());
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
                            self.state.search_query.pop();
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
                        Screen::Home => {
                            if self.state.tv_config_popup {
                                let filtered_opts = self.state.filtered_tv_wizard_options();
                                match key.code {
                                    KeyCode::Esc => {
                                        if !self.state.tv_wizard_filter.is_empty() {
                                            self.state.tv_wizard_filter.clear();
                                            self.state.tv_wizard_selected_idx = 0;
                                        } else if self.state.tv_wizard_step == 1 {
                                            self.state.tv_wizard_step = 0;
                                            self.state.tv_wizard_selected_idx = 0;
                                            self.state.tv_wizard_filter.clear();
                                            self.state.tv_wizard_options = vec![
                                                "Grouped by category".to_string(),
                                                "Grouped by language".to_string(),
                                                "Grouped by broadcast area".to_string(),
                                            ];
                                        } else {
                                            self.state.tv_config_popup = false;
                                            self.state.tv_wizard_filter.clear();
                                        }
                                    }
                                    KeyCode::Up => {
                                        let len = filtered_opts.len();
                                        if len > 0 {
                                            if self.state.tv_wizard_selected_idx > 0 {
                                                self.state.tv_wizard_selected_idx -= 1;
                                            } else {
                                                self.state.tv_wizard_selected_idx = len - 1;
                                            }
                                        }
                                    }
                                    KeyCode::Down => {
                                        let len = filtered_opts.len();
                                        if len > 0 {
                                            if self.state.tv_wizard_selected_idx < len.saturating_sub(1) {
                                                self.state.tv_wizard_selected_idx += 1;
                                            } else {
                                                self.state.tv_wizard_selected_idx = 0;
                                            }
                                        }
                                    }
                                    KeyCode::Backspace if self.state.tv_wizard_step == 1 => {
                                        self.state.tv_wizard_filter.pop();
                                        self.state.tv_wizard_selected_idx = 0;
                                    }
                                    KeyCode::Char(c) if self.state.tv_wizard_step == 1 => {
                                        if c == ' ' {
                                            if let Some(opt) = filtered_opts
                                                .get(self.state.tv_wizard_selected_idx)
                                                .cloned()
                                            {
                                                if self.state.tv_wizard_selections.contains(&opt) {
                                                    self.state.tv_wizard_selections.remove(&opt);
                                                } else {
                                                    self.state.tv_wizard_selections.insert(opt);
                                                }
                                            }
                                        } else {
                                            self.state.tv_wizard_filter.push(c);
                                            self.state.tv_wizard_selected_idx = 0;
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
                                                self.state.tv_wizard_filter.clear();
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
                                            self.state.tv_wizard_filter.clear();

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
                                    self.action_sender.send(Action::Submit).ok();
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
                                    if (key.modifiers.is_empty()
                                        || key.modifiers == KeyModifiers::SHIFT) =>
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

                            KeyCode::Up | KeyCode::Char('k') => {
                                self.action_sender.send(Action::MoveUp).ok();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.action_sender.send(Action::MoveDown).ok();
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                if self.state.show_season_download_confirm {
                                    self.state.season_download_confirm_yes_selected = true;
                                } else if self.state.show_episode_download_confirm {
                                    self.state.episode_download_confirm_yes_selected = true;
                                } else {
                                    self.action_sender.send(Action::MoveLeft).ok();
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                if self.state.show_season_download_confirm {
                                    self.state.season_download_confirm_yes_selected = false;
                                } else if self.state.show_episode_download_confirm {
                                    self.state.episode_download_confirm_yes_selected = false;
                                } else {
                                    self.action_sender.send(Action::MoveRight).ok();
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
                                            self.action_sender.send(Action::MoveRight).ok();
                                        }
                                        crate::tui::state::DetailsPane::Episodes => {
                                            self.trigger_episode_fetch();
                                            self.action_sender.send(Action::MoveRight).ok();
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
                if matches!(self.state.active_screen, Screen::Home | Screen::Details) {
                    self.state.show_help = !self.state.show_help;
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
                        if !self.state.search_results.is_empty() {
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
                }
            }
            Action::Refresh => match self.state.active_screen {
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
                        tokio::task::spawn_blocking(move || {
                            crate::cache::invalidate_stream_cache(&id_clone, se, ep);
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
                if let Some(details) = &self.state.selected_details
                    && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                    && let Some(dub) = dubs.get(idx)
                    && let Some(id) = dub.get("subjectId").and_then(|i| i.as_str())
                {
                    let next_id = id.to_string();
                    self.state.selected_resources = None;
                    self.state.resource_list_state.select(None);
                    self.state.language_chosen = true;
                    self.state.status_message = "Switching language...".to_string();
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
                    self.state.updater_progress = None;
                    self.state.updater_status = None;
                    self.state.updater_done = false;
                    self.action_sender.send(Action::CheckForUpdates).ok();
                    return None;
                }
                if lower_query == "/toggle-update" {
                    self.state.auto_update = !self.state.auto_update;
                    if let Some(config_dir) = dirs::config_dir() {
                        let config_path = config_dir.join("moviebox-tui").join("config.json");
                        let _ = std::fs::create_dir_all(config_dir.join("moviebox-tui"));
                        let config_json = serde_json::json!({
                            "auto_update": self.state.auto_update,
                            "last_update_check": self.state.last_update_check
                        });
                        let _ = std::fs::write(config_path, config_json.to_string());
                    }
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.toast_message = Some(format!(
                        "{} Auto Update is now {}",
                        if self.state.basic_terminal {
                            "[!]"
                        } else {
                            "!"
                        },
                        if self.state.auto_update {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    ));
                    self.state.toast_timer = 50;
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
                self.state.status_message = format!("Searching for '{}'...", query);
                self.state.status_timer = 150;

                let query_clone = query.clone();
                let sender = self.action_sender.clone();
                let client = self.client.clone();
                tokio::spawn(async move {
                    if !force_refresh {
                        if let Some(cached) = crate::cache::get_search_cache(&query_clone) {
                            sender
                                .send(Action::SearchSuccess {
                                    query: query_clone.clone(),
                                    payload: cached,
                                })
                                .ok();
                            return;
                        }
                    }
                    match client.search(&query_clone, 1).await {
                        Ok(res) => {
                            crate::cache::set_search_cache(&query_clone, &res);
                            sender
                                .send(Action::SearchSuccess {
                                    query: query_clone,
                                    payload: res,
                                })
                                .ok();
                        }
                        Err(e) => {
                            sender.send(Action::SearchFailure(format!("{:?}", e))).ok();
                        }
                    }
                });
            }
            Action::FetchHomepage { tab_id, page } => {
                if self.state.is_tv_mode {
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
            Action::SearchSuccess { query, payload } => {
                if query != self.state.search_query.trim() {
                    return None;
                }
                self.state.is_loading = false;
                if self.state.current_page <= 1 {
                    self.state.search_results.clear();
                }
                let mut count = 0;
                let subjects_opt = payload
                    .get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("subjects"))
                    .and_then(|s| s.as_array());

                if let Some(subjects) = subjects_opt {
                    for item in subjects {
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

                        let normalized_query = query
                            .to_lowercase()
                            .replace(|c: char| !c.is_alphanumeric(), "");
                        let normalized_title = raw_title
                            .to_lowercase()
                            .replace(|c: char| !c.is_alphanumeric(), "");
                        if !normalized_title.contains(&normalized_query)
                            && !normalized_query.is_empty()
                        {
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
                            r.title == clean_title
                                && r.release_year == release_year
                                && r.stype == stype
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
                            });
                            count += 1;
                        }
                    }
                    let query_lower = query.to_lowercase();
                    self.state.search_results.sort_by(|a, b| {
                        let a_title = a.title.to_lowercase();
                        let b_title = b.title.to_lowercase();

                        let a_exact = a_title == query_lower;
                        let b_exact = b_title == query_lower;

                        let a_starts = a_title.starts_with(&query_lower);
                        let b_starts = b_title.starts_with(&query_lower);

                        b_exact
                            .cmp(&a_exact)
                            .then_with(|| b_starts.cmp(&a_starts))
                            .then_with(|| b.stype.cmp(&a.stype))
                            .then_with(|| b.release_year.cmp(&a.release_year))
                    });
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

                self.state.status_message = format!("Found {} results.", count);
                self.state.status_timer = 150;
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

            Action::SearchFailure(err) => {
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
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.state.language_list_state.select(Some(current - 1));
                                self.action_sender
                                    .send(Action::SelectLanguage(current - 1))
                                    .ok();
                            }
                        }
                    },
                }
            }
            Action::TabPane => {
                if self.state.active_screen == Screen::Details {
                    use crate::tui::state::DetailsPane;
                    let has_languages = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("dubs"))
                        .and_then(|d| d.as_array())
                        .is_some_and(|a| a.len() > 1);

                    let is_series = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("stype").or_else(|| d.get("subjectType")))
                        .and_then(|t| t.as_i64())
                        .is_some_and(|t| t == 2);

                    self.state.details_pane = match self.state.details_pane {
                        DetailsPane::Languages => {
                            if is_series {
                                DetailsPane::Seasons
                            } else {
                                DetailsPane::Streams
                            }
                        }
                        DetailsPane::Seasons => DetailsPane::Episodes,
                        DetailsPane::Episodes => DetailsPane::Streams,
                        DetailsPane::Streams => {
                            if has_languages {
                                DetailsPane::Languages
                            } else if is_series {
                                DetailsPane::Seasons
                            } else {
                                DetailsPane::Streams
                            }
                        }
                    };
                }
            }
            Action::MoveDown => {
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
                                let sender = self.action_sender.clone();
                                self.state.is_loading = true;
                                self.state.status_message =
                                    format!("Loading page {}...", next_page);
                                tokio::spawn(async move {
                                    match client.search(&query, next_page).await {
                                        Ok(res) => {
                                            sender
                                                .send(Action::SearchSuccess {
                                                    query,
                                                    payload: res,
                                                })
                                                .ok();
                                        }
                                        Err(e) => {
                                            sender
                                                .send(Action::SearchFailure(format!("{:?}", e)))
                                                .ok();
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
                            let current = self.state.language_list_state.selected().unwrap_or(0);
                            if let Some(details) = &self.state.selected_details
                                && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                                && current + 1 < dubs.len()
                            {
                                self.state.language_list_state.select(Some(current + 1));
                                self.action_sender
                                    .send(Action::SelectLanguage(current + 1))
                                    .ok();
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
                } else if self.state.active_screen == Screen::Details {
                    let has_languages = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("dubs"))
                        .and_then(|d| d.as_array())
                        .is_some_and(|a| a.len() > 1);
                    let is_series = !self.state.available_seasons.is_empty();

                    match self.state.details_pane {
                        crate::tui::state::DetailsPane::Streams => {
                            if is_series {
                                self.state.details_pane = crate::tui::state::DetailsPane::Episodes;
                            } else if has_languages {
                                self.state.details_pane = crate::tui::state::DetailsPane::Languages;
                            }
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            self.state.details_pane = crate::tui::state::DetailsPane::Seasons;
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            if has_languages {
                                self.state.details_pane = crate::tui::state::DetailsPane::Languages;
                            }
                        }
                        crate::tui::state::DetailsPane::Languages => {}
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
                } else if self.state.active_screen == Screen::Details {
                    let has_languages = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("dubs"))
                        .and_then(|d| d.as_array())
                        .is_some_and(|a| a.len() > 1);
                    let is_series = !self.state.available_seasons.is_empty();

                    match self.state.details_pane {
                        crate::tui::state::DetailsPane::Languages => {
                            if !has_languages || self.state.language_chosen {
                                if is_series {
                                    self.state.details_pane =
                                        crate::tui::state::DetailsPane::Seasons;
                                } else {
                                    self.state.details_pane =
                                        crate::tui::state::DetailsPane::Streams;
                                }
                            }
                        }
                        crate::tui::state::DetailsPane::Seasons => {
                            self.state.details_pane = crate::tui::state::DetailsPane::Episodes;
                        }
                        crate::tui::state::DetailsPane::Episodes => {
                            self.state.details_pane = crate::tui::state::DetailsPane::Streams;
                        }
                        crate::tui::state::DetailsPane::Streams => {}
                    }
                }
            }
            Action::Submit => {
                if self.state.is_loading {
                    return None;
                }
                if self.state.last_search_edit.elapsed().as_millis() < 500 {
                    return None;
                }
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    let idx = self.state.player_picker_state.selected().unwrap_or(0);
                    if let Some(player) = self.state.available_players.get(idx).copied() {
                        if let Some(link) = self.state.player_picker_link.take() {
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
                    let sub_url = self.state.subtitle_list.get(idx).map(|(_, u)| u.clone());
                    if let Some(link) = self.state.pending_play_link.take() {
                        let open_with = self.state.pending_open_with;
                        if open_with {
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
                        self.state.active_screen = Screen::Details;
                        self.state.selected_details = None;
                        self.state.selected_resources = None;
                        self.state.resource_list_state.select(None);
                        self.state.language_list_state.select(Some(0));
                        self.state.season_list_state.select(Some(0));
                        self.state.episode_list_state.select(Some(0));
                        self.state.language_chosen = false;
                        self.state.poster_image = None;
                        self.state.available_seasons.clear();
                        self.state.status_message =
                            format!("Loading details for {}...", item.title);
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
                let sender = self.action_sender.clone();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    if !force_refresh {
                        let id_for_cache = id_clone.clone();
                        if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                            crate::cache::get_details_cache(&id_for_cache)
                        })
                        .await
                        {
                            sender
                                .send(Action::DetailsSuccess(id_clone.clone(), cached))
                                .ok();
                            return;
                        }
                    }
                    match client.get_details(&id_clone).await {
                        Ok(details) => {
                            let id_for_cache = id_clone.clone();
                            let details_for_cache = details.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_details_cache(&id_for_cache, &details_for_cache)
                            })
                            .await;
                            sender.send(Action::DetailsSuccess(id_clone, details)).ok();
                        }
                        Err(e) => {
                            sender.send(Action::DetailsFailure(format!("{:?}", e))).ok();
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
                        self.state.toast_message = Some(format!(
                            "{} Fetching subtitles...",
                            if self.state.basic_terminal {
                                "[OK]"
                            } else {
                                "✓"
                            }
                        ));
                        self.state.toast_timer = 40;
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
                    self.state.is_download_subtitle_popup = true;
                    self.state.subtitle_list = options;
                    self.state.subtitle_list_state.select(Some(0));
                } else {
                    self.action_sender.send(Action::DownloadStream(None)).ok();
                }
            }
            Action::LaunchMpv(link, subtitle_url) => {
                let player = self.state.available_players.first().cloned();
                match player {
                    None => {
                        self.state.toast_message = Some(format!(
                            "{} No media player found. Install mpv, IINA, or VLC.",
                            if self.state.basic_terminal {
                                "[X]"
                            } else {
                                "✗"
                            }
                        ));
                        self.state.toast_timer = 150;
                    }
                    Some(kind) => {
                        let player_name = match kind {
                            crate::tui::state::PlayerKind::Mpv => "MPV",
                            crate::tui::state::PlayerKind::Iina => "IINA",
                            crate::tui::state::PlayerKind::Vlc => "VLC",
                        };
                        self.state.toast_message = Some(format!(
                            "{} Launching {}...",
                            if self.state.basic_terminal {
                                "[OK]"
                            } else {
                                "✓"
                            },
                            player_name
                        ));
                        self.state.toast_timer = 40;

                        self.action_sender
                            .send(Action::LaunchPlayer(kind, link, subtitle_url))
                            .ok();
                    }
                }
            }
            Action::DownloadStream(subtitle_url) => {
                if self.state.download_progress.is_some() {
                    return None;
                }
                if self.state.active_screen == Screen::Details {
                    let link_opt = self.get_selected_link();
                    let title = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("title"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("MovieBox-Tui_Stream")
                        .to_string();
                    let stype = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("stype").or_else(|| d.get("subjectType")))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1);
                    let season = self.state.selected_season;
                    let episode = self.state.selected_episode;

                    let mut sanitized_title = title.replace(" ", "_");
                    for c in &['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
                        sanitized_title = sanitized_title.replace(*c, "_");
                    }

                    if let Some(link) = link_opt {
                        self.state.is_waiting_for_download_stream = false;
                        self.state.toast_message = Some(format!(
                            "{} Starting native download...",
                            if self.state.basic_terminal {
                                "[OK]"
                            } else {
                                "✓"
                            }
                        ));
                        self.state.toast_timer = 40;
                        self.state.download_status = Some("Connecting...".to_string());
                        self.state.download_progress = Some(0.0);
                        self.state
                            .cancel_download
                            .store(false, std::sync::atomic::Ordering::SeqCst);

                        let cancel_token = self.state.cancel_download.clone();
                        let sender = self.action_sender.clone();
                        let client = reqwest::Client::builder()
                            .connect_timeout(std::time::Duration::from_secs(10))
                            .tcp_keepalive(std::time::Duration::from_secs(30))
                            .build()
                            .unwrap_or_else(|_| self.client.http_client().clone());
                        tokio::spawn(async move {
                            let head_res = client.head(&link).send().await;
                            let (total_size, supports_ranges, ext) = match head_res {
                                Ok(r) => {
                                    let size = r.content_length().unwrap_or(0);
                                    let ranges = r
                                        .headers()
                                        .get(reqwest::header::ACCEPT_RANGES)
                                        .and_then(|v| v.to_str().ok())
                                        .unwrap_or("")
                                        == "bytes";
                                    let ext = r
                                        .headers()
                                        .get(reqwest::header::CONTENT_DISPOSITION)
                                        .and_then(|v| v.to_str().ok())
                                        .and_then(|s| s.split('.').next_back())
                                        .unwrap_or("mp4")
                                        .to_string();
                                    (size, ranges, ext)
                                }
                                Err(e) => {
                                    sender
                                        .send(Action::UpdateDownload(
                                            None,
                                            Some(format!("Head Error: {}", e)),
                                        ))
                                        .ok();
                                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                    sender.send(Action::UpdateDownload(None, None)).ok();
                                    return;
                                }
                            };

                            let base_dir = dirs::download_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."))
                                .join("MovieBox-TUI");

                            let (target_dir, base_filename) = if stype == 2 {
                                let dir = base_dir
                                    .join("Series")
                                    .join(&sanitized_title)
                                    .join(format!("Season {}", season));
                                let fname =
                                    format!("{}_S{:02}E{:02}", sanitized_title, season, episode);
                                (dir, fname)
                            } else {
                                let dir = base_dir.join("Movies");
                                (dir, sanitized_title.clone())
                            };

                            std::fs::create_dir_all(&target_dir).ok();

                            let mut filename = format!("{}.{}", base_filename, ext);
                            let mut filepath = target_dir.join(&filename);

                            let mut counter = 2;
                            while filepath.exists() {
                                filename = format!("{}_{}.{}", base_filename, counter, ext);
                                filepath = target_dir.join(&filename);
                                counter += 1;
                            }

                            if let Some(sub_url) = subtitle_url {
                                let sub_ext = sub_url.split('.').next_back().unwrap_or("srt");
                                let sub_ext = if sub_ext.len() <= 4 { sub_ext } else { "srt" };

                                let mut sub_filename = filename.clone();
                                if let Some(dot_idx) = sub_filename.rfind('.') {
                                    sub_filename.truncate(dot_idx);
                                }
                                sub_filename.push_str(&format!(".{}", sub_ext));

                                let sub_target = target_dir.join(&sub_filename);
                                let sub_client = client.clone();
                                tokio::spawn(async move {
                                    if let Ok(res) = sub_client.get(&sub_url).send().await {
                                        if let Ok(bytes) = res.bytes().await {
                                            let _ = tokio::fs::write(sub_target, bytes).await;
                                        }
                                    }
                                });
                            }

                            if supports_ranges && total_size > 5 * 1024 * 1024 {
                                let num_connections = 16;
                                let chunk_size = total_size / num_connections;
                                let mut handles = vec![];

                                let downloaded_total =
                                    std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                                let start_time = std::time::Instant::now();

                                let ui_downloaded = downloaded_total.clone();
                                let ui_sender = sender.clone();
                                let ui_cancel = cancel_token.clone();
                                let ui_handle = tokio::spawn(async move {
                                    loop {
                                        tokio::time::sleep(std::time::Duration::from_millis(200))
                                            .await;
                                        if ui_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                                            break;
                                        }

                                        let current_dl = ui_downloaded
                                            .load(std::sync::atomic::Ordering::Relaxed);
                                        let progress = (current_dl as f64 / total_size as f64)
                                            .clamp(0.0, 1.0)
                                            * 100.0;
                                        let elapsed = start_time.elapsed().as_secs_f64();
                                        let speed_bps = if elapsed > 0.0 {
                                            current_dl as f64 / elapsed
                                        } else {
                                            0.0
                                        };
                                        let speed_mbps = speed_bps / 1024.0 / 1024.0;
                                        let remaining_bytes =
                                            total_size.saturating_sub(current_dl) as f64;
                                        let eta_secs = if speed_bps > 0.0 {
                                            remaining_bytes / speed_bps
                                        } else {
                                            0.0
                                        };

                                        let status = format!(
                                            "{:.1} MB / {:.1} MB | {:.1} MB/s | ETA: {:.0}s [16x]",
                                            current_dl as f64 / 1024.0 / 1024.0,
                                            total_size as f64 / 1024.0 / 1024.0,
                                            speed_mbps,
                                            eta_secs
                                        );
                                        ui_sender
                                            .send(Action::UpdateDownload(
                                                Some(progress),
                                                Some(status),
                                            ))
                                            .ok();
                                        if current_dl >= total_size {
                                            break;
                                        }
                                    }
                                });

                                let mut temp_files = vec![];
                                for i in 0..num_connections {
                                    let start = i * chunk_size;
                                    let end = if i == num_connections - 1 {
                                        total_size - 1
                                    } else {
                                        start + chunk_size - 1
                                    };

                                    let part_filepath =
                                        filepath.with_extension(format!("part{}", i));
                                    temp_files.push(part_filepath.clone());

                                    let client_clone = client.clone();
                                    let link_clone = link.clone();
                                    let dl_total = downloaded_total.clone();
                                    let c_token = cancel_token.clone();

                                    handles.push(tokio::spawn(async move {
                                        let file_res =
                                            tokio::fs::File::create(&part_filepath).await;
                                        if file_res.is_err() {
                                            return Err(());
                                        }
                                        let mut file = tokio::io::BufWriter::with_capacity(
                                            128 * 1024,
                                            file_res.unwrap(),
                                        );

                                        let req = client_clone
                                            .get(&link_clone)
                                            .header(
                                                reqwest::header::RANGE,
                                                format!("bytes={}-{}", start, end),
                                            )
                                            .send()
                                            .await;
                                        if req.is_err() {
                                            return Err(());
                                        }
                                        let mut res = req.unwrap();

                                        use tokio::io::AsyncWriteExt;
                                        let expected_size = end - start + 1;
                                        let mut part_downloaded = 0;
                                        loop {
                                            match res.chunk().await {
                                                Ok(Some(chunk)) => {
                                                    if c_token
                                                        .load(std::sync::atomic::Ordering::Relaxed)
                                                    {
                                                        return Err(());
                                                    }

                                                    let chunk_to_write = if part_downloaded
                                                        + chunk.len() as u64
                                                        > expected_size
                                                    {
                                                        &chunk[..(expected_size - part_downloaded)
                                                            as usize]
                                                    } else {
                                                        &chunk[..]
                                                    };

                                                    if file.write_all(chunk_to_write).await.is_err()
                                                    {
                                                        return Err(());
                                                    }
                                                    dl_total.fetch_add(
                                                        chunk_to_write.len() as u64,
                                                        std::sync::atomic::Ordering::Relaxed,
                                                    );
                                                    part_downloaded += chunk_to_write.len() as u64;

                                                    if part_downloaded >= expected_size {
                                                        break;
                                                    }
                                                }
                                                Ok(None) => break,
                                                Err(_) => return Err(()),
                                            }
                                        }

                                        if part_downloaded < expected_size {
                                            return Err(());
                                        }
                                        let _ = file.flush().await;
                                        Ok(())
                                    }));
                                }

                                let mut any_err = false;
                                for h in handles {
                                    if let Ok(res) = h.await {
                                        if res.is_err() {
                                            any_err = true;
                                        }
                                    } else {
                                        any_err = true;
                                    }
                                }
                                ui_handle.abort();

                                if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                                    for file in temp_files {
                                        let _ = tokio::fs::remove_file(file).await;
                                    }
                                    let _ = tokio::fs::remove_file(&filepath).await;
                                    sender
                                        .send(Action::UpdateDownload(
                                            None,
                                            Some("Download Cancelled".to_string()),
                                        ))
                                        .ok();
                                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                    sender.send(Action::UpdateDownload(None, None)).ok();
                                    return;
                                }

                                if any_err {
                                    for tmp in &temp_files {
                                        let _ = tokio::fs::remove_file(tmp).await;
                                    }
                                    sender
                                        .send(Action::UpdateDownload(
                                            None,
                                            Some("Failed to download parts".to_string()),
                                        ))
                                        .ok();
                                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                    sender.send(Action::UpdateDownload(None, None)).ok();
                                    return;
                                }

                                sender
                                    .send(Action::UpdateDownload(
                                        Some(100.0),
                                        Some("Merging parts...".to_string()),
                                    ))
                                    .ok();
                                if let Ok(final_file) = tokio::fs::File::create(&filepath).await {
                                    use tokio::io::AsyncWriteExt;
                                    let mut final_buf = tokio::io::BufWriter::with_capacity(
                                        1024 * 1024,
                                        final_file,
                                    );
                                    let mut merge_ok = true;
                                    for tmp in &temp_files {
                                        if let Ok(mut part_file) = tokio::fs::File::open(tmp).await
                                        {
                                            if tokio::io::copy(&mut part_file, &mut final_buf)
                                                .await
                                                .is_err()
                                            {
                                                merge_ok = false;
                                                break;
                                            }
                                        } else {
                                            merge_ok = false;
                                            break;
                                        }
                                    }
                                    let _ = final_buf.flush().await;
                                    for tmp in &temp_files {
                                        let _ = tokio::fs::remove_file(tmp).await;
                                    }

                                    if merge_ok {
                                        sender
                                            .send(Action::UpdateDownload(
                                                Some(100.0),
                                                Some("Completed!".to_string()),
                                            ))
                                            .ok();
                                    } else {
                                        let _ = tokio::fs::remove_file(&filepath).await;
                                        sender
                                            .send(Action::UpdateDownload(
                                                None,
                                                Some("Failed to merge parts".to_string()),
                                            ))
                                            .ok();
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                sender.send(Action::UpdateDownload(None, None)).ok();
                            } else {
                                match client.get(&link).send().await {
                                    Ok(mut response) => {
                                        if !response.status().is_success() {
                                            sender
                                                .send(Action::UpdateDownload(
                                                    None,
                                                    Some(format!(
                                                        "Error: Status {}",
                                                        response.status()
                                                    )),
                                                ))
                                                .ok();
                                            return;
                                        }
                                        let mut downloaded: u64 = 0;
                                        if let Ok(file) = tokio::fs::File::create(&filepath).await {
                                            use tokio::io::AsyncWriteExt;
                                            let mut buf_writer =
                                                tokio::io::BufWriter::with_capacity(
                                                    1024 * 1024,
                                                    file,
                                                );
                                            let start_time = std::time::Instant::now();
                                            let mut last_ui_update = std::time::Instant::now();

                                            sender
                                                .send(Action::UpdateDownload(
                                                    None,
                                                    Some(format!("Downloading to {}", filename)),
                                                ))
                                                .ok();
                                            loop {
                                                match response.chunk().await {
                                                    Ok(Some(chunk)) => {
                                                        if cancel_token.load(
                                                            std::sync::atomic::Ordering::Relaxed,
                                                        ) {
                                                            let _ =
                                                                tokio::fs::remove_file(&filepath)
                                                                    .await;
                                                            sender
                                                                .send(Action::UpdateDownload(
                                                                    None,
                                                                    Some(
                                                                        "Download Cancelled"
                                                                            .to_string(),
                                                                    ),
                                                                ))
                                                                .ok();
                                                            tokio::time::sleep(
                                                                std::time::Duration::from_secs(2),
                                                            )
                                                            .await;
                                                            sender
                                                                .send(Action::UpdateDownload(
                                                                    None, None,
                                                                ))
                                                                .ok();
                                                            return;
                                                        }
                                                        if buf_writer
                                                            .write_all(&chunk)
                                                            .await
                                                            .is_err()
                                                        {
                                                            sender
                                                                .send(Action::UpdateDownload(
                                                                    None,
                                                                    Some(
                                                                        "File write error!"
                                                                            .to_string(),
                                                                    ),
                                                                ))
                                                                .ok();
                                                            return;
                                                        }
                                                        downloaded += chunk.len() as u64;

                                                        let now = std::time::Instant::now();
                                                        if now
                                                            .duration_since(last_ui_update)
                                                            .as_millis()
                                                            > 200
                                                        {
                                                            last_ui_update = now;
                                                            let progress = if total_size > 0 {
                                                                (downloaded as f64
                                                                    / total_size as f64)
                                                                    * 100.0
                                                            } else {
                                                                0.0
                                                            };

                                                            let elapsed = now
                                                                .duration_since(start_time)
                                                                .as_secs_f64();
                                                            let speed_bps = if elapsed > 0.0 {
                                                                downloaded as f64 / elapsed
                                                            } else {
                                                                0.0
                                                            };
                                                            let speed_mbps =
                                                                speed_bps / 1024.0 / 1024.0;

                                                            let remaining_bytes = total_size
                                                                .saturating_sub(downloaded)
                                                                as f64;
                                                            let eta_secs = if speed_bps > 0.0 {
                                                                remaining_bytes / speed_bps
                                                            } else {
                                                                0.0
                                                            };

                                                            let status = format!(
                                                                "{:.1} MB / {:.1} MB | {:.1} MB/s | ETA: {:.0}s [1x]",
                                                                downloaded as f64 / 1024.0 / 1024.0,
                                                                total_size as f64 / 1024.0 / 1024.0,
                                                                speed_mbps,
                                                                eta_secs
                                                            );
                                                            sender
                                                                .send(Action::UpdateDownload(
                                                                    Some(progress),
                                                                    Some(status),
                                                                ))
                                                                .ok();
                                                        }
                                                    }
                                                    Ok(None) => break,
                                                    Err(e) => {
                                                        let _ =
                                                            tokio::fs::remove_file(&filepath).await;
                                                        sender
                                                            .send(Action::UpdateDownload(
                                                                None,
                                                                Some(format!(
                                                                    "Stream Error: {}",
                                                                    e
                                                                )),
                                                            ))
                                                            .ok();
                                                        tokio::time::sleep(
                                                            std::time::Duration::from_secs(3),
                                                        )
                                                        .await;
                                                        sender
                                                            .send(Action::UpdateDownload(
                                                                None, None,
                                                            ))
                                                            .ok();
                                                        return;
                                                    }
                                                }
                                            }

                                            if total_size > 0 && downloaded != total_size {
                                                let _ = tokio::fs::remove_file(&filepath).await;
                                                sender
                                                    .send(Action::UpdateDownload(
                                                        None,
                                                        Some("Incomplete download".to_string()),
                                                    ))
                                                    .ok();
                                                tokio::time::sleep(std::time::Duration::from_secs(
                                                    3,
                                                ))
                                                .await;
                                                sender
                                                    .send(Action::UpdateDownload(None, None))
                                                    .ok();
                                                return;
                                            }
                                            let _ = buf_writer.flush().await;
                                            sender
                                                .send(Action::UpdateDownload(
                                                    Some(100.0),
                                                    Some("Completed!".to_string()),
                                                ))
                                                .ok();
                                            tokio::time::sleep(std::time::Duration::from_secs(3))
                                                .await;
                                            sender.send(Action::UpdateDownload(None, None)).ok();
                                        } else {
                                            sender
                                                .send(Action::UpdateDownload(
                                                    None,
                                                    Some("Failed to create file".to_string()),
                                                ))
                                                .ok();
                                            tokio::time::sleep(std::time::Duration::from_secs(3))
                                                .await;
                                            sender.send(Action::UpdateDownload(None, None)).ok();
                                        }
                                    }
                                    Err(e) => {
                                        sender
                                            .send(Action::UpdateDownload(
                                                None,
                                                Some(format!("Network Error: {}", e)),
                                            ))
                                            .ok();
                                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                        sender.send(Action::UpdateDownload(None, None)).ok();
                                    }
                                }
                            }
                        });
                    } else if self.state.is_fetching_streams {
                        self.state.toast_message =
                            Some("Waiting for streams to load...".to_string());
                        self.state.toast_timer = 60;
                        self.state.is_waiting_for_download_stream = true;
                    } else {
                        self.state.toast_message =
                            Some("No streams available to download.".to_string());
                        self.state.toast_timer = 60;
                        if self.state.download_queue_total > 0 {
                            self.action_sender.send(Action::ProcessDownloadQueue).ok();
                        }
                    }
                }
            }

            Action::PromptDownloadEpisode => {
                self.state.show_episode_download_confirm = true;
                self.state.episode_download_confirm_yes_selected = true;
            }

            Action::ConfirmDownloadEpisode => {
                self.state.show_episode_download_confirm = false;

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
                    self.state.toast_message = Some(format!(
                        "{} Fetching subtitles...",
                        if self.state.basic_terminal {
                            "[OK]"
                        } else {
                            "✓"
                        }
                    ));
                    self.state.toast_timer = 40;
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
                self.state.season_download_confirm_yes_selected = true;
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

                    self.state.toast_message = Some(format!(
                        "Preparing S{:02}E{:02} ({}/{})",
                        season, episode, num, total
                    ));
                    self.state.toast_timer = 60;

                    let subject_id = self
                        .state
                        .selected_details
                        .as_ref()
                        .and_then(|d| d.get("id").or_else(|| d.get("idStr")))
                        .and_then(|v| {
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        })
                        .unwrap_or_default();

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
                    self.state.toast_message = Some(format!(
                        "Season download complete! ({} files)",
                        self.state.download_queue_total
                    ));
                    self.state.toast_timer = 100;
                    self.state.download_queue_total = 0;
                }
            }

            Action::DetailsSuccess(id, payload) => {
                if self.state.active_screen != Screen::Details {
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
                    self.state.status_message = "Please select a language dubbing.".to_string();
                    self.state.status_timer = 150;
                } else {
                    if stype == 2 && !self.state.available_seasons.is_empty() {
                        self.state.details_pane = crate::tui::state::DetailsPane::Seasons;
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
            Action::DetailsFailure(err) => {
                self.state.is_loading = false;
                self.state.status_message = format!("Details fetch failed: {}", err);
                self.state.status_timer = 150;
            }
            Action::SetStatus(msg) => {
                self.state.status_message = msg;
                self.state.status_timer = 150;
            }
            Action::InitStreamPool(subject_id) => {
                let pool = crate::tui::state::SubjectStreamPool {
                    available_resolutions: vec![],
                    ..Default::default()
                };
                self.state.stream_pool.insert(subject_id.clone(), pool);
                self.trigger_episode_fetch();

                let client = self.client.clone();
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    if let Ok(resolutions) = client.fetch_collection_resolutions(&subject_id).await
                    {
                        if !resolutions.is_empty() {
                            sender
                                .send(Action::StreamPoolInitialized(subject_id, resolutions))
                                .ok();
                        }
                    }
                });
            }
            Action::StreamPoolInitialized(subject_id, resolutions) => {
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

                self.action_sender
                    .send(Action::FetchEpisodeStreams {
                        subject_id,
                        season: se,
                        episode: ep,
                        force_refresh: true,
                    })
                    .ok();
            }
            Action::FetchEpisodeStreams {
                subject_id,
                season,
                episode,
                force_refresh,
            } => {
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.selected_resources = None;

                if let Some(pool) = self.state.stream_pool.get_mut(&subject_id) {
                    if !force_refresh {
                        if let Some(cached) = pool.episode_index.get(&(season, episode)) {
                            self.action_sender
                                .send(Action::EpisodeStreamsReady(
                                    subject_id.clone(),
                                    season,
                                    episode,
                                    serde_json::Value::Array(cached.clone()),
                                ))
                                .ok();
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
                                crate::cache::get_stream_cache(&id_for_cache, season, episode)
                            })
                            .await
                            {
                                sender
                                    .send(Action::SetStatus("Loaded from cache.".to_string()))
                                    .ok();
                                sender
                                    .send(Action::EpisodeStreamsReady(
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

                        if is_movie {
                            let mut page = 1usize;
                            loop {
                                if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                                    break;
                                }
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(15),
                                    client.fetch_resource_page(&id_clone, 0, page),
                                )
                                .await
                                {
                                    Ok(Ok((items, pager))) => {
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
                                    _ => break,
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
                                            return (Vec::new(), serde_json::json!({}));
                                        }
                                        match tokio::time::timeout(
                                            std::time::Duration::from_secs(15),
                                            c.fetch_resource_page(&id, res, page),
                                        )
                                        .await
                                        {
                                            Ok(Ok((items, pager))) => (items, pager),
                                            _ => (Vec::new(), serde_json::json!({})),
                                        }
                                    }));
                                }

                                let mut page_empty = true;
                                let mut has_more = false;
                                for handle in page_handles {
                                    if let Ok((items, pager)) = handle.await {
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

                        if !target_ok || all_items.is_empty() {
                            sender
                                .send(Action::EpisodeStreamsFailed(
                                    id_clone,
                                    season,
                                    episode,
                                    "Rate Limit".into(),
                                ))
                                .ok();
                        } else {
                            sender
                                .send(Action::EpisodeStreamsReady(
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
            Action::EpisodeStreamsReady(subject_id, target_se, target_ep, payload) => {
                if Some(&subject_id) != self.state.active_subject_id.as_ref() {
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
                            crate::cache::set_stream_cache(
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
                self.state
                    .resource_list_state
                    .select(if count > 0 { Some(0) } else { None });
                self.state.status_message = format!("{} streams available.", count);
                self.state.status_timer = 150;

                if self.state.is_waiting_for_download_stream {
                    self.state.is_waiting_for_download_stream = false;

                    let is_season_queue = self.state.download_queue_total > 0;
                    if is_season_queue {
                        let subject_id = self
                            .state
                            .selected_details
                            .as_ref()
                            .and_then(|d| d.get("id"))
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                            .to_string();
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
            Action::EpisodeStreamsFailed(subject_id, target_se, target_ep, err) => {
                if Some(&subject_id) != self.state.active_subject_id.as_ref() {
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
                self.state.status_message = format!("Error: {}", err);
                self.state.status_timer = 150;
            }
            Action::UpdateDownload(prog, stat) => {
                if self.state.download_progress != prog || self.state.download_status != stat {
                    self.state.download_progress = prog;
                    self.state.download_status = stat;
                    self.state.dirty = true;
                }

                if prog.is_none() && !self.state.download_queue.is_empty() {
                    self.action_sender.send(Action::ProcessDownloadQueue).ok();
                }
            }
            Action::CancelDownload => {
                self.state
                    .cancel_download
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                self.state.download_status = Some("Cancelling...".to_string());
                self.state.toast_message = Some(format!(
                    "{} Cancelling download...",
                    if self.state.basic_terminal {
                        "[X]"
                    } else {
                        "✗"
                    }
                ));
                self.state.toast_timer = 40;
            }

            Action::PlayersDetected(players) => {
                self.state.available_players = players;
            }
            Action::ShowPlayerPicker(link, subtitle) => {
                if self.state.available_players.is_empty() {
                    self.state.toast_message = Some(format!(
                        "{} No media player found. Install mpv, IINA, or VLC.",
                        if self.state.basic_terminal {
                            "[X]"
                        } else {
                            "✗"
                        }
                    ));
                    self.state.toast_timer = 150;
                    return None;
                }
                self.state.player_picker_popup = true;
                self.state.player_picker_link = Some(link);
                self.state.player_picker_subtitle = subtitle;
                self.state.player_picker_state.select(Some(0));
                self.state.subtitle_popup = false;
            }
            Action::LaunchPlayer(kind, link, sub) => {
                self.state.player_picker_popup = false;
                tokio::spawn(async move {
                    let mut local_sub = sub.clone();
                    if kind == crate::tui::state::PlayerKind::Vlc
                        || kind == crate::tui::state::PlayerKind::Iina
                    {
                        if let Some(s_url) = sub {
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
                                    }
                                }
                            }
                        }
                    }

                    let mut cmd = match kind {
                        crate::tui::state::PlayerKind::Mpv => {
                            let mut c = if std::path::Path::new("C:\\Program Files\\mpv\\mpv.exe")
                                .exists()
                            {
                                std::process::Command::new("C:\\Program Files\\mpv\\mpv.exe")
                            } else if std::path::Path::new(
                                "/Applications/mpv.app/Contents/MacOS/mpv",
                            )
                            .exists()
                            {
                                std::process::Command::new(
                                    "/Applications/mpv.app/Contents/MacOS/mpv",
                                )
                            } else {
                                std::process::Command::new("mpv")
                            };
                            c.arg(&link);
                            if let Some(s) = local_sub {
                                c.arg(format!("--sub-file={}", s));
                            }
                            c
                        }
                        crate::tui::state::PlayerKind::Iina => {
                            #[cfg(target_os = "macos")]
                            {
                                let mut c = std::process::Command::new("open");
                                c.arg("-a").arg("IINA");
                                if let Some(s) = &local_sub {
                                    c.arg("--args")
                                        .arg(&link)
                                        .arg(format!("--mpv-sub-files={}", s));
                                } else {
                                    c.arg(&link);
                                }
                                c
                            }
                            #[cfg(not(target_os = "macos"))]
                            {
                                let mut c = std::process::Command::new("mpv");
                                c.arg(&link);
                                if let Some(s) = local_sub {
                                    c.arg(format!("--sub-file={}", s));
                                }
                                c
                            }
                        }
                        crate::tui::state::PlayerKind::Vlc => {
                            let mut c = if std::path::Path::new("/Applications/VLC.app").exists() {
                                std::process::Command::new(
                                    "/Applications/VLC.app/Contents/MacOS/VLC",
                                )
                            } else if std::path::Path::new(
                                "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe",
                            )
                            .exists()
                            {
                                std::process::Command::new(
                                    "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe",
                                )
                            } else if std::path::Path::new(
                                "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe",
                            )
                            .exists()
                            {
                                std::process::Command::new(
                                    "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe",
                                )
                            } else {
                                std::process::Command::new("vlc")
                            };
                            c.arg(&link);
                            if let Some(s) = local_sub {
                                c.arg(format!("--sub-file={}", s));
                            }
                            c
                        }
                    };
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());

                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        cmd.process_group(0);
                    }

                    let _ = cmd.spawn();
                });
            }
            Action::CheckForUpdates => {
                let update_sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let start = std::time::Instant::now();

                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .unwrap_or_default();

                    let result = match client
                        .get("https://github.com/mesamirh/MovieBox-Tui/releases/latest")
                        .send()
                        .await
                    {
                        Ok(res) => {
                            let url = res.url().as_str();
                            if let Some(tag) = url.split("/tag/").last() {
                                let version = tag.trim_start_matches('v');
                                let current = env!("CARGO_PKG_VERSION");
                                if self_update::version::bump_is_greater(current, version)
                                    .unwrap_or(false)
                                {
                                    Some(version.to_string())
                                } else {
                                    Some("none".to_string())
                                }
                            } else {
                                Some("error".to_string())
                            }
                        }
                        Err(_) => Some("error".to_string()),
                    };

                    let elapsed = start.elapsed();
                    if elapsed.as_millis() < 1500 {
                        tokio::time::sleep(std::time::Duration::from_millis(1500) - elapsed).await;
                    }

                    if let Some(res) = result {
                        update_sender.send(Action::UpdateAvailable(res)).ok();
                    }
                });
            }
            Action::UpdateAvailable(version) => {
                if version == "none" || version == "error" {
                    if self.state.active_screen == Screen::Startup {
                        self.state.active_screen = Screen::Home;
                        if version == "none" {
                            self.state.toast_message =
                                Some("You are on the latest version!".to_string());
                        } else {
                            self.state.toast_message =
                                Some("Failed to check for updates.".to_string());
                        }
                        self.state.toast_timer = 40;
                    }
                } else {
                    self.state.update_available = Some(version);
                    if self.state.active_screen == Screen::Startup {
                        self.action_sender.send(Action::StartUpdate).ok();
                    }
                }
            }
            Action::StartUpdate => {
                let version = self.state.update_available.clone().unwrap_or_default();
                self.state.updater_progress = Some(0.0);
                self.state.updater_status = Some("Starting update...".to_string());
                let update_sender = self.action_sender.clone();

                tokio::task::spawn_blocking(move || {
                    let os = std::env::consts::OS;
                    let asset_name = match os {
                        "macos" => "MovieBox_macOS_Universal.tar.gz",
                        "windows" => "MovieBox_Windows_x64.zip",
                        "linux" => "MovieBox_Linux_x64.tar.gz",
                        _ => {
                            return update_sender
                                .send(Action::UpdateFailure("Unsupported OS".into()))
                                .unwrap_or(());
                        }
                    };

                    let download_url = format!(
                        "https://github.com/mesamirh/MovieBox-Tui/releases/download/v{}/{}",
                        version, asset_name
                    );

                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let tmp_dir = std::env::temp_dir().join(format!("moviebox-update-{}", ts));
                    let _ = std::fs::create_dir_all(&tmp_dir);

                    let archive_path = tmp_dir.join(asset_name);
                    if let Ok(mut archive_file) = std::fs::File::create(&archive_path) {
                        if self_update::Download::from_url(&download_url)
                            .show_progress(false)
                            .download_to(&mut archive_file)
                            .is_ok()
                        {
                            let bin_name = if os == "windows" {
                                "MovieBox.exe"
                            } else {
                                "moviebox"
                            };
                            let archive_kind = if os == "windows" {
                                self_update::ArchiveKind::Zip
                            } else {
                                self_update::ArchiveKind::Tar(Some(self_update::Compression::Gz))
                            };

                            if self_update::Extract::from_source(&archive_path)
                                .archive(archive_kind)
                                .extract_file(&tmp_dir, bin_name)
                                .is_ok()
                            {
                                if let Ok(current_exe) = std::env::current_exe() {
                                    let tmp_old = current_exe.with_extension("old");
                                    let _ = std::fs::rename(&current_exe, &tmp_old);
                                    let new_bin = tmp_dir.join(bin_name);
                                    if std::fs::copy(&new_bin, &current_exe).is_ok() {
                                        let _ = std::fs::remove_file(&new_bin);
                                        let _ = std::fs::remove_dir_all(&tmp_dir);
                                        update_sender.send(Action::UpdateSuccess).ok();
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    update_sender
                        .send(Action::UpdateFailure("Failed".into()))
                        .ok();
                });

                let progress_sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let mut p = 0.0;
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        p += 0.01;
                        if p > 0.98 {
                            p = 0.98;
                        }
                        progress_sender.send(Action::UpdateProgress(p)).ok();
                    }
                });
            }
            Action::UpdateProgress(p) => {
                if !self.state.updater_done {
                    self.state.updater_progress = Some(p);
                }
            }
            Action::UpdateSuccess => {
                self.state.updater_done = true;
                self.state.updater_progress = Some(1.0);

                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                return Some(());
            }
            Action::UpdateFailure(_err) => {
                self.state.active_screen = Screen::Home;
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
        }

        if self.state.show_help {
            super::screens::help::draw(frame, main_area, &self.state, &self.theme);
        }

        if let Some(prog) = self.state.download_progress {
            if let Some(dl_area) = download_area {
                use ratatui::widgets::{Block, Borders, Clear, Gauge};

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

                frame.render_widget(Clear, dl_area);
                frame.render_widget(gauge, dl_area);
            }
        }

        if let Some(msg) = &self.state.toast_message {
            use ratatui::layout::{Constraint, Direction, Layout};

            use ratatui::widgets::Paragraph;

            let inner_area = area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 2,
            });
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner_area);

            let toast_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(msg.chars().count() as u16 + 2),
                ])
                .split(chunks[0])[1];

            let color = if self.state.toast_timer < 10 {
                self.theme.muted
            } else {
                self.theme.success
            };

            let p = Paragraph::new(msg.clone())
                .style(color.add_modifier(ratatui::style::Modifier::BOLD));
            frame.render_widget(p, toast_area);
        }
    }
}
