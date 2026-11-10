//! What the net keeps between runs, in its state folder
//! (`~/.local/state/net` unless `MERGE_NET_STATE` names another), so it
//! carries on where it was after a restart or a reboot:
//!
//! - `state.json`: the last head every changed check passed at, which the
//!   next head is compared with, and the last head it finished looking at.
//! - `results.jsonl`: a line for each run of a check at a commit, with its
//!   exit and its log, so no check that passed at a commit runs there again.
//! - `hold.json`: while a merge broke a test and its revert isn't on main,
//!   the bad commit and the revert, which a switch reads to stay put.
//! - `unsettled.json`: a held test whose search couldn't tell which merge
//!   broke it, kept like a hold but leaving switches alone.
//! - `already-runs.json`: a hold whose bad merge the machine already runs,
//!   kept like a hold but leaving switches alone, since holding them keeps
//!   only fixes off the machine.
//! - `findings/`: a file for each failure, for a session to tell its owner.
//! - `logs/`: each run's log, and the net's own in `net.log`.
//! - `core/`: the net's own checkout of the repository it watches.
//! - `drv/`: the derivation each check builds at a commit, named
//!   `<commit>-<check>`, so a search evaluates each once.
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
    /// Stopped before it ended, for running too long or going silent, so
    /// it says nothing of the test.
    Stalled,
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
        for folder in ["findings", "logs", "settling"] {
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

    /// The head a round began on and hasn't finished, until `set_heads`.
    pub fn under_way(&self) -> Option<String> {
        read_json(&self.root.join("state.json"))
            .and_then(|state| state["round"].as_str().map(str::to_owned))
    }

    /// Write down that a round begins on `head`, keeping the heads.
    pub fn begin(&self, head: &str) -> Result<(), String> {
        let path = self.root.join("state.json");
        let mut written = read_json(&path).unwrap_or_else(|| json!({}));
        written["round"] = json!(head);
        write_json(&path, &written)
    }

    /// Write down the heads, which ends any round under way.
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
            "stalled": ran == Ran::Stalled,
            "code": code,
            "log": log.display().to_string(),
        });
        append(&self.root.join("results.jsonl"), &line)
    }

    /// The hold a broken merge put on switching, if one stands.
    /// Where a hold marked `unsettled` is kept. Its search couldn't tell
    /// which merge broke its test, because runs stalled or no builder took
    /// them, so nothing is known to be broken. On 8 November a hold like
    /// that kept every switch off the machine while each run of the test
    /// had only waited in a builder's queue. So it's run again like a hold
    /// but kept apart from `hold.json`, which is all a switch reads.
    fn unsettled_file(&self) -> PathBuf {
        self.root.join("unsettled.json")
    }

    /// Where a hold marked `already_runs` is kept: the machine runs its bad
    /// merge already, so a switch brings it nothing new, and holding one
    /// keeps only the merges after it off the machine.
    fn already_runs_file(&self) -> PathBuf {
        self.root.join("already-runs.json")
    }

    /// Every file a hold is kept in, the one a switch reads first.
    fn hold_files(&self) -> [PathBuf; 3] {
        [
            self.hold_file(),
            self.unsettled_file(),
            self.already_runs_file(),
        ]
    }

    pub fn hold(&self) -> Option<Value> {
        self.hold_files().iter().find_map(|file| read_json(file))
    }

    pub fn set_hold(&self, hold: &Value) -> Result<(), String> {
        let kept = if hold["unsettled"] == true {
            self.unsettled_file()
        } else if hold["already_runs"] == true {
            self.already_runs_file()
        } else {
            self.hold_file()
        };
        write_json(&kept, hold)?;
        self.hold_files()
            .iter()
            .filter(|file| **file != kept)
            .try_for_each(|file| remove(file))
    }

    pub fn clear_hold(&self) -> Result<(), String> {
        self.hold_files().iter().try_for_each(|file| remove(file))
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

    /// Keep a job handed to the settler for `test` until it's settled.
    pub fn keep_settling(&self, test: &str, job: &Value) -> Result<(), String> {
        write_json(&self.settling_file(test), job)
    }

    /// Let go of `test`'s job, settled.
    pub fn settled(&self, test: &str) -> Result<(), String> {
        let kept = self.settling_file(test);
        match fs::remove_file(&kept) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("letting go of {}: {error}", kept.display()))
            }
            _ => Ok(()),
        }
    }

    /// Every job kept for the settler.
    pub fn settling(&self) -> Vec<Value> {
        let Ok(entries) = fs::read_dir(self.root.join("settling")) else {
            return Vec::new();
        };
        let mut kept: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect();
        kept.sort();
        kept.iter().filter_map(|path| read_json(path)).collect()
    }

    fn settling_file(&self, test: &str) -> PathBuf {
        self.root.join("settling").join(format!("{test}.json"))
    }

    /// Say something in the net's own log, and on its standard error.
    pub fn say(&self, what: &str) {
        eprintln!("merge-net: {what}");
        let _ = append_line(
            &self.root.join("logs/net.log"),
            &format!("{} {what}", now()),
        );
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
            } else if line["stalled"] == true {
                Ran::Stalled
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

/// Remove a file the hold is kept in, which may not be there.
fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("clearing the hold: {error}"))
        }
        _ => Ok(()),
    }
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
    append_line(path, &value.to_string())
        .map_err(|error| format!("writing {}: {error}", path.display()))
}

/// Add `line` to the end of `path` in one write. Two tests ending at once
/// append from two threads, and a line written in pieces, as `writeln!`
/// writes a JSON value, comes out woven into the other.
fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(format!("{line}\n").as_bytes())
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

    /// A state of the test's own. The tests run side by side, so a number
    /// counted for each keeps two that chose the same name from emptying
    /// each other's folder.
    fn state(name: &str) -> State {
        static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let made = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("merge-net-{name}-{}-{made}", std::process::id()));
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
        state
            .record("cccc", "vm-a", Ran::Stalled, None, &log)
            .unwrap();
        assert_eq!(state.runs("cccc", "vm-a"), vec![Ran::Stalled]);
        assert_eq!(state.runs("aaaa", "vm-b"), vec![]);
        fs::remove_dir_all(&state.root).unwrap();
    }

    #[test]
    fn runs_ending_together_are_each_kept_whole() {
        let state = state("together");
        let checks = ["vm-a", "vm-b", "vm-c", "vm-d"];
        std::thread::scope(|scope| {
            for check in checks {
                let state = &state;
                scope.spawn(move || {
                    let log = state.log("aaaa", check, 1);
                    for _ in 0..200 {
                        state
                            .record("aaaa", check, Ran::Passed, Some(0), &log)
                            .unwrap();
                    }
                });
            }
        });
        for check in checks {
            assert_eq!(state.runs("aaaa", check).len(), 200, "{check}'s runs");
        }
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
        state
            .set_hold(&json!({ "bad": "cccc", "already_runs": true }))
            .unwrap();
        assert!(
            !state.hold_file().exists(),
            "a switch reads no hold on a merge the machine runs"
        );
        assert_eq!(state.hold().unwrap()["already_runs"], true);
        state.set_hold(&json!({ "bad": "dddd" })).unwrap();
        assert!(
            !state.already_runs_file().exists(),
            "one hold stands at a time"
        );
        state.clear_hold().unwrap();
        assert!(state.hold().is_none());
        state.clear_hold().unwrap();
        fs::remove_dir_all(&state.root).unwrap();
    }
}
