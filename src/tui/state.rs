use crate::providers::models::ProviderKind;
use ratatui::widgets::{ListState, TableState};

pub use crate::player::PlayerKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Details,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DetailsPane {
    #[default]
    Streams,
    Seasons,
    Episodes,
    Languages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Streaming,
    Tv,
    Addon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub use crate::models::{
    BrowseMetric, BrowseMetrics, BrowsePreset, Notification, NotificationKind, SearchResult,
    SubjectStreamPool,
};
pub use crate::service::{caption_options, caption_url_for, stype, subject_id};

pub struct AppState {
    pub active_provider: ProviderKind,
    pub provider_generation: u64,
    pub active_screen: Screen,
    pub dirty: bool,
    pub input_mode: InputMode,
    pub search_query: String,
    pub last_suggest_query: String,
    pub last_search_edit: std::time::Instant,
    pub search_suggestions: Vec<String>,
    pub suggest_index: Option<usize>,
    pub search_results: Vec<SearchResult>,
    pub search_error: Option<String>,
    pub is_homepage_mode: bool,
    pub current_tab_id: String,
    pub current_page: usize,
    pub search_posters: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,
    pub failed_posters: lru::LruCache<String, ()>,
    pub search_poster_protocols:
        lru::LruCache<String, ((u16, u16), ratatui_image::protocol::Protocol)>,
    pub in_flight_posters: std::collections::HashSet<String>,
    pub search_list_state: TableState,

    pub selected_details: Option<serde_json::Value>,
    pub active_subject_id: Option<String>,
    pub selected_resources: Option<serde_json::Value>,
    pub stream_pool: std::collections::HashMap<String, SubjectStreamPool>,
    pub fetch_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub show_season_download_confirm: bool,
    pub season_download_confirm_yes_selected: bool,
    pub show_episode_download_confirm: bool,
    pub episode_download_confirm_yes_selected: bool,
    pub is_waiting_for_download_stream: bool,
    pub is_fetching_streams: bool,
    pub stream_error: Option<String>,
    pub preview_cache: lru::LruCache<String, serde_json::Value>,
    pub resource_list_state: ListState,

    pub details_pane: DetailsPane,
    pub selected_season: usize,
    pub selected_episode: usize,
    pub season_list_state: ListState,
    pub episode_list_state: ListState,
    pub language_list_state: ListState,
    pub available_seasons: Vec<serde_json::Value>,
    pub available_episode_numbers: Vec<Vec<usize>>,

    pub search_preview: Option<serde_json::Value>,
    pub preview_loading: bool,

    pub tick_count: u64,
    pub poster_image: Option<image::DynamicImage>,

    pub show_theme_popup: bool,
    pub active_theme_kind: String,
    pub original_theme_kind: Option<String>,
    pub theme_list_state: ListState,
    pub show_browse_popup: bool,
    pub browse_list_state: ListState,
    pub active_browse_preset: Option<BrowsePreset>,
    pub active_addon_catalog: Option<crate::providers::addons::models::AddonCatalogTarget>,
    pub browse_metrics: std::collections::HashMap<String, BrowseMetrics>,

    pub poster_protocol: Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub image_picker: Option<ratatui_image::picker::Picker>,
    pub image_supported: bool,
    pub clear_terminal_before_draw: bool,
    pub poster_rows: u16,
    pub image_cache: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,

    pub show_help: bool,
    pub visible_items: usize,

    pub active_resource_request: u64,
    pub active_search_request: u64,
    pub active_homepage_request: u64,
    pub active_details_request: u64,
    pub active_preview_request: u64,
    pub active_suggest_request: u64,
    pub pending_episode_fetch: Option<(String, usize, usize)>,
    pub last_episode_nav: std::time::Instant,
    pub last_resize_time: Option<(std::time::Instant, u16, u16)>,
    pub player_picker_popup: bool,
    pub player_picker_state: ListState,
    pub player_picker_link: Option<String>,
    pub player_picker_subtitle: Option<String>,
    pub player_picker_playback: Option<crate::providers::models::PlaybackSource>,
    pub available_players: Vec<PlayerKind>,
    pub default_player: Option<String>,
    pub is_loading: bool,
    pub is_resolving_playback: bool,
    pub is_playing: bool,
    pub last_playback_launch: std::time::Instant,
    pub status_message: String,
    pub status_timer: usize,
    pub notifications: std::collections::VecDeque<crate::tui::overlay::Notification>,
    pub update_available: Option<(String, String)>,
    pub auto_update: bool,
    pub last_update_check: u64,
    pub manual_update_check: bool,
    pub is_checking_updates: bool,
    pub is_updating: bool,
    pub update_release: Option<crate::updater::Release>,

    pub download_progress: Option<f64>,
    pub download_status: Option<String>,
    pub cancel_download: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub download_dir: Option<std::path::PathBuf>,

    pub download_queue: std::collections::VecDeque<(usize, usize)>,
    pub download_queue_total: usize,

    pub language_chosen: bool,

    pub subtitle_popup: bool,
    pub is_download_subtitle_popup: bool,
    pub season_subtitle_preference: Option<String>,
    pub last_download_subtitle_language: Option<String>,
    pub subtitle_list: Vec<(String, String)>,
    pub subtitle_list_state: ListState,
    pub pending_play_link: Option<String>,
    pub pending_open_with: bool,
    pub basic_terminal: bool,
    pub bdix_enabled: bool,
    pub streaming_enabled: bool,

    pub is_tv_mode: bool,
    pub tv_enabled: bool,
    pub tv_config_popup: bool,
    pub tv_channels: Vec<crate::providers::tv::Channel>,
    pub tv_playlists: Vec<String>,
    pub tv_manager_selected: usize,
    pub tv_input_active: bool,
    pub tv_input_buffer: String,
    pub tv_input_is_file: bool,

    pub is_addon_mode: bool,
    pub addons_enabled: bool,
    pub installed_addons: Vec<crate::providers::addons::models::InstalledAddon>,
    pub addon_manager_popup: bool,
    pub addon_manager_selected: usize,
    pub addon_input_active: bool,
    pub addon_input_buffer: String,
    pub addon_input_cursor: usize,
    pub history: crate::history::HistoryManager,
    pub favorites: crate::favorites::FavoritesManager,
    pub favorites_focus: bool,
    pub favorites_landing_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::MovieBox,
            provider_generation: 0,
            active_screen: Screen::Home,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            last_suggest_query: String::new(),
            last_search_edit: std::time::Instant::now(),
            search_suggestions: Vec::new(),
            suggest_index: None,
            search_results: Vec::new(),
            search_error: None,
            is_homepage_mode: false,
            current_tab_id: String::new(),
            current_page: 1,
            search_posters: lru::LruCache::new(cache_capacity(300)),
            failed_posters: lru::LruCache::new(cache_capacity(300)),
            search_poster_protocols: lru::LruCache::new(cache_capacity(300)),
            in_flight_posters: std::collections::HashSet::new(),
            search_list_state: TableState::default(),
            basic_terminal: crate::tui::terminal::uses_basic_ui(),
            selected_details: None,
            active_subject_id: None,
            selected_resources: None,
            stream_pool: std::collections::HashMap::new(),
            fetch_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            show_season_download_confirm: false,
            season_download_confirm_yes_selected: false,
            show_episode_download_confirm: false,
            episode_download_confirm_yes_selected: false,
            is_waiting_for_download_stream: false,
            is_fetching_streams: false,
            stream_error: None,
            preview_cache: lru::LruCache::new(cache_capacity(30)),
            resource_list_state: ListState::default(),

            details_pane: DetailsPane::default(),
            selected_season: 1,
            selected_episode: 1,
            season_list_state: ListState::default(),
            episode_list_state: ListState::default(),
            language_list_state: ListState::default(),
            available_seasons: vec![],
            available_episode_numbers: vec![],

            search_preview: None,
            preview_loading: false,
            tick_count: 0,
            poster_image: None,
            active_theme_kind: String::new(),
            original_theme_kind: None,
            show_theme_popup: false,
            theme_list_state: ListState::default(),
            show_browse_popup: false,
            browse_list_state: ListState::default(),
            active_browse_preset: None,
            active_addon_catalog: None,
            browse_metrics: std::collections::HashMap::new(),
            poster_protocol: None,
            image_picker: None,
            image_supported: crate::tui::terminal::should_query_images(),
            clear_terminal_before_draw: false,
            poster_rows: 3,
            image_cache: lru::LruCache::new(cache_capacity(10)),
            show_help: false,
            visible_items: 10,
            active_resource_request: 0,
            active_search_request: 0,
            active_homepage_request: 0,
            active_details_request: 0,
            active_preview_request: 0,
            active_suggest_request: 0,
            pending_episode_fetch: None,
            last_episode_nav: std::time::Instant::now(),
            last_resize_time: None,
            player_picker_popup: false,
            player_picker_state: ListState::default(),
            player_picker_link: None,
            player_picker_subtitle: None,
            player_picker_playback: None,
            available_players: Vec::new(),
            default_player: None,
            dirty: true,
            is_loading: false,
            is_resolving_playback: false,
            is_playing: false,
            last_playback_launch: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(5))
                .unwrap_or_else(std::time::Instant::now),
            status_message: String::new(),
            status_timer: 0,
            notifications: std::collections::VecDeque::new(),
            update_available: None,
            auto_update: true,
            last_update_check: 0,
            manual_update_check: false,
            is_checking_updates: false,
            is_updating: false,
            update_release: None,

            download_progress: None,
            download_status: None,
            cancel_download: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            download_dir: None,
            download_queue: std::collections::VecDeque::new(),
            download_queue_total: 0,
            language_chosen: false,

            subtitle_popup: false,
            is_download_subtitle_popup: false,
            season_subtitle_preference: None,
            last_download_subtitle_language: None,
            subtitle_list: Vec::new(),
            subtitle_list_state: ListState::default(),
            pending_play_link: None,
            pending_open_with: false,
            bdix_enabled: false,
            streaming_enabled: true,
            is_tv_mode: false,
            tv_enabled: true,
            tv_config_popup: false,
            tv_channels: Vec::new(),
            tv_playlists: Vec::new(),
            tv_manager_selected: 0,
            tv_input_active: false,
            tv_input_buffer: String::new(),
            tv_input_is_file: false,
            is_addon_mode: false,
            addons_enabled: false,
            installed_addons: Vec::new(),
            addon_manager_popup: false,
            addon_manager_selected: 0,
            addon_input_active: false,
            addon_input_buffer: String::new(),
            addon_input_cursor: 0,
            history: crate::history::HistoryManager::new(),
            favorites: crate::favorites::FavoritesManager::new(),
            favorites_focus: false,
            favorites_landing_state: ListState::default(),
        }
    }
}

