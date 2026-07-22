use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Clear, List, ListItem},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(60),
            ratatui::layout::Constraint::Percentage(20),
        ])
        .split(area);

    let popup_area = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(70),
            ratatui::layout::Constraint::Percentage(15),
        ])
        .split(popup_layout[1])[1];

    frame.render_widget(Clear, popup_area);

    let list_items: Vec<ListItem> = state
        .logs
        .iter()
        .map(|log| ListItem::new(log.as_str()))
        .collect();

    let logs_block = Block::default()
        .borders(Borders::ALL)
        .title(" Trace Log Console [Press 'l' to toggle] ")
        .title_style(theme.title)
        .border_style(theme.border_focus);

    let logs_list = List::new(list_items)
        .block(logs_block)
        .style(theme.text_dim);

    frame.render_widget(logs_list, popup_area);
}
