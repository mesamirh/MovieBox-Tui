use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub const AVAILABLE_THEMES: [&str; 9] = [
    "Mocha",
    "Latte",
    "Macchiato",
    "Frappe",
    "Nord",
    "TokyoNight",
    "Dracula",
    "Gruvbox",
    "RosePine",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Mocha,
    Latte,
    Macchiato,
    Frappe,
    Nord,
    TokyoNight,
    Dracula,
    Gruvbox,
    RosePine,
}

impl ThemeKind {
    pub const ALL: [Self; 9] = [
        Self::Mocha,
        Self::Latte,
        Self::Macchiato,
        Self::Frappe,
        Self::Nord,
        Self::TokyoNight,
        Self::Dracula,
        Self::Gruvbox,
        Self::RosePine,
    ];

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "latte" | "light" => ThemeKind::Latte,
            "macchiato" => ThemeKind::Macchiato,
            "frappe" => ThemeKind::Frappe,
            "nord" => ThemeKind::Nord,
            "tokyonight" => ThemeKind::TokyoNight,
            "dracula" => ThemeKind::Dracula,
            "gruvbox" | "gruvboxdark" => ThemeKind::Gruvbox,
            "rosepine" => ThemeKind::RosePine,
            "catppuccin" | "mocha" => ThemeKind::Mocha,
            _ => ThemeKind::Mocha,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeKind::Mocha => "Mocha",
            ThemeKind::Latte => "Latte",
            ThemeKind::Macchiato => "Macchiato",
            ThemeKind::Frappe => "Frappe",
            ThemeKind::Nord => "Nord",
            ThemeKind::TokyoNight => "TokyoNight",
            ThemeKind::Dracula => "Dracula",
            ThemeKind::Gruvbox => "Gruvbox",
            ThemeKind::RosePine => "RosePine",
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSupport {
    NoColor,
    Truecolor,
    Color256,
    Basic,
}
impl ColorSupport {
    pub fn current() -> Self {
        if std::env::var("NO_COLOR").is_ok() {
            return ColorSupport::NoColor;
        }
        if std::env::var("WT_SESSION").is_ok() {
            return ColorSupport::Truecolor;
        }
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        classify_terminal(&colorterm, &term, &term_program)
    }
    pub fn label(&self) -> &'static str {
        match self {
            ColorSupport::Truecolor => "TrueColor (24-bit RGB)",
            ColorSupport::Color256 => "256 Colors (8-bit ANSI)",
            ColorSupport::Basic => "Basic (16 Colors)",
            ColorSupport::NoColor => "No Color (Monochrome)",
        }
    }
}

