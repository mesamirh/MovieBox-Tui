use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn width(value: &str) -> usize {
    if value.is_ascii() {
        value.len()
    } else {
        UnicodeWidthStr::width(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInputBuffer {
    content: String,
    cursor: usize,
}

impl TextInputBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        let content = s.to_string();
        let cursor = content.graphemes(true).count();
        Self { content, cursor }
    }

    pub fn as_str(&self) -> &str {
        &self.content
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn len_graphemes(&self) -> usize {
        self.content.graphemes(true).count()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.len_graphemes());
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.cursor = self.len_graphemes();
    }

    pub fn graphemes(&self) -> Vec<&str> {
        self.content.graphemes(true).collect()
    }

    pub fn cursor_byte_offset(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }
        self.content
            .grapheme_indices(true)
            .nth(self.cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.content.len())
    }
    pub fn cursor_prefix_str(&self) -> &str {
        &self.content[..self.cursor_byte_offset()]
    }

    pub fn cursor_column_offset(&self) -> usize {
        width(self.cursor_prefix_str())
    }

    pub fn cursor_split_parts(&self) -> (&str, &str, &str) {
        let cursor_byte = self.cursor_byte_offset();
        let before = &self.content[..cursor_byte];
        if cursor_byte >= self.content.len() {
            (before, " ", "")
        } else {
            let mut indices = self.content[cursor_byte..].grapheme_indices(true);
            if let Some((_, grapheme)) = indices.next() {
                let after_offset = cursor_byte + grapheme.len();
                (before, grapheme, &self.content[after_offset..])
            } else {
                (before, " ", "")
            }
        }
    }

    pub fn insert(&mut self, c: char) {
        let offset = self.cursor_byte_offset();
        self.content.insert(offset, c);
        self.cursor = (self.cursor + 1).min(self.len_graphemes());
    }

    pub fn insert_str(&mut self, s: &str) {
        let offset = self.cursor_byte_offset();
        self.content.insert_str(offset, s);
        let s_graphemes = s.graphemes(true).count();
        self.cursor = (self.cursor + s_graphemes).min(self.len_graphemes());
    }

    pub fn delete_backwards(&mut self) -> bool {
        if self.cursor == 0 || self.content.is_empty() {
            return false;
        }
        let target_idx = self.cursor - 1;
        let mut indices = self.content.grapheme_indices(true);
        if let Some((start_byte, grapheme)) = indices.nth(target_idx) {
            let end_byte = start_byte + grapheme.len();
            self.content.replace_range(start_byte..end_byte, "");
            self.cursor = self.cursor.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn delete_forwards(&mut self) -> bool {
        if self.content.is_empty() || self.cursor >= self.len_graphemes() {
            return false;
        }
        let mut indices = self.content.grapheme_indices(true);
        if let Some((start_byte, grapheme)) = indices.nth(self.cursor) {
            let end_byte = start_byte + grapheme.len();
            self.content.replace_range(start_byte..end_byte, "");
            true
        } else {
            false
        }
    }

    pub fn delete_word_backwards(&mut self) -> bool {
        if self.cursor == 0 || self.content.is_empty() {
            return false;
        }
        let graphemes: Vec<&str> = self.graphemes();
        let old_cursor = self.cursor.min(graphemes.len());
        if old_cursor == 0 {
            return false;
        }

        let is_delim = |g: &str| -> bool {
            g.chars()
                .all(|c| c.is_whitespace() || matches!(c, '/' | '-' | '_' | ':' | '.' | '\\'))
        };

        let mut new_cursor = old_cursor;
        while new_cursor > 0 && is_delim(graphemes[new_cursor - 1]) {
            new_cursor -= 1;
        }
        while new_cursor > 0 && !is_delim(graphemes[new_cursor - 1]) {
            new_cursor -= 1;
        }
        if new_cursor == old_cursor {
            return false;
        }

        let start_byte = if new_cursor == 0 {
            0
        } else {
            self.content
                .grapheme_indices(true)
                .nth(new_cursor)
                .map(|(idx, _)| idx)
                .unwrap_or(self.content.len())
        };

        let end_byte = if old_cursor >= graphemes.len() {
            self.content.len()
        } else {
            self.content
                .grapheme_indices(true)
                .nth(old_cursor)
                .map(|(idx, _)| idx)
                .unwrap_or(self.content.len())
        };

        self.content.replace_range(start_byte..end_byte, "");
        self.cursor = new_cursor;
        true
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len_graphemes());
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.len_graphemes();
    }
}

impl From<&str> for TextInputBuffer {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<String> for TextInputBuffer {
    fn from(s: String) -> Self {
        let cursor = s.graphemes(true).count();
        Self { content: s, cursor }
    }
}

impl std::fmt::Display for TextInputBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

impl AsRef<str> for TextInputBuffer {
    fn as_ref(&self) -> &str {
        &self.content
    }
}

impl std::str::FromStr for TextInputBuffer {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_str(s))
    }
}

