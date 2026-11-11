//! Each source followed on a thread of its own, its answers handed to the
//! daemon's one loop, which holds every record (inputs.rs): the node's
//! watches, the to-do daemon's, the files heard as they land, and what
//! the shell says over the daemon's own socket. A daemon's watch is
//! followed as every daemon of the shell follows one (daemon-serve's
//! `watch`).

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
}

impl Paths {
    pub fn all(&self) -> Vec<PathBuf> {
        vec![
            self.spend.clone(),
            self.budgets.clone(),
            self.orchestrators.clone(),
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
        }
    }

    fn read(&mut self, path: &Path, paths: &Paths) {
        if path == paths.spend {
            self.spend = file::read(path);
        } else if path == paths.budgets {
            self.budgets = file::read(path);
        } else if path == paths.orchestrators {
            self.orchestrators = file::read(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use serde_json::json;

    use super::*;

    #[test]
    fn what_each_source_says_changes_its_own_part() {
        let folder = std::env::temp_dir().join(format!("agents-sources-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = Paths {
            spend: folder.join("collection.json"),
            budgets: folder.join("budgets.json"),
            orchestrators: folder.join("orchestrators.json"),
        };
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
    fn the_files_are_handed_over_once_as_they_stand_at_the_start() {
        let folder = std::env::temp_dir().join(format!("agents-start-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let paths = Paths {
            spend: folder.join("collection.json"),
            budgets: folder.join("budgets.json"),
            orchestrators: folder.join("orchestrators.json"),
        };
        let (to, heard) = mpsc::channel();
        follow_files(&paths, to).expect("the files are followed");
        let Ok(Heard::Files(first)) = heard.recv() else {
            panic!("the files first");
        };
        assert_eq!(first, paths.all());
        let _ = fs::remove_dir_all(&folder);
    }
}
