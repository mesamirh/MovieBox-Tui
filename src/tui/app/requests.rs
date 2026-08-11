use super::{App, network};
use crate::providers::{fourkhdhub::releases_to_moviebox_json, models::ProviderKind};
use crate::tui::{
    action::Action,
    overlay::NotificationKind,
    state::{InputMode, Screen, SearchResult},
};

impl App {
    pub(super) async fn handle_requests(&mut self, action: Action) -> Option<()> {
        match action {
            Action::Suggest(query) => {
                if query.starts_with('/') {
                    let mut commands = vec![
                        "/clear-cache",
                        "/update",
                        "/toggle-update",
                        "/github",
                        "/enable-bdix",
                        "/disable-bdix",
                    ];
                    if self.state.is_tv_mode {
                        commands.push("/list");
                        commands.push("/config");
                    } else {
                        commands.extend(vec![
                            "/browse",
                            "/discover",
                            "/home",
                            "/history",
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

            Action::ToggleBrowseMenu => {
                if self.state.is_tv_mode {
                    return None;
                }
                self.state.browse_menu_open = !self.state.browse_menu_open;
                if self.state.browse_menu_open {
                    if let Some(view) = self.state.browse_view {
                        if let Some(idx) = crate::providers::browse::BrowseView::ALL
                            .iter()
                            .position(|v| *v == view)
                        {
                            self.state.browse_list_state.select(Some(idx));
                        }
                    } else {
                        self.state.browse_list_state.select(Some(0));
                    }
                }
            }

            Action::SelectBrowseView(view) => {
                self.state.browse_menu_open = false;
                self.state.browse_view = Some(view);
                self.state.is_homepage_mode = false;
                self.state.current_page = 1;
                self.state.active_screen = Screen::Home;
                self.state.selected_details = None;
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.search_error = None;
                self.state.search_results.clear();
                self.state.search_list_state.select(Some(0));
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state.search_preview = None;
                self.state.preview_loading = false;
                self.state.search_query.clear();
                self.state
                    .set_status(format!("Loading {}...", view.label()), 150);
                let sort = self.state.browse_sort;
                self.action_sender
                    .send(Action::FetchBrowse { view, sort })
                    .ok();
            }

            Action::ToggleBrowseSort => {
                let view = self.state.browse_view?;
                self.state.browse_sort = self.state.browse_sort.toggle();
                let sort = self.state.browse_sort;
                self.state.is_loading = true;
                self.state.search_error = None;
                self.state.search_results.clear();
                self.state.search_list_state.select(Some(0));
                self.state
                    .set_status(format!("Sorting {}...", view.label()), 150);
                self.action_sender
                    .send(Action::FetchBrowse { view, sort })
                    .ok();
            }

            Action::FetchBrowse { view, sort } => {
                if self.state.is_tv_mode || self.state.active_provider != ProviderKind::MovieBox {
                    self.state.is_loading = false;
                    self.state.browse_view = None;
                    self.state.browse_menu_open = false;
                    self.state.set_status(
                        "Browse is available on the MovieBox provider.".to_string(),
                        180,
                    );
                    return None;
                }
                self.state.active_browse_request = self.state.active_browse_request.wrapping_add(1);
                let request_id = self.state.active_browse_request;
                let client = self.client.clone();
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    match crate::providers::browse::fetch_browse_feed(&client, view, sort).await {
                        Ok(subjects) => {
                            sender
                                .send(Action::BrowseSuccess {
                                    view,
                                    request_id,
                                    payload: serde_json::Value::Array(subjects),
                                })
                                .ok();
                        }
                        Err(error) => {
                            sender.send(Action::BrowseFailure(error)).ok();
                        }
                    }
                });
            }

            Action::BrowseSuccess {
                view,
                request_id,
                payload,
            } => {
                if self.state.browse_view != Some(view)
                    || request_id != self.state.active_browse_request
                    || self.state.is_tv_mode
                {
                    return None;
                }
                self.state.is_loading = false;
                self.state.search_error = None;
                self.state.search_results.clear();

                if let Some(subjects) = payload.as_array() {
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
                        let clean_title =
                            crate::providers::moviebox::clean_moviebox_title(&raw_title);
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
                        let season =
                            item.get("season").and_then(|s| s.as_u64()).unwrap_or(0) as usize;
                        let imdb_rating = crate::providers::browse::subject_rating_text(item);

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

                        self.state.search_results.push(SearchResult {
                            id,
                            title: clean_title,
                            stype,
                            release_year,
                            cover_url,
                            season,
                            episode: 1,
                            provider: ProviderKind::MovieBox,
                            imdb_rating,
                        });
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
                                    if let Some(bytes) =
                                        network::fetch_poster_bytes(&client, &url).await
                                    {
                                        if let Some(img) = network::decode_poster(bytes).await {
                                            tx.send(Action::SearchPosterLoaded(id, Some(img))).ok();
                                        }
                                    }
                                });
                            }
                        }
                    });
                }

