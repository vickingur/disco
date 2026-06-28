//! Interactive terminal UI: owns the terminal lifecycle and the input loop, and
//! delegates state to [`app::App`] and painting to [`ui`].

mod app;
mod ui;

pub use app::App;
use app::View;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::scan::{self, Progress, Tree};

/// Scan `root` (showing a live, cancellable scanning screen) and then browse it.
/// Restores the terminal on exit, including panics, via `ratatui::init`'s hook.
pub fn run(root: PathBuf) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = scan_then_browse(&mut terminal, root);
    ratatui::restore();
    result
}

/// Phase 1: walk the tree on a worker thread while animating progress, abortable with
/// `q`/`Esc`. Phase 2: hand the finished tree to the browser loop.
fn scan_then_browse(terminal: &mut ratatui::DefaultTerminal, root: PathBuf) -> Result<()> {
    let progress = Arc::new(Progress::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    {
        let root = root.clone();
        let progress = Arc::clone(&progress);
        let cancel = Arc::clone(&cancel);
        thread::spawn(move || {
            let _ = tx.send(scan::scan_with_progress(&root, &progress, &cancel));
        });
    }

    let mut frame = 0usize;
    let tree: Tree = loop {
        terminal.draw(|f| ui::scanning(f, &root, &progress, frame))?;
        match rx.try_recv() {
            Ok(result) => break result?,
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                anyhow::bail!("scan worker stopped unexpectedly")
            }
        }
        if event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            // Ask the worker to stop, wait for it to unwind, then exit cleanly.
            cancel.store(true, Ordering::Relaxed);
            let _ = rx.recv();
            return Ok(());
        }
        frame += 1;
    };

    event_loop(terminal, App::new(tree))
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
