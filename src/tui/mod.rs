//! Interactive terminal UI: owns the terminal lifecycle and the input loop, and
//! delegates state to [`app::App`] and painting to [`ui`].

mod app;
mod ui;

pub use app::App;
use app::View;

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
    // Any key dismisses a transient status message.
    app.status = None;

    // The confirmation prompt is a modal: only yes/no get through.
    if app.confirming {
        match code {
            KeyCode::Char('y') => app.confirm_reclaim(),
            KeyCode::Char('n') | KeyCode::Esc => app.cancel_reclaim(),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.move_cursor(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_cursor(-1),
        // In the flat cleanable view there's nothing to descend into, so Enter
        // reveals instead of drilling.
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter if app.view == View::Browser => {
            app.enter()
        }
        KeyCode::Enter => app.reveal(),
        KeyCode::Left | KeyCode::Char('h') => app.leave(),
        KeyCode::Char('o') | KeyCode::Char('f') => app.reveal(),
        KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_mark(),
        KeyCode::Char('c') | KeyCode::Tab => app.toggle_view(),
        KeyCode::Char('d') => app.request_reclaim(),
        KeyCode::Char('g') => app.cursor_to(0),
        KeyCode::Char('G') => app.cursor_to(usize::MAX),
        _ => {}
    }
}
