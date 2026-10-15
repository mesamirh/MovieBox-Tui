pub mod tracker;

use std::{path::Path, process::Command};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const ENV_MOVIEBOX_PLAYER: &str = "MOVIEBOX_PLAYER";

#[cfg(not(target_os = "windows"))]
pub const STANDARD_UNIX_BIN_DIRS: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/run/current-system/sw/bin",
    "/data/data/com.termux/files/usr/bin",
];

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
            PlayerKind::Mpv => "MPV",
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

pub fn has_graphical_display() -> bool {
    std::env::var("DISPLAY").is_ok_and(|v| !v.trim().is_empty())
        || std::env::var("WAYLAND_DISPLAY").is_ok_and(|v| !v.trim().is_empty())
}

pub fn detect() -> Vec<PlayerKind> {
    let mut players = Vec::new();

    let is_termux = crate::config::is_termux_environment();
    let has_gui = has_graphical_display();
    if is_termux && !has_gui && !android_openers().is_empty() {
        players.push(PlayerKind::AndroidIntent);
    }

    #[cfg(target_os = "macos")]
    if iina_resolution().is_some() {
        players.push(PlayerKind::Iina);
    }

    if mpv_executable().is_some() {
        players.push(PlayerKind::Mpv);
    }

    if vlc_executable().is_some() {
        players.push(PlayerKind::Vlc);
    }

    if (!is_termux || has_gui) && !android_openers().is_empty() {
        players.push(PlayerKind::AndroidIntent);
    }

    players
}

pub fn supports_headers(kind: PlayerKind, headers: &[(String, String)]) -> bool {
    if headers.is_empty() {
        return true;
    }
    #[cfg(target_os = "macos")]
    if kind == PlayerKind::Iina && matches!(iina_resolution(), Some(IinaResolution::AppFallback)) {
        return false;
    }
    match kind {
        PlayerKind::Mpv => true,
        PlayerKind::Iina => true,
        PlayerKind::Vlc => true,
        PlayerKind::AndroidIntent => true,
    }
}

pub fn header_capable_players() -> &'static [PlayerKind] {
    #[cfg(target_os = "macos")]
    {
        &[
            PlayerKind::Mpv,
            PlayerKind::Iina,
            PlayerKind::Vlc,
            PlayerKind::AndroidIntent,
        ]
    }
    #[cfg(not(target_os = "macos"))]
    {
        &[PlayerKind::Mpv, PlayerKind::Vlc, PlayerKind::AndroidIntent]
    }
}

pub fn is_dash_url(url: &str) -> bool {
    let clean = url.split('?').next().unwrap_or(url);
    clean.ends_with(".mpd") || clean.contains("/dash/")
}

pub fn ytdlp_format_selector(max_height: Option<u64>) -> String {
    if let Some(height) = max_height.filter(|&h| h > 0) {
        format!(
            "bestvideo[height<={height}]+bestaudio/best[height<={height}]/bestvideo+bestaudio/best"
        )
    } else {
        "bestvideo+bestaudio/best".to_string()
    }
}

pub fn configure_detached_process(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                let _ = libc::setsid();
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn command(
    kind: PlayerKind,
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
    max_height: Option<u64>,
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
            max_height,
        ),
        PlayerKind::Iina => iina_command(
            url,
            subtitle,
            headers,
            window,
            resume_seconds,
            tracker,
            max_height,
        ),
        PlayerKind::Vlc => vlc_command(url, subtitle, headers, window, resume_seconds, max_height),
        PlayerKind::AndroidIntent => android_intent_command(url, subtitle, headers),
    }
}

fn is_flatpak_executable(executable: &str) -> bool {
    executable.starts_with("flatpak run ")
        || executable.contains("/flatpak/exports/bin/")
        || matches!(executable, "io.mpv.Mpv" | "org.videolan.VLC")
}

fn build_player_process_command(executable: &str) -> Command {
    if let Some(rest) = executable.strip_prefix("flatpak run ") {
        let mut cmd = Command::new("flatpak");
        cmd.arg("run")
            .arg("--file-forwarding")
            .arg("--filesystem=xdg-cache/moviebox-tui:ro")
            .arg("--filesystem=xdg-data/moviebox-tui")
            .arg("--filesystem=/tmp:ro")
            .args(rest.split_whitespace());
        cmd
    } else if executable.contains("/flatpak/exports/bin/") {
        let app_id = executable.rsplit('/').next().unwrap_or(executable);
        let mut cmd = Command::new("flatpak");
        cmd.arg("run")
            .arg("--file-forwarding")
            .arg("--filesystem=xdg-cache/moviebox-tui:ro")
            .arg("--filesystem=xdg-data/moviebox-tui")
            .arg("--filesystem=/tmp:ro")
            .arg(app_id);
        cmd
    } else if matches!(executable, "io.mpv.Mpv" | "org.videolan.VLC") {
        let mut cmd = Command::new("flatpak");
        cmd.arg("run")
            .arg("--file-forwarding")
            .arg("--filesystem=xdg-cache/moviebox-tui:ro")
            .arg("--filesystem=xdg-data/moviebox-tui")
            .arg("--filesystem=/tmp:ro")
            .arg(executable);
        cmd
    } else {
        Command::new(executable)
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AndroidOpener {
    TermuxAm(String),
    TermuxOpen(String),
    TermuxOpenUrl(String),
    #[cfg(target_os = "android")]
    SystemAm(String),
}

pub fn probe_android_openers() -> Vec<AndroidOpener> {
    let mut openers = Vec::new();

    if let Some(custom) = configured_executable("MOVIEBOX_ANDROID_PLAYER_PATH") {
        if custom.ends_with("termux-open-url") {
            openers.push(AndroidOpener::TermuxOpenUrl(custom));
        } else if custom.ends_with("termux-am") {
            openers.push(AndroidOpener::TermuxAm(custom));
        } else {
            openers.push(AndroidOpener::TermuxOpen(custom));
        }
        return openers;
    }

    let mut push_unique = |opener: AndroidOpener| {
        if !openers.iter().any(|existing| match (existing, &opener) {
            (AndroidOpener::TermuxAm(a), AndroidOpener::TermuxAm(b))
            | (AndroidOpener::TermuxOpen(a), AndroidOpener::TermuxOpen(b))
            | (AndroidOpener::TermuxOpenUrl(a), AndroidOpener::TermuxOpenUrl(b)) => a == b,
            #[cfg(target_os = "android")]
            (AndroidOpener::SystemAm(a), AndroidOpener::SystemAm(b)) => a == b,
            _ => false,
        }) {
            openers.push(opener);
        }
    };

    #[cfg(target_os = "android")]
    let is_termux = crate::config::is_termux_environment();

    if let Ok(prefix) = std::env::var("PREFIX") {
        let termux_am = format!("{prefix}/bin/termux-am");
        if Path::new(&termux_am).is_file() {
            push_unique(AndroidOpener::TermuxAm(termux_am));
        }
        let termux_open = format!("{prefix}/bin/termux-open");
        if Path::new(&termux_open).is_file() {
            push_unique(AndroidOpener::TermuxOpen(termux_open));
        }
        let termux_open_url = format!("{prefix}/bin/termux-open-url");
        if Path::new(&termux_open_url).is_file() {
            push_unique(AndroidOpener::TermuxOpenUrl(termux_open_url));
        }
    }

    let termux_am_static = format!("{}/bin/termux-am", crate::config::TERMUX_PREFIX_USR);
    if Path::new(&termux_am_static).is_file() {
        push_unique(AndroidOpener::TermuxAm(termux_am_static));
    }
    let termux_open_static = format!("{}/bin/termux-open", crate::config::TERMUX_PREFIX_USR);
    if Path::new(&termux_open_static).is_file() {
        push_unique(AndroidOpener::TermuxOpen(termux_open_static));
    }
    let termux_open_url_static =
        format!("{}/bin/termux-open-url", crate::config::TERMUX_PREFIX_USR);
    if Path::new(&termux_open_url_static).is_file() {
        push_unique(AndroidOpener::TermuxOpenUrl(termux_open_url_static));
    }

    if let Some(path) = find_in_path("termux-am") {
        push_unique(AndroidOpener::TermuxAm(path));
    }
    if let Some(path) = find_in_path("termux-open") {
        push_unique(AndroidOpener::TermuxOpen(path));
    }
    if let Some(path) = find_in_path("termux-open-url") {
        push_unique(AndroidOpener::TermuxOpenUrl(path));
    }

    #[cfg(target_os = "android")]
    if !is_termux {
        let is_root = unsafe { libc::getuid() == 0 };
        if is_root {
            if Path::new("/system/bin/am").is_file() {
                push_unique(AndroidOpener::SystemAm("/system/bin/am".to_string()));
            }
            if let Some(path) = find_in_path("am") {
                push_unique(AndroidOpener::SystemAm(path));
            }
        }
    }

    openers
}

static ANDROID_OPENERS: std::sync::LazyLock<Vec<AndroidOpener>> =
    std::sync::LazyLock::new(probe_android_openers);

pub fn android_openers() -> &'static [AndroidOpener] {
    ANDROID_OPENERS.as_slice()
}

fn append_android_intent_extras(
    cmd: &mut Command,
    subtitle: Option<&str>,
    headers: &[(String, String)],
) {
    if let Some(sub) = subtitle {
        cmd.arg("-e").arg("subtitles_location").arg(sub);
        cmd.arg("--eu").arg("subtitles_location").arg(sub);
        cmd.arg("-e").arg("subs").arg(sub);
        cmd.arg("--esal").arg("subs").arg(sub);
        cmd.arg("-e").arg("subs.enable").arg(sub);
        cmd.arg("--esal").arg("subs.enable").arg(sub);
        cmd.arg("-e").arg("sub").arg(sub);
        cmd.arg("--eu").arg("sub").arg(sub);
        cmd.arg("-e").arg("title_subtitle").arg(sub);
    }
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("user-agent") {
            cmd.arg("-e").arg("User-Agent").arg(value);
        } else if name.eq_ignore_ascii_case("referer") {
            cmd.arg("-e").arg("Referer").arg(value);
        }
    }
}