impl std::ops::Deref for TextInputBuffer {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl PartialEq<str> for TextInputBuffer {
    fn eq(&self, other: &str) -> bool {
        self.content == other
    }
}

impl PartialEq<&str> for TextInputBuffer {
    fn eq(&self, other: &&str) -> bool {
        self.content == *other
    }
}

impl PartialEq<String> for TextInputBuffer {
    fn eq(&self, other: &String) -> bool {
        self.content == *other
    }
}

impl PartialEq<TextInputBuffer> for &str {
    fn eq(&self, other: &TextInputBuffer) -> bool {
        *self == other.content
    }
}

impl PartialEq<TextInputBuffer> for String {
    fn eq(&self, other: &TextInputBuffer) -> bool {
        *self == other.content
    }
}

pub fn truncate_width<'a>(value: &'a str, max_width: usize) -> Cow<'a, str> {
    if value.is_ascii() {
        if value.len() <= max_width {
            return Cow::Borrowed(value);
        }
        if max_width <= 3 {
            return Cow::Owned(".".repeat(max_width));
        }
        let content_width = max_width - 3;
        let mut output = String::with_capacity(content_width + 3);
        output.push_str(&value[..content_width]);
        output.push_str("...");
        return Cow::Owned(output);
    }

    if width(value) <= max_width {
        return Cow::Borrowed(value);
    }
    if max_width <= 3 {
        return Cow::Owned(".".repeat(max_width));
    }

    let content_width = max_width - 3;
    let mut cut_byte = 0;
    let mut used = 0;
    for (offset, grapheme) in value.grapheme_indices(true) {
        let grapheme_width = width(grapheme);
        if used + grapheme_width > content_width {
            break;
        }
        used += grapheme_width;
        cut_byte = offset + grapheme.len();
    }
    let mut output = String::with_capacity(cut_byte + 3);
    output.push_str(&value[..cut_byte]);
    output.push_str("...");
    Cow::Owned(output)
}

pub fn truncate_middle_width(value: &str, max_width: usize) -> String {
    if width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let content_width = max_width - 1;
    let start_width = content_width.div_ceil(2);
    let end_width = content_width - start_width;

    if value.is_ascii() {
        let front = &value[..start_width];
        let rear = &value[value.len() - end_width..];
        let mut output = String::with_capacity(front.len() + 3 + rear.len());
        output.push_str(front);
        output.push('…');
        output.push_str(rear);
        return output;
    }

    let mut start_cut = 0;
    let mut used = 0;
    for (offset, grapheme) in value.grapheme_indices(true) {
        let grapheme_width = width(grapheme);
        if used + grapheme_width > start_width {
            break;
        }
        used += grapheme_width;
        start_cut = offset + grapheme.len();
    }

    let mut end_cut = value.len();
    used = 0;
    for (offset, grapheme) in value.grapheme_indices(true).rev() {
        let grapheme_width = width(grapheme);
        if used + grapheme_width > end_width {
            break;
        }
        used += grapheme_width;
        end_cut = offset;
    }

    let front = &value[..start_cut];
    let rear = &value[end_cut..];
    let mut output = String::with_capacity(front.len() + 3 + rear.len());
    output.push_str(front);
    output.push('…');
    output.push_str(rear);
    output
}

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

