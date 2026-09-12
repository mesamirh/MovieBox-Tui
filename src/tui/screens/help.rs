use crate::tui::{state::AppState, theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

fn help_row(key: &str, desc: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<17} ", key),
            theme.header.add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), theme.text),
    ])
}

fn help_section_header(title: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        format!("  {title}"),
        theme.title.add_modifier(Modifier::BOLD),
    )])
}

pub fn build_help_columns(
    state: &AppState,
    theme: &Theme,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut left = Vec::new();
    let mut right = Vec::new();

    left.push(help_section_header("Navigation", theme));
    left.push(help_row(
        "[↑] [↓] / [k] [j]",
        "Scroll & Navigate Lists",
        theme,
    ));
    left.push(help_row(
        "[←] [→] / [Tab]",
        "Switch Panes / Step Grid",
        theme,
    ));
    left.push(help_row("[Home] / [End]", "Top / Bottom of List", theme));
    left.push(help_row("[PgUp] / [PgDn]", "Page Up / Down", theme));
    left.push(help_row("[Esc] / [c]", "Back / Clear Search", theme));
    left.push(help_row(
        "[Ctrl+U] / [W]",
        "Clear Line / Delete Word",
        theme,
    ));
    left.push(Line::from(""));

    if state.is_tv_mode {
        left.push(help_section_header("TV Actions", theme));
        left.push(help_row("[Enter]", "Play Selected Channel", theme));
        left.push(help_row("[r]", "Reload M3U Playlists", theme));
        left.push(help_row("/list", "Show All TV Channels", theme));
    } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
        left.push(help_section_header("Addon Actions", theme));
        left.push(help_row("[Enter]", "Select Title / Play Stream", theme));
        left.push(help_row("[d]", "Download Video Stream", theme));
        left.push(help_row("[f]", "Favorite / Unfavorite Title", theme));
        left.push(help_row("[s]", "Subtitles Picker (Details)", theme));
        left.push(help_row("[r]", "Refresh Catalog / Streams", theme));
    } else {
        left.push(help_section_header("Streaming Actions", theme));
        left.push(help_row("[Enter]", "Play with Default Player", theme));
        left.push(help_row("[d]", "Download Episode / Season", theme));
        left.push(help_row("[f]", "Favorite / Unfavorite Title", theme));
        left.push(help_row("[s]", "Subtitles Picker (Details)", theme));
        left.push(help_row(
            "[Ctrl+P]",
            &format!("Switch Provider ({})", state.active_provider.label()),
            theme,
        ));
        left.push(help_row("[r]", "Refresh Results / Streams", theme));
    }
    right.push(help_section_header("Content Modes", theme));
    if state.streaming_enabled {
        let key = format!("[{}]", crate::tui::text::CTRL_S_STR);
        right.push(help_row(&key, "Switch to Streaming Mode", theme));
    }
    if state.tv_enabled {
        let key = format!("[{}]", crate::tui::text::CTRL_T_STR);
        right.push(help_row(&key, "Switch to Live TV Mode", theme));
    }
    right.push(Line::from(""));

    right.push(help_section_header("Commands & Shortcuts", theme));
    right.push(help_row("/settings", "Preferences, Themes & Modes", theme));
    if state.is_tv_mode {
        right.push(help_row("/list", "Show All TV Channels", theme));
    } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
        right.push(help_row("/browse", "Browse Addon Catalogs", theme));
    } else {
        right.push(help_row("/browse", "Browse Curated Categories", theme));
    }
    if !state.is_tv_mode {
        right.push(help_row(
            "/history",
            "Watch History ([Space] Resume)",
            theme,
        ));
        right.push(help_row("/favorites", "Starred Titles", theme));
    }
    right.push(help_row("/clear", "Clear Search Results", theme));
    right.push(help_row("/exit", "Exit App ([q] / Ctrl+C)", theme));
    right.push(help_row("[?]", "Toggle This Help Menu", theme));

    (left, right)
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let mode_title = if state.is_tv_mode {
        "Live TV"
    } else {
        "Streaming"
    };

    let (left_col, right_col) = build_help_columns(state, theme);

    let mut all_lines = left_col.clone();
    all_lines.push(Line::from(""));
    all_lines.extend(right_col.clone());

    let left_width = left_col.iter().map(Line::width).max().unwrap_or(40);
    let right_width = right_col.iter().map(Line::width).max().unwrap_or(40);
    let content_width = left_width.max(right_width) as u16;

    let max_lines = left_col.len().max(right_col.len());
    let capacity = area.height.saturating_sub(6).max(4) as usize;

    let two_columns = area.width >= 102 && capacity >= max_lines.saturating_sub(4);

    if !two_columns && all_lines.len() > capacity {
        let max_scroll = all_lines.len().saturating_sub(capacity);
        let scroll = state.help_scroll.min(max_scroll);
        let window: Vec<Line> = all_lines[scroll..scroll + capacity.min(all_lines.len())].to_vec();
        let position = if max_scroll > 0 {
            format!(" · {}/{scroll_max}", scroll + 1, scroll_max = max_scroll)
        } else {
            String::new()
        };
        let title = format!(" Help · {mode_title}{position} ");
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .title_style(theme.title)
            .borders(Borders::ALL)
            .border_type(crate::tui::overlay::border_type(state.basic_terminal))
            .border_style(theme.border_focus);
        let p = Paragraph::new(window)
            .block(block)
            .alignment(Alignment::Left);
        let popup_chunk = crate::tui::overlay::centered(
            area,
            (content_width + 8).min(area.width.saturating_sub(2)),
            capacity as u16 + 2,
            46,
            120,
        );
        crate::tui::overlay::clear_modal_area(frame, area, popup_chunk, theme);
        frame.render_widget(p, popup_chunk);
        return;
    }

    let desired_width = if two_columns {
        (left_width + right_width + 8) as u16
    } else {
        content_width.saturating_add(6)
    };

    let desired_height = if two_columns {
        (max_lines as u16 + 2).min(area.height.saturating_sub(2))
    } else {
        (all_lines.len() as u16 + 2).min(area.height.saturating_sub(2))
    };

    let popup_chunk = crate::tui::overlay::centered(area, desired_width, desired_height, 46, 120);

    crate::tui::overlay::clear_modal_area(frame, area, popup_chunk, theme);

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

        frame.render_widget(
            Paragraph::new(left_col).alignment(Alignment::Left),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(right_col).alignment(Alignment::Left),
            chunks[1],
        );
    } else {
        let p = Paragraph::new(all_lines)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(p, popup_chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_help_columns_streaming_mode() {
        let state = AppState::default();
        let theme = Theme::mocha();
        let (left, right) = build_help_columns(&state, &theme);

        let left_text = left
            .iter()
            .map(Line::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let right_text = right
            .iter()
            .map(Line::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(left_text.contains("Navigation"));
        assert!(left_text.contains("Streaming Actions"));
        assert!(left_text.contains("Play with Default Player"));
        assert!(left_text.contains("Download Episode / Season"));

        assert!(right_text.contains("Content Modes"));
        assert!(right_text.contains("Commands & Shortcuts"));
        assert!(right_text.contains("/settings"));
        assert!(right_text.contains("/exit"));
    }

    #[test]
    fn test_help_modal_renders_two_columns_without_panic() {
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = AppState::default();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                draw(frame, frame.area(), &state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Help · Streaming"));
        assert!(content.contains("Navigation"));
        assert!(content.contains("Streaming Actions"));
        assert!(content.contains("Commands & Shortcuts"));
    }
}
