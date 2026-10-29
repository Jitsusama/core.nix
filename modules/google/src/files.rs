//! Where a file mail carries goes once it's fetched: into a folder only
//! Joel can read, under the session's runtime folder, to be opened from
//! there and gone when he logs out; or, to keep, where he picked in the
//! launcher's chooser, else into his Downloads. A name is the sender's, so
//! it's made safe to be a file's name, and a copy saved into Downloads
//! never writes over a file already there; one put where he picked does,
//! since the chooser asked him first.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

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
}

/// The two folders a file goes to.
pub struct Places {
    /// Where one to open is kept: the session's, which goes at logout.
    pub kept: PathBuf,
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

/// Writes `bytes` as `name` in a folder of the mail `mail`'s own under
/// `folder`, readable only by Joel, replacing what an earlier fetch of it
/// left, and says where.
pub fn kept(folder: &Path, mail: &str, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let inside = folder.join(mail);
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&inside)?;
    let path = inside.join(named(name));
    let mut new = path.as_os_str().to_owned();
    new.push(".new");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&new)?;
    file.write_all(bytes)?;
    fs::rename(&new, &path)?;
    Ok(path)
}

/// Writes `bytes` as the file `path`, replacing one there, whole or not at
/// all, and says where. Its folder is made when it's missing, as the
/// chooser offers Downloads before anything was ever saved there.
pub fn put(path: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
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
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&new)?;
    file.write_all(bytes)?;
    fs::rename(&new, path).inspect_err(|_| {
        let _ = fs::remove_file(&new);
    })?;
    Ok(path.to_owned())
}

/// Writes `bytes` as `name` in `folder`, or, when that's taken, as the
/// name with the first number from 2 that isn't, before its ending, as a
/// browser saves, and says where.
pub fn saved(folder: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
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
            Ok(mut file) => {
                file.write_all(bytes)?;
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
        let path = kept(&folder, "m1", "menu.pdf", b"one").unwrap();
        assert_eq!(path, folder.join("m1").join("menu.pdf"));
        let path = kept(&folder, "m1", "menu.pdf", b"two").unwrap();
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
            saved(&folder, "menu.pdf", b"1").unwrap(),
            folder.join("menu.pdf")
        );
        assert_eq!(
            saved(&folder, "menu.pdf", b"2").unwrap(),
            folder.join("menu (2).pdf")
        );
        assert_eq!(
            saved(&folder, "menu.pdf", b"3").unwrap(),
            folder.join("menu (3).pdf")
        );
        assert_eq!(saved(&folder, "notes", b"4").unwrap(), folder.join("notes"));
        assert_eq!(
            saved(&folder, "notes", b"5").unwrap(),
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
        assert_eq!(put(&path, b"new").unwrap(), path);
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        let unmade = folder.join("unmade").join("plan.pdf");
        assert_eq!(put(&unmade, b"x").unwrap(), unmade);
        let held = folder.join("held");
        fs::create_dir(&held).unwrap();
        assert!(put(&held, b"x").is_err(), "a folder was written over");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 3);
        fs::remove_dir_all(&folder).unwrap();
    }
}
