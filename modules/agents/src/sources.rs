//! Each source followed on a thread of its own, its answers handed to the
//! daemon's one loop, which holds every record (inputs.rs): the node's
//! watches, the to-do daemon's, the files heard as they land, and what
//! the shell says over the daemon's own socket. A daemon's watch is
//! followed as every daemon of the shell follows one (daemon-serve's
//! `watch`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;

pub use daemon_serve::watch::follow;
use serde_json::Value;

use crate::file::{self, Files};
use crate::inputs::Inputs;

/// Something a source said.
#[derive(Debug)]
pub enum Heard {
    /// An answer to the node's watch of its sessions.
    Sessions(Value),
    /// An answer to the node's watch of the machine's queue's jobs.
    Jobs(Value),
    /// An answer to the to-do daemon's watch of its to-dos.
    ToDos(Value),
    /// An answer to the windows daemon's watch of its windows.
    Windows(Value),
    /// Files that landed or went.
    Files(Vec<PathBuf>),
    /// Whether the shell says the screen is shared.
    Sharing(bool),
    /// Joel puts off the last boot's sessions, as the shell says.
    PutOff,
    /// Joel forgets one of the sessions to reopen, by pi's id for it.
    Forget(String),
    /// The sessions the boot being restored kept a place of, by pi's id,
    /// as the shell says.
    Arranged(HashSet<String>),
}

/// Where the files the rows are made from are kept.
#[derive(Clone, Debug)]
pub struct Paths {
    /// Spend-watch's collection.
    pub spend: PathBuf,
    /// Spend-watch's budgets, beside its collection.
    pub budgets: PathBuf,
    /// The harness's list of orchestrators.
    pub orchestrators: PathBuf,
    /// The harness's list of sessions to reopen.
    pub restore: PathBuf,
    /// What Joel said of those, which the daemon alone writes, so it's
    /// read at the start rather than followed.
    pub kept: PathBuf,
}

impl Paths {
    /// The files followed.
    pub fn all(&self) -> Vec<PathBuf> {
        vec![
            self.spend.clone(),
            self.budgets.clone(),
            self.orchestrators.clone(),
            self.restore.clone(),
        ]
    }
}

/// Follows the files the rows are made from, handing each that lands to
/// `to`, after handing them all once, as they stand at the start.
pub fn follow_files(paths: &Paths, to: Sender<Heard>) -> std::io::Result<()> {
    let mut files = Files::open(&paths.all())?;
    let _ = to.send(Heard::Files(paths.all()));
    thread::spawn(move || {
        while let Ok(landed) = files.heard() {
            if to.send(Heard::Files(landed)).is_err() {
                return;
            }
        }
    });
    Ok(())
}

impl Inputs {
    /// Takes what a source said into the part of the inputs it changes.
    pub fn hear(&mut self, heard: Heard, paths: &Paths) {
        match heard {
            Heard::Sessions(answer) => {
                self.sessions.took(&answer);
            }
            Heard::Jobs(answer) => {
                self.jobs.took(&answer);
            }
            Heard::ToDos(answer) => {
                self.to_dos.took(&answer);
            }
            Heard::Windows(answer) => {
                self.windows.took(&answer);
            }
            Heard::Files(landed) => {
                for path in landed {
                    self.read(&path, paths);
                }
            }
            Heard::Sharing(sharing) => self.sharing = sharing,
            Heard::PutOff => {
                self.kept.put_off = self.restore.as_ref().map_or(String::new(), |said| {
                    said["boot"].as_str().unwrap_or("").to_owned()
                });
                self.keep(paths);
            }
            Heard::Forget(id) => {
                self.kept.forget(&id, self.restore.as_ref());
                self.keep(paths);
            }
            Heard::Arranged(ids) => self.arranged = ids,
        }
    }

    /// Writes what Joel said, saying when it can't, as the next start
    /// would offer again what he put off or forgot.
    fn keep(&self, paths: &Paths) {
        if let Err(why) = self.kept.write(&paths.kept) {
            eprintln!("agents: can't keep {}: {why}", paths.kept.display());
        }
    }

    fn read(&mut self, path: &Path, paths: &Paths) {
        if path == paths.spend {
            self.spend = file::read(path);
        } else if path == paths.budgets {
            self.budgets = file::read(path);
        } else if path == paths.orchestrators {
            self.orchestrators = file::read(path);
        } else if path == paths.restore {
            self.restore = file::read(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use serde_json::json;

    use super::*;
    use crate::reopen::Kept;

    fn paths_in(folder: &Path) -> Paths {
        Paths {
            spend: folder.join("collection.json"),
            budgets: folder.join("budgets.json"),
            orchestrators: folder.join("orchestrators.json"),
            restore: folder.join("restore.json"),
            kept: folder.join("agents").join("reopen.json"),
        }
    }

    #[test]
    fn what_each_source_says_changes_its_own_part() {
        let folder = std::env::temp_dir().join(format!("agents-sources-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        fs::write(&paths.spend, r#"{"agentsToday": "$4"}"#).expect("the collection");
        fs::write(&paths.budgets, r#"{"sessions": {}}"#).expect("the budgets");
        let mut inputs = Inputs::default();
        inputs.hear(
            Heard::Sessions(json!({"records": [{"key": "session:a"}]})),
            &paths,
        );
        inputs.hear(Heard::Jobs(json!({"records": [{"key": "job:1"}]})), &paths);
        inputs.hear(Heard::Files(paths.all()), &paths);
        inputs.hear(Heard::Sharing(true), &paths);
        assert!(
            inputs.sessions.get("session:a").is_some(),
            "the node's session held"
        );
        assert!(inputs.jobs.get("job:1").is_some(), "the node's job held");
        assert_eq!(inputs.spend, Some(json!({"agentsToday": "$4"})));
        assert_eq!(inputs.budgets, Some(json!({"sessions": {}})));
        assert_eq!(inputs.orchestrators, None, "a file not there holds nothing");
        assert!(inputs.sharing, "the shell's sharing held");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn what_joel_says_of_the_sessions_to_reopen_is_kept() {
        let folder = std::env::temp_dir().join(format!("agents-kept-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        fs::write(
            &paths.restore,
            r#"{"boot": "the-last", "sessions": [{"id": "01-a", "cwd": "/tmp"}]}"#,
        )
        .expect("the restore list");
        let mut inputs = Inputs::default();
        inputs.hear(Heard::Files(paths.all()), &paths);
        inputs.hear(Heard::PutOff, &paths);
        inputs.hear(Heard::Forget("01-a".into()), &paths);
        let kept = Kept {
            put_off: "the-last".into(),
            forgotten: vec!["01-a".into()],
        };
        assert_eq!(
            inputs.kept, kept,
            "the boot offered put off, and the one forgotten"
        );
        assert_eq!(Kept::read(&paths.kept), kept, "written for the next start");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_files_are_handed_over_once_as_they_stand_at_the_start() {
        let folder = std::env::temp_dir().join(format!("agents-start-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = paths_in(&folder);
        let (to, heard) = mpsc::channel();
        follow_files(&paths, to).expect("the files are followed");
        let Ok(Heard::Files(first)) = heard.recv() else {
            panic!("the files first");
        };
        assert_eq!(first, paths.all());
        let _ = fs::remove_dir_all(&folder);
    }
}
