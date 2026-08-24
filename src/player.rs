pub mod tracker;

use std::{path::Path, process::Command};

/// Preferred audio track languages, most preferred first. Falls back to the
/// container's default track if none of these are present.
const PREFERRED_AUDIO_LANGS: &str = "eng,en,english";

/// Preferred embedded subtitle track languages, most preferred first. Only
/// affects tracks muxed into the container; external subtitle files passed
/// via `subtitle` are shown regardless of this preference.
const PREFERRED_SUBTITLE_LANGS: &str = "eng,en,english";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Mpv,
    Iina,
    Vlc,
    AndroidIntent,
}

impl PlayerKind {
    pub fn label(&self) -> &'static str {
        match self {
            PlayerKind::Mpv => "mpv",
            PlayerKind::Iina => "IINA",
            PlayerKind::Vlc => "VLC",
            PlayerKind::AndroidIntent => "Android Player",
        }
    }

    pub fn config_key(&self) -> &'static str {
        match self {
            PlayerKind::Mpv => "mpv",
            PlayerKind::Iina => "iina",
            PlayerKind::Vlc => "vlc",
            PlayerKind::AndroidIntent => "android",
        }
    }

    pub fn parse(value: &str) -> Option<PlayerKind> {
        match value.to_ascii_lowercase().as_str() {
            "mpv" => Some(PlayerKind::Mpv),
            "iina" => Some(PlayerKind::Iina),
            "vlc" => Some(PlayerKind::Vlc),
            "android" | "androidintent" | "android-intent" => Some(PlayerKind::AndroidIntent),
            _ => None,
        }
    }
}

pub fn detect() -> Vec<PlayerKind> {
    let mut players = Vec::new();

    #[cfg(target_os = "macos")]
    if iina_available() {
        players.push(PlayerKind::Iina);
    }

    if mpv_executable().is_some() {
        players.push(PlayerKind::Mpv);
    }

    if vlc_executable().is_some() {
        players.push(PlayerKind::Vlc);
    }

    let is_android = cfg!(target_os = "android")
        || std::path::Path::new("/system/bin/am").exists()
        || executable_on_path("termux-open")
        || executable_on_path("am");

    if is_android {
        players.push(PlayerKind::AndroidIntent);
    }

    players
}

pub fn supports_headers(kind: PlayerKind, headers: &[(String, String)]) -> bool {
    if kind == PlayerKind::AndroidIntent {
        return false;
    }
    #[cfg(target_os = "macos")]
    if kind == PlayerKind::Iina && !iina_cli_exists() {
        return false;
    }
    kind != PlayerKind::Vlc
        || headers.iter().all(|(name, _)| {
            name.eq_ignore_ascii_case("referer") || name.eq_ignore_ascii_case("user-agent")
        })
}

pub fn command(
    kind: PlayerKind,
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
) -> Command {
    match kind {
        PlayerKind::Mpv => mpv_command(
            url,
            subtitle,
            headers,
            false,
            window,
            resume_seconds,
            tracker,
        ),
        PlayerKind::Iina => iina_command(url, subtitle, headers, window, resume_seconds, tracker),
        PlayerKind::Vlc => vlc_command(url, subtitle, headers, window, resume_seconds),
        PlayerKind::AndroidIntent => android_intent_command(url),
    }
}

fn build_player_process_command(executable: &str) -> Command {
    if executable.starts_with("flatpak run ") {
        let parts = executable.split_whitespace().collect::<Vec<_>>();
        let mut cmd = Command::new(parts.first().unwrap_or(&"flatpak"));
        if parts.len() > 1 && parts[1] == "run" {
            cmd.arg("run");
            cmd.arg("--file-forwarding");
            cmd.args(&parts[2..]);
        } else {
            cmd.args(&parts[1..]);
        }
        cmd
    } else {
        Command::new(executable)
    }
}

fn android_intent_command(url: &str) -> Command {
    let mut command;
    if executable_on_path("termux-open") {
        command = Command::new("termux-open");
        command
            .arg("--chooser")
            .arg("--content-type")
            .arg("video/*")
            .arg(url);
    } else {
        let am_path = if std::path::Path::new("/system/bin/am").exists() {
            "/system/bin/am"
        } else {
            "am"
        };
        command = Command::new(am_path);
        command
            .arg("start")
            .arg("--user")
            .arg("0")
            .arg("-a")
            .arg("android.intent.action.VIEW")
            .arg("-d")
            .arg(url)
            .arg("-t")
            .arg("video/*");
    }

    command.env_remove("LD_LIBRARY_PATH");
    command.env_remove("LD_PRELOAD");

    command
}

