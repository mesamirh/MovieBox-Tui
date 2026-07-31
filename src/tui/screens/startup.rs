use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let panel = centered_rect(68.min(area.width.saturating_sub(4)), 10, area);
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" MOVIEBOX ", theme.title),
            Span::styled("· STARTUP ", theme.text_dim),
        ]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border_focus)
        .padding(Padding::new(3, 3, 1, 1));
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let dots = match (state.tick_count / 4) % 4 {
        0 => "",
        1 => ".",
        2 => "..",
        _ => "...",
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("Checking GitHub for a newer release{dots}"),
            theme.text,
        )]))
        .alignment(Alignment::Center),
        rows[1],
    );

    let footer = Rect {
        x: panel.x,
        y: panel.y.saturating_add(panel.height),
        width: panel.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("CURRENT  ", theme.muted),
            Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                theme.text_dim.add_modifier(Modifier::BOLD),
            ),
            Span::styled("   •   AUTOMATIC UPDATES ON", theme.muted),
        ]))
        .alignment(Alignment::Center),
        footer,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height: height.min(area.height),
    }
}
