use super::adapter::stream_item_to_release;
use super::client::AddonClient;
use super::models::InstalledAddon;
use super::torrent::{is_gateway_stream_url, is_low_quality_source, stream_seeders};
use crate::providers::models::Release;

/// Longest an addon gets to answer, retry included.
const ADDON_STREAM_DEADLINE: std::time::Duration = std::time::Duration::from_secs(22);

pub fn clean_addon_subject_id(subject_id: &str) -> &str {
    subject_id
        .strip_prefix("series:")
        .or_else(|| subject_id.strip_prefix("movie:"))
        .unwrap_or(subject_id)
}

/// Everything one stream lookup learned: the ranked releases, the addons that
/// answered but offered nothing playable, and the addons that failed outright
/// (with the reason), so the UI can say why a source is missing.
#[derive(Debug, Default)]
pub struct StreamAggregation {
    pub releases: Vec<Release>,
    pub blocked: Vec<String>,
    pub failed: Vec<(String, String)>,
}

pub async fn aggregate_streams(
    client: &AddonClient,
    addons: &[InstalledAddon],
    subject_id: &str,
    season: usize,
    episode: usize,
    is_series: bool,
) -> (Vec<Release>, Vec<String>) {
    let agg =
        aggregate_streams_detailed(client, addons, subject_id, season, episode, is_series).await;
    (agg.releases, agg.blocked)
}

