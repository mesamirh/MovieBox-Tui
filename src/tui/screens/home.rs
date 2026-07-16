use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let show_cursor = (state.tick_count % 16) < 8;
    let search_content = if !state.status_message.is_empty() && state.input_mode == InputMode::Normal {
        format!("> {}", state.status_message)
    } else if state.search_query.is_empty() {
        let prompts = ["Search for a movie...", "Search for a TV series...", "Search by genre..."];
        let type_speed = 3;
        let del_speed = 1;
        let pause1 = 60; // ~1 sec
        let pause2 = 15; // ~250 ms
        
        let mut total_ticks = 0;
        for p in prompts.iter() {
            total_ticks += p.len() * type_speed + pause1 + p.len() * del_speed + pause2;
        }
        let mut t = (state.tick_count as usize) % total_ticks;
        
        let mut animated_text = String::new();
        for p in prompts.iter() {
            let t_type = p.len() * type_speed;
            let t_del = p.len() * del_speed;
            let cycle = t_type + pause1 + t_del + pause2;
            
            if t < cycle {
                let display_len = if t < t_type {
                    t / type_speed
                } else if t < t_type + pause1 {
                    p.len()
                } else if t < t_type + pause1 + t_del {
                    p.len().saturating_sub((t - (t_type + pause1)) / del_speed)
                } else {
                    0
                };
                animated_text = p[0..display_len].to_string();
                break;
            } else {
                t -= cycle;
            }
        }

        if state.input_mode == InputMode::Editing {
            if show_cursor { format!("> {}█", animated_text) } else { format!("> {} ", animated_text) }
        } else {
            if show_cursor { format!("> {}|", animated_text) } else { format!("> {} ", animated_text) }
        }
    } else {
        if state.input_mode == InputMode::Editing {
            if show_cursor { format!("> {}█", state.search_query) } else { format!("> {} ", state.search_query) }
        } else {
            format!("> {}", state.search_query)
        }
    };



    if state.search_results.is_empty()
        && !state.is_loading
        && !state.status_message.to_lowercase().contains("fail")
    {
        if state.tick_count < 1 {
            return; // terminal opens black
        }

        let is_narrow = area.width < 60;
        let is_wide = area.width >= 100;
        let logo_height = if is_narrow { 2 } else if is_wide { 6 } else { 4 };
        let logo_text = if is_narrow {
            r"█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀
█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █"
        } else if is_wide {
            r" __  __   ____   __     __  ___   _____   ____     ____   __  __ 
|  \/  | / __ \  \ \   / / |_ _| | ____| | __ )   / __ \  \ \/ / 
| \  / || |  | |  \ \ / /   | |  |  _|   |  _ \  | |  | |  \  /  
| |\/| || |  | |   \ V /    | |  | |___  | |_) | | |  | |  /  \  
| |  | || |__| |    \ /     | |  |  ___| |  _ <  | |__| | / /\ \ 
|_|  |_| \____/      V     |___| |_____| |_| \_\  \____/ /_/  \_\ "
        } else {
            r"  __  __  ___  __   __ ___  ___  ___   ___  __  __ 
 |  \/  |/ _ \ \ \ / /|_ _|| __|| _ ) / _ \ \ \/ / 
 | |\/| | (_) | \ V /  | | | _| | _ \| (_) | >  <  
 |_|  |_|\___/   \_/  |___||___||___/ \___/ /_/\_\ "
        };

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Length(logo_height),
                Constraint::Length(1),
                Constraint::Percentage(15),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let logo_width = if is_narrow { 31 } else if is_wide { 73 } else { 55 };
        let pad = area.width.saturating_sub(logo_width) / 2;
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[1]);

        let version_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(pad),
                Constraint::Length(logo_width),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[2]);

        let logo_style = if state.tick_count == 1 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(60, 60, 60))
        } else if state.tick_count == 2 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(140, 140, 140))
        } else {
            theme.title
        };

        let title_art = Paragraph::new(logo_text)
            .alignment(Alignment::Left)
            .style(logo_style);

        frame.render_widget(title_art, horizontal_chunks[1]);

        let version_style = if state.tick_count == 1 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(40, 40, 40))
        } else if state.tick_count == 2 {
            ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(90, 90, 90))
        } else {
            theme.text_dim
        };
        let version = Paragraph::new("v0.1.0")
            .alignment(Alignment::Right)
            .style(version_style);
        frame.render_widget(version, version_chunks[1]);

        if state.tick_count >= 3 {
            let search_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(vertical_chunks[4]);

            let search_bar = Paragraph::new(search_content.clone())
                .alignment(Alignment::Center)
                .style(match state.input_mode {
                    InputMode::Editing => theme.title,
                    InputMode::Normal => theme.text,
                });

            frame.render_widget(search_bar, search_chunks[1]);

            let legend = Paragraph::new("/ Search   ↑↓ Browse   ? Help")
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(legend, vertical_chunks[6]);
        }
    } else {
        let desired_height = if state.is_loading {
            10
        } else {
            std::cmp::max(state.search_results.len() as u16 + 4, 6)
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(desired_height),
                Constraint::Min(0),
            ])
            .split(area);

        let search_bar = Paragraph::new(search_content.clone())
            .style(match state.input_mode {
                InputMode::Editing => theme.title,
                InputMode::Normal => theme.text,
            });
        frame.render_widget(search_bar, chunks[0]);

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Results ")
            .title_style(theme.title)
            .border_style(theme.border);

        if state.is_loading {
            let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

            let inner_area = list_block.inner(chunks[1]);
            frame.render_widget(list_block, chunks[1]);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let p = Paragraph::new(format!("{} Searching...", spinner))
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(p, v_chunks[1]);
        } else if !state.search_results.is_empty() {
            let rows: Vec<Row> = state
                .search_results
                .iter()
                .map(|res| {
                    let type_tag = if res.stype == 1 {
                        "MOVIE"
                    } else if res.stype == 2 {
                        "TV"
                    } else {
                        "OTHER"
                    };
                    Row::new(vec![
                        Cell::from(type_tag).style(theme.header),
                        Cell::from(res.title.as_str()).style(theme.text),
                        Cell::from(res.release_year.as_str()).style(theme.text_dim),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Length(8),
                Constraint::Percentage(70),
                Constraint::Length(10),
            ];

            let table = Table::new(rows, widths)
                .header(Row::new(vec!["Type", "Title", "Date"]).style(theme.title))
                .block(list_block)
                .row_highlight_style(theme.highlight.bg(ratatui::style::Color::Rgb(30, 30, 50)))
                .highlight_symbol(">> ");

            frame.render_stateful_widget(table, chunks[1], &mut state.search_list_state);
        } else {
            let inner_area = list_block.inner(chunks[1]);
            frame.render_widget(list_block, chunks[1]);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let p = Paragraph::new(state.status_message.clone())
                .alignment(Alignment::Center)
                .style(theme.error);
            frame.render_widget(p, v_chunks[1]);
        }
    }

    if state.input_mode == InputMode::Editing && !state.search_suggestions.is_empty() {
        let search_area = if state.search_results.is_empty() && !state.is_loading && !state.status_message.to_lowercase().contains("fail") {
            let is_narrow = area.width < 60;
            let is_wide = area.width >= 100;
            let logo_height = if is_narrow { 2 } else if is_wide { 6 } else { 4 };
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(15),
                    Constraint::Length(logo_height),
                    Constraint::Length(1),
                    Constraint::Percentage(15),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(50), Constraint::Percentage(25)])
                .split(vertical_chunks[4])[1]
        } else {
            Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(4)]).split(area)[0]
        };

        let dropdown_height = std::cmp::min(state.search_suggestions.len() as u16 + 2, 10);
        
        let is_home_screen = state.search_results.is_empty() && !state.is_loading && !state.status_message.to_lowercase().contains("fail");
        
        let dropdown_y = if !is_home_screen && search_area.y > area.height / 2 {
            search_area.y.saturating_sub(dropdown_height)
        } else {
            search_area.y + search_area.height
        };
        
        let max_len = state.search_suggestions.iter().map(|s| s.len()).max().unwrap_or(0) as u16;
        let dropdown_width = std::cmp::min(std::cmp::max(max_len + 8, 30), search_area.width);
        let dropdown_x = search_area.x + (search_area.width.saturating_sub(dropdown_width)) / 2;

        let dropdown_area = Rect {
            x: dropdown_x,
            y: dropdown_y,
            width: dropdown_width,
            height: dropdown_height,
        };

        if dropdown_area.y + dropdown_area.height <= area.height || search_area.y > area.height / 2 {
            frame.render_widget(ratatui::widgets::Clear, dropdown_area);
            let items: Vec<ratatui::widgets::ListItem> = state.search_suggestions.iter().enumerate().map(|(i, s)| {
                let text = if Some(i) == state.suggest_index { format!(">> {}", s) } else { format!("   {}", s) };
                let style = if Some(i) == state.suggest_index { theme.highlight } else { theme.text };
                ratatui::widgets::ListItem::new(ratatui::text::Line::from(ratatui::text::Span::styled(text, style)).alignment(ratatui::layout::Alignment::Left))
            }).collect();
            let list = ratatui::widgets::List::new(items)
                .block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(theme.border_focus)
                        .border_type(ratatui::widgets::BorderType::Rounded)
                );
            frame.render_widget(list, dropdown_area);
        }
    }
}
