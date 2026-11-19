//! Files another program keeps, each written whole and renamed into
//! place, heard as each lands rather than read again on a clock: until
//! spend-watch streams its collection over its socket (1KZFHC, Fri 13
//! Nov), what each session spent today is its collection file, and the
//! shell's own state files join them. A folder not there yet is waited
//! for from the nearest one that is, so a program started later is heard.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use serde_json::Value;

/// The files watched, by the folder each is in.
pub struct Files {
    inotify: Inotify,
    wanted: Vec<PathBuf>,
    watching: HashMap<WatchDescriptor, PathBuf>,
}

/// What lands a file or takes it away: a rename into place, a write
/// closed, a removal, or a folder made on the way to one.
fn mask() -> WatchMask {
    WatchMask::CLOSE_WRITE
        | WatchMask::MOVED_TO
        | WatchMask::MOVED_FROM
        | WatchMask::DELETE
        | WatchMask::CREATE
}

impl Files {
    /// Watches `wanted`, each from its folder, or the nearest folder
    /// above it that's there.
    pub fn open(wanted: &[PathBuf]) -> io::Result<Self> {
        let mut files = Self {
            inotify: Inotify::init()?,
            wanted: wanted.to_vec(),
            watching: HashMap::new(),
        };
        files.watch_toward();
        Ok(files)
    }

    /// Adds a watch on each wanted file's folder, or the nearest above it
    /// that's there, where none watches it yet, answering the folders it
    /// began watching. A folder made below one while its watch was being
    /// added said nothing to it, so it looks again until a look begins none.
    fn watch_toward(&mut self) -> Vec<PathBuf> {
        let mut began = Vec::new();
        loop {
            let before = began.len();
            for file in self.wanted.clone() {
                let Some(folder) = file.ancestors().skip(1).find(|folder| folder.is_dir()) else {
                    continue;
                };
                if self.watching.values().any(|watched| watched == folder) {
                    continue;
                }
                if let Ok(watch) = self.inotify.watches().add(folder, mask()) {
                    self.watching.insert(watch, folder.to_owned());
                    began.push(folder.to_owned());
                }
            }
            if began.len() == before {
                return began;
            }
        }
    }

    /// Waits until a wanted file lands or goes, answering which; a folder
    /// made on the way to one is watched in turn.
    pub fn heard(&mut self) -> io::Result<Vec<PathBuf>> {
        let mut buffer = [0; 4096];
        loop {
            let mut named: Vec<(PathBuf, OsString)> = Vec::new();
            for event in self.inotify.read_events_blocking(&mut buffer)? {
                // A folder removed takes its watch with it, so it's watched
                // again once it's made again.
                if event.mask.contains(EventMask::IGNORED) {
                    self.watching.remove(&event.wd);
                    continue;
                }
                if let (Some(folder), Some(name)) = (self.watching.get(&event.wd), event.name) {
                    named.push((folder.clone(), name.to_owned()));
                }
            }
            let began = self.watch_toward();
            let mut landed = landed(&self.wanted, &named);
            // One that landed in a folder made since, before its watch
            // began, said nothing it could hear, so being there says it.
            for file in &self.wanted {
                let watched_since = file
                    .parent()
                    .is_some_and(|folder| began.iter().any(|one| one == folder));
                if watched_since && !landed.contains(file) && file.exists() {
                    landed.push(file.clone());
                }
            }
            if !landed.is_empty() {
                return Ok(landed);
            }
        }
    }
}

/// Of the files wanted, those an event in their own folder named.
fn landed(wanted: &[PathBuf], named: &[(PathBuf, OsString)]) -> Vec<PathBuf> {
    wanted
        .iter()
        .filter(|file| {
            named.iter().any(|(folder, name)| {
                file.parent() == Some(folder) && file.file_name() == Some(name)
            })
        })
        .cloned()
        .collect()
}

