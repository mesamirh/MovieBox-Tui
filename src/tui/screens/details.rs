use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailsLayoutTier {
    Wide,
    Medium,
    Narrow,
    Tiny,
}

impl DetailsLayoutTier {
    fn for_area(area: Rect) -> Self {
        if area.width < 60 || area.height < 24 {
            Self::Tiny
        } else if area.width < 80 {
            Self::Narrow
        } else if area.width < 120 {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    fn header_height(self, area: Rect, details: Option<&serde_json::Value>) -> u16 {
        let (minimum, maximum, synopsis_limit, reserved_width) = match self {
            Self::Wide => (9, 12, 3, 30),
            Self::Medium => (8, 11, 2, 24),
            Self::Narrow => (7, 9, 2, 4),
            Self::Tiny => (4, 6, 1, 4),
        };
        let available_maximum = area.height.saturating_sub(match self {
            Self::Wide => 18,
            Self::Medium => 17,
            Self::Narrow => 16,
            Self::Tiny => 12,
        });
        let maximum = maximum.min(available_maximum.max(minimum));

        let Some(details) = details else {
            return minimum.min(maximum);
        };
        let synopsis = details
            .get("description")
            .and_then(|value| value.as_str())
            .or_else(|| details.get("intro").and_then(|value| value.as_str()))
            .unwrap_or_default();
        let text_width = area.width.saturating_sub(reserved_width).max(20) as usize;
        let title = details
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let title_rows = (crate::tui::text::width(title) + 14)
            .div_ceil(text_width)
            .clamp(1, 2);
        let synopsis_rows = crate::tui::text::width(synopsis)
            .div_ceil(text_width)
            .clamp(1, synopsis_limit);
        let metadata_rows = match self {
            Self::Wide | Self::Medium => 5,
            Self::Narrow => 4,
            Self::Tiny => 3,
        };
        let content_rows = metadata_rows + title_rows.saturating_sub(1) + synopsis_rows;
        (content_rows as u16 + 2).clamp(minimum, maximum)
    }

    fn footer_height(self) -> u16 {
        if matches!(self, Self::Wide) { 1 } else { 2 }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let tier = DetailsLayoutTier::for_area(area);
    let header_height = tier.header_height(area, state.selected_details.as_ref());
    let footer_height = tier.footer_height();
    let chunks = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(footer_height),
    ])
    .split(area);
    let workflow_area = chunks[1];
    let bottom_area = chunks[2];

    let details_json = match &state.selected_details {
        Some(d) => d,
        None => {
            let dots = match (state.tick_count / 4) % 4 {
                0 => "",
                1 => ".",
                2 => "..",
                _ => "...",
            };

            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(area);

            let loading_p = Paragraph::new(format!("Loading details{dots}"))
                .alignment(ratatui::layout::Alignment::Center)
                .style(theme.text_dim);

            frame.render_widget(loading_p, vertical_chunks[1]);
            return;
        }
    };

    let raw_title = details_json
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Unknown Title");
    let title = crate::tui::app::clean_moviebox_title(raw_title);
    let intro = details_json
        .get("description")
        .and_then(|d| d.as_str())
        .or_else(|| details_json.get("intro").and_then(|i| i.as_str()))
        .unwrap_or("No description available.");
    let year = details_json
        .get("releaseDate")
        .and_then(|y| y.as_str())
        .or_else(|| details_json.get("year").and_then(|y| y.as_str()))
        .unwrap_or("N/A");
    let type_val = details_json
        .get("subjectType")
        .and_then(|s| s.as_i64())
        .or_else(|| details_json.get("stype").and_then(|s| s.as_i64()))
        .unwrap_or(1);
    let type_str = if type_val == 2 { "Series" } else { "Movie" };

    let genres = details_json
        .get("genre")
        .and_then(|g| {
            if let Some(a) = g.as_array() {
                let joined = a
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if joined.is_empty() {
                    None
                } else {
                    Some(joined)
                }
            } else if let Some(s) = g.as_str() {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "N/A".to_string());
    let duration = details_json
        .get("duration")
        .and_then(|d| d.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("N/A");
    let country = details_json
        .get("countryName")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("N/A");

    let imdb_rating = details_json
        .get("imdbRatingValue")
        .and_then(|r| {
            r.as_f64()
                .map(|rf| rf.to_string())
                .or_else(|| r.as_str().map(|s| s.to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "N/A".to_string());
    let tagline = details_json
        .get("tagline")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            ratatui::widgets::BorderType::Plain
        } else {
            ratatui::widgets::BorderType::Rounded
        })
        .border_style(theme.surface1)
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

    let inner_area = details_block.inner(chunks[0]);
    frame.render_widget(details_block.clone(), chunks[0]);

    let show_poster = !matches!(tier, DetailsLayoutTier::Tiny)
        && inner_area.height >= 6
        && inner_area.width >= 60;
    let poster_width = if show_poster {
        let width_for_height = state
            .poster_image
            .as_ref()
            .zip(state.image_picker.as_ref())
            .map(|(image, picker)| {
                let font = picker.font_size();
                let target_pixel_height =
                    u64::from(inner_area.height) * u64::from(font.height.max(1));
                let target_pixel_width = target_pixel_height * u64::from(image.width())
                    / u64::from(image.height().max(1));
                target_pixel_width.div_ceil(u64::from(font.width.max(1))) as u16
            })
            .unwrap_or_else(|| (inner_area.height as f32 * 1.5).ceil() as u16);
        width_for_height.clamp(10, 26).min(inner_area.width / 3)
    } else {
        0
    };

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(poster_width),
            Constraint::Length(if show_poster { 2 } else { 0 }),
            Constraint::Min(1),
        ])
        .split(inner_area);

    let poster_area = h_chunks[0];
    let right_area = h_chunks[2];

    if show_poster && state.image_supported {
        if let Some(img) = &state.poster_image {
            if state.poster_protocol.as_ref().map(|(r, _)| *r) != Some(poster_area)
                && let Some(picker) = &mut state.image_picker
            {
                let size = ratatui::layout::Size::new(poster_area.width, poster_area.height);
                if let Ok(proto) =
                    picker.new_protocol(img.clone(), size, ratatui_image::Resize::Fit(None))
                {
                    state.poster_protocol = Some((poster_area, proto));
                }
            }
            if let Some((_, proto)) = &state.poster_protocol {
                if !state.show_help {
                    frame.render_widget(ratatui_image::Image::new(proto), poster_area);
                }
            }
        } else {
            let dots = match (state.tick_count / 4) % 4 {
                0 => "",
                1 => ".",
                2 => "..",
                _ => "...",
            };

            let placeholder_block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.muted);

            let inner = placeholder_block.inner(poster_area);

            let (pad, msg) = if state.is_loading {
                let p = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
                (p, format!("Loading Art{dots}"))
            } else {
                let p = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
                (p, title.to_string())
            };

            let placeholder = Paragraph::new(format!("{}{}", pad, msg))
                .style(theme.text_dim)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(placeholder_block);
            frame.render_widget(placeholder, poster_area);
        }
    } else if show_poster {
        let placeholder_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.muted);

        let inner = placeholder_block.inner(poster_area);
        let lines = if inner.height >= 5 {
            let pad_top = "\n".repeat((inner.height.saturating_sub(5) / 2) as usize);
            format!("{pad_top}Poster preview\nunsupported\n\nUse a graphics-\ncapable terminal")
        } else {
            "Poster\nunsupported".to_string()
        };

        let placeholder = Paragraph::new(lines)
            .style(theme.text_dim)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(placeholder_block);
        frame.render_widget(placeholder, poster_area);
    }

    let title_line = Line::from(vec![
        Span::styled(
            title.to_string(),
            theme.text.add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled("   ", theme.text),
        Span::styled(format!("★ IMDb {}", imdb_rating), theme.rating),
    ]);

    let duration_str = if duration.is_empty() || duration == "N/A" {
        "".to_string()
    } else {
        format!(" • {}", duration)
    };

    let mut metadata = vec![type_str.to_string()];
    if year != "N/A" {
        metadata.push(year.to_string());
    }
    if country != "N/A" {
        metadata.push(country.to_string());
    }
    if !duration_str.is_empty() {
        metadata.push(duration.to_string());
    }
    let meta_line = Line::from(vec![Span::styled(
        metadata.join(" • "),
        metadata_style(theme),
    )]);

    let genre_line = Line::from(vec![Span::styled(
        genres.to_string(),
        metadata_style(theme),
    )]);

    let mut top_meta = vec![
        title_line,
        meta_line,
        genre_line,
        Line::from(vec![Span::styled(
            tagline.unwrap_or_default(),
            theme
                .overlay1
                .add_modifier(ratatui::style::Modifier::ITALIC),
        )]),
        Line::from(vec![Span::styled("Synopsis", theme.title)]),
    ];
    if matches!(tier, DetailsLayoutTier::Tiny) {
        top_meta.truncate(3);
    } else if matches!(tier, DetailsLayoutTier::Narrow) {
        top_meta.truncate(4);
    }
    let title_width = crate::tui::text::width(&title)
        + crate::tui::text::width(&format!("   ★ IMDb {}", imdb_rating));
    let title_rows = title_width
        .div_ceil(right_area.width.max(1) as usize)
        .clamp(1, 2);
    let metadata_height = (top_meta.len() + title_rows.saturating_sub(1)) as u16;

    let meta_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(metadata_height), Constraint::Min(0)])
        .split(right_area);

