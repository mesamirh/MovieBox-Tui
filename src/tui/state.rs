use crate::providers::models::ProviderKind;
use ratatui::widgets::{ListState, TableState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Mpv,
    Iina,
    Vlc,
    AndroidIntent,
}

impl PlayerKind {
    pub fn label(&self) -> &'static str {
        match self {
            PlayerKind::Mpv => "mpv",
            PlayerKind::Iina => "IINA",
            PlayerKind::Vlc => "VLC",
            PlayerKind::AndroidIntent => "Android Player",
        }
    }

    pub fn parse(value: &str) -> Option<PlayerKind> {
        match value.to_ascii_lowercase().as_str() {
            "mpv" => Some(PlayerKind::Mpv),
            "iina" => Some(PlayerKind::Iina),
            "vlc" => Some(PlayerKind::Vlc),
            "android" | "androidintent" | "android-intent" => Some(PlayerKind::AndroidIntent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Startup,
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
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub stype: i64,
    pub release_year: String,
    pub cover_url: Option<String>,
    pub season: usize,
    pub episode: usize,
    pub provider: crate::providers::models::ProviderKind,
    /// IMDb rating badge shown on browse result rows (e.g. "8.4").
    pub imdb_rating: Option<String>,
}

#[derive(Debug, Default)]
pub struct SubjectStreamPool {
    pub episode_index: std::collections::HashMap<(usize, usize), Vec<serde_json::Value>>,
    pub fetched_pages: std::collections::HashMap<u32, std::collections::HashSet<usize>>,
    pub total_pages: std::collections::HashMap<u32, usize>,
    pub available_resolutions: Vec<u32>,
}

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
    pub search_poster_protocols:
        lru::LruCache<String, ((u16, u16), ratatui_image::protocol::Protocol)>,
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
    pub theme_list_state: ListState,

    pub poster_protocol: Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub image_picker: Option<ratatui_image::picker::Picker>,
    pub image_supported: bool,
    pub clear_terminal_before_draw: bool,
    pub poster_rows: u16,
    pub image_cache: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,

    pub show_help: bool,
    pub visible_items: usize,

    pub active_resource_request: u64,
    pub pending_episode_fetch: Option<(String, usize, usize)>,
    pub last_episode_nav: std::time::Instant,
    pub player_picker_popup: bool,
    pub player_picker_state: ListState,
    pub player_picker_link: Option<String>,
    pub player_picker_subtitle: Option<String>,
    pub player_picker_playback: Option<crate::providers::models::PlaybackSource>,
    pub available_players: Vec<PlayerKind>,
    pub default_player: Option<String>,
    pub is_loading: bool,
    pub is_resolving_playback: bool,
    pub status_message: String,
    pub status_timer: usize,
    pub notifications: std::collections::VecDeque<crate::tui::overlay::Notification>,
    pub update_available: Option<(String, String)>,
    pub auto_update: bool,
    pub last_update_check: u64,
    pub manual_update_check: bool,

    pub download_progress: Option<f64>,
    pub download_status: Option<String>,
    pub cancel_download: std::sync::Arc<std::sync::atomic::AtomicBool>,

    pub download_queue: std::collections::VecDeque<(usize, usize)>,
    pub download_queue_total: usize,

    pub language_chosen: bool,

    pub subtitle_popup: bool,
    pub is_download_subtitle_popup: bool,
    pub season_subtitle_preference: Option<String>,
    pub subtitle_list: Vec<(String, String)>,
    pub subtitle_list_state: ListState,
    pub pending_play_link: Option<String>,
    pub pending_open_with: bool,
    pub basic_terminal: bool,
    pub bdix_enabled: bool,

    pub is_tv_mode: bool,
    pub tv_config_popup: bool,
    pub browse_view: Option<crate::providers::browse::BrowseView>,
    pub browse_sort: crate::providers::browse::SortOrder,
    pub browse_menu_open: bool,
    pub browse_list_state: ListState,
    pub active_browse_request: u64,
    pub tv_channels: Vec<crate::providers::m3u::Channel>,
    pub tv_playlists: Vec<String>,
    pub tv_manager_selected: usize,
    pub tv_input_active: bool,
    pub tv_input_buffer: String,
    pub tv_input_is_file: bool,
    pub history: crate::history::HistoryManager,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::MovieBox,
            provider_generation: 0,
            active_screen: Screen::Startup,
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
            search_posters: lru::LruCache::new(std::num::NonZeroUsize::new(30).unwrap()),
            search_poster_protocols: lru::LruCache::new(std::num::NonZeroUsize::new(30).unwrap()),
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
            preview_cache: lru::LruCache::new(std::num::NonZeroUsize::new(30).unwrap()),
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
            show_theme_popup: false,
            theme_list_state: ListState::default(),
            poster_protocol: None,
            image_picker: None,
            image_supported: crate::tui::terminal::should_query_images(),
            clear_terminal_before_draw: false,
            poster_rows: 3,
            image_cache: lru::LruCache::new(std::num::NonZeroUsize::new(10).unwrap()),
            show_help: false,
            visible_items: 10,
            active_resource_request: 0,
            pending_episode_fetch: None,
            last_episode_nav: std::time::Instant::now(),
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
            status_message: String::new(),
            status_timer: 0,
            notifications: std::collections::VecDeque::new(),
            update_available: None,
            auto_update: true,
            last_update_check: 0,
            manual_update_check: false,

            download_progress: None,
            download_status: None,
            cancel_download: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            download_queue: std::collections::VecDeque::new(),
            download_queue_total: 0,
            language_chosen: false,

            subtitle_popup: false,
            is_download_subtitle_popup: false,
            season_subtitle_preference: None,
            subtitle_list: Vec::new(),
            subtitle_list_state: ListState::default(),
            pending_play_link: None,
            pending_open_with: false,
            bdix_enabled: false,
            is_tv_mode: false,
            tv_config_popup: false,
            browse_view: None,
            browse_sort: crate::providers::browse::SortOrder::Desc,
            browse_menu_open: false,
            browse_list_state: ListState::default(),
            active_browse_request: 0,
            tv_channels: Vec::new(),
            tv_playlists: Vec::new(),
            tv_manager_selected: 0,
            tv_input_active: false,
            tv_input_buffer: String::new(),
            tv_input_is_file: false,
            history: crate::history::HistoryManager::new(),
        }
    }
}

impl AppState {
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

    pub fn loading_dots(&self) -> &'static str {
        match (self.tick_count / 4) % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        }
    }
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
    let trimmed = source.trim_start();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
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
}

pub fn subject_id(value: &serde_json::Value) -> Option<String> {
    value
        .as_i64()
        .map(|n| n.to_string())
        .or_else(|| value.as_str().map(|s| s.to_string()))
}

pub fn stype(value: &serde_json::Value) -> i64 {
    value
        .get("subjectType")
        .and_then(|s| s.as_i64())
        .or_else(|| value.get("stype").and_then(|s| s.as_i64()))
        .unwrap_or(1)
}

pub fn caption_options(payload: &serde_json::Value) -> Vec<(String, String)> {
    let mut options = vec![("None".to_string(), "".to_string())];
    options.extend(
        crate::providers::moviebox::adapt::captions_json_to_options(payload)
            .into_iter()
            .map(|subtitle| (subtitle.name, subtitle.url)),
    );
    options
}

pub fn caption_url_for(payload: &serde_json::Value, language: &str) -> Option<String> {
    crate::providers::moviebox::adapt::captions_json_to_options(payload)
        .into_iter()
        .find(|subtitle| subtitle.name == language)
        .map(|subtitle| subtitle.url)
}