fn mpv_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    iina: bool,
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
) -> Command {
    let fallback = if cfg!(target_os = "windows") {
        "mpv.exe"
    } else {
        "mpv"
    };
    let executable = mpv_executable().unwrap_or_else(|| fallback.into());
    let mut command = build_player_process_command(&executable);
    let prefix = if iina { "--mpv-" } else { "--" };

    if let Some((width, height)) = window {
        command.arg(format!("{prefix}autofit={width}x{height}"));
    }
    command.arg(format!("{prefix}geometry=50%:50%"));

    if !iina {
        command.arg("--idle=no").arg("--keep-open=no");
    }

    if let Some(start) = resume_seconds {
        if start > 0 {
            command.arg(format!("{prefix}start={start}"));
        }
    }

    if let Some((provider, subject_id, season, episode)) = tracker {
        if let Some(script_path) = tracker::ensure_tracker_script() {
            let script_str = script_path.to_string_lossy().replace('\\', "/");
            command.arg(format!("{prefix}script={script_str}"));
            if let Some(state_file) =
                tracker::state_file_path(provider, subject_id, season, episode)
            {
                let opts =
                    format_mpv_script_opts(provider, subject_id, season, episode, &state_file);
                command.arg(format!("{prefix}script-opts={opts}"));
            }
        }
    }

    if !headers.is_empty() {
        let fields = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join(",");
        command.arg(format!("{prefix}http-header-fields={fields}"));
    }
    if let Some(subtitle) = subtitle {
        let opt = if iina {
            "--mpv-sub-files"
        } else {
            "--sub-file"
        };
        command.arg(format!("{opt}={subtitle}"));
    }

    command.arg(format!("{prefix}alang={PREFERRED_AUDIO_LANGS}"));
    command.arg(format!("{prefix}slang={PREFERRED_SUBTITLE_LANGS}"));

    command.arg(url);

    command
}

#[cfg(target_os = "macos")]
fn iina_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
) -> Command {
    let configured = configured_executable("MOVIEBOX_IINA_PATH");
    let cli_global = std::path::Path::new("/Applications/IINA.app/Contents/MacOS/iina-cli");
    let cli_local = dirs::home_dir()
        .map(|h| h.join("Applications/IINA.app/Contents/MacOS/iina-cli"))
        .unwrap_or_default();

    let mut command = if let Some(executable) = configured {
        Command::new(executable)
    } else if cli_global.exists() {
        let mut c = Command::new(cli_global);
        c.arg("--keep-running").arg("--no-stdin");
        c
    } else if cli_local.exists() {
        let mut c = Command::new(cli_local);
        c.arg("--keep-running").arg("--no-stdin");
        c
    } else if iina_app_exists() {
        let mut c = Command::new("open");
        c.arg("-a").arg("IINA").arg(url);
        return c;
    } else {
        Command::new("iina")
    };

    let mpv = mpv_command(
        url,
        subtitle,
        headers,
        true,
        window,
        resume_seconds,
        tracker,
    );
    for arg in mpv.get_args() {
        command.arg(arg);
    }
    command
}

#[cfg(not(target_os = "macos"))]
fn iina_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
) -> Command {
    mpv_command(
        url,
        subtitle,
        headers,
        false,
        window,
        resume_seconds,
        tracker,
    )
}

fn vlc_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
) -> Command {
    let fallback = if cfg!(target_os = "windows") {
        "vlc.exe"
    } else {
        "vlc"
    };
    let executable = vlc_executable().unwrap_or_else(|| fallback.into());
    let mut command = build_player_process_command(&executable);

    if let Some((width, height)) = window {
        command
            .arg(format!("--width={width}"))
            .arg(format!("--height={height}"));
    }
    command.arg("--play-and-exit");

    if let Some(start) = resume_seconds {
        if start > 0 {
            command.arg(format!("--start-time={start}"));
        }
    }

    for (name, value) in headers {
        if name.eq_ignore_ascii_case("referer") {
            command.arg(format!("--http-referrer={value}"));
        } else if name.eq_ignore_ascii_case("user-agent") {
            command.arg(format!("--http-user-agent={value}"));
        }
    }
    if let Some(subtitle) = subtitle {
        command.arg(format!("--sub-file={subtitle}"));
    }

    command.arg(format!("--audio-language={PREFERRED_AUDIO_LANGS}"));
    command.arg(format!("--sub-language={PREFERRED_SUBTITLE_LANGS}"));

    command.arg(url);
    command
}

