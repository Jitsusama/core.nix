//! The last good answer of each owner, kept so a read missed under load
//! leaves what was last read standing, said with when that was and why the
//! newest read failed, rather than blanking a part and every machine's tie
//! to it at once: one slow `mycelium health` once blanked both machines'
//! cells. Kept on disk, so the command and the daemon each stand on what
//! either last read.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::parts::Answer;
use crate::record::System;

/// How long a kept answer may stand for reads that missed: several of the
/// daemon's paces under load, and short enough that an owner gone for good
/// is soon said not read rather than standing on an old answer.
pub const KEPT_FOR: i64 = 10 * 60;

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
                Some((at, value)) if (0..=KEPT_FOR).contains(&(now - at)) => Used {
                    answer: Ok(value),
                    note: Some(format!("as read at {}; {why} just now", clock(at))),
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
        fs::create_dir_all(folder)?;
    }
    let writing = path.with_extension(format!("{}.part", std::process::id()));
    fs::write(&writing, &text)?;
    File::options()
        .write(true)
        .open(&writing)?
        .set_modified(at)?;
    fs::rename(&writing, path)
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

/// Says, on each part and each machine's tie read from a kept answer, when
/// that was read and why the newest read wasn't: a part after what it
/// says, and a machine's cells, network and code under its ties' `kept`.
pub fn mark(system: &mut System, notes: &Notes) {
    let tied = [
        ("cells", &notes.health),
        ("network", &notes.net),
        ("code", &notes.repos),
    ];
    let kept: Map<String, Value> = tied
        .into_iter()
        .filter_map(|(tie, note)| Some((tie.to_owned(), Value::from(note.as_deref()?))))
        .collect();
    for part in &mut system.parts {
        let note = match part.name {
            "machines" => &notes.machines,
            "mycelium" => &notes.health,
            "network" => &notes.net,
            "code" => &notes.repos,
            _ => &None,
        };
        if let Some(note) = note {
            part.says = format!("{}, {note}", part.says);
        }
        if kept.is_empty() {
            continue;
        }
        for thing in &mut part.things {
            if let Some(ties) = thing.ties.as_object_mut() {
                ties.insert("kept".to_owned(), Value::Object(kept.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts;
    use serde_json::json;

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("system-kept-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    fn clock(at: i64) -> String {
        format!("t{at}")
    }

    #[test]
    fn a_missed_read_stands_on_the_last_good_answer_saying_when_and_why() {
        let kept = Kept::in_folder(folder("missed"));
        let read = kept.used("mycelium health", Ok(json!({"sessions": 3})), 1_000, clock);
        assert_eq!(
            read.answer,
            Ok(json!({"sessions": 3})),
            "a good answer is used as it is"
        );
        assert_eq!(read.note, None, "a good answer needs no note");
        let missed = kept.used(
            "mycelium health",
            Err("mycelium health gave no answer in 10 s".to_owned()),
            1_030,
            clock,
        );
        assert_eq!(
            missed,
            Used {
                answer: Ok(json!({"sessions": 3})),
                note: Some(
                    "as read at t1000; mycelium health gave no answer in 10 s just now".to_owned()
                ),
            },
            "a missed read keeps what was last read, with when and why"
        );
    }

    #[test]
    fn an_unchanged_answer_is_read_again_at_its_newest_time() {
        let kept = Kept::in_folder(folder("again"));
        kept.used("net --json", Ok(json!({"devices": []})), 1_000, clock);
        kept.used("net --json", Ok(json!({"devices": []})), 1_500, clock);
        let missed = kept.used(
            "net --json",
            Err("net --json ended with 1".to_owned()),
            1_520,
            clock,
        );
        assert_eq!(
            missed.note.as_deref(),
            Some("as read at t1500; net --json ended with 1 just now"),
            "an answer read again as it was is said read at the newest time"
        );
    }

    #[test]
    fn an_answer_kept_too_long_or_never_leaves_the_read_missed() {
        let kept = Kept::in_folder(folder("old"));
        let why = || Err("repos --json gave no answer in 10 s".to_owned());
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
        assert_eq!(
            kept.used("repos --json", why(), 1_000 + KEPT_FOR + 1, clock)
                .answer,
            why(),
            "an owner gone past KEPT_FOR is said missed, not standing on its old answer"
        );
    }

    #[test]
    fn a_kept_answer_is_said_on_its_part_and_each_machines_tie() {
        let realm = json!({"Ok": {"Machines": [
            {"name": "kestrel", "state": "ready", "disk_band": "healthy", "room": {"over": []}},
        ]}});
        let health = json!({"sessions": {}});
        let mut system = System {
            machine: "kestrel".to_owned(),
            read_at: "2026-11-21T12:00:00Z".to_owned(),
            parts: vec![
                parts::machines(&Ok(realm), &Ok(health.clone()), None, None, None),
                parts::mycelium(&Ok(health)),
            ],
        };
        let note = "as read at 11:54; mycelium health gave no answer in 10 s just now";
        let notes = Notes {
            health: Some(note.to_owned()),
            ..Notes::default()
        };
        mark(&mut system, &notes);
        let mycelium = &system.parts[1];
        assert!(
            mycelium.says.ends_with(&format!(", {note}")),
            "the part read from a kept answer says so: {}",
            mycelium.says
        );
        assert!(
            !system.parts[0].says.contains(note),
            "the machines part, read from its own answer, says nothing of it"
        );
        let ties = &system.parts[0].things[0].ties;
        assert_eq!(
            ties["kept"],
            json!({"cells": note}),
            "each machine's cells, read from the kept health, say so"
        );
    }
}
