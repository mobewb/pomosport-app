//! Shared fixtures for the integration tests.
#![allow(dead_code)] // each test crate uses a different subset

use std::time::Duration;

use pomosport::app::{App, Config};

pub const S: Duration = Duration::from_secs(1);

/// Tick in steps below the app's per-tick cap; true if any tick reported a work end.
pub fn adv(a: &mut App, total: Duration) -> bool {
    let mut ended = false;
    let mut left = total;
    while !left.is_zero() {
        let step = left.min(S);
        ended |= a.tick(step);
        left -= step;
    }
    ended
}

/// 1 second for every phase.
pub fn tcfg() -> Config {
    Config {
        work: S,
        short: S,
        long: S,
        ..Config::default()
    }
}

/// 10 s work, 3 s short break, 6 s long break.
pub fn app() -> App {
    App::new(Config {
        work: 10 * S,
        short: 3 * S,
        long: 6 * S,
        ..Config::default()
    })
}

/// App whose first work session has just ended, so the wheel is spinning.
pub fn spun_app(seed: u64) -> App {
    let mut a = App::with_seed(tcfg(), seed);
    a.toggle();
    a.tick(S);
    a
}
