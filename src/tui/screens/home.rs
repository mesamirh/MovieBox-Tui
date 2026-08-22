use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchViewState {
    Empty,
    Editing,
    Loading,
    Results,
    NoResults,
    Error,
}

fn search_view_state(state: &AppState) -> SearchViewState {
    if state.input_mode == InputMode::Editing {
        SearchViewState::Editing
    } else if state.is_loading
        && (!state.search_query.trim().is_empty()
            || state.active_browse_preset.is_some()
            || state.active_addon_catalog.is_some()
            || state.is_homepage_mode)
    {
        SearchViewState::Loading
    } else if state.search_error.is_some() {
        SearchViewState::Error
    } else if !state.search_results.is_empty() {
        SearchViewState::Results
    } else if !state.search_query.trim().is_empty()
        || state.active_browse_preset.is_some()
        || state.active_addon_catalog.is_some()
    {
        SearchViewState::NoResults
    } else {
        SearchViewState::Empty
    }
}

fn centered_width(area: Rect, maximum: u16) -> Rect {
    let width = area.width.min(maximum).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        width,
        ..area
    }
}

pub(crate) fn slash_command_description(cmd: &str, state: &AppState) -> Option<&'static str> {
    crate::tui::commands::SlashCommand::description_for(cmd, state)
}

pub(crate) fn search_deck_width(area: Rect, state: &AppState, landing: bool) -> u16 {
    let query_width = if state.search_query.is_empty() {
        crate::tui::text::width(if state.is_tv_mode {
            "Search live channels…"
        } else {
            "Search movies and series…"
        }) as u16
    } else {
        crate::tui::text::width(&state.search_query) as u16
    };
    let minimum = if landing { 38 } else { 48 };
    let maximum = if landing && area.width >= 120 {
        88
    } else if landing {
        72
    } else {
        104
    }
    .min(area.width.saturating_sub(4));

    let status_width = if !landing && !state.search_results.is_empty() {
        crate::tui::text::width(&format!("{} results", state.search_results.len())) as u16 + 4
    } else {
        0
    };

    query_width
        .saturating_add(10)
        .saturating_add(status_width)
        .max(minimum.min(maximum))
        .min(maximum)
}

fn render_search_state(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    view: SearchViewState,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let card_width = area.width.min(64);
    let card = Rect {
        x: area.x + area.width.saturating_sub(card_width) / 2,
        y: area.y + area.height.saturating_sub(3) / 2,
        width: card_width,
        height: 3,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(card);

    let pulse = match (state.tick_count / 4) % 4 {
        0 => "·",
        1 | 3 => "◦",
        _ => "○",
    };
    let query = crate::tui::text::truncate_width(
        &state.search_query,
        card_width.saturating_sub(10) as usize,
    );

    let line = match view {
        SearchViewState::Loading => {
            let dots = state.loading_dots();
            let msg = if let Some(preset) = state.active_browse_preset {
                format!("Loading {}{dots}", preset.label())
            } else if let Some(catalog) = &state.active_addon_catalog {
                format!("Loading {}{dots}", catalog.label)
            } else if state.is_homepage_mode {
                format!("Loading discover{dots}")
            } else if !state.search_query.trim().is_empty() {
                format!("Searching for “{query}”{dots}")
            } else {
                format!("Loading{dots}")
            };
            Line::from(vec![Span::styled(msg, theme.lavender)])
        }
        SearchViewState::NoResults => {
            let symbol = if state.basic_terminal { "-" } else { pulse };
            let style = if (state.tick_count / 4) % 2 == 0 {
                theme.lavender
            } else {
                theme.subtext1
            };
            let msg = if let Some(preset) = state.active_browse_preset {
                format!("No items found for {}", preset.label())
            } else if let Some(catalog) = &state.active_addon_catalog {
                format!("No items found for {}", catalog.label)
            } else if state.search_query.trim().eq_ignore_ascii_case("/history") {
                "No watch history found".to_string()
            } else if !state.search_query.trim().is_empty() {
                format!("No matches for “{query}”")
            } else {
                "No results found".to_string()
            };
            Line::from(vec![
                Span::styled(format!("{symbol} "), style),
                Span::styled(msg, theme.text),
            ])
        }
        SearchViewState::Error => {
            let symbol = if state.basic_terminal { "!" } else { "×" };
            let err_text = state.search_error.as_deref().unwrap_or_else(|| {
                if !state.status_message.is_empty() {
                    &state.status_message
                } else {
                    "Search request failed"
                }
            });
            Line::from(vec![
                Span::styled(format!("{symbol} "), theme.error),
                Span::styled(
                    crate::tui::text::truncate_width(
                        err_text,
                        card_width.saturating_sub(4) as usize,
                    ),
                    theme.error,
                ),
            ])
        }
        _ => return,
    };

    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), rows[1]);
}

