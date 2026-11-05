use super::models::{MetaDetail, MetaItem, StreamItem};
use crate::providers::models::{
    CatalogItem, Episode, MediaDetails, MediaType, PlaybackSource, ProviderKind, ProviderMediaId,
    Release, Season, SourceMirror,
};
use std::collections::BTreeMap;

type SeasonEpisodeMap = BTreeMap<usize, BTreeMap<usize, (Option<String>, Option<String>)>>;

pub fn meta_to_catalog_item(item: &MetaItem) -> CatalogItem {
    let is_series = item.r#type.eq_ignore_ascii_case("series")
        || item.r#type.eq_ignore_ascii_case("tv")
        || item.r#type.eq_ignore_ascii_case("anime");
    let year: Option<String> = item
        .release_info
        .as_deref()
        .or(item.year.as_deref())
        .or(item.released.as_deref())
        .map(crate::providers::models::extract_4digit_year)
        .filter(|y| !y.is_empty());

    let title = if !item.name.trim().is_empty() {
        item.name.clone()
    } else {
        item.title.clone().unwrap_or_else(|| "Unknown".to_string())
    };

    let poster_url = item.poster.clone().or_else(|| item.cover.clone());

    let type_prefix = if is_series { "series" } else { "movie" };
    let composite_id = format!("{type_prefix}:{}", item.id);

    CatalogItem {
        id: ProviderMediaId {
            provider: ProviderKind::Addons,
            value: composite_id,
        },
        title,
        media_type: if is_series {
            MediaType::Series
        } else {
            MediaType::Movie
        },
        year,
        poster_url,
        season_count: None,
    }
}

pub fn meta_detail_to_media_details(detail: &MetaDetail) -> MediaDetails {
    let is_series = detail.r#type.eq_ignore_ascii_case("series")
        || detail.r#type.eq_ignore_ascii_case("tv")
        || detail.r#type.eq_ignore_ascii_case("anime")
        || !detail.videos.is_empty();

    let mut season_map: SeasonEpisodeMap = BTreeMap::new();
    for video in &detail.videos {
        let s = video.season.unwrap_or(1);
        let e = video.episode.or(video.number).unwrap_or(1);
        let ep_title = video.title.clone().or_else(|| video.name.clone());
        let ep_overview = video.overview.clone().or_else(|| video.description.clone());
        season_map
            .entry(s)
            .or_default()
            .entry(e)
            .or_insert((ep_title, ep_overview));
    }

    let seasons = season_map
        .into_iter()
        .map(|(season_num, eps)| Season {
            number: season_num,
            episodes: eps
                .into_iter()
                .map(|(ep_num, (ep_title, ep_overview))| Episode {
                    season: season_num,
                    number: ep_num,
                    title: ep_title,
                    overview: ep_overview,
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    let year_raw = detail
        .release_info
        .as_deref()
        .or(detail.year.as_deref())
        .or(detail.released.as_deref())
        .unwrap_or_default();
    let year = crate::providers::models::extract_4digit_year(year_raw);
    let year = if !year.is_empty() {
        Some(year)
    } else if !year_raw.is_empty() {
        Some(year_raw.to_string())
    } else {
        None
    };

    let title = if !detail.name.trim().is_empty() {
        detail.name.clone()
    } else {
        detail
            .title
            .clone()
            .unwrap_or_else(|| "Unknown".to_string())
    };

    let poster_url = detail
        .poster
        .clone()
        .or_else(|| detail.cover.clone())
        .or_else(|| detail.background.clone());

    let description = detail
        .description
        .clone()
        .or_else(|| detail.overview.clone())
        .or_else(|| detail.synopsis.clone());

    let type_prefix = if is_series { "series" } else { "movie" };
    let composite_id = format!("{type_prefix}:{}", detail.id);

    MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::Addons,
            value: composite_id,
        },
        title,
        media_type: if is_series {
            MediaType::Series
        } else {
            MediaType::Movie
        },
        year,
        description,
        tagline: None,
        imdb_rating: detail.imdb_rating.clone().or_else(|| detail.rating.clone()),
        director: if !detail.director.is_empty() {
            Some(detail.director.join(", "))
        } else if !detail.directors.is_empty() {
            Some(detail.directors.join(", "))
        } else if !detail.writers.is_empty() {
            Some(detail.writers.join(", "))
        } else if !detail.writer.is_empty() {
            Some(detail.writer.join(", "))
        } else {
            None
        },
        stars: if !detail.cast.is_empty() {
            Some(detail.cast.join(", "))
        } else if !detail.stars.is_empty() {
            Some(detail.stars.join(", "))
        } else {
            None
        },
        prints: None,
        audios: None,
        poster_url,
        duration: detail.runtime.clone(),
        genres: if !detail.genres.is_empty() {
            detail.genres.clone()
        } else {
            detail.genre.clone()
        },
        seasons,
        dubs: vec![],
    }
}

pub fn parse_quality(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    if upper.contains("2160P") || upper.contains("4K") || upper.contains("UHD") {
        Some("2160p".to_string())
    } else if upper.contains("1080P")
        || upper.contains("FHD")
        || upper.contains("FULL HD")
        || upper.contains("FULLHD")
    {
        Some("1080p".to_string())
    } else if upper.contains("720P")
        || upper
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "HD")
    {
        Some("720p".to_string())
    } else if upper.contains("480P")
        || upper
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "SD")
    {
        Some("480p".to_string())
    } else {
        None
    }
}

