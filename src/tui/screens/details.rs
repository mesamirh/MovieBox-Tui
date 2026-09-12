use crate::models::MediaDetails;
pub(crate) use crate::tui::widgets::{
    extract_media_tags, loading_spinner as stream_loading_spinner, render_poster_placeholder,
    resolution_badge_spans,
};
use crate::tui::{
    state::AppState,
    theme::{Theme, theme_color},
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsLayoutTier {
    Wide,
    Medium,
    Narrow,
    Tiny,
}

impl DetailsLayoutTier {
    pub fn for_area(area: Rect) -> Self {
        if area.width < 60 || area.height < 22 {
            Self::Tiny
        } else if area.width < 85 {
            Self::Narrow
        } else if area.width < 115 {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    pub(crate) fn header_height(self, area: Rect, details: Option<&MediaDetails>) -> u16 {
        let (minimum, maximum, synopsis_limit, reserved_width) = match self {
            Self::Wide => (5, 11, 4, 18),
            Self::Medium => (5, 10, 3, 16),
            Self::Narrow => (4, 9, 3, 4),
            Self::Tiny => (4, 7, 2, 4),
        };
        let available_maximum = area.height.saturating_sub(match self {
            Self::Wide => 14,
            Self::Medium => 12,
            Self::Narrow => 10,
            Self::Tiny => 8,
        });
        let maximum = maximum.min(available_maximum.max(minimum));

        let Some(details) = details else {
            return minimum.min(maximum);
        };
        let synopsis = details.description.as_deref().unwrap_or_default();
        let show_poster = !matches!(self, Self::Tiny | Self::Narrow) && area.width >= 75;
        let text_width = area
            .width
            .saturating_sub(if show_poster { reserved_width } else { 4 })
            .max(20) as usize;
        let title = &details.title;
        let title_rows = crate::tui::text::width(title)
            .div_ceil(text_width)
            .clamp(1, 2);
        let has_cast = details.stars.as_ref().is_some_and(|s| !s.trim().is_empty());
        let has_director = details
            .director
            .as_ref()
            .is_some_and(|d| !d.trim().is_empty());
        let has_genres = !details.genres.is_empty();
        let has_extra_meta = has_cast || has_director || has_genres;
        let meta_lines = title_rows + 1 + 1 + (if has_extra_meta { 1 } else { 0 });
        let synopsis_rows = if synopsis.trim().is_empty() {
            0
        } else {
            crate::tui::text::width(synopsis)
                .div_ceil(text_width)
                .clamp(1, synopsis_limit)
        };
        let content_rows = if show_poster {
            (meta_lines + synopsis_rows).max(5)
        } else {
            meta_lines + synopsis_rows
        };
        (content_rows as u16 + 2).clamp(minimum, maximum)
    }
    pub fn footer_height(self, width: u16) -> u16 {
        if width >= DETAILS_FOOTER_SPLIT_THRESHOLD {
            1
        } else {
            2
        }
    }
}

pub const DETAILS_FOOTER_SPLIT_THRESHOLD: u16 = 106;

pub(crate) fn visible_selector_panes(
    available_panes: &[crate::tui::state::DetailsPane],
    current_pane: crate::tui::state::DetailsPane,
    width: u16,
) -> Vec<crate::tui::state::DetailsPane> {
    if available_panes.is_empty() {
        return Vec::new();
    }
    if width < 85 {
        if available_panes.contains(&current_pane) {
            vec![current_pane]
        } else if let Some(last) = available_panes.last() {
            vec![*last]
        } else {
            Vec::new()
        }
    } else {
        available_panes.to_vec()
    }
}

pub(crate) fn selector_pane_constraints(
    visible_panes: &[crate::tui::state::DetailsPane],
    total_width: u16,
) -> Vec<Constraint> {
    use crate::tui::state::DetailsPane;
    match visible_panes.len() {
        0 => Vec::new(),
        1 => vec![Constraint::Min(20)],
        2 => {
            if visible_panes.contains(&DetailsPane::Languages) {
                if total_width < 100 {
                    vec![Constraint::Percentage(38), Constraint::Percentage(62)]
                } else {
                    vec![Constraint::Length(24), Constraint::Min(30)]
                }
            } else if visible_panes.contains(&DetailsPane::Seasons)
                && visible_panes.contains(&DetailsPane::Episodes)
            {
                if total_width < 100 {
                    vec![Constraint::Percentage(34), Constraint::Percentage(66)]
                } else {
                    vec![Constraint::Length(18), Constraint::Min(24)]
                }
            } else {
                vec![Constraint::Percentage(50), Constraint::Percentage(50)]
            }
        }
        3 => {
            if total_width < 115 {
                vec![
                    Constraint::Percentage(30),
                    Constraint::Percentage(26),
                    Constraint::Percentage(44),
                ]
            } else {
                vec![
                    Constraint::Length(24),
                    Constraint::Length(18),
                    Constraint::Min(28),
                ]
            }
        }
        _ => visible_panes
            .iter()
            .map(|_| Constraint::Ratio(1, visible_panes.len() as u32))
            .collect(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DetailsScreenLayout {
    pub tier: DetailsLayoutTier,
    pub header_area: Rect,
    pub workflow_area: Rect,
    pub bottom_area: Rect,
    pub footer_area: Rect,
}

pub fn details_screen_layout(
    area: Rect,
    selected_details: Option<&MediaDetails>,
) -> DetailsScreenLayout {
    let tier = DetailsLayoutTier::for_area(area);
    let header_height = tier.header_height(area, selected_details);
    let footer_height = tier.footer_height(area.width);
    let workflow_height = if area.width < 85 { 1 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(workflow_height),
        Constraint::Min(5),
        Constraint::Length(footer_height),
    ])
    .split(area);

    DetailsScreenLayout {
        tier,
        header_area: chunks[0],
        workflow_area: chunks[1],
        bottom_area: chunks[2],
        footer_area: chunks[3],
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let modal_active = state.has_active_modal();
    let layout = details_screen_layout(area, state.selected_details.as_ref());
    let tier = layout.tier;
    let header_area = layout.header_area;
    let workflow_area = layout.workflow_area;
    let bottom_area = layout.bottom_area;
    let footer_area = layout.footer_area;
    let details = match &state.selected_details {
        Some(d) => d,
        None => {
            if let Some(err) = &state.details_error {
                let box_width = area.width.saturating_sub(4).clamp(30, 60).min(area.width);
                let box_height = area.height.saturating_sub(2).clamp(5, 7).min(area.height);
                let x = area.x + (area.width.saturating_sub(box_width)) / 2;
                let y = area.y + (area.height.saturating_sub(box_height)) / 2;
                let error_area = Rect::new(x, y, box_width, box_height);

                let error_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(crate::tui::overlay::border_type(state.basic_terminal))
                    .border_style(theme.error)
                    .title(Line::from(vec![Span::styled(
                        " Error Loading Details ",
                        theme.error.add_modifier(Modifier::BOLD),
                    )]));

                let err_msg =
                    crate::tui::text::truncate_width(err, (box_width.saturating_sub(4)) as usize);
                let mut text = vec![
                    Line::from(""),
                    Line::from(Span::styled(err_msg, theme.text)),
                    Line::from(""),
                ];
                let mut hint_spans = crate::tui::overlay::key_hint("r", "Retry", theme);
                hint_spans.push(Span::raw("   "));
                hint_spans.extend(crate::tui::overlay::key_hint("Esc", "Back", theme));
                text.push(Line::from(hint_spans));

                let error_p = Paragraph::new(text)
                    .block(error_block)
                    .alignment(Alignment::Center);

                frame.render_widget(error_p, error_area);
                return;
            }

            let spinner = stream_loading_spinner(state.tick_count, state.basic_terminal);

            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(area);

            let loading_spans = if state.basic_terminal {
                vec![Span::styled(
                    format!("Loading details {spinner}"),
                    theme.lavender,
                )]
            } else {
                vec![
                    Span::styled(
                        format!("{spinner} "),
                        theme.accent.add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        "Loading details...",
                        theme.subtext1.add_modifier(Modifier::BOLD),
                    ),
                ]
            };
            let loading_p = Paragraph::new(Line::from(loading_spans))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(loading_p, vertical_chunks[1]);
            return;
        }
    };

    let raw_title = if !details.title.trim().is_empty() {
        &details.title
    } else if let Some(res) = state.search_results.iter().find(|r| {
        if let Some(act_id) = state.active_subject_id.as_deref() {
            r.id == act_id
        } else {
            false
        }
    }) {
        &res.title
    } else {
        "Unknown Title"
    };
    let title = crate::providers::moviebox::clean_moviebox_title(raw_title);
    let intro = details
        .description
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            state
                .search_preview
                .as_ref()
                .filter(|p| p.id.value == details.id.value && p.id.provider == details.id.provider)
                .and_then(|p| p.description.as_deref())
        })
        .unwrap_or("No description available.");
    let year = details
        .year
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            state.search_results.iter().find_map(|r| {
                if let Some(act_id) = state.active_subject_id.as_deref() {
                    (r.id == act_id).then_some(r.release_year.as_str())
                } else {
                    None
                }
            })
        })
        .unwrap_or("N/A");
    let is_series = details.is_series() && !state.available_seasons.is_empty();
    let has_languages = details.has_languages();
    let type_str = if is_series { "Series" } else { "Movie" };

    let genres = if !details.genres.is_empty() {
        details.genres.join(", ")
    } else {
        "N/A".to_string()
    };
    let duration = details
        .duration
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("N/A");
    let imdb_rating = details
        .imdb_rating
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("N/A");
    let tagline = details.tagline.as_deref();

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .border_style(if modal_active {
            theme.muted
        } else {
            theme.surface1
        })
        .padding(ratatui::widgets::Padding::new(
            if matches!(tier, DetailsLayoutTier::Wide) {
                2
            } else {
                1
            },
            1,
            0,
            0,
        ));

    let inner_area = details_block.inner(header_area);
    frame.render_widget(details_block.clone(), header_area);

    let show_poster = !matches!(tier, DetailsLayoutTier::Tiny | DetailsLayoutTier::Narrow)
        && inner_area.height >= 5
        && inner_area.width >= 75;
    let poster_width = if show_poster {
        let width_for_height = state
            .poster_image
            .as_ref()
            .zip(state.image_picker.as_ref())
            .map(|(image, picker)| {
                let font_size = picker.font_size();
                let font_w = font_size.width.max(1);
                let font_h = font_size.height.max(1);
                let area_h_px = (inner_area.height as f32) * (font_h as f32);
                let aspect = image.width() as f32 / (image.height() as f32).max(1.0);
                let area_w_px = area_h_px * aspect;
                ((area_w_px / font_w as f32).round() as u16).max(1)
            })
            .unwrap_or_else(|| ((inner_area.height as f32 * (4.0 / 3.0)).round() as u16).max(6));
        width_for_height.clamp(12, 22)
    } else {
        0
    };

    let meta_constraints = if show_poster {
        vec![
            Constraint::Length(poster_width),
            Constraint::Length(2),
            Constraint::Min(20),
        ]
    } else {
        vec![Constraint::Min(20)]
    };

    let meta_area = if show_poster {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(meta_constraints)
            .split(inner_area);

        let poster_area = chunks[0];
        if let Some(img) = &state.poster_image {
            if !modal_active {
                if let Some(picker) = &mut state.image_picker {
                    let img_width = poster_area.width;
                    let img_height = poster_area.height;
                    if img_width > 0 && img_height > 0 {
                        crate::tui::clear_area(frame, poster_area, theme);
                        if let Some((proto_area, proto)) = &mut state.poster_protocol {
                            if proto_area.width == img_width && proto_area.height == img_height {
                                let image_widget = ratatui_image::Image::new(proto);
                                frame.render_widget(image_widget, poster_area);
                            } else if let Ok(protocol) = picker.new_protocol(
                                (**img).clone(),
                                poster_area.into(),
                                ratatui_image::Resize::Fit(None),
                            ) {
                                state.poster_protocol = Some((poster_area, protocol));
                                if let Some((_, p)) = &state.poster_protocol {
                                    let image_widget = ratatui_image::Image::new(p);
                                    frame.render_widget(image_widget, poster_area);
                                }
                            }
                        } else if let Ok(protocol) = picker.new_protocol(
                            (**img).clone(),
                            poster_area.into(),
                            ratatui_image::Resize::Fit(None),
                        ) {
                            state.poster_protocol = Some((poster_area, protocol));
                            if let Some((_, p)) = &state.poster_protocol {
                                let image_widget = ratatui_image::Image::new(p);
                                frame.render_widget(image_widget, poster_area);
                            }
                        }
                    }
                }
            }
        } else {
            let is_in_flight = state.image_supported && (state.is_loading || state.preview_loading);
            render_poster_placeholder(
                frame,
                poster_area,
                theme,
                state.basic_terminal,
                is_in_flight,
                state.tick_count,
            );
        }

        chunks[2]
    } else {
        inner_area
    };

    let text_width = meta_area.width as usize;
    let total_height = meta_area.height as usize;

    let mut rendered_lines: Vec<Line> = Vec::new();
    let title_style = if modal_active {
        theme.overlay0.add_modifier(Modifier::BOLD)
    } else {
        theme.title.add_modifier(Modifier::BOLD)
    };
    let title_w = crate::tui::text::width(&title);
    if title_w <= text_width {
        rendered_lines.push(Line::from(vec![Span::styled(title.clone(), title_style)]));
    } else {
        let wrapped_title = crate::tui::text::wrap_text(&title, text_width);
        for line in wrapped_title.into_iter().take(2) {
            rendered_lines.push(Line::from(vec![Span::styled(line, title_style)]));
        }
    }

    let bullet_sep = if state.basic_terminal { " - " } else { "  " };
    let (b_year_s, b_sep_s, b_type_s, b_dur_s, b_rating_s, b_audio_lbl_s, b_audio_val_s, b_prov_s) =
        if modal_active {
            (
                theme.muted,
                theme.muted,
                theme.muted,
                theme.muted,
                theme.muted,
                theme.muted,
                theme.muted,
                theme.muted,
            )
        } else {
            (
                theme.subtext1,
                theme.overlay0,
                theme.accent.add_modifier(Modifier::BOLD),
                theme.subtext1,
                theme.rating.add_modifier(Modifier::BOLD),
                theme.subtext1.add_modifier(Modifier::BOLD),
                theme.accent,
                theme.subtext1,
            )
        };
    let mut badge_spans = vec![Span::styled(type_str, b_type_s)];
    if !year.is_empty() && year != "N/A" {
        badge_spans.insert(0, Span::styled(bullet_sep, b_sep_s));
        badge_spans.insert(0, Span::styled(year, b_year_s));
    }

    if !duration.trim().is_empty() && duration != "N/A" {
        badge_spans.push(Span::styled(bullet_sep, b_sep_s));
        badge_spans.push(Span::styled(duration, b_dur_s));
    }

    if !imdb_rating.trim().is_empty() && imdb_rating != "N/A" {
        badge_spans.push(Span::styled(bullet_sep, b_sep_s));
        if state.basic_terminal {
            badge_spans.push(Span::styled("IMDb ", b_rating_s));
        } else {
            badge_spans.push(Span::styled("★ ", b_rating_s));
        }
        badge_spans.push(Span::styled(imdb_rating, b_rating_s));
    }

    let audio_str = if has_languages {
        None
    } else {
        details
            .audios
            .as_deref()
            .filter(|s| !s.trim().is_empty() && *s != "N/A")
            .map(|s| s.to_string())
    };

    let bullet_w = crate::tui::text::width(bullet_sep);
    let provider_name = details.id.provider.label();
    let provider_w = if !provider_name.is_empty() {
        bullet_w + crate::tui::text::width(provider_name)
    } else {
        0
    };
    let current_badge_w: usize = badge_spans
        .iter()
        .map(|s| crate::tui::text::width(&s.content))
        .sum();

    if let Some(audios) = audio_str {
        let label_w = 7;
        if current_badge_w + provider_w + bullet_w + label_w + 4 <= text_width {
            badge_spans.push(Span::styled(bullet_sep, b_sep_s));
            badge_spans.push(Span::styled("Audio: ", b_audio_lbl_s));
            let available_audio_w =
                text_width.saturating_sub(current_badge_w + provider_w + bullet_w + label_w);
            let display_audios = if crate::tui::text::width(&audios) > available_audio_w {
                crate::tui::text::truncate_width(&audios, available_audio_w).into_owned()
            } else {
                audios
            };
            badge_spans.push(Span::styled(display_audios, b_audio_val_s));
        }
    }

    if !provider_name.is_empty() {
        badge_spans.push(Span::styled(bullet_sep, b_sep_s));
        badge_spans.push(Span::styled(provider_name.to_string(), b_prov_s));
    }

    rendered_lines.push(Line::from(badge_spans));

    if let Some(t) = tagline.filter(|t| !t.trim().is_empty()) {
        rendered_lines.push(Line::from(vec![Span::styled(
            format!("\"{t}\""),
            if modal_active {
                theme.muted
            } else {
                theme.subtext1.add_modifier(Modifier::ITALIC)
            },
        )]));
    }

    let mut extra_meta_spans = Vec::new();
    let mut extra_meta_w = 0;
    let (meta_lbl_s, meta_val_s, meta_sep_s) = if modal_active {
        (theme.muted, theme.muted, theme.muted)
    } else {
        (
            theme.subtext1.add_modifier(Modifier::BOLD),
            Style::default().fg(theme_color(
                theme.subtext1,
                theme.text.fg.unwrap_or(theme.base),
            )),
            theme.overlay1,
        )
    };
    if !details.genres.is_empty() {
        let label_w = 7;
        let val_w = crate::tui::text::width(&genres);
        if label_w + 4 <= text_width {
            extra_meta_spans.push(Span::styled("Genre: ", meta_lbl_s));
            let available_genre_w = text_width.saturating_sub(label_w);
            let display_genres = if val_w > available_genre_w {
                crate::tui::text::truncate_width(&genres, available_genre_w)
            } else {
                std::borrow::Cow::Borrowed(genres.as_str())
            };
            extra_meta_w += label_w + crate::tui::text::width(&display_genres);
            extra_meta_spans.push(Span::styled(display_genres, meta_val_s));
        }
    }
    if let Some(dir) = details
        .director
        .as_deref()
        .filter(|d| !d.trim().is_empty() && *d != "N/A")
    {
        let sep_w = if !extra_meta_spans.is_empty() {
            bullet_w
        } else {
            0
        };
        let label_w = 10;
        let val_w = crate::tui::text::width(dir);
        if extra_meta_w + sep_w + label_w + 4 <= text_width {
            if !extra_meta_spans.is_empty() {
                extra_meta_spans.push(Span::styled(bullet_sep, meta_sep_s));
            }
            extra_meta_spans.push(Span::styled("Director: ", meta_lbl_s));
            let available_dir_w = text_width.saturating_sub(extra_meta_w + sep_w + label_w);
            let display_dir = if val_w > available_dir_w {
                crate::tui::text::truncate_width(dir, available_dir_w)
            } else {
                std::borrow::Cow::Borrowed(dir)
            };
            extra_meta_w += sep_w + label_w + crate::tui::text::width(&display_dir);
            extra_meta_spans.push(Span::styled(display_dir, meta_val_s));
        }
    }
    if let Some(cast) = details
        .stars
        .as_deref()
        .filter(|s| !s.trim().is_empty() && *s != "N/A")
    {
        let sep_w = if !extra_meta_spans.is_empty() {
            bullet_w
        } else {
            0
        };
        let label_w = 6;
        let val_w = crate::tui::text::width(cast);
        if extra_meta_w + sep_w + label_w + 4 <= text_width {
            if !extra_meta_spans.is_empty() {
                extra_meta_spans.push(Span::styled(bullet_sep, meta_sep_s));
            }
            extra_meta_spans.push(Span::styled("Cast: ", meta_lbl_s));
            let available_cast_w = text_width.saturating_sub(extra_meta_w + sep_w + label_w);
            let display_cast = if val_w > available_cast_w {
                crate::tui::text::truncate_width(cast, available_cast_w)
            } else {
                std::borrow::Cow::Borrowed(cast)
            };
            extra_meta_spans.push(Span::styled(display_cast, meta_val_s));
        }
    }
    let has_extra_meta = !extra_meta_spans.is_empty();

    let reserved_bottom = if has_extra_meta { 1 } else { 0 };
    let remaining_height = total_height.saturating_sub(rendered_lines.len() + reserved_bottom);
    let (include_spacer, max_synopsis_lines) = if remaining_height >= 2 {
        (true, remaining_height.saturating_sub(1))
    } else {
        (false, remaining_height)
    };

    if include_spacer {
        rendered_lines.push(Line::from(""));
    }

    if max_synopsis_lines > 0 {
        let mut wrapped_synopsis = crate::tui::text::wrap_text(intro, text_width);
        if wrapped_synopsis.len() > max_synopsis_lines {
            wrapped_synopsis.truncate(max_synopsis_lines);
            if let Some(last) = wrapped_synopsis.last_mut() {
                let last_w = crate::tui::text::width(last);
                if last_w + 3 <= text_width {
                    last.push_str("...");
                } else {
                    let truncated =
                        crate::tui::text::truncate_width(last, text_width.saturating_sub(3));
                    *last = format!("{truncated}...");
                }
            }
        }
        for line in wrapped_synopsis {
            rendered_lines.push(Line::from(vec![Span::styled(
                line,
                if modal_active {
                    theme.muted
                } else {
                    theme.subtext1
                },
            )]));
        }
    }

    if has_extra_meta {
        rendered_lines.push(Line::from(extra_meta_spans));
    }

    let top_offset = total_height.saturating_sub(rendered_lines.len()) / 2;
    let final_lines = if top_offset > 0 {
        let mut padded = Vec::with_capacity(total_height);
        for _ in 0..top_offset {
            padded.push(Line::from(""));
        }
        padded.extend(rendered_lines);
        padded
    } else {
        rendered_lines
    };

    let meta_p = Paragraph::new(final_lines);
    frame.render_widget(meta_p, meta_area);

    let streams_count = state.selected_resources.len();

    render_workflow(
        frame,
        workflow_area,
        state,
        details,
        has_languages,
        is_series,
        streams_count,
        theme,
        modal_active,
    );

    let mut available_selector_panes = Vec::new();
    if has_languages {
        available_selector_panes.push(crate::tui::state::DetailsPane::Languages);
    }
    if is_series {
        available_selector_panes.push(crate::tui::state::DetailsPane::Seasons);
        available_selector_panes.push(crate::tui::state::DetailsPane::Episodes);
    }

    let visible_selector_panes =
        visible_selector_panes(&available_selector_panes, state.details_pane, area.width);

    let selector_height = if visible_selector_panes.is_empty() {
        0
    } else {
        let episode_count = state
            .available_episode_numbers
            .get(state.season_list_state.selected().unwrap_or(0))
            .map_or(0, Vec::len);
        let language_count = details.dubs.len();
        let max_visible_items = visible_selector_panes
            .iter()
            .map(|pane| match pane {
                crate::tui::state::DetailsPane::Languages => language_count,
                crate::tui::state::DetailsPane::Seasons => state.available_seasons.len(),
                crate::tui::state::DetailsPane::Episodes => episode_count,
                crate::tui::state::DetailsPane::Streams => 0,
            })
            .max()
            .unwrap_or(0);
        let max_rows = (bottom_area.height / 3).clamp(4, 10) as usize;
        (max_visible_items.min(max_rows) as u16) + 2
    };

    let streams_height_cap = if state.has_streams_settled && streams_count > 0 {
        let capped =
            (streams_count as u16 + 3).min(bottom_area.height.saturating_sub(selector_height));
        Constraint::Length(capped)
    } else {
        Constraint::Min(3)
    };

    let lower_chunks = Layout::vertical([Constraint::Length(selector_height), streams_height_cap])
        .split(bottom_area);
    let selector_area = lower_chunks[0];
    let streams_area = lower_chunks[1];

    let selector_chunks = if visible_selector_panes.is_empty() {
        Vec::new()
    } else {
        Layout::horizontal(selector_pane_constraints(
            &visible_selector_panes,
            selector_area.width,
        ))
        .split(selector_area)
        .to_vec()
    };

    let mut lang_area = None;
    let mut seasons_area = None;
    let mut eps_area = None;
    for (pane, pane_area) in visible_selector_panes
        .iter()
        .copied()
        .zip(selector_chunks.iter().copied())
    {
        match pane {
            crate::tui::state::DetailsPane::Languages => lang_area = Some(pane_area),
            crate::tui::state::DetailsPane::Seasons => seasons_area = Some(pane_area),
            crate::tui::state::DetailsPane::Episodes => eps_area = Some(pane_area),
            crate::tui::state::DetailsPane::Streams => {}
        }
    }

    if has_languages {
        use ratatui::widgets::{List, ListItem};
        let language_focused =
            !modal_active && state.details_pane == crate::tui::state::DetailsPane::Languages;
        let selected_lang = state.language_list_state.selected();
        let mut lang_items = Vec::new();
        for (i, dub) in details.dubs.iter().enumerate() {
            let name = clean_language_name(&dub.language);
            let is_selected = Some(i) == selected_lang;
            let item_style = if modal_active {
                theme.muted
            } else if is_selected && language_focused {
                crate::tui::overlay::selection_style(theme, state.basic_terminal)
            } else if is_selected {
                theme.subtext1.add_modifier(Modifier::BOLD)
            } else {
                theme.text
            };
            lang_items.push(ListItem::new(name).style(item_style));
        }
        let language_count = lang_items.len();

        let (lang_border, lang_title_style, _lang_highlight_style, _lang_highlight_symbol) =
            pane_styles(language_focused, modal_active, state.basic_terminal, theme);
        let lang_list = List::new(lang_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(crate::tui::overlay::border_type(state.basic_terminal))
                .title(pane_title(
                    "Audio",
                    language_count,
                    crate::tui::state::DetailsPane::Languages,
                    language_focused,
                    state,
                    lang_area.map_or(0, |a| a.width),
                    modal_active,
                ))
                .title_style(lang_title_style)
                .border_style(lang_border)
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );

        if let Some(area) = lang_area {
            frame.render_stateful_widget(lang_list, area, &mut state.language_list_state);
            render_scroll_indicator(
                frame,
                area,
                language_count,
                state.language_list_state.selected().unwrap_or(0),
                theme,
                state.basic_terminal,
            );
        }
    }

    if is_series {
        use ratatui::widgets::{List, ListItem};
        let seasons_focused =
            !modal_active && state.details_pane == crate::tui::state::DetailsPane::Seasons;
        let selected_season = state.season_list_state.selected();
        let seasons_items: Vec<ListItem> = state
            .available_seasons
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let is_selected = Some(i) == selected_season;
                let item_style = if modal_active {
                    theme.muted
                } else if is_selected && seasons_focused {
                    crate::tui::overlay::selection_style(theme, state.basic_terminal)
                } else if is_selected {
                    theme.subtext1.add_modifier(Modifier::BOLD)
                } else {
                    theme.text
                };
                ListItem::new(format!("Season {}", s.number)).style(item_style)
            })
            .collect();
        let (
            seasons_border,
            seasons_title_style,
            _seasons_highlight_style,
            _seasons_highlight_symbol,
        ) = pane_styles(seasons_focused, modal_active, state.basic_terminal, theme);
        let seasons_list = List::new(seasons_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(crate::tui::overlay::border_type(state.basic_terminal))
                .title(pane_title(
                    "Seasons",
                    state.available_seasons.len(),
                    crate::tui::state::DetailsPane::Seasons,
                    seasons_focused,
                    state,
                    seasons_area.map_or(0, |a| a.width),
                    modal_active,
                ))
                .title_style(seasons_title_style)
                .border_style(seasons_border)
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );

        if let Some(area) = seasons_area {
            frame.render_stateful_widget(seasons_list, area, &mut state.season_list_state);
            render_scroll_indicator(
                frame,
                area,
                state.available_seasons.len(),
                state.season_list_state.selected().unwrap_or(0),
                theme,
                state.basic_terminal,
            );
        }

        let episodes_focused =
            !modal_active && state.details_pane == crate::tui::state::DetailsPane::Episodes;
        let (eps_border, eps_title_style, _eps_highlight_style, _eps_highlight_symbol) =
            pane_styles(episodes_focused, modal_active, state.basic_terminal, theme);
        let selected_ep = state.episode_list_state.selected();

        let ep_items: Vec<ListItem> = if let Some(ep_numbers) = state
            .available_episode_numbers
            .get(state.season_list_state.selected().unwrap_or(0))
        {
            let season_idx = state.season_list_state.selected().unwrap_or(0);
            let se_num = state
                .available_seasons
                .get(season_idx)
                .map(|s| s.number)
                .unwrap_or(1);
            let subject_id = state.active_subject_id.as_deref().unwrap_or("");
            let provider = state.provider_for_subject(subject_id).cache_key();

            let check_sym = if state.basic_terminal { "[x] " } else { "✓ " };
            let play_sym = if state.basic_terminal { "[>] " } else { "▶ " };
            let unwatched_sym = if state.basic_terminal { "    " } else { "  " };
            let ep_max_w = eps_area.map_or(40, |a| a.width.saturating_sub(6)) as usize;
            ep_numbers
                .iter()
                .enumerate()
                .map(|(i, &ep)| {
                    let is_selected = Some(i) == selected_ep;
                    let ep_title_opt = details
                        .seasons
                        .iter()
                        .find(|s| s.number == se_num)
                        .and_then(|s| s.episodes.iter().find(|e| e.number == ep))
                        .and_then(|e| e.title.as_deref())
                        .filter(|t| {
                            let trim = t.trim();
                            !trim.is_empty()
                                && !trim.eq_ignore_ascii_case(&format!("Episode {ep}"))
                                && !trim.eq_ignore_ascii_case(&format!("EP {ep}"))
                        });
                    let ep_base = if let Some(t) = ep_title_opt {
                        let candidate = format!("Episode {ep:02} · {t}");
                        crate::tui::text::truncate_width(&candidate, ep_max_w).into_owned()
                    } else {
                        format!("Episode {ep:02}")
                    };
                    let item_style = if modal_active {
                        theme.muted
                    } else if is_selected && episodes_focused {
                        crate::tui::overlay::selection_style(theme, state.basic_terminal)
                    } else if is_selected {
                        theme.subtext1.add_modifier(Modifier::BOLD)
                    } else if let Some(hist) =
                        state
                            .history
                            .get_item(provider, subject_id, se_num, ep, Some(&title))
                    {
                        if hist.completed {
                            theme.text_dim
                        } else if hist.is_in_progress() {
                            theme.accent
                        } else {
                            theme.text
                        }
                    } else if state.history.is_watched(provider, subject_id, se_num, ep) {
                        theme.text_dim
                    } else {
                        theme.text
                    };
                    let sym = if let Some(hist) =
                        state
                            .history
                            .get_item(provider, subject_id, se_num, ep, Some(&title))
                    {
                        if hist.completed {
                            check_sym
                        } else if hist.is_in_progress() {
                            play_sym
                        } else {
                            unwatched_sym
                        }
                    } else if state.history.is_watched(provider, subject_id, se_num, ep) {
                        check_sym
                    } else {
                        unwatched_sym
                    };
                    let progress_info = state
                        .history
                        .get_item(provider, subject_id, se_num, ep, Some(&title))
                        .filter(|h| !h.completed && h.is_in_progress())
                        .map(
                            |h| match (h.progress_percentage(), h.formatted_remaining()) {
                                (Some(p), Some(r)) => format!(" ({:.0}% · {r})", p),
                                (Some(p), None) => format!(" ({:.0}%)", p),
                                (None, Some(r)) => format!(" ({r})"),
                                (None, None) => String::new(),
                            },
                        )
                        .unwrap_or_default();
                    ListItem::new(format!("{sym}{ep_base}{progress_info}")).style(item_style)
                })
                .collect()
        } else {
            vec![]
        };
        let episode_count = ep_items.len();

        let eps_list = List::new(ep_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(crate::tui::overlay::border_type(state.basic_terminal))
                .title(pane_title(
                    "Episodes",
                    episode_count,
                    crate::tui::state::DetailsPane::Episodes,
                    episodes_focused,
                    state,
                    eps_area.map_or(0, |a| a.width),
                    modal_active,
                ))
                .title_style(eps_title_style)
                .border_style(eps_border)
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );
        if let Some(area) = eps_area {
            frame.render_stateful_widget(eps_list, area, &mut state.episode_list_state);
            let episode_count = state
                .available_episode_numbers
                .get(state.season_list_state.selected().unwrap_or(0))
                .map_or(0, Vec::len);
            render_scroll_indicator(
                frame,
                area,
                episode_count,
                state.episode_list_state.selected().unwrap_or(0),
                theme,
                state.basic_terminal,
            );
        }
    }

    let streams_focused =
        !modal_active && state.details_pane == crate::tui::state::DetailsPane::Streams;
    let (streams_border, streams_title_style, streams_highlight_style, streams_highlight_symbol) =
        pane_styles(streams_focused, modal_active, state.basic_terminal, theme);

    let list = &state.selected_resources;
    let visible_count = list.len();

    let streams_title = if streams_count > 0 {
        let selected = state
            .resource_list_state
            .selected()
            .unwrap_or(0)
            .min(visible_count.saturating_sub(1));
        let marker = if streams_focused && !modal_active {
            focus_title_marker(state.basic_terminal)
        } else {
            ""
        };
        let avail = streams_area.width.saturating_sub(4) as usize;
        let title_full = if visible_count > 1 {
            format!(
                " {marker}Streams ({visible_count}) · {}/{} ",
                selected + 1,
                visible_count
            )
        } else {
            format!(" {marker}Streams (1) ")
        };
        if avail > 0 && crate::tui::text::width(&title_full) > avail {
            let title_medium = format!(" {marker}Streams ({visible_count}) ");
            if crate::tui::text::width(&title_medium) <= avail {
                title_medium
            } else {
                format!(" {marker}Streams ")
            }
        } else {
            title_full
        }
    } else if streams_focused && !modal_active {
        format!(" {}Streams ", focus_title_marker(state.basic_terminal))
    } else {
        " Streams ".to_string()
    };
    let streams_block = Block::default()
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .title(ratatui::text::Line::from(streams_title).alignment(Alignment::Left))
        .title_style(streams_title_style)
        .border_style(streams_border)
        .padding(ratatui::widgets::Padding::horizontal(1));

    if !list.is_empty() {
        let selected_idx = state.resource_list_state.selected();

        use ratatui::widgets::{Cell, Row, Table};

        let is_ultra_compact = streams_area.width < 58;
        let is_compact = streams_area.width < 85;
        let is_wide = streams_area.width >= 115;

        let header_style = if modal_active {
            theme.muted
        } else {
            theme.subtext1.add_modifier(Modifier::BOLD)
        };

        let (header_cells, widths): (Vec<Cell<'static>>, Vec<Constraint>) = if is_ultra_compact {
            (
                vec![Cell::from("RES"), Cell::from("SIZE"), Cell::from("RELEASE")],
                vec![
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Min(15),
                ],
            )
        } else if is_compact {
            (
                vec![
                    Cell::from("RES"),
                    Cell::from("SIZE"),
                    Cell::from("CODEC"),
                    Cell::from("RELEASE"),
                ],
                vec![
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Min(20),
                ],
            )
        } else if is_wide {
            (
                vec![
                    Cell::from("RES"),
                    Cell::from("SIZE"),
                    Cell::from("MEDIA TAGS"),
                    Cell::from("SOURCE"),
                    Cell::from("RELEASE"),
                ],
                vec![
                    Constraint::Length(8),
                    Constraint::Length(9),
                    Constraint::Length(18),
                    Constraint::Length(16),
                    Constraint::Min(24),
                ],
            )
        } else {
            (
                vec![
                    Cell::from("RES"),
                    Cell::from("SIZE"),
                    Cell::from("MEDIA TAGS"),
                    Cell::from("RELEASE"),
                ],
                vec![
                    Constraint::Length(8),
                    Constraint::Length(9),
                    Constraint::Length(18),
                    Constraint::Min(20),
                ],
            )
        };

        let header = Row::new(header_cells).style(header_style).bottom_margin(0);

        let table_rows: Vec<Row> = list
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let resolution = file.resolution_i64();
                let codec = file.codec.as_deref().unwrap_or("None");

                let size_formatted = file
                    .size_bytes
                    .map(|s| crate::tui::text::format_file_size(s as f64))
                    .unwrap_or_else(|| "--".to_string());

                let is_selected = Some(i) == selected_idx;

                let row_style = if modal_active {
                    theme.muted
                } else if is_selected {
                    selection_style(streams_focused, state.basic_terminal, theme)
                } else {
                    metadata_style(theme)
                };
                let primary_style = if modal_active {
                    theme.muted
                } else if is_selected {
                    with_selection_surface(theme.text, state.basic_terminal, theme)
                        .add_modifier(Modifier::BOLD)
                } else {
                    theme.text
                };
                let secondary_style = if modal_active {
                    theme.muted
                } else if is_selected {
                    with_selection_surface(metadata_style(theme), state.basic_terminal, theme)
                } else {
                    metadata_style(theme)
                };

                let release_title = clean_stream_release_title(&file.filename);
                let clean_source = crate::tui::text::clean_stream_text(file.source_label());
                let is_redundant_source = {
                    let l = clean_source.to_ascii_lowercase();
                    let parts: Vec<&str> = l
                        .split(|c: char| c == '-' || c.is_whitespace())
                        .filter(|p| !p.is_empty())
                        .collect();
                    !parts.is_empty()
                        && parts.iter().all(|p| {
                            matches!(
                                *p,
                                "multi"
                                    | "res"
                                    | "hevc"
                                    | "h264"
                                    | "x265"
                                    | "x264"
                                    | "1080p"
                                    | "720p"
                                    | "480p"
                                    | "360p"
                                    | "4k"
                                    | "direct"
                            )
                        })
                };
                let upload_by = if !clean_source.is_empty() && !is_redundant_source {
                    clean_source
                } else {
                    match file.provider {
                        crate::providers::models::ProviderKind::MovieBox => {
                            "MovieBox CDN".to_string()
                        }
                        crate::providers::models::ProviderKind::FourKHdHub => "4KHDHub".to_string(),
                        crate::providers::models::ProviderKind::BdixCircleFtp => {
                            "CircleFTP".to_string()
                        }
                        crate::providers::models::ProviderKind::BdixDhakaFlix => {
                            "DhakaFlix".to_string()
                        }
                        crate::providers::models::ProviderKind::Addons => "Addon".to_string(),
                    }
                };

                let res_badge = resolution_badge_spans(
                    resolution,
                    theme,
                    state.basic_terminal,
                    modal_active,
                    is_selected && streams_focused,
                );

                let codec_str = codec.to_uppercase();
                let tags = extract_media_tags(&file.filename, &codec_str);
                let mut tag_parts = Vec::new();
                if let Some(hdr) = tags.hdr {
                    tag_parts.push(hdr);
                }
                if let Some(codec_tag) = tags.codec {
                    tag_parts.push(codec_tag);
                } else if codec != "None" && !codec.is_empty() {
                    tag_parts.push(codec_str.as_str());
                }
                if let Some(audio) = tags.audio {
                    tag_parts.push(audio);
                }
                if let Some(source) = tags.source {
                    tag_parts.push(source);
                }
                let sep = if state.basic_terminal { " - " } else { " · " };
                let tags_or_codec = tag_parts.join(sep);
                let tags_col = if tags_or_codec.is_empty() {
                    "-".to_string()
                } else {
                    tags_or_codec
                };

                let cells: Vec<Cell> = if is_ultra_compact {
                    vec![
                        Cell::from(ratatui::text::Line::from(res_badge)),
                        Cell::from(Span::styled(size_formatted.clone(), primary_style)),
                        Cell::from(Span::styled(release_title, primary_style)),
                    ]
                } else if is_compact {
                    vec![
                        Cell::from(ratatui::text::Line::from(res_badge)),
                        Cell::from(Span::styled(size_formatted.clone(), primary_style)),
                        Cell::from(Span::styled(format!("{codec:<6}"), secondary_style)),
                        Cell::from(Span::styled(release_title, primary_style)),
                    ]
                } else if is_wide {
                    vec![
                        Cell::from(ratatui::text::Line::from(res_badge)),
                        Cell::from(Span::styled(size_formatted.clone(), primary_style)),
                        Cell::from(Span::styled(tags_col, secondary_style)),
                        Cell::from(Span::styled(upload_by, secondary_style)),
                        Cell::from(Span::styled(release_title, primary_style)),
                    ]
                } else {
                    vec![
                        Cell::from(ratatui::text::Line::from(res_badge)),
                        Cell::from(Span::styled(size_formatted.clone(), primary_style)),
                        Cell::from(Span::styled(tags_col, secondary_style)),
                        Cell::from(Span::styled(release_title, primary_style)),
                    ]
                };

                Row::new(cells).style(row_style)
            })
            .collect();

        let streams_table = Table::new(table_rows, widths)
            .header(header)
            .block(streams_block)
            .row_highlight_style(streams_highlight_style)
            .highlight_symbol(streams_highlight_symbol);

        frame.render_stateful_widget(streams_table, streams_area, &mut state.resource_list_state);

        render_scroll_indicator(
            frame,
            streams_area,
            list.len(),
            selected_idx.unwrap_or(0),
            theme,
            state.basic_terminal,
        );
    } else {
        let has_multiple_dubs = details.has_languages();
        let provider_label = state.current_subject_provider().label();

        let waiting_for_language = has_multiple_dubs && !state.language_chosen;
        let has_error = state.stream_error.is_some();

        let is_loading_streams = state.is_fetching_streams
            || state.is_loading
            || state.pending_episode_fetch.is_some()
            || !state.has_streams_settled;

        let msg = if waiting_for_language {
            "Choose an audio track to load streams.".to_string()
        } else if let Some(error) = &state.stream_error {
            if state.active_provider == crate::providers::ProviderKind::Addons {
                error.clone()
            } else if error.contains("No stream sources") || error.contains("not listed") {
                format!("{error}.")
            } else {
                error.clone()
            }
        } else if is_loading_streams {
            let spinner = stream_loading_spinner(state.tick_count, state.basic_terminal);
            if state.basic_terminal {
                format!("Loading streams {spinner}")
            } else {
                format!("{spinner}  Loading streams...")
            }
        } else if state.has_streams_settled && state.selected_resources.is_empty() {
            format!("No stream sources found on {provider_label}.")
        } else {
            let spinner = stream_loading_spinner(state.tick_count, state.basic_terminal);
            if state.basic_terminal {
                format!("Loading streams {spinner}")
            } else {
                format!("{spinner}  Loading streams...")
            }
        };

        let style = if has_error {
            theme.error
        } else if waiting_for_language {
            theme.text_dim
        } else if is_loading_streams {
            theme.lavender
        } else {
            theme.text_dim
        };

        let inner = streams_block.inner(streams_area);
        let pad = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
        let p = Paragraph::new(format!("{}{}", pad, msg))
            .style(style)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(streams_block.clone());
        frame.render_widget(p, streams_area);
    }

    let (mut primary_footer, secondary_footer) =
        details_footer(state, theme, area.width, modal_active);
    let footer_p = if area.width >= DETAILS_FOOTER_SPLIT_THRESHOLD {
        primary_footer.extend(secondary_footer);
        Paragraph::new(Line::from(primary_footer))
    } else {
        if let Some(last) = primary_footer.last_mut() {
            *last = Span::raw("");
        }
        Paragraph::new(vec![
            Line::from(primary_footer),
            Line::from(secondary_footer),
        ])
    }
    .alignment(Alignment::Center);
    frame.render_widget(footer_p, footer_area);

    if state.show_season_download_confirm {
        let summary_strings = season_confirm_summary(state);
        let summary_lines: Vec<Line<'_>> = summary_strings
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        crate::tui::overlay::confirmation(
            frame,
            area,
            " Confirm Season Download ",
            &summary_lines,
            state.season_download_confirm_yes_selected,
            theme,
            state.basic_terminal,
        );
    } else if state.show_episode_download_confirm {
        let summary_strings = episode_confirm_summary(state);
        let summary_lines: Vec<Line<'_>> = summary_strings
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        crate::tui::overlay::confirmation(
            frame,
            area,
            " Confirm Episode Download ",
            &summary_lines,
            state.episode_download_confirm_yes_selected,
            theme,
            state.basic_terminal,
        );
    }
}

