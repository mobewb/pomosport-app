use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::Line;
use ratatui::widgets::canvas::{Canvas, Circle, Line as CLine};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::app::{App, Phase, Wheel, EXERCISES, SECTOR_DEG};

pub fn draw(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 64, if app.wheel.is_some() { 32 } else { 18 });
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
        let [wheel_area, banner] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(body);
        draw_wheel(f, w, wheel_area);
        let line = match app.result() {
            Some(r) => Line::styled(format!("Do {r}! Enter to start break"), bold),
            None => Line::from("Spinning..."),
        };
        f.render_widget(Paragraph::new(line).alignment(Alignment::Center), banner);
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

const R: f64 = 1.0;
const BOUND: f64 = 1.25;

/// Circular wheel on a Canvas. Sector `i` spans local angles
/// `[i, i+1) * SECTOR_DEG`; the wheel is turned so that local angle `theta`
/// sits under the fixed pointer at the top (screen angle 90 degrees).
fn draw_wheel(f: &mut Frame, w: &Wheel, area: Rect) {
    // Braille dots are square when the cell area is twice as wide as tall.
    let h = area.height.min(area.width / 2);
    let area = centered(area, h * 2, h);
    let per_cell = 2.0 * BOUND / f64::from(area.width.max(1));
    let screen = |local: f64| (90.0 + w.theta - local).to_radians();
    let n = EXERCISES.len();

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([-BOUND, BOUND])
        .y_bounds([-BOUND, BOUND])
        .paint(|ctx| {
            if w.landed {
                let from = w.sector() as f64 * SECTOR_DEG;
                let mut a = from;
                while a <= from + SECTOR_DEG {
                    let (s, c) = screen(a).sin_cos();
                    ctx.draw(&CLine::new(0.0, 0.0, R * c, R * s, Color::Yellow));
                    a += 1.0;
                }
            }
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: R,
                color: Color::White,
            });
            for i in 0..n {
                let (s, c) = screen(i as f64 * SECTOR_DEG).sin_cos();
                ctx.draw(&CLine::new(0.0, 0.0, R * c, R * s, Color::White));
            }
            for (i, name) in EXERCISES.iter().enumerate() {
                let (s, c) = screen((i as f64 + 0.5) * SECTOR_DEG).sin_cos();
                let x = 0.58 * c - name.len() as f64 * per_cell / 2.0;
                let style = if w.landed && i == w.sector() {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                ctx.print(x, 0.58 * s, Line::styled(*name, style));
            }
            // Fixed pointer above the rim, tip pointing down into the wheel.
            let (l, r, top, tip) = ((-0.09, 1.2), (0.09, 1.2), (0.0, 1.2), (0.0, 1.02));
            ctx.draw(&CLine::new(l.0, l.1, tip.0, tip.1, Color::Red));
            ctx.draw(&CLine::new(r.0, r.1, tip.0, tip.1, Color::Red));
            ctx.draw(&CLine::new(l.0, l.1, r.0, r.1, Color::Red));
            ctx.draw(&CLine::new(top.0, top.1, tip.0, tip.1, Color::Red));
        });
    f.render_widget(canvas, area);
}
