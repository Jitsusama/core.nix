//! What the guard did, a JSON line at a time, so `disk-guard journal` and
//! anyone reading after a surprise can see each delete and why. A journal
//! past its limit is set aside as one earlier file, replacing the one
//! before, so it never grows without end.

use crate::band::Band;
use crate::pass::{Outcome, Reclaimer};
use crate::plan::Candidate;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Where a journal starts again, and keeps one earlier file.
pub const LIMIT: u64 = 16 << 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// One candidate, and what became of it.
    Action {
        at: f64,
        band: Band,
        path: PathBuf,
        kind: String,
        size: u64,
        outcome: Outcome,
    },
    /// A directory about to be moved aside, beneath `root`.
    Begin {
        at: f64,
        path: PathBuf,
        root: PathBuf,
    },
    /// A pass, how it ended, and how long it took.
    Pass {
        at: f64,
        band: Band,
        end: String,
        freed: u64,
        seconds: f64,
    },
}

impl Line {
    fn json(&self) -> Value {
        match self {
            Line::Action {
                at,
                band,
                path,
                kind,
                size,
                outcome,
            } => {
                let (outcome, reason) = match outcome {
                    Outcome::Reclaimed => ("reclaimed", None),
                    Outcome::Kept(reason) => ("kept", Some(reason)),
                    Outcome::Failed(reason) => ("failed", Some(reason)),
                };
                json!({
                    "event": "action", "at": at, "band": band, "path": path,
                    "kind": kind, "size": size, "outcome": outcome, "reason": reason,
                })
            }
            Line::Pass {
                at,
                band,
                end,
                freed,
                seconds,
            } => json!({
                "event": "pass", "at": at, "band": band, "end": end, "freed": freed,
                "seconds": seconds,
            }),
            Line::Begin { at, path, root } => {
                json!({ "event": "begin", "at": at, "path": path, "root": root })
            }
        }
    }
}

/// A reclaimer that journals each candidate as it's taken: a directory
/// is begun first, durably, so a guard that dies with it moved aside
/// knows where to look when it starts again. A file goes in one unlink
/// and needs no beginning. Journalling never stands in the way of
/// reclaiming: a full disk refuses the journal just when a pass matters
/// most, so what can't be written is told of and the pass goes on.
pub struct Journaled<'a, R> {
    pub inner: R,
    pub journal: &'a mut Journal,
    /// The root each candidate's delete is anchored to.
    pub roots: HashMap<PathBuf, PathBuf>,
    pub at: f64,
    pub band: Band,
}

impl<R: Reclaimer> Reclaimer for Journaled<'_, R> {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let directory = fs::symlink_metadata(&candidate.path).is_ok_and(|meta| meta.is_dir());
        if directory && let Some(root) = self.roots.get(&candidate.path) {
            let begun = self.journal.write(&Line::Begin {
                at: self.at,
                path: candidate.path.clone(),
                root: root.clone(),
            });
            if let Err(error) = begun {
                eprintln!(
                    "disk-guard: journalling {}: {error}",
                    candidate.path.display()
                );
            }
        }
        let outcome = self.inner.reclaim(candidate);
        let done = self.journal.write(&Line::Action {
            at: self.at,
            band: self.band,
            path: candidate.path.clone(),
            kind: candidate.class.kind().to_owned(),
            size: candidate.size,
            outcome: outcome.clone(),
        });
        if let Err(error) = done {
            eprintln!(
                "disk-guard: journalling {}: {error}",
                candidate.path.display()
            );
        }
        outcome
    }
}

pub struct Journal {
    path: PathBuf,
    limit: u64,
}

impl Journal {
    pub fn new(path: &Path, limit: u64) -> Self {
        Journal {
            path: path.to_owned(),
            limit,
        }
    }

    /// Each directory begun and never ended with an action, oldest first,
    /// with the root it was reached through: what a guard that died mid
    /// reclaim left moved aside.
    pub fn unfinished(&self) -> io::Result<Vec<(PathBuf, PathBuf)>> {
        let mut begun: Vec<(PathBuf, PathBuf)> = Vec::new();
        for path in [self.earlier(), self.path.clone()] {
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            for line in text.lines() {
                // A line cut short by the crash says nothing.
                let Ok(line) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let Some(path) = line["path"].as_str().map(PathBuf::from) else {
                    continue;
                };
                begun.retain(|(seen, _)| *seen != path);
                if line["event"] == "begin"
                    && let Some(root) = line["root"].as_str()
                {
                    begun.push((path, PathBuf::from(root)));
                }
            }
        }
        Ok(begun)
    }

    fn earlier(&self) -> PathBuf {
        let mut earlier = self.path.clone().into_os_string();
        earlier.push(".1");
        PathBuf::from(earlier)
    }

    /// Appends one line, setting the journal aside first if the line would
    /// take it past its limit.
    pub fn write(&mut self, line: &Line) -> io::Result<()> {
        let mut text = line.json().to_string();
        text.push('\n');
        let size = match fs::metadata(&self.path) {
            Ok(meta) => meta.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error),
        };
        if size > 0 && size + text.len() as u64 > self.limit {
            fs::rename(&self.path, self.earlier())?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(text.as_bytes())?;
        if let Line::Begin { .. } = line {
            // On the disk before the move it announces, so even a power cut
            // can't leave a moved directory nobody knows of.
            file.sync_data()?;
        }
        Ok(())
    }
}
