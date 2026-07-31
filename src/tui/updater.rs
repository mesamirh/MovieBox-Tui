const OWNER: &str = "mesamirh";
const REPOSITORY: &str = "MovieBox-Tui";

pub fn check(current: &str) -> Result<Option<String>, String> {
    let releases = fetch_releases()?;
    let release = releases
        .first()
        .ok_or_else(|| "No published releases".to_string())?;

    if !is_newer(current, &release.version) {
        return Ok(None);
    }

    Ok(Some(release.version.clone()))
}

fn is_newer(current: &str, other: &str) -> bool {
    let parse = |v: &str| semver::Version::parse(v.trim_start_matches('v'));
    match (parse(current), parse(other)) {
        (Ok(cur), Ok(o)) => o > cur,
        _ => other != current,
    }
}

fn fetch_releases() -> Result<Vec<Release>, String> {
    let url = format!("https://api.github.com/repos/{OWNER}/{REPOSITORY}/releases?per_page=20");
    let client = http_client()?;

    let mut request = client.get(&url);
    if let Some(token) = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()) {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    let resp = request
        .send()
        .map_err(|e| format!("GitHub request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(format!("GitHub API {status}: {body}"));
    }

    let items: Vec<serde_json::Value> = resp.json().map_err(|e| format!("bad JSON: {e}"))?;

    items
        .into_iter()
        .map(|r| {
            let tag = r["tag_name"]
                .as_str()
                .ok_or("missing tag_name")?
                .to_string();
            Ok(Release {
                version: tag.trim_start_matches('v').to_string(),
            })
        })
        .collect()
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("MovieBox-Tui")
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))
}

struct Release {
    version: String,
}
