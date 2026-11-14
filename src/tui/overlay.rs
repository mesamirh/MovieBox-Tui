use ratatui::{
    Frame,
    layout::{Alignment, Rect},
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

pub fn settings_picker_layout(
    area: Rect,
    category: crate::tui::state::SettingsCategory,
    items: &[String],
    minimum_width: u16,
) -> Rect {
    let settings_area = settings_modal_layout(area, category);
    let max_item_w = items
        .iter()
        .map(|item| crate::tui::text::width(item))
        .max()
        .unwrap_or(0);
    let max_avail_w = settings_area.width.saturating_sub(2).max(1);
    let min_w = minimum_width.min(max_avail_w).max(1);
    let content_width = (max_item_w as u16 + 4).clamp(min_w, max_avail_w);
    let visible_rows = items.len().clamp(1, max_picker_rows(area));
    let popup_height = (visible_rows as u16 + 2).min(area.height.saturating_sub(2));
    let x = settings_area.x + settings_area.width.saturating_sub(content_width) / 2;
    let y = settings_area.y + settings_area.height.saturating_sub(popup_height) / 2;
    let y = y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, content_width, popup_height)
}

pub fn browse_picker_layout(area: Rect, items: &[String], minimum_width: u16) -> Rect {
    let visible_rows = items.len().clamp(1, max_picker_rows(area));
    let content_width = items
        .iter()
        .map(|item| crate::tui::text::width(item))
        .max()
        .unwrap_or(0)
        .saturating_add(2);
    let popup_width = (content_width as u16)
        .max(minimum_width.min(area.width.saturating_sub(2)))
        .min(64)
        .min(area.width.saturating_sub(2));
    let popup_height = (visible_rows as u16 + 2).min(area.height.saturating_sub(2));
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, popup_width, popup_height)
}

pub(crate) fn home_search_y(area: Rect) -> u16 {
    if area.height >= 24 {
        if area.width < 76 {
            area.y + 7
        } else {
            area.y + 11
        }
    } else {
        area.y + 5
    }
}

pub fn tv_config_layout(
    area: Rect,
    longest_source_width: usize,
    total_rows: usize,
    input_active: bool,
) -> Rect {
    let (popup_width, popup_height) = if input_active {
        let w = 52u16.min(area.width.saturating_sub(2)).max(28);
        (w, 5u16)
    } else {
        let longest_line = (longest_source_width as u16).saturating_add(6).max(24);
        let w = longest_line
            .saturating_add(2)
            .min(area.width.saturating_sub(2))
            .max(28);
        let h = (total_rows as u16)
            .saturating_add(2)
            .min(area.height.saturating_sub(2))
            .max(3);
        (w, h)
    };
    let available_width = area.width.saturating_sub(2).max(1);
    let width = popup_width.min(available_width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, width, popup_height)
}

pub fn addon_manager_layout(
    area: Rect,
    addons_count: usize,
    max_name_len: usize,
    input_active: bool,
) -> Rect {
    let (popup_width, popup_height) = if input_active {
        let w = 48u16.min(area.width.saturating_sub(2)).max(28);
        (w, 5u16)
    } else {
        let longest_line = (max_name_len as u16).saturating_add(6).max(22);
        let w = longest_line
            .saturating_add(2)
            .min(area.width.saturating_sub(2))
            .max(24);
        let total_items = (addons_count as u16).saturating_add(1);
        let h = total_items
            .saturating_add(2)
            .min(area.height.saturating_sub(2))
            .max(3);
        (w, h)
    };
    let available_width = area.width.saturating_sub(2).max(1);
    let width = popup_width.min(available_width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, width, popup_height)
}

pub fn settings_modal_layout(area: Rect, category: crate::tui::state::SettingsCategory) -> Rect {
    let min_width = 44u16.min(area.width.saturating_sub(2));
    let popup_width = 58u16.min(area.width.saturating_sub(2)).max(min_width);
    let content_height = category.row_count() as u16;
    let popup_height = (content_height + 4).min(area.height.saturating_sub(2));
    let available_width = area.width.saturating_sub(2).max(1);
    let width = popup_width.min(available_width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(popup_height));
    Rect::new(x, y, width, popup_height)
}

