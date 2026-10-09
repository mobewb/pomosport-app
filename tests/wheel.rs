mod common;

use std::time::Duration;

use common::*;
use pomosport::app::{App, Config, Phase};

#[test]
fn skip_on_wheel_waits_for_landing() {
    let mut a = spun_app(4);
    a.skip();
    assert_eq!(a.phase, Phase::Wheel, "still spinning");
    adv(&mut a, 60 * S);
    a.skip();
    assert_eq!(a.phase, Phase::Break { long: false });
    assert!(a.wheel.is_none());
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
