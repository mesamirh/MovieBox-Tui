use crate::tui::{
    library::LibraryItem,
    state::AppState,
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], state, theme);
    draw_list(frame, chunks[1], state, theme);
    draw_footer(frame, chunks[2], state, theme);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let root = crate::tui::library::download_root();
    let count = state.library_items.len();
    let title = if count == 0 {
        " Library — no downloads yet ".to_string()
    } else {
        format!(" Library — {count} download{} ", if count == 1 { "" } else { "s" })
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.border_focus);

    let path_line = Line::from(vec![
        Span::styled("Folder: ", theme.text_dim),
        Span::styled(root.to_string_lossy().into_owned(), theme.sapphire),
    ]);

    frame.render_widget(
        Paragraph::new(path_line)
            .block(block)
            .alignment(Alignment::Left),
        area,
    );
}

fn draw_list(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let border = Block::default()
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.border)
        .title(" Downloads ")
        .title_style(theme.header);

    if state.library_items.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Nothing downloaded yet.",
                theme.text,
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", theme.text_dim),
                Span::styled("d", theme.shortcut),
                Span::styled(" on a movie/episode to download, then ", theme.text_dim),
                Span::styled("r", theme.shortcut),
                Span::styled(" here to refresh.", theme.text_dim),
            ]),
            Line::from(vec![
                Span::styled("Press ", theme.text_dim),
                Span::styled("Esc", theme.shortcut),
                Span::styled(" or ", theme.text_dim),
                Span::styled("l", theme.shortcut),
                Span::styled(" to go back.", theme.text_dim),
            ]),
        ])
        .block(border)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = state
        .library_items
        .iter()
        .map(|item| library_row(item, theme))
        .collect();

    // Keep selection valid
    if let Some(sel) = state.library_list_state.selected() {
        if sel >= state.library_items.len() {
            state
                .library_list_state
                .select(Some(state.library_items.len().saturating_sub(1)));
        }
    } else if !state.library_items.is_empty() {
        state.library_list_state.select(Some(0));
    }

    let widths = [
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(18),
        Constraint::Length(6),
    ];

    let header = Row::new(vec![
        Cell::from(Span::styled("Type", theme.header)),
        Cell::from(Span::styled("Title", theme.header)),
        Cell::from(Span::styled("Size", theme.header)),
        Cell::from(Span::styled("Location", theme.header)),
        Cell::from(Span::styled("Subs", theme.header)),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let table = Table::new(rows, widths)
        .header(header)
        .block(border)
        .row_highlight_style(
            Style::default()
                .fg(theme.highlight.fg.unwrap_or(ratatui::style::Color::Cyan))
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol(if state.basic_terminal { "> " } else { "❯ " });

    frame.render_stateful_widget(table, area, &mut state.library_list_state);
}

fn library_row<'a>(item: &'a LibraryItem, theme: &'a Theme) -> Row<'a> {
    let kind_style = match item.kind {
        crate::tui::library::LibraryKind::Movie => theme.teal,
        crate::tui::library::LibraryKind::Series => theme.lavender,
    };
    let subs = if item.subtitle.is_some() { "yes" } else { "—" };
    Row::new(vec![
        Cell::from(Span::styled(item.kind_label(), kind_style)),
        Cell::from(Span::styled(item.title.clone(), theme.text)),
        Cell::from(Span::styled(item.size_label(), theme.text_dim)),
        Cell::from(Span::styled(
            truncate(&item.location, 16),
            theme.subtext1,
        )),
        Cell::from(Span::styled(subs, theme.text_dim)),
    ])
}

fn draw_footer(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let selected = state
        .library_list_state
        .selected()
        .and_then(|i| state.library_items.get(i));

    let path_hint = selected
        .map(|item| item.path.to_string_lossy().into_owned())
        .unwrap_or_default();

    let hints = Line::from(vec![
        Span::styled("[↑↓]", theme.shortcut),
        Span::styled(" Move  ", theme.text_dim),
        Span::styled("[Enter]", theme.shortcut),
        Span::styled(" Choose player  ", theme.text_dim),
        Span::styled("[o]", theme.shortcut),
        Span::styled(" Player  ", theme.text_dim),
        Span::styled("[r]", theme.shortcut),
        Span::styled(" Refresh  ", theme.text_dim),
        Span::styled("[Esc]", theme.shortcut),
        Span::styled(" Back  ", theme.text_dim),
        Span::styled("[?]", theme.shortcut),
        Span::styled(" Help", theme.text_dim),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.border);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(Paragraph::new(hints).alignment(Alignment::Center), rows[0]);
    if !path_hint.is_empty() && rows[1].height > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                crate::tui::text::truncate_width(&path_hint, rows[1].width as usize),
                theme.muted,
            )))
            .alignment(Alignment::Center),
            rows[1],
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
