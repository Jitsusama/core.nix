//! Where a file mail carries, or one of Drive's, goes once it's fetched:
//! a mail's into a folder only Joel can read, under the session's runtime
//! folder, to be opened from there and gone when he logs out, and a Drive
//! file's into the cache, as below, to be opened or handed to the program
//! that asked the picker for it; or, to keep, where he picked
//! in the launcher's chooser, else into his Downloads. A name is the
//! sender's, so it's made safe to be a file's name, and a copy saved into
//! Downloads never writes over a file already there; one put where he
//! picked does, since the chooser asked him first. Each is written as it
//! comes, never held whole, and a file left half written is taken away.
//!
//! A Drive file to open or hand on is kept apart, in the cache home's
//! google/drive, so a large one never sits in memory as the runtime
//! folder's do, and a program handed its path still finds it after a
//! logout. Each is the version Drive last changed, marked with that
//! change as its own, so asking again while it's unchanged fetches
//! nothing; it's readable only, so a program that opens it says it can't
//! be written rather than letting a change be lost, since nothing written
//! there goes back to Drive. Past `CACHED_MOST` together, the copies
//! handed out longest ago are let go.

use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{self, Read};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The longest a name is kept, in bytes, short of what a file system
/// takes, leaving room for a number to tell copies apart.
const NAME_MOST: usize = 200;

/// What Joel does with a file once it's fetched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum How {
    /// Open it in the program the desktop opens its kind with.
    Open,
    /// Keep a copy where Joel picked, else in Downloads.
    Save,
    /// Hand it to the program that asked the picker for a file: kept as
    /// one to open is, but never opened here, so one that runs may be.
    Pick,
}

/// A reader that remembers whether reading it failed, which tells Google
/// breaking off from the disk refusing what came.
pub struct Heard<R> {
    from: R,
    pub fell: bool,
}

impl<R> Heard<R> {
    pub fn new(from: R) -> Heard<R> {
        Heard { from, fell: false }
    }
}

impl<R: Read> Read for Heard<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.from.read(into).inspect_err(|_| self.fell = true)
    }
}

/// Writes what `from` gives into `file`, made at `path`, taking the file
/// away when that fails, so nothing half written is left.
fn filled(mut file: File, path: &Path, from: &mut dyn Read) -> io::Result<()> {
    io::copy(from, &mut file)
        .and_then(|_| file.sync_all())
        .inspect_err(|_| {
            let _ = fs::remove_file(path);
        })
}

/// How much the copies of Drive's files kept to open or hand on may take
/// together, in bytes, before those handed out longest ago are let go:
/// room for a few videos, and far past the documents a week picks.
pub const CACHED_MOST: u64 = 2_000_000_000;

/// The folders a file goes to.
pub struct Places {
    /// Where one to open is kept: the session's, which goes at logout.
    pub kept: PathBuf,
    /// Where a Drive file to open or hand on is kept: the cache home's,
    /// which outlasts the session.
    pub drive: PathBuf,
    /// Where a copy to keep is saved: Downloads.
    pub saves: PathBuf,
}

/// The file last asked to be opened or saved, as it stands, which the
/// shell waits on: being fetched, then where it is or why it isn't.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fetched {
    /// The mail it's on, by its id.
    pub mail: String,
    /// Which of the mail's files, from 0.
    pub file: usize,
    pub how: How,
    /// Where Joel picked to save it, kept from the shell's ask; the shell
    /// knows it already, so it isn't said back.
    #[serde(skip)]
    pub to: Option<String>,
    /// When it was asked, in milliseconds since the epoch, which tells
    /// one ask from the next of the same file.
    pub asked: i64,
    /// Where it is once it's fetched.
    pub path: Option<String>,
    /// Why it couldn't be, which the shell says in its own words.
    pub failed: Option<Failed>,
}

/// Why a file couldn't be opened or saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Failed {
    /// The mail, or the file on it, isn't held any more.
    Gone,
    /// It's a program or a script, which is only ever saved.
    Runs,
    /// Gmail couldn't be reached, or wouldn't give it.
    Unreachable,
    /// It couldn't be written down.
    Unwritten,
}

