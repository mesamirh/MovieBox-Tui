fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

pub fn uses_basic_ui() -> bool {
    let term = env("TERM");

    term == "dumb" || term == "linux"
}

pub fn should_query_images() -> bool {
    let term = env("TERM");

    term != "dumb" && term != "linux"
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

    std::env::var("TERM_BACKGROUND").is_ok_and(|value| value.eq_ignore_ascii_case("light"))
}
