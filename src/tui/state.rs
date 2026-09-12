use crate::providers::models::ProviderKind;
use ratatui::widgets::{ListState, TableState};

pub use crate::player::PlayerKind;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomeDeckTab {
    #[default]
    ContinueWatching,
    Favorites,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    #[default]
    General,
    ContentModes,
    Appearance,
    StorageInfo,
}

impl SettingsCategory {
    pub const ALL: [Self; 4] = [
        Self::General,
        Self::ContentModes,
        Self::Appearance,
        Self::StorageInfo,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::ContentModes => "Content Modes",
            Self::Appearance => "Appearance",
            Self::StorageInfo => "Maintenance",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::General => "GEN",
            Self::ContentModes => "MODES",
            Self::Appearance => "THEME",
            Self::StorageInfo => "MAINT",
        }
    }

    pub fn row_count(self) -> usize {
        match self {
            Self::General => 3,
            Self::ContentModes => 3,
            Self::Appearance => 1,
            Self::StorageInfo => 4,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::General => Self::ContentModes,
            Self::ContentModes => Self::Appearance,
            Self::Appearance => Self::StorageInfo,
            Self::StorageInfo => Self::General,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::General => Self::StorageInfo,
            Self::ContentModes => Self::General,
            Self::Appearance => Self::ContentModes,
            Self::StorageInfo => Self::Appearance,
        }
    }
}

pub fn settings_player_label(choice: Option<&str>) -> &'static str {
    match choice {
        None | Some("auto") => "None",
        Some("mpv") => "mpv",
        Some("vlc") => "VLC",
        Some("iina") => "IINA",
        Some("android") => "Android Player",
        Some(_) => "Custom",
    }
}

pub use crate::models::{
    AudioTrackOption, BrowseMetric, BrowseMetrics, BrowsePreset, CatalogItem, Episode,
    MediaDetails, MediaType, Notification, NotificationKind, Release, SearchResult, Season,
    SourceMirror, SubjectStreamPool,
};

pub type HomepageCacheData = (
    Vec<CatalogItem>,
    std::collections::HashMap<String, BrowseMetrics>,
);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultMetrics {
    pub poster_rows_eff: u16,
    pub row_height: u16,
    pub visible_items: usize,
    pub columns: u16,
    pub col_width: u16,
}

pub fn result_columns_for(width: u16) -> u16 {
    if width < 110 {
        1
    } else if width < 160 {
        2
    } else if width < 220 {
        3
    } else if width < 280 {
        4
    } else {
        5
    }
}

#[derive(Debug, Default)]
pub struct UiState {
    pub active_screen: Screen,
    pub input_mode: InputMode,
    pub dirty: bool,
    pub show_theme_popup: bool,
    pub active_theme_kind: String,
    pub theme_is_auto: bool,
    pub show_browse_popup: bool,
    pub show_settings_popup: bool,
    pub settings_category: SettingsCategory,
    pub settings_selected_row: usize,
    pub show_help: bool,
    pub help_scroll: usize,
    pub cursor_beam: bool,
    pub details_pane: DetailsPane,
    pub player_picker_popup: bool,
    pub settings_player_picker: bool,
    pub show_sources_popup: bool,
    pub subtitle_popup: bool,
    pub is_download_subtitle_popup: bool,
    pub tv_config_popup: bool,
    pub addon_manager_popup: bool,
    pub favorites_focus: bool,
    pub is_loading: bool,
    pub is_resolving_playback: bool,
    pub is_fetching_streams: bool,
}

#[derive(Debug, Default)]
pub struct CatalogState {
    pub active_provider: ProviderKind,
    pub provider_generation: u64,
    pub current_tab_id: String,
    pub current_page: usize,
    pub is_homepage_mode: bool,
    pub active_subject_id: Option<String>,
    pub search_error: Option<String>,
    pub details_error: Option<String>,
    pub stream_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub default_player: Option<String>,
    pub available_players: Vec<PlayerKind>,
    pub pending_play_link: Option<String>,
    pub pending_playback_source: Option<crate::providers::models::PlaybackSource>,
}

#[derive(Debug, Default)]
pub struct DownloadState {
    pub download_progress: Option<f64>,
    pub download_status: Option<String>,
    pub download_title: Option<String>,
    pub download_dir: Option<std::path::PathBuf>,
    pub download_queue_total: usize,
    pub show_season_download_confirm: bool,
    pub season_download_confirm_yes_selected: bool,
    pub show_episode_download_confirm: bool,
    pub episode_download_confirm_yes_selected: bool,
    pub is_waiting_for_download_stream: bool,
    pub auto_play_on_ready: bool,
}

pub struct AppState {
    pub active_provider: ProviderKind,
    pub provider_generation: u64,
    pub active_screen: Screen,
    pub dirty: bool,
    pub input_mode: InputMode,
    pub search_query: crate::tui::text::TextInputBuffer,
    pub last_suggest_query: String,
    pub last_search_edit: std::time::Instant,
    pub search_suggestions: Vec<String>,
    pub suggest_index: Option<usize>,
    pub suggest_cache: lru::LruCache<String, Vec<String>>,
    pub homepage_cache: lru::LruCache<(String, usize), HomepageCacheData>,
    pub search_results: Vec<SearchResult>,
    pub search_error: Option<String>,
    pub is_homepage_mode: bool,
    pub current_tab_id: String,
    pub current_page: usize,
    pub search_posters: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,
    pub failed_posters: lru::LruCache<String, std::time::Instant>,
    pub search_poster_protocols:
        lru::LruCache<String, ((u16, u16), ratatui_image::protocol::Protocol)>,
    pub poster_fetch_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    pub in_flight_posters: std::collections::HashSet<String>,
    pub search_list_state: TableState,

