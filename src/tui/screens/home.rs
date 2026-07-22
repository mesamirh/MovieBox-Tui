use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Cell, Paragraph, Row, Table},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let show_cursor = (state.tick_count % 16) < 8;
    let search_content =
        if !state.status_message.is_empty() && state.input_mode == InputMode::Normal {
            format!("> {}", state.status_message)
        } else if state.search_query.is_empty() {
            let prompts = [
                "Search for a movie...",
                "Search for a TV series...",
                "Search by genre...",
            ];
            let type_speed = 3;
            let del_speed = 1;
            let pause1 = 60; // ~1 sec
            let pause2 = 15; // ~250 ms

            let mut total_ticks = 0;
            for p in prompts.iter() {
                total_ticks += p.len() * type_speed + pause1 + p.len() * del_speed + pause2;
            }
            let mut t = (state.tick_count as usize) % total_ticks;

            let mut animated_text = String::new();
            for p in prompts.iter() {
                let t_type = p.len() * type_speed;
                let t_del = p.len() * del_speed;
                let cycle = t_type + pause1 + t_del + pause2;

                if t < cycle {
                    let display_len = if t < t_type {
                        t / type_speed
                    } else if t < t_type + pause1 {
                        p.len()
                    } else if t < t_type + pause1 + t_del {
                        p.len().saturating_sub((t - (t_type + pause1)) / del_speed)
                    } else {
                        0
                    };
                    animated_text = p[0..display_len].to_string();
                    break;
                } else {
                    t -= cycle;
                }
            }

            if state.input_mode == InputMode::Editing {
                if show_cursor {
                    format!("> {}█", animated_text)
                } else {
                    format!("> {} ", animated_text)
                }
            } else {
                if show_cursor {
                    format!("> {}|", animated_text)
                } else {
                    format!("> {} ", animated_text)
                }
            }
        } else {
            if state.input_mode == InputMode::Editing {
                if show_cursor {
                    format!("> {}█", state.search_query)
                } else {
                    format!("> {} ", state.search_query)
                }
            } else {
                format!("> {}", state.search_query)
            }
        };

    let mut search_bar_area = Rect::default();

    if state.search_results.is_empty()
        && !state.is_loading
        && !state.status_message.to_lowercase().contains("fail")
    {
        if state.tick_count < 1 {
            return;
        }

        let is_narrow = area.width < 60;
        let is_wide = area.width >= 100;
        let logo_height = if is_narrow {
            2
        } else if is_wide {
            6
        } else {
            4
        };
        let logo_text = if is_narrow {
            "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █"
        } else if is_wide {
            r"███╗   ███╗  ██████╗  ██╗   ██╗ ██╗ ███████╗ ██████╗   ██████╗  ██╗  ██╗
████╗ ████║ ██╔═══██╗ ██║   ██║ ██║ ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗██╔╝
██╔████╔██║ ██║   ██║ ██║   ██║ ██║ █████╗   ██████╔╝ ██║   ██║  ╚███╔╝ 
██║╚██╔╝██║ ██║   ██║ ╚██╗ ██╔╝ ██║ ██╔══╝   ██╔══██╗ ██║   ██║  ██╔██╗ 
██║ ╚═╝ ██║ ╚██████╔╝  ╚████╔╝  ██║ ███████╗ ██████╔╝ ╚██████╔╝ ██╔╝ ██╗
╚═╝     ╚═╝  ╚═════╝    ╚═══╝   ╚═╝ ╚══════╝ ╚═════╝   ╚═════╝  ╚═╝  ╚═╝"
        } else {
            r"  __  __  ___  __   __ ___  ___  ___   ___  __  __ 
 |  \/  |/ _ \ \ \ / /|_ _|| __|| _ ) / _ \ \ \/ / 
 | |\/| | (_) | \ V /  | | | _| | _ \| (_) | >  <  
 |_|  |_|\___/   \_/  |___||___||___/ \___/ /_/\_\ "
        };

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Length(logo_height),
                Constraint::Length(1),
                Constraint::Percentage(15),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let logo_width = if is_narrow {
            31
        } else if is_wide {
            73
        } else {
            55
        };
        let pad = area.width.saturating_sub(logo_width) / 2;
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[1]);

        let version_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[2]);

        let logo_style = if state.tick_count == 1 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(49, 50, 68))
        } else if state.tick_count == 2 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(108, 112, 134))
        } else {
            theme.title
        };

        let title_art = Paragraph::new(logo_text)
            .alignment(Alignment::Left)
            .style(logo_style);

        frame.render_widget(title_art, horizontal_chunks[1]);

        let version_style = if state.tick_count == 1 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(49, 50, 68))
        } else if state.tick_count == 2 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(108, 112, 134))
        } else {
            theme.text_dim
        };
        let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(version_style);
        frame.render_widget(version, version_chunks[1]);

        if state.tick_count >= 3 {
            let search_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(vertical_chunks[4]);

            search_bar_area = search_chunks[1];

            let search_bar = Paragraph::new(search_content.clone())
                .alignment(Alignment::Center)
                .style(match state.input_mode {
                    InputMode::Editing => theme.title,
                    InputMode::Normal => theme.text,
                });

            frame.render_widget(search_bar, search_bar_area);

            let legend = Paragraph::new("Type to Search   ↑↓ Browse   ? Help")
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(legend, vertical_chunks[6]);

            if let Some(version_str) = &state.update_available {
                let update_text = Paragraph::new(format!(
                    "Update v{} available! Run: cargo install moviebox-tui",
                    version_str
                ))
                .alignment(Alignment::Center)
                .style(theme.highlight);
                frame.render_widget(update_text, vertical_chunks[7]);
            }
        }
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        search_bar_area = chunks[0];

        let search_bar = Paragraph::new(search_content.clone())
            .style(match state.input_mode {
                InputMode::Editing => theme.title,
                InputMode::Normal => theme.text,
            })
            .block(ratatui::widgets::Block::default().padding(ratatui::widgets::Padding::left(1)));
        frame.render_widget(search_bar, search_bar_area);

        let list_block = Block::default();

        if state.is_loading {
            let spinner = if state.basic_terminal {
                let frames = ['-', '\\', '|', '/'];
                frames[(state.tick_count as usize) % frames.len()]
            } else {
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                frames[(state.tick_count as usize) % frames.len()]
            };

            let inner_area = list_block.inner(chunks[1]);
            frame.render_widget(list_block, chunks[1]);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let p = Paragraph::new(format!("{} Searching...", spinner))
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(p, v_chunks[1]);
        } else if !state.search_results.is_empty() {
            let selected_idx = state.search_list_state.selected();
            let offset = state.search_list_state.offset();

            let row_height = 3;
            state.visible_items = (chunks[1].height as usize) / (row_height as usize);
            let rows: Vec<Row> = state
                .search_results
                .iter()
                .map(|_| Row::new(vec![Cell::from("")]).height(row_height))
                .collect();

            let table = Table::new(rows, [Constraint::Percentage(100)]).block(list_block);

            frame.render_stateful_widget(table, chunks[1], &mut state.search_list_state);

            let mut inner_area = chunks[1];
            inner_area.x += 1;
            inner_area.y += 1;
            inner_area.width = inner_area.width.saturating_sub(2);
            inner_area.height = inner_area.height.saturating_sub(2);

            let mut current_y = inner_area.y;

            for (i, res) in state.search_results.iter().enumerate().skip(offset) {
                if current_y >= inner_area.y + inner_area.height {
                    break;
                }

                let item_area = Rect {
                    x: inner_area.x,
                    y: current_y,
                    width: inner_area.width,
                    height: 3.min(inner_area.y + inner_area.height.saturating_sub(current_y)),
                };

                if item_area.height == 0 {
                    break;
                }

                let is_selected = Some(i) == selected_idx;

                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(2),
                        Constraint::Length(4),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(item_area);

                let highlight_area = layout[0];
                let poster_area = layout[1];
                let text_area = layout[3];

                if is_selected {
                    let indicator = Paragraph::new(ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("▌ ", theme.accent.clone()),
                    ]));

                    let v_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(item_area.height.saturating_sub(1) / 2),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(highlight_area);

                    frame.render_widget(indicator, v_layout[1]);
                }

                let mut poster_rendered = false;
                if let Some(img) = state.search_posters.peek(&res.id) {
                    if state.image_supported {
                        let needs_protocol =
                            state.search_poster_protocols.get(&res.id).map(|(r, _)| *r)
                                != Some(poster_area);
                        if needs_protocol {
                            if let Some(picker) = &mut state.image_picker {
                                let size = ratatui::layout::Size::new(
                                    poster_area.width,
                                    poster_area.height.min(3),
                                );
                                if let Ok(proto) = picker.new_protocol(
                                    (**img).clone(),
                                    size,
                                    ratatui_image::Resize::Fit(None),
                                ) {
                                    state
                                        .search_poster_protocols
                                        .insert(res.id.clone(), (poster_area, proto));
                                }
                            }
                        }
                        if let Some((_, proto)) = state.search_poster_protocols.get(&res.id) {
                            let p_area = Rect {
                                height: poster_area.height.min(3),
                                ..poster_area
                            };
                            frame.render_widget(ratatui_image::Image::new(proto), p_area);
                            poster_rendered = true;
                        }
                    }
                }

                if !poster_rendered {
                    let p = Paragraph::new("████\n████\n████").style(theme.muted);
                    let p_area = Rect {
                        height: poster_area.height.min(3),
                        ..poster_area
                    };
                    frame.render_widget(p, p_area);
                }

                let text_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(text_area);

                let title_style = if is_selected { theme.title } else { theme.text };
                let max_title_width = text_area.width.saturating_sub(4) as usize;
                let mut display_title = res.title.clone();
                if display_title.chars().count() > max_title_width && max_title_width > 3 {
                    display_title = display_title
                        .chars()
                        .take(max_title_width - 3)
                        .collect::<String>();
                    display_title.push_str("...");
                }

                let type_tag = if res.stype == 1 {
                    "Movie"
                } else if res.stype == 2 {
                    "TV Series"
                } else {
                    "Other"
                };

                let title_line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                    display_title,
                    title_style,
                )]);
                if text_layout[0].height > 0 {
                    frame.render_widget(Paragraph::new(title_line), text_layout[0]);
                }

                let mut info_spans = vec![];

                if is_selected {
                    if state.preview_loading || state.is_loading {
                        info_spans.push(ratatui::text::Span::styled(&res.release_year, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        info_spans.push(ratatui::text::Span::styled(type_tag, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        info_spans.push(ratatui::text::Span::styled("Loading...", theme.text_dim));
                    } else if let Some(meta) = &state.search_preview {
                        let rating = meta
                            .get("imdbRating")
                            .or_else(|| meta.get("imdbRatingValue"))
                            .and_then(|v| v.as_str());
                        if let Some(r) = rating {
                            info_spans
                                .push(ratatui::text::Span::styled("★ ", theme.rating.clone()));
                            info_spans.push(ratatui::text::Span::styled(r, theme.text));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        }
                        info_spans.push(ratatui::text::Span::styled(&res.release_year, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));

                        let mut g_names = vec![];
                        if let Some(genres) = meta.get("genres").and_then(|g| g.as_array()) {
                            g_names = genres
                                .iter()
                                .filter_map(|g| {
                                    g.get("name")
                                        .and_then(|n| n.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect();
                        }
                        if !g_names.is_empty() {
                            info_spans
                                .push(ratatui::text::Span::styled(g_names.join(" • "), theme.text));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        }
                        info_spans.push(ratatui::text::Span::styled(type_tag, theme.text));
                    } else {
                        info_spans.push(ratatui::text::Span::styled(&res.release_year, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        info_spans.push(ratatui::text::Span::styled(type_tag, theme.text));
                    }
                } else {
                    info_spans.push(ratatui::text::Span::styled(&res.release_year, theme.text));
                    info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                    info_spans.push(ratatui::text::Span::styled(type_tag, theme.text));
                }

                if text_layout[1].height > 0 && !info_spans.is_empty() {
                    frame.render_widget(
                        Paragraph::new(ratatui::text::Line::from(info_spans)),
                        text_layout[1],
                    );
                }

                current_y += row_height;
            }

            let content_len = state.search_results.len();
            if content_len > state.visible_items {
                let scrollbar = ratatui::widgets::Scrollbar::default()
                    .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");

                let mut scrollbar_state = ratatui::widgets::ScrollbarState::default()
                    .content_length(content_len.saturating_sub(state.visible_items))
                    .position(offset);

                let mut sb_area = chunks[1];
                sb_area.y += 1;
                sb_area.height = sb_area.height.saturating_sub(2);

                frame.render_stateful_widget(scrollbar, sb_area, &mut scrollbar_state);
            }
        } else {
            let inner_area = list_block.inner(chunks[1]);
            frame.render_widget(list_block, chunks[1]);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let p = Paragraph::new(state.status_message.clone())
                .alignment(Alignment::Center)
                .style(theme.error);
            frame.render_widget(p, v_chunks[1]);
        }
    }

    if state.input_mode == InputMode::Editing
        && !state.search_suggestions.is_empty()
        && search_bar_area.width > 0
    {
        let search_area = search_bar_area;

        let dropdown_height = std::cmp::min(state.search_suggestions.len() as u16 + 2, 10);

        let is_home_screen = state.search_results.is_empty()
            && !state.is_loading
            && !state.status_message.to_lowercase().contains("fail");

        let dropdown_y = if !is_home_screen && search_area.y > area.height / 2 {
            search_area.y.saturating_sub(dropdown_height)
        } else {
            search_area.y + search_area.height
        };

        let max_len = state
            .search_suggestions
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0) as u16;
        let dropdown_width = std::cmp::min(std::cmp::max(max_len + 8, 30), search_area.width);
        let dropdown_x = search_area.x + (search_area.width.saturating_sub(dropdown_width)) / 2;

        let dropdown_area = Rect {
            x: dropdown_x,
            y: dropdown_y,
            width: dropdown_width,
            height: dropdown_height,
        };

        if dropdown_area.y + dropdown_area.height <= area.height || search_area.y > area.height / 2
        {
            frame.render_widget(ratatui::widgets::Clear, dropdown_area);
            let items: Vec<ratatui::widgets::ListItem> = state
                .search_suggestions
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let text = if Some(i) == state.suggest_index {
                        format!("▌ {}", s)
                    } else {
                        format!("   {}", s)
                    };
                    let style = if Some(i) == state.suggest_index {
                        theme.highlight
                    } else {
                        theme.text
                    };
                    ratatui::widgets::ListItem::new(
                        ratatui::text::Line::from(ratatui::text::Span::styled(text, style))
                            .alignment(ratatui::layout::Alignment::Left),
                    )
                })
                .collect();
            let list = ratatui::widgets::List::new(items).block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(theme.border_focus)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            );
            frame.render_widget(list, dropdown_area);
        }
    }
}
