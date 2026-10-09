use pomosport::app::{App, Config};
use pomosport::ui::draw;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    let app = App::new(Config::default());
    t.draw(|f| draw(f, &app)).unwrap();
    t.backend().to_string()
}

#[test]
fn timer_shows_paused_after_pause() {
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::new(Config::default());
    app.toggle();
    app.tick(std::time::Duration::from_secs(1));
    t.draw(|f| draw(f, &app)).unwrap();
    assert!(!t.backend().to_string().contains("paused"));
    app.toggle();
    t.draw(|f| draw(f, &app)).unwrap();
    assert!(t.backend().to_string().contains("24:59  paused"));
}

#[test]
fn shows_today_count() {
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::new(Config::default());
    app.today = 7;
    t.draw(|f| draw(f, &app)).unwrap();
    assert!(t.backend().to_string().contains("today: 7"));
}

#[test]
fn wheel_view_spins_then_shows_result() {
    let sec = std::time::Duration::from_secs(1);
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::new(Config {
        work: sec,
        ..Config::default()
    });
    app.toggle();
    app.tick(sec);
    t.draw(|f| draw(f, &app)).unwrap();
    let out = t.backend().to_string();
    assert!(out.contains("EXERCISE TIME") && out.contains("Spinning"));
    assert!(out.contains("20 squats"));
    for _ in 0..70 {
        app.tick(sec / 10);
    }
    t.draw(|f| draw(f, &app)).unwrap();
    assert!(t.backend().to_string().contains("Enter to start break"));
}

#[test]
fn small_terminal_shows_message() {
    assert!(render(30, 10).contains("Terminal too small"));
}

#[test]
fn normal_terminal_shows_timer_and_hints() {
    let out = render(80, 24);
    assert!(out.contains("25:00") && out.contains("q quit"));
}
