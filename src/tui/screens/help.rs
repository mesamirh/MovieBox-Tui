use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, _state: &AppState, theme: &Theme) {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(16),
            Constraint::Percentage(25),
        ])
        .split(area);

    let popup_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Min(45),
            Constraint::Percentage(25),
        ])
        .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_chunk);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "  Global",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    [?]        ", theme.header),
            Span::styled("Toggle Help Menu", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [q]        ", theme.header),
            Span::styled("Quit Application", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Esc]      ", theme.header),
            Span::styled("Go Back / Clear", theme.text),
        ]),
        Line::from(vec![]),
        Line::from(vec![Span::styled(
            "  Navigation",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    [↑] / [↓]  ", theme.header),
            Span::styled("Scroll Lists", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [←] / [→]  ", theme.header),
            Span::styled("Switch Panels", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Enter]    ", theme.header),
            Span::styled("Select / Submit", theme.text),
        ]),
        Line::from(vec![]),
        Line::from(vec![Span::styled(
            "  Discover & Search",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    /movies    ", theme.header),
            Span::styled("Discover Movies", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    /shows     ", theme.header),
            Span::styled("Discover TV Shows", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    /anime     ", theme.header),
            Span::styled("Discover Anime", theme.text),
        ]),
        Line::from(vec![]),
        Line::from(vec![Span::styled(
            "  Stream Controls",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    [p]        ", theme.header),
            Span::styled("Play Video", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [d]        ", theme.header),
            Span::styled("Download Video", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [c]        ", theme.header),
            Span::styled("Copy URL to Clipboard", theme.text),
        ]),
    ];

    let block = Block::default()
        .title(" Keybindings Help ")
        .title_alignment(Alignment::Center)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border_focus);

    let p = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(p, popup_chunk);
}
