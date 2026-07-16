use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(23), // Metadata header (Title, Release, Synopsis)
            Constraint::Min(4),     // Stream links lists
        ])
        .split(area);

    let details_json = match &state.selected_details {
        Some(d) => d,
        None => {
            let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

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

    let title = details_json
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Unknown");
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
        .and_then(|g| g.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "N/A".to_string());
    let duration = details_json
        .get("duration")
        .and_then(|d| d.as_str())
        .unwrap_or("N/A");
    let country = details_json
        .get("countryName")
        .and_then(|c| c.as_str())
        .unwrap_or("N/A");
    let content_rating = details_json
        .get("contentRating")
        .and_then(|c| c.as_str())
        .unwrap_or("N/A");
    let imdb_rating = details_json
        .get("imdbRatingValue")
        .and_then(|r| {
            r.as_f64()
                .map(|rf| rf.to_string())
                .or_else(|| r.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "N/A".to_string());

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Subject Info ")
        .title_style(theme.title)
        .border_style(theme.border);

    let inner_area = details_block.inner(chunks[0]);
    frame.render_widget(details_block.clone(), chunks[0]);

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(15), Constraint::Min(1)])
        .split(inner_area);

    let top_area = v_chunks[0];
    let synopsis_area = v_chunks[1];

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(top_area);

    let poster_area = h_chunks[0];
    let meta_area = h_chunks[2];

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
    } else {
        let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let current_spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

        let placeholder = Paragraph::new(format!("\n\n\n\n\n  {} Loading Art...", current_spinner))
            .style(theme.text_dim)
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, poster_area);
    }

    let mut staff_list = Vec::new();
    if let Some(staff) = details_json.get("staffList").and_then(|s| s.as_array()) {
        for person in staff.iter().take(3) {
            if let Some(name) = person.get("name").and_then(|n| n.as_str()) {
                staff_list.push(name.to_string());
            }
        }
    }
    let cast = if staff_list.is_empty() {
        "N/A".to_string()
    } else {
        staff_list.join(", ")
    };

    let mut dubs_list = Vec::new();
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
                if !dubs_list.contains(&name) {
                    dubs_list.push(name);
                }
            }
        }
    }

    dubs_list.sort_by(|a, b| {
        if a == "Original" {
            std::cmp::Ordering::Less
        } else if b == "Original" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });
    let dubbing_str = if dubs_list.is_empty() {
        "N/A".to_string()
    } else {
        dubs_list.join(", ")
    };

    let viewers = details_json
        .get("viewers")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let label_style = theme.header;
    let val_style = theme.text;

    let meta_lines = vec![
        Line::from(vec![
            Span::styled("Title:    ", label_style),
            Span::styled(title, val_style),
        ]),
        Line::from(vec![
            Span::styled("Type:     ", label_style),
            Span::styled(type_str, val_style),
        ]),
        Line::from(vec![
            Span::styled("Genre:    ", label_style),
            Span::styled(genres, val_style),
        ]),
        Line::from(vec![
            Span::styled("Released: ", label_style),
            Span::styled(year, val_style),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", label_style),
            Span::styled(duration, val_style),
        ]),
        Line::from(vec![
            Span::styled("Rating:   ", label_style),
            Span::styled(
                format!("{} (IMDb: {})", content_rating, imdb_rating),
                val_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("Country:  ", label_style),
            Span::styled(country, val_style),
        ]),
        Line::from(vec![
            Span::styled("Cast:     ", label_style),
            Span::styled(cast, val_style),
        ]),
        Line::from(vec![
            Span::styled("Dubbing:  ", label_style),
            Span::styled(dubbing_str, val_style),
        ]),
        Line::from(vec![
            Span::styled("Viewers:  ", label_style),
            Span::styled(viewers.to_string(), val_style),
        ]),
    ];

    let synopsis_lines = vec![
        Line::from(vec![Span::styled("Synopsis:", label_style)]),
        Line::from(vec![]),
    ];

    frame.render_widget(
        Paragraph::new(meta_lines).wrap(Wrap { trim: true }),
        meta_area,
    );

    let mut syn_lines = synopsis_lines;
    syn_lines.push(Line::from(vec![Span::styled(intro, theme.text_dim)]));
    let intro_p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
    frame.render_widget(intro_p, synopsis_area);

    let has_languages = if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
        dubs.len() > 1
    } else {
        false
    };

    let (is_series, bottom_chunks) = if type_val == 2 && !state.available_seasons.is_empty() {
        if has_languages {
            (
                true,
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(15), // Languages
                        Constraint::Percentage(15), // Seasons
                        Constraint::Percentage(15), // Episodes
                        Constraint::Percentage(55), // Streams
                    ])
                    .split(chunks[1]),
            )
        } else {
            (
                true,
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(15), // Seasons
                        Constraint::Percentage(15), // Episodes
                        Constraint::Percentage(70), // Streams
                    ])
                    .split(chunks[1]),
            )
        }
    } else {
        if has_languages {
            (
                false,
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(20), // Languages
                        Constraint::Percentage(80), // Streams
                    ])
                    .split(chunks[1]),
            )
        } else {
            (
                false,
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(100)])
                    .split(chunks[1]),
            )
        }
    };

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
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .title(" Audio/Dub ")
                    .border_style(lang_border),
            )
            .highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50)))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(lang_list, bottom_chunks[0], &mut state.language_list_state);
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
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .title(" Seasons ")
                    .border_style(seasons_border),
            )
            .highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50)))
            .highlight_symbol(">> ");

        let seasons_area = if has_languages {
            bottom_chunks[1]
        } else {
            bottom_chunks[0]
        };
        frame.render_stateful_widget(seasons_list, seasons_area, &mut state.season_list_state);

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
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .title(" Episodes ")
                    .border_style(eps_border),
            )
            .highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50)))
            .highlight_symbol(">> ");

        let eps_area = if has_languages {
            bottom_chunks[2]
        } else {
            bottom_chunks[1]
        };
        frame.render_stateful_widget(eps_list, eps_area, &mut state.episode_list_state);
    }

    let streams_area = if is_series {
        if has_languages {
            bottom_chunks[3]
        } else {
            bottom_chunks[2]
        }
    } else {
        if has_languages {
            bottom_chunks[1]
        } else {
            bottom_chunks[0]
        }
    };
    let streams_border = if state.details_pane == crate::tui::state::DetailsPane::Streams {
        theme.border_focus
    } else {
        theme.border
    };

    let streams_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Streams | [Enter/P] Play  [D] Download  [C] Copy Link  [B/Esc] Back ")
        .title_style(theme.title)
        .border_style(streams_border);

    let table = match &state.selected_resources {
        Some(res) => {
            if let Some(list) = res.get("list").and_then(|l| l.as_array()) {
                let rows: Vec<Row> = list
                    .iter()
                    .map(|file| {
                        let resolution =
                            file.get("resolution").and_then(|r| r.as_i64()).unwrap_or(0);
                        let codec = file
                            .get("codecName")
                            .and_then(|c| c.as_str())
                            .unwrap_or("None");
                        let upload_by = file
                            .get("uploadBy")
                            .and_then(|u| u.as_str())
                            .unwrap_or("None");
                        let size_str = file.get("size").and_then(|s| s.as_str()).unwrap_or("0");
                        let size_formatted = if let Ok(bytes) = size_str.parse::<f64>() {
                            let mb = bytes / 1024.0 / 1024.0;
                            if mb > 1024.0 {
                                format!("{:.1} GB", mb / 1024.0)
                            } else {
                                format!("{:.0} MB", mb)
                            }
                        } else {
                            "Unknown".to_string()
                        };

                        let subs_count = file
                            .get("extCaptions")
                            .and_then(|c| c.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let subs_str = if subs_count > 0 {
                            format!("{} subs", subs_count)
                        } else {
                            "None".to_string()
                        };

                        Row::new(vec![
                            Cell::from(format!("{}p", resolution)).style(theme.header),
                            Cell::from(codec).style(theme.text),
                            Cell::from(size_formatted).style(theme.text_dim),
                            Cell::from(subs_str).style(if subs_count > 0 {
                                theme.border_focus
                            } else {
                                theme.text_dim
                            }),
                            Cell::from(upload_by).style(theme.text),
                        ])
                    })
                    .collect();

                let widths = [
                    Constraint::Length(6),
                    Constraint::Length(6),
                    Constraint::Length(9),
                    Constraint::Length(8),
                    Constraint::Min(15),
                ];

                Table::new(rows, widths)
                    .header(
                        Row::new(vec!["Res", "Codec", "Size", "Subs", "Uploader"])
                            .style(theme.title),
                    )
                    .block(streams_block)
                    .row_highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50))) // Cool subtle glow animation
                    .highlight_symbol(">> ")
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
                let r = Row::new(vec![Cell::from(msg)]);
                Table::new(vec![r], [Constraint::Percentage(100)]).block(streams_block)
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
                let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];
                format!("{} Loading streams...", spinner)
            } else if state.status_message.to_lowercase().contains("failed")
                || state.status_message.to_lowercase().contains("error")
            {
                state.status_message.clone()
            } else if has_multiple_dubs && !state.language_chosen {
                "Select a language/dubbing and press Enter to load streams.".to_string()
            } else {
                "Failed to load streams.".to_string()
            };
            let r = Row::new(vec![Cell::from(msg).style(if state.is_loading {
                theme.text_dim
            } else if has_multiple_dubs && !state.language_chosen {
                theme.border_focus
            } else {
                theme.error
            })]);
            Table::new(vec![r], [Constraint::Percentage(100)]).block(streams_block)
        }
    };

    frame.render_stateful_widget(table, streams_area, &mut state.resource_list_state);
}