/// A file's JSON, or none while it isn't there or can't be read whole,
/// as one being replaced.
pub fn read(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// The file `name` in whichever of the folders under `shells` wrote it
/// last, as each start of the shell keeps its state in a folder of its
/// own, so what the shell kept before the daemon did is read from there.
pub fn newest(shells: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(shells)
        .into_iter()
        .flatten()
        .flatten()
        .map(|folder| folder.path().join(name))
        .filter_map(|file| Some((fs::metadata(&file).ok()?.modified().ok()?, file)))
        .max()
        .map(|(_, file)| file)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    /// Long past what hearing a file takes, so one never heard fails rather
    /// than hangs.
    const HEARING: Duration = Duration::from_secs(5);

    /// Long enough for a file that will be heard to have been.
    const QUIET: Duration = Duration::from_secs(1);

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("agents-file-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder to watch");
        folder
    }

    #[test]
    fn a_file_renamed_into_place_is_heard_and_read() {
        let at = folder("renamed");
        let file = at.join("collection.json");
        let mut files = Files::open(std::slice::from_ref(&file)).expect("the watch opens");
        let writing = at.join("collection.json.new");
        let writer = thread::spawn(move || {
            fs::write(at.join("other.json"), "{}").expect("another file");
            fs::write(&writing, r#"{"agentsToday": "$4"}"#).expect("the file written");
            fs::rename(&writing, at.join("collection.json")).expect("renamed into place");
        });
        assert_eq!(
            files.heard().expect("it lands"),
            std::slice::from_ref(&file)
        );
        writer.join().expect("the writer ends");
        assert_eq!(
            read(&file).map(|said| said["agentsToday"].clone()),
            Some("$4".into())
        );
    }

    #[test]
    fn a_folder_made_later_is_waited_for() {
        let at = folder("later");
        let file = at.join("spend-watch").join("collection.json");
        let mut files = Files::open(std::slice::from_ref(&file)).expect("the watch opens");
        let making = at.clone();
        let writer = thread::spawn(move || {
            fs::create_dir_all(making.join("spend-watch")).expect("the folder made");
            // The folder's own watch may not be added before this write is
            // over, so it writes again until the reader has heard it.
            while !making.join("heard").exists() {
                fs::write(making.join("spend-watch").join("collection.json"), "{}")
                    .expect("the file written");
                thread::yield_now();
            }
        });
        assert_eq!(files.heard().expect("it lands"), [file]);
        fs::write(at.join("heard"), "").expect("the reader says it heard");
        writer.join().expect("the writer ends");
    }

    #[test]
    fn a_file_landed_in_a_folder_before_its_watch_is_heard() {
        let at = folder("before");
        let file = at.join("agents").join("arrangement.json");
        let mut files = Files::open(std::slice::from_ref(&file)).expect("the watch opens");
        // Both land before the reader looks, as a test's mkdir -p then mv
        // did on 2026-11-19, so the folder's watch comes after the file.
        fs::create_dir_all(at.join("agents")).expect("the folder made");
        fs::write(at.join("agents").join("arrangement.json.q"), "{}").expect("written");
        fs::rename(
            at.join("agents").join("arrangement.json.q"),
            at.join("agents").join("arrangement.json"),
        )
        .expect("renamed into place");
        let (said, heard) = mpsc::channel();
        thread::spawn(move || said.send(files.heard().expect("it lands")));
        assert_eq!(
            heard
                .recv_timeout(HEARING)
                .expect("heard, not waited on forever"),
            [file]
        );
    }

    /// Hears `files` on a thread of its own, saying each landing it hears.
    fn hearing(mut files: Files) -> mpsc::Receiver<Vec<PathBuf>> {
        let (said, heard) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(landed) = files.heard() {
                if said.send(landed).is_err() {
                    return;
                }
            }
        });
        heard
    }

    #[test]
    fn a_folder_removed_and_made_again_is_heard_again() {
        let at = folder("again");
        let file = at.join("agents").join("arrangement.json");
        fs::create_dir_all(at.join("agents")).expect("the folder made");
        fs::write(&file, "{}").expect("written");
        let heard = hearing(Files::open(std::slice::from_ref(&file)).expect("the watch opens"));
        fs::remove_dir_all(at.join("agents")).expect("the folder removed");
        assert_eq!(
            heard.recv_timeout(HEARING).expect("its going heard"),
            std::slice::from_ref(&file)
        );
        fs::create_dir_all(at.join("agents")).expect("the folder made again");
        fs::write(&file, "{}").expect("written again");
        assert_eq!(
            heard
                .recv_timeout(HEARING)
                .expect("heard again, not waited on forever"),
            [file]
        );
    }

    #[test]
    fn a_folder_made_with_other_files_says_nothing_of_one_not_there() {
        let at = folder("empty");
        let file = at.join("agents").join("arrangement.json");
        let heard = hearing(Files::open(std::slice::from_ref(&file)).expect("the watch opens"));
        fs::create_dir_all(at.join("agents")).expect("the folder made");
        fs::write(at.join("agents").join("other.json"), "{}").expect("another file");
        assert_eq!(
            heard.recv_timeout(QUIET).ok(),
            None,
            "a file not there isn't heard"
        );
    }

    #[test]
    fn a_file_not_there_reads_as_nothing() {
        assert_eq!(read(Path::new("/nonexistent/collection.json")), None);
    }
}
