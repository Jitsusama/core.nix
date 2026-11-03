//! Watching the file for saves. Editors save in two ways: writing the file
//! in place, or writing a new one beside it and moving it over the old. The
//! folder is watched rather than the file, since a file moved over is a new
//! file the old watch never sees. A link followed to another file moves the
//! watch to it, so only the file shown is ever heard.

use std::ffi::{OsStr, OsString};
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};

/// The file heard: its folder's watch and its name in that folder.
struct Heard {
    folder: i32,
    name: OsString,
}

/// A watch on one file's saves, which can be moved to another file.
pub struct Watch {
    fd: Arc<OwnedFd>,
    heard: Arc<Mutex<Heard>>,
}

/// The folder `file` is in, and its name there.
fn split(file: &Path) -> (PathBuf, OsString) {
    let folder = match file.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
        _ => Path::new(".").to_path_buf(),
    };
    let name = file
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default();
    (folder, name)
}

fn watch_folder(fd: &OwnedFd, folder: &Path) -> std::io::Result<i32> {
    Ok(inotify::add_watch(
        fd,
        folder,
        WatchFlags::CLOSE_WRITE | WatchFlags::MOVED_TO | WatchFlags::CREATE,
    )?)
}

/// Calls `saved` each time the file watched is saved, from a thread of its
/// own, until `saved` says to stop. A burst of events from one save calls
/// it once for each read of them, and the reloader takes only the newest.
pub fn saves(
    file: &Path,
    mut saved: impl FnMut() -> bool + Send + 'static,
) -> std::io::Result<Watch> {
    let fd = Arc::new(inotify::init(CreateFlags::CLOEXEC)?);
    let (folder, name) = split(file);
    let heard = Arc::new(Mutex::new(Heard {
        folder: watch_folder(&fd, &folder)?,
        name,
    }));
    let reading = Arc::clone(&fd);
    let hearing = Arc::clone(&heard);
    std::thread::Builder::new()
        .name("watch".into())
        .spawn(move || {
            let mut buffer = [MaybeUninit::<u8>::uninit(); 4096];
            let mut events = inotify::Reader::new(&*reading, &mut buffer);
            loop {
                let mut ours = false;
                loop {
                    match events.next() {
                        Ok(event) => {
                            let heard = hearing
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            // A folder watched before the file moved on may
                            // still have events on their way.
                            let named = event.wd() == heard.folder
                                && event
                                    .file_name()
                                    .is_some_and(|it| it.to_bytes() == heard.name.as_bytes());
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
    Ok(Watch { fd, heard })
}

impl Watch {
    /// Hears `file`'s saves from now on, and no longer the last file's. A
    /// file in the same folder shares its watch, which inotify gives once a
    /// folder, so that watch is kept.
    pub fn follow(&self, file: &Path) -> std::io::Result<()> {
        let (folder, name) = split(file);
        let wd = watch_folder(&self.fd, &folder)?;
        let mut heard = self
            .heard
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if heard.folder != wd {
            let _ = inotify::remove_watch(&*self.fd, heard.folder);
        }
        *heard = Heard { folder: wd, name };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::saves;

    fn place(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("reader-watch-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    #[test]
    fn a_save_in_place_and_a_save_moved_over_are_each_heard() {
        let dir = place("saves");
        let file = dir.join("page.md");
        std::fs::write(&file, "one").expect("the file");
        let (heard, hearing) = mpsc::channel();
        let _watch = saves(&file, move || heard.send(()).is_ok()).expect("a watch");

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
        let dir = place("another");
        let file = dir.join("page.md");
        std::fs::write(&file, "one").expect("the file");
        let (heard, hearing) = mpsc::channel();
        let _watch = saves(&file, move || heard.send(()).is_ok()).expect("a watch");

        std::fs::write(dir.join("other.md"), "two").expect("another file");
        assert!(
            hearing.recv_timeout(Duration::from_millis(500)).is_err(),
            "another file isn't heard"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn a_watch_moved_on_hears_the_new_file_and_not_the_old() {
        let dir = place("moved");
        let first = dir.join("page.md");
        let beside = dir.join("beside.md");
        let below = dir.join("notes/below.md");
        std::fs::create_dir_all(dir.join("notes")).expect("a folder below");
        for file in [&first, &beside, &below] {
            std::fs::write(file, "one").expect("a file");
        }
        let (heard, hearing) = mpsc::channel();
        let watch = saves(&first, move || heard.send(()).is_ok()).expect("a watch");

        // In the same folder, whose watch the two share.
        watch.follow(&beside).expect("moved on");
        std::fs::write(&first, "two").expect("the old file saved");
        assert!(
            hearing.recv_timeout(Duration::from_millis(500)).is_err(),
            "the old file isn't heard"
        );
        std::fs::write(&beside, "two").expect("the new file saved");
        assert!(
            hearing.recv_timeout(Duration::from_secs(5)).is_ok(),
            "the file beside is heard"
        );
        while hearing.try_recv().is_ok() {}

        // In another folder.
        watch.follow(&below).expect("moved on again");
        std::fs::write(&beside, "three").expect("the last file saved");
        assert!(
            hearing.recv_timeout(Duration::from_millis(500)).is_err(),
            "the last folder's file isn't heard"
        );
        std::fs::write(&below, "three").expect("the file below saved");
        assert!(
            hearing.recv_timeout(Duration::from_secs(5)).is_ok(),
            "the file below is heard"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }
}
