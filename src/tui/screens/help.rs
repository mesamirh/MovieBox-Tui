use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let mut help_text = vec![
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
            Span::styled("    [Ctrl+P]   ", theme.header),
            Span::styled(
                format!(
                    "Search sources (now: {})",
                    state.search_scope.short_label()
                ),
                theme.text,
            ),
        ]),
        Line::from(vec![
            Span::styled("    [Ctrl+T]   ", theme.header),
            Span::styled("Switch Streaming / TV Mode", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Ctrl+L]   ", theme.header),
            Span::styled("Open Download Library", theme.text),
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
            Span::styled("Page Through Search Results", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Tab]      ", theme.header),
            Span::styled("Next Details Pane", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Shift+Tab]", theme.header),
            Span::styled("Previous Details Pane", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Enter]    ", theme.header),
            Span::styled("Select / Submit", theme.text),
        ]),
        Line::from(vec![]),
    ];
    if state.is_tv_mode {
        help_text.remove(4);
    }

    if state.is_tv_mode {
        help_text.extend(vec![
            Line::from(vec![Span::styled(
                "  TV Controls",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [Enter]    ", theme.header),
                Span::styled("Play Channel", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /list      ", theme.header),
                Span::styled("Show Available Channels", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /config    ", theme.header),
                Span::styled("Open TV Config Popup", theme.text),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  System",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [r]        ", theme.header),
                Span::styled("Refresh Channels/Search", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /clear-cache   ", theme.header),
                Span::styled("Clear App Cache", theme.text),
            ]),
        ]);
    } else {
        help_text.extend(vec![
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
            Line::from(vec![
                Span::styled("    [Ctrl+L]   ", theme.header),
                Span::styled("Browse Downloaded Library", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    Search     ", theme.header),
                Span::styled(
                    "Uses selected sources (All or one); best matches first",
                    theme.text,
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  Library",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    [Enter]    ", theme.header),
                Span::styled("Play Selected Download", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [o]        ", theme.header),
                Span::styled("Choose Player for Download", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [r]        ", theme.header),
                Span::styled("Rescan Download Folder", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    [Esc]      ", theme.header),
                Span::styled("Leave Library", theme.text),
            ]),
            Line::from(vec![]),
            Line::from(vec![Span::styled(
                "  Discover & Search",
                theme.header.add_modifier(ratatui::style::Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("    /home      ", theme.header),
                Span::styled("Trending & Featured", theme.text),
            ]),
            Line::from(vec![
                Span::styled("    /discover  ", theme.header),
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
        ]);
    }

    let content_width = help_text.iter().map(Line::width).max().unwrap_or(42);
    let popup_chunk = crate::tui::overlay::centered(
        area,
        content_width.saturating_add(4) as u16,
        help_text.len() as u16 + 2,
        46,
        64,
    );

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

    let p = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(p, popup_chunk);
}