pub fn strip_emojis(input: &str) -> String {
    input
        .chars()
        .filter(|&c| {
            let u = c as u32;
            !((0x1F000..=0x1FAFF).contains(&u)
                || (0x2600..=0x27BF).contains(&u)
                || (0x2300..=0x23FF).contains(&u)
                || (0x2B00..=0x2BFF).contains(&u)
                || (0xFE00..=0xFE0F).contains(&u)
                || u == 0x200D)
        })
        .collect::<String>()
}

pub fn clean_stream_text(input: &str) -> String {
    let without_emojis = strip_emojis(input);
    let mut cleaned = String::new();
    let mut last_was_space = false;
    for c in without_emojis.chars() {
        if c.is_whitespace() {
            if !last_was_space && !cleaned.is_empty() {
                cleaned.push(' ');
                last_was_space = true;
            }
        } else {
            cleaned.push(c);
            last_was_space = false;
        }
    }
    cleaned.trim().to_string()
}

pub const CTRL_S_STR: &str = if cfg!(target_os = "macos") {
    "^S"
} else {
    "Ctrl+S"
};
pub const CTRL_T_STR: &str = if cfg!(target_os = "macos") {
    "^T"
} else {
    "Ctrl+T"
};
pub const CTRL_P_STR: &str = if cfg!(target_os = "macos") {
    "^P"
} else {
    "Ctrl+P"
};

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut current_line = String::new();
        let mut current_len = 0;

        for word in trimmed.split_whitespace() {
            let word_len = width(word);
            if current_len == 0 {
                if word_len > max_width {
                    lines.push(truncate_width(word, max_width).into_owned());
                } else {
                    current_line.push_str(word);
                    current_len = word_len;
                }
            } else if current_len + 1 + word_len <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
                current_len += 1 + word_len;
            } else {
                lines.push(current_line);
                if word_len > max_width {
                    lines.push(truncate_width(word, max_width).into_owned());
                    current_line = String::new();
                    current_len = 0;
                } else {
                    current_line = word.to_string();
                    current_len = word_len;
                }
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}

