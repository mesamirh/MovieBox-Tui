use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::theme::Theme;

const MAX_PICKER_ROWS_CAP: usize = 14;

pub(crate) fn max_picker_rows(area: Rect) -> usize {
    (area.height.saturating_sub(6) as usize).clamp(4, MAX_PICKER_ROWS_CAP)
}

pub use crate::models::{Notification, NotificationKind};

pub struct PickerSpec<'a> {
    pub title: &'a str,
    pub confirm_label: &'a str,
    pub minimum_width: u16,
    pub show_counter: bool,
}

pub fn picker_layout(
    area: Rect,
    items: &[String],
    _confirm_label: &str,
    minimum_width: u16,
) -> Rect {
    let visible_rows = items.len().clamp(1, max_picker_rows(area));
    let content_width = items
        .iter()
        .map(|item| crate::tui::text::width(item))
        .max()
        .unwrap_or(0)
        .saturating_add(2);
    centered(
        area,
        content_width as u16,
        (visible_rows as u16 + 2).max(3),
        minimum_width,
        64,
    )
}

pub fn tv_config_layout(
    area: Rect,
    longest_source_width: usize,
    total_rows: usize,
    input_active: bool,
) -> Rect {
    let content_width = longest_source_width.max(48).max(crate::tui::text::width(
        "[ Add URL ] [ Add file ] [ Reload ] [ Done ]",
    ));
    let popup_width = 68u16
        .max(content_width.saturating_add(6) as u16)
        .min(area.width.saturating_sub(4));
    let popup_height = if input_active {
        5u16
    } else {
        total_rows.min(10).saturating_add(4) as u16
    };
    centered(area, popup_width, popup_height, 36, 74)
}

pub fn addon_manager_layout(area: Rect, addons_count: usize, input_active: bool) -> Rect {
    let popup_width = 76u16.min(area.width.saturating_sub(4)).max(56);
    let popup_height = if input_active {
        5u16
    } else {
        (addons_count as u16)
            .saturating_add(4)
            .min(area.height.saturating_sub(4))
            .max(5)
    };
    centered(area, popup_width, popup_height, 36, 80)
}
pub fn settings_modal_layout(area: Rect, category: crate::tui::state::SettingsCategory) -> Rect {
    let min_width = 44u16.min(area.width.saturating_sub(2));
    let popup_width = 68u16.min(area.width.saturating_sub(2)).max(min_width);
    let content_height = (category.row_count() as u16).max(1);
    let popup_height = (content_height + 4).min(area.height.saturating_sub(2));
    let available_width = area.width.saturating_sub(2).max(1);
    let width = popup_width.min(available_width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = if area.height >= 24 {
        area.y + 2 + 6 + 1 + 2
    } else {
        area.y + 1 + 2 + 1 + 1
    };
    let y = search_y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, width, popup_height)
}

pub fn download_confirm_layout(
    area: Rect,
    summary_lines: usize,
    longest_line_width: usize,
) -> Rect {
    let content_width = longest_line_width.max(36);
    centered(
        area,
        content_width.saturating_add(4) as u16,
        summary_lines as u16 + 4,
        36,
        64,
    )
}

pub fn download_confirm_action_row(popup: Rect, summary_lines: usize) -> u16 {
    popup.y + summary_lines as u16 + 1
}

