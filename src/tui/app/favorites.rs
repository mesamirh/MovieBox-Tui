use super::App;
use crate::providers::models::ProviderKind;
use crate::tui::{
    action::Action,
    overlay::NotificationKind,
    state::{InputMode, Screen, SearchResult},
};

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl App {
    pub(super) async fn handle_favorites(&mut self, action: Action) -> Option<()> {
        match action {
            Action::ToggleFavorite => {
                self.toggle_current_favorite();
            }
            Action::ShowFavorites => {
                self.load_favorites_virtual_list();
            }
            Action::OpenFavorite(index) => {
                self.open_favorite(index);
            }
            _ => return None,
        }
        None
    }

    fn current_favorite_candidate(&self) -> Option<crate::favorites::FavoriteItem> {
        match self.state.active_screen {
            Screen::Details => {
                let subject_id = self.state.active_subject_id.clone()?;
                let details = self.state.selected_details.as_ref()?;
                let provider = self.current_subject_provider().cache_key().to_string();
                let raw_title = details
                    .get("title")
                    .or_else(|| details.get("name"))
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                let title = crate::providers::moviebox::clean_moviebox_title(raw_title);
                let stype = crate::tui::state::stype(details);
                let release_year = details
                    .get("releaseDate")
                    .or_else(|| details.get("year"))
                    .or_else(|| details.get("releaseInfo"))
                    .and_then(|y| y.as_str())
                    .unwrap_or_default()
                    .to_string();
                let cover_url = details
                    .get("cover")
                    .and_then(|c| c.get("url"))
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string());
                Some(crate::favorites::FavoriteItem {
                    provider,
                    subject_id,
                    title,
                    cover_url,
                    stype,
                    release_year,
                    added_at: now_secs(),
                })
            }
            Screen::Home => {
                if self.state.favorites_focus {
                    let idx = self.state.favorites_landing_state.selected()?;
                    self.state
                        .favorites_landing_items()
                        .get(idx)
                        .map(|item| (*item).clone())
                } else {
                    let idx = self.state.search_list_state.selected()?;
                    let res = self.state.search_results.get(idx)?;
                    if res.stype == 3 {
                        return None;
                    }
                    Some(crate::favorites::FavoriteItem {
                        provider: res.provider.cache_key().to_string(),
                        subject_id: res.id.clone(),
                        title: res.title.clone(),
                        cover_url: res.cover_url.clone(),
                        stype: res.stype,
                        release_year: res.release_year.clone(),
                        added_at: now_secs(),
                    })
                }
            }
        }
    }

    fn toggle_current_favorite(&mut self) {
        if !self.state.favorites_available() {
            return;
        }
        let Some(candidate) = self.current_favorite_candidate() else {
            return;
        };
        let title = candidate.title.clone();
        let now_favorited = self.state.favorites.toggle(candidate);

        if self.state.favorites_focus && !now_favorited {
            let remaining = self.state.favorites_landing_items().len();
            if remaining == 0 {
                self.state.favorites_focus = false;
                self.state.favorites_landing_state.select(None);
            } else {
                let selected = self
                    .state
                    .favorites_landing_state
                    .selected()
                    .unwrap_or(0)
                    .min(remaining - 1);
                self.state.favorites_landing_state.select(Some(selected));
            }
        }

        if self
            .state
            .search_query
            .trim()
            .eq_ignore_ascii_case("/favorites")
        {
            self.load_favorites_virtual_list();
        }

        self.state.notify(
            if now_favorited {
                NotificationKind::Success
            } else {
                NotificationKind::Info
            },
            "Favorites",
            if now_favorited {
                format!("Added \"{title}\" to Favorites.")
            } else {
                format!("Removed \"{title}\" from Favorites.")
            },
        );
    }

    fn open_favorite(&mut self, index: usize) {
        let Some(item) = self
            .state
            .favorites_landing_items()
            .get(index)
            .map(|item| (*item).clone())
        else {
            return;
        };
        let subject_id = item.subject_id.clone();
        let title = item.title.clone();

        self.state.active_screen = Screen::Details;
        self.state.active_subject_id = Some(subject_id.clone());
        self.state.selected_details = Some(serde_json::json!({
            "id": subject_id.clone(),
            "subjectId": subject_id.clone(),
            "title": item.title,
            "subjectType": item.stype,
            "releaseDate": item.release_year,
            "cover": { "url": item.cover_url },
        }));
        self.state.selected_resources = None;
        self.state.is_loading = true;
        self.state.is_fetching_streams = false;
        self.state.stream_error = None;
        self.state.resource_list_state.select(None);
        self.state.language_list_state.select(Some(0));
        self.state.season_list_state.select(Some(0));
        self.state.episode_list_state.select(Some(0));
        self.state.language_chosen = false;
        self.state.available_seasons.clear();
        if let Some(cached) = self
            .state
            .image_cache
            .get(&subject_id)
            .or_else(|| self.state.search_posters.get(&subject_id))
        {
            self.state.poster_image = Some((**cached).clone());
        } else {
            self.state.poster_image = None;
        }
        self.state
            .set_status(format!("Loading details for {title}..."), 150);

        self.action_sender
            .send(Action::FetchDetails(subject_id, false))
            .ok();
    }

    pub(super) fn load_favorites_virtual_list(&mut self) {
        self.state.input_mode = InputMode::Normal;
        self.state.is_loading = false;
        self.state.is_homepage_mode = false;
        self.state.active_browse_preset = None;
        self.state.browse_metrics.clear();
        self.state.active_screen = Screen::Home;
        self.state.active_subject_id = None;
        self.state.active_preview_request = self.state.active_preview_request.wrapping_add(1);
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
        self.state.favorites_focus = false;

        self.state.search_query = "/favorites".to_string();
        let mut items = self.state.favorites.items.clone();
        if items.is_empty() {
            self.state.notify(
                NotificationKind::Info,
                "Favorites",
                "No favorites yet. Star a title with * to add one.",
            );
        } else {
            items.sort_by_key(|item| std::cmp::Reverse(item.added_at));

            for item in items {
                let provider = ProviderKind::parse(&item.provider).unwrap_or_else(|| {
                    log::warn!(
                        "unknown favorites provider '{}'; defaulting to MovieBox",
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
                    season: 0,
                    episode: 0,
                    provider,
                });
            }

            self.state.search_list_state.select(Some(0));
            self.prefetch_visible_posters();
        }
    }
}
