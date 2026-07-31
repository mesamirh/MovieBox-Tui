use super::client::FourKHdHubError;
use reqwest::Url;
use scraper::{Html, Selector};
use std::net::IpAddr;

pub async fn resolve(
    client: &reqwest::Client,
    drive_url: &str,
) -> Result<Vec<(String, String, Vec<(String, String)>)>, FourKHdHubError> {
    validate_resolver_url(drive_url)?;
    let drive_html = client
        .get(drive_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let sportverse_url = {
        let document = Html::parse_document(&drive_html);
        let download = Selector::parse("a#download")
            .map_err(|_| FourKHdHubError::Parse("invalid resolver selector".into()))?;
        document
            .select(&download)
            .filter_map(|node| node.value().attr("href"))
            .find(|href| href.starts_with("https://sportverse."))
            .map(str::to_string)
            .ok_or_else(|| FourKHdHubError::Parse("HubCloud resolver link missing".into()))?
    };

    let resolver_html = client
        .get(&sportverse_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let resolver = Html::parse_document(&resolver_html);
    let links = Selector::parse("a[href]")
        .map_err(|_| FourKHdHubError::Parse("invalid link selector".into()))?;

    let mut candidates = extract_script_pixeldrain_urls(&resolver_html)
        .into_iter()
        .map(|url| (0, url, "PixelDrain".to_string()))
        .collect::<Vec<_>>();
    candidates.extend(
        resolver
            .select(&links)
            .filter_map(|node| {
                let href = node.value().attr("href")?;
                let label = node.text().collect::<String>();
                validate_playback_url(href).ok().map(|url| {
                    let url = pixeldrain_api_url(&url).unwrap_or(url);
                    (score(&url, &label), url, clean_label(&label))
                })
            })
            .collect::<Vec<_>>(),
    );
    candidates.sort_by_key(|candidate| candidate.0);
    let mut resolved = candidates
        .into_iter()
        .map(|(_, url, label)| (url, label, Vec::new()))
        .collect::<Vec<_>>();
    resolved.dedup_by(|left, right| left.0 == right.0);
    if resolved.is_empty() {
        Err(FourKHdHubError::NoPlayableMirror)
    } else {
        Ok(resolved)
    }
}

pub async fn resolve_hubdrive(
    client: &reqwest::Client,
    drive_url: &str,
) -> Result<Vec<(String, String, Vec<(String, String)>)>, FourKHdHubError> {
    validate_hubdrive_url(drive_url)?;
    let html = client
        .get(drive_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let hubcloud_url = extract_hubcloud_drive_url(&html)
        .ok_or_else(|| FourKHdHubError::Parse("HubDrive HubCloud mirror missing".into()))?;
    resolve(client, &hubcloud_url).await
}

fn extract_hubcloud_drive_url(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let links = Selector::parse("a[href]").ok()?;
    document.select(&links).find_map(|node| {
        let raw = node.value().attr("href")?;
        let url = Url::parse(raw).ok()?;
        let host = url.host_str()?;
        (host.starts_with("hubcloud.") && url.path().starts_with("/drive/"))
            .then(|| url.to_string())
    })
}

fn extract_script_pixeldrain_urls(html: &str) -> Vec<String> {
    const PREFIX: &str = "https://pixeldrain.dev/u/";
    let mut urls = Vec::new();
    let mut remainder = html;
    while let Some(variable_offset) = remainder.find("var pxl") {
        let variable = &remainder[variable_offset..];
        let Some(url_offset) = variable.find(PREFIX) else {
            break;
        };
        let candidate = &variable[url_offset..];
        let end = candidate
            .find(|character: char| {
                character == '"' || character == '\'' || character.is_whitespace()
            })
            .unwrap_or(candidate.len());
        if let Some(url) = pixeldrain_api_url(&candidate[..end])
            && !urls.contains(&url)
        {
            urls.push(url);
        }
        remainder = &candidate[end..];
    }
    urls
}

fn pixeldrain_api_url(raw: &str) -> Option<String> {
    let url = Url::parse(raw).ok()?;
    if url.host_str() != Some("pixeldrain.dev") {
        return None;
    }
    let id = url.path().strip_prefix("/u/")?.trim_matches('/');
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return None;
    }
    Some(format!("https://pixeldrain.dev/api/file/{id}?download"))
}

fn validate_resolver_url(raw: &str) -> Result<(), FourKHdHubError> {
    let url = Url::parse(raw).map_err(|_| FourKHdHubError::InvalidUrl(raw.into()))?;
    let host = url.host_str().unwrap_or_default();
    if url.scheme() != "https"
        || !host.starts_with("hubcloud.")
        || !url.path().starts_with("/drive/")
    {
        return Err(FourKHdHubError::InvalidUrl(raw.into()));
    }
    Ok(())
}

fn validate_hubdrive_url(raw: &str) -> Result<(), FourKHdHubError> {
    let url = Url::parse(raw).map_err(|_| FourKHdHubError::InvalidUrl(raw.into()))?;
    let host = url.host_str().unwrap_or_default();
    if url.scheme() != "https"
        || !host.starts_with("hubdrive.")
        || !url.path().starts_with("/file/")
    {
        return Err(FourKHdHubError::InvalidUrl(raw.into()));
    }
    Ok(())
}

pub fn validate_playback_url(raw: &str) -> Result<String, FourKHdHubError> {
    let url = Url::parse(raw).map_err(|_| FourKHdHubError::InvalidUrl(raw.into()))?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(FourKHdHubError::InvalidUrl(raw.into()));
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".local")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| !is_public_ip(address))
        || path.ends_with(".zip")
        || path.contains("login.php")
        || path.contains("logout")
    {
        return Err(FourKHdHubError::InvalidUrl(raw.into()));
    }
    Ok(url.to_string())
}

fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !(address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_documentation()
                || address.is_unspecified())
        }
        IpAddr::V6(address) => {
            !(address.is_loopback()
                || address.is_unspecified()
                || address.is_unique_local()
                || address.is_unicast_link_local())
        }
    }
}

fn score(url: &str, label: &str) -> u8 {
    let value = format!("{} {}", url, label).to_ascii_lowercase();
    if value.contains("pixeldrain") || value.contains("pixel.hubcloud") {
        0
    } else if value.contains("gpdl.") || value.contains("googleusercontent") {
        1
    } else if value.contains("workers.dev") || value.contains("r2.dev") {
        2
    } else if value.contains("latent.click") || value.contains("fsl") {
        3
    } else {
        4
    }
}

fn clean_label(label: &str) -> String {
    let clean = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() {
        "Direct".into()
    } else {
        clean
    }
}