pub fn android_intent_command_for_opener(
    opener: &AndroidOpener,
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
) -> Command {
    match opener {
        AndroidOpener::TermuxOpen(path) => {
            let mut cmd = Command::new(path);
            cmd.arg("--chooser")
                .arg("--content-type")
                .arg("video/*")
                .arg(url);
            cmd
        }
        AndroidOpener::TermuxOpenUrl(path) => {
            let mut cmd = Command::new(path);
            cmd.arg(url);
            cmd
        }
        AndroidOpener::TermuxAm(path) => {
            let mut cmd = Command::new(path);
            cmd.arg("start")
                .arg("-a")
                .arg("android.intent.action.VIEW")
                .arg("-d")
                .arg(url)
                .arg("-t")
                .arg("video/*");
            append_android_intent_extras(&mut cmd, subtitle, headers);
            cmd
        }
        #[cfg(target_os = "android")]
        AndroidOpener::SystemAm(path) => {
            let mut cmd = Command::new(path);
            cmd.arg("start")
                .arg("--user")
                .arg("0")
                .arg("-a")
                .arg("android.intent.action.VIEW")
                .arg("-d")
                .arg(url)
                .arg("-t")
                .arg("video/*");
            append_android_intent_extras(&mut cmd, subtitle, headers);
            let current_path = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("/system/bin:/system/xbin:{current_path}"));
            cmd.env_remove("LD_LIBRARY_PATH");
            cmd.env_remove("LD_PRELOAD");
            cmd
        }
    }
}

pub fn android_intent_commands(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
) -> Vec<(AndroidOpener, Command)> {
    let openers = android_openers();
    if openers.is_empty() {
        let mut cmd = Command::new("termux-open");
        cmd.arg("--chooser")
            .arg("--content-type")
            .arg("video/*")
            .arg(url);
        return vec![(AndroidOpener::TermuxOpen("termux-open".to_string()), cmd)];
    }
    openers
        .iter()
        .map(|opener| {
            let cmd = android_intent_command_for_opener(opener, url, subtitle, headers);
            (opener.clone(), cmd)
        })
        .collect()
}

fn android_intent_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
) -> Command {
    let commands = android_intent_commands(url, subtitle, headers);
    commands
        .into_iter()
        .next()
        .map(|(_, cmd)| cmd)
        .unwrap_or_else(|| {
            let mut cmd = Command::new("termux-open");
            cmd.arg("--chooser")
                .arg("--content-type")
                .arg("video/*")
                .arg(url);
            cmd
        })
}

