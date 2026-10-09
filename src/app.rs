use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;

pub const DEFAULT_EXERCISES: [&str; 4] = ["10 push-ups", "20 squats", "10 crunches", "5 burpees"];

/// Longest time step applied in one tick, so a laptop sleep can't jump the timer.
const MAX_DT: Duration = Duration::from_secs(1);

/// State of the spinning wheel; each step takes longer than the last.
pub struct Wheel {
    pub pos: usize,
    pub landed: bool,
    len: usize,
    steps_done: u32,
    steps_total: u32,
    acc: Duration,
}

impl Wheel {
    fn new(rng: &mut StdRng, len: usize) -> Self {
        Self {
            pos: rng.gen_range(0..len),
            landed: false,
            len,
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
            self.pos = (self.pos + 1) % self.len;
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

#[derive(Clone, Debug)]
pub struct Config {
    pub work: Duration,
    pub short: Duration,
    pub long: Duration,
    /// Work sessions per cycle; the break after the last one is long.
    pub cycles: u32,
    pub exercises: Vec<String>,
    /// Start timers without pressing Space (work after a break, and at launch).
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work: Duration::from_secs(25 * 60),
            short: Duration::from_secs(5 * 60),
            long: Duration::from_secs(15 * 60),
            cycles: 4,
            exercises: DEFAULT_EXERCISES.map(String::from).to_vec(),
            auto_start: false,
        }
    }
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
    landed_event: bool,
    break_ended_event: bool,
    rng: StdRng,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        Self::with_seed(cfg, rand::random())
    }

    pub fn with_seed(cfg: Config, seed: u64) -> Self {
        Self {
            phase: Phase::Work,
            remaining: cfg.work,
            running: cfg.auto_start,
            completed: 0,
            quit: false,
            wheel: None,
            clacks: 0,
            landed_event: false,
            break_ended_event: false,
            rng: StdRng::seed_from_u64(seed),
            cfg,
        }
    }

    /// Sessions filled in the current cycle (a full cycle stays full until the
    /// next work session starts).
    pub fn cycle_progress(&self) -> u32 {
        let n = self.completed % self.cfg.cycles;
        if n == 0 && self.completed > 0 && self.phase != Phase::Work {
            self.cfg.cycles
        } else {
            n
        }
    }

    /// Whether the next break will be the long one.
    pub fn next_break_long(&self) -> bool {
        let sessions = self.completed + u32::from(self.phase == Phase::Work);
        sessions.is_multiple_of(self.cfg.cycles)
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
        *self = Self::with_seed(self.cfg.clone(), seed);
    }

    pub fn skip(&mut self) {
        match self.phase {
            // A skipped session earns no exercise and does not count.
            Phase::Work => self.start_break(),
            Phase::Wheel => self.enter(),
            Phase::Break { .. } => self.start_work(),
        }
    }

    /// Continue from the wheel to the break.
    pub fn enter(&mut self) {
        if self.phase == Phase::Wheel && self.wheel.as_ref().is_some_and(|w| w.landed) {
            self.start_break();
        }
    }

    fn start_break(&mut self) {
        let long = self.completed > 0 && self.completed.is_multiple_of(self.cfg.cycles);
        self.phase = Phase::Break { long };
        self.remaining = if long { self.cfg.long } else { self.cfg.short };
        self.running = true;
        self.wheel = None;
    }

    pub fn take_clacks(&mut self) -> u32 {
        std::mem::take(&mut self.clacks)
    }

    /// True once, right after the wheel stops on its result.
    pub fn take_wheel_landed(&mut self) -> bool {
        std::mem::take(&mut self.landed_event)
    }

    /// True once, right after a break runs out on its own.
    pub fn take_break_ended(&mut self) -> bool {
        std::mem::take(&mut self.break_ended_event)
    }

    /// The exercise the wheel landed on, once it has stopped.
    pub fn result(&self) -> Option<&str> {
        self.wheel
            .as_ref()
            .filter(|w| w.landed)
            .map(|w| self.cfg.exercises[w.pos].as_str())
    }