pub(crate) fn classify_terminal(colorterm: &str, term: &str, term_program: &str) -> ColorSupport {
    let colorterm = colorterm.to_lowercase();
    let term = term.to_lowercase();
    let term_program = term_program.to_lowercase();
    let truecolor = colorterm == "truecolor"
        || colorterm == "24bit"
        || term.contains("truecolor")
        || term.contains("kitty")
        || term.contains("ghostty")
        || term.starts_with("foot")
        || term.contains("alacritty")
        || term_program == "iterm.app"
        || term_program == "hyper"
        || term_program == "tabby"
        || term_program == "wezterm"
        || term_program == "warpterminal"
        || term_program == "warp"
        || term_program == "vscode"
        || term_program == "ghostty"
        || term_program == "konsole"
        || term_program == "xfce4-terminal"
        || std::env::var("VTE_VERSION").is_ok()
        || std::env::var("WEZTERM_EXECUTABLE").is_ok()
        || std::env::var("ALACRITTY_WINDOW_ID").is_ok()
        || std::env::var("TILIX_ID").is_ok();
    let basic =
        term == "dumb" || term == "linux" || term.contains("fbterm") || term.starts_with("vt");
    let apple_term = term.contains("apple") || term_program == "apple_terminal";
    let has_256 = term.contains("256") || term.contains("xterm") || term.contains("screen");
    if basic || (apple_term && !has_256) || (term == "xterm" && !truecolor) {
        ColorSupport::Basic
    } else if truecolor {
        ColorSupport::Truecolor
    } else if has_256 || cfg!(target_os = "windows") {
        ColorSupport::Color256
    } else {
        ColorSupport::Basic
    }
}
pub fn theme_color(style: Style, fallback: Color) -> Color {
    style.fg.unwrap_or(fallback)
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
            border: Style::default().fg(cp(140, 143, 161)),
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
            overlay: Style::default().fg(cp(108, 111, 133)),
            rating: Style::default().fg(cp(223, 142, 29)),
            accent: Style::default()
                .fg(cp(23, 146, 153))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(108, 111, 133)),
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
            overlay0: Style::default().fg(cp(108, 111, 133)),
            overlay1: Style::default().fg(cp(92, 95, 119)),
            overlay2: Style::default().fg(cp(76, 79, 105)),
            mantle: Style::default().fg(cp(230, 233, 239)),
            crust: Style::default().fg(cp(220, 224, 232)),
            is_light: true,
        }
    }

    pub fn from_kind(kind: ThemeKind) -> Self {
        let base = match kind {
            ThemeKind::Latte => Self::latte(),
            ThemeKind::Macchiato => Self::macchiato(),
            ThemeKind::Frappe => Self::frappe(),
            ThemeKind::Nord => Self::nord(),
            ThemeKind::TokyoNight => Self::tokyo_night(),
            ThemeKind::Dracula => Self::dracula(),
            ThemeKind::Gruvbox => Self::gruvbox(),
            ThemeKind::RosePine => Self::rose_pine(),
            ThemeKind::Mocha => Self::mocha(),
        };
        match ColorSupport::current() {
            ColorSupport::Color256 => base.quantized_to_256(),
            _ => base,
        }
    }
    pub fn palette_swatch_spans(theme_name: &str, basic_terminal: bool) -> Vec<Span<'static>> {
        let kind = ThemeKind::parse(theme_name);
        let sample = Self::from_kind(kind);

        if basic_terminal {
            return vec![Span::styled("* * *", sample.text_dim)];
        }

        let accent_color = sample.accent.fg.unwrap_or(Color::Cyan);
        let surface_color = sample
            .surface0
            .fg
            .or(sample.surface1.fg)
            .unwrap_or(Color::DarkGray);
        let base_color = sample.base;

        vec![
            Span::styled("■ ", Style::default().fg(accent_color)),
            Span::styled("■ ", Style::default().fg(surface_color)),
            Span::styled("■", Style::default().fg(base_color)),
        ]
    }
}

impl Theme {
    pub fn detect() -> Self {
        Self::detect_with_light(None)
    }

    pub fn detect_with_light(light: Option<bool>) -> Self {
        let resolved_light = light.unwrap_or_else(crate::tui::terminal::background_is_light);
        if std::env::var("NO_COLOR").is_ok() {
            return Self::monochrome(resolved_light);
        }
        Self::detect_from(ColorSupport::current(), resolved_light)
    }

    pub(crate) fn detect_from(support: ColorSupport, is_light: bool) -> Self {
        match support {
            ColorSupport::NoColor => Self::monochrome(is_light),
            ColorSupport::Truecolor => {
                if is_light {
                    Self::latte()
                } else {
                    Self::mocha()
                }
            }
            ColorSupport::Color256 => {
                if is_light {
                    Self::latte().quantized_to_256()
                } else {
                    Self::mocha().quantized_to_256()
                }
            }
            ColorSupport::Basic => Self::fallback(is_light),
        }
    }