pub fn help_modal_layout(area: Rect, desired_width: u16, desired_height: u16) -> Rect {
    let available_width = area.width.saturating_sub(2).max(1);
    let available_height = area.height.saturating_sub(2).max(1);
    let width = desired_width.clamp(46, 120).min(available_width);
    let height = desired_height.min(available_height).max(1);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(height));
    Rect::new(x, y, width, height)
}

pub fn overview_modal_layout_with_wrapped(area: Rect, content: &str) -> (Rect, Vec<String>) {
    let available_w = area.width.saturating_sub(4);
    let width = 72u16.min(available_w).max(36);
    let inner_w = width.saturating_sub(4) as usize;
    let wrapped = crate::tui::text::wrap_text(content, inner_w);
    let content_lines = wrapped.len();
    let desired_h = (content_lines as u16 + 2).max(4);
    let max_h = area.height.saturating_sub(2).max(4);
    let height = desired_h.min(max_h);
    let popup = centered(area, width, height, 36, width);
    (popup, wrapped)
}

pub fn overview_modal_layout(area: Rect, content: &str) -> (Rect, usize) {
    let (popup, wrapped) = overview_modal_layout_with_wrapped(area, content);
    (popup, wrapped.len())
}

pub fn overview_modal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
    scroll_offset: usize,
    theme: &Theme,
    basic_terminal: bool,
) -> Rect {
    let (popup, wrapped) = overview_modal_layout_with_wrapped(area, content);
    let total_lines = wrapped.len();
    clear_modal_area(frame, area, popup, theme);
    let inner_h = popup.height.saturating_sub(2) as usize;

    let has_scroll = total_lines > inner_h;

    let title_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(title, theme.title.add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type(basic_terminal))
        .border_style(theme.surface1)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0))
        .title(title_line)
        .title_alignment(Alignment::Left);

    if has_scroll {
        block = block.title_bottom(
            Line::from(vec![Span::styled(" [↑/↓] Scroll ", theme.subtext1)])
                .alignment(Alignment::Right),
        );
    }

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = wrapped
        .into_iter()
        .map(|line| Line::from(Span::styled(line, theme.text)))
        .collect();

    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, inner);

    popup
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
    let popup = picker_layout(area, items, spec.confirm_label, spec.minimum_width);
    picker_at(
        frame,
        area,
        popup,
        items,
        state,
        spec,
        theme,
        basic_terminal,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn picker_at(
    frame: &mut Frame,
    area: Rect,
    popup: Rect,
    items: &[String],
    state: &mut ListState,
    spec: PickerSpec<'_>,
    theme: &Theme,
    basic_terminal: bool,
) {
    let padded_items: Vec<String> = items.iter().map(|item| format!("  {item}  ")).collect();
    let lines: Vec<Line<'static>> = items
        .iter()
        .map(|item| {
            Line::from(vec![
                Span::raw("  "),
                Span::raw(item.clone()),
                Span::raw("  "),
            ])
        })
        .collect();
    picker_with_lines_at(
        frame,
        area,
        popup,
        &lines,
        &padded_items,
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
    let popup = picker_layout(area, raw_items, spec.confirm_label, spec.minimum_width);
    picker_with_lines_at(
        frame,
        area,
        popup,
        lines,
        raw_items,
        state,
        spec,
        theme,
        basic_terminal,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn picker_with_lines_at<'a>(
    frame: &mut Frame,
    area: Rect,
    popup: Rect,
    lines: &[Line<'a>],
    _raw_items: &[String],
    state: &mut ListState,
    spec: PickerSpec<'_>,
    theme: &Theme,
    basic_terminal: bool,
) {
    let selected = state
        .selected()
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
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

    let visible_rows = inner.height as usize;
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

pub fn browse_category_badge_text(label: &str) -> &'static str {
    let lower = label.to_ascii_lowercase();
    if lower.contains("movie")
        || lower.contains("top rated (all-time)")
        || lower.contains("top rated (recent")
    {
        "[MOVIES]"
    } else if lower.contains("series")
        || lower.contains("airing")
        || lower.contains("show")
        || lower.contains("tv")
    {
        "[SERIES]"
    } else {
        "[DISCOVER]"
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

struct NotificationCard<'a> {
    original_idx: usize,
    notification: &'a Notification,
    rect: Rect,
    inner_width: usize,
    msg_lines: Vec<String>,
}

fn notification_badge(kind: NotificationKind, basic_terminal: bool) -> &'static str {
    match kind {
        NotificationKind::Info => {
            if basic_terminal {
                "i INFO"
            } else {
                "ℹ INFO"
            }
        }
        NotificationKind::Success => {
            if basic_terminal {
                "+ SUCCESS"
            } else {
                "✔ SUCCESS"
            }
        }
        NotificationKind::Warning => {
            if basic_terminal {
                "! WARNING"
            } else {
                "⚠ WARNING"
            }
        }
        NotificationKind::Error => {
            if basic_terminal {
                "x ERROR"
            } else {
                "✖ ERROR"
            }
        }
    }
}

fn compute_notification_cards<'a>(
    area: Rect,
    notifications: &'a std::collections::VecDeque<Notification>,
    basic_terminal: bool,
    download_active: bool,
) -> Vec<NotificationCard<'a>> {
    let mut cards = Vec::new();
    let bottom_offset = if download_active { 5 } else { 2 };
    let mut y = area.bottom().saturating_sub(bottom_offset);

    let (max_visible, max_card_w, max_msg_lines) = if area.height < 20 {
        (1, 42.min(area.width.saturating_sub(4) as usize), 1)
    } else if area.width < 65 {
        (1, 42.min(area.width.saturating_sub(4) as usize), 2)
    } else if area.height < 30 {
        (2, 56.min(area.width.saturating_sub(4) as usize), 2)
    } else {
        (3, 64.min(area.width.saturating_sub(4) as usize), 3)
    };

    for (rev_idx, notification) in notifications.iter().rev().take(max_visible).enumerate() {
        let badge = notification_badge(notification.kind, basic_terminal);
        let has_message =
            !notification.message.is_empty() && notification.message != notification.title;

        let title_w = crate::tui::text::width(&notification.title).saturating_add(6);
        let badge_w = crate::tui::text::width(badge).saturating_add(6);
        let raw_msg_w = if has_message {
            notification
                .message
                .lines()
                .map(|line| crate::tui::text::width(line.trim()))
                .max()
                .unwrap_or(0)
                .saturating_add(6)
        } else {
            0
        };

        let target_card_width = title_w
            .max(badge_w)
            .max(raw_msg_w)
            .clamp(20, max_card_w.max(20)) as u16;

        let inner_width = (target_card_width.saturating_sub(4) as usize).max(1);

        let msg_lines: Vec<String> = if has_message {
            crate::tui::text::wrap_text(&notification.message, inner_width)
                .into_iter()
                .take(max_msg_lines)
                .collect()
        } else {
            Vec::new()
        };

        let height = 2 + 1 + msg_lines.len() as u16;

        if target_card_width < 10 || y < area.y.saturating_add(height) {
            break;
        }
        y = y.saturating_sub(height);

        let rect = Rect::new(
            area.right()
                .saturating_sub(target_card_width)
                .saturating_sub(2),
            y,
            target_card_width,
            height,
        );

        let original_idx = notifications.len().saturating_sub(1 + rev_idx);
        cards.push(NotificationCard {
            original_idx,
            notification,
            rect,
            inner_width,
            msg_lines,
        });

        y = y.saturating_sub(1);
    }
    cards
}

/// Spinner frame for sticky notifications, driven by wall-clock time so it
/// keeps moving on every redraw.
fn activity_indicator(basic_terminal: bool) -> String {
    const FANCY: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    const BASIC: [&str; 4] = ["|", "/", "-", "\\"];
    let step = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_millis() / 100) as usize)
        .unwrap_or(0);
    if basic_terminal {
        format!("{} running", BASIC[step % BASIC.len()])
    } else {
        format!("{} running", FANCY[step % FANCY.len()])
    }
}