#[allow(clippy::too_many_arguments)]
fn mpv_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    iina: bool,
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
    max_height: Option<u64>,
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
    let is_dash = is_dash_url(url);
    command.arg(format!("{prefix}hwdec=auto-safe"));
    command.arg(format!("{prefix}cache=yes"));
    command.arg(format!("{prefix}cache-secs=120"));
    command.arg(format!("{prefix}cache-pause=yes"));
    command.arg(format!("{prefix}cache-pause-wait=3"));
    command.arg(format!("{prefix}cache-pause-initial=no"));
    let (max_bytes, back_bytes) =
        if cfg!(target_os = "android") || crate::config::is_termux_environment() {
            ("128M", "50M")
        } else {
            ("256M", "100M")
        };
    command.arg(format!("{prefix}demuxer-max-bytes={max_bytes}"));
    command.arg(format!("{prefix}demuxer-max-back-bytes={back_bytes}"));
    command.arg(format!("{prefix}demuxer-readahead-secs=120"));
    command.arg(format!("{prefix}demuxer-lavf-buffersize=1048576"));
    command.arg(format!("{prefix}stream-buffer-size=4M"));
    if is_dash {
        command.arg(format!("{prefix}force-seekable=yes"));
    } else {
        command.arg(format!("{prefix}ytdl=no"));
    }
    command.arg(format!(
        "{prefix}stream-lavf-o=reconnect=1,reconnect_streamed=1,reconnect_delay_max=5"
    ));
    if !iina {
        command.arg("--idle=no").arg("--keep-open=no");
    }
    let format_selector = ytdlp_format_selector(max_height);
    command.arg(format!("{prefix}ytdl-format={format_selector}"));
    if max_height.filter(|&h| h > 0).is_none() {
        command.arg(format!("{prefix}hls-bitrate=max"));
    }
    if let Some(start) = resume_seconds {
        if start > 0 {
            command.arg(format!("{prefix}start={start}"));
        }
    }

    if let Some((provider, subject_id, season, episode)) = tracker {
        if let Some(script_path) = tracker::ensure_tracker_script() {
            let script_str = normalize_player_path(&script_path.to_string_lossy());
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
        for (name, value) in headers {
            if name.eq_ignore_ascii_case("user-agent") {
                command.arg(format!("{prefix}user-agent={value}"));
            } else if name.eq_ignore_ascii_case("referer") {
                command.arg(format!("{prefix}referrer={value}"));
            }
        }
        for (name, value) in headers {
            if !name.eq_ignore_ascii_case("user-agent") && !name.eq_ignore_ascii_case("referer") {
                command.arg(format!("{prefix}http-header-fields={name}: {value}"));
                command.arg(format!(
                    "{prefix}ytdl-raw-options-append=add-header={name}:{value}"
                ));
            }
        }
    }
    if let Some(subtitle) = subtitle {
        let opt = if iina {
            "--mpv-sub-files"
        } else {
            "--sub-file"
        };
        let sub_path = normalize_player_path(subtitle);
        command.arg(format!("{opt}={sub_path}"));
    }

    if is_flatpak_executable(&executable) && (url.starts_with('/') || url.starts_with("file://")) {
        command.arg("@@").arg(url).arg("@@");
    } else {
        command.arg(url);
    }
    command
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
enum IinaResolution {
    Cli(String),
    AppFallback,
}

#[cfg(target_os = "macos")]
fn probe_iina_resolution() -> Option<IinaResolution> {
    if let Some(executable) = configured_executable("MOVIEBOX_IINA_PATH") {
        return Some(IinaResolution::Cli(executable));
    }

    let cli_global = "/Applications/IINA.app/Contents/MacOS/iina-cli";
    if Path::new(cli_global).exists() {
        return Some(IinaResolution::Cli(cli_global.to_string()));
    }
    if let Some(home) = dirs::home_dir() {
        let user_cli = home.join("Applications/IINA.app/Contents/MacOS/iina-cli");
        if user_cli.exists() {
            return Some(IinaResolution::Cli(user_cli.to_string_lossy().into_owned()));
        }
        let nix_iina = home.join(".nix-profile/bin/iina-cli");
        if nix_iina.exists() {
            return Some(IinaResolution::Cli(nix_iina.to_string_lossy().into_owned()));
        }
    }

    for candidate in &[
        "/opt/homebrew/bin/iina-cli",
        "/usr/local/bin/iina-cli",
        "/opt/local/bin/iina-cli",
        "/run/current-system/sw/bin/iina-cli",
    ] {
        if Path::new(candidate).exists() {
            return Some(IinaResolution::Cli(candidate.to_string()));
        }
    }

    if let Some(path) = find_in_path("iina").or_else(|| find_in_path("iina-cli")) {
        return Some(IinaResolution::Cli(path));
    }
    if Path::new("/Applications/IINA.app").exists()
        || dirs::home_dir().is_some_and(|home| home.join("Applications/IINA.app").exists())
    {
        return Some(IinaResolution::AppFallback);
    }

    None
}

#[cfg(target_os = "macos")]
static IINA_CACHED: std::sync::RwLock<Option<Option<IinaResolution>>> =
    std::sync::RwLock::new(None);

#[cfg(target_os = "macos")]
fn iina_resolution() -> Option<IinaResolution> {
    if let Ok(guard) = IINA_CACHED.read() {
        if let Some(cached) = &*guard {
            return cached.clone();
        }
    }

    let detected = probe_iina_resolution();
    if let Ok(mut guard) = IINA_CACHED.write() {
        *guard = Some(detected.clone());
    }
    detected
}

#[allow(clippy::too_many_arguments)]
fn iina_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    tracker: Option<(&str, &str, usize, usize)>,
    max_height: Option<u64>,
) -> Command {
    #[cfg(target_os = "macos")]
    let mut command = match iina_resolution() {
        Some(IinaResolution::Cli(executable)) => {
            let mut c = Command::new(executable);
            c.arg("--keep-running").arg("--no-stdin");
            c
        }
        Some(IinaResolution::AppFallback) => {
            let mut c = Command::new("open");
            c.arg("-a").arg("IINA").arg(url);
            return c;
        }
        None => Command::new("iina"),
    };
    #[cfg(not(target_os = "macos"))]
    let mut command = Command::new("iina");
    let mpv = mpv_command(
        url,
        subtitle,
        headers,
        true,
        window,
        resume_seconds,
        tracker,
        max_height,
    );
    for arg in mpv.get_args() {
        let s = arg.to_string_lossy();
        if !s.starts_with("--mpv-script") {
            command.arg(arg);
        }
    }
    command
}

#[cfg(target_os = "macos")]
pub fn iina_is_app_fallback() -> bool {
    matches!(iina_resolution(), Some(IinaResolution::AppFallback))
}

#[cfg(not(target_os = "macos"))]
pub fn iina_is_app_fallback() -> bool {
    false
}

fn vlc_command(
    url: &str,
    subtitle: Option<&str>,
    headers: &[(String, String)],
    window: Option<(u32, u32)>,
    resume_seconds: Option<u64>,
    max_height: Option<u64>,
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
    #[cfg(not(target_os = "macos"))]
    command.arg("--no-one-instance");
    command.arg("--network-caching=3000");
    command.arg("--file-caching=3000");
    command.arg("--http-reconnect");
    command.arg("--adaptive-logic=predictive");
    if let Some(height) = max_height.filter(|&h| h > 0) {
        command.arg(format!("--adaptive-maxheight={height}"));
    }
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
        let sub_path = vlc_subtitle_path(subtitle);
        command.arg(format!("--sub-file={sub_path}"));
    }

    if is_flatpak_executable(&executable) && (url.starts_with('/') || url.starts_with("file://")) {
        command.arg("@@").arg(url).arg("@@");
    } else {
        command.arg(url);
    }
    command
}

