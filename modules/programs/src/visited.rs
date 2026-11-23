//! When zoxide last saw each folder, read from its own database as
//! `visited:<database>` names it, for kitty's page in the launcher
//! (modules/kitty/Kitty.qml), and for Code's order (`code_rows.rs`), since
//! zoxide hears only the folders Joel's own shell goes into: each folder
//! still there, the one visited
//! last first, with when, in milliseconds. zoxide's `query` prints only a
//! score, so its database is read here, as bincode lays it out: a
//! version, read only at 3, zoxide's since 0.8, then how many folders,
//! each its path's length and bytes, its score, eight bytes, and when it
//! was last visited, in seconds since 1970; each number little-endian, a
//! count in eight bytes. A database of another version says none, and
//! one cut short says the folders before the cut. It's read as an ask
//! names it, and again as zoxide writes it while its key is watched, so
//! nothing is read at rest.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

use serde_json::{Value, json};

/// The one version of zoxide's database this reads.
const VERSION: u64 = 3;

/// What was last said of a database that couldn't be read, so each
/// trouble is told once in the log, not on every pass that meets it.
static TOLD: Mutex<Option<String>> = Mutex::new(None);

/// The record a key names, or none for a key of another kind or a path
/// that isn't whole.
#[must_use]
pub fn record(key: &str) -> Option<Value> {
    let database = key
        .strip_prefix("visited:")
        .filter(|path| path.starts_with('/'))?;
    let folders: Vec<Value> = visits_in(Path::new(database))
        .into_iter()
        .filter(|(path, _)| Path::new(path).is_dir())
        .map(|(path, at)| json!({"path": path, "at": at}))
        .collect();
    Some(json!({
        "key": key, "kind": "visited", "database": database, "folders": folders,
    }))
}

/// Each folder in the database at `database` and when Joel was last in
/// it, in milliseconds, the one visited last first; none where it can't
/// be read, which is told in the log, as one of another version is,
/// since either leaves nothing ordered by Joel's visits. One missing is
/// told nothing: zoxide writes none until a folder is first visited.
#[must_use]
pub fn visits_in(database: &Path) -> Vec<(String, u64)> {
    let shown = database.display();
    match fs::read(database) {
        Ok(bytes) => {
            let version = Bytes {
                bytes: &bytes,
                at: 0,
            }
            .number(4);
            let other = version
                .filter(|version| *version != VERSION)
                .map(|version| {
                    format!("zoxide's database at {shown} is version {version}, not {VERSION}")
                });
            tell(&TOLD, other);
            visits(&bytes)
        }
        Err(why) if why.kind() == ErrorKind::NotFound => {
            tell(&TOLD, None);
            Vec::new()
        }
        Err(why) => {
            let why = format!("zoxide's database at {shown} couldn't be read: {why}");
            tell(&TOLD, Some(why));
            Vec::new()
        }
    }
}

/// Says `trouble` in the log once it differs from the last `told`, so a
/// database left unreadable is told once, and again only once it changes;
/// true when it said it.
fn tell(told: &Mutex<Option<String>>, trouble: Option<String>) -> bool {
    let mut told = told.lock().unwrap_or_else(PoisonError::into_inner);
    if *told == trouble {
        return false;
    }
    let said = trouble
        .as_ref()
        .inspect(|why| eprintln!("{why}, so no visits order anything"));
    let said = said.is_some();
    *told = trouble;
    said
}

/// `visits` with each folder's links resolved, since zoxide keeps the path
/// the shell named, links and all, and repos names a checkout by its real
/// one: a folder that can't be resolved, as one gone, is kept as written.
/// One `placed` says is already within a checkout or tree as written is
/// kept as it is, so only the rest touch the disk. A folder on a mount
/// that no longer answers stalls this, so a caller that mustn't stall runs
/// it `within` a deadline.
#[must_use]
pub fn resolved(visits: Vec<(String, u64)>, placed: impl Fn(&str) -> bool) -> Vec<(String, u64)> {
    visits
        .into_iter()
        .map(|(folder, at)| {
            if placed(&folder) {
                return (folder, at);
            }
            match fs::canonicalize(&folder) {
                Ok(real) => (real.to_string_lossy().into_owned(), at),
                Err(_) => (folder, at),
            }
        })
        .collect()
}

