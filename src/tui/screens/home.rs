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
    } else if state.is_loading {
        SearchViewState::Loading
    } else if state.search_error.is_some() {
        SearchViewState::Error
    } else if !state.search_results.is_empty() {
        SearchViewState::Results
    } else if !state.search_query.trim().is_empty()
        && state
            .status_message
            .to_ascii_lowercase()
            .starts_with("no matches")
    {
        SearchViewState::NoResults
    } else {
        SearchViewState::Empty
    }
}

fn search_hint(view: SearchViewState, width: u16, theme: &Theme) -> Line<'static> {
    let text = match view {
        SearchViewState::Editing if width >= 82 => "[↑↓] Suggestions  [Enter] Search  [Esc] Cancel",
        SearchViewState::Editing if width >= 54 => "[↑↓] Suggest  [Enter] Search  [Esc] Cancel",
        SearchViewState::Editing => "[Enter] Search  [Esc] Cancel",
        SearchViewState::Error if width >= 62 => "[Enter] Retry  [Type] Edit  [Esc] Clear",
        SearchViewState::Error => "[Enter] Retry  [Esc] Clear",
        SearchViewState::Results if width >= 62 => "[Type] Edit  [↑↓] Browse  [Enter] Open",
        SearchViewState::Results => "[↑↓] Browse  [Enter] Open",
        SearchViewState::NoResults if width >= 62 => "[Type] Edit  [Enter] Retry  [Esc] Clear",
        SearchViewState::NoResults => "[Type] Edit  [Esc] Clear",
        SearchViewState::Loading => "",
        SearchViewState::Empty => "",
    };

    let mut spans = Vec::new();
    let mut remaining = text;
    while let Some(open) = remaining.find('[') {
        if open > 0 {
            spans.push(Span::styled(remaining[..open].to_string(), theme.text_dim));
        }
        let Some(close) = remaining[open..].find(']') else {
            spans.push(Span::styled(remaining[open..].to_string(), theme.text_dim));
            remaining = "";
            break;
        };
        let close = open + close;
        spans.push(Span::styled("[", theme.text_dim));
        spans.push(Span::styled(
            remaining[open + 1..close].to_string(),
            theme.shortcut,
        ));
        spans.push(Span::styled("]", theme.text_dim));
        remaining = &remaining[close + 1..];
    }
    if !remaining.is_empty() {
        spans.push(Span::styled(remaining.to_string(), theme.text_dim));
    }
    Line::from(spans).centered()
}

fn centered_width(area: Rect, maximum: u16) -> Rect {
    let width = area.width.min(maximum).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        width,
        ..area
    }
}

