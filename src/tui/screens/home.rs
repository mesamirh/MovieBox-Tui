use crate::tui::widgets::render_poster_placeholder;
use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchViewState {
    Empty,
    Editing,
    Loading,
    Results,
    NoResults,
    Error,
}

fn search_view_state(state: &AppState) -> SearchViewState {
    if state.input_mode == InputMode::Editing {
        SearchViewState::Editing
    } else if state.is_loading
        || (!state.has_search_settled
            && (!state.search_query.trim().is_empty()
                || state.active_browse_preset.is_some()
                || state.active_addon_catalog.is_some()))
    {
        SearchViewState::Loading
    } else if state.search_error.is_some() {
        SearchViewState::Error
    } else if !state.search_results.is_empty() {
        SearchViewState::Results
    } else if state.has_search_settled
        && (!state.search_query.trim().is_empty()
            || state.active_browse_preset.is_some()
            || state.active_addon_catalog.is_some())
    {
        SearchViewState::NoResults
    } else {
        SearchViewState::Empty
    }
}

pub(crate) fn slash_command_description(cmd: &str, state: &AppState) -> Option<&'static str> {
    crate::tui::commands::SlashCommand::description_for(cmd, state)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeLayoutTier {
    Compact,
    Normal,
    Wide,
}

impl HomeLayoutTier {
    pub(crate) fn for_width(width: u16) -> Self {
        if width < 76 {
            Self::Compact
        } else if width < 110 {
            Self::Normal
        } else {
            Self::Wide
        }
    }

    pub(crate) fn is_compact(self) -> bool {
        self == Self::Compact
    }
}

pub struct LandingRows {
    pub rects: std::rc::Rc<[Rect]>,
    pub logo: usize,
    pub version: usize,
    pub search: usize,
    pub favorites: usize,
    pub mode_row: usize,
    pub logo_width: u16,
}

pub fn landing_split(
    area: Rect,
    tv_mode: bool,
    basic_terminal: bool,
    favorites_visible: bool,
) -> (HomeLayoutTier, LandingRows) {
    let tier = HomeLayoutTier::for_width(area.width);
    let compact_logo = tier.is_compact() || (tv_mode && area.width < 80);
    let effective_basic = basic_terminal || compact_logo;
    let logo_height = if effective_basic { 2 } else { 6 };
    let logo_width = if effective_basic {
        if tv_mode { 33 } else { 31 }
    } else if tv_mode {
        75
    } else {
        73
    };
    let top_pad = if area.height < 24 { 1 } else { 2 };
    let header_gap = if area.height < 24 { 1 } else { 2 };
    let favorites_gap = if favorites_visible { 1 } else { 0 };
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(logo_height),
            Constraint::Length(1),
            Constraint::Length(header_gap),
            Constraint::Length(3),
            Constraint::Length(favorites_gap),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    (
        tier,
        LandingRows {
            rects,
            logo: 1,
            version: 2,
            search: 4,
            favorites: 6,
            mode_row: 7,
            logo_width,
        },
    )
}

pub fn search_deck_width(area: Rect, _state: &AppState, landing: bool) -> u16 {
    let tier = HomeLayoutTier::for_width(area.width);
    if landing {
        let target_width = match tier {
            HomeLayoutTier::Compact => 54,
            HomeLayoutTier::Normal | HomeLayoutTier::Wide => 64,
        };
        target_width.min(area.width.saturating_sub(4)).max(24)
    } else {
        let target_width = match tier {
            HomeLayoutTier::Compact => 64,
            HomeLayoutTier::Normal => 84,
            HomeLayoutTier::Wide => 104,
        };
        target_width.min(area.width.saturating_sub(4)).max(30)
    }
}

