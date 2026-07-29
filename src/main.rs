mod app;
mod ui;
mod update;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let icon_setting = std::env::var("TUMETO_ICONS").ok();
    let ui_config = ui::UiConfig::from_icon_setting(icon_setting.as_deref());

    enable_raw_mode()?;
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::load();
    let result = run_app(&mut terminal, &mut app, ui_config);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("오류 발생: {err}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ui_config: ui::UiConfig,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        app.tick_timer(now.saturating_duration_since(last_tick));
        last_tick = now;

        terminal.draw(|frame| ui::render(frame, app, ui_config))?;
        update::handle_events(app)?;

        if app.dirty {
            app.save()?;
            app.dirty = false;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
