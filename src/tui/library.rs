use std::path::{Path, PathBuf};
use std::time::SystemTime;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "avi", "mov", "m4v", "ts", "flv"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    Movie,
    Series,
}

#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub title: String,
    pub kind: LibraryKind,
    pub path: PathBuf,
    pub subtitle: Option<PathBuf>,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
    /// Relative location under the download root, e.g. "Movies" or "Series/Show/Season 1".
    pub location: String,
}

impl LibraryItem {
    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            LibraryKind::Movie => "Movie",
            LibraryKind::Series => "Series",
        }
    }

    pub fn size_label(&self) -> String {
        format_bytes(self.size_bytes)
    }

    pub fn play_path(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    pub fn subtitle_path(&self) -> Option<String> {
        self.subtitle
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
    }
}

pub fn download_root() -> PathBuf {
    dirs::download_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MovieBox-TUI")
}

pub fn scan_library() -> Vec<LibraryItem> {
    let root = download_root();
    let mut items = Vec::new();

    collect_videos(&root.join("Movies"), LibraryKind::Movie, "Movies", &mut items);
    collect_series(&root.join("Series"), &mut items);

    // Also pick up anything sitting directly under the root (older layouts).
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_video(&path) {
                if let Some(item) = item_from_path(path, LibraryKind::Movie, "Downloads") {
                    items.push(item);
                }
            }
        }
    }

    items.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    items
}

fn collect_videos(dir: &Path, kind: LibraryKind, location: &str, out: &mut Vec<LibraryItem>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_video(&path) {
            if let Some(item) = item_from_path(path, kind, location) {
                out.push(item);
            }
        }
    }
}

fn collect_series(series_root: &Path, out: &mut Vec<LibraryItem>) {
    let Ok(shows) = std::fs::read_dir(series_root) else {
        return;
    };
    for show_entry in shows.flatten() {
        let show_path = show_entry.path();
        if !show_path.is_dir() {
            if is_video(&show_path) {
                if let Some(item) =
                    item_from_path(show_path, LibraryKind::Series, "Series")
                {
                    out.push(item);
                }
            }
            continue;
        }
        let show_name = show_entry.file_name().to_string_lossy().into_owned();
        // Direct episodes under show folder
        collect_videos(
            &show_path,
            LibraryKind::Series,
            &format!("Series/{show_name}"),
            out,
        );
        // Season folders
        if let Ok(seasons) = std::fs::read_dir(&show_path) {
            for season_entry in seasons.flatten() {
                let season_path = season_entry.path();
                if !season_path.is_dir() {
                    continue;
                }
                let season_name = season_entry.file_name().to_string_lossy().into_owned();
                collect_videos(
                    &season_path,
                    LibraryKind::Series,
                    &format!("Series/{show_name}/{season_name}"),
                    out,
                );
            }
        }
    }
}

fn item_from_path(path: PathBuf, kind: LibraryKind, location: &str) -> Option<LibraryItem> {
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() == 0 {
        return None;
    }
    let title = path
        .file_stem()
        .map(|s| s.to_string_lossy().replace('_', " "))
        .unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Unknown".into())
        });

    let subtitle = sibling_subtitle(&path);
    Some(LibraryItem {
        title,
        kind,
        path,
        subtitle,
        size_bytes: meta.len(),
        modified: meta.modified().ok(),
        location: location.to_string(),
    })
}

fn sibling_subtitle(video: &Path) -> Option<PathBuf> {
    let stem = video.file_stem()?.to_string_lossy();
    let parent = video.parent()?;
    for ext in ["srt", "vtt", "ass", "ssa"] {
        let candidate = parent.join(format!("{stem}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = bytes as f64;
    if n >= GB {
        format!("{:.1} GB", n / GB)
    } else if n >= MB {
        format!("{:.0} MB", n / MB)
    } else if n >= KB {
        format!("{:.0} KB", n / KB)
    } else {
        format!("{bytes} B")
    }
}