    pub fn quantized_to_256(mut self) -> Self {
        self.border = quantize_style(self.border);
        self.border_focus = quantize_style(self.border_focus);
        self.text = quantize_style(self.text);
        self.text_dim = quantize_style(self.text_dim);
        self.title = quantize_style(self.title);
        self.highlight = quantize_style(self.highlight);
        self.header = quantize_style(self.header);
        self.error = quantize_style(self.error);
        self.success = quantize_style(self.success);
        self.shortcut = quantize_style(self.shortcut);
        self.overlay = quantize_style(self.overlay);
        self.rating = quantize_style(self.rating);
        self.accent = quantize_style(self.accent);
        self.muted = quantize_style(self.muted);
        self.teal = quantize_style(self.teal);
        self.lavender = quantize_style(self.lavender);
        self.sapphire = quantize_style(self.sapphire);
        self.subtext1 = quantize_style(self.subtext1);
        self.base = to_indexed_256(self.base);
        self.bg = quantize_style(self.bg);
        self.rosewater = quantize_style(self.rosewater);
        self.flamingo = quantize_style(self.flamingo);
        self.maroon = quantize_style(self.maroon);
        self.surface0 = quantize_style(self.surface0);
        self.surface1 = quantize_style(self.surface1);
        self.surface2 = quantize_style(self.surface2);
        self.overlay0 = quantize_style(self.overlay0);
        self.overlay1 = quantize_style(self.overlay1);
        self.overlay2 = quantize_style(self.overlay2);
        self.mantle = quantize_style(self.mantle);
        self.crust = quantize_style(self.crust);
        self
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
            return Self {
                border: Style::default().fg(Color::DarkGray),
                border_focus: Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                text: Style::default().fg(Color::Black),
                text_dim: Style::default().fg(Color::DarkGray),
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
                shortcut: Style::default().fg(Color::Red),
                overlay: Style::default().fg(Color::DarkGray),
                rating: Style::default().fg(Color::Yellow),
                accent: Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                muted: Style::default().fg(Color::DarkGray),
                teal: Style::default().fg(Color::Cyan),
                lavender: Style::default().fg(Color::Blue),
                sapphire: Style::default().fg(Color::Blue),
                subtext1: Style::default().fg(Color::Black),
                base: Color::White,
                bg: Style::default(),
                rosewater: Style::default().fg(Color::Black),
                flamingo: Style::default().fg(Color::Magenta),
                maroon: Style::default().fg(Color::Red),
                surface0: Style::default().fg(Color::DarkGray),
                surface1: Style::default().fg(Color::DarkGray),
                surface2: Style::default().fg(Color::Black),
                overlay0: Style::default().fg(Color::DarkGray),
                overlay1: Style::default().fg(Color::DarkGray),
                overlay2: Style::default().fg(Color::Black),
                mantle: Style::default().fg(Color::White),
                crust: Style::default().fg(Color::White),
                is_light: true,
            };
        }
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focus: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            text: Style::default().fg(Color::White),
            text_dim: Style::default().fg(Color::Gray),
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::Cyan)
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
            lavender: Style::default().fg(Color::Cyan),
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
            is_light: false,
        }
    }

    pub fn macchiato() -> Self {
        Self {
            border: Style::default().fg(cp(91, 96, 120)),
            border_focus: Style::default().fg(cp(138, 173, 244)),
            text: Style::default().fg(cp(202, 211, 245)),
            text_dim: Style::default().fg(cp(165, 173, 203)),
            title: Style::default()
                .fg(cp(198, 160, 246))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(138, 173, 244))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(245, 189, 230))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(237, 135, 150)),
            success: Style::default().fg(cp(166, 218, 149)),
            shortcut: Style::default().fg(cp(245, 169, 127)),
            overlay: Style::default().fg(cp(110, 115, 141)),
            rating: Style::default().fg(cp(238, 212, 159)),
            accent: Style::default()
                .fg(cp(125, 196, 228))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(91, 96, 120)),
            teal: Style::default().fg(cp(139, 213, 202)),
            lavender: Style::default().fg(cp(183, 189, 248)),
            sapphire: Style::default().fg(cp(125, 196, 228)),
            subtext1: Style::default().fg(cp(184, 192, 224)),
            base: cp(36, 39, 58),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(244, 219, 214)),
            flamingo: Style::default().fg(cp(240, 198, 198)),
            maroon: Style::default().fg(cp(238, 153, 160)),
            surface0: Style::default().fg(cp(54, 58, 79)),
            surface1: Style::default().fg(cp(73, 77, 100)),
            surface2: Style::default().fg(cp(91, 96, 120)),
            overlay0: Style::default().fg(cp(110, 115, 141)),
            overlay1: Style::default().fg(cp(128, 135, 162)),
            overlay2: Style::default().fg(cp(147, 154, 183)),
            mantle: Style::default().fg(cp(30, 32, 48)),
            crust: Style::default().fg(cp(24, 25, 38)),
            is_light: false,
        }
    }

    pub fn frappe() -> Self {
        Self {
            border: Style::default().fg(cp(98, 104, 128)),
            border_focus: Style::default().fg(cp(140, 170, 238)),
            text: Style::default().fg(cp(198, 208, 245)),
            text_dim: Style::default().fg(cp(165, 173, 206)),
            title: Style::default()
                .fg(cp(202, 158, 230))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(140, 170, 238))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(244, 184, 228))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(231, 130, 132)),
            success: Style::default().fg(cp(166, 209, 137)),
            shortcut: Style::default().fg(cp(239, 159, 118)),
            overlay: Style::default().fg(cp(115, 121, 148)),
            rating: Style::default().fg(cp(229, 200, 144)),
            accent: Style::default()
                .fg(cp(129, 200, 190))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(98, 104, 128)),
            teal: Style::default().fg(cp(129, 200, 190)),
            lavender: Style::default().fg(cp(186, 187, 241)),
            sapphire: Style::default().fg(cp(133, 193, 220)),
            subtext1: Style::default().fg(cp(181, 191, 226)),
            base: cp(48, 52, 70),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(242, 213, 207)),
            flamingo: Style::default().fg(cp(238, 190, 190)),
            maroon: Style::default().fg(cp(234, 153, 156)),
            surface0: Style::default().fg(cp(65, 69, 89)),
            surface1: Style::default().fg(cp(81, 87, 109)),
            surface2: Style::default().fg(cp(98, 104, 128)),
            overlay0: Style::default().fg(cp(115, 121, 148)),
            overlay1: Style::default().fg(cp(131, 139, 167)),
            overlay2: Style::default().fg(cp(148, 156, 187)),
            mantle: Style::default().fg(cp(41, 44, 60)),
            crust: Style::default().fg(cp(35, 38, 52)),
            is_light: false,
        }
    }

    pub fn nord() -> Self {
        Self {
            border: Style::default().fg(cp(67, 76, 94)),
            border_focus: Style::default().fg(cp(136, 192, 208)),
            text: Style::default().fg(cp(216, 222, 233)),
            text_dim: Style::default().fg(cp(146, 153, 163)),
            title: Style::default()
                .fg(cp(180, 142, 173))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(136, 192, 208))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(163, 190, 140))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(191, 97, 106)),
            success: Style::default().fg(cp(163, 190, 140)),
            shortcut: Style::default().fg(cp(208, 135, 112)),
            overlay: Style::default().fg(cp(76, 86, 106)),
            rating: Style::default().fg(cp(235, 203, 139)),
            accent: Style::default()
                .fg(cp(143, 188, 187))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(67, 76, 94)),
            teal: Style::default().fg(cp(143, 188, 187)),
            lavender: Style::default().fg(cp(129, 161, 193)),
            sapphire: Style::default().fg(cp(94, 129, 172)),
            subtext1: Style::default().fg(cp(229, 233, 240)),
            base: cp(46, 52, 64),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(216, 222, 233)),
            flamingo: Style::default().fg(cp(216, 222, 233)),
            maroon: Style::default().fg(cp(191, 97, 106)),
            surface0: Style::default().fg(cp(59, 66, 82)),
            surface1: Style::default().fg(cp(67, 76, 94)),
            surface2: Style::default().fg(cp(76, 86, 106)),
            overlay0: Style::default().fg(cp(94, 106, 131)),
            overlay1: Style::default().fg(cp(114, 126, 151)),
            overlay2: Style::default().fg(cp(146, 153, 163)),
            mantle: Style::default().fg(cp(40, 45, 56)),
            crust: Style::default().fg(cp(36, 40, 50)),
            is_light: false,
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            border: Style::default().fg(cp(56, 62, 90)),
            border_focus: Style::default().fg(cp(122, 162, 247)),
            text: Style::default().fg(cp(192, 202, 245)),
            text_dim: Style::default().fg(cp(169, 177, 214)),
            title: Style::default()
                .fg(cp(187, 154, 247))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(122, 162, 247))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(255, 158, 100))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(247, 118, 142)),
            success: Style::default().fg(cp(158, 206, 106)),
            shortcut: Style::default().fg(cp(255, 158, 100)),
            overlay: Style::default().fg(cp(86, 95, 137)),
            rating: Style::default().fg(cp(224, 175, 104)),
            accent: Style::default()
                .fg(cp(42, 203, 213))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(56, 62, 90)),
            teal: Style::default().fg(cp(115, 218, 202)),
            lavender: Style::default().fg(cp(187, 154, 247)),
            sapphire: Style::default().fg(cp(125, 207, 255)),
            subtext1: Style::default().fg(cp(169, 177, 214)),
            base: cp(26, 27, 38),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(224, 175, 104)),
            flamingo: Style::default().fg(cp(247, 118, 142)),
            maroon: Style::default().fg(cp(247, 118, 142)),
            surface0: Style::default().fg(cp(31, 35, 53)),
            surface1: Style::default().fg(cp(41, 46, 66)),
            surface2: Style::default().fg(cp(65, 72, 104)),
            overlay0: Style::default().fg(cp(86, 95, 137)),
            overlay1: Style::default().fg(cp(106, 115, 157)),
            overlay2: Style::default().fg(cp(126, 135, 177)),
            mantle: Style::default().fg(cp(22, 22, 30)),
            crust: Style::default().fg(cp(22, 22, 30)),
            is_light: false,
        }
    }

    pub fn dracula() -> Self {
        Self {
            border: Style::default().fg(cp(98, 114, 164)),
            border_focus: Style::default().fg(cp(189, 147, 249)),
            text: Style::default().fg(cp(248, 248, 242)),
            text_dim: Style::default().fg(cp(158, 164, 186)),
            title: Style::default()
                .fg(cp(189, 147, 249))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(139, 233, 253))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(255, 121, 198))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(255, 85, 85)),
            success: Style::default().fg(cp(80, 250, 123)),
            shortcut: Style::default().fg(cp(255, 184, 108)),
            overlay: Style::default().fg(cp(98, 114, 164)),
            rating: Style::default().fg(cp(241, 250, 140)),
            accent: Style::default()
                .fg(cp(139, 233, 253))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(98, 114, 164)),
            teal: Style::default().fg(cp(139, 233, 253)),
            lavender: Style::default().fg(cp(189, 147, 249)),
            sapphire: Style::default().fg(cp(139, 233, 253)),
            subtext1: Style::default().fg(cp(248, 248, 242)),
            base: cp(40, 42, 54),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(248, 248, 242)),
            flamingo: Style::default().fg(cp(255, 121, 198)),
            maroon: Style::default().fg(cp(255, 85, 85)),
            surface0: Style::default().fg(cp(68, 71, 90)),
            surface1: Style::default().fg(cp(80, 84, 106)),
            surface2: Style::default().fg(cp(98, 114, 164)),
            overlay0: Style::default().fg(cp(98, 114, 164)),
            overlay1: Style::default().fg(cp(110, 126, 176)),
            overlay2: Style::default().fg(cp(130, 146, 196)),
            mantle: Style::default().fg(cp(33, 34, 44)),
            crust: Style::default().fg(cp(25, 26, 34)),
            is_light: false,
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            border: Style::default().fg(cp(102, 92, 84)),
            border_focus: Style::default().fg(cp(250, 189, 47)),
            text: Style::default().fg(cp(235, 219, 178)),
            text_dim: Style::default().fg(cp(168, 153, 132)),
            title: Style::default()
                .fg(cp(211, 134, 155))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(142, 192, 124))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(254, 128, 25))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(251, 73, 52)),
            success: Style::default().fg(cp(184, 187, 38)),
            shortcut: Style::default().fg(cp(250, 189, 47)),
            overlay: Style::default().fg(cp(146, 131, 116)),
            rating: Style::default().fg(cp(250, 189, 47)),
            accent: Style::default()
                .fg(cp(142, 192, 124))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(102, 92, 84)),
            teal: Style::default().fg(cp(142, 192, 124)),
            lavender: Style::default().fg(cp(211, 134, 155)),
            sapphire: Style::default().fg(cp(131, 165, 152)),
            subtext1: Style::default().fg(cp(235, 219, 178)),
            base: cp(40, 40, 40),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(235, 219, 178)),
            flamingo: Style::default().fg(cp(211, 134, 155)),
            maroon: Style::default().fg(cp(251, 73, 52)),
            surface0: Style::default().fg(cp(60, 56, 54)),
            surface1: Style::default().fg(cp(80, 73, 69)),
            surface2: Style::default().fg(cp(102, 92, 84)),
            overlay0: Style::default().fg(cp(124, 111, 100)),
            overlay1: Style::default().fg(cp(146, 131, 116)),
            overlay2: Style::default().fg(cp(168, 153, 132)),
            mantle: Style::default().fg(cp(29, 32, 33)),
            crust: Style::default().fg(cp(20, 22, 23)),
            is_light: false,
        }
    }

    pub fn rose_pine() -> Self {
        Self {
            border: Style::default().fg(cp(110, 106, 134)),
            border_focus: Style::default().fg(cp(196, 167, 231)),
            text: Style::default().fg(cp(224, 222, 244)),
            text_dim: Style::default().fg(cp(144, 140, 170)),
            title: Style::default()
                .fg(cp(196, 167, 231))
                .add_modifier(Modifier::BOLD),
            highlight: Style::default()
                .fg(cp(156, 207, 216))
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(cp(235, 188, 186))
                .add_modifier(Modifier::BOLD),
            error: Style::default().fg(cp(235, 111, 146)),
            success: Style::default().fg(cp(156, 207, 216)),
            shortcut: Style::default().fg(cp(246, 193, 119)),
            overlay: Style::default().fg(cp(110, 106, 134)),
            rating: Style::default().fg(cp(246, 193, 119)),
            accent: Style::default()
                .fg(cp(49, 116, 143))
                .add_modifier(Modifier::BOLD),
            muted: Style::default().fg(cp(110, 106, 134)),
            teal: Style::default().fg(cp(49, 116, 143)),
            lavender: Style::default().fg(cp(196, 167, 231)),
            sapphire: Style::default().fg(cp(156, 207, 216)),
            subtext1: Style::default().fg(cp(224, 222, 244)),
            base: cp(25, 23, 36),
            bg: Style::default(),
            rosewater: Style::default().fg(cp(224, 222, 244)),
            flamingo: Style::default().fg(cp(235, 188, 186)),
            maroon: Style::default().fg(cp(235, 111, 146)),
            surface0: Style::default().fg(cp(31, 29, 46)),
            surface1: Style::default().fg(cp(38, 35, 58)),
            surface2: Style::default().fg(cp(110, 106, 134)),
            overlay0: Style::default().fg(cp(110, 106, 134)),
            overlay1: Style::default().fg(cp(144, 140, 170)),
            overlay2: Style::default().fg(cp(144, 140, 170)),
            mantle: Style::default().fg(cp(25, 23, 36)),
            crust: Style::default().fg(cp(21, 19, 30)),
            is_light: false,
        }
    }
}

