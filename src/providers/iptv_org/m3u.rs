use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub logo: String,
    pub group: String,
    pub stream_url: String,
}

pub struct M3UParser {
    cache_dir: PathBuf,
}

impl Default for M3UParser {
    fn default() -> Self {
        Self::new()
    }
}

impl M3UParser {
    pub fn new() -> Self {
        let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache_dir.push("moviebox-tui");
        cache_dir.push("tv_playlists");
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    pub async fn fetch_playlist(
        &self,
        url: &str,
        filename: &str,
    ) -> Result<Vec<Channel>, Box<dyn std::error::Error>> {
        let file_path = self.cache_dir.join(filename);
        let mut needs_download = true;

        if file_path.exists() {
            if let Ok(metadata) = fs::metadata(&file_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = SystemTime::now().duration_since(modified) {
                        if duration.as_secs() < 24 * 3600 {
                            needs_download = false;
                        }
                    }
                }
            }
        }

        let content = if needs_download {
            let client = reqwest::Client::new();
            let res = client.get(url).send().await?.text().await?;
            fs::write(&file_path, &res).ok();
            res
        } else {
            fs::read_to_string(&file_path)?
        };

        Ok(self.parse_m3u(&content))
    }

    fn parse_m3u(&self, content: &str) -> Vec<Channel> {
        let mut channels = Vec::new();
        let mut current_channel = Channel {
            id: String::new(),
            name: String::new(),
            logo: String::new(),
            group: String::new(),
            stream_url: String::new(),
        };

        let extract_attr = |line: &str, attr: &str| -> String {
            if let Some(idx) = line.find(attr) {
                let start = idx + attr.len();
                if let Some(end) = line[start..].find('"') {
                    return line[start..start + end].to_string();
                }
            }
            String::new()
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("#EXTINF:") {
                current_channel.id = extract_attr(line, "tvg-id=\"");
                current_channel.logo = extract_attr(line, "tvg-logo=\"");
                current_channel.group = extract_attr(line, "group-title=\"");

                if let Some(idx) = line.rfind(',') {
                    current_channel.name = line[idx + 1..].trim().to_string();
                }
            } else if !line.starts_with('#') {
                current_channel.stream_url = line.to_string();
                if current_channel.id.is_empty() {
                    current_channel.id = current_channel.name.clone();
                }
                channels.push(current_channel.clone());

                current_channel = Channel {
                    id: String::new(),
                    name: String::new(),
                    logo: String::new(),
                    group: String::new(),
                    stream_url: String::new(),
                };
            }
        }

        channels
    }
}
