//! Side effects outside the terminal: sounds, notifications and the temp
//! files that carry embedded assets to `afplay` and `terminal-notifier`.
use std::fs::{DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const WIN_SOUND: &[u8] = include_bytes!("../assets/win.wav");
pub const WORK_DONE: &str = "Time for an exercise!";
pub const BREAK_DONE: &str = "Break over, back to work!";

/// Write an embedded asset into `dir` so external tools (terminal-notifier,
/// afplay) can read it. The file is trusted only if its content matches;
/// otherwise it is written under a fresh temp name and renamed into place, so
/// a symlink planted at the target is replaced rather than followed.
pub fn write_asset_in(dir: &Path, name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let path = dir.join(name);
    if std::fs::read(&path).is_ok_and(|b| b == bytes) {
        return Some(path);
    }
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .ok()?;
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut f| f.write_all(bytes))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return None;
    }
    Some(path)
}

/// Assets live in the per-user cache dir (`~/Library/Caches/pomosport` on macOS).
fn write_asset(name: &str, bytes: &[u8]) -> Option<PathBuf> {
    write_asset_in(&dirs::cache_dir()?.join("pomosport"), name, bytes)
}

fn icon_path() -> Option<&'static PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| write_asset("icon.png", ICON)).as_ref()
}

/// Drop finished child processes so they don't linger as zombies.
pub fn reap(children: &mut Vec<Child>) {
    children.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
}

/// Terminal bell (unless muted) plus a macOS notification (unless disabled);
/// failures are ignored. Spawned helpers are tracked in `running` for reaping.
/// Uses `terminal-notifier` (custom icon) when installed, else `osascript`.
pub fn notify(message: &str, mute: bool, no_notify: bool, running: &mut Vec<Child>) {
    if !mute {
        let _ = io::stdout().write_all(b"\x07");
        let _ = io::stdout().flush();
    }
    reap(running);
    if no_notify {
        return;
    }
    let mut tn = Command::new("terminal-notifier");
    tn.args(["-title", "pomosport", "-message", message]);
    if let Some(icon) = icon_path() {
        tn.arg("-appIcon").arg(icon);
    }
    let spawned = tn.stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    let child = spawned.or_else(|_| {
        Command::new("osascript")
            .args([
                "-e",
                &format!("display notification \"{message}\" with title \"pomosport\""),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    });
    running.extend(child);
}

const MAX_CLACKS: usize = 8;

/// One short clack per highlight step, overlapping if needed.
/// Finished players are reaped; at most `MAX_CLACKS` run at once.
pub fn clack(playing: &mut Vec<Child>, steps: u32) {
    reap(playing);
    for _ in 0..steps {
        if playing.len() >= MAX_CLACKS {
            break;
        }
        let child = Command::new("afplay")
            .args(["-t", "0.15", "/System/Library/Sounds/Tink.aiff"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        playing.extend(child);
    }
}

/// Victory fanfare when the wheel lands. Fire and forget; it plays alongside
/// any clack still ringing, but never overlaps another fanfare.
pub fn win(playing: &mut Option<Child>) {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    if playing
        .as_mut()
        .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
    {
        return;
    }
    let Some(path) = PATH.get_or_init(|| write_asset("win.wav", WIN_SOUND)) else {
        return;
    };
    *playing = Command::new("afplay")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
}