fn search_deck_width(area: Rect, state: &AppState, landing: bool) -> u16 {
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
            Line::from(vec![Span::styled(
                format!("Searching for “{query}”{dots}"),
                theme.lavender,
            )])
        }
        SearchViewState::NoResults => {
            let symbol = if state.basic_terminal { "-" } else { pulse };
            let style = if (state.tick_count / 4) % 2 == 0 {
                theme.lavender
            } else {
                theme.subtext1
            };
            Line::from(vec![
                Span::styled(format!("{symbol} "), style),
                Span::styled(format!("Nothing found for “{query}”"), theme.text),
            ])
        }
        SearchViewState::Error => {
            let symbol = if state.basic_terminal { "!" } else { "×" };
            Line::from(vec![
                Span::styled(format!("{symbol} "), theme.error),
                Span::styled(
                    crate::tui::text::truncate_width(
                        &state.status_message,
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

fn render_browse_bar(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    browse_view: crate::providers::browse::BrowseView,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let count = state.search_results.len();
    let status = if state.is_loading && count == 0 {
        "loading…".to_string()
    } else {
        format!("{count} results")
    };
    let arrow = state.browse_sort.arrow();
    let sort_label = state.browse_sort.label();
    let text = format!(" {} {arrow} · {} ", browse_view.label(), status);
    let text_width = crate::tui::text::width(&text) as u16;
    let hint = if area.width >= 60 {
        format!("[s] Sort {sort_label}  [Alt+b] Switch view  [Esc] Exit")
    } else {
        "[s] Sort  [Esc] Exit".to_string()
    };
    let hint_width = crate::tui::text::width(&hint) as u16;
    let total = text_width.saturating_add(hint_width).saturating_add(4);
    let pad = area.width.saturating_sub(total) / 2;
    let mut spans = vec![
        Span::styled(" ".repeat(pad as usize), theme.text_dim),
        Span::styled(text, theme.accent),
    ];
    if area.width >= total + 4 {
        spans.push(Span::styled(" ".repeat(2), theme.text_dim));
        spans.push(Span::styled(hint.to_string(), theme.text_dim));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), rows[0]);

    let rule = if state.basic_terminal { "-" } else { "─" };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            rule.repeat(area.width as usize),
            theme.border,
        ))),
        rows[1],
    );
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
    let rule_style = if view == SearchViewState::Editing {
        theme.border_focus
    } else if view == SearchViewState::Error {
        theme.error
    } else {
        theme.border
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    let result_status = if view == SearchViewState::Results {
        Some(format!("{} results", state.search_results.len()))
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
        .split(rows[0]);
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

    let rule = if state.basic_terminal { "-" } else { "─" };
    let rule_text = rule.repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(rule_text, rule_style))),
        rows[1],
    );
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let show_cursor = (state.tick_count % 16) < 8;
    let view = search_view_state(state);
    let mut search_bar_area = Rect::default();
    let mut suggestion_area = Rect::default();

    if view == SearchViewState::Empty
        || (view == SearchViewState::Editing && state.search_results.is_empty())
    {
        if state.tick_count < 1 {
            return;
        }

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
        let suggestions_open =
            state.input_mode == InputMode::Editing && !state.search_suggestions.is_empty();
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(18),
                Constraint::Length(logo_height),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(2),
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

        let logo_style = if state.basic_terminal || state.tick_count >= 8 {
            theme.title
        } else {
            let t = state.tick_count as f32 / 8.0;
            let (start, end) = logo_fade_colors(theme);
            let r = (start.0 + (end.0 - start.0) * t) as u8;
            let g = (start.1 + (end.1 - start.1) * t) as u8;
            let b = (start.2 + (end.2 - start.2) * t) as u8;
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(r, g, b))
        };

        if !state.basic_terminal && state.tick_count < 15 {
            let rows: Vec<&str> = logo_text.split('\n').collect();
            for (i, row) in rows.iter().enumerate() {
                let row_tick_start = 1 + i as u64;
                if state.tick_count >= row_tick_start {
                    let row_t = ((state.tick_count - row_tick_start) as f32 / 7.0).clamp(0.0, 1.0);
                    let (start, end) = logo_fade_colors(theme);
                    let r = (start.0 + (end.0 - start.0) * row_t) as u8;
                    let g = (start.1 + (end.1 - start.1) * row_t) as u8;
                    let b = (start.2 + (end.2 - start.2) * row_t) as u8;
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
            theme.surface1
        } else {
            theme.text_dim
        };
        let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Right)
            .style(version_style);
        frame.render_widget(version, version_chunks[1]);

        if state.tick_count >= 3 {
            let search_width = search_deck_width(area, state, true);
            search_bar_area = centered_width(vertical_chunks[4], search_width);
            suggestion_area = Rect {
                x: search_bar_area.x,
                y: search_bar_area.bottom(),
                width: search_bar_area.width,
                height: area.bottom().saturating_sub(search_bar_area.bottom()),
            };

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

            let context = if state.is_tv_mode {
                Line::from(vec![
                    Span::styled("Live TV", theme.accent),
                    Span::styled(" • ", theme.muted),
                    Span::styled("Local playlists", theme.text_dim),
                ])
            } else {
                Line::from(vec![
                    Span::styled(state.active_provider.label().to_string(), theme.accent),
                    Span::styled(" • ", theme.muted),
                    Span::styled("Streaming", theme.text_dim),
                ])
            };
            let context_area = if suggestions_open {
                Rect::default()
            } else if view == SearchViewState::Empty {
                vertical_chunks[5]
            } else {
                frame.render_widget(
                    Paragraph::new(search_hint(view, search_bar_area.width, theme))
                        .alignment(Alignment::Center),
                    vertical_chunks[5],
                );
                vertical_chunks[6]
            };
            if context_area.width > 0 {
                frame.render_widget(
                    Paragraph::new(context).alignment(Alignment::Center),
                    context_area,
                );
            }

            let footer = Line::from(vec![
                Span::styled("[", theme.text_dim),
                Span::styled("?", theme.shortcut),
                Span::styled("] ", theme.text_dim),
                Span::styled("Help", theme.text_dim),
                Span::raw("    "),
                Span::styled("[", theme.text_dim),
                Span::styled("q", theme.shortcut),
                Span::styled("] ", theme.text_dim),
                Span::styled("Quit", theme.text_dim),
            ]);
            frame.render_widget(
                Paragraph::new(footer).alignment(Alignment::Center),
                vertical_chunks[8],
            );
        }
    } else {
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
                .constraints([Constraint::Length(2), Constraint::Min(0)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Length(suggestion_height),
                    Constraint::Length(0),
                    Constraint::Min(0),
                ])
                .split(area)
        };

        let search_width = search_deck_width(area, state, false);
        search_bar_area = centered_width(chunks[0], search_width);
        let results_chunk = if has_results { chunks[1] } else { chunks[4] };
        suggestion_area = if has_results { chunks[1] } else { chunks[2] };
        if let Some(browse_view) = state.browse_view
            && state.input_mode == InputMode::Normal
        {
            render_browse_bar(frame, search_bar_area, state, theme, browse_view);
        } else {
            render_search_bar(
                frame,
                search_bar_area,
                state,
                theme,
                view,
                show_cursor,
                false,
            );
        }
        let suggestions_open =
            state.input_mode == InputMode::Editing && !state.search_suggestions.is_empty();
        if !suggestions_open && !has_results {
            frame.render_widget(
                Paragraph::new(search_hint(view, search_bar_area.width, theme))
                    .alignment(Alignment::Center),
                if has_results { chunks[0] } else { chunks[1] },
            );
        }

        let list_block = Block::default();
        if state.is_loading && state.search_results.is_empty() {
            render_search_state(frame, results_chunk, state, theme, SearchViewState::Loading);
        } else if !state.search_results.is_empty() {
            let poster_width = if state.image_supported {
                state.poster_rows.saturating_mul(4).div_ceil(3).max(6)
            } else {
                12
            };
            let results_area = results_chunk;
            let selected_idx = state.search_list_state.selected();
            let offset = state.search_list_state.offset();

            let row_height = state.poster_rows.max(3) + 1;
            state.visible_items = (results_area.height as usize) / (row_height as usize);
            let rows = state
                .search_results
                .iter()
                .map(|_| Row::new(vec![Cell::from("")]).height(row_height));

            let table = Table::new(rows, [Constraint::Percentage(100)]).block(list_block);

            frame.render_stateful_widget(table, results_area, &mut state.search_list_state);

            let inner_area = results_area;

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
                        let placeholder = Paragraph::new("Poster\nunavailable")
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
                    let placeholder = Paragraph::new("Poster\nunsupported")
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
                let max_title_width = text_area.width.saturating_sub(4) as usize;
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
                if is_history {
                    let mut extra = vec![];
                    if res.season > 0 {
                        extra.push(format!("S{:02}E{:02}", res.season, res.episode));
                    }
                    extra.push(res.provider.to_string());

                    if !extra.is_empty() {
                        if type_tag.is_empty() {
                            type_tag = extra.join(" • ");
                        } else {
                            type_tag = format!("{} • {}", type_tag, extra.join(" • "));
                        }
                    }
                } else if type_tag.is_empty() {
                    type_tag = "Unknown".to_string();
                }

                let title_line = ratatui::text::Line::from(vec![
                    ratatui::text::Span::raw(" "),
                    ratatui::text::Span::styled(display_title, title_style),
                ]);
                if text_layout[1].height > 0 {
                    frame.render_widget(Paragraph::new(title_line), text_layout[1]);
                }

                let mut info_spans = vec![];

                macro_rules! push_year {
                    () => {
                        if res.release_year != "Unknown" && !res.release_year.is_empty() {
                            info_spans
                                .push(ratatui::text::Span::styled(&res.release_year, theme.text));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        }
                    };
                }

                // IMDb badge from curated browse data — shown on every row.
                if let Some(rating) = &res.imdb_rating {
                    let star = if state.basic_terminal { "* " } else { "★ " };
                    info_spans.push(ratatui::text::Span::styled(star, theme.rating));
                    info_spans.push(ratatui::text::Span::styled(rating.clone(), theme.text));
                    info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                }

                if is_selected {
                    if state.preview_loading || state.is_loading {
                        push_year!();
                        info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                        info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        info_spans.push(ratatui::text::Span::styled("Loading...", theme.text_dim));
                    } else if let Some(meta) = &state.search_preview {
                        if res.imdb_rating.is_none() {
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
                            info_spans
                                .push(ratatui::text::Span::styled(g_names.join(" • "), theme.text));
                            info_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                        }
                        info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                    } else {
                        push_year!();
                        info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                    }
                } else {
                    push_year!();
                    info_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
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
        let search_area = search_bar_area;
        let visible_count = state.search_suggestions.len().min(6);
        let dropdown_height = visible_count as u16 + 4;
        let selected_index = state.suggest_index.unwrap_or(0);
        let suggestion_offset = selected_index
            .saturating_add(1)
            .saturating_sub(visible_count)
            .min(state.search_suggestions.len().saturating_sub(visible_count));
        let dropdown_width = search_area.width;
        let dropdown_x = search_area.x;
        let dropdown_area = if suggestion_area.height >= dropdown_height {
            Rect {
                x: dropdown_x,
                y: suggestion_area.y,
                width: dropdown_width,
                height: dropdown_height,
            }
        } else {
            Rect {
                x: dropdown_x,
                y: search_area.y + search_area.height,
                width: dropdown_width,
                height: dropdown_height,
            }
        };

        if dropdown_area.y + dropdown_area.height <= area.y + area.height {
            let surface = theme.surface0.fg.unwrap_or(theme.base);
            let selected_surface = theme.surface1.fg.unwrap_or(surface);
            frame.render_widget(
                Block::default().style(Style::default().bg(surface)),
                dropdown_area,
            );
            let dropdown_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(visible_count as u16),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(dropdown_area);
            let items: Vec<ratatui::widgets::ListItem> = state
                .search_suggestions
                .iter()
                .enumerate()
                .skip(suggestion_offset)
                .take(visible_count)
                .map(|(i, s)| {
                    let selected = Some(i) == state.suggest_index;
                    let marker = if selected {
                        if state.basic_terminal { "> " } else { "▌ " }
                    } else {
                        "  "
                    };
                    let text = format!("{marker}{s}");
                    let style = if selected {
                        theme.highlight
                    } else {
                        theme.text
                    };
                    ratatui::widgets::ListItem::new(
                        ratatui::text::Line::from(ratatui::text::Span::styled(text, style))
                            .alignment(ratatui::layout::Alignment::Left),
                    )
                    .style(if selected {
                        theme.lavender.bg(selected_surface)
                    } else {
                        theme.text.bg(surface)
                    })
                })
                .collect();
            let position = state
                .suggest_index
                .map(|index| format!("{}/{}", index + 1, state.search_suggestions.len()))
                .unwrap_or_else(|| state.search_suggestions.len().to_string());
            let heading = Line::from(vec![
                Span::styled(" Suggestions", theme.title),
                Span::styled(" · ", theme.overlay0),
                Span::styled(position, theme.subtext1),
            ]);
            frame.render_widget(
                Paragraph::new(heading).style(Style::default().bg(surface)),
                dropdown_rows[0],
            );
            let list = ratatui::widgets::List::new(items)
                .highlight_style(
                    theme
                        .lavender
                        .bg(selected_surface)
                        .add_modifier(Modifier::BOLD),
                )
                .style(Style::default().bg(surface));
            frame.render_widget(list, dropdown_rows[1]);

            let footer = if dropdown_area.width >= 50 {
                Line::from(vec![
                    Span::styled(" [", theme.text_dim),
                    Span::styled("↑↓", theme.shortcut),
                    Span::styled("] Move   [", theme.text_dim),
                    Span::styled("Enter", theme.shortcut),
                    Span::styled("] Use   [", theme.text_dim),
                    Span::styled("Esc", theme.shortcut),
                    Span::styled("] Close", theme.text_dim),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" [", theme.text_dim),
                    Span::styled("↑↓", theme.shortcut),
                    Span::styled("] Move  [", theme.text_dim),
                    Span::styled("Enter", theme.shortcut),
                    Span::styled("] Use  [", theme.text_dim),
                    Span::styled("Esc", theme.shortcut),
                    Span::styled("]", theme.text_dim),
                ])
            }
            .centered();
            let separator_symbol = if state.basic_terminal { "-" } else { "─" };
            let separator = separator_symbol.repeat(dropdown_rows[2].width as usize);
            frame.render_widget(
                Paragraph::new(separator)
                    .style(theme.surface1.bg(surface))
                    .alignment(Alignment::Center),
                dropdown_rows[2],
            );
            frame.render_widget(
                Paragraph::new(footer).style(Style::default().bg(surface)),
                dropdown_rows[3],
            );
            if state.search_suggestions.len() > visible_count {
                let mut scrollbar_state = ratatui::widgets::ScrollbarState::default()
                    .content_length(state.search_suggestions.len())
                    .position(selected_index);
                let scrollbar = ratatui::widgets::Scrollbar::default()
                    .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol(if state.basic_terminal { "|" } else { "█" });
                let scrollbar_area = Rect {
                    x: dropdown_rows[1].x,
                    y: dropdown_rows[1].y,
                    width: dropdown_rows[1].width,
                    height: dropdown_rows[1].height,
                };
                frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
            }
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
            .max(40)
            .max(crate::tui::text::width(
                "[ Add URL ] [ Add file ] [ Reload ] [ Done ]",
            ));
        let (popup_width, popup_height) = if state.tv_input_active {
            (content_width.saturating_add(6) as u16, 6)
        } else {
            (
                content_width.saturating_add(6) as u16,
                total_rows.min(10).saturating_add(5) as u16,
            )
        };
        let popup_area = crate::tui::overlay::centered(area, popup_width, popup_height, 36, 70);
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
            .border_type(if state.basic_terminal {
                ratatui::widgets::BorderType::Plain
            } else {
                ratatui::widgets::BorderType::Rounded
            })
            .border_style(theme.lavender);

        let inner_area = popup_block.inner(popup_area);
        frame.render_widget(popup_block, popup_area);

        let sections = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(2),
        ])
        .split(inner_area);

        if state.tv_input_active {
            let prompt = format!(
                "{}: {}{}",
                if state.tv_input_is_file {
                    "Enter playlist file path"
                } else {
                    "Enter playlist URL"
                },
                state.tv_input_buffer,
                "_"
            );
            frame.render_widget(
                ratatui::widgets::Paragraph::new(ratatui::text::Line::from(
                    ratatui::text::Span::styled(prompt, theme.text),
                ))
                .wrap(ratatui::widgets::Wrap { trim: true }),
                sections[0],
            );
        } else {
            let items: Vec<ratatui::widgets::ListItem> = rows
                .iter()
                .map(|row| {
                    use crate::tui::state::TvManagerRow;
                    let (text, style) = match row {
                        TvManagerRow::Header(label) => (label.to_string(), theme.muted),
                        TvManagerRow::Playlist(index) => {
                            let source =
                                state.tv_playlists.get(*index).cloned().unwrap_or_default();
                            (format!("{} {}", index + 1, source), theme.text)
                        }
                        TvManagerRow::AddUrl => ("[ Add URL ]".to_string(), theme.sapphire),
                        TvManagerRow::AddFile => ("[ Add file ]".to_string(), theme.sapphire),
                        TvManagerRow::Reload => ("[ Reload ]".to_string(), theme.rating),
                        TvManagerRow::Done => ("[ Done ]".to_string(), theme.success),
                    };
                    ratatui::widgets::ListItem::new(ratatui::text::Line::from(
                        ratatui::text::Span::styled(text, style),
                    ))
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

    if state.browse_menu_open {
        let items = crate::providers::browse::BrowseView::ALL
            .iter()
            .map(|view| view.label().to_string())
            .collect::<Vec<_>>();
        crate::tui::overlay::picker(
            frame,
            area,
            &items,
            &mut state.browse_list_state,
            crate::tui::overlay::PickerSpec {
                title: "Browse",
                confirm_label: "Open",
                minimum_width: 30,
            },
            theme,
            state.basic_terminal,
        );
    }
}

fn logo_fade_colors(theme: &Theme) -> ((f32, f32, f32), (f32, f32, f32)) {
    let get_rgb = |style: ratatui::style::Style, default: (f32, f32, f32)| {
        if let Some(ratatui::style::Color::Rgb(r, g, b)) = style.fg {
            (r as f32, g as f32, b as f32)
        } else {
            default
        }
    };

    let start = get_rgb(theme.surface1, (73.0, 76.0, 94.0));
    let end = get_rgb(theme.title, (203.0, 166.0, 247.0));
    (start, end)
}
