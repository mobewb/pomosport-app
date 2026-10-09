mod app;
mod ui;

use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, Config};

#[derive(Parser)]
#[command(about = "Pomodoro timer with an exercise wheel")]
struct Cli {
    /// Work session length in minutes
    #[arg(long, default_value_t = 25.0)]
    work: f64,
    /// Short break length in minutes
    #[arg(long, default_value_t = 5.0)]
    short: f64,
    /// Long break length in minutes
    #[arg(long, default_value_t = 15.0)]
    long: f64,
}

fn minutes(m: f64) -> Duration {
    Duration::from_secs_f64((m * 60.0).max(0.0))
}

/// Terminal bell plus a macOS notification; failures are ignored.
fn notify() {
    let _ = io::stdout().write_all(b"\x07");
    let _ = io::stdout().flush();
    let _ = Command::new("osascript")
        .args([
            "-e",
            "display notification \"Time for an exercise!\" with title \"pomosport\"",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Short clack for a wheel divider. Skipped while the previous one still plays.
/// To mute, make this function return early.
fn clack(playing: &mut Option<Child>) {
    if let Some(c) = playing {
        if matches!(c.try_wait(), Ok(None)) {
            return;
        }
    }
    *playing = Command::new("afplay")
        .arg("/System/Library/Sounds/Tink.aiff")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(LeaveAlternateScreen);
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut app = App::new(Config {
        work: minutes(cli.work),
        short: minutes(cli.short),
        long: minutes(cli.long),
    });

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = run(&mut terminal, &mut app);
    restore();
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut last = Instant::now();
    let mut playing = None;
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char(' ') => app.toggle(),
                        KeyCode::Char('s') => app.skip(),
                        KeyCode::Char('r') => app.reset(),
                        KeyCode::Enter => app.enter(),
                        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
                        _ => {}
                    }
                }
            }
        }
        let now = Instant::now();
        if app.tick(now - last) {
            notify();
        }
        last = now;
        if app.take_clacks() > 0 {
            clack(&mut playing);
        }
    }
    Ok(())
}