fn cp(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn quantize_style(style: Style) -> Style {
    let mut quantized = style;
    if let Some(fg) = style.fg {
        quantized.fg = Some(to_indexed_256(fg));
    }
    if let Some(bg) = style.bg {
        quantized.bg = Some(to_indexed_256(bg));
    }
    quantized
}

fn to_indexed_256(color: Color) -> Color {
    match color {
        Color::Rgb(red, green, blue) => Color::Indexed(rgb_to_xterm256(red, green, blue)),
        other => other,
    }
}

const CUBE_LEVELS: [u16; 6] = [0, 95, 135, 175, 215, 255];

fn rgb_to_xterm256(red: u8, green: u8, blue: u8) -> u8 {
    if red == green && green == blue {
        if red < 8 {
            return 16;
        }
        if red > 248 {
            return 231;
        }
        return 232 + ((u16::from(red) - 8) / 10) as u8;
    }
    let nearest = |channel: u8| -> u16 {
        let channel = u16::from(channel);
        let mut best_index = 0_u16;
        let mut best_distance = u16::MAX;
        for (index, &level) in CUBE_LEVELS.iter().enumerate() {
            let distance = channel.abs_diff(level);
            if distance < best_distance {
                best_distance = distance;
                best_index = index as u16;
            }
        }
        best_index
    };
    (16 + 36 * nearest(red) + 6 * nearest(green) + nearest(blue)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_maps_to_gray_ramp() {
        assert_eq!(rgb_to_xterm256(0, 0, 0), 16);
        assert_eq!(rgb_to_xterm256(255, 255, 255), 231);
        assert_eq!(rgb_to_xterm256(128, 128, 128), 232 + 12);
    }

    #[test]
    fn pure_colors_map_to_cube() {
        assert_eq!(rgb_to_xterm256(255, 0, 0), 196);
        assert_eq!(rgb_to_xterm256(0, 255, 0), 46);
        assert_eq!(rgb_to_xterm256(0, 0, 255), 21);
    }

    #[test]
    fn non_rgb_colors_pass_through() {
        assert_eq!(to_indexed_256(Color::Red), Color::Red);
        assert_eq!(to_indexed_256(Color::Indexed(42)), Color::Indexed(42));
        assert_eq!(to_indexed_256(Color::Rgb(255, 0, 0)), Color::Indexed(196));
    }

    #[test]
    fn quantized_mocha_contains_no_rgb() {
        let quantized = Theme::mocha().quantized_to_256();
        for style in [
            quantized.border,
            quantized.text,
            quantized.accent,
            quantized.bg,
            quantized.lavender,
        ] {
            assert!(
                style
                    .fg
                    .map(|c| !matches!(c, Color::Rgb(..)))
                    .unwrap_or(true)
            );
            assert!(
                style
                    .bg
                    .map(|c| !matches!(c, Color::Rgb(..)))
                    .unwrap_or(true)
            );
        }
    }

    #[test]
    fn quantized_latte_contains_no_rgb() {
        let quantized = Theme::latte().quantized_to_256();
        assert!(
            quantized
                .border
                .fg
                .map(|c| !matches!(c, Color::Rgb(..)))
                .unwrap_or(true)
        );
        assert!(
            quantized
                .bg
                .fg
                .map(|c| !matches!(c, Color::Rgb(..)))
                .unwrap_or(true)
        );
    }

    #[test]
    fn monochrome_dark_uses_light_foreground() {
        let mono = Theme::monochrome(false);
        assert_eq!(mono.text.fg, Some(Color::White));
    }

    #[test]
    fn classify_maps_common_terminals() {
        use crate::tui::theme::ColorSupport;
        assert_eq!(
            classify_terminal("truecolor", "xterm-256color", ""),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("truecolor", "xterm", ""),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "xterm-kitty", ""),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "xterm-ghostty", ""),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "xterm-256color", "ghostty"),
            ColorSupport::Truecolor
        );
        assert_eq!(classify_terminal("", "foot", ""), ColorSupport::Truecolor);
        assert_eq!(
            classify_terminal("", "alacritty", ""),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "xterm-256color", ""),
            ColorSupport::Color256
        );
        assert_eq!(
            classify_terminal("", "screen.xterm-256color", ""),
            ColorSupport::Color256
        );
        assert_eq!(classify_terminal("", "vt100", ""), ColorSupport::Basic);
        assert_eq!(classify_terminal("", "dumb", ""), ColorSupport::Basic);
        assert_eq!(classify_terminal("", "xterm", ""), ColorSupport::Basic);
        assert_eq!(
            classify_terminal("", "xterm", "Apple_Terminal"),
            ColorSupport::Basic
        );
        assert_eq!(
            classify_terminal("", "", "iterm.app"),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "", "wezterm"),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "", "warpterminal"),
            ColorSupport::Truecolor
        );
        assert_eq!(classify_terminal("", "", "warp"), ColorSupport::Truecolor);
        #[cfg(target_os = "windows")]
        assert_eq!(classify_terminal("", "", ""), ColorSupport::Color256);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(classify_terminal("", "", ""), ColorSupport::Basic);
    }

    #[test]
    fn detect_from_matches_support_matrix() {
        let quantized = Theme::detect_from(ColorSupport::Color256, false);
        for style in [quantized.border, quantized.accent, quantized.bg] {
            assert!(
                style
                    .fg
                    .map(|c| !matches!(c, Color::Rgb(..)))
                    .unwrap_or(true)
            );
        }
        assert!(!Theme::detect_from(ColorSupport::Truecolor, false).is_light);
        assert!(Theme::detect_from(ColorSupport::Truecolor, true).is_light);
        assert!(!Theme::detect_from(ColorSupport::NoColor, false).is_light);
    }

    #[test]
    fn fallback_light_uses_high_contrast_ansi_palette_with_no_rgb() {
        let fallback_light = Theme::fallback(true);
        assert!(fallback_light.is_light);
        assert_eq!(fallback_light.text.fg, Some(Color::Black));
        assert_eq!(fallback_light.base, Color::White);

        for style in [
            fallback_light.border,
            fallback_light.border_focus,
            fallback_light.text,
            fallback_light.text_dim,
            fallback_light.title,
            fallback_light.highlight,
            fallback_light.header,
            fallback_light.error,
            fallback_light.success,
            fallback_light.shortcut,
            fallback_light.accent,
        ] {
            assert!(
                style
                    .fg
                    .map(|c| !matches!(c, Color::Rgb(..)))
                    .unwrap_or(true)
            );
        }
    }
    #[test]
    fn fallback_dark_uses_high_contrast_ansi_palette() {
        let fallback_dark = Theme::fallback(false);
        assert!(!fallback_dark.is_light);
        assert_eq!(fallback_dark.text.fg, Some(Color::White));
        assert_eq!(fallback_dark.base, Color::Black);
        assert_eq!(fallback_dark.border_focus.fg, Some(Color::Cyan));
        assert_eq!(fallback_dark.title.fg, Some(Color::Cyan));
        assert_eq!(fallback_dark.highlight.fg, Some(Color::Cyan));

        for style in [
            fallback_dark.border,
            fallback_dark.border_focus,
            fallback_dark.text,
            fallback_dark.text_dim,
            fallback_dark.title,
            fallback_dark.highlight,
            fallback_dark.header,
            fallback_dark.error,
            fallback_dark.success,
            fallback_dark.shortcut,
            fallback_dark.accent,
        ] {
            assert!(
                style
                    .fg
                    .map(|c| !matches!(c, Color::Rgb(..)))
                    .unwrap_or(true)
            );
        }
    }

    #[test]
    fn test_palette_swatch_spans() {
        let basic_swatches = Theme::palette_swatch_spans("Mocha", true);
        assert_eq!(basic_swatches.len(), 1);
        assert_eq!(basic_swatches[0].content, "* * *");

        if ColorSupport::current() != ColorSupport::NoColor {
            let swatches = Theme::palette_swatch_spans("Mocha", false);
            assert_eq!(swatches.len(), 3);
            assert_eq!(swatches[0].content, "■ ");
            assert_eq!(swatches[1].content, "■ ");
            assert_eq!(swatches[2].content, "■");
        }
    }
    #[test]
    fn test_theme_color() {
        let with_fg = Style::default().fg(Color::Red);
        assert_eq!(theme_color(with_fg, Color::Blue), Color::Red);
        let without_fg = Style::default();
        assert_eq!(theme_color(without_fg, Color::Blue), Color::Blue);
    }
    #[test]
    fn test_classify_terminal_konsole_and_xfce4_are_truecolor() {
        assert_eq!(
            classify_terminal("", "xterm-256color", "Konsole"),
            ColorSupport::Truecolor
        );
        assert_eq!(
            classify_terminal("", "xterm-256color", "xfce4-terminal"),
            ColorSupport::Truecolor
        );
    }
}
