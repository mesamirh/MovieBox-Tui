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
    pub teal: Style,
    pub lavender: Style,
    pub sapphire: Style,
    pub subtext1: Style,
    pub base: Color,
    pub bg: Style,
    pub rosewater: Style,
    pub flamingo: Style,
    pub maroon: Style,
    pub surface0: Style,
    pub surface1: Style,
    pub surface2: Style,
    pub overlay0: Style,
    pub overlay1: Style,
    pub overlay2: Style,
    pub mantle: Style,
    pub crust: Style,
    pub is_light: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::mocha()
    }
}

impl Theme {
    pub fn mocha() -> Self {
        Self {
            border: Style::default().fg(cp(88, 91, 112)),
            border_focus: Style::default().fg(cp(137, 180, 250)),
            text: Style::default().fg(cp(205, 214, 244)),
            text_dim: Style::default().fg(cp(166, 173, 200)),
            title: Style::default()
                .fg(cp(203, 166, 247))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(137, 180, 250))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(245, 194, 231))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(243, 139, 168)),
            success: Style::default().fg(cp(166, 227, 161)),
            shortcut: Style::default().fg(cp(250, 179, 135)),
            overlay: Style::default().fg(cp(108, 112, 134)),
            rating: Style::default().fg(cp(249, 226, 175)),
            accent: Style::default()
                .fg(cp(137, 220, 235))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(88, 91, 112)),
            teal: Style::default().fg(cp(148, 226, 213)),
            lavender: Style::default().fg(cp(180, 190, 254)),
            sapphire: Style::default().fg(cp(116, 199, 236)),
            subtext1: Style::default().fg(cp(186, 194, 222)),
            base: cp(30, 30, 46),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(245, 224, 220)),
            flamingo: Style::default().fg(cp(242, 205, 205)),
            maroon: Style::default().fg(cp(235, 160, 172)),
            surface0: Style::default().fg(cp(56, 58, 74)),
            surface1: Style::default().fg(cp(73, 76, 94)),
            surface2: Style::default().fg(cp(88, 91, 112)),
            overlay0: Style::default().fg(cp(108, 112, 134)),
            overlay1: Style::default().fg(cp(127, 132, 156)),
            overlay2: Style::default().fg(cp(147, 153, 178)),
            mantle: Style::default().fg(cp(24, 24, 37)),
            crust: Style::default().fg(cp(17, 17, 27)),
            is_light: false,
        }
    }

    pub fn latte() -> Self {
        Self {
            border: Style::default().fg(cp(156, 160, 176)),
            border_focus: Style::default().fg(cp(30, 102, 245)),
            text: Style::default().fg(cp(76, 79, 105)),
            text_dim: Style::default().fg(cp(108, 111, 133)),
            title: Style::default()
                .fg(cp(136, 57, 239))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(30, 102, 245))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(234, 118, 203))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(210, 15, 57)),
            success: Style::default().fg(cp(64, 160, 43)),
            shortcut: Style::default().fg(cp(254, 100, 11)),
            overlay: Style::default().fg(cp(156, 160, 176)),
            rating: Style::default().fg(cp(223, 142, 29)),
            accent: Style::default()
                .fg(cp(23, 146, 153))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(156, 160, 176)),
            teal: Style::default().fg(cp(23, 146, 153)),
            lavender: Style::default().fg(cp(114, 135, 253)),
            sapphire: Style::default().fg(cp(32, 159, 181)),
            subtext1: Style::default().fg(cp(92, 95, 119)),
            base: cp(239, 241, 245),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(220, 138, 120)),
            flamingo: Style::default().fg(cp(221, 120, 120)),
            maroon: Style::default().fg(cp(230, 69, 83)),
            surface0: Style::default().fg(cp(204, 208, 218)),
            surface1: Style::default().fg(cp(188, 192, 204)),
            surface2: Style::default().fg(cp(172, 176, 190)),
            overlay0: Style::default().fg(cp(156, 160, 176)),
            overlay1: Style::default().fg(cp(140, 143, 161)),
            overlay2: Style::default().fg(cp(124, 127, 147)),
            mantle: Style::default().fg(cp(230, 233, 239)),
            crust: Style::default().fg(cp(220, 224, 232)),
            is_light: true,
        }
    }

    pub fn new() -> Self {
        let colorterm = std::env::var("COLORTERM")
            .unwrap_or_default()
            .to_lowercase();
        let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let has_no_color = std::env::var("NO_COLOR").is_ok();
        let is_light = crate::tui::terminal::background_is_light();

        match std::env::var("MOVIEBOX_THEME")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "latte" | "light" => return Self::latte(),
            "mocha" | "dark" => return Self::mocha(),
            _ => {}
        }

        if has_no_color {
            return Self::monochrome(is_light);
        }

        let truecolor = colorterm == "truecolor"
            || colorterm == "24bit"
            || term.contains("truecolor")
            || term.contains("kitty")
            || std::env::var("WT_SESSION").is_ok()
            || term_program == "iTerm.app"
            || term_program == "Hyper"
            || term_program == "Tabby"
            || term_program == "WezTerm"
            || term_program == "WarpTerminal"
            || term_program == "vscode"
            || term_program == "ghostty";

        let basic = term == "dumb" || term == "linux" || term.contains("fbterm");
        let apple_term = term.contains("apple") || term_program == "Apple_Terminal";
        let has_256 = term.contains("256") || term.contains("xterm") || term.contains("screen");

        if truecolor {
            if is_light {
                Self::latte()
            } else {
                Self::mocha()
            }
        } else if basic || (apple_term && !has_256) || term == "xterm" {
            Self::fallback(is_light)
        } else if has_256 {
            if is_light {
                Self::latte()
            } else {
                Self::mocha()
            }
        } else {
            Self::fallback(is_light)
        }
    }

    pub fn monochrome(is_light: bool) -> Self {
        let foreground = if is_light { Color::Black } else { Color::White };
        let dim = if is_light {
            Color::DarkGray
        } else {
            Color::Gray
        };
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focus: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            text: Style::default().fg(foreground),
            text_dim: Style::default().fg(dim),
            title: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            highlight: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            header: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            error: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            success: Style::default().fg(foreground),
            shortcut: Style::default().fg(foreground),
            overlay: Style::default().fg(Color::DarkGray),
            rating: Style::default().fg(foreground),
            accent: Style::default().fg(foreground).add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::DarkGray),
            teal: Style::default().fg(foreground),
            lavender: Style::default().fg(foreground),
            sapphire: Style::default().fg(foreground),
            subtext1: Style::default().fg(foreground),
            base: Color::Reset,
            bg: Style::default(),
            rosewater: Style::default().fg(foreground),
            flamingo: Style::default().fg(foreground),
            maroon: Style::default().fg(foreground),
            surface0: Style::default().fg(Color::DarkGray),
            surface1: Style::default().fg(Color::DarkGray),
            surface2: Style::default().fg(Color::DarkGray),
            overlay0: Style::default().fg(Color::DarkGray),
            overlay1: Style::default().fg(dim),
            overlay2: Style::default().fg(dim),
            mantle: Style::default().fg(foreground),
            crust: Style::default().fg(foreground),
            is_light,
        }
    }

    pub fn fallback(is_light: bool) -> Self {
        if is_light {
            return Self::latte();
        }
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focus: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            text: Style::default().fg(Color::White),
            text_dim: Style::default().fg(Color::Gray),
            title: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Green),
            shortcut: Style::default().fg(Color::Yellow),
            overlay: Style::default().fg(Color::DarkGray),
            rating: Style::default().fg(Color::Yellow),
            accent: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(Color::DarkGray),
            teal: Style::default().fg(Color::Cyan),
            lavender: Style::default().fg(Color::Blue),
            sapphire: Style::default().fg(Color::Cyan),
            subtext1: Style::default().fg(Color::White),
            base: Color::Black,
            bg: Style::default(),
            rosewater: Style::default().fg(Color::White),
            flamingo: Style::default().fg(Color::Magenta),
            maroon: Style::default().fg(Color::Red),
            surface0: Style::default().fg(Color::DarkGray),
            surface1: Style::default().fg(Color::DarkGray),
            surface2: Style::default().fg(Color::DarkGray),
            overlay0: Style::default().fg(Color::DarkGray),
            overlay1: Style::default().fg(Color::Gray),
            overlay2: Style::default().fg(Color::Gray),
            mantle: Style::default().fg(Color::Black),
            crust: Style::default().fg(Color::Black),
            is_light,
        }
    }
}

fn cp(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}
