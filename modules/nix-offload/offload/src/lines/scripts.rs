//! The startup scripts root has named for lent builders, each with when
//! it was first named, kept where only `lines` reads so they outlast the
//! switch that changes them.
//!
//! A switch that names a new script restarts `lines`, which then saw every
//! builder standing run another script than the machine's, and refused
//! them all, cutting every run on them. Kept here, the script before is
//! still trusted on a builder made before the change, for as long as such
//! a builder can live, so a change drains as builders end rather than
//! cutting them. The record starts with the script named when `lines`
//! first runs with it, so only a switch that ships this and changes the
//! script at once still cuts.
//!
//! `lines` names the script as it starts, so a script's time is when the
//! switch that named it restarted the service, not when a builder was
//! first vetted after. It reads the record once and keeps it in memory
//! after, since it alone writes it: a later failed read can't cut a
//! builder, and a failed write can't move a script's time on.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use serde_json::{Value, json};

use super::vet::Earlier;

/// How long after a change a builder made before it may run on: a day,
/// the longest a lender's deadline or `longest` lets one live, and an
/// hour's margin for its boot. Only builders made before the change are
/// trusted for it, so a longer drain widens nothing they weren't given.
pub const DRAINS_FOR: i64 = 25 * 3600;

/// A script root named, and when it first named it, in seconds since 1970.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub script: String,
    pub since: i64,
}

/// What follows from `kept`, oldest first, once `script` is named at
/// `now`: what to keep, when it differs from `kept`, and the scripts before
/// it that a builder may still run. Each earlier script is trusted on a
/// builder made before the one after it was named, for [`DRAINS_FOR`]
/// after that; one past it is dropped. A script named again is kept as a
/// new entry beside its old one, so the window of each script before it
/// stays where its change closed it, rather than stretching to the new
/// naming; today's script is never among those before.
pub fn named(kept: &[Named], script: &str, now: i64) -> (Option<Vec<Named>>, Vec<Earlier>) {
    named_at(kept, script, now, now)
}

/// [`named`], with `script`, when new, named as of `since` rather than
/// `now`: when it was first asked after, though the record couldn't be
/// read until now.
fn named_at(
    kept: &[Named],
    script: &str,
    since: i64,
    now: i64,
) -> (Option<Vec<Named>>, Vec<Earlier>) {
    let mut all = kept.to_vec();
    if all.last().is_none_or(|last| last.script != script) {
        all.push(Named {
            script: script.to_owned(),
            since,
        });
    }
    let afters: Vec<i64> = all.iter().skip(1).map(|one| one.since).collect();
    let mut keep = Vec::new();
    let mut earlier = Vec::new();
    for (one, after) in all.iter().zip(afters.iter().map(Some).chain([None])) {
        match after {
            Some(&after) if now >= after + DRAINS_FOR => {}
            Some(_) if one.script == script => keep.push(one.clone()),
            Some(&after) => {
                keep.push(one.clone());
                earlier.push(Earlier {
                    script: one.script.clone(),
                    made_before: after,
                });
            }
            None => keep.push(one.clone()),
        }
    }
    let changed = (keep != kept).then_some(keep);
    (changed, earlier)
}

/// Why the record couldn't be read.
#[derive(Debug, PartialEq, Eq)]
enum Unread {
    /// The file couldn't be opened or read, which may pass.
    Unreadable(String),
    /// It was read, but it isn't a record, and reading it again won't help.
    Spoiled(String),
}

/// The scripts kept in `file`, none when it isn't there yet.
fn read(file: &Path) -> Result<Vec<Named>, Unread> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(Unread::Unreadable(format!("{}: {error}", file.display()))),
    };
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| Unread::Spoiled(format!("{} isn't JSON: {error}", file.display())))?;
    value
        .as_array()
        .ok_or(Unread::Spoiled(format!("{} holds no list", file.display())))?
        .iter()
        .map(|one| {
            Some(Named {
                script: one["script"].as_str()?.to_owned(),
                since: one["since"].as_i64()?,
            })
        })
        .collect::<Option<_>>()
        .ok_or(Unread::Spoiled(format!(
            "{} holds an entry without its script or time",
            file.display()
        )))
}

/// Writes `named` to `file` whole, through a draft beside it, so a reader
/// never sees half of it.
fn write(file: &Path, named: &[Named]) -> Result<(), String> {
    let list: Vec<Value> = named
        .iter()
        .map(|one| json!({"script": one.script, "since": one.since}))
        .collect();
    let draft = file.with_extension("draft");
    // The draft reaches the disk before it takes the file's name, so a
    // power loss just after a switch leaves the old record or the new,
    // never an empty one that would cut every builder at the next change.
    fs::File::create(&draft)
        .and_then(|mut made| {
            made.write_all(Value::Array(list).to_string().as_bytes())?;
            made.sync_all()
        })
        .and_then(|()| fs::rename(&draft, file))
        .map_err(|error| format!("{}: {error}", file.display()))
}

