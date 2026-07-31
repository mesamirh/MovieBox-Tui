use super::client::FourKHdHubError;
use crate::providers::models::{
    CatalogItem, Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release, Season,
    SourceMirror,
};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use std::collections::{BTreeMap, HashMap};

pub fn parse_search(base: &Url, html: &str) -> Result<Vec<CatalogItem>, FourKHdHubError> {
    let document = Html::parse_document(html);
    let card = selector("a.movie-card")?;
    let title = selector(".movie-card-title")?;
    let meta = selector(".movie-card-meta")?;
    let image = selector("img")?;
    let mut items = Vec::new();

    for node in document.select(&card) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        let Ok(url) = base.join(href) else { continue };
        if url.host_str() != base.host_str() {
            continue;
        }
        let item_title = text_of(node.select(&title).next()).unwrap_or_default();
        if item_title.is_empty() {
            continue;
        }
        let meta_text = text_of(node.select(&meta).next()).unwrap_or_default();
        let year = first_four_digit_year(&meta_text);
        let media_type = if href.contains("-series-") {
            MediaType::Series
        } else {
            MediaType::Movie
        };
        let poster_url = node
            .select(&image)
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(str::to_string);
        items.push(CatalogItem {
            id: ProviderMediaId {
                provider: ProviderKind::FourKHdHub,
                value: url.path().to_string(),
            },
            title: item_title,
            media_type,
            year,
            poster_url,
            season_count: parse_season_count(&meta_text),
        });
    }
    Ok(items)
}

pub fn parse_details(id: &str, html: &str) -> Result<MediaDetails, FourKHdHubError> {
    let document = Html::parse_document(html);
    let h1 = selector("h1")?;
    let raw_title = document
        .select(&h1)
        .find_map(|node| text_of(Some(node)))
        .filter(|text| !text.is_empty())
        .or_else(|| meta_content(&document, "meta[property=\"og:title\"]"))
        .ok_or_else(|| FourKHdHubError::Parse("title missing".into()))?;
    let title = strip_trailing_year(&raw_title);
    let media_type = if id.contains("-series-") {
        MediaType::Series
    } else {
        MediaType::Movie
    };
    let description = document
        .select(&selector(".content-section p.mt-4")?)
        .find_map(|node| text_of(Some(node)))
        .or_else(|| meta_content(&document, "meta[name=\"description\"]"));
    let tagline = document
        .select(&selector(".movie-tagline")?)
        .find_map(|node| text_of(Some(node)));
    let imdb_rating = document
        .select(&selector(".imdb-score")?)
        .find_map(|node| text_of(Some(node)));
    let poster_url = meta_content(&document, "meta[property=\"og:image\"]");
    let year = find_metadata(&document, "Release:")
        .and_then(|value| first_four_digit_year(&value))
        .or_else(|| {
            find_metadata(&document, "Last Air:").and_then(|value| first_four_digit_year(&value))
        })
        .or_else(|| first_four_digit_year(&raw_title));
    let genres = document
        .select(&selector(".badge-outline a")?)
        .filter_map(|node| text_of(Some(node)))
        .filter(|value| is_genre(value))
        .collect();
    let seasons = parse_seasons(&document)?;

    Ok(MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::FourKHdHub,
            value: id.to_string(),
        },
        title,
        media_type,
        year,
        description,
        tagline,
        imdb_rating,
        director: find_metadata(&document, "Director:"),
        stars: find_metadata(&document, "Stars:"),
        prints: find_metadata(&document, "Prints:").or_else(|| find_metadata(&document, "Print:")),
        audios: find_metadata(&document, "Audios:"),
        poster_url,
        genres,
        seasons,
    })
}

