use crate::tui::{
    state::{AppState, InputMode},
    theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let search_content = if state.input_mode == InputMode::Editing {
        format!(" {}█", state.search_query)
    } else {
        if state.search_query.is_empty() {
            " (Press '/' to search)".to_string()
        } else {
            format!(" {}", state.search_query)
        }
    };

    let border_style = match state.input_mode {
        InputMode::Editing => theme.border_focus,
        InputMode::Normal => theme.border,
    };

    if state.search_results.is_empty()
        && !state.is_loading
        && !state.status_message.to_lowercase().contains("fail")
    {
        let is_narrow = area.width < 60;
        let logo_height = if is_narrow { 3 } else { 5 };
        let logo_text = if is_narrow {
            r"█▀▄▀█ █▀█ █ █ █ █▀▀ █▀▄ █▀█ ▀▄▀
█ ▀ █ █▄█ ▀▄▀ █ ██▄ █▄▀ █▄█ █ █"
        } else {
            r"  __  __  ___  __   __ ___  ___  ___   ___  __  __ 
 |  \/  |/ _ \ \ \ / /|_ _|| __|| _ ) / _ \ \ \/ / 
 | |\/| | (_) | \ V /  | | | _| | _ \| (_) | >  <  
 |_|  |_|\___/   \_/  |___||___||___/ \___/ /_/\_\ "
        };

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),      // Top padding
                Constraint::Length(logo_height), // Dynamic logo height
                Constraint::Length(3),           // Search bar height
                Constraint::Length(2),           // Status text
                Constraint::Percentage(30),      // Bottom padding
            ])
            .split(area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(5),  // Left padding
                Constraint::Percentage(90), // Ample centered space for logo
                Constraint::Percentage(5),  // Right padding
            ])
            .split(vertical_chunks[1]); // Logo horizontal constraints

        let search_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical_chunks[2]); // Search bar horizontal constraints

        let title_art = Paragraph::new(logo_text)
            .alignment(Alignment::Center)
            .style(theme.title);

        let search_bar = Paragraph::new(search_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Search ")
                    .title_style(theme.title)
                    .border_style(border_style),
            )
            .style(theme.text);

        let intro = Paragraph::new(state.status_message.as_str())
            .alignment(Alignment::Center)
            .style(theme.text_dim);

        frame.render_widget(title_art, horizontal_chunks[1]);
        frame.render_widget(search_bar, search_chunks[1]);
        frame.render_widget(intro, vertical_chunks[3]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top header / search input
                Constraint::Min(4),    // Main content panel
            ])
            .split(area);

        let search_bar = Paragraph::new(search_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Search ")
                    .title_style(theme.title)
                    .border_style(border_style),
            )
            .style(theme.text);
        frame.render_widget(search_bar, chunks[0]);

        let content_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Results ")
            .title_style(theme.title)
            .border_style(theme.border);

        if state.is_loading {
            let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

            let inner_area = list_block.inner(content_split[0]);
            frame.render_widget(list_block, content_split[0]);

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

            frame.render_stateful_widget(table, content_split[0], &mut state.search_list_state);
        } else {
            let inner_area = list_block.inner(content_split[0]);
            frame.render_widget(list_block, content_split[0]);

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

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Info Preview ")
            .title_style(theme.title)
            .border_style(theme.border);

        let preview_area = content_split[1];

        if state.preview_loading {
            let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let spinner = spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

            let inner_area = preview_block.inner(preview_area);
            frame.render_widget(preview_block, preview_area);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let loading_p = Paragraph::new(format!("{} Loading metadata...", spinner))
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(loading_p, v_chunks[1]);
        } else if let Some(preview) = &state.search_preview {
            let inner_area = preview_block.inner(preview_area);
            frame.render_widget(preview_block.clone(), preview_area);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(15), Constraint::Min(1)])
                .split(inner_area);

            let top_area = v_chunks[0];
            let synopsis_area = v_chunks[1];

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(20),
                    Constraint::Length(2),
                    Constraint::Min(1),
                ])
                .split(top_area);

            let poster_area = chunks[0];
            let meta_area = chunks[2];

            if let Some(img) = &state.poster_image {
                if state.poster_protocol.as_ref().map(|(r, _)| *r) != Some(poster_area)
                    && let Some(picker) = &mut state.image_picker
                {
                    let size = ratatui::layout::Size::new(poster_area.width, poster_area.height);
                    if let Ok(proto) =
                        picker.new_protocol(img.clone(), size, ratatui_image::Resize::Fit(None))
                    {
                        state.poster_protocol = Some((poster_area, proto));
                    }
                }
                if let Some((_, proto)) = &state.poster_protocol {
                    frame.render_widget(ratatui_image::Image::new(proto), poster_area);
                }
            } else {
                let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let current_spinner =
                    spinner_frames[(state.tick_count as usize) % spinner_frames.len()];

                let v_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(45),
                        Constraint::Length(1),
                        Constraint::Percentage(50),
                    ])
                    .split(poster_area);

                let placeholder = Paragraph::new(format!("{} Loading Art...", current_spinner))
                    .style(theme.text_dim)
                    .alignment(Alignment::Center);
                frame.render_widget(placeholder, v_chunks[1]);
            }

            let title = preview
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown");
            let description = preview
                .get("description")
                .and_then(|d| d.as_str())
                .or_else(|| preview.get("intro").and_then(|i| i.as_str()))
                .unwrap_or("No description available.");
            let release_date = preview
                .get("releaseDate")
                .and_then(|y| y.as_str())
                .or_else(|| preview.get("year").and_then(|y| y.as_str()))
                .unwrap_or("N/A");
            let imdb_rating = preview
                .get("imdbRatingValue")
                .and_then(|r| {
                    r.as_f64()
                        .map(|rf| rf.to_string())
                        .or_else(|| r.as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "N/A".to_string());

            let genres = preview
                .get("genre")
                .and_then(|g| g.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "N/A".to_string());
            let duration = preview
                .get("duration")
                .and_then(|d| d.as_str())
                .unwrap_or("N/A");
            let country = preview
                .get("countryName")
                .and_then(|c| c.as_str())
                .unwrap_or("N/A");
            let content_rating = preview
                .get("contentRating")
                .and_then(|c| c.as_str())
                .unwrap_or("N/A");

            let type_val = preview
                .get("subjectType")
                .and_then(|s| s.as_i64())
                .unwrap_or(1);
            let type_str = if type_val == 2 { "TV Series" } else { "Movie" };

            let viewers = preview.get("viewers").and_then(|v| v.as_i64()).unwrap_or(0);

            let mut staff_list = Vec::new();
            if let Some(staff) = preview.get("staffList").and_then(|s| s.as_array()) {
                for person in staff.iter().take(3) {
                    if let Some(name) = person.get("name").and_then(|n| n.as_str()) {
                        staff_list.push(name.to_string());
                    }
                }
            }
            let cast = if staff_list.is_empty() {
                "N/A".to_string()
            } else {
                staff_list.join(", ")
            };

            let label_style = theme.header;
            let val_style = theme.text;

            let meta_lines = vec![
                Line::from(vec![
                    Span::styled("Title:    ", label_style),
                    Span::styled(title, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Type:     ", label_style),
                    Span::styled(type_str, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Genre:    ", label_style),
                    Span::styled(genres, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Released: ", label_style),
                    Span::styled(release_date, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Duration: ", label_style),
                    Span::styled(duration, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Rating:   ", label_style),
                    Span::styled(
                        format!("{} (IMDb: {})", content_rating, imdb_rating),
                        val_style,
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Country:  ", label_style),
                    Span::styled(country, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Cast:     ", label_style),
                    Span::styled(cast, val_style),
                ]),
                Line::from(vec![
                    Span::styled("Viewers:  ", label_style),
                    Span::styled(viewers.to_string(), val_style),
                ]),
            ];

            let synopsis_lines = vec![
                Line::from(vec![Span::styled("Synopsis:", label_style)]),
                Line::from(vec![]),
            ];

            let meta_p = Paragraph::new(meta_lines).wrap(Wrap { trim: true });
            frame.render_widget(meta_p, meta_area);

            let mut syn_lines = synopsis_lines;
            syn_lines.push(Line::from(vec![Span::styled(description, theme.text_dim)]));
            let p = Paragraph::new(syn_lines).wrap(Wrap { trim: true });
            frame.render_widget(p, synopsis_area);
        } else {
            let inner_area = preview_block.inner(preview_area);
            frame.render_widget(preview_block, preview_area);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner_area);

            let empty_p = Paragraph::new("Select a result to preview info.")
                .alignment(Alignment::Center)
                .style(theme.text_dim);
            frame.render_widget(empty_p, v_chunks[1]);
        }
    }
}
