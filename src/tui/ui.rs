//! Rendering for the interactive browser. Reads [`App`] state and paints a header,
//! a size-ranked table, and a key-hint footer. No state mutation here.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState};

use super::app::{App, View};
use crate::format;

const BAR_WIDTH: usize = 12;

pub fn render(f: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());

    f.render_widget(header_widget(app), header);
    render_table(f, app, body);
    f.render_widget(footer_widget(app), footer);
}

fn header_widget(app: &App) -> Paragraph<'_> {
    let location = match app.view {
        View::Browser => app.tree.nodes[app.cwd].path.display().to_string(),
        View::Cleanable => "cleanable artifacts".to_string(),
    };
    let title = Line::from(vec![
        "disco ".bold(),
        Span::styled(location, Style::new().fg(Color::Cyan)),
    ]);

    let mut totals = vec![Span::styled(
        format!("{} on disk", format::size(app.tree.total_size())),
        Style::new().fg(Color::DarkGray),
    )];
    if !app.marked.is_empty() {
        totals.push(Span::raw("  ·  "));
        totals.push(Span::styled(
            format!(
                "{} marked to reclaim ({})",
                format::size(app.marked_size()),
                app.marked.len()
            ),
            Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    Paragraph::new(vec![title, Line::from(totals)])
}

fn render_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let scale = app.scale_size().max(1);
    let show_path = app.view == View::Cleanable;

    let rows = app.rows.iter().map(|&idx| {
        let n = &app.tree.nodes[idx];
        let marked = app.marked.contains(&idx);
        let ratio = n.size as f64 / scale as f64;

        let label = if show_path {
            n.path
                .strip_prefix(&app.tree.nodes[app.tree.root].path)
                .unwrap_or(&n.path)
                .display()
                .to_string()
        } else if n.is_dir {
            format!("{}/", n.name)
        } else {
            n.name.clone()
        };
        let name_style = if marked {
            Style::new().fg(Color::Red)
        } else if n.is_dir {
            Style::new().fg(Color::Cyan)
        } else {
            Style::new()
        };

        Row::new(vec![
            Cell::from(if marked { "✓" } else { " " }).style(Style::new().fg(Color::Red)),
            Cell::from(Line::from(format::size(n.size)).right_aligned()),
            Cell::from(format::bar(ratio, BAR_WIDTH)).style(Style::new().fg(Color::DarkGray)),
            Cell::from(label).style(name_style),
            Cell::from(n.kind.unwrap_or("")).style(Style::new().fg(Color::Yellow)),
            Cell::from(Line::from(format::age(n.mtime)).right_aligned())
                .style(Style::new().fg(Color::DarkGray)),
        ])
    });

    let widths = [
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Length(BAR_WIDTH as u16),
        Constraint::Min(10),
        Constraint::Length(12),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    let mut state = TableState::default();
    if !app.rows.is_empty() {
        state.select(Some(app.cursor));
    }
    f.render_stateful_widget(table, area, &mut state);
}

fn footer_widget(app: &App) -> Paragraph<'_> {
    let keys = match app.view {
        View::Browser => "↑↓ move · → enter · ← up · space mark · c cleanable · q quit",
        View::Cleanable => "↑↓ move · space mark · c browser · q quit",
    };
    Paragraph::new(Line::from(keys).style(Style::new().fg(Color::DarkGray)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        let p = std::env::temp_dir().join(format!("disco_ui_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("web/node_modules")).unwrap();
        fs::write(p.join("web/package.json"), "{}").unwrap();
        fs::write(p.join("web/node_modules/x.js"), vec![0u8; 100_000]).unwrap();
        p
    }

    /// Flatten the rendered buffer into a single string for substring assertions.
    fn rendered(app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(90, 12)).unwrap();
        terminal.draw(|f| render(f, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn browser_paints_header_and_rows() {
        let root = fixture();
        let app = App::new(crate::scan::scan(&root).unwrap());
        let out = rendered(&app);
        assert!(out.contains("disco"), "header present");
        assert!(out.contains("web/"), "directory row present: {out:?}");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cleanable_view_shows_kind_tag() {
        let root = fixture();
        let mut app = App::new(crate::scan::scan(&root).unwrap());
        app.toggle_view();
        let out = rendered(&app);
        assert!(out.contains("Node"), "kind tag present: {out:?}");
        assert!(out.contains("node_modules"), "artifact path present");
        fs::remove_dir_all(&root).ok();
    }
}
