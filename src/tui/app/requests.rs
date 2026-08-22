use super::{App, network};
use crate::providers::{fourkhdhub::releases_to_moviebox_json, models::ProviderKind};
use crate::tui::{
    action::Action,
    state::{InputMode, Screen, SearchResult},
};

impl App {
    pub(super) async fn handle_requests(&mut self, action: Action) -> Option<()> {
        match action {
            Action::Suggest(query) => {
                self.state.active_suggest_request =
                    self.state.active_suggest_request.wrapping_add(1);
                let request_id = self.state.active_suggest_request;
                if query.starts_with('/') {
                    let matching_commands =
                        crate::tui::commands::SlashCommand::suggest(&self.state, &query);
                    let suggestions: Vec<serde_json::Value> = matching_commands
                        .into_iter()
                        .map(|cmd_name| serde_json::json!({ "title": cmd_name }))
                        .collect();
                    if !suggestions.is_empty() {
                        let fake_payload = serde_json::json!({
                            "results": [{
                                "subjects": suggestions
                            }]
                        });
                        self.action_sender
                            .send(Action::SuggestSuccess(request_id, query, fake_payload))
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

                let service = self.service.clone();
                let sender = self.action_sender.clone();
                let query_clone = query.clone();
                tokio::spawn(async move {
                    if let Ok(res) = service.suggest(&query_clone).await {
                        sender
                            .send(Action::SuggestSuccess(request_id, query_clone, res))
                            .ok();
                    }
                });
            }

            Action::SuggestSuccess(request_id, query, payload) => {
                if request_id != self.state.active_suggest_request {
                    return None;
                }
                if !query.starts_with('/')
                    && (self.state.is_tv_mode
                        || self.state.active_provider != ProviderKind::MovieBox)
                {
                    return None;
                }
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
                    let limit = if query.starts_with('/') { 20 } else { 10 };
                    for item in subjects.iter().take(limit) {
                        let raw_title = item
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let clean_title = if query.starts_with('/') {
                            raw_title
                        } else {
                            crate::providers::moviebox::clean_moviebox_title(&raw_title)
                        };

                        if query.starts_with('/') {
                            if clean_title.starts_with(&query)
                                && !self.state.search_suggestions.contains(&clean_title)
                            {
                                self.state.search_suggestions.push(clean_title);
                            }
                            continue;
                        }

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
                self.state.search_query = query.clone();
                self.state.suggest_index = None;
                self.state.search_suggestions.clear();
                self.state.input_mode = InputMode::Normal;
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

                if lower_query == "/history"
                    && (self.state.mode() == crate::tui::state::AppMode::Streaming
                        || self.state.mode() == crate::tui::state::AppMode::Addon)
                {
                    self.state.input_mode = InputMode::Normal;
                    self.state.is_loading = false;
                    self.state.is_homepage_mode = false;
                    self.state.active_browse_preset = None;
                    self.state.browse_metrics.clear();
                    self.state.active_screen = Screen::Home;
                    self.state.active_subject_id = None;
                    self.state.active_preview_request =
                        self.state.active_preview_request.wrapping_add(1);
                    self.state.search_results.clear();
                    self.state.search_error = None;
                    self.state.search_preview = None;
                    self.state.preview_loading = false;
                    self.state.poster_image = None;
                    self.state.poster_protocol = None;
                    self.state.failed_posters.clear();
                    self.state.in_flight_posters.clear();
                    self.state.search_list_state.select(None);
                    self.state.search_suggestions.clear();
                    self.state.suggest_index = None;

                    self.state.search_query = "/history".to_string();
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
                            let provider = ProviderKind::parse(&item.provider).unwrap_or_else(|| {
                                log::warn!(
                                    "unknown watch-history provider '{}'; defaulting to MovieBox",
                                    item.provider
                                );
                                ProviderKind::MovieBox
                            });
                            self.state.search_results.push(SearchResult {
                                id: item.subject_id.clone(),
                                title: item.title.clone(),
                                stype: item.stype,
                                release_year: item.release_year.clone(),
                                cover_url: item.cover_url.clone(),
                                season: item.season,
                                episode: item.episode,
                                provider,
                            });
                        }

                        self.state.search_list_state.select(Some(0));
                        self.prefetch_visible_posters();
                    }
                    return None;
                }

                if lower_query == "/favorites" && self.state.favorites_available() {
                    self.load_favorites_virtual_list();
                    return None;
                }

                if self.handle_search_command(&query, &lower_query).is_some() {
                    return None;
                }
                if query.trim().starts_with('/') {
                    return None;
                }
                let context = self.prepare_search_request(&query);
                self.run_search_request(query.clone(), force_refresh, context);
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
                self.prepare_homepage_request(&tab_id, page);
                self.run_homepage_request(tab_id, page);
            }

            Action::SelectBrowse(preset) => {
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state.set_status(
                        "Browse is available only with the MovieBox provider.".to_string(),
                        180,
                    );
                    return None;
                }
                self.state.show_browse_popup = false;
                self.state.active_browse_preset = Some(preset);
                self.state.active_addon_catalog = None;
                self.state.browse_list_state.select(None);
                self.state.search_query.clear();
                self.action_sender
                    .send(Action::FetchHomepage {
                        tab_id: "2".to_string(),
                        page: 1,
                    })
                    .ok();
            }

            Action::SelectAddonCatalog(target) => {
                self.state.show_browse_popup = false;
                self.state.browse_list_state.select(None);
                let context = self.prepare_addon_catalog_request(&target);
                let request_id = self.state.active_search_request;
                let sender = self.action_sender.clone();
                let service = self.service.clone();
                let manifest_url = target.manifest_url.clone();
                let r_type = target.r#type.clone();
                let cat_id = target.catalog_id.clone();

                tokio::spawn(async move {
                    let result = service
                        .fetch_addon_catalog(&manifest_url, &r_type, &cat_id)
                        .await;
                    match result {
                        Ok(res) => {
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    request_id,
                                    query: String::new(),
                                    page: 1,
                                    payload: res,
                                })
                                .ok();
                        }
                        Err(error) => {
                            sender
                                .send(Action::SearchFailure(context, request_id, 1, error))
                                .ok();
                        }
                    }
                });
            }

