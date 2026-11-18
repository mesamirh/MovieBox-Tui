//! Keyless online subtitles: IMDb id resolution plus the OpenSubtitles v3 Stremio addon.
//!
//! Two keyless endpoints are used, neither of which needs an API key or an account:
//!
//! * `https://v3.sg.media-imdb.com/suggestion/x/{query}.json` resolves a title to an IMDb
//!   id (`tt…`) by matching title, optional year and kind (`movie` vs `series`).
//! * `https://opensubtitles-v3.strem.io/subtitles/{movie|series}/{tt-id}.json` lists
//!   directly downloadable UTF-8 `.srt` subtitles for that id.
//!
//! Everything that can be tested without a network is a pure function
//! ([`parse_suggestions`], [`pick_best_candidate`], [`resolve_suggestion`],
//! [`subtitles_path`], [`parse_subtitles`], [`language_rank`], [`language_label`]);
//! [`OpenSubtitlesClient`] only adds the HTTP transport.

use serde::Deserialize;

const IMDB_SUGGESTION_BASE: &str = "https://v3.sg.media-imdb.com/suggestion/x";
const OPENSUBTITLES_BASE: &str = "https://opensubtitles-v3.strem.io";
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Failure modes of the online subtitle source.
///
/// Every variant is recoverable: callers keep playback and provider captions working and
/// only lose the online list (spec S6).
#[derive(thiserror::Error, Debug)]
pub enum OpenSubtitlesError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid imdb id: {0}")]
    InvalidImdbId(String),
    #[error("parse error: {0}")]
    Parse(String),
}

impl OpenSubtitlesError {
    /// Short message suitable for the TUI/iOS status line.
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Network(err) if err.is_timeout() => "Subtitle search timed out.",
            Self::Network(_) => "Cannot reach the subtitle service.",
            Self::InvalidImdbId(_) => "No IMDb id for this title.",
            Self::Parse(_) => "Subtitle service returned an unexpected answer.",
        }
    }
}

/// Kind reported by the IMDb suggestion endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImdbKind {
    Movie,
    Series,
    Other,
}

impl ImdbKind {
    /// Reads the kind from `qid`, falling back to the human readable `q` field.
    fn from_suggestion(q: Option<&str>, qid: Option<&str>) -> Self {
        let raw = qid.or(q).unwrap_or_default().trim().to_ascii_lowercase();
        match raw.as_str() {
            "movie" | "feature" | "tv movie" | "tvmovie" => Self::Movie,
            "series" | "tvseries" | "tv series" | "miniseries" | "tvminiseries"
            | "tv mini-series" | "tv mini series" => Self::Series,
            _ => Self::Other,
        }
    }
}

/// One IMDb suggestion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImdbCandidate {
    pub id: String,
    pub title: String,
    pub year: Option<i32>,
    pub kind: ImdbKind,
    /// IMDb popularity rank; higher means more prominent.
    pub rank: Option<i64>,
}

/// True when `raw` is an IMDb title id (`tt` followed by digits).
pub fn is_imdb_id(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.len() > 2
        && trimmed.starts_with("tt")
        && trimmed[2..].bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Deserialize)]
struct SuggestionResponse {
    #[serde(default)]
    d: Vec<SuggestionEntry>,
}

#[derive(Deserialize)]
struct SuggestionEntry {
    id: Option<String>,
    l: Option<String>,
    q: Option<String>,
    qid: Option<String>,
    y: Option<serde_json::Value>,
    rank: Option<serde_json::Value>,
}

/// Numbers arrive either as JSON numbers or as strings depending on the field.
fn value_to_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    let value = value?;
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    if let Some(number) = value.as_f64() {
        return Some(number.round() as i64);
    }
    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
}

