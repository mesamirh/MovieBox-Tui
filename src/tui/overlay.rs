use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::tui::theme::Theme;

const MAX_PICKER_ROWS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub title: String,
    pub message: String,
    expires_at: Instant,
}

impl Notification {
    pub fn new(
        kind: NotificationKind,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let duration = match kind {
            NotificationKind::Info => Duration::from_secs(4),
            NotificationKind::Success => Duration::from_secs(5),
            NotificationKind::Warning => Duration::from_secs(7),
            NotificationKind::Error => Duration::from_secs(10),
        };
        Self {
            kind,
            title: title.into(),
            message: message.into(),
            expires_at: Instant::now() + duration,
        }
    }

    pub fn expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub struct PickerSpec<'a> {
    pub title: &'a str,
    pub confirm_label: &'a str,
    pub minimum_width: u16,
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
    let selected = state
        .selected()
        .unwrap_or(0)
        .min(items.len().saturating_sub(1));
    let visible_rows = items.len().clamp(1, MAX_PICKER_ROWS);
    let footer_width = crate::tui::text::width("[↑↓] Move  [Enter] Download  [Esc] Back");
    let content_width = items
        .iter()
        .map(|item| crate::tui::text::width(item))
        .max()
        .unwrap_or(0)
        .max(footer_width)
        .saturating_add(4);
    let popup = centered(
        area,
        content_width as u16,
        visible_rows as u16 + 4,
        spec.minimum_width,
        64,
    );
    clear_modal_area(frame, area, popup, theme);

    let title = format!(
        " {} · {}/{} ",
        spec.title,
        selected.saturating_add(1),
        items.len().max(1)
    );
    let block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(if basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.lavender);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let sections = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
    let list_items = items
        .iter()
        .map(|item| ListItem::new(item.clone()).style(theme.text))
        .collect::<Vec<_>>();
    let list = List::new(list_items)
        .highlight_style(selection_style(theme, basic_terminal))
        .highlight_symbol(if basic_terminal { "> " } else { "▌ " });
    frame.render_stateful_widget(list, sections[0], state);

    if items.len() > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(items.len())
            .viewport_content_length(visible_rows)
            .position(selected);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(theme.lavender)
            .track_style(theme.surface1)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        frame.render_stateful_widget(scrollbar, sections[0], &mut scrollbar_state);
    }

    let confirm_label = if sections[1].width < 40 && spec.confirm_label == "Download" {
        "Save"
    } else {
        spec.confirm_label
    };
    let footer = Line::from(vec![
        key_hint("↑↓", "Move", theme),
        Span::raw("  "),
        key_hint("Enter", confirm_label, theme),
        Span::raw("  "),
        key_hint("Esc", "Back", theme),
    ]);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme.muted),
        ),
        sections[1],
    );
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
    clear_modal_area(frame, area, popup, theme);

    let block = Block::default()
        .title(format!(" {title} "))
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_type(if basic_terminal {
            BorderType::Plain
        } else {
            BorderType::Rounded
        })
        .border_style(theme.lavender);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let sections = Layout::vertical([
        Constraint::Length(summary.len() as u16),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(summary.to_vec()).alignment(Alignment::Center),
        sections[0],
    );

    let selected_style = selection_style(theme, basic_terminal);
    let actions = Line::from(vec![
        Span::styled(
            " Download ",
            if confirm_selected {
                selected_style
            } else {
                theme.text_dim
            },
        ),
        Span::raw("    "),
        Span::styled(
            " Cancel ",
            if confirm_selected {
                theme.text_dim
            } else {
                selected_style
            },
        ),
    ]);
    frame.render_widget(
        Paragraph::new(actions).alignment(Alignment::Center),
        sections[1],
    );
    let footer = if sections[2].width < 44 {
        Line::from(vec![
            key_hint("←→", "Choose", theme),
            Span::raw(" "),
            key_hint("Enter", "OK", theme),
            Span::raw(" "),
            key_hint("Esc", "Back", theme),
        ])
    } else {
        Line::from(vec![
            key_hint("←→", "Choose", theme),
            Span::raw("  "),
            key_hint("Enter", "Confirm", theme),
            Span::raw("  "),
            key_hint("Esc", "Back", theme),
        ])
    };
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        sections[2],
    );
}

