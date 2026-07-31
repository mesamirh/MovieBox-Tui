use crate::tui::state::PlayerKind;
use std::{path::Path, process::Command};

const MPV_WINDOWS: &str = r"C:\Program Files\mpv\mpv.exe";
const MPV_MACOS: &str = "/Applications/mpv.app/Contents/MacOS/mpv";
const VLC_WINDOWS: &str = r"C:\Program Files\VideoLAN\VLC\vlc.exe";
const VLC_WINDOWS_X86: &str = r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe";
const VLC_MACOS: &str = "/Applications/VLC.app/Contents/MacOS/VLC";

pub fn detect() -> Vec<PlayerKind> {
    let mut players = Vec::new();

    // Prefer mpv first — it handles signed/headered stream URLs most reliably.
    if mpv_executable().is_some() {
        players.push(PlayerKind::Mpv);
    }

    #[cfg(target_os = "macos")]
    if Path::new("/Applications/IINA.app").exists() || command_exists("iina") {
        players.push(PlayerKind::Iina);
    }

    if vlc_executable().is_some() {
        players.push(PlayerKind::Vlc);
    }

    players
}

/// Index to pre-select in the player picker (mpv when available).
pub fn preferred_index(players: &[PlayerKind]) -> usize {
    players
        .iter()
        .position(|p| *p == PlayerKind::Mpv)
        .unwrap_or(0)
}

pub fn supports_headers(kind: PlayerKind, headers: &[(String, String)]) -> bool {
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
) -> Command {
    match kind {
        PlayerKind::Mpv => mpv_command(url, subtitle, headers, false),
        PlayerKind::Iina => iina_command(url, subtitle, headers),
        PlayerKind::Vlc => vlc_command(url, subtitle, headers),
    }
}

fn mpv_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    iina: bool,
) -> Command {
    let executable = mpv_executable().unwrap_or_else(|| "mpv".into());
    let mut command = Command::new(executable);
    let prefix = if iina { "--mpv-" } else { "--" };

    command
        .arg(format!("{prefix}autofit=960x540"))
        .arg(format!("{prefix}autofit-larger=640x360"))
        .arg(format!("{prefix}geometry=50%:50%"))
        .arg(url);

    if !headers.is_empty() {
        let fields = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join(",");
        command.arg(format!("{prefix}http-header-fields={fields}"));
    }
    if let Some(subtitle) = subtitle {
        if iina {
            command.arg(format!("--mpv-sub-files={subtitle}"));
        } else {
            command.arg(format!("--sub-file={subtitle}"));
        }
    }

    command
}

#[cfg(target_os = "macos")]
fn iina_command(url: &str, subtitle: Option<&str>, headers: &[(String, String)]) -> Command {
    let mut command = Command::new("open");
    command.arg("-a").arg("IINA").arg("--args");
    let mpv = mpv_command(url, subtitle, headers, true);
    command.args(mpv.get_args());
    command
}

#[cfg(not(target_os = "macos"))]
fn iina_command(url: &str, subtitle: Option<&str>, headers: &[(String, String)]) -> Command {
    mpv_command(url, subtitle, headers, false)
}

fn vlc_command(url: &str, subtitle: Option<&str>, headers: &[(String, String)]) -> Command {
    let executable = vlc_executable().unwrap_or_else(|| "vlc".into());
    let mut command = Command::new(executable);
    command.arg("--width=960").arg("--height=540").arg(url);

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

    command
}

fn mpv_executable() -> Option<String> {
    first_executable(&[MPV_WINDOWS, MPV_MACOS], "mpv")
}

fn vlc_executable() -> Option<String> {
    first_executable(&[VLC_WINDOWS, VLC_WINDOWS_X86, VLC_MACOS], "vlc")
}

fn first_executable(paths: &[&str], fallback: &str) -> Option<String> {
    paths
        .iter()
        .find(|path| Path::new(path).exists())
        .map(|path| (*path).to_string())
        .or_else(|| command_exists(fallback).then(|| fallback.to_string()))
}

fn command_exists(command: &str) -> bool {
    let finder = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    Command::new(finder)
        .arg(command)
        .output()
        .is_ok_and(|output| output.status.success())
}