/// Parses the suggestion payload, dropping entries without a usable `tt` id or title.
pub fn parse_suggestions(json: &str) -> Result<Vec<ImdbCandidate>, OpenSubtitlesError> {
    let parsed: SuggestionResponse = serde_json::from_str(json)
        .map_err(|err| OpenSubtitlesError::Parse(format!("imdb suggestions: {err}")))?;
    Ok(parsed
        .d
        .into_iter()
        .filter_map(|entry| {
            let id = entry.id.unwrap_or_default().trim().to_string();
            if !is_imdb_id(&id) {
                return None;
            }
            let title = entry.l.unwrap_or_default().trim().to_string();
            if title.is_empty() {
                return None;
            }
            Some(ImdbCandidate {
                id,
                title,
                year: value_to_i64(entry.y.as_ref()).and_then(|year| i32::try_from(year).ok()),
                kind: ImdbKind::from_suggestion(entry.q.as_deref(), entry.qid.as_deref()),
                rank: value_to_i64(entry.rank.as_ref()),
            })
        })
        .collect())
}

/// Lowercases and collapses punctuation and whitespace so titles compare reliably.
fn normalize_title(title: &str) -> String {
    let mut normalized = String::with_capacity(title.len());
    let mut pending_separator = false;
    for character in title.chars() {
        if character.is_alphanumeric() {
            if pending_separator && !normalized.is_empty() {
                normalized.push(' ');
            }
            pending_separator = false;
            for lower in character.to_lowercase() {
                normalized.push(lower);
            }
        } else {
            pending_separator = true;
        }
    }
    normalized
}

/// Picks the most confident candidate for `title`, or `None` when nothing is convincing.
///
/// A candidate must carry a `tt` id, match the requested kind (movie vs series), and have
/// the same normalized title. A year is required to match exactly first and then within
/// one year; ties are broken by IMDb rank (first wins on an equal rank).
pub fn pick_best_candidate<'a>(
    candidates: &'a [ImdbCandidate],
    title: &str,
    year: Option<i32>,
    is_series: bool,
) -> Option<&'a ImdbCandidate> {
    let wanted_title = normalize_title(title);
    if wanted_title.is_empty() {
        return None;
    }
    let wanted_kind = if is_series {
        ImdbKind::Series
    } else {
        ImdbKind::Movie
    };
    let matching: Vec<&ImdbCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.kind == wanted_kind
                && is_imdb_id(&candidate.id)
                && normalize_title(&candidate.title) == wanted_title
        })
        .collect();
    let pool: Vec<&ImdbCandidate> = match year {
        None => matching,
        Some(year) => {
            let exact: Vec<&ImdbCandidate> = matching
                .iter()
                .copied()
                .filter(|candidate| candidate.year == Some(year))
                .collect();
            if exact.is_empty() {
                matching
                    .iter()
                    .copied()
                    .filter(|candidate| {
                        candidate
                            .year
                            .is_some_and(|candidate_year| (candidate_year - year).abs() <= 1)
                    })
                    .collect()
            } else {
                exact
            }
        }
    };
    let mut best: Option<&ImdbCandidate> = None;
    for candidate in pool {
        let is_better = best.is_none_or(|current| {
            candidate.rank.unwrap_or(i64::MIN) > current.rank.unwrap_or(i64::MIN)
        });
        if is_better {
            best = Some(candidate);
        }
    }
    best
}

/// Pure resolve step: parse the suggestion payload and keep only a confident match.
pub fn resolve_suggestion(
    json: &str,
    title: &str,
    year: Option<i32>,
    is_series: bool,
) -> Result<Option<ImdbCandidate>, OpenSubtitlesError> {
    let candidates = parse_suggestions(json)?;
    Ok(pick_best_candidate(&candidates, title, year, is_series).cloned())
}

/// Relative addon path for a movie (`None`) or an episode (`Some((season, episode))`).
pub fn subtitles_path(imdb_id: &str, season_episode: Option<(usize, usize)>) -> String {
    match season_episode {
        Some((season, episode)) => {
            format!("/subtitles/series/{imdb_id}:{season}:{episode}.json")
        }
        None => format!("/subtitles/movie/{imdb_id}.json"),
    }
}

fn subtitles_url(imdb_id: &str, season_episode: Option<(usize, usize)>) -> String {
    format!(
        "{OPENSUBTITLES_BASE}{}",
        subtitles_path(imdb_id.trim(), season_episode)
    )
}

