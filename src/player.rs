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

    let is_termux = crate::updater::artifact::is_termux_environment();
    if is_termux && !android_openers().is_empty() {
        players.push(PlayerKind::AndroidIntent);
    }

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

    if !is_termux && !android_openers().is_empty() {
        players.push(PlayerKind::AndroidIntent);
    }

    players
}

pub fn supports_headers(kind: PlayerKind, headers: &[(String, String)]) -> bool {
    if headers.is_empty() {
        return true;
    }
    #[cfg(target_os = "macos")]
    if kind == PlayerKind::Iina && !iina_cli_exists() {
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
        PlayerKind::Vlc => vlc_command(url, subtitle, headers, window, resume_seconds),
        PlayerKind::AndroidIntent => android_intent_command(url, subtitle, headers),
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
    let is_termux = crate::updater::artifact::is_termux_environment();

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

    let termux_am_static = format!(
        "{}/bin/termux-am",
        crate::updater::artifact::TERMUX_PREFIX_USR
    );
    if Path::new(&termux_am_static).is_file() {
        push_unique(AndroidOpener::TermuxAm(termux_am_static));
    }
    let termux_open_static = format!(
        "{}/bin/termux-open",
        crate::updater::artifact::TERMUX_PREFIX_USR
    );
    if Path::new(&termux_open_static).is_file() {
        push_unique(AndroidOpener::TermuxOpen(termux_open_static));
    }
    let termux_open_url_static = format!(
        "{}/bin/termux-open-url",
        crate::updater::artifact::TERMUX_PREFIX_USR
    );
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

pub fn android_openers() -> Vec<AndroidOpener> {
    probe_android_openers()
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
        .into_iter()
        .map(|opener| {
            let cmd = android_intent_command_for_opener(&opener, url, subtitle, headers);
            (opener, cmd)
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

/// yt-dlp format selector honouring a height cap. DASH and HLS manifests carry
/// every rendition, so this is the only place the quality preference can be
/// applied for them. The trailing fallback keeps a title playable when nothing
/// is at or below the cap.
pub fn ytdl_format_selector(max_height: Option<u64>) -> String {
    match max_height {
        Some(height) => format!(
            "bestvideo[height<={height}]+bestaudio/best[height<={height}]/bestvideo+bestaudio/best"
        ),
        None => "bestvideo+bestaudio/best".to_string(),
    }
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

    if !iina {
        command.arg("--idle=no").arg("--keep-open=no");
    }
    command.arg(format!(
        "{prefix}ytdl-format={}",
        ytdl_format_selector(max_height)
    ));
    command.arg(format!("{prefix}hls-bitrate=max"));
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

    if executable.starts_with("flatpak run ")
        && (url.starts_with('/') || url.starts_with("file://"))
    {
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
fn iina_resolution() -> Option<IinaResolution> {
    static CACHED: std::sync::RwLock<Option<IinaResolution>> = std::sync::RwLock::new(None);

    if let Ok(guard) = CACHED.read() {
        if let Some(res) = &*guard {
            return Some(res.clone());
        }
    }

    let detected = probe_iina_resolution();
    if let Some(res) = &detected {
        if let Ok(mut guard) = CACHED.write() {
            *guard = Some(res.clone());
        }
    }
    detected
}

#[cfg(target_os = "macos")]
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
    let resolution = iina_resolution();
    let mut command = match resolution {
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
        command.arg(arg);
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

#[cfg(not(target_os = "macos"))]
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
    mpv_command(
        url,
        subtitle,
        headers,
        false,
        window,
        resume_seconds,
        tracker,
        max_height,
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
    command.arg("--adaptive-logic=highest");
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
        let sub_path = normalize_player_path(subtitle);
        command.arg(format!("--sub-file={sub_path}"));
    }

    if executable.starts_with("flatpak run ")
        && (url.starts_with('/') || url.starts_with("file://"))
    {
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

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/mpv"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/mpv".to_string());
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
        candidates.push("/run/current-system/sw/bin/mpv".to_string());
        candidates.push("/usr/bin/mpv".to_string());
        candidates.push("/usr/local/bin/mpv".to_string());
        candidates.push("/bin/mpv".to_string());
        candidates.push("/app/bin/mpv".to_string());
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

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if let Ok(prefix) = std::env::var("PREFIX") {
            candidates.push(format!("{prefix}/bin/vlc"));
        }
        candidates.push("/data/data/com.termux/files/usr/bin/vlc".to_string());
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
        candidates.push("/run/current-system/sw/bin/vlc".to_string());
        candidates.push("/usr/bin/vlc".to_string());
        candidates.push("/usr/local/bin/vlc".to_string());
        candidates.push("/bin/vlc".to_string());
        candidates.push("/app/bin/vlc".to_string());
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

fn mpv_executable() -> Option<String> {
    static CACHED: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

    if let Ok(guard) = CACHED.read() {
        if let Some(path) = &*guard {
            if path.starts_with("flatpak run ") || Path::new(path).is_file() {
                return Some(path.clone());
            }
        }
    }

    let detected = probe_mpv();
    if let Some(path) = &detected {
        if let Ok(mut guard) = CACHED.write() {
            *guard = Some(path.clone());
        }
    }
    detected
}

fn vlc_executable() -> Option<String> {
    static CACHED: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

    if let Ok(guard) = CACHED.read() {
        if let Some(path) = &*guard {
            if path.starts_with("flatpak run ") || Path::new(path).is_file() {
                return Some(path.clone());
            }
        }
    }

    let detected = probe_vlc();
    if let Some(path) = &detected {
        if let Ok(mut guard) = CACHED.write() {
            *guard = Some(path.clone());
        }
    }
    detected
}

#[cfg(target_os = "macos")]
fn iina_available() -> bool {
    iina_resolution().is_some()
}

#[cfg(target_os = "macos")]
fn iina_cli_exists() -> bool {
    matches!(iina_resolution(), Some(IinaResolution::Cli(_)))
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

fn executable_on_path(name: &str) -> bool {
    find_in_path(name).is_some()
}

fn normalize_player_path(path: &str) -> String {
    if path.starts_with(r"\\") || path.starts_with("//") {
        path.to_string()
    } else {
        path.replace('\\', "/")
    }
}

pub fn format_mpv_script_opts(
    provider: &str,
    subject_id: &str,
    season: usize,
    episode: usize,
    state_file: &Path,
) -> String {
    let state_file_str = normalize_player_path(&state_file.to_string_lossy());
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
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.contains(&"--width=1280".into()));
        assert!(args.contains(&"--height=720".into()));
        assert!(args.contains(&"--play-and-exit".into()));
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
    fn vlc_command_normalizes_windows_subtitle_paths() {
        let command = vlc_command(
            "https://example.test/video.mp4",
            Some(r"C:\Users\User\AppData\Local\MovieBox-Tui\subs\sub.srt"),
            &[],
            None,
            None,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            args.contains(
                &"--sub-file=C:/Users/User/AppData/Local/MovieBox-Tui/subs/sub.srt".into()
            )
        );
    }
    #[test]
    fn vlc_command_preserves_unc_subtitle_paths() {
        let command = vlc_command(
            "https://example.test/video.mp4",
            Some(r"\\server\share\subs\sub.srt"),
            &[],
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
    #[test]
    fn test_android_intent_commands_fallback_order() {
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
    fn test_ytdl_format_selector_caps_height_with_fallback() {
        assert_eq!(
            ytdl_format_selector(None),
            "bestvideo+bestaudio/best",
            "no preference keeps the previous selector"
        );
        assert_eq!(
            ytdl_format_selector(Some(720)),
            "bestvideo[height<=720]+bestaudio/best[height<=720]/bestvideo+bestaudio/best"
        );
    }

    #[test]
    fn test_mpv_command_passes_capped_ytdl_format() {
        let headers: Vec<(String, String)> = Vec::new();
        let cmd = mpv_command(
            "https://example.com/index.mpd",
            None,
            &headers,
            false,
            None,
            None,
            None,
            Some(720),
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.iter().any(|a| a
                == "--ytdl-format=bestvideo[height<=720]+bestaudio/best[height<=720]/bestvideo+bestaudio/best"),
            "dash playback must carry the height cap, got {args:?}"
        );
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
}