    pub selected_details: Option<MediaDetails>,
    pub active_subject_id: Option<String>,
    pub selected_resources: Vec<Release>,
    pub stream_pool: std::collections::HashMap<String, SubjectStreamPool>,
    pub fetch_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub show_season_download_confirm: bool,
    pub season_download_confirm_yes_selected: bool,
    pub show_episode_download_confirm: bool,
    pub episode_download_confirm_yes_selected: bool,
    pub is_waiting_for_download_stream: bool,
    pub auto_play_on_ready: bool,
    pub is_fetching_streams: bool,
    pub stream_error: Option<String>,
    pub details_error: Option<String>,
    pub preview_cache: lru::LruCache<String, MediaDetails>,
    pub resource_list_state: ListState,

    pub details_pane: DetailsPane,
    pub selected_season: usize,
    pub selected_episode: usize,
    pub season_list_state: ListState,
    pub episode_list_state: ListState,
    pub language_list_state: ListState,
    pub available_seasons: Vec<Season>,
    pub available_episode_numbers: Vec<Vec<usize>>,

    pub search_preview: Option<MediaDetails>,
    pub preview_loading: bool,

    pub tick_count: u64,
    pub poster_image: Option<std::sync::Arc<image::DynamicImage>>,

    pub show_theme_popup: bool,
    pub active_theme_kind: String,
    pub theme_is_auto: bool,
    pub original_theme_kind: Option<String>,
    pub theme_list_state: ListState,
    pub show_browse_popup: bool,
    pub browse_list_state: ListState,
    pub active_browse_preset: Option<BrowsePreset>,
    pub active_addon_catalog: Option<crate::providers::addons::models::AddonCatalogTarget>,
    pub show_provider_popup: bool,
    pub provider_list_state: ListState,
    pub browse_metrics: std::collections::HashMap<String, BrowseMetrics>,
    pub show_settings_popup: bool,
    pub settings_category: SettingsCategory,
    pub settings_selected_row: usize,
    pub settings_download_dir_input: Option<crate::tui::text::TextInputBuffer>,
    pub show_sources_popup: bool,
    pub sources_list_state: ListState,

    pub poster_protocol: Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>,
    pub image_picker: Option<ratatui_image::picker::Picker>,
    pub image_supported: bool,
    pub clear_terminal_before_draw: bool,
    pub poster_rows: u16,
    pub image_cache: lru::LruCache<String, std::sync::Arc<image::DynamicImage>>,

    pub show_help: bool,
    pub help_scroll: usize,
    pub cursor_beam: bool,
    pub last_result_metrics: Option<ResultMetrics>,
    pub result_scroll: usize,

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
    pub settings_player_picker: bool,
    pub available_players: Vec<PlayerKind>,
    pub default_player: Option<String>,
    pub is_loading: bool,
    pub is_resolving_playback: bool,
    pub has_streams_settled: bool,
    pub has_search_settled: bool,
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
    pub update_progress_msg: Option<String>,
    pub download_progress: Option<f64>,
    pub download_status: Option<String>,
    pub download_title: Option<String>,
    pub cancel_download: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub download_dir: Option<std::path::PathBuf>,

    pub download_queue: std::collections::VecDeque<(usize, usize)>,
    pub download_queue_total: usize,

    pub language_chosen: bool,

    pub subtitle_popup: bool,
    pub is_download_subtitle_popup: bool,
    pub season_subtitle_preference: Option<Option<String>>,
    pub last_download_subtitle_language: Option<String>,
    pub subtitle_list: Vec<(String, String)>,
    pub subtitle_list_state: ListState,
    pub pending_play_link: Option<String>,
    pub pending_playback_source: Option<crate::providers::models::PlaybackSource>,
    pub basic_terminal: bool,
    pub moviebox_enabled: bool,
    pub fourkhdhub_enabled: bool,
    pub bdix_circleftp_enabled: bool,
    pub bdix_dhakaflix_enabled: bool,
    pub bdix_probed: bool,
    pub streaming_enabled: bool,

    pub is_tv_mode: bool,
    pub tv_enabled: bool,
    pub tv_config_popup: bool,
    pub tv_channels: Vec<crate::providers::tv::Channel>,
    pub tv_playlists: Vec<String>,
    pub tv_manager_selected: usize,
    pub tv_input_active: bool,
    pub tv_input_buffer: crate::tui::text::TextInputBuffer,
    pub tv_input_is_file: bool,

