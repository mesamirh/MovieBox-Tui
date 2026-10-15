use crate::providers::models::{
    AudioTrackOption, CatalogItem, Episode, MediaDetails, MediaType, ProviderError, ProviderKind,
    ProviderMediaId, Release, Season, SourceMirror, SubtitleOption,
};
use base64::Engine;
pub fn captions_json_to_options(payload: &serde_json::Value) -> Vec<SubtitleOption> {
    let Some(captions) = payload
        .get("extCaptions")
        .or_else(|| payload.get("data").and_then(|d| d.get("extCaptions")))
        .and_then(|c| c.as_array())
    else {
        return Vec::new();
    };
    let mut seen_urls = std::collections::HashSet::new();
    captions
        .iter()
        .filter_map(|cap| {
            let url = cap.get("url").and_then(|u| u.as_str())?;
            if url.is_empty() || url.contains("aa348f2541d13ffe") {
                return None;
            }
            let size = cap
                .get("size")
                .and_then(|s| {
                    if let Some(n) = s.as_u64() {
                        Some(n)
                    } else if let Some(n) = s.as_i64() {
                        Some(n as u64)
                    } else {
                        s.as_str().and_then(|str_val| str_val.parse::<u64>().ok())
                    }
                })
                .unwrap_or(0);
            if size > 0 && size <= 50 {
                return None;
            }
            let raw_name = cap
                .get("lanName")
                .and_then(|n| n.as_str())
                .filter(|s| !s.trim().is_empty())
                .or_else(|| cap.get("lan").and_then(|l| l.as_str()))
                .unwrap_or("Unknown");
            if raw_name.eq_ignore_ascii_case("in") && (size == 0 || size <= 100) {
                return None;
            }
            if !seen_urls.insert(url.to_string()) {
                return None;
            }
            Some(SubtitleOption {
                name: raw_name.to_string(),
                url: url.to_string(),
            })
        })
        .collect()
}

pub fn moviebox_subject_json_to_catalog_item(s: &serde_json::Value) -> Option<CatalogItem> {
    let id_str = s.get("subjectId").or_else(|| s.get("id")).and_then(|v| {
        if let Some(num) = v.as_i64() {
            Some(num.to_string())
        } else {
            v.as_str().map(|str_val| str_val.to_string())
        }
    })?;

    if id_str.is_empty() {
        return None;
    }

    let title = s
        .get("title")
        .or_else(|| s.get("name"))
        .and_then(|t| t.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let stype = s
        .get("subjectType")
        .or_else(|| s.get("stype"))
        .and_then(|st| st.as_i64())
        .unwrap_or(1);

    let media_type = if stype == 2 {
        MediaType::Series
    } else {
        MediaType::Movie
    };

    let year = s
        .get("releaseDate")
        .or_else(|| s.get("year"))
        .or_else(|| s.get("releaseInfo"))
        .and_then(|y| y.as_str())
        .map(crate::providers::models::extract_4digit_year)
        .filter(|y| !y.is_empty());

    let poster_url = s
        .get("cover")
        .and_then(|c| c.get("url"))
        .or_else(|| s.get("coverUrl"))
        .or_else(|| s.get("poster"))
        .or_else(|| s.get("pic"))
        .and_then(|u| u.as_str())
        .map(|u| u.to_string());

    let season_count = s
        .get("season")
        .and_then(|sc| sc.as_u64())
        .map(|sc| sc as usize);

    Some(CatalogItem {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: id_str,
        },
        title,
        media_type,
        year,
        poster_url,
        season_count,
    })
}

pub fn moviebox_search_json_to_catalog(payload: &serde_json::Value) -> Vec<CatalogItem> {
    let mut items = Vec::new();
    let subjects = payload
        .get("data")
        .and_then(|d| d.get("results"))
        .or_else(|| payload.get("results"))
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("subjects"))
        .and_then(|s| s.as_array());

    let subjects_slice = match subjects {
        Some(s) => s.as_slice(),
        None => {
            if let Some(list) = payload
                .get("data")
                .and_then(|d| d.get("list"))
                .or_else(|| payload.get("list"))
                .and_then(|l| l.as_array())
            {
                list.as_slice()
            } else {
                &[]
            }
        }
    };

    for s in subjects_slice {
        if let Some(item) = moviebox_subject_json_to_catalog_item(s) {
            items.push(item);
        }
    }

    items
}

pub fn moviebox_homepage_json_to_catalog(
    payload: &serde_json::Value,
) -> (
    Vec<CatalogItem>,
    std::collections::HashMap<String, crate::models::BrowseMetrics>,
) {
    let mut items = Vec::new();
    let mut metrics_map = std::collections::HashMap::new();
    let mut seen_ids = std::collections::HashSet::new();

    let groups = payload
        .get("items")
        .and_then(|i| i.as_array())
        .or_else(|| payload.as_array());

    if let Some(groups_arr) = groups {
        for group in groups_arr {
            let mut group_subjects = Vec::new();
            if let Some(banner) = group
                .get("banner")
                .and_then(|b| b.get("banners"))
                .and_then(|b| b.as_array())
            {
                for b_item in banner {
                    if let Some(subject) = b_item.get("subject") {
                        group_subjects.push(subject);
                    }
                }
            }
            if let Some(custom_data) = group
                .get("customData")
                .and_then(|c| c.get("items"))
                .and_then(|i| i.as_array())
            {
                for c_item in custom_data {
                    if let Some(subject) = c_item.get("subject") {
                        group_subjects.push(subject);
                    }
                }
            }
            if let Some(subjects) = group.get("subjects").and_then(|s| s.as_array()) {
                for s in subjects {
                    group_subjects.push(s);
                }
            }

            for (index, subject_val) in group_subjects.into_iter().enumerate() {
                if let Some(catalog_item) = moviebox_subject_json_to_catalog_item(subject_val) {
                    let id = catalog_item.id.value.clone();
                    if seen_ids.insert(id.clone()) {
                        let mut metric = crate::service::extract_browse_metrics(subject_val);
                        if metric.trending.is_none() {
                            metric.trending = Some((1000 - index.min(999)) as f64);
                        }
                        metrics_map.insert(id, metric);
                        items.push(catalog_item);
                    }
                }
            }
        }
    }

    (items, metrics_map)
}

