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
}