pub fn parse_codec(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    if upper.contains("HEVC")
        || upper.contains("X265")
        || upper.contains("H.265")
        || upper.contains("H265")
    {
        Some("HEVC/x265".to_string())
    } else if upper.contains("X264")
        || upper.contains("H.264")
        || upper.contains("H264")
        || upper.contains("AVC")
    {
        Some("AVC/x264".to_string())
    } else if upper.contains("AV1") {
        Some("AV1".to_string())
    } else {
        None
    }
}

pub fn parse_audio_tracks(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    let mut langs = Vec::new();
    let candidates = [
        ("HINDI", "Hindi"),
        ("ENGLISH", "English"),
        ("ENG", "English"),
        ("HIN", "Hindi"),
        ("TAMIL", "Tamil"),
        ("TELUGU", "Telugu"),
        ("BENGALI", "Bengali"),
        ("BEN", "Bengali"),
        ("MALAYALAM", "Malayalam"),
        ("KANNADA", "Kannada"),
        ("MARATHI", "Marathi"),
        ("PUNJABI", "Punjabi"),
        ("GUJARATI", "Gujarati"),
        ("URDU", "Urdu"),
        ("SPANISH", "Spanish"),
        ("FRENCH", "French"),
        ("GERMAN", "German"),
        ("ITALIAN", "Italian"),
        ("JAPANESE", "Japanese"),
        ("JAP", "Japanese"),
        ("KOREAN", "Korean"),
        ("KOR", "Korean"),
        ("RUSSIAN", "Russian"),
        ("CHINESE", "Chinese"),
        ("DUAL", "Dual Audio"),
        ("MULTI", "Multi Audio"),
    ];

    for (needle, label) in candidates {
        if upper.contains(needle) && !langs.contains(&label) {
            langs.push(label);
        }
    }

    if langs.is_empty() {
        None
    } else {
        Some(langs.join(" + "))
    }
}

fn extract_domain_label(url: &str) -> Option<String> {
    let url_clean = url.trim();
    let without_proto = url_clean
        .strip_prefix("https://")
        .or_else(|| url_clean.strip_prefix("http://"))
        .unwrap_or(url_clean);

    let host = without_proto.split(['/', ':', '?', '#']).next()?.trim();
    if host.is_empty() || host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }

    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let main_part = if parts.len() >= 2 {
        if (parts[parts.len() - 2] == "co" || parts[parts.len() - 2] == "com") && parts.len() >= 3 {
            parts[parts.len() - 3]
        } else {
            parts[parts.len() - 2]
        }
    } else {
        parts[0]
    };

    if main_part.is_empty() || main_part == "www" || main_part == "api" || main_part == "cdn" {
        return None;
    }

    let capitalized = main_part
        .split(['-', '_'])
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Some(capitalized)
}

