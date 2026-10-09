use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::app::{App, Phase, EXERCISES};

pub fn draw(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 50, 18);
    let [title, timer, gauge, count, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    let (name, color) = match app.phase {
        Phase::Work => ("WORK", Color::Red),
        Phase::Wheel => ("EXERCISE TIME", Color::Yellow),
        Phase::Break { long: false } => ("SHORT BREAK", Color::Green),
        Phase::Break { long: true } => ("LONG BREAK", Color::Cyan),
    };
    let bold = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let center = |s: String, st: Style| Paragraph::new(s).style(st).alignment(Alignment::Center);

    f.render_widget(center(name.into(), bold), title);

    let secs = app.remaining.as_secs() + u64::from(app.remaining.subsec_nanos() > 0);
    let clock = format!("{:02}:{:02}", secs / 60, secs % 60);
    f.render_widget(center(clock, bold), timer);

    let total = app.phase_total().as_secs_f64();
    let ratio = if total > 0.0 {
        (1.0 - app.remaining.as_secs_f64() / total).clamp(0.0, 1.0)
    } else {
        1.0
    };
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(color))
            .ratio(ratio),
        gauge,
    );

    let state = if app.phase != Phase::Wheel && !app.running {
        " (paused)"
    } else {
        ""
    };
    f.render_widget(
        center(
            format!("Sessions completed: {}{state}", app.completed),
            Style::default(),
        ),
        count,
    );

    if let Some(w) = &app.wheel {
        let mut lines: Vec<Line> = EXERCISES
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if i == w.sector() {
                    Line::styled(
                        format!("> {e} <"),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    Line::from(format!("  {e}  "))
                }
            })
            .collect();
        lines.push(Line::from(""));
        lines.push(match app.result() {
            Some(r) => Line::styled(format!("Do {r}! Enter to start break"), bold),
            None => Line::from("Spinning..."),
        });
        f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), body);
    }

    let hints = "Space start/pause  s skip  r reset  Enter continue  q quit";
    f.render_widget(
        center(hints.into(), Style::default().fg(Color::DarkGray)),
        footer,
    );
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}
