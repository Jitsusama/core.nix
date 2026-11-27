//! The mesh's messages read strictly, for a measure that asks their
//! senders: each one named as its file is, and every one there read or
//! said why not, since a measure reading what's left would close asks on
//! messages it never saw. It reads the folder as the symbiosis measure's
//! own reader does: plain files only, never a link or a pipe, and day
//! folders only by their dates.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{Mesh, Message, stamped_at};

/// A day, in milliseconds: a message is filed into its day's folder a
/// little after it's sent, so a day's folder from the day before the
/// first wanted may still hold one.
const DAY_MS: i64 = 24 * 3600 * 1000;

impl Mesh {
    /// Every message stamped from `from_ms` to `to_ms` whose file's stem
    /// is `wanted` and whose body names the id its file is named by, and
    /// the ids of the files there that aren't messages. As
    /// [`Mesh::senders`] holds, a body naming another id is no one's, and
    /// a measure that asks its sender would ask the wrong cell.
    ///
    /// Unlike [`Mesh::sent_since`], the messages folder or a day's folder
    /// that can't be listed, or a message that can't be read for any
    /// reason but its file having moved, is said why. The harness files
    /// each message into its day's folder after a while, so one gone from
    /// the first listing is sought again in a second, and a day's folder
    /// gone is one with nothing in it. A file that isn't a message, as one
    /// torn in writing, is named rather than read, so one bad file never
    /// blinds the rest; `to_ms` keeps a file stamped in the future from
    /// being named every hour for good. Only plain files are opened, so a
    /// pipe or a link to a device can't hold the read for ever.
    pub fn sent_as_named(
        &self,
        from_ms: i64,
        to_ms: i64,
        wanted: impl Fn(&str) -> bool,
    ) -> Result<(Vec<Message>, Vec<String>), String> {
        let messages = self.messages();
        read_listed(|| files(&messages, from_ms), from_ms, to_ms, wanted)
    }
}

/// The messages and torn files among those `listing` gives, stamped from
/// `from_ms` to `to_ms` and `wanted`, listing again to find any that moved
/// between a listing and their reading.
fn read_listed(
    listing: impl Fn() -> Result<Vec<PathBuf>, String>,
    from_ms: i64,
    to_ms: i64,
    wanted: impl Fn(&str) -> bool,
) -> Result<(Vec<Message>, Vec<String>), String> {
    let stamped = |p: &&PathBuf| {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        stamped_at(stem).is_some_and(|at| (from_ms..=to_ms).contains(&at)) && wanted(stem)
    };
    let (mut read, mut moved) = (Read::default(), BTreeSet::<Option<OsString>>::new());
    for path in listing()?.iter().filter(stamped) {
        if !read.file(path)? {
            moved.insert(path.file_name().map(ToOwned::to_owned));
        }
    }
    if !moved.is_empty() {
        for path in listing()?.iter().filter(stamped) {
            if moved.remove(&path.file_name().map(ToOwned::to_owned)) {
                read.file(path)?;
            }
        }
    }
    Ok((read.sent, read.torn))
}