pub fn detect_stream_host(
    addon_name: &str,
    stream_name: &str,
    _title: &str,
    _description: &str,
    url: &str,
) -> String {
    let mut detected_labels = Vec::new();

    for line in stream_name.lines() {
        let trimmed = line.trim().trim_matches(['[', ']', '(', ')', ' ']);
        if !trimmed.is_empty()
            && !trimmed.eq_ignore_ascii_case(addon_name)
            && !detected_labels.contains(&trimmed.to_string())
        {
            detected_labels.push(trimmed.to_string());
        }
    }

    if let Some(domain_label) = extract_domain_label(url) {
        if !domain_label.eq_ignore_ascii_case(addon_name)
            && !detected_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case(&domain_label))
        {
            detected_labels.push(domain_label);
        }
    }

    if let Some(first_tag) = detected_labels.first() {
        format!("{addon_name} · {first_tag}")
    } else {
        addon_name.to_string()
    }
}

pub fn parse_season_episode(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        if (bytes[i] == b'S' || bytes[i] == b's') && i + 1 < len && bytes[i + 1].is_ascii_digit() {
            if i == 0 || !bytes[i - 1].is_ascii_alphanumeric() {
                let s_start = i + 1;
                let mut s_end = s_start;
                while s_end < len && bytes[s_end].is_ascii_digit() {
                    s_end += 1;
                }
                if s_end < len && (s_end - s_start) <= 3 {
                    if let Ok(s_num) = text[s_start..s_end].parse::<usize>() {
                        let mut e_idx = s_end;
                        while e_idx < len
                            && (bytes[e_idx] == b'.'
                                || bytes[e_idx] == b' '
                                || bytes[e_idx] == b'_'
                                || bytes[e_idx] == b'-')
                        {
                            e_idx += 1;
                        }
                        if e_idx < len
                            && (bytes[e_idx] == b'E' || bytes[e_idx] == b'e')
                            && e_idx + 1 < len
                            && bytes[e_idx + 1].is_ascii_digit()
                        {
                            let e_start = e_idx + 1;
                            let mut e_end = e_start;
                            while e_end < len && bytes[e_end].is_ascii_digit() {
                                e_end += 1;
                            }
                            if (e_end - e_start) <= 4 {
                                if let Ok(e_num) = text[e_start..e_end].parse::<usize>() {
                                    return Some((s_num, e_num));
                                }
                            }
                        }
                    }
                }
            }
        }

        if (bytes[i] == b'x' || bytes[i] == b'X')
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && i + 1 < len
            && bytes[i + 1].is_ascii_digit()
        {
            let mut s_start = i - 1;
            while s_start > 0 && bytes[s_start - 1].is_ascii_digit() {
                s_start -= 1;
            }
            if s_start == 0 || !bytes[s_start - 1].is_ascii_alphanumeric() {
                let s_str = &text[s_start..i];
                let mut e_end = i + 1;
                while e_end < len && bytes[e_end].is_ascii_digit() {
                    e_end += 1;
                }
                let e_str = &text[i + 1..e_end];
                if s_str.len() <= 3 && e_str.len() <= 4 {
                    if let (Ok(s_num), Ok(e_num)) = (s_str.parse::<usize>(), e_str.parse::<usize>())
                    {
                        if s_num > 0 && s_num < 100 && e_num > 0 && e_num < 10000 {
                            return Some((s_num, e_num));
                        }
                    }
                }
            }
        }

        i += 1;
    }

    let upper = text.to_ascii_uppercase();
    if let Some(season_pos) = upper.find("SEASON ") {
        let after_season = &upper[season_pos + 7..];
        let s_digits: String = after_season
            .chars()
            .skip_while(|c| *c == ' ')
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(s_num) = s_digits.parse::<usize>() {
            if let Some(ep_pos) = upper.find("EPISODE ") {
                let after_ep = &upper[ep_pos + 8..];
                let e_digits: String = after_ep
                    .chars()
                    .skip_while(|c| *c == ' ')
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(e_num) = e_digits.parse::<usize>() {
                    return Some((s_num, e_num));
                }
            }
        }
    }

    if let Some(ep_pos) = upper.find("EPISODE ") {
        let after = &upper[ep_pos + 8..];
        let digits: String = after
            .chars()
            .skip_while(|c| *c == ' ')
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(e_num) = digits.parse::<usize>() {
            return Some((1, e_num));
        }
    }

    None
}