    let meta_p = Paragraph::new(top_meta).wrap(Wrap { trim: true });
    frame.render_widget(meta_p, meta_chunks[0]);

    let synopsis_capacity =
        (meta_chunks[1].width as usize).saturating_mul(meta_chunks[1].height as usize);
    let synopsis = truncate_with_ellipsis(intro, synopsis_capacity);
    let syn_lines = vec![Line::from(vec![Span::styled(synopsis, theme.overlay1)])];
    let intro_p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
    frame.render_widget(intro_p, meta_chunks[1]);

    let has_languages = if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
        dubs.len() > 1
    } else {
        false
    };

    let is_series = type_val == 2 && !state.available_seasons.is_empty();
    let streams_count = state
        .selected_resources
        .as_ref()
        .and_then(|resources| resources.get("list"))
        .and_then(|list| list.as_array())
        .map_or(0, Vec::len);

    render_workflow(
        frame,
        workflow_area,
        state,
        details_json,
        has_languages,
        is_series,
        streams_count,
        theme,
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
        if matches!(tier, DetailsLayoutTier::Narrow | DetailsLayoutTier::Tiny) {
            available_selector_panes
                .iter()
                .copied()
                .filter(|pane| *pane == state.details_pane)
                .collect::<Vec<_>>()
        } else {
            available_selector_panes
        };

    let selector_height = if visible_selector_panes.is_empty() {
        0
    } else {
        let episode_count = state
            .available_episode_numbers
            .get(state.season_list_state.selected().unwrap_or(0))
            .map_or(0, Vec::len);
        let language_count = details_json
            .get("dubs")
            .and_then(|dubs| dubs.as_array())
            .map_or(0, Vec::len);
        language_count
            .max(state.available_seasons.len())
            .max(episode_count)
            .min(4) as u16
            + 2
    };

    let lower_chunks = Layout::vertical([Constraint::Length(selector_height), Constraint::Min(3)])
        .split(bottom_area);
    let selector_area = lower_chunks[0];
    let streams_area = lower_chunks[1];

    let selector_chunks = if visible_selector_panes.is_empty() {
        Vec::new()
    } else {
        Layout::horizontal(vec![
            Constraint::Ratio(
                1,
                visible_selector_panes.len() as u32
            );
            visible_selector_panes.len()
        ])
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
        let mut lang_items = Vec::new();
        if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
            for dub in dubs {
                if let Some(lang) = dub.get("lanName").and_then(|n| n.as_str()) {
                    let mut name = if lang.to_lowercase().starts_with("original") {
                        "Original".to_string()
                    } else {
                        lang.replace("dub", "")
                            .replace("Dub", "")
                            .trim()
                            .to_string()
                    };
                    if name.to_lowercase() == "ptbr" {
                        name = "Portuguese (BR)".to_string();
                    }
                    lang_items.push(ListItem::new(name).style(theme.text));
                }
            }
        }
        let language_count = lang_items.len();

        let language_focused = state.details_pane == crate::tui::state::DetailsPane::Languages;
        let lang_border = if language_focused {
            focused_border_style(theme)
        } else {
            unfocused_border_style(theme)
        };
        let lang_list = List::new(lang_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(if state.basic_terminal {
                        ratatui::widgets::BorderType::Plain
                    } else {
                        ratatui::widgets::BorderType::Rounded
                    })
                    .title(pane_title(
                        "Audio",
                        language_count,
                        crate::tui::state::DetailsPane::Languages,
                        language_focused,
                        state,
                    ))
                    .title_style(if language_focused {
                        focused_title_style(theme)
                    } else {
                        unfocused_title_style(theme)
                    })
                    .border_style(lang_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(selection_style(
                language_focused,
                state.basic_terminal,
                theme,
            ))
            .highlight_symbol(selection_symbol(language_focused, state.basic_terminal));

        if let Some(area) = lang_area {
            frame.render_stateful_widget(lang_list, area, &mut state.language_list_state);
            render_scroll_indicator(
                frame,
                area,
                language_count,
                state.language_list_state.selected().unwrap_or(0),
                theme,
            );
        }
    }

    if is_series {
        use ratatui::widgets::{List, ListItem};
        let seasons_items: Vec<ListItem> = state
            .available_seasons
            .iter()
            .map(|s| {
                let se_num = s.get("se").and_then(|v| v.as_i64()).unwrap_or(1);
                ListItem::new(format!("Season {}", se_num)).style(theme.text)
            })
            .collect();

        let seasons_focused = state.details_pane == crate::tui::state::DetailsPane::Seasons;
        let seasons_border = if seasons_focused {
            focused_border_style(theme)
        } else {
            unfocused_border_style(theme)
        };
        let seasons_list = List::new(seasons_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(if state.basic_terminal {
                        ratatui::widgets::BorderType::Plain
                    } else {
                        ratatui::widgets::BorderType::Rounded
                    })
                    .title(pane_title(
                        "Seasons",
                        state.available_seasons.len(),
                        crate::tui::state::DetailsPane::Seasons,
                        seasons_focused,
                        state,
                    ))
                    .title_style(if seasons_focused {
                        focused_title_style(theme)
                    } else {
                        unfocused_title_style(theme)
                    })
                    .border_style(seasons_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(selection_style(
                seasons_focused,
                state.basic_terminal,
                theme,
            ))
            .highlight_symbol(selection_symbol(seasons_focused, state.basic_terminal));

        if let Some(area) = seasons_area {
            frame.render_stateful_widget(seasons_list, area, &mut state.season_list_state);
            render_scroll_indicator(
                frame,
                area,
                state.available_seasons.len(),
                state.season_list_state.selected().unwrap_or(0),
                theme,
            );
        }

        let ep_items: Vec<ListItem> = if let Some(ep_numbers) = state
            .available_episode_numbers
            .get(state.season_list_state.selected().unwrap_or(0))
        {
            ep_numbers
                .iter()
                .map(|&ep| ListItem::new(format!("Episode {}", ep)).style(theme.text))
                .collect()
        } else {
            vec![]
        };
        let episode_count = ep_items.len();

        let episodes_focused = state.details_pane == crate::tui::state::DetailsPane::Episodes;
        let eps_border = if episodes_focused {
            focused_border_style(theme)
        } else {
            unfocused_border_style(theme)
        };
        let eps_list = List::new(ep_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(if state.basic_terminal {
                        ratatui::widgets::BorderType::Plain
                    } else {
                        ratatui::widgets::BorderType::Rounded
                    })
                    .title(pane_title(
                        "Episodes",
                        episode_count,
                        crate::tui::state::DetailsPane::Episodes,
                        episodes_focused,
                        state,
                    ))
                    .title_style(if episodes_focused {
                        focused_title_style(theme)
                    } else {
                        unfocused_title_style(theme)
                    })
                    .border_style(eps_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(selection_style(
                episodes_focused,
                state.basic_terminal,
                theme,
            ))
            .highlight_symbol(selection_symbol(episodes_focused, state.basic_terminal));

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
            );
        }
    }

    let streams_focused = state.details_pane == crate::tui::state::DetailsPane::Streams;
    let streams_border = if streams_focused {
        focused_border_style(theme)
    } else {
        unfocused_border_style(theme)
    };

    let streams_title = if streams_count > 0 {
        let selected = state
            .resource_list_state
            .selected()
            .unwrap_or(0)
            .min(streams_count.saturating_sub(1));
        format!(
            " {}Streams · {} available · {}/{} ",
            if streams_focused {
                focus_title_marker(state.basic_terminal)
            } else {
                ""
            },
            streams_count,
            selected + 1,
            streams_count
        )
    } else if streams_focused {
        format!(" {}Streams ", focus_title_marker(state.basic_terminal))
    } else {
        " Streams ".to_string()
    };

    let streams_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            ratatui::widgets::BorderType::Plain
        } else {
            ratatui::widgets::BorderType::Rounded
        })
        .title(ratatui::text::Line::from(streams_title).alignment(Alignment::Left))
        .title_style(if streams_focused {
            focused_title_style(theme)
        } else {
            unfocused_title_style(theme)
        })
        .border_style(streams_border)
        .padding(ratatui::widgets::Padding::horizontal(1));

    match &state.selected_resources {
        Some(res) => {
            if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                let mut prev_quality = String::new();
                let selected_idx = state.resource_list_state.selected();
                let mut quality_counts = std::collections::HashMap::new();
                for file in list {
                    let resolution = file
                        .get("resolution")
                        .and_then(|value| value.as_i64())
                        .unwrap_or(0);
                    *quality_counts.entry(resolution).or_insert(0usize) += 1;
                }

                let list_items: Vec<ListItem> = list
                    .iter()
                    .enumerate()
                    .map(|(i, file)| {
                        let resolution =
                            file.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                        let quality_str = format!("{}p", resolution);

                        let is_first_of_quality = quality_str != prev_quality;
                        prev_quality = quality_str.clone();

                        let codec = file
                            .get("codecName")
                            .and_then(|c| c.as_str())
                            .unwrap_or("None");
                        let upload_by = file
                            .get("uploadBy")
                            .and_then(|u| u.as_str())
                            .unwrap_or("Unknown");
                        let size_str = file.get("size").and_then(|s| s.as_str()).unwrap_or("0");

                        let duration = file.get("duration").and_then(|d| d.as_u64()).unwrap_or(0);
                        let duration_str = if duration > 0 {
                            let hours = duration / 3600;
                            let mins = (duration % 3600) / 60;
                            let secs = duration % 60;
                            if hours > 0 {
                                format!("{:02}:{:02}:{:02}", hours, mins, secs)
                            } else {
                                format!("{:02}:{:02}", mins, secs)
                            }
                        } else {
                            "--:--".to_string()
                        };

                        let size_formatted = if let Ok(bytes) = size_str.parse::<f64>() {
                            let mb = bytes / 1024.0 / 1024.0;
                            if mb > 1024.0 {
                                format!("{:.1}GB", mb / 1024.0)
                            } else {
                                format!("{:.0}MB", mb)
                            }
                        } else {
                            "Unknown".to_string()
                        };

                        let is_selected = Some(i) == selected_idx;
                        let pointer = if is_selected {
                            selection_symbol(streams_focused, state.basic_terminal)
                        } else {
                            "  "
                        };

                        let row_style = if is_selected {
                            selection_style(streams_focused, state.basic_terminal, theme)
                        } else {
                            metadata_style(theme)
                        };
                        let marker_style = if is_selected && streams_focused {
                            with_selection_surface(theme.lavender, state.basic_terminal, theme)
                                .add_modifier(Modifier::BOLD)
                        } else if is_selected {
                            theme.title
                        } else {
                            metadata_style(theme)
                        };
                        let primary_style = if is_selected {
                            with_selection_surface(theme.text, state.basic_terminal, theme)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            theme.text
                        };
                        let secondary_style = if is_selected {
                            with_selection_surface(
                                metadata_style(theme),
                                state.basic_terminal,
                                theme,
                            )
                        } else {
                            metadata_style(theme)
                        };

                        let is_fourk = file.get("_fourk_release").is_some();
                        let language = file
                            .get("language")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Unknown");
                        let source_count = file
                            .get("sourceCount")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0);
                        let stream_width = streams_area.width.saturating_sub(6) as usize;
                        let codec = codec.to_uppercase();
                        let mut stream_spans = vec![
                            Span::styled(pointer, marker_style),
                            Span::styled(format!("{size_formatted:<9}"), primary_style),
                            Span::styled(format!("{codec:<8}"), secondary_style),
                        ];
                        if is_fourk && stream_width >= 58 {
                            stream_spans
                                .push(Span::styled(format!("{language:<16}"), secondary_style));
                            stream_spans.push(Span::styled(
                                format!(
                                    "{source_count} mirror{}",
                                    if source_count == 1 { "" } else { "s" }
                                ),
                                secondary_style,
                            ));
                        } else if is_fourk && stream_width >= 38 {
                            stream_spans.push(Span::styled(language.to_string(), secondary_style));
                        } else if !is_fourk && stream_width >= 64 {
                            let fixed_width = 9 + 8 + 12;
                            let uploader = crate::tui::text::truncate_width(
                                upload_by,
                                stream_width.saturating_sub(fixed_width).max(4),
                            );
                            stream_spans
                                .push(Span::styled(format!("{duration_str:<12}"), secondary_style));
                            stream_spans.push(Span::styled(uploader, secondary_style));
                        } else if !is_fourk && stream_width >= 38 {
                            stream_spans.push(Span::styled(duration_str, secondary_style));
                        }
                        if is_selected {
                            let used_width = stream_spans
                                .iter()
                                .map(|span| crate::tui::text::width(span.content.as_ref()))
                                .sum::<usize>();
                            stream_spans.push(Span::styled(
                                " ".repeat(stream_width.saturating_sub(used_width)),
                                row_style,
                            ));
                        }
                        let stream_line = Line::from(stream_spans);

                        let mut lines = vec![];
                        if is_first_of_quality {
                            if i > 0 {
                                lines.push(ratatui::text::Line::from(""));
                            }
                            let option_count =
                                quality_counts.get(&resolution).copied().unwrap_or(1);
                            lines.push(Line::from(vec![
                                Span::styled(quality_str, group_heading_style(theme)),
                                Span::styled(" · ", theme.overlay0),
                                Span::styled(
                                    format!(
                                        "{} option{}",
                                        option_count,
                                        if option_count == 1 { "" } else { "s" }
                                    ),
                                    metadata_style(theme),
                                ),
                            ]));
                        }

                        lines.push(stream_line);
                        ListItem::new(lines)
                    })
                    .collect();

                let content_height = list_items.iter().map(ListItem::height).sum();
                let l = List::new(list_items).block(streams_block.clone());

                frame.render_stateful_widget(l, streams_area, &mut state.resource_list_state);
                let rendered_position = selected_idx.map_or(0, |selected| {
                    let mut headings = 0;
                    let mut previous = None;
                    for file in list.iter().take(selected.saturating_add(1)) {
                        let resolution = file
                            .get("resolution")
                            .and_then(|value| value.as_i64())
                            .unwrap_or(0);
                        if previous != Some(resolution) {
                            headings += 1;
                            previous = Some(resolution);
                        }
                    }
                    selected + headings
                });
                render_scroll_indicator(
                    frame,
                    streams_area,
                    content_height,
                    rendered_position,
                    theme,
                );
            } else {
                let has_multiple_dubs = state
                    .selected_details
                    .as_ref()
                    .and_then(|d| d.get("dubs"))
                    .and_then(|d| d.as_array())
                    .is_some_and(|a| a.len() > 1);
                let msg = if has_multiple_dubs && !state.language_chosen {
                    "↑↓ pick audio, then Enter to confirm (does not auto-apply)."
                } else {
                    "No streams found — press r to retry."
                };

                let inner = streams_block.inner(streams_area);
                let pad = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
                let p = Paragraph::new(format!("{}{}", pad, msg))
                    .style(theme.text_dim)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(streams_block.clone());
                frame.render_widget(p, streams_area);
            }
        }
        None => {
            let has_multiple_dubs = state
                .selected_details
                .as_ref()
                .and_then(|d| d.get("dubs"))
                .and_then(|d| d.as_array())
                .is_some_and(|a| a.len() > 1);

            let waiting_for_language = has_multiple_dubs && !state.language_chosen;
            let has_error = state.stream_error.is_some();

            let msg = if waiting_for_language {
                "↑↓ pick audio, then Enter to confirm.".to_string()
            } else if let Some(error) = &state.stream_error {
                format!("{error} — press r to retry.")
            } else {
                let dots = match (state.tick_count / 4) % 4 {
                    0 => "",
                    1 => ".",
                    2 => "..",
                    _ => "...",
                };
                format!("Loading streams{dots}")
            };

            let style = if has_error {
                theme.error
            } else {
                theme.text_dim
            };

            if !msg.is_empty() {
                let inner = streams_block.inner(streams_area);
                let pad = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
                let p = Paragraph::new(format!("{}{}", pad, msg))
                    .style(style)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(streams_block.clone());
                frame.render_widget(p, streams_area);
            } else {
                frame.render_widget(streams_block.clone(), streams_area);
            }
        }
    }
    if !state.selected_resources.is_some() {
        frame.render_widget(streams_block, streams_area);
    }
    if state.subtitle_popup || state.is_download_subtitle_popup {
        let items = state
            .subtitle_list
            .iter()
            .map(|(name, _)| {
                if name == "None" {
                    "No subtitles".to_string()
                } else {
                    name.clone()
                }
            })
            .collect::<Vec<_>>();
        crate::tui::overlay::picker(
            frame,
            area,
            &items,
            &mut state.subtitle_list_state,
            crate::tui::overlay::PickerSpec {
                title: "Subtitles",
                confirm_label: if state.is_download_subtitle_popup {
                    "Download"
                } else {
                    "Use"
                },
                minimum_width: 32,
            },
            theme,
            state.basic_terminal,
        );
    }

    if state.player_picker_popup {
        let items = state
            .available_players
            .iter()
            .map(|k| {
                match k {
                    crate::tui::state::PlayerKind::Mpv => "mpv",
                    crate::tui::state::PlayerKind::Iina => "IINA",
                    crate::tui::state::PlayerKind::Vlc => "VLC",
                }
                .to_string()
            })
            .collect::<Vec<_>>();
        crate::tui::overlay::picker(
            frame,
            area,
            &items,
            &mut state.player_picker_state,
            crate::tui::overlay::PickerSpec {
                title: "Open with",
                confirm_label: "Open",
                minimum_width: 24,
            },
            theme,
            state.basic_terminal,
        );
    }

    let (mut primary_footer, secondary_footer) = details_footer(state, theme, area.width);
    let footer_p = if matches!(tier, DetailsLayoutTier::Wide) {
        primary_footer.extend(secondary_footer);
        Paragraph::new(Line::from(primary_footer))
    } else {
        Paragraph::new(vec![
            Line::from(primary_footer),
            Line::from(secondary_footer),
        ])
    }
    .alignment(Alignment::Center);
    frame.render_widget(footer_p, chunks[3]);

    if state.show_season_download_confirm {
        let season_idx = state.selected_season;
        let eps_count = if season_idx > 0 && season_idx <= state.available_episode_numbers.len() {
            state.available_episode_numbers[season_idx - 1].len()
        } else {
            0
        };
        crate::tui::overlay::confirmation(
            frame,
            area,
            "Download season",
            &[
                Line::from(format!("Season {season_idx}")),
                Line::from(format!("{eps_count} episodes")),
            ],
            state.season_download_confirm_yes_selected,
            theme,
            state.basic_terminal,
        );
    } else if state.show_episode_download_confirm {
        let season_idx = state.selected_season;
        let ep_idx = state.selected_episode;
        let mut summary = if type_val == 2 {
            vec![Line::from(format!(
                "Season {season_idx} · Episode {ep_idx}"
            ))]
        } else {
            vec![Line::from("Download this movie")]
        };
        if let Some(stream) = selected_stream_summary(state) {
            summary.push(Line::from(stream));
        }
        crate::tui::overlay::confirmation(
            frame,
            area,
            if type_val == 2 {
                "Download episode"
            } else {
                "Download movie"
            },
            &summary,
            state.episode_download_confirm_yes_selected,
            theme,
            state.basic_terminal,
        );
    }
}

