//! Hide Hindi releases and Hindi audio tracks from every client.
//!
//! Spec S2 of `odd/tasks/hindi-release-filter.md` hides only release and
//! dub/audio options: catalog titles, search results and details stay visible,
//! so a Hollywood film with a Hindi dub keeps its original and English versions
//! and simply stops offering the Hindi one.
//!
//! Detection reuses [`crate::text_util::sanitize_language_label`], the language
//! table the subtitle picker already trusts, so the codes and spellings the
//! repository accepts (`Hindi`, `hin`, `hi`, `हिन्दी`) all resolve to the same
//! canonical name, and a longer label such as `Hindi Dubbed` is recognised
//! token by token instead of by substring: `Chinese` contains `hin`, `Hindi`
//! does not contain `Chinese`.

use crate::providers::models::{AudioTrackOption, Release};
use crate::text_util::sanitize_language_label;

/// Canonical spelling produced by [`sanitize_language_label`] for this language.
const HINDI: &str = "Hindi";

/// `true` when `label` names Hindi, in any spelling the language table accepts.
pub fn is_hindi_language(label: &str) -> bool {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return false;
    }
    // A label that is exactly a known spelling ("Hindi", "hin", "हिन्दी").
    if is_hindi_token(trimmed) {
        return true;
    }
    // A longer label that embeds it ("Hindi Dubbed", "Hindi, English").
    trimmed
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(is_hindi_token)
}

/// Compare one word against the canonical name the language table produces.
///
/// [`sanitize_language_label`] is case-sensitive, so `hindi` and `HINDI` come
/// back unchanged; comparing to the canonical [`HINDI`] name case-insensitively
/// keeps one spelling source instead of a second list of variants.
fn is_hindi_token(token: &str) -> bool {
    sanitize_language_label(token).eq_ignore_ascii_case(HINDI)
}

/// `true` when the release's detected language is Hindi.
///
/// The provider parsers fill [`Release::language`] from the filename and page
/// metadata they already parse, so this never guesses from the filename.
pub fn release_is_hindi(release: &Release) -> bool {
    release.language.as_deref().is_some_and(is_hindi_language)
}

/// `true` when the audio track is offered in Hindi.
pub fn dub_is_hindi(dub: &AudioTrackOption) -> bool {
    is_hindi_language(&dub.language) || is_hindi_language(&dub.label)
}

/// Remove every Hindi release, keeping the original list when that would leave
/// nothing to play (spec S4).
pub fn filter_releases(releases: Vec<Release>) -> Vec<Release> {
    drop_hindi(releases, release_is_hindi)
}

/// Remove every Hindi audio track, keeping the original list when that would
/// leave nothing to play (spec S4).
pub fn filter_dubs(dubs: Vec<AudioTrackOption>) -> Vec<AudioTrackOption> {
    drop_hindi(dubs, dub_is_hindi)
}

/// Partition out the Hindi entries, then fall back to the full list when
/// filtering would empty a non-empty one.
///
/// Hiding the only playable option would turn the preference into a broken
/// screen, so a title that only exists in Hindi stays playable.
fn drop_hindi<T>(items: Vec<T>, is_hindi: impl Fn(&T) -> bool) -> Vec<T> {
    let (hindi, kept): (Vec<T>, Vec<T>) = items.into_iter().partition(|item| is_hindi(item));
    if kept.is_empty() { hindi } else { kept }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::models::ProviderKind;

    fn release(language: Option<&str>) -> Release {
        Release {
            provider: ProviderKind::FourKHdHub,
            filename: "Obsession (2025) 1080p BluRay AVC".to_string(),
            quality: Some("1080p".to_string()),
            codec: Some("AVC".to_string()),
            language: language.map(str::to_string),
            size_bytes: None,
            season: None,
            episode: None,
            mirrors: vec![],
            resource_id: None,
        }
    }

    fn dub(subject_id: &str, language: &str) -> AudioTrackOption {
        AudioTrackOption {
            subject_id: subject_id.to_string(),
            language: language.to_string(),
            label: language.to_string(),
        }
    }

    #[test]
    fn hindi_predicate_accepts_every_known_spelling() {
        for label in ["Hindi", "hindi", "HINDI", "hin", "Hindi Dubbed", "हिन्दी"] {
            assert!(is_hindi_language(label), "{label} should be Hindi");
        }
    }

    #[test]
    fn hindi_predicate_rejects_other_languages() {
        for label in ["English", "Tamil", "Telugu", "Spanish", "", "Multi Audio"] {
            assert!(!is_hindi_language(label), "{label} should not be Hindi");
        }
        // A release with no detected language is never assumed to be Hindi.
        assert_eq!(filter_releases(vec![release(None)]).len(), 1);
    }

    #[test]
    fn filter_releases_drops_hindi_and_keeps_the_rest() {
        let filtered = filter_releases(vec![
            release(Some("Hindi")),
            release(Some("English")),
            release(None),
            release(Some("Tamil")),
        ]);
        let languages: Vec<Option<&str>> = filtered
            .iter()
            .map(|release| release.language.as_deref())
            .collect();
        assert_eq!(languages, vec![Some("English"), None, Some("Tamil")]);
    }

    #[test]
    fn filter_releases_keeps_hindi_when_it_is_the_only_option() {
        // S4: hiding the only playable release would break the title.
        let all_hindi = vec![release(Some("Hindi")), release(Some("hindi"))];
        assert_eq!(filter_releases(all_hindi.clone()), all_hindi);

        let single = vec![release(Some("Hindi Dubbed"))];
        assert_eq!(filter_releases(single.clone()), single);

        assert!(filter_releases(Vec::new()).is_empty());
    }

    #[test]
    fn filter_dubs_drops_hindi_and_keeps_the_rest() {
        let filtered = filter_dubs(vec![
            dub("main", "Original"),
            dub("hi", "Hindi"),
            dub("es", "Spanish"),
        ]);
        assert_eq!(
            filtered,
            vec![dub("main", "Original"), dub("es", "Spanish")]
        );
    }

    #[test]
    fn filter_dubs_keeps_hindi_when_it_is_the_only_option() {
        let all_hindi = vec![dub("hi", "Hindi"), dub("hi2", "Hindi dub")];
        assert_eq!(filter_dubs(all_hindi.clone()), all_hindi);

        let single = vec![dub("hi", "Hindi")];
        assert_eq!(filter_dubs(single.clone()), single);

        assert!(filter_dubs(Vec::new()).is_empty());
    }
}