pub fn stream_item_to_release(
    addon_name: &str,
    stream: &StreamItem,
    season: usize,
    episode: usize,
) -> Option<Release> {
    let url = stream.url.as_ref()?.trim();
    if !crate::net::is_http_url(url) {
        return None;
    }

    let stream_name_str = stream.name.as_deref().unwrap_or_default();
    let title_str = stream.title.as_deref().unwrap_or_default();
    let desc_str = stream.description.as_deref().unwrap_or_default();

    let combined_text = format!("{stream_name_str} {title_str} {desc_str}");

    if season > 0 && episode > 0 {
        if let Some((stream_s, stream_e)) = parse_season_episode(&combined_text) {
            if stream_s != season || stream_e != episode {
                return None;
            }
        }
    }

    let quality = parse_quality(&combined_text);
    let codec = parse_codec(&combined_text);
    let language = parse_audio_tracks(&combined_text);

    let size_bytes = stream
        .behavior_hints
        .as_ref()
        .and_then(|h| h.video_size)
        .or_else(|| crate::providers::models::parse_size_bytes(&combined_text));

    let raw_filename = stream
        .title
        .as_ref()
        .or(stream.description.as_ref())
        .or(stream.name.as_ref())
        .map(|s| s.lines().next().unwrap_or(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{addon_name} Stream"));

    let filename = crate::providers::models::clean_stream_text(&raw_filename);
    let raw_source_label =
        detect_stream_host(addon_name, stream_name_str, title_str, desc_str, url);
    let source_label = crate::providers::models::clean_stream_text(&raw_source_label);
    let language = language.map(|l| crate::providers::models::clean_stream_text(&l));

    let mut headers = Vec::new();
    if let Some(hints) = &stream.behavior_hints
        && let Some(hdr_map) = &hints.headers
    {
        const ALLOWED_HEADERS: &[&str] = &[
            "user-agent",
            "referer",
            "origin",
            "range",
            "x-forwarded-for",
            "accept",
            "accept-language",
        ];
        for (k, v) in hdr_map {
            if ALLOWED_HEADERS.contains(&k.to_ascii_lowercase().as_str()) {
                headers.push((k.clone(), v.clone()));
            }
        }
    }

    Some(Release {
        provider: ProviderKind::Addons,
        filename,
        quality,
        codec,
        language,
        size_bytes,
        season: if season > 0 { Some(season) } else { None },
        episode: if episode > 0 { Some(episode) } else { None },
        mirrors: vec![SourceMirror {
            label: source_label,
            resolver_url: url.to_string(),
            headers,
            direct_file: true,
        }],
        resource_id: None,
    })
}

pub fn release_to_playback_source(release: &Release) -> Option<PlaybackSource> {
    let mirror = release.mirrors.first()?;
    Some(PlaybackSource {
        provider: ProviderKind::Addons,
        url: mirror.resolver_url.clone(),
        headers: mirror.headers.clone(),
        subtitle: None,
        source_label: mirror.label.clone(),
        max_height: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_to_catalog_item() {
        let item = MetaItem {
            id: "tt1234".to_string(),
            r#type: "movie".to_string(),
            name: "Test Movie".to_string(),
            title: None,
            poster: Some("https://example.com/poster.jpg".to_string()),
            cover: None,
            description: Some("Description".to_string()),
            overview: None,
            synopsis: None,
            release_info: Some("2022".to_string()),
            year: None,
            released: None,
            imdb_rating: Some("7.5".to_string()),
            rating: None,
            genres: vec!["Action".to_string()],
            genre: Vec::new(),
        };

        let catalog = meta_to_catalog_item(&item);
        assert_eq!(catalog.id.value, "movie:tt1234");
        assert_eq!(catalog.title, "Test Movie");
        assert_eq!(catalog.media_type, MediaType::Movie);
        assert_eq!(catalog.year.as_deref(), Some("2022"));
        assert_eq!(
            catalog.poster_url.as_deref(),
            Some("https://example.com/poster.jpg")
        );
    }

    #[test]
    fn test_meta_detail_to_media_details() {
        let detail = MetaDetail {
            id: "tt5678".to_string(),
            r#type: "series".to_string(),
            name: "Test Series".to_string(),
            title: None,
            poster: Some("https://example.com/series.jpg".to_string()),
            cover: None,
            background: None,
            logo: None,
            description: Some("Series description".to_string()),
            overview: None,
            synopsis: None,
            release_info: Some("2021".to_string()),
            year: None,
            released: None,
            imdb_rating: Some("8.2".to_string()),
            rating: None,
            genres: vec!["Drama".to_string()],
            genre: Vec::new(),
            runtime: Some("45m".to_string()),
            cast: vec!["Actor One".to_string(), "Actor Two".to_string()],
            stars: Vec::new(),
            director: vec!["Director Name".to_string()],
            directors: Vec::new(),
            writer: Vec::new(),
            writers: Vec::new(),
            videos: vec![
                super::super::models::MetaVideo {
                    id: Some("ep1".to_string()),
                    title: Some("Pilot".to_string()),
                    name: None,
                    season: Some(1),
                    episode: Some(1),
                    number: None,
                    released: None,
                    thumbnail: None,
                    overview: Some("Pilot episode overview".to_string()),
                    description: None,
                },
                super::super::models::MetaVideo {
                    id: Some("ep2".to_string()),
                    title: Some("Episode 2".to_string()),
                    name: None,
                    season: Some(1),
                    episode: Some(2),
                    number: None,
                    released: None,
                    thumbnail: None,
                    overview: None,
                    description: None,
                },
            ],
        };

        let media = meta_detail_to_media_details(&detail);
        assert_eq!(media.id.value, "series:tt5678");
        assert_eq!(media.title, "Test Series");
        assert_eq!(media.media_type, MediaType::Series);
        assert_eq!(media.year.as_deref(), Some("2021"));
        assert_eq!(media.director.as_deref(), Some("Director Name"));
        assert_eq!(media.stars.as_deref(), Some("Actor One, Actor Two"));
        assert_eq!(media.seasons.len(), 1);
        assert_eq!(media.seasons[0].number, 1);
        assert_eq!(media.seasons[0].episodes.len(), 2);
        assert_eq!(
            media.seasons[0].episodes[0].overview.as_deref(),
            Some("Pilot episode overview")
        );
        assert_eq!(media.seasons[0].episodes[0].title.as_deref(), Some("Pilot"));
    }

    #[test]
    fn test_meta_detail_movie_composite_id() {
        let detail = MetaDetail {
            id: "tt9999".to_string(),
            r#type: "movie".to_string(),
            name: "Test Movie".to_string(),
            title: None,
            poster: Some("https://example.com/movie.jpg".to_string()),
            cover: None,
            background: None,
            logo: None,
            description: Some("Movie description".to_string()),
            overview: None,
            synopsis: None,
            release_info: Some("2020".to_string()),
            year: None,
            released: None,
            imdb_rating: Some("7.9".to_string()),
            rating: None,
            genres: vec!["Sci-Fi".to_string()],
            genre: Vec::new(),
            runtime: Some("120m".to_string()),
            cast: Vec::new(),
            stars: Vec::new(),
            director: Vec::new(),
            directors: Vec::new(),
            writer: Vec::new(),
            writers: Vec::new(),
            videos: Vec::new(),
        };

        let media = meta_detail_to_media_details(&detail);
        assert_eq!(media.id.value, "movie:tt9999");
        assert_eq!(media.title, "Test Movie");
        assert_eq!(media.media_type, MediaType::Movie);
        assert!(media.seasons.is_empty());
    }
}