fn render_search_state(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    view: SearchViewState,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let query = crate::tui::text::truncate_width(
        &state.search_query,
        area.width.min(64).saturating_sub(10) as usize,
    );

    let ctrl_p = crate::tui::text::CTRL_P_STR;
    let bullet = if state.basic_terminal {
        " - "
    } else {
        "  •  "
    };

    let mut lines: Vec<Line> = Vec::new();

    match view {
        SearchViewState::Loading => {
            if state.basic_terminal {
                let dots = match (state.tick_count / 4) % 3 {
                    0 => ".",
                    1 => "..",
                    _ => "...",
                };
                let text = if let Some(preset) = state.active_browse_preset {
                    format!("Loading {}{dots}", preset.label())
                } else if let Some(catalog) = &state.active_addon_catalog {
                    format!("Loading {}{dots}", catalog.label)
                } else if state.is_homepage_mode {
                    format!("Loading discover{dots}")
                } else if !state.search_query.trim().is_empty() {
                    format!("Searching for “{query}”{dots}")
                } else {
                    format!("Loading{dots}")
                };
                lines.push(Line::from(vec![Span::styled(text, theme.lavender)]));
            } else {
                let spinner =
                    crate::tui::widgets::loading_spinner(state.tick_count, state.basic_terminal);
                let text = if let Some(preset) = state.active_browse_preset {
                    format!("Loading {}", preset.label())
                } else if let Some(catalog) = &state.active_addon_catalog {
                    format!("Loading {}", catalog.label)
                } else if state.is_homepage_mode {
                    "Loading discover".to_string()
                } else if !state.search_query.trim().is_empty() {
                    format!("Searching for “{query}”")
                } else {
                    "Loading".to_string()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{spinner} "),
                        theme.accent.add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(text, theme.subtext1.add_modifier(Modifier::BOLD)),
                ]));
            }
        }
        SearchViewState::NoResults => {
            let provider_label = state.active_provider.label();
            let next_provider = state.next_provider();
            let msg = if let Some(preset) = state.active_browse_preset {
                format!("No items found for {}", preset.label())
            } else if let Some(catalog) = &state.active_addon_catalog {
                format!("No items found for {}", catalog.label)
            } else if state.search_query.trim().eq_ignore_ascii_case("/history") {
                "No watch history found".to_string()
            } else if state.search_query.trim().eq_ignore_ascii_case("/favorites")
                || state.search_query.trim().eq_ignore_ascii_case("/fav")
            {
                "No favorites saved yet".to_string()
            } else if !state.search_query.trim().is_empty() {
                format!("No results for “{query}” on {provider_label}")
            } else {
                "No results found".to_string()
            };
            lines.push(Line::from(vec![Span::styled(
                msg,
                theme.text.add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            let is_compact_btn = area.width < 56;
            let sep = if is_compact_btn { "  " } else { "        " };
            let pills = if state.is_tv_mode {
                let (btn1_label, btn2_label) = if is_compact_btn {
                    ("[ Reload (", "[ Clear (")
                } else {
                    ("[ Reload Playlists (", "[ Clear Search (")
                };
                vec![
                    Span::styled(btn1_label, theme.subtext1),
                    Span::styled("r", theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                    Span::raw(sep),
                    Span::styled(btn2_label, theme.subtext1),
                    Span::styled("c", theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                ]
            } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
                let (btn1_label, btn2_label) = if is_compact_btn {
                    ("[ Addons (", "[ Clear (")
                } else {
                    ("[ Addon Manager (", "[ Clear Search (")
                };
                vec![
                    Span::styled(btn1_label, theme.subtext1),
                    Span::styled(ctrl_p, theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                    Span::raw(sep),
                    Span::styled(btn2_label, theme.subtext1),
                    Span::styled("c", theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                ]
            } else {
                let btn2_label = if is_compact_btn {
                    "[ Clear ("
                } else {
                    "[ Clear Search ("
                };
                let btn1_label = if is_compact_btn {
                    "[ Try Provider (".to_string()
                } else {
                    format!("[ Try on {} (", next_provider.label())
                };
                vec![
                    Span::styled(btn1_label, theme.subtext1),
                    Span::styled(ctrl_p, theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                    Span::raw(sep),
                    Span::styled(btn2_label, theme.subtext1),
                    Span::styled("c", theme.shortcut),
                    Span::styled(") ]", theme.subtext1),
                ]
            };

            lines.push(Line::from(pills));
        }
        SearchViewState::Error => {
            let symbol = if state.basic_terminal { "!" } else { "×" };
            let err_text = state.search_error.as_deref().unwrap_or_else(|| {
                if !state.status_message.is_empty() {
                    &state.status_message
                } else {
                    "Search request failed"
                }
            });
            let wrap_width = area.width.min(72).saturating_sub(8).max(10) as usize;
            let wrapped_err_lines = crate::tui::text::wrap_text(err_text, wrap_width);

            lines.push(Line::from(vec![
                Span::styled(format!("{symbol} "), theme.error),
                Span::styled(
                    "Search Request Error",
                    theme.error.add_modifier(Modifier::BOLD),
                ),
            ]));
            if !wrapped_err_lines.is_empty() {
                for eline in wrapped_err_lines {
                    lines.push(Line::from(vec![Span::styled(eline, theme.subtext1)]));
                }
            } else {
                lines.push(Line::from(vec![Span::styled(
                    "Unable to complete search request with the current provider.",
                    theme.subtext1,
                )]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("[", theme.text_dim),
                Span::styled("r", theme.shortcut),
                Span::styled("] ", theme.text_dim),
                Span::styled("Retry", theme.subtext1),
                Span::styled(bullet, theme.text_dim),
                Span::styled("[", theme.text_dim),
                Span::styled(ctrl_p, theme.shortcut),
                Span::styled("] ", theme.text_dim),
                Span::styled("Switch Provider", theme.subtext1),
                Span::styled(bullet, theme.text_dim),
                Span::styled("[", theme.text_dim),
                Span::styled("Esc", theme.shortcut),
                Span::styled("] ", theme.text_dim),
                Span::styled("Back", theme.subtext1),
            ]));
        }
        _ => return,
    };

    let is_boxed = view == SearchViewState::Error;
    let card_height = if is_boxed {
        (lines.len() as u16 + 4).min(area.height)
    } else {
        lines.len() as u16
    };
    let card_y = if is_boxed {
        area.y + area.height.saturating_sub(card_height) / 2
    } else {
        area.y + area.height.saturating_sub(card_height) / 3
    };
    let card = Rect {
        x: area.x,
        y: card_y,
        width: area.width,
        height: card_height,
    };

    if is_boxed && card.height >= 3 {
        let (border_style, title_text, title_style) = (
            theme.error,
            " Error ",
            theme.error.add_modifier(Modifier::BOLD),
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(crate::tui::overlay::border_type(state.basic_terminal))
            .border_style(border_style)
            .title(Line::from(vec![Span::styled(title_text, title_style)]))
            .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .alignment(Alignment::Left),
            card,
        );
    } else {
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), card);
    }
}

pub(crate) fn no_results_button_hitboxes(
    area: Rect,
    next_provider_label: &str,
    ctrl_p: &str,
    is_tv_mode: bool,
    is_addon_mode: bool,
) -> (Rect, Rect) {
    let is_compact_btn = area.width < 56;
    let (btn1_w, btn2_w, sep_w) = if is_compact_btn {
        let b1 = if is_tv_mode {
            14
        } else if is_addon_mode {
            12 + ctrl_p.len() as u16
        } else {
            18 + ctrl_p.len() as u16
        };
        (b1, 13, 2)
    } else {
        let b1 = if is_tv_mode {
            24
        } else if is_addon_mode {
            19 + ctrl_p.len() as u16
        } else {
            (14 + next_provider_label.len() + ctrl_p.len()) as u16
        };
        (b1, 20, 8)
    };
    let total_w = btn1_w + sep_w + btn2_w;
    let start_x = area.x + area.width.saturating_sub(total_w) / 2;
    let card_y = area.y + area.height.saturating_sub(3) / 3;
    let btn_y = card_y + 2;

    let btn1 = Rect {
        x: start_x,
        y: btn_y,
        width: btn1_w,
        height: 1,
    };
    let btn2 = Rect {
        x: start_x + btn1_w + sep_w,
        y: btn_y,
        width: btn2_w,
        height: 1,
    };
    (btn1, btn2)
}

pub(crate) fn render_landing_deck(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let tab = state.effective_home_deck_tab();
    let has_cw = state.continue_watching_available();
    let has_fav = state.favorites_available() && !state.favorites.items.is_empty();

    if !has_cw && !has_fav {
        return;
    }

    let modal_active = state.has_active_modal();
    let is_focused = state.favorites_focus && !modal_active;

    let cw_items = if has_cw {
        state.continue_watching_items()
    } else {
        Vec::new()
    };
    let fav_items = if has_fav {
        state.favorites_landing_items()
    } else {
        Vec::new()
    };

    let (item_count, total_count) = match tab {
        crate::tui::state::HomeDeckTab::ContinueWatching => {
            let total = state
                .history
                .recent
                .iter()
                .filter(|i| i.is_in_progress())
                .count();
            (cw_items.len(), total)
        }
        crate::tui::state::HomeDeckTab::Favorites => (fav_items.len(), state.favorites.items.len()),
    };

    if item_count == 0 {
        return;
    }

    let overflow = total_count.saturating_sub(item_count);
    let card_width = search_deck_width(area, state, true);
    let row_count = item_count as u16;
    let overflow_row = u16::from(overflow > 0);
    let content_height = (row_count + overflow_row + 2).min(area.height);

    let card_area = Rect {
        x: area.x + area.width.saturating_sub(card_width) / 2,
        y: area.y,
        width: card_width,
        height: content_height,
    };

    let border_style = if modal_active {
        theme.muted
    } else if is_focused {
        theme.border_focus
    } else {
        theme.surface1
    };

    let mut title_spans: Vec<Span> = Vec::new();
    let (active_style, inactive_style, hint_style, sep_style) = if modal_active {
        (theme.muted, theme.muted, theme.muted, theme.muted)
    } else {
        (
            if is_focused {
                theme.title.add_modifier(Modifier::BOLD)
            } else {
                theme.text.add_modifier(Modifier::BOLD)
            },
            theme.subtext1,
            theme.text_dim,
            theme.surface1,
        )
    };
    if has_cw && has_fav {
        let sep = if state.basic_terminal { " | " } else { " │ " };
        match tab {
            crate::tui::state::HomeDeckTab::ContinueWatching => {
                title_spans.push(Span::styled(" Continue Watching ", active_style));
                title_spans.push(Span::styled(sep, sep_style));
                title_spans.push(Span::styled("Favorites", inactive_style));
                title_spans.push(Span::styled(" (Tab) ", hint_style));
            }
            crate::tui::state::HomeDeckTab::Favorites => {
                title_spans.push(Span::styled(" Continue Watching", inactive_style));
                title_spans.push(Span::styled(" (Tab)", hint_style));
                title_spans.push(Span::styled(sep, sep_style));
                title_spans.push(Span::styled(" Favorites ", active_style));
            }
        }
    } else if has_cw {
        title_spans.push(Span::styled(" Continue Watching ", active_style));
    } else {
        title_spans.push(Span::styled(" Favorites ", active_style));
    }

    let block = Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .border_style(border_style);

    frame.render_widget(block, card_area);

    let inner_area = card_area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner_area.height == 0 || inner_area.width == 0 {
        return;
    }

    let mut list_state = if is_focused {
        state.favorites_landing_state
    } else {
        let mut s = ListState::default();
        s.select(None);
        s
    };

    let hl_sym = "";
    let hl_style = crate::tui::overlay::selection_style(theme, state.basic_terminal);
    let list_height = item_count as u16;
    let list_area = Rect {
        x: inner_area.x,
        y: inner_area.y,
        width: inner_area.width,
        height: list_height.min(inner_area.height),
    };

    let list_items: Vec<ListItem> = match tab {
        crate::tui::state::HomeDeckTab::ContinueWatching => cw_items
            .iter()
            .map(|item| {
                let episode_tag = if item.season > 0 {
                    format!("S{:02}E{:02}", item.season, item.episode)
                } else {
                    String::new()
                };

                let progress_str = if let Some(rem) = item.formatted_remaining() {
                    if let Some(pct) = item.progress_percentage() {
                        format!("{:.0}% · {rem}", pct)
                    } else {
                        rem
                    }
                } else if let Some(pct) = item.progress_percentage() {
                    format!("{:.0}%", pct)
                } else {
                    item.formatted_progress()
                };

                let right_tag = if !episode_tag.is_empty() {
                    format!("{episode_tag} · {progress_str}")
                } else if !item.release_year.is_empty() {
                    format!("{} · {progress_str}", item.release_year)
                } else {
                    progress_str
                };

                let left_margin = "  ";
                let right_margin = "  ";
                let tag_len = crate::tui::text::width(&right_tag);
                let hl_sym_len = 0;
                let margins_len = 4;
                let fixed_overhead = margins_len + tag_len;

                let max_title_width =
                    (inner_area.width as usize).saturating_sub(fixed_overhead + 1);
                let truncated_title =
                    crate::tui::text::truncate_width(&item.title, max_title_width);
                let title_width = crate::tui::text::width(&truncated_title);
                let pad_len = (inner_area.width as usize)
                    .saturating_sub(margins_len + hl_sym_len + title_width + tag_len);

                let line = Line::from(vec![
                    Span::raw(left_margin),
                    Span::styled(truncated_title, theme.text),
                    Span::raw(" ".repeat(pad_len)),
                    Span::styled(right_tag, theme.text_dim),
                    Span::raw(right_margin),
                ]);
                ListItem::new(line)
            })
            .collect(),
        crate::tui::state::HomeDeckTab::Favorites => fav_items
            .iter()
            .map(|item| {
                let type_tag = if item.stype == 2 { "Series" } else { "Movie" };
                let right_tag = if item.release_year.is_empty() {
                    type_tag.to_string()
                } else {
                    format!("{} {type_tag}", item.release_year)
                };

                let left_margin = "  ";
                let right_margin = "  ";
                let tag_len = crate::tui::text::width(&right_tag);
                let hl_sym_len = 0;
                let margins_len = 4;
                let fixed_overhead = margins_len + tag_len;

                let max_title_width =
                    (inner_area.width as usize).saturating_sub(fixed_overhead + 1);
                let truncated_title =
                    crate::tui::text::truncate_width(&item.title, max_title_width);
                let title_width = crate::tui::text::width(&truncated_title);
                let pad_len = (inner_area.width as usize)
                    .saturating_sub(margins_len + hl_sym_len + title_width + tag_len);

                let line = Line::from(vec![
                    Span::raw(left_margin),
                    Span::styled(truncated_title, theme.text),
                    Span::raw(" ".repeat(pad_len)),
                    Span::styled(right_tag, theme.text_dim),
                    Span::raw(right_margin),
                ]);
                ListItem::new(line)
            })
            .collect(),
    };

    let list = List::new(list_items)
        .highlight_symbol(hl_sym)
        .highlight_style(hl_style);
    frame.render_stateful_widget(list, list_area, &mut list_state);
    let curr_y = list_area.bottom();
    if overflow > 0 && curr_y < inner_area.bottom() {
        let sep = if state.basic_terminal { "-" } else { "·" };
        let cmd = match tab {
            crate::tui::state::HomeDeckTab::ContinueWatching => "/history",
            crate::tui::state::HomeDeckTab::Favorites => "/favorites",
        };
        let pill_text = format!("+{overflow} more {sep} {cmd}");
        let pill_style = if state.basic_terminal {
            theme.sapphire
        } else {
            let bg = theme.surface1.fg.unwrap_or(Color::Rgb(69, 71, 90));
            let fg = theme.sapphire.fg.unwrap_or(Color::Rgb(116, 199, 236));
            Style::default().bg(bg).fg(fg)
        };
        let pill_area = Rect {
            x: inner_area.x,
            y: curr_y,
            width: inner_area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(pill_text, pill_style)]))
                .alignment(Alignment::Center),
            pill_area,
        );
    }
}

pub(crate) fn render_discover_landing(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let presets = if state.active_provider == crate::providers::models::ProviderKind::Addons {
        [
            ("Top Movies", "Cinemeta Curated Catalog"),
            ("Top Series", "Popular & Episodic TV"),
            ("Addon Catalogs", "Installed Community Manifests"),
            ("Community Streams", "Multi-Source Aggregated Feeds"),
        ]
    } else {
        [
            ("Trending Now", "Popular & In-Theaters"),
            ("Top Rated Series", "Critically Acclaimed TV"),
            ("Latest Releases", "Recent 4K & HD Additions"),
            ("Most Watched", "Community Favorites"),
        ]
    };

    let card_width = search_deck_width(area, state, true);
    let row_count = presets.len() as u16;
    let content_height = (row_count + 2).min(area.height);

    let card_area = Rect {
        x: area.x + area.width.saturating_sub(card_width) / 2,
        y: area.y,
        width: card_width,
        height: content_height,
    };
    let modal_active = state.has_active_modal();
    let (bar, compass) = if state.basic_terminal {
        ("-", "*")
    } else {
        ("─", "✦")
    };
    let block = Block::default()
        .title(format!("{bar} {compass}  Discover Categories "))
        .title(
            Line::from(vec![Span::styled(
                "[ /browse ] ",
                if modal_active {
                    theme.muted
                } else {
                    theme.accent.add_modifier(Modifier::BOLD)
                },
            )])
            .alignment(Alignment::Right),
        )
        .title_style(if modal_active {
            theme.muted
        } else {
            theme.subtext1
        })
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .border_style(if modal_active {
            theme.muted
        } else {
            theme.surface1
        });

    frame.render_widget(block, card_area);

    let inner_area = card_area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner_area.height == 0 || inner_area.width == 0 {
        return;
    }

    for (curr_y, (title, desc)) in (inner_area.y..inner_area.bottom()).zip(presets) {
        let pointer = if state.basic_terminal { " - " } else { " · " };
        let pointer_w = crate::tui::text::width(pointer);
        let title_w = crate::tui::text::width(title);
        let margins_len = 2 + pointer_w + 1;
        let max_desc_w = (inner_area.width as usize).saturating_sub(title_w + margins_len + 1);
        let display_desc = if max_desc_w >= 4 && crate::tui::text::width(desc) > max_desc_w {
            crate::tui::text::truncate_width(desc, max_desc_w)
        } else if max_desc_w < 4 {
            std::borrow::Cow::Borrowed("")
        } else {
            std::borrow::Cow::Borrowed(desc)
        };
        let tag_len = crate::tui::text::width(&display_desc);
        let pad_len = (inner_area.width as usize).saturating_sub(margins_len + title_w + tag_len);

        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(pointer, theme.accent),
            Span::styled(title, theme.text.add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(pad_len)),
            Span::styled(display_desc, theme.text_dim),
            Span::raw(" "),
        ]);

        let row_area = Rect {
            x: inner_area.x,
            y: curr_y,
            width: inner_area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(line), row_area);
    }
}

pub(crate) fn dynamic_search_placeholder(state: &AppState) -> &'static str {
    if state.is_tv_mode {
        "Search live TV channels…"
    } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
        "Search via addons…"
    } else {
        "Search movies, series & anime…"
    }
}

#[cfg(test)]
fn search_content(state: &AppState, view: SearchViewState, width: u16) -> String {
    let prefix = if state.basic_terminal { "> " } else { "❯ " };
    let editing = view == SearchViewState::Editing;
    let available = width
        .saturating_sub(4)
        .saturating_sub(crate::tui::text::width(prefix) as u16) as usize;
    let has_status = state.status_timer > 0 && !state.status_message.is_empty();

    if state.search_query.is_empty() {
        let content = if has_status && !editing {
            crate::tui::text::truncate_width(&state.status_message, available)
        } else {
            std::borrow::Cow::Borrowed(dynamic_search_placeholder(state))
        };
        format!("{prefix}{content}")
    } else {
        let content = crate::tui::text::truncate_width(state.search_query.as_str(), available);
        format!("{prefix}{content}")
    }
}