pub fn parse_releases(
    html: &str,
    season: usize,
    episode: usize,
) -> Result<Vec<Release>, FourKHdHubError> {
    let document = Html::parse_document(html);
    let item_selector = if season > 0 {
        selector("#episodes .episode-download-item")?
    } else {
        selector(".download-item")?
    };
    let filename_selector = if season > 0 {
        selector(".episode-file-title")?
    } else {
        selector(".file-title")?
    };
    let link_selector = selector("a[href]")?;
    let size_selector = selector(".badge-size, .badge")?;
    let page_language =
        find_metadata(&document, "Audios:").and_then(|value| normalize_language_label(&value));
    let mut grouped: HashMap<String, Release> = HashMap::new();

    for item in document.select(&item_selector) {
        let filename = text_of(item.select(&filename_selector).next()).unwrap_or_default();
        if filename.is_empty() || is_archive(&filename) {
            continue;
        }
        let parsed_episode = parse_season_episode(&filename);
        if season > 0 && parsed_episode != Some((season, episode)) {
            continue;
        }
        let mirrors = item
            .select(&link_selector)
            .filter_map(|link| {
                let href = link.value().attr("href")?;
                if !href.starts_with("https://") || href.contains("logout") {
                    return None;
                }
                let label = text_of(Some(link)).unwrap_or_else(|| "Source".into());
                Some(SourceMirror {
                    label,
                    resolver_url: href.to_string(),
                    headers: Vec::new(),
                    direct_file: !href.contains("hubcloud.") && !href.contains("hubdrive."),
                })
            })
            .collect::<Vec<_>>();
        if mirrors.is_empty() {
            continue;
        }
        let size_text = item
            .select(&size_selector)
            .filter_map(|node| text_of(Some(node)))
            .find(|text| parse_size_bytes(text).is_some());
        let key = normalize_filename(&filename);
        let release = grouped.entry(key).or_insert_with(|| Release {
            provider: ProviderKind::FourKHdHub,
            quality: detect_quality(&filename),
            codec: detect_codec(&filename),
            language: detect_language(&filename).or_else(|| page_language.clone()),
            size_bytes: size_text.as_deref().and_then(parse_size_bytes),
            season: parsed_episode.map(|value| value.0),
            episode: parsed_episode.map(|value| value.1),
            filename: filename.clone(),
            mirrors: Vec::new(),
        });
        for mirror in mirrors {
            if !release
                .mirrors
                .iter()
                .any(|existing| existing.resolver_url == mirror.resolver_url)
            {
                release.mirrors.push(mirror);
            }
        }
    }
    let mut releases = grouped.into_values().collect::<Vec<_>>();
    releases.sort_by(|left, right| right.quality.cmp(&left.quality));
    Ok(releases)
}

fn parse_seasons(document: &Html) -> Result<Vec<Season>, FourKHdHubError> {
    let item = selector("#episodes .episode-download-item")?;
    let title = selector(".episode-file-title")?;
    let mut seasons: BTreeMap<usize, BTreeMap<usize, Episode>> = BTreeMap::new();
    for node in document.select(&item) {
        let filename = text_of(node.select(&title).next()).unwrap_or_default();
        let Some((season, episode)) = parse_season_episode(&filename) else {
            continue;
        };
        seasons
            .entry(season)
            .or_default()
            .entry(episode)
            .or_insert(Episode {
                season,
                number: episode,
                title: None,
            });
    }
    Ok(seasons
        .into_iter()
        .map(|(number, episodes)| Season {
            number,
            episodes: episodes.into_values().collect(),
        })
        .collect())
}

