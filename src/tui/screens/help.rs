use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let global = vec![
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
        Line::from(vec![
            Span::styled("    [Ctrl+T]   ", theme.header),
            Span::styled("Switch Streaming / TV Mode", theme.text),
        ]),
    ];

    let navigation = vec![
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
            Span::styled("Page Through Results", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Enter]    ", theme.header),
            Span::styled("Select / Submit", theme.text),
        ]),
    ];

    let mut help_text = global;
    if state.is_tv_mode {
        help_text.extend(navigation);
        help_text.extend(vec![
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  TV Controls",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [Enter]    ", theme.header),
                Span::styled("Play Channel", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [r]        ", theme.header),
                Span::styled("Reload Playlists", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /list      ", theme.header),
                Span::styled("Show Available Channels", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /config    ", theme.header),
                Span::styled("Add / Remove Playlists", theme.text),
            ]),
        ]);
    } else {
        help_text.extend(vec![Line::from(vec![
            Span::styled("    [Ctrl+P]   ", theme.header),
            Span::styled(
                format!(
                    "Switch Provider (active: {})",
                    state.active_provider.label()
                ),
                theme.text,
            ),
        ])]);
        help_text.extend(navigation);
        help_text.extend(vec![
            Line::from(vec![
                Span::styled("    [Tab]      ", theme.header),
                Span::styled("Next Details Pane", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [Shift+Tab]", theme.header),
                Span::styled("Previous Details Pane", theme.text),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  Playback & Download",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [Enter]    ", theme.header),
                Span::styled("Play with Default Player", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [o]        ", theme.header),
                Span::styled("Open Player Picker", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [d]        ", theme.header),
                Span::styled("Download Video", theme.text),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  Discover & Search",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [Alt+b]    ", theme.header),
                Span::styled("Open Browse Menu", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [s]        ", theme.header),
                Span::styled("Toggle Sort Order (in Browse)", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /browse    ", theme.header),
                Span::styled("Trending / Top Rated / Popular", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /home      ", theme.header),
                Span::styled("Trending & Featured", theme.text),
            ]),
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
            Line::from(vec![
                Span::styled("    /github    ", theme.header),
                Span::styled("Open GitHub Repo", theme.text),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  System",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [r]        ", theme.header),
                Span::styled("Refresh Streams/Search", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /update    ", theme.header),
                Span::styled("Check for Updates", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /toggle-update ", theme.header),
                Span::styled("Toggle Auto Updates", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /clear-cache   ", theme.header),
                Span::styled("Clear App Cache", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /enable-bdix   ", theme.header),
                Span::styled("Enable BDIX FTP", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /disable-bdix  ", theme.header),
                Span::styled("Disable BDIX FTP", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /theme         ", theme.header),
                Span::styled("Change UI Theme", theme.text),
            ]),
        ]);
    }

    let content_width = help_text.iter().map(Line::width).max().unwrap_or(42) as u16;
    let total_lines = help_text.len() as u16;
    let available_height = area.height.saturating_sub(4);

    let two_columns = total_lines > available_height;

    let desired_width = if two_columns {
        content_width.saturating_mul(2).saturating_add(8)
    } else {
        content_width.saturating_add(4)
    };

    let desired_height = if two_columns {
        total_lines.div_ceil(2) + 2
    } else {
        total_lines + 2
    };

    let popup_chunk = crate::tui::overlay::centered(area, desired_width, desired_height, 46, 120);

    crate::tui::overlay::clear_modal_area(frame, area, popup_chunk, theme);

    let block = Block::default()
        .title(" Keybindings Help ")
        .title_alignment(Alignment::Center)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(if state.basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.border_focus);

    if two_columns {
        let inner = block.inner(popup_chunk);
        frame.render_widget(block, popup_chunk);

        let chunks = ratatui::layout::Layout::horizontal([
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(inner);

        let mid = help_text.len().div_ceil(2);
        let left = help_text[..mid].to_vec();
        let right = help_text[mid..].to_vec();

        frame.render_widget(Paragraph::new(left).alignment(Alignment::Left), chunks[0]);
        frame.render_widget(Paragraph::new(right).alignment(Alignment::Left), chunks[1]);
    } else {
        let p = Paragraph::new(help_text)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(p, popup_chunk);
    }
}
