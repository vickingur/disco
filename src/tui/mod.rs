//! Interactive terminal UI: owns the terminal lifecycle and the input loop, and
//! delegates state to [`app::App`] and painting to [`ui`].

mod app;
mod ui;

pub use app::App;

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::scan::Tree;

/// Launch the browser over `tree`, restoring the terminal on exit (including panics,
/// via the hook installed by `ratatui::init`).
pub fn run(tree: Tree) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, App::new(tree));
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;
        // Wake periodically so the age column stays current even when idle.
        if event::poll(Duration::from_millis(500))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_key(&mut app, key.code);
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor(-1),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => app.enter(),
        KeyCode::Left | KeyCode::Char('h') => app.leave(),
        KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_mark(),
        KeyCode::Char('c') | KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('g') => app.cursor_to(0),
        KeyCode::Char('G') => app.cursor_to(usize::MAX),
        _ => {}
    }
}
