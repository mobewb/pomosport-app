use std::time::Duration;

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
}

impl App {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            phase: Phase::Work,
            remaining: cfg.work,
            running: false,
            completed: 0,
            quit: false,
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
        *self = Self::new(self.cfg);
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
        if self.phase == Phase::Wheel {
            let long = self.completed % 4 == 0;
            self.phase = Phase::Break { long };
            self.remaining = if long { self.cfg.long } else { self.cfg.short };
            self.running = true;
        }
    }

    pub fn tick(&mut self, dt: Duration) {
        if !self.running || self.phase == Phase::Wheel {
            return;
        }
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            match self.phase {
                Phase::Work => self.finish_work(),
                _ => self.start_work(),
            }
        }
    }

    fn finish_work(&mut self) {
        self.completed += 1;
        self.phase = Phase::Wheel;
        self.running = false;
        self.remaining = Duration::ZERO;
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
}