/// One playable online subtitle returned by the addon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineSubtitle {
    /// Directly downloadable UTF-8 `.srt` URL.
    pub url: String,
    /// Addon language code (`spa`, `eng`, …).
    pub lang: String,
    /// Release/format label built from the release name, file name or group.
    pub label: String,
    /// Addon `releaseFormat` (`BluRay`, `WEB`, `Cam`, …) when reported.
    pub format: Option<String>,
}

#[derive(Deserialize)]
struct SubtitlesResponse {
    #[serde(default)]
    subtitles: Vec<SubtitleEntry>,
}

#[derive(Deserialize)]
struct SubtitleEntry {
    url: Option<String>,
    lang: Option<String>,
    #[serde(rename = "subtitleFileName")]
    subtitle_file_name: Option<String>,
    #[serde(rename = "movieReleaseName")]
    movie_release_name: Option<String>,
    #[serde(rename = "releaseGroup")]
    release_group: Option<String>,
    #[serde(rename = "releaseFormat")]
    release_format: Option<String>,
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn strip_srt_extension(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".srt") {
        &name[..name.len() - 4]
    } else {
        name
    }
}

fn release_label(
    release_name: Option<&str>,
    file_name: Option<&str>,
    group: Option<&str>,
) -> String {
    [release_name, file_name.map(strip_srt_extension), group]
        .into_iter()
        .flatten()
        .filter_map(non_empty)
        .next()
        .unwrap_or("Subtitle")
        .to_string()
}

impl OnlineSubtitle {
    fn from_entry(entry: SubtitleEntry) -> Option<Self> {
        let url = entry.url.unwrap_or_default().trim().to_string();
        if !crate::net::is_http_url(&url) {
            return None;
        }
        let label = release_label(
            entry.movie_release_name.as_deref(),
            entry.subtitle_file_name.as_deref(),
            entry.release_group.as_deref(),
        );
        Some(Self {
            url,
            lang: entry.lang.unwrap_or_default().trim().to_ascii_lowercase(),
            label,
            format: entry
                .release_format
                .as_deref()
                .and_then(non_empty)
                .map(str::to_string),
        })
    }
}

/// Parses the addon payload, skipping entries that cannot be downloaded.
pub fn parse_subtitles(json: &str) -> Result<Vec<OnlineSubtitle>, OpenSubtitlesError> {
    let parsed: SubtitlesResponse = serde_json::from_str(json)
        .map_err(|err| OpenSubtitlesError::Parse(format!("opensubtitles: {err}")))?;
    Ok(parsed
        .subtitles
        .into_iter()
        .filter_map(OnlineSubtitle::from_entry)
        .collect())
}

