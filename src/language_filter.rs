//! Hide Hindi releases, Hindi audio tracks and Hindi catalog entries from every
//! client.
//!
//! Spec S2 of `odd/tasks/hindi-release-filter.md` hides release and dub/audio
//! options, so a Hollywood film with a Hindi dub keeps its original and English
//! versions and simply stops offering the Hindi one.
//!
//! Spec S6 of the same document adds the catalog level: the provider tags
//! Hindi-original content in the subject title itself (`[Hindi] X`, `X - Hindi`),
//! because `CatalogItem` carries no language field, so [`hindi_catalog_items`]
//! drops those entries from every listing page.
//!
//! Detection reuses [`crate::text_util::sanitize_language_label`], the language
//! table the subtitle picker already trusts, so the codes and spellings the
//! repository accepts (`Hindi`, `hin`, `hi`, `हिन्दी`) all resolve to the same
//! canonical name, and a longer label such as `Hindi Dubbed` is recognised
//! token by token instead of by substring: `Chinese` contains `hin`, `Hindi`
//! does not contain `Chinese`.

use crate::providers::models::{AudioTrackOption, CatalogItem, Release};
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

/// `true` when a raw catalog title carries the provider's Hindi marker.
///
/// Spec S6c: the marker is the marked form the provider writes, never the bare
/// word — a real title such as `Hindi Medium` is not Hindi content and must stay
/// in the listings, so only the two forms the repository already recognises
/// count:
///
/// * a bracketed tag anywhere in the title (`[Hindi] X`, `[hindi] X`,
///   `[Hindi Dub] X`), the marker the terminal client already scans for;
/// * a ` - <language>` suffix (`X - Hindi`), the tag
///   [`crate::providers::moviebox::clean_moviebox_title`] already strips.
///
/// Both are matched with [`is_hindi_language`], so the recognised spellings come
/// from the language table instead of a second ad-hoc list. Detection reads the
/// raw title, because cleaning it would erase the only signal (spec S6b).
pub fn is_hindi_title(title: &str) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Bracketed marker, anywhere in the title: `[Hindi] X`, `[hindi] X`,
    // `[Hindi Dub] X`.
    let mut rest = trimmed;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        if is_hindi_language(&after_open[..close]) {
            return true;
        }
        rest = &after_open[close + 1..];
    }
    // `X - Hindi`: the ` - <language>` suffix form `clean_moviebox_title`
    // already treats as a tag.
    if let Some(pos) = trimmed.rfind(" - ") {
        if is_hindi_language(&trimmed[pos + 3..]) {
            return true;
        }
    }
    false
}

/// Drop Hindi entries from one catalog listing page (spec S6b).
///
/// `hide_hindi` is passed in instead of read from the config so the rule is
/// testable without global state; callers pass
/// [`crate::config::Config::hide_hindi`]. A page is never emptied by the filter
/// (spec S4 in this document): when every entry is Hindi the page comes back
/// unchanged, so a pager that ends on an empty page keeps asking for the next one
/// instead of losing the rest of the list.
pub fn hindi_catalog_items(items: Vec<CatalogItem>, hide_hindi: bool) -> Vec<CatalogItem> {
    drop_hindi_when(items, hide_hindi, |item| is_hindi_title(&item.title))
}

/// Drop Hindi entries from a type-ahead suggestion list.
///
/// Suggestions are titles the provider offers as query completions, so they
/// carry the same marker. Same S4 guard as [`hindi_catalog_items`].
pub fn hindi_suggestions(suggestions: Vec<String>, hide_hindi: bool) -> Vec<String> {
    drop_hindi_when(suggestions, hide_hindi, |suggestion| {
        is_hindi_title(suggestion)
    })
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

/// Apply [`drop_hindi`] only while the viewer asked for it.
fn drop_hindi_when<T>(items: Vec<T>, hide_hindi: bool, is_hindi: impl Fn(&T) -> bool) -> Vec<T> {
    if hide_hindi {
        drop_hindi(items, is_hindi)
    } else {
        items
    }
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

    fn catalog_item(id: &str, title: &str) -> CatalogItem {
        CatalogItem {
            id: crate::providers::models::ProviderMediaId {
                provider: ProviderKind::MovieBox,
                value: id.to_string(),
            },
            title: title.to_string(),
            media_type: crate::providers::models::MediaType::Movie,
            year: Some("2025".to_string()),
            poster_url: None,
            season_count: None,
        }
    }

    fn titles(items: &[CatalogItem]) -> Vec<&str> {
        items.iter().map(|item| item.title.as_str()).collect()
    }

    #[test]
    fn hindi_title_predicate_accepts_the_provider_markers() {
        for title in [
            "[Hindi] Ek Deewane",
            "[hindi] X",
            "[Hindi Dub] X",
            "X - Hindi",
        ] {
            assert!(is_hindi_title(title), "{title} should be Hindi");
        }
    }

    #[test]
    fn hindi_title_predicate_rejects_the_bare_word_and_other_languages() {
        // S6c: `Hindi` as a word of a real title ("Hindi Medium") is not a
        // marker; only the marked form the provider writes is.
        for title in [
            "Hindi Medium",
            "[Tamil] X",
            "[Telugu] X",
            "Inception",
            "",
            "   ",
        ] {
            assert!(!is_hindi_title(title), "{title} should not be Hindi");
        }
    }

    #[test]
    fn hindi_catalog_items_drops_hindi_and_keeps_the_rest() {
        let filtered = hindi_catalog_items(
            vec![
                catalog_item("1", "[Hindi] Ek Deewane"),
                catalog_item("2", "Inception"),
                catalog_item("3", "Hindi Medium"),
                catalog_item("4", "Animal - Hindi"),
            ],
            true,
        );

        assert_eq!(titles(&filtered), ["Inception", "Hindi Medium"]);
    }

    #[test]
    fn hindi_catalog_items_keeps_the_page_when_every_item_is_hindi() {
        // S4: emptying the page would end a client's pagination and blank the
        // listing, so an all-Hindi page comes back unchanged.
        let page = vec![
            catalog_item("1", "[Hindi] Ek Deewane"),
            catalog_item("2", "[hindi] X"),
        ];
        assert_eq!(hindi_catalog_items(page.clone(), true), page);
        assert!(hindi_catalog_items(Vec::new(), true).is_empty());
    }

    #[test]
    fn hindi_catalog_items_is_a_no_op_when_hiding_is_off() {
        let page = vec![
            catalog_item("1", "[Hindi] Ek Deewane"),
            catalog_item("2", "Inception"),
        ];
        assert_eq!(hindi_catalog_items(page.clone(), false), page);
    }

    #[test]
    fn hindi_suggestions_drop_marked_terms_and_keep_the_page_guard() {
        assert_eq!(
            hindi_suggestions(
                vec!["[Hindi] Ek Deewane".to_string(), "Hindi Medium".to_string()],
                true,
            ),
            ["Hindi Medium"]
        );
        let all_hindi = vec!["[Hindi] Ek Deewane".to_string()];
        assert_eq!(hindi_suggestions(all_hindi.clone(), true), all_hindi);
        assert_eq!(
            hindi_suggestions(all_hindi.clone(), false),
            all_hindi,
            "the viewer turned hiding off"
        );
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
