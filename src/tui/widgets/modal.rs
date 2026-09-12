use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::{overlay, theme::Theme};

pub struct ModalFrame<'a> {
    title: &'a str,
    theme: &'a Theme,
    basic_terminal: bool,
    border_style: Option<Style>,
    title_style: Option<Style>,
}

impl<'a> ModalFrame<'a> {
    pub fn new(title: &'a str, theme: &'a Theme, basic_terminal: bool) -> Self {
        Self {
            title,
            theme,
            basic_terminal,
            border_style: None,
            title_style: None,
        }
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = Some(style);
        self
    }
    pub fn render(&self, frame: &mut Frame, area: Rect, full_area: Rect) -> Rect {
        overlay::clear_modal_area(frame, full_area, area, self.theme);
        let trimmed_title = self.title.trim();
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(overlay::border_type(self.basic_terminal))
            .border_style(self.border_style.unwrap_or(self.theme.lavender));
        if !trimmed_title.is_empty() {
            let title_budget = (area.width as usize).saturating_sub(4);
            let display_title = crate::tui::text::truncate_width(trimmed_title, title_budget);
            block = block
                .title(format!(" {display_title} "))
                .title_style(self.title_style.unwrap_or(self.theme.title));
        }
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    }
}
pub fn render_modal_footer(
    frame: &mut Frame,
    area: Rect,
    spans: Vec<Span<'static>>,
    theme: &Theme,
) {
    let footer = Line::from(spans);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme.muted),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_modal_frame_render() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::mocha();

        let popup = Rect::new(10, 5, 60, 14);
        terminal
            .draw(|f| {
                let full = Rect::new(0, 0, 80, 24);
                let modal = ModalFrame::new("Test Modal", &theme, false);
                let inner = modal.render(f, popup, full);
                assert_eq!(inner.width, 58);
                assert_eq!(inner.height, 12);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(popup.x, popup.y)].bg, ratatui::style::Color::Reset);
        assert_eq!(
            buffer[(popup.x + popup.width - 1, popup.y)].bg,
            ratatui::style::Color::Reset
        );
        assert_eq!(
            buffer[(popup.x, popup.y + popup.height - 1)].bg,
            ratatui::style::Color::Reset
        );
        assert_eq!(
            buffer[(popup.x + popup.width - 1, popup.y + popup.height - 1)].bg,
            ratatui::style::Color::Reset
        );
    }

    #[test]
    fn test_render_modal_footer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                let footer_area = Rect::new(10, 15, 60, 2);
                let spans = vec![Span::raw("Enter: Save  Esc: Cancel")];
                render_modal_footer(f, footer_area, spans, &theme);
            })
            .unwrap();
    }
}