    /// Advance time; returns true when a work session just ended on its own.
    pub fn tick(&mut self, dt: Duration) -> bool {
        let dt = dt.min(MAX_DT);
        if let Some(w) = self.wheel.as_mut() {
            let was_landed = w.landed;
            self.clacks += w.advance(dt);
            self.landed_event |= w.landed && !was_landed;
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
            self.break_ended_event = true;
            self.start_work();
        }
        false
    }

    fn finish_work(&mut self) {
        self.completed += 1;
        self.phase = Phase::Wheel;
        self.running = false;
        self.remaining = Duration::ZERO;
        self.wheel = Some(Wheel::new(&mut self.rng, self.cfg.exercises.len()));
    }

    fn start_work(&mut self) {
        self.phase = Phase::Work;
        self.remaining = self.cfg.work;
        self.running = self.cfg.auto_start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);

    /// Tick in steps below `MAX_DT`; true if any tick reported a work end.
    fn adv(a: &mut App, total: Duration) -> bool {
        let mut ended = false;
        let mut left = total;
        while !left.is_zero() {
            let step = left.min(S);
            ended |= a.tick(step);
            left -= step;
        }
        ended
    }

    fn tcfg() -> Config {
        Config {
            work: S,
            short: S,
            long: S,
            ..Config::default()
        }
    }

    fn app() -> App {
        App::new(Config {
            work: 10 * S,
            short: 3 * S,
            long: 6 * S,
            ..Config::default()
        })
    }

    #[test]
    fn work_expires_into_wheel_then_short_break() {
        let mut a = app();
        a.toggle();
        adv(&mut a, 10 * S);
        assert_eq!(a.phase, Phase::Wheel);
        assert_eq!(a.completed, 1);
        adv(&mut a, 60 * S);
        a.enter();
        assert_eq!(a.phase, Phase::Break { long: false });
        assert_eq!(a.remaining, 3 * S);
    }

    #[test]
    fn fourth_break_is_long() {
        let mut a = app();
        for i in 1..=4 {
            a.toggle();
            adv(&mut a, 10 * S);
            adv(&mut a, 60 * S);
            a.enter();
            assert_eq!(a.phase, Phase::Break { long: i == 4 });
            adv(&mut a, 10 * S);
            assert_eq!(a.phase, Phase::Work);
        }
    }

    #[test]
    fn cycles_option_sets_long_break_frequency() {
        let mut a = App::new(Config {
            cycles: 2,
            ..tcfg()
        });
        assert!(!a.next_break_long());
        for i in 1..=4 {
            a.toggle();
            adv(&mut a, S);
            assert_eq!(a.cycle_progress(), if i % 2 == 0 { 2 } else { 1 });
            adv(&mut a, 60 * S);
            a.enter();
            assert_eq!(a.phase, Phase::Break { long: i % 2 == 0 });
            adv(&mut a, 2 * S);
        }
    }

    #[test]
    fn timer_counts_down_and_expires() {
        let mut a = app();
        a.toggle();
        adv(&mut a, 4 * S);
        assert_eq!(a.remaining, 6 * S);
        adv(&mut a, 6 * S);
        assert_eq!(a.phase, Phase::Wheel);
    }

    #[test]
    fn tick_reports_work_end_only() {
        let mut a = app();
        a.toggle();
        assert!(!adv(&mut a, 4 * S));
        assert!(adv(&mut a, 6 * S));
        adv(&mut a, 60 * S);
        a.enter();
        assert!(!adv(&mut a, 10 * S));
    }

    #[test]
    fn skipping_work_does_not_count_or_spin_wheel() {
        let mut a = app();
        a.skip();
        assert_eq!(a.phase, Phase::Break { long: false });
        assert_eq!(a.completed, 0);
        assert!(a.wheel.is_none());
        assert!(a.running);
    }