pub use crate::net::is_http_url;
pub fn extract_4digit_year(raw: &str) -> String {
    raw.as_bytes()
        .windows(4)
        .find(|window| window.iter().all(u8::is_ascii_digit) && matches!(window[0], b'1' | b'2'))
        .and_then(|window| std::str::from_utf8(window).ok())
        .map(str::to_string)
        .unwrap_or_default()
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

pub fn format_file_size(bytes: f64) -> String {
    let mb = bytes / 1024.0 / 1024.0;
    if mb >= 1024.0 {
        format!("{:.1}GB", mb / 1024.0)
    } else {
        format!("{:.0}MB", mb)
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
    fn test_text_input_buffer_basic_ascii() {
        let mut buf = TextInputBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len_graphemes(), 0);
        assert_eq!(buf.cursor(), 0);
        assert_eq!(buf.as_str(), "");

        buf.insert('a');
        buf.insert('b');
        buf.insert('c');
        assert_eq!(buf.as_str(), "abc");
        assert_eq!(buf.cursor(), 3);
        assert_eq!(buf.len_graphemes(), 3);

        buf.move_left();
        assert_eq!(buf.cursor(), 2);
        buf.insert('X');
        assert_eq!(buf.as_str(), "abXc");
        assert_eq!(buf.cursor(), 3);

        buf.move_home();
        assert_eq!(buf.cursor(), 0);
        buf.insert('Z');
        assert_eq!(buf.as_str(), "ZabXc");
        assert_eq!(buf.cursor(), 1);

        buf.move_end();
        assert_eq!(buf.cursor(), 5);
    }

    #[test]
    fn test_text_input_buffer_unicode_multibyte() {
        let mut buf = TextInputBuffer::from_str("café");
        assert_eq!(buf.len_graphemes(), 4);
        assert_eq!(buf.cursor(), 4);

        buf.insert_str(" ☕ 日本語");
        assert_eq!(buf.as_str(), "café ☕ 日本語");
        assert_eq!(buf.len_graphemes(), 10);
        assert_eq!(buf.cursor(), 10);

        buf.move_left();
        assert_eq!(buf.cursor(), 9);
        assert!(buf.delete_forwards());
        assert_eq!(buf.as_str(), "café ☕ 日本");
        assert_eq!(buf.cursor(), 9);

        buf.move_left();
        assert_eq!(buf.cursor(), 8);
        assert!(buf.delete_backwards());
        assert_eq!(buf.as_str(), "café ☕ 本");
        assert_eq!(buf.cursor(), 7);
    }

    #[test]
    fn test_text_input_buffer_emojis() {
        let mut buf = TextInputBuffer::new();
        buf.insert_str("🦀🚀");
        assert_eq!(buf.len_graphemes(), 2);
        assert_eq!(buf.cursor(), 2);

        let family = "👨‍👩‍👧‍👦";
        buf.insert_str(family);
        assert_eq!(buf.len_graphemes(), 3);
        assert_eq!(buf.cursor(), 3);

        assert!(buf.delete_backwards());
        assert_eq!(buf.as_str(), "🦀🚀");
        assert_eq!(buf.cursor(), 2);

        buf.move_home();
        assert_eq!(buf.cursor(), 0);
        assert!(buf.delete_forwards());
        assert_eq!(buf.as_str(), "🚀");
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn test_text_input_buffer_cursor_boundaries_and_clamping() {
        let mut buf = TextInputBuffer::from_str("movie");
        assert_eq!(buf.cursor(), 5);

        buf.move_right();
        assert_eq!(buf.cursor(), 5);

        buf.move_home();
        assert_eq!(buf.cursor(), 0);
        buf.move_left();
        assert_eq!(buf.cursor(), 0);

        buf.set_cursor(100);
        assert_eq!(buf.cursor(), 5);

        buf.set_cursor(2);
        assert_eq!(buf.cursor(), 2);

        buf.set_content("tv");
        assert_eq!(buf.as_str(), "tv");
        assert_eq!(buf.cursor(), 2);

        buf.set_cursor(2);
        buf.set_content("x");
        assert_eq!(buf.cursor(), 1);
    }

    #[test]
    fn test_text_input_buffer_backwards_and_forwards_deletion() {
        let mut buf = TextInputBuffer::from_str("abc");
        buf.move_home();
        assert!(!buf.delete_backwards());
        assert_eq!(buf.as_str(), "abc");

        buf.move_end();
        assert!(!buf.delete_forwards());
        assert_eq!(buf.as_str(), "abc");
        buf.set_cursor(1);
        assert!(buf.delete_backwards());
        assert_eq!(buf.as_str(), "bc");
        assert_eq!(buf.cursor(), 0);

        assert!(buf.delete_forwards());
        assert_eq!(buf.as_str(), "c");
        assert_eq!(buf.cursor(), 0);

        assert!(buf.delete_forwards());
        assert_eq!(buf.as_str(), "");
        assert!(buf.is_empty());
        assert!(!buf.delete_forwards());
        assert!(!buf.delete_backwards());
    }

    #[test]
    fn test_text_input_buffer_word_deletion_backwards() {
        let mut buf = TextInputBuffer::from_str("https://example.com/path/file.m3u");
        assert!(buf.delete_word_backwards());
        assert_eq!(buf.as_str(), "https://example.com/path/file.");
        assert_eq!(buf.cursor(), 30);

        assert!(buf.delete_word_backwards());
        assert_eq!(buf.as_str(), "https://example.com/path/");

        assert!(buf.delete_word_backwards());
        assert_eq!(buf.as_str(), "https://example.com/");

        let mut buf2 = TextInputBuffer::from_str("foo - bar_baz:test\\file.txt");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "foo - bar_baz:test\\file.");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "foo - bar_baz:test\\");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "foo - bar_baz:");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "foo - bar_");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "foo - ");
        assert!(buf2.delete_word_backwards());
        assert_eq!(buf2.as_str(), "");
        assert!(!buf2.delete_word_backwards());

        let mut buf3 = TextInputBuffer::from_str("hello   world   ");
        assert!(buf3.delete_word_backwards());
        assert_eq!(buf3.as_str(), "hello   ");
        assert!(buf3.delete_word_backwards());
        assert_eq!(buf3.as_str(), "");
    }

    #[test]
    fn test_text_input_buffer_clear() {
        let mut buf = TextInputBuffer::from_str("some text");
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.cursor(), 0);
        assert_eq!(buf.as_str(), "");
    }
    #[test]
    fn test_extract_4digit_year() {
        assert_eq!(extract_4digit_year("2024"), "2024");
        assert_eq!(extract_4digit_year("Movie 2 (2024)"), "2024");
        assert_eq!(extract_4digit_year("Classic Film (1999)"), "1999");
        assert_eq!(extract_4digit_year("2024-05-12"), "2024");
        assert_eq!(extract_4digit_year("No year here"), "");
        assert_eq!(extract_4digit_year("Not A Year (20x4)"), "");
    }
    #[test]
    fn test_text_input_buffer_cursor_helpers() {
        let mut buf = TextInputBuffer::from_str("hello");
        assert_eq!(buf.cursor_prefix_str(), "hello");
        assert_eq!(buf.cursor_column_offset(), 5);
        assert_eq!(buf.cursor_split_parts(), ("hello", " ", ""));

        buf.set_cursor(2);
        assert_eq!(buf.cursor_prefix_str(), "he");
        assert_eq!(buf.cursor_column_offset(), 2);
        assert_eq!(buf.cursor_split_parts(), ("he", "l", "lo"));

        buf.set_cursor(0);
        assert_eq!(buf.cursor_prefix_str(), "");
        assert_eq!(buf.cursor_column_offset(), 0);
        assert_eq!(buf.cursor_split_parts(), ("", "h", "ello"));

        let mut cjk_buf = TextInputBuffer::from_str("电影");
        assert_eq!(cjk_buf.cursor_prefix_str(), "电影");
        assert_eq!(cjk_buf.cursor_column_offset(), 4);
        cjk_buf.set_cursor(1);
        assert_eq!(cjk_buf.cursor_prefix_str(), "电");
        assert_eq!(cjk_buf.cursor_column_offset(), 2);
        assert_eq!(cjk_buf.cursor_split_parts(), ("电", "影", ""));
    }
    #[test]
    fn test_format_subtitle_label() {
        assert_eq!(format_subtitle_label("None"), "No subtitles");
        assert_eq!(format_subtitle_label("eng"), "English");
        assert_eq!(format_subtitle_label("Spanish"), "Spanish");
    }
    #[test]
    fn test_truncate_width_and_simd_ascii_zero_alloc() {
        let ascii_fit = "MovieBox";
        match truncate_width(ascii_fit, 10) {
            Cow::Borrowed(b) => assert_eq!(b, "MovieBox"),
            Cow::Owned(_) => panic!("expected borrowed Cow for fitting ascii string"),
        }
        let ascii_exact = "MovieBox";
        match truncate_width(ascii_exact, 8) {
            Cow::Borrowed(b) => assert_eq!(b, "MovieBox"),
            Cow::Owned(_) => panic!("expected borrowed Cow for exact ascii string"),
        }
        assert_eq!(truncate_width("MovieBox-Tui Terminal", 11), "MovieBox...");
        assert_eq!(truncate_width("Short", 3), "...");
        assert_eq!(truncate_width("Short", 2), "..");
        assert_eq!(truncate_width("Short", 1), ".");
        assert_eq!(truncate_width("Short", 0), "");

        let cjk = "电影院线上映";
        assert_eq!(width(cjk), 12);
        match truncate_width(cjk, 12) {
            Cow::Borrowed(b) => assert_eq!(b, "电影院线上映"),
            Cow::Owned(_) => panic!("expected borrowed Cow for fitting non-ascii string"),
        }
        assert_eq!(truncate_width(cjk, 8), "电影...");

        let mixed = "Inception 2 (2010)";
        assert_eq!(truncate_middle_width(mixed, 18), "Inception 2 (2010)");
        assert_eq!(truncate_middle_width(mixed, 10), "Incep…010)");
        assert_eq!(truncate_middle_width("Short", 2), "..");
        assert_eq!(truncate_middle_width("Short", 0), "");

        assert_eq!(width(""), 0);
        assert_eq!(width("abc 123"), 7);
        assert_eq!(width("🦀"), 2);
    }
}
