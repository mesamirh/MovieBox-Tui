use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub border: Style,
    pub border_focus: Style,
    pub text: Style,
    pub text_dim: Style,
    pub title: Style,
    pub highlight: Style,
    pub header: Style,
    pub error: Style,
    pub success: Style,
    pub shortcut: Style,
    pub overlay: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Rgb(88, 91, 112)),
            border_focus: Style::default().fg(Color::Rgb(137, 180, 250)),
            text: Style::default().fg(Color::Rgb(205, 214, 244)),
            text_dim: Style::default().fg(Color::Rgb(166, 173, 200)),
            title: Style::default()
                .fg(Color::Rgb(203, 166, 247))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .bg(Color::Rgb(49, 50, 68))
                .fg(Color::Rgb(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(245, 194, 231))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(Color::Rgb(243, 139, 168)),
            success: Style::default().fg(Color::Rgb(166, 227, 161)),
            shortcut: Style::default().fg(Color::Rgb(250, 179, 135)),
            overlay: Style::default().fg(Color::Rgb(108, 112, 134)),
        }
    }
}