pub(crate) fn season_confirm_summary(state: &AppState) -> Vec<String> {
    let title = state
        .selected_details
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or("Series");
    let season_idx = state.selected_season;
    let eps_count = if season_idx > 0 && season_idx <= state.available_episode_numbers.len() {
        state.available_episode_numbers[season_idx - 1].len()
    } else {
        0
    };
    let mut summary = vec![format!(
        "{title} • Season {season_idx} ({eps_count} Episodes)"
    )];
    if let Some(stream) = selected_stream_summary(state) {
        summary.push(format!("Quality: {stream}"));
    }
    let dest = state
        .download_dir
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            crate::service::resolve_download_dir(None)
                .to_string_lossy()
                .to_string()
        });
    summary.push(format!("Save to: {dest}"));
    summary
}

pub(crate) fn episode_confirm_summary(state: &AppState) -> Vec<String> {
    let title = state
        .selected_details
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or("Media");
    let year = state
        .selected_details
        .as_ref()
        .and_then(|d| d.year.as_deref())
        .unwrap_or("");
    let season_idx = state.selected_season;
    let ep_idx = state.selected_episode;
    let is_series = state
        .selected_details
        .as_ref()
        .is_some_and(|d| d.is_series());
    let mut summary = if is_series {
        vec![format!("{title} • Season {season_idx} Episode {ep_idx}")]
    } else if !year.is_empty() && year != "N/A" {
        vec![format!("{title} ({year}) • Movie")]
    } else {
        vec![format!("{title} • Movie")]
    };
    if let Some(stream) = selected_stream_summary(state) {
        summary.push(format!("Quality: {stream}"));
    }
    let dest = state
        .download_dir
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            crate::service::resolve_download_dir(None)
                .to_string_lossy()
                .to_string()
        });
    summary.push(format!("Save to: {dest}"));
    summary
}