    #[test]
    fn huge_tick_is_capped() {
        let mut a = app();
        a.toggle();
        a.tick(Duration::from_secs(3600));
        assert_eq!(a.phase, Phase::Work);
        assert_eq!(a.remaining, 9 * S);
    }

    #[test]
    fn auto_start_runs_work_after_break_and_reports_it() {
        let mut a = App::new(Config {
            auto_start: true,
            ..tcfg()
        });
        assert!(a.running, "starts at launch");
        adv(&mut a, 2 * S);
        adv(&mut a, 60 * S);
        a.enter();
        assert!(!a.take_break_ended());
        adv(&mut a, S);
        assert_eq!(a.phase, Phase::Work);
        assert!(a.running);
        assert!(a.take_break_ended());
        assert!(!a.take_break_ended());
    }

    #[test]
    fn manual_start_waits_for_space_after_break() {
        let mut a = App::new(tcfg());
        a.skip();
        adv(&mut a, 2 * S);
        assert_eq!(a.phase, Phase::Work);
        assert!(!a.running);
    }

    #[test]
    fn pause_stops_countdown() {
        let mut a = app();
        adv(&mut a, 5 * S);
        assert_eq!(a.remaining, 10 * S, "not started yet");
        a.toggle();
        adv(&mut a, 2 * S);
        a.toggle();
        adv(&mut a, 5 * S);
        assert_eq!(a.remaining, 8 * S);
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut a = app();
        a.toggle();
        adv(&mut a, 10 * S);
        a.reset();
        assert_eq!(a.phase, Phase::Work);
        assert_eq!(a.completed, 0);
        assert!(!a.running);
    }

    #[test]
    fn wheel_lands_on_valid_index_and_stops() {
        for seed in 0..50 {
            let mut a = App::with_seed(tcfg(), seed);
            a.toggle();
            a.tick(S);
            assert!(a.result().is_none());
            for _ in 0..1000 {
                a.tick(Duration::from_millis(50));
            }
            let w = a.wheel.as_ref().unwrap();
            assert!(w.landed && w.pos < a.cfg.exercises.len());
            let pos = w.pos;
            adv(&mut a, 10 * S);
            assert_eq!(a.wheel.as_ref().unwrap().pos, pos, "stays put");
            assert_eq!(a.result(), Some(a.cfg.exercises[pos].as_str()));
        }
    }

    #[test]
    fn clacks_equal_highlight_steps() {
        for (seed, step_ms) in [(1, 10), (2, 50), (3, 333)] {
            let mut a = App::with_seed(tcfg(), seed);
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
            assert_eq!(w.pos, (start + seen as usize) % a.cfg.exercises.len());
            assert_eq!(a.take_clacks(), 0);
        }
    }

    #[test]
    fn one_tick_can_yield_several_clacks() {
        let mut a = spun_app(5);
        a.tick(Duration::from_millis(300));
        assert!(a.take_clacks() > 1);
    }

    fn spun_app(seed: u64) -> App {
        let mut a = App::with_seed(tcfg(), seed);
        a.toggle();
        a.tick(S);
        a
    }

    #[test]
    fn wheel_landed_fires_exactly_once() {
        let mut a = spun_app(9);
        assert!(!a.take_wheel_landed());
        let mut fired = 0;
        for _ in 0..400 {
            a.tick(Duration::from_millis(50));
            fired += u32::from(a.take_wheel_landed());
        }
        assert_eq!(fired, 1);
    }

    #[test]
    fn custom_exercises_drive_the_wheel() {
        let mut a = App::with_seed(
            Config {
                exercises: vec!["only one".into()],
                ..tcfg()
            },
            1,
        );
        a.toggle();
        a.tick(S);
        adv(&mut a, 60 * S);
        assert_eq!(a.result(), Some("only one"));
    }

    #[test]
    fn enter_ignored_while_spinning() {
        let mut a = app();
        a.toggle();
        adv(&mut a, 10 * S);
        a.enter();
        assert_eq!(a.phase, Phase::Wheel);
    }
}
