//! What a step did that observing can't show afterwards: a recovery key
//! proven by being typed back, state restored, a card read. Everything else
//! is observed, so the journal holds as little as it can. Each fact is a
//! line of JSON, written through to the disk before the step goes on, so a
//! reboot or a crash keeps it.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::Fault;

/// A fact as the journal holds it: the step that recorded it, then what.
pub type Fact = (String, String);

#[derive(Debug, Default)]
pub struct Journal {
    path: Option<PathBuf>,
    facts: BTreeSet<Fact>,
    /// A second file every fact goes to as well, where a later start can
    /// read what this one did.
    copy: Option<PathBuf>,
}

/// The facts a journal's file holds, named `what` when it's refused. A
/// last line cut off before its newline, as by power lost while it was
/// written, is one whose fact never counted, so it's passed over.
pub fn facts_in(text: &str, what: &str) -> Result<BTreeSet<Fact>, Fault> {
    let whole = text.rfind('\n').map_or("", |end| &text[..=end]);
    let mut facts = BTreeSet::new();
    for (number, line) in whole.lines().enumerate() {
        let line: Line = serde_json::from_str(line)
            .map_err(|why| Fault::unreadable(format!("{what} line {}", number + 1), why))?;
        facts.insert((line.step, line.fact));
    }
    Ok(facts)
}

/// A fact added to the end of a journal's file, on the disk before this
/// returns.
fn append(path: &Path, step: &str, fact: &str) -> Result<(), Fault> {
    let unkept = |why: std::io::Error| Fault::Unkept {
        path: path.display().to_string(),
        why: why.to_string(),
    };
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let mut line = serde_json::to_string(&Line {
        step: step.into(),
        fact: fact.into(),
        at,
    })
    .map_err(|why| Fault::unreadable("a journal line", why))?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .map_err(unkept)?;
    file.write_all(line.as_bytes()).map_err(unkept)?;
    file.sync_all().map_err(unkept)
}