fn selected_stream_summary(state: &AppState) -> Option<String> {
    let idx = state.resource_list_state.selected().unwrap_or(0);
    let resource = state.selected_resources.get(idx)?;
    let resolution = if resource.is_multi_resolution() {
        Some("Multi-Res".to_string())
    } else if resource.resolution_u64() > 0 {
        Some(format!("{}p", resource.resolution_u64()))
    } else {
        None
    };
    let codec = resource
        .codec
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(str::to_uppercase);
    let size = resource
        .size_bytes
        .map(|value| crate::tui::text::format_file_size(value as f64));
    let fields = [size, resolution, codec]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    (!fields.is_empty()).then(|| fields.join(" · "))
}

fn clean_language_name(value: &str) -> String {
    let mut name = if value.to_ascii_lowercase().starts_with("original") {
        "Original".to_string()
    } else {
        value
            .replace("dub", "")
            .replace("Dub", "")
            .trim()
            .to_string()
    };
    if name.eq_ignore_ascii_case("ptbr") {
        name = "Portuguese (BR)".to_string();
    } else if name.eq_ignore_ascii_case("esla") {
        name = "Spanish (LA)".to_string();
    }
    name
}

pub fn clean_stream_release_title(filename: &str) -> String {
    let clean = crate::tui::text::clean_stream_text(filename);
    let mut title = clean.as_str();
    let redundant_suffixes = [
        "Multi-Res hevc",
        "Multi-Res h264",
        "Multi-Res",
        "multi-res hevc",
        "multi-res h264",
        "multi-res",
        "HEVC",
        "hevc",
        "H264",
        "h264",
        "x265",
        "x264",
        "X265",
        "X264",
        "1080p",
        "720p",
        "480p",
        "2160p",
        "4K",
    ];
    for _ in 0..3 {
        let before = title;
        for suffix in &redundant_suffixes {
            if let Some(stripped) = title.strip_suffix(suffix) {
                title = stripped.trim_end_matches([' ', '-', '.', '_']);
            }
        }
        if title == before {
            break;
        }
    }
    if title.is_empty() {
        clean
    } else {
        title.to_string()
    }
}