pub fn moviebox_suggest_json_to_strings(payload: &serde_json::Value) -> Vec<String> {
    let items = moviebox_search_json_to_catalog(payload);
    items.into_iter().map(|item| item.title).collect()
}

pub fn moviebox_details_json_to_media_details(
    payload: &serde_json::Value,
) -> Result<MediaDetails, ProviderError> {
    let subject = payload
        .get("data")
        .and_then(|d| d.get("subject"))
        .or_else(|| payload.get("subject"))
        .unwrap_or(payload);

    let id_str = subject
        .get("subjectId")
        .or_else(|| subject.get("id"))
        .and_then(|v| {
            if let Some(num) = v.as_i64() {
                Some(num.to_string())
            } else {
                v.as_str().map(|str_val| str_val.to_string())
            }
        })
        .ok_or(ProviderError::NotFound)?;

    let title = subject
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let stype = subject
        .get("subjectType")
        .or_else(|| subject.get("stype"))
        .and_then(|st| st.as_i64())
        .unwrap_or(1);

    let media_type = if stype == 2 {
        MediaType::Series
    } else {
        MediaType::Movie
    };

    let year = subject
        .get("releaseDate")
        .or_else(|| subject.get("year"))
        .and_then(|y| y.as_str())
        .map(crate::providers::models::extract_4digit_year)
        .filter(|y| !y.is_empty());

    let description = subject
        .get("description")
        .or_else(|| subject.get("intro"))
        .and_then(|d| d.as_str())
        .map(|d| d.to_string());

    let tagline = subject
        .get("tagline")
        .and_then(|t| t.as_str())
        .map(|t| t.to_string());

    let imdb_rating = subject
        .get("imdbRatingValue")
        .or_else(|| subject.get("rating"))
        .and_then(|r| {
            if let Some(n) = r.as_f64() {
                Some(format!("{:.1}", n))
            } else {
                r.as_str().map(|s| s.to_string())
            }
        });

    let director = subject
        .get("director")
        .and_then(|d| d.as_str())
        .map(|d| d.to_string());

    let stars = subject
        .get("stars")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let prints = subject
        .get("prints")
        .and_then(|p| p.as_str())
        .map(|p| p.to_string());

    let audios = subject
        .get("audios")
        .and_then(|a| a.as_str())
        .map(|a| a.to_string());

    let poster_url = subject
        .get("cover")
        .and_then(|c| c.get("url"))
        .or_else(|| subject.get("coverUrl"))
        .and_then(|u| u.as_str())
        .map(|u| u.to_string());

    let duration = subject.get("duration").and_then(|d| {
        if let Some(n) = d.as_u64() {
            if n > 0 {
                Some(format!("{}m", n / 60))
            } else {
                None
            }
        } else {
            d.as_str()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s != "0m" && s != "0")
        }
    });

    let genres = subject
        .get("genre")
        .or_else(|| subject.get("genres"))
        .and_then(|g| g.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut seasons = Vec::new();
    if let Some(seasons_arr) = subject
        .get("seasons")
        .and_then(|s| s.get("seasons").or(Some(s)))
        .and_then(|s| s.as_array())
    {
        for s in seasons_arr {
            let se_num = s.get("se").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let mut episodes = Vec::new();
            if let Some(eps) = s.get("episodeNumbers").and_then(|e| e.as_array()) {
                for ep in eps {
                    if let Some(ep_num) = ep.as_u64() {
                        episodes.push(Episode {
                            season: se_num,
                            number: ep_num as usize,
                            title: None,
                            overview: None,
                        });
                    }
                }
            } else if let Some(max_ep) = s.get("maxEp").and_then(|m| m.as_u64()) {
                for ep_num in 1..=max_ep as usize {
                    episodes.push(Episode {
                        season: se_num,
                        number: ep_num,
                        title: None,
                        overview: None,
                    });
                }
            }
            seasons.push(Season {
                number: se_num,
                episodes,
            });
        }
    }

    let mut dubs = Vec::new();
    if let Some(dubs_arr) = subject.get("dubs").and_then(|d| d.as_array()) {
        for d in dubs_arr {
            let subject_id = d
                .get("subjectId")
                .or_else(|| d.get("id"))
                .and_then(|v| {
                    if let Some(num) = v.as_i64() {
                        Some(num.to_string())
                    } else {
                        v.as_str().map(|str_val| str_val.to_string())
                    }
                })
                .unwrap_or_default();
            let language = d
                .get("lanName")
                .or_else(|| d.get("language"))
                .or_else(|| d.get("lang"))
                .and_then(|l| l.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let label = d
                .get("title")
                .or_else(|| d.get("name"))
                .or_else(|| d.get("lanName"))
                .and_then(|l| l.as_str())
                .unwrap_or(&language)
                .to_string();
            dubs.push(AudioTrackOption {
                subject_id,
                language,
                label,
            });
        }
    }

    Ok(MediaDetails {
        id: ProviderMediaId {
            provider: ProviderKind::MovieBox,
            value: id_str,
        },
        title,
        media_type,
        year,
        description,
        tagline,
        imdb_rating,
        director,
        stars,
        prints,
        audios,
        poster_url,
        duration,
        genres,
        seasons,
        dubs,
    })
}

pub fn moviebox_resource_item_to_release(item: &serde_json::Value) -> Release {
    if let Some(r) = item
        .get("_addon_release")
        .and_then(|val| serde_json::from_value::<Release>(val.clone()).ok())
    {
        return r;
    }

    let filename = item
        .get("fileName")
        .or_else(|| item.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Release")
        .to_string();

    let resolution = item.get("resolution").and_then(|r| {
        if let Some(n) = r.as_u64() {
            Some(format!("{n}p"))
        } else if let Some(n) = r.as_i64() {
            Some(format!("{n}p"))
        } else {
            r.as_str().map(|s| s.to_string())
        }
    });

    let codec = item
        .get("codecName")
        .or_else(|| item.get("codec"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    let language = item
        .get("language")
        .or_else(|| item.get("lanName"))
        .and_then(|l| l.as_str())
        .map(|s| s.to_string());

    let size_bytes = item.get("size").and_then(|s| {
        if let Some(n) = s.as_u64() {
            Some(n)
        } else if let Some(n) = s.as_i64() {
            Some(n as u64)
        } else if let Some(str_val) = s.as_str() {
            str_val.parse::<u64>().ok()
        } else {
            None
        }
    });

    let season = item.get("se").and_then(|v| {
        if let Some(n) = v.as_u64() {
            Some(n as usize)
        } else if let Some(n) = v.as_i64() {
            Some(n as usize)
        } else {
            v.as_str().and_then(|s| s.parse().ok())
        }
    });

    let episode = item.get("ep").and_then(|v| {
        if let Some(n) = v.as_u64() {
            Some(n as usize)
        } else if let Some(n) = v.as_i64() {
            Some(n as usize)
        } else {
            v.as_str().and_then(|s| s.parse().ok())
        }
    });
    let resource_id = item
        .get("resourceId")
        .or_else(|| item.get("id"))
        .and_then(|v| {
            if let Some(num) = v.as_i64() {
                Some(num.to_string())
            } else if let Some(num) = v.as_u64() {
                Some(num.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        });

    let mut mirrors = Vec::new();
    let resource_link = item
        .get("resourceLink")
        .or_else(|| item.get("url"))
        .and_then(|l| l.as_str())
        .filter(|s| !s.is_empty())
        .filter(|s| !is_deprecation_notice_url(s));
    if let Some(link) = resource_link {
        let label = item
            .get("uploadBy")
            .or_else(|| item.get("source"))
            .and_then(|u| u.as_str())
            .unwrap_or("Direct")
            .to_string();

        mirrors.push(SourceMirror {
            label,
            resolver_url: link.to_string(),
            headers: vec![],
            direct_file: false,
        });
    }

    Release {
        provider: ProviderKind::MovieBox,
        filename,
        quality: resolution,
        codec,
        language,
        size_bytes,
        season,
        episode,
        mirrors,
        resource_id,
    }
}

pub fn moviebox_resource_position_json_to_release(
    payload: &serde_json::Value,
    original: &Release,
    user_agent: &str,
) -> Option<Release> {
    let resource_id = original.resource_id.as_deref()?;
    let data = payload.get("data").unwrap_or(payload);
    let items = data.get("list")?.as_array()?;
    for item in items {
        let candidate = moviebox_resource_item_to_release(item);
        if candidate.resource_id.as_deref() != Some(resource_id)
            || candidate.season != original.season
            || candidate.episode != original.episode
        {
            continue;
        }
        let Some(url) = candidate.direct_url() else {
            continue;
        };
        let Ok(parsed) = url::Url::parse(url) else {
            continue;
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            continue;
        }
        let mut release = original.clone();
        release.mirrors = candidate.mirrors;
        for mirror in &mut release.mirrors {
            mirror.direct_file = true;
            // Direct MP4s reject the DASH referer with HTTP 429.
            mirror.headers = vec![("User-Agent".to_string(), user_agent.to_string())];
        }
        return Some(release);
    }
    None
}

pub fn is_deprecation_notice_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("1c7de0bd3393702d9191801f15f88f8d")
        || lower.contains("9a0461bc39da389663bf3dbb17091d3f")
        || lower.contains("b164fbfb4347792950bdfbfb563d39d9")
        || lower.contains("/notice.mp4")
        || (lower.contains("macdn.aoneroom.com") && lower.contains("/other/"))
}

fn to_dash_mpd(url_str: &str) -> Option<String> {
    let base = url_str.trim_end_matches('*').trim_end_matches('/');
    if !base.is_empty()
        && (base.starts_with("http://") || base.starts_with("https://"))
        && base.contains("/dash/")
    {
        Some(format!("{base}/index.mpd"))
    } else {
        None
    }
}

fn decode_b64_padded(mut s: String) -> Option<Vec<u8>> {
    let padding = (4 - s.len() % 4) % 4;
    if padding > 0 {
        s.push_str(&"=".repeat(padding));
    }
    base64::engine::general_purpose::STANDARD
        .decode(s.as_bytes())
        .ok()
}

pub fn resolve_dash_manifest_from_policy(sign_cookie: &str) -> Option<String> {
    for part in sign_cookie.split(';') {
        let trimmed = part.trim();
        if let Some(idx) = trimmed.find("urlprefix=") {
            let prefix_part = &trimmed[idx + "urlprefix=".len()..];
            let b64_token = prefix_part.split(':').next().unwrap_or(prefix_part).trim();
            let normalized: String = b64_token
                .chars()
                .map(|c| match c {
                    '-' => '+',
                    '_' => '/',
                    other => other,
                })
                .collect();
            if let Some(decoded_bytes) = decode_b64_padded(normalized) {
                if let Ok(url_str) = String::from_utf8(decoded_bytes) {
                    if let Some(mpd) = to_dash_mpd(&url_str) {
                        return Some(mpd);
                    }
                }
            }
        }
        if let Some(policy_raw) = trimmed.strip_prefix("CloudFront-Policy=") {
            let policy_clean = policy_raw.trim();
            let normalized: String = policy_clean
                .chars()
                .map(|c| match c {
                    '-' => '+',
                    '_' => '=',
                    '~' => '/',
                    other => other,
                })
                .collect();

            let Some(decoded_bytes) = decode_b64_padded(normalized) else {
                continue;
            };

            let Ok(json) = serde_json::from_slice::<serde_json::Value>(&decoded_bytes) else {
                continue;
            };

            let Some(resource) = json
                .get("Statement")
                .and_then(|s| s.as_array())
                .and_then(|arr| arr.first())
                .and_then(|st| st.get("Resource"))
                .and_then(|r| r.as_str())
            else {
                continue;
            };

            if let Some(mpd) = to_dash_mpd(resource) {
                return Some(mpd);
            }
        }
    }
    None
}

pub fn moviebox_play_info_json_to_releases(
    payload: &serde_json::Value,
    season: usize,
    episode: usize,
    user_agent: &str,
) -> Vec<Release> {
    let data = payload.get("data").unwrap_or(payload);
    let raw_title_prefix = data
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("MovieBox Stream");
    let title_prefix = crate::providers::moviebox::title::clean_moviebox_title(raw_title_prefix);
    let Some(streams) = data.get("streams").and_then(|s| s.as_array()) else {
        return Vec::new();
    };

    let mut releases = Vec::new();

    for stream in streams {
        let stream_id = stream.get("id").and_then(|v| {
            if let Some(n) = v.as_i64() {
                Some(n.to_string())
            } else if let Some(n) = v.as_u64() {
                Some(n.to_string())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        });
        let format_type = stream
            .get("format")
            .and_then(|f| f.as_str())
            .unwrap_or("MP4");
        let codec = stream
            .get("codecName")
            .or_else(|| stream.get("codec"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());
        let size_bytes = stream.get("size").and_then(|s| {
            if let Some(n) = s.as_u64() {
                Some(n)
            } else if let Some(n) = s.as_i64() {
                Some(n as u64)
            } else if let Some(str_val) = s.as_str() {
                str_val.parse::<u64>().ok()
            } else {
                None
            }
        });

        let resolutions_str = stream
            .get("resolutions")
            .or_else(|| data.get("displayResolutions"))
            .and_then(|r| r.as_str())
            .unwrap_or("1080,720,480");

        let sign_cookie = stream
            .get("signCookie")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let stream_url = stream.get("url").and_then(|u| u.as_str()).unwrap_or("");

        let manifest_url = resolve_dash_manifest_from_policy(sign_cookie).or_else(|| {
            if is_deprecation_notice_url(stream_url) {
                None
            } else if stream_url.starts_with("http") {
                Some(stream_url.to_string())
            } else {
                None
            }
        });
        let Some(playable_url) = manifest_url else {
            continue;
        };

        let mut headers = vec![
            ("Referer".to_string(), super::STREAM_REFERER.to_string()),
            ("User-Agent".to_string(), user_agent.to_string()),
        ];
        if !sign_cookie.is_empty() {
            let clean_cookie = sign_cookie
                .trim_end_matches(';')
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            headers.push(("Cookie".to_string(), clean_cookie));
        }
        let mut parsed_resolutions = resolutions_str
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect::<Vec<_>>();
        parsed_resolutions.sort_unstable_by(|a, b| b.cmp(a));
        parsed_resolutions.dedup();

        let res_list = if parsed_resolutions.is_empty() {
            vec![1080]
        } else {
            parsed_resolutions
        };

        let codec_disp = codec.as_deref().unwrap_or(format_type);

        let highest_res = res_list.first().copied().unwrap_or(1080) as f64;

        for res in res_list {
            let res_label = format!("{res}p");
            let filename = if season > 0 && episode > 0 {
                format!("{title_prefix} S{season:02}E{episode:02} {res_label} {codec_disp}")
            } else {
                format!("{title_prefix} {res_label} {codec_disp}")
            };

            let mirror = SourceMirror {
                label: format!("{res_label} {codec_disp}"),
                resolver_url: playable_url.clone(),
                headers: headers.clone(),
                direct_file: true,
            };

            let scaled_size = size_bytes.map(|total| {
                if (res as f64) >= highest_res || highest_res <= 0.0 {
                    total
                } else {
                    let scale = (res as f64 / highest_res).powf(1.6);
                    (total as f64 * scale.clamp(0.15, 1.0)) as u64
                }
            });

            releases.push(Release {
                provider: ProviderKind::MovieBox,
                filename,
                quality: Some(res_label),
                codec: codec.clone(),
                language: None,
                size_bytes: scaled_size,
                season: if season > 0 { Some(season) } else { None },
                episode: if episode > 0 { Some(episode) } else { None },
                mirrors: vec![mirror],
                resource_id: stream_id.clone(),
            });
        }
    }

    releases
}

pub fn moviebox_resource_json_to_releases(payload: &serde_json::Value) -> Vec<Release> {
    let items = if let Some(list) = payload.get("list").and_then(|l| l.as_array()) {
        list.as_slice()
    } else if let Some(arr) = payload.as_array() {
        arr.as_slice()
    } else {
        &[]
    };
    items
        .iter()
        .map(moviebox_resource_item_to_release)
        .filter(|r| !r.mirrors.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_position_recovers_file_and_preserves_release_metadata() {
        let original = moviebox_resource_item_to_release(&json!({
            "resourceId": 6154402015721538488_i64,
            "title": "Example Movie",
            "resourceLink": "https://macdn.aoneroom.com/other/2026/10/10/upgrade.mp4",
            "resolution": 1080,
            "codecName": "hevc",
            "size": "1010288591",
            "se": 0,
            "ep": 0
        }));
        assert!(original.mirrors.is_empty());
        let payload = json!({"data": {"list": [{
            "resourceId": "6154402015721538488",
            "resourceLink": "https://cdn.example.com/resource/movie.mp4?sign=abc&t=123",
            "se": 0,
            "ep": 0
        }]}});
        let recovered =
            moviebox_resource_position_json_to_release(&payload, &original, "TestAgent")
                .expect("recover original file");
        assert_eq!(recovered.filename, original.filename);
        assert_eq!(recovered.quality.as_deref(), Some("1080p"));
        assert_eq!(recovered.codec, original.codec);
        assert_eq!(recovered.size_bytes, Some(1010288591));
        assert_eq!(recovered.resource_id, original.resource_id);
        assert_eq!(
            recovered.direct_url(),
            Some("https://cdn.example.com/resource/movie.mp4?sign=abc&t=123")
        );
        assert!(recovered.mirrors[0].direct_file);
        assert_eq!(
            recovered.mirrors[0].headers,
            vec![("User-Agent".to_string(), "TestAgent".to_string())]
        );
    }

    #[test]
    fn test_resource_position_rejects_notices_wrong_resources_and_wrong_episodes() {
        let original = moviebox_resource_item_to_release(&json!({
            "resourceId": "episode2",
            "se": "1",
            "ep": "2"
        }));
        for item in [
            json!({"resourceId": "episode1", "se": 1, "ep": 2,
                "resourceLink": "https://cdn.example.com/another.mp4"}),
            json!({"resourceId": "episode2", "se": 1, "ep": 1,
                "resourceLink": "https://cdn.example.com/wrong-episode.mp4"}),
            json!({"resourceId": "episode2", "se": 2, "ep": 2,
                "resourceLink": "https://cdn.example.com/wrong-season.mp4"}),
            json!({"resourceId": "episode2", "se": 1, "ep": 2,
                "resourceLink": "https://macdn.aoneroom.com/other/2026/10/10/upgrade.mp4"}),
            json!({"resourceId": "episode2", "se": 1, "ep": 2,
                "resourceLink": "file:///tmp/movie.mp4"}),
            json!({"resourceId": "episode2", "se": 1, "ep": 2,
                "resourceLink": ""}),
        ] {
            assert!(
                moviebox_resource_position_json_to_release(
                    &json!({"list": [item]}),
                    &original,
                    "TestAgent"
                )
                .is_none()
            );
        }
        for payload in [json!({}), json!({"list": []}), json!({"data": null})] {
            assert!(
                moviebox_resource_position_json_to_release(&payload, &original, "TestAgent")
                    .is_none()
            );
        }

        let payload = json!({"list": [
            {"resourceId": "episode1", "se": 1, "ep": 1,
                "resourceLink": "https://cdn.example.com/episode1.mp4"},
            {"resourceId": "episode2", "se": 1, "ep": 2,
                "resourceLink": "https://cdn.example.com/episode2.mp4"}
        ]});
        let recovered =
            moviebox_resource_position_json_to_release(&payload, &original, "TestAgent")
                .expect("recover matching episode only");
        assert_eq!(recovered.season, Some(1));
        assert_eq!(recovered.episode, Some(2));
        assert_eq!(
            recovered.direct_url(),
            Some("https://cdn.example.com/episode2.mp4")
        );
    }

    #[test]
    fn test_moviebox_search_json_to_catalog() {
        let payload = json!({
            "data": {
                "results": [{
                    "subjects": [
                        {
                            "subjectId": "12345",
                            "title": "Inception",
                            "subjectType": 1,
                            "releaseDate": "2010-07-16",
                            "cover": { "url": "https://example.com/cover.jpg" },
                            "season": 0
                        },
                        {
                            "subjectId": "67890",
                            "title": "Breaking Bad",
                            "subjectType": 2,
                            "releaseDate": "2008",
                            "cover": { "url": "https://example.com/bb.jpg" },
                            "season": 5
                        }
                    ]
                }]
            }
        });

        let catalog = moviebox_search_json_to_catalog(&payload);
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog[0].id.value, "12345");
        assert_eq!(catalog[0].title, "Inception");
        assert_eq!(catalog[0].media_type, MediaType::Movie);
        assert_eq!(catalog[0].year.as_deref(), Some("2010"));

        assert_eq!(catalog[1].id.value, "67890");
        assert_eq!(catalog[1].title, "Breaking Bad");
        assert_eq!(catalog[1].media_type, MediaType::Series);
        assert_eq!(catalog[1].season_count, Some(5));
    }

    #[test]
    fn test_moviebox_details_json_to_media_details() {
        let payload = json!({
            "data": {
                "subject": {
                    "subjectId": "999",
                    "title": "Interstellar",
                    "subjectType": 1,
                    "releaseDate": "2014",
                    "description": "Space exploration film",
                    "imdbRatingValue": "8.7",
                    "director": "Christopher Nolan",
                    "stars": "Matthew McConaughey, Anne Hathaway",
                    "duration": 10140,
                    "genre": ["Sci-Fi", "Adventure"],
                    "cover": { "url": "https://example.com/interstellar.jpg" }
                }
            }
        });

        let details = moviebox_details_json_to_media_details(&payload).unwrap();
        assert_eq!(details.id.value, "999");
        assert_eq!(details.title, "Interstellar");
        assert_eq!(details.media_type, MediaType::Movie);
        assert_eq!(details.director.as_deref(), Some("Christopher Nolan"));
        assert_eq!(details.imdb_rating.as_deref(), Some("8.7"));
        assert_eq!(details.duration.as_deref(), Some("169m"));
        assert_eq!(details.genres, vec!["Sci-Fi", "Adventure"]);
    }

    #[test]
    fn test_moviebox_details_with_dubs_and_resource_conversion() {
        let payload = json!({
            "data": {
                "subject": {
                    "subjectId": "500",
                    "title": "Money Heist",
                    "subjectType": 2,
                    "dubs": [
                        { "subjectId": "500", "lanName": "Original", "title": "Spanish (Original)" },
                        { "subjectId": "501", "lanName": "English", "title": "English Dub" }
                    ],
                    "seasons": {
                        "seasons": [
                            { "se": 1, "maxEp": 13 }
                        ]
                    }
                }
            }
        });

        let details = moviebox_details_json_to_media_details(&payload).unwrap();
        assert!(details.is_series());
        assert!(details.has_languages());
        assert_eq!(details.dubs.len(), 2);
        assert_eq!(details.dubs[0].subject_id, "500");
        assert_eq!(details.dubs[0].language, "Original");
        assert_eq!(details.dubs[1].subject_id, "501");
        assert_eq!(details.dubs[1].language, "English");

        let resource_payload = json!({
            "list": [
                {
                    "fileName": "Money.Heist.S01E01.1080p.NF.WEB-DL.x265",
                    "resolution": 1080,
                    "codecName": "hevc",
                    "size": "850000000",
                    "se": 1,
                    "ep": 1,
                    "resourceLink": "https://stream.example.com/mh0101.mp4",
                    "uploadBy": "NF"
                }
            ]
        });

        let releases = moviebox_resource_json_to_releases(&resource_payload);
        assert_eq!(releases.len(), 1);
        assert_eq!(
            releases[0].filename,
            "Money.Heist.S01E01.1080p.NF.WEB-DL.x265"
        );
        assert_eq!(releases[0].resolution_u64(), 1080);
        assert_eq!(releases[0].codec.as_deref(), Some("hevc"));
        assert_eq!(releases[0].season, Some(1));
        assert_eq!(releases[0].episode, Some(1));
        assert_eq!(
            releases[0].direct_url(),
            Some("https://stream.example.com/mh0101.mp4")
        );
        assert_eq!(releases[0].source_label(), "NF");
    }

    #[test]
    fn test_resolve_dash_manifest_from_policy() {
        let policy_json = r#"{"Statement":[{"Resource":"https://sacdn.hakunaymatata.com/dash/4179386086617137184_0_0_1080_h265_518/*","Condition":{"DateLessThan":{"AWS:EpochTime":1788879894}}}]}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(policy_json.as_bytes());
        let cookie = format!(
            "CloudFront-Policy={b64};CloudFront-Signature=abc;CloudFront-Key-Pair-Id=KMHN1LQ1HEUPL;"
        );

        let resolved = resolve_dash_manifest_from_policy(&cookie);
        assert_eq!(
            resolved.as_deref(),
            Some(
                "https://sacdn.hakunaymatata.com/dash/4179386086617137184_0_0_1080_h265_518/index.mpd"
            )
        );
    }

    #[test]
    fn test_moviebox_play_info_json_to_releases_with_signed_cookie() {
        let policy_json = r#"{"Statement":[{"Resource":"https://sacdn.example.com/dash/12345_0_0_1080_h265_518/*"}]}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(policy_json.as_bytes());
        let cookie = format!(
            "CloudFront-Policy={b64};CloudFront-Signature=xyz;CloudFront-Key-Pair-Id=K123;"
        );

        let payload = json!({
            "code": 0,
            "message": "ok",
            "data": {
                "title": "Sample Movie",
                "displayResolutions": "480,720,1080",
                "streams": [
                    {
                        "id": "9999",
                        "format": "MP4",
                        "codecName": "hevc",
                        "duration": 7200,
                        "size": "1500000000",
                        "resolutions": "1080,720,480",
                        "url": "https://macdn.example.com/notice.mp4",
                        "signCookie": cookie
                    }
                ]
            }
        });

        let releases = moviebox_play_info_json_to_releases(&payload, 0, 0, "TestAgent/1.0");
        assert_eq!(releases.len(), 3);
        assert_eq!(releases[0].quality.as_deref(), Some("1080p"));
        assert_eq!(releases[0].resolution_u64(), 1080);
        assert_eq!(releases[1].quality.as_deref(), Some("720p"));
        assert_eq!(releases[1].resolution_u64(), 720);
        assert_eq!(releases[2].quality.as_deref(), Some("480p"));
        assert_eq!(releases[2].resolution_u64(), 480);
        assert_eq!(releases[0].codec.as_deref(), Some("hevc"));
        assert_eq!(
            releases[0].direct_url(),
            Some("https://sacdn.example.com/dash/12345_0_0_1080_h265_518/index.mpd")
        );
        let mirror = &releases[0].mirrors[0];
        assert!(
            mirror
                .headers
                .iter()
                .any(|(k, v)| k == "Referer" && v == crate::providers::moviebox::STREAM_REFERER)
        );
        assert!(
            mirror
                .headers
                .iter()
                .any(|(k, v)| k == "Cookie" && v.contains("CloudFront-Policy="))
        );
    }

    #[test]
    fn test_moviebox_play_info_empty_or_invalid_streams() {
        let empty_payload = json!({
            "code": 0,
            "data": {
                "title": "Empty",
                "streams": []
            }
        });
        let releases = moviebox_play_info_json_to_releases(&empty_payload, 0, 0, "TestAgent/1.0");
        assert!(releases.is_empty());

        let invalid_cookie = "CloudFront-Policy=invalid_base64;";
        assert_eq!(resolve_dash_manifest_from_policy(invalid_cookie), None);
    }

    #[test]
    fn test_resolve_dash_manifest_from_policy_base64_variants() {
        for (slug, transform) in [
            ("item1", "standard"),
            ("test_dash", "dash"),
            ("test_underscore", "underscore"),
            ("test_tilde", "tilde"),
            ("test_pad", "unpadded"),
        ] {
            let policy_json = format!(
                r#"{{"Statement":[{{"Resource":"https://sacdn.hakunaymatata.com/dash/{slug}/*"}}]}}"#
            );
            let b64 = base64::engine::general_purpose::STANDARD.encode(policy_json.as_bytes());
            let cf_b64 = match transform {
                "dash" => b64.replace('+', "-"),
                "underscore" => b64.replace('=', "_"),
                "tilde" => b64.replace('/', "~"),
                "unpadded" => b64.trim_end_matches('=').to_string(),
                _ => b64,
            };
            let cookie = format!("CloudFront-Policy={cf_b64};CloudFront-Signature=abc;");
            let resolved = resolve_dash_manifest_from_policy(&cookie);
            assert_eq!(
                resolved.as_deref(),
                Some(format!("https://sacdn.hakunaymatata.com/dash/{slug}/index.mpd").as_str())
            );
        }
    }

    #[test]
    fn test_resolve_dash_manifest_from_malformed_policy() {
        assert_eq!(
            resolve_dash_manifest_from_policy("CloudFront-Policy=invalid!!not_b64"),
            None
        );
        assert_eq!(
            resolve_dash_manifest_from_policy("CloudFront-Policy=e30="),
            None
        );
        assert_eq!(
            resolve_dash_manifest_from_policy("SomeOtherCookie=123"),
            None
        );
    }

    #[test]
    fn test_resolve_dash_manifest_from_edge_cache_cookies() {
        for (cookie, expected) in [
            (
                "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20vZGFzaC8zMjY0NzcyNTg4MzMzMTU3NDI0XzBfMF8xMDgwX2gyNjVfNTYwLw:sign=f1a522f7bf4c548c981ad6efa88925ad:t=1789908742",
                "https://sbcdn3.hakunaymatata.com/dash/3264772588333157424_0_0_1080_h265_560/index.mpd",
            ),
            (
                "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20vZGFzaC9pdGVtMQ:sign=abc:t=123",
                "https://sbcdn3.hakunaymatata.com/dash/item1/index.mpd",
            ),
        ] {
            assert_eq!(
                resolve_dash_manifest_from_policy(cookie).as_deref(),
                Some(expected)
            );
        }
        let hls_cookie = "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20vaGxzLzUyODM0MjAzNzkzODAzMzU1NzYtMS0xLTcyMC05NDYv:sign=9d668fe770e7d41d067f80422544d47e:t=1791308037";
        assert_eq!(resolve_dash_manifest_from_policy(hls_cookie), None);
    }

    #[test]
    fn test_moviebox_play_info_with_edge_cache_cookie_produces_release() {
        let payload = json!({
            "code": 0,
            "data": {
                "title": "Ek Deewane Ki Deewaniyat",
                "displayResolutions": "1080,720,480",
                "streams": [
                    {
                        "id": "1849447804066746512",
                        "format": "MP4",
                        "codecName": "hevc",
                        "resolutions": "1080,720,480",
                        "size": "1682353414",
                        "duration": 8372,
                        "url": "https://macdn.aoneroom.com/other/2026/09/04/b164fbfb4347792950bdfbfb563d39d9.mp4",
                        "signCookie": "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20vZGFzaC8zMjY0NzcyNTg4MzMzMTU3NDI0XzBfMF8xMDgwX2gyNjVfNTYwLw:sign=f1a522f7bf4c548c981ad6efa88925ad:t=1789908742"
                    }
                ]
            }
        });

        let releases = moviebox_play_info_json_to_releases(&payload, 0, 0, "TestAgent/1.0");
        assert_eq!(releases.len(), 3);
        assert_eq!(
            releases[0].direct_url(),
            Some(
                "https://sbcdn3.hakunaymatata.com/dash/3264772588333157424_0_0_1080_h265_560/index.mpd"
            )
        );
        assert_eq!(releases[0].quality.as_deref(), Some("1080p"));
        assert_eq!(releases[1].quality.as_deref(), Some("720p"));
        assert_eq!(releases[2].quality.as_deref(), Some("480p"));
        assert_eq!(releases[0].codec.as_deref(), Some("hevc"));
    }
    #[test]
    fn test_notice_fallback_rejected_when_signed_policy_missing() {
        let payload = json!({
            "code": 0,
            "data": {
                "title": "Deprecation Notice Title",
                "streams": [
                    {
                        "id": "1",
                        "format": "MP4",
                        "codecName": "h264",
                        "resolutions": "480",
                        "url": "https://macdn.aoneroom.com/other/2026/09/01/9a0461bc39da389663bf3dbb17091d3f.mp4",
                        "signCookie": ""
                    }
                ]
            }
        });

        let releases = moviebox_play_info_json_to_releases(&payload, 0, 0, "TestAgent/1.0");
        assert!(
            releases.is_empty(),
            "Deprecation notice MP4 must be rejected"
        );
    }
    #[test]
    fn test_hls_signed_policy_rejected_as_dash_manifest() {
        let payload = json!({
            "code": 0,
            "data": {
                "title": "Shark: The Storm",
                "displayResolutions": "720",
                "streams": [
                    {
                        "id": "1886864337231398728",
                        "format": "MP4",
                        "codecName": "h264",
                        "resolutions": "720",
                        "size": "198337682",
                        "duration": 1595,
                        "url": "https://macdn.aoneroom.com/other/2026/09/04/b164fbfb4347792950bdfbfb563d39d9.mp4",
                        "signCookie": "Edge-Cache-Cookie=urlprefix=aHR0cHM6Ly9zYmNkbjMuaGFrdW5heW1hdGF0YS5jb20vaGxzLzQwNDIzMTc5OTc2NzAyNDg1Ni0xLTEtNzIwLTk3NC8:sign=abc:t=1791308126"
                    }
                ]
            }
        });
        let releases = moviebox_play_info_json_to_releases(&payload, 1, 1, "TestAgent/1.0");
        assert!(releases.is_empty());
    }

    #[test]
    fn test_legitimate_direct_stream_url_fallback_preserved() {
        let payload = json!({
            "code": 0,
            "data": {
                "title": "Legitimate Movie",
                "streams": [
                    {
                        "id": "1",
                        "format": "MP4",
                        "codecName": "h264",
                        "resolutions": "1080",
                        "url": "https://cdn.example.com/legitimate_movie.mp4",
                        "signCookie": ""
                    }
                ]
            }
        });

        let releases = moviebox_play_info_json_to_releases(&payload, 0, 0, "TestAgent/1.0");
        assert_eq!(releases.len(), 1);
        assert_eq!(
            releases[0].direct_url(),
            Some("https://cdn.example.com/legitimate_movie.mp4")
        );
    }

    #[test]
    fn test_captions_json_to_options_wrapper_and_language_fallback() {
        let wrapped_payload = json!({
            "code": 0,
            "data": {
                "extCaptions": [
                    {
                        "id": "1",
                        "lan": "en",
                        "lanName": "English",
                        "size": "1000",
                        "url": "https://example.com/en.srt"
                    },
                    {
                        "id": "2",
                        "lan": "es",
                        "lanName": "",
                        "size": "1200",
                        "url": "https://example.com/es.srt"
                    },
                    {
                        "id": "3",
                        "lan": "",
                        "lanName": "",
                        "url": ""
                    },
                    {
                        "id": "4",
                        "lan": "en",
                        "lanName": "IN",
                        "size": "34",
                        "url": "https://pacdn.aoneroom.com/other/2024/09/18/aa348f2541d13ffe1a8ea6f9e14f3ed5.srt"
                    }
                ]
            }
        });

        let options = captions_json_to_options(&wrapped_payload);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].name, "English");
        assert_eq!(options[0].url, "https://example.com/en.srt");
        assert_eq!(options[1].name, "es");
        assert_eq!(options[1].url, "https://example.com/es.srt");

        let unwrapped_payload = json!({
            "extCaptions": [
                {
                    "id": "10",
                    "lan": "fr",
                    "lanName": "French",
                    "size": "500",
                    "url": "https://example.com/fr.srt"
                },
                {
                    "id": "11",
                    "lan": "fr",
                    "lanName": "French Duplicate",
                    "size": "500",
                    "url": "https://example.com/fr.srt"
                }
            ]
        });
        let unwrapped_options = captions_json_to_options(&unwrapped_payload);
        assert_eq!(unwrapped_options.len(), 1);
        assert_eq!(unwrapped_options[0].name, "French");
    }

    #[test]
    fn test_moviebox_play_info_preserves_numeric_and_string_resource_id() {
        let payload = json!({
            "code": 0,
            "data": {
                "title": "Movie Title",
                "streams": [
                    {
                        "id": 167282974499786072_i64,
                        "format": "MP4",
                        "codecName": "hevc",
                        "resolutions": "1080",
                        "url": "https://cdn.example.com/stream1.mp4",
                        "signCookie": ""
                    },
                    {
                        "id": "9026715103487476952",
                        "format": "MP4",
                        "codecName": "h264",
                        "resolutions": "720",
                        "url": "https://cdn.example.com/stream2.mp4",
                        "signCookie": ""
                    }
                ]
            }
        });

        let releases = moviebox_play_info_json_to_releases(&payload, 0, 0, "TestAgent/1.0");
        assert_eq!(releases.len(), 2);
        assert_eq!(
            releases[0].resource_id.as_deref(),
            Some("167282974499786072")
        );
        assert_eq!(
            releases[1].resource_id.as_deref(),
            Some("9026715103487476952")
        );
    }

    #[test]
    fn test_moviebox_resource_item_preserves_resource_id() {
        let item_numeric = json!({
            "resourceId": 6065869889208832296_i64,
            "title": "Test Release",
            "url": "https://example.com/file.mkv"
        });
        let release_numeric = moviebox_resource_item_to_release(&item_numeric);
        assert_eq!(
            release_numeric.resource_id.as_deref(),
            Some("6065869889208832296")
        );

        let item_str = json!({
            "id": "1234567890",
            "title": "Test Release 2",
            "url": "https://example.com/file2.mkv"
        });
        let release_str = moviebox_resource_item_to_release(&item_str);
        assert_eq!(release_str.resource_id.as_deref(), Some("1234567890"));
    }
    #[test]
    fn test_moviebox_resource_item_filters_deprecation_notice_url() {
        let notice_item = json!({
            "resourceId": 4087841675127701904_i64,
            "title": "Steins;Gate S01E07",
            "url": "https://macdn.aoneroom.com/other/2026/09/04/b164fbfb4347792950bdfbfb563d39d9.mp4"
        });
        let release = moviebox_resource_item_to_release(&notice_item);
        assert!(release.mirrors.is_empty());

        let payload = json!({
            "list": [
                notice_item,
                {
                    "resourceId": 123,
                    "title": "Valid Stream",
                    "url": "https://example.com/stream.mp4"
                }
            ]
        });
        let releases = moviebox_resource_json_to_releases(&payload);
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].filename, "Valid Stream");
    }
}