fn primary_language_tag(lang: &str) -> String {
    lang.trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Ordering weight for spec S4: Spanish `0`, English `1`, everything else `2`.
pub fn language_rank(lang: &str) -> u8 {
    match primary_language_tag(lang).as_str() {
        "spa" | "es" => 0,
        "eng" | "en" => 1,
        _ => 2,
    }
}

/// Label shown by the pickers: `Español`, `English`, or the uppercased language code.
pub fn language_label(lang: &str) -> String {
    match primary_language_tag(lang).as_str() {
        "spa" | "es" => "Español".to_string(),
        "eng" | "en" => "English".to_string(),
        _ => lang.trim().to_ascii_uppercase(),
    }
}

fn build_client() -> reqwest::Client {
    crate::net::http_client_builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .user_agent(crate::net::APP_HTTP_USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_default()
}

fn suggestion_url(query: &str) -> String {
    let encoded =
        percent_encoding::utf8_percent_encode(query.trim(), percent_encoding::NON_ALPHANUMERIC);
    format!("{IMDB_SUGGESTION_BASE}/{encoded}.json")
}

/// Keyless client for IMDb id resolution and the OpenSubtitles v3 addon.
#[derive(Clone, Debug)]
pub struct OpenSubtitlesClient {
    client: reqwest::Client,
}

impl Default for OpenSubtitlesClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenSubtitlesClient {
    pub fn new() -> Self {
        Self {
            client: build_client(),
        }
    }

    /// Resolves the IMDb id for `title`. `Ok(None)` means no confident match.
    pub async fn resolve_imdb_id(
        &self,
        title: &str,
        year: Option<i32>,
        is_series: bool,
    ) -> Result<Option<String>, OpenSubtitlesError> {
        if title.trim().is_empty() {
            return Ok(None);
        }
        let body = self.fetch_text(&suggestion_url(title)).await?;
        let candidate = resolve_suggestion(&body, title, year, is_series)?;
        if candidate.is_none() {
            log::debug!(
                "no confident imdb match for {title:?} (year {year:?}, series {is_series})"
            );
        }
        Ok(candidate.map(|candidate| candidate.id))
    }

    /// Lists online subtitles for a resolved IMDb id.
    ///
    /// `season_episode` is `Some((season, episode))` for series and `None` for movies.
    pub async fn fetch_subtitles(
        &self,
        imdb_id: &str,
        season_episode: Option<(usize, usize)>,
    ) -> Result<Vec<OnlineSubtitle>, OpenSubtitlesError> {
        if !is_imdb_id(imdb_id) {
            return Err(OpenSubtitlesError::InvalidImdbId(imdb_id.to_string()));
        }
        let body = self
            .fetch_text(&subtitles_url(imdb_id.trim(), season_episode))
            .await?;
        parse_subtitles(&body)
    }

    async fn fetch_text(&self, url: &str) -> Result<String, OpenSubtitlesError> {
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUGGESTIONS: &str = r#"{"d":[
        {"id":"tt1375666","l":"Inception","q":"feature","qid":"movie","y":2010,"tl":"2010","rank":185},
        {"id":"tt9999999","l":"Inception","q":"feature","qid":"movie","y":2011,"tl":"2011","rank":900}
    ]}"#;

    const SUBTITLES: &str = r#"{"subtitles":[
        {"id":"3728010","url":"https://subs5.strem.io/subtitles/1/file/1952382875","lang":"eng","SubEncoding":"UTF-8","subtitleFileName":"Inception.2010.CAM.Xvid-LKRG.srt","movieReleaseName":"Inception.2010.CAM.Xvid-LKRG","releaseGroup":"LKRG","releaseFormat":"Cam"},
        {"id":"3728011","url":"","lang":"spa","subtitleFileName":"Inception.2010.srt"},
        {"id":"3728012","lang":"spa","subtitleFileName":"Inception.2010.srt"}
    ]}"#;

    #[test]
    fn parse_suggestions_reads_verified_shape() {
        let candidates = parse_suggestions(SUGGESTIONS).expect("parse");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "tt1375666");
        assert_eq!(candidates[0].title, "Inception");
        assert_eq!(candidates[0].year, Some(2010));
        assert_eq!(candidates[0].kind, ImdbKind::Movie);
        assert_eq!(candidates[0].rank, Some(185));
    }

    #[test]
    fn exact_year_beats_higher_ranked_near_year() {
        let best = resolve_suggestion(SUGGESTIONS, "Inception", Some(2010), false)
            .expect("resolve")
            .expect("match");
        assert_eq!(best.id, "tt1375666");
    }

    #[test]
    fn near_year_within_one_is_accepted() {
        // 2012 is one year away from 2011 and two away from 2010, so only the second
        // candidate stays eligible.
        let best = resolve_suggestion(SUGGESTIONS, "inception", Some(2012), false)
            .expect("resolve")
            .expect("match");
        assert_eq!(best.id, "tt9999999");
    }

    #[test]
    fn year_beyond_one_returns_none() {
        assert!(
            resolve_suggestion(SUGGESTIONS, "Inception", Some(2005), false)
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn series_query_rejects_movie_candidate() {
        assert!(
            resolve_suggestion(SUGGESTIONS, "Inception", Some(2010), true)
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn series_candidate_accepted_for_series_query() {
        let payload = r#"{"d":[{"id":"tt0903747","l":"Breaking Bad","q":"TV series","qid":"tvSeries","y":2008,"rank":20}]}"#;
        let best = resolve_suggestion(payload, "Breaking Bad", Some(2008), true)
            .expect("resolve")
            .expect("match");
        assert_eq!(best.id, "tt0903747");
        assert_eq!(best.kind, ImdbKind::Series);
    }

    #[test]
    fn title_mismatch_returns_none() {
        assert!(
            resolve_suggestion(SUGGESTIONS, "Interstellar", None, false)
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn title_match_ignores_case_whitespace_and_punctuation() {
        let payload =
            r#"{"d":[{"id":"tt10872600","l":"Spider-Man: No Way Home","qid":"movie","y":2021}]}"#;
        let best = resolve_suggestion(payload, "  spider man   no way home ", Some(2021), false)
            .expect("resolve")
            .expect("match");
        assert_eq!(best.id, "tt10872600");
    }

    #[test]
    fn candidate_without_tt_id_is_rejected() {
        let payload = r#"{"d":[{"id":"nm1234567","l":"Inception","qid":"movie","y":2010}]}"#;
        assert!(parse_suggestions(payload).expect("parse").is_empty());
        let candidate = ImdbCandidate {
            id: "nm1234567".to_string(),
            title: "Inception".to_string(),
            year: Some(2010),
            kind: ImdbKind::Movie,
            rank: Some(10),
        };
        assert!(pick_best_candidate(&[candidate], "Inception", Some(2010), false).is_none());
    }

    #[test]
    fn empty_suggestion_payload_yields_no_candidates() {
        assert!(parse_suggestions("{}").expect("parse").is_empty());
    }

    #[test]
    fn movie_subtitles_path() {
        assert_eq!(
            subtitles_path("tt1375666", None),
            "/subtitles/movie/tt1375666.json"
        );
    }

    #[test]
    fn series_subtitles_path_embeds_season_and_episode() {
        assert_eq!(
            subtitles_path("tt0903747", Some((2, 7))),
            "/subtitles/series/tt0903747:2:7.json"
        );
    }

    #[test]
    fn subtitles_parsing_skips_entries_without_url() {
        let entries = parse_subtitles(SUBTITLES).expect("parse");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn subtitles_parsing_keeps_url_lang_and_label() {
        let entries = parse_subtitles(SUBTITLES).expect("parse");
        let entry = &entries[0];
        assert_eq!(
            entry.url,
            "https://subs5.strem.io/subtitles/1/file/1952382875"
        );
        assert_eq!(entry.lang, "eng");
        assert_eq!(entry.label, "Inception.2010.CAM.Xvid-LKRG");
        assert_eq!(entry.format.as_deref(), Some("Cam"));
    }

    #[test]
    fn subtitles_parsing_falls_back_to_file_name() {
        let payload = r#"{"subtitles":[{"url":"https://subs5.strem.io/f/1.srt","lang":"spa","subtitleFileName":"Inception.2010.SPA.srt"}]}"#;
        let entries = parse_subtitles(payload).expect("parse");
        assert_eq!(entries[0].label, "Inception.2010.SPA");
    }

    #[test]
    fn is_imdb_id_accepts_tt_ids_only() {
        assert!(is_imdb_id("tt1375666"));
        assert!(!is_imdb_id("nm1375666"));
        assert!(!is_imdb_id("tt"));
        assert!(!is_imdb_id("tt12ab"));
        assert!(!is_imdb_id(""));
    }

    #[test]
    fn language_rank_orders_spanish_then_english() {
        assert_eq!(language_rank("spa"), 0);
        assert_eq!(language_rank("es"), 0);
        assert_eq!(language_rank("SPA"), 0);
        assert_eq!(language_rank("eng"), 1);
        assert_eq!(language_rank("en"), 1);
        assert_eq!(language_rank("por"), 2);
        assert_eq!(language_rank("hin"), 2);
        assert_eq!(language_rank("fre"), 2);
    }

    #[test]
    fn language_label_maps_known_codes() {
        assert_eq!(language_label("spa"), "Español");
        assert_eq!(language_label("eng"), "English");
        assert_eq!(language_label("por"), "POR");
    }
}
