use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use crate::providers::models::ProviderKind;
use crate::tui::theme::Theme;

pub fn resolution_label(resolution: i64) -> &'static str {
    match resolution {
        -1 => "Multi",
        2160 | 4320 => "4K",
        1080 => "1080p",
        720 => "720p",
        480 | 540 | 576 => "480p",
        360 => "360p",
        _ if resolution > 0 => "HD",
        _ => "SD",
    }
}

pub fn resolution_badge_spans<'a>(
    resolution: i64,
    theme: &'a Theme,
    basic_terminal: bool,
    modal_active: bool,
    is_selected: bool,
) -> Vec<Span<'a>> {
    if modal_active {
        if basic_terminal {
            let label = match resolution {
                -1 => "[Multi]",
                2160 | 4320 => "[4K]",
                1080 => "[1080p]",
                720 => "[720p]",
                480 | 540 | 576 => "[480p]",
                360 => "[360p]",
                _ if resolution > 0 => "[HD]",
                _ => "[SD]",
            };
            return vec![
                Span::styled(format!("{:<7}", label), theme.muted),
                Span::raw(" "),
            ];
        }

        let label = match resolution {
            -1 => " Multi ",
            2160 | 4320 => "  4K   ",
            1080 => " 1080p ",
            720 => " 720p  ",
            480 | 540 | 576 => " 480p  ",
            360 => " 360p  ",
            _ if resolution > 0 => "  HD   ",
            _ => "  SD   ",
        };
        let badge_bg = theme.surface0_color();
        let contrast_fg = theme.overlay1.fg.unwrap_or(theme.base);
        return vec![
            Span::styled(label, Style::default().bg(badge_bg).fg(contrast_fg)),
            Span::raw(" "),
        ];
    }

    if basic_terminal {
        let (label, style) = match resolution {
            -1 => ("[Multi]", theme.lavender.add_modifier(Modifier::BOLD)),
            2160 | 4320 => ("[4K]", theme.rating.add_modifier(Modifier::BOLD)),
            1080 => ("[1080p]", theme.highlight.add_modifier(Modifier::BOLD)),
            720 => ("[720p]", theme.teal.add_modifier(Modifier::BOLD)),
            480 | 540 | 576 => ("[480p]", theme.text_dim.add_modifier(Modifier::BOLD)),
            360 => ("[360p]", theme.text_dim.add_modifier(Modifier::BOLD)),
            _ if resolution > 0 => ("[HD]", theme.text.add_modifier(Modifier::BOLD)),
            _ => ("[SD]", theme.text_dim),
        };
        return vec![Span::styled(format!("{:<7}", label), style), Span::raw(" ")];
    }

    let (badge_bg, contrast_fg, label) = match resolution {
        -1 => (
            if is_selected {
                theme.lavender.fg.unwrap_or(theme.base)
            } else {
                theme.surface1_color()
            },
            if is_selected {
                theme.crust_color()
            } else {
                theme.lavender.fg.unwrap_or(theme.base)
            },
            " Multi ",
        ),
        2160 | 4320 => (
            if is_selected {
                theme.rating.fg.unwrap_or(theme.base)
            } else {
                theme.surface1_color()
            },
            if is_selected {
                theme.crust_color()
            } else {
                theme.rating.fg.unwrap_or(theme.base)
            },
            "  4K   ",
        ),
        1080 => (
            if is_selected {
                theme.sapphire.fg.unwrap_or(theme.base)
            } else {
                theme.surface1_color()
            },
            if is_selected {
                theme.crust_color()
            } else {
                theme.sapphire.fg.unwrap_or(theme.base)
            },
            " 1080p ",
        ),
        720 => (
            if is_selected {
                theme.teal.fg.unwrap_or(theme.base)
            } else {
                theme.surface1_color()
            },
            if is_selected {
                theme.crust_color()
            } else {
                theme.teal.fg.unwrap_or(theme.base)
            },
            " 720p  ",
        ),
        480 | 540 | 576 => (
            theme.surface2_color(),
            theme.text.fg.unwrap_or(theme.base),
            " 480p  ",
        ),
        360 => (
            theme.surface2_color(),
            theme.text.fg.unwrap_or(theme.base),
            " 360p  ",
        ),
        _ if resolution > 0 => (
            theme.surface2_color(),
            theme.text.fg.unwrap_or(theme.base),
            "  HD   ",
        ),
        _ => (
            theme.surface2_color(),
            theme.text_dim.fg.unwrap_or(theme.base),
            "  SD   ",
        ),
    };

    vec![
        Span::styled(
            label,
            Style::default()
                .bg(badge_bg)
                .fg(contrast_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]
}

pub fn provider_origin_tag(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::MovieBox => "[MovieBox]",
        ProviderKind::FourKHdHub => "[4KHD]",
        ProviderKind::BdixCircleFtp => "[CircleFTP]",
        ProviderKind::BdixDhakaFlix => "[DhakaFlix]",
        ProviderKind::Addons => "[Addon]",
    }
}