fn render_search_bar(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    view: SearchViewState,
    landing: bool,
) {
    if landing {
        let modal_active = state.has_active_modal();
        let border_style = if modal_active {
            theme.muted
        } else if state.input_mode == InputMode::Editing {
            theme.border_focus
        } else {
            theme.surface1
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(crate::tui::overlay::border_type(state.basic_terminal))
            .border_style(border_style);
        frame.render_widget(block, area);

        let inner_width = area.width.saturating_sub(4);
        let inner_row_area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: inner_width,
            height: 1,
        };

        let is_ultra_compact = area.width < 58;
        let ctrl_p = if is_ultra_compact {
            "P"
        } else {
            crate::tui::text::CTRL_P_STR
        };
        let ctrl_t = if is_ultra_compact {
            "T"
        } else {
            crate::tui::text::CTRL_T_STR
        };

        let is_query_empty = state.search_query.is_empty();
        let (pill_text, pill_style) = if state.show_provider_popup {
            let label = state.active_provider.label();
            let text = if is_ultra_compact {
                format!("[{label}]")
            } else {
                let sep = if state.basic_terminal { "-" } else { "·" };
                format!("[{label} {sep} {ctrl_p}]")
            };
            let style = if state.basic_terminal {
                theme.sapphire.add_modifier(Modifier::BOLD)
            } else {
                let bg = theme.surface1.fg.unwrap_or(Color::Rgb(69, 71, 90));
                let fg = theme.sapphire.fg.unwrap_or(Color::Rgb(116, 199, 236));
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
            };
            (text, style)
        } else if modal_active {
            let text = if !is_query_empty {
                if is_ultra_compact {
                    "[Enter]".to_string()
                } else {
                    "[Enter] Search".to_string()
                }
            } else if state.is_tv_mode {
                if is_ultra_compact {
                    "[TV]".to_string()
                } else {
                    let sep = if state.basic_terminal { "-" } else { "·" };
                    format!("[Live TV {sep} {ctrl_t}]")
                }
            } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
                "[Addons]".to_string()
            } else {
                let label = state.active_provider.label();
                if is_ultra_compact {
                    format!("[{label}]")
                } else {
                    let sep = if state.basic_terminal { "-" } else { "·" };
                    format!("[{label} {sep} {ctrl_p}]")
                }
            };
            (text, theme.muted)
        } else if !is_query_empty {
            if is_ultra_compact {
                ("[Enter]".to_string(), theme.accent)
            } else if view == SearchViewState::NoResults {
                ("[0 results]".to_string(), theme.text_dim)
            } else {
                ("[Enter] Search".to_string(), theme.accent)
            }
        } else if state.is_tv_mode {
            let text = if is_ultra_compact {
                "[TV]".to_string()
            } else {
                let sep = if state.basic_terminal { "-" } else { "·" };
                format!("[Live TV {sep} {ctrl_t}]")
            };
            let style = if state.basic_terminal {
                theme.lavender
            } else {
                let bg = theme.surface1.fg.unwrap_or(Color::Rgb(69, 71, 90));
                let fg = theme.lavender.fg.unwrap_or(Color::Rgb(180, 190, 254));
                Style::default().bg(bg).fg(fg)
            };
            (text, style)
        } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
            let text = "[Addons]".to_string();
            let style = if state.basic_terminal {
                theme.teal
            } else {
                let bg = theme.surface1.fg.unwrap_or(Color::Rgb(69, 71, 90));
                let fg = theme.teal.fg.unwrap_or(Color::Rgb(148, 226, 213));
                Style::default().bg(bg).fg(fg)
            };
            (text, style)
        } else {
            let label = state.active_provider.label();
            let text = if is_ultra_compact {
                format!("[{label}]")
            } else {
                let sep = if state.basic_terminal { "-" } else { "·" };
                format!("[{label} {sep} {ctrl_p}]")
            };
            let style = if state.basic_terminal {
                theme.sapphire
            } else {
                let bg = theme.surface1.fg.unwrap_or(Color::Rgb(69, 71, 90));
                let fg = theme.sapphire.fg.unwrap_or(Color::Rgb(116, 199, 236));
                Style::default().bg(bg).fg(fg)
            };
            (text, style)
        };

        let pill_rect = search_bar_provider_pill_rect(area, state);
        let search_split = if is_query_empty {
            let left = Rect {
                x: inner_row_area.x,
                y: inner_row_area.y,
                width: inner_row_area.width.saturating_sub(pill_rect.width),
                height: inner_row_area.height,
            };
            vec![left, pill_rect]
        } else {
            let pill_width = crate::tui::text::width(&pill_text) as u16;
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(pill_width.saturating_add(1)),
                ])
                .split(inner_row_area)
                .to_vec()
        };

        let prefix = if state.basic_terminal { "> " } else { "❯ " };
        let prefix_width = crate::tui::text::width(prefix) as u16;
        let editing = view == SearchViewState::Editing;
        let real_cursor = editing && !state.basic_terminal && !modal_active;
        let prefix_style = if modal_active {
            theme.text_dim
        } else if editing {
            theme.accent
        } else {
            theme.text_dim
        };

        let has_status = state.status_timer > 0
            && !state.status_message.is_empty()
            && is_query_empty
            && !editing;
        let available_text_w =
            (search_split[0].width as usize).saturating_sub(prefix_width as usize + 2);
        let raw_placeholder = if has_status {
            state.status_message.as_str()
        } else {
            dynamic_search_placeholder(state)
        };
        let placeholder_text = crate::tui::text::truncate_width(raw_placeholder, available_text_w);
        let search_line = if is_query_empty {
            let placeholder_text: &str = placeholder_text.as_ref();
            if has_status {
                Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(placeholder_text, theme.accent),
                ])
            } else {
                Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(placeholder_text, theme.text_dim),
                ])
            }
        } else {
            let text_style = if modal_active {
                theme.text_dim
            } else {
                theme.text
            };
            Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(state.search_query.as_str(), text_style),
            ])
        };

        frame.render_widget(Paragraph::new(search_line), search_split[0]);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(pill_text, pill_style)]))
                .alignment(Alignment::Right),
            search_split[1],
        );

        if real_cursor {
            let cx = if is_query_empty {
                search_split[0].x + prefix_width
            } else {
                (search_split[0].x
                    + prefix_width
                    + state.search_query.cursor_column_offset() as u16)
                    .min(search_split[0].right().saturating_sub(1))
            };
            frame.set_cursor_position((cx, search_split[0].y));
        }
    } else {
        let result_status = if view == SearchViewState::Results && !state.search_results.is_empty()
        {
            let total = state.search_results.len();
            let selected_idx = state
                .search_list_state
                .selected()
                .unwrap_or(0)
                .min(total.saturating_sub(1));
            let selected_num = selected_idx + 1;
            let visible_items = state
                .last_result_metrics
                .map(|m| m.visible_items)
                .unwrap_or(8)
                .max(1);
            let total_pages = (total.saturating_sub(1) / visible_items) + 1;
            let page = (selected_idx / visible_items) + 1;

            if total_pages > 1 {
                if area.width < 58 {
                    Some(format!(
                        "{}/{} • p{}/{}",
                        selected_num, total, page, total_pages
                    ))
                } else {
                    Some(format!(
                        "Item {} of {} • Page {}/{}",
                        selected_num, total, page, total_pages
                    ))
                }
            } else if total == 1 {
                Some("1 result".to_string())
            } else if area.width < 45 {
                Some(format!("{}/{}", selected_num, total))
            } else {
                Some(format!("Item {} of {}", selected_num, total))
            }
        } else if view == SearchViewState::NoResults {
            Some("0 results".to_string())
        } else {
            None
        };

        let status_width = result_status
            .as_deref()
            .map(crate::tui::text::width)
            .unwrap_or(0) as u16;
        let content_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(status_width.saturating_add(u16::from(status_width > 0) * 2)),
            ])
            .split(area);

        let editing = view == SearchViewState::Editing;
        let real_cursor = editing && !state.basic_terminal;
        let has_status = state.status_timer > 0
            && !state.status_message.is_empty()
            && state.search_query.is_empty()
            && !editing;

        let prefix = if state.basic_terminal { "> " } else { "❯ " };
        let prefix_width = crate::tui::text::width(prefix) as u16;

        let search_line = if state.search_query.is_empty() {
            let placeholder_text = if has_status {
                state.status_message.as_str()
            } else {
                dynamic_search_placeholder(state)
            };

            if has_status {
                Line::from(vec![
                    Span::styled(prefix, theme.accent),
                    Span::styled(placeholder_text, theme.accent),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        prefix,
                        if editing {
                            theme.accent
                        } else {
                            theme.text_dim
                        },
                    ),
                    Span::styled(placeholder_text, theme.text_dim),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled(prefix, if editing { theme.accent } else { theme.text }),
                Span::styled(state.search_query.as_str(), theme.text),
            ])
        };

        frame.render_widget(Paragraph::new(search_line), content_row[0]);

        if real_cursor {
            let (cursor_x, cursor_y) = if state.search_query.is_empty() {
                let cx =
                    (content_row[0].x + prefix_width).min(content_row[0].right().saturating_sub(1));
                (cx, content_row[0].y)
            } else {
                let cx = (content_row[0].x
                    + prefix_width
                    + state.search_query.cursor_column_offset() as u16)
                    .min(content_row[0].right().saturating_sub(1));
                (cx, content_row[0].y)
            };
            frame.set_cursor_position((cursor_x, cursor_y));
        }

        if let Some(status) = result_status {
            frame.render_widget(
                Paragraph::new(status)
                    .style(theme.accent)
                    .alignment(Alignment::Right),
                content_row[1],
            );
        }
    }
}