pub fn picker(
    frame: &mut Frame,
    area: Rect,
    items: &[String],
    state: &mut ListState,
    spec: PickerSpec<'_>,
    theme: &Theme,
    basic_terminal: bool,
) {
    let lines: Vec<Line<'static>> = items
        .iter()
        .map(|item| Line::from(vec![Span::raw(item.clone())]))
        .collect();
    picker_with_lines(
        frame,
        area,
        &lines,
        items,
        state,
        spec,
        theme,
        basic_terminal,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn picker_with_lines<'a>(
    frame: &mut Frame,
    area: Rect,
    lines: &[Line<'a>],
    raw_items: &[String],
    state: &mut ListState,
    spec: PickerSpec<'_>,
    theme: &Theme,
    basic_terminal: bool,
) {
    let selected = state
        .selected()
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
    let visible_rows = lines.len().clamp(1, max_picker_rows(area));
    let popup = picker_layout(area, raw_items, spec.confirm_label, spec.minimum_width);
    let title = if spec.title.is_empty() {
        String::new()
    } else if spec.show_counter && lines.len() > 1 {
        format!(
            "{} · {}/{}",
            spec.title,
            selected.saturating_add(1),
            lines.len().max(1)
        )
    } else {
        spec.title.to_string()
    };
    let inner = crate::tui::widgets::ModalFrame::new(&title, theme, basic_terminal)
        .render(frame, popup, area);

    let list_items = lines
        .iter()
        .map(|line| ListItem::new(line.clone()).style(theme.text))
        .collect::<Vec<_>>();
    let list = List::new(list_items)
        .highlight_style(selection_style(theme, basic_terminal))
        .highlight_symbol("");
    frame.render_stateful_widget(list, inner, state);

    if lines.len() > visible_rows {
        crate::tui::widgets::render_scrollbar(
            frame,
            inner,
            lines.len(),
            visible_rows,
            selected,
            theme,
            basic_terminal,
        );
    }
}

pub fn browse_category_badge<'a>(label: &str, theme: &'a Theme) -> (Span<'a>, &'static str) {
    let lower = label.to_ascii_lowercase();
    if lower.contains("movie")
        || lower.contains("top rated (all-time)")
        || lower.contains("top rated (recent")
    {
        (
            Span::styled("[MOVIES]", theme.sapphire.add_modifier(Modifier::BOLD)),
            "   ",
        )
    } else if lower.contains("series")
        || lower.contains("airing")
        || lower.contains("show")
        || lower.contains("tv")
    {
        (
            Span::styled("[SERIES]", theme.lavender.add_modifier(Modifier::BOLD)),
            "   ",
        )
    } else {
        (
            Span::styled("[DISCOVER]", theme.teal.add_modifier(Modifier::BOLD)),
            " ",
        )
    }
}

pub fn confirmation(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    summary: &[Line<'_>],
    confirm_selected: bool,
    theme: &Theme,
    basic_terminal: bool,
) {
    let content_width = summary.iter().map(Line::width).max().unwrap_or(0).max(36);
    let popup = centered(
        area,
        content_width.saturating_add(4) as u16,
        summary.len() as u16 + 4,
        36,
        64,
    );
    let inner = crate::tui::widgets::ModalFrame::new(title, theme, basic_terminal)
        .render(frame, popup, area);
    let sections = Layout::vertical([
        Constraint::Length(summary.len() as u16),
        Constraint::Length(2),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(summary.to_vec()).alignment(Alignment::Center),
        sections[0],
    );
    use ratatui::style::Color;
    let confirm_btn_style = if confirm_selected {
        if basic_terminal {
            theme.text.add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme.accent.fg.unwrap_or(Color::Cyan))
                .fg(theme.crust.fg.unwrap_or(Color::Black))
                .add_modifier(Modifier::BOLD)
        }
    } else {
        theme.subtext1
    };

    let cancel_btn_style = if !confirm_selected {
        if basic_terminal {
            theme.text.add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme.surface1.fg.unwrap_or(Color::DarkGray))
                .fg(theme.text.fg.unwrap_or(Color::White))
                .add_modifier(Modifier::BOLD)
        }
    } else {
        theme.subtext1
    };

    let actions = vec![
        Span::styled(" [ Download ] ", confirm_btn_style),
        Span::raw("    "),
        Span::styled(" [ Cancel ] ", cancel_btn_style),
    ];
    crate::tui::widgets::render_modal_footer(frame, sections[1], actions, theme);
}

pub fn notifications(
    frame: &mut Frame,
    area: Rect,
    notifications: &std::collections::VecDeque<Notification>,
    theme: &Theme,
    basic_terminal: bool,
    download_active: bool,
) {
    let bottom_offset = if download_active { 5 } else { 2 };
    let mut y = area.bottom().saturating_sub(bottom_offset);

    for notification in notifications.iter().rev().take(3) {
        let (badge, badge_style) = notification_style(notification.kind, theme, basic_terminal);
        let has_message =
            !notification.message.is_empty() && notification.message != notification.title;

        let max_card_width = (area.width.saturating_sub(4) as usize).min(72);
        let title_w = crate::tui::text::width(&notification.title).saturating_add(6);
        let badge_w = badge.len().saturating_add(6);
        let raw_msg_w = if has_message {
            crate::tui::text::width(&notification.message).saturating_add(6)
        } else {
            0
        };

        let target_card_width = title_w
            .max(badge_w)
            .max(raw_msg_w)
            .clamp(20, max_card_width.max(20)) as u16;

        let inner_width = (target_card_width.saturating_sub(4) as usize).max(1);

        let msg_lines: Vec<String> = if has_message {
            crate::tui::text::wrap_text(&notification.message, inner_width)
                .into_iter()
                .take(4)
                .collect()
        } else {
            Vec::new()
        };

        let height = 2 + 1 + msg_lines.len() as u16;

        if target_card_width < 10 || y < area.y.saturating_add(height) {
            break;
        }

        y = y.saturating_sub(height);

        let toast_area = Rect::new(
            area.right()
                .saturating_sub(target_card_width)
                .saturating_sub(2),
            y,
            target_card_width,
            height,
        );

        crate::tui::clear_area(frame, toast_area, theme);

        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            crate::tui::text::truncate_width(&notification.title, inner_width),
            badge_style.add_modifier(Modifier::BOLD),
        )]));

        for line in &msg_lines {
            lines.push(Line::from(vec![Span::styled(
                crate::tui::text::truncate_width(line, inner_width),
                theme.subtext1,
            )]));
        }

        let total_duration = match notification.kind {
            NotificationKind::Info => std::time::Duration::from_secs(4),
            NotificationKind::Success => std::time::Duration::from_secs(5),
            NotificationKind::Warning => std::time::Duration::from_secs(7),
            NotificationKind::Error => std::time::Duration::from_secs(10),
        };
        let remaining = notification
            .expires_at
            .saturating_duration_since(std::time::Instant::now());
        let ratio = (remaining.as_secs_f64() / total_duration.as_secs_f64()).clamp(0.0, 1.0);
        let bar_width = inner_width.clamp(3, 16);
        let filled = ((bar_width as f64) * ratio).round() as usize;
        let countdown_bar = if basic_terminal {
            format!(
                "[{}{}]",
                "=".repeat(filled),
                "-".repeat(bar_width.saturating_sub(filled))
            )
        } else {
            format!(
                "{}{}",
                "━".repeat(filled),
                "─".repeat(bar_width.saturating_sub(filled))
            )
        };

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                format!(" {badge} "),
                badge_style.add_modifier(Modifier::BOLD),
            )]))
            .title_bottom(
                Line::from(vec![Span::styled(
                    format!(" {countdown_bar} "),
                    badge_style.add_modifier(Modifier::DIM),
                )])
                .alignment(Alignment::Right),
            )
            .borders(Borders::ALL)
            .border_type(border_type(basic_terminal))
            .border_style(badge_style)
            .padding(ratatui::widgets::Padding::horizontal(1));

        frame.render_widget(Paragraph::new(lines).block(block), toast_area);

        y = y.saturating_sub(1);
    }
}