/// A sender's name for a file, made one a folder can hold: no folder in
/// it, nothing hidden, nothing that isn't printed, at most `NAME_MOST`
/// bytes, and "file" when nothing's left.
pub fn named(name: &str) -> String {
    let kept: String = name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let kept = kept.trim().trim_start_matches('.');
    let mut end = kept.len().min(NAME_MOST);
    while !kept.is_char_boundary(end) {
        end -= 1;
    }
    match kept[..end].trim() {
        "" => "file".into(),
        name => name.into(),
    }
}

/// Writes what `from` gives as `name` in a folder of `whose` own (the
/// mail or the file it came from) under `folder`, readable only by Joel,
/// replacing what an earlier fetch of it left, and says where.
pub fn kept(folder: &Path, whose: &str, name: &str, from: &mut dyn Read) -> io::Result<PathBuf> {
    let inside = folder.join(whose);
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&inside)?;
    let path = inside.join(named(name));
    let mut new = path.as_os_str().to_owned();
    new.push(".new");
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&new)?;
    let new = PathBuf::from(new);
    filled(file, &new, from)?;
    fs::rename(&new, &path).inspect_err(|_| {
        let _ = fs::remove_file(&new);
    })?;
    Ok(path)
}

/// A moment in milliseconds since the epoch as the file system keeps one.
fn moment(at: i64) -> Option<SystemTime> {
    let after = Duration::from_millis(u64::try_from(at).ok()?);
    UNIX_EPOCH.checked_add(after)
}

/// The copy of Drive's file `whose`, named `name`, kept under `folder`,
/// when it's the version Drive last changed at `changed`, in milliseconds
/// since the epoch; none when there's no copy, an older or a newer one, or
/// Drive gave no change to know it by. One found is marked handed out now.
pub fn fresh(folder: &Path, whose: &str, name: &str, changed: Option<i64>) -> Option<PathBuf> {
    let inside = folder.join(whose);
    let path = inside.join(named(name));
    let kept = fs::metadata(&path).ok()?.modified().ok()?;
    if Some(kept) != moment(changed?) {
        return None;
    }
    used(&inside);
    Some(path)
}

/// Writes what `from` gives as `name` in a folder of `whose` own under
/// `folder`, as `kept` does, but readable only and marked changed when
/// Drive changed it, `changed`, so `fresh` knows it again; anything else
/// in that folder, a copy under the name it had before, goes.
pub fn cached(
    folder: &Path,
    whose: &str,
    name: &str,
    changed: Option<i64>,
    from: &mut dyn Read,
) -> io::Result<PathBuf> {
    let inside = folder.join(whose);
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&inside)?;
    let path = inside.join(named(name));
    let new = inside.join(".new");
    let _ = fs::remove_file(&new);
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&new)?;
    let marked = file.try_clone()?;
    filled(file, &new, from)?;
    let done = changed
        .and_then(moment)
        .map_or(Ok(()), |at| marked.set_modified(at))
        .and_then(|()| marked.set_permissions(Permissions::from_mode(0o400)))
        .and_then(|()| fs::rename(&new, &path));
    done.inspect_err(|_| {
        let _ = fs::remove_file(&new);
    })?;
    for other in fs::read_dir(&inside)?.flatten() {
        if other.path() != path {
            let _ = fs::remove_file(other.path());
        }
    }
    used(&inside);
    Ok(path)
}

/// Marks a copy's folder handed out now, so `pruned` lets it go last.
fn used(inside: &Path) {
    if let Err(error) = File::open(inside).and_then(|folder| folder.set_modified(SystemTime::now()))
    {
        eprintln!("<4>couldn't mark a Drive copy used: {error}");
    }
}

/// Lets go of the copies under `folder` handed out longest ago until those
/// left take at most `most` bytes together, never `whose`, the one just
/// handed out, which a program may be opening.
pub fn pruned(folder: &Path, most: u64, whose: &str) {
    let Ok(read) = fs::read_dir(folder) else {
        return;
    };
    let mut copies: Vec<(SystemTime, u64, PathBuf)> = read
        .flatten()
        .filter(|copy| copy.file_name() != whose)
        .filter_map(|copy| {
            let used = copy.metadata().ok()?.modified().ok()?;
            let size = fs::read_dir(copy.path())
                .ok()?
                .flatten()
                .filter_map(|file| file.metadata().ok())
                .map(|file| file.len())
                .sum();
            Some((used, size, copy.path()))
        })
        .collect();
    let held: u64 = fs::read_dir(folder.join(whose))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|file| file.metadata().ok())
        .map(|file| file.len())
        .sum();
    let mut total = held + copies.iter().map(|(_, size, _)| size).sum::<u64>();
    copies.sort();
    for (_, size, path) in copies {
        if total <= most {
            break;
        }
        match fs::remove_dir_all(&path) {
            Ok(()) => total -= size,
            Err(error) => eprintln!("<4>couldn't let a Drive copy go: {error}"),
        }
    }
}

