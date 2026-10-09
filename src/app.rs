use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
    /// Sessions completed in the last 24 hours, across runs (set by the caller).
    pub today: u32,
    pub quit: bool,
    pub wheel: Option<Wheel>,
    /// False while the wheel runs for a skipped session: not counted, not logged.
    counts: bool,
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
            today: 0,
            quit: false,
            wheel: None,
            counts: true,
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

    pub fn on_key(&mut self, k: KeyEvent) {
        if k.kind != KeyEventKind::Press {
            return;
        }
        match k.code {
            KeyCode::Char(' ') => self.toggle(),
            KeyCode::Char('s') => self.skip(),
            KeyCode::Char('r') => self.reset(),
            KeyCode::Enter => self.enter(),
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    pub fn toggle(&mut self) {
        if self.phase != Phase::Wheel {
            self.running = !self.running;
        }
    }

    pub fn reset(&mut self) {
        let seed = self.rng.gen();
        let today = self.today;
        *self = Self::with_seed(self.cfg.clone(), seed);
        self.today = today;
    }

    pub fn skip(&mut self) {
        match self.phase {
            // A skipped session still spins the wheel but does not count.
            Phase::Work => self.start_wheel(false),
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
        let long =
            self.counts && self.completed > 0 && self.completed.is_multiple_of(self.cfg.cycles);
        self.phase = Phase::Break { long };
        self.remaining = if long { self.cfg.long } else { self.cfg.short };
        self.running = true;
        self.wheel = None;
    }

    pub fn take_clacks(&mut self) -> u32 {
        std::mem::take(&mut self.clacks)
    }

    /// Whether the current wheel belongs to a completed session (so it is
    /// logged), as opposed to a skipped one.
    pub fn session_counts(&self) -> bool {
        self.counts
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
        self.start_wheel(true);
    }

    fn start_wheel(&mut self, counts: bool) {
        self.counts = counts;
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
