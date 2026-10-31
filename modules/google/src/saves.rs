//! What Joel saves into Google Drive through the launcher's picker. The
//! picker stages each file at `drive-saves/<folder>/<name>` in the state
//! home, the folder `root` for My Drive or a folder's or shared drive's
//! id, and asks nothing; this program watches those folders and sends a
//! file up once it's written and left alone a moment, as Joel's program
//! may write it in pieces (VKCF3Z and QGB4ON, 8b69fe8a and c70b6e00).
//!
//! The staged copy stays where it is, so nothing saved is ever lost while
//! Drive can't be reached, and a mark beside it, `.<name>.drive`, keeps
//! the id Drive gave it: saving it again sends a new version of the same
//! file, and one written since its mark goes up again after a restart.
//! Copies sent up a week ago are let go as the program starts. A name
//! starting `.` or `~`, a program's lock or a draft, is never sent.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};

use crate::drive::{self, Folder};

/// How long a staged file is left alone before it's sent up.
pub const SETTLES: Duration = Duration::from_secs(2);
/// How long a copy sent up is kept staged before it's let go.
pub const KEPT: Duration = Duration::from_hours(7 * 24);

/// A file staged to go into Drive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staged {
    pub path: PathBuf,
    /// The folder it goes into, as the shell names it: `root` or an id.
    pub folder: String,
    pub name: String,
}