/// Writes what `from` gives as the file `path`, replacing one there, whole
/// or not at all, and says where. Its folder is made when it's missing, as
/// the chooser offers Downloads before anything was ever saved there.
pub fn put(path: &Path, from: &mut dyn Read) -> io::Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file named"));
    };
    let mut new = std::ffi::OsString::from(".");
    new.push(name);
    new.push(".new");
    let new = path.with_file_name(new);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&new)?;
    filled(file, &new, from)?;
    fs::rename(&new, path).inspect_err(|_| {
        let _ = fs::remove_file(&new);
    })?;
    Ok(path.to_owned())
}

/// Writes what `from` gives as `name` in `folder`, or, when that's taken,
/// as the name with the first number from 2 that isn't, before its ending,
/// as a browser saves, and says where.
pub fn saved(folder: &Path, name: &str, from: &mut dyn Read) -> io::Result<PathBuf> {
    fs::create_dir_all(folder)?;
    let name = named(name);
    let (stem, ending) = match name.rsplit_once('.') {
        Some((stem, ending)) if !stem.is_empty() => (stem, format!(".{ending}")),
        _ => (name.as_str(), String::new()),
    };
    for copy in 1.. {
        let path = if copy == 1 {
            folder.join(&name)
        } else {
            folder.join(format!("{stem} ({copy}){ending}"))
        };
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                filled(file, &path, from)?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    unreachable!("some number from 2 is free")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A reader that gives a little, then breaks off, as a download cut
    /// short does.
    struct Cut(bool);

    impl Read for Cut {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            if std::mem::replace(&mut self.0, true) {
                return Err(io::ErrorKind::ConnectionReset.into());
            }
            into[..4].copy_from_slice(b"half");
            Ok(4)
        }
    }

    fn folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("google-files-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn a_name_never_leaves_its_folder_or_hides() {
        assert_eq!(named("menu.pdf"), "menu.pdf");
        assert_eq!(named("../../.bashrc"), "_.._.bashrc");
        assert_eq!(named(".hidden"), "hidden");
        assert_eq!(named("a\nb\\c"), "a_b_c");
        assert_eq!(named(" .. "), "file");
        assert_eq!(named(&"é".repeat(150)).len(), NAME_MOST);
    }

    #[test]
    fn a_file_to_open_is_kept_where_only_joel_reads_it_and_fetched_again_over_itself() {
        let folder = folder("kept");
        let path = kept(&folder, "m1", "menu.pdf", &mut &b"one"[..]).unwrap();
        assert_eq!(path, folder.join("m1").join("menu.pdf"));
        let path = kept(&folder, "m1", "menu.pdf", &mut &b"two"[..]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&folder.join("m1")), 0o700);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_saved_copy_never_writes_over_one_already_there() {
        let folder = folder("saved");
        assert_eq!(
            saved(&folder, "menu.pdf", &mut &b"1"[..]).unwrap(),
            folder.join("menu.pdf")
        );
        assert_eq!(
            saved(&folder, "menu.pdf", &mut &b"2"[..]).unwrap(),
            folder.join("menu (2).pdf")
        );
        assert_eq!(
            saved(&folder, "menu.pdf", &mut &b"3"[..]).unwrap(),
            folder.join("menu (3).pdf")
        );
        assert_eq!(
            saved(&folder, "notes", &mut &b"4"[..]).unwrap(),
            folder.join("notes")
        );
        assert_eq!(
            saved(&folder, "notes", &mut &b"5"[..]).unwrap(),
            folder.join("notes (2)")
        );
        assert_eq!(fs::read(folder.join("menu.pdf")).unwrap(), b"1");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_copy_put_where_joel_picked_replaces_what_was_there_and_leaves_nothing_beside() {
        let folder = folder("put");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("plan.pdf");
        fs::write(&path, b"old").unwrap();
        assert_eq!(put(&path, &mut &b"new"[..]).unwrap(), path);
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        let unmade = folder.join("unmade").join("plan.pdf");
        assert_eq!(put(&unmade, &mut &b"x"[..]).unwrap(), unmade);
        let held = folder.join("held");
        fs::create_dir(&held).unwrap();
        assert!(
            put(&held, &mut &b"x"[..]).is_err(),
            "a folder was written over"
        );
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 3);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_download_cut_short_leaves_nothing_and_says_it_was_the_far_side() {
        let folder = folder("cut");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("plan.pdf");
        fs::write(&path, b"old").unwrap();
        let mut heard = Heard::new(Cut(false));
        assert!(put(&path, &mut heard).is_err());
        assert!(heard.fell);
        assert_eq!(fs::read(&path).unwrap(), b"old");
        let mut heard = Heard::new(Cut(false));
        assert!(saved(&folder, "notes.txt", &mut heard).is_err());
        let mut heard = Heard::new(Cut(false));
        assert!(kept(&folder, "f1", "notes.txt", &mut heard).is_err());
        assert_eq!(
            fs::read_dir(&folder).unwrap().count(),
            2,
            "something was left"
        );
        assert_eq!(fs::read_dir(folder.join("f1")).unwrap().count(), 0);
        let mut heard = Heard::new(&b"whole"[..]);
        assert_eq!(put(&path, &mut heard).unwrap(), path);
        assert!(!heard.fell);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_drive_copy_is_found_again_only_while_drive_hasnt_changed_it_and_reads_only() {
        let folder = folder("cached");
        let at = 1_793_037_600_000;
        assert_eq!(fresh(&folder, "f1", "plan.pdf", Some(at)), None);
        let path = cached(&folder, "f1", "plan.pdf", Some(at), &mut &b"one"[..]).unwrap();
        assert_eq!(path, folder.join("f1").join("plan.pdf"));
        assert_eq!(
            fresh(&folder, "f1", "plan.pdf", Some(at)),
            Some(path.clone())
        );
        assert_eq!(fresh(&folder, "f1", "plan.pdf", Some(at + 1)), None);
        assert_eq!(fresh(&folder, "f1", "plan.pdf", None), None);
        assert_eq!(fresh(&folder, "f1", "renamed.pdf", Some(at)), None);
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o400);
        assert_eq!(mode(&folder.join("f1")), 0o700);
        // A newer version replaces it, and a copy under its old name goes.
        let path = cached(&folder, "f1", "plan v2.pdf", Some(at + 1), &mut &b"two"[..]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        let names: Vec<_> = fs::read_dir(folder.join("f1"))
            .unwrap()
            .map(|file| file.unwrap().file_name())
            .collect();
        assert_eq!(names, ["plan v2.pdf"]);
        // One cut short leaves the copy before it as it was.
        let mut heard = Heard::new(Cut(false));
        assert!(cached(&folder, "f1", "plan v2.pdf", Some(at + 2), &mut heard).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert_eq!(fs::read_dir(folder.join("f1")).unwrap().count(), 1);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_drive_copies_handed_out_longest_ago_go_first_never_the_one_just_handed_out() {
        let folder = folder("pruned");
        for whose in ["old", "mid", "new"] {
            cached(&folder, whose, "f", None, &mut &[0; 10][..]).unwrap();
            // Each handed out a moment after the last, as the file
            // system tells moments apart.
            std::thread::sleep(Duration::from_millis(20));
        }
        // Handing out the oldest again makes it the newest.
        assert!(fresh(&folder, "old", "f", None).is_none());
        let at = 1_793_037_600_000;
        cached(&folder, "old", "f", Some(at), &mut &[0; 10][..]).unwrap();
        pruned(&folder, 30, "new");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 3, "room was there");
        pruned(&folder, 20, "mid");
        let mut left: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|copy| copy.unwrap().file_name())
            .collect();
        left.sort();
        assert_eq!(left, ["mid", "old"]);
        pruned(&folder, 0, "mid");
        let left: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|copy| copy.unwrap().file_name())
            .collect();
        assert_eq!(left, ["mid"]);
        fs::remove_dir_all(&folder).unwrap();
    }
}