const fn cache_capacity(n: usize) -> std::num::NonZeroUsize {
    match std::num::NonZeroUsize::new(n) {
        Some(value) => value,
        None => std::num::NonZeroUsize::MIN,
    }
}

impl AppState {
    pub fn mode(&self) -> AppMode {
        if self.is_tv_mode && !self.is_addon_mode {
            AppMode::Tv
        } else if self.is_addon_mode && !self.is_tv_mode {
            AppMode::Addon
        } else {
            AppMode::Streaming
        }
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        match mode {
            AppMode::Streaming => {
                self.is_tv_mode = false;
                self.is_addon_mode = false;
            }
            AppMode::Tv => {
                self.is_tv_mode = true;
                self.is_addon_mode = false;
            }
            AppMode::Addon => {
                self.is_tv_mode = false;
                self.is_addon_mode = true;
            }
        }
    }

    pub fn notify(
        &mut self,
        kind: crate::tui::overlay::NotificationKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) {
        if self.notifications.len() >= 3 {
            let removable = self
                .notifications
                .iter()
                .position(|notification| {
                    notification.kind != crate::tui::overlay::NotificationKind::Error
                })
                .unwrap_or(0);
            self.notifications.remove(removable);
        }
        self.notifications
            .push_back(crate::tui::overlay::Notification::new(kind, title, message));
    }