/// Where the picker stages saves: the state home's `google/drive-saves`,
/// the state home being `~/.local/state` when it isn't set, as the
/// shell's is, since a reboot empties the runtime folder.
pub fn root(state_home: Option<OsString>, home: Option<OsString>) -> PathBuf {
    state_home
        .filter(|home| !home.is_empty())
        .map_or_else(
            || PathBuf::from(home.unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        )
        .join("google/drive-saves")
}

/// The staged file a path names, or none: one right inside a folder of
/// `root` that Drive can add to, My Drive or a folder by its id, and
/// never a hidden name or a draft's.
pub fn staged(root: &Path, path: &Path) -> Option<Staged> {
    let folder = path.parent()?;
    if folder.parent()? != root {
        return None;
    }
    let named = folder.file_name()?.to_str()?;
    if !matches!(Folder::named(named), Some(Folder::Mine | Folder::Id(_))) {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    if name.starts_with('.') || name.starts_with('~') {
        return None;
    }
    Some(Staged {
        path: path.to_owned(),
        folder: named.to_owned(),
        name: name.to_owned(),
    })
}

/// The mark kept beside a staged file once it's up: `.<name>.drive`.
fn mark(staged: &Path) -> PathBuf {
    let name = staged.file_name().unwrap_or_default().to_string_lossy();
    staged.with_file_name(format!(".{name}.drive"))
}

/// The id Drive gave the file staged at `staged`, as kept beside it.
pub fn sent(staged: &Path) -> Option<String> {
    let id = fs::read_to_string(mark(staged)).ok()?;
    let id = id.trim();
    drive::is_id(id).then(|| id.to_owned())
}

/// Keeps the id Drive gave the file staged at `staged` beside it, written
/// whole, so a later save of it sends a new version of the same file.
pub fn keep(staged: &Path, id: &str) -> io::Result<()> {
    let mark = mark(staged);
    let writing = mark.with_extension("drive.new");
    fs::write(&writing, id)?;
    fs::rename(writing, mark)
}

/// Whether a staged file is still to go up: it has no mark, or it was
/// written since its mark was.
pub fn waiting(staged: &Path) -> bool {
    let written = |path: &Path| fs::metadata(path).and_then(|found| found.modified()).ok();
    match (written(staged), written(&mark(staged))) {
        (Some(file), Some(mark)) => file > mark,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// The staged files in one of `root`'s folders still to go up.
fn pending(root: &Path, folder: &Path) -> Vec<Staged> {
    let Ok(found) = fs::read_dir(folder) else {
        return Vec::new();
    };
    found
        .flatten()
        .filter_map(|entry| staged(root, &entry.path()))
        .filter(|staged| waiting(&staged.path))
        .collect()
}

/// Lets go of each staged copy sent up more than `KEPT` before `now`, and
/// of its mark; one written since it was sent, or never sent, stays.
pub fn prune(root: &Path, now: SystemTime) {
    let Ok(folders) = fs::read_dir(root) else {
        return;
    };
    for folder in folders.flatten() {
        let Ok(found) = fs::read_dir(folder.path()) else {
            continue;
        };
        for entry in found.flatten() {
            let Some(staged) = staged(root, &entry.path()) else {
                continue;
            };
            let marked = fs::metadata(mark(&staged.path)).and_then(|found| found.modified());
            let old = marked.is_ok_and(|at| now.duration_since(at).is_ok_and(|since| since > KEPT));
            if old && !waiting(&staged.path) {
                let _ = fs::remove_file(&staged.path);
                let _ = fs::remove_file(mark(&staged.path));
            }
        }
    }
}

/// The line that asks this program to send a staged file up, as a shell
/// would ask it on the socket.
pub fn asking(staged: &Staged) -> String {
    serde_json::json!({ "upload": { "path": staged.path } }).to_string()
}

/// Watches `root` and its folders, making `root` if there's none, and
/// asks for each staged file to be sent up once it has been left alone
/// `SETTLES`, by a line on `asks`: those already waiting at once, and
/// those in a folder made later as its watch is added, since the picker
/// may write one before the watch is there.
pub fn watch(root: PathBuf, asks: Sender<String>) -> io::Result<()> {
    fs::create_dir_all(&root)?;
    let mut inotify = Inotify::init()?;
    let top = inotify.watches().add(
        &root,
        WatchMask::CREATE | WatchMask::MOVED_TO | WatchMask::ONLYDIR,
    )?;
    let (touch, touched) = mpsc::channel::<PathBuf>();
    let tell = touch.clone();
    let mut folders: HashMap<WatchDescriptor, PathBuf> = HashMap::new();
    let staging = root.clone();
    let follow = move |inotify: &mut Inotify,
                       folders: &mut HashMap<WatchDescriptor, PathBuf>,
                       folder: PathBuf| {
        let added = inotify
            .watches()
            .add(&folder, WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO);
        match added {
            Ok(watched) => {
                for staged in pending(&staging, &folder) {
                    let _ = touch.send(staged.path);
                }
                folders.insert(watched, folder);
            }
            Err(error) => eprintln!("<4>couldn't watch a Drive save's folder: {error}"),
        }
    };
    for folder in fs::read_dir(&root)?.flatten() {
        if folder.path().is_dir() {
            follow(&mut inotify, &mut folders, folder.path());
        }
    }
    let made = root.clone();
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            let Ok(events) = inotify.read_events_blocking(&mut buffer) else {
                eprintln!("<4>stopped watching Drive saves");
                return;
            };
            let mut added = Vec::new();
            for event in events {
                let Some(name) = event.name else { continue };
                if event.wd == top {
                    if event.mask.contains(EventMask::ISDIR) {
                        added.push(made.join(name));
                    }
                } else if let Some(folder) = folders.get(&event.wd) {
                    let _ = tell.send(folder.join(name));
                }
            }
            for folder in added {
                follow(&mut inotify, &mut folders, folder);
            }
        }
    });
    thread::spawn(move || settle(&root, &touched, &asks));
    Ok(())
}

/// Asks for each file touched to be sent up once it has been left alone
/// `SETTLES`, if it's still there and still to go up.
fn settle(root: &Path, touched: &mpsc::Receiver<PathBuf>, asks: &Sender<String>) {
    let mut last: HashMap<PathBuf, Instant> = HashMap::new();
    loop {
        let next = last
            .values()
            .min()
            .map(|at| (*at + SETTLES).saturating_duration_since(Instant::now()));
        let heard = match next {
            Some(wait) => touched.recv_timeout(wait),
            None => touched.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match heard {
            Ok(path) => {
                last.insert(path, Instant::now());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let now = Instant::now();
        let (due, still): (HashMap<_, _>, HashMap<_, _>) = last
            .drain()
            .partition(|(_, at)| now.duration_since(*at) >= SETTLES);
        last = still;
        for path in due.into_keys() {
            if let Some(staged) = staged(root, &path).filter(|staged| waiting(&staged.path))
                && asks.send(asking(&staged)).is_err()
            {
                return;
            }
        }
    }
}

/// The type a file's name says it is, for Drive, which otherwise guesses.
pub fn mime(name: &str) -> &'static str {
    let ending = name
        .rsplit_once('.')
        .map_or("", |(_, ending)| ending)
        .to_ascii_lowercase();
    match ending.as_str() {
        "pdf" => "application/pdf",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "zip" => "application/zip",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(test: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("google-saves-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("root")).unwrap();
        root
    }

    #[test]
    fn the_state_home_holds_the_saves_or_the_home_s_local_state_does() {
        assert_eq!(
            root(Some("/s".into()), Some("/h".into())),
            PathBuf::from("/s/google/drive-saves")
        );
        assert_eq!(
            root(Some("".into()), Some("/h".into())),
            PathBuf::from("/h/.local/state/google/drive-saves")
        );
    }

    #[test]
    fn only_a_file_right_in_a_folder_drive_can_add_to_is_staged() {
        let root = Path::new("/s/drive-saves");
        assert_eq!(
            staged(root, &root.join("root/Notes.txt")),
            Some(Staged {
                path: root.join("root/Notes.txt"),
                folder: "root".into(),
                name: "Notes.txt".into(),
            })
        );
        assert!(staged(root, &root.join("1AbC-d_9/Plan.pdf")).is_some());
        for path in [
            "root/.Notes.txt.drive",
            "root/~lock",
            "shared/a",
            "drives/a",
            "a b/x",
            "x",
        ] {
            assert_eq!(staged(root, &root.join(path)), None, "{path}");
        }
        assert_eq!(staged(root, &root.join("root/deeper/x")), None);
        assert_eq!(staged(root, Path::new("/elsewhere/root/x")), None);
    }

    #[test]
    fn a_file_waits_until_its_mark_is_kept_and_again_once_written_since() {
        let root = folder("waits");
        let path = root.join("root/Notes.txt");
        fs::write(&path, "one").unwrap();
        assert!(waiting(&path));
        assert_eq!(pending(&root, &root.join("root")).len(), 1);
        keep(&path, "f1").unwrap();
        assert!(!waiting(&path));
        assert_eq!(sent(&path).as_deref(), Some("f1"));
        assert!(pending(&root, &root.join("root")).is_empty());
        let later = SystemTime::now() + Duration::from_secs(5);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(waiting(&path));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_a_copy_sent_up_a_week_ago_is_let_go() {
        let root = folder("prune");
        let sent_up = root.join("root/Old.txt");
        let never = root.join("root/Never.txt");
        fs::write(&sent_up, "x").unwrap();
        fs::write(&never, "x").unwrap();
        keep(&sent_up, "f1").unwrap();
        prune(&root, SystemTime::now() + KEPT / 2);
        assert!(sent_up.exists());
        prune(&root, SystemTime::now() + KEPT + Duration::from_secs(60));
        assert!(!sent_up.exists());
        assert!(sent(&sent_up).is_none());
        assert!(never.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_file_written_is_asked_for_once_it_settles_as_is_one_in_a_folder_made_later() {
        let root = folder("watch");
        let early = root.join("root/Early.txt");
        fs::write(&early, "x").unwrap();
        let (asks, asked) = mpsc::channel();
        watch(root.clone(), asks).unwrap();
        let wait = SETTLES * 3;
        assert_eq!(
            asked.recv_timeout(wait).unwrap(),
            asking(&staged(&root, &early).unwrap())
        );
        let written = root.join("root/Notes.txt");
        fs::write(&written, "x").unwrap();
        fs::write(root.join("root/.hidden"), "x").unwrap();
        assert_eq!(
            asked.recv_timeout(wait).unwrap(),
            asking(&staged(&root, &written).unwrap())
        );
        fs::create_dir(root.join("abc123")).unwrap();
        let later = root.join("abc123/Plan.pdf");
        fs::write(&later, "x").unwrap();
        assert_eq!(
            asked.recv_timeout(wait).unwrap(),
            asking(&staged(&root, &later).unwrap())
        );
        assert!(asked.recv_timeout(SETTLES).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_name_says_its_type_and_an_unknown_one_is_bytes() {
        assert_eq!(mime("Plan.PDF"), "application/pdf");
        assert_eq!(mime("notes"), "application/octet-stream");
    }
}