fn home_bottom_bar_spans(
    state: &AppState,
    theme: &Theme,
    width: u16,
    modal_active: bool,
) -> Vec<Span<'static>> {
    let compact_tabs = width < 76;
    let ultra_compact_tabs = width < 58;

    let ctrl_s = if ultra_compact_tabs || compact_tabs {
        "S"
    } else {
        crate::tui::text::CTRL_S_STR
    };
    let ctrl_t = if ultra_compact_tabs || compact_tabs {
        "T"
    } else {
        crate::tui::text::CTRL_T_STR
    };

    let current_mode = state.mode();
    let mut bar_spans: Vec<Span<'static>> = Vec::new();
    let shortcut_style = if modal_active {
        theme.muted
    } else {
        theme.shortcut
    };
    let bracket_style = if modal_active {
        theme.muted
    } else {
        theme.text_dim
    };
    let text_style = if modal_active {
        theme.muted
    } else {
        theme.text_dim
    };

    let sep = if state.basic_terminal {
        " - "
    } else if compact_tabs {
        " · "
    } else {
        "  ·  "
    };

    if state.streaming_enabled && current_mode != crate::tui::state::AppMode::Streaming {
        bar_spans.push(Span::styled("[", bracket_style));
        bar_spans.push(Span::styled(ctrl_s, shortcut_style));
        bar_spans.push(Span::styled("]", bracket_style));
        if !compact_tabs {
            bar_spans.push(Span::styled(" Stream", text_style));
        }
    }

    if state.tv_enabled && current_mode != crate::tui::state::AppMode::Tv {
        if !bar_spans.is_empty() {
            bar_spans.push(Span::raw(sep));
        }
        bar_spans.push(Span::styled("[", bracket_style));
        bar_spans.push(Span::styled(ctrl_t, shortcut_style));
        bar_spans.push(Span::styled("]", bracket_style));
        if !compact_tabs {
            bar_spans.push(Span::styled(" TV", text_style));
        }
    }

    if !bar_spans.is_empty() {
        let util_gap = if compact_tabs { "    " } else { "       " };
        bar_spans.push(Span::raw(util_gap));
    }

    bar_spans.push(Span::styled("[", bracket_style));
    bar_spans.push(Span::styled("?", shortcut_style));
    bar_spans.push(Span::styled("]", bracket_style));
    if !compact_tabs {
        bar_spans.push(Span::styled(" Help", text_style));
        bar_spans.push(Span::raw("  "));
    } else if !ultra_compact_tabs {
        bar_spans.push(Span::raw(" "));
    }
    bar_spans.push(Span::styled("[", bracket_style));
    bar_spans.push(Span::styled("q", shortcut_style));
    bar_spans.push(Span::styled("]", bracket_style));
    if !compact_tabs {
        bar_spans.push(Span::styled(" Quit", text_style));
    }
    bar_spans
}

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let view = search_view_state(state);
    let search_bar_area;

    if view == SearchViewState::Empty
        || (view == SearchViewState::Editing && state.search_results.is_empty())
    {
        let basic_terminal = state.basic_terminal;
        let (tier, rows) = landing_split(
            area,
            state.is_tv_mode,
            basic_terminal,
            state.landing_deck_visible(),
        );
        let vertical_chunks = rows.rects.clone();

        let logo_text = if basic_terminal || tier.is_compact() {
            if state.is_tv_mode {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █TV".to_string()
            } else {
                "█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀\n█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █".to_string()
            }
        } else if state.is_tv_mode {
            r"███╗   ███╗  ██████╗  ██╗   ██╗ ██╗ ███████╗ ██████╗   ██████╗  ██╗  ██╗
████╗ ████║ ██╔═══██╗ ██║   ██║ ██║ ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗██╔╝
██╔████╔██║ ██║   ██║ ██║   ██║ ██║ █████╗   ██████╔╝ ██║   ██║  ╚███╔╝ 
██║╚██╔╝██║ ██║   ██║ ╚██╗ ██╔╝ ██║ ██╔══╝   ██╔══██╗ ██║   ██║  ██╔██╗ TV
██║ ╚═╝ ██║ ╚██████╔╝  ╚████╔╝  ██║ ███████╗ ██████╔╝ ╚██████╔╝ ██╔╝ ██╗
╚═╝     ╚═╝  ╚═════╝    ╚═══╝   ╚═╝ ╚══════╝ ╚═════╝   ╚═════╝  ╚═╝  ╚═╝"
                .to_string()
        } else {
            r"███╗   ███╗  ██████╗  ██╗   ██╗ ██╗ ███████╗ ██████╗   ██████╗  ██╗  ██╗
████╗ ████║ ██╔═══██╗ ██║   ██║ ██║ ██╔════╝ ██╔══██╗ ██╔═══██╗ ╚██╗██╔╝
██╔████╔██║ ██║   ██║ ██║   ██║ ██║ █████╗   ██████╔╝ ██║   ██║  ╚███╔╝ 
██║╚██╔╝██║ ██║   ██║ ╚██╗ ██╔╝ ██║ ██╔══╝   ██╔══██╗ ██║   ██║  ██╔██╗ 
██║ ╚═╝ ██║ ╚██████╔╝  ╚████╔╝  ██║ ███████╗ ██████╔╝ ╚██████╔╝ ██╔╝ ██╗
╚═╝     ╚═╝  ╚═════╝    ╚═══╝   ╚═╝ ╚══════╝ ╚═════╝   ╚═════╝  ╚═╝  ╚═╝"
                .to_string()
        };

        let logo_width: u16 = rows.logo_width;

        let pad = area.width.saturating_sub(logo_width) / 2;
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[rows.logo]);

        let version_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[rows.version]);

        let modal_active = state.has_active_modal() || state.show_settings_popup;
        let update_active = (state.update_available.is_some()
            && state.input_mode != InputMode::Editing)
            || state.is_updating;
        if !update_active {
            let logo_style = if modal_active {
                theme.overlay0
            } else {
                theme.title
            };
            let title_art = Paragraph::new(logo_text)
                .alignment(Alignment::Left)
                .style(logo_style);
            frame.render_widget(title_art, horizontal_chunks[1]);

            let version_style = if modal_active {
                theme.muted
            } else {
                theme.text_dim
            };
            let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                .alignment(Alignment::Right)
                .style(version_style);
            frame.render_widget(version, version_chunks[1]);
        }

        let card_width = search_deck_width(area, state, true);
        let card_x = area.x + area.width.saturating_sub(card_width) / 2;
        let search_card_area = Rect {
            x: card_x,
            y: vertical_chunks[rows.search].y,
            width: card_width,
            height: vertical_chunks[rows.search].height,
        };

        if !state.tv_config_popup && !update_active && !state.show_settings_popup {
            render_search_bar(frame, search_card_area, state, theme, view, true);
        }
        search_bar_area = search_card_area;
        let suggestions_open =
            state.input_mode == InputMode::Editing && !state.search_suggestions.is_empty();

        if state.landing_deck_visible()
            && !state.tv_config_popup
            && !update_active
            && !state.show_settings_popup
        {
            render_landing_deck(frame, vertical_chunks[rows.favorites], state, theme);
        } else if !state.is_tv_mode
            && !state.tv_config_popup
            && !suggestions_open
            && !update_active
            && !state.show_settings_popup
            && area.height >= 26
        {
            render_discover_landing(frame, vertical_chunks[rows.favorites], state, theme);
        }
        let bar_spans = home_bottom_bar_spans(state, theme, area.width, modal_active);
        let bottom_bar_area = vertical_chunks[rows.mode_row];
        frame.render_widget(
            Paragraph::new(Line::from(bar_spans)).alignment(Alignment::Center),
            bottom_bar_area,
        );
    } else {
        if state.is_loading && state.search_results.is_empty() {
            render_search_state(frame, area, state, theme, SearchViewState::Loading);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        let results_chunk = chunks[2];
        search_bar_area = Rect {
            x: chunks[0].x + 2,
            width: chunks[0].width.saturating_sub(4),
            ..chunks[0]
        };
        render_search_bar(frame, search_bar_area, state, theme, view, false);
        let list_block = Block::default();
        if !state.search_results.is_empty() {
            let initial_metrics = state.result_metrics(results_chunk.height, results_chunk.width);
            let has_scrollbar = state.search_results.len() > initial_metrics.visible_items;
            let results_area = if has_scrollbar {
                Rect {
                    width: results_chunk.width.saturating_sub(1),
                    ..results_chunk
                }
            } else {
                results_chunk
            };
            crate::tui::clear_area(frame, results_chunk, theme);
            let selected_idx = state.search_list_state.selected();

            let metrics = state.result_metrics(results_area.height, results_area.width);
            let target_width = if state.image_supported {
                state
                    .image_picker
                    .as_ref()
                    .map(|picker| {
                        let font = picker.font_size();
                        let pixel_height =
                            u64::from(state.poster_rows.max(3)) * u64::from(font.height.max(1));
                        let pixel_width = pixel_height * 2 / 3;
                        u16::try_from(pixel_width.div_ceil(u64::from(font.width.max(1))))
                            .unwrap_or(u16::MAX)
                            .max(6)
                    })
                    .unwrap_or_else(|| state.poster_rows.saturating_mul(4).div_ceil(3).max(6))
            } else {
                state.poster_rows.saturating_mul(4).div_ceil(3).clamp(8, 12)
            };
            let poster_width = target_width
                .min(metrics.col_width.saturating_sub(18).max(6))
                .max(6);
            state.last_result_metrics = Some(metrics);
            let row_height = metrics.row_height;
            frame.render_widget(list_block, results_area);

            let inner_area = results_area;
            let is_editing = state.input_mode == InputMode::Editing;
            let modal_active = state.has_active_modal();

            for slot in 0..metrics.visible_items {
                let i = state.result_scroll + slot;
                let Some(res) = state.search_results.get(i) else {
                    break;
                };
                let visible_index = slot / metrics.columns as usize;
                let column = (slot % metrics.columns as usize) as u16;
                let current_y =
                    inner_area.y + (visible_index as u16 * row_height).min(inner_area.height);

                let gutter = 1_u16;
                let col_x = inner_area.x + column * (metrics.col_width + gutter);
                let col_w = if column + 1 == metrics.columns {
                    inner_area
                        .width
                        .saturating_sub(column * (metrics.col_width + gutter))
                } else {
                    metrics.col_width
                };
                let item_area = Rect {
                    x: col_x,
                    y: current_y,
                    width: col_w,
                    height: metrics.poster_rows_eff,
                };

                let (poster_area, text_area) = item_slot_rects(item_area, poster_width);

                let is_selected = Some(i) == selected_idx;

                let img_height = poster_area.height.min(state.poster_rows);
                let img_y_offset = item_area.height.saturating_sub(img_height) / 2;
                let p_area = Rect {
                    y: poster_area.y + img_y_offset,
                    height: img_height,
                    ..poster_area
                };
                if state.image_supported && poster_area.width > 0 {
                    if let Some(img) = state.search_posters.peek(&res.id) {
                        let target_dims = (poster_area.width, state.poster_rows);
                        let needs_protocol =
                            state.search_poster_protocols.peek(&res.id).map(|(d, _)| *d)
                                != Some(target_dims);
                        if needs_protocol {
                            if let Some(picker) = &mut state.image_picker {
                                let size = ratatui::layout::Size::new(target_dims.0, target_dims.1);
                                if let Ok(proto) = picker.new_protocol(
                                    (**img).clone(),
                                    size,
                                    ratatui_image::Resize::Fit(None),
                                ) {
                                    state
                                        .search_poster_protocols
                                        .put(res.id.clone(), (target_dims, proto));
                                }
                            }
                        }
                        if let Some((_, proto)) = state.search_poster_protocols.peek(&res.id) {
                            if !state.has_active_modal() {
                                frame.render_widget(ratatui_image::Image::new(proto), p_area);
                            }
                        }
                    } else {
                        let is_in_flight = state.in_flight_posters.contains(&res.id);
                        render_poster_placeholder(
                            frame,
                            p_area,
                            theme,
                            state.basic_terminal,
                            is_in_flight,
                            state.tick_count,
                        );
                    }
                } else if poster_area.width > 0 {
                    render_poster_placeholder(
                        frame,
                        p_area,
                        theme,
                        state.basic_terminal,
                        false,
                        state.tick_count,
                    );
                }

                let text_height = text_area.height;
                let (text_top_padding, use_three_rows) = if text_height >= 4 {
                    ((text_height.saturating_sub(3)) / 2, true)
                } else if text_height == 3 {
                    (0, true)
                } else {
                    ((text_height.saturating_sub(2)) / 2, false)
                };

                let text_layout = if use_three_rows {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(text_top_padding),
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(text_area)
                } else {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(text_top_padding),
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(text_area)
                };

                let is_active_selection = is_selected && !is_editing && !modal_active;
                let title_style = if modal_active {
                    theme.muted
                } else if is_active_selection {
                    crate::tui::overlay::selection_style(theme, state.basic_terminal)
                } else {
                    theme.text
                };
                let is_favorited = res.stype != 3
                    && state
                        .favorites
                        .is_favorite(&crate::models::SubjectIdentity {
                            provider: res.provider.cache_key(),
                            subject_id: &res.id,
                            title: &res.title,
                            stype: res.stype,
                            release_year: &res.release_year,
                        });

                let mut type_tag = if state.is_tv_mode || res.stype == 3 {
                    "TV Channel".to_string()
                } else if res.stype == 1 {
                    "Movie".to_string()
                } else if res.stype == 2 {
                    "Series".to_string()
                } else {
                    "".to_string()
                };

                let is_history = state.search_query.trim().to_lowercase() == "/history";
                if !is_history && type_tag.is_empty() {
                    type_tag = "Unknown".to_string();
                }

                let res_badge = if !is_history {
                    crate::tui::widgets::badge::extract_resolution(&res.title, None)
                } else {
                    None
                };
                let badge_spans = res_badge.map(|r| {
                    crate::tui::widgets::badge::resolution_badge_spans(
                        r,
                        theme,
                        state.basic_terminal,
                        modal_active,
                    )
                });
                let badge_width = badge_spans.as_ref().map_or(0, |spans| {
                    spans
                        .iter()
                        .map(|s| crate::tui::text::width(s.content.as_ref()))
                        .sum::<usize>()
                });

                let title_reserved = if is_favorited { 3 } else { 1 }
                    + badge_width
                    + if badge_width > 0 { 1 } else { 0 };
                let max_title_width = (text_area.width as usize)
                    .saturating_sub(title_reserved)
                    .max(4);
                let display_title = crate::tui::text::truncate_width(&res.title, max_title_width);

                let mut row1_spans = Vec::new();
                if is_active_selection {
                    row1_spans.push(ratatui::text::Span::styled(" ", title_style));
                } else {
                    row1_spans.push(ratatui::text::Span::raw(" "));
                }
                if is_favorited {
                    row1_spans.push(ratatui::text::Span::styled(
                        if state.basic_terminal { "* " } else { "★ " },
                        if modal_active {
                            theme.muted
                        } else if is_active_selection {
                            title_style
                        } else {
                            theme.rating
                        },
                    ));
                }
                row1_spans.push(ratatui::text::Span::styled(
                    display_title.as_ref(),
                    title_style,
                ));
                if is_active_selection {
                    row1_spans.push(ratatui::text::Span::styled(" ", title_style));
                }

                if let Some(b_spans) = badge_spans {
                    let title_actual_w = 1
                        + if is_favorited { 2 } else { 0 }
                        + crate::tui::text::width(&display_title)
                        + if is_active_selection { 1 } else { 0 };
                    let gap =
                        (text_area.width as usize).saturating_sub(title_actual_w + badge_width);
                    if gap > 0 {
                        row1_spans.push(ratatui::text::Span::raw(" ".repeat(gap)));
                    }
                    row1_spans.extend(b_spans);
                }

                if text_layout[1].height > 0 {
                    frame.render_widget(
                        Paragraph::new(ratatui::text::Line::from(row1_spans)),
                        text_layout[1],
                    );
                }

                let mut row2_spans = vec![ratatui::text::Span::raw(" ")];
                let mut row3_spans = vec![ratatui::text::Span::raw(" ")];

                if is_history {
                    if !type_tag.is_empty() {
                        row2_spans.push(ratatui::text::Span::styled(&type_tag, theme.text));
                        row2_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                    }
                    if res.season > 0 {
                        row2_spans.push(ratatui::text::Span::styled(
                            format!("S{:02}E{:02}", res.season, res.episode),
                            theme.text,
                        ));
                        row2_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                    }

                    if let Some(hist) = state.history.get_item(
                        res.provider.cache_key(),
                        &res.id,
                        res.season,
                        res.episode,
                        Some(&res.title),
                    ) {
                        if hist.is_in_progress() {
                            let (filled, empty) = hist.progress_bar_parts(8);
                            let mut progress_spans = vec![
                                ratatui::text::Span::styled(
                                    filled,
                                    theme.accent.add_modifier(ratatui::style::Modifier::BOLD),
                                ),
                                ratatui::text::Span::styled(empty, theme.text_dim),
                            ];
                            let pct = hist
                                .progress_percentage()
                                .map(|p| format!(" {:.0}%", p))
                                .unwrap_or_default();
                            progress_spans.push(ratatui::text::Span::styled(pct, theme.text));

                            if let Some(r) = hist.formatted_remaining() {
                                progress_spans.push(ratatui::text::Span::styled(
                                    format!(" ({r})"),
                                    theme.text_dim,
                                ));
                            }

                            if use_three_rows {
                                row3_spans.extend(progress_spans);
                                row3_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                                row3_spans.push(ratatui::text::Span::styled(
                                    format!("Watched {}", hist.formatted_relative_time()),
                                    theme.text_dim,
                                ));
                            } else {
                                row2_spans.extend(progress_spans);
                                row2_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            }
                        } else if hist.completed {
                            let comp_span = ratatui::text::Span::styled(
                                if state.basic_terminal {
                                    "[Completed]"
                                } else {
                                    "[✓ Completed]"
                                },
                                theme.text_dim,
                            );
                            if use_three_rows {
                                row3_spans.push(comp_span);
                                row3_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                                row3_spans.push(ratatui::text::Span::styled(
                                    format!("Watched {}", hist.formatted_relative_time()),
                                    theme.text_dim,
                                ));
                            } else {
                                row2_spans.push(comp_span);
                                row2_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            }
                        }
                    }
                    row2_spans.push(crate::tui::widgets::badge::provider_badge_span(
                        res.provider,
                        theme,
                        state.basic_terminal,
                        modal_active,
                    ));
                } else {
                    let matching_meta = state
                        .search_preview
                        .as_ref()
                        .filter(|m| m.id.value == res.id && m.id.provider == res.provider);
                    if let Some(meta) = matching_meta {
                        if is_selected {
                            if let Some(r) = meta.imdb_rating.as_deref() {
                                let star = if state.basic_terminal { "* " } else { "★ " };
                                row2_spans.push(ratatui::text::Span::styled(star, theme.rating));
                                row2_spans.push(ratatui::text::Span::styled(r, theme.text));
                                row2_spans.push(ratatui::text::Span::styled(" • ", theme.text_dim));
                            }

                            let genre_buf = meta.genres.join(", ");
                            if !genre_buf.is_empty() {
                                let g_trunc = crate::tui::text::truncate_width(
                                    &genre_buf,
                                    text_area.width.saturating_sub(2) as usize,
                                )
                                .into_owned();
                                row3_spans
                                    .push(ratatui::text::Span::styled(g_trunc, theme.subtext1));
                            }
                        }
                    } else if is_selected && state.preview_loading {
                        let dots = match (state.tick_count / 4) % 4 {
                            0 => "",
                            1 => ".",
                            2 => "..",
                            _ => "...",
                        };
                        row3_spans.push(ratatui::text::Span::styled(
                            format!("Loading{dots}"),
                            theme.text_dim,
                        ));
                    }
                    let has_year = res.release_year != "Unknown" && !res.release_year.is_empty();
                    if has_year {
                        row2_spans.push(ratatui::text::Span::styled(
                            &res.release_year,
                            if modal_active {
                                theme.muted
                            } else {
                                theme.text
                            },
                        ));
                        row2_spans.push(ratatui::text::Span::styled(
                            " • ",
                            if modal_active {
                                theme.muted
                            } else {
                                theme.text_dim
                            },
                        ));
                    }

                    if text_area.width >= 36 || !has_year {
                        row2_spans.push(ratatui::text::Span::styled(
                            &type_tag,
                            if modal_active {
                                theme.muted
                            } else {
                                theme.text
                            },
                        ));
                        row2_spans.push(ratatui::text::Span::styled(
                            " • ",
                            if modal_active {
                                theme.muted
                            } else {
                                theme.text_dim
                            },
                        ));
                    }
                    row2_spans.push(crate::tui::widgets::badge::provider_badge_span(
                        res.provider,
                        theme,
                        state.basic_terminal,
                        modal_active,
                    ));
                }

                if text_layout[2].height > 0 && row2_spans.len() > 1 {
                    frame.render_widget(
                        Paragraph::new(ratatui::text::Line::from(row2_spans)),
                        text_layout[2],
                    );
                }

                if use_three_rows
                    && text_layout.len() > 3
                    && text_layout[3].height > 0
                    && row3_spans.len() > 1
                {
                    frame.render_widget(
                        Paragraph::new(ratatui::text::Line::from(row3_spans)),
                        text_layout[3],
                    );
                }
            }

            crate::tui::widgets::render_scrollbar(
                frame,
                results_chunk,
                state.search_results.len(),
                metrics.visible_items,
                state.result_scroll,
                theme,
                state.basic_terminal,
            );
        } else {
            render_search_state(frame, results_chunk, state, theme, view);
        }
    }

    if !state.tv_config_popup && !state.show_settings_popup {
        render_search_suggestions(frame, area, search_bar_area, state, theme, view);
    }
    render_provider_popup(frame, area, search_bar_area, state, theme);
    if state.tv_config_popup {
        let rows = state.tv_manager_rows();
        let total_rows = rows.len();
        let longest_source_width = state
            .tv_playlists
            .iter()
            .map(|source| crate::tui::text::width(source))
            .max()
            .unwrap_or(28);
        let popup_area = crate::tui::overlay::tv_config_layout(
            area,
            longest_source_width,
            total_rows,
            state.tv_input_active,
        );
        let title = format!(
            "TV Playlists · {}/{}",
            state.tv_manager_selected.saturating_add(1),
            total_rows.max(1)
        );
        let inner_area = crate::tui::widgets::ModalFrame::new(&title, theme, state.basic_terminal)
            .render(frame, popup_area, area);

        if state.tv_input_active {
            let label = if state.tv_input_is_file {
                "Enter playlist file path:"
            } else {
                "Enter playlist URL:"
            };
            crate::tui::widgets::render_single_line_input(
                frame,
                inner_area,
                label,
                &state.tv_input_buffer,
                theme,
                state.basic_terminal,
            );
        } else {
            let items: Vec<ratatui::widgets::ListItem> = rows
                .iter()
                .map(|row| {
                    use crate::tui::state::TvManagerRow;
                    match row {
                        TvManagerRow::Header(label) => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled(label.to_string(), theme.muted),
                            ]))
                        }
                        TvManagerRow::Playlist(index) => {
                            let source =
                                state.tv_playlists.get(*index).cloned().unwrap_or_default();
                            let index_str = format!("{}", index + 1);
                            let url_budget = (inner_area.width as usize)
                                .saturating_sub(index_str.len() + 6)
                                .max(10);
                            let display_source =
                                crate::tui::text::truncate_middle_width(&source, url_budget);
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled(
                                    format!("{} {}", index_str, display_source),
                                    theme.text,
                                ),
                            ]))
                        }
                        TvManagerRow::AddUrl => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Add URL ]", theme.sapphire),
                            ]))
                        }
                        TvManagerRow::AddFile => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Add file ]", theme.sapphire),
                            ]))
                        }
                        TvManagerRow::Reload => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Reload ]", theme.rating),
                            ]))
                        }
                        TvManagerRow::Done => {
                            ratatui::widgets::ListItem::new(ratatui::text::Line::from(vec![
                                ratatui::text::Span::raw(" "),
                                ratatui::text::Span::styled("[ Done ]", theme.success),
                            ]))
                        }
                    }
                })
                .collect();

            let list = ratatui::widgets::List::new(items)
                .highlight_style(crate::tui::overlay::selection_style(
                    theme,
                    state.basic_terminal,
                ))
                .highlight_symbol("");

            let mut list_state = ratatui::widgets::ListState::default();
            list_state.select(Some(state.tv_manager_selected));
            frame.render_stateful_widget(list, inner_area, &mut list_state);
        }
    }

    if state.addon_manager_popup {
        let addons_count = state.installed_addons.len();
        let total_rows = state.addon_manager_rows().len();
        let popup_area =
            crate::tui::overlay::addon_manager_layout(area, addons_count, state.addon_input_active);
        let title = format!(
            "Addons Manager · {}/{}",
            state.addon_manager_selected.saturating_add(1),
            total_rows.max(1)
        );
        let inner_area = crate::tui::widgets::ModalFrame::new(&title, theme, state.basic_terminal)
            .render(frame, popup_area, area);

        if state.addon_input_active {
            crate::tui::widgets::render_single_line_input(
                frame,
                inner_area,
                "Enter Addon Manifest URL:",
                &state.addon_input_buffer,
                theme,
                state.basic_terminal,
            );
        } else {
            let sections = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Length(2),
            ])
            .split(inner_area);

            let is_add_selected = state.addon_manager_selected == state.installed_addons.len() + 1;

            let mut items = vec![ratatui::widgets::ListItem::new(ratatui::text::Line::from(
                vec![
                    ratatui::text::Span::raw("   "),
                    ratatui::text::Span::styled("Installed Addons", theme.muted),
                ],
            ))];

            for (idx, a) in state.installed_addons.iter().enumerate() {
                let row_idx = idx + 1;
                let is_selected = state.addon_manager_selected == row_idx;
                let prefix = ratatui::text::Span::raw("  ");
                let check = if a.enabled {
                    ratatui::text::Span::styled("[x] ", theme.success)
                } else {
                    ratatui::text::Span::styled("[ ] ", theme.text_dim)
                };
                let mut badges = Vec::new();
                let mut badges_w = 0;
                if a.is_core() {
                    badges.push(ratatui::text::Span::styled("[Core] ", theme.lavender));
                    badges_w += 7;
                }
                if a.provides_meta {
                    badges.push(ratatui::text::Span::styled("[Meta] ", theme.sapphire));
                    badges_w += 7;
                }
                if a.provides_stream {
                    badges.push(ratatui::text::Span::styled("[Streams] ", theme.rating));
                    badges_w += 10;
                }
                if a.provides_catalog {
                    badges.push(ratatui::text::Span::styled("[Catalog]", theme.teal));
                    badges_w += 9;
                }
                let prefix_w = 2;
                let check_w = 4;
                let name_budget = (inner_area.width as usize)
                    .saturating_sub(prefix_w + check_w + badges_w + 2)
                    .max(8);
                let addon_label = format!("{} v{} ", a.name, a.version.as_deref().unwrap_or("1.0"));
                let name = ratatui::text::Span::styled(
                    crate::tui::text::truncate_width(&addon_label, name_budget).into_owned(),
                    if is_selected {
                        theme.text.add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        theme.text
                    },
                );
                let mut spans = vec![ratatui::text::Span::raw(" "), prefix, check, name];
                spans.extend(badges);
                let item = ratatui::widgets::ListItem::new(ratatui::text::Line::from(spans));
                let item = if is_selected {
                    item.style(crate::tui::overlay::selection_style(
                        theme,
                        state.basic_terminal,
                    ))
                } else {
                    item
                };
                items.push(item);
            }

            let list = ratatui::widgets::List::new(items);
            frame.render_widget(list, sections[0]);

            let add_button = if is_add_selected {
                ratatui::text::Span::styled(
                    "  [ Add Manifest URL ]",
                    crate::tui::overlay::selection_style(theme, state.basic_terminal),
                )
            } else {
                ratatui::text::Span::styled("  [ Add Manifest URL ]", theme.sapphire)
            };

            let button_line =
                ratatui::text::Line::from(vec![ratatui::text::Span::raw(" "), add_button]);
            frame.render_widget(ratatui::widgets::Paragraph::new(button_line), sections[1]);
        }
    }

    if state.show_browse_popup {
        let raw_labels: Vec<String> =
            if state.active_provider == crate::providers::models::ProviderKind::Addons {
                crate::providers::addons::models::curated_catalog_presets(&state.installed_addons)
                    .into_iter()
                    .map(|target| target.label)
                    .collect()
            } else {
                crate::tui::state::BrowsePreset::ALL
                    .iter()
                    .map(|preset| preset.label().to_string())
                    .collect()
            };

        let raw_items: Vec<String> = raw_labels
            .iter()
            .map(|label| {
                let (_, spacing) = crate::tui::overlay::browse_category_badge(label, theme);
                let badge_str = if label.to_ascii_lowercase().contains("movie")
                    || label.to_ascii_lowercase().contains("top rated (all-time)")
                    || label.to_ascii_lowercase().contains("top rated (recent")
                {
                    "[MOVIES]"
                } else if label.to_ascii_lowercase().contains("series")
                    || label.to_ascii_lowercase().contains("airing")
                    || label.to_ascii_lowercase().contains("show")
                    || label.to_ascii_lowercase().contains("tv")
                {
                    "[SERIES]"
                } else {
                    "[DISCOVER]"
                };
                format!("{badge_str}{spacing}{label}")
            })
            .collect();

        let lines: Vec<Line> = raw_labels
            .iter()
            .map(|label| {
                let (badge, spacing) = crate::tui::overlay::browse_category_badge(label, theme);
                Line::from(vec![
                    badge,
                    Span::raw(spacing),
                    Span::styled(label.to_string(), theme.text),
                ])
            })
            .collect();

        crate::tui::overlay::picker_with_lines(
            frame,
            area,
            &lines,
            &raw_items,
            &mut state.browse_list_state,
            crate::tui::overlay::PickerSpec {
                title: "Browse",
                confirm_label: "Open",
                minimum_width: 36,
                show_counter: true,
            },
            theme,
            state.basic_terminal,
        );
    }
}
fn suggestion_source_badge<'a>(
    suggestion: &str,
    state: &AppState,
    theme: &'a Theme,
    basic_terminal: bool,
) -> Option<Span<'a>> {
    let (tag, style) = if suggestion.starts_with('/') {
        if suggestion.eq_ignore_ascii_case("/history") {
            ("[HISTORY]", theme.sapphire)
        } else if suggestion.eq_ignore_ascii_case("/favorites") {
            ("[FAVORITES]", theme.rating)
        } else if suggestion.eq_ignore_ascii_case("/settings")
            || suggestion.eq_ignore_ascii_case("/config")
            || suggestion.eq_ignore_ascii_case("/preferences")
            || suggestion.eq_ignore_ascii_case("/pref")
            || suggestion.eq_ignore_ascii_case("/options")
        {
            ("[SETTINGS]", theme.teal)
        } else {
            ("[CMD]", theme.teal)
        }
    } else if state.is_tv_mode {
        ("[TV]", theme.lavender)
    } else {
        return None;
    };

    if basic_terminal {
        Some(Span::styled(
            format!("{tag} "),
            style.add_modifier(Modifier::BOLD),
        ))
    } else {
        Some(Span::styled(format!("{tag} "), style))
    }
}

