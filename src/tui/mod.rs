pub mod action;
pub mod app;
pub mod event;
pub mod iptv_data;
pub mod library;
pub mod overlay;
pub mod player;
pub mod state;
pub mod terminal;
pub mod text;
pub mod theme;
pub mod updater;

pub fn clear_area(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, _theme: &theme::Theme) {
    frame.render_widget(ratatui::widgets::Clear, area);
}

pub mod screens {
    pub mod details;
    pub mod help;
    pub mod home;
    pub mod library;
    pub mod startup;
}