/// The scripts named, as `lines` knows them: read from `file` once, then
/// kept here, and written back whenever they change.
#[derive(Debug, Default)]
pub struct Record {
    /// What was last known, none until the file has been read.
    named: Option<Vec<Named>>,
    /// Whether what's known has yet to reach the file.
    unwritten: bool,
    /// What last went wrong with the file, none once it's kept again.
    pub trouble: Option<String>,
    /// The script first asked after, and when, so one asked while the file
    /// can't be read is named as of then once it can, not as of the first
    /// read that works.
    asked: Option<(String, i64)>,
}

impl Record {
    /// The scripts before `script` that a builder may still run at `now`,
    /// naming `script` once it's new. A file that isn't a record begins
    /// again from `script`, so a spoiled one heals at once rather than
    /// cutting every builder at each later change; one that can't be read
    /// trusts only `script` until it can. Either is said in `trouble`.
    pub fn earlier(&mut self, file: &Path, script: &str, now: i64) -> Vec<Earlier> {
        let since = match &self.asked {
            Some((asked, at)) if asked == script => *at,
            _ => {
                self.asked = Some((script.to_owned(), now));
                now
            }
        };
        let mut trouble = None;
        let kept = match self.named.take() {
            Some(kept) => kept,
            None => match read(file) {
                Ok(kept) => kept,
                Err(Unread::Spoiled(why)) => {
                    trouble = Some(format!("{why}, so it begins again from today's script"));
                    self.unwritten = true;
                    Vec::new()
                }
                Err(Unread::Unreadable(why)) => {
                    self.trouble = Some(format!("trusting only today's startup script: {why}"));
                    return Vec::new();
                }
            },
        };
        let (changed, earlier) = named_at(&kept, script, since, now);
        self.unwritten |= changed.is_some();
        let known = changed.unwrap_or(kept);
        if self.unwritten {
            match write(file, &known) {
                Ok(()) => self.unwritten = false,
                Err(why) => {
                    trouble = Some(format!(
                        "the startup scripts named are kept only until this service restarts: \
                         {why}"
                    ));
                }
            }
        }
        self.named = Some(known);
        self.trouble = trouble;
        earlier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(script: &str, since: i64) -> Named {
        Named {
            script: script.to_owned(),
            since,
        }
    }

    #[test]
    fn the_first_script_is_kept_with_nothing_before_it() {
        assert_eq!(
            named(&[], "a", 100),
            (Some(vec![one("a", 100)]), Vec::new()),
            "the first script wasn't kept alone"
        );
        assert_eq!(
            named(&[one("a", 100)], "a", 500),
            (None, Vec::new()),
            "naming the same script again changed the record"
        );
    }

    #[test]
    fn a_changed_script_trusts_the_one_before_on_builders_made_before() {
        let (changed, earlier) = named(&[one("a", 100)], "b", 500);
        assert_eq!(
            changed,
            Some(vec![one("a", 100), one("b", 500)]),
            "the new script wasn't kept after the old"
        );
        assert_eq!(
            earlier,
            vec![Earlier {
                script: "a".to_owned(),
                made_before: 500
            }],
            "the old script wasn't trusted on builders made before the change"
        );
        let kept = changed.unwrap_or_default();
        assert_eq!(
            named(&kept, "b", 500 + DRAINS_FOR - 1).1.len(),
            1,
            "the old script went before its builders could have ended"
        );
        assert_eq!(
            named(&kept, "b", 500 + DRAINS_FOR),
            (Some(vec![one("b", 500)]), Vec::new()),
            "the old script outlived every builder that could run it"
        );
    }

    #[test]
    fn a_script_named_again_is_named_anew() {
        let (changed, earlier) = named(&[one("a", 100), one("b", 200)], "a", 300);
        assert_eq!(
            changed,
            Some(vec![one("a", 100), one("b", 200), one("a", 300)]),
            "a script named again wasn't kept anew beside its first naming"
        );
        assert_eq!(
            earlier,
            vec![Earlier {
                script: "b".to_owned(),
                made_before: 300
            }],
            "the script between wasn't the one trusted before"
        );
    }

    #[test]
    fn a_script_named_again_stretches_no_window_before_it() {
        let kept = [one("x", 100), one("a", 200), one("b", 300)];
        let (_, earlier) = named(&kept, "a", 400);
        assert_eq!(
            earlier,
            vec![
                Earlier {
                    script: "x".to_owned(),
                    made_before: 200
                },
                Earlier {
                    script: "b".to_owned(),
                    made_before: 400
                },
            ],
            "naming a again moved x's window past the change that closed it"
        );
        assert_eq!(
            named(&kept, "a", 200 + DRAINS_FOR).1,
            vec![Earlier {
                script: "b".to_owned(),
                made_before: 200 + DRAINS_FOR
            }],
            "x outlived its drain once a was named again"
        );
    }

    #[test]
    fn each_earlier_script_is_trusted_up_to_the_one_after_it() {
        let kept = [one("a", 100), one("b", 200), one("c", 300)];
        assert_eq!(
            named(&kept, "c", 400).1,
            vec![
                Earlier {
                    script: "a".to_owned(),
                    made_before: 200
                },
                Earlier {
                    script: "b".to_owned(),
                    made_before: 300
                },
            ],
            "an earlier script was trusted past the change that followed it"
        );
        assert_eq!(
            named(&kept, "c", 200 + DRAINS_FOR).1,
            vec![Earlier {
                script: "b".to_owned(),
                made_before: 300
            }],
            "the oldest script outlived its drain, or took the next one's with it"
        );
    }

    fn a_and_then_b() -> Vec<Earlier> {
        vec![Earlier {
            script: "a".to_owned(),
            made_before: 500,
        }]
    }

    #[test]
    fn the_record_outlasts_a_restart() {
        let folder = tempfile::tempdir().expect("no folder for the record");
        let file = folder.path().join("scripts.json");
        let mut record = Record::default();
        assert_eq!(
            record.earlier(&file, "a", 100),
            Vec::new(),
            "the first script read as having one before it"
        );
        assert_eq!(
            Record::default().earlier(&file, "b", 500),
            a_and_then_b(),
            "the script before the change wasn't read back"
        );
        assert_eq!(
            read(&file),
            Ok(vec![one("a", 100), one("b", 500)]),
            "the record didn't hold both scripts"
        );
    }

    #[test]
    fn a_spoiled_record_begins_again_from_todays_script() {
        let folder = tempfile::tempdir().expect("no folder for the record");
        let file = folder.path().join("scripts.json");
        fs::write(&file, "[{\"script\": 1}]").expect("couldn't spoil the record");
        let mut record = Record::default();
        assert_eq!(
            record.earlier(&file, "a", 100),
            Vec::new(),
            "a spoiled record trusted a script before today's"
        );
        assert!(
            record
                .trouble
                .as_deref()
                .is_some_and(|why| why.contains("begins again")),
            "a spoiled record went unsaid: {:?}",
            record.trouble
        );
        assert_eq!(
            read(&file),
            Ok(vec![one("a", 100)]),
            "the spoiled record wasn't begun again"
        );
        assert_eq!(
            Record::default().earlier(&file, "b", 500),
            a_and_then_b(),
            "the change after a healed record cut its builders"
        );
    }

    #[test]
    fn an_unreadable_record_trusts_todays_script_until_it_reads() {
        let folder = tempfile::tempdir().expect("no folder for the record");
        // A folder where the file should be can't be read as one.
        let file = folder.path().join("scripts.json");
        fs::create_dir(&file).expect("couldn't stand a folder in the record's place");
        let mut record = Record::default();
        assert_eq!(
            record.earlier(&file, "b", 500),
            Vec::new(),
            "an unreadable record trusted a script before today's"
        );
        assert!(record.trouble.is_some(), "an unreadable record went unsaid");
        fs::remove_dir(&file).expect("couldn't clear the folder");
        fs::write(&file, "[{\"script\":\"a\",\"since\":100}]").expect("the record");
        assert_eq!(
            record.earlier(&file, "b", 900),
            a_and_then_b(),
            "the record wasn't read again once it could be, or b was named as of \
             the first read that worked rather than when it was first asked after"
        );
        assert_eq!(
            record.trouble, None,
            "a record read again was still said to fail"
        );
        assert_eq!(
            read(&file),
            Ok(vec![one("a", 100), one("b", 500)]),
            "b wasn't kept as of when it was first asked after"
        );
    }

    #[test]
    fn a_failed_write_keeps_the_change_in_memory_and_tries_again() {
        let folder = tempfile::tempdir().expect("no folder for the record");
        let file = folder.path().join("scripts.json");
        let mut record = Record::default();
        record.earlier(&file, "a", 100);
        // The draft beside the file can't be written while a folder holds
        // its name.
        let draft = file.with_extension("draft");
        fs::create_dir(&draft).expect("couldn't block the draft");
        assert_eq!(
            record.earlier(&file, "b", 500),
            a_and_then_b(),
            "a failed write lost the script before the change"
        );
        assert!(record.trouble.is_some(), "a failed write went unsaid");
        assert_eq!(
            record.earlier(&file, "b", 900),
            a_and_then_b(),
            "a failed write moved the change's time on"
        );
        fs::remove_dir(&draft).expect("couldn't clear the draft");
        record.earlier(&file, "b", 950);
        assert_eq!(
            read(&file),
            Ok(vec![one("a", 100), one("b", 500)]),
            "the change wasn't written once the file could take it"
        );
    }
}
