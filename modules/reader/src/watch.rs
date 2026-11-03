//! Watching the file for saves. Editors save in two ways: writing the file
//! in place, or writing a new one beside it and moving it over the old. The
//! folder is watched rather than the file, since a file moved over is a new
//! file the old watch never sees.

use std::ffi::OsStr;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};

/// Calls `saved` each time `file` is saved, from a thread of its own, until
/// `saved` says to stop. A burst of events from one save calls it once for
/// each read of them, and the reloader takes only the newest.
pub fn saves(file: &Path, mut saved: impl FnMut() -> bool + Send + 'static) -> std::io::Result<()> {
    let folder = match file.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
        _ => Path::new(".").to_path_buf(),
    };
    let name = file
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default();
    let fd = inotify::init(CreateFlags::CLOEXEC)?;
    inotify::add_watch(
        &fd,
        &folder,
        WatchFlags::CLOSE_WRITE | WatchFlags::MOVED_TO | WatchFlags::CREATE,
    )?;
    std::thread::Builder::new()
        .name("watch".into())
        .spawn(move || {
            let mut buffer = [MaybeUninit::<u8>::uninit(); 4096];
            let mut events = inotify::Reader::new(&fd, &mut buffer);
            loop {
                let mut ours = false;
                loop {
                    match events.next() {
                        Ok(event) => {
                            let named = event
                                .file_name()
                                .is_some_and(|it| it.to_bytes() == name.as_bytes());
                            let written = event.events().intersects(
                                ReadFlags::CLOSE_WRITE | ReadFlags::MOVED_TO | ReadFlags::CREATE,
                            );
                            ours |= named && written;
                        }
                        Err(rustix::io::Errno::INTR) => {}
                        Err(_) => return,
                    }
                    if events.is_buffer_empty() {
                        break;
                    }
                }
                if ours && !saved() {
                    return;
                }
            }
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::saves;

    fn place() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "reader-watch-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    #[test]
    fn a_save_in_place_and_a_save_moved_over_are_each_heard() {
        let dir = place();
        let file = dir.join("page.md");
        std::fs::write(&file, "one").expect("the file");
        let (heard, hearing) = mpsc::channel();
        saves(&file, move || heard.send(()).is_ok()).expect("a watch");

        std::fs::write(&file, "two").expect("a save in place");
        assert!(
            hearing.recv_timeout(Duration::from_secs(5)).is_ok(),
            "a save in place is heard"
        );
        while hearing.try_recv().is_ok() {}

        let fresh = dir.join(".page.md.swp");
        std::fs::write(&fresh, "three").expect("a new file");
        std::fs::rename(&fresh, &file).expect("moved over");
        assert!(
            hearing.recv_timeout(Duration::from_secs(5)).is_ok(),
            "a save moved over is heard"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn another_file_in_the_folder_is_not_heard() {
        let dir = place();
        let file = dir.join("page.md");
        std::fs::write(&file, "one").expect("the file");
        let (heard, hearing) = mpsc::channel();
        saves(&file, move || heard.send(()).is_ok()).expect("a watch");

        std::fs::write(dir.join("other.md"), "two").expect("another file");
        assert!(
            hearing.recv_timeout(Duration::from_millis(500)).is_err(),
            "another file isn't heard"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }
}
