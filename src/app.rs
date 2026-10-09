use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;

pub const EXERCISES: [&str; 4] = ["10 push-ups", "20 squats", "10 crunches", "5 burpees"];

pub const SECTOR_DEG: f64 = 360.0 / EXERCISES.len() as f64;

/// Sector under the pointer when the wheel is at `theta` degrees.
pub fn sector_of(theta: f64) -> usize {
    ((theta.rem_euclid(360.0) / SECTOR_DEG) as usize).min(EXERCISES.len() - 1)
}

/// A wheel spinning with an ease-out curve towards a pre-chosen sector.
/// `theta` only grows; a sector divider passes the pointer each time it
/// crosses a multiple of `SECTOR_DEG`.
pub struct Wheel {
    pub theta: f64,
    pub landed: bool,
    pub target: usize,
    start: f64,
    total: f64,
    duration: Duration,
    elapsed: Duration,
}

impl Wheel {
    fn new(rng: &mut StdRng) -> Self {
        let start = rng.gen_range(0.0..360.0);
        let target = rng.gen_range(0..EXERCISES.len());
        let end_local = target as f64 * SECTOR_DEG + rng.gen_range(0.15..0.85) * SECTOR_DEG;
        let turns = f64::from(rng.gen_range(3..=5));
        Self {
            theta: start,
            landed: false,
            target,
            start,
            total: (end_local - start).rem_euclid(360.0) + 360.0 * turns,
            duration: Duration::from_millis(rng.gen_range(4500..6500)),
            elapsed: Duration::ZERO,
        }
    }

    pub fn sector(&self) -> usize {
        sector_of(self.theta)
    }