fn pane_title(
    label: &str,
    count: usize,
    _pane: crate::tui::state::DetailsPane,
    focused: bool,
    state: &AppState,
    max_width: u16,
    modal_active: bool,
) -> Line<'static> {
    let marker = if focused && !modal_active {
        focus_title_marker(state.basic_terminal)
    } else {
        ""
    };
    let count_suffix = if count > 0 {
        format!(" ({count})")
    } else {
        String::new()
    };

    let title_full = format!(" {marker}{label}{count_suffix} ");
    let avail = max_width.saturating_sub(2) as usize;
    let title = if avail > 0 && crate::tui::text::width(&title_full) > avail {
        format!(" {marker}{label} ")
    } else {
        title_full
    };

    Line::from(title)
}

fn focused_border_style(theme: &Theme) -> Style {
    theme.lavender
}

fn unfocused_border_style(theme: &Theme) -> Style {
    theme.surface1
}

fn focused_title_style(theme: &Theme) -> Style {
    theme.title
}

fn unfocused_title_style(theme: &Theme) -> Style {
    theme.subtext1
}

fn pane_styles(
    focused: bool,
    modal_active: bool,
    basic_terminal: bool,
    theme: &Theme,
) -> (Style, Style, Style, &'static str) {
    if modal_active {
        (theme.muted, theme.muted, theme.muted, "  ")
    } else {
        (
            if focused {
                focused_border_style(theme)
            } else {
                unfocused_border_style(theme)
            },
            if focused {
                focused_title_style(theme)
            } else {
                unfocused_title_style(theme)
            },
            selection_style(focused, basic_terminal, theme),
            selection_symbol(focused, basic_terminal),
        )
    }
}

