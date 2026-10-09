use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use serde::Deserialize;

use crate::app::Config;

#[derive(Parser)]
#[command(about = "Pomodoro timer with an exercise wheel")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    /// Work session length in minutes [default: 25]
    #[arg(long, value_parser = parse_minutes)]
    work: Option<f64>,
    /// Short break length in minutes [default: 5]
    #[arg(long, value_parser = parse_minutes)]
    short: Option<f64>,
    /// Long break length in minutes [default: 15]
    #[arg(long, value_parser = parse_minutes)]
    long: Option<f64>,
    /// Work sessions before a long break [default: 4]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=99))]
    cycles: Option<u32>,
    /// Comma-separated exercises for the wheel (1 to 8)
    #[arg(long, value_parser = parse_exercises)]
    exercises: Option<Vec<String>>,
    /// Start timers without pressing Space
    #[arg(long)]
    auto_start: bool,
    /// Disable all sounds (clacks, fanfare, bell)
    #[arg(long)]
    mute: bool,
    /// Disable desktop notifications
    #[arg(long)]
    no_notify: bool,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Show completed-session statistics from the history file
    Stats,
}

/// Settings from `~/.config/pomosport/config.toml`; the command line overrides them.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    work: Option<f64>,
    short: Option<f64>,
    long: Option<f64>,
    cycles: Option<u32>,
    exercises: Option<Vec<String>>,
    auto_start: Option<bool>,
    mute: Option<bool>,
    no_notify: Option<bool>,
}

pub struct Settings {
    pub config: Config,
    pub mute: bool,
    pub no_notify: bool,
}

/// Accepts finite minutes in (0, 1440]; rejects 0, negatives, NaN and inf.
pub fn check_minutes(m: f64) -> Result<f64, String> {
    if m.is_finite() && m > 0.0 && m <= 1440.0 {
        Ok(m)
    } else {
        Err("must be greater than 0 and at most 1440 minutes".into())
    }
}

pub fn parse_minutes(s: &str) -> Result<f64, String> {
    check_minutes(s.parse().map_err(|_| format!("`{s}` is not a number"))?)
}

fn check_exercises(list: Vec<String>) -> Result<Vec<String>, String> {
    if (1..=8).contains(&list.len()) {
        Ok(list)
    } else {
        Err("give between 1 and 8 exercises".into())
    }
}

/// Splits a comma-separated list, ignoring blanks; 1 to 8 entries fit the wheel.
pub fn parse_exercises(s: &str) -> Result<Vec<String>, String> {
    check_exercises(
        s.split(',')
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .map(String::from)
            .collect(),
    )
}

pub fn default_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/pomosport/config.toml"))
}

/// A missing file means defaults; an unreadable or invalid one is an error.
pub fn load(path: &Path) -> Result<FileConfig, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileConfig::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn minutes(m: f64) -> Result<Duration, String> {
    check_minutes(m).map(|m| Duration::from_secs_f64(m * 60.0))
}

/// Merge: command line, then file, then built-in defaults.
pub fn resolve(cli: Cli, file: FileConfig) -> Result<Settings, String> {
    let d = Config::default();
    let dur = |cli: Option<f64>, file: Option<f64>, default: Duration| match cli.or(file) {
        Some(m) => minutes(m),
        None => Ok(default),
    };
    let cycles = cli.cycles.or(file.cycles).unwrap_or(d.cycles);
    if !(1..=99).contains(&cycles) {
        return Err("cycles must be between 1 and 99".into());
    }
    let exercises = match cli.exercises.or(file.exercises) {
        Some(list) => check_exercises(list.into_iter().map(|e| e.trim().to_string()).collect())?,
        None => d.exercises,
    };
    Ok(Settings {
        config: Config {
            work: dur(cli.work, file.work, d.work)?,
            short: dur(cli.short, file.short, d.short)?,
            long: dur(cli.long, file.long, d.long)?,
            cycles,
            exercises,
            auto_start: cli.auto_start || file.auto_start.unwrap_or(false),
        },
        mute: cli.mute || file.mute.unwrap_or(false),
        no_notify: cli.no_notify || file.no_notify.unwrap_or(false),
    })
}