/// What `work` answers, run on a thread of its own, or `fallback` when it
/// hasn't answered within `deadline`, or when one run before it still
/// hasn't, as `late` counts them. So a folder on a mount that no longer
/// answers holds one thread, never its caller, and no further thread
/// starts while it does.
pub fn within<T: Send + 'static>(
    deadline: Duration,
    late: &Arc<AtomicUsize>,
    fallback: T,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    if late.load(Ordering::SeqCst) > 0 {
        return fallback;
    }
    let (sent, answer) = mpsc::channel();
    // Whichever of the work and its caller comes second, after the work
    // has ended or the caller has stopped waiting, sees it set, so `late`
    // is raised once for a run past its deadline and lowered once as it ends.
    let ended = Arc::new(AtomicBool::new(false));
    let (late_here, ended_here) = (Arc::clone(late), Arc::clone(&ended));
    std::thread::spawn(move || {
        let answered = work();
        if ended_here.swap(true, Ordering::SeqCst) {
            late_here.fetch_sub(1, Ordering::SeqCst);
        }
        // A caller gone past its deadline no longer hears it.
        let _ = sent.send(answered);
    });
    if let Ok(answered) = answer.recv_timeout(deadline) {
        return answered;
    }
    late.fetch_add(1, Ordering::SeqCst);
    if ended.swap(true, Ordering::SeqCst) {
        late.fetch_sub(1, Ordering::SeqCst);
    }
    fallback
}

/// A database's bytes, read from the front.
struct Bytes<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, size: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(size)?;
        let taken = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(taken)
    }

    /// A little-endian number of `size` bytes.
    fn number(&mut self, size: usize) -> Option<u64> {
        let taken = self.take(size)?;
        Some(
            taken
                .iter()
                .rev()
                .fold(0, |number, byte| (number << 8) | u64::from(*byte)),
        )
    }
}

/// Each folder and when it was last visited, in milliseconds, the one
/// visited last first.
fn visits(bytes: &[u8]) -> Vec<(String, u64)> {
    let mut read = Bytes { bytes, at: 0 };
    if read.number(4) != Some(VERSION) {
        return Vec::new();
    }
    let Some(count) = read.number(8) else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    for _ in 0..count {
        let Some(one) = folder(&mut read) else {
            break;
        };
        seen.push(one);
    }
    seen.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    seen
}

