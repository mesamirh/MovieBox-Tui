//! Text helpers shared by the terminal UI and non-terminal front ends.
//!
//! These functions used to live in [`crate::tui::text`], which pulls in
//! `crossterm` for its input-buffer type. Keeping them here lets the service
//! layer and the watch history compile without the terminal stack, which is what
//! makes an iOS/mobile build possible.

pub fn sanitize_language_label(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(*c as u32, 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED))
        .collect();

    match cleaned.trim() {
        "العربية" | "Arabic" | "ara" | "ar" => "Arabic".to_string(),
        "اردو" | "أردو" | "Urdu" | "urd" | "ur" => "Urdu".to_string(),
        "বাংলা" | "Bengali" | "ben" | "bn" => "Bengali".to_string(),
        "हिन्दी" | "हिंदी" | "Hindi" | "hin" | "hi" => "Hindi".to_string(),
        "Filipino" | "Tagalog" | "fil" | "tl" => "Filipino".to_string(),
        "Indonesian" | "ind" | "id" | "in_id" => "Indonesian".to_string(),
        "English" | "eng" | "en" => "English".to_string(),
        "Español" | "Spanish" | "spa" | "es" => "Spanish".to_string(),
        "Français" | "French" | "fra" | "fre" | "fr" => "French".to_string(),
        "Deutsch" | "German" | "deu" | "ger" | "de" => "German".to_string(),
        "Italiano" | "Italian" | "ita" | "it" => "Italian".to_string(),
        "Português" | "Portuguese" | "por" | "pt" => "Portuguese".to_string(),
        "Русский" | "Russian" | "rus" | "ru" => "Russian".to_string(),
        "Türkçe" | "Turkish" | "tur" | "tr" => "Turkish".to_string(),
        "Tiếng Việt" | "Vietnamese" | "vie" | "vi" => "Vietnamese".to_string(),
        "中文" | "Chinese" | "zho" | "chi" | "zh" => "Chinese".to_string(),
        "日本語" | "Japanese" | "jpn" | "ja" => "Japanese".to_string(),
        "한국어" | "Korean" | "kor" | "ko" => "Korean".to_string(),
        "ไทย" | "Thai" | "tha" | "th" => "Thai".to_string(),
        "தமிழ்" | "Tamil" | "tam" | "ta" => "Tamil".to_string(),
        "తెలుగు" | "Telugu" | "tel" | "te" => "Telugu".to_string(),
        "മലയാളം" | "Malayalam" | "mal" | "ml" => "Malayalam".to_string(),
        "ಕನ್ನಡ" | "Kannada" | "kan" | "kn" => "Kannada".to_string(),
        "मराठी" | "Marathi" | "mar" | "mr" => "Marathi".to_string(),
        "ગુજરાતી" | "Gujarati" | "guj" | "gu" => "Gujarati".to_string(),
        "ਪੰਜਾਬੀ" | "Punjabi" | "pan" | "pa" => "Punjabi".to_string(),
        "فارسی" | "Persian" | "fas" | "per" | "fa" => "Persian".to_string(),
        "עברית" | "Hebrew" | "heb" | "he" => "Hebrew".to_string(),
        "Ελληνικά" | "Greek" | "ell" | "gre" | "el" => "Greek".to_string(),
        "Polski" | "Polish" | "pol" | "pl" => "Polish".to_string(),
        "Nederlands" | "Dutch" | "nld" | "dut" | "nl" => "Dutch".to_string(),
        "Svenska" | "Swedish" | "swe" | "sv" => "Swedish".to_string(),
        "Norsk" | "Norwegian" | "nor" | "no" => "Norwegian".to_string(),
        "Dansk" | "Danish" | "dan" | "da" => "Danish".to_string(),
        "Suomi" | "Finnish" | "fin" | "fi" => "Finnish".to_string(),
        "Čeština" | "Czech" | "ces" | "cze" | "cs" => "Czech".to_string(),
        "Magyar" | "Hungarian" | "hun" | "hu" => "Hungarian".to_string(),
        "Română" | "Romanian" | "ron" | "rum" | "ro" => "Romanian".to_string(),
        "Українська" | "Ukrainian" | "ukr" | "uk" => "Ukrainian".to_string(),
        "" => "Unknown".to_string(),
        other => other.to_string(),
    }
}

pub fn format_subtitle_label(name: &str) -> String {
    if name == "None" {
        "No subtitles".to_string()
    } else {
        sanitize_language_label(name)
    }
}

pub fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn parse_duration_seconds(d: &str) -> Option<u64> {
    let s = d.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let m: u64 = parts[0].trim().parse().ok()?;
            let s: u64 = parts[1].trim().parse().ok()?;
            return Some(m * 60 + s);
        } else if parts.len() == 3 {
            let h: u64 = parts[0].trim().parse().ok()?;
            let m: u64 = parts[1].trim().parse().ok()?;
            let s: u64 = parts[2].trim().parse().ok()?;
            return Some(h * 3600 + m * 60 + s);
        }
    }
    let mut total = 0u64;
    let mut current_num = String::new();
    let mut found_any = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
        } else if c == 'h' || c == 'H' {
            if let Ok(n) = current_num.parse::<u64>() {
                total += n * 3600;
                found_any = true;
            }
            current_num.clear();
        } else if c == 'm' || c == 'M' {
            if let Ok(n) = current_num.parse::<u64>() {
                total += n * 60;
                found_any = true;
            }
            current_num.clear();
        } else if c == 's' || c == 'S' {
            if let Ok(n) = current_num.parse::<u64>() {
                total += n;
                found_any = true;
            }
            current_num.clear();
        }
    }
    if !current_num.is_empty() && !found_any {
        if let Ok(n) = current_num.parse::<u64>() {
            total += n * 60;
            found_any = true;
        }
    }
    if found_any && total > 0 {
        Some(total)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_and_parse_duration() {
        assert_eq!(format_duration(3665), "1:01:05");
        assert_eq!(format_duration(125), "2:05");
        assert_eq!(parse_duration_seconds("1:01:05"), Some(3665));
        assert_eq!(parse_duration_seconds("2h 15m"), Some(8100));
        assert_eq!(parse_duration_seconds("45m"), Some(2700));
        assert_eq!(parse_duration_seconds("N/A"), None);
        assert_eq!(parse_duration_seconds(""), None);
    }

    #[test]
    fn test_sanitize_language_label() {
        assert_eq!(sanitize_language_label("English"), "English");
        assert_eq!(sanitize_language_label("en"), "English");
        assert_eq!(sanitize_language_label(""), "Unknown");
        assert_eq!(format_subtitle_label("None"), "No subtitles");
    }
}
