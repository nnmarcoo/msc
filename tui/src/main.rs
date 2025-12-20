mod app;

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app::App::new()?.run(terminal);
    ratatui::restore();
    result
}