pub fn notifications(
    frame: &mut Frame,
    area: Rect,
    notifications: &std::collections::VecDeque<Notification>,
    theme: &Theme,
    basic_terminal: bool,
    download_active: bool,
) {
    for card in compute_notification_cards(area, notifications, basic_terminal, download_active) {
        let (badge, badge_style) =
            notification_style(card.notification.kind, theme, basic_terminal);

        crate::tui::clear_area(frame, card.rect, theme);

        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            crate::tui::text::truncate_width(&card.notification.title, card.inner_width),
            badge_style.add_modifier(Modifier::BOLD),
        )]));

        for line in &card.msg_lines {
            lines.push(Line::from(vec![Span::styled(
                crate::tui::text::truncate_width(line, card.inner_width),
                theme.subtext1,
            )]));
        }

        let countdown_bar = if card.notification.sticky {
            // No countdown: the card stays until it is dismissed, so show
            // that something is still running instead.
            activity_indicator(basic_terminal)
        } else {
            let total_duration = card.notification.kind.total_duration();
            let remaining = card
                .notification
                .expires_at
                .saturating_duration_since(std::time::Instant::now());
            let ratio = (remaining.as_secs_f64() / total_duration.as_secs_f64()).clamp(0.0, 1.0);
            let bar_width = card.inner_width.clamp(3, 16);
            let filled = ((bar_width as f64) * ratio).round() as usize;
            if basic_terminal {
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
            }
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

        frame.render_widget(Paragraph::new(lines).block(block), card.rect);
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

pub fn render_modal_frame(
    frame: &mut Frame,
    popup_area: Rect,
    block: Block,
    theme: &Theme,
) -> Rect {
    clear_modal_area(frame, popup_area, popup_area, theme);
    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    inner_area
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
    } else if theme.is_light {
        let bg = theme.surface0_color();
        let fg = theme.text.fg.unwrap_or(theme.base);
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
    } else {
        let bg = theme.surface1_color();
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
    let style = match kind {
        NotificationKind::Info => theme.sapphire,
        NotificationKind::Success => theme.success,
        NotificationKind::Warning => theme.rating,
        NotificationKind::Error => theme.error,
    };
    (notification_badge(kind, basic_terminal), style)
}

pub fn notification_rects(
    area: Rect,
    notifications: &std::collections::VecDeque<Notification>,
    basic_terminal: bool,
    download_active: bool,
) -> Vec<(usize, Rect)> {
    compute_notification_cards(area, notifications, basic_terminal, download_active)
        .into_iter()
        .map(|card| (card.original_idx, card.rect))
        .collect()
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
    let has_managed_notice = std::env::current_exe()
        .ok()
        .map(|p| crate::updater::apply::detect_environment(&p).has_managed_notice())
        .unwrap_or(false);
    update_modal_layout_with_env(area, notes, has_managed_notice)
}

pub fn update_modal_layout_with_env(
    area: Rect,
    notes: &str,
    has_managed_notice: bool,
) -> UpdateModalLayout {
    let mut count = 0;
    for line in notes.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.to_ascii_lowercase().contains("read full changelog")
            && !trimmed.to_ascii_lowercase().contains("press [o]")
        {
            count += 1;
        }
    }
    let note_lines_count = count.max(1);

    let min_w: u16 = 48;
    let max_w: u16 = 64;
    let available_w = area.width.saturating_sub(4);
    let desired_w = max_w.min(available_w).max(min_w.min(available_w));

    let managed_rows: u16 = if has_managed_notice { 1 } else { 0 };
    let available_height = area.height.saturating_sub(4);
    let available_note_rows =
        (available_height.saturating_sub(managed_rows + 4) as usize).clamp(1, 10);

    let display_count = note_lines_count.min(available_note_rows);
    let has_more = note_lines_count > display_count;
    let total_rows = (display_count as u16) + managed_rows + 4;
    let desired_h = total_rows.min(available_height.max(6));

    const UPDATE_SEGMENT: u16 = 14;
    const OPEN_SEGMENT: u16 = 14;

    let available_width = area.width.saturating_sub(2).max(1);
    let width = desired_w.min(available_width);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let search_y = home_search_y(area);
    let y = search_y.min(area.bottom().saturating_sub(desired_h));
    let popup_area = Rect::new(x, y, width, desired_h);
    let footer_width = UPDATE_SEGMENT + OPEN_SEGMENT;

    let button_row_y = popup_area.bottom().saturating_sub(1);
    let inner_width = popup_area.width.saturating_sub(2);
    let footer_start = popup_area.x + 1 + inner_width.saturating_sub(footer_width) / 2;
    let update_btn_end_x = footer_start + UPDATE_SEGMENT;
    let open_btn_end_x = update_btn_end_x + OPEN_SEGMENT;
    let open_button_midpoint_x = update_btn_end_x + OPEN_SEGMENT / 2;
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

        assert_eq!(layout.popup_area.width, 64);
        assert_eq!(layout.popup_area.height, 8);
        assert_eq!(layout.popup_area.x, (80 - 64) / 2);
        assert_eq!(layout.popup_area.y, home_search_y(area));
        assert_eq!(
            layout.button_row_y,
            layout.popup_area.bottom().saturating_sub(1)
        );
        let footer_start = layout.popup_area.x + 1 + (62 - 28) / 2;
        assert_eq!(layout.update_btn_end_x, footer_start + 14);
        assert_eq!(layout.open_btn_end_x, layout.update_btn_end_x + 14);
        assert_eq!(layout.open_button_midpoint_x, layout.update_btn_end_x + 7);
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
        assert_eq!(large.popup_area.width, 64);
        assert_eq!(large.display_count, 10);
        assert!(large.has_more);
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
    fn test_picker_layout_symmetrical_margins() {
        let area = Rect::new(0, 0, 80, 24);
        let items = vec![
            "  [✓] MovieBox          ".to_string(),
            "  [✓] CircleFTP (BDIX)  ".to_string(),
        ];
        let layout = picker_layout(area, &items, "", 10);
        assert_eq!(layout.width, 26);
        let inner_width = layout.width.saturating_sub(2);
        assert_eq!(inner_width, 24);
    }

    #[test]
    fn test_overview_modal_layout_bounds_and_lines() {
        let standard_area = Rect::new(0, 0, 80, 24);
        let short_content = "A brief synopsis.";
        let (short_popup, short_lines) = overview_modal_layout(standard_area, short_content);
        assert_eq!(short_lines, 1);
        assert!(short_popup.width <= 76);
        assert!(short_popup.height >= 4);

        let long_content =
            "This is a much longer overview describing the story in great detail. ".repeat(20);
        let (long_popup, long_lines) = overview_modal_layout(standard_area, &long_content);
        assert!(long_lines > 10);
        assert_eq!(long_popup.height, 22);
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

    #[test]
    fn test_notification_rects_adaptive_tiering() {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(Notification::new(NotificationKind::Info, "N1", "M1"));
        queue.push_back(Notification::new(NotificationKind::Info, "N2", "M2"));
        queue.push_back(Notification::new(NotificationKind::Info, "N3", "M3"));

        let compact_area = Rect::new(0, 0, 60, 18);
        let compact_rects = notification_rects(compact_area, &queue, false, false);
        assert_eq!(compact_rects.len(), 1);

        let standard_area = Rect::new(0, 0, 80, 24);
        let standard_rects = notification_rects(standard_area, &queue, false, false);
        assert_eq!(standard_rects.len(), 2);

        let large_area = Rect::new(0, 0, 120, 35);
        let large_rects = notification_rects(large_area, &queue, false, false);
        assert_eq!(large_rects.len(), 3);
    }

    #[test]
    fn test_notification_rects_mobile_portrait() {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(Notification::new(
            NotificationKind::Error,
            "Title",
            "Line 1\nLine 2",
        ));
        let mobile_portrait = Rect::new(0, 0, 40, 26);
        let rects = notification_rects(mobile_portrait, &queue, false, false);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].1.height >= 5);
    }
    #[test]
    fn test_notification_rects_multiline_width_scales_to_longest_line() {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(Notification::new(
            NotificationKind::Error,
            "Download failed",
            "DASH streams require yt-dlp & ffmpeg.\nRun: winget install yt-dlp.yt-dlp Gyan.FFmpeg",
        ));
        let area = Rect::new(0, 0, 100, 30);
        let rects = notification_rects(area, &queue, false, false);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].1.width >= 48);
        assert!(rects[0].1.width <= 64);
        assert_eq!(rects[0].1.height, 5);
    }
    #[test]
    fn test_addon_manager_layout_anchors_at_search_bar_position() {
        let standard_area = Rect::new(0, 0, 80, 24);
        let layout = addon_manager_layout(standard_area, 2, 10, false);
        assert_eq!(layout.y, 11);
        assert_eq!(layout.height, 5);
        assert_eq!(layout.width, 24);
        assert_eq!(layout.x, 28);

        let compact_area = Rect::new(0, 0, 70, 24);
        let compact_layout = addon_manager_layout(compact_area, 1, 8, false);
        assert_eq!(compact_layout.y, 7);
        assert_eq!(compact_layout.height, 4);
        assert_eq!(compact_layout.width, 24);
        assert_eq!(compact_layout.x, 23);

        let short_area = Rect::new(0, 0, 80, 20);
        let short_layout = addon_manager_layout(short_area, 1, 8, false);
        assert_eq!(short_layout.y, 5);

        let input_layout = addon_manager_layout(standard_area, 2, 10, true);
        assert_eq!(input_layout.height, 5);
        assert_eq!(input_layout.y, 11);
        assert_eq!(input_layout.width, 48);
        assert_eq!(input_layout.x, 16);
    }
    #[test]
    fn test_picker_layouts_cross_platform_tiers() {
        let items = vec![
            "MovieBox".to_string(),
            "4KHDHub".to_string(),
            "Dramachi".to_string(),
            "CircleFTP (BDIX)".to_string(),
        ];
        let dimensions = [
            (200, 60),
            (120, 40),
            (80, 24),
            (60, 20),
            (45, 18),
            (35, 14),
            (20, 10),
        ];
        for (w, h) in dimensions {
            let area = Rect::new(0, 0, w, h);
            let layout = settings_picker_layout(
                area,
                crate::tui::state::SettingsCategory::ContentModes,
                &items,
                20,
            );
            assert!(layout.x >= area.x);
            assert!(layout.y >= area.y);
            assert!(layout.right() <= area.right());
            assert!(layout.bottom() <= area.bottom());

            let browse = browse_picker_layout(area, &items, 36);
            assert!(browse.x >= area.x);
            assert!(browse.y >= area.y);
            assert!(browse.right() <= area.right());
            assert!(browse.bottom() <= area.bottom());
        }
    }
    #[test]
    fn test_picker_with_lines_at_basic_terminal_rendering() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let area = Rect::new(0, 0, 80, 24);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::mocha();
        let items = vec![
            "Item 1".to_string(),
            "Item 2".to_string(),
            "Item 3".to_string(),
        ];
        let lines: Vec<Line> = items.iter().map(|s| Line::from(s.clone())).collect();
        let mut state = ListState::default();
        state.select(Some(0));

        let popup = settings_picker_layout(
            area,
            crate::tui::state::SettingsCategory::General,
            &items,
            20,
        );

        terminal
            .draw(|f| {
                picker_with_lines_at(
                    f,
                    area,
                    popup,
                    &lines,
                    &items,
                    &mut state,
                    PickerSpec {
                        title: "Pick",
                        confirm_label: "Select",
                        minimum_width: 20,
                        show_counter: false,
                    },
                    &theme,
                    true,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("Item 1"));
        assert!(!content.contains("╭"));
        assert!(!content.contains("╰"));
    }
    #[test]
    fn test_settings_picker_layout_empty_items_does_not_panic() {
        let area = Rect::new(0, 0, 80, 24);
        let items: Vec<String> = Vec::new();
        let layout = settings_picker_layout(
            area,
            crate::tui::state::SettingsCategory::General,
            &items,
            20,
        );
        assert!(layout.width >= 20);
        assert!(layout.height >= 3);
    }
}
