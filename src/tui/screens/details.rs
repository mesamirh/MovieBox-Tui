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
            Constraint::Length(14),
            Constraint::Length(1),
            Constraint::Min(10),
        ])
        .split(area);
    let bottom_area = chunks[2];

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
        .and_then(|g| {
            if let Some(a) = g.as_array() {
                let joined = a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ");
                if joined.is_empty() { None } else { Some(joined) }
            } else if let Some(s) = g.as_str() {
                if s.is_empty() { None } else { Some(s.to_string()) }
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
    let content_rating = details_json
        .get("contentRating")
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
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(" Subject Info ")
        .title_style(theme.title)
        .border_style(theme.border);

    let inner_area = details_block.inner(chunks[0]);
    frame.render_widget(details_block.clone(), chunks[0]);

    let poster_width = (inner_area.height as f32 * 2.2).ceil() as u16;

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(poster_width),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner_area);

    let poster_area = h_chunks[0];
    let right_area = h_chunks[2];

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(1),
        ])
        .split(right_area);

    let meta_area = right_chunks[0];
    let synopsis_area = right_chunks[1];

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
        let current_spinner = if state.basic_terminal {
            let frames = ['-', '\\', '|', '/'];
            frames[(state.tick_count as usize) % frames.len()]
        } else {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            frames[(state.tick_count as usize) % frames.len()]
        };

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

    let meta_rows = vec![
        Row::new(vec![Cell::from("Title:").style(label_style), Cell::from(title).style(val_style)]),
        Row::new(vec![Cell::from("Type:").style(label_style), Cell::from(type_str).style(val_style)]),
        Row::new(vec![Cell::from("Genre:").style(label_style), Cell::from(genres).style(val_style)]),
        Row::new(vec![Cell::from("Released:").style(label_style), Cell::from(year).style(val_style)]),
        Row::new(vec![Cell::from("Duration:").style(label_style), Cell::from(duration).style(val_style)]),
        Row::new(vec![Cell::from("Rating:").style(label_style), Cell::from(format!("{} (IMDb: {})", content_rating, imdb_rating)).style(val_style)]),
        Row::new(vec![Cell::from("Country:").style(label_style), Cell::from(country).style(val_style)]),
        Row::new(vec![Cell::from("Cast:").style(label_style), Cell::from(cast).style(val_style)]),
        Row::new(vec![Cell::from("Dubbing:").style(label_style), Cell::from(dubbing_str).style(val_style)]),
        Row::new(vec![Cell::from("Viewers:").style(label_style), Cell::from(viewers.to_string()).style(val_style)]),
    ];

    let meta_table = Table::new(meta_rows, [Constraint::Length(10), Constraint::Min(10)]);
    frame.render_widget(meta_table, meta_area);

    let syn_lines = vec![
        Line::from(vec![Span::styled("Synopsis: ", label_style), Span::styled(intro, theme.text_dim)])
    ];
    let intro_p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
    frame.render_widget(intro_p, synopsis_area);

    let has_languages = if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
        dubs.len() > 1
    } else {
        false
    };

    let is_series = type_val == 2 && !state.available_seasons.is_empty();
    let bottom_chunks = if has_languages && !state.language_chosen {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(20), Constraint::Min(0)])
            .split(bottom_area)
    } else {
        let mut c = Vec::new();
        if has_languages || is_series {
            c.push(Constraint::Length(22)); // Left side panel
        }
        c.push(Constraint::Min(1)); // Streams panel
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(c)
            .split(bottom_area)
    };
    
    let mut left_panel_chunks: std::rc::Rc<[ratatui::layout::Rect]> = std::rc::Rc::new([]);
    if (!has_languages || state.language_chosen) && (has_languages || is_series) {
        let mut v_constraints = Vec::new();
        if has_languages {
            let mut h = 3;
            if let Some(dubs) = details_json.get("dubs").and_then(|d| d.as_array()) {
                h = (dubs.len() as u16) + 2;
            }
            v_constraints.push(Constraint::Max(h));
        }
        if is_series {
            let seasons_count = state.available_seasons.len() as u16;
            let mut eps_count = 1;
            if let Some(season) = state.available_seasons.get(state.season_list_state.selected().unwrap_or(0)) {
                eps_count = season.get("maxEp").and_then(|m| m.as_i64()).unwrap_or(1) as u16;
            }
            v_constraints.push(Constraint::Max(seasons_count + 2));
            v_constraints.push(Constraint::Max(eps_count + 2));
        }
        v_constraints.push(Constraint::Min(0));
        left_panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(v_constraints)
            .split(bottom_chunks[0]);
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
        let list_height = (lang_items.len() as u16).saturating_add(2);
        
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

        let lang_area = if !state.language_chosen {
            let v_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(list_height),
                    Constraint::Min(0),
                ])
                .split(bottom_chunks[1]);
            v_split[1]
        } else {
            left_panel_chunks[0]
        };
        frame.render_stateful_widget(lang_list, lang_area, &mut state.language_list_state);
    }

    if !has_languages || state.language_chosen {

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
            left_panel_chunks[1]
        } else {
            left_panel_chunks[0]
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
            left_panel_chunks[2]
        } else {
            left_panel_chunks[1]
        };
        frame.render_stateful_widget(eps_list, eps_area, &mut state.episode_list_state);
    }

    let streams_area = if has_languages || is_series {
        bottom_chunks[1]
    } else {
        bottom_chunks[0]
    };
    let streams_border = if state.details_pane == crate::tui::state::DetailsPane::Streams {
        theme.border_focus
    } else {
        theme.border
    };

    let streams_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(ratatui::text::Line::from(" Streams ").alignment(Alignment::Left))
        .title(ratatui::text::Line::from(" [Enter/P] Play  [D] Download  [C] Copy Link  [B/Esc] Back ").alignment(Alignment::Right))
        .title_style(theme.title)
        .border_style(streams_border);

    let mut render_table = None;
    match &state.selected_resources {
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

                let t = Table::new(rows, widths)
                    .header(
                        Row::new(vec!["Res", "Codec", "Size", "Subs", "Uploader"])
                            .style(theme.title),
                    )
                    .block(streams_block)
                    .row_highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50)))
                    .highlight_symbol(">> ");
                render_table = Some(t);
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
                    .block(streams_block);
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
                let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];
                format!("{} Loading streams...", spinner)
            } else if has_multiple_dubs && !state.language_chosen {
                "Please select a language dubbing from the right panel to view streams.".to_string()
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
                .block(streams_block);
            frame.render_widget(p, streams_area);
        }
    }

    if let Some(t) = render_table {
        frame.render_stateful_widget(t, streams_area, &mut state.resource_list_state);
    }
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
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(theme.border),
            )
            .highlight_style(theme.highlight)
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, popup_area, &mut state.subtitle_list_state);
    }
}