pub fn search_to_moviebox_json(items: &[CatalogItem]) -> serde_json::Value {
    let subjects = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "subjectId": item.id.value,
                "title": item.title,
                "subjectType": if item.media_type == MediaType::Series { 2 } else { 1 },
                "releaseDate": item.year,
                "cover": { "url": item.poster_url },
                "season": item.season_count.unwrap_or_default(),
                "_provider": ProviderKind::FourKHdHub.cache_key(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({ "results": [{ "subjects": subjects }] })
}

pub fn details_to_moviebox_json(details: &MediaDetails) -> serde_json::Value {
    let seasons = details
        .seasons
        .iter()
        .map(|season| {
            serde_json::json!({
                "se": season.number,
                "maxEp": season.episodes.len(),
                "episodeNumbers": season.episodes.iter().map(|episode| episode.number).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "id": details.id.value,
        "subjectId": details.id.value,
        "title": details.title,
        "subjectType": if details.media_type == MediaType::Series { 2 } else { 1 },
        "releaseDate": details.year,
        "description": details.description,
        "tagline": details.tagline,
        "imdbRatingValue": details.imdb_rating,
        "director": details.director,
        "stars": details.stars,
        "prints": details.prints,
        "audios": details.audios,
        "cover": { "url": details.poster_url },
        "genre": details.genres,
        "seasons": { "seasons": seasons }
    })
}

pub fn releases_to_moviebox_json(releases: &[Release]) -> serde_json::Value {
    let list = releases
        .iter()
        .enumerate()
        .map(|(index, release)| {
            let resolution = release
                .quality
                .as_deref()
                .and_then(|quality| quality.trim_end_matches('p').parse::<u64>().ok())
                .unwrap_or_default();
            serde_json::json!({
                "resourceId": format!("fourk-{}", index),
                "resourceLink": release.mirrors.first().map(|mirror| mirror.resolver_url.clone()),
                "title": release.filename,
                "fileName": release.filename,
                "size": release.size_bytes.map(|size| size.to_string()),
                "resolution": resolution,
                "codecName": release.codec,
                "language": release.language,
                "sourceCount": release.mirrors.len(),
                "uploadBy": "4KHDHub",
                "se": release.season.unwrap_or_default(),
                "ep": release.episode.unwrap_or_default(),
                "_fourk_release": release
            })
        })
        .collect::<Vec<_>>();
    serde_json::Value::Array(list)
}

fn selector(value: &str) -> Result<Selector, FourKHdHubError> {
    Selector::parse(value).map_err(|_| FourKHdHubError::Parse(format!("invalid selector: {value}")))
}

fn text_of(node: Option<ElementRef<'_>>) -> Option<String> {
    node.map(|node| node.text().collect::<Vec<_>>().join(" "))
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|text| !text.is_empty())
}

fn meta_content(document: &Html, query: &str) -> Option<String> {
    let selector = Selector::parse(query).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(str::to_string)
}

fn find_metadata(document: &Html, label: &str) -> Option<String> {
    let item = Selector::parse(".metadata-item").ok()?;
    let label_selector = Selector::parse(".metadata-label").ok()?;
    let value_selector = Selector::parse(".metadata-value").ok()?;
    document.select(&item).find_map(|node| {
        let current = text_of(node.select(&label_selector).next())?;
        (current == label).then(|| text_of(node.select(&value_selector).next()))?
    })
}

fn first_four_digit_year(value: &str) -> Option<String> {
    value
        .as_bytes()
        .windows(4)
        .find(|window| window.iter().all(u8::is_ascii_digit) && matches!(window[0], b'1' | b'2'))
        .and_then(|window| std::str::from_utf8(window).ok())
        .map(str::to_string)
}

fn is_genre(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "action"
            | "adventure"
            | "animation"
            | "comedy"
            | "crime"
            | "documentary"
            | "drama"
            | "family"
            | "fantasy"
            | "history"
            | "horror"
            | "music"
            | "mystery"
            | "romance"
            | "science fiction"
            | "sci-fi"
            | "thriller"
            | "war"
            | "western"
    )
}

fn strip_trailing_year(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 6 {
        let suffix = &trimmed[trimmed.len() - 6..];
        if suffix.starts_with('(')
            && suffix.ends_with(')')
            && suffix[1..5].bytes().all(|byte| byte.is_ascii_digit())
        {
            return trimmed[..trimmed.len() - 6].trim_end().to_string();
        }
    }
    trimmed.to_string()
}

fn parse_season_count(value: &str) -> Option<usize> {
    let marker = value.find('S')?;
    let suffix = &value[marker..];
    suffix
        .split(['-', ' ', '•'])
        .filter_map(|part| part.trim_start_matches('S').parse::<usize>().ok())
        .max()
}

fn parse_season_episode(value: &str) -> Option<(usize, usize)> {
    let upper = value.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    for index in 0..bytes.len().saturating_sub(4) {
        if bytes[index] != b'S' {
            continue;
        }
        let Some(season_end) = bytes[index + 1..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| index + 1 + offset)
        else {
            continue;
        };
        if season_end == index + 1 || bytes.get(season_end) != Some(&b'E') {
            continue;
        }
        let episode_end = bytes[season_end + 1..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| season_end + 1 + offset)
            .unwrap_or(bytes.len());
        if episode_end == season_end + 1 {
            continue;
        }
        if let (Ok(season), Ok(episode)) = (
            upper[index + 1..season_end].parse(),
            upper[season_end + 1..episode_end].parse(),
        ) {
            return Some((season, episode));
        }
    }
    None
}

fn parse_size_bytes(value: &str) -> Option<u64> {
    let normalized = value.replace(' ', "").to_ascii_uppercase();
    for (suffix, multiplier) in [
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024_u64),
    ] {
        if let Some(number) = normalized.strip_suffix(suffix)
            && let Ok(number) = number.parse::<f64>()
        {
            return Some((number * multiplier as f64) as u64);
        }
    }
    None
}

fn detect_quality(value: &str) -> Option<String> {
    ["2160p", "1080p", "720p", "480p"]
        .into_iter()
        .find(|quality| {
            value
                .to_ascii_lowercase()
                .contains(&quality.to_ascii_lowercase())
        })
        .map(str::to_string)
}

fn detect_codec(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("av1") {
        Some("AV1".into())
    } else if lower.contains("h.265") || lower.contains("h265") || lower.contains("x265") {
        Some("H.265".into())
    } else if lower.contains("hevc") {
        Some("HEVC".into())
    } else if lower.contains("h.264") || lower.contains("h264") || lower.contains("x264") {
        Some("H.264".into())
    } else if lower.contains("remux") {
        Some("REMUX".into())
    } else {
        None
    }
}

fn detect_language(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    match (lower.contains("hindi"), lower.contains("english")) {
        (true, true) => Some("Hindi, English".into()),
        (true, false) => Some("Hindi".into()),
        (false, true) => Some("English".into()),
        _ if lower.contains("dual audio") => Some("Dual Audio".into()),
        _ if lower.contains("multi audio") || lower.contains("multi-audio") => {
            Some("Multi Audio".into())
        }
        _ => None,
    }
}

fn normalize_language_label(value: &str) -> Option<String> {
    let value = value
        .split(['|', '/', '+'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if value.is_empty()
        || value.len() > 80
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "n/a" | "na" | "unknown"
        )
    {
        None
    } else {
        Some(value)
    }
}

fn is_archive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".zip") || lower.contains("complete season") || lower.contains("season pack")
}

fn normalize_filename(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}
