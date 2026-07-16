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
            Constraint::Length(14),
            Constraint::Percentage(25),
        ])
        .split(area);

    let popup_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Min(40),
            Constraint::Percentage(25),
        ])
        .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_chunk);

    let help_text = vec![
        Line::from(vec![
            Span::styled(" [q]      ", theme.header),
            Span::styled("Quit Application", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [?]      ", theme.header),
            Span::styled("Toggle Help Menu", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [/], [s] ", theme.header),
            Span::styled("Search Movies/Series", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [l]      ", theme.header),
            Span::styled("Toggle Logs", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [b], ESC ", theme.header),
            Span::styled("Go Back / Clear", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [↑]/[k]  ", theme.header),
            Span::styled("Move Up", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [↓]/[j]  ", theme.header),
            Span::styled("Move Down", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [Enter]  ", theme.header),
            Span::styled("Select / Submit", theme.text),
        ]),
        Line::from(vec![
            Span::styled(" [y]      ", theme.header),
            Span::styled("Copy URL (Details Screen)", theme.text),
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