                self.prepare_image_refresh();

                self.state
                    .search_list_state
                    .select(if self.state.search_results.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                if let Some(first) = self.state.search_results.first() {
                    self.action_sender
                        .send(Action::FetchPreview(first.id.clone()))
                        .ok();
                }
                let arrow = self.state.browse_sort.arrow();
                self.state.set_status(
                    format!(
                        "{} {} · {} results",
                        view.label(),
                        arrow,
                        self.state.search_results.len()
                    ),
                    150,
                );
            }

            Action::BrowseFailure(error) => {
                log::error!("browse failed: {error}");
                self.state.is_loading = false;
                self.state
                    .set_status(format!("Browse failed: {error}"), 150);
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

                if lower_query == "/history" {
                    self.state.input_mode = InputMode::Normal;
                    self.state.is_loading = false;
                    self.state.is_homepage_mode = false;
                    self.state.active_screen = Screen::Home;
                    self.state.search_results.clear();
                    self.state.search_error = None;
                    self.state.search_posters.clear();
                    self.state.search_poster_protocols.clear();

                    let mut recent = self.state.history.recent.clone();
                    if recent.is_empty() {
                        self.state.notify(
                            crate::tui::overlay::NotificationKind::Info,
                            "History",
                            "No watch history found.",
                        );
                    } else {
                        recent.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

                        for item in recent {
                            use crate::providers::models::ProviderKind;
                            let stored = item.provider.to_ascii_lowercase();
                            let provider = ProviderKind::ENABLED
                                .iter()
                                .copied()
                                .find(|provider| {
                                    stored == provider.label().to_ascii_lowercase()
                                        || stored == provider.cache_key()
                                })
                                .unwrap_or(self.state.active_provider);
                            self.state.search_results.push(SearchResult {
                                id: item.subject_id.clone(),
                                title: item.title.clone(),
                                stype: item.stype,
                                release_year: item.release_year.clone(),
                                cover_url: item.cover_url.clone(),
                                season: item.season,
                                episode: item.episode,
                                provider,
                                imdb_rating: None,
                            });
                        }

                        self.state.search_list_state.select(Some(0));

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
                                        if let Some(bytes) =
                                            network::fetch_poster_bytes(&client, &url).await
                                        {
                                            if let Some(img) = network::decode_poster(bytes).await {
                                                tx.send(Action::SearchPosterLoaded(id, Some(img)))
                                                    .ok();
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    }
                    return None;
                }

                if lower_query == "/browse" {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.action_sender.send(Action::ToggleBrowseMenu).ok();
                    return None;
                }

                if lower_query == "/clear-cache" {
                    self.action_sender.send(Action::ClearCache).ok();
                    self.state.search_query.clear();
                    return None;
                }

                if lower_query == "/github" {
                    let _ = open::that("https://github.com/mesamirh/MovieBox-Tui");
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
                if lower_query == "/theme" || lower_query == "/themes" {
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.action_sender.send(Action::ToggleThemePopup).ok();
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

                if lower_query == "/enable-bdix" || lower_query == "/disable-bdix" {
                    let enable_req = lower_query == "/enable-bdix";

                    if self.state.bdix_enabled == enable_req {
                        self.state.search_query.clear();
                        self.state.input_mode = InputMode::Normal;
                        self.state.notify(
                            NotificationKind::Info,
                            "BDIX Providers",
                            if enable_req {
                                "Already Enabled"
                            } else {
                                "Already Disabled"
                            },
                        );
                        return None;
                    }

                    self.state.bdix_enabled = enable_req;
                    self.persist_config();
                    self.state.search_query.clear();
                    self.state.input_mode = InputMode::Normal;
                    self.state.notify(
                        NotificationKind::Info,
                        "BDIX Providers",
                        if self.state.bdix_enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                    );

                    if !self.state.bdix_enabled && self.state.active_provider.is_bdix() {
                        let mut new_provider = crate::providers::models::ProviderKind::MovieBox;
                        for provider in crate::providers::models::ProviderKind::ENABLED.iter() {
                            if !provider.is_bdix() {
                                new_provider = *provider;
                                break;
                            }
                        }
                        self.action_sender
                            .send(Action::SwitchProvider(new_provider))
                            .ok();
                    }

                    return None;
                }

                if self.state.is_tv_mode {
                    if lower_query == "/config" {
                        self.action_sender.send(Action::ShowTvConfig).ok();
                        self.state.search_query.clear();
                        return None;
                    }
                    if lower_query.starts_with('/') && lower_query != "/list" {
                        self.state.set_status(
                            "Switch to streaming mode to use this command".to_string(),
                            150,
                        );
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
                            episode: 1,
                            provider: ProviderKind::MovieBox,
                            imdb_rating: None,
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
                                        if let Some(bytes) =
                                            network::fetch_poster_bytes(&client, &url).await
                                        {
                                            if let Some(img) = network::decode_poster(bytes).await {
                                                tx.send(Action::SearchPosterLoaded(id, Some(img)))
                                                    .ok();
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    }
                    self.state.set_status(
                        if self.state.search_results.is_empty() {
                            format!("No matches for '{}'.", query)
                        } else {
                            format!("Found {} channels.", self.state.search_results.len())
                        },
                        150,
                    );
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
                        self.state.set_status(
                            "4KHDHub has no discover feed; enter a title to search.",
                            180,
                        );
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
                self.state.browse_view = None;
                self.state.browse_menu_open = false;
                self.state.current_page = 1;
                self.state.active_screen = Screen::Home;
                self.state.selected_details = None;
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.search_error = None;
                self.state.search_list_state.select(Some(0));
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state.search_preview = None;
                self.state.preview_loading = false;
                self.state
                    .set_status(format!("Searching for '{}'...", query), 150);

                let query_clone = query.clone();
                let sender = self.action_sender.clone();
                let client = self.client.clone();
                let fourk_client = self.fourk_client.clone();
                let circleftp_client = self.circleftp_client.clone();
                let dhakaflix_client = self.dhakaflix_client.clone();
                let context = self.request_context();
                tokio::spawn(async move {
                    if !force_refresh {
                        let q = query_clone.clone();
                        let p = context.provider;
                        if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                            crate::cache::get_provider_search_cache(p, &q)
                        })
                        .await
                        {
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
                    let result = network::provider_search(
                        &client,
                        &fourk_client,
                        &circleftp_client,
                        &dhakaflix_client,
                        context.provider,
                        &query_clone,
                        1,
                    )
                    .await;
                    match result {
                        Ok(res) => {
                            let q = query_clone.clone();
                            let p = context.provider;
                            let r = res.clone();
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_search_cache(p, &q, &r);
                            });
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    query: query_clone,
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

            Action::FetchHomepage { tab_id, page } => {
                if self.state.is_tv_mode {
                    return None;
                }
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state.is_loading = false;
                    self.state.set_status(
                        "This provider exposes search, not a shared MovieBox homepage.",
                        180,
                    );
                    return None;
                }
                self.state.is_homepage_mode = true;
                self.state.browse_view = None;
                self.state.browse_menu_open = false;
                self.state.current_tab_id = tab_id.clone();
                self.state.current_page = page;
                self.state.active_screen = Screen::Home;
                self.state.selected_details = None;
                self.state.selected_resources = None;
                self.state.is_loading = true;
                self.state.search_error = None;
                if page == 1 {
                    self.state.search_results.clear();
                    self.state.search_list_state.select(Some(0));
                }
                self.state.search_suggestions.clear();
                self.state.suggest_index = None;
                self.state
                    .set_status("Loading discover tab...".to_string(), 150);

                let client = self.client.clone();
                let sender = self.action_sender.clone();
                let force_refresh = false;

                tokio::spawn(async move {
                    if !force_refresh {
                        let t_clone = tab_id.clone();
                        let p_clone = page;
                        if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                            crate::cache::get_homepage_cache(&t_clone, p_clone)
                        })
                        .await
                        {
                            sender
                                .send(Action::HomepageSuccess {
                                    tab_id: tab_id.clone(),
                                    page,
                                    payload: cached,
                                })
                                .ok();
                            return;
                        }
                    }

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
                self.state.search_error = None;
                self.state.is_loading = false;
                if self.state.current_page <= 1 {
                    self.state.search_results.clear();
                }
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

                        let clean_title =
                            crate::providers::moviebox::clean_moviebox_title(&raw_title);

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
                                episode: 1,
                                provider: context.provider,
                                imdb_rating: None,
                            });
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
                                    if let Some(bytes) =
                                        network::fetch_poster_bytes(&client, &url).await
                                    {
                                        if let Some(img) = network::decode_poster(bytes).await {
                                            tx.send(Action::SearchPosterLoaded(id, Some(img))).ok();
                                        }
                                    }
                                });
                            }
                        }
                    });
                }

                self.prepare_image_refresh();

                self.state.set_status(
                    if self.state.search_results.is_empty() {
                        format!(
                            "No matches for '{}' on {}. Press Ctrl+P to try another provider.",
                            query,
                            context.provider.label()
                        )
                    } else {
                        format!(
                            "Found {} results on {}.",
                            self.state.search_results.len(),
                            context.provider.label()
                        )
                    },
                    150,
                );
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
                log::error!(
                    "search failed (provider {}): {err}",
                    context.provider.cache_key()
                );
                self.state.is_loading = false;
                self.state.search_error = Some(err.clone());
                self.state
                    .set_status(format!("Search failed: {}", err), 150);
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
                    self.state.search_error = None;
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
                    let clean_title = crate::providers::moviebox::clean_moviebox_title(&raw_title);
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
                            episode: 1,
                            provider: ProviderKind::MovieBox,
                            imdb_rating: None,
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
                                    if let Some(bytes) =
                                        network::fetch_poster_bytes(&client, &url).await
                                    {
                                        if let Some(img) = network::decode_poster(bytes).await {
                                            tx.send(Action::SearchPosterLoaded(id, Some(img))).ok();
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

                self.prepare_image_refresh();

                self.state.set_status(
                    format!("Found {} discover items", self.state.search_results.len()),
                    150,
                );
            }

            Action::HomepageFailure(err) => {
                log::error!("discover failed: {err}");
                self.state.is_loading = false;
                self.state
                    .set_status(format!("Discover failed: {}", err), 150);
            }

            Action::FetchDetails(id, force_refresh) => {
                self.state.poster_protocol = None;
                self.state.is_loading = true;
                self.state
                    .fetch_cancel
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.state
                    .set_status("Fetching details...".to_string(), 150);
                self.state.stream_pool.clear();
                let client = self.client.clone();
                let fourk_client = self.fourk_client.clone();
                let circleftp_client = self.circleftp_client.clone();
                let dhakaflix_client = self.dhakaflix_client.clone();
                let sender = self.action_sender.clone();
                let id_clone = id.clone();
                let mut target_prov = self.state.active_provider;
                if let Some(res) = self.state.search_results.iter().find(|r| r.id == id) {
                    target_prov = res.provider;
                }
                let mut context = self.request_context();
                context.provider = target_prov;

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
                    let result = network::provider_details(
                        &client,
                        &fourk_client,
                        &circleftp_client,
                        &dhakaflix_client,
                        context.provider,
                        &id_clone,
                    )
                    .await;
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
                                    if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                                        let id_clone = id2.clone();
                                        move || {
                                            crate::cache::get_namespaced_image_cache(
                                                "iptv", &id_clone,
                                            )
                                        }
                                    })
                                    .await
                                    {
                                        if let Some(img) = network::decode_poster(bytes).await {
                                            tx.send(Action::SearchPosterLoaded(id2, Some(img)))
                                                .ok();
                                            return;
                                        }
                                    }
                                    if let Some(bytes) =
                                        network::fetch_poster_bytes(&client, &cover_url).await
                                    {
                                        let bytes_clone = bytes.clone();
                                        let id_clone = id2.clone();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            crate::cache::set_namespaced_image_cache(
                                                "iptv",
                                                &id_clone,
                                                &bytes_clone,
                                            )
                                        })
                                        .await;
                                        if let Some(img) = network::decode_poster(bytes).await {
                                            tx.send(Action::SearchPosterLoaded(id2, Some(img)))
                                                .ok();
                                        }
                                    }
                                });
                            }
                        }
                    }
                    return None;
                }
                let mut prov = self.state.active_provider;
                if let Some(res) = self.state.search_results.iter().find(|r| r.id == id) {
                    prov = res.provider;
                }

                if prov == ProviderKind::FourKHdHub
                    || prov == ProviderKind::BdixCircleFtp
                    || prov == ProviderKind::BdixDhakaFlix
                {
                    self.state.preview_loading = false;
                    self.state.search_preview = None;
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
                        let prov = prov;
                        tokio::spawn(async move {
                            if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                                let id_clone = id2.clone();
                                move || {
                                    crate::cache::get_namespaced_image_cache(
                                        prov.cache_key(),
                                        &id_clone,
                                    )
                                }
                            })
                            .await
                            {
                                if let Some(img) = network::decode_poster(bytes).await {
                                    tx.send(Action::PosterSuccess(id2, img)).ok();
                                    return;
                                }
                            }
                            if let Some(bytes) = network::fetch_poster_bytes(&client, &url).await {
                                let bytes_clone = bytes.clone();
                                let id_clone = id2.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    crate::cache::set_namespaced_image_cache(
                                        prov.cache_key(),
                                        &id_clone,
                                        &bytes_clone,
                                    )
                                })
                                .await;
                                if let Some(img) = network::decode_poster(bytes).await {
                                    tx.send(Action::PosterSuccess(id2, img)).ok();
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
                    if let Ok(Some(cached_disk)) = tokio::task::spawn_blocking({
                        let id_clone = id_clone.clone();
                        move || crate::cache::get_provider_details_cache(prov, &id_clone)
                    })
                    .await
                    {
                        sender
                            .send(Action::PreviewSuccess(id_clone, cached_disk))
                            .ok();
                        return;
                    }

                    match client.get_details(&id_clone).await {
                        Ok(details) => {
                            let id_save = id_clone.clone();
                            let det_save = details.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_details_cache(prov, &id_save, &det_save)
                            })
                            .await;
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
                        .and_then(crate::tui::state::subject_id)
                } else {
                    self.state
                        .search_list_state
                        .selected()
                        .and_then(|idx| self.state.search_results.get(idx))
                        .map(|res| res.id.clone())
                };

                self.state.preview_loading = false;

                if current_id.as_deref() != Some(id.as_str()) {
                    return None;
                }

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
                    let provider = self.state.active_provider;
                    tokio::spawn(async move {
                        if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                            let id_clone = id_clone.clone();
                            move || {
                                crate::cache::get_namespaced_image_cache(
                                    provider.cache_key(),
                                    &id_clone,
                                )
                            }
                        })
                        .await
                        {
                            if let Some(img) = network::decode_poster(bytes).await {
                                let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
                                return;
                            }
                        }
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(5))
                            .build()
                            .unwrap_or_default();
                        if let Some(bytes) = network::fetch_poster_bytes(&client, &url_clone).await
                        {
                            let bytes_clone = bytes.clone();
                            let id_clone2 = id_clone.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_namespaced_image_cache(
                                    provider.cache_key(),
                                    &id_clone2,
                                    &bytes_clone,
                                )
                            })
                            .await;
                            if let Some(img) = network::decode_poster(bytes).await {
                                let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
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
                self.state
                    .set_status(format!("Preview failed: {}", err), 150);
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
                    if let Some(cached_img) = self
                        .state
                        .image_cache
                        .get(&id)
                        .or_else(|| self.state.search_posters.get(&id))
                    {
                        self.state.poster_image = Some((**cached_img).clone());
                    } else if let Some(cover_val) = payload.get("cover")
                        && let Some(url) = cover_val.get("url").and_then(|u| u.as_str())
                    {
                        let url_clone = url.to_string();
                        let action_tx = self.action_sender.clone();
                        let id_clone = id.clone();
                        let provider = context.provider;
                        let http_client = self.client.http_client().clone();
                        tokio::spawn(async move {
                            if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                                let id_clone = id_clone.clone();
                                move || {
                                    crate::cache::get_namespaced_image_cache(
                                        provider.cache_key(),
                                        &id_clone,
                                    )
                                }
                            })
                            .await
                            {
                                if let Some(img) = network::decode_poster(bytes).await {
                                    let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
                                    return;
                                }
                            }
                            let client = reqwest::Client::builder()
                                .timeout(std::time::Duration::from_secs(5))
                                .build()
                                .unwrap_or(http_client);
                            if let Some(bytes) =
                                network::fetch_poster_bytes(&client, &url_clone).await
                            {
                                let bytes_clone = bytes.clone();
                                let id_clone2 = id_clone.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    crate::cache::set_namespaced_image_cache(
                                        provider.cache_key(),
                                        &id_clone2,
                                        &bytes_clone,
                                    )
                                })
                                .await;
                                if let Some(img) = network::decode_poster(bytes).await {
                                    let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
                                }
                            }
                        });
                    }
                }

                let stype = crate::tui::state::stype(&payload);

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

                let mut default_season = 1;
                let mut default_episode = 1;
                if let Some(res) = self.state.search_results.iter().find(|r| r.id == id) {
                    if res.season > 0 {
                        default_season = res.season;
                    }
                    if res.episode > 0 {
                        default_episode = res.episode;
                    }
                }

                let season_idx = self
                    .state
                    .available_seasons
                    .iter()
                    .position(|s| {
                        s.get("se")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as usize == default_season)
                            .unwrap_or(false)
                    })
                    .unwrap_or(0);

                self.state.season_list_state.select(Some(season_idx));

                let ep_idx = self
                    .state
                    .available_episode_numbers
                    .get(season_idx)
                    .and_then(|eps| eps.iter().position(|&e| e == default_episode))
                    .unwrap_or(0);

                self.state.episode_list_state.select(Some(ep_idx));

                if let Some(dubs) = payload.get("dubs").and_then(|d| d.as_array()) {
                    let mut current_idx = 0;
                    for (i, dub) in dubs.iter().enumerate() {
                        let dub_id = dub.get("subjectId").and_then(crate::tui::state::subject_id);
                        if dub_id == Some(id.clone()) {
                            current_idx = i;
                        }
                    }
                    self.state.language_list_state.select(Some(current_idx));
                } else {
                    self.state.language_list_state.select(Some(0));
                }

                if !self.state.language_chosen {
                    self.state.selected_season = default_season;
                    self.state.selected_episode = default_episode;
                }

                let has_multiple_dubs = payload
                    .get("dubs")
                    .and_then(|d| d.as_array())
                    .is_some_and(|a| a.len() > 1);

                if has_multiple_dubs && !self.state.language_chosen {
                    self.state.details_pane = crate::tui::state::DetailsPane::Languages;
                    self.state.is_loading = false;
                    self.state
                        .set_status("Please select a language dubbing.".to_string(), 150);
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

            Action::DetailsFailure(context, err) => {
                if !self.context_is_current(context) {
                    return None;
                }
                log::error!(
                    "details fetch failed (provider {}): {err}",
                    context.provider.cache_key()
                );
                self.state.is_loading = false;
                self.state
                    .set_status(format!("Details fetch failed: {}", err), 150);
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
                    let stype = crate::tui::state::stype(details);
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

                if !force_refresh {
                    let id_clone = subject_id.clone();
                    let prov = context.provider;
                    let sender = self.action_sender.clone();
                    let req_id = request_id;
                    if let Ok(Some(cached)) = tokio::task::spawn_blocking(move || {
                        crate::cache::get_provider_stream_cache(prov, &id_clone, season, episode)
                            .and_then(|v| v.as_array().cloned())
                    })
                    .await
                    {
                        tokio::spawn(async move {
                            sender
                                .send(Action::EpisodeStreamsReady(
                                    context,
                                    req_id,
                                    subject_id.clone(),
                                    season,
                                    episode,
                                    serde_json::Value::Array(cached),
                                ))
                                .ok();
                        });
                        return None;
                    }
                }

                if context.provider == ProviderKind::FourKHdHub || context.provider.is_bdix() {
                    let sender = self.action_sender.clone();
                    let fourk_client = self.fourk_client.clone();
                    let circleftp_client = self.circleftp_client.clone();
                    let dhakaflix_client = self.dhakaflix_client.clone();
                    let id = subject_id.clone();
                    tokio::spawn(async move {
                        let result = match context.provider {
                            ProviderKind::FourKHdHub => {
                                crate::providers::ReleaseProvider::episode_streams(
                                    &fourk_client,
                                    &id,
                                    season,
                                    episode,
                                )
                                .await
                            }
                            ProviderKind::BdixCircleFtp => {
                                crate::providers::ReleaseProvider::episode_streams(
                                    &circleftp_client,
                                    &id,
                                    season,
                                    episode,
                                )
                                .await
                            }
                            _ => {
                                crate::providers::ReleaseProvider::episode_streams(
                                    &dhakaflix_client,
                                    &id,
                                    season,
                                    episode,
                                )
                                .await
                            }
                        };
                        match result {
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
                                    context,
                                    request_id,
                                    id_clone,
                                    season,
                                    episode,
                                    "Rate Limit".into(),
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
                            let rid = item.get("resourceId").and_then(|r| r.as_str());
                            let link = item
                                .get("resourceLink")
                                .and_then(|l| l.as_str())
                                .unwrap_or("");

                            let mut exists = false;
                            for i in entry.iter_mut() {
                                let i_rid = i.get("resourceId").and_then(|r| r.as_str());
                                if rid.is_some() && i_rid == rid {
                                    if let Some(obj) = i.as_object_mut() {
                                        obj.insert(
                                            "resourceLink".to_string(),
                                            serde_json::Value::String(link.to_string()),
                                        );
                                    }
                                    exists = true;
                                    break;
                                }

                                let i_link =
                                    i.get("resourceLink").and_then(|l| l.as_str()).unwrap_or("");
                                let base_link = link.split('?').next().unwrap_or(link);
                                let i_base_link = i_link.split('?').next().unwrap_or(i_link);

                                if base_link == i_base_link && !base_link.is_empty() {
                                    if let Some(obj) = i.as_object_mut() {
                                        obj.insert(
                                            "resourceLink".to_string(),
                                            serde_json::Value::String(link.to_string()),
                                        );
                                    }
                                    exists = true;
                                    break;
                                }
                            }

                            if !exists {
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
                self.state
                    .set_status(format!("{} streams available.", count), 150);

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
                                        let sub_url =
                                            crate::tui::state::caption_url_for(&res, &pref_lang);
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
                log::error!(
                    "episode streams failed ({} s{}e{}): {err}",
                    context.provider.cache_key(),
                    target_se,
                    target_ep
                );
                self.state.stream_error = Some(err.clone());
                self.state.set_status(format!("Error: {}", err), 150);
            }
            _ => return None,
        }
        None
    }
}