/// What a folder holds, on the disk before this returns: the folder the
/// copy is in and the one above, which holds that folder's own entry. An
/// fsync of a file keeps its data but, on a FAT boot partition, not a new
/// folder's name, so a restart in the seconds after could lose the copy.
fn sync_folders_of(path: &Path) -> Result<(), Fault> {
    let unkept = |why: std::io::Error| Fault::Unkept {
        path: path.display().to_string(),
        why: why.to_string(),
    };
    let mut folder = path.parent();
    for _ in 0..2 {
        let Some(here) = folder else { break };
        fs::File::open(here)
            .and_then(|opened| opened.sync_all())
            .map_err(unkept)?;
        folder = here.parent();
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Line {
    step: String,
    fact: String,
    /// Seconds since 1970, for a person reading the file.
    at: u64,
}

impl Journal {
    /// A journal kept only while the guide runs, for tests.
    pub fn in_memory() -> Journal {
        Journal::default()
    }

    /// The journal at this path, with what it already holds. Its directory
    /// is made, for its owner alone, when there's none.
    pub fn open(path: &Path) -> Result<Journal, Fault> {
        let unkept = |why: std::io::Error| Fault::Unkept {
            path: path.display().to_string(),
            why: why.to_string(),
        };
        let mut facts = BTreeSet::new();
        match fs::read_to_string(path) {
            Ok(text) => facts = facts_in(&text, &path.display().to_string())?,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                if let Some(directory) = path.parent() {
                    fs::DirBuilder::new()
                        .recursive(true)
                        .mode(0o700)
                        .create(directory)
                        .map_err(unkept)?;
                }
            }
            Err(why) => return Err(unkept(why)),
        }
        Ok(Journal {
            path: Some(path.to_path_buf()),
            facts,
            copy: None,
        })
    }

    /// Keeps a copy at `path` too, of every fact known and every one
    /// recorded from now on, in place of any copy it held there. A journal
    /// kept only in memory, as a test's, only says where it would be.
    pub fn keep_a_copy(&mut self, path: &Path) -> Result<(), Fault> {
        if self.path.is_some() {
            let unkept = |why: std::io::Error| Fault::Unkept {
                path: path.display().to_string(),
                why: why.to_string(),
            };
            if let Some(directory) = path.parent() {
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(directory)
                    .map_err(unkept)?;
            }
            match fs::remove_file(path) {
                Err(why) if why.kind() != std::io::ErrorKind::NotFound => return Err(unkept(why)),
                _ => {}
            }
            for (step, fact) in &self.facts {
                append(path, step, fact)?;
            }
            sync_folders_of(path)?;
        }
        self.copy = Some(path.to_path_buf());
        Ok(())
    }

    /// Where the copy is kept, if one is.
    pub fn copy(&self) -> Option<&Path> {
        self.copy.as_deref()
    }

    /// The copy taken away, once nothing later needs what it holds.
    pub fn drop_copy(&mut self) -> Result<(), Fault> {
        if let Some(path) = self.copy.take()
            && self.path.is_some()
        {
            match fs::remove_file(&path) {
                // Gone from the disk too, so a restart can't find a copy of
                // an install that has since finished.
                Ok(()) => sync_folders_of(&path)?,
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
                Err(why) => {
                    return Err(Fault::Unkept {
                        path: path.display().to_string(),
                        why: why.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// What an earlier start recorded, taken as known, but for the facts
    /// of `except`'s steps, which were true only of that start.
    pub fn learn(&mut self, facts: &BTreeSet<Fact>, except: &[&str]) -> Result<(), Fault> {
        for (step, fact) in facts {
            if !except.contains(&step.as_str()) {
                self.record(step, fact)?;
            }
        }
        Ok(())
    }

    /// Whether a step recorded this fact.
    pub fn knows(&self, step: &str, fact: &str) -> bool {
        self.facts.contains(&(step.to_string(), fact.to_string()))
    }

    /// Records a fact, on the disk before this returns.
    pub fn record(&mut self, step: &str, fact: &str) -> Result<(), Fault> {
        if self.knows(step, fact) {
            return Ok(());
        }
        if let Some(path) = &self.path {
            append(path, step, fact)?;
            if let Some(copy) = &self.copy {
                append(copy, step, fact)?;
            }
        }
        self.facts.insert((step.into(), fact.into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fact_outlives_the_guide() {
        let directory = crate::Scratch::new("journal-outlives");
        let path = directory.join("state").join("journal");
        let mut journal = Journal::open(&path).unwrap();
        assert!(!journal.knows("recovery-key", "typed back"));
        journal.record("recovery-key", "typed back").unwrap();

        let again = Journal::open(&path).unwrap();
        assert!(again.knows("recovery-key", "typed back"));
        assert!(!again.knows("recovery-key", "printed"));
    }

    #[test]
    fn a_fact_is_written_once() {
        let directory = crate::Scratch::new("journal-once");
        let path = directory.join("journal");
        let mut journal = Journal::open(&path).unwrap();
        journal.record("restore", "done").unwrap();
        journal.record("restore", "done").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 1);
    }

    #[test]
    fn the_journal_is_its_owners_alone() {
        use std::os::unix::fs::PermissionsExt;
        let directory = crate::Scratch::new("journal-owner");
        let path = directory.join("state").join("journal");
        Journal::open(&path).unwrap().record("a", "b").unwrap();
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&directory.join("state")), 0o700);
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn a_copy_holds_what_was_known_and_what_came_after() {
        let directory = crate::Scratch::new("journal-copy");
        let mut journal = Journal::open(&directory.join("journal")).unwrap();
        journal.record("confirm-disk", "/dev/nvme0n1").unwrap();
        let copy = directory.join("boot").join("guide").join("install-journal");
        journal.keep_a_copy(&copy).unwrap();
        journal.record("enrol-nano", "opened:abc").unwrap();

        let kept = facts_in(&fs::read_to_string(&copy).unwrap(), "the copy").unwrap();
        assert_eq!(
            kept,
            BTreeSet::from([
                ("confirm-disk".into(), "/dev/nvme0n1".into()),
                ("enrol-nano".into(), "opened:abc".into()),
            ])
        );
        journal.drop_copy().unwrap();
        assert!(!copy.exists());
        journal.record("enrol-5ci", "opened:def").unwrap();
        assert!(!copy.exists(), "a dropped copy takes no more facts");
    }

    #[test]
    fn a_copy_kept_again_starts_over() {
        let directory = crate::Scratch::new("journal-copy-again");
        fs::create_dir_all(&directory).unwrap();
        let copy = directory.join("install-journal");
        fs::write(&copy, "{\"step\":\"old\",\"fact\":\"start\",\"at\":1}\n").unwrap();
        let mut journal = Journal::open(&directory.join("journal")).unwrap();
        journal.record("a", "b").unwrap();
        journal.keep_a_copy(&copy).unwrap();
        let kept = facts_in(&fs::read_to_string(&copy).unwrap(), "the copy").unwrap();
        assert_eq!(kept, BTreeSet::from([("a".into(), "b".into())]));
    }

    #[test]
    fn a_line_cut_off_before_its_end_never_counted() {
        let text = "{\"step\":\"a\",\"fact\":\"b\",\"at\":1}\n{\"step\":\"c\",\"fa";
        let facts = facts_in(text, "the copy").unwrap();
        assert_eq!(facts, BTreeSet::from([("a".into(), "b".into())]));
    }

    #[test]
    fn what_an_earlier_start_knew_is_learned_but_for_its_own_steps() {
        let mut journal = Journal::in_memory();
        let earlier = BTreeSet::from([
            ("run-from-memory".into(), "in memory".into()),
            ("enrol-nano".into(), "opened:abc".into()),
        ]);
        journal.learn(&earlier, &["run-from-memory"]).unwrap();
        assert!(journal.knows("enrol-nano", "opened:abc"));
        assert!(!journal.knows("run-from-memory", "in memory"));
    }

    #[test]
    fn a_damaged_journal_is_refused_by_its_line() {
        let directory = crate::Scratch::new("journal-damaged");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("journal");
        fs::write(
            &path,
            "{\"step\":\"a\",\"fact\":\"b\",\"at\":1}\nnot json\n",
        )
        .unwrap();
        let Err(Fault::Unreadable { what, .. }) = Journal::open(&path) else {
            panic!("a damaged journal")
        };
        assert!(what.ends_with("line 2"), "{what}");
    }
}