fn folder(read: &mut Bytes) -> Option<(String, u64)> {
    let length = usize::try_from(read.number(8)?).ok()?;
    let path = String::from_utf8_lossy(read.take(length)?).into_owned();
    // Its score, whose frecency has no unit Joel could read.
    read.take(8)?;
    let seconds = read.number(8)?;
    Some((path, seconds.saturating_mul(1000)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn little(value: u64, size: usize) -> Vec<u8> {
        value.to_le_bytes()[..size].to_vec()
    }

    fn database(version: u64, folders: &[(&str, u64)]) -> Vec<u8> {
        let mut bytes = little(version, 4);
        bytes.extend(little(u64::try_from(folders.len()).unwrap(), 8));
        for (path, seconds) in folders {
            bytes.extend(little(u64::try_from(path.len()).unwrap(), 8));
            bytes.extend(path.as_bytes());
            bytes.extend(1.0_f64.to_le_bytes());
            bytes.extend(little(*seconds, 8));
        }
        bytes
    }

    #[test]
    fn each_folder_says_when_it_was_seen_the_last_seen_first() {
        let bytes = database(3, &[("/s/a", 1_700_000_000), ("/s/\u{e9}", 1_700_003_600)]);
        assert_eq!(
            visits(&bytes),
            [
                ("/s/\u{e9}".to_owned(), 1_700_003_600_000),
                ("/s/a".to_owned(), 1_700_000_000_000),
            ],
            "the folders weren't read as zoxide wrote them"
        );
    }

    #[test]
    fn another_version_says_none_and_one_cut_short_what_came_before() {
        let bytes = database(3, &[("/s/a", 1), ("/s/b", 2)]);
        assert!(
            visits(&database(4, &[("/s/a", 1)])).is_empty(),
            "another version's database was read"
        );
        assert_eq!(
            visits(&bytes[..bytes.len() - 3]),
            [("/s/a".to_owned(), 1000)],
            "a database cut short didn't say what came before the cut"
        );
        assert!(visits(&[]).is_empty(), "no database said a folder");
    }

    #[test]
    fn a_database_on_the_disk_is_read_and_one_missing_says_none() {
        let folder = std::env::temp_dir().join(format!("visited-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let at = folder.join("db.zo");
        fs::write(&at, database(3, &[("/s/a", 2)])).unwrap();
        let read = visits_in(&at);
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            read,
            [("/s/a".to_owned(), 2000)],
            "the database on the disk wasn't read"
        );
        assert!(
            visits_in(Path::new("/nowhere/db.zo")).is_empty(),
            "a missing database said a folder"
        );
    }

    #[test]
    fn a_trouble_is_told_once_and_again_only_once_it_changes() {
        let told = Mutex::new(None);
        let unread = || Some("unread".to_owned());
        let said = [
            tell(&told, unread()),
            tell(&told, unread()),
            tell(&told, Some("version 4".to_owned())),
            tell(&told, None),
            tell(&told, None),
            tell(&told, unread()),
        ];
        assert_eq!(
            said,
            [true, false, true, false, false, true],
            "a trouble wasn't told once per change"
        );
    }

    #[test]
    fn a_folder_reached_through_a_link_is_named_by_its_real_path() {
        let folder = std::env::temp_dir().join(format!("resolved-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("real/beta")).unwrap();
        std::os::unix::fs::symlink(folder.join("real"), folder.join("linked")).unwrap();
        let through = folder.join("linked/beta").to_string_lossy().into_owned();
        let real = fs::canonicalize(folder.join("real/beta")).unwrap();
        let placed = format!("{}/linked", folder.to_string_lossy());
        let named = resolved(
            vec![(through.clone(), 2), ("/nowhere/gone".to_owned(), 1)],
            |_| false,
        );
        let kept = resolved(vec![(through.clone(), 2)], |seen| seen.starts_with(&placed));
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            named,
            [
                (real.to_string_lossy().into_owned(), 2),
                ("/nowhere/gone".to_owned(), 1),
            ],
            "a folder through a link wasn't named by its real path, or one gone was lost"
        );
        assert_eq!(
            kept,
            [(through, 2)],
            "a folder already within a place as written was resolved all the same"
        );
    }

    #[test]
    fn work_past_its_deadline_holds_its_own_thread_and_starts_no_other() {
        let late = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            within(Duration::from_secs(30), &late, 0, || 7),
            7,
            "work that answered in time wasn't taken"
        );
        let (release, held) = mpsc::channel::<()>();
        let stalled = within(Duration::from_millis(50), &late, 0, move || {
            let _ = held.recv();
            7
        });
        assert_eq!(stalled, 0, "work past its deadline held its caller");
        assert_eq!(
            within(Duration::from_secs(30), &late, 0, || 9),
            0,
            "work started while an earlier run was still out"
        );
        release.send(()).unwrap();
        for _ in 0..3000 {
            if late.load(Ordering::SeqCst) == 0 {
                break;
            }
            // The released thread lowers `late` on its own time; a
            // millisecond a look keeps this wait from spinning a core.
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            within(Duration::from_secs(30), &late, 0, || 9),
            9,
            "work once the late run ended wasn't started again"
        );
    }

    #[test]
    fn only_a_whole_path_is_a_database() {
        assert!(
            record("visited:db.zo").is_none(),
            "a part of a path was read"
        );
        assert!(record("folder:/db.zo").is_none(), "another kind was read");
        assert_eq!(
            record("visited:/nowhere/db.zo").map(|record| record["folders"].clone()),
            Some(json!([])),
            "a database that isn't there didn't say no folders"
        );
    }
}
