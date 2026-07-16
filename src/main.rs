use moviebox_tui::tui::app::App;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new();
    let result = app.run(&mut terminal).await;

    ratatui::restore();

    result
}