    /// Advance the spin; returns how many dividers passed the pointer.
    fn advance(&mut self, dt: Duration) -> u32 {
        if self.landed {
            return 0;
        }
        let prev = self.theta;
        self.elapsed = (self.elapsed + dt).min(self.duration);
        let left = 1.0 - self.elapsed.as_secs_f64() / self.duration.as_secs_f64();
        self.theta = self.start + self.total * (1.0 - left.powi(3));
        self.landed = self.elapsed >= self.duration;
        ((self.theta / SECTOR_DEG).floor() - (prev / SECTOR_DEG).floor()) as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Work,
    Wheel,
    Break { long: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub work: Duration,
    pub short: Duration,
    pub long: Duration,
}

pub struct App {
    pub cfg: Config,
    pub phase: Phase,
    pub remaining: Duration,
    pub running: bool,
    /// Work sessions completed so far.
    pub completed: u32,
    pub quit: bool,
    pub wheel: Option<Wheel>,
    /// Dividers that passed the pointer since the last `take_clacks`.
    clacks: u32,
    rng: StdRng,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        Self::with_seed(cfg, rand::random())
    }

    pub fn with_seed(cfg: Config, seed: u64) -> Self {
        Self {
            cfg,
            phase: Phase::Work,
            remaining: cfg.work,
            running: false,
            completed: 0,
            quit: false,
            wheel: None,
            clacks: 0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Total length of the current timed phase (0 during the wheel).
    pub fn phase_total(&self) -> Duration {
        match self.phase {
            Phase::Work => self.cfg.work,
            Phase::Break { long: false } => self.cfg.short,
            Phase::Break { long: true } => self.cfg.long,
            Phase::Wheel => Duration::ZERO,
        }
    }

    pub fn toggle(&mut self) {
        if self.phase != Phase::Wheel {
            self.running = !self.running;
        }
    }

    pub fn reset(&mut self) {
        let seed = self.rng.gen();
        *self = Self::with_seed(self.cfg, seed);
    }

    pub fn skip(&mut self) {
        match self.phase {
            Phase::Work => self.finish_work(),
            Phase::Wheel => self.enter(),
            Phase::Break { .. } => self.start_work(),
        }
    }

    /// Continue from the wheel to the break.
    pub fn enter(&mut self) {
        if self.phase == Phase::Wheel && self.wheel.as_ref().is_some_and(|w| w.landed) {
            let long = self.completed.is_multiple_of(4);
            self.phase = Phase::Break { long };
            self.remaining = if long { self.cfg.long } else { self.cfg.short };
            self.running = true;
            self.wheel = None;
        }
    }

    pub fn take_clacks(&mut self) -> u32 {
        std::mem::take(&mut self.clacks)
    }

    /// The exercise the wheel landed on, once it has stopped.
    pub fn result(&self) -> Option<&'static str> {
        self.wheel
            .as_ref()
            .filter(|w| w.landed)
            .map(|w| EXERCISES[w.sector()])
    }

    /// Advance time; returns true when a work session just ended on its own.
    pub fn tick(&mut self, dt: Duration) -> bool {
        if let Some(w) = self.wheel.as_mut() {
            self.clacks += w.advance(dt);
        }
        if !self.running || self.phase == Phase::Wheel {
            return false;
        }
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            if self.phase == Phase::Work {
                self.finish_work();
                return true;
            }
            self.start_work();
        }
        false
    }

    fn finish_work(&mut self) {
        self.completed += 1;
        self.phase = Phase::Wheel;
        self.running = false;
        self.remaining = Duration::ZERO;
        self.wheel = Some(Wheel::new(&mut self.rng));
    }

    fn start_work(&mut self) {
        self.phase = Phase::Work;
        self.remaining = self.cfg.work;
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);

    fn app() -> App {
        App::new(Config {
            work: 10 * S,
            short: 3 * S,
            long: 6 * S,
        })
    }

    #[test]
    fn work_expires_into_wheel_then_short_break() {
        let mut a = app();
        a.toggle();
        a.tick(10 * S);
        assert_eq!(a.phase, Phase::Wheel);
        assert_eq!(a.completed, 1);
        a.tick(60 * S);
        a.enter();
        assert_eq!(a.phase, Phase::Break { long: false });
        assert_eq!(a.remaining, 3 * S);
    }

    #[test]
    fn fourth_break_is_long() {
        let mut a = app();
        for i in 1..=4 {
            a.toggle();
            a.tick(10 * S);
            a.tick(60 * S);
            a.enter();
            assert_eq!(a.phase, Phase::Break { long: i == 4 });
            a.tick(10 * S);
            assert_eq!(a.phase, Phase::Work);
        }
    }

    #[test]
    fn timer_counts_down_and_expires() {
        let mut a = app();
        a.toggle();
        a.tick(4 * S);
        assert_eq!(a.remaining, 6 * S);
        a.tick(6 * S);
        assert_eq!(a.phase, Phase::Wheel);
    }

    #[test]
    fn tick_reports_work_end_only() {
        let mut a = app();
        a.toggle();
        assert!(!a.tick(4 * S));
        assert!(a.tick(6 * S));
        a.tick(60 * S);
        a.enter();
        assert!(!a.tick(10 * S));
    }

    #[test]
    fn pause_stops_countdown() {
        let mut a = app();
        a.tick(5 * S);
        assert_eq!(a.remaining, 10 * S, "not started yet");
        a.toggle();
        a.tick(2 * S);
        a.toggle();
        a.tick(5 * S);
        assert_eq!(a.remaining, 8 * S);
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut a = app();
        a.toggle();
        a.tick(10 * S);
        a.reset();
        assert_eq!(a.phase, Phase::Work);
        assert_eq!(a.completed, 0);
        assert!(!a.running);
    }

    #[test]
    fn angle_maps_to_sector() {
        assert_eq!(sector_of(0.0), 0);
        assert_eq!(sector_of(89.9), 0);
        assert_eq!(sector_of(90.0), 1);
        assert_eq!(sector_of(359.0), 3);
        assert_eq!(sector_of(360.0), 0);
        assert_eq!(sector_of(450.0), 1);
        assert_eq!(sector_of(-10.0), 3);
    }

    fn spun(seed: u64) -> App {
        let mut a = App::with_seed(
            Config {
                work: S,
                short: S,
                long: S,
            },
            seed,
        );
        a.toggle();
        a.tick(S);
        a
    }

    #[test]
    fn wheel_lands_on_target_sector_and_stops() {
        for seed in 0..50 {
            let mut a = spun(seed);
            assert!(a.result().is_none());
            for _ in 0..200 {
                a.tick(Duration::from_millis(50));
            }
            let w = a.wheel.as_ref().unwrap();
            assert!(w.landed);
            assert_eq!(w.sector(), w.target);
            let theta = w.theta;
            a.tick(10 * S);
            assert_eq!(a.wheel.as_ref().unwrap().theta, theta, "stays put");
            assert_eq!(
                a.result(),
                Some(EXERCISES[a.wheel.as_ref().unwrap().target])
            );
        }
    }

    #[test]
    fn boundary_crossings_match_expected() {
        for (seed, step_ms) in [(1, 10), (2, 50), (3, 333)] {
            let mut a = spun(seed);
            let w = a.wheel.as_ref().unwrap();
            let first = (w.theta / SECTOR_DEG).floor();
            let mut seen = 0;
            for _ in 0..(10_000 / step_ms) {
                a.tick(Duration::from_millis(step_ms));
                seen += a.take_clacks();
            }
            let w = a.wheel.as_ref().unwrap();
            assert!(w.landed);
            assert_eq!(f64::from(seen), (w.theta / SECTOR_DEG).floor() - first);
            assert!(seen >= 12, "at least 3 full turns of 4 dividers");
            assert_eq!(a.take_clacks(), 0);
        }
    }

    #[test]
    fn spin_decelerates_and_terminates() {
        let mut a = spun(7);
        let dt = Duration::from_millis(50);
        let mut prev = a.wheel.as_ref().unwrap().theta;
        let mut last_step = f64::MAX;
        let mut ticks = 0;
        while !a.wheel.as_ref().unwrap().landed {
            a.tick(dt);
            ticks += 1;
            let t = a.wheel.as_ref().unwrap().theta;
            let step = t - prev;
            assert!(step <= last_step + 1e-9, "never speeds up");
            (prev, last_step) = (t, step);
            assert!(ticks <= 140, "lands within 7s");
        }
    }

    #[test]
    fn enter_ignored_while_spinning() {
        let mut a = app();
        a.toggle();
        a.tick(10 * S);
        a.enter();
        assert_eq!(a.phase, Phase::Wheel);
    }
}
