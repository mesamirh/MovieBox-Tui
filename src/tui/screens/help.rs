use crate::tui::{overlay, state::AppState, theme::Theme, widgets::ModalFrame};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
};

fn help_row(key: &str, desc: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<15} ", key),
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
    left.push(help_row("↑ / ↓", "Navigate rows & lists", theme));
    left.push(help_row("← / →", "Step search card columns", theme));
    left.push(help_row("Tab / S-Tab", "Switch tabs & detail panes", theme));
    left.push(help_row("Home / End", "Jump to start / end of list", theme));
    left.push(help_row("PgUp / PgDn", "Scroll one page up / down", theme));
    left.push(help_row("Esc / c", "Clear search / back / dismiss", theme));
    left.push(help_row("Ctrl+U", "Clear entire search input", theme));
    left.push(help_row("Ctrl+W", "Delete previous word", theme));
    left.push(Line::from(""));

    if state.is_tv_mode {
        left.push(help_section_header("TV Actions", theme));
        left.push(help_row("Enter", "Play selected channel", theme));
        left.push(help_row("d / Delete", "Delete selected playlist", theme));
        left.push(help_row("r", "Reload M3U playlists", theme));
        left.push(help_row("/list", "Browse all TV channels", theme));
    } else {
        left.push(help_section_header("Streaming Actions", theme));
        left.push(help_row("Enter", "Play selected release", theme));
        left.push(help_row("Space / P", "Direct resume playback", theme));
        left.push(help_row("d / Delete", "Download or remove history", theme));
        left.push(help_row("f", "Favorite / unfavorite title", theme));
        left.push(help_row(
            "Ctrl+P",
            &format!("Cycle provider ({})", state.active_provider.label()),
            theme,
        ));
        left.push(help_row("r", "Refresh active results", theme));
    }

    right.push(help_section_header("Content Modes", theme));
    if state.streaming_enabled {
        let key = crate::tui::text::CTRL_S_STR;
        right.push(help_row(key, "Switch to Streaming Mode", theme));
    }
    if state.tv_enabled {
        let key = crate::tui::text::CTRL_T_STR;
        right.push(help_row(key, "Switch to Live TV Mode", theme));
    }
    right.push(Line::from(""));

    right.push(help_section_header("Commands & Shortcuts", theme));
    right.push(help_row("/settings", "Preferences & maintenance", theme));
    if state.is_tv_mode {
        right.push(help_row("/list", "Browse all TV channels", theme));
    } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
        right.push(help_row("/browse", "Browse addon catalogs", theme));
    } else {
        right.push(help_row("/browse", "Browse curated genres", theme));
    }
    if !state.is_tv_mode {
        right.push(help_row("/history", "Watch history & resume", theme));
        right.push(help_row("/favorites", "Starred media library", theme));
    }
    right.push(help_row("/clear", "Clear search query & results", theme));
    right.push(help_row("/exit", "Exit app (q / Ctrl+C)", theme));
    right.push(help_row("?", "Toggle this help menu", theme));

    (left, right)
}

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let mode_title = if state.is_tv_mode {
        "Live TV"
    } else {
        "Streaming"
    };

    let (left_col, right_col) = build_help_columns(state, theme);

    let left_width = left_col.iter().map(Line::width).max().unwrap_or(36);
    let right_width = right_col.iter().map(Line::width).max().unwrap_or(36);
    let content_width = left_width.max(right_width) as u16;

    let max_lines = left_col.len().max(right_col.len());
    let capacity = area.height.saturating_sub(6).max(4) as usize;

    let two_columns = area.width >= 78 && capacity >= max_lines.saturating_sub(2);

    let mut all_lines = left_col.clone();
    all_lines.push(Line::from(""));
    all_lines.extend(right_col.clone());

    if !two_columns && all_lines.len() > capacity {
        let max_scroll = all_lines.len().saturating_sub(capacity);
        let scroll = state.help_scroll.min(max_scroll);
        let window: Vec<Line> = all_lines[scroll..scroll + capacity.min(all_lines.len())].to_vec();
        let position = if max_scroll > 0 {
            format!(" · {}/{}", scroll + 1, max_scroll)
        } else {
            String::new()
        };
        let title = format!("Help · {mode_title}{position}");
        let desired_w = (content_width + 8).min(area.width.saturating_sub(2));
        let desired_h = capacity as u16 + 2;
        let popup_chunk = overlay::help_modal_layout(area, desired_w, desired_h);
        let inner =
            ModalFrame::new(&title, theme, state.basic_terminal).render(frame, popup_chunk, area);
        let p = Paragraph::new(window).alignment(Alignment::Left);
        frame.render_widget(p, inner);
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

    let popup_chunk = overlay::help_modal_layout(area, desired_width, desired_height);
    let title = format!("Help · {mode_title}");
    let inner =
        ModalFrame::new(&title, theme, state.basic_terminal).render(frame, popup_chunk, area);

    if two_columns {
        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
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
        let p = Paragraph::new(all_lines).alignment(Alignment::Left);
        frame.render_widget(p, inner);
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
        assert!(left_text.contains("Play selected release"));
        assert!(left_text.contains("Download or remove history"));

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
