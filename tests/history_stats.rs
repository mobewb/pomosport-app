use std::path::PathBuf;

use pomosport::history::{append, load, render, summarize, Entry, Stats};

const DAY: u64 = 24 * 60 * 60;

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pomosport-hist-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("nested/history.jsonl")
}

#[test]
fn append_creates_dirs_and_writes_one_json_line_per_entry() {
    let path = temp_file("rt");
    let a = Entry {
        ts: 10,
        exercise: "5 burpees".into(),
        work_minutes: 25.0,
    };
    let b = Entry {
        ts: 20,
        exercise: "20 squats".into(),
        work_minutes: 0.5,
    };
    append(&path, &a).unwrap();
    append(&path, &b).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let back: Vec<Entry> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(back, vec![a, b]);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn load_skips_garbage_and_missing_file() {
    let path = temp_file("bad");
    assert!(load(&path).is_empty());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "nope\n{\"ts\":1,\"exercise\":\"x\",\"work_minutes\":1.0}\n",
    )
    .unwrap();
    assert_eq!(load(&path).len(), 1);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn summarize_counts_windows_and_exercises() {
    let now = 10 * DAY;
    let e = |ago: u64, ex: &str| Entry {
        ts: now - ago,
        exercise: ex.into(),
        work_minutes: 25.0,
    };
    let entries = [e(60, "squats"), e(2 * DAY, "squats"), e(8 * DAY, "burpees")];
    let st = summarize(&entries, now);
    assert_eq!((st.today, st.week, st.total), (1, 2, 3));
    assert_eq!(st.work_minutes, 75.0);
    assert_eq!(st.per_exercise["squats"], 2);
    let text = render(&st);
    assert!(text.contains("1 today, 2 this week, 3 total") && text.contains("2 x squats"));
    assert_eq!(summarize(&[], now), Stats::default());
}
