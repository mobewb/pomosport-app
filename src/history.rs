use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// One completed work session, stored as a line of JSON.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Unix time in seconds when the session completed.
    pub ts: u64,
    pub exercise: String,
    pub work_minutes: f64,
}

pub fn default_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".local/share/pomosport/history.jsonl"))
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn append(path: &Path, entry: &Entry) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut line = serde_json::to_string(entry).map_err(std::io::Error::other)?;
    line.push('\n');
    file.write_all(line.as_bytes())
}

/// Missing file means no history; unparseable lines are skipped.
pub fn load(path: &Path) -> Vec<Entry> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

const DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Default, PartialEq)]
pub struct Stats {
    pub today: u32,
    pub week: u32,
    pub total: u32,
    pub work_minutes: f64,
    pub per_exercise: BTreeMap<String, u32>,
}

/// Counts sessions in rolling windows (last 24 hours, last 7 days), which
/// avoids needing the local time zone.
pub fn summarize(entries: &[Entry], now: u64) -> Stats {
    let mut st = Stats::default();
    for e in entries {
        let age = now.saturating_sub(e.ts);
        st.today += u32::from(age < DAY);
        st.week += u32::from(age < 7 * DAY);
        st.total += 1;
        st.work_minutes += e.work_minutes;
        *st.per_exercise.entry(e.exercise.clone()).or_default() += 1;
    }
    st
}

pub fn render(st: &Stats) -> String {
    let mut out = format!(
        "Sessions: {} today, {} this week, {} total\nWork time: {:.0} min\n",
        st.today, st.week, st.total, st.work_minutes
    );
    for (exercise, n) in &st.per_exercise {
        out.push_str(&format!("  {n:>4} x {exercise}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pomosport-hist-{}-{name}", std::process::id()));
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
}