pub fn provider_badge_span<'a>(
    provider: ProviderKind,
    theme: &'a Theme,
    basic_terminal: bool,
    modal_active: bool,
) -> Span<'a> {
    let tag = provider_origin_tag(provider);
    if modal_active {
        Span::styled(tag, theme.muted)
    } else if basic_terminal {
        Span::styled(tag, theme.text_dim)
    } else {
        let style = match provider {
            ProviderKind::MovieBox => theme.lavender,
            ProviderKind::FourKHdHub => theme.rating,
            ProviderKind::BdixCircleFtp => theme.teal,
            ProviderKind::BdixDhakaFlix => theme.sapphire,
            ProviderKind::Addons => theme.accent,
        };
        Span::styled(tag, style)
    }
}

pub fn extract_resolution(title: &str, quality: Option<&str>) -> Option<i64> {
    if let Some(q) = quality {
        let q_lower = q.trim().to_ascii_lowercase();
        if q_lower.contains("2160") || q_lower.contains("4k") || q_lower.contains("uhd") {
            return Some(2160);
        } else if q_lower.contains("1080") || q_lower.contains("fhd") {
            return Some(1080);
        } else if q_lower.contains("720") || q_lower.contains("hd") {
            return Some(720);
        } else if q_lower.contains("480") || q_lower.contains("sd") {
            return Some(480);
        } else if q_lower.contains("576") {
            return Some(576);
        } else if q_lower.contains("360") {
            return Some(360);
        }
    }

    let title_lower = title.to_ascii_lowercase();
    if title_lower.contains("2160p")
        || title_lower.contains("2160")
        || title_lower.contains("4k")
        || title_lower.contains("uhd")
    {
        Some(2160)
    } else if title_lower.contains("1080p")
        || title_lower.contains("1080")
        || title_lower.contains("fhd")
    {
        Some(1080)
    } else if title_lower.contains("720p") || title_lower.contains("720") {
        Some(720)
    } else if title_lower.contains("480p") || title_lower.contains("480") {
        Some(480)
    } else if title_lower.contains("576p") || title_lower.contains("576") {
        Some(576)
    } else if title_lower.contains("360p") || title_lower.contains("360") {
        Some(360)
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaTags {
    pub hdr: Option<&'static str>,
    pub audio: Option<&'static str>,
    pub codec: Option<&'static str>,
    pub source: Option<&'static str>,
}

pub fn extract_media_tags(title: &str, codec_name: &str) -> MediaTags {
    let lower_title = title.to_ascii_lowercase();
    let lower_codec = codec_name.to_ascii_lowercase();

    let hdr = if lower_title.contains("hdr10+") || lower_title.contains("hdr10plus") {
        Some("HDR10+")
    } else if lower_title.contains("dovi")
        || lower_title.contains("dolby vision")
        || lower_title.contains("dolbyvision")
        || lower_title.contains(".dv.")
        || lower_title.contains(" dv ")
        || lower_title.contains("-dv")
    {
        Some("DV")
    } else if lower_title.contains("hdr") {
        Some("HDR")
    } else {
        None
    };

    let audio = if lower_title.contains("atmos") {
        Some("ATMOS")
    } else if lower_title.contains("7.1") {
        Some("7.1")
    } else if lower_title.contains("5.1")
        || lower_title.contains("ddp5.1")
        || lower_title.contains("dd5.1")
        || lower_title.contains("ac3")
    {
        Some("5.1")
    } else {
        None
    };

    let codec = if lower_codec.contains("hevc")
        || lower_codec.contains("x265")
        || lower_codec.contains("h265")
        || lower_title.contains("hevc")
        || lower_title.contains("x265")
        || lower_title.contains("h.265")
        || lower_title.contains("h265")
    {
        Some("HEVC")
    } else if lower_codec.contains("av1") || lower_title.contains("av1") {
        Some("AV1")
    } else if lower_codec.contains("h264")
        || lower_codec.contains("x264")
        || lower_codec.contains("avc")
        || lower_title.contains("x264")
        || lower_title.contains("h.264")
        || lower_title.contains("h264")
        || lower_title.contains("avc")
    {
        Some("H.264")
    } else {
        None
    };

    let source = if lower_title.contains("remux") {
        Some("REMUX")
    } else if lower_title.contains("bluray")
        || lower_title.contains("bdrip")
        || lower_title.contains("brrip")
    {
        Some("BluRay")
    } else if lower_title.contains("web-dl")
        || lower_title.contains("webdl")
        || lower_title.contains("webrip")
    {
        Some("WEB-DL")
    } else {
        None
    };

    MediaTags {
        hdr,
        audio,
        codec,
        source,
    }
}

pub fn render_media_tag_spans<'a>(
    tags: &MediaTags,
    theme: &'a Theme,
    basic_terminal: bool,
) -> Vec<Span<'a>> {
    let mut tag_items = Vec::new();

    if let Some(hdr) = tags.hdr {
        tag_items.push(Span::styled(hdr, theme.rating.add_modifier(Modifier::BOLD)));
    }

    if let Some(audio) = tags.audio {
        tag_items.push(Span::styled(
            audio,
            theme.sapphire.add_modifier(Modifier::BOLD),
        ));
    }

    if let Some(codec) = tags.codec {
        tag_items.push(Span::styled(codec, theme.teal.add_modifier(Modifier::BOLD)));
    }

    if let Some(source) = tags.source {
        tag_items.push(Span::styled(
            source,
            theme.lavender.add_modifier(Modifier::BOLD),
        ));
    }

    if tag_items.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let sep = if basic_terminal { " - " } else { " · " };

    for (i, tag_span) in tag_items.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(sep, theme.text_dim));
        }
        spans.push(tag_span);
    }
    spans.push(Span::raw("  "));

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_media_tags() {
        let tags = extract_media_tags(
            "Dune.Part.Two.2024.2160p.UHD.BluRay.x265.Atmos.TrueHD.7.1.DV.HDR",
            "hevc",
        );
        assert_eq!(tags.hdr, Some("DV"));
        assert_eq!(tags.audio, Some("ATMOS"));
        assert_eq!(tags.codec, Some("HEVC"));
        assert_eq!(tags.source, Some("BluRay"));

        let tags_web = extract_media_tags(
            "Movie.Title.2023.1080p.HDR10Plus.WEB-DL.DDP5.1.H.264",
            "H264",
        );
        assert_eq!(tags_web.hdr, Some("HDR10+"));
        assert_eq!(tags_web.audio, Some("5.1"));
        assert_eq!(tags_web.codec, Some("H.264"));
        assert_eq!(tags_web.source, Some("WEB-DL"));
    }

    #[test]
    fn test_resolution_badge_spans() {
        let theme = Theme::default();
        let spans_4k = resolution_badge_spans(2160, &theme, false, false, false);
        assert_eq!(spans_4k[0].content, "  4K   ");

        let spans_1080 = resolution_badge_spans(1080, &theme, false, false, false);
        assert_eq!(spans_1080[0].content, " 1080p ");

        let spans_720 = resolution_badge_spans(720, &theme, false, false, false);
        assert_eq!(spans_720[0].content, " 720p  ");

        let spans_sd = resolution_badge_spans(480, &theme, false, false, false);
        assert_eq!(spans_sd[0].content, " 480p  ");

        let spans_multi = resolution_badge_spans(-1, &theme, false, false, false);
        assert_eq!(spans_multi[0].content, " Multi ");

        let basic_4k = resolution_badge_spans(2160, &theme, true, false, false);
        assert_eq!(basic_4k[0].content.trim(), "[4K]");
        let basic_multi = resolution_badge_spans(-1, &theme, true, false, false);
        assert_eq!(basic_multi[0].content.trim(), "[Multi]");

        let muted_multi = resolution_badge_spans(-1, &theme, false, true, false);
        assert_eq!(muted_multi[0].style.fg, theme.overlay1.fg);
        let muted_basic = resolution_badge_spans(-1, &theme, true, true, false);
        assert_eq!(muted_basic[0].style.fg, theme.muted.fg);
    }

    #[test]
    fn test_all_themes_badge_readability() {
        for theme_name in crate::tui::theme::AVAILABLE_THEMES {
            let kind = crate::tui::theme::ThemeKind::parse(theme_name);
            let theme = crate::tui::theme::Theme::from_kind(kind);
            for res in [-1, 2160, 1080, 720, 480] {
                let unselected = resolution_badge_spans(res, &theme, false, false, false);
                let selected = resolution_badge_spans(res, &theme, false, false, true);
                assert!(unselected[0].style.bg.is_some());
                assert!(unselected[0].style.fg.is_some());
                assert!(selected[0].style.bg.is_some());
                assert!(selected[0].style.fg.is_some());
                assert_ne!(
                    unselected[0].style.bg, unselected[0].style.fg,
                    "Theme {theme_name} res {res} has identical fg and bg!"
                );
                assert_ne!(
                    selected[0].style.bg, selected[0].style.fg,
                    "Theme {theme_name} selected res {res} has identical fg and bg!"
                );
            }
        }
    }

    #[test]
    fn test_resolution_label() {
        assert_eq!(resolution_label(-1), "Multi");
        assert_eq!(resolution_label(4320), "4K");
        assert_eq!(resolution_label(2160), "4K");
        assert_eq!(resolution_label(1080), "1080p");
        assert_eq!(resolution_label(720), "720p");
        assert_eq!(resolution_label(480), "480p");
        assert_eq!(resolution_label(576), "480p");
        assert_eq!(resolution_label(360), "360p");
        assert_eq!(resolution_label(0), "SD");
    }
    #[test]
    fn test_render_media_tag_spans() {
        let theme = Theme::default();
        let tags = MediaTags {
            hdr: Some("HDR"),
            audio: Some("5.1"),
            codec: Some("HEVC"),
            source: Some("WEB-DL"),
        };
        let spans = render_media_tag_spans(&tags, &theme, false);
        assert_eq!(spans.len(), 8);
        let basic_spans = render_media_tag_spans(&tags, &theme, true);
        assert_eq!(basic_spans[0].content, "HDR");
    }
    #[test]
    fn test_provider_origin_tag() {
        assert_eq!(provider_origin_tag(ProviderKind::MovieBox), "[MovieBox]");
        assert_eq!(provider_origin_tag(ProviderKind::FourKHdHub), "[4KHD]");
        assert_eq!(
            provider_origin_tag(ProviderKind::BdixCircleFtp),
            "[CircleFTP]"
        );
        assert_eq!(
            provider_origin_tag(ProviderKind::BdixDhakaFlix),
            "[DhakaFlix]"
        );
        assert_eq!(provider_origin_tag(ProviderKind::Addons), "[Addon]");
    }

    #[test]
    fn test_extract_resolution() {
        assert_eq!(extract_resolution("Movie 1080p BluRay", None), Some(1080));
        assert_eq!(extract_resolution("Movie 4K UHD", None), Some(2160));
        assert_eq!(extract_resolution("Movie 720p WEB", None), Some(720));
        assert_eq!(extract_resolution("Movie", Some("2160p")), Some(2160));
        assert_eq!(extract_resolution("Plain Title", None), None);
    }
}
