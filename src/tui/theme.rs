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
    pub rating: Style,
    pub accent: Style,
    pub muted: Style,
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
                .fg(Color::Rgb(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Rgb(245, 194, 231))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(Color::Rgb(243, 139, 168)),
            success: Style::default().fg(Color::Rgb(166, 227, 161)),
            shortcut: Style::default().fg(Color::Rgb(250, 179, 135)),
            overlay: Style::default().fg(Color::Rgb(108, 112, 134)),
            rating: Style::default().fg(Color::Rgb(249, 226, 175)),
            accent: Style::default()
                .fg(Color::Rgb(137, 220, 235))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::Rgb(88, 91, 112)),
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default().to_lowercase();
        let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
        
        let truecolor = colorterm == "truecolor" || colorterm == "24bit" || term.contains("truecolor");
        
        if !truecolor && (term.contains("apple") || term == "dumb" || term == "linux" || std::env::var("TERM_PROGRAM").unwrap_or_default() == "Apple_Terminal") {
            Self::fallback()
        } else {
            Self::default()
        }
    }

    pub fn fallback() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focus: Style::default().fg(Color::Blue),
            text: Style::default().fg(Color::Reset),
            text_dim: Style::default().fg(Color::Gray),
            title: Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            highlight: Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            header: Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            error: Style::default().fg(Color::Red),
            success: Style::default().fg(Color::Green),
            shortcut: Style::default().fg(Color::Yellow),
            overlay: Style::default().fg(Color::DarkGray),
            rating: Style::default().fg(Color::Yellow),
            accent: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::DarkGray),
        }
    }
}
