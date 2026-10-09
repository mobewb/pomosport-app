use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;

pub const EXERCISES: [&str; 4] = ["10 push-ups", "20 squats", "10 crunches", "5 burpees"];

/// State of the spinning wheel; each step takes longer than the last.
pub struct Wheel {
    pub pos: usize,
    pub landed: bool,
    steps_done: u32,
    steps_total: u32,
    acc: Duration,
}

impl Wheel {
    fn new(rng: &mut StdRng) -> Self {
        Self {
            pos: rng.gen_range(0..EXERCISES.len()),
            landed: false,
            steps_done: 0,
            steps_total: rng.gen_range(24..40),
            acc: Duration::ZERO,
        }
    }

    /// Advance the spin; returns how many highlight steps were taken.
    fn advance(&mut self, dt: Duration) -> u32 {
        let before = self.steps_done;
        self.acc += dt;
        while !self.landed {
            let interval = Duration::from_millis(40 + 6 * u64::from(self.steps_done));
            if self.acc < interval {
                break;
            }
            self.acc -= interval;
            self.pos = (self.pos + 1) % EXERCISES.len();
            self.steps_done += 1;
            self.landed = self.steps_done >= self.steps_total;
        }
        self.steps_done - before
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
    /// Highlight steps since the last `take_clacks`.
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
            .map(|w| EXERCISES[w.pos])
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
    fn wheel_lands_on_valid_index_and_stops() {
        for seed in 0..50 {
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
            assert!(a.result().is_none());
            for _ in 0..1000 {
                a.tick(Duration::from_millis(50));
            }
            let w = a.wheel.as_ref().unwrap();
            assert!(w.landed && w.pos < EXERCISES.len());
            let pos = w.pos;
            a.tick(10 * S);
            assert_eq!(a.wheel.as_ref().unwrap().pos, pos, "stays put");
            assert_eq!(a.result(), Some(EXERCISES[pos]));
        }
    }

    #[test]
    fn clacks_equal_highlight_steps() {
        for (seed, step_ms) in [(1, 10), (2, 50), (3, 333)] {
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
            let start = a.wheel.as_ref().unwrap().pos;
            let mut seen = 0;
            for _ in 0..(30_000 / step_ms) {
                a.tick(Duration::from_millis(step_ms));
                seen += a.take_clacks();
            }
            let w = a.wheel.as_ref().unwrap();
            assert!(w.landed);
            assert!((24..40).contains(&seen));
            assert_eq!(w.pos, (start + seen as usize) % EXERCISES.len());
            assert_eq!(a.take_clacks(), 0);
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
