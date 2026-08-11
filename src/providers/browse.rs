use serde_json::Value;
use std::cmp::Ordering;

/// Curated discovery views for the Browse feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseView {
    Trending,
    TopRatedAllTime,
    TopRatedRecent,
    Popular,
}

impl BrowseView {
    pub const ALL: [Self; 4] = [
        Self::Trending,
        Self::TopRatedAllTime,
        Self::TopRatedRecent,
        Self::Popular,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Trending => "Trending",
            Self::TopRatedAllTime => "Top Rated · All-Time",
            Self::TopRatedRecent => "Top Rated · Last 30 Days",
            Self::Popular => "Popular",
        }
    }

    /// MovieBox homepage tab ids aggregated for this view (movies tab first).
    pub const fn tabs(self) -> &'static [&'static str] {
        &["2"]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Desc,
    Asc,
}

impl SortOrder {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Desc => Self::Asc,
            Self::Asc => Self::Desc,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Desc => "Descending",
            Self::Asc => "Ascending",
        }
    }

    pub const fn arrow(self) -> &'static str {
        match self {
            Self::Desc => "↓",
            Self::Asc => "↑",
        }
    }
}

/// Flattens every subject embedded in a `tab-operating` homepage payload:
/// banner slides, customData items, and inline subject lists.
pub fn collect_feed_subjects(payload: &Value) -> Vec<Value> {
    let mut subjects = Vec::new();
    let Some(items) = payload.get("items").and_then(|i| i.as_array()) else {
        return subjects;
    };
    for item in items {
        if let Some(banner) = item
            .get("banner")
            .and_then(|b| b.get("banners"))
            .and_then(|b| b.as_array())
        {
            for b in banner {
                if let Some(subject) = b.get("subject") {
                    subjects.push(subject.clone());
                }
            }
        }
        if let Some(custom_data) = item
            .get("customData")
            .and_then(|c| c.get("items"))
            .and_then(|i| i.as_array())
        {
            for c in custom_data {
                if let Some(subject) = c.get("subject") {
                    subjects.push(subject.clone());
                }
            }
        }
        if let Some(list) = item.get("subjects").and_then(|s| s.as_array()) {
            for subject in list {
                subjects.push(subject.clone());
            }
        }
    }
    subjects
}

/// IMDb rating from a subject payload (`imdbRatingValue` or `imdbRate`).
pub fn subject_rating(value: &Value) -> Option<f64> {
    value
        .get("imdbRatingValue")
        .or_else(|| value.get("imdbRate"))
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| v.as_f64())
        })
        .filter(|r| *r > 0.0 && *r <= 10.0)
}

/// IMDb rating as a display string (e.g. "8.4"), preserving the payload's own
/// text form where possible.
pub fn subject_rating_text(value: &Value) -> Option<String> {
    let raw = value
        .get("imdbRatingValue")
        .or_else(|| value.get("imdbRate"))?;
    if let Some(text) = raw.as_str() {
        if !text.trim().is_empty() && subject_rating(value).is_some() {
            return Some(text.trim().to_string());
        }
        return None;
    }
    // Numeric payloads (no quotes): format from the parsed value.
    subject_rating(value).map(|rating| format!("{rating:.1}"))
}

fn subject_viewers(value: &Value) -> u64 {
    value.get("viewers").and_then(|v| v.as_u64()).unwrap_or(0)
}

