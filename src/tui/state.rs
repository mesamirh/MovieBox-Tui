use ratatui::widgets::{ListState, TableState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Mpv,
    Iina,
    Vlc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
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
    pub is_homepage_mode: bool,
    pub current_tab_id: String,
    pub current_page: usize,
    pub search_posters: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,
    pub search_poster_protocols: std::collections::HashMap<
        String,
        (ratatui::layout::Rect, ratatui_image::protocol::Protocol),
    >,
    pub search_list_state: TableState,

    pub selected_details: Option<serde_json::Value>,
    pub active_subject_id: Option<String>,
    pub selected_resources: Option<serde_json::Value>,
    pub stream_cache: lru::LruCache<(String, usize, usize), Vec<serde_json::Value>>,
    pub preview_cache: lru::LruCache<String, serde_json::Value>,
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
    pub available_players: Vec<PlayerKind>,
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
    pub pending_open_with: bool,
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
            is_homepage_mode: false,
            current_tab_id: String::new(),
            current_page: 1,
            search_posters: lru::LruCache::new(std::num::NonZeroUsize::new(30).unwrap()),
            search_poster_protocols: std::collections::HashMap::new(),
            search_list_state: TableState::default(),
            basic_terminal: {
                let term = std::env::var("TERM").unwrap_or_default();
                let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
                let is_windows = cfg!(target_os = "windows");
                let is_dumb = term == "dumb" || term == "linux";
                let is_apple_terminal = term_program == "Apple_Terminal";
                is_windows || is_dumb || is_apple_terminal
            },
            selected_details: None,
            active_subject_id: None,
            selected_resources: None,
            stream_cache: lru::LruCache::new(std::num::NonZeroUsize::new(50).unwrap()),
            preview_cache: lru::LruCache::new(std::num::NonZeroUsize::new(30).unwrap()),
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
            image_supported: {
                let term = std::env::var("TERM").unwrap_or_default();
                let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
                if term_program == "Apple_Terminal" || term == "dumb" {
                    false
                } else {
                    std::env::var("KITTY_WINDOW_ID").is_ok()
                        || term_program.to_lowercase() == "ghostty"
                        || term_program.to_lowercase() == "wezterm"
                        || std::env::var("WEZTERM_UNIX_SOCKET").is_ok()
                        || std::env::var("WEZTERM_EXECUTABLE").is_ok()
                        || std::env::var("ITERM_SESSION_ID").is_ok()
                        || term == "xterm-kitty"
                }
            },
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
            available_players: {
                let mut players = Vec::new();
                let which_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
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
                        players.push(PlayerKind::Iina);
                    }
                }
                if std::path::Path::new("/Applications/mpv.app").exists()
                    || std::path::Path::new("C:\\Program Files\\mpv\\mpv.exe").exists()
                    || check_player("mpv")
                {
                    players.push(PlayerKind::Mpv);
                }
                if std::path::Path::new("/Applications/VLC.app").exists()
                    || std::path::Path::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe").exists()
                    || check_player("vlc")
                {
                    players.push(PlayerKind::Vlc);
                }
                players
            },
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
            pending_open_with: false,
        }
    }
}
