use std::process::{Child, Command};
use std::time::{Duration, Instant};

use pomosport::platform::reap;

fn wait_until_exited(c: &mut Child) {
    let start = Instant::now();
    while c.try_wait().unwrap().is_none() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn reap_removes_finished_children_and_keeps_running_ones() {
    let mut done = Command::new("true").spawn().unwrap();
    wait_until_exited(&mut done);
    // try_wait already reaped it above; a second finished child exercises `reap` itself.
    let finished = Command::new("true").spawn().unwrap();
    let running = Command::new("sleep").arg("30").spawn().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let mut kids = vec![finished, running];
    reap(&mut kids);
    assert_eq!(kids.len(), 1, "only the sleeping child remains");
    kids[0].kill().unwrap();
    kids[0].wait().unwrap();
}

mod assets {
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;

    use pomosport::platform::write_asset_in;

    fn fresh_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pomosport-assets-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn written_once_and_reused_when_content_matches() {
        let dir = fresh_dir("reuse").join("nested");
        let path = write_asset_in(&dir, "a.bin", b"abc").unwrap();
        let first = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(write_asset_in(&dir, "a.bin", b"abc"), Some(path.clone()));
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), first);
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn same_size_but_different_content_is_rewritten() {
        let dir = fresh_dir("content");
        write_asset_in(&dir, "a.bin", b"abc").unwrap();
        let path = write_asset_in(&dir, "a.bin", b"xyz").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"xyz");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn directory_is_private_and_symlinks_are_not_followed() {
        let dir = fresh_dir("symlink");
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("victim.txt");
        std::fs::write(&victim, "keep me").unwrap();
        let private = dir.join("cache");
        let path = write_asset_in(&private, "a.bin", b"abc").unwrap();
        assert_eq!(
            std::fs::metadata(&private).unwrap().permissions().mode() & 0o777,
            0o700
        );
        std::fs::remove_file(&path).unwrap();
        symlink(&victim, &path).unwrap();
        write_asset_in(&private, "a.bin", b"abc").unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep me");
        assert!(!std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