    pub fn expire_notifications(&mut self) {
        self.notifications
            .retain(|notification| !notification.expired());
    }

    pub fn set_status(&mut self, message: impl Into<String>, timer: usize) {
        self.status_message = message.into();
        self.status_timer = timer;
    }

    pub fn clear_search_state(&mut self) {
        self.search_query.clear();
        self.search_results.clear();
        self.search_error = None;
        self.search_suggestions.clear();
        self.suggest_index = None;
        self.search_preview = None;
        self.preview_loading = false;
        self.active_browse_preset = None;
        self.active_addon_catalog = None;
        self.browse_metrics.clear();
        self.poster_image = None;
        self.poster_protocol = None;
        self.failed_posters.clear();
        self.in_flight_posters.clear();
        self.search_list_state.select(None);
        self.is_homepage_mode = false;
        self.favorites_focus = false;
        self.favorites_landing_state.select(None);
    }

    pub fn clear_details_state(&mut self) {
        self.active_subject_id = None;
        self.selected_details = None;
        self.selected_resources = None;
        self.is_fetching_streams = false;
        self.pending_episode_fetch = None;
        self.stream_error = None;
        self.available_seasons.clear();
        self.available_episode_numbers.clear();
        self.season_list_state.select(None);
        self.episode_list_state.select(None);
        self.resource_list_state.select(None);
        self.language_list_state.select(None);
        self.details_pane = DetailsPane::Streams;
    }

