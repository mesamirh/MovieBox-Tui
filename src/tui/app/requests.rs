use super::{App, network};
use crate::models::{Episode, MediaType, ProviderKind, Release, Season};
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
                    if !matching_commands.is_empty() {
                        self.action_sender
                            .send(Action::SuggestSuccess(request_id, query, matching_commands))
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
                if let Some(cached) = self.state.suggest_cache.get(&query).cloned() {
                    self.action_sender
                        .send(Action::SuggestSuccess(request_id, query, cached))
                        .ok();
                    return None;
                }

                self.request_tasks.cancel_suggest();
                let service = self.service.clone();
                let sender = self.action_sender.clone();
                let query_clone = query.clone();
                self.request_tasks.suggest = Some(tokio::spawn(async move {
                    if let Ok(res) = service.suggest(&query_clone).await {
                        sender
                            .send(Action::SuggestSuccess(request_id, query_clone, res))
                            .ok();
                    }
                }));
            }

            Action::SuggestSuccess(request_id, query, suggestions) => {
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
                if !query.starts_with('/') {
                    self.state
                        .suggest_cache
                        .put(query.clone(), suggestions.clone());
                }

                self.state.search_suggestions.clear();

                let limit = if query.starts_with('/') { 20 } else { 10 };
                for raw_title in suggestions.into_iter().take(limit) {
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
                    if !normalized_title.contains(&normalized_query) && !normalized_query.is_empty()
                    {
                        continue;
                    }

                    if !self.state.search_suggestions.contains(&clean_title) {
                        self.state.search_suggestions.push(clean_title);
                    }
                }
            }

            Action::SelectSuggestion { query } => {
                self.state.search_query.set_content(&query);
                self.state.suggest_index = None;
                self.state.search_suggestions.clear();
                self.state.input_mode = InputMode::Normal;
                self.state.is_loading = true;
                self.state.has_search_settled = false;
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
                    && self.state.mode() == crate::tui::state::AppMode::Streaming
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

                    self.state.search_query.set_content("/history");
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
                    self.state.set_status_long(
                        "This provider exposes search, not a shared MovieBox homepage.",
                    );
                    return None;
                }
                self.prepare_homepage_request(&tab_id, page);
                self.run_homepage_request(tab_id, page);
            }

            Action::SelectBrowse(preset) => {
                if self.state.active_provider != ProviderKind::MovieBox {
                    self.state
                        .set_status_long("Browse is available only with the MovieBox provider.");
                    return None;
                }
                self.state.show_browse_popup = false;
                self.state.active_browse_preset = Some(preset);
                self.state.active_addon_catalog = None;
                self.state.browse_list_state.select(None);
                self.state.search_query.clear();
                self.state.is_loading = true;
                self.state.has_search_settled = false;
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
                        Ok(items) => {
                            sender
                                .send(Action::SearchSuccess {
                                    context,
                                    request_id,
                                    query: String::new(),
                                    page: 1,
                                    items,
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
                items,
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
                self.state.has_search_settled = true;
                if page <= 1 {
                    self.state.search_results.clear();
                }

                for item in items {
                    let id = item.id.value.clone();
                    let raw_title = item.title.clone();
                    let clean_title = crate::providers::moviebox::clean_moviebox_title(&raw_title);

                    let normalized_query = query
                        .to_lowercase()
                        .replace(|c: char| !c.is_alphanumeric(), "");
                    let normalized_title = raw_title
                        .to_lowercase()
                        .replace(|c: char| !c.is_alphanumeric(), "");
                    if !normalized_title.contains(&normalized_query) && !normalized_query.is_empty()
                    {
                        continue;
                    }

                    let stype = if item.media_type == crate::models::MediaType::Series {
                        2
                    } else {
                        1
                    };
                    let release_year = item.year.clone().unwrap_or_default();
                    let cover_url = item.poster_url.clone();
                    if let Some(existing) =
                        self.state.search_results.iter_mut().find(|r| r.id == id)
                    {
                        if existing.title.is_empty() {
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
                            season: 0,
                            episode: 1,
                            provider: context.provider,
                        });
                    }
                }

                let previous_selected_id = if page > 1 {
                    self.state
                        .search_list_state
                        .selected()
                        .and_then(|idx| self.state.search_results.get(idx).map(|r| r.id.clone()))
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

                if !self.state.search_results.is_empty() {
                    self.prefetch_visible_posters();
                }

                if page <= 1 {
                    self.prepare_image_refresh();
                }

                self.state
                    .set_status_default(if self.state.search_results.is_empty() {
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
                    });
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
                self.state.has_search_settled = true;
                if page <= 1 {
                    self.state.search_results.clear();
                    self.state.search_list_state.select(None);
                    self.state.search_preview = None;
                    self.state.clear_poster_cache();
                }
                self.state.search_error = Some(err.clone());
                self.state
                    .set_status_default(format!("Search failed: {}", err));
            }

            Action::HomepageSuccess {
                request_id,
                tab_id,
                page,
                items,
                metrics,
            } => {
                if request_id != self.state.active_homepage_request {
                    return None;
                }
                if !self.state.is_homepage_mode || self.state.current_tab_id != tab_id {
                    return None;
                }
                self.state.is_loading = false;
                self.state.has_search_settled = true;
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
                self.state
                    .homepage_cache
                    .put((tab_id.clone(), page), (items.clone(), metrics.clone()));
                let count = self.append_homepage_items(items, metrics);
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
                self.state.set_status_default(status);
            }

            Action::HomepageFailure(request_id, err) => {
                if request_id != self.state.active_homepage_request {
                    return None;
                }
                log::error!("discover failed: {err}");
                self.state.is_loading = false;
                self.state.has_search_settled = true;
                self.state.search_error = Some(format!("Discover failed: {err}"));
                self.state.search_results.clear();
                self.state.search_list_state.select(None);
                self.state.search_preview = None;
                self.state.clear_poster_cache();
                self.state
                    .set_status_default(format!("Discover failed: {}", err));
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
                        self.state.poster_image = Some(std::sync::Arc::clone(img));
                    } else if self.state.image_supported
                        && let Some(url) = cached.cover_url()
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
                let service = self.service.clone();
                let sender = self.action_sender.clone();
                let id_clone = id.clone();

                tokio::spawn(async move {
                    if let Ok(Some(cached_disk)) = tokio::task::spawn_blocking({
                        let id_clone = id_clone.clone();
                        move || crate::cache::get_provider_details_cache_typed(prov, &id_clone)
                    })
                    .await
                    {
                        sender
                            .send(Action::PreviewSuccess(request_id, id_clone, cached_disk))
                            .ok();
                        return;
                    }

                    match service.details_typed(prov, &id_clone).await {
                        Ok(details) => {
                            let id_save = id_clone.clone();
                            let det_save = details.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_details_cache_typed(
                                    prov, &id_save, &det_save,
                                )
                            })
                            .await;
                            sender
                                .send(Action::PreviewSuccess(request_id, id_clone, details))
                                .ok();
                        }
                        Err(e) => {
                            sender
                                .send(Action::PreviewFailure(request_id, e.user_message(prov)))
                                .ok();
                        }
                    }
                });
            }

            Action::PreviewSuccess(request_id, id, details) => {
                if request_id != self.state.active_preview_request {
                    return None;
                }
                let current_id = if self.state.active_screen == Screen::Details {
                    self.state
                        .selected_details
                        .as_ref()
                        .map(|d| d.id.value.clone())
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

                self.state.preview_cache.put(id.clone(), details.clone());
                self.state.search_preview = Some(details.clone());
                self.state.poster_image = None;
                self.state.poster_protocol = None;
                if let Some(cached_img) = self.state.image_cache.get(&id) {
                    self.state.poster_image = Some(std::sync::Arc::clone(cached_img));
                } else if self.state.image_supported
                    && let Some(url) = details.cover_url()
                {
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
                    self.state.poster_image = Some(img);
                    self.state.poster_protocol = None;
                }
            }

            Action::SearchPosterLoaded(id, img_opt) => {
                self.state.in_flight_posters.remove(&id);
                if let Some(img) = img_opt {
                    self.state
                        .image_cache
                        .put(id.clone(), std::sync::Arc::clone(&img));
                    self.state.search_posters.put(id, img);
                } else {
                    self.state.failed_posters.put(id, std::time::Instant::now());
                }
            }

            Action::PreviewFailure(request_id, err) => {
                if request_id != self.state.active_preview_request {
                    return None;
                }
                self.state.preview_loading = false;
                self.state
                    .set_status_default(format!("Preview failed: {}", err));
            }

            Action::DetailsSuccess(context, request_id, id, mut details) => {
                if request_id != self.state.active_details_request {
                    return None;
                }
                if !self.context_is_current(context) || self.state.active_screen != Screen::Details
                {
                    return None;
                }
                self.state.is_loading = false;
                self.state.details_error = None;

                if let Some(existing) = &self.state.selected_details {
                    if existing.id.value == id && existing.id.provider == details.id.provider {
                        if details.title.trim().is_empty() {
                            details.title = existing.title.clone();
                        }
                        if details.description.is_none() {
                            details.description = existing.description.clone();
                        }
                        if details.poster_url.is_none() {
                            details.poster_url = existing.poster_url.clone();
                        }
                        if details.year.is_none() {
                            details.year = existing.year.clone();
                        }
                        if details.duration.is_none() {
                            details.duration = existing.duration.clone();
                        }
                        if details.genres.is_empty() {
                            details.genres = existing.genres.clone();
                        }
                        if details.imdb_rating.is_none() {
                            details.imdb_rating = existing.imdb_rating.clone();
                        }
                        if details.director.is_none() {
                            details.director = existing.director.clone();
                        }
                        if details.stars.is_none() {
                            details.stars = existing.stars.clone();
                        }
                        if details.dubs.is_empty() {
                            details.dubs = existing.dubs.clone();
                        }
                    }
                }

                if let Some(res) = self.state.search_results.iter().find(|r| r.id == id) {
                    if details.title.trim().is_empty() {
                        details.title = res.title.clone();
                    }
                    if details.year.is_none() {
                        details.year = Some(res.release_year.clone());
                    }
                    if details.poster_url.is_none() {
                        details.poster_url = res.cover_url.clone();
                    }
                }

                self.state.active_subject_id = Some(id.clone());
                self.state.selected_details = Some(details.clone());

                if self.state.poster_image.is_none() {
                    if let Some(cached_img) = self
                        .state
                        .image_cache
                        .get(&id)
                        .or_else(|| self.state.search_posters.get(&id))
                    {
                        self.state.poster_image = Some(std::sync::Arc::clone(cached_img));
                    } else if let Some(url) = details.cover_url() {
                        self.state.history.update_cover_url(&id, url);
                        if self.state.image_supported {
                            let url_clone = url.to_string();
                            let action_tx = self.action_sender.clone();
                            let id_clone = id.clone();
                            let http_client = self.service.http_client().clone();
                            tokio::spawn(async move {
                                if let Ok(Some(bytes)) = tokio::task::spawn_blocking({
                                    let id_clone = id_clone.clone();
                                    move || {
                                        crate::cache::get_namespaced_image_cache(
                                            "posters", &id_clone,
                                        )
                                    }
                                })
                                .await
                                {
                                    if let Some(img) = network::decode_poster(bytes).await {
                                        let _ =
                                            action_tx.send(Action::PosterSuccess(id_clone, img));
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
                                        let _ =
                                            action_tx.send(Action::PosterSuccess(id_clone, img));
                                    }
                                }
                            });
                        }
                    }
                }

                if !details.seasons.is_empty() {
                    self.state.available_seasons = details.seasons.clone();
                } else if details.media_type == MediaType::Series {
                    self.state.available_seasons = vec![Season {
                        number: 1,
                        episodes: vec![Episode {
                            season: 1,
                            number: 1,
                            title: None,
                        }],
                    }];
                } else {
                    self.state.available_seasons.clear();
                }

                self.state.available_episode_numbers.clear();
                for season in &self.state.available_seasons {
                    let ep_numbers: Vec<usize> = if !season.episodes.is_empty() {
                        season.episodes.iter().map(|e| e.number).collect()
                    } else {
                        vec![1]
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
                    if history.completed {
                        let current_season_eps = self
                            .state
                            .available_seasons
                            .iter()
                            .find(|s| s.number == history.season)
                            .map(|s| s.episodes.len())
                            .unwrap_or(0);
                        if current_season_eps > 0 && history.episode >= current_season_eps {
                            default_season = history.season.saturating_add(1);
                            default_episode = 1;
                        } else {
                            default_episode = history.episode.saturating_add(1);
                        }
                    } else {
                        default_episode = history.episode;
                    }
                }

                let (target_season, target_episode) = if self.state.language_chosen {
                    (
                        if self.state.selected_season > 0 {
                            self.state.selected_season
                        } else {
                            default_season
                        },
                        if self.state.selected_episode > 0 {
                            self.state.selected_episode
                        } else {
                            default_episode
                        },
                    )
                } else {
                    (default_season, default_episode)
                };

                let season_idx = self
                    .state
                    .available_seasons
                    .iter()
                    .position(|s| s.number == target_season)
                    .unwrap_or(0);

                self.state.season_list_state.select(Some(season_idx));

                let ep_idx = self
                    .state
                    .available_episode_numbers
                    .get(season_idx)
                    .and_then(|eps| eps.iter().position(|&e| e == target_episode))
                    .unwrap_or(0);

                self.state.episode_list_state.select(Some(ep_idx));

                if !details.dubs.is_empty() {
                    let find_by_pattern = |patterns: &[&str]| {
                        details.dubs.iter().position(|dub| {
                            let lower =
                                format!("{} {}", dub.language, dub.label).to_ascii_lowercase();
                            patterns.iter().any(|pat| lower.contains(pat))
                        })
                    };

                    let preferred_idx = if self.state.language_chosen {
                        details
                            .dubs
                            .iter()
                            .position(|dub| dub.subject_id == id)
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

                let has_multiple_dubs = details.has_languages();

                if has_multiple_dubs
                    && !self.state.language_chosen
                    && !self.state.auto_play_on_ready
                {
                    self.state.details_pane = crate::tui::state::DetailsPane::Languages;
                    self.state.is_loading = false;
                    self.state
                        .set_status_default("Please select a language dubbing.");
                } else {
                    if !self.state.language_chosen {
                        if details.is_series() && !self.state.available_seasons.is_empty() {
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
                self.state.auto_play_on_ready = false;
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
                self.state.details_error = Some(err.clone());
                self.state
                    .set_status_default(format!("Details fetch failed: {}", err));

                if self.state.active_screen == Screen::Details {
                    if let Some(id) = self.state.active_subject_id.clone().or_else(|| {
                        self.state
                            .selected_details
                            .as_ref()
                            .map(|d| d.id.value.clone())
                    }) {
                        self.state.active_subject_id = Some(id.clone());

                        if self.state.poster_image.is_none() {
                            if let Some(cached_img) = self
                                .state
                                .image_cache
                                .get(&id)
                                .or_else(|| self.state.search_posters.get(&id))
                            {
                                self.state.poster_image = Some(std::sync::Arc::clone(cached_img));
                            } else if self.state.image_supported
                                && let Some(details) = &self.state.selected_details
                            {
                                if let Some(url) = details.cover_url() {
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
                self.state.is_loading = true;
                self.state.is_fetching_streams = true;
                self.state.has_streams_settled = false;
                self.state.stream_error = None;
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

                let is_series = self
                    .state
                    .selected_details
                    .as_ref()
                    .is_some_and(|d| d.is_series());

                let (se, ep) = if is_series {
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
                };

                self.state.selected_season = se;
                self.state.selected_episode = ep;

                let already_loaded = !self.state.selected_resources.is_empty();
                if already_loaded {
                    if let Some(pool) = self.state.stream_pool.get_mut(&subject_id) {
                        pool.episode_index
                            .insert((se, ep), self.state.selected_resources.clone());
                    }
                    self.state.is_loading = false;
                    self.state.is_fetching_streams = false;
                    if self.state.auto_play_on_ready {
                        self.state.auto_play_on_ready = false;
                        self.action_sender.send(Action::PlayStream).ok();
                    }
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
                self.state.has_streams_settled = false;
                self.state.selected_resources.clear();
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
                        crate::cache::get_provider_stream_cache_typed(
                            prov, &id_clone, season, episode,
                        )
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
                                    cached,
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
                        .map(|d| d.is_series())
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
                                    "No streaming addons are currently installed or enabled.\nOpen /settings to install/enable a stream provider.".into(),
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
                            let id_clone = id.clone();
                            let releases_clone = releases.clone();
                            let provider = context.provider;
                            tokio::task::spawn_blocking(move || {
                                crate::cache::set_provider_stream_cache_typed(
                                    provider,
                                    &id_clone,
                                    season,
                                    episode,
                                    &releases_clone,
                                );
                            });

                            sender
                                .send(Action::EpisodeStreamsReady(
                                    context, request_id, id, season, episode, releases,
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
                                    Err(crate::providers::models::ProviderError::Unavailable(
                                        "4KHDHub provider is unavailable".to_string(),
                                    ))
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
                                let id_clone = id.clone();
                                let releases_clone = releases.clone();
                                let provider = context.provider;
                                tokio::task::spawn_blocking(move || {
                                    crate::cache::set_provider_stream_cache_typed(
                                        provider,
                                        &id_clone,
                                        season,
                                        episode,
                                        &releases_clone,
                                    );
                                });

                                sender
                                    .send(Action::EpisodeStreamsReady(
                                        context, request_id, id, season, episode, releases,
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
                                    cached,
                                ))
                                .ok();
                        });
                        return None;
                    }
                }

                let mut absolute_episode = 0;
                for s_val in &self.state.available_seasons {
                    if s_val.number < season {
                        absolute_episode += s_val.episodes.len().max(1);
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

                self.request_tasks.cancel_streams();
                self.request_tasks.streams = Some(tokio::spawn(async move {
                    sender
                        .send(Action::SetStatus("Fetching streams...".to_string()))
                        .ok();

                    if let Ok(streams) = crate::providers::ReleaseProvider::episode_streams(
                        &client, &id_clone, season, episode,
                    )
                    .await
                    {
                        if !streams.is_empty() {
                            sender
                                .send(Action::EpisodeStreamsReady(
                                    context, request_id, id_clone, season, episode, streams,
                                ))
                                .ok();
                            return;
                        }
                    }

                    let mut all_items: Vec<Release> = Vec::new();
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
                                    for item in items {
                                        all_items.push(
                                            crate::providers::moviebox::adapt::moviebox_resource_item_to_release(
                                                &item,
                                            ),
                                        );
                                    }
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
                                    for item in items {
                                        let release =
                                            crate::providers::moviebox::adapt::moviebox_resource_item_to_release(
                                                &item,
                                            );
                                        if release.season == Some(season)
                                            && release.episode == Some(episode)
                                        {
                                            found_target = true;
                                        }
                                        all_items.push(release);
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
                                context, request_id, id_clone, season, episode, all_items,
                            ))
                            .ok();
                    }
                }));
            }

            Action::EpisodeStreamsReady(
                context,
                request_id,
                subject_id,
                target_se,
                target_ep,
                mut raw_list,
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

                if let Some(subject_id) = &self.state.active_subject_id {
                    let id = subject_id.clone();
                    if let Some(pool) = self.state.stream_pool.get_mut(&id) {
                        let mut actual_resolutions = std::collections::HashSet::new();

                        for mut item in raw_list.clone() {
                            let r = item.resolution_u64();
                            if r > 0 {
                                actual_resolutions.insert(r as u32);
                            }

                            let mut se = item.season.unwrap_or(0);
                            let mut ep = item.episode.unwrap_or(0);

                            if target_se == 0 && target_ep == 0 {
                                se = 0;
                                ep = 0;
                            } else if se == 0 && ep == 0 {
                                se = target_se;
                                ep = target_ep;
                            }

                            item.season = Some(se);
                            item.episode = Some(ep);

                            let entry = pool.episode_index.entry((se, ep)).or_default();
                            let link = item.direct_url().unwrap_or("");

                            let mut exists = false;
                            for i in entry.iter_mut() {
                                let i_link = i.direct_url().unwrap_or("");
                                let base_link = link.split('?').next().unwrap_or(link);
                                let i_base_link = i_link.split('?').next().unwrap_or(i_link);

                                if (!base_link.is_empty() && base_link == i_base_link)
                                    || (!item.filename.is_empty() && item.filename == i.filename)
                                {
                                    if !item.mirrors.is_empty() && i.mirrors.is_empty() {
                                        i.mirrors = item.mirrors.clone();
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
                filtered.sort_by_key(|b| std::cmp::Reverse(b.resolution_u64()));

                let count = filtered.len();
                if count > 0 {
                    if let Some(subject_id) = &self.state.active_subject_id {
                        let id_clone = subject_id.clone();
                        let filtered_clone = filtered.clone();
                        tokio::task::spawn_blocking(move || {
                            crate::cache::set_provider_stream_cache_typed(
                                context.provider,
                                &id_clone,
                                target_se,
                                target_ep,
                                &filtered_clone,
                            );
                        });
                    }
                }

                self.state.selected_resources = filtered;
                self.state.is_loading = false;
                self.state.is_fetching_streams = false;
                self.state.has_streams_settled = true;
                self.state.stream_error = None;
                self.state
                    .resource_list_state
                    .select(if count > 0 { Some(0) } else { None });
                self.state
                    .set_status_default(format!("{} streams available.", count));

                if count > 0 {
                    if let Some(rid) = self
                        .state
                        .selected_resources
                        .first()
                        .and_then(|r| r.resource_id.clone())
                    {
                        let subject_id = self.state.active_subject_id.clone().unwrap_or_default();
                        let service = self.service.clone();
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
                        tokio::spawn(async move {
                            let already_cached = tokio::task::spawn_blocking({
                                let subject_id = subject_id.clone();
                                let rid = rid.clone();
                                move || {
                                    crate::cache::get_captions_cache_typed(&subject_id, &rid)
                                        .is_some()
                                }
                            })
                            .await
                            .unwrap_or(false);

                            if !already_cached {
                                if let Ok(res) = service
                                    .get_ext_captions(&subject_id, &rid, &sibling_ids)
                                    .await
                                {
                                    tokio::task::spawn_blocking(move || {
                                        crate::cache::set_captions_cache_typed(
                                            &subject_id,
                                            &rid,
                                            &res,
                                        );
                                    });
                                }
                            }
                        });
                    }
                }

                if self.state.is_waiting_for_download_stream {
                    self.state.is_waiting_for_download_stream = false;

                    let is_season_queue = self.state.download_queue_total > 0;
                    if is_season_queue {
                        let subject_id = self.state.active_subject_id.clone().unwrap_or_default();
                        if let Some(rid) = self.get_selected_resource_id() {
                            let service = self.service.clone();
                            let sender = self.action_sender.clone();
                            let pref = self.state.season_subtitle_preference.clone();
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

                            tokio::spawn(async move {
                                if let Ok(res) = service
                                    .get_ext_captions(&subject_id, &rid, &sibling_ids)
                                    .await
                                {
                                    if pref.is_none() {
                                        sender.send(Action::ShowDownloadSubtitlePopup(res)).ok();
                                    } else if let Some(pref_lang) = pref.flatten() {
                                        let sub_url = res
                                            .into_iter()
                                            .find(|s| s.name == pref_lang)
                                            .map(|s| s.url);
                                        sender.send(Action::DownloadStream(sub_url)).ok();
                                    } else {
                                        sender.send(Action::DownloadStream(None)).ok();
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
                if self.state.auto_play_on_ready {
                    self.state.auto_play_on_ready = false;
                    self.action_sender.send(Action::PlayStream).ok();
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
                self.state.auto_play_on_ready = false;
                self.state.is_loading = false;
                self.state.is_fetching_streams = false;
                self.state.has_streams_settled = true;
                self.state.selected_resources.clear();
                self.state.resource_list_state.select(None);
                self.state.stream_error = Some(err.clone());
                log::error!(
                    "episode streams failed ({} s{}e{}): {err}",
                    context.provider.cache_key(),
                    target_se,
                    target_ep
                );
                self.state.notify(
                    crate::tui::overlay::NotificationKind::Error,
                    "Streams Failed",
                    err,
                );
            }
            _ => return None,
        }
        None
    }
}
