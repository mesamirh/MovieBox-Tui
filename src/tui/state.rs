use ratatui::widgets::{ListState, TableState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Search,
    Details,
}

#[derive(Default, PartialEq)]
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
}

pub struct AppState {
    pub active_screen: Screen,
    pub input_mode: InputMode,
    pub search_query: String,
    pub last_suggest_query: String,
    pub last_search_edit: std::time::Instant,
    pub search_suggestions: Vec<String>,
    pub suggest_index: Option<usize>,
    pub search_results: Vec<SearchResult>,
    pub search_posters: std::collections::HashMap<String, std::sync::Arc<image::DynamicImage>>,
    pub search_poster_protocols: std::collections::HashMap<String, (ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub search_list_state: TableState,

    pub selected_details: Option<serde_json::Value>,
    pub selected_resources: Option<serde_json::Value>,
    pub active_popup: Option<String>,
    pub selected_poster: Option<std::sync::Arc<image::DynamicImage>>,
    pub selected_poster_protocol: Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub resource_list_state: ListState,

    pub details_pane: DetailsPane,
    pub selected_season: usize,
    pub selected_episode: usize,
    pub season_list_state: ListState,
    pub episode_list_state: ListState,
    pub language_list_state: ListState,
    pub available_seasons: Vec<serde_json::Value>,

    pub search_preview: Option<serde_json::Value>,
    pub preview_loading: bool,

    pub tick_count: u64,
    pub poster_image: Option<image::DynamicImage>,
    pub poster_protocol: Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub image_picker: Option<ratatui_image::picker::Picker>,
    pub image_supported: bool,
    pub image_cache: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,

    pub show_logs: bool,
    pub show_help: bool,
    pub visible_items: usize,
    pub logs: Vec<String>,
    pub logs_scroll: usize,

    pub active_error: Option<String>,
    pub is_loading: bool,
    pub status_message: String,
    pub status_timer: usize,
    pub toast_message: Option<String>,
    pub toast_timer: usize,
    pub update_available: Option<String>,

    pub download_progress: Option<f64>,
    pub download_status: Option<String>,
    pub cancel_download: std::sync::Arc<std::sync::atomic::AtomicBool>,

    pub language_chosen: bool,

    pub subtitle_popup: bool,
    pub subtitle_list: Vec<(String, String)>,
    pub subtitle_list_state: ListState,
    pub pending_play_link: Option<String>,
    pub basic_terminal: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_screen: Screen::Home,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            last_suggest_query: String::new(),
            last_search_edit: std::time::Instant::now(),
            search_suggestions: Vec::new(),
            suggest_index: None,
            search_results: Vec::new(),
            search_posters: std::collections::HashMap::new(),
            search_poster_protocols: std::collections::HashMap::new(),
            search_list_state: TableState::default(),
            basic_terminal: std::env::var("TERM_PROGRAM").unwrap_or_default() == "Apple_Terminal",
            selected_details: None,
            selected_resources: None,
            active_popup: None,
            selected_poster: None,
            selected_poster_protocol: None,
            resource_list_state: ListState::default(),

            details_pane: DetailsPane::default(),
            selected_season: 1,
            selected_episode: 1,
            season_list_state: ListState::default(),
            episode_list_state: ListState::default(),
            language_list_state: ListState::default(),
            available_seasons: vec![],

            search_preview: None,
            preview_loading: false,
            tick_count: 0,
            poster_image: None,
            poster_protocol: None,
            image_picker: None,
            image_supported: true,
            image_cache: lru::LruCache::new(std::num::NonZeroUsize::new(10).unwrap()),
            show_logs: false,
            show_help: false,
            visible_items: 10,
            logs: vec!["MovieBox-Tui started.".to_string()],
            logs_scroll: 0,
            active_error: None,
            is_loading: false,
            status_message: String::new(),
            status_timer: 0,
            toast_message: None,
            toast_timer: 0,
            update_available: None,
            download_progress: None,
            download_status: None,
            cancel_download: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            language_chosen: false,

            subtitle_popup: false,
            subtitle_list: Vec::new(),
            subtitle_list_state: ListState::default(),
            pending_play_link: None,
        }
    }
}

impl AppState {
    pub fn add_log(&mut self, msg: String) {
        self.logs.push(msg);
        if self.logs.len() > 200 {
            self.logs.remove(0);
        }
        self.logs_scroll = self.logs.len().saturating_sub(1);
    }
}