fn probe_player_executable(
    env_var: &str,
    candidates: &[String],
    bin_names: &[&str],
    flatpak_id: Option<&str>,
) -> Option<String> {
    if let Some(executable) = configured_executable(env_var) {
        return Some(executable);
    }

    for path in candidates {
        if Path::new(path).is_file() {
            return Some(path.to_string());
        }
    }

    for bin in bin_names {
        if let Some(path) = find_in_path(bin) {
            return Some(path);
        }
    }

    if let Some(id) = flatpak_id {
        flatpak_executable(id)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn query_windows_registry_value(key: &str, value_name: Option<&str>) -> Option<String> {
    use std::os::windows::process::CommandExt;

    let mut cmd = Command::new("reg.exe");
    cmd.arg("query").arg(key);
    if let Some(val) = value_name {
        cmd.arg("/v").arg(val);
    } else {
        cmd.arg("/ve");
    }
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("HKEY_") || trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Some(pos) = parts
                .iter()
                .position(|&p| p == "REG_SZ" || p == "REG_EXPAND_SZ")
            {
                let is_expand = parts[pos] == "REG_EXPAND_SZ";
                if pos + 1 < parts.len() {
                    let val = parts[pos + 1..].join(" ");
                    let clean = val.trim_matches('"').trim();
                    if !clean.is_empty() {
                        let expanded = if is_expand {
                            expand_env_vars(clean)
                        } else {
                            clean.to_string()
                        };
                        return Some(expanded);
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn expand_env_vars(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut var_name = String::new();
            let mut found_end = false;
            for next_ch in chars.by_ref() {
                if next_ch == '%' {
                    found_end = true;
                    break;
                }
                var_name.push(next_ch);
            }
            if found_end && !var_name.is_empty() {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

pub fn windows_mpv_candidate_paths(
    localappdata: Option<&str>,
    appdata: Option<&str>,
    userprofile: Option<&Path>,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for name in &[
                "mpv.exe",
                "mpv.com",
                "mpvnet.exe",
                "mpvnet.com",
                r"mpv\mpv.exe",
                r"mpv\mpv.com",
                r"mpv.net\mpvnet.exe",
                r"mpv.net\mpv.exe",
            ] {
                candidates.push(parent.join(name).to_string_lossy().into_owned());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for name in &[
            "mpv.exe",
            "mpv.com",
            "mpvnet.exe",
            "mpvnet.com",
            r"mpv\mpv.exe",
            r"mpv\mpv.com",
            r"mpv.net\mpvnet.exe",
            r"mpv.net\mpv.exe",
        ] {
            candidates.push(cwd.join(name).to_string_lossy().into_owned());
        }
    }

    if let Some(local) = localappdata {
        candidates.push(format!(r"{local}\Microsoft\WinGet\Links\mpv.exe"));
        candidates.push(format!(r"{local}\Microsoft\WinGet\Links\mpv.com"));
        candidates.push(format!(r"{local}\Microsoft\WinGet\Links\mpvnet.exe"));
        candidates.push(format!(r"{local}\Programs\mpv\mpv.exe"));
        candidates.push(format!(r"{local}\Programs\mpv\mpv.com"));
        candidates.push(format!(r"{local}\Programs\mpv.net\mpvnet.exe"));
        candidates.push(format!(r"{local}\Programs\mpv.net\mpv.exe"));

        let packages_dir = std::path::PathBuf::from(format!(r"{local}\Microsoft\WinGet\Packages"));
        if packages_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&packages_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if name.contains("mpv") && path.is_dir() {
                        candidates.push(path.join("mpv.exe").to_string_lossy().into_owned());
                        candidates.push(path.join("mpv.com").to_string_lossy().into_owned());
                        candidates.push(path.join("mpvnet.exe").to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    if let Some(appdata_dir) = appdata {
        candidates.push(format!(r"{appdata_dir}\mpv\mpv.exe"));
        candidates.push(format!(r"{appdata_dir}\mpv\mpv.com"));
    }

    if let Some(home) = userprofile {
        for sub in &["Downloads", "Desktop"] {
            let folder = home.join(sub);
            candidates.push(folder.join("mpv.exe").to_string_lossy().into_owned());
            candidates.push(folder.join("mpv.com").to_string_lossy().into_owned());
            candidates.push(folder.join("mpvnet.exe").to_string_lossy().into_owned());

            if folder.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&folder) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let folder_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if folder_name.contains("mpv") && path.is_dir() {
                            candidates.push(path.join("mpv.exe").to_string_lossy().into_owned());
                            candidates.push(path.join("mpv.com").to_string_lossy().into_owned());
                            candidates.push(path.join("mpvnet.exe").to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }

        candidates.push(
            home.join(r"scoop\shims\mpv.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\shims\mpv.com")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\shims\mpvnet.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\apps\mpv\current\mpv.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\apps\mpv\current\mpv.com")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\apps\mpv-git\current\mpv.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\apps\mpv.net\current\mpvnet.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(home.join(r"mpv\mpv.exe").to_string_lossy().into_owned());
        candidates.push(home.join(r"mpv\mpv.com").to_string_lossy().into_owned());
        candidates.push(home.join(r"bin\mpv.exe").to_string_lossy().into_owned());
    }

    candidates.push(r"C:\Program Files\mpv\mpv.exe".to_string());
    candidates.push(r"C:\Program Files\mpv\mpv.com".to_string());
    candidates.push(r"C:\Program Files\MPV Player\mpv.exe".to_string());
    candidates.push(r"C:\Program Files\MPV Player\mpv.com".to_string());
    candidates.push(r"C:\Program Files\mpv-player\mpv.exe".to_string());
    candidates.push(r"C:\Program Files\mpv-player\mpv.com".to_string());
    candidates.push(r"C:\Program Files\mpv.net\mpvnet.exe".to_string());
    candidates.push(r"C:\Program Files\mpv.net\mpv.exe".to_string());
    candidates.push(r"C:\Program Files (x86)\mpv\mpv.exe".to_string());
    candidates.push(r"C:\Program Files (x86)\mpv\mpv.com".to_string());
    candidates.push(r"C:\Program Files (x86)\mpv.net\mpvnet.exe".to_string());
    candidates.push(r"C:\mpv\mpv.exe".to_string());
    candidates.push(r"C:\mpv\mpv.com".to_string());
    candidates.push(r"D:\mpv\mpv.exe".to_string());
    candidates.push(r"D:\mpv\mpv.com".to_string());
    candidates.push(r"C:\tools\mpv\mpv.exe".to_string());
    candidates.push(r"C:\tools\mpv\mpv.com".to_string());
    candidates.push(r"C:\ProgramData\chocolatey\bin\mpv.exe".to_string());
    candidates.push(r"C:\ProgramData\scoop\shims\mpv.exe".to_string());
    candidates.push(r"C:\ProgramData\scoop\apps\mpv\current\mpv.exe".to_string());

    #[cfg(target_os = "windows")]
    {
        for key in &[
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\mpv.exe",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\mpv.exe",
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\mpvnet.exe",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\mpvnet.exe",
        ] {
            if let Some(reg_path) = query_windows_registry_value(key, None) {
                candidates.push(reg_path);
            }
        }
    }

    candidates
}

pub fn windows_vlc_candidate_paths(
    localappdata: Option<&str>,
    appdata: Option<&str>,
    userprofile: Option<&Path>,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for name in &["vlc.exe", r"vlc\vlc.exe", r"VideoLAN\VLC\vlc.exe"] {
                candidates.push(parent.join(name).to_string_lossy().into_owned());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for name in &["vlc.exe", r"vlc\vlc.exe", r"VideoLAN\VLC\vlc.exe"] {
            candidates.push(cwd.join(name).to_string_lossy().into_owned());
        }
    }

    if let Some(local) = localappdata {
        candidates.push(format!(r"{local}\Microsoft\WinGet\Links\vlc.exe"));
        candidates.push(format!(r"{local}\Programs\VLC\vlc.exe"));
        candidates.push(format!(r"{local}\Programs\VideoLAN\VLC\vlc.exe"));

        let packages_dir = std::path::PathBuf::from(format!(r"{local}\Microsoft\WinGet\Packages"));
        if packages_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&packages_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if (name.contains("vlc") || name.contains("videolan")) && path.is_dir() {
                        candidates.push(path.join("vlc.exe").to_string_lossy().into_owned());
                        candidates.push(path.join(r"vlc\vlc.exe").to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    if let Some(appdata_dir) = appdata {
        candidates.push(format!(r"{appdata_dir}\vlc\vlc.exe"));
        candidates.push(format!(r"{appdata_dir}\VideoLAN\VLC\vlc.exe"));
    }

    if let Some(home) = userprofile {
        for sub in &["Downloads", "Desktop"] {
            let folder = home.join(sub);
            candidates.push(folder.join("vlc.exe").to_string_lossy().into_owned());
            if folder.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&folder) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let folder_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if (folder_name.contains("vlc") || folder_name.contains("videolan"))
                            && path.is_dir()
                        {
                            candidates.push(path.join("vlc.exe").to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }

        candidates.push(
            home.join(r"scoop\shims\vlc.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(
            home.join(r"scoop\apps\vlc\current\vlc.exe")
                .to_string_lossy()
                .into_owned(),
        );
        candidates.push(home.join(r"vlc\vlc.exe").to_string_lossy().into_owned());
        candidates.push(home.join(r"bin\vlc.exe").to_string_lossy().into_owned());
    }

    candidates.push(r"C:\Program Files\VideoLAN\VLC\vlc.exe".to_string());
    candidates.push(r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe".to_string());
    candidates.push(r"C:\vlc\vlc.exe".to_string());
    candidates.push(r"D:\vlc\vlc.exe".to_string());
    candidates.push(r"C:\tools\vlc\vlc.exe".to_string());
    candidates.push(r"C:\ProgramData\chocolatey\bin\vlc.exe".to_string());
    candidates.push(r"C:\ProgramData\scoop\shims\vlc.exe".to_string());
    candidates.push(r"C:\ProgramData\scoop\apps\vlc\current\vlc.exe".to_string());

    #[cfg(target_os = "windows")]
    {
        for key in &[
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\vlc.exe",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\vlc.exe",
        ] {
            if let Some(reg_path) = query_windows_registry_value(key, None) {
                candidates.push(reg_path);
            }
        }
    }

    candidates
}

fn probe_mpv() -> Option<String> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let localappdata = std::env::var("LOCALAPPDATA").ok();
        let appdata = std::env::var("APPDATA").ok();
        let home = dirs::home_dir();
        candidates.extend(windows_mpv_candidate_paths(
            localappdata.as_deref(),
            appdata.as_deref(),
            home.as_deref(),
        ));
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
            candidates.push(
                home.join(".nix-profile/bin/mpv")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        candidates.push("/opt/homebrew/bin/mpv".to_string());
        candidates.push("/opt/local/bin/mpv".to_string());
        candidates.push("/usr/local/bin/mpv".to_string());
        candidates.push("/run/current-system/sw/bin/mpv".to_string());
        candidates.push("/bin/mpv".to_string());
    }

    #[cfg(target_os = "android")]
    {
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/mpv"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/mpv".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push("/usr/bin/mpv".to_string());
        candidates.push("/usr/local/bin/mpv".to_string());
        candidates.push("/bin/mpv".to_string());
        candidates.push("/run/current-system/sw/bin/mpv".to_string());
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join(".local/share/flatpak/exports/bin/io.mpv.Mpv")
                    .to_string_lossy()
                    .into_owned(),
            );
            candidates.push(home.join(".local/bin/mpv").to_string_lossy().into_owned());
            candidates.push(
                home.join(".nix-profile/bin/mpv")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        candidates.push("/var/lib/flatpak/exports/bin/io.mpv.Mpv".to_string());
        candidates.push("/snap/bin/mpv".to_string());
        candidates.push("/var/lib/snapd/snap/bin/mpv".to_string());
        candidates.push("/app/bin/mpv".to_string());
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/mpv"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/mpv".to_string());
    }
    let bin_names = if cfg!(target_os = "windows") {
        &[
            "mpv.exe",
            "mpv.com",
            "mpv",
            "mpvnet.exe",
            "mpvnet.com",
            "mpvnet",
        ][..]
    } else {
        &["mpv", "io.mpv.Mpv"][..]
    };

    probe_player_executable(
        "MOVIEBOX_MPV_PATH",
        &candidates,
        bin_names,
        Some("io.mpv.Mpv"),
    )
}

fn probe_vlc() -> Option<String> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let localappdata = std::env::var("LOCALAPPDATA").ok();
        let appdata = std::env::var("APPDATA").ok();
        let home = dirs::home_dir();
        candidates.extend(windows_vlc_candidate_paths(
            localappdata.as_deref(),
            appdata.as_deref(),
            home.as_deref(),
        ));
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
            candidates.push(
                home.join(".nix-profile/bin/vlc")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        candidates.push("/opt/homebrew/bin/vlc".to_string());
        candidates.push("/opt/local/bin/vlc".to_string());
        candidates.push("/usr/local/bin/vlc".to_string());
        candidates.push("/run/current-system/sw/bin/vlc".to_string());
        candidates.push("/bin/vlc".to_string());
    }

    #[cfg(target_os = "android")]
    {
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/vlc"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/vlc".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        candidates.push("/usr/bin/vlc".to_string());
        candidates.push("/usr/local/bin/vlc".to_string());
        candidates.push("/bin/vlc".to_string());
        candidates.push("/run/current-system/sw/bin/vlc".to_string());
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join(".local/share/flatpak/exports/bin/org.videolan.VLC")
                    .to_string_lossy()
                    .into_owned(),
            );
            candidates.push(home.join(".local/bin/vlc").to_string_lossy().into_owned());
            candidates.push(
                home.join(".nix-profile/bin/vlc")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        candidates.push("/var/lib/flatpak/exports/bin/org.videolan.VLC".to_string());
        candidates.push("/snap/bin/vlc".to_string());
        candidates.push("/var/lib/snapd/snap/bin/vlc".to_string());
        candidates.push("/app/bin/vlc".to_string());
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/vlc"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/vlc".to_string());
    }
    let bin_names = if cfg!(target_os = "windows") {
        &["vlc.exe", "vlc"][..]
    } else {
        &["vlc", "org.videolan.VLC"][..]
    };

    probe_player_executable(
        "MOVIEBOX_VLC_PATH",
        &candidates,
        bin_names,
        Some("org.videolan.VLC"),
    )
}

static MPV_CACHED: std::sync::RwLock<Option<Option<String>>> = std::sync::RwLock::new(None);
static VLC_CACHED: std::sync::RwLock<Option<Option<String>>> = std::sync::RwLock::new(None);

fn mpv_executable() -> Option<String> {
    if let Ok(guard) = MPV_CACHED.read() {
        if let Some(cached) = &*guard {
            if let Some(path) = cached {
                if path.starts_with("flatpak run ") || Path::new(path).is_file() {
                    return Some(path.clone());
                }
            } else {
                return None;
            }
        }
    }

    let detected = probe_mpv();
    if let Ok(mut guard) = MPV_CACHED.write() {
        *guard = Some(detected.clone());
    }
    detected
}

fn vlc_executable() -> Option<String> {
    if let Ok(guard) = VLC_CACHED.read() {
        if let Some(cached) = &*guard {
            if let Some(path) = cached {
                if path.starts_with("flatpak run ") || Path::new(path).is_file() {
                    return Some(path.clone());
                }
            } else {
                return None;
            }
        }
    }

    let detected = probe_vlc();
    if let Ok(mut guard) = VLC_CACHED.write() {
        *guard = Some(detected.clone());
    }
    detected
}
pub fn clear_cached_player_executables() {
    if let Ok(mut guard) = MPV_CACHED.write() {
        *guard = None;
    }
    if let Ok(mut guard) = VLC_CACHED.write() {
        *guard = None;
    }
    #[cfg(target_os = "macos")]
    if let Ok(mut guard) = IINA_CACHED.write() {
        *guard = None;
    }
}

fn flatpak_executable(app_id: &str) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    if find_in_path("flatpak").is_some() {
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
    let raw = std::env::var(variable).ok().or_else(|| {
        let cfg = crate::config::load();
        match variable {
            "MOVIEBOX_VLC_PATH" => cfg.vlc_path,
            "MOVIEBOX_MPV_PATH" => cfg.mpv_path,
            "MOVIEBOX_IINA_PATH" => cfg.iina_path,
            _ => None,
        }
    })?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("flatpak run ")
        || Path::new(trimmed).exists()
        || find_in_path(trimmed).is_some()
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

pub(crate) fn find_in_path(name: &str) -> Option<String> {
    if std::path::Path::new(name).is_file() {
        return Some(name.to_string());
    }
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new(&format!("{name}.exe")).is_file() {
            return Some(format!("{name}.exe"));
        }
        if std::path::Path::new(&format!("{name}.com")).is_file() {
            return Some(format!("{name}.com"));
        }
    }

    let mut paths_to_search: Vec<std::path::PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        paths_to_search.extend(std::env::split_paths(&path));
    }

    #[cfg(target_os = "windows")]
    {
        for (reg_key, reg_val) in &[
            (r"HKCU\Environment", "Path"),
            (
                r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                "Path",
            ),
        ] {
            if let Some(raw_path) = query_windows_registry_value(reg_key, Some(reg_val)) {
                paths_to_search.extend(std::env::split_paths(&raw_path));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        for d in STANDARD_UNIX_BIN_DIRS {
            let p = std::path::PathBuf::from(d);
            if !paths_to_search.contains(&p) {
                paths_to_search.push(p);
            }
        }
        if let Ok(prefix) = std::env::var("PREFIX") {
            let p = std::path::PathBuf::from(format!("{prefix}/bin"));
            if !paths_to_search.contains(&p) {
                paths_to_search.push(p);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let local_bin = home.join(".local/bin");
            if !paths_to_search.contains(&local_bin) {
                paths_to_search.push(local_bin);
            }
            let nix_bin = home.join(".nix-profile/bin");
            if !paths_to_search.contains(&nix_bin) {
                paths_to_search.push(nix_bin);
            }
        }
    }

    for dir in paths_to_search {
        let candidate = dir.join(name);
        #[cfg(target_os = "windows")]
        {
            let candidates = [
                candidate.clone(),
                candidate.with_extension("exe"),
                candidate.with_extension("com"),
                candidate.with_extension("cmd"),
                candidate.with_extension("bat"),
            ];
            for c in candidates {
                if c.is_file() {
                    return Some(c.to_string_lossy().into_owned());
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
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

fn normalize_player_path(path: &str) -> String {
    if path.starts_with(r"\\") || path.starts_with("//") {
        path.to_string()
    } else {
        path.replace('\\', "/")
    }
}

fn vlc_subtitle_path(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let bytes = path.as_bytes();
    let is_windows_drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    let is_unc = path.starts_with(r"\\") || path.starts_with("//");
    if is_windows_drive || is_unc {
        path.replace('/', r"\")
    } else {
        path.to_string()
    }
}

pub fn format_mpv_script_opts(
    provider: &str,
    subject_id: &str,
    season: usize,
    episode: usize,
    state_file: &Path,
) -> String {
    let state_file_str = normalize_player_path(&state_file.to_string_lossy()).replace(',', "_");
    let safe_provider = provider.replace(',', "_");
    let safe_subject = subject_id.replace(',', "_");
    format!(
        "moviebox-provider={safe_provider},moviebox-subject_id={safe_subject},moviebox-season={season},moviebox-episode={episode},moviebox-state_file={state_file_str}"
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

    #[test]
    fn vlc_command_preserves_supported_playback_options() {
        let command = vlc_command(
            "https://example.test/video.m3u8",
            Some("/tmp/subtitle.srt"),
            &[
                ("Referer".into(), "https://example.test/".into()),
                ("User-Agent".into(), "MovieBox-Test".into()),
                ("Cookie".into(), "ignored=by-vlc-filter".into()),
            ],
            Some((1280, 720)),
            Some(42),
            None,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.contains(&"--width=1280".into()));
        assert!(args.contains(&"--height=720".into()));
        assert!(!args.contains(&"--play-and-exit".into()));
        assert!(args.contains(&"--start-time=42".into()));
        assert!(args.contains(&"--http-referrer=https://example.test/".into()));
        assert!(args.contains(&"--http-user-agent=MovieBox-Test".into()));
        assert!(args.contains(&"--sub-file=/tmp/subtitle.srt".into()));
        assert!(!args.iter().any(|arg| arg.starts_with("--http-cookie")));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://example.test/video.m3u8")
        );
    }

    #[test]
    fn vlc_command_preserves_windows_native_subtitle_separators() {
        for input in [
            r"C:\Users\User\AppData\Local\MovieBox-Tui\subs\sub.srt",
            "C:/Users/User/AppData/Local/MovieBox-Tui/subs/sub.srt",
        ] {
            let command = vlc_command(
                "https://example.test/video.mp4",
                Some(input),
                &[],
                None,
                None,
                None,
            );
            let args = command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            assert!(args.contains(
                &r"--sub-file=C:\Users\User\AppData\Local\MovieBox-Tui\subs\sub.srt".into()
            ));
        }
    }
    #[test]
    fn vlc_command_preserves_unc_subtitle_paths() {
        let command = vlc_command(
            "https://example.test/video.mp4",
            Some(r"\\server\share\subs\sub.srt"),
            &[],
            None,
            None,
            None,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&r"--sub-file=\\server\share\subs\sub.srt".into()));
    }
    #[test]
    fn mpv_command_passes_multiple_header_fields_individually() {
        let headers = vec![
            ("Cookie".to_string(), "session=abc, token=123".to_string()),
            ("Accept".to_string(), "text/html, */*".to_string()),
            ("User-Agent".to_string(), "CustomUA".to_string()),
            ("Referer".to_string(), "https://example.com/".to_string()),
        ];
        let cmd = mpv_command(
            "https://example.com/video.mp4",
            None,
            &headers,
            false,
            None,
            None,
            None,
            None,
        );

        let args = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&"--user-agent=CustomUA".to_string()));
        assert!(args.contains(&"--referrer=https://example.com/".to_string()));
        assert!(args.contains(&"--http-header-fields=Cookie: session=abc, token=123".to_string()));
        assert!(args.contains(&"--http-header-fields=Accept: text/html, */*".to_string()));
    }
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_android_intent_commands_fallback_order() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("termux_fallback_test_{}", std::process::id()));
        let bin_dir = temp_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let termux_am = bin_dir.join("termux-am");
        let termux_open = bin_dir.join("termux-open");
        std::fs::write(&termux_am, "#!/bin/sh\nexit 0").unwrap();
        std::fs::write(&termux_open, "#!/bin/sh\nexit 0").unwrap();

        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
            std::env::set_var("PREFIX", temp_dir.to_str().unwrap());
        }
        let commands = android_intent_commands("https://example.test/stream.m3u8", None, &[]);
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
            std::env::remove_var("PREFIX");
        }
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(commands.len() >= 2);
        assert!(matches!(commands[0].0, AndroidOpener::TermuxAm(_)));
        assert!(matches!(commands[1].0, AndroidOpener::TermuxOpen(_)));
    }

    #[test]
    fn header_support_allows_android_cookies_and_vlc_proxy() {
        let headers = vec![("Cookie".into(), "session=secret".into())];
        assert!(supports_headers(PlayerKind::AndroidIntent, &headers));
        assert!(supports_headers(PlayerKind::Vlc, &headers));
        assert!(supports_headers(
            PlayerKind::Vlc,
            &[("referer".into(), "https://example.test/".into())]
        ));
        assert!(supports_headers(PlayerKind::AndroidIntent, &[]));
        assert!(supports_headers(
            PlayerKind::AndroidIntent,
            &[
                ("referer".into(), "https://example.test/".into()),
                ("user-agent".into(), "TestAgent/1.0".into())
            ]
        ));
        assert!(supports_headers(PlayerKind::Vlc, &[]));
        assert!(supports_headers(PlayerKind::Mpv, &headers));
    }

    #[test]
    fn test_android_intent_command_structure() {
        let cmd = android_intent_command("https://example.test/video.mp4", None, &[]);
        let args = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&"https://example.test/video.mp4".to_string()));
    }
    #[test]
    fn test_mpv_command_headers_no_broken_ytdl_raw_options() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let headers = vec![
            ("User-Agent".into(), ua.into()),
            ("Referer".into(), "https://4khdhub.one".into()),
            ("Cookie".into(), "auth=token123".into()),
        ];
        let cmd = mpv_command(
            "https://example.test/stream.m3u8",
            None,
            &headers,
            false,
            None,
            None,
            None,
            None,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == &format!("--user-agent={ua}")));
        assert!(args.iter().any(|a| a == "--referrer=https://4khdhub.one"));
        assert!(
            args.iter()
                .any(|a| a == "--http-header-fields=Cookie: auth=token123")
        );
        assert!(
            args.iter()
                .any(|a| a == "--ytdl-raw-options-append=add-header=Cookie:auth=token123")
        );
        assert!(!args.iter().any(|a| a.starts_with("--ytdl-raw-options=")));
    }

    #[test]
    fn test_detect_prioritizes_android_intent_on_termux() {
        let temp_dir = std::env::temp_dir().join(format!("termux_test_{}", std::process::id()));
        let bin_dir = temp_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let termux_am = bin_dir.join("termux-am");
        std::fs::write(&termux_am, "#!/bin/sh\nexit 0").unwrap();
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("TERMUX_VERSION", "0.118.0");
            std::env::set_var("PREFIX", temp_dir.to_str().unwrap());
        }
        let detected = detect();
        unsafe {
            std::env::remove_var("TERMUX_VERSION");
            std::env::remove_var("PREFIX");
        }
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(!detected.is_empty());
        assert_eq!(detected[0], PlayerKind::AndroidIntent);
    }

    #[test]
    fn test_windows_mpv_candidate_paths_comprehensive() {
        let home = PathBuf::from(r"C:\Users\TestUser");
        let candidates = windows_mpv_candidate_paths(
            Some(r"C:\Users\TestUser\AppData\Local"),
            Some(r"C:\Users\TestUser\AppData\Roaming"),
            Some(&home),
        );

        assert!(
            candidates
                .iter()
                .any(|c| c.contains("WinGet") && c.contains("mpv.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("WinGet") && c.contains("mpv.com"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("mpv.net") && c.contains("mpvnet.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Downloads") && c.contains("mpv.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Desktop") && c.contains("mpv.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("scoop") && c.contains("mpv.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Program Files") && c.contains("mpv.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Program Files") && c.contains("mpvnet.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("C:") && c.contains("mpv.exe"))
        );
    }

    #[test]
    fn test_windows_vlc_candidate_paths_comprehensive() {
        let home = PathBuf::from(r"C:\Users\TestUser");
        let candidates = windows_vlc_candidate_paths(
            Some(r"C:\Users\TestUser\AppData\Local"),
            Some(r"C:\Users\TestUser\AppData\Roaming"),
            Some(&home),
        );

        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Program Files") && c.contains("vlc.exe"))
        );
        assert!(!candidates.iter().any(|c| c.contains("WindowsApps")));
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("WinGet") && c.contains("vlc.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Downloads") && c.contains("vlc.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("Desktop") && c.contains("vlc.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("scoop") && c.contains("vlc.exe"))
        );
        assert!(
            candidates
                .iter()
                .any(|c| c.contains("C:") && c.contains("vlc.exe"))
        );
    }
    #[test]
    fn test_create_no_window_constant() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }
    #[test]
    fn test_configured_executable_fallback_to_config() {
        clear_cached_player_executables();
        let _cfg = crate::config::Config {
            vlc_path: Some("/nonexistent/vlc.exe".to_string()),
            ..Default::default()
        };
        assert!(configured_executable("MOVIEBOX_VLC_PATH").is_none());
    }

    #[test]
    fn test_mpv_command_headers_and_arguments_assembly() {
        let headers = vec![
            ("User-Agent".to_string(), "MovieBox-Tui/0.1.23".to_string()),
            ("Referer".to_string(), "https://upstream.cdn/".to_string()),
            ("Origin".to_string(), "https://upstream.cdn".to_string()),
        ];
        let cmd = mpv_command(
            "https://upstream.cdn/stream.m3u8",
            Some("/tmp/test.srt"),
            &headers,
            false,
            Some((1920, 1080)),
            Some(120),
            None,
            None,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        assert!(args.contains(&"--sub-file=/tmp/test.srt".to_string()));
        assert!(args.contains(&"--start=120".to_string()));
        assert!(args.contains(&"--autofit=1920x1080".to_string()));
        assert!(args.contains(&"--hwdec=auto-safe".to_string()));
        assert!(args.contains(&"--cache=yes".to_string()));
        assert!(args.contains(&"--cache-secs=120".to_string()));
        assert!(args.contains(&"--cache-pause=yes".to_string()));
        assert!(args.contains(&"--cache-pause-wait=3".to_string()));
        assert!(args.contains(&"--cache-pause-initial=no".to_string()));
        assert!(args.contains(&"--demuxer-max-bytes=256M".to_string()));
        assert!(args.contains(&"--demuxer-readahead-secs=120".to_string()));
        assert!(args.contains(&"--demuxer-lavf-buffersize=1048576".to_string()));
        assert!(args.contains(&"--stream-buffer-size=4M".to_string()));
        assert!(args.contains(&"--ytdl=no".to_string()));
        assert!(args.contains(
            &"--stream-lavf-o=reconnect=1,reconnect_streamed=1,reconnect_delay_max=5".to_string()
        ));
        assert!(
            args.iter().any(|a| a.starts_with("--http-header-fields="))
                || args
                    .iter()
                    .any(|a| a.starts_with("--ytdl-raw-options-append="))
        );
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://upstream.cdn/stream.m3u8")
        );
    }

    #[test]
    fn test_vlc_command_cookie_header_filtering() {
        let headers = vec![
            ("User-Agent".to_string(), "VLC-Agent".to_string()),
            ("Referer".to_string(), "https://cdn.example.com".to_string()),
            ("Cookie".to_string(), "CloudFront-Signature=abc".to_string()),
        ];
        let cmd = vlc_command(
            "http://127.0.0.1:4567/proxy/manifest.mpd",
            None,
            &headers,
            None,
            None,
            None,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        assert!(args.contains(&"--network-caching=3000".to_string()));
        assert!(args.contains(&"--file-caching=3000".to_string()));
        assert!(args.contains(&"--http-reconnect".to_string()));
        assert!(args.contains(&"--adaptive-logic=predictive".to_string()));
        assert!(args.contains(&"--http-user-agent=VLC-Agent".to_string()));
        assert!(args.contains(&"--http-referrer=https://cdn.example.com".to_string()));
        assert!(!args.iter().any(|a| a.contains("CloudFront-Signature")));
        assert_eq!(
            args.last().map(String::as_str),
            Some("http://127.0.0.1:4567/proxy/manifest.mpd")
        );
    }

    #[test]
    fn test_has_graphical_display_detection() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let orig_display = std::env::var("DISPLAY").ok();
        let orig_wayland = std::env::var("WAYLAND_DISPLAY").ok();

        unsafe {
            std::env::set_var("DISPLAY", ":0");
            std::env::remove_var("WAYLAND_DISPLAY");
        }
        assert!(has_graphical_display());

        unsafe {
            std::env::remove_var("DISPLAY");
            std::env::set_var("WAYLAND_DISPLAY", "wayland-1");
        }
        assert!(has_graphical_display());

        unsafe {
            std::env::set_var("DISPLAY", "   ");
            std::env::remove_var("WAYLAND_DISPLAY");
        }
        assert!(!has_graphical_display());

        unsafe {
            if let Some(val) = orig_display {
                std::env::set_var("DISPLAY", val);
            } else {
                std::env::remove_var("DISPLAY");
            }
            if let Some(val) = orig_wayland {
                std::env::set_var("WAYLAND_DISPLAY", val);
            } else {
                std::env::remove_var("WAYLAND_DISPLAY");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_configure_detached_process_survives_sighup() {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", "sleep 0.2"]);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        super::configure_detached_process(&mut cmd);
        let mut child = cmd.spawn().expect("spawn detached sh");
        let pid = child.id() as libc::pid_t;
        std::thread::sleep(std::time::Duration::from_millis(40));
        unsafe {
            libc::kill(pid, libc::SIGHUP);
            libc::kill(-pid, libc::SIGHUP);
        }
        let status = child.wait().expect("wait detached sh");
        assert!(status.success());
    }

    #[test]
    fn test_iina_command_dash_vs_direct_and_script_filtering() {
        let direct_cmd = command(
            PlayerKind::Iina,
            "https://cdn.example.com/movie.mkv",
            Some("/tmp/sub.srt"),
            &[("Referer".to_string(), "https://example.com".to_string())],
            Some((1280, 720)),
            Some(120),
            Some(("moviebox", "subj_1", 1, 1)),
            Some(1080),
        );
        let direct_args: Vec<String> = direct_cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(direct_args.iter().any(|a| a.ends_with("hwdec=auto-safe")));
        assert!(direct_args.contains(&"--mpv-sub-files=/tmp/sub.srt".to_string()));
        assert!(
            direct_args
                .iter()
                .any(|a| a.ends_with("stream-buffer-size=4M"))
        );
        assert!(direct_args.iter().any(|a| a.ends_with("ytdl=no")));
        assert!(!direct_args.iter().any(|a| a.contains("force-seekable=yes")));
        #[cfg(target_os = "macos")]
        assert!(!direct_args.iter().any(|a| a.starts_with("--mpv-script=")));

        let dash_cmd = command(
            PlayerKind::Iina,
            "http://127.0.0.1:9000/https/cdn.example.com/dash/index.mpd",
            None,
            &[],
            None,
            None,
            None,
            Some(720),
        );
        let dash_args: Vec<String> = dash_cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(dash_args.iter().any(|a| a.ends_with("force-seekable=yes")));
        assert!(!dash_args.iter().any(|a| a.ends_with("ytdl=no")));
    }

    #[test]
    fn test_flatpak_process_command_preserves_flags_and_mounts() {
        let cmd = build_player_process_command("flatpak run --user org.videolan.VLC");
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "run",
                "--file-forwarding",
                "--filesystem=xdg-cache/moviebox-tui:ro",
                "--filesystem=xdg-data/moviebox-tui",
                "--filesystem=/tmp:ro",
                "--user",
                "org.videolan.VLC",
            ]
        );

        let export_cmd = build_player_process_command("/var/lib/flatpak/exports/bin/io.mpv.Mpv");
        let export_args: Vec<String> = export_cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(export_args.last().map(String::as_str), Some("io.mpv.Mpv"));
        assert!(export_args.contains(&"--filesystem=xdg-data/moviebox-tui".to_string()));
    }
}
