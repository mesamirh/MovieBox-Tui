use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(12),
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);
    let bottom_area = chunks[2];

    let details_json = match &state.selected_details {
        Some(d) => d,
        None => {
            let spinner = if state.basic_terminal {
                let frames = ['-', '\\', '|', '/'];
                frames[(state.tick_count as usize) % frames.len()]
            } else {
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                frames[(state.tick_count as usize) % frames.len()]
            };

            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(area);

            let loading_p = Paragraph::new(format!("{} Loading details...", spinner))
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

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            ratatui::widgets::BorderType::Plain
        } else {
            ratatui::widgets::BorderType::Rounded
        })
        .border_style(theme.border)
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));

    let inner_area = details_block.inner(chunks[0]);
    frame.render_widget(details_block.clone(), chunks[0]);

    let poster_width = if state.image_supported {
        (inner_area.height as f32 * 1.33).ceil() as u16
    } else {
        0
    };

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(poster_width),
            Constraint::Length(if state.image_supported { 4 } else { 0 }),
            Constraint::Min(1),
        ])
        .split(inner_area);

    let poster_height = ((poster_width as f32) / 1.33).ceil() as u16;
    let poster_area = ratatui::layout::Rect {
        height: h_chunks[0].height.min(poster_height),
        ..h_chunks[0]
    };
    let right_area = h_chunks[2];

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
            frame.render_widget(ratatui_image::Image::new(proto), poster_area);
        }
    } else if state.image_supported {
        let current_spinner = if state.basic_terminal {
            let frames = ['-', '\\', '|', '/'];
            frames[(state.tick_count as usize) % frames.len()]
        } else {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            frames[(state.tick_count as usize) % frames.len()]
        };

        let placeholder_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.muted);

        let inner = placeholder_block.inner(poster_area);

        let (pad, msg) = if state.is_loading {
            let p = "\n".repeat((inner.height.saturating_sub(1) / 2) as usize);
            (p, format!("{}\nLoading Art", current_spinner))
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

    let meta_line = Line::from(vec![Span::styled(
        format!("{} • {} • {}{}", type_str, year, country, duration_str),
        theme.text,
    )]);

    let genre_line = Line::from(vec![Span::styled(genres.to_string(), theme.text_dim)]);

    let top_meta = vec![
        title_line,
        meta_line,
        genre_line,
        Line::from(vec![]),
        Line::from(vec![Span::styled("Synopsis", theme.text)]),
    ];

    let meta_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(right_area);

    let meta_p = Paragraph::new(top_meta).wrap(Wrap { trim: true });
    frame.render_widget(meta_p, meta_chunks[0]);

    let syn_lines = vec![Line::from(vec![Span::styled(
        intro,
        theme.text_dim.add_modifier(ratatui::style::Modifier::DIM),
    )])];
    let intro_p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
    frame.render_widget(intro_p, meta_chunks[1]);

    let has_languages = if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
        dubs.len() > 1
    } else {
        false
    };

    let is_series = type_val == 2 && !state.available_seasons.is_empty();

    let mut c = Vec::new();
    if has_languages {
        c.push(Constraint::Length(22));
    }
    if is_series {
        c.push(Constraint::Length(18));
        c.push(Constraint::Length(18));
    }
    c.push(Constraint::Min(1));

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(c)
        .split(bottom_area);

    let mut chunk_idx = 0;
    let mut lang_area = None;
    let mut seasons_area = None;
    let mut eps_area = None;

    if has_languages {
        lang_area = Some(bottom_chunks[chunk_idx]);
        chunk_idx += 1;
    }
    if is_series {
        seasons_area = Some(bottom_chunks[chunk_idx]);
        chunk_idx += 1;
        eps_area = Some(bottom_chunks[chunk_idx]);
        chunk_idx += 1;
    }
    let streams_area = bottom_chunks[chunk_idx];

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

        let lang_border = if state.details_pane == crate::tui::state::DetailsPane::Languages {
            theme.border_focus
        } else {
            theme.border
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
                    .title(" Audio ")
                    .border_style(lang_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        if let Some(area) = lang_area {
            frame.render_stateful_widget(lang_list, area, &mut state.language_list_state);
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

        let seasons_border = if state.details_pane == crate::tui::state::DetailsPane::Seasons {
            theme.border_focus
        } else {
            theme.border
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
                    .title(" Seasons ")
                    .border_style(seasons_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        if let Some(area) = seasons_area {
            frame.render_stateful_widget(seasons_list, area, &mut state.season_list_state);
        }

        let ep_items: Vec<ListItem> = if let Some(season) = state
            .available_seasons
            .get(state.season_list_state.selected().unwrap_or(0))
        {
            let max_ep = season.get("maxEp").and_then(|m| m.as_i64()).unwrap_or(1);
            (1..=max_ep)
                .map(|ep| ListItem::new(format!("Episode {}", ep)).style(theme.text))
                .collect()
        } else {
            vec![]
        };

        let eps_border = if state.details_pane == crate::tui::state::DetailsPane::Episodes {
            theme.border_focus
        } else {
            theme.border
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
                    .title(" Episodes ")
                    .border_style(eps_border)
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        if let Some(area) = eps_area {
            frame.render_stateful_widget(eps_list, area, &mut state.episode_list_state);
        }
    }

    let streams_border = if state.details_pane == crate::tui::state::DetailsPane::Streams {
        theme.border_focus
    } else {
        theme.border
    };

    let mut streams_count = 0;
    if let Some(res) = &state.selected_resources {
        if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
            streams_count = list.len();
        }
    }

    let streams_title = if streams_count > 0 {
        format!(" Streams • {} Available ", streams_count)
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
        .title_style(theme.title)
        .border_style(streams_border)
        .padding(ratatui::widgets::Padding::horizontal(1));

    match &state.selected_resources {
        Some(res) => {
            if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                let mut prev_quality = String::new();
                let selected_idx = state.resource_list_state.selected();

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
                        let _upload_by = file
                            .get("uploadBy")
                            .and_then(|u| u.as_str())
                            .unwrap_or("None");
                        let size_str = file.get("size").and_then(|s| s.as_str()).unwrap_or("0");

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
                            if state.basic_terminal { "> " } else { "▌ " }
                        } else {
                            "  "
                        };

                        let stream_style = if is_selected {
                            theme.highlight
                        } else {
                            theme.text_dim
                        };

                        let stream_line = ratatui::text::Line::from(vec![
                            ratatui::text::Span::styled(pointer, theme.accent),
                            ratatui::text::Span::styled(
                                format!("{:<7} {:<6} ", size_formatted, codec),
                                stream_style,
                            ),
                        ]);

                        let mut lines = vec![];
                        if is_first_of_quality {
                            if i > 0 {
                                lines.push(ratatui::text::Line::from(""));
                            }
                            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                                quality_str,
                                theme.accent,
                            )));
                        }

                        lines.push(stream_line);
                        ListItem::new(lines)
                    })
                    .collect();

                let l = List::new(list_items)
                    .block(streams_block.clone())
                    .highlight_symbol("");

                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(
                        streams_count
                            .saturating_sub(streams_area.height.saturating_sub(2) as usize),
                    )
                    .position(state.resource_list_state.selected().unwrap_or(0));

                let scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"));

                frame.render_stateful_widget(l, streams_area, &mut state.resource_list_state);
                frame.render_stateful_widget(
                    scrollbar,
                    streams_area.inner(ratatui::layout::Margin {
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut scrollbar_state,
                );
            } else {
                let has_multiple_dubs = state
                    .selected_details
                    .as_ref()
                    .and_then(|d| d.get("dubs"))
                    .and_then(|d| d.as_array())
                    .is_some_and(|a| a.len() > 1);
                let msg = if has_multiple_dubs && !state.language_chosen {
                    "Please select a language dubbing first."
                } else {
                    "No streaming files available."
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

            let msg = if state.is_loading {
                let spinner = if state.basic_terminal {
                    let frames = ['-', '\\', '|', '/'];
                    frames[(state.tick_count as usize) % frames.len()]
                } else {
                    let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                    frames[(state.tick_count as usize) % frames.len()]
                };
                format!("{} Loading streams...", spinner)
            } else if has_multiple_dubs && !state.language_chosen {
                "Please select a language dubbing from the Audio panel to view streams.".to_string()
            } else if state.status_message.to_lowercase().contains("failed")
                || state.status_message.to_lowercase().contains("error")
            {
                state.status_message.clone()
            } else {
                "Failed to load streams.".to_string()
            };

            let style = if state.is_loading || (has_multiple_dubs && !state.language_chosen) {
                theme.text_dim
            } else {
                theme.error
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
    }
    if !state.selected_resources.is_some() {
        frame.render_widget(streams_block, streams_area);
    }
    if state.subtitle_popup {
        let popup_width = 50;
        let popup_height = std::cmp::min(15, state.subtitle_list.len() as u16 + 2);

        let area = frame.area();
        let popup_area = ratatui::layout::Rect {
            x: area.width.saturating_sub(popup_width) / 2,
            y: area.height.saturating_sub(popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let items: Vec<ratatui::widgets::ListItem> = state
            .subtitle_list
            .iter()
            .map(|(name, _)| ratatui::widgets::ListItem::new(name.clone()))
            .collect();

        let list = ratatui::widgets::List::new(items)
            .block(
                ratatui::widgets::Block::default()
                    .title(" Select Subtitle ")
                    .title_style(theme.title)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(if state.basic_terminal {
                        ratatui::widgets::BorderType::Plain
                    } else {
                        ratatui::widgets::BorderType::Rounded
                    })
                    .border_style(theme.border),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        frame.render_stateful_widget(list, popup_area, &mut state.subtitle_list_state);
    }

    if state.player_picker_popup {
        let popup_width = 24;
        let popup_height = std::cmp::min(15, state.available_players.len() as u16 + 2);

        let area = frame.area();
        let popup_area = ratatui::layout::Rect {
            x: area.width.saturating_sub(popup_width) / 2,
            y: area.height.saturating_sub(popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let items: Vec<ratatui::widgets::ListItem> = state
            .available_players
            .iter()
            .map(|k| {
                let text = match k {
                    crate::tui::state::PlayerKind::Mpv => "mpv",
                    crate::tui::state::PlayerKind::Iina => "IINA",
                    crate::tui::state::PlayerKind::Vlc => "VLC",
                };
                ratatui::widgets::ListItem::new(text)
            })
            .collect();

        let list = ratatui::widgets::List::new(items)
            .block(
                ratatui::widgets::Block::default()
                    .title(" Open With ")
                    .title_style(theme.title)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(if state.basic_terminal {
                        ratatui::widgets::BorderType::Plain
                    } else {
                        ratatui::widgets::BorderType::Rounded
                    })
                    .border_style(theme.border),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        frame.render_stateful_widget(list, popup_area, &mut state.player_picker_state);
    }

    let footer_text =
        "Enter Play      o Open With...      D Download      C Copy Link      Esc Back";
    let footer_p = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(theme.muted);
    frame.render_widget(footer_p, chunks[3]);
}
