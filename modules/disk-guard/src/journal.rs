//! What the guard did, a JSON line at a time, so `disk-guard journal` and
//! anyone reading after a surprise can see each delete and why. A journal
//! past its limit is set aside as one earlier file, replacing the one
//! before, so it never grows without end.

use crate::band::Band;
use crate::pass::Outcome;
use serde_json::{Value, json};
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
    /// A pass, and how it ended.
    Pass {
        at: f64,
        band: Band,
        end: String,
        freed: u64,
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
            } => json!({ "event": "pass", "at": at, "band": band, "end": end, "freed": freed }),
        }
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
            let mut earlier = self.path.clone().into_os_string();
            earlier.push(".1");
            fs::rename(&self.path, earlier)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(text.as_bytes())
    }
}
