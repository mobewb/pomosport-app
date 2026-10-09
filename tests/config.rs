use std::time::Duration;

use clap::Parser;
use pomosport::config::{load, resolve, Cli, FileConfig};

fn cli(args: &[&str]) -> Cli {
    Cli::parse_from(std::iter::once("pomosport").chain(args.iter().copied()))
}

fn file(text: &str) -> FileConfig {
    toml::from_str(text).unwrap()
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
    assert_eq!(
        resolve(cli(&[]), load(&ok).unwrap()).unwrap().config.short,
        Duration::from_secs(120)
    );
    assert_eq!(
        load(&dir.join("missing.toml")).unwrap(),
        FileConfig::default()
    );
    let bad = dir.join("bad.toml");
    std::fs::write(&bad, "wrok = 1\n").unwrap();
    assert!(load(&bad).is_err(), "unknown keys are errors");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cli_can_override_file_booleans_both_ways() {
    let f = || file("mute = true\nauto_start = true\nno_notify = true");
    let s = resolve(cli(&[]), f()).unwrap();
    assert!(s.mute && s.no_notify && s.config.auto_start, "file applies");
    let s = resolve(cli(&["--no-mute", "--no-auto-start", "--notify"]), f()).unwrap();
    assert!(
        !s.mute && !s.no_notify && !s.config.auto_start,
        "cli turns off"
    );
    let s = resolve(
        cli(&["--mute", "--auto-start", "--no-notify"]),
        file("mute = false\nauto_start = false\nno_notify = false"),
    )
    .unwrap();
    assert!(s.mute && s.no_notify && s.config.auto_start, "cli turns on");
}

#[test]
fn contradictory_flags_are_rejected() {
    for pair in [
        ["--mute", "--no-mute"],
        ["--auto-start", "--no-auto-start"],
        ["--no-notify", "--notify"],
    ] {
        assert!(Cli::try_parse_from(["pomosport", pair[0], pair[1]]).is_err());
    }
}