fn selected_stream_summary(state: &AppState) -> Option<String> {
    let resource = state
        .selected_resources
        .as_ref()?
        .get("list")?
        .as_array()?
        .get(state.resource_list_state.selected().unwrap_or(0))?;
    let resolution = resource
        .get("resolution")
        .and_then(|value| value.as_i64())
        .filter(|value| *value > 0)
        .map(|value| format!("{value}p"));
    let codec = resource
        .get("codecName")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_uppercase);
    let size = resource
        .get("size")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .map(|bytes| {
            let megabytes = bytes / 1024.0 / 1024.0;
            if megabytes > 1024.0 {
                format!("{:.1} GB", megabytes / 1024.0)
            } else {
                format!("{megabytes:.0} MB")
            }
        });
    let fields = [size, resolution, codec]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    (!fields.is_empty()).then(|| fields.join(" · "))
}

fn truncate_with_ellipsis(value: &str, capacity: usize) -> String {
    if capacity == 0 {
        return String::new();
    }
    if crate::tui::text::width(value) <= capacity {
        return value.to_string();
    }
    if capacity == 1 {
        return "…".to_string();
    }
    format!(
        "{}…",
        crate::tui::text::truncate_width(value, capacity.saturating_sub(1))
    )
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

fn pane_title(
    label: &str,
    count: usize,
    pane: crate::tui::state::DetailsPane,
    focused: bool,
    state: &AppState,
) -> Line<'static> {
    let marker = if focused {
        focus_title_marker(state.basic_terminal)
    } else {
        ""
    };
    let mut title = format!(" {marker}{label} · {count}");
    if focused {
        let mut panes = Vec::new();
        let has_languages = state
            .selected_details
            .as_ref()
            .and_then(|details| details.get("dubs"))
            .and_then(|dubs| dubs.as_array())
            .is_some_and(|dubs| dubs.len() > 1);
        if has_languages {
            panes.push(crate::tui::state::DetailsPane::Languages);
        }
        if !state.available_seasons.is_empty() {
            panes.push(crate::tui::state::DetailsPane::Seasons);
            panes.push(crate::tui::state::DetailsPane::Episodes);
        }
        panes.push(crate::tui::state::DetailsPane::Streams);
        if let Some(position) = panes.iter().position(|candidate| *candidate == pane) {
            title.push_str(&format!("  {}/{}", position + 1, panes.len()));
        }
    }
    title.push(' ');
    Line::from(title)
}