fn mpv_executable() -> Option<String> {
    static CACHED: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            if let Some(executable) = configured_executable("MOVIEBOX_MPV_PATH") {
                return Some(executable);
            }

            let mut candidates = Vec::new();

            #[cfg(target_os = "windows")]
            {
                candidates.push(r"C:\Program Files\mpv\mpv.exe".to_string());
                candidates.push(r"C:\Program Files (x86)\mpv\mpv.exe".to_string());
                if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    candidates.push(format!(r"{local}\Programs\mpv\mpv.exe"));
                }
                if let Ok(appdata) = std::env::var("APPDATA") {
                    candidates.push(format!(r"{appdata}\mpv\mpv.exe"));
                }
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join(r"scoop\shims\mpv.exe")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push(r"C:\ProgramData\chocolatey\bin\mpv.exe".to_string());
            }

            #[cfg(target_os = "macos")]
            {
                candidates.push("/Applications/mpv.app/Contents/MacOS/mpv".to_string());
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join("Applications/mpv.app/Contents/MacOS/mpv")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push("/opt/homebrew/bin/mpv".to_string());
                candidates.push("/usr/local/bin/mpv".to_string());
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join(".local/share/flatpak/exports/bin/io.mpv.Mpv")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push("/var/lib/flatpak/exports/bin/io.mpv.Mpv".to_string());
                candidates.push("/snap/bin/mpv".to_string());
                candidates.push("/var/lib/snapd/snap/bin/mpv".to_string());
                candidates.push("/usr/bin/mpv".to_string());
                candidates.push("/usr/local/bin/mpv".to_string());
                candidates.push("/app/bin/mpv".to_string());
            }

            for path in candidates {
                if Path::new(&path).is_file() {
                    return Some(path);
                }
            }

            let bin_names = if cfg!(target_os = "windows") {
                &["mpv.exe", "mpv"][..]
            } else {
                &["mpv", "io.mpv.Mpv"][..]
            };

            for bin in bin_names {
                if let Some(path) = find_in_path(bin) {
                    return Some(path);
                }
            }

            flatpak_executable("io.mpv.Mpv")
        })
        .clone()
}

fn vlc_executable() -> Option<String> {
    static CACHED: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            if let Some(executable) = configured_executable("MOVIEBOX_VLC_PATH") {
                return Some(executable);
            }

            let mut candidates = Vec::new();

            #[cfg(target_os = "windows")]
            {
                candidates.push(r"C:\Program Files\VideoLAN\VLC\vlc.exe".to_string());
                candidates.push(r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe".to_string());
                if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    candidates.push(format!(r"{local}\Microsoft\WindowsApps\vlc.exe"));
                    candidates.push(format!(r"{local}\Programs\VLC\vlc.exe"));
                }
                if let Ok(appdata) = std::env::var("APPDATA") {
                    candidates.push(format!(r"{appdata}\vlc\vlc.exe"));
                }
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join(r"scoop\shims\vlc.exe")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push(r"C:\ProgramData\chocolatey\bin\vlc.exe".to_string());
            }

            #[cfg(target_os = "macos")]
            {
                candidates.push("/Applications/VLC.app/Contents/MacOS/VLC".to_string());
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join("Applications/VLC.app/Contents/MacOS/VLC")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push("/opt/homebrew/bin/vlc".to_string());
                candidates.push("/usr/local/bin/vlc".to_string());
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(home) = dirs::home_dir() {
                    candidates.push(
                        home.join(".local/share/flatpak/exports/bin/org.videolan.VLC")
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                candidates.push("/var/lib/flatpak/exports/bin/org.videolan.VLC".to_string());
                candidates.push("/snap/bin/vlc".to_string());
                candidates.push("/var/lib/snapd/snap/bin/vlc".to_string());
                candidates.push("/usr/bin/vlc".to_string());
                candidates.push("/usr/local/bin/vlc".to_string());
                candidates.push("/app/bin/vlc".to_string());
            }

            for path in candidates {
                if Path::new(&path).is_file() {
                    return Some(path);
                }
            }

            let bin_names = if cfg!(target_os = "windows") {
                &["vlc.exe", "vlc"][..]
            } else {
                &["vlc", "org.videolan.VLC"][..]
            };

            for bin in bin_names {
                if let Some(path) = find_in_path(bin) {
                    return Some(path);
                }
            }

            flatpak_executable("org.videolan.VLC")
        })
        .clone()
}