            Action::SearchSuccess {
                context,
                request_id,
                query,
                page,
                payload,
            } => {
                if request_id != self.state.active_search_request {
                    return None;
                }
                if !self.context_is_current(context) || query != self.state.search_query.trim() {
                    if self.state.search_query.trim().is_empty() {
                        self.state.is_loading = false;
                    }
                    return None;
                }
                self.state.current_page = page;
                self.state.search_error = None;
                self.state.is_loading = false;
                if page <= 1 {
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
                            });
                        }
                    }
                    let previous_selected_id = if page > 1 {
                        self.state.search_list_state.selected().and_then(|idx| {
                            self.state.search_results.get(idx).map(|r| r.id.clone())
                        })
                    } else {
                        None
                    };

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

                    if let Some(prev_id) = previous_selected_id {
                        if let Some(new_idx) = self
                            .state
                            .search_results
                            .iter()
                            .position(|r| r.id == prev_id)
                        {
                            self.state.search_list_state.select(Some(new_idx));
                        }
                    }
                }

                if !self.state.search_results.is_empty() {
                    self.prefetch_visible_posters();
                }

                if page <= 1 {
                    self.prepare_image_refresh();
                }

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
                if page <= 1 {
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

            Action::SearchFailure(context, request_id, page, err) => {
                if request_id != self.state.active_search_request {
                    return None;
                }
                if !self.context_is_current(context) {
                    if self.state.search_query.trim().is_empty() {
                        self.state.is_loading = false;
                    }
                    return None;
                }
                if page > 1 && self.state.current_page >= page {
                    self.state.current_page = page - 1;
                }
                log::error!(
                    "search failed (provider {}): {err}",
                    context.provider.cache_key()
                );
                self.state.is_loading = false;
                if page <= 1 {
                    self.state.search_results.clear();
                    self.state.search_list_state.select(None);
                    self.state.search_preview = None;
                    self.state.poster_image = None;
                    self.state.poster_protocol = None;
                    self.state.search_posters.clear();
                    self.state.failed_posters.clear();
                    self.state.search_poster_protocols.clear();
                    self.state.in_flight_posters.clear();
                }
                self.state.search_error = Some(err.clone());
                self.state
                    .set_status(format!("Search failed: {}", err), 150);
            }

            Action::HomepageSuccess {
                request_id,
                tab_id,
                page,
                payload,
            } => {
                if request_id != self.state.active_homepage_request {
                    return None;
                }
                if !self.state.is_homepage_mode || self.state.current_tab_id != tab_id {
                    return None;
                }
                self.state.is_loading = false;
                if page == 1 {
                    self.state.search_results.clear();
                    self.state.search_error = None;
                }

                let previous_selected_id = if page > 1 {
                    self.state
                        .search_list_state
                        .selected()
                        .and_then(|idx| self.state.search_results.get(idx).map(|r| r.id.clone()))
                } else {
                    None
                };

                let extracted_subjects = self
                    .state
                    .active_browse_preset
                    .map(|preset| Self::extract_browse_subjects(&payload, preset))
                    .unwrap_or_else(|| Self::extract_homepage_subjects(&payload));
                let count = self.append_homepage_subjects(extracted_subjects);
                self.sort_browse_results();

                if let Some(prev_id) = previous_selected_id {
                    if let Some(new_idx) = self
                        .state
                        .search_results
                        .iter()
                        .position(|r| r.id == prev_id)
                    {
                        self.state.search_list_state.select(Some(new_idx));
                    }
                } else if count > 0 && self.state.current_page <= 1 {
                    self.state.search_list_state.select(Some(0));
                    if let Some(first) = self.state.search_results.first() {
                        self.action_sender
                            .send(Action::FetchPreview(first.id.clone()))
                            .ok();
                    }
                } else if count == 0 && self.state.current_page <= 1 {
                    self.state.search_list_state.select(None);
                }

                if count > 0 {
                    self.prefetch_visible_posters();
                }

                if self.state.current_page <= 1 {
                    self.prepare_image_refresh();
                }

                let status = self
                    .state
                    .active_browse_preset
                    .map(|preset| {
                        format!(
                            "{} · {} items",
                            preset.label(),
                            self.state.search_results.len()
                        )
                    })
                    .unwrap_or_else(|| {
                        format!("Found {} discover items", self.state.search_results.len())
                    });
                self.state.set_status(status, 150);
            }

            Action::HomepageFailure(request_id, err) => {
                if request_id != self.state.active_homepage_request {
                    return None;
                }
                log::error!("discover failed: {err}");
                self.state.is_loading = false;
                self.state.search_error = Some(format!("Discover failed: {err}"));
                self.state.search_results.clear();
                self.state.search_list_state.select(None);
                self.state.search_preview = None;
                self.state.poster_image = None;
                self.state.poster_protocol = None;
                self.state.search_posters.clear();
                self.state.failed_posters.clear();
                self.state.search_poster_protocols.clear();
                self.state.in_flight_posters.clear();
                self.state
                    .set_status(format!("Discover failed: {}", err), 150);
            }

            Action::FetchDetails(id, force_refresh) => {
                let context = self.prepare_details_request(&id);
                self.run_details_request(id, force_refresh, context);
            }

            Action::FetchPreview(id) => {
                self.state.active_preview_request =
                    self.state.active_preview_request.wrapping_add(1);
                let request_id = self.state.active_preview_request;
                if self.state.is_tv_mode {
                    self.state.preview_loading = false;
                    if !self.state.image_cache.contains(&id) {
                        if let Some(channel) =
                            self.state.tv_channels.iter().find(|c| c.stream_url == id)
                        {
                            let cover_url = channel.logo.clone();
                            if !cover_url.is_empty() {
                                let tx = self.action_sender.clone();
                                let client = self.service.http_client().clone();
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
                let prov = self.provider_for_subject(&id);

                if prov == ProviderKind::FourKHdHub
                    || prov == ProviderKind::BdixCircleFtp
                    || prov == ProviderKind::BdixDhakaFlix
                {
                    self.state.preview_loading = false;
                    self.state.search_preview = None;
                    self.state.poster_image = None;
                    self.state.poster_protocol = None;
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
                        let client = self.service.http_client().clone();
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
                let client = self.service.client.clone();
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
                            .send(Action::PreviewSuccess(request_id, id_clone, cached_disk))
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
                            sender
                                .send(Action::PreviewSuccess(request_id, id_clone, details))
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::PreviewFailure(request_id, format!("{:?}", e)))
                                .ok();
                        }
                    }
                });
            }

            Action::PreviewSuccess(request_id, id, json) => {
                if request_id != self.state.active_preview_request {
                    return None;
                }
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
                } else if let Some(url) = crate::tui::app::playback::extract_cover_url(&json) {
                    self.state.history.update_cover_url(&id, &url);
                    let url_clone = url.to_string();
                    let action_tx = self.action_sender.clone();
                    let id_clone = id.clone();
                    let http_client = self.service.http_client().clone();
                    tokio::spawn(async move {
                        if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                            let id_clone = id_clone.clone();
                            move || crate::cache::get_namespaced_image_cache("posters", &id_clone)
                        })
                        .await
                        {
                            if let Some(img) = network::decode_poster(bytes).await {
                                let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
                                return;
                            }
                        }
                        if let Some(bytes) =
                            network::fetch_poster_bytes(&http_client, &url_clone).await
                        {
                            let bytes_clone = bytes.clone();
                            let id_clone2 = id_clone.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_namespaced_image_cache(
                                    "posters",
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
                self.state.search_posters.put(id.clone(), img.clone());

                let current_id = if self.state.active_screen == Screen::Details {
                    self.state.active_subject_id.clone()
                } else {
                    self.state
                        .search_list_state
                        .selected()
                        .and_then(|idx| self.state.search_results.get(idx))
                        .map(|res| res.id.clone())
                };

                if current_id.as_deref() == Some(id.as_str()) {
                    self.state.poster_image = Some((*img).clone());
                    self.state.poster_protocol = None;
                }
            }

            Action::SearchPosterLoaded(id, img_opt) => {
                self.state.in_flight_posters.remove(&id);
                if let Some(img) = img_opt {
                    self.state.image_cache.put(id.clone(), img.clone());
                    self.state.search_posters.put(id, img);
                } else {
                    self.state.failed_posters.put(id, ());
                }
            }

            Action::PreviewFailure(request_id, err) => {
                if request_id != self.state.active_preview_request {
                    return None;
                }
                self.state.preview_loading = false;
                self.state
                    .set_status(format!("Preview failed: {}", err), 150);
            }

            Action::DetailsSuccess(context, request_id, id, payload) => {
                if request_id != self.state.active_details_request {
                    return None;
                }
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

                if let Some(final_obj) = final_payload.as_object_mut() {
                    if let Some(existing) = &self.state.selected_details {
                        if let Some(existing_obj) = existing.as_object() {
                            if final_obj
                                .get("title")
                                .and_then(|t| t.as_str())
                                .is_none_or(|s| s.trim().is_empty())
                            {
                                if let Some(v) = existing_obj.get("title") {
                                    final_obj.insert("title".to_string(), v.clone());
                                }
                            }
                            if final_obj
                                .get("releaseDate")
                                .and_then(|y| y.as_str())
                                .is_none_or(|s| s.trim().is_empty())
                            {
                                if let Some(v) = existing_obj.get("releaseDate") {
                                    final_obj.insert("releaseDate".to_string(), v.clone());
                                }
                            }
                            if (!final_obj.contains_key("cover")
                                || final_obj
                                    .get("cover")
                                    .and_then(|c| c.get("url"))
                                    .and_then(|u| u.as_str())
                                    .is_none_or(|s| s.trim().is_empty()))
                                && let Some(v) = existing_obj.get("cover")
                            {
                                final_obj.insert("cover".to_string(), v.clone());
                            }
                            if final_obj
                                .get("description")
                                .and_then(|d| d.as_str())
                                .is_none_or(|s| s.trim().is_empty())
                                && let Some(v) = existing_obj
                                    .get("description")
                                    .or_else(|| existing_obj.get("intro"))
                            {
                                final_obj.insert("description".to_string(), v.clone());
                            }
                            if final_obj
                                .get("imdbRatingValue")
                                .and_then(|r| r.as_str())
                                .is_none_or(|s| s.trim().is_empty())
                                && let Some(v) = existing_obj.get("imdbRatingValue")
                            {
                                final_obj.insert("imdbRatingValue".to_string(), v.clone());
                            }
                            if (!final_obj.contains_key("genre")
                                || final_obj
                                    .get("genre")
                                    .and_then(|g| g.as_array())
                                    .is_none_or(|a| a.is_empty()))
                                && let Some(v) = existing_obj.get("genre")
                            {
                                final_obj.insert("genre".to_string(), v.clone());
                            }
                        }
                    }

                    if let Some(res) = self.state.search_results.iter().find(|r| r.id == id) {
                        if final_obj
                            .get("title")
                            .and_then(|t| t.as_str())
                            .is_none_or(|s| s.trim().is_empty())
                        {
                            final_obj.insert(
                                "title".to_string(),
                                serde_json::Value::String(res.title.clone()),
                            );
                        }
                        if final_obj
                            .get("releaseDate")
                            .and_then(|y| y.as_str())
                            .is_none_or(|s| s.trim().is_empty())
                        {
                            final_obj.insert(
                                "releaseDate".to_string(),
                                serde_json::Value::String(res.release_year.clone()),
                            );
                        }
                        if (!final_obj.contains_key("cover")
                            || final_obj
                                .get("cover")
                                .and_then(|c| c.get("url"))
                                .and_then(|u| u.as_str())
                                .is_none_or(|s| s.trim().is_empty()))
                            && res.cover_url.is_some()
                        {
                            final_obj.insert(
                                "cover".to_string(),
                                serde_json::json!({ "url": res.cover_url }),
                            );
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
                    } else if let Some(url) = crate::tui::app::playback::extract_cover_url(&payload)
                    {
                        self.state.history.update_cover_url(&id, &url);
                        let url_clone = url.to_string();
                        let action_tx = self.action_sender.clone();
                        let id_clone = id.clone();
                        let http_client = self.service.http_client().clone();
                        tokio::spawn(async move {
                            if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                                let id_clone = id_clone.clone();
                                move || {
                                    crate::cache::get_namespaced_image_cache("posters", &id_clone)
                                }
                            })
                            .await
                            {
                                if let Some(img) = network::decode_poster(bytes).await {
                                    let _ = action_tx.send(Action::PosterSuccess(id_clone, img));
                                    return;
                                }
                            }
                            if let Some(bytes) =
                                network::fetch_poster_bytes(&http_client, &url_clone).await
                            {
                                let bytes_clone = bytes.clone();
                                let id_clone2 = id_clone.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    crate::cache::set_namespaced_image_cache(
                                        "posters",
                                        &id_clone2,
                                        &bytes_clone,
                                    );
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
                    .filter(|a| !a.is_empty())
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
                    } else if let Some(arr) =
                        season.get("episodeNumbers").and_then(|e| e.as_array())
                    {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as usize))
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
                if let Some(history) = self.state.history.recent.iter().rev().find(|item| {
                    (ProviderKind::parse(&item.provider) == Some(context.provider)
                        || item.provider == context.provider.label()
                        || item.provider == context.provider.cache_key())
                        && item.subject_id == id
                        && item.season > 0
                        && item.episode > 0
                }) {
                    default_season = history.season;
                    default_episode = history.episode;
                }

                let target_season = if self.state.language_chosen && self.state.selected_season > 0
                {
                    self.state.selected_season
                } else {
                    default_season
                };
                let target_episode =
                    if self.state.language_chosen && self.state.selected_episode > 0 {
                        self.state.selected_episode
                    } else {
                        default_episode
                    };

                let season_idx = self
                    .state
                    .available_seasons
                    .iter()
                    .position(|s| {
                        s.get("se")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as usize == target_season)
                            .unwrap_or(false)
                    })
                    .unwrap_or(0);

                self.state.season_list_state.select(Some(season_idx));

                let ep_idx = self
                    .state
                    .available_episode_numbers
                    .get(season_idx)
                    .and_then(|eps| eps.iter().position(|&e| e == target_episode))
                    .unwrap_or(0);

                self.state.episode_list_state.select(Some(ep_idx));

                if let Some(dubs) = payload.get("dubs").and_then(|d| d.as_array()) {
                    let find_by_pattern = |patterns: &[&str]| {
                        dubs.iter().position(|dub| {
                            let labels = [
                                dub.get("title").and_then(|v| v.as_str()),
                                dub.get("name").and_then(|v| v.as_str()),
                                dub.get("lanName").and_then(|v| v.as_str()),
                                dub.get("audioName").and_then(|v| v.as_str()),
                            ];
                            let label = labels.into_iter().flatten().collect::<Vec<_>>().join(" ");
                            let lower = label.to_ascii_lowercase();
                            patterns.iter().any(|pat| lower.contains(pat))
                        })
                    };

                    let preferred_idx = if self.state.language_chosen {
                        dubs.iter()
                            .position(|dub| {
                                dub.get("subjectId").and_then(crate::tui::state::subject_id)
                                    == Some(id.clone())
                            })
                            .unwrap_or(0)
                    } else {
                        find_by_pattern(&["original", "orig"])
                            .or_else(|| find_by_pattern(&["english", "eng"]))
                            .unwrap_or(0)
                    };

                    self.state.language_list_state.select(Some(preferred_idx));
                } else {
                    self.state.language_list_state.select(Some(0));
                }

                self.state.selected_season = target_season;
                self.state.selected_episode = target_episode;

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
                    if !self.state.language_chosen {
                        if stype == 2 && !self.state.available_seasons.is_empty() {
                            self.state.details_pane = crate::tui::state::DetailsPane::Seasons;
                        } else {
                            self.state.details_pane = crate::tui::state::DetailsPane::Streams;
                        }
                    }

                    self.state.is_loading = true;
                    self.state
                        .fetch_cancel
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    self.action_sender.send(Action::InitStreamPool(id)).ok();
                }
            }
            Action::DetailsFailure(context, request_id, err) => {
                if request_id != self.state.active_details_request {
                    return None;
                }
                if !self.context_is_current(context) {
                    return None;
                }
                log::error!(
                    "details fetch failed (provider {}): {err}",
                    context.provider.cache_key()
                );
                self.state.is_loading = false;
                self.state.is_resolving_playback = false;
                self.state.is_waiting_for_download_stream = false;
                if self.state.selected_details.is_none() {
                    self.state.details_pane = crate::tui::state::DetailsPane::default();
                    self.state.selected_season = 1;
                    self.state.selected_episode = 1;
                }
                self.state.is_fetching_streams = false;
                self.state.stream_error = None;
                self.state
                    .set_status(format!("Details fetch failed: {}", err), 150);

                if self.state.active_screen == Screen::Details {
                    if let Some(id) = self.state.active_subject_id.clone().or_else(|| {
                        self.state
                            .selected_details
                            .as_ref()
                            .and_then(|d| d.get("id"))
                            .and_then(|i| i.as_str())
                            .map(|s| s.to_string())
                    }) {
                        self.state.active_subject_id = Some(id.clone());

                        if self.state.poster_image.is_none() {
                            if let Some(cached_img) = self
                                .state
                                .image_cache
                                .get(&id)
                                .or_else(|| self.state.search_posters.get(&id))
                            {
                                self.state.poster_image = Some((**cached_img).clone());
                            } else if let Some(details) = &self.state.selected_details {
                                if let Some(url) =
                                    crate::tui::app::playback::extract_cover_url(details)
                                {
                                    let url_clone = url.to_string();
                                    let action_tx = self.action_sender.clone();
                                    let id_clone = id.clone();
                                    let http_client = self.service.http_client().clone();
                                    tokio::spawn(async move {
                                        if let Some(bytes) =
                                            network::fetch_poster_bytes(&http_client, &url_clone)
                                                .await
                                        {
                                            if let Some(img) = network::decode_poster(bytes).await {
                                                let _ = action_tx
                                                    .send(Action::PosterSuccess(id_clone, img));
                                            }
                                        }
                                    });
                                }
                            }
                        }

                        self.action_sender.send(Action::InitStreamPool(id)).ok();
                    }
                }
            }

            Action::InitStreamPool(subject_id) => {
                if self.provider_for_subject(&subject_id) != ProviderKind::MovieBox {
                    self.state
                        .stream_pool
                        .insert(subject_id.clone(), Default::default());
                    self.trigger_episode_fetch();
                    return None;
                }
                let service = self.service.clone();
                let sender = self.action_sender.clone();
                tokio::spawn(async move {
                    let resolutions = service
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
                    (0usize, 0usize)
                };

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

                let mut context = self.request_context();
                context.provider = self.provider_for_subject(&subject_id);

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

                if context.provider == ProviderKind::Addons {
                    let sender = self.action_sender.clone();
                    let client = self.service.addon_client.clone();
                    let addons = crate::config::load_addons();
                    let id = subject_id.clone();
                    let is_series = self
                        .state
                        .selected_details
                        .as_ref()
                        .map(|d| crate::tui::state::stype(d) == 2)
                        .unwrap_or(season > 0);

                    let has_stream_addons = addons.iter().any(|a| a.enabled && a.provides_stream);
                    tokio::spawn(async move {
                        if !has_stream_addons {
                            sender
                                .send(Action::EpisodeStreamsFailed(
                                    context,
                                    request_id,
                                    id,
                                    season,
                                    episode,
                                    "No streaming addons are currently installed or enabled.\nPress Ctrl+P or open /config to install/enable a stream provider.".into(),
                                ))
                                .ok();
                            return;
                        }

                        let (releases, blocked_addons) =
                            crate::providers::addons::aggregate_streams(
                                &client, &addons, &id, season, episode, is_series,
                            )
                            .await;

                        if !blocked_addons.is_empty() {
                            sender.send(Action::SetStatus(format!(
                                "Warning: {} streams blocked (raw torrents). Only HTTP streams are supported.",
                                blocked_addons.join(", ")
                            ))).ok();
                        }

                        if !releases.is_empty() {
                            let json = crate::providers::addons::adapter::releases_to_moviebox_json(
                                &releases,
                            );
                            let id_clone = id.clone();
                            let json_clone = json.clone();
                            let provider = context.provider;
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_stream_cache(
                                    provider,
                                    &id_clone,
                                    season,
                                    episode,
                                    &json_clone,
                                );
                            });

                            sender
                                .send(Action::EpisodeStreamsReady(
                                    context, request_id, id, season, episode, json,
                                ))
                                .ok();
                        } else {
                            sender
                                .send(Action::EpisodeStreamsFailed(
                                    context,
                                    request_id,
                                    id,
                                    season,
                                    episode,
                                    "No HTTP streams found from active addons for this title.\nPress r to retry or install additional stream addons via /config.".into(),
                                ))
                                .ok();
                        }
                    });
                    return None;
                }

                if context.provider == ProviderKind::FourKHdHub || context.provider.is_bdix() {
                    let sender = self.action_sender.clone();
                    let fourk_client = self.service.fourk_client.clone();
                    let circleftp_client = self.service.circleftp_client.clone();
                    let dhakaflix_client = self.service.dhakaflix_client.clone();
                    let id = subject_id.clone();
                    tokio::spawn(async move {
                        let result = match context.provider {
                            ProviderKind::FourKHdHub => {
                                if let Some(client) = fourk_client.as_ref() {
                                    crate::providers::ReleaseProvider::episode_streams(
                                        client, &id, season, episode,
                                    )
                                    .await
                                } else {
                                    Err("4KHDHub provider is unavailable".to_string())
                                }
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

                let pool = self
                    .state
                    .stream_pool
                    .entry(subject_id.clone())
                    .or_default();
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

                let client = self.service.client.clone();
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
                    let mut any_fetch_failed = false;

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
                                _ => {
                                    any_fetch_failed = true;
                                    break;
                                }
                            }
                        }
                    } else {
                        let concurrency_limit = std::sync::Arc::new(tokio::sync::Semaphore::new(2));
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
                                let permit = concurrency_limit.clone();
                                page_handles.push(tokio::spawn(async move {
                                    let _permit = permit.acquire_owned().await.ok();
                                    if ct.load(std::sync::atomic::Ordering::Relaxed) {
                                        return (Vec::new(), serde_json::json!({}), false);
                                    }
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(15),
                                        c.fetch_resource_page(&id, res, page),
                                    )
                                    .await
                                    {
                                        Ok(Ok((items, pager))) => (items, pager, true),
                                        _ => (Vec::new(), serde_json::json!({}), false),
                                    }
                                }));
                            }

                            let mut page_empty = true;
                            let mut has_more = false;
                            for handle in page_handles {
                                if let Ok((items, pager, ok)) = handle.await {
                                    if !ok {
                                        any_fetch_failed = true;
                                    }
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
                                        let se =
                                            item.get("se").and_then(|v| v.as_i64()).unwrap_or(0)
                                                as usize;
                                        let ep =
                                            item.get("ep").and_then(|v| v.as_i64()).unwrap_or(0)
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
                        let provider_name = context.provider.label();
                        let err_msg = if any_fetch_failed && all_items.is_empty() {
                            format!("Network connection failed to {provider_name}")
                        } else if any_fetch_failed {
                            format!("Rate limited by {provider_name}")
                        } else if all_items.is_empty() {
                            format!("No stream sources available on {provider_name}")
                        } else {
                            format!("Episode S{season}E{episode} is not listed on {provider_name}")
                        };
                        sender
                            .send(Action::EpisodeStreamsFailed(
                                context, request_id, id_clone, season, episode, err_msg,
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
                            } else if se == 0 && ep == 0 {
                                se = target_se;
                                ep = target_ep;
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
                            let service = self.service.clone();
                            let sender = self.action_sender.clone();
                            let pref = self.state.season_subtitle_preference.clone();
                            let no_pref = pref.is_none();

                            tokio::spawn(async move {
                                if let Ok(res) = service.get_ext_captions(&subject_id, &rid).await {
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
