//! The last good answer of each owner, kept so a read missed under load
//! leaves what was last read standing, said with when that was and why the
//! newest read failed, rather than blanking a part and every machine's tie
//! to it at once: one slow `mycelium health` once blanked both machines'
//! cells. Kept on disk, so the command and the daemon each stand on what
//! either last read. A part standing on one is judged no better than
//! watch, as its owner didn't answer, so a fault it would have said isn't
//! hidden behind a good reading.

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::parts::Answer;
use crate::record::{Health, System};

/// How long a kept answer may stand for reads that missed: several of the
/// daemon's paces under load, and short enough that an owner gone for good
/// is soon said not read rather than standing on an old answer.
pub const KEPT_FOR: i64 = 10 * 60;

/// Who may read a kept answer and its folder: the account alone, as the
/// daemon's socket is, since the answers name the home network's devices
/// and every checkout.
const ITS_OWN: u32 = 0o600;
const ITS_OWN_FOLDER: u32 = 0o700;

/// The owners' last good answers, a file each in one folder.
pub struct Kept {
    folder: PathBuf,
}

/// An owner's answer as it's used: its own, or its last kept one, with a
/// note saying when that was read and why the newest read wasn't.
#[derive(Debug, PartialEq)]
pub struct Used {
    pub answer: Answer,
    pub note: Option<String>,
}

impl Kept {
    /// Answers kept in `folder`, made when the first is kept.
    pub fn in_folder(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// The answer to use for the owner asked with `said`, read at `now`
    /// (epoch seconds) as `answer`: a good one is kept and used, and a
    /// failed one stands on the last kept within `KEPT_FOR`, said by
    /// `clock` as a time of day.
    pub fn used(
        &self,
        said: &str,
        answer: Answer,
        now: i64,
        clock: impl Fn(i64) -> String,
    ) -> Used {
        let path = self.folder.join(format!("{}.json", said.replace(' ', "-")));
        match answer {
            Ok(value) => {
                // Keeping is a help, not the answer: one not kept leaves the
                // next missed read said as missed, as it was before.
                let _ = keep(&path, &value, now);
                Used {
                    answer: Ok(value),
                    note: None,
                }
            }
            Err(why) => match kept(&path) {
                // One read later than now, as after the clock steps back,
                // isn't known to be recent, so it doesn't stand.
                Some((at, value)) if (0..=KEPT_FOR).contains(&(now - at)) => Used {
                    answer: Ok(value),
                    note: Some(format!("as read at {}; just now {why}", clock(at))),
                },
                _ => Used {
                    answer: Err(why),
                    note: None,
                },
            },
        }
    }
}

/// Writes `value` to `path` as read at `now`, whole or not at all, so the
/// command and the daemon never read one half written; an answer as it
/// was kept is only marked read again, so a large one isn't rewritten
/// every pace.
fn keep(path: &Path, value: &Value, now: i64) -> std::io::Result<()> {
    let text = value.to_string();
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(u64::try_from(now).unwrap_or(0));
    if fs::read_to_string(path).is_ok_and(|was| was == text) {
        return File::options().write(true).open(path)?.set_modified(at);
    }
    if let Some(folder) = path.parent() {
        DirBuilder::new()
            .recursive(true)
            .mode(ITS_OWN_FOLDER)
            .create(folder)?;
    }
    let writing = path.with_extension(format!("{}.part", std::process::id()));
    let written = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(ITS_OWN)
        .open(&writing)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.set_modified(at)
        })
        .and_then(|()| fs::rename(&writing, path));
    if written.is_err() {
        // A half-kept answer is no use, and each run's would pile up; it
        // may never have been made, which leaves nothing to take away.
        let _ = fs::remove_file(&writing);
    }
    written
}

/// The answer kept at `path` and when it was read, by the file's time.
fn kept(path: &Path) -> Option<(i64, Value)> {
    let at = fs::metadata(path).and_then(|file| file.modified()).ok()?;
    let at = i64::try_from(at.duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()?;
    let value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    Some((at, value))
}

/// What was read from a kept answer, by owner: each a note, when it was.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Notes {
    pub machines: Option<String>,
    pub health: Option<String>,
    pub net: Option<String>,
    pub repos: Option<String>,
}

impl Notes {
    /// The note of the owner the part `name` is read from, when it was
    /// read from a kept answer.
    pub fn of(&self, name: &str) -> Option<&str> {
        match name {
            "machines" => self.machines.as_deref(),
            "mycelium" => self.health.as_deref(),
            "network" => self.net.as_deref(),
            "code" => self.repos.as_deref(),
            _ => None,
        }
    }
}