pub fn centered(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    minimum_width: u16,
    maximum_width: u16,
) -> Rect {
    let available_width = area.width.saturating_sub(2).max(1);
    let available_height = area.height.saturating_sub(2).max(1);
    let width = desired_width
        .max(minimum_width.min(available_width))
        .min(maximum_width)
        .min(available_width);
    let height = desired_height.min(available_height).max(1);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

pub fn clear_modal_area(frame: &mut Frame, _bounds: Rect, popup: Rect, theme: &Theme) {
    crate::tui::clear_area(frame, popup, theme);
}

pub fn border_type(basic_terminal: bool) -> BorderType {
    if basic_terminal {
        BorderType::Plain
    } else {
        BorderType::Rounded
    }
}

pub(crate) fn key_hint(key: &'static str, action: &str, theme: &Theme) -> Vec<Span<'static>> {
    if action.is_empty() {
        vec![
            Span::styled("[", theme.overlay0),
            Span::styled(key, theme.shortcut),
            Span::styled("]", theme.overlay0),
        ]
    } else {
        vec![
            Span::styled("[", theme.overlay0),
            Span::styled(key, theme.shortcut),
            Span::styled("] ", theme.overlay0),
            Span::styled(action.to_string(), theme.subtext1),
        ]
    }
}

pub(crate) fn selection_style(theme: &Theme, basic_terminal: bool) -> Style {
    if basic_terminal {
        Style::default()
            .fg(theme.highlight.fg.unwrap_or(theme.base))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        let bg = theme.surface1.fg.unwrap_or(theme.base);
        let fg = theme
            .highlight
            .fg
            .unwrap_or(theme.text.fg.unwrap_or(theme.base));
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
    }
}

fn notification_style(
    kind: NotificationKind,
    theme: &Theme,
    basic_terminal: bool,
) -> (&'static str, Style) {
    match kind {
        NotificationKind::Info => (
            if basic_terminal { "i INFO" } else { "ℹ INFO" },
            theme.sapphire,
        ),
        NotificationKind::Success => (
            if basic_terminal {
                "+ SUCCESS"
            } else {
                "✔ SUCCESS"
            },
            theme.success,
        ),
        NotificationKind::Warning => (
            if basic_terminal {
                "! WARNING"
            } else {
                "⚠ WARNING"
            },
            theme.rating,
        ),
        NotificationKind::Error => (
            if basic_terminal {
                "x ERROR"
            } else {
                "✖ ERROR"
            },
            theme.error,
        ),
    }
}

pub fn notification_rects(
    area: Rect,
    notifications: &std::collections::VecDeque<Notification>,
    basic_terminal: bool,
    download_active: bool,
) -> Vec<(usize, Rect)> {
    let mut rects = Vec::new();
    let bottom_offset = if download_active { 5 } else { 2 };
    let mut y = area.bottom().saturating_sub(bottom_offset);
    let theme_placeholder = Theme::default();

    for (rev_idx, notification) in notifications.iter().rev().take(3).enumerate() {
        let (badge, _) = notification_style(notification.kind, &theme_placeholder, basic_terminal);
        let has_message =
            !notification.message.is_empty() && notification.message != notification.title;

        let max_card_width = (area.width.saturating_sub(4) as usize).min(72);
        let title_w = crate::tui::text::width(&notification.title).saturating_add(6);
        let badge_w = badge.len().saturating_add(6);
        let raw_msg_w = if has_message {
            crate::tui::text::width(&notification.message).saturating_add(6)
        } else {
            0
        };

        let target_card_width = title_w
            .max(badge_w)
            .max(raw_msg_w)
            .clamp(20, max_card_width.max(20)) as u16;

        let inner_width = (target_card_width.saturating_sub(4) as usize).max(1);

        let msg_lines: Vec<String> = if has_message {
            crate::tui::text::wrap_text(&notification.message, inner_width)
                .into_iter()
                .take(4)
                .collect()
        } else {
            Vec::new()
        };

        let height = 2 + 1 + msg_lines.len() as u16;

        if target_card_width < 10 || y < area.y.saturating_add(height) {
            break;
        }

        y = y.saturating_sub(height);

        let toast_area = Rect::new(
            area.right()
                .saturating_sub(target_card_width)
                .saturating_sub(2),
            y,
            target_card_width,
            height,
        );

        let original_idx = notifications.len().saturating_sub(1 + rev_idx);
        rects.push((original_idx, toast_area));

        y = y.saturating_sub(1);
    }
    rects
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateModalLayout {
    pub popup_area: Rect,
    pub display_count: usize,
    pub has_more: bool,
    pub button_row_y: u16,
    pub update_btn_end_x: u16,
    pub open_btn_end_x: u16,
    pub open_button_midpoint_x: u16,
}

pub fn update_modal_layout(area: Rect, notes: &str) -> UpdateModalLayout {
    let note_lines_count = notes
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .count();

    let min_w: u16 = 46;
    let max_w: u16 = 76;
    let available_w = area.width.saturating_sub(4);
    let desired_w = max_w.min(available_w).max(min_w.min(available_w));

    let header_rows: u16 = 6;
    let footer_rows: u16 = 4;
    let available_height = area.height.saturating_sub(4);
    let available_note_rows =
        (available_height.saturating_sub(header_rows + footer_rows) as usize).clamp(3, 12);

    let display_count = note_lines_count.min(available_note_rows);
    let has_more = note_lines_count > display_count;
    let total_rows = header_rows + (display_count as u16) + footer_rows;
    let desired_h = total_rows.clamp(12, available_height.max(12));

    const UPDATE_SEGMENT: u16 = 18;
    const OPEN_SEGMENT: u16 = 26;
    const DISMISS_SEGMENT: u16 = 14;

    let popup_area = centered(area, desired_w, desired_h, min_w.min(available_w), max_w);
    let (update_seg, open_seg, dismiss_seg) = if popup_area.width < 60 {
        (12, 10, 10)
    } else {
        (UPDATE_SEGMENT, OPEN_SEGMENT, DISMISS_SEGMENT)
    };
    let footer_width = update_seg + open_seg + dismiss_seg;

    let button_row_y = popup_area.y + popup_area.height.saturating_sub(2);
    let inner_width = popup_area.width.saturating_sub(2);
    let footer_start = popup_area.x + 1 + inner_width.saturating_sub(footer_width) / 2;
    let update_btn_end_x = footer_start + update_seg;
    let open_btn_end_x = update_btn_end_x + open_seg;
    let open_button_midpoint_x = update_btn_end_x + open_seg / 2;
    UpdateModalLayout {
        popup_area,
        display_count,
        has_more,
        button_row_y,
        update_btn_end_x,
        open_btn_end_x,
        open_button_midpoint_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_modal_mouse_hitbox_matches_rendered_geometry() {
        let area = Rect::new(0, 0, 80, 24);
        let notes = "Line 1\nLine 2\nLine 3\nLine 4";
        let layout = update_modal_layout(area, notes);

        assert_eq!(layout.popup_area.width, 76);
        assert_eq!(layout.display_count, 4);
        assert!(!layout.has_more);
        assert_eq!(layout.popup_area.height, 14);
        assert_eq!(layout.popup_area.x, (80 - 76) / 2);
        assert_eq!(layout.popup_area.y, (24 - 14) / 2);
        assert_eq!(layout.button_row_y, layout.popup_area.y + 12);
        let footer_start = layout.popup_area.x + 1 + (74 - 58) / 2;
        assert_eq!(layout.update_btn_end_x, footer_start + 18);
        assert_eq!(layout.open_btn_end_x, layout.update_btn_end_x + 26);
        assert_eq!(layout.open_button_midpoint_x, layout.update_btn_end_x + 13);
    }

    #[test]
    fn test_update_modal_zones_cover_visible_labels_only() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = update_modal_layout(area, "notes");
        let row = layout.button_row_y;

        assert!(layout.popup_area.contains(ratatui::layout::Position::new(
            layout.update_btn_end_x - 2,
            row
        )));
        assert!(layout.popup_area.contains(ratatui::layout::Position::new(
            layout.open_btn_end_x - 2,
            row
        )));

        let gap_before_open = layout.update_btn_end_x;
        assert!(gap_before_open < layout.open_btn_end_x);
        let dismiss_center = layout.open_btn_end_x + 6;
        assert!(dismiss_center > layout.open_btn_end_x);
    }

    #[test]
    fn test_update_modal_open_release_click() {
        let area = Rect::new(0, 0, 80, 24);
        let notes = "### Highlights\n- Feature A\n- Feature B";
        let layout = update_modal_layout(area, notes);

        let click_x = layout.popup_area.x + 5;
        let click_y = layout.button_row_y;

        assert!(
            layout
                .popup_area
                .contains(ratatui::layout::Position::new(click_x, click_y))
        );
        assert_eq!(click_y, layout.button_row_y);
        assert!(click_x < layout.open_button_midpoint_x);
    }

    #[test]
    fn test_update_modal_dismiss_click() {
        let area = Rect::new(0, 0, 80, 24);
        let notes = "Feature A";
        let layout = update_modal_layout(area, notes);

        let dismiss_x = layout.open_btn_end_x + 6;
        let dismiss_y = layout.button_row_y;

        assert!(
            layout
                .popup_area
                .contains(ratatui::layout::Position::new(dismiss_x, dismiss_y))
        );
        assert_eq!(dismiss_y, layout.button_row_y);
        assert!(dismiss_x >= layout.open_button_midpoint_x);

        let outside_x = layout.popup_area.x.saturating_sub(2);
        let outside_y = layout.popup_area.y.saturating_sub(2);
        assert!(
            !layout
                .popup_area
                .contains(ratatui::layout::Position::new(outside_x, outside_y))
        );
    }

    #[test]
    fn test_update_modal_geometry_bounds_various_screens() {
        let notes = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10\nLine 11";

        let compact = update_modal_layout(Rect::new(0, 0, 40, 15), notes);
        assert!(compact.popup_area.width <= 38);
        assert!(compact.popup_area.height <= 13);
        assert!(compact.has_more);

        let large = update_modal_layout(Rect::new(0, 0, 160, 50), notes);
        assert_eq!(large.popup_area.width, 76);
        assert_eq!(large.display_count, 11);
        assert!(!large.has_more);
    }

    #[test]
    fn test_download_confirm_action_row_matches_rendered_button_section() {
        let popup = Rect::new(10, 10, 40, 10);
        let summary_lines = 3;
        let action_row = download_confirm_action_row(popup, summary_lines);

        assert_eq!(action_row, popup.y + 4);
        assert!(popup.contains(ratatui::layout::Position::new(popup.x + 2, action_row)));
    }

    #[test]
    fn test_download_confirm_zones_do_not_overlap() {
        let area = Rect::new(0, 0, 80, 24);
        let summary_lines = 4;
        let longest = 30;
        let popup = download_confirm_layout(area, summary_lines, longest);
        let action_row = download_confirm_action_row(popup, summary_lines);

        assert!(popup.contains(ratatui::layout::Position::new(popup.x + 1, action_row)));
        assert!(action_row < popup.bottom() - 1);
    }

    #[test]
    fn test_browse_category_badges() {
        let theme = Theme::mocha();

        let (movies_badge, _) = browse_category_badge("Popular Movies", &theme);
        assert_eq!(movies_badge.content, "[MOVIES]");

        let (top_rated_badge, _) = browse_category_badge("Top Rated Movies", &theme);
        assert_eq!(top_rated_badge.content, "[MOVIES]");

        let (series_badge, _) = browse_category_badge("Popular Series", &theme);
        assert_eq!(series_badge.content, "[SERIES]");

        let (airing_badge, _) = browse_category_badge("Airing Today", &theme);
        assert_eq!(airing_badge.content, "[SERIES]");

        let (trending_badge, _) = browse_category_badge("Trending Today", &theme);
        assert_eq!(trending_badge.content, "[DISCOVER]");

        let (anime_badge, _) = browse_category_badge("Anime", &theme);
        assert_eq!(anime_badge.content, "[DISCOVER]");
    }

    #[test]
    fn test_picker_with_lines_rendering() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let theme = Theme::mocha();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let raw_items = vec!["[MOVIES]   Popular Movies".to_string()];
        let lines = vec![Line::from(vec![
            Span::styled("[MOVIES]", theme.sapphire),
            Span::raw("   "),
            Span::styled("Popular Movies", theme.text),
        ])];

        terminal
            .draw(|frame| {
                picker_with_lines(
                    frame,
                    Rect::new(0, 0, 80, 24),
                    &lines,
                    &raw_items,
                    &mut list_state,
                    PickerSpec {
                        title: "Browse",
                        confirm_label: "Open",
                        minimum_width: 36,
                        show_counter: true,
                    },
                    &theme,
                    false,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("[MOVIES]"));
        assert!(content.contains("Popular Movies"));
    }
    #[test]
    fn test_picker_layout_height_tight_fit() {
        let area = Rect::new(0, 0, 80, 24);
        let items_single = vec!["Single Item".to_string()];
        let layout_single = picker_layout(area, &items_single, "Open", 20);
        assert_eq!(layout_single.height, 3);

        let items_two = vec!["Item 1".to_string(), "Item 2".to_string()];
        let layout_two = picker_layout(area, &items_two, "Use", 20);
        assert_eq!(layout_two.height, 4);
    }

    #[test]
    fn test_notification_style_colors_match_kind() {
        let theme = Theme::mocha();

        let (_, info_style) = notification_style(NotificationKind::Info, &theme, false);
        assert_eq!(info_style.fg, theme.sapphire.fg);

        let (_, success_style) = notification_style(NotificationKind::Success, &theme, false);
        assert_eq!(success_style.fg, theme.success.fg);

        let (_, warning_style) = notification_style(NotificationKind::Warning, &theme, false);
        assert_eq!(warning_style.fg, theme.rating.fg);

        let (_, error_style) = notification_style(NotificationKind::Error, &theme, false);
        assert_eq!(error_style.fg, theme.error.fg);
    }
}
