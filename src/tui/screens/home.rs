use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let show_cursor = (state.tick_count % 16) < 8;
    let search_prefix = if state.basic_terminal { "> " } else { "❯ " };

    let search_content =
        if !state.status_message.is_empty() && state.input_mode == InputMode::Normal {
            format!("{}{}", search_prefix, state.status_message)
        } else if state.search_query.is_empty() {
            let animated_text = &state.cached_animated_text;

            if state.input_mode == InputMode::Editing {
                if show_cursor {
                    format!("{}{}{}█", search_prefix, animated_text, "")
                } else {
                    format!("{}{}{} ", search_prefix, animated_text, "")
                }
            } else {
                if show_cursor {
                    format!("{}{}|", search_prefix, animated_text)
                } else {
                    format!("{}{}", search_prefix, animated_text)
                }
            }
        } else {
            if state.input_mode == InputMode::Editing {
                if show_cursor {
                    format!("{} {}█", search_prefix, state.search_query)
                } else {
                    format!("{} {} ", search_prefix, state.search_query)
                }
            } else {
                format!("{} {}", search_prefix, state.search_query)
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

        let is_narrow = area.width < 60 || state.basic_terminal;
        let is_wide = area.width >= 100 && !state.basic_terminal;
        let logo_height = if is_narrow {
            2
        } else if is_wide {
            6
        } else {
            4
        };

        let logo_text = if is_narrow {
            if state.is_tv_mode {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █TV".to_string()
            } else {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █".to_string()
            }
        } else if is_wide {
            if state.is_tv_mode {
                r"███╗   ███╗  ██████╗  ██╗   ██╗ ██╗ ███████╗ ██████╗   ██████╗  ██╗  ██╗
████╗ ████║ ██╔═══██╗ ██║   ██║ ██║ ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗██╔╝
██╔████╔██║ ██║   ██║ ██║   ██║ ██║ █████╗   ██████╔╝ ██║   ██║  ╚███╔╝ 
██║╚██╔╝██║ ██║   ██║ ╚██╗ ██╔╝ ██║ ██╔══╝   ██╔══██╗ ██║   ██║  ██╔██╗ TV
██║ ╚═╝ ██║ ╚██████╔╝  ╚████╔╝  ██║ ███████╗ ██████╔╝ ╚██████╔╝ ██╔╝ ██╗
╚═╝     ╚═╝  ╚═════╝    ╚═══╝   ╚═╝ ╚══════╝ ╚═════╝   ╚═════╝  ╚═╝  ╚═╝"
                    .to_string()
            } else {
                r"███╗   ███╗  ██████╗  ██╗   ██╗ ██╗ ███████╗ ██████╗   ██████╗  ██╗  ██╗
████╗ ████║ ██╔═══██╗ ██║   ██║ ██║ ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗██╔╝
██╔████╔██║ ██║   ██║ ██║   ██║ ██║ █████╗   ██████╔╝ ██║   ██║  ╚███╔╝ 
██║╚██╔╝██║ ██║   ██║ ╚██╗ ██╔╝ ██║ ██╔══╝   ██╔══██╗ ██║   ██║  ██╔██╗ 
██║ ╚═╝ ██║ ╚██████╔╝  ╚████╔╝  ██║ ███████╗ ██████╔╝ ╚██████╔╝ ██╔╝ ██╗
╚═╝     ╚═╝  ╚═════╝    ╚═══╝   ╚═╝ ╚══════╝ ╚═════╝   ╚═════╝  ╚═╝  ╚═╝"
                    .to_string()
            }
        } else {
            if state.is_tv_mode {
                r"  __  __  ___  __   __ ___  ___  ___   ___  __  __ 
 |  \/  |/ _ \ \ \ / /|_ _|| __|| _ ) / _ \ \ \/ / 
 | |\/| | (_) | \ V /  | | | _| | _ \| (_) | >  <  TV
 |_|  |_|\___/   \_/  |___||___||___/ \___/ /_/\_\ "
                    .to_string()
            } else {
                r"  __  __  ___  __   __ ___  ___  ___   ___  __  __ 
 |  \/  |/ _ \ \ \ / /|_ _|| __|| _ ) / _ \ \ \/ / 
 | |\/| | (_) | \ V /  | | | _| | _ \| (_) | >  <  
 |_|  |_|\___/   \_/  |___||___||___/ \___/ /_/\_\ "
                    .to_string()
            }
        };

        let logo_width: u16 = if is_narrow {
            if state.is_tv_mode { 33 } else { 31 }
        } else if is_wide {
            if state.is_tv_mode { 75 } else { 73 }
        } else {
            if state.is_tv_mode { 57 } else { 55 }
        };

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Length(logo_height),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

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

        let logo_style = if state.basic_terminal || state.tick_count >= 8 {
            theme.title
        } else {
            let t = state.tick_count as f32 / 8.0;

            let r = (49.0 + (203.0 - 49.0) * t) as u8;
            let g = (50.0 + (166.0 - 50.0) * t) as u8;
            let b = (68.0 + (247.0 - 68.0) * t) as u8;
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(r, g, b))
        };

        if is_wide && !state.basic_terminal && state.tick_count < 15 {
            let rows: Vec<&str> = logo_text.split('\n').collect();
            for (i, row) in rows.iter().enumerate() {
                let row_tick_start = 1 + i as u64;
                if state.tick_count >= row_tick_start {
                    let row_t = ((state.tick_count - row_tick_start) as f32 / 7.0).clamp(0.0, 1.0);
                    let r = (49.0 + (203.0 - 49.0) * row_t) as u8;
                    let g = (50.0 + (166.0 - 50.0) * row_t) as u8;
                    let b = (68.0 + (247.0 - 68.0) * row_t) as u8;
                    let row_style =
                        ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(r, g, b));

                    let row_area = Rect {
                        x: horizontal_chunks[1].x,
                        y: horizontal_chunks[1].y + i as u16,
                        width: horizontal_chunks[1].width,
                        height: 1,
                    };
                    frame.render_widget(Paragraph::new(*row).style(row_style), row_area);
                }
            }
        } else {
            let title_art = Paragraph::new(logo_text)
                .alignment(Alignment::Left)
                .style(logo_style);
            frame.render_widget(title_art, horizontal_chunks[1]);
        }

        let version_style = if state.tick_count < 6 {
            ratatui::style::Style::default().fg(theme.base)
        } else {
            theme.text_dim
        };
        let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(version_style);
        frame.render_widget(version, version_chunks[1]);

        if state.tick_count >= 3 {
            search_bar_area = vertical_chunks[5];

            let search_bar = Paragraph::new(search_content.clone())
                .alignment(Alignment::Center)
                .style(match state.input_mode {
                    InputMode::Editing => theme.title,
                    InputMode::Normal => theme.text_dim,
                });

            if !state.tv_config_popup {
                frame.render_widget(search_bar, search_bar_area);
            }

            let legend_line = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled("[", theme.text_dim),
                ratatui::text::Span::styled("Ctrl+T", theme.shortcut),
                ratatui::text::Span::styled("] ", theme.text_dim),
                ratatui::text::Span::styled(
                    if state.is_tv_mode {
                        "TV Mode   "
                    } else {
                        "Streaming Mode   "
                    },
                    theme.text,
                ),
                ratatui::text::Span::styled("[", theme.text_dim),
                ratatui::text::Span::styled("Type", theme.shortcut),
                ratatui::text::Span::styled("] ", theme.text_dim),
                ratatui::text::Span::styled("Search   ", theme.text),
                ratatui::text::Span::styled("[", theme.text_dim),
                ratatui::text::Span::styled("↑↓", theme.shortcut),
                ratatui::text::Span::styled("] ", theme.text_dim),
                ratatui::text::Span::styled("Browse   ", theme.text),
                ratatui::text::Span::styled("[", theme.text_dim),
                ratatui::text::Span::styled("?", theme.shortcut),
                ratatui::text::Span::styled("] ", theme.text_dim),
                ratatui::text::Span::styled("Help   ", theme.text),
                ratatui::text::Span::styled("[", theme.text_dim),
                ratatui::text::Span::styled("q", theme.shortcut),
                ratatui::text::Span::styled("] ", theme.text_dim),
                ratatui::text::Span::styled("Quit", theme.text),
            ]);

            let legend = Paragraph::new(legend_line).alignment(Alignment::Center);
            frame.render_widget(legend, vertical_chunks[7]);

            if let Some(version_str) = &state.update_available {
                let update_text = Paragraph::new(format!(
                    "Update v{} available! Auto-update failed, please reinstall manually.",
                    version_str
                ))
                .alignment(Alignment::Center)
                .style(theme.highlight);
                frame.render_widget(update_text, vertical_chunks[8]);
            }
        }
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        search_bar_area = chunks[0];
        let border_style = if state.input_mode == InputMode::Editing {
            theme.border_focus
        } else {
            theme.border
        };

        let loading_title = if state.is_loading && !state.search_results.is_empty() {
            let spinner = if state.basic_terminal {
                let frames = ['-', '\\', '|', '/'];
                frames[(state.tick_count as usize) % frames.len()]
            } else {
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                frames[(state.tick_count as usize) % frames.len()]
            };
            Some(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!(" {} ", spinner),
                theme.accent,
            )))
        } else {
            None
        };

        let mut search_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(if state.basic_terminal {
                ratatui::widgets::BorderType::Plain
            } else {
                ratatui::widgets::BorderType::Rounded
            })
            .padding(ratatui::widgets::Padding::left(1));
        if let Some(title) = loading_title {
            search_block = search_block
                .title_top(title)
                .title_alignment(Alignment::Right);
        }
        let search_bar = Paragraph::new(search_content.clone())
            .style(match state.input_mode {
                InputMode::Editing => theme.title,
                InputMode::Normal => theme.text,
            })
            .block(search_block);
        frame.render_widget(search_bar, search_bar_area);

        let list_block = Block::default();

        if state.is_loading && state.search_results.is_empty() {
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

            let spinner_color = if state.basic_terminal {
                theme.accent
            } else {
                let is_sky = (state.tick_count % 16) < 8;
                if is_sky { theme.accent } else { theme.teal }
            };

            let loading_text = if state.is_homepage_mode {
                state.status_message.clone()
            } else {
                format!("Searching for \"{}\"...", state.search_query)
            };

            let p = Paragraph::new(ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(format!("{}  ", spinner), spinner_color),
                ratatui::text::Span::styled(loading_text, theme.title),
            ]))
            .alignment(Alignment::Center);
            frame.render_widget(p, v_chunks[1]);
        } else if !state.search_results.is_empty() {
            let selected_idx = state.search_list_state.selected();
            let offset = state.search_list_state.offset();

            let row_height = state.poster_rows;
            state.visible_items = (chunks[1].height as usize) / (row_height as usize);
            let rows = state
                .search_results
                .iter()
                .map(|_| Row::new(vec![Cell::from("")]).height(row_height));

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
                    height: state
                        .poster_rows
                        .min(inner_area.y + inner_area.height.saturating_sub(current_y)),
                };

                if item_area.height == 0 {
                    break;
                }

                let is_selected = Some(i) == selected_idx;

                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(2),
                        Constraint::Length(if state.image_supported {
                            state.poster_rows + 1
                        } else {
                            0
                        }),
                        Constraint::Length(if state.image_supported { 1 } else { 0 }),
                        Constraint::Min(0),
                    ])
                    .split(item_area);

                let highlight_area = layout[0];
                let poster_area = layout[1];
                let text_area = layout[3];

                if is_selected {
                    let indicator = Paragraph::new(ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            if state.basic_terminal { "> " } else { "▌ " },
                            theme.accent,
                        ),
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

                if let Some(img) = state.search_posters.peek(&res.id) {
                    if state.image_supported {
                        let target_dims = (poster_area.width, state.poster_rows);
                        let needs_protocol =
                            state.search_poster_protocols.peek(&res.id).map(|(d, _)| *d)
                                != Some(target_dims);
                        if needs_protocol {
                            if let Some(picker) = &mut state.image_picker {
                                let size = ratatui::layout::Size::new(target_dims.0, target_dims.1);
                                if let Ok(proto) = picker.new_protocol(
                                    (**img).clone(),
                                    size,
                                    ratatui_image::Resize::Fit(None),
                                ) {
                                    state
                                        .search_poster_protocols
                                        .put(res.id.clone(), (target_dims, proto));
                                }
                            }
                        }
                        if let Some((_, proto)) = state.search_poster_protocols.peek(&res.id) {
                            let p_area = Rect {
                                height: poster_area.height.min(state.poster_rows),
                                ..poster_area
                            };
                            frame.render_widget(ratatui_image::Image::new(proto), p_area);
                        }
                    }
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

                let type_tag = if state.is_tv_mode || res.stype == 3 {
                    "TV Channel"
                } else if res.stype == 1 {
                    "Movie"
                } else if res.stype == 2 {
                    "Series"
                } else {
                    "Unknown"
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
                            let star = if state.basic_terminal { "* " } else { "★ " };
                            info_spans.push(ratatui::text::Span::styled(star, theme.rating));
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
                    .thumb_symbol(if state.basic_terminal { "|" } else { "█" });

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

        let dropdown_height = std::cmp::min(state.search_suggestions.len() as u16, 10);

        let is_home_screen = state.search_results.is_empty()
            && !state.is_loading
            && !state.status_message.to_lowercase().contains("fail");

        let dropdown_y = if !is_home_screen && search_area.y > area.height / 2 {
            search_area.y.saturating_sub(dropdown_height)
        } else {
            search_area.y + 1
        };

        let max_len = state
            .search_suggestions
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0) as u16;

        let dropdown_width = std::cmp::min(std::cmp::max(max_len + 8, 30), search_area.width);

        let text_len = search_content.chars().count() as u16;
        let text_start_x = search_area.x + search_area.width.saturating_sub(text_len) / 2;

        let dropdown_x = if is_home_screen {
            text_start_x + 2
        } else {
            search_area.x + 3
        };

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
            let list = ratatui::widgets::List::new(items)
                .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::NONE));
            frame.render_widget(list, dropdown_area);
        }
    }
    if state.tv_config_popup {
        let filtered_options = state.filtered_tv_wizard_options();
        let content_height = if state.tv_wizard_step == 1 {
            (filtered_options.len() as u16 + 2).max(6)
        } else {
            state.tv_wizard_options.len() as u16
        };
        let max_height = area.height.saturating_sub(8).max(8);
        let popup_height = (content_height + 4).clamp(8, max_height);

        let popup_area = ratatui::layout::Rect {
            x: area.width.saturating_sub(48) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2 + 2,
            width: 48,
            height: popup_height,
        };

        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let popup_block = ratatui::widgets::Block::default()
            .title(if state.tv_wizard_step == 0 {
                " TV Setup: Select Grouping "
            } else {
                " TV Setup: Select Items "
            })
            .title_alignment(ratatui::layout::Alignment::Center)
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(theme.border_focus)
            .style(ratatui::style::Style::default());

        let inner_area = popup_block.inner(popup_area);
        frame.render_widget(popup_block, popup_area);

        let (search_area, list_area, hint_area) = if state.tv_wizard_step == 1 {
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Min(1),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(inner_area);
            (Some(chunks[0]), chunks[1], chunks[2])
        } else {
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Min(1),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(inner_area);
            (None, chunks[0], chunks[1])
        };

        if let Some(s_area) = search_area {
            let search_line = ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(" Search: ", theme.highlight),
                ratatui::text::Span::styled(format!("{}█", state.tv_wizard_filter), theme.text),
            ]);
            frame.render_widget(ratatui::widgets::Paragraph::new(search_line), s_area);
        }

        let items: Vec<ratatui::widgets::ListItem> = filtered_options
            .iter()
            .map(|opt| {
                let is_checked = state.tv_wizard_selections.contains(opt);

                let checkbox = if state.tv_wizard_step == 1 {
                    if is_checked { "[x] " } else { "[ ] " }
                } else {
                    ""
                };

                let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                    format!("{}{}", checkbox, opt),
                    theme.text,
                )]);
                ratatui::widgets::ListItem::new(line)
            })
            .collect();

        let list = ratatui::widgets::List::new(items)
            .highlight_style(theme.highlight.add_modifier(ratatui::style::Modifier::BOLD))
            .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.tv_wizard_selected_idx));

        frame.render_stateful_widget(list, list_area, &mut list_state);

        let scrollbar =
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(
            filtered_options
                .len()
                .saturating_sub(list_area.height as usize),
        )
        .position(list_state.offset());

        frame.render_stateful_widget(
            scrollbar,
            list_area.inner(ratatui::layout::Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );

        let hint = if state.tv_wizard_step == 0 {
            " [Enter] Select   [Esc] Cancel "
        } else {
            " [Type] Filter   [Space] Toggle   [Enter] Confirm   [Esc] Back "
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(hint)
                .alignment(ratatui::layout::Alignment::Center)
                .style(theme.text_dim),
            hint_area,
        );
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
}
