use super::adapter::stream_item_to_release;
use super::client::AddonClient;
use super::models::InstalledAddon;
use crate::providers::models::Release;

pub fn clean_addon_subject_id(subject_id: &str) -> &str {
    subject_id
        .strip_prefix("series:")
        .or_else(|| subject_id.strip_prefix("movie:"))
        .unwrap_or(subject_id)
}

pub async fn aggregate_streams(
    client: &AddonClient,
    addons: &[InstalledAddon],
    subject_id: &str,
    season: usize,
    episode: usize,
    is_series: bool,
) -> (Vec<Release>, Vec<String>) {
    let stream_addons: Vec<&InstalledAddon> = addons
        .iter()
        .filter(|a| a.enabled && a.provides_stream)
        .collect();

    if stream_addons.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let clean_id = clean_addon_subject_id(subject_id);
    let media_type = if is_series { "series" } else { "movie" };
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

        tasks.push(async move {
            let streams_res = tokio::time::timeout(
                std::time::Duration::from_millis(5000),
                client_clone.fetch_streams(&base_url, &m_type, &id_clone),
            )
            .await;
            match streams_res {
                Ok(Ok(items)) => {
                    let mut releases = Vec::new();
                    for item in &items {
                        if let Some(rel) =
                            stream_item_to_release(&addon_name, item, season, episode)
                        {
                            releases.push(rel);
                        }
                    }
                    if !items.is_empty() && releases.is_empty() {
                        (releases, Some(addon_name))
                    } else {
                        (releases, None)
                    }
                }
                Ok(Err(err)) => {
                    log::warn!("addon {addon_name} failed to fetch streams: {err}");
                    (Vec::new(), None)
                }
                Err(_) => {
                    log::warn!("addon {addon_name} timed out fetching streams");
                    (Vec::new(), None)
                }
            }
        });
    }

    let results = futures::future::join_all(tasks).await;
    let mut all_releases = Vec::new();
    let mut blocked_addons = Vec::new();

    for res in results {
        all_releases.extend(res.0);
        if let Some(blocked) = res.1 {
            blocked_addons.push(blocked);
        }
    }

    let mut deduplicated: Vec<Release> = Vec::new();
    for rel in all_releases {
        let direct = rel.direct_url().unwrap_or("").to_string();
        if direct.is_empty() {
            deduplicated.push(rel);
            continue;
        }
        if let Some(existing) = deduplicated
            .iter_mut()
            .find(|r| r.direct_url() == Some(&direct))
        {
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
            deduplicated.push(rel);
        }
    }

    deduplicated.sort_by(|a, b| {
        let q_a = quality_score(a.quality.as_deref());
        let q_b = quality_score(b.quality.as_deref());
        match q_b.cmp(&q_a) {
            std::cmp::Ordering::Equal => {
                let size_a = a.size_bytes.unwrap_or(0);
                let size_b = b.size_bytes.unwrap_or(0);
                match size_b.cmp(&size_a) {
                    std::cmp::Ordering::Equal => {
                        let label_a = a.mirrors.first().map(|m| m.label.as_str()).unwrap_or("");
                        let label_b = b.mirrors.first().map(|m| m.label.as_str()).unwrap_or("");
                        label_a.cmp(label_b)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    (deduplicated, blocked_addons)
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
    async fn test_aggregate_streams_merges_deduplicates_sorts_and_reports_blocked_torrents() {
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
                    {"name":"TorrentOnly","title":"P2P Swarm","infoHash":"0123456789abcdef0123456789abcdef01234567"}
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
    fn test_clean_addon_subject_id_preserves_namespaces() {
        assert_eq!(clean_addon_subject_id("series:kitsu:12345"), "kitsu:12345");
        assert_eq!(clean_addon_subject_id("movie:kitsu:999"), "kitsu:999");
        assert_eq!(clean_addon_subject_id("series:tt0903747"), "tt0903747");
        assert_eq!(clean_addon_subject_id("movie:tt1234567"), "tt1234567");
        assert_eq!(clean_addon_subject_id("tt1234567"), "tt1234567");
    }
}