pub fn search_suggestions_bounds(area: Rect, search_bar_area: Rect, count: usize) -> (Rect, Rect) {
    if count == 0 || search_bar_area.width == 0 || area.width == 0 || area.height == 0 {
        return (Rect::default(), Rect::default());
    }

    let start_y = search_bar_area.bottom();
    let max_h = area.bottom().saturating_sub(start_y);
    let container_h = ((count as u16).saturating_add(2)).min(max_h);

    let container_w = search_bar_area
        .width
        .min(area.width.saturating_sub(2))
        .max(24);
    let x = search_bar_area.x;

    let container_area = Rect {
        x,
        y: start_y,
        width: container_w,
        height: container_h,
    };

    let inner_area = Rect {
        x: container_area.x.saturating_add(1),
        y: container_area.y.saturating_add(1),
        width: container_area.width.saturating_sub(2),
        height: container_area.height.saturating_sub(2),
    };

    (container_area, inner_area)
}

fn render_search_suggestions(
    frame: &mut Frame,
    area: Rect,
    search_bar_area: Rect,
    state: &AppState,
    theme: &Theme,
    _view: SearchViewState,
) {
    if state.input_mode != InputMode::Editing
        || state.search_suggestions.is_empty()
        || search_bar_area.width == 0
    {
        return;
    }

    let visible_count = state.search_suggestions.len().min(6);
    let selected_index = state.suggest_index.unwrap_or(0);
    let suggestion_offset = selected_index
        .saturating_add(1)
        .saturating_sub(visible_count)
        .min(state.search_suggestions.len().saturating_sub(visible_count));

    let visible_slice: Vec<(usize, &String)> = state
        .search_suggestions
        .iter()
        .enumerate()
        .skip(suggestion_offset)
        .take(visible_count)
        .collect();

    if visible_slice.is_empty() {
        return;
    }

    let (container_area, inner_area) =
        search_suggestions_bounds(area, search_bar_area, visible_slice.len());

    if container_area.width == 0 || container_area.height <= 2 || inner_area.height == 0 {
        return;
    }

    crate::tui::clear_area(frame, container_area, theme);
    let remaining_below = state
        .search_suggestions
        .len()
        .saturating_sub(suggestion_offset + visible_count);
    let mut container_block = Block::default()
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .border_style(theme.border_focus);

    if remaining_below > 0 {
        let hint_text = if state.basic_terminal {
            format!(" v (+{} more) ", remaining_below)
        } else {
            format!(" ▼ (+{} more) ", remaining_below)
        };
        container_block = container_block.title_bottom(
            Line::from(vec![Span::styled(hint_text, theme.overlay1)]).alignment(Alignment::Right),
        );
    }
    frame.render_widget(container_block, container_area);

    let mut list_items = Vec::with_capacity(visible_slice.len());
    let mut selected_in_slice = None;

    for (slice_idx, &(orig_idx, suggestion)) in visible_slice.iter().enumerate() {
        let is_selected = Some(orig_idx) == state.suggest_index;
        if is_selected {
            selected_in_slice = Some(slice_idx);
        }

        let is_slash_cmd = suggestion.starts_with('/');
        let display_name = if is_slash_cmd {
            suggestion.strip_prefix('/').unwrap_or(suggestion)
        } else {
            suggestion.as_str()
        };

        let desc = slash_command_description(suggestion, state);
        let text_style = if is_selected {
            theme.highlight.add_modifier(Modifier::BOLD)
        } else {
            theme.text_dim
        };
        let desc_style = if is_selected {
            theme.subtext1.add_modifier(Modifier::BOLD)
        } else {
            theme.overlay1
        };

        let badge_span = suggestion_source_badge(suggestion, state, theme, state.basic_terminal);
        let badge_width = badge_span
            .as_ref()
            .map_or(0, |b| crate::tui::text::width(&b.content));

        let mut spans = vec![Span::raw(" ")];
        if let Some(badge) = badge_span {
            spans.push(badge);
        }
        spans.push(Span::styled(display_name, text_style));

        if let Some(description) = desc {
            let name_len = crate::tui::text::width(display_name) + badge_width;
            let pad = 28usize.saturating_sub(name_len).max(2);
            spans.push(Span::raw(" ".repeat(pad)));
            let left_pad = 1;
            let desc_budget = (inner_area.width as usize).saturating_sub(left_pad + name_len + pad);
            if desc_budget > 0 {
                spans.push(Span::styled(
                    crate::tui::text::truncate_width(description, desc_budget),
                    desc_style,
                ));
            }
        }

        list_items.push(ListItem::new(Line::from(spans)));
    }

    let mut list_state = ListState::default();
    list_state.select(selected_in_slice);

    let hl_style = crate::tui::overlay::selection_style(theme, state.basic_terminal);
    let list = List::new(list_items)
        .highlight_symbol("")
        .highlight_style(hl_style);

    let list_area = Rect {
        x: inner_area.x,
        y: inner_area.y,
        width: inner_area.width,
        height: (visible_slice.len() as u16).min(inner_area.height),
    };
    frame.render_stateful_widget(list, list_area, &mut list_state);
}
pub fn search_bar_provider_pill_rect(search_card_area: Rect, state: &AppState) -> Rect {
    let inner_width = search_card_area.width.saturating_sub(4);
    let inner_row_area = Rect {
        x: search_card_area.x + 2,
        y: search_card_area.y + 1,
        width: inner_width,
        height: 1,
    };
    let is_ultra_compact = search_card_area.width < 58;
    let ctrl_p = if is_ultra_compact {
        "P"
    } else {
        crate::tui::text::CTRL_P_STR
    };
    let ctrl_t = if is_ultra_compact {
        "T"
    } else {
        crate::tui::text::CTRL_T_STR
    };
    let sep = if state.basic_terminal { "-" } else { "·" };
    let pill_text = if state.is_tv_mode {
        if is_ultra_compact {
            "[TV]".to_string()
        } else {
            format!("[Live TV {sep} {ctrl_t}]")
        }
    } else if state.active_provider == crate::providers::models::ProviderKind::Addons {
        "[Addons]".to_string()
    } else {
        let label = state.active_provider.label();
        if is_ultra_compact {
            format!("[{label}]")
        } else {
            format!("[{label} {sep} {ctrl_p}]")
        }
    };
    let pill_width = crate::tui::text::width(&pill_text) as u16;
    let search_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(pill_width.saturating_add(1)),
        ])
        .split(inner_row_area);
    search_split[1]
}

