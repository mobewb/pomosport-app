mod app;
mod config;
mod ui;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use config::{Cli, FileConfig};

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const WIN_SOUND: &[u8] = include_bytes!("../assets/win.wav");
const WORK_DONE: &str = "Time for an exercise!";
const BREAK_DONE: &str = "Break over, back to work!";

/// Write an embedded asset to a stable file in the temp dir so external tools
/// (terminal-notifier, afplay) can read it. Reused across runs when unchanged.
fn write_asset(name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let path = std::env::temp_dir().join(format!("pomosport-{}-{name}", env!("CARGO_PKG_VERSION")));
    let fresh = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64);
    if !fresh {
        std::fs::write(&path, bytes).ok()?;
    }
    Some(path)
}

fn icon_path() -> Option<&'static PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| write_asset("icon.png", ICON)).as_ref()
}

/// Terminal bell (unless muted) plus a macOS notification (unless disabled);
/// failures are ignored.
/// Uses `terminal-notifier` (custom icon) when installed, else `osascript`.
fn notify(message: &str, mute: bool, no_notify: bool) {
    if !mute {
        let _ = io::stdout().write_all(b"\x07");
        let _ = io::stdout().flush();
    }
    if no_notify {
        return;
    }
    let mut tn = Command::new("terminal-notifier");
    tn.args(["-title", "pomosport", "-message", message]);
    if let Some(icon) = icon_path() {
        tn.arg("-appIcon").arg(icon);
    }
    let sent = tn
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if !sent {
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!("display notification \"{message}\" with title \"pomosport\""),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

const MAX_CLACKS: usize = 8;

/// One short clack per highlight step, overlapping if needed.
/// Finished players are reaped; at most `MAX_CLACKS` run at once.
fn clack(playing: &mut Vec<Child>, steps: u32) {
    playing.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
    for _ in 0..steps {
        if playing.len() >= MAX_CLACKS {
            break;
        }
        let child = Command::new("afplay")
            .args(["-t", "0.15", "/System/Library/Sounds/Tink.aiff"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        playing.extend(child);
    }
}

/// Victory fanfare when the wheel lands. Fire and forget; it plays alongside
/// any clack still ringing, but never overlaps another fanfare.
fn win(playing: &mut Option<Child>) {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    if playing
        .as_mut()
        .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    {
        return;
    }
    let Some(path) = PATH.get_or_init(|| write_asset("win.wav", WIN_SOUND)) else {
        return;
    };
    *playing = Command::new("afplay")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
}

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
    let mut app = App::new(settings.config);

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let _guard = TermGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    run(&mut terminal, &mut app, mute, no_notify)
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mute: bool,
    no_notify: bool,
) -> io::Result<()> {
    let mut last = Instant::now();
    let mut playing = Vec::new();
    let mut winning = None;
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
                        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit = true
                        }
                        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
                        _ => {}
                    }
                }
            }
        }
        let now = Instant::now();
        if app.tick(now - last) {
            notify(WORK_DONE, mute, no_notify);
        }
        if app.take_break_ended() {
            notify(BREAK_DONE, mute, no_notify);
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
