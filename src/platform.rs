//! Side effects outside the terminal: sounds, notifications and the temp
//! files that carry embedded assets to `afplay` and `terminal-notifier`.
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const WIN_SOUND: &[u8] = include_bytes!("../assets/win.wav");
pub const WORK_DONE: &str = "Time for an exercise!";
pub const BREAK_DONE: &str = "Break over, back to work!";

/// Write an embedded asset to a stable file in the temp dir so external tools
/// (terminal-notifier, afplay) can read it. Reused across runs when unchanged.
fn write_asset(name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let path = std::env::temp_dir().join(format!("pomosport-{}-{name}", env!("CARGO_PKG_VERSION")));
    let fresh = std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64);
    if !fresh {
        std::fs::write(&path, bytes).ok()?;
    }
    Some(path)
}

fn icon_path() -> Option<&'static PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| write_asset("icon.png", ICON)).as_ref()
}

/// Terminal bell (unless muted) plus a macOS notification (unless disabled);
/// failures are ignored.
/// Uses `terminal-notifier` (custom icon) when installed, else `osascript`.
pub fn notify(message: &str, mute: bool, no_notify: bool) {
    if !mute {
        let _ = io::stdout().write_all(b"\x07");
        let _ = io::stdout().flush();
    }
    if no_notify {
        return;
    }
    let mut tn = Command::new("terminal-notifier");
    tn.args(["-title", "pomosport", "-message", message]);
    if let Some(icon) = icon_path() {
        tn.arg("-appIcon").arg(icon);
    }
    let sent = tn
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if !sent {
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!("display notification \"{message}\" with title \"pomosport\""),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

const MAX_CLACKS: usize = 8;

/// One short clack per highlight step, overlapping if needed.
/// Finished players are reaped; at most `MAX_CLACKS` run at once.
pub fn clack(playing: &mut Vec<Child>, steps: u32) {
    playing.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
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

#[cfg(test)]
mod tests {
    use super::*;

    // Inline: exercises the private `write_asset`.
    #[test]
    fn asset_file_is_written_once_and_reused() {
        let name = format!("test-{}.bin", std::process::id());
        let path = write_asset(&name, b"abc").unwrap();
        let first = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(write_asset(&name, b"xyz"), Some(path.clone()));
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), first);
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        std::fs::remove_file(path).unwrap();
    }
}