    /// Same availability predicate `/history` and `/browse` already use:
    /// Streaming and Addon modes only.
    pub fn favorites_available(&self) -> bool {
        (self.streaming_enabled && !self.is_tv_mode && !self.is_addon_mode)
            || (self.addons_enabled && self.is_addon_mode)
    }

    pub fn favorites_landing_visible(&self) -> bool {
        self.favorites_available() && !self.favorites.items.is_empty()
    }

    /// Up to 5 most-recently-favorited titles, newest first. This is the
    /// fixed set the landing row navigates over; the rest are reachable via
    /// `/favorites`.
    pub fn favorites_landing_items(&self) -> Vec<&crate::favorites::FavoriteItem> {
        let mut items: Vec<&crate::favorites::FavoriteItem> = self.favorites.items.iter().collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.added_at));
        items.truncate(5);
        items
    }

    pub fn loading_dots(&self) -> &'static str {
        match (self.tick_count / 4) % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        }
    }
}

pub fn cycle_list_selection(state: &mut ListState, total_items: usize, forward: bool) {
    if total_items == 0 {
        state.select(None);
        return;
    }
    let max = total_items.saturating_sub(1);
    let next = if forward {
        match state.selected() {
            Some(i) if i >= max => 0,
            Some(i) => i + 1,
            None => 0,
        }
    } else {
        match state.selected() {
            Some(0) | None => max,
            Some(i) => i.saturating_sub(1),
        }
    };
    state.select(Some(next));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvManagerRow {
    Header(&'static str),
    Playlist(usize),
    AddUrl,
    AddFile,
    Reload,
    Done,
}

fn playlist_is_url(source: &str) -> bool {
    crate::tui::text::is_http_url(source)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonManagerRow {
    Header(&'static str),
    Addon(usize),
    AddUrl,
}

impl AppState {
    pub fn tv_manager_rows(&self) -> Vec<TvManagerRow> {
        let mut rows = vec![TvManagerRow::Header("URL playlists")];
        for (index, source) in self.tv_playlists.iter().enumerate() {
            if playlist_is_url(source) {
                rows.push(TvManagerRow::Playlist(index));
            }
        }
        rows.push(TvManagerRow::AddUrl);
        rows.push(TvManagerRow::Header("File playlists"));
        for (index, source) in self.tv_playlists.iter().enumerate() {
            if !playlist_is_url(source) {
                rows.push(TvManagerRow::Playlist(index));
            }
        }
        rows.push(TvManagerRow::AddFile);
        rows.push(TvManagerRow::Reload);
        rows.push(TvManagerRow::Done);
        rows
    }

    pub fn addon_manager_rows(&self) -> Vec<AddonManagerRow> {
        let mut rows = vec![AddonManagerRow::Header("Installed Addons")];
        for index in 0..self.installed_addons.len() {
            rows.push(AddonManagerRow::Addon(index));
        }
        rows.push(AddonManagerRow::AddUrl);
        rows
    }
}
