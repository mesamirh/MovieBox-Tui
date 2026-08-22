use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let mode_title = if state.is_tv_mode {
        "TV Mode"
    } else if state.is_addon_mode {
        "Addon Mode"
    } else {
        "Streaming Mode"
    };

    let mut help_text: Vec<Line> = vec![Line::from(vec![Span::styled(
        "  Mode Switching & Navigation",
        theme.header.add_modifier(ratatui::style::Modifier::BOLD),
    )])];

    if state.streaming_enabled {
        let key = format!("    [{}]", crate::tui::text::ctrl_key("S"));
        let padded = format!("{:<19}", key);
        help_text.push(Line::from(vec![
            Span::styled(padded, theme.header),
            Span::styled("Switch to Streaming Mode", theme.text),
        ]));
    }
    if state.tv_enabled {
        let key = format!("    [{}]", crate::tui::text::ctrl_key("T"));
        let padded = format!("{:<19}", key);
        help_text.push(Line::from(vec![
            Span::styled(padded, theme.header),
            Span::styled("Switch to TV Mode", theme.text),
        ]));
    }
    if state.addons_enabled {
        let key = format!("    [{}]", crate::tui::text::ctrl_key("A"));
        let padded = format!("{:<19}", key);
        help_text.push(Line::from(vec![
            Span::styled(padded, theme.header),
            Span::styled("Switch to Addon Mode", theme.text),
        ]));
    }

    help_text.extend(vec![
        Line::from(vec![
            Span::styled("    [↑] / [↓]      ", theme.header),
            Span::styled("Scroll Lists / Navigate", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [←] / [→]      ", theme.header),
            Span::styled("Jump Page of Results", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [Esc]          ", theme.header),
            Span::styled("Go Back / Clear Input", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [?]            ", theme.header),
            Span::styled("Toggle Help Menu", theme.text),
        ]),
        Line::from(vec![
            Span::styled("    [q]            ", theme.header),
            Span::styled("Quit Application", theme.text),
        ]),
        Line::from(vec![]),
    ]);

    if state.is_tv_mode {
        help_text.push(Line::from(vec![Span::styled(
            "  TV Mode Controls",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]));
        help_text.push(Line::from(vec![
            Span::styled("    [Enter]        ", theme.header),
            Span::styled("Play Selected Channel", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [o]            ", theme.header),
            Span::styled("Open Alternative Player Picker", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [r]            ", theme.header),
            Span::styled("Reload TV Playlists", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /config        ", theme.header),
            Span::styled("Manage M3U Playlists", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /list          ", theme.header),
            Span::styled("Show All TV Channels", theme.text),
        ]));
    } else if state.is_addon_mode {
        help_text.push(Line::from(vec![Span::styled(
            "  Addon Mode Controls",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]));
        help_text.push(Line::from(vec![
            Span::styled("    [Ctrl+P]       ", theme.header),
            Span::styled("Open Addons Manager", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [Enter]        ", theme.header),
            Span::styled("Select Movie / Play Stream", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [o]            ", theme.header),
            Span::styled("Open Alternative Player Picker", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [d]            ", theme.header),
            Span::styled("Download Video Stream", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [*]            ", theme.header),
            Span::styled("Favorite / Unfavorite (Home)", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [f]            ", theme.header),
            Span::styled("Favorite / Unfavorite (Details)", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /favorites     ", theme.header),
            Span::styled("Favorited Titles", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /browse        ", theme.header),
            Span::styled("Browse Addon Catalogs", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /config        ", theme.header),
            Span::styled("Manage Addons", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [r]            ", theme.header),
            Span::styled("Refresh Catalog / Streams", theme.text),
        ]));
    } else {
        help_text.push(Line::from(vec![Span::styled(
            "  Streaming Controls",
            theme.header.add_modifier(ratatui::style::Modifier::BOLD),
        )]));
        help_text.push(Line::from(vec![
            Span::styled("    [Ctrl+P]       ", theme.header),
            Span::styled(
                format!("Switch Provider ({})", state.active_provider.label()),
                theme.text,
            ),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [Tab] / [S-Tab]", theme.header),
            Span::styled("Next / Previous Details Pane", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [Enter]        ", theme.header),
            Span::styled("Play with Default Player", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [o]            ", theme.header),
            Span::styled("Open Alternative Player Picker", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [d]            ", theme.header),
            Span::styled("Download Episode / Season Batch", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [*]            ", theme.header),
            Span::styled("Favorite / Unfavorite (Home)", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [f]            ", theme.header),
            Span::styled("Favorite / Unfavorite (Details)", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /browse        ", theme.header),
            Span::styled("Browse Movies & Series Categories", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /history       ", theme.header),
            Span::styled("Watch History", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /favorites     ", theme.header),
            Span::styled("Favorited Titles", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    [r]            ", theme.header),
            Span::styled("Refresh Results / Streams", theme.text),
        ]));
    }

    help_text.push(Line::from(vec![]));

    help_text.push(Line::from(vec![Span::styled(
        "  System & Settings",
        theme.header.add_modifier(ratatui::style::Modifier::BOLD),
    )]));
    help_text.push(Line::from(vec![
        Span::styled("    /theme         ", theme.header),
        Span::styled("Change Color Theme", theme.text),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("    /download-dir  ", theme.header),
        Span::styled("Configure Download Directory", theme.text),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("    /clear-cache   ", theme.header),
        Span::styled("Clear Application Cache", theme.text),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("    /update        ", theme.header),
        Span::styled("Check for GitHub Updates", theme.text),
    ]));
    help_text.push(Line::from(vec![
        Span::styled("    /toggle-update ", theme.header),
        Span::styled("Toggle Auto Updates on Startup", theme.text),
    ]));
    if !state.is_tv_mode && !state.is_addon_mode {
        help_text.push(Line::from(vec![
            Span::styled("    /enable-bdix   ", theme.header),
            Span::styled("Enable BDIX FTP Providers", theme.text),
        ]));
        help_text.push(Line::from(vec![
            Span::styled("    /disable-bdix  ", theme.header),
            Span::styled("Disable BDIX FTP Providers", theme.text),
        ]));
    }
    help_text.push(Line::from(vec![
        Span::styled("    /github        ", theme.header),
        Span::styled("Open GitHub Repository", theme.text),
    ]));

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

    crate::tui::clear_area(frame, area, theme);

    let title = format!(" Help · {mode_title} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
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