/// Says, on each part and each machine read from a kept answer, when that
/// was read and why the newest read wasn't: a part after what it says, a
/// machine after what it says when the machines' own answer was kept, and
/// a machine's cells, network and code under its ties' `kept`, each only
/// on a tie that answer gave it. A part so read is judged no better than
/// watch.
pub fn mark(system: &mut System, notes: &Notes) {
    let owners = [
        ("cells", &notes.health),
        ("network", &notes.net),
        ("code", &notes.repos),
    ];
    for part in &mut system.parts {
        if let Some(note) = notes.of(part.name) {
            part.says = format!("{}, {note}", part.says);
            part.health = part.health.max(Health::Watch);
        }
        for thing in &mut part.things {
            if part.name == "machines"
                && let Some(note) = &notes.machines
            {
                thing.says = format!("{}, {note}", thing.says);
            }
            let Some(ties) = thing.ties.as_object_mut() else {
                continue;
            };
            let kept: Map<String, Value> = owners
                .iter()
                .filter(|(tie, _)| ties.get(*tie).is_some_and(|read| !read.is_null()))
                .filter_map(|(tie, note)| Some(((*tie).to_owned(), Value::from(note.as_deref()?))))
                .collect();
            if !kept.is_empty() {
                ties.insert("kept".to_owned(), Value::Object(kept));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts;
    use serde_json::json;

    /// A folder of a case's own, taken away when the case ends.
    struct Folder(PathBuf);

    impl Drop for Folder {
        fn drop(&mut self) {
            // A folder never made leaves nothing to take away.
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn folder(name: &str) -> Folder {
        let folder =
            std::env::temp_dir().join(format!("system-kept-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        Folder(folder)
    }

    fn clock(at: i64) -> String {
        format!("t{at}")
    }

    fn late(said: &str) -> Answer {
        Err(format!("{said} gave no answer in 20 s"))
    }

    #[test]
    fn a_missed_read_stands_on_the_last_good_answer_saying_when_and_why() {
        let folder = folder("missed");
        let kept = Kept::in_folder(folder.0.clone());
        let read = kept.used("mycelium health", Ok(json!({"sessions": 3})), 1_000, clock);
        assert_eq!(
            read.answer,
            Ok(json!({"sessions": 3})),
            "a good answer is used as it is"
        );
        assert_eq!(read.note, None, "a good answer needs no note");
        let missed = kept.used("mycelium health", late("mycelium health"), 1_030, clock);
        assert_eq!(
            missed,
            Used {
                answer: Ok(json!({"sessions": 3})),
                note: Some(
                    "as read at t1000; just now mycelium health gave no answer in 20 s".to_owned()
                ),
            },
            "a missed read keeps what was last read, with when and why"
        );
    }

    #[test]
    fn an_unchanged_answer_is_read_again_at_its_newest_time() {
        let folder = folder("again");
        let kept = Kept::in_folder(folder.0.clone());
        kept.used("net --json", Ok(json!({"devices": []})), 1_000, clock);
        // A rewrite would go through this, and can't, so only a time moved
        // on the file as it is leaves the answer read at the newest time.
        let writing = folder
            .0
            .join(format!("net---json.{}.part", std::process::id()));
        fs::create_dir(&writing).expect("a stand-in for the file a rewrite would write");
        kept.used("net --json", Ok(json!({"devices": []})), 1_500, clock);
        let missed = kept.used(
            "net --json",
            Err("net --json ended with 1".to_owned()),
            1_520,
            clock,
        );
        assert_eq!(
            missed.note.as_deref(),
            Some("as read at t1500; just now net --json ended with 1"),
            "an answer read again as it was is said read at the newest time, not written again"
        );
    }

    #[test]
    fn an_answer_kept_too_long_or_never_leaves_the_read_missed() {
        let folder = folder("old");
        let kept = Kept::in_folder(folder.0.clone());
        let why = || late("repos --json");
        assert_eq!(
            kept.used("repos --json", why(), 1_000, clock),
            Used {
                answer: why(),
                note: None
            },
            "with nothing kept a missed read is said missed"
        );
        kept.used(
            "repos --json",
            Ok(json!({"repositories": []})),
            1_000,
            clock,
        );
        assert!(
            kept.used("repos --json", why(), 1_000 + KEPT_FOR, clock)
                .answer
                .is_ok(),
            "an answer kept exactly KEPT_FOR stands"
        );
        assert_eq!(
            kept.used("repos --json", why(), 1_000 + KEPT_FOR + 1, clock)
                .answer,
            why(),
            "an owner gone past KEPT_FOR is said missed, not standing on its old answer"
        );
        assert_eq!(
            kept.used("repos --json", why(), 999, clock).answer,
            why(),
            "an answer read later than now, as after the clock steps back, doesn't stand"
        );
    }

    #[test]
    fn a_keep_that_fails_or_a_kept_file_that_wont_read_leaves_the_read_as_it_was() {
        let folder = folder("fails");
        fs::create_dir_all(&folder.0).expect("a folder for the case");
        let blocked = folder.0.join("a-file-not-a-folder");
        fs::write(&blocked, "").expect("a file where the kept folder would go");
        let kept = Kept::in_folder(blocked);
        assert_eq!(
            kept.used("net --json", Ok(json!({"devices": []})), 1_000, clock),
            Used {
                answer: Ok(json!({"devices": []})),
                note: None
            },
            "an answer that can't be kept is still used"
        );
        let kept = Kept::in_folder(folder.0.clone());
        fs::write(folder.0.join("net---json.json"), "{\"devic").expect("a kept file cut short");
        assert_eq!(
            kept.used("net --json", late("net --json"), 1_000, clock),
            Used {
                answer: late("net --json"),
                note: None
            },
            "a kept file that won't read leaves the read missed"
        );
    }

    #[test]
    fn a_keep_that_fails_leaves_no_half_kept_answer() {
        let folder = folder("half");
        // A folder in the kept file's place, holding something, can't be
        // renamed over, so the keep fails after writing its part.
        fs::create_dir_all(folder.0.join("net---json.json/held")).expect("a folder in the way");
        let kept = Kept::in_folder(folder.0.clone());
        kept.used("net --json", Ok(json!({"devices": []})), 1_000, clock);
        let left: Vec<_> = fs::read_dir(&folder.0)
            .expect("the kept folder")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "part")
            })
            .collect();
        assert!(left.is_empty(), "a failed keep left {left:?}");
    }

    #[test]
    fn a_kept_answer_is_the_accounts_alone() {
        let folder = folder("own");
        let kept = Kept::in_folder(folder.0.join("kept"));
        kept.used("net --json", Ok(json!({"devices": []})), 1_000, clock);
        let mode = |path: PathBuf| {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(path).expect("kept").permissions().mode() & 0o777
        };
        assert_eq!(
            mode(folder.0.join("kept/net---json.json")),
            ITS_OWN,
            "the file is its own"
        );
        assert_eq!(
            mode(folder.0.join("kept")),
            ITS_OWN_FOLDER,
            "the folder is its own"
        );
    }

    fn system(repos_on: &str) -> System {
        let realm = json!({"Ok": {"Machines": [
            {"name": "kestrel", "state": "ready", "disk_band": "healthy", "room": {"over": []}},
            {"name": "osprey", "state": "ready", "disk_band": "healthy", "room": {"over": []}},
        ]}});
        let health = json!({"sessions": {}});
        let repos = json!({"machine": repos_on, "repositories": []});
        System {
            machine: "kestrel".to_owned(),
            read_at: "2026-11-21T12:00:00Z".to_owned(),
            parts: vec![
                parts::machines(&Ok(realm), &Ok(health.clone()), None, Some(&repos), None),
                parts::mycelium(&Ok(health)),
                parts::code(&Ok(repos)),
            ],
        }
    }

    #[test]
    fn a_kept_answer_is_said_on_its_part_and_each_machines_tie() {
        let mut system = system("kestrel");
        let cells = "as read at 11:54; just now mycelium health gave no answer in 20 s";
        let code = "as read at 11:55; just now repos --json gave no answer in 20 s";
        let notes = Notes {
            health: Some(cells.to_owned()),
            repos: Some(code.to_owned()),
            ..Notes::default()
        };
        mark(&mut system, &notes);
        let mycelium = &system.parts[1];
        assert!(
            mycelium.says.ends_with(&format!(", {cells}")),
            "the part read from a kept answer says so: {}",
            mycelium.says
        );
        assert!(
            !system.parts[0].says.contains(cells),
            "the machines part, read from its own answer, says nothing of it"
        );
        let kestrel = &system.parts[0].things[0];
        assert_eq!(
            kestrel.ties["kept"],
            json!({"cells": cells, "code": code}),
            "this machine's cells and code, read from kept answers, say so"
        );
        let osprey = &system.parts[0].things[1];
        assert_eq!(
            osprey.ties["kept"],
            json!({"cells": cells}),
            "another machine's code, never read from this machine's repos, says nothing of it"
        );
    }

    #[test]
    fn a_part_on_a_kept_answer_is_judged_no_better_than_watch() {
        let mut system = system("kestrel");
        assert_eq!(
            system.parts[1].health,
            Health::Good,
            "the mycelium reads good"
        );
        mark(
            &mut system,
            &Notes {
                health: Some("as read at 11:54; just now mycelium health ended with 1".to_owned()),
                ..Notes::default()
            },
        );
        assert_eq!(
            system.parts[1].health,
            Health::Watch,
            "an owner that didn't answer is watched, whatever it last said"
        );
    }

    #[test]
    fn kept_machines_are_said_on_each_machine() {
        let mut system = system("kestrel");
        let note = "as read at 11:54; just now mycelium machines gave no answer in 20 s";
        mark(
            &mut system,
            &Notes {
                machines: Some(note.to_owned()),
                ..Notes::default()
            },
        );
        for thing in &system.parts[0].things {
            assert!(
                thing.says.ends_with(&format!(", {note}")),
                "{} read from the kept machines says so: {}",
                thing.name,
                thing.says
            );
        }
    }
}
