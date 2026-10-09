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