fn render_favorites_landing(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    if area.height < 2 || area.width < 20 {
        return;
    }

    let items: Vec<crate::favorites::FavoriteItem> = state
        .favorites_landing_items()
        .into_iter()
        .cloned()
        .collect();
    if items.is_empty() {
        return;
    }

    let overflow = state.favorites.items.len().saturating_sub(items.len());
    let card_width = area.width.clamp(20, 56);
    let row_count = items.len() as u16;
    let overflow_row = u16::from(overflow > 0);
    let content_height = (1 + row_count + overflow_row).min(area.height);

    let card = Rect {
        x: area.x + area.width.saturating_sub(card_width) / 2,
        y: area.y,
        width: card_width,
        height: content_height,
    };

    let mut constraints = vec![Constraint::Length(1)];
    constraints.extend(std::iter::repeat_n(Constraint::Length(1), items.len()));
    if overflow > 0 {
        constraints.push(Constraint::Length(1));
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(card);

    let header_style = if state.favorites_focus {
        theme.title
    } else {
        theme.subtext1
    };
    frame.render_widget(
        Paragraph::new("Favorites")
            .style(header_style)
            .alignment(Alignment::Center),
        sections[0],
    );

    let selected = if state.favorites_focus {
        state.favorites_landing_state.selected()
    } else {
        None
    };

    for (i, item) in items.iter().enumerate() {
        let Some(row_area) = sections.get(1 + i) else {
            break;
        };
        let is_selected = selected == Some(i);
        let type_tag = if item.stype == 2 { "Series" } else { "Movie" };
        let prefix = if is_selected {
            if state.basic_terminal { "> " } else { "▌ " }
        } else {
            "  "
        };
        let title_style = if is_selected {
            theme.title.add_modifier(Modifier::BOLD)
        } else {
            theme.text
        };
        let max_title_width = row_area.width.saturating_sub(14) as usize;
        let line = Line::from(vec![
            Span::styled(prefix, theme.accent),
            Span::styled(
                crate::tui::text::truncate_width(&item.title, max_title_width),
                title_style,
            ),
            Span::styled(format!("  {type_tag}"), theme.text_dim),
        ]);
        frame.render_widget(Paragraph::new(line), *row_area);
    }

    if overflow > 0 {
        if let Some(row_area) = sections.last() {
            frame.render_widget(
                Paragraph::new(format!("+{overflow} more • /favorites"))
                    .style(theme.text_dim)
                    .alignment(Alignment::Center),
                *row_area,
            );
        }
    }
}

fn search_content(
    state: &AppState,
    view: SearchViewState,
    show_cursor: bool,
    width: u16,
) -> String {
    let prefix = if state.basic_terminal { "> " } else { "❯ " };
    let cursor_width = usize::from(view == SearchViewState::Editing);
    let available = width
        .saturating_sub(4)
        .saturating_sub(crate::tui::text::width(prefix) as u16)
        .saturating_sub(cursor_width as u16) as usize;
    let content = if state.search_query.is_empty() {
        if state.is_tv_mode {
            "Search live channels…".to_string()
        } else if state.is_addon_mode {
            "Search movies and series via addons…".to_string()
        } else {
            "Search movies and series…".to_string()
        }
    } else {
        crate::tui::text::truncate_width(&state.search_query, available)
    };
    let cursor = if view == SearchViewState::Editing {
        if show_cursor { "█" } else { " " }
    } else {
        ""
    };
    format!("{prefix}{content}{cursor}")
}

fn render_search_bar(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    view: SearchViewState,
    show_cursor: bool,
    centered: bool,
) {
    let result_status = if view == SearchViewState::Results {
        Some(if state.search_results.len() == 1 {
            "1 result".to_string()
        } else {
            format!("{} results", state.search_results.len())
        })
    } else {
        None
    };
    let status_width = result_status
        .as_deref()
        .map(crate::tui::text::width)
        .unwrap_or(0) as u16;
    let content_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(status_width.saturating_add(u16::from(status_width > 0) * 2)),
        ])
        .split(area);
    let mut paragraph = Paragraph::new(search_content(
        state,
        view,
        show_cursor,
        content_row[0].width,
    ))
    .style(if view == SearchViewState::Editing {
        theme.text
    } else if state.search_query.is_empty() {
        theme.text_dim
    } else {
        theme.text
    });
    if centered {
        paragraph = paragraph.alignment(Alignment::Center);
    }
    frame.render_widget(paragraph, content_row[0]);
    if let Some(status) = result_status {
        frame.render_widget(
            Paragraph::new(status)
                .style(theme.accent)
                .alignment(Alignment::Right),
            content_row[1],
        );
    }
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let show_cursor = (state.tick_count % 16) < 8;
    let view = search_view_state(state);
    let search_bar_area;

    if view == SearchViewState::Empty
        || (view == SearchViewState::Editing && state.search_results.is_empty())
    {
        let basic_terminal = state.basic_terminal;
        let logo_height = if basic_terminal { 2 } else { 6 };

        let logo_text = if basic_terminal {
            if state.is_tv_mode {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █TV".to_string()
            } else {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █".to_string()
            }
        } else if state.is_tv_mode {
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
        };

        let logo_width: u16 = if basic_terminal {
            if state.is_tv_mode { 33 } else { 31 }
        } else if state.is_tv_mode {
            75
        } else {
            73
        };
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(16),
                Constraint::Length(logo_height),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
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

        let title_art = Paragraph::new(logo_text)
            .alignment(Alignment::Left)
            .style(theme.title);
        frame.render_widget(title_art, horizontal_chunks[1]);

        let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(theme.text_dim);
        frame.render_widget(version, version_chunks[1]);

        let search_width = search_deck_width(area, state, true);
        search_bar_area = centered_width(vertical_chunks[4], search_width);

        if !state.tv_config_popup {
            render_search_bar(
                frame,
                search_bar_area,
                state,
                theme,
                view,
                show_cursor,
                true,
            );
        }

        if state.favorites_landing_visible() {
            render_favorites_landing(frame, vertical_chunks[6], state, theme);
        }

        let ctrl_s = crate::tui::text::ctrl_key("S");
        let ctrl_t = crate::tui::text::ctrl_key("T");
        let ctrl_a = crate::tui::text::ctrl_key("A");
        let ctrl_p = crate::tui::text::ctrl_key("P");

        let current_mode = state.mode();
        let mut mode_tabs: Vec<Vec<Span>> = Vec::new();

        if state.streaming_enabled {
            let mut spans = vec![];
            if current_mode == crate::tui::state::AppMode::Streaming {
                spans.push(Span::styled("[", theme.text_dim));
                spans.push(Span::styled(&ctrl_p, theme.shortcut));
                spans.push(Span::styled("] ", theme.text_dim));
                spans.push(Span::styled(
                    state.active_provider.label(),
                    theme.highlight.add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("[", theme.text_dim));
                spans.push(Span::styled(&ctrl_s, theme.shortcut));
                spans.push(Span::styled("] ", theme.text_dim));
                spans.push(Span::styled("Streaming", theme.text_dim));
            }
            mode_tabs.push(spans);
        }

        if state.tv_enabled {
            let mut spans = vec![];
            if current_mode == crate::tui::state::AppMode::Tv {
                spans.push(Span::styled("[ ", theme.text_dim));
                spans.push(Span::styled(
                    "TV",
                    theme.highlight.add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(" ]", theme.text_dim));
            } else {
                spans.push(Span::styled("[", theme.text_dim));
                spans.push(Span::styled(&ctrl_t, theme.shortcut));
                spans.push(Span::styled("] ", theme.text_dim));
                spans.push(Span::styled("TV", theme.text_dim));
            }
            mode_tabs.push(spans);
        }

        if state.addons_enabled {
            let mut spans = vec![];
            if current_mode == crate::tui::state::AppMode::Addon {
                spans.push(Span::styled("[ ", theme.text_dim));
                spans.push(Span::styled(
                    "Addons",
                    theme.highlight.add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(" ]", theme.text_dim));
            } else {
                spans.push(Span::styled("[", theme.text_dim));
                spans.push(Span::styled(&ctrl_a, theme.shortcut));
                spans.push(Span::styled("] ", theme.text_dim));
                spans.push(Span::styled("Addons", theme.text_dim));
            }
            mode_tabs.push(spans);
        }

        let mut mode_spans = vec![];
        for (i, tab) in mode_tabs.into_iter().enumerate() {
            if i > 0 {
                mode_spans.push(Span::raw("     "));
            }
            mode_spans.extend(tab);
        }

        frame.render_widget(
            Paragraph::new(Line::from(mode_spans)).alignment(Alignment::Center),
            vertical_chunks[7],
        );

        let util_spans = vec![
            Span::styled("[", theme.text_dim),
            Span::styled("?", theme.shortcut),
            Span::styled("] ", theme.text_dim),
            Span::styled("Help", theme.text_dim),
            Span::raw("     "),
            Span::styled("[", theme.text_dim),
            Span::styled("q", theme.shortcut),
            Span::styled("] ", theme.text_dim),
            Span::styled("Quit", theme.text_dim),
        ];

        frame.render_widget(
            Paragraph::new(Line::from(util_spans)).alignment(Alignment::Center),
            vertical_chunks[8],
        );
    } else {
        if state.is_loading && state.search_results.is_empty() {
            render_search_state(frame, area, state, theme, SearchViewState::Loading);
            return;
        }

        let has_results = !state.search_results.is_empty();
        let suggestion_height =
            if state.input_mode == InputMode::Editing && !state.search_suggestions.is_empty() {
                state.search_suggestions.len().min(6) as u16 + 3
            } else {
                0
            };
        let chunks = if has_results {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(suggestion_height),
                    Constraint::Length(0),
                    Constraint::Min(0),
                ])
                .split(area)
        };

        let results_chunk = if has_results { chunks[2] } else { chunks[4] };
        search_bar_area = if has_results {
            Rect {
                x: chunks[0].x + 2,
                width: chunks[0].width.saturating_sub(4),
                ..chunks[0]
            }
        } else {
            let search_width = search_deck_width(area, state, false);
            centered_width(chunks[0], search_width)
        };
        render_search_bar(
            frame,
            search_bar_area,
            state,
            theme,
            view,
            show_cursor,
            false,
        );

        let list_block = Block::default();
        if !state.search_results.is_empty() {
            let poster_width = if state.image_supported {
                state.poster_rows.saturating_mul(4).div_ceil(3).max(6)
            } else {
                12
            };
            let results_area = results_chunk;
            let selected_idx = state.search_list_state.selected();

            let row_height = state.poster_rows.max(3) + 1;
            state.visible_items = (results_area.height as usize) / (row_height as usize);
            let rows = state
                .search_results
                .iter()
                .map(|_| Row::new(vec![Cell::from("")]).height(row_height));

            let table = Table::new(rows, [Constraint::Percentage(100)]).block(list_block);

            frame.render_stateful_widget(table, results_area, &mut state.search_list_state);
            let offset = state.search_list_state.offset();

            let inner_area = results_area;

            let mut current_y = inner_area.y;

            for (i, res) in state.search_results.iter().enumerate().skip(offset) {
                if current_y + state.poster_rows > inner_area.y + inner_area.height {
                    break;
                }

                let item_area = Rect {
                    x: inner_area.x,
                    y: current_y,
                    width: inner_area.width,
                    height: state.poster_rows,
                };

                let is_selected = Some(i) == selected_idx;
                if is_selected {
                    let selected_bg = theme.surface0.fg.unwrap_or(theme.base);
                    frame.render_widget(
                        Block::default().style(Style::default().bg(selected_bg)),
                        item_area,
                    );
                }

                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(2),
                        Constraint::Length(poster_width),
                        Constraint::Length(1),
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

                if state.image_supported {
                    if let Some(img) = state.search_posters.peek(&res.id) {
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
                            let img_height = poster_area.height.min(state.poster_rows);
                            let img_y_offset = item_area.height.saturating_sub(img_height) / 2;
                            let p_area = Rect {
                                y: poster_area.y + img_y_offset,
                                height: img_height,
                                ..poster_area
                            };
                            frame.render_widget(ratatui_image::Image::new(proto), p_area);
                        }
                    } else {
                        let placeholder = Paragraph::new("No\nPoster")
                            .style(theme.text_dim)
                            .alignment(Alignment::Center);
                        frame.render_widget(placeholder, poster_area);
                    }
                } else {
                    let placeholder_height = item_area.height.min(2);
                    let v_center = item_area.height.saturating_sub(placeholder_height) / 2;
                    let p_area = Rect {
                        x: poster_area.x,
                        y: poster_area.y + v_center,
                        width: 12,
                        height: placeholder_height,
                    };
                    let placeholder = Paragraph::new("No\nPoster")
                        .style(theme.text_dim)
                        .alignment(Alignment::Center);
                    frame.render_widget(placeholder, p_area);
                }

                let text_top_padding = text_area.height.saturating_sub(2) / 2;
                let text_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(text_top_padding),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(text_area);

                let title_style = if is_selected { theme.title } else { theme.text };
                // TV channels are never favoritable (see current_favorite_candidate), so
                // this is always false for TV mode's channel list, keeping it byte-identical.
                let is_favorited = res.stype != 3
                    && state
                        .favorites
                        .is_favorite(&crate::models::SubjectIdentity {
                            provider: res.provider.cache_key(),
                            subject_id: &res.id,
                            title: &res.title,
                            stype: res.stype,
                            release_year: &res.release_year,
                        });
                let title_reserved = if is_favorited { 6 } else { 4 };
                let max_title_width = text_area.width.saturating_sub(title_reserved) as usize;
                let display_title = crate::tui::text::truncate_width(&res.title, max_title_width);

                let mut type_tag = if state.is_tv_mode || res.stype == 3 {
                    "TV Channel".to_string()
                } else if res.stype == 1 {
                    "Movie".to_string()
                } else if res.stype == 2 {
                    "Series".to_string()
                } else {
                    "".to_string()
                };

                let is_history = state.search_query.trim().to_lowercase() == "/history";
                if !is_history && type_tag.is_empty() {
                    type_tag = "Unknown".to_string();
                }

                let mut title_spans = vec![ratatui::text::Span::raw(" ")];
                if is_favorited {
                    title_spans.push(ratatui::text::Span::styled(
                        if state.basic_terminal { "* " } else { "★ " },
                        theme.rating,
                    ));
                }
                title_spans.push(ratatui::text::Span::styled(display_title, title_style));
                let title_line = ratatui::text::Line::from(title_spans);
                if text_layout[1].height > 0 {
                    frame.render_widget(Paragraph::new(title_line), text_layout[1]);
                }

                let mut info_spans = vec![];

                if is_history {
                    if !type_tag.is_empty() {
                        info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                    }
                    if res.season > 0 {
                        info_spans.push(ratatui::text::Span::styled(
                            format!("S{:02}E{:02}", res.season, res.episode),
                            theme.text,
                        ));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                    }

                    if let Some(hist) = state.history.get_item(
                        res.provider.cache_key(),
                        &res.id,
                        res.season,
                        res.episode,
                        Some(&res.title),
                    ) {
                        if hist.is_in_progress() {
                            let (filled, empty) = hist.progress_bar_parts(8);
                            info_spans.push(ratatui::text::Span::styled(
                                filled,
                                theme.accent.add_modifier(ratatui::style::Modifier::BOLD),
                            ));
                            info_spans.push(ratatui::text::Span::styled(empty, theme.text_dim));

                            let pct = hist
                                .progress_percentage()
                                .map(|p| format!(" {:.0}%", p))
                                .unwrap_or_default();
                            info_spans.push(ratatui::text::Span::styled(pct, theme.text));

                            if let Some(r) = hist.formatted_remaining() {
                                info_spans.push(ratatui::text::Span::styled(
                                    format!(" ({r})"),
                                    theme.text_dim,
                                ));
                            }
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            info_spans.push(ratatui::text::Span::styled(
                                format!("Watched {}", hist.formatted_relative_time()),
                                theme.text_dim,
                            ));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        } else if hist.completed {
                            info_spans
                                .push(ratatui::text::Span::styled("[✓ Completed]", theme.text_dim));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            info_spans.push(ratatui::text::Span::styled(
                                format!("Watched {}", hist.formatted_relative_time()),
                                theme.text_dim,
                            ));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        }
                    }

                    info_spans.push(ratatui::text::Span::styled(
                        res.provider.to_string(),
                        theme.text,
                    ));
                } else {
                    macro_rules! push_year {
                        () => {
                            if res.release_year != "Unknown" && !res.release_year.is_empty() {
                                info_spans
                                    .push(ratatui::text::Span::styled(&res.release_year, theme.text));
                                info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            }
                        };
                    }

                    if is_selected {
                        if let Some(meta) = &state.search_preview {
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
                            push_year!();

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
                                info_spans.push(ratatui::text::Span::styled(
                                    g_names.join(" • "),
                                    theme.text,
                                ));
                                info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            }
                            info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                        } else if state.preview_loading {
                            push_year!();
                            info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            info_spans
                                .push(ratatui::text::Span::styled("Loading...", theme.text_dim));
                        } else {
                            push_year!();
                            info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                        }
                    } else {
                        push_year!();
                        info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                    }
                }

                if text_layout[2].height > 0 && !info_spans.is_empty() {
                    let mut padded = vec![ratatui::text::Span::raw(" ")];
                    padded.extend(info_spans);
                    frame.render_widget(
                        Paragraph::new(ratatui::text::Line::from(padded)),
                        text_layout[2],
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

                let sb_area = results_area;

                frame.render_stateful_widget(scrollbar, sb_area, &mut scrollbar_state);
            }
        } else {
            render_search_state(frame, chunks[4], state, theme, view);
        }
    }

    if state.input_mode == InputMode::Editing
        && !state.search_suggestions.is_empty()
        && search_bar_area.width > 0
    {
        let visible_count = state.search_suggestions.len().min(6);
        let selected_index = state.suggest_index.unwrap_or(0);
        let suggestion_offset = selected_index
            .saturating_add(1)
            .saturating_sub(visible_count)
            .min(state.search_suggestions.len().saturating_sub(visible_count));

        let is_centered = state.search_results.is_empty();

        let search_text_len = if is_centered {
            let content_sample = search_content(state, view, false, search_bar_area.width);
            crate::tui::text::width(&content_sample) as u16
        } else {
            0
        };

        let prompt_x = if is_centered {
            search_bar_area.x + search_bar_area.width.saturating_sub(search_text_len) / 2
        } else {
            search_bar_area.x
        };

        let available_width = area.right().saturating_sub(prompt_x).saturating_sub(2);
        let start_y = search_bar_area.bottom();

        let visible_slice: Vec<(usize, &String)> = state
            .search_suggestions
            .iter()
            .enumerate()
            .skip(suggestion_offset)
            .take(visible_count)
            .collect();

        for (row_idx, &(orig_idx, suggestion)) in visible_slice.iter().enumerate() {
            let current_y = start_y + row_idx as u16;
            if current_y >= area.bottom() {
                break;
            }

            let is_selected = Some(orig_idx) == state.suggest_index;
            let is_last = row_idx + 1 == visible_slice.len()
                && suggestion_offset + visible_count >= state.search_suggestions.len();

            let branch_symbol = if is_last {
                if state.basic_terminal {
                    "\\- "
                } else {
                    "└─ "
                }
            } else if state.basic_terminal {
                "|- "
            } else {
                "├─ "
            };

            let is_slash_cmd = suggestion.starts_with('/');
            let display_name = if is_slash_cmd {
                suggestion.strip_prefix('/').unwrap_or(suggestion)
            } else {
                suggestion.as_str()
            };

            let desc = slash_command_description(suggestion, state);

            let branch_style = if is_selected {
                theme.lavender.add_modifier(Modifier::BOLD)
            } else {
                theme.overlay0
            };

            let text_style = if is_selected {
                theme.highlight.add_modifier(Modifier::BOLD)
            } else {
                theme.text_dim
            };

            let desc_style = if is_selected {
                theme.subtext1.add_modifier(Modifier::BOLD)
            } else {
                theme.overlay1
            };

            let mut spans = vec![
                Span::styled(branch_symbol, branch_style),
                Span::styled(display_name, text_style),
            ];

            if let Some(description) = desc {
                let name_len = crate::tui::text::width(display_name);
                let pad = 24usize.saturating_sub(name_len).max(2);
                spans.push(Span::raw(" ".repeat(pad)));
                spans.push(Span::styled(description, desc_style));
            }

            let row_area = Rect {
                x: prompt_x,
                y: current_y,
                width: available_width,
                height: 1,
            };

            frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
        }
    }
    if state.tv_config_popup {
        let rows = state.tv_manager_rows();
        let total_rows = rows.len();
        let content_width = state
            .tv_playlists
            .iter()
            .map(|source| crate::tui::text::width(source))
            .max()
            .unwrap_or(28)
            .max(48)
            .max(crate::tui::text::width(
                "[ Add URL ] [ Add file ] [ Reload ] [ Done ]",
            ));
        let popup_width = 68u16
            .max(content_width.saturating_add(6) as u16)
            .min(area.width.saturating_sub(4));
        let popup_height = if state.tv_input_active {
            7u16
        } else {
            total_rows.min(10).saturating_add(6) as u16
        };
        let popup_area = crate::tui::overlay::centered(area, popup_width, popup_height, 36, 74);
        crate::tui::overlay::clear_modal_area(frame, area, popup_area, theme);

        let title = format!(
            " TV Playlists · {}/{} ",
            state.tv_manager_selected.saturating_add(1),
            total_rows.max(1)
        );
        let popup_block = ratatui::widgets::Block::default()
            .title(title)
            .title_style(theme.title)
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(crate::tui::overlay::border_type(state.basic_terminal))
            .border_style(theme.lavender);

        let inner_area = popup_block.inner(popup_area);
        frame.render_widget(popup_block, popup_area);

        let sections = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(2),
        ])
        .split(inner_area);

        if state.tv_input_active {
            let label = if state.tv_input_is_file {
                "Enter playlist file path:"
            } else {
                "Enter playlist URL:"
            };
            let lines = vec![
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::raw(" "),
                    ratatui::text::Span::styled(label, theme.sapphire),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(" ❯ ", theme.sapphire),
                    ratatui::text::Span::styled(&state.tv_input_buffer, theme.text),
                    ratatui::text::Span::styled("█", theme.rating),
                ]),
            ];
            frame.render_widget(
                ratatui::widgets::Paragraph::new(lines)
                    .wrap(ratatui::widgets::Wrap { trim: false }),
                sections[0],
            );
        } else {
            let items: Vec<ratatui::widgets::ListItem> = rows
                .iter()
                .map(|row| {
                    use crate::tui::state::TvManagerRow;
                    match row {
                        TvManagerRow::Header(label) => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled(label.to_string(), theme.muted),
                            ]))
                        }
                        TvManagerRow::Playlist(index) => {
                            let source =
                                state.tv_playlists.get(*index).cloned().unwrap_or_default();
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled(
                                    format!("{} {}", index + 1, source),
                                    theme.text,
                                ),
                            ]))
                        }
                        TvManagerRow::AddUrl => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Add URL ]", theme.sapphire),
                            ]))
                        }
                        TvManagerRow::AddFile => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Add file ]", theme.sapphire),
                            ]))
                        }
                        TvManagerRow::Reload => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Reload ]", theme.rating),
                            ]))
                        }
                        TvManagerRow::Done => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Done ]", theme.success),
                            ]))
                        }
                    }
                })
                .collect();

            let list = ratatui::widgets::List::new(items)
                .highlight_style(crate::tui::overlay::selection_style(
                    theme,
                    state.basic_terminal,
                ))
                .highlight_symbol(if state.basic_terminal { "> " } else { "▌ " });

            let mut list_state = ratatui::widgets::ListState::default();
            list_state.select(Some(state.tv_manager_selected));
            frame.render_stateful_widget(list, sections[0], &mut list_state);
        }

        let footer = if state.tv_input_active {
            ratatui::text::Line::from(vec![
                crate::tui::overlay::key_hint("Enter", "Add", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Esc", "Cancel", theme),
            ])
        } else {
            ratatui::text::Line::from(vec![
                crate::tui::overlay::key_hint("↑↓", "Move", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Enter", "Select", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("d", "Remove", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Esc", "Close", theme),
            ])
        };

        frame.render_widget(
            ratatui::widgets::Paragraph::new(footer)
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::TOP)
                        .border_style(theme.muted),
                ),
            sections[1],
        );
    }

    if state.addon_manager_popup {
        let addons_count = state.installed_addons.len();
        let total_rows = state.addon_manager_rows().len();
        let popup_width = 76u16.min(area.width.saturating_sub(4)).max(56);
        let popup_height = if state.addon_input_active {
            7u16
        } else {
            (addons_count as u16)
                .saturating_add(6)
                .min(area.height.saturating_sub(4))
                .max(7)
        };
        let popup_area = crate::tui::overlay::centered(area, popup_width, popup_height, 36, 80);
        crate::tui::overlay::clear_modal_area(frame, area, popup_area, theme);

        let title = format!(
            " Addons Manager · {}/{} ",
            state.addon_manager_selected.saturating_add(1),
            total_rows.max(1)
        );
        let popup_block = ratatui::widgets::Block::default()
            .title(title)
            .title_style(theme.title)
            .borders(ratatui::widgets::Borders::ALL)
            .border_type(crate::tui::overlay::border_type(state.basic_terminal))
            .border_style(theme.lavender);

        let inner_area = popup_block.inner(popup_area);
        frame.render_widget(popup_block, popup_area);

        if state.addon_input_active {
            let sections = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(2),
            ])
            .split(inner_area);

            let chars: Vec<char> = state.addon_input_buffer.chars().collect();
            let cursor = state.addon_input_cursor.min(chars.len());
            let max_width = inner_area.width.saturating_sub(6) as usize;

            let mut start = 0;
            if cursor >= max_width {
                start = cursor - max_width + 1;
            }

            let mut before_cursor: String = chars[start..cursor].iter().collect();
            if start > 0 && before_cursor.chars().count() > 3 {
                before_cursor = format!("...{}", &before_cursor[3..]);
            }

            let cursor_char = if cursor < chars.len() {
                chars[cursor].to_string()
            } else {
                " ".to_string()
            };

            let end = (start + max_width).min(chars.len());
            let mut after_cursor: String = chars[cursor.saturating_add(1).min(chars.len())..end]
                .iter()
                .collect();
            if end < chars.len() {
                let len = after_cursor.chars().count();
                if len > 3 {
                    let mut a_chars: Vec<char> = after_cursor.chars().collect();
                    a_chars.truncate(len - 3);
                    after_cursor = format!("{}...", a_chars.into_iter().collect::<String>());
                } else if len > 0 {
                    after_cursor = "...".to_string();
                }
            }

            let lines = vec![
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::raw(" "),
                    ratatui::text::Span::styled("Enter Addon Manifest URL:", theme.sapphire),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(" ❯ ", theme.sapphire),
                    ratatui::text::Span::styled(before_cursor, theme.text),
                    ratatui::text::Span::styled(
                        cursor_char,
                        theme.text.add_modifier(ratatui::style::Modifier::REVERSED),
                    ),
                    ratatui::text::Span::styled(after_cursor, theme.text),
                ]),
            ];
            frame.render_widget(ratatui::widgets::Paragraph::new(lines), sections[0]);

            let footer = ratatui::text::Line::from(vec![
                crate::tui::overlay::key_hint("Enter", "Add", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Esc", "Cancel", theme),
            ]);
            frame.render_widget(
                ratatui::widgets::Paragraph::new(footer)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::TOP)
                            .border_style(theme.muted),
                    ),
                sections[1],
            );
        } else {
            let sections = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(2),
            ])
            .split(inner_area);

            let is_add_selected = state.addon_manager_selected == state.installed_addons.len() + 1;

            let mut items = vec![ratatui::widgets::ListItem::new(ratatui::text::Line::from(
                vec![
                    ratatui::text::Span::raw("   "),
                    ratatui::text::Span::styled("Installed Addons", theme.muted),
                ],
            ))];

            for (idx, a) in state.installed_addons.iter().enumerate() {
                let row_idx = idx + 1;
                let is_selected = state.addon_manager_selected == row_idx;
                let prefix = if is_selected {
                    ratatui::text::Span::styled(
                        if state.basic_terminal { "> " } else { "▌ " },
                        theme.sapphire,
                    )
                } else {
                    ratatui::text::Span::raw("  ")
                };
                let check = if a.enabled {
                    ratatui::text::Span::styled("[x] ", theme.success)
                } else {
                    ratatui::text::Span::styled("[ ] ", theme.text_dim)
                };
                let name = ratatui::text::Span::styled(
                    format!("{} v{} ", a.name, a.version.as_deref().unwrap_or("1.0")),
                    if is_selected {
                        theme.text.add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        theme.text
                    },
                );
                let mut badges = Vec::new();
                if a.is_core() {
                    badges.push(ratatui::text::Span::styled("[Core] ", theme.lavender));
                }
                if a.provides_meta {
                    badges.push(ratatui::text::Span::styled("[Meta] ", theme.sapphire));
                }
                if a.provides_stream {
                    badges.push(ratatui::text::Span::styled("[Streams] ", theme.rating));
                }
                if a.provides_catalog {
                    badges.push(ratatui::text::Span::styled("[Catalog]", theme.teal));
                }
                let mut spans = vec![ratatui::text::Span::raw(" "), prefix, check, name];
                spans.extend(badges);
                items.push(ratatui::widgets::ListItem::new(ratatui::text::Line::from(
                    spans,
                )));
            }

            let list = ratatui::widgets::List::new(items);
            frame.render_widget(list, sections[0]);

            let add_prefix = if is_add_selected {
                if state.basic_terminal { "> " } else { "▌ " }
            } else {
                "  "
            };
            let add_button = if is_add_selected {
                ratatui::text::Span::styled(
                    format!("{add_prefix}[ Add Manifest URL ]"),
                    theme.sapphire.add_modifier(ratatui::style::Modifier::BOLD),
                )
            } else {
                ratatui::text::Span::styled(
                    format!("{add_prefix}[ Add Manifest URL ]"),
                    theme.sapphire,
                )
            };

            let button_line =
                ratatui::text::Line::from(vec![ratatui::text::Span::raw(" "), add_button]);
            frame.render_widget(ratatui::widgets::Paragraph::new(button_line), sections[1]);

            let footer = ratatui::text::Line::from(vec![
                crate::tui::overlay::key_hint("↑↓←→", "Move", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Enter/Space", "Toggle/Select", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("d", "Remove", theme),
                ratatui::text::Span::raw("  "),
                crate::tui::overlay::key_hint("Esc", "Close", theme),
            ]);

            frame.render_widget(
                ratatui::widgets::Paragraph::new(footer)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::TOP)
                            .border_style(theme.muted),
                    ),
                sections[2],
            );
        }
    }

    if state.show_browse_popup {
        let items: Vec<String> = if state.mode() == crate::tui::state::AppMode::Addon {
            crate::providers::addons::models::curated_catalog_presets(&state.installed_addons)
                .into_iter()
                .map(|target| target.label)
                .collect()
        } else {
            crate::tui::state::BrowsePreset::ALL
                .iter()
                .map(|preset| preset.label().to_string())
                .collect()
        };
        crate::tui::clear_area(frame, area, theme);
        crate::tui::overlay::picker(
            frame,
            area,
            &items,
            &mut state.browse_list_state,
            crate::tui::overlay::PickerSpec {
                title: "Browse",
                confirm_label: "Open",
                minimum_width: 36,
            },
            theme,
            state.basic_terminal,
        );
    }

    if state.player_picker_popup {
        let items = state
            .available_players
            .iter()
            .map(|k| k.label().to_string())
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
}
