use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use pomosport::app::App;
use pomosport::config::{self, Cli, FileConfig};
use pomosport::platform::{clack, notify, win, BREAK_DONE, WORK_DONE};
use pomosport::{history, ui};

fn restore() {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(LeaveAlternateScreen);
}

/// Restores the terminal on drop, so an early `?` return can't leave raw mode on.
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if cli.command.is_some() {
        let entries = history::default_path()
            .map(|p| history::load(&p))
            .unwrap_or_default();
        print!(
            "{}",
            history::render(&history::summarize(&entries, history::now()))
        );
        return Ok(());
    }
    let file = match config::default_path() {
        Some(path) => config::load(&path),
        None => Ok(FileConfig::default()),
    };
    let settings = match file.and_then(|file| config::resolve(cli, file)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pomosport: {e}");
            std::process::exit(2);
        }
    };
    let (mute, no_notify) = (settings.mute, settings.no_notify);
    let history_path = history::default_path();
    let mut app = App::new(settings.config);
    if let Some(path) = &history_path {
        app.today = history::summarize(&history::load(path), history::now()).today;
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let _guard = TermGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    run(
        &mut terminal,
        &mut app,
        mute,
        no_notify,
        history_path.as_deref(),
    )
}

/// Log the finished session; history is best effort and never interrupts the timer.
fn record(app: &App, path: Option<&Path>) {
    if let (Some(path), Some(exercise)) = (path, app.pending_exercise()) {
        let entry = history::Entry {
            ts: history::now(),
            exercise: exercise.to_string(),
            work_minutes: app.cfg.work.as_secs_f64() / 60.0,
        };
        let _ = history::append(path, &entry);
    }
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mute: bool,
    no_notify: bool,
    history_path: Option<&Path>,
) -> io::Result<()> {
    let mut last = Instant::now();
    let mut playing = Vec::new();
    let mut notifying = Vec::new();
    let mut winning = None;
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                app.on_key(k);
            }
        }
        let now = Instant::now();
        if app.tick(now - last) {
            // Log as soon as work ends so quitting mid-spin can't lose it.
            record(app, history_path);
            app.today += 1;
            notify(WORK_DONE, mute, no_notify, &mut notifying);
        }
        if app.take_break_ended() {
            notify(BREAK_DONE, mute, no_notify, &mut notifying);
        }
        last = now;
        let clacks = app.take_clacks();
        let landed = app.take_wheel_landed();
        if !mute {
            clack(&mut playing, clacks);
            if landed {
                win(&mut winning);
            }
        }
    }
    Ok(())
}
