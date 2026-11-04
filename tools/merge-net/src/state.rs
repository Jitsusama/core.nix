//! What the net keeps between runs, in its state folder
//! (`~/.local/state/net` unless `MERGE_NET_STATE` names another), so it
//! carries on where it was after a restart or a reboot:
//!
//! - `state.json`: the last head every changed VM test passed at, which the
//!   next head is compared with, and the last head it finished looking at.
//! - `results.jsonl`: a line for each run of a check at a commit, with its
//!   exit and its log, so no check that passed at a commit runs there again.
//! - `hold.json`: while a merge broke a test and its revert isn't on main,
//!   the bad commit and the revert, which a switch reads to stay put.
//! - `findings/`: a file for each failure, for a session to tell its owner.
//! - `logs/`: each run's log, and the net's own in `net.log`.
//! - `core/`: the net's own checkout of the repository it watches.
//! - `pause`: while it's there, the net starts nothing.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// The net's state folder and what it holds.
pub struct State {
    pub root: PathBuf,
}

/// How a run of a check at a commit ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ran {
    Passed,
    Failed,
}

impl State {
    /// The folder `MERGE_NET_STATE` names, or `~/.local/state/net`.
    pub fn from_env() -> Self {
        let root = std::env::var_os("MERGE_NET_STATE")
            .filter(|folder| !folder.is_empty())
            .map_or_else(
                || {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                        .join(".local/state/net")
                },
                PathBuf::from,
            );
        Self { root }
    }

    /// Make the folders it writes into.
    pub fn make(&self) -> Result<(), String> {
        for folder in ["findings", "logs"] {
            fs::create_dir_all(self.root.join(folder))
                .map_err(|error| format!("making {}: {error}", self.root.join(folder).display()))?;
        }
        Ok(())
    }

    pub fn hold_file(&self) -> PathBuf {
        self.root.join("hold.json")
    }

    pub fn paused(&self) -> bool {
        self.root.join("pause").exists()
    }

    /// Where a run of `check` at `commit`, its `attempt`th, logs.
    pub fn log(&self, commit: &str, check: &str, attempt: u32) -> PathBuf {
        self.root
            .join("logs")
            .join(format!("{}-{check}-{attempt}.log", short(commit)))
    }

    /// The head the next is compared with, and the last one looked at.
    pub fn heads(&self) -> (Option<String>, Option<String>) {
        let read = read_json(&self.root.join("state.json"));
        let field = |name: &str| {
            read.as_ref()
                .and_then(|state| state[name].as_str().map(str::to_owned))
        };
        (field("passed"), field("seen"))
    }

    pub fn set_heads(&self, passed: &str, seen: &str) -> Result<(), String> {
        write_json(
            &self.root.join("state.json"),
            &json!({ "passed": passed, "seen": seen, "at": now() }),
        )
    }

    /// Each run of `check` at `commit` so far, oldest first.
    pub fn runs(&self, commit: &str, check: &str) -> Vec<Ran> {
        let Ok(text) = fs::read_to_string(self.root.join("results.jsonl")) else {
            return Vec::new();
        };
        runs_in(&text, commit, check)
    }

    pub fn record(
        &self,
        commit: &str,
        check: &str,
        ran: Ran,
        code: Option<i32>,
        log: &Path,
    ) -> Result<(), String> {
        let line = json!({
            "at": now(),
            "commit": commit,
            "check": check,
            "passed": ran == Ran::Passed,
            "code": code,
            "log": log.display().to_string(),
        });
        append(&self.root.join("results.jsonl"), &line)
    }

    /// The hold a broken merge put on switching, if one stands.
    pub fn hold(&self) -> Option<Value> {
        read_json(&self.hold_file())
    }

    pub fn set_hold(&self, hold: &Value) -> Result<(), String> {
        write_json(&self.hold_file(), hold)
    }