fn theme_color(style: Style, fallback: ratatui::style::Color) -> ratatui::style::Color {
    style.fg.unwrap_or(fallback)
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

fn metadata_style(theme: &Theme) -> Style {
    theme.subtext1
}

fn group_heading_style(theme: &Theme) -> Style {
    theme.lavender.add_modifier(Modifier::BOLD)
}

fn with_selection_surface(style: Style, basic_terminal: bool, theme: &Theme) -> Style {
    if basic_terminal {
        style
    } else {
        style.bg(theme_color(theme.surface0, theme.base))
    }
}

fn selection_style(focused: bool, basic_terminal: bool, theme: &Theme) -> Style {
    if focused {
        let style =
            with_selection_surface(theme.text, basic_terminal, theme).add_modifier(Modifier::BOLD);
        if basic_terminal {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        }
    } else {
        theme.text.add_modifier(Modifier::BOLD)
    }
}

fn focus_title_marker(basic_terminal: bool) -> &'static str {
    if basic_terminal { "> " } else { "● " }
}

fn selection_symbol(focused: bool, basic_terminal: bool) -> &'static str {
    if focused {
        if basic_terminal { "> " } else { "▌ " }
    } else if basic_terminal {
        "* "
    } else {
        "· "
    }
}

#[allow(clippy::too_many_arguments)]
fn render_workflow(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    details: &serde_json::Value,
    has_languages: bool,
    is_series: bool,
    streams_count: usize,
    theme: &Theme,
) {
    let compact = area.width < 100;
    let mut steps = Vec::new();

    if has_languages {
        let language = details
            .get("dubs")
            .and_then(|dubs| dubs.as_array())
            .and_then(|dubs| dubs.get(state.language_list_state.selected().unwrap_or(0)))
            .and_then(|dub| dub.get("lanName"))
            .and_then(|name| name.as_str())
            .map(clean_language_name)
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
        steps.push((
            crate::tui::state::DetailsPane::Episodes,
            if compact {
                format!("E{}", state.selected_episode)
            } else {
                format!("Episode {}", state.selected_episode)
            },
        ));
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
                .style(focused_title_style(theme))
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
                theme.overlay0,
            ));
        }
        if *pane == state.details_pane {
            spans.push(Span::styled(
                focus_title_marker(state.basic_terminal),
                theme.lavender.add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(label.clone(), focused_title_style(theme)));
        } else {
            spans.push(Span::styled(
                label.clone(),
                if index < active_index {
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
    action: &'static str,
    prominent: bool,
    theme: &Theme,
) -> Vec<Span<'static>> {
    vec![
        Span::styled("[", theme.overlay0),
        Span::styled(key, theme.shortcut),
        Span::styled("] ", theme.overlay0),
        Span::styled(
            action,
            if prominent {
                theme.text
            } else {
                theme.subtext1
            },
        ),
        Span::raw("   "),
    ]
}

fn details_footer(
    state: &AppState,
    theme: &Theme,
    width: u16,
) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let compact = width < 80;
    let very_compact = width < 45;
    let mut primary = footer_group(
        "Tab",
        if compact { "Pane" } else { "Next pane" },
        false,
        theme,
    );
    primary.extend(footer_group("↑↓", "Move", false, theme));
    if !very_compact {
        let enter_label = match state.details_pane {
            crate::tui::state::DetailsPane::Streams => "Play",
            crate::tui::state::DetailsPane::Languages => {
                if compact {
                    "Confirm"
                } else {
                    "Confirm audio"
                }
            }
            crate::tui::state::DetailsPane::Seasons => {
                if compact {
                    "Episodes"
                } else {
                    "Open episodes"
                }
            }
            crate::tui::state::DetailsPane::Episodes => {
                if compact {
                    "Streams"
                } else {
                    "Open streams"
                }
            }
        };
        primary.extend(footer_group("Enter", enter_label, true, theme));
    }

    let mut secondary = Vec::new();
    if very_compact {
        secondary.extend(footer_group(
            "Enter",
            if state.details_pane == crate::tui::state::DetailsPane::Streams {
                "Play"
            } else {
                "Select"
            },
            true,
            theme,
        ));
    } else {
        if state.details_pane == crate::tui::state::DetailsPane::Streams {
            secondary.extend(footer_group(
                "o",
                if compact { "Open" } else { "Open with" },
                false,
                theme,
            ));
        }
        if !matches!(
            state.details_pane,
            crate::tui::state::DetailsPane::Languages
        ) {
            secondary.extend(footer_group(
                "d",
                if compact { "Save" } else { "Download" },
                false,
                theme,
            ));
        }
        if !very_compact {
            secondary.extend(footer_group(
                "r",
                if compact { "Retry" } else { "Refresh" },
                false,
                theme,
            ));
        }
    }
    secondary.extend(footer_group("Esc", "Back", false, theme));

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
) {
    let viewport_length = area.height.saturating_sub(2) as usize;
    if content_length <= viewport_length || viewport_length == 0 {
        return;
    }

    let mut state = ScrollbarState::default()
        .content_length(content_length)
        .viewport_content_length(viewport_length)
        .position(position);
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .thumb_style(theme.lavender)
        .track_style(theme.surface1)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"));
    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut state,
    );
}