    pub addons_enabled: bool,
    pub installed_addons: Vec<crate::providers::addons::models::InstalledAddon>,
    pub addon_manager_popup: bool,
    pub addon_manager_selected: usize,
    pub addon_input_active: bool,
    pub addon_input_buffer: crate::tui::text::TextInputBuffer,
    pub history: crate::history::HistoryManager,
    pub favorites: crate::favorites::FavoritesManager,
    pub favorites_focus: bool,
    pub home_deck_tab: HomeDeckTab,
    pub favorites_landing_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::MovieBox,
            provider_generation: 0,
            active_screen: Screen::Home,
            input_mode: InputMode::Normal,
            search_query: crate::tui::text::TextInputBuffer::new(),
            last_suggest_query: String::new(),
            last_search_edit: std::time::Instant::now(),
            search_suggestions: Vec::new(),
            suggest_index: None,
            suggest_cache: lru::LruCache::new(cache_capacity(128)),
            homepage_cache: lru::LruCache::new(cache_capacity(32)),
            search_results: Vec::new(),
            search_error: None,
            is_homepage_mode: false,
            current_tab_id: String::new(),
            current_page: 1,
            search_posters: lru::LruCache::new(cache_capacity(96)),
            failed_posters: lru::LruCache::new(cache_capacity(300)),
            search_poster_protocols: lru::LruCache::new(cache_capacity(128)),
            poster_fetch_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
            in_flight_posters: std::collections::HashSet::new(),
            search_list_state: TableState::default(),
            basic_terminal: crate::tui::terminal::uses_basic_ui(),
            selected_details: None,
            active_subject_id: None,
            selected_resources: vec![],
            stream_pool: std::collections::HashMap::new(),
            fetch_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            show_season_download_confirm: false,
            season_download_confirm_yes_selected: false,
            show_episode_download_confirm: false,
            episode_download_confirm_yes_selected: false,
            is_waiting_for_download_stream: false,
            is_fetching_streams: false,
            auto_play_on_ready: false,
            stream_error: None,
            details_error: None,
            resource_list_state: ListState::default(),
            preview_cache: lru::LruCache::new(cache_capacity(64)),
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
            theme_is_auto: true,
            original_theme_kind: None,
            show_theme_popup: false,
            theme_list_state: ListState::default(),
            show_browse_popup: false,
            browse_list_state: ListState::default(),
            active_browse_preset: None,
            active_addon_catalog: None,
            show_provider_popup: false,
            provider_list_state: ListState::default(),
            browse_metrics: std::collections::HashMap::new(),
            show_settings_popup: false,
            settings_category: SettingsCategory::General,
            settings_selected_row: 0,
            settings_download_dir_input: None,
            show_sources_popup: false,
            sources_list_state: ListState::default(),

