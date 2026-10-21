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

#[derive(Debug, Default)]
pub struct Journal {
    path: Option<PathBuf>,
    facts: BTreeSet<(String, String)>,
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
            Ok(text) => {
                for (number, line) in text.lines().enumerate() {
                    let line: Line = serde_json::from_str(line).map_err(|why| {
                        Fault::unreadable(format!("{} line {}", path.display(), number + 1), why)
                    })?;
                    facts.insert((line.step, line.fact));
                }
            }
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
        })
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
            file.sync_all().map_err(unkept)?;
        }
        self.facts.insert((step.into(), fact.into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("guide-journal-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn a_fact_outlives_the_guide() {
        let directory = scratch("outlives");
        let path = directory.join("state").join("journal");
        let mut journal = Journal::open(&path).unwrap();
        assert!(!journal.knows("recovery-key", "typed back"));
        journal.record("recovery-key", "typed back").unwrap();

        let again = Journal::open(&path).unwrap();
        assert!(again.knows("recovery-key", "typed back"));
        assert!(!again.knows("recovery-key", "printed"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_fact_is_written_once() {
        let directory = scratch("once");
        let path = directory.join("journal");
        let mut journal = Journal::open(&path).unwrap();
        journal.record("restore", "done").unwrap();
        journal.record("restore", "done").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn the_journal_is_its_owners_alone() {
        use std::os::unix::fs::PermissionsExt;
        let directory = scratch("owner");
        let path = directory.join("state").join("journal");
        Journal::open(&path).unwrap().record("a", "b").unwrap();
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&directory.join("state")), 0o700);
        assert_eq!(mode(&path), 0o600);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_damaged_journal_is_refused_by_its_line() {
        let directory = scratch("damaged");
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
        fs::remove_dir_all(directory).unwrap();
    }
}
