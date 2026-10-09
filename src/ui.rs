use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Phase};

const WIDTH: u16 = 60;
const HEIGHT: u16 = 18;
const HINTS: &str = "Space start/pause  s skip  r reset  Enter continue  q quit";

pub fn draw(f: &mut Frame, app: &App) {
    let full = f.area();
    if full.width < WIDTH || full.height < HEIGHT {
        let msg = format!(
            "Terminal too small: need {WIDTH}x{HEIGHT}, have {}x{}",
            full.width, full.height
        );
        f.render_widget(
            Paragraph::new(msg)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center),
            full,
        );
        return;
    }
    let area = centered(full, WIDTH, HEIGHT);
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
    let status = match app.phase {
        Phase::Wheel => "",
        _ if app.running => "",
        _ if app.remaining == app.phase_total() => "  ready",
        _ => "  paused",
    };
    f.render_widget(center(format!("{clock}{status}"), bold), timer);

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

    let done = app.cycle_progress() as usize;
    let dots: String = (0..app.cfg.cycles as usize)
        .map(|i| if i < done { '●' } else { '○' })
        .collect();
    let next = if matches!(app.phase, Phase::Break { .. }) {
        String::new()
    } else {
        format!(
            "  next: {} break",
            if app.next_break_long() {
                "long"
            } else {
                "short"
            }
        )
    };
    f.render_widget(
        center(
            format!("{dots}  today: {}{next}", app.today),
            Style::default(),
        ),
        count,
    );

    if let Some(w) = &app.wheel {
        let mut lines: Vec<Line> = app
            .cfg
            .exercises
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if i == w.pos {
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

    f.render_widget(
        center(HINTS.into(), Style::default().fg(Color::DarkGray)),
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

#[cfg(test)]
mod tests {
    use super::*;

    // Inline: needs the private layout constants.
    #[test]
    fn hints_fit_the_box() {
        assert!(HINTS.len() <= usize::from(WIDTH));
    }
}