pub async fn aggregate_streams_detailed(
    client: &AddonClient,
    addons: &[InstalledAddon],
    subject_id: &str,
    season: usize,
    episode: usize,
    is_series: bool,
) -> StreamAggregation {
    let clean_id = clean_addon_subject_id(subject_id);
    let media_type = if is_series { "series" } else { "movie" };
    let stream_addons: Vec<&InstalledAddon> = addons
        .iter()
        .filter(|a| {
            a.enabled
                && a.provides_stream
                && (a.types.is_empty()
                    || a.types.iter().any(|t| t.eq_ignore_ascii_case(media_type)))
                && (a.id_prefixes.is_empty()
                    || a.id_prefixes
                        .iter()
                        .any(|p| clean_id.starts_with(p.as_str())))
        })
        .collect();

    if stream_addons.is_empty() {
        return StreamAggregation::default();
    }
    let stream_id = if is_series && episode > 0 {
        let suffix = format!(":{season}:{episode}");
        if clean_id.ends_with(&suffix) {
            clean_id.to_string()
        } else {
            format!("{clean_id}{suffix}")
        }
    } else {
        clean_id.to_string()
    };

    let mut tasks = Vec::new();
    for addon in stream_addons {
        let base_url = AddonClient::base_addon_url(&addon.manifest_url);
        let addon_name = addon.name.clone();
        let client_clone = client.clone();
        let id_clone = stream_id.clone();
        let m_type = media_type.to_string();
        // Nuvio scrapers are looked up by the bare IMDb id, not "tt…:season:episode".
        let nuvio_addon = super::nuvio::is_nuvio_addon(addon).then(|| addon.clone());
        let imdb_id = clean_id
            .split(':')
            .next()
            .filter(|id| id.starts_with("tt"))
            .unwrap_or("")
            .to_string();

        tasks.push(async move {
            let streams_res = if let Some(nuvio_addon) = nuvio_addon {
                tokio::time::timeout(
                    super::nuvio::SCRAPER_DEADLINE,
                    super::nuvio::fetch_streams(&nuvio_addon, &imdb_id, &m_type, season, episode),
                )
                .await
            } else {
                // One quick retry covers a dropped connection or a rate-limit
                // blip; the whole attempt still has to fit in the deadline.
                tokio::time::timeout(ADDON_STREAM_DEADLINE, async {
                    let first = client_clone
                        .fetch_streams(&base_url, &m_type, &id_clone)
                        .await;
                    if first.is_ok() {
                        return first;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    client_clone
                        .fetch_streams(&base_url, &m_type, &id_clone)
                        .await
                })
                .await
            };
            match streams_res {
                Ok(Ok(items)) => {
                    let mut releases: Vec<(Release, Option<u32>)> = Vec::new();
                    for item in &items {
                        if let Some(rel) =
                            stream_item_to_release(&addon_name, item, season, episode)
                        {
                            let seeders = rel
                                .direct_url()
                                .filter(|u| is_gateway_stream_url(u))
                                .and_then(|_| stream_seeders(item));
                            releases.push((rel, seeders));
                        }
                    }
                    if !items.is_empty() && releases.is_empty() {
                        (releases, Some(addon_name), None)
                    } else {
                        (releases, None, None)
                    }
                }
                Ok(Err(err)) => {
                    log::warn!("addon {addon_name} failed to fetch streams: {err}");
                    (Vec::new(), None, Some((addon_name, err)))
                }
                Err(_) => {
                    log::warn!("addon {addon_name} timed out fetching streams");
                    (
                        Vec::new(),
                        None,
                        Some((addon_name, "timed out".to_string())),
                    )
                }
            }
        });
    }

    let results = futures::future::join_all(tasks).await;
    let mut all_releases: Vec<(Release, Option<u32>)> = Vec::new();
    let mut blocked_addons = Vec::new();
    let mut failed_addons: Vec<(String, String)> = Vec::new();

    for res in results {
        all_releases.extend(res.0);
        if let Some(blocked) = res.1 {
            blocked_addons.push(blocked);
        }
        if let Some(failed) = res.2 {
            failed_addons.push(failed);
        }
    }

    let mut deduplicated: Vec<(Release, Option<u32>)> = Vec::new();
    for (rel, seeders) in all_releases {
        let direct = rel.direct_url().unwrap_or("").to_string();
        if direct.is_empty() {
            deduplicated.push((rel, seeders));
            continue;
        }
        if let Some((existing, existing_seeders)) = deduplicated
            .iter_mut()
            .find(|(r, _)| r.direct_url() == Some(&direct))
        {
            *existing_seeders = (*existing_seeders).max(seeders);
            for m in rel.mirrors {
                if !existing
                    .mirrors
                    .iter()
                    .any(|em| em.resolver_url == m.resolver_url && em.label == m.label)
                {
                    existing.mirrors.push(m);
                }
            }
        } else {
            deduplicated.push((rel, seeders));
        }
    }

    // Ranking, best first:
    //   1. swarm tier  - healthy swarms and direct HTTP streams on top, dead or
    //      barely seeded torrents and CAM/telesync rips at the bottom
    //   2. quality     - 2160p > 1080p > 720p > 480p
    //   3. seeders     - more seeders first; streams without a count (HTTP)
    //      count as the best, since they start instantly
    //   4. size, then source label, for a stable order
    deduplicated.sort_by(|(a, seeders_a), (b, seeders_b)| {
        let is_torrent_a = a.direct_url().is_some_and(is_gateway_stream_url);
        let is_torrent_b = b.direct_url().is_some_and(is_gateway_stream_url);
        let tier_a = swarm_tier(*seeders_a, is_torrent_a, &a.filename);
        let tier_b = swarm_tier(*seeders_b, is_torrent_b, &b.filename);
        tier_b
            .cmp(&tier_a)
            .then_with(|| {
                quality_score(b.quality.as_deref()).cmp(&quality_score(a.quality.as_deref()))
            })
            .then_with(|| {
                seeders_b
                    .unwrap_or(u32::MAX)
                    .cmp(&seeders_a.unwrap_or(u32::MAX))
            })
            .then_with(|| b.size_bytes.unwrap_or(0).cmp(&a.size_bytes.unwrap_or(0)))
            .then_with(|| {
                let label_a = a.mirrors.first().map(|m| m.label.as_str()).unwrap_or("");
                let label_b = b.mirrors.first().map(|m| m.label.as_str()).unwrap_or("");
                label_a.cmp(label_b)
            })
    });

    let deduplicated: Vec<Release> = deduplicated.into_iter().map(|(rel, _)| rel).collect();
    StreamAggregation {
        releases: deduplicated,
        blocked: blocked_addons,
        failed: failed_addons,
    }
}

/// Seeder counts that split torrents into tiers. Within a tier the better
/// quality wins, so a healthy 1080p ranks above a barely-seeded 2160p.
const SEEDS_HEALTHY: u32 = 30;
const SEEDS_OK: u32 = 5;

/// Playability tier, best first:
/// 0 = CAM/telesync-type rip, 1 = dead torrent (0 seeds), 2 = barely alive (1–4),
/// 3 = torrent with unknown seeder count, 4 = workable swarm (5–29),
/// 5 = healthy swarm (30+) or direct HTTP / Debrid stream.
fn swarm_tier(seeders: Option<u32>, is_torrent: bool, filename: &str) -> u8 {
    if is_low_quality_source(filename) {
        return 0;
    }
    if !is_torrent {
        return 5;
    }
    match seeders {
        Some(0) => 1,
        Some(n) if n < SEEDS_OK => 2,
        None => 3,
        Some(n) if n < SEEDS_HEALTHY => 4,
        Some(_) => 5,
    }
}

fn quality_score(quality: Option<&str>) -> u32 {
    match quality {
        Some("2160p") => 40,
        Some("1080p") => 30,
        Some("720p") => 20,
        Some("480p") => 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_aggregate_streams_merges_deduplicates_sorts_and_reports_unplayable_addons() {
        let listener1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr1 = listener1.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener1.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"streams":[
                    {"name":"AddonA 720p","title":"Movie.720p.mp4\n💾 1.0 GB","url":"https://cdn.example.com/720p.mp4"},
                    {"name":"AddonA 4K","title":"Movie.2160p.HEVC.mkv\n💾 10.0 GB","url":"https://cdn.example.com/2160p.mkv"}
                ]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener2.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"streams":[
                    {"name":"TorrentOnly","title":"P2P Swarm","infoHash":"not-a-hash"}
                ]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            }
        });

        let addons = vec![
            InstalledAddon {
                manifest_url: format!("http://{addr1}/manifest.json"),
                name: "HTTP Addon".to_string(),
                version: Some("1.0.0".to_string()),
                description: None,
                enabled: true,
                provides_catalog: false,
                provides_meta: false,
                provides_stream: true,
                id_prefixes: vec![],
                types: vec!["movie".to_string()],
                catalogs: vec![],
            },
            InstalledAddon {
                manifest_url: format!("http://{addr2}/manifest.json"),
                name: "P2P Addon".to_string(),
                version: Some("1.0.0".to_string()),
                description: None,
                enabled: true,
                provides_catalog: false,
                provides_meta: false,
                provides_stream: true,
                id_prefixes: vec![],
                types: vec!["movie".to_string()],
                catalogs: vec![],
            },
        ];

        let client = AddonClient::new();
        let (releases, blocked) =
            aggregate_streams(&client, &addons, "tt1234567", 0, 0, false).await;
        assert_eq!(blocked, vec!["P2P Addon".to_string()]);
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].quality.as_deref(), Some("2160p"));
        assert_eq!(releases[1].quality.as_deref(), Some("720p"));
    }

    #[test]
    fn swarm_tiers_rank_health_before_quality() {
        let name = "Movie.2024.1080p.WEB-DL.mkv";
        assert!(swarm_tier(Some(0), true, name) < swarm_tier(Some(2), true, name));
        assert!(swarm_tier(Some(2), true, name) < swarm_tier(None, true, name));
        assert!(swarm_tier(None, true, name) < swarm_tier(Some(5), true, name));
        assert!(swarm_tier(Some(29), true, name) < swarm_tier(Some(30), true, name));
        assert_eq!(
            swarm_tier(None, false, name),
            swarm_tier(Some(500), true, name)
        );
        // a CAM rip is worse than a dead torrent of a proper release
        assert!(
            swarm_tier(Some(900), true, "Movie.2024.HDCAM.mkv") < swarm_tier(Some(0), true, name)
        );
    }

    #[test]
    fn torrents_sort_by_seeds_then_quality() {
        fn release(filename: &str, quality: &str, size: u64) -> Release {
            Release {
                provider: crate::providers::models::ProviderKind::Addons,
                filename: filename.to_string(),
                quality: Some(quality.to_string()),
                codec: None,
                language: None,
                size_bytes: Some(size),
                season: None,
                episode: None,
                mirrors: Vec::new(),
                resource_id: None,
            }
        }
        let mut list = vec![
            (release("dead.2160p.mkv", "2160p", 9), Some(0), true),
            (release("weak.2160p.mkv", "2160p", 8), Some(3), true),
            (release("unknown.2160p.mkv", "2160p", 8), None, true),
            (release("ok.720p.mkv", "720p", 2), Some(12), true),
            (release("healthy.720p.mkv", "720p", 2), Some(400), true),
            (release("healthy.1080p.mkv", "1080p", 4), Some(60), true),
            (
                release("movie.1080p.hdcam.mkv", "1080p", 4),
                Some(900),
                true,
            ),
            (release("direct.1080p.mp4", "1080p", 3), None, false),
        ];
        list.sort_by(|(a, sa, ta), (b, sb, tb)| {
            swarm_tier(*sb, *tb, &b.filename)
                .cmp(&swarm_tier(*sa, *ta, &a.filename))
                .then_with(|| {
                    quality_score(b.quality.as_deref()).cmp(&quality_score(a.quality.as_deref()))
                })
                .then_with(|| sb.unwrap_or(u32::MAX).cmp(&sa.unwrap_or(u32::MAX)))
        });
        let order: Vec<&str> = list.iter().map(|(r, _, _)| r.filename.as_str()).collect();
        assert_eq!(
            order,
            [
                "direct.1080p.mp4",
                "healthy.1080p.mkv",
                "healthy.720p.mkv",
                "ok.720p.mkv",
                "unknown.2160p.mkv",
                "weak.2160p.mkv",
                "dead.2160p.mkv",
                "movie.1080p.hdcam.mkv",
            ]
        );
    }

    #[test]
    fn test_clean_addon_subject_id_preserves_namespaces() {
        assert_eq!(clean_addon_subject_id("series:kitsu:12345"), "kitsu:12345");
        assert_eq!(clean_addon_subject_id("movie:kitsu:999"), "kitsu:999");
        assert_eq!(clean_addon_subject_id("series:tt0903747"), "tt0903747");
        assert_eq!(clean_addon_subject_id("movie:tt1234567"), "tt1234567");
        assert_eq!(clean_addon_subject_id("tt1234567"), "tt1234567");
    }
}
