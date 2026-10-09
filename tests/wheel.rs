mod common;

use std::time::Duration;

use common::*;
use pomosport::app::{App, Config, Phase};

#[test]
fn skip_on_spinning_wheel_lands_it_then_second_skip_starts_break() {
    let mut a = spun_app(4);
    a.skip();
    assert_eq!(a.phase, Phase::Wheel, "landed, still showing the result");
    assert!(a.result().is_some());
    assert!(a.take_wheel_landed(), "win sound fires");
    a.skip();
    assert_eq!(a.phase, Phase::Break { long: false });
    assert!(a.wheel.is_none());
    assert!(!a.take_wheel_landed());
}

#[test]
fn landing_early_picks_the_same_exercise_as_the_full_spin() {
    for seed in 0..20 {
        let mut early = spun_app(seed);
        early.skip();
        let mut full = spun_app(seed);
        adv(&mut full, 60 * S);
        assert_eq!(early.result(), full.result(), "seed {seed}");
    }
}

#[test]
fn landing_early_does_not_double_fire_the_one_shot() {
    let mut a = spun_app(2);
    a.skip();
    adv(&mut a, 10 * S);
    assert!(a.take_wheel_landed());
    assert!(!a.take_wheel_landed());
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

#[test]
fn pending_exercise_is_known_when_work_ends_and_matches_the_result() {
    for seed in 0..20 {
        let mut a = spun_app(seed);
        let pending = a.pending_exercise().unwrap().to_string();
        assert!(a.result().is_none(), "still spinning");
        adv(&mut a, 60 * S);
        assert_eq!(a.result(), Some(pending.as_str()), "seed {seed}");
    }
}

#[test]
fn no_pending_exercise_outside_the_wheel() {
    assert!(app().pending_exercise().is_none());
}
