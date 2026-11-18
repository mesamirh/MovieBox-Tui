use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchHistoryItem {
    pub provider: String,
    pub subject_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub stype: i64,
    pub release_year: String,
    pub season: usize,
    pub episode: usize,
    pub timestamp: u64,
    #[serde(default)]
    pub duration_seconds: Option<u64>,
    #[serde(default)]
    pub progress_seconds: u64,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub stream_filename: Option<String>,
}

impl WatchHistoryItem {
    pub fn from_details(
        provider: &str,
        details: &crate::models::MediaDetails,
        season: usize,
        episode: usize,
    ) -> Self {
        let title = crate::providers::moviebox::clean_moviebox_title(&details.title);
        let title = if title.trim().is_empty() {
            details.title.clone()
        } else {
            title.to_string()
        };
        let stype = if details.is_series() { 2 } else { 1 };
        let release_year = details.year.clone().unwrap_or_default();
        let cover_url = details.cover_url().map(|s| s.to_string());
        let duration_seconds = details
            .duration
            .as_deref()
            .and_then(crate::text_util::parse_duration_seconds);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            provider: provider.to_string(),
            subject_id: details.id.value.clone(),
            title,
            cover_url,
            stype,
            release_year,
            season,
            episode,
            timestamp,
            duration_seconds,
            progress_seconds: 0,
            completed: false,
            stream_filename: None,
        }
    }

    pub fn identity(&self) -> crate::models::SubjectIdentity<'_> {
        crate::models::SubjectIdentity {
            provider: &self.provider,
            subject_id: &self.subject_id,
            title: &self.title,
            stype: self.stype,
            release_year: &self.release_year,
        }
    }

    pub fn is_in_progress(&self) -> bool {
        if self.completed {
            return false;
        }
        if self.progress_seconds < 30 {
            return false;
        }
        let Some(dur) = self.duration_seconds else {
            return false;
        };
        if dur == 0 || self.progress_seconds >= (dur as f64 * 0.90) as u64 {
            return false;
        }
        true
    }

    pub fn progress_percentage(&self) -> Option<f32> {
        self.duration_seconds
            .filter(|&d| d > 0)
            .map(|d| ((self.progress_seconds as f32 / d as f32) * 100.0).clamp(0.0, 100.0))
    }

    pub fn progress_bar_parts(&self, width: usize) -> (String, String) {
        let pct = self.progress_percentage().unwrap_or(0.0) / 100.0;
        let filled = if pct > 0.0 {
            ((width as f32 * pct).round() as usize).max(1).min(width)
        } else {
            0
        };
        let empty = width.saturating_sub(filled);
        ("━".repeat(filled), "─".repeat(empty))
    }

    pub fn progress_bar(&self, width: usize) -> String {
        let (filled, empty) = self.progress_bar_parts(width);
        format!("{filled}{empty}")
    }

    pub fn formatted_progress(&self) -> String {
        let pos = crate::text_util::format_duration(self.progress_seconds);
        if let Some(dur) = self.duration_seconds {
            if dur > 0 {
                return format!("{} / {}", pos, crate::text_util::format_duration(dur));
            }
        }
        pos
    }

    pub fn formatted_remaining(&self) -> Option<String> {
        let dur = self.duration_seconds?;
        if dur <= self.progress_seconds {
            return None;
        }
        let remaining = dur.saturating_sub(self.progress_seconds);
        if remaining < 60 {
            Some("< 1m left".to_string())
        } else if remaining < 3600 {
            Some(format!("{}m left", remaining / 60))
        } else {
            let h = remaining / 3600;
            let m = (remaining % 3600) / 60;
            if m == 0 {
                Some(format!("{h}h left"))
            } else {
                Some(format!("{h}h {m}m left"))
            }
        }
    }

    pub fn formatted_relative_time(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if self.timestamp == 0 || now < self.timestamp {
            return "recently".to_string();
        }
        let delta = now - self.timestamp;
        if delta < 60 {
            "just now".to_string()
        } else if delta < 3600 {
            format!("{}m ago", delta / 60)
        } else if delta < 86400 {
            format!("{}h ago", delta / 3600)
        } else if delta < 172800 {
            "Yesterday".to_string()
        } else if delta < 604800 {
            format!("{}d ago", delta / 86400)
        } else {
            format!("{}w ago", delta / 604800)
        }
    }

    pub fn to_search_result(&self) -> crate::models::SearchResult {
        crate::models::SearchResult {
            id: self.subject_id.clone(),
            title: self.title.clone(),
            stype: self.stype,
            release_year: self.release_year.clone(),
            cover_url: self.cover_url.clone(),
            season: self.season,
            episode: self.episode.max(1),
            provider: crate::models::ProviderKind::parse(&self.provider)
                .unwrap_or(crate::models::ProviderKind::MovieBox),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPlaybackState {
    pub provider: String,
    pub subject_id: String,
    pub season: usize,
    pub episode: usize,
    pub progress_seconds: u64,
    pub duration_seconds: Option<u64>,
    pub completed: bool,
    pub timestamp: u64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub stype: Option<i64>,
    #[serde(default)]
    pub release_year: Option<String>,
    #[serde(default)]
    pub stream_filename: Option<String>,
}

impl PendingPlaybackState {
    pub fn from_item(
        item: &WatchHistoryItem,
        progress: u64,
        duration: Option<u64>,
        completed: bool,
    ) -> Self {
        Self {
            provider: item.provider.clone(),
            subject_id: item.subject_id.clone(),
            season: item.season,
            episode: item.episode,
            progress_seconds: progress,
            duration_seconds: duration,
            completed,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            title: Some(item.title.clone()),
            cover_url: item.cover_url.clone(),
            stype: Some(item.stype),
            release_year: Some(item.release_year.clone()),
            stream_filename: item.stream_filename.clone(),
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct HistoryManager {
    #[serde(default)]
    watched: HashSet<String>,
    #[serde(default)]
    pub recent: Vec<WatchHistoryItem>,
}

impl HistoryManager {
    pub fn new() -> Self {
        let mut history = if let Some(path) = crate::config::history_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(mut hist) = serde_json::from_str::<Self>(&content) {
                        hist.hydrate_watched_index();
                        hist
                    } else {
                        crate::config::rotate_corrupt_file(&path, "watch history");
                        Self::default()
                    }
                } else {
                    Self::default()
                }
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        history.reconcile_pending_playback_states();
        history
    }

    pub fn save(&self) {
        if let Some(path) = crate::config::history_path() {
            if let Ok(content) = serde_json::to_string(self) {
                if let Err(error) = crate::cache::atomic_write_file(&path, content.as_bytes()) {
                    log::warn!("failed to save watch history: {error}");
                }
            }
        }
    }

    fn same_provider(a: &str, b: &str) -> bool {
        crate::providers::models::ProviderKind::parse(a)
            .zip(crate::providers::models::ProviderKind::parse(b))
            .map_or_else(
                || a.trim().eq_ignore_ascii_case(b.trim()),
                |(p1, p2)| p1 == p2,
            )
    }

    fn key(provider: &str, subject_id: &str, season: usize, episode: usize) -> String {
        let canon_provider = crate::providers::models::ProviderKind::parse(provider)
            .map(|p| p.cache_key())
            .unwrap_or(provider);
        format!("{canon_provider}::{subject_id}::{season}::{episode}")
    }

    pub fn is_same_show(a: &WatchHistoryItem, b: &WatchHistoryItem) -> bool {
        a.identity().matches(&b.identity())
    }

    fn hydrate_watched_index(&mut self) {
        if self.watched.is_empty() {
            self.watched = self
                .recent
                .iter()
                .filter(|item| item.completed)
                .map(|item| Self::key(&item.provider, &item.subject_id, item.season, item.episode))
                .collect();
        } else {
            for item in &mut self.recent {
                let key = Self::key(&item.provider, &item.subject_id, item.season, item.episode);
                if self.watched.contains(&key) {
                    item.completed = true;
                } else if item.completed {
                    self.watched.insert(key);
                }
            }
        }
        self.consolidate_recent();
    }

    fn consolidate_recent(&mut self) {
        let original_len = self.recent.len();
        let mut consolidated: Vec<WatchHistoryItem> = Vec::new();

        let mut sorted = self.recent.clone();
        sorted.sort_by_key(|item| item.timestamp);

        for item in sorted {
            if let Some(existing) = consolidated
                .iter_mut()
                .find(|e| Self::is_same_show(e, &item))
            {
                if item.timestamp >= existing.timestamp {
                    let cover = item
                        .cover_url
                        .clone()
                        .or_else(|| existing.cover_url.clone());
                    *existing = item;
                    existing.cover_url = cover;
                } else if existing.cover_url.is_none() && item.cover_url.is_some() {
                    existing.cover_url = item.cover_url.clone();
                }
            } else {
                consolidated.push(item);
            }
        }

        self.recent = consolidated;
        if self.recent.len() != original_len {
            self.save();
        }
    }

    pub fn get_item(
        &self,
        provider: &str,
        subject_id: &str,
        season: usize,
        episode: usize,
        title: Option<&str>,
    ) -> Option<&WatchHistoryItem> {
        self.recent.iter().find(|i| {
            let same_provider = Self::same_provider(&i.provider, provider);

            if same_provider && i.subject_id == subject_id {
                if i.stype == 1 {
                    return true;
                }
                if i.season == season && i.episode == episode {
                    return true;
                }
            }

            if let Some(t) = title {
                let clean_i = crate::providers::moviebox::clean_moviebox_title(&i.title);
                let clean_t = crate::providers::moviebox::clean_moviebox_title(t);
                if !clean_i.is_empty() && clean_i.eq_ignore_ascii_case(clean_t) {
                    if i.stype == 1 {
                        return true;
                    }
                    if i.season == season && i.episode == episode {
                        return true;
                    }
                }
            }

            false
        })
    }

    pub fn mark_watched(&mut self, mut item: WatchHistoryItem) {
        item.completed = true;
        let key = Self::key(&item.provider, &item.subject_id, item.season, item.episode);
        self.watched.insert(key);

        if item.cover_url.is_none() {
            if let Some(existing) = self
                .recent
                .iter()
                .find(|i| Self::is_same_show(i, &item))
                .and_then(|i| i.cover_url.clone())
            {
                item.cover_url = Some(existing);
            }
        }

        self.recent.retain(|i| !Self::is_same_show(i, &item));
        self.recent.push(item);

        if self.recent.len() > 100 {
            let excess = self.recent.len() - 100;
            self.recent.drain(0..excess);
        }
        self.save();
    }

    pub fn update_progress(
        &mut self,
        mut item: WatchHistoryItem,
        progress: u64,
        duration: Option<u64>,
        completed: bool,
    ) {
        item.progress_seconds = progress;
        item.duration_seconds = duration;
        item.completed = completed;
        item.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let key = Self::key(&item.provider, &item.subject_id, item.season, item.episode);
        if completed {
            self.watched.insert(key);
        } else {
            self.watched.remove(&key);
        }

        if let Some(existing) = self.recent.iter().find(|i| Self::is_same_show(i, &item)) {
            if item.cover_url.is_none() {
                item.cover_url = existing.cover_url.clone();
            }
            if existing.season == item.season
                && existing.episode == item.episode
                && existing.progress_seconds >= progress
                && (existing.timestamp >= item.timestamp
                    || item.timestamp.saturating_sub(existing.timestamp) < 60)
            {
                return;
            }
        }

        self.recent.retain(|i| !Self::is_same_show(i, &item));
        self.recent.push(item);

        if self.recent.len() > 100 {
            let excess = self.recent.len() - 100;
            self.recent.drain(0..excess);
        }
        self.save();
    }
    pub fn record_start(&mut self, item: &WatchHistoryItem, start_pos: u64) {
        let mut new_item = item.clone();
        if let Some(existing) = self.recent.iter().find(|i| Self::is_same_show(i, item)) {
            if new_item.cover_url.is_none() {
                new_item.cover_url = existing.cover_url.clone();
            }
            if existing.season == new_item.season && existing.episode == new_item.episode {
                new_item.progress_seconds = existing.progress_seconds.max(start_pos);
                new_item.duration_seconds = existing.duration_seconds.or(new_item.duration_seconds);
                new_item.completed = existing.completed;
            } else {
                new_item.progress_seconds = start_pos;
            }
        } else {
            new_item.progress_seconds = start_pos;
        }
        new_item.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.recent.retain(|i| !Self::is_same_show(i, &new_item));
        self.recent.push(new_item);

        if self.recent.len() > 100 {
            let excess = self.recent.len() - 100;
            self.recent.drain(0..excess);
        }
        self.save();
    }

    pub fn remove(&mut self, provider: &str, subject_id: &str, season: usize, episode: usize) {
        let key = Self::key(provider, subject_id, season, episode);
        self.watched.remove(&key);
        self.recent.retain(|i| {
            let same_provider = Self::same_provider(&i.provider, provider);
            !(same_provider
                && i.subject_id == subject_id
                && i.season == season
                && i.episode == episode)
        });
        self.save();
    }

    pub fn clear(&mut self) {
        self.watched.clear();
        self.recent.clear();
        self.save();
    }

    pub fn update_cover_url(&mut self, subject_id: &str, cover_url: &str) {
        let mut modified = false;
        for item in &mut self.recent {
            if item.subject_id == subject_id && item.cover_url.is_none() {
                item.cover_url = Some(cover_url.to_string());
                modified = true;
            }
        }
        if modified {
            self.save();
        }
    }

    pub fn is_watched(
        &self,
        provider: &str,
        subject_id: &str,
        season: usize,
        episode: usize,
    ) -> bool {
        let key = Self::key(provider, subject_id, season, episode);
        if self.watched.contains(&key) {
            return true;
        }
        if let Some(item) = self.get_item(provider, subject_id, season, episode, None) {
            return item.completed;
        }
        false
    }

    pub fn reconcile_pending_playback_states(&mut self) {
        if let Some(dir) = crate::config::playback_state_dir() {
            if self.reconcile_from_dir(&dir) {
                self.save();
            }
        }
    }

    pub fn reconcile_from_dir(&mut self, dir: &std::path::Path) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };

        let mut pending_files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(state) = serde_json::from_str::<PendingPlaybackState>(&content) {
                        pending_files.push((path, state));
                    }
                }
            }
        }

        pending_files.sort_by_key(|(_, s)| s.timestamp);

        let mut modified = false;
        for (path, state) in pending_files {
            let key = Self::key(
                &state.provider,
                &state.subject_id,
                state.season,
                state.episode,
            );

            if state.completed {
                if self.watched.insert(key.clone()) {
                    modified = true;
                }
            }

            if let Some(existing) = self.recent.iter_mut().find(|i| {
                let same_provider = Self::same_provider(&i.provider, &state.provider);
                if !same_provider {
                    return false;
                }
                if i.subject_id == state.subject_id {
                    if i.stype == 1 {
                        return true;
                    }
                    return i.season == state.season && i.episode == state.episode;
                }
                false
            }) {
                existing.progress_seconds = state.progress_seconds;
                existing.duration_seconds = state.duration_seconds;
                existing.completed = state.completed;
                existing.timestamp = state.timestamp;
                if state.stream_filename.is_some() {
                    existing.stream_filename = state.stream_filename.clone();
                }
                if !existing.completed {
                    self.watched.remove(&key);
                }
                modified = true;
            } else if let Some(existing_series) = self.recent.iter_mut().find(|i| {
                let same_provider = Self::same_provider(&i.provider, &state.provider);
                same_provider && i.stype == 2 && i.subject_id == state.subject_id
            }) {
                if (state.season, state.episode)
                    >= (existing_series.season, existing_series.episode)
                    && state.timestamp >= existing_series.timestamp
                {
                    existing_series.season = state.season;
                    existing_series.episode = state.episode;
                    existing_series.progress_seconds = state.progress_seconds;
                    existing_series.duration_seconds = state.duration_seconds;
                    existing_series.completed = state.completed;
                    existing_series.timestamp = state.timestamp;
                    if state.stream_filename.is_some() {
                        existing_series.stream_filename = state.stream_filename.clone();
                    }
                    if !existing_series.completed {
                        self.watched.remove(&key);
                    }
                    modified = true;
                }
            } else if let Some(title) = state.title.filter(|t| !t.trim().is_empty()) {
                let new_item = WatchHistoryItem {
                    provider: state.provider.clone(),
                    subject_id: state.subject_id.clone(),
                    title,
                    cover_url: state.cover_url,
                    stype: state
                        .stype
                        .unwrap_or(if state.season > 0 || state.episode > 0 {
                            2
                        } else {
                            1
                        }),
                    release_year: state.release_year.unwrap_or_default(),
                    season: state.season,
                    episode: state.episode,
                    timestamp: state.timestamp,
                    duration_seconds: state.duration_seconds,
                    progress_seconds: state.progress_seconds,
                    completed: state.completed,
                    stream_filename: state.stream_filename,
                };
                if new_item.completed {
                    self.watched.insert(key);
                }
                self.recent.retain(|i| !Self::is_same_show(i, &new_item));
                self.recent.push(new_item);
                if self.recent.len() > 100 {
                    let excess = self.recent.len() - 100;
                    self.recent.drain(0..excess);
                }
                modified = true;
            }
            let _ = fs::remove_file(&path);
        }

        modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_item(
        provider: &str,
        subject_id: &str,
        title: &str,
        stype: i64,
        release_year: &str,
        season: usize,
        episode: usize,
    ) -> WatchHistoryItem {
        WatchHistoryItem {
            provider: provider.to_string(),
            subject_id: subject_id.to_string(),
            title: title.to_string(),
            cover_url: None,
            stype,
            release_year: release_year.to_string(),
            season,
            episode,
            timestamp: 1000,
            duration_seconds: Some(3600),
            progress_seconds: 1800,
            completed: false,
            stream_filename: None,
        }
    }

    #[test]
    fn test_is_same_show_canonical_identity() {
        let ep1 = dummy_item("moviebox", "mb_100", "Breaking Bad", 2, "2008", 1, 1);
        let ep2 = dummy_item("moviebox", "mb_100", "Breaking Bad", 2, "2008", 1, 2);
        assert!(HistoryManager::is_same_show(&ep1, &ep2));
    }

    #[test]
    fn test_is_same_show_movie_vs_series_differentiation() {
        let movie = dummy_item("moviebox", "mb_1", "Home", 1, "2015", 0, 0);
        let series = dummy_item("moviebox", "mb_2", "Home", 2, "2020", 1, 1);
        assert!(!HistoryManager::is_same_show(&movie, &series));
    }

    #[test]
    fn test_is_same_show_remakes_with_different_years() {
        let classic = dummy_item("moviebox", "mb_old", "Halloween", 1, "1978", 0, 0);
        let remake = dummy_item("moviebox", "mb_new", "Halloween", 1, "2018", 0, 0);
        assert!(!HistoryManager::is_same_show(&classic, &remake));
    }

    #[test]
    fn test_is_same_show_different_providers() {
        let mb = dummy_item("moviebox", "mb_1", "Dune", 1, "2021", 0, 0);
        let addon = dummy_item("addons", "tt1160419", "Dune", 1, "2021", 0, 0);
        assert!(!HistoryManager::is_same_show(&mb, &addon));
    }

    #[test]
    fn test_is_same_show_fallback_when_id_empty() {
        let a = dummy_item("moviebox", "", "Inception", 1, "2010", 0, 0);
        let b = dummy_item("moviebox", "", "Inception", 1, "2010", 0, 0);
        assert!(HistoryManager::is_same_show(&a, &b));

        let c = dummy_item("moviebox", "", "Inception", 1, "2020", 0, 0);
        assert!(!HistoryManager::is_same_show(&a, &c));
    }

    #[test]
    fn test_reconciliation_preserves_completed_episodes_when_recent_advanced() {
        let mut manager = HistoryManager {
            recent: vec![dummy_item("moviebox", "mb_series", "Show", 2, "2024", 1, 2)],
            watched: HashSet::new(),
        };

        let temp_dir = std::env::temp_dir().join(format!("mb_test_hist_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let state_file = temp_dir.join("moviebox_mb_series_1_1.json");
        let state = PendingPlaybackState {
            provider: "moviebox".to_string(),
            subject_id: "mb_series".to_string(),
            season: 1,
            episode: 1,
            progress_seconds: 3600,
            duration_seconds: Some(3600),
            completed: true,
            timestamp: 2000,
            title: None,
            cover_url: None,
            stype: None,
            release_year: None,
            stream_filename: None,
        };
        std::fs::write(&state_file, serde_json::to_string(&state).unwrap()).unwrap();

        let modified = manager.reconcile_from_dir(&temp_dir);
        assert!(modified);
        assert!(manager.is_watched("moviebox", "mb_series", 1, 1));
        assert_eq!(manager.recent.first().unwrap().episode, 2);
        assert!(!state_file.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    #[test]
    fn test_history_item_live_stream_without_duration_not_in_progress() {
        let mut item = dummy_item("tv", "live_1", "BBC News", 1, "", 0, 0);
        item.duration_seconds = None;
        item.progress_seconds = 600;
        assert!(!item.is_in_progress());

        item.duration_seconds = Some(0);
        assert!(!item.is_in_progress());

        item.duration_seconds = Some(3600);
        assert!(item.is_in_progress());
    }
}
