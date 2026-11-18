pub fn clean_moviebox_title(raw_title: &str) -> &str {
    let mut title = raw_title.trim();
    if title.is_empty() {
        return "";
    }

    while title.starts_with('[') {
        if let Some(close_pos) = title.find(']') {
            let remainder = title[close_pos + 1..].trim();
            if !remainder.is_empty() {
                title = remainder;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if let Some(pos) = title.find('[') {
        if pos > 0 {
            title = title[..pos].trim();
        }
    }

    if let Some(pos) = title.find('(') {
        if pos > 0 {
            let inside = &title[pos + 1..];
            let inside_content = inside.split(')').next().unwrap_or("").trim();
            let is_year = inside_content.len() == 4
                && inside_content.chars().all(|c| c.is_ascii_digit())
                && inside_content
                    .parse::<u32>()
                    .is_ok_and(|y| (1900..=2099).contains(&y));
            if !is_year {
                title = title[..pos].trim();
            }
        }
    }

    if let Some(pos) = title.rfind(" - ") {
        let suffix = &title[pos + 3..];
        let is_tag = contains_ignore_ascii_case(suffix, "hindi")
            || contains_ignore_ascii_case(suffix, "tamil")
            || contains_ignore_ascii_case(suffix, "telugu")
            || contains_ignore_ascii_case(suffix, "kannada")
            || contains_ignore_ascii_case(suffix, "malayalam")
            || contains_ignore_ascii_case(suffix, "bengali")
            || contains_ignore_ascii_case(suffix, "marathi")
            || contains_ignore_ascii_case(suffix, "punjabi")
            || contains_ignore_ascii_case(suffix, "gujarati")
            || contains_ignore_ascii_case(suffix, "urdu")
            || contains_ignore_ascii_case(suffix, "english")
            || contains_ignore_ascii_case(suffix, "spanish")
            || contains_ignore_ascii_case(suffix, "french")
            || contains_ignore_ascii_case(suffix, "german")
            || contains_ignore_ascii_case(suffix, "italian")
            || contains_ignore_ascii_case(suffix, "japanese")
            || contains_ignore_ascii_case(suffix, "korean")
            || contains_ignore_ascii_case(suffix, "chinese")
            || contains_ignore_ascii_case(suffix, "russian")
            || contains_ignore_ascii_case(suffix, "portuguese")
            || contains_ignore_ascii_case(suffix, "turkish")
            || contains_ignore_ascii_case(suffix, "arabic")
            || contains_ignore_ascii_case(suffix, "dub")
            || contains_ignore_ascii_case(suffix, "audio")
            || contains_ignore_ascii_case(suffix, "multi")
            || contains_ignore_ascii_case(suffix, "season")
            || (suffix.chars().next().is_some_and(|c| c == 's' || c == 'S')
                && suffix[1..].chars().all(|c| c.is_ascii_digit() || c == '-'));
        if is_tag {
            title = title[..pos].trim();
        }
    }
    if let Some(s_idx) = title.rfind(" S") {
        let suffix = &title[s_idx + 2..];
        let is_season = suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == 'S');
        if is_season && suffix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            title = title[..s_idx].trim();
        }
    }

    if let Some(s_idx) = rfind_ignore_ascii_case(title, " season ") {
        title = title[..s_idx].trim();
    }

    for sep in ['_', ' ', '.', '-'] {
        if let Some(pos) = title.rfind(sep) {
            let suffix = &title[pos + 1..];
            let is_res = (suffix.ends_with('p') || suffix.ends_with('P'))
                && suffix.len() > 1
                && suffix[..suffix.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit())
                && suffix[..suffix.len() - 1]
                    .parse::<u32>()
                    .is_ok_and(|r| (144..=8640).contains(&r));
            if is_res {
                title = title[..pos].trim();
            }
        }
    }
    let cleaned = title.trim_end_matches(['-', ':', '_', '.', ' ']).trim();

    if cleaned.is_empty() {
        raw_title.trim()
    } else {
        cleaned
    }
}

/// Cleans a subject title for the online subtitle lookup and for the player display (S8).
///
/// [`clean_moviebox_title`] drops dub markers such as `[Hindi]` and ` - Hindi` but keeps a
/// release year in parentheses (`Inception (2010)` -> `Inception (2010)`), which the strict
/// normalized-title match in `providers::opensubtitles::pick_best_candidate` cannot equal,
/// so a trailing four-digit year is dropped here as well. A title with no marker and no
/// trailing year comes back byte-identical.
pub fn clean_subtitle_title(raw_title: &str) -> String {
    strip_trailing_parenthesised_year(clean_moviebox_title(raw_title)).to_string()
}

/// Drops a `(YYYY)` that ends the title, leaving any other parentheses alone.
fn strip_trailing_parenthesised_year(title: &str) -> &str {
    let trimmed = title.trim_end();
    if !trimmed.ends_with(')') {
        return title;
    }
    let Some(open_pos) = trimmed.rfind('(') else {
        return title;
    };
    let inside = trimmed[open_pos + 1..trimmed.len() - 1].trim();
    let is_year = inside.len() == 4
        && inside.chars().all(|character| character.is_ascii_digit())
        && inside
            .parse::<u32>()
            .is_ok_and(|year| (1900..=2099).contains(&year));
    if !is_year {
        return title;
    }
    let without_year = trimmed[..open_pos].trim();
    if without_year.is_empty() {
        title
    } else {
        without_year
    }
}

fn rfind_ignore_ascii_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }
    if haystack.len() < needle.len() {
        return None;
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    (0..=h.len() - n.len())
        .rev()
        .find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    rfind_ignore_ascii_case(haystack, needle).is_some()
}

