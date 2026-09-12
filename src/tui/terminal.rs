fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

pub fn uses_basic_ui() -> bool {
    let term = env("TERM");
    if term == "dumb" || term == "linux" {
        return true;
    }
    crate::tui::theme::ColorSupport::current() == crate::tui::theme::ColorSupport::Basic
}

pub fn should_query_images() -> bool {
    if std::env::var("MOVIEBOX_NO_IMAGE").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        return false;
    }
    if let Ok(forced) = std::env::var("MOVIEBOX_IMAGE_PROTOCOL") {
        let forced = forced.trim();
        if forced.eq_ignore_ascii_case("none")
            || forced.eq_ignore_ascii_case("off")
            || forced.eq_ignore_ascii_case("false")
        {
            return false;
        }
        if !forced.is_empty() {
            return true;
        }
    }
    if std::env::var("TMUX").is_ok() && std::env::var("MOVIEBOX_IMAGE_PROTOCOL").is_err() {
        return false;
    }
    if std::env::var("TERM_PROGRAM").is_ok_and(|v| v == "Apple_Terminal") {
        return false;
    }
    let term = env("TERM");
    if term == "dumb"
        || term == "linux"
        || term == "cygwin"
        || term.starts_with("vt")
        || term.starts_with("cons")
    {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        let is_modern_terminal = std::env::var("WT_SESSION").is_ok()
            || std::env::var("TERM_PROGRAM").is_ok()
            || std::env::var("ALACRITTY_LOG").is_ok()
            || std::env::var("WEZTERM_EXECUTABLE").is_ok()
            || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok();
        if !is_modern_terminal {
            return false;
        }
    }
    true
}

pub fn background_is_light() -> bool {
    if let Ok(value) = std::env::var("COLORFGBG")
        && let Some(background) = value
            .split([';', ':'])
            .next_back()
            .and_then(|value| value.parse::<u8>().ok())
    {
        return matches!(background, 7 | 10..=15);
    }

    std::env::var("TERM_BACKGROUND")
        .or_else(|_| std::env::var("BACKGROUND"))
        .is_ok_and(|value| value.eq_ignore_ascii_case("light"))
}

pub fn set_window_title(title: &str) -> std::io::Result<()> {
    crossterm::execute!(std::io::stdout(), crossterm::terminal::SetTitle(title))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_query_images_guards() {
        unsafe {
            std::env::set_var("MOVIEBOX_NO_IMAGE", "1");
            assert!(!should_query_images());
            std::env::remove_var("MOVIEBOX_NO_IMAGE");

            std::env::set_var("MOVIEBOX_IMAGE_PROTOCOL", "none");
            assert!(!should_query_images());
            std::env::remove_var("MOVIEBOX_IMAGE_PROTOCOL");

            std::env::set_var("TERM_PROGRAM", "Apple_Terminal");
            assert!(!should_query_images());
            std::env::remove_var("TERM_PROGRAM");

            std::env::set_var("TERM", "dumb");
            assert!(!should_query_images());
            std::env::remove_var("TERM");
            std::env::set_var("MOVIEBOX_IMAGE_PROTOCOL", "kitty");
            assert!(should_query_images());
            std::env::remove_var("MOVIEBOX_IMAGE_PROTOCOL");
        }
    }
}