fn metadata_style(theme: &Theme) -> Style {
    theme.subtext1
}

fn with_selection_surface(style: Style, basic_terminal: bool, _theme: &Theme) -> Style {
    if basic_terminal {
        style
    } else {
        style.add_modifier(Modifier::BOLD)
    }
}

fn selection_style(focused: bool, basic_terminal: bool, theme: &Theme) -> Style {
    if focused {
        crate::tui::overlay::selection_style(theme, basic_terminal)
    } else {
        theme.subtext1.add_modifier(Modifier::BOLD)
    }
}

fn focus_title_marker(basic_terminal: bool) -> &'static str {
    if basic_terminal { "> " } else { "› " }
}

fn selection_symbol(_focused: bool, _basic_terminal: bool) -> &'static str {
    ""
}

#[allow(clippy::too_many_arguments)]
fn render_workflow(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    details: &MediaDetails,
    has_languages: bool,
    is_series: bool,
    streams_count: usize,
    theme: &Theme,
    modal_active: bool,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let compact = area.width < 100;
    let mut steps = Vec::new();

    if has_languages {
        let active_idx = details
            .dubs
            .iter()
            .position(|dub| {
                dub.subject_id == state.active_subject_id.as_deref().unwrap_or_default()
            })
            .or_else(|| state.language_list_state.selected())
            .unwrap_or(0);

        let language = details
            .dubs
            .get(active_idx)
            .map(|dub| clean_language_name(&dub.language))
            .unwrap_or_else(|| "Choose".to_string());
        steps.push((
            crate::tui::state::DetailsPane::Languages,
            format!("Audio: {language}"),
        ));
    }
    if is_series {
        steps.push((
            crate::tui::state::DetailsPane::Seasons,
            if compact {
                format!("S{}", state.selected_season)
            } else {
                format!("Season {}", state.selected_season)
            },
        ));

        let ep_label = if compact {
            format!("E{}", state.selected_episode)
        } else {
            format!("Episode {}", state.selected_episode)
        };

        steps.push((crate::tui::state::DetailsPane::Episodes, ep_label));
    }
    steps.push((
        crate::tui::state::DetailsPane::Streams,
        format!("Streams: {streams_count}"),
    ));

    if area.width < 60 {
        let position = steps
            .iter()
            .position(|(pane, _)| *pane == state.details_pane)
            .unwrap_or(0);
        let label = steps
            .get(position)
            .map(|(_, label)| label.as_str())
            .unwrap_or("Streams");
        let text = format!("{label}  ·  {}/{}", position + 1, steps.len());
        frame.render_widget(
            Paragraph::new(text)
                .style(if modal_active {
                    theme.muted
                } else {
                    focused_title_style(theme)
                })
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let mut spans = Vec::new();
    let active_index = steps
        .iter()
        .position(|(pane, _)| *pane == state.details_pane)
        .unwrap_or(0);
    for (index, (pane, label)) in steps.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(
                if state.basic_terminal {
                    " > "
                } else {
                    "  ›  "
                },
                if modal_active {
                    theme.muted
                } else {
                    theme.overlay0
                },
            ));
        }
        if *pane == state.details_pane && !modal_active {
            spans.push(Span::styled(
                focus_title_marker(state.basic_terminal),
                theme.lavender.add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(label.clone(), focused_title_style(theme)));
        } else {
            spans.push(Span::styled(
                label.clone(),
                if modal_active {
                    theme.muted
                } else if index < active_index {
                    theme.text
                } else {
                    theme.overlay0
                },
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn footer_group(
    key: &'static str,
    action: &str,
    prominent: bool,
    theme: &Theme,
    modal_active: bool,
) -> Vec<Span<'static>> {
    if modal_active {
        vec![
            Span::styled("[", theme.muted),
            Span::styled(key, theme.muted),
            Span::styled("] ", theme.muted),
            Span::styled(action.to_string(), theme.muted),
            Span::raw("   "),
        ]
    } else {
        vec![
            Span::styled("[", theme.overlay0),
            Span::styled(key, theme.shortcut),
            Span::styled("] ", theme.overlay0),
            Span::styled(
                action.to_string(),
                if prominent {
                    theme.text
                } else {
                    theme.subtext1
                },
            ),
            Span::raw("   "),
        ]
    }
}

fn details_footer(
    state: &AppState,
    theme: &Theme,
    width: u16,
    modal_active: bool,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let compact = width < DETAILS_FOOTER_SPLIT_THRESHOLD;
    let is_streams = state.details_pane == crate::tui::state::DetailsPane::Streams;
    let is_seasons = state.details_pane == crate::tui::state::DetailsPane::Seasons;
    let is_episodes = state.details_pane == crate::tui::state::DetailsPane::Episodes;

    let is_favorited = state.is_selected_details_favorited();
    let fav_label = if is_favorited {
        "Unfavorite"
    } else {
        "Favorite"
    };
    let show_provider = state.active_provider != crate::providers::ProviderKind::Addons
        && !state
            .selected_details
            .as_ref()
            .is_some_and(|d| d.id.provider == crate::providers::ProviderKind::Addons);

    let (enter_label, d_label) = if is_streams {
        ("Play", Some(if compact { "Save" } else { "Download" }))
    } else if is_seasons {
        (
            "Select",
            Some(if compact {
                "Download"
            } else {
                "Download Season"
            }),
        )
    } else if is_episodes {
        (
            "Select",
            Some(if compact {
                "Download"
            } else {
                "Download Episode"
            }),
        )
    } else {
        ("Select", None)
    };

    let mut primary = footer_group("Enter", enter_label, true, theme, modal_active);
    if let Some(d) = d_label {
        primary.extend(footer_group("d", d, false, theme, modal_active));
    }

    let mut secondary = footer_group("f", fav_label, false, theme, modal_active);
    if show_provider {
        secondary.extend(footer_group(
            crate::tui::text::CTRL_P_STR,
            "Provider",
            false,
            theme,
            modal_active,
        ));
    }
    if !is_streams {
        secondary.extend(footer_group("Tab", "Streams", false, theme, modal_active));
    }
    secondary.extend(footer_group("Esc", "Back", false, theme, modal_active));
    if let Some(last) = secondary.last_mut() {
        *last = Span::raw("");
    }
    (primary, secondary)
}

fn render_scroll_indicator(
    frame: &mut Frame,
    area: Rect,
    content_length: usize,
    position: usize,
    theme: &Theme,
    basic_terminal: bool,
) {
    let scroll_area = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 0,
    });
    crate::tui::widgets::render_scrollbar(
        frame,
        scroll_area,
        content_length,
        scroll_area.height as usize,
        position,
        theme,
        basic_terminal,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AudioTrackOption, Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release,
        Season, SourceMirror,
    };

    #[test]
    fn test_stream_loading_spinner_frames() {
        assert_eq!(stream_loading_spinner(0, false), "⠋");
        assert_eq!(stream_loading_spinner(1, false), "⠙");
        assert_eq!(stream_loading_spinner(9, false), "⠏");
        assert_eq!(stream_loading_spinner(10, false), "⠋");
        assert_eq!(stream_loading_spinner(0, true), "..");
    }

    #[test]
    fn test_stream_list_tabular_alignment_and_headers() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "dune2".to_string(),
                },
                title: "Dune: Part Two".to_string(),
                media_type: MediaType::Movie,
                year: Some("2024".to_string()),
                description: Some("Epic continuation".to_string()),
                tagline: None,
                imdb_rating: Some("8.6".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("166m".to_string()),
                genres: vec!["Sci-Fi".to_string()],
                seasons: vec![],
                dubs: vec![],
            }),
            selected_resources: vec![
                Release {
                    provider: ProviderKind::MovieBox,
                    filename: "Dune.Part.Two.2024.1080p.WEBRip.x265".to_string(),
                    quality: Some("1080p".to_string()),
                    codec: Some("hevc".to_string()),
                    language: Some("English".to_string()),
                    size_bytes: Some(1073741824),
                    season: None,
                    episode: None,
                    mirrors: vec![SourceMirror {
                        label: "Pahe.in".to_string(),
                        resolver_url: "https://example.com/1".to_string(),
                        headers: vec![],
                        direct_file: false,
                    }],
                    resource_id: None,
                },
                Release {
                    provider: ProviderKind::MovieBox,
                    filename: "Dune.Part.Two.2024.1080p.WEBRip.x264".to_string(),
                    quality: Some("1080p".to_string()),
                    codec: Some("h264".to_string()),
                    language: Some("English".to_string()),
                    size_bytes: Some(250000000),
                    season: None,
                    episode: None,
                    mirrors: vec![SourceMirror {
                        label: "GalaxyRG".to_string(),
                        resolver_url: "https://example.com/2".to_string(),
                        headers: vec![],
                        direct_file: false,
                    }],
                    resource_id: None,
                },
                Release {
                    provider: ProviderKind::MovieBox,
                    filename: "Dune.Part.Two.2024.720p.WEBRip.x265".to_string(),
                    quality: Some("720p".to_string()),
                    codec: Some("hevc".to_string()),
                    language: Some("English".to_string()),
                    size_bytes: Some(500000000),
                    season: None,
                    episode: None,
                    mirrors: vec![SourceMirror {
                        label: "PSA".to_string(),
                        resolver_url: "https://example.com/3".to_string(),
                        headers: vec![],
                        direct_file: false,
                    }],
                    resource_id: None,
                },
            ],
            details_pane: crate::tui::state::DetailsPane::Streams,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("RES"));
        assert!(content.contains("SIZE"));
        assert!(content.contains("MEDIA TAGS"));
        assert!(content.contains("SOURCE"));
        assert!(!content.contains("DURATION"));
        assert!(!content.contains("--:--"));
        assert!(content.contains("1080p"));
        assert!(content.contains("720p"));
        assert!(content.contains("1.0GB"));
        assert!(content.contains("HEVC"));
        assert!(content.contains("Pahe.in"));

        let header_line = buffer
            .content()
            .chunks(120)
            .find(|line| {
                let text: String = line.iter().map(|c| c.symbol()).collect();
                text.contains("RES") && text.contains("SIZE") && text.contains("SOURCE")
            })
            .map(|line| line.iter().map(|c| c.symbol()).collect::<String>())
            .expect("header line must exist");

        let res_idx = header_line.find("RES").unwrap();
        let size_idx = header_line.find("SIZE").unwrap();
        let tags_idx = header_line.find("MEDIA TAGS").unwrap();
        let source_idx = header_line.find("SOURCE").unwrap();
        let release_idx = header_line.find("RELEASE").unwrap();
        assert_eq!(size_idx - res_idx, 9);
        assert_eq!(tags_idx - size_idx, 10);
        assert_eq!(source_idx - tags_idx, 19);
        assert_eq!(release_idx - source_idx, 17);

        let data_line = buffer
            .content()
            .chunks(120)
            .find(|line| {
                let text: String = line.iter().map(|c| c.symbol()).collect();
                text.contains("1080p") && text.contains("1.0GB")
            })
            .map(|line| line.iter().map(|c| c.symbol()).collect::<String>())
            .expect("data line must exist");
        let data_size_idx = data_line.find("1.0GB").unwrap();
        assert_eq!(data_size_idx, size_idx);
    }

    #[test]
    fn test_redundant_source_labels_fallback_to_provider_origin() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "test_show".to_string(),
                },
                title: "Test Show".to_string(),
                media_type: MediaType::Series,
                year: Some("2024".to_string()),
                description: None,
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![],
                dubs: vec![],
            }),
            selected_resources: vec![Release {
                provider: ProviderKind::MovieBox,
                filename: "Summer Dress S01E02 Multi-Res hevc".to_string(),
                quality: Some("multi".to_string()),
                codec: Some("hevc".to_string()),
                language: None,
                size_bytes: Some(400 * 1024 * 1024),
                season: Some(1),
                episode: Some(2),
                mirrors: vec![SourceMirror {
                    label: "Multi-Res hevc".to_string(),
                    resolver_url: "https://example.com/stream.mpd".to_string(),
                    headers: vec![],
                    direct_file: true,
                }],
                resource_id: None,
            }],
            details_pane: crate::tui::state::DetailsPane::Streams,
            ..Default::default()
        };
        let theme = Theme::dracula();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("MovieBox CDN"));
        assert!(!content.contains("Multi-Res hevc"));
        assert!(content.contains("Summer Dress S01E02"));
    }

    #[test]
    fn test_stream_selection_highlighting_isolated_to_row() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "dune2".to_string(),
                },
                title: "Dune: Part Two".to_string(),
                media_type: MediaType::Movie,
                year: Some("2024".to_string()),
                description: Some("Epic continuation".to_string()),
                tagline: None,
                imdb_rating: Some("8.6".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("166m".to_string()),
                genres: vec!["Sci-Fi".to_string()],
                seasons: vec![],
                dubs: vec![],
            }),
            selected_resources: vec![
                Release {
                    provider: ProviderKind::MovieBox,
                    filename: "Dune.Part.Two.2024.1080p.WEBRip.x265".to_string(),
                    quality: Some("1080p".to_string()),
                    codec: Some("hevc".to_string()),
                    language: Some("English".to_string()),
                    size_bytes: Some(1073741824),
                    season: None,
                    episode: None,
                    mirrors: vec![],
                    resource_id: None,
                },
                Release {
                    provider: ProviderKind::MovieBox,
                    filename: "Dune.Part.Two.2024.720p.WEBRip.x264".to_string(),
                    quality: Some("720p".to_string()),
                    codec: Some("h264".to_string()),
                    language: Some("English".to_string()),
                    size_bytes: Some(500000000),
                    season: None,
                    episode: None,
                    mirrors: vec![],
                    resource_id: None,
                },
            ],
            details_pane: crate::tui::state::DetailsPane::Streams,
            ..Default::default()
        };
        state.resource_list_state.select(Some(0));
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut found_header = false;
        let mut header_has_selection_bg = false;

        let sel_style = selection_style(true, false, &theme);

        for y in 0..buffer.area.height {
            let row_symbols: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            if row_symbols.contains("RES") && row_symbols.contains("SIZE") {
                found_header = true;
                for x in 0..buffer.area.width {
                    let cell = &buffer[(x, y)];
                    if cell.style().bg == sel_style.bg && sel_style.bg.is_some() {
                        header_has_selection_bg = true;
                    }
                }
            }
        }

        assert!(found_header, "Streams table header row should be rendered");
        assert!(
            !header_has_selection_bg,
            "Streams table header should not have selection background"
        );
    }

    #[test]
    fn test_details_error_state_rendering() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            details_error: Some("Failed to fetch details: network timeout".to_string()),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Error Loading Details"));
        assert!(content.contains("Retry"));
        assert!(content.contains("Back"));
    }

    #[test]
    fn test_synopsis_wrap_clamping() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "test".to_string(),
                },
                title: "Test Movie".to_string(),
                media_type: MediaType::Movie,
                year: Some("2024".to_string()),
                description: Some("A very long synopsis that will definitely exceed the single line boundary and should be clamped cleanly with an ellipsis without overflowing the paragraph bounds.".to_string()),
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![],
                dubs: vec![],
            }),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_details_footer_contextual_hints() {
        let theme = Theme::mocha();
        let mut state = AppState::default();

        state.details_pane = crate::tui::state::DetailsPane::Streams;
        state.subtitle_list = vec![("English".to_string(), "http://sub".to_string())];
        let (primary, secondary) = details_footer(&state, &theme, 120, false);
        let mut all_spans = primary;
        all_spans.extend(secondary);
        let footer_text: String = all_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(footer_text.contains("[Enter] Play"));
        assert!(!footer_text.contains("[o] Open With"));
        assert!(footer_text.contains("[d] Download"));
        assert!(footer_text.contains("[f] Favorite"));
        assert!(footer_text.contains("Provider"));
        assert!(footer_text.contains("[Esc] Back"));

        let (compact_primary, compact_secondary) = details_footer(&state, &theme, 90, false);
        let mut compact_spans = compact_primary;
        compact_spans.extend(compact_secondary);
        let compact_text: String = compact_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(compact_text.contains("[d] Save"));

        state.active_provider = crate::providers::ProviderKind::Addons;
        let (addon_primary, addon_secondary) = details_footer(&state, &theme, 120, false);
        let mut addon_spans = addon_primary;
        addon_spans.extend(addon_secondary);
        let addon_text: String = addon_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(!addon_text.contains("Provider"));
        state.active_provider = crate::providers::ProviderKind::MovieBox;

        state.details_pane = crate::tui::state::DetailsPane::Seasons;
        let (primary, secondary) = details_footer(&state, &theme, 120, false);
        let mut all_spans = primary;
        all_spans.extend(secondary);
        let footer_text: String = all_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(footer_text.contains("[Enter] Select"));
        assert!(footer_text.contains("[d] Download Season"));
        assert!(footer_text.contains("[f] Favorite"));
        assert!(footer_text.contains("[Tab] Streams"));
        assert!(footer_text.contains("[Esc] Back"));

        state.details_pane = crate::tui::state::DetailsPane::Episodes;
        let (primary, secondary) = details_footer(&state, &theme, 120, false);
        let mut all_spans = primary;
        all_spans.extend(secondary);
        let footer_text: String = all_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(footer_text.contains("[Enter] Select"));
        assert!(footer_text.contains("[d] Download Episode"));
        assert!(footer_text.contains("[f] Favorite"));
        assert!(footer_text.contains("[Tab] Streams"));
        assert!(footer_text.contains("[Esc] Back"));
    }

    #[test]
    fn test_episode_list_zero_padding_and_alignment() {
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            active_subject_id: Some("test_series".to_string()),
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "test_series".to_string(),
                },
                title: "Test Series".to_string(),
                media_type: MediaType::Series,
                year: Some("2024".to_string()),
                description: None,
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![Season {
                    number: 1,
                    episodes: vec![
                        Episode {
                            season: 1,
                            number: 1,
                            title: None,
                        },
                        Episode {
                            season: 1,
                            number: 2,
                            title: None,
                        },
                        Episode {
                            season: 1,
                            number: 10,
                            title: None,
                        },
                    ],
                }],
                dubs: vec![],
            }),
            available_seasons: vec![Season {
                number: 1,
                episodes: vec![
                    Episode {
                        season: 1,
                        number: 1,
                        title: None,
                    },
                    Episode {
                        season: 1,
                        number: 2,
                        title: None,
                    },
                    Episode {
                        season: 1,
                        number: 10,
                        title: None,
                    },
                ],
            }],
            available_episode_numbers: vec![vec![1, 2, 10]],
            details_pane: crate::tui::state::DetailsPane::Episodes,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Episode 01"));
        assert!(content.contains("Episode 02"));
        assert!(content.contains("Episode 10"));
        assert!(!content.contains("· ·"));
        assert!(!content.contains("·  EP"));
    }

    #[test]
    fn test_details_header_metadata_suppresses_empty_duration_and_double_middot() {
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            active_subject_id: Some("test_series_metadata".to_string()),
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "test_series_metadata".to_string(),
                },
                title: "Breaking Bad".to_string(),
                media_type: MediaType::Series,
                year: Some("2008".to_string()),
                description: None,
                tagline: None,
                imdb_rating: Some("9.5".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("".to_string()),
                genres: vec![],
                seasons: vec![Season {
                    number: 1,
                    episodes: vec![],
                }],
                dubs: vec![],
            }),
            available_seasons: vec![Season {
                number: 1,
                episodes: vec![],
            }],
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("2008"));
        assert!(content.contains("Series"));
        assert!(content.contains("9.5"));
        assert!(!content.contains("·  ·"));
        assert!(!content.contains("· ·"));
    }

    #[test]
    fn test_responsive_selector_panes_single_pane_on_narrow_screen() {
        use crate::tui::state::DetailsPane;
        let all_panes = vec![
            DetailsPane::Languages,
            DetailsPane::Seasons,
            DetailsPane::Episodes,
        ];

        let narrow = visible_selector_panes(&all_panes, DetailsPane::Languages, 50);
        assert_eq!(narrow, vec![DetailsPane::Languages]);

        let narrow_season = visible_selector_panes(&all_panes, DetailsPane::Seasons, 60);
        assert_eq!(narrow_season, vec![DetailsPane::Seasons]);
        let wide = visible_selector_panes(&all_panes, DetailsPane::Languages, 100);
        assert_eq!(wide.len(), 3);
    }

    #[test]
    fn test_pane_title_compact_width_does_not_overflow() {
        use crate::tui::state::DetailsPane;
        let state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "dubs_test".to_string(),
                },
                title: "Dubs Test".to_string(),
                media_type: MediaType::Movie,
                year: None,
                description: None,
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![],
                dubs: vec![
                    AudioTrackOption {
                        subject_id: "1".to_string(),
                        language: "Original".to_string(),
                        label: "Original".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "2".to_string(),
                        language: "Hindi".to_string(),
                        label: "Hindi".to_string(),
                    },
                ],
            }),
            basic_terminal: false,
            ..Default::default()
        };

        let title_wide = pane_title("Audio", 2, DetailsPane::Languages, true, &state, 40, false);
        assert_eq!(title_wide.to_string(), " › Audio (2) ");

        let title_unfocused =
            pane_title("Audio", 2, DetailsPane::Languages, false, &state, 40, false);
        assert_eq!(title_unfocused.to_string(), " Audio (2) ");

        let title_narrow = pane_title("Audio", 2, DetailsPane::Languages, true, &state, 10, false);
        assert_eq!(title_narrow.to_string(), " › Audio ");
    }
    #[test]
    fn test_details_screen_renders_in_narrow_terminal_without_clipping() {
        let backend = ratatui::backend::TestBackend::new(50, 35);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            active_subject_id: Some("breaking_bad".to_string()),
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "breaking_bad".to_string(),
                },
                title: "Breaking Bad".to_string(),
                media_type: MediaType::Series,
                year: Some("2008".to_string()),
                genres: vec!["Crime".to_string(), "Drama".to_string(), "Thriller".to_string()],
                description: Some("A chemistry teacher diagnosed with inoperable lung cancer turns to manufacturing and selling methamphetamine.".to_string()),
                tagline: None,
                imdb_rating: Some("9.5".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("45m".to_string()),
                seasons: vec![Season {
                    number: 1,
                    episodes: (1..=7).map(|n| Episode { season: 1, number: n, title: None }).collect(),
                }],
                dubs: vec![
                    AudioTrackOption { subject_id: "1".to_string(), language: "Original".to_string(), label: "Original".to_string() },
                    AudioTrackOption { subject_id: "2".to_string(), language: "Hindi".to_string(), label: "Hindi".to_string() },
                    AudioTrackOption { subject_id: "3".to_string(), language: "Spanish (LA)".to_string(), label: "Spanish (LA)".to_string() },
                    AudioTrackOption { subject_id: "4".to_string(), language: "Portuguese (Brazil)".to_string(), label: "Portuguese (Brazil)".to_string() },
                ],
            }),
            available_seasons: vec![Season {
                number: 1,
                episodes: (1..=7).map(|n| Episode { season: 1, number: n, title: None }).collect(),
            }],
            available_episode_numbers: vec![vec![1, 2, 3]],
            details_pane: crate::tui::state::DetailsPane::Languages,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Breaking Bad"));
        assert!(content.contains("Portuguese (Brazil)"));
    }
    #[test]
    fn test_details_header_compact_content_height() {
        let details = MediaDetails {
            id: ProviderMediaId {
                provider: ProviderKind::MovieBox,
                value: "ek_deewane".to_string(),
            },
            title: "Ek Deewane Ki Deewaniyat".to_string(),
            media_type: MediaType::Movie,
            year: Some("2025".to_string()),
            description: Some("When a powerful politician falls for a strong-willed superstar, their passionate romance quickly spirals into a dangerous game of obsession, pride, and heartbreak.".to_string()),
            tagline: None,
            imdb_rating: Some("4.6".to_string()),
            director: None,
            stars: None,
            prints: None,
            audios: None,
            poster_url: Some("https://example.com/poster.jpg".to_string()),
            duration: Some("2h 20m".to_string()),
            genres: vec![],
            seasons: vec![],
            dubs: vec![
                AudioTrackOption {
                    subject_id: "1".to_string(),
                    language: "Original Audio".to_string(),
                    label: "Original Audio".to_string(),
                },
                AudioTrackOption {
                    subject_id: "2".to_string(),
                    language: "Hindi dub".to_string(),
                    label: "Hindi dub".to_string(),
                },
            ],
        };

        let area = Rect::new(0, 0, 120, 30);
        let tier = DetailsLayoutTier::for_area(area);
        let height = tier.header_height(area, Some(&details));
        assert_eq!(height, 7);
    }

    #[test]
    fn test_details_audio_fallback_from_dubs() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "ek_deewane".to_string(),
                },
                title: "Ek Deewane Ki Deewaniyat".to_string(),
                media_type: MediaType::Movie,
                year: Some("2025".to_string()),
                description: Some(
                    "When a powerful politician falls for a strong-willed superstar.".to_string(),
                ),
                tagline: None,
                imdb_rating: Some("4.6".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: Some("https://example.com/poster.jpg".to_string()),
                duration: Some("2h 20m".to_string()),
                genres: vec![],
                seasons: vec![],
                dubs: vec![
                    AudioTrackOption {
                        subject_id: "1".to_string(),
                        language: "Original Audio".to_string(),
                        label: "Original Audio".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "2".to_string(),
                        language: "Hindi dub".to_string(),
                        label: "Hindi dub".to_string(),
                    },
                ],
            }),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Ek Deewane Ki Deewaniyat"));
        assert!(content.contains("2025"));
        assert!(content.contains("Movie"));
        assert!(content.contains("Audio (2)"));
        assert!(content.contains("Original"));
        assert!(content.contains("MovieBox"));
        assert!(content.contains("No Art"));
    }

    #[test]
    fn test_details_no_flash_on_unsettled_streams() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "ek_deewane".to_string(),
                },
                title: "Ek Deewane Ki Deewaniyat".to_string(),
                media_type: MediaType::Movie,
                year: Some("2025".to_string()),
                description: Some("Description".to_string()),
                tagline: None,
                imdb_rating: Some("4.6".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("2h 20m".to_string()),
                genres: vec![],
                seasons: vec![],
                dubs: vec![
                    AudioTrackOption {
                        subject_id: "1".to_string(),
                        language: "Original".to_string(),
                        label: "Original".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "2".to_string(),
                        language: "Hindi".to_string(),
                        label: "Hindi".to_string(),
                    },
                ],
            }),
            language_chosen: true,
            has_streams_settled: false,
            is_fetching_streams: false,
            is_loading: false,
            selected_resources: vec![],
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let content_unsettled = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(content_unsettled.contains("Loading streams"));
        assert!(!content_unsettled.contains("No stream sources found"));

        state.has_streams_settled = true;
        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let content_settled = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(content_settled.contains("No stream sources found"));
    }

    #[test]
    fn test_details_poster_no_art_basic_terminal() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            basic_terminal: true,
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "sample".to_string(),
                },
                title: "Sample Movie".to_string(),
                media_type: MediaType::Movie,
                year: Some("2024".to_string()),
                description: Some("A movie synopsis for testing.".to_string()),
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![],
                dubs: vec![],
            }),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("[No Art]"));
    }

    #[test]
    fn test_details_metadata_audio_summary() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "bb_test".to_string(),
                },
                title: "Breaking Bad".to_string(),
                media_type: MediaType::Series,
                year: Some("2008".to_string()),
                description: Some("Chemistry teacher".to_string()),
                tagline: None,
                imdb_rating: Some("9.5".to_string()),
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec!["Crime".to_string(), "Drama".to_string()],
                seasons: vec![],
                dubs: vec![
                    AudioTrackOption {
                        subject_id: "1".to_string(),
                        language: "Original".to_string(),
                        label: "Original".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "2".to_string(),
                        language: "Hindi".to_string(),
                        label: "Hindi".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "3".to_string(),
                        language: "Spanish (LA)".to_string(),
                        label: "Spanish (LA)".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "4".to_string(),
                        language: "Portuguese (BR)".to_string(),
                        label: "Portuguese (BR)".to_string(),
                    },
                ],
            }),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Audio (4)"));
        assert!(!content.contains("Audio: 4 Tracks"));
        assert!(!content.contains("Audio: Original, Hindi, Spanish (LA), Portuguese (BR)"));
    }

    #[test]
    fn test_details_screen_layout_wide_and_compact() {
        let wide_area = Rect::new(0, 0, 120, 35);
        let wide_layout = details_screen_layout(wide_area, None);
        assert_eq!(wide_layout.workflow_area.height, 0);

        let compact_area = Rect::new(0, 0, 75, 35);
        let compact_layout = details_screen_layout(compact_area, None);
        assert_eq!(compact_layout.workflow_area.height, 1);
    }

    #[test]
    fn test_stream_table_deduplication() {
        assert_eq!(
            clean_stream_release_title("Pilot S01E01 Multi-Res hevc"),
            "Pilot S01E01"
        );
        assert_eq!(
            clean_stream_release_title("Breaking Bad S01E01 1080p HEVC"),
            "Breaking Bad S01E01"
        );
        assert_eq!(
            clean_stream_release_title("Inception 2010 1080p x264"),
            "Inception 2010"
        );
        assert_eq!(clean_stream_release_title("Short Film"), "Short Film");
    }

    #[test]
    fn test_episode_item_spacing() {
        let check_sym = "✓ ";
        let play_sym = "▶ ";
        let unwatched_sym = "  ";

        assert_eq!(crate::tui::text::width(check_sym), 2);
        assert_eq!(crate::tui::text::width(play_sym), 2);
        assert_eq!(crate::tui::text::width(unwatched_sym), 2);
    }

    #[test]
    fn test_details_background_unfocused_when_modal_active() {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = AppState {
            selected_details: Some(MediaDetails {
                id: ProviderMediaId {
                    provider: ProviderKind::MovieBox,
                    value: "sample".to_string(),
                },
                title: "Sample Movie".to_string(),
                media_type: MediaType::Movie,
                year: Some("2024".to_string()),
                description: Some("Synopsis".to_string()),
                tagline: None,
                imdb_rating: None,
                director: None,
                stars: None,
                prints: None,
                audios: None,
                poster_url: None,
                duration: None,
                genres: vec![],
                seasons: vec![],
                dubs: vec![
                    AudioTrackOption {
                        subject_id: "sample".to_string(),
                        language: "Original".to_string(),
                        label: "Original".to_string(),
                    },
                    AudioTrackOption {
                        subject_id: "sample-hi".to_string(),
                        language: "Hindi".to_string(),
                        label: "Hindi".to_string(),
                    },
                ],
            }),
            selected_resources: vec![Release {
                provider: ProviderKind::MovieBox,
                filename: "Sample Stream Multi-Res hevc".to_string(),
                quality: Some("Multi".to_string()),
                codec: Some("HEVC".to_string()),
                language: None,
                size_bytes: Some(1_600_000_000),
                season: None,
                episode: None,
                mirrors: vec![],
                resource_id: Some("res-1".to_string()),
            }],
            details_pane: crate::tui::state::DetailsPane::Streams,
            show_episode_download_confirm: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.language_list_state.select(Some(0));
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Confirm Episode Download"));
        assert!(!content.contains("› Streams"));
        assert!(content.contains("Multi"));
        assert!(content.contains("Hindi"));

        for y in 0..30 {
            for x in 0..120 {
                let cell = &buffer[(x, y)];
                if x < 20
                    && cell.symbol() == "M"
                    && x + 4 < 120
                    && buffer[(x + 1, y)].symbol() == "u"
                {
                    assert_eq!(
                        cell.style().fg,
                        theme.overlay1.fg,
                        "Multi badge foreground should be overlay1"
                    );
                }
                if cell.symbol() == "H" && x + 4 < 120 && buffer[(x + 1, y)].symbol() == "i" {
                    assert_eq!(
                        cell.style().fg,
                        theme.muted.fg,
                        "Hindi text foreground should be muted"
                    );
                }
            }
        }
    }
}
