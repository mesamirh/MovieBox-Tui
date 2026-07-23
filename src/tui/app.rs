use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::providers::moviebox::client::MovieBoxClient;
use crate::tui::{
    action::Action,
    event::EventHandler,
    state::{AppState, InputMode, Screen, SearchResult},
    theme::Theme,
};
use update_informer::Check;

pub fn clean_moviebox_title(raw_title: &str) -> String {
    let mut clean = raw_title.to_string();
    if let Some(start) = clean.find(" [") {
        clean = clean[..start].to_string();
    }
    if let Some(start) = clean.find(" (") {
        let lower = clean.to_lowercase();
        let inside = &lower[start..];
        if inside.contains("dub") || inside.contains("hindi") {
            clean = clean[..start].to_string();
        }
    }

    if let Some(s_idx) = clean.rfind(" S") {
        let suffix = &clean[s_idx + 2..];
        let is_season = suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == 'S');
        if is_season && suffix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            clean = clean[..s_idx].trim_end().to_string();
        }
    }
    clean
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
        Self {
            state: AppState::default(),
            theme: Theme::new(),
            client: MovieBoxClient::new(),
            action_sender,
            action_receiver,
        }
    }

    fn trigger_episode_fetch(&mut self) {
        if let Some(id) = &self.state.active_subject_id {
            let se_idx = self.state.season_list_state.selected().unwrap_or(0);
            let ep_idx = self.state.episode_list_state.selected().unwrap_or(0);

            if let Some(season) = self.state.available_seasons.get(se_idx) {
                let se = season.get("se").and_then(|s| s.as_i64()).unwrap_or(1) as usize;
                let ep = ep_idx + 1;
                self.state.selected_season = se;
                self.state.selected_episode = ep;
                self.state.resource_list_state.select(None);

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

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        if self.state.image_picker.is_none() && self.state.image_supported {
            match ratatui_image::picker::Picker::from_query_stdio() {
                Ok(picker) => {
                    if matches!(picker.protocol_type(), ratatui_image::picker::ProtocolType::Halfblocks) {
                        self.state.image_supported = false;
                    } else {
                        let cell_h = picker.font_size().height;
                        if cell_h > 0 {
                            self.state.poster_rows = (96_u16.div_ceil(cell_h)).max(3);
                        }
                        self.state.image_picker = Some(picker);
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    while crossterm::event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                        let _ = crossterm::event::read();
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

        let update_sender = self.action_sender.clone();
        tokio::task::spawn_blocking(move || {
            let pkg_name = env!("CARGO_PKG_NAME");
            let current_version = env!("CARGO_PKG_VERSION");
            let informer =
                update_informer::new(update_informer::registry::Crates, pkg_name, current_version);
            if let Ok(Some(version)) = informer.check_version() {
                update_sender
                    .send(Action::UpdateAvailable(version.to_string()))
                    .ok();
            }
        });

        loop {
            terminal.draw(|frame| self.draw(frame))?;

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
        match action {
            Action::Tick => {
                self.state.tick_count = self.state.tick_count.wrapping_add(1);
                if self.state.toast_timer > 0 {
                    self.state.toast_timer -= 1;
                    if self.state.toast_timer == 0 {
                        self.state.toast_message = None;
                    }
                }
                if self.state.status_timer > 0 {
                    self.state.status_timer -= 1;
                    if self.state.status_timer == 0 {
                        self.state.status_message.clear();
                    }
                }

                let current_query = self.state.search_query.trim().to_string();
                if current_query != self.state.last_suggest_query
                    && self.state.last_search_edit.elapsed()
                        >= std::time::Duration::from_millis(250)
                {
                    self.state.last_suggest_query = current_query.clone();
                    if !current_query.is_empty() {
                        self.action_sender.send(Action::Suggest(current_query)).ok();
                    } else {
                        self.state.search_suggestions.clear();
                    }
                }

                if self.state.pending_episode_fetch.is_some()
                    && self.state.last_episode_nav.elapsed()
                        >= std::time::Duration::from_millis(300)
                {
                    if let Some((subject_id, se, ep)) = self.state.pending_episode_fetch.take() {
                        if let Some(cached) = self
                            .state
                            .stream_cache
                            .get(&(subject_id.clone(), se, ep))
                            .cloned()
                        {
                            let count = cached.len();
                            let mut result = serde_json::Map::new();
                            result.insert("list".to_string(), serde_json::Value::Array(cached));
                            self.state.selected_resources = Some(serde_json::Value::Object(result));
                            self.state.is_loading = false;
                            self.state.resource_list_state.select(if count > 0 {
                                Some(0)
                            } else {
                                None
                            });
                            self.state.status_message =
                                format!("Resolved {} direct stream sources (cached).", count);
                            self.state.status_timer = 150;
                        } else {
                            self.action_sender
                                .send(Action::FetchResources {
                                    subject_id,
                                    season: se,
                                    episode: ep,
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
                if self.state.image_picker.is_some() {
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    while crossterm::event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                        let _ = crossterm::event::read();
                    }
                }
            }
            Action::Resize(_w, _h) => {
                self.state.poster_protocol = None;
                self.state.search_poster_protocols.clear();
                if self.state.image_picker.is_some() {
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    while crossterm::event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                        let _ = crossterm::event::read();
                    }
                }
            }
            Action::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};

                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char('c') = key.code {
                        self.action_sender.send(Action::Quit).ok();
                        return Some(());
                    }
                }

                if let KeyCode::Char('x') | KeyCode::Char('X') = key.code
                    && self.state.download_progress.is_some()
                {
                    self.action_sender.send(Action::CancelDownload).ok();
                    return Some(());
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
                                self.state.search_results.clear();
                                self.state.search_list_state.select(None);
                                self.state.last_search_edit = std::time::Instant::now();
                                let action = if selected_suggestion {
                                    Action::SelectSuggestion { query }
                                } else {
                                    Action::Search { query }
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
                            self.state.search_query =
                                self.state.search_suggestions[next_idx].clone();
                        }
                        KeyCode::Down if !self.state.search_suggestions.is_empty() => {
                            let max_idx = self.state.search_suggestions.len() - 1;
                            let next_idx = match self.state.suggest_index {
                                None => 0,
                                Some(i) if i == max_idx => 0,
                                Some(i) => i + 1,
                            };
                            self.state.suggest_index = Some(next_idx);
                            self.state.search_query =
                                self.state.search_suggestions[next_idx].clone();
                        }
                        _ => {}
                    },
                    InputMode::Normal => match self.state.active_screen {
                        Screen::Home => match key.code {
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
                            }
                            _ => {}
                        },
                        Screen::Details => match key.code {
                            KeyCode::Char('q') => {
                                self.action_sender.send(Action::Quit).ok();
                            }
                            KeyCode::Char('y') | KeyCode::Char('c') => {
                                self.action_sender.send(Action::CopyLink).ok();
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
                                    self.action_sender.send(Action::DownloadStream).ok();
                                }
                            }
                            KeyCode::Char('R') => {
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
                                    self.action_sender
                                        .send(Action::FetchResources {
                                            subject_id: id,
                                            season: se,
                                            episode: ep,
                                        })
                                        .ok();
                                }
                            }
                            KeyCode::Char('?') => {
                                self.action_sender.send(Action::ToggleHelp).ok();
                            }
                            KeyCode::Char('b') | KeyCode::Esc => {
                                self.action_sender.send(Action::GoBack).ok();
                            }

                            KeyCode::Up | KeyCode::Char('k') => {
                                self.action_sender.send(Action::MoveUp).ok();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.action_sender.send(Action::MoveDown).ok();
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                self.action_sender.send(Action::MoveLeft).ok();
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                self.action_sender.send(Action::MoveRight).ok();
                            }
                            KeyCode::Enter => {
                                let open_with = key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::SHIFT);
                                if self.state.subtitle_popup || self.state.player_picker_popup {
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
                                                .send(Action::FetchResources {
                                                    subject_id: self
                                                        .state
                                                        .active_subject_id
                                                        .clone()
                                                        .unwrap_or("".to_string()),
                                                    season: self.state.selected_season,
                                                    episode: self.state.selected_episode,
                                                })
                                                .ok();
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
            Action::GoBack => {
                if self.state.player_picker_popup {
                    self.state.player_picker_popup = false;
                    self.state.player_picker_link = None;
                    self.state.player_picker_subtitle = None;
                    return None;
                }
                if self.state.subtitle_popup {
                    self.state.subtitle_popup = false;
                    self.state.pending_play_link = None;
                    return None;
                }
                if self.state.show_help {
                    self.state.show_help = false;
                    return None;
                }
                match self.state.active_screen {
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
                        self.state.active_screen = Screen::Home;
                        self.state.is_loading = false;
                        self.state.language_chosen = false;
                        self.state.status_message =
                            "Select a movie/series and press Enter".to_string();
                        self.state.status_timer = 150;
                    }
                }
            }
            Action::SelectLanguage(idx) => {
                if let Some(details) = &self.state.selected_details
                    && let Some(dubs) = details.get("dubs").and_then(|d| d.as_array())
                    && let Some(dub) = dubs.get(idx)
                    && let Some(id) = dub.get("subjectId").and_then(|i| i.as_str())
                {
                    let next_id = id.to_string();
                    self.state.selected_details = None;
                    self.state.selected_resources = None;
                    self.state.resource_list_state.select(None);
                    self.state.language_chosen = true;
                    self.state.status_message = "Switching language...".to_string();
                    self.state.status_timer = 150;
                    self.action_sender.send(Action::FetchDetails(next_id)).ok();
                }
            }
            Action::Suggest(query) => {
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
                self.action_sender.send(Action::Search { query }).ok();
            }
            Action::Search { query } => {
                let lower_query = query.trim().to_lowercase();
                let tab_id = match lower_query.as_str() {
                    "/home" | "/discover" => Some("0"),
                    "/movies" => Some("2"),
                    "/shows" => Some("5"),
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
                self.state.search_results.clear();
                self.state.search_list_state.select(Some(0));
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state.status_message = format!("Searching for '{}'...", query);
                self.state.status_timer = 150;

                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let query_clone = query.clone();
                tokio::spawn(async move {
                    match client.search(&query_clone, 1).await {
                        Ok(res) => {
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
                tokio::spawn(async move {
                    match client.get_homepage(&tab_id, page).await {
                        Ok(res) => {
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

                        let is_duplicate = self.state.search_results.iter().any(|r| {
                            r.title == clean_title
                                && r.release_year == release_year
                                && r.stype == stype
                        });

                        if !id.is_empty() && !is_duplicate {
                            self.state.search_results.push(SearchResult {
                                id,
                                title: clean_title,
                                stype,
                                release_year,
                                cover_url,
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
                                            if let Ok(img) = image::load_from_memory(&bytes) {
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

                    if !id.is_empty() {
                        self.state.search_results.push(SearchResult {
                            id,
                            title: clean_title,
                            stype,
                            release_year,
                            cover_url,
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
                                            if let Ok(img) = image::load_from_memory(&bytes) {
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
                } else if self.state.subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current > 0 {
                        self.state.subtitle_list_state.select(Some(current - 1));
                    }
                    return None;
                }
                match self.state.active_screen {
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
                } else if self.state.subtitle_popup {
                    let current = self.state.subtitle_list_state.selected().unwrap_or(0);
                    if current + 1 < self.state.subtitle_list.len() {
                        self.state.subtitle_list_state.select(Some(current + 1));
                    }
                    return None;
                }
                match self.state.active_screen {
                    Screen::Home => {
                        let current = self.state.search_list_state.selected().unwrap_or(0);
                        if current + 1 < self.state.search_results.len() {
                            self.state.search_list_state.select(Some(current + 1));
                            if let Some(res) = self.state.search_results.get(current + 1) {
                                self.action_sender
                                    .send(Action::FetchPreview(res.id.clone()))
                                    .ok();
                            }
                        } else if !self.state.is_loading && !self.state.search_results.is_empty() {
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
                            if let Some(season) = self
                                .state
                                .available_seasons
                                .get(self.state.season_list_state.selected().unwrap_or(0))
                            {
                                let max_ep =
                                    season.get("maxEp").and_then(|m| m.as_i64()).unwrap_or(1)
                                        as usize;
                                if current + 1 < max_ep {
                                    self.state.episode_list_state.select(Some(current + 1));
                                    self.trigger_episode_fetch();
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
                }
                if self.state.active_screen == Screen::Home {
                    let idx_opt = self.state.search_list_state.selected();
                    let item_opt =
                        idx_opt.and_then(|idx| self.state.search_results.get(idx).cloned());
                    if let Some(item) = item_opt {
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
                        sender.send(Action::FetchDetails(item.id)).ok();
                    }
                }
            }
            Action::FetchDetails(id) => {
                self.state.poster_protocol = None;
                self.state.is_loading = true;
                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let id_clone = id.clone();
                tokio::spawn(async move {
                    match client.get_details(&id_clone).await {
                        Ok(details) => {
                            sender.send(Action::DetailsSuccess(id_clone, details)).ok();
                        }
                        Err(e) => {
                            sender.send(Action::DetailsFailure(format!("{:?}", e))).ok();
                        }
                    }
                });
            }
            Action::FetchPreview(id) => {
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
                            && let Ok(bytes) = resp.bytes().await
                            && let Ok(img) = image::load_from_memory(&bytes)
                        {
                            let _ = action_tx
                                .send(Action::PosterSuccess(id_clone, std::sync::Arc::new(img)));
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

            Action::CopyLink => {
                if self.state.active_screen == Screen::Details
                    && let Some(link) = self.get_selected_link()
                {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(link.clone());
                        self.state.toast_message = Some(format!("{} Copied stream link!", if self.state.basic_terminal { "[OK]" } else { "✓" }));
                        self.state.toast_timer = 30;
                    } else {
                        self.state.status_message = format!("Link: {}", link);
                        self.state.status_timer = 150;
                    }
                }
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
                        self.state.toast_message = Some(format!("{} Fetching subtitles...", if self.state.basic_terminal { "[OK]" } else { "✓" }));
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
            Action::LaunchMpv(link, subtitle_url) => {
                use std::process::{Command, Stdio};
                self.state.toast_message = Some(format!("{} Launching MPV...", if self.state.basic_terminal { "[OK]" } else { "✓" }));
                self.state.toast_timer = 40;

                let mut cmd = Command::new("mpv");
                cmd.arg(&link).stdout(Stdio::null()).stderr(Stdio::null());

                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    cmd.process_group(0);
                }

                if let Some(sub) = subtitle_url {
                    if !sub.is_empty() {
                        cmd.arg(format!("--sub-file={}", sub));
                    }
                }

                if cmd.spawn().is_ok() {
                } else {
                    self.state.toast_message =
                        Some(format!("{} Error: mpv player not found in PATH", if self.state.basic_terminal { "[X]" } else { "✗" }));
                    self.state.toast_timer = 60;
                }
            }
            Action::DownloadStream => {
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
                    let ext = "mp4";
                    let filename = format!(
                        "{}_{}.{}",
                        title.replace(" ", "_").replace("/", "_"),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        ext
                    );

                    let filepath = dirs::download_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join(&filename);

                    if let Some(link) = link_opt {
                        self.state.toast_message =
                            Some(format!("{} Starting native download...", if self.state.basic_terminal { "[OK]" } else { "✓" }));
                        self.state.toast_timer = 40;
                        self.state.download_status = Some("Connecting...".to_string());
                        self.state.download_progress = Some(0.0);
                        self.state
                            .cancel_download
                            .store(false, std::sync::atomic::Ordering::SeqCst);

                        let cancel_token = self.state.cancel_download.clone();
                        let sender = self.action_sender.clone();
                        let client = self.client.http_client().clone();
                        tokio::spawn(async move {
                            let head_res = client.head(&link).send().await;
                            let (total_size, supports_ranges) = match head_res {
                                Ok(r) => {
                                    let size = r.content_length().unwrap_or(0);
                                    let ranges = r
                                        .headers()
                                        .get(reqwest::header::ACCEPT_RANGES)
                                        .and_then(|v| v.to_str().ok())
                                        .unwrap_or("")
                                        == "bytes";
                                    (size, ranges)
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

                            if total_size > 1024 * 1024 && supports_ranges {
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
                                        while let Ok(Some(chunk)) = res.chunk().await {
                                            if c_token.load(std::sync::atomic::Ordering::Relaxed) {
                                                return Err(());
                                            }

                                            let chunk_to_write = if part_downloaded
                                                + chunk.len() as u64
                                                > expected_size
                                            {
                                                &chunk[..(expected_size - part_downloaded) as usize]
                                            } else {
                                                &chunk[..]
                                            };

                                            if file.write_all(chunk_to_write).await.is_err() {
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
                                    for tmp in &temp_files {
                                        let _ = tokio::fs::remove_file(tmp).await;
                                    }
                                    sender
                                        .send(Action::UpdateDownload(
                                            None,
                                            Some("Download Cancelled".to_string()),
                                        ))
                                        .ok();
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
                                            while let Ok(Some(chunk)) = response.chunk().await {
                                                if cancel_token
                                                    .load(std::sync::atomic::Ordering::Relaxed)
                                                {
                                                    let _ = tokio::fs::remove_file(&filepath).await;
                                                    sender
                                                        .send(Action::UpdateDownload(
                                                            None,
                                                            Some("Download Cancelled".to_string()),
                                                        ))
                                                        .ok();
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_secs(2),
                                                    )
                                                    .await;
                                                    sender
                                                        .send(Action::UpdateDownload(None, None))
                                                        .ok();
                                                    return;
                                                }
                                                if buf_writer.write_all(&chunk).await.is_err() {
                                                    sender
                                                        .send(Action::UpdateDownload(
                                                            None,
                                                            Some("File write error!".to_string()),
                                                        ))
                                                        .ok();
                                                    return;
                                                }
                                                downloaded += chunk.len() as u64;

                                                let now = std::time::Instant::now();
                                                if now.duration_since(last_ui_update).as_millis()
                                                    > 200
                                                {
                                                    last_ui_update = now;
                                                    let progress = if total_size > 0 {
                                                        (downloaded as f64 / total_size as f64)
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
                                                    let speed_mbps = speed_bps / 1024.0 / 1024.0;

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
                    }
                }
            }

            Action::DetailsSuccess(id, payload) => {
                self.state.active_subject_id = Some(id.clone());
                self.state.selected_details = Some(payload.clone());

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
                                && let Ok(bytes) = resp.bytes().await
                                && let Ok(img) = image::load_from_memory(&bytes)
                            {
                                let _ = action_tx.send(Action::PosterSuccess(
                                    id_clone,
                                    std::sync::Arc::new(img),
                                ));
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
                        "allEp": format!("1-{}", max_ep)
                    })];
                } else {
                    self.state.available_seasons.clear();
                }

                self.state.season_list_state.select(Some(0));
                self.state.episode_list_state.select(Some(0));

                if let Some(dubs) = payload.get("dubs").and_then(|d| d.as_array()) {
                    let mut current_idx = 0;
                    let current_id = payload
                        .get("id")
                        .and_then(|i| {
                            i.as_i64()
                                .map(|n| n.to_string())
                                .or_else(|| i.as_str().map(|s| s.to_string()))
                        })
                        .unwrap_or_default();
                    for (i, dub) in dubs.iter().enumerate() {
                        let dub_id = dub.get("subjectId").and_then(|i| {
                            i.as_i64()
                                .map(|n| n.to_string())
                                .or_else(|| i.as_str().map(|s| s.to_string()))
                        });
                        if dub_id == Some(current_id.clone()) {
                            current_idx = i;
                        }
                    }
                    self.state.language_list_state.select(Some(current_idx));
                } else {
                    self.state.language_list_state.select(Some(0));
                }

                self.state.selected_season = 1;
                self.state.selected_episode = 1;

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
                        self.state.details_pane = crate::tui::state::DetailsPane::Episodes;
                    } else {
                        self.state.details_pane = crate::tui::state::DetailsPane::Streams;
                    }

                    let (se, ep) = if stype == 2 {
                        (1usize, 1usize)
                    } else {
                        (0usize, 0usize)
                    };
                    let sender = self.action_sender.clone();
                    sender
                        .send(Action::FetchResources {
                            subject_id: id,
                            season: se,
                            episode: ep,
                        })
                        .ok();
                }
            }
            Action::DetailsFailure(err) => {
                self.state.is_loading = false;
                self.state.status_message = format!("Details fetch failed: {}", err);
                self.state.status_timer = 150;
            }
            Action::FetchResources {
                subject_id,
                season,
                episode,
            } => {
                self.state.active_resource_request =
                    self.state.active_resource_request.wrapping_add(1);
                let req_id = self.state.active_resource_request;
                let client = self.client.clone();
                let sender = self.action_sender.clone();
                self.state.is_loading = true;
                self.state.selected_resources = None;
                tokio::spawn(async move {
                    match client.get_all_resources(&subject_id, season, episode).await {
                        Ok(res) => {
                            sender
                                .send(Action::ResourcesSuccess(req_id, season, episode, res))
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::ResourcesFailure(req_id, format!("{:?}", e)))
                                .ok();
                        }
                    }
                });
            }
            Action::ResourcesSuccess(req_id, target_se, target_ep, payload) => {
                if req_id != self.state.active_resource_request {
                    return None;
                }

                let raw_list = payload
                    .get("list")
                    .and_then(|l| l.as_array())
                    .cloned()
                    .unwrap_or_default();

                let mut filtered: Vec<serde_json::Value> = if target_se == 0 && target_ep == 0 {
                    raw_list
                } else {
                    raw_list
                        .into_iter()
                        .filter(|stream| {
                            let s = stream
                                .get("se")
                                .and_then(|v| {
                                    v.as_i64()
                                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                                })
                                .unwrap_or(0) as usize;
                            let e = stream
                                .get("ep")
                                .and_then(|v| {
                                    v.as_i64()
                                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                                })
                                .unwrap_or(0) as usize;
                            s == target_se && e == target_ep
                        })
                        .collect()
                };

                filtered.sort_by(|a, b| {
                    let res_a = a.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                    let res_b = b.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                    res_b.cmp(&res_a)
                });

                let count = filtered.len();

                if let Some(ref subject_id) = self.state.active_subject_id {
                    self.state
                        .stream_cache
                        .put((subject_id.clone(), target_se, target_ep), filtered.clone());
                }

                let mut result = serde_json::Map::new();
                result.insert("list".to_string(), serde_json::Value::Array(filtered));
                self.state.selected_resources = Some(serde_json::Value::Object(result));
                self.state.is_loading = false;

                self.state
                    .resource_list_state
                    .select(if count > 0 { Some(0) } else { None });
                self.state.status_message = format!("Resolved {} direct stream sources.", count);
                self.state.status_timer = 150;
            }
            Action::ResourcesFailure(req_id, err) => {
                if req_id != self.state.active_resource_request {
                    return None;
                }
                self.state.is_loading = false;
                if err.contains("406") || err.to_lowercase().contains("exhausted") {
                    self.state.status_message =
                        "Error: No streams found on server (unreleased or removed).".to_string();
                    self.state.status_timer = 150;
                } else {
                    self.state.status_message = format!("Error: Links resolution failed: {}", err);
                    self.state.status_timer = 150;
                }
            }
            Action::UpdateDownload(prog, stat) => {
                self.state.download_progress = prog;
                self.state.download_status = stat;
            }
            Action::CancelDownload => {
                self.state
                    .cancel_download
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                self.state.download_status = Some("Cancelling...".to_string());
                self.state.toast_message = Some(format!("{} Cancelling download...", if self.state.basic_terminal { "[X]" } else { "✗" }));
                self.state.toast_timer = 40;
            }

            Action::ShowPlayerPicker(link, subtitle) => {
                if self.state.available_players.is_empty() {
                    self.state.toast_message =
                        Some(format!("{} No media player found. Install mpv, IINA, or VLC.", if self.state.basic_terminal { "[X]" } else { "✗" }));
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
                    if kind == crate::tui::state::PlayerKind::Vlc {
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
                                    if std::fs::write(&temp_path, bytes).is_ok() {
                                        local_sub = Some(temp_path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }

                    let mut cmd = match kind {
                        crate::tui::state::PlayerKind::Mpv => {
                            let mut c = std::process::Command::new("mpv");
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
                                c.arg("-a").arg("IINA").arg(&link);
                                c
                            }
                            #[cfg(not(target_os = "macos"))]
                            {
                                let mut c = std::process::Command::new("mpv");
                                c.arg(&link);
                                c
                            }
                        }
                        crate::tui::state::PlayerKind::Vlc => {
                            let mut c = if std::path::Path::new("/Applications/VLC.app").exists() {
                                std::process::Command::new("/Applications/VLC.app/Contents/MacOS/VLC")
                            } else if std::path::Path::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe").exists() {
                                std::process::Command::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe")
                            } else if std::path::Path::new("C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe").exists() {
                                std::process::Command::new("C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe")
                            } else {
                                std::process::Command::new("vlc")
                            };
                            c.arg(&link);
                            if let Some(s) = local_sub {
                                c.arg("--sub-file").arg(s);
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
            Action::UpdateAvailable(version) => {
                self.state.update_available = Some(version);
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

        match self.state.active_screen {
            Screen::Home => {
                super::screens::home::draw(frame, area, &mut self.state, &self.theme);
            }
            Screen::Details => {
                super::screens::details::draw(frame, area, &mut self.state, &self.theme);
            }
        }

        if self.state.show_help {
            super::screens::help::draw(frame, area, &self.state, &self.theme);
        }

        if let Some(prog) = self.state.download_progress {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::style::{Color, Modifier, Style};
            use ratatui::widgets::{Block, Borders, Gauge};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(area);

            let status = self
                .state
                .download_status
                .as_deref()
                .unwrap_or("Downloading...");
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Download: {} [X] Cancel ", status)),
                )
                .gauge_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .ratio((prog / 100.0).clamp(0.0, 1.0));
            frame.render_widget(gauge, chunks[1]);
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