fn subject_want_to_see(value: &Value) -> u64 {
    value
        .get("wantToSeeCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

/// Release date as unix seconds, tolerating `YYYY-MM-DD`, `YYYY`, or epoch
/// millis/seconds payloads.
pub fn subject_release_epoch(value: &Value) -> Option<u64> {
    let raw = value.get("releaseDate")?;
    if let Some(n) = raw.as_u64() {
        return Some(if n > 1_000_000_000_000 { n / 1000 } else { n });
    }
    let text = raw.as_str()?;
    let date_part = text.split('T').next().unwrap_or(text);
    let mut parts = date_part.split('-');
    let year: u32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let day: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    Some(days_from_civil(year, month, day).max(0) as u64 * 86_400)
}

/// Drops subjects whose release date is older than `days`.
pub fn filter_recent(subjects: &mut Vec<Value>, days: u64) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff = now.saturating_sub(days * 86_400);
    subjects.retain(|s| subject_release_epoch(s).is_some_and(|ts| ts >= cutoff));
}

/// Client-side sort for a curated browse view. Unrated items always sink last,
/// regardless of direction.
pub fn sort_subjects(subjects: &mut [Value], view: BrowseView, order: SortOrder) {
    let desc = order == SortOrder::Desc;
    match view {
        BrowseView::Trending => {
            let key = subject_viewers;
            if desc {
                subjects.sort_by_key(|b| std::cmp::Reverse(key(b)));
            } else {
                subjects.sort_by_key(key);
            }
        }
        BrowseView::Popular => {
            let key = subject_want_to_see;
            if desc {
                subjects.sort_by_key(|b| std::cmp::Reverse(key(b)));
            } else {
                subjects.sort_by_key(key);
            }
        }
        BrowseView::TopRatedAllTime | BrowseView::TopRatedRecent => {
            subjects.sort_by(|a, b| match (subject_rating(a), subject_rating(b)) {
                (Some(x), Some(y)) => {
                    let ord = x.partial_cmp(&y).unwrap_or(Ordering::Equal);
                    if desc { ord.reverse() } else { ord }
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
        }
    }
}

/// Fetches, dedupes, filters and sorts the aggregate subject pool for a browse
/// view. Uses the shared homepage cache, so repeat opens are cheap.
pub async fn fetch_browse_feed(
    client: &crate::providers::moviebox::client::MovieBoxClient,
    view: BrowseView,
    sort: SortOrder,
) -> Result<Vec<Value>, String> {
    let mut subjects: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for &tab in view.tabs() {
        let tab_owned = tab.to_string();
        let cached =
            tokio::task::spawn_blocking(move || crate::cache::get_homepage_cache(&tab_owned, 1))
                .await
                .unwrap_or(None);

        let payload = match cached {
            Some(p) => p,
            None => match client.get_homepage(tab, 1).await {
                Ok(p) => {
                    let tab_save = tab.to_string();
                    let p_clone = p.clone();
                    tokio::task::spawn_blocking(move || {
                        crate::cache::set_homepage_cache(&tab_save, 1, &p_clone);
                    });
                    p
                }
                Err(error) => {
                    log::warn!(
                        "browse: tab {tab} fetch failed: {error} [{}]",
                        crate::logging::sanitize_url(&format!("homepage/{tab}"))
                    );
                    continue;
                }
            },
        };

        for subject in collect_feed_subjects(&payload) {
            let id = subject
                .get("subjectId")
                .map(|v| v.to_string())
                .unwrap_or_default();
            if id.is_empty() || !seen.insert(id) {
                continue;
            }
            subjects.push(subject);
        }
    }

    if view == BrowseView::TopRatedRecent {
        filter_recent(&mut subjects, 30);
    }
    sort_subjects(&mut subjects, view, sort);
    subjects.truncate(200);
    Ok(subjects)
}

/// Days from civil date to unix epoch (Howard Hinnant's algorithm).
fn days_from_civil(year: u32, month: u32, day: u32) -> i64 {
    let y = year as i64 - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn subject(id: &str, rating: &str, viewers: u64, want: u64, release: &str) -> Value {
        json!({
            "subjectId": id,
            "title": id,
            "subjectType": 1,
            "imdbRatingValue": rating,
            "viewers": viewers,
            "wantToSeeCount": want,
            "releaseDate": release,
        })
    }

    #[test]
    fn rating_text_preserves_string_and_rejects_empty() {
        let with_str = json!({"imdbRatingValue": "8.4"});
        assert_eq!(subject_rating_text(&with_str).as_deref(), Some("8.4"));
        let with_num = json!({"imdbRate": "7.7"});
        assert_eq!(subject_rating_text(&with_num).as_deref(), Some("7.7"));
        let empty = json!({"imdbRatingValue": ""});
        assert_eq!(subject_rating_text(&empty), None);
        let missing = json!({"title": "x"});
        assert_eq!(subject_rating_text(&missing), None);
        let invalid = json!({"imdbRatingValue": "abc"});
        assert_eq!(subject_rating_text(&invalid), None);
        let numeric = json!({"imdbRatingValue": 8.4});
        assert_eq!(subject_rating_text(&numeric).as_deref(), Some("8.4"));
        let numeric_whole = json!({"imdbRate": 9.0});
        assert_eq!(subject_rating_text(&numeric_whole).as_deref(), Some("9.0"));
    }

    #[test]
    fn days_from_civil_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        // 56 years to 2026-01-01 (14 leap days) + 212 days to Aug 1 + 10 = 20676
        assert_eq!(days_from_civil(2026, 8, 11), 20676);
    }

    #[test]
    fn release_epoch_parses_formats() {
        let v = subject("a", "8.0", 0, 0, "2026-08-11");
        let epoch = subject_release_epoch(&v).unwrap();
        assert_eq!(epoch, 20_676 * 86_400);
        assert_eq!(
            subject_release_epoch(&json!({"releaseDate": 1_700_000_000_000i64})),
            Some(1_700_000_000)
        );
    }

    #[test]
    fn filter_recent_keeps_only_new() {
        let mut subjects = vec![
            subject("old", "8.0", 0, 0, "2000-01-01"),
            subject("new", "7.0", 0, 0, "2026-08-01"),
            subject("today", "9.0", 0, 0, "2026-08-11"),
        ];
        filter_recent(&mut subjects, 30);
        assert_eq!(subjects.len(), 2);
        assert_eq!(subjects[0]["title"], "new");
        assert_eq!(subjects[1]["title"], "today");
    }

    #[test]
    fn sort_top_rated_desc_puts_unrated_last() {
        let mut subjects = vec![
            subject("low", "6.0", 0, 0, ""),
            subject("none", "", 0, 0, ""),
            subject("high", "9.0", 0, 0, ""),
        ];
        sort_subjects(&mut subjects, BrowseView::TopRatedAllTime, SortOrder::Desc);
        let titles: Vec<&str> = subjects
            .iter()
            .map(|s| s["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["high", "low", "none"]);
    }

    #[test]
    fn sort_top_rated_asc_keeps_unrated_last() {
        let mut subjects = vec![
            subject("high", "9.0", 0, 0, ""),
            subject("none", "", 0, 0, ""),
            subject("low", "6.0", 0, 0, ""),
        ];
        sort_subjects(&mut subjects, BrowseView::TopRatedAllTime, SortOrder::Asc);
        let titles: Vec<&str> = subjects
            .iter()
            .map(|s| s["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["low", "high", "none"]);
    }

    #[test]
    fn sort_trending_by_viewers() {
        let mut subjects = vec![
            subject("a", "", 10, 0, ""),
            subject("b", "", 500, 0, ""),
            subject("c", "", 300, 0, ""),
        ];
        sort_subjects(&mut subjects, BrowseView::Trending, SortOrder::Desc);
        let titles: Vec<&str> = subjects
            .iter()
            .map(|s| s["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["b", "c", "a"]);
    }

    #[test]
    fn sort_popular_by_want_to_see() {
        let mut subjects = vec![
            subject("a", "", 0, 5, ""),
            subject("b", "", 0, 90, ""),
            subject("c", "", 0, 30, ""),
        ];
        sort_subjects(&mut subjects, BrowseView::Popular, SortOrder::Desc);
        let titles: Vec<&str> = subjects
            .iter()
            .map(|s| s["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["b", "c", "a"]);
    }

    #[test]
    fn collect_feed_subjects_extracts_all_shapes() {
        let payload = json!({
            "items": [
                {"banner": {"banners": [{"subject": {"subjectId": "1"}}]}},
                {"customData": {"items": [{"subject": {"subjectId": "2"}}]}},
                {"subjects": [{"subjectId": "3"}]},
                {"title": "no subjects here"}
            ]
        });
        let ids: Vec<String> = collect_feed_subjects(&payload)
            .iter()
            .map(|s| s["subjectId"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["1", "2", "3"]);
    }
}
