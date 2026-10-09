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

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Cli {
        Cli::parse_from(std::iter::once("pomosport").chain(args.iter().copied()))
    }

    fn file(text: &str) -> FileConfig {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn exercises_parser_trims_and_bounds() {
        assert_eq!(parse_exercises(" 5 a , ,b ").unwrap(), ["5 a", "b"]);
        assert!(parse_exercises(" , ").is_err());
        assert!(parse_exercises("1,2,3,4,5,6,7,8,9").is_err());
    }

    #[test]
    fn minutes_parser_accepts_only_sane_values() {
        for ok in ["25", "0.1", "1440"] {
            assert!(parse_minutes(ok).is_ok(), "{ok}");
        }
        for bad in ["0", "-1", "NaN", "inf", "1441", "abc", ""] {
            assert!(parse_minutes(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn defaults_when_nothing_given() {
        let s = resolve(cli(&[]), FileConfig::default()).unwrap();
        assert_eq!(s.config.work, Duration::from_secs(25 * 60));
        assert_eq!(s.config.cycles, 4);
        assert!(!s.mute && !s.no_notify && !s.config.auto_start);
    }

    #[test]
    fn file_values_apply_and_cli_overrides_them() {
        let f = file("work = 50\ncycles = 3\nmute = true\nexercises = [\"a\", \"b\"]");
        let s = resolve(cli(&["--work", "10"]), f).unwrap();
        assert_eq!(s.config.work, Duration::from_secs(600));
        assert_eq!(s.config.cycles, 3);
        assert_eq!(s.config.exercises, ["a", "b"]);
        assert!(s.mute);
    }

    #[test]
    fn invalid_file_values_are_rejected() {
        assert!(resolve(cli(&[]), file("work = -5")).is_err());
        assert!(resolve(cli(&[]), file("cycles = 0")).is_err());
        assert!(resolve(cli(&[]), file("exercises = []")).is_err());
    }

    #[test]
    fn load_reads_temp_file_and_handles_missing_and_bad() {
        let dir = std::env::temp_dir().join(format!("pomosport-cfg-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ok = dir.join("ok.toml");
        std::fs::write(&ok, "short = 2\n").unwrap();
        assert_eq!(load(&ok).unwrap().short, Some(2.0));
        assert_eq!(
            load(&dir.join("missing.toml")).unwrap(),
            FileConfig::default()
        );
        let bad = dir.join("bad.toml");
        std::fs::write(&bad, "wrok = 1\n").unwrap();
        assert!(load(&bad).is_err(), "unknown keys are errors");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