/// The message files under `messages`: those in it, and those in each
/// day's folder from the day before `from_ms` on.
fn files(messages: &Path, from_ms: i64) -> Result<Vec<PathBuf>, String> {
    let earliest = jiff::Timestamp::from_millisecond(from_ms - DAY_MS)
        .map(|at| at.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let dated = |name: &str| {
        let bytes = name.as_bytes();
        bytes.len() == 10
            && bytes.iter().enumerate().all(|(at, &b)| {
                if at == 4 || at == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            })
    };
    let (mut files, folders) = listed(messages, true)?;
    for (name, folder) in folders {
        if dated(&name) && name >= earliest {
            files.extend(listed(&folder, false)?.0);
        }
    }
    Ok(files)
}

/// A folder's plain message files, and its folders by name.
type Listed = (Vec<PathBuf>, Vec<(String, PathBuf)>);

/// The plain `.json` files in `folder` and its folders by name, never
/// following a link. A day's folder gone between listings has nothing in
/// it; the messages folder itself gone is said, as anything else that
/// can't be listed is.
fn listed(folder: &Path, root: bool) -> Result<Listed, String> {
    let why = |e: &io::Error| format!("{} can't be listed: {e}", folder.display());
    let entries = match fs::read_dir(folder) {
        Err(e) if e.kind() == io::ErrorKind::NotFound && !root => return Ok(Listed::default()),
        entries => entries.map_err(|e| why(&e))?,
    };
    let (mut files, mut folders) = Listed::default();
    for entry in entries {
        let entry = entry.map_err(|e| why(&e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let kind = match entry.file_type() {
            Ok(kind) => kind,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(why(&e)),
        };
        if kind.is_dir() {
            folders.push((name, entry.path()));
        } else if kind.is_file() && Path::new(&name).extension().is_some_and(|e| e == "json") {
            files.push(entry.path());
        }
    }
    Ok((files, folders))
}

/// What [`Mesh::sent_as_named`] has read: the messages named as their
/// files are, and the ids of the files that aren't messages.
#[derive(Default)]
struct Read {
    sent: Vec<Message>,
    torn: Vec<String>,
}

impl Read {
    /// Reads the message at `path`, saying false when its file has moved
    /// away and why when it can't be read for any other reason.
    fn file(&mut self, path: &Path) -> Result<bool, String> {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            return Ok(true);
        };
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(format!("{stem} can't be read: {e}")),
        };
        match serde_json::from_slice::<Loose>(&bytes) {
            Ok(loose) if loose.id == stem => self.sent.push(loose.into()),
            Ok(_) => {}
            Err(_) => self.torn.push(stem.to_owned()),
        }
        Ok(true)
    }
}

/// A message as far as a measure needs one: its id, sender and words,
/// with whatever else it carries, since a message missing a field
/// cadence never reads is still a message.
#[derive(Deserialize)]
struct Loose {
    id: String,
    from: String,
    text: String,
    #[serde(default)]
    v: u32,
    #[serde(default)]
    at: String,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    intent: String,
    #[serde(default)]
    pid: u32,
    #[serde(default)]
    re: Option<String>,
    #[serde(default)]
    supersedes: Option<String>,
    #[serde(default, rename = "endsHold")]
    ends_hold: bool,
}

impl From<Loose> for Message {
    fn from(m: Loose) -> Self {
        Message {
            v: m.v,
            id: m.id,
            at: m.at,
            from: m.from,
            to: m.to,
            intent: m.intent,
            text: m.text,
            pid: m.pid,
            re: m.re,
            supersedes: m.supersedes,
            ends_hold: m.ends_hold,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// 12:00 on 26 November 2026, in milliseconds.
    const NOON: i64 = 1_795_694_400_000;

    fn mesh() -> (tempfile::TempDir, Mesh) {
        let dir = tempfile::tempdir().expect("the test's folder couldn't be made");
        fs::create_dir_all(dir.path().join("mesh-workflow/messages"))
            .expect("the messages folder couldn't be made");
        let mesh = Mesh {
            root: dir.path().to_owned(),
        };
        (dir, mesh)
    }

    /// Writes a message `id` from `from` into `folder` under the mesh's
    /// messages folder, or into that folder itself when `folder` is empty.
    fn put(mesh: &Mesh, folder: &str, id: &str, from: &str) {
        let folder = mesh.messages().join(folder);
        fs::create_dir_all(&folder).expect("the day's folder couldn't be made");
        let body = serde_json::json!({"id": id, "from": from, "text": "hi", "to": ["@B"]});
        fs::write(folder.join(format!("{id}.json")), body.to_string())
            .expect("the message couldn't be written");
    }

    fn ids(sent: &[Message]) -> Vec<&str> {
        let mut ids: Vec<&str> = sent.iter().map(|m| m.id.as_str()).collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn a_message_whose_body_names_another_id_than_its_file_is_read_as_no_ones() {
        let (_dir, mesh) = mesh();
        put(&mesh, "", "10-A-1-aa", "A");
        put(&mesh, "", "11-A-1-bb", "A");
        fs::rename(
            mesh.messages().join("11-A-1-bb.json"),
            mesh.messages().join("12-B-1-cc.json"),
        )
        .expect("the message couldn't be renamed");
        let (sent, _) = mesh
            .sent_as_named(0, 100, |_| true)
            .expect("the messages were read");
        assert_eq!(
            ids(&sent),
            ["10-A-1-aa"],
            "a body naming another id than its file was read as sent"
        );
    }

    /// A file that isn't a message is named and read past, so the rest are
    /// still read, and a messages folder that can't be listed is said why
    /// rather than read as no messages, since a measure would close the
    /// asks of the messages it missed.
    #[test]
    fn what_isnt_a_message_is_named_and_a_folder_that_cant_be_listed_is_said() {
        let (dir, mesh) = mesh();
        put(&mesh, "", "10-A-1-aa", "A");
        fs::write(mesh.messages().join("11-A-1-bb.json"), "{")
            .expect("the torn file couldn't be written");
        let (sent, torn) = mesh
            .sent_as_named(0, 100, |_| true)
            .expect("the messages were read");
        assert_eq!(ids(&sent), ["10-A-1-aa"], "one torn file hid the others");
        assert_eq!(torn, ["11-A-1-bb"], "a torn file wasn't named");
        fs::remove_dir_all(dir.path().join("mesh-workflow")).expect("the mesh couldn't be removed");
        assert!(
            mesh.sent_as_named(0, 100, |_| true).is_err(),
            "a messages folder that can't be listed was read as no messages"
        );
    }

    /// A message missing what cadence never reads of it, as its version,
    /// time or process, is still read; one stamped past `to_ms`, or whose
    /// stem isn't wanted, is neither read nor named.
    #[test]
    fn a_spare_shape_is_read_and_the_future_and_the_unwanted_left_alone() {
        let (_dir, mesh) = mesh();
        put(&mesh, "", "10-A-1-aa", "A");
        put(&mesh, "", "20-B-1-bb", "B");
        fs::write(mesh.messages().join("900-A-1-zz.json"), "{")
            .expect("the future file couldn't be written");
        let (sent, torn) = mesh
            .sent_as_named(0, 100, |stem| !stem.contains("-B-"))
            .expect("the messages were read");
        let read: Vec<(&str, &[String])> = sent
            .iter()
            .map(|m| (m.id.as_str(), m.to.as_slice()))
            .collect();
        assert_eq!(
            read,
            [("10-A-1-aa", ["@B".to_owned()].as_slice())],
            "a message missing fields cadence doesn't read was read past, or an unwanted one read"
        );
        assert!(
            torn.is_empty(),
            "a file stamped past the read was named: {torn:?}"
        );
    }

    /// Messages filed into the day's folders are read from the day before
    /// the first wanted on, and nothing in a folder named for no day, nor
    /// anything but a plain file, as a link to a device that would never
    /// stop being read.
    #[test]
    fn days_folders_are_read_by_their_dates_and_only_plain_files_opened() {
        let (_dir, mesh) = mesh();
        let (top, kept, before, old, odd) = (
            format!("{}-A-1-aa", NOON + 60_000),
            format!("{}-A-1-bb", NOON + 120_000),
            format!("{}-A-1-cc", NOON + 180_000),
            format!("{}-A-1-dd", NOON + 240_000),
            format!("{}-A-1-ee", NOON + 300_000),
        );
        put(&mesh, "", &top, "A");
        put(&mesh, "2026-11-26", &kept, "A");
        put(&mesh, "2026-11-25", &before, "A");
        put(&mesh, "2026-11-24", &old, "A");
        put(&mesh, "kept-aside", &odd, "A");
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            "/dev/zero",
            mesh.messages()
                .join(format!("{}-A-1-ff.json", NOON + 360_000)),
        )
        .expect("the link couldn't be made");
        let (sent, torn) = mesh
            .sent_as_named(NOON, NOON + DAY_MS, |_| true)
            .expect("the messages were read");
        assert_eq!(
            ids(&sent),
            [top.as_str(), kept.as_str(), before.as_str()],
            "a day's folder was missed, one long past read, a folder for no day read, or a \
             link opened"
        );
        assert!(
            torn.is_empty(),
            "a message filed away was named torn: {torn:?}"
        );
    }

    /// A message gone between a listing and its reading has been filed
    /// into its day, so it's sought again in a second listing and read
    /// once, rather than said to be unreadable.
    #[test]
    fn a_message_moved_away_mid_read_is_read_where_it_went() {
        let (_dir, mesh) = mesh();
        let id = "10-A-1-aa";
        put(&mesh, "2026-11-26", id, "A");
        let (gone, went) = (
            mesh.messages().join(format!("{id}.json")),
            mesh.messages().join(format!("2026-11-26/{id}.json")),
        );
        let listings = Cell::new(0);
        let listing = || {
            listings.set(listings.get() + 1);
            Ok(vec![if listings.get() == 1 {
                gone.clone()
            } else {
                went.clone()
            }])
        };
        let (sent, torn) =
            read_listed(listing, 0, 100, |_| true).expect("the moved message was read");
        assert_eq!(
            (ids(&sent), torn.len(), listings.get()),
            (vec![id], 0, 2),
            "a message moved mid-read was lost, said unreadable or listed for again needlessly"
        );
    }

    /// A day's folder gone between listings has nothing in it, but one
    /// that can't be listed for any other reason is said why.
    #[cfg(unix)]
    #[test]
    fn a_days_folder_gone_is_empty_and_one_that_cant_be_listed_is_said() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, mesh) = mesh();
        assert_eq!(
            listed(&mesh.messages().join("2026-11-26"), false),
            Ok(Listed::default()),
            "a day's folder gone mid-read was said unlistable"
        );
        put(
            &mesh,
            "2026-11-26",
            &format!("{}-A-1-aa", NOON + 60_000),
            "A",
        );
        let day = mesh.messages().join("2026-11-26");
        fs::set_permissions(&day, fs::Permissions::from_mode(0o000))
            .expect("the day's folder couldn't be shut");
        let shut = fs::read_dir(&day).is_err();
        let read = mesh.sent_as_named(NOON, NOON + DAY_MS, |_| true);
        fs::set_permissions(&day, fs::Permissions::from_mode(0o755))
            .expect("the day's folder couldn't be opened again");
        // Run as root, a shut folder still lists, so there's nothing to see.
        if shut {
            assert!(
                read.is_err(),
                "a day's folder that can't be listed was read as none"
            );
        }
    }
}