pub fn provider_popup_bounds(
    area: Rect,
    search_card_area: Rect,
    provider_count: usize,
) -> (Rect, Rect) {
    if provider_count == 0 || search_card_area.width == 0 || area.width == 0 || area.height == 0 {
        return (Rect::default(), Rect::default());
    }

    let min_width = 20u16.min(area.width.saturating_sub(2));
    let popup_width = 24u16.min(area.width.saturating_sub(2)).max(min_width);
    let popup_height =
        ((provider_count as u16).saturating_add(2)).min(area.height.saturating_sub(2));
    let x = search_card_area
        .right()
        .saturating_sub(popup_width)
        .max(area.x)
        .min(area.right().saturating_sub(popup_width));

    let start_y = search_card_area.bottom();
    let y = if start_y.saturating_add(popup_height) <= area.bottom() {
        start_y
    } else {
        search_card_area.y.saturating_sub(popup_height)
    };

    let container_area = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    let inner_area = Rect {
        x: container_area.x.saturating_add(1),
        y: container_area.y.saturating_add(1),
        width: container_area.width.saturating_sub(2),
        height: container_area.height.saturating_sub(2),
    };

    (container_area, inner_area)
}

fn render_provider_popup(
    frame: &mut Frame,
    area: Rect,
    search_bar_area: Rect,
    state: &AppState,
    theme: &Theme,
) {
    if !state.show_provider_popup || search_bar_area.width == 0 {
        return;
    }

    let providers = state.available_providers();
    if providers.is_empty() {
        return;
    }

    let (container_area, inner_area) =
        provider_popup_bounds(area, search_bar_area, providers.len());

    if container_area.width == 0 || container_area.height <= 2 || inner_area.height == 0 {
        return;
    }

    crate::tui::clear_area(frame, container_area, theme);

    let container_block = Block::default()
        .borders(Borders::ALL)
        .border_type(crate::tui::overlay::border_type(state.basic_terminal))
        .border_style(theme.border_focus)
        .title(" Providers ")
        .title_style(theme.title);

    frame.render_widget(container_block, container_area);

    let list_items: Vec<ListItem> = providers
        .iter()
        .map(|provider| {
            let is_active = *provider == state.active_provider;
            let active_prefix = if is_active {
                if state.basic_terminal { "* " } else { "✓ " }
            } else {
                "  "
            };
            let active_style = if is_active {
                theme.success.add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let label_style = theme.text;
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(active_prefix, active_style),
                Span::styled(provider.label(), label_style),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .highlight_style(crate::tui::overlay::selection_style(
            theme,
            state.basic_terminal,
        ))
        .highlight_symbol("");
    let mut list_state = state.provider_list_state;
    frame.render_stateful_widget(list, inner_area, &mut list_state);
}

#[inline(always)]
fn item_slot_rects(item_area: Rect, poster_width: u16) -> (Rect, Rect) {
    if poster_width == 0 {
        return (
            Rect {
                x: item_area.x,
                y: item_area.y,
                width: 0,
                height: item_area.height,
            },
            item_area,
        );
    }
    let poster_w = poster_width.min(item_area.width);
    let text_x = item_area.x + poster_w + 1;
    let text_w = item_area.width.saturating_sub(poster_w + 1);

    (
        Rect {
            x: item_area.x,
            y: item_area.y,
            width: poster_w,
            height: item_area.height,
        },
        Rect {
            x: text_x,
            y: item_area.y,
            width: text_w,
            height: item_area.height,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_suggestion_source_badge_types() {
        let theme = Theme::mocha();
        let state = AppState::default();

        let badge_cmd = suggestion_source_badge("/help", &state, &theme, false);
        assert!(badge_cmd.is_some());
        assert!(badge_cmd.unwrap().content.contains("[CMD]"));

        let badge_hist = suggestion_source_badge("/history", &state, &theme, false);
        assert!(badge_hist.is_some());
        assert!(badge_hist.unwrap().content.contains("[HISTORY]"));

        let badge_fav = suggestion_source_badge("/favorites", &state, &theme, false);
        assert!(badge_fav.is_some());
        assert!(badge_fav.unwrap().content.contains("[FAVORITES]"));

        let badge_sug = suggestion_source_badge("Breaking Bad", &state, &theme, false);
        assert!(badge_sug.is_none());

        let tv_state = AppState {
            is_tv_mode: true,
            ..Default::default()
        };
        let badge_tv = suggestion_source_badge("CNN", &tv_state, &theme, false);
        assert!(badge_tv.is_some());
        assert!(badge_tv.unwrap().content.contains("[TV]"));
    }

    #[test]
    fn test_render_search_suggestions_dropdown() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState {
            input_mode: InputMode::Editing,
            search_suggestions: vec![
                "/help".to_string(),
                "/history".to_string(),
                "Inception".to_string(),
            ],
            suggest_index: Some(0),
            basic_terminal: false,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                let search_bar = Rect::new(10, 2, 60, 3);
                render_search_suggestions(
                    frame,
                    area,
                    search_bar,
                    &state,
                    &theme,
                    SearchViewState::Empty,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains("help"));
        assert!(rendered.contains("history"));
        assert!(rendered.contains("Inception"));
        assert!(rendered.contains("[CMD]"));
        assert!(rendered.contains("[HISTORY]"));
        assert!(!rendered.contains("[SUGGEST]"));
        assert!(!rendered.contains('▌'));
        assert!(!rendered.contains('├'));
        assert!(!rendered.contains('└'));
    }
    #[test]
    fn test_render_search_suggestions_overflow_cue() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState {
            input_mode: InputMode::Editing,
            search_suggestions: (1..=10).map(|i| format!("Movie Title {i}")).collect(),
            suggest_index: Some(0),
            basic_terminal: false,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                let search_bar = Rect::new(10, 2, 60, 3);
                render_search_suggestions(
                    frame,
                    area,
                    search_bar,
                    &state,
                    &theme,
                    SearchViewState::Empty,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains("▼ (+4 more)"));
    }

    #[test]
    fn test_landing_draw_hides_favorites_when_suggestions_open() {
        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Editing,
            streaming_enabled: true,
            search_suggestions: vec!["/help".to_string(), "Inception".to_string()],
            suggest_index: Some(0),
            ..Default::default()
        };
        state.favorites.items.push(crate::favorites::FavoriteItem {
            provider: "moviebox".to_string(),
            subject_id: "fav-1".to_string(),
            title: "Secret Favorite Movie".to_string(),
            cover_url: None,
            stype: 1,
            release_year: "2024".to_string(),
            added_at: 0,
        });
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 90, 24);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..90 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("help"));
        assert!(rendered.contains("Inception"));
        assert!(!rendered.contains("Favorites"));
        assert!(!rendered.contains("Secret Favorite Movie"));
    }

    #[test]
    fn test_landing_draw_hides_discover_when_suggestions_open() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Editing,
            streaming_enabled: true,
            search_suggestions: vec!["/help".to_string(), "Inception".to_string()],
            suggest_index: Some(0),
            ..Default::default()
        };
        state.favorites.items.clear();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 90, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..90 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("help"));
        assert!(rendered.contains("Inception"));
        assert!(!rendered.contains("Discover Categories"));
        assert!(!rendered.contains("Trending Now"));
    }

    #[test]
    fn test_discover_landing_rendering_streaming_mode() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            streaming_enabled: true,
            active_provider: crate::providers::models::ProviderKind::MovieBox,
            ..Default::default()
        };
        state.history.recent.clear();
        state.favorites.items.clear();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 90, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..90 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Discover Categories"));
        assert!(rendered.contains("[ /browse ]"));
        assert!(rendered.contains("Trending Now"));
        assert!(rendered.contains("Popular & In-Theaters"));
        assert!(!rendered.contains("/browse ·"));
        assert!(!rendered.contains("Top Movies"));
    }

    #[test]
    fn test_discover_landing_rendering_addon_mode() {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            addons_enabled: true,
            active_provider: crate::providers::models::ProviderKind::Addons,
            ..Default::default()
        };
        state.history.recent.clear();
        state.favorites.items.clear();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 90, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..90 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Discover Categories"));
        assert!(rendered.contains("[ /browse ]"));
        assert!(rendered.contains("Top Movies"));
        assert!(rendered.contains("Cinemeta Curated Catalog"));
        assert!(!rendered.contains("/browse ·"));
        assert!(!rendered.contains("Trending Now"));
    }

    #[test]
    fn test_search_suggestions_bounds() {
        let area = Rect::new(0, 0, 80, 24);
        let search_bar = Rect::new(10, 2, 60, 3);
        let (container, inner) = search_suggestions_bounds(area, search_bar, 3);
        assert_eq!(container.x, 10);
        assert_eq!(container.y, 5);
        assert_eq!(container.width, 60);
        assert_eq!(container.height, 5);
        assert_eq!(inner.x, 11);
        assert_eq!(inner.y, 6);
        assert_eq!(inner.width, 58);
        assert_eq!(inner.height, 3);
    }
    #[test]
    fn test_search_bar_provider_pill_rect() {
        let state = AppState::default();
        let normal_card = Rect::new(10, 2, 64, 3);
        let pill_rect = search_bar_provider_pill_rect(normal_card, &state);
        assert_eq!(pill_rect.y, 3);
        assert_eq!(pill_rect.height, 1);
        assert!(pill_rect.right() <= normal_card.right().saturating_sub(2));
        assert!(pill_rect.width >= 12);

        let compact_card = Rect::new(5, 2, 40, 3);
        let compact_pill = search_bar_provider_pill_rect(compact_card, &state);
        assert_eq!(compact_pill.y, 3);
        assert_eq!(compact_pill.height, 1);
        assert!(compact_pill.right() <= compact_card.right().saturating_sub(2));
        assert!(compact_pill.width < pill_rect.width);
    }

    #[test]
    fn test_provider_popup_bounds() {
        let area = Rect::new(0, 0, 80, 24);
        let search_bar = Rect::new(10, 2, 60, 3);
        let (container, inner) = provider_popup_bounds(area, search_bar, 4);
        assert_eq!(container.y, 5);
        assert_eq!(container.height, 6);
        assert_eq!(container.width, 24);
        assert_eq!(container.right(), 70);
        assert_eq!(inner.x, container.x + 1);
        assert_eq!(inner.y, 6);
        assert_eq!(inner.width, 22);
        assert_eq!(inner.height, 4);
        let bottom_bar = Rect::new(10, 20, 60, 3);
        let (flipped, _) = provider_popup_bounds(area, bottom_bar, 4);
        assert_eq!(flipped.y, 14);
        assert_eq!(flipped.height, 6);
        let narrow_area = Rect::new(0, 0, 16, 24);
        let narrow_bar = Rect::new(1, 2, 14, 3);
        let (narrow_container, _) = provider_popup_bounds(narrow_area, narrow_bar, 4);
        assert!(narrow_container.width <= 14);
        assert!(narrow_container.right() <= 16);
    }

    #[test]
    fn test_render_provider_popup() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            show_provider_popup: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.provider_list_state.select(Some(0));
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                let search_bar = Rect::new(10, 2, 60, 3);
                render_provider_popup(frame, area, search_bar, &state, &theme);
            })
            .unwrap();

        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                let cell = terminal.backend().buffer().cell((x, y)).unwrap();
                rendered.push_str(cell.symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains("Providers"));
        assert!(rendered.contains("MovieBox"));
        assert!(rendered.contains("4KHDHub"));
        assert!(rendered.contains("✓"));
        assert!(!rendered.contains('●'));
        assert!(!rendered.contains('○'));
    }

    #[test]
    fn test_search_deck_width_stability() {
        let mut state = AppState::default();
        let compact_area = Rect::new(0, 0, 70, 24);
        let normal_area = Rect::new(0, 0, 90, 24);
        let wide_area = Rect::new(0, 0, 120, 24);

        let w_compact_empty = search_deck_width(compact_area, &state, true);
        let w_normal_empty = search_deck_width(normal_area, &state, true);
        let w_wide_empty = search_deck_width(wide_area, &state, true);
        assert_eq!(w_compact_empty, 54);
        assert_eq!(w_normal_empty, 64);
        assert_eq!(w_wide_empty, 64);

        state.search_query = "Inception 2010 1080p".into();
        assert_eq!(search_deck_width(compact_area, &state, true), 54);
        assert_eq!(search_deck_width(normal_area, &state, true), 64);
        assert_eq!(search_deck_width(wide_area, &state, true), 64);
    }

    #[test]
    fn test_search_content_ghost_placeholder() {
        let state_rich = AppState {
            input_mode: InputMode::Editing,
            basic_terminal: false,
            ..Default::default()
        };
        let content_rich = search_content(&state_rich, SearchViewState::Editing, 80);
        assert!(content_rich.contains("❯ Search movies, series & anime…"));
        assert!(!content_rich.contains("▎"));

        let state_basic = AppState {
            input_mode: InputMode::Editing,
            basic_terminal: true,
            ..Default::default()
        };
        let content_basic = search_content(&state_basic, SearchViewState::Editing, 80);
        assert!(content_basic.contains("> Search movies, series & anime…"));
        assert!(!content_basic.contains("█"));
    }

    #[test]
    fn test_dynamic_search_placeholder() {
        let mut state = AppState::default();
        assert_eq!(
            dynamic_search_placeholder(&state),
            "Search movies, series & anime…"
        );
        state.tick_count = 100;
        assert_eq!(
            dynamic_search_placeholder(&state),
            "Search movies, series & anime…"
        );

        let tv_state = AppState {
            is_tv_mode: true,
            ..Default::default()
        };
        assert_eq!(
            dynamic_search_placeholder(&tv_state),
            "Search live TV channels…"
        );

        let addon_state = AppState {
            active_provider: crate::providers::models::ProviderKind::Addons,
            ..Default::default()
        };
        assert_eq!(
            dynamic_search_placeholder(&addon_state),
            "Search via addons…"
        );
    }

    #[test]
    fn test_favorites_landing_rendering() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            favorites_focus: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.history.recent.clear();
        state.favorites.items.clear();
        state.favorites_landing_state.select(Some(0));
        for i in 0..10 {
            state.favorites.items.push(crate::favorites::FavoriteItem {
                provider: "moviebox".to_string(),
                subject_id: format!("fav-{i}"),
                title: format!("Favorite Movie {i}"),
                cover_url: None,
                stype: 1,
                release_year: "2024".to_string(),
                added_at: 10 - i as u64,
            });
        }
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                render_landing_deck(frame, area, &state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains("Favorites"));
        assert!(!rendered.contains("★"));
        assert!(rendered.contains("Favorite Movie 0"));
        assert!(rendered.contains("Favorite Movie 1"));
        assert!(!rendered.contains("▌"));
        assert!(rendered.contains("2024 Movie"));
        assert!(rendered.contains("+5 more · /favorites"));
    }
    #[test]
    fn test_favorites_landing_rendering_basic_terminal() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            favorites_focus: true,
            basic_terminal: true,
            ..Default::default()
        };
        state.history.recent.clear();
        state.favorites.items.clear();
        state.favorites_landing_state.select(Some(0));
        for i in 0..10 {
            state.favorites.items.push(crate::favorites::FavoriteItem {
                provider: "moviebox".to_string(),
                subject_id: format!("fav-{i}"),
                title: format!("Favorite Movie {i}"),
                cover_url: None,
                stype: 1,
                release_year: "2024".to_string(),
                added_at: 10 - i as u64,
            });
        }
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                render_landing_deck(frame, area, &state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Favorites"));
        assert!(!rendered.contains("*  Favorites"));
        assert!(rendered.contains("Favorite Movie 0"));
        assert!(rendered.contains("Favorite Movie 1"));
        assert!(rendered.contains("2024 Movie"));
        assert!(rendered.contains("+5 more - /favorites"));
    }
    #[test]
    fn test_landing_deck_continue_watching_rendering() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            favorites_focus: true,
            streaming_enabled: true,
            ..Default::default()
        };
        state.history.recent.push(crate::history::WatchHistoryItem {
            provider: "moviebox".to_string(),
            subject_id: "show-1".to_string(),
            title: "Severance".to_string(),
            cover_url: None,
            stype: 2,
            release_year: "2022".to_string(),
            season: 1,
            episode: 3,
            progress_seconds: 1200,
            duration_seconds: Some(3000),
            completed: false,
            timestamp: 100,
        });
        state.favorites.items.push(crate::favorites::FavoriteItem {
            provider: "moviebox".to_string(),
            subject_id: "fav-1".to_string(),
            title: "Inception".to_string(),
            cover_url: None,
            stype: 1,
            release_year: "2010".to_string(),
            added_at: 100,
        });
        state.favorites_landing_state.select(Some(0));
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                render_landing_deck(frame, area, &state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Continue Watching"));
        assert!(rendered.contains("Favorites"));
        assert!(rendered.contains("(Tab)"));
        assert!(rendered.contains("Severance"));
        assert!(rendered.contains("S01E03"));
        assert!(rendered.contains("40%"));
    }

    #[test]
    fn test_search_results_card_and_scrollbar_draw() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            search_query: "Matrix".into(),
            search_results: vec![
                crate::models::SearchResult {
                    id: "1".into(),
                    title: "The Matrix 1080p".into(),
                    stype: 1,
                    release_year: "1999".into(),
                    cover_url: None,
                    season: 0,
                    episode: 0,
                    provider: crate::providers::models::ProviderKind::MovieBox,
                },
                crate::models::SearchResult {
                    id: "2".into(),
                    title: "The Matrix Reloaded 4K".into(),
                    stype: 1,
                    release_year: "2003".into(),
                    cover_url: None,
                    season: 0,
                    episode: 0,
                    provider: crate::providers::models::ProviderKind::FourKHdHub,
                },
            ],
            ..Default::default()
        };
        state.search_list_state.select(Some(0));
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        state.input_mode = InputMode::Editing;
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_landing_rendering_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            streaming_enabled: true,
            tv_enabled: true,
            addons_enabled: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.favorites.items.clear();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(rendered.contains('❯'));
        assert!(rendered.contains("Search movies, series & anime…"));
        assert!(rendered.contains("[MovieBox"));
        assert!(rendered.contains(crate::tui::text::CTRL_P_STR));

        assert!(!rendered.contains("Stream"));
        assert!(rendered.contains("TV"));
        assert!(!rendered.contains("Addon"));
        assert!(rendered.contains("Quit"));
    }

    #[test]
    fn test_landing_rendering_with_favorites() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            streaming_enabled: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.history.recent.clear();
        for i in 1..=6 {
            state.favorites.items.push(crate::favorites::FavoriteItem {
                provider: "moviebox".to_string(),
                subject_id: format!("fav-{i}"),
                title: format!("Interstellar {i}"),
                cover_url: None,
                stype: if i % 2 == 0 { 2 } else { 1 },
                release_year: "2014".to_string(),
                added_at: i as u64,
            });
        }
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("Favorites"));
        assert!(rendered.contains("Interstellar"));
        assert!(rendered.contains("2014 Movie") || rendered.contains("2014 Series"));
        assert!(rendered.contains("more · /favorites"));
    }

    #[test]
    fn test_search_card_typing_transitions_pill() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Editing,
            search_query: "Inception".into(),
            basic_terminal: false,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains("[Enter] Search"));
    }

    #[test]
    fn test_search_card_modes_pill_labels() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::mocha();

        let mut tv_state = AppState {
            is_tv_mode: true,
            basic_terminal: false,
            ..Default::default()
        };
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                draw(frame, area, &mut tv_state, &theme);
            })
            .unwrap();
        let mut tv_rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                tv_rendered.push_str(terminal.backend().buffer()[(x, y)].symbol());
            }
            tv_rendered.push('\n');
        }
        assert!(tv_rendered.contains("[Live TV"));
        assert!(tv_rendered.contains("Search live TV channels…"));
        let mut addon_state = AppState {
            active_provider: crate::providers::models::ProviderKind::Addons,
            basic_terminal: false,
            ..Default::default()
        };
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 24);
                draw(frame, area, &mut addon_state, &theme);
            })
            .unwrap();
        let mut addon_rendered = String::new();
        for y in 0..24 {
            for x in 0..80 {
                addon_rendered.push_str(terminal.backend().buffer()[(x, y)].symbol());
            }
            addon_rendered.push('\n');
        }
        assert!(addon_rendered.contains("[Addons]"));
        assert!(addon_rendered.contains("Search via addons…"));
    }

    #[test]
    fn test_search_view_state_loading_before_settle() {
        let mut state = AppState {
            input_mode: InputMode::Normal,
            is_loading: false,
            has_search_settled: false,
            search_results: vec![],
            ..Default::default()
        };
        state.search_query.set_content("Inception");

        assert_eq!(search_view_state(&state), SearchViewState::Loading);

        state.has_search_settled = true;
        assert_eq!(search_view_state(&state), SearchViewState::NoResults);

        state.search_results.push(crate::models::SearchResult {
            id: "1".to_string(),
            title: "Inception".to_string(),
            stype: 1,
            release_year: "2010".to_string(),
            provider: crate::models::ProviderKind::MovieBox,
            cover_url: None,
            season: 0,
            episode: 0,
        });
        assert_eq!(search_view_state(&state), SearchViewState::Results);
    }

    #[test]
    fn test_no_results_screen_layout_and_elements() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            is_loading: false,
            has_search_settled: true,
            search_results: vec![],
            ..Default::default()
        };
        state.search_query.set_content("deewaniyat");
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains("deewaniyat"));
        assert!(content.contains("0 results"));
        assert!(content.contains("No results for “deewaniyat” on MovieBox"));
        assert!(content.contains("Try on 4KHDHub"));
        assert!(content.contains("Clear Search"));
    }

    #[test]
    fn test_item_slot_rects_responsive_zero_width() {
        let area = Rect::new(10, 5, 80, 4);
        let (poster_zero, text_zero) = item_slot_rects(area, 0);
        assert_eq!(poster_zero.width, 0);
        assert_eq!(text_zero.x, 10);
        assert_eq!(text_zero.width, 80);

        let (poster, text) = item_slot_rects(area, 12);
        assert_eq!(poster.width, 12);
        assert_eq!(text.x, 23);
        assert_eq!(text.width, 67);
    }

    #[test]
    fn test_search_result_selection_indicator_without_opaque_background() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            search_query: "Matrix".into(),
            search_results: vec![crate::models::SearchResult {
                id: "test_1".into(),
                title: "The Matrix".into(),
                stype: 1,
                release_year: "1999".into(),
                cover_url: None,
                season: 0,
                episode: 0,
                provider: crate::providers::models::ProviderKind::MovieBox,
            }],
            has_search_settled: true,
            basic_terminal: false,
            ..Default::default()
        };
        state.search_list_state.select(Some(0));
        let theme = Theme::mocha();
        let selected_bg = theme.surface0.fg.unwrap_or(theme.base);
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        for x in 0..100 {
            assert_ne!(
                buffer[(x, 2)].style().bg,
                Some(selected_bg),
                "opaque background leaked at x={x}"
            );
        }
        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..100 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(
            rendered.contains("The Matrix"),
            "rendered content:\n{rendered}"
        );
        let mut title_has_highlight = false;
        let expected_hl = crate::tui::overlay::selection_style(&theme, false);
        for y in 0..30 {
            for x in 0..100 {
                let cell = &buffer[(x, y)];
                if cell.symbol() == "T"
                    && buffer[(x + 1, y)].symbol() == "h"
                    && buffer[(x + 2, y)].symbol() == "e"
                {
                    if cell.style().bg == expected_hl.bg && cell.style().fg == expected_hl.fg {
                        title_has_highlight = true;
                    }
                }
            }
        }
        assert!(
            title_has_highlight,
            "expected selected card title to have selection_style highlight"
        );
        let mut meta_has_reversed = false;
        for y in 0..30 {
            for x in 0..100 {
                let cell = &buffer[(x, y)];
                if cell.symbol() == "1"
                    && buffer[(x + 1, y)].symbol() == "9"
                    && buffer[(x + 2, y)].symbol() == "9"
                    && buffer[(x + 3, y)].symbol() == "9"
                {
                    if cell.style().add_modifier.contains(Modifier::REVERSED) {
                        meta_has_reversed = true;
                    }
                }
            }
        }
        assert!(
            !meta_has_reversed,
            "expected metadata row to remain transparent and un-reversed"
        );
        let mut far_right_has_reversed = false;
        for y in 0..30 {
            for x in 70..100 {
                let cell = &buffer[(x, y)];
                if cell.style().add_modifier.contains(Modifier::REVERSED) {
                    far_right_has_reversed = true;
                }
            }
        }
        assert!(
            !far_right_has_reversed,
            "expected title highlight to be a tight pill, not stretching across full slot width"
        );
    }
    #[test]
    fn test_search_result_selection_indicator_basic_terminal() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            input_mode: InputMode::Normal,
            search_query: "Matrix".into(),
            search_results: vec![crate::models::SearchResult {
                id: "test_1".into(),
                title: "The Matrix".into(),
                stype: 1,
                release_year: "1999".into(),
                cover_url: None,
                season: 0,
                episode: 0,
                provider: crate::providers::models::ProviderKind::MovieBox,
            }],
            has_search_settled: true,
            basic_terminal: true,
            ..Default::default()
        };
        state.search_list_state.select(Some(0));
        let theme = Theme::mocha();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..30 {
            for x in 0..100 {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        assert!(
            rendered.contains("The Matrix"),
            "rendered content:\n{rendered}"
        );
        let mut title_has_underlined = false;
        for y in 0..30 {
            for x in 0..100 {
                let cell = &buffer[(x, y)];
                if cell.symbol() == "T"
                    && buffer[(x + 1, y)].symbol() == "h"
                    && buffer[(x + 2, y)].symbol() == "e"
                {
                    if cell.style().add_modifier.contains(Modifier::UNDERLINED) {
                        title_has_underlined = true;
                    }
                }
            }
        }
        assert!(
            title_has_underlined,
            "expected basic terminal selected title to have UNDERLINED style"
        );
    }

    #[test]
    fn test_home_deck_unfocused_when_modal_active() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            favorites_focus: true,
            show_settings_popup: true,
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!content.contains("●"));
    }

    #[test]
    fn test_home_banner_suppressed_when_update_modal_active() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState {
            update_available: Some(("0.1.18".to_string(), "Release Notes".to_string())),
            ..Default::default()
        };
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(!content.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn test_home_banner_rendered_when_no_update_modal() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::default();
        let theme = Theme::mocha();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 100, 30);
                draw(frame, area, &mut state, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(content.contains(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    }
}