            poster_protocol: None,
            image_picker: None,
            image_supported: crate::tui::terminal::should_query_images(),
            clear_terminal_before_draw: false,
            poster_rows: 3,
            image_cache: lru::LruCache::new(cache_capacity(10)),
            show_help: false,
            help_scroll: 0,
            cursor_beam: false,
            last_result_metrics: None,
            result_scroll: 0,
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
            settings_player_picker: false,
            available_players: Vec::new(),
            default_player: None,
            dirty: true,
            is_loading: false,
            is_resolving_playback: false,
            has_streams_settled: false,
            has_search_settled: false,
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
            update_progress_msg: None,
            download_progress: None,
            download_status: None,
            download_title: None,
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
            pending_playback_source: None,
            moviebox_enabled: true,
            fourkhdhub_enabled: true,
            bdix_circleftp_enabled: false,
            bdix_dhakaflix_enabled: false,
            bdix_probed: false,
            streaming_enabled: true,
            is_tv_mode: false,
            tv_enabled: true,
            tv_config_popup: false,
            tv_channels: Vec::new(),
            tv_playlists: Vec::new(),
            tv_manager_selected: 0,
            tv_input_active: false,
            tv_input_buffer: crate::tui::text::TextInputBuffer::new(),
            tv_input_is_file: false,
            addons_enabled: false,
            installed_addons: Vec::new(),
            addon_manager_popup: false,
            addon_manager_selected: 0,
            addon_input_active: false,
            addon_input_buffer: crate::tui::text::TextInputBuffer::new(),
            history: crate::history::HistoryManager::new(),
            favorites: crate::favorites::FavoritesManager::new(),
            favorites_focus: false,
            home_deck_tab: HomeDeckTab::default(),
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
        if self.is_tv_mode {
            AppMode::Tv
        } else {
            AppMode::Streaming
        }
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        match mode {
            AppMode::Streaming => {
                self.is_tv_mode = false;
            }
            AppMode::Tv => {
                self.is_tv_mode = true;
            }
        }
    }
    pub fn provider_for_subject(&self, subject_id: &str) -> ProviderKind {
        self.search_results
            .iter()
            .find(|r| r.id == subject_id)
            .map(|r| r.provider)
            .or_else(|| {
                self.selected_details
                    .as_ref()
                    .filter(|d| d.id.value == subject_id)
                    .map(|d| d.id.provider)
            })
            .or_else(|| self.selected_resources.first().map(|r| r.provider))
            .unwrap_or(self.active_provider)
    }

    pub fn current_subject_provider(&self) -> ProviderKind {
        self.active_subject_id
            .as_deref()
            .map(|id| self.provider_for_subject(id))
            .unwrap_or(self.active_provider)
    }

    pub fn is_selected_details_favorited(&self) -> bool {
        let Some(details) = &self.selected_details else {
            return false;
        };
        let details_subject_id = self.active_subject_id.as_deref().unwrap_or("");
        let title = &details.title;
        let type_val = if details.is_series() { 2 } else { 1 };
        let year = details.year.as_deref().unwrap_or("N/A");
        let provider = self.provider_for_subject(details_subject_id);
        self.favorites.is_favorite(&crate::models::SubjectIdentity {
            provider: provider.cache_key(),
            subject_id: details_subject_id,
            title,
            stype: type_val,
            release_year: year,
        })
    }

    pub fn provider_enabled(&self, p: ProviderKind) -> bool {
        match p {
            ProviderKind::MovieBox => self.moviebox_enabled,
            ProviderKind::FourKHdHub => self.fourkhdhub_enabled,
            ProviderKind::BdixCircleFtp => self.bdix_circleftp_enabled,
            ProviderKind::BdixDhakaFlix => self.bdix_dhakaflix_enabled,
            ProviderKind::Addons => self.addons_enabled,
        }
    }

    pub fn set_provider_enabled(&mut self, p: ProviderKind, enabled: bool) {
        match p {
            ProviderKind::MovieBox => self.moviebox_enabled = enabled,
            ProviderKind::FourKHdHub => self.fourkhdhub_enabled = enabled,
            ProviderKind::BdixCircleFtp => self.bdix_circleftp_enabled = enabled,
            ProviderKind::BdixDhakaFlix => self.bdix_dhakaflix_enabled = enabled,
            ProviderKind::Addons => self.addons_enabled = enabled,
        }
    }

    pub fn available_providers(&self) -> Vec<ProviderKind> {
        let mut providers: Vec<ProviderKind> = crate::models::ProviderKind::ENABLED
            .into_iter()
            .filter(|p| self.provider_enabled(*p))
            .collect();
        if self.addons_enabled || !self.installed_addons.is_empty() {
            providers.push(ProviderKind::Addons);
        }
        providers
    }

    pub fn next_provider(&self) -> ProviderKind {
        let available_providers = self.available_providers();
        if available_providers.is_empty() {
            return self.active_provider;
        }
        let current = available_providers
            .iter()
            .position(|provider| *provider == self.active_provider)
            .unwrap_or(0);
        available_providers[(current + 1) % available_providers.len()]
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

    pub const STATUS_TICKS_SHORT: u16 = 90;
    pub const STATUS_TICKS_DEFAULT: u16 = 150;
    pub const STATUS_TICKS_LONG: u16 = 240;

    pub fn set_status(&mut self, message: impl Into<String>, timer: usize) {
        self.status_message = message.into();
        self.status_timer = timer;
    }

    pub fn set_status_default(&mut self, msg: impl Into<String>) {
        self.set_status(msg, Self::STATUS_TICKS_DEFAULT as usize);
    }

    pub fn set_status_short(&mut self, msg: impl Into<String>) {
        self.set_status(msg, Self::STATUS_TICKS_SHORT as usize);
    }

    pub fn set_status_long(&mut self, msg: impl Into<String>) {
        self.set_status(msg, Self::STATUS_TICKS_LONG as usize);
    }

    pub fn result_metrics(&self, results_height: u16, results_width: u16) -> ResultMetrics {
        let max_rows = results_height.saturating_sub(1).max(3);
        let poster_rows_eff = self.poster_rows.max(3).min(max_rows);
        let row_height = poster_rows_eff.saturating_add(1).max(4);
        let columns = crate::tui::state::result_columns_for(results_width);
        let total_gutters = columns.saturating_sub(1);
        let usable_width = results_width.saturating_sub(total_gutters);
        let col_width = (usable_width / columns.max(1)).max(1);
        let visible_rows = (results_height as usize / row_height as usize).max(1);
        let visible_items = visible_rows * columns as usize;
        ResultMetrics {
            poster_rows_eff,
            row_height,
            visible_items,
            columns,
            col_width,
        }
    }

    pub fn normalize_result_view(&mut self) {
        let total = self.search_results.len();
        if total == 0 {
            self.result_scroll = 0;
            self.search_list_state.select(None);
            return;
        }
        let selected = self
            .search_list_state
            .selected()
            .unwrap_or(0)
            .min(total - 1);
        self.search_list_state.select(Some(selected));
        let columns = match self.last_result_metrics {
            Some(metrics) => metrics.columns as usize,
            None => 1,
        };
        let visible_items = match self.last_result_metrics {
            Some(metrics) => metrics.visible_items,
            None => 8,
        };
        let cols = columns.max(1);
        let rows_visible = (visible_items / cols).max(1);
        let selected_row = selected / cols;
        let mut base_row = self.result_scroll / cols;
        if selected_row < base_row {
            base_row = selected_row;
        } else if selected_row >= base_row + rows_visible {
            base_row = selected_row.saturating_sub(rows_visible - 1);
        }
        let max_base = total.saturating_sub(rows_visible * cols) / cols;
        let base_row = base_row.min(max_base);
        self.result_scroll = base_row * cols;
    }

    pub fn effective_visible_items(&self) -> usize {
        self.last_result_metrics
            .map(|metrics| metrics.visible_items)
            .unwrap_or(8)
    }

    pub fn effective_row_height(&self) -> u16 {
        self.last_result_metrics
            .map(|metrics| metrics.row_height)
            .unwrap_or(4)
    }

    const FAILED_POSTER_TTL_SECS: u64 = 600;

    pub fn failed_poster_recently(&mut self, id: &str) -> bool {
        match self.failed_posters.peek(id) {
            Some(failed_at) if failed_at.elapsed().as_secs() < Self::FAILED_POSTER_TTL_SECS => true,
            Some(_) => {
                self.failed_posters.pop(id);
                false
            }
            None => false,
        }
    }
    pub fn has_active_modal(&self) -> bool {
        self.show_help
            || self.show_theme_popup
            || self.show_browse_popup
            || self.show_provider_popup
            || self.show_settings_popup
            || self.show_sources_popup
            || self.addon_manager_popup
            || self.tv_config_popup
            || self.player_picker_popup
            || self.subtitle_popup
            || self.is_download_subtitle_popup
            || self.show_season_download_confirm
            || self.show_episode_download_confirm
            || (self.update_available.is_some() && self.input_mode != InputMode::Editing)
            || self.is_updating
            || (self.input_mode == InputMode::Editing && !self.search_suggestions.is_empty())
    }

    pub fn clear_search_state(&mut self) {
        self.search_query.clear();
        self.search_results.clear();
        self.search_error = None;
        self.search_suggestions.clear();
        self.suggest_cache.clear();
        self.suggest_index = None;
        self.search_preview = None;
        self.preview_loading = false;
        self.active_browse_preset = None;
        self.active_addon_catalog = None;
        self.browse_metrics.clear();
        self.clear_poster_cache();
        self.result_scroll = 0;
        self.search_list_state.select(None);
        self.is_homepage_mode = false;
        self.favorites_focus = false;
        self.favorites_landing_state.select(None);
        self.has_search_settled = false;
    }

    pub fn clear_details_state(&mut self) {
        self.active_subject_id = None;
        self.selected_details = None;
        self.selected_resources.clear();
        self.is_fetching_streams = false;
        self.pending_episode_fetch = None;
        self.auto_play_on_ready = false;
        self.stream_error = None;
        self.details_error = None;
        self.available_seasons.clear();
        self.available_episode_numbers.clear();
        self.season_list_state.select(None);
        self.episode_list_state.select(None);
        self.resource_list_state.select(None);
        self.language_list_state.select(None);
        self.details_pane = DetailsPane::Streams;
        self.has_streams_settled = false;
    }

    pub fn favorites_available(&self) -> bool {
        self.streaming_enabled && !self.is_tv_mode
    }

    pub fn favorites_landing_visible(&self) -> bool {
        self.favorites_available()
            && !self.favorites.items.is_empty()
            && !(self.input_mode == InputMode::Editing && !self.search_suggestions.is_empty())
    }

    pub fn favorites_landing_items(&self) -> Vec<&crate::favorites::FavoriteItem> {
        let mut items: Vec<&crate::favorites::FavoriteItem> = self.favorites.items.iter().collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.added_at));
        items.truncate(5);
        items
    }

    pub fn continue_watching_items(&self) -> Vec<&crate::history::WatchHistoryItem> {
        let mut items: Vec<&crate::history::WatchHistoryItem> = self
            .history
            .recent
            .iter()
            .filter(|item| item.is_in_progress())
            .collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.timestamp));
        items.truncate(5);
        items
    }

    pub fn continue_watching_available(&self) -> bool {
        self.streaming_enabled
            && !self.is_tv_mode
            && self.history.recent.iter().any(|item| item.is_in_progress())
    }

    pub fn effective_home_deck_tab(&self) -> HomeDeckTab {
        let cw_has = self.continue_watching_available();
        let fav_has = self.favorites_available() && !self.favorites.items.is_empty();
        match self.home_deck_tab {
            HomeDeckTab::ContinueWatching => {
                if cw_has {
                    HomeDeckTab::ContinueWatching
                } else if fav_has {
                    HomeDeckTab::Favorites
                } else {
                    HomeDeckTab::ContinueWatching
                }
            }
            HomeDeckTab::Favorites => {
                if fav_has {
                    HomeDeckTab::Favorites
                } else if cw_has {
                    HomeDeckTab::ContinueWatching
                } else {
                    HomeDeckTab::Favorites
                }
            }
        }
    }

    pub fn landing_deck_visible(&self) -> bool {
        (self.continue_watching_available() || self.favorites_landing_visible())
            && !(self.input_mode == InputMode::Editing && !self.search_suggestions.is_empty())
    }

    pub fn landing_deck_items_count(&self) -> usize {
        match self.effective_home_deck_tab() {
            HomeDeckTab::ContinueWatching => self.continue_watching_items().len(),
            HomeDeckTab::Favorites => self.favorites_landing_items().len(),
        }
    }

    pub fn landing_deck_total_items_count(&self) -> usize {
        match self.effective_home_deck_tab() {
            HomeDeckTab::ContinueWatching => self
                .history
                .recent
                .iter()
                .filter(|i| i.is_in_progress())
                .count(),
            HomeDeckTab::Favorites => self.favorites.items.len(),
        }
    }

    pub fn cycle_home_deck_tab(&mut self) {
        let cw_has = self.continue_watching_available();
        let fav_has = self.favorites_available() && !self.favorites.items.is_empty();
        if cw_has && fav_has {
            self.home_deck_tab = match self.effective_home_deck_tab() {
                HomeDeckTab::ContinueWatching => HomeDeckTab::Favorites,
                HomeDeckTab::Favorites => HomeDeckTab::ContinueWatching,
            };
            if self.favorites_focus {
                self.favorites_landing_state.select(Some(0));
            }
        }
    }

    pub fn loading_dots(&self) -> &'static str {
        match (self.tick_count / 4) % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        }
    }
    pub fn settings_next_category(&mut self) {
        self.settings_category = self.settings_category.next();
        self.settings_selected_row = 0;
        self.settings_download_dir_input = None;
    }

    pub fn settings_previous_category(&mut self) {
        self.settings_category = self.settings_category.previous();
        self.settings_selected_row = 0;
        self.settings_download_dir_input = None;
    }

    pub fn settings_select_category(&mut self, cat: SettingsCategory) {
        if self.settings_category != cat {
            self.settings_category = cat;
            self.settings_selected_row = 0;
            self.settings_download_dir_input = None;
        }
    }

    pub fn settings_row_up(&mut self) {
        let count = self.settings_category.row_count();
        if count > 0 {
            self.settings_selected_row = (self.settings_selected_row + count - 1) % count;
        }
    }

    pub fn settings_row_down(&mut self) {
        let count = self.settings_category.row_count();
        if count > 0 {
            self.settings_selected_row = (self.settings_selected_row + 1) % count;
        }
    }

    pub fn ensure_default_player(&mut self) {
        if (self.default_player.is_none() || self.default_player.as_deref() == Some("auto"))
            && let Some(first) = self.available_players.first()
        {
            self.default_player = Some(first.config_key().to_string());
        }
    }

    pub fn settings_player_choices(&self) -> Vec<&str> {
        let mut choices: Vec<&str> = Vec::with_capacity(self.available_players.len());
        for player in &self.available_players {
            let key = player.config_key();
            if !choices.iter().any(|&c| c.eq_ignore_ascii_case(key)) {
                choices.push(key);
            }
        }
        choices
    }

    pub fn cycle_settings_player(&mut self, forward: bool) {
        let choices = self.settings_player_choices();
        if choices.is_empty() {
            return;
        }
        let current_key = self.default_player.as_deref();
        let current_idx = current_key
            .and_then(|k| choices.iter().position(|&opt| opt.eq_ignore_ascii_case(k)))
            .unwrap_or(0);

        let total = choices.len();
        let next_idx = if forward {
            (current_idx + 1) % total
        } else {
            (current_idx + total - 1) % total
        };

        self.default_player = Some(choices[next_idx].to_string());
    }

    pub fn cycle_settings_theme(&mut self, forward: bool) -> String {
        let themes = crate::tui::theme::AVAILABLE_THEMES;
        let total = themes.len();
        let current_idx = themes
            .iter()
            .position(|&t| t.eq_ignore_ascii_case(&self.active_theme_kind))
            .unwrap_or(0);

        let next_idx = if forward {
            (current_idx + 1) % total
        } else {
            (current_idx + total - 1) % total
        };

        let next_theme = themes[next_idx].to_string();
        self.active_theme_kind = next_theme.clone();
        self.theme_is_auto = false;
        next_theme
    }

    pub fn can_disable_streaming_mode(&self) -> bool {
        self.tv_enabled || self.addons_enabled
    }

    pub fn can_disable_tv_mode(&self) -> bool {
        self.streaming_enabled
    }
    pub fn expand_download_path(raw: &str) -> Option<std::path::PathBuf> {
        let clean = raw.trim_matches(|c| c == '\'' || c == '"').trim();
        if clean.is_empty()
            || clean.eq_ignore_ascii_case("default")
            || clean.eq_ignore_ascii_case("reset")
            || clean == "<path>"
            || clean == "path"
            || clean == "<dir>"
            || clean == "dir"
        {
            return None;
        }
        let pb = if let Some(stripped) = clean
            .strip_prefix("~/")
            .or_else(|| clean.strip_prefix("~\\"))
        {
            if let Some(home) = dirs::home_dir() {
                home.join(stripped)
            } else {
                std::path::PathBuf::from(clean)
            }
        } else if clean == "~" {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(clean))
        } else {
            std::path::PathBuf::from(clean)
        };
        Some(pb)
    }
    pub fn clear_poster_cache(&mut self) {
        self.poster_image = None;
        self.poster_protocol = None;
        self.search_posters.clear();
        self.search_poster_protocols.clear();
        self.failed_posters.clear();
        self.in_flight_posters.clear();
    }

    pub fn clear_poster_protocols(&mut self) {
        self.poster_protocol = None;
        self.search_poster_protocols.clear();
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

pub fn step_list_selection(state: &mut ListState, total_items: usize, step: isize) {
    if total_items == 0 {
        state.select(None);
        return;
    }
    let max = total_items.saturating_sub(1);
    let cur = state.selected().unwrap_or(0);
    let next = if step < 0 {
        cur.saturating_sub(step.unsigned_abs())
    } else {
        (cur + step as usize).min(max)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonManagerRow {
    Header(&'static str),
    Addon(usize),
    AddUrl,
}
pub fn step_header_aware_list<F>(current: usize, total: usize, step: isize, is_header: F) -> usize
where
    F: Fn(usize) -> bool,
{
    if total == 0 {
        return 0;
    }
    if step == 0 {
        return current;
    }

    if step < -1 {
        let jump = (-step) as usize;
        let mut target = current.saturating_sub(jump);
        while target > 0 && is_header(target) {
            target = target.saturating_sub(1);
        }
        if is_header(target) {
            if let Some(first_valid) = (0..total).find(|&i| !is_header(i)) {
                target = first_valid;
            }
        }
        return target;
    } else if step > 1 {
        let jump = step as usize;
        let mut target = (current + jump).min(total.saturating_sub(1));
        while target < total && is_header(target) {
            target += 1;
        }
        if target >= total || is_header(target) {
            if let Some(last_valid) = (0..total).rposition(|i| !is_header(i)) {
                target = last_valid;
            }
        }
        return target;
    }

    let forward = step > 0;
    let mut next = if forward {
        if current + 1 >= total { 0 } else { current + 1 }
    } else if current == 0 {
        total.saturating_sub(1)
    } else {
        current - 1
    };

    while next != current && is_header(next) {
        next = if forward {
            if next + 1 >= total { 0 } else { next + 1 }
        } else if next == 0 {
            total.saturating_sub(1)
        } else {
            next - 1
        };
    }

    next
}

impl AppState {
    pub fn tv_manager_rows(&self) -> Vec<TvManagerRow> {
        let mut rows = vec![TvManagerRow::Header("URL playlists")];
        for (index, source) in self.tv_playlists.iter().enumerate() {
            if crate::tui::text::is_http_url(source) {
                rows.push(TvManagerRow::Playlist(index));
            }
        }
        rows.push(TvManagerRow::AddUrl);
        rows.push(TvManagerRow::Header("File playlists"));
        for (index, source) in self.tv_playlists.iter().enumerate() {
            if !crate::tui::text::is_http_url(source) {
                rows.push(TvManagerRow::Playlist(index));
            }
        }
        rows.push(TvManagerRow::AddFile);
        rows.push(TvManagerRow::Reload);
        rows.push(TvManagerRow::Done);
        rows
    }

    pub fn step_tv_manager_selected(&mut self, step: isize) {
        let rows = self.tv_manager_rows();
        self.tv_manager_selected =
            step_header_aware_list(self.tv_manager_selected, rows.len(), step, |idx| {
                matches!(rows.get(idx), Some(TvManagerRow::Header(_)))
            });
    }

    pub fn first_tv_manager_selected(&mut self) {
        let rows = self.tv_manager_rows();
        if let Some(idx) = rows
            .iter()
            .position(|r| !matches!(r, TvManagerRow::Header(_)))
        {
            self.tv_manager_selected = idx;
        }
    }

    pub fn last_tv_manager_selected(&mut self) {
        let rows = self.tv_manager_rows();
        if let Some(idx) = rows
            .iter()
            .rposition(|r| !matches!(r, TvManagerRow::Header(_)))
        {
            self.tv_manager_selected = idx;
        }
    }

    pub fn addon_manager_rows(&self) -> Vec<AddonManagerRow> {
        let mut rows = vec![AddonManagerRow::Header("Installed Addons")];
        for index in 0..self.installed_addons.len() {
            rows.push(AddonManagerRow::Addon(index));
        }
        rows.push(AddonManagerRow::AddUrl);
        rows
    }

    pub fn step_addon_manager_selected(&mut self, step: isize) {
        let rows = self.addon_manager_rows();
        self.addon_manager_selected =
            step_header_aware_list(self.addon_manager_selected, rows.len(), step, |idx| {
                matches!(rows.get(idx), Some(AddonManagerRow::Header(_)))
            });
    }

    pub fn first_addon_manager_selected(&mut self) {
        let rows = self.addon_manager_rows();
        if let Some(idx) = rows
            .iter()
            .position(|r| !matches!(r, AddonManagerRow::Header(_)))
        {
            self.addon_manager_selected = idx;
        }
    }

    pub fn last_addon_manager_selected(&mut self) {
        let rows = self.addon_manager_rows();
        if let Some(idx) = rows
            .iter()
            .rposition(|r| !matches!(r, AddonManagerRow::Header(_)))
        {
            self.addon_manager_selected = idx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(results: usize) -> AppState {
        let mut state = AppState::default();
        for i in 0..results {
            state.search_results.push(crate::models::SearchResult {
                id: i.to_string(),
                title: format!("T{i}"),
                stype: 1,
                release_year: "2020".to_string(),
                cover_url: None,
                season: 0,
                episode: 0,
                provider: crate::providers::models::ProviderKind::MovieBox,
            });
        }
        state
    }

    #[test]
    fn columns_follow_width_tiers() {
        assert_eq!(result_columns_for(60), 1);
        assert_eq!(result_columns_for(80), 1);
        assert_eq!(result_columns_for(110), 2);
        assert_eq!(result_columns_for(159), 2);
        assert_eq!(result_columns_for(160), 3);
        assert_eq!(result_columns_for(219), 3);
        assert_eq!(result_columns_for(220), 4);
        assert_eq!(result_columns_for(279), 4);
        assert_eq!(result_columns_for(280), 5);
        assert_eq!(result_columns_for(320), 5);
    }

    #[test]
    fn addons_provider_appears_when_enabled() {
        let mut s = AppState {
            addons_enabled: true,
            ..Default::default()
        };
        assert!(s.available_providers().contains(&ProviderKind::Addons));
        s.addons_enabled = false;
        assert!(!s.available_providers().contains(&ProviderKind::Addons));
    }

    #[test]
    fn poster_rows_clamp_to_viewport() {
        let state = AppState {
            poster_rows: 12,
            ..Default::default()
        };
        let metrics = state.result_metrics(10, 100);
        assert!(metrics.poster_rows_eff <= 9, "rows must fit viewport");
        assert!(metrics.visible_items >= 1);
    }

    #[test]
    fn normalize_clamps_selection_and_aligns_scroll() {
        let mut state = state_with(20);
        state.last_result_metrics = Some(ResultMetrics {
            poster_rows_eff: 3,
            row_height: 4,
            visible_items: 9,
            columns: 3,
            col_width: 60,
        });
        state.search_list_state.select(Some(25));
        state.result_scroll = 50;
        state.normalize_result_view();
        assert_eq!(state.search_list_state.selected(), Some(19));
        assert!(state.result_scroll <= 11);
        assert_eq!(state.result_scroll % 3, 0);
    }

    #[test]
    fn normalize_keeps_selection_visible_when_scrolling_down() {
        let mut state = state_with(30);
        state.last_result_metrics = Some(ResultMetrics {
            poster_rows_eff: 3,
            row_height: 4,
            visible_items: 6,
            columns: 3,
            col_width: 40,
        });
        state.search_list_state.select(Some(10));
        state.result_scroll = 0;
        state.normalize_result_view();
        assert_eq!(state.result_scroll, 6);
        assert!(state.search_list_state.selected().unwrap() < state.result_scroll + 6);
    }

    #[test]
    fn test_has_active_modal_detection() {
        let mut state = AppState::default();
        assert!(!state.has_active_modal());

        state.show_help = true;
        assert!(state.has_active_modal());
        state.show_help = false;

        state.show_theme_popup = true;
        assert!(state.has_active_modal());
        state.show_theme_popup = false;

        state.show_browse_popup = true;
        assert!(state.has_active_modal());
        state.show_browse_popup = false;
        state.show_provider_popup = true;
        assert!(state.has_active_modal());
        state.show_provider_popup = false;

        state.addon_manager_popup = true;
        assert!(state.has_active_modal());
        state.addon_manager_popup = false;

        state.tv_config_popup = true;
        assert!(state.has_active_modal());
        state.tv_config_popup = false;

        state.player_picker_popup = true;
        assert!(state.has_active_modal());
        state.player_picker_popup = false;

        state.subtitle_popup = true;
        assert!(state.has_active_modal());
        state.subtitle_popup = false;

        state.is_download_subtitle_popup = true;
        assert!(state.has_active_modal());
        state.is_download_subtitle_popup = false;

        state.show_season_download_confirm = true;
        assert!(state.has_active_modal());
        state.show_season_download_confirm = false;

        state.show_episode_download_confirm = true;
        assert!(state.has_active_modal());
        state.show_episode_download_confirm = false;

        state.update_available = Some(("v2.0.0".to_string(), "Notes".to_string()));
        assert!(state.has_active_modal());
        state.input_mode = InputMode::Editing;
        assert!(!state.has_active_modal());
        state.input_mode = InputMode::Normal;
        state.update_available = None;

        state.input_mode = InputMode::Editing;
        state.search_suggestions = vec!["suggestion".to_string()];
        assert!(state.has_active_modal());
        state.search_suggestions.clear();
        assert!(!state.has_active_modal());
        state.input_mode = InputMode::Normal;

        assert!(!state.has_active_modal());
    }

    #[test]
    fn test_favorites_landing_visible_conditions() {
        let mut state = AppState::default();
        state.favorites.items.clear();
        state.streaming_enabled = true;
        assert!(!state.favorites_landing_visible());
        state.favorites.items.push(crate::favorites::FavoriteItem {
            provider: "moviebox".to_string(),
            subject_id: "fav-1".to_string(),
            title: "Favorite Movie".to_string(),
            cover_url: None,
            stype: 1,
            release_year: "2024".to_string(),
            added_at: 0,
        });
        assert!(state.favorites_landing_visible());

        state.input_mode = InputMode::Editing;
        state.search_suggestions = vec!["suggestion".to_string()];
        assert!(!state.favorites_landing_visible());

        state.search_suggestions.clear();
        assert!(state.favorites_landing_visible());

        state.search_suggestions = vec!["suggestion".to_string()];
        state.input_mode = InputMode::Normal;
        assert!(state.favorites_landing_visible());
    }

    #[test]
    fn test_cycle_settings_player_dynamic_detection() {
        let mut state = AppState {
            available_players: vec![PlayerKind::Mpv],
            ..Default::default()
        };
        assert_eq!(state.settings_player_choices(), vec!["mpv"]);

        state.default_player = None;
        state.cycle_settings_player(true);
        assert_eq!(state.default_player.as_deref(), Some("mpv"));

        state.cycle_settings_player(true);
        assert_eq!(state.default_player.as_deref(), Some("mpv"));

        state.cycle_settings_player(false);
        assert_eq!(state.default_player.as_deref(), Some("mpv"));
    }
    #[test]
    fn test_step_list_selection() {
        let mut state = ListState::default();
        step_list_selection(&mut state, 0, 5);
        assert_eq!(state.selected(), None);

        step_list_selection(&mut state, 10, 5);
        assert_eq!(state.selected(), Some(5));

        step_list_selection(&mut state, 10, 10);
        assert_eq!(state.selected(), Some(9));

        step_list_selection(&mut state, 10, -3);
        assert_eq!(state.selected(), Some(6));

        step_list_selection(&mut state, 10, -20);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_clear_poster_cache() {
        let mut state = AppState::default();
        state.in_flight_posters.insert("p1".to_string());
        state
            .failed_posters
            .put("p2".to_string(), std::time::Instant::now());
        state.clear_poster_cache();
        assert!(state.in_flight_posters.is_empty());
        assert!(state.failed_posters.is_empty());
        assert!(state.poster_image.is_none());
        assert!(state.poster_protocol.is_none());
    }
}
