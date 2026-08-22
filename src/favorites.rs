use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteItem {
    pub provider: String,
    pub subject_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub stype: i64,
    pub release_year: String,
    pub added_at: u64,
}

impl FavoriteItem {
    pub fn identity(&self) -> crate::models::SubjectIdentity<'_> {
        crate::models::SubjectIdentity {
            provider: &self.provider,
            subject_id: &self.subject_id,
            title: &self.title,
            stype: self.stype,
            release_year: &self.release_year,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct FavoritesManager {
    #[serde(default)]
    pub items: Vec<FavoriteItem>,
}

impl FavoritesManager {
    pub fn new() -> Self {
        match Self::favorites_file_path() {
            Some(path) => Self::load_from_path(&path),
            None => Self::default(),
        }
    }

    fn favorites_file_path() -> Option<PathBuf> {
        crate::config::favorites_path()
    }

    pub fn load_from_path(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(manager) = serde_json::from_str::<Self>(&content) {
                    return manager;
                }
            }
            let _ = fs::remove_file(path);
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::favorites_file_path() {
            self.save_to_path(&path);
        }
    }

    pub fn save_to_path(&self, path: &Path) {
        if let Ok(content) = serde_json::to_string(self) {
            if let Err(error) = crate::cache::atomic_write_file(path, content.as_bytes()) {
                log::warn!("failed to save favorites: {error}");
            }
        }
    }

    pub fn is_favorite(&self, identity: &crate::models::SubjectIdentity<'_>) -> bool {
        self.items
            .iter()
            .any(|item| item.identity().matches(identity))
    }

    /// Toggles the given item and returns `true` if it is now favorited,
    /// `false` if it was just removed.
    pub fn toggle(&mut self, item: FavoriteItem) -> bool {
        let now_favorited = if let Some(pos) = self
            .items
            .iter()
            .position(|existing| existing.identity().matches(&item.identity()))
        {
            self.items.remove(pos);
            false
        } else {
            self.items.push(item);
            true
        };
        self.save();
        now_favorited
    }

    pub fn remove(&mut self, identity: &crate::models::SubjectIdentity<'_>) {
        self.items.retain(|item| !item.identity().matches(identity));
        self.save();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.save();
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
    ) -> FavoriteItem {
        FavoriteItem {
            provider: provider.to_string(),
            subject_id: subject_id.to_string(),
            title: title.to_string(),
            cover_url: None,
            stype,
            release_year: release_year.to_string(),
            added_at: 1700000000,
        }
    }

    #[test]
    fn test_toggle_idempotency() {
        let mut manager = FavoritesManager::default();
        let item = dummy_item("moviebox", "mb_1", "Dune", 1, "2021");

        assert!(manager.toggle(item.clone()));
        assert_eq!(manager.items.len(), 1);

        assert!(!manager.toggle(item.clone()));
        assert!(manager.items.is_empty());

        assert!(manager.toggle(item));
        assert_eq!(manager.items.len(), 1);
    }

    #[test]
    fn test_is_favorite_identity_dedup() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_1", "Breaking Bad", 2, "2008"));

        let target = crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "mb_1",
            title: "Breaking Bad",
            stype: 2,
            release_year: "2008",
        };
        assert!(manager.is_favorite(&target));

        let different_series = crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "mb_2",
            title: "Breaking Bad",
            stype: 1,
            release_year: "2008",
        };
        assert!(!manager.is_favorite(&different_series));
    }

    #[test]
    fn test_remakes_with_different_years_are_distinct() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "", "Halloween", 1, "1978"));

        let remake = crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "",
            title: "Halloween",
            stype: 1,
            release_year: "2018",
        };
        assert!(!manager.is_favorite(&remake));
    }

    #[test]
    fn test_movie_vs_series_same_title_distinct() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_home_1", "Home", 1, "2015"));

        let series = crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "mb_home_2",
            title: "Home",
            stype: 2,
            release_year: "2020",
        };
        assert!(!manager.is_favorite(&series));
    }

    #[test]
    fn test_same_title_across_providers_distinct() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_1", "Dune", 1, "2021"));

        let addon = crate::models::SubjectIdentity {
            provider: "addons",
            subject_id: "tt1160419",
            title: "Dune",
            stype: 1,
            release_year: "2021",
        };
        assert!(!manager.is_favorite(&addon));
    }

    #[test]
    fn test_empty_subject_id_fallback() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "", "Inception", 1, "2010"));

        let same = crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "",
            title: "Inception",
            stype: 1,
            release_year: "2010",
        };
        assert!(manager.is_favorite(&same));
    }

    #[test]
    fn test_unstar_removal() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_1", "Dune", 1, "2021"));
        manager
            .items
            .push(dummy_item("moviebox", "mb_2", "Arrival", 1, "2016"));

        manager.remove(&crate::models::SubjectIdentity {
            provider: "moviebox",
            subject_id: "mb_1",
            title: "Dune",
            stype: 1,
            release_year: "2021",
        });

        assert_eq!(manager.items.len(), 1);
        assert_eq!(manager.items[0].subject_id, "mb_2");
    }

    #[test]
    fn test_clear() {
        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_1", "Dune", 1, "2021"));
        manager.clear();
        assert!(manager.items.is_empty());
    }

    #[test]
    fn test_persistence_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!(
            "mb_test_favorites_{}_roundtrip",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("favorites.json");

        let mut manager = FavoritesManager::default();
        manager
            .items
            .push(dummy_item("moviebox", "mb_1", "Dune", 1, "2021"));
        manager.save_to_path(&path);

        let loaded = FavoritesManager::load_from_path(&path);
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].subject_id, "mb_1");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_corrupt_file_recovery() {
        let temp_dir =
            std::env::temp_dir().join(format!("mb_test_favorites_{}_corrupt", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("favorites.json");
        std::fs::write(&path, "not valid json").unwrap();

        let loaded = FavoritesManager::load_from_path(&path);
        assert!(loaded.items.is_empty());
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_no_cap_on_favorites_count() {
        let mut manager = FavoritesManager::default();
        for i in 0..250 {
            manager.items.push(dummy_item(
                "moviebox",
                &format!("mb_{i}"),
                "Title",
                1,
                "2020",
            ));
        }
        assert_eq!(manager.items.len(), 250);
    }
}