#[cfg(target_os = "macos")]
fn iina_available() -> bool {
    configured_executable("MOVIEBOX_IINA_PATH").is_some()
        || iina_app_exists()
        || executable_on_path("iina")
        || executable_on_path("iina-cli")
}

#[cfg(target_os = "macos")]
fn iina_app_exists() -> bool {
    Path::new("/Applications/IINA.app").exists()
        || dirs::home_dir().is_some_and(|home| home.join("Applications/IINA.app").exists())
}

#[cfg(target_os = "macos")]
fn iina_cli_exists() -> bool {
    let cli_global = Path::new("/Applications/IINA.app/Contents/MacOS/iina-cli");
    let cli_local = dirs::home_dir()
        .map(|h| h.join("Applications/IINA.app/Contents/MacOS/iina-cli"))
        .unwrap_or_default();
    configured_executable("MOVIEBOX_IINA_PATH").is_some()
        || cli_global.exists()
        || cli_local.exists()
        || executable_on_path("iina")
        || executable_on_path("iina-cli")
}

fn flatpak_executable(app_id: &str) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    if executable_on_path("flatpak") {
        let mut cmd = Command::new("flatpak");
        cmd.arg("info")
            .arg(app_id)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if cmd.output().map(|o| o.status.success()).unwrap_or(false) {
            return Some(format!("flatpak run {}", app_id));
        }

        let mut user_cmd = Command::new("flatpak");
        user_cmd
            .arg("info")
            .arg("--user")
            .arg(app_id)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if user_cmd
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(format!("flatpak run {}", app_id));
        }
    }
    None
}

fn configured_executable(variable: &str) -> Option<String> {
    let val = std::env::var(variable).ok()?;
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("flatpak run ")
        || Path::new(trimmed).exists()
        || executable_on_path(trimmed)
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn find_in_path(name: &str) -> Option<String> {
    if std::path::Path::new(name).is_file() {
        return Some(name.to_string());
    }
    #[cfg(windows)]
    if std::path::Path::new(&format!("{name}.exe")).is_file() {
        return Some(format!("{name}.exe"));
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        #[cfg(windows)]
        {
            let candidates = [
                candidate.clone(),
                candidate.with_extension("exe"),
                candidate.with_extension("cmd"),
            ];
            for c in candidates {
                if c.is_file() {
                    return Some(c.to_string_lossy().into_owned());
                }
            }
        }
        #[cfg(not(windows))]
        {
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if candidate
                        .metadata()
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
                    {
                        return Some(candidate.to_string_lossy().into_owned());
                    }
                }
                #[cfg(not(unix))]
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn executable_on_path(name: &str) -> bool {
    find_in_path(name).is_some()
}

pub fn format_mpv_script_opts(
    provider: &str,
    subject_id: &str,
    season: usize,
    episode: usize,
    state_file: &Path,
) -> String {
    let state_file_str = state_file.to_string_lossy().replace('\\', "/");
    format!(
        "moviebox-provider={provider},moviebox-subject_id={subject_id},moviebox-season={season},moviebox-episode={episode},moviebox-state_file={state_file_str}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_mpv_script_opts_windows_paths() {
        let win_path = PathBuf::from(
            r"C:\Users\User\AppData\Local\MovieBox-Tui\playback\moviebox_123_1_1.json",
        );
        let opts = format_mpv_script_opts("moviebox", "123", 1, 1, &win_path);
        assert!(!opts.contains(r"\"));
        assert!(opts.contains("moviebox-state_file=C:/Users/User/AppData/Local/MovieBox-Tui/playback/moviebox_123_1_1.json"));
    }

    #[test]
    fn test_format_mpv_script_opts_unix_paths() {
        let unix_path =
            PathBuf::from("/home/user/.local/share/moviebox-tui/playback/moviebox_123_1_1.json");
        let opts = format_mpv_script_opts("moviebox", "123", 1, 1, &unix_path);
        assert!(opts.contains("moviebox-state_file=/home/user/.local/share/moviebox-tui/playback/moviebox_123_1_1.json"));
    }
}