pub fn notifications(
    frame: &mut Frame,
    area: Rect,
    notifications: &std::collections::VecDeque<Notification>,
    theme: &Theme,
    basic_terminal: bool,
) {
    let mut y = area.y.saturating_add(1);
    for notification in notifications.iter().rev().take(3) {
        let message = middle_truncate(&notification.message, 48);
        let title_width = crate::tui::text::width(&notification.title).saturating_add(6);
        let message_width = crate::tui::text::width(&message);
        let width = title_width
            .max(message_width.saturating_add(4))
            .clamp(24, 52)
            .min(area.width.saturating_sub(4) as usize) as u16;
        if width < 4 || y.saturating_add(3) > area.bottom() {
            break;
        }
        let toast_area = Rect::new(
            area.right().saturating_sub(width).saturating_sub(2),
            y,
            width,
            3,
        );
        crate::tui::clear_area(frame, toast_area, theme);
        let (symbol, style) = notification_style(notification.kind, theme, basic_terminal);
        let block = Block::default()
            .title(Line::from(vec![
                Span::styled(format!(" {symbol} "), style),
                Span::styled(
                    notification.title.clone(),
                    style.add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
            ]))
            .borders(Borders::ALL)
            .border_type(if basic_terminal {
                BorderType::Plain
            } else {
                BorderType::Rounded
            })
            .border_style(style)
            .padding(ratatui::widgets::Padding::horizontal(1));
        frame.render_widget(
            Paragraph::new(message)
                .style(theme.text)
                .wrap(Wrap { trim: true })
                .block(block),
            toast_area,
        );
        y = y.saturating_add(3);
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

pub fn clear_modal_area(frame: &mut Frame, bounds: Rect, popup: Rect, theme: &Theme) {
    const HORIZONTAL_HALO: u16 = 3;
    const VERTICAL_HALO: u16 = 1;

    let x = popup.x.saturating_sub(HORIZONTAL_HALO).max(bounds.x);
    let y = popup.y.saturating_sub(VERTICAL_HALO).max(bounds.y);
    let right = popup
        .right()
        .saturating_add(HORIZONTAL_HALO)
        .min(bounds.right());
    let bottom = popup
        .bottom()
        .saturating_add(VERTICAL_HALO)
        .min(bounds.bottom());
    crate::tui::clear_area(
        frame,
        Rect::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y)),
        theme,
    );
}

pub(crate) fn key_hint(key: &str, action: &str, theme: &Theme) -> Span<'static> {
    Span::styled(format!("[{key}] {action}"), theme.text_dim)
}

pub(crate) fn selection_style(theme: &Theme, basic_terminal: bool) -> Style {
    let style = theme.text.add_modifier(Modifier::BOLD);
    if basic_terminal {
        style.add_modifier(Modifier::UNDERLINED)
    } else {
        style.bg(theme.surface0.fg.unwrap_or(theme.base))
    }
}

fn notification_style(
    kind: NotificationKind,
    theme: &Theme,
    basic_terminal: bool,
) -> (&'static str, Style) {
    match kind {
        NotificationKind::Info => ("i", theme.sapphire),
        NotificationKind::Success => (if basic_terminal { "+" } else { "✓" }, theme.success),
        NotificationKind::Warning => ("!", theme.rating),
        NotificationKind::Error => (if basic_terminal { "x" } else { "×" }, theme.error),
    }
}

fn middle_truncate(value: &str, maximum_width: usize) -> String {
    crate::tui::text::truncate_middle_width(value, maximum_width)
}
