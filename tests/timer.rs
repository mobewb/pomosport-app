mod common;

use std::time::Duration;

use common::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use pomosport::app::{App, Config, Phase};

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
fn skipping_work_spins_wheel_but_does_not_count() {
    let mut a = app();
    a.skip();
    assert_eq!(a.phase, Phase::Wheel);
    assert_eq!(a.completed, 0);
    assert!(a.wheel.is_some());
    assert!(!a.session_counts(), "not logged");
    adv(&mut a, 60 * S);
    a.enter();
    assert_eq!(a.phase, Phase::Break { long: false });
    assert_eq!(a.completed, 0);
}

#[test]
fn skipped_session_never_earns_the_long_break() {
    let mut a = app();
    for _ in 0..4 {
        a.toggle();
        adv(&mut a, 10 * S);
        assert!(a.session_counts());
        adv(&mut a, 60 * S);
        a.enter();
        adv(&mut a, 10 * S);
    }
    assert_eq!(a.completed, 4);
    assert!(
        !a.next_break_long(),
        "work ahead: 5th session, not a multiple"
    );
    a.skip();
    assert!(!a.next_break_long(), "hint agrees with the real break");
    adv(&mut a, 60 * S);
    a.enter();
    assert_eq!(a.phase, Phase::Break { long: false });
    assert_eq!(a.completed, 4);
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
    adv(&mut a, 60 * S);
    a.enter();
    adv(&mut a, 2 * S);
    assert_eq!(a.phase, Phase::Work);
    assert!(!a.running);
}

fn press(a: &mut App, code: KeyCode, mods: KeyModifiers) {
    a.on_key(KeyEvent::new(code, mods));
}

#[test]
fn keys_drive_the_app() {
    let mut a = app();
    let none = KeyModifiers::NONE;
    press(&mut a, KeyCode::Char(' '), none);
    assert!(a.running);
    press(&mut a, KeyCode::Char(' '), none);
    assert!(!a.running);
    press(&mut a, KeyCode::Char('s'), none);
    assert_eq!(a.phase, Phase::Wheel);
    press(&mut a, KeyCode::Char('r'), none);
    assert_eq!(a.phase, Phase::Work);
    press(&mut a, KeyCode::Char('x'), none);
    assert!(!a.quit);
    press(&mut a, KeyCode::Char('c'), none);
    assert!(!a.quit, "plain c does not quit");
    press(&mut a, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(a.quit);
}

#[test]
fn q_and_esc_quit() {
    for code in [KeyCode::Char('q'), KeyCode::Esc] {
        let mut a = app();
        press(&mut a, code, KeyModifiers::NONE);
        assert!(a.quit);
    }
}

#[test]
fn key_release_is_ignored() {
    let mut a = app();
    let mut ev = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    ev.kind = KeyEventKind::Release;
    a.on_key(ev);
    assert!(!a.quit);
}

#[test]
fn skip_from_break_returns_to_work() {
    let mut a = app();
    a.skip();
    adv(&mut a, 60 * S);
    a.skip();
    a.skip();
    assert_eq!(a.phase, Phase::Work);
    assert_eq!(a.remaining, a.cfg.work);
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
    a.today = 3;
    a.reset();
    assert_eq!(a.today, 3, "today survives reset");
    assert_eq!(a.phase, Phase::Work);
    assert_eq!(a.completed, 0);
    assert!(!a.running);
}

#[test]
fn hint_predicts_the_long_break_before_the_fourth_session_ends() {
    let mut a = app();
    for _ in 0..3 {
        a.toggle();
        adv(&mut a, 10 * S);
        adv(&mut a, 60 * S);
        a.enter();
        adv(&mut a, 10 * S);
    }
    assert!(a.next_break_long(), "4th session ahead");
    a.toggle();
    adv(&mut a, 10 * S);
    assert!(a.next_break_long(), "wheel after the 4th session");
}