    pub fn clear_hold(&self) -> Result<(), String> {
        match fs::remove_file(self.hold_file()) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("clearing the hold: {error}"))
            }
            _ => Ok(()),
        }
    }

    /// Write a finding for whoever owns what failed, as a file of its own.
    /// A finding of the same kind for the same test at the same commit is
    /// written once, however many rounds look at that commit.
    pub fn find(&self, finding: &Value) -> Result<PathBuf, String> {
        let commit = finding["commit"].as_str().unwrap_or_default();
        let check = finding["test"].as_str().unwrap_or("net");
        if let Some(kept) = self.kept(finding["kind"].as_str().unwrap_or_default(), check, commit) {
            return Ok(kept);
        }
        let path =
            self.root
                .join("findings")
                .join(format!("{}-{}-{check}.json", now(), short(commit)));
        write_json(&path, finding)?;
        Ok(path)
    }

    /// Whether a finding of `kind` for `test` at `commit` is written.
    pub fn found(&self, kind: &str, test: &str, commit: &str) -> bool {
        self.kept(kind, test, commit).is_some()
    }

    fn kept(&self, kind: &str, test: &str, commit: &str) -> Option<PathBuf> {
        fs::read_dir(self.root.join("findings"))
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                read_json(path).is_some_and(|finding| {
                    finding["kind"] == kind
                        && finding["test"] == test
                        && finding["commit"] == commit
                })
            })
    }

    /// Say something in the net's own log, and on its standard error.
    pub fn say(&self, what: &str) {
        eprintln!("merge-net: {what}");
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("logs/net.log"))
            .and_then(|mut log| writeln!(log, "{} {what}", now()));
    }
}

/// Each run of `check` at `commit` in a results journal's text.
pub fn runs_in(text: &str, commit: &str, check: &str) -> Vec<Ran> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["commit"] == commit && line["check"] == check)
        .map(|line| {
            if line["passed"] == true {
                Ran::Passed
            } else {
                Ran::Failed
            }
        })
        .collect()
}

/// A commit's first twelve characters, as names and logs give it.
pub fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

/// Write a file whole, by a rename, so a reader never sees half of it.
fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let partial = path.with_extension("partial");
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&partial, text + "\n")
        .and_then(|()| fs::rename(&partial, path))
        .map_err(|error| format!("writing {}: {error}", path.display()))
}

fn append(path: &Path, value: &Value) -> Result<(), String> {
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| writeln!(file, "{value}"))
        .map_err(|error| format!("writing {}: {error}", path.display()))
}

/// The time now, in seconds since the epoch, as every file here gives it.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str) -> State {
        let root = std::env::temp_dir().join(format!("merge-net-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let state = State { root };
        state.make().expect("made");
        state
    }

    #[test]
    fn a_run_is_kept_by_its_commit_and_check() {
        let state = state("runs");
        let log = state.log("aaaa", "vm-a", 1);
        state
            .record("aaaa", "vm-a", Ran::Failed, Some(1), &log)
            .unwrap();
        state
            .record("aaaa", "vm-a", Ran::Passed, Some(0), &log)
            .unwrap();
        state
            .record("bbbb", "vm-a", Ran::Passed, Some(0), &log)
            .unwrap();
        assert_eq!(state.runs("aaaa", "vm-a"), vec![Ran::Failed, Ran::Passed]);
        assert_eq!(state.runs("aaaa", "vm-b"), vec![]);
        fs::remove_dir_all(&state.root).unwrap();
    }

    #[test]
    fn the_heads_and_the_hold_outlast_the_program() {
        let state = state("heads");
        assert_eq!(state.heads(), (None, None));
        state.set_heads("aaaa", "bbbb").unwrap();
        assert_eq!(
            State {
                root: state.root.clone()
            }
            .heads(),
            (Some("aaaa".to_owned()), Some("bbbb".to_owned()))
        );
        state.set_hold(&json!({ "bad": "cccc" })).unwrap();
        assert_eq!(state.hold().unwrap()["bad"], "cccc");
        state.clear_hold().unwrap();
        assert!(state.hold().is_none());
        state.clear_hold().unwrap();
        fs::remove_dir_all(&state.root).unwrap();
    }
}