pub fn language_to_code(name: &str) -> Option<&'static str> {
    let lower = name.trim().to_lowercase();
    match lower.as_str() {
        "english" | "en" | "eng" => Some("en"),
        "spanish" | "es" | "spa" | "español" | "castellano" => Some("es"),
        "hindi" | "hi" | "hin" => Some("hi"),
        "french" | "fr" | "fre" | "fra" | "français" => Some("fr"),
        "german" | "de" | "ger" | "deu" | "deutsch" => Some("de"),
        "italian" | "it" | "ita" | "italiano" => Some("it"),
        "japanese" | "ja" | "jpn" | "日本語" => Some("ja"),
        "korean" | "ko" | "kor" | "한국어" => Some("ko"),
        "chinese" | "zh" | "zho" | "chi" | "中文" | "mandarin" | "cantonese" => Some("zh"),
        "portuguese" | "pt" | "por" | "português" => Some("pt"),
        "russian" | "ru" | "rus" | "русский" => Some("ru"),
        "arabic" | "ar" | "ara" | "العربية" => Some("ar"),
        "turkish" | "tr" | "tur" | "türkçe" => Some("tr"),
        "bengali" | "bn" | "ben" | "বাংলা" => Some("bn"),
        "tamil" | "ta" | "tam" | "தமிழ்" => Some("ta"),
        "telugu" | "te" | "tel" | "తెలుగు" => Some("te"),
        "malayalam" | "ml" | "mal" | "മലയാളം" => Some("ml"),
        "kannada" | "kn" | "kan" | "ಕನ್ನಡ" => Some("kn"),
        "marathi" | "mr" | "mar" | "मराठी" => Some("mr"),
        "punjabi" | "pa" | "pan" | "ਪੰਜਾਬੀ" => Some("pa"),
        "gujarati" | "gu" | "guj" | "ગુજરાતી" => Some("gu"),
        "urdu" | "ur" | "urd" | "اردو" => Some("ur"),
        "indonesian" | "id" | "ind" | "bahasa" => Some("id"),
        "thai" | "th" | "tha" | "ไทย" => Some("th"),
        "vietnamese" | "vi" | "vie" | "tiếng việt" => Some("vi"),
        "dutch" | "nl" | "dut" | "nld" | "nederlands" => Some("nl"),
        "polish" | "pl" | "pol" | "polski" => Some("pl"),
        "swedish" | "sv" | "swe" | "svenska" => Some("sv"),
        "danish" | "da" | "dan" | "dansk" => Some("da"),
        "norwegian" | "no" | "nor" | "norsk" => Some("no"),
        "finnish" | "fi" | "fin" | "suomi" => Some("fi"),
        "greek" | "el" | "ell" | "gre" | "ελληνικά" => Some("el"),
        "hebrew" | "he" | "heb" | "עברית" => Some("he"),
        "czech" | "cs" | "cze" | "ces" | "čeština" => Some("cs"),
        "hungarian" | "hu" | "hun" | "magyar" => Some("hu"),
        "romanian" | "ro" | "rum" | "ron" | "română" => Some("ro"),
        "ukrainian" | "uk" | "ukr" | "українська" => Some("uk"),
        "persian" | "fa" | "fas" | "per" | "فارسی" => Some("fa"),
        "tagalog" | "tl" | "fil" | "filipino" => Some("tl"),
        "malay" | "ms" | "msa" | "may" | "melayu" => Some("ms"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_moviebox_title_leading_tags() {
        assert_eq!(
            clean_moviebox_title("[Dub] Attack on Titan"),
            "Attack on Titan"
        );
        assert_eq!(clean_moviebox_title("[1080p] Dune"), "Dune");
        assert_eq!(
            clean_moviebox_title("[RAW] 千と千尋の神隠し"),
            "千と千尋の神隠し"
        );
        assert_eq!(
            clean_moviebox_title("[Dub] [1080p] Solo Leveling"),
            "Solo Leveling"
        );
    }

    #[test]
    fn test_clean_moviebox_title_parentheses_and_years() {
        assert_eq!(
            clean_moviebox_title("(500) Days of Summer"),
            "(500) Days of Summer"
        );
        assert_eq!(clean_moviebox_title("Inception (2010)"), "Inception (2010)");
        assert_eq!(
            clean_moviebox_title("Movie Name (Director's Cut)"),
            "Movie Name"
        );
    }

    #[test]
    fn test_clean_moviebox_title_trailing_brackets_and_tags() {
        assert_eq!(clean_moviebox_title("Dune [2021]"), "Dune");
        assert_eq!(clean_moviebox_title("Movie - Hindi Dub"), "Movie");
        assert_eq!(clean_moviebox_title("Breaking Bad S01"), "Breaking Bad");
        assert_eq!(clean_moviebox_title("Dark Season 2"), "Dark");
        assert_eq!(
            clean_moviebox_title("Ek Deewane Ki Deewaniyat_360P"),
            "Ek Deewane Ki Deewaniyat"
        );
        assert_eq!(clean_moviebox_title("Interstellar_1080P"), "Interstellar");
    }
    #[test]
    fn test_clean_moviebox_title_empty_and_whitespace() {
        assert_eq!(clean_moviebox_title(""), "");
        assert_eq!(clean_moviebox_title("   "), "");
        assert_eq!(clean_moviebox_title("Interstellar"), "Interstellar");
    }

    #[test]
    fn test_clean_subtitle_title_drops_dub_tags_and_a_trailing_year() {
        // Spec S8: the online subtitle lookup and the iOS title use this form.
        assert_eq!(clean_subtitle_title("[Hindi] Animal"), "Animal");
        assert_eq!(clean_subtitle_title("Animal - Hindi"), "Animal");
        assert_eq!(clean_subtitle_title("Animal (2023)"), "Animal");
        assert_eq!(clean_subtitle_title("[Hindi] Animal (2023)"), "Animal");
        // A title with no marker and no trailing year comes back byte-identical.
        assert_eq!(clean_subtitle_title("Animal"), "Animal");
        // `clean_moviebox_title` only treats a language word as a tag when it is a
        // ` - `-separated suffix, so the leading word of the real title "Hindi Medium"
        // is not a marker and must survive untouched.
        assert_eq!(clean_subtitle_title("Hindi Medium"), "Hindi Medium");
        // Only a trailing four-digit year is stripped: a leading parenthesised number
        // is part of the title, and the year must be the last thing in it.
        assert_eq!(
            clean_subtitle_title("(500) Days of Summer"),
            "(500) Days of Summer"
        );
        assert_eq!(clean_subtitle_title("  "), "");
    }
}
