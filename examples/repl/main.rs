use std::{error::Error, io};

use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use ratatui_textarea::{Input, Key};

mod app;
mod ui;
use crate::{app::App, ui::ui};

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr(); // This is a special case. Normally using stdout is fine
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()>
where
    io::Error: From<B::Error>,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        let event = event::read()?;
        if matches!(&event, Event::Key(key) if key.kind == KeyEventKind::Release) {
            continue;
        }

        match event.into() {
            Input { key: Key::Esc, .. }
            | Input {
                key: Key::Char('c'),
                ctrl: true,
                ..
            } => return Ok(()),
            Input {
                key: Key::Enter, ..
            } => app.submit(),
            Input { key: Key::Up, .. } => app.history_prev(),
            Input { key: Key::Down, .. } => app.history_next(),
            // Ctrl+M inserts a newline in the textarea, keep the input single-line.
            Input {
                key: Key::Char('m'),
                ctrl: true,
                ..
            } => {}
            input => {
                // TextArea::input returns true if the input modified the text
                if app.textarea.input(input) {
                    app.on_edit();
                }
            }
        }
    }
}
