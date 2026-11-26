//! What the guard does each time it looks: takes what the calendar said,
//! decides whether a meeting is near, and puts the program on the agents'
//! cgroups or takes it off, saying in the journal what changed.
//!
//! Each look sets every cgroup afresh rather than remembering which it set,
//! so a slice systemd made again, empty and unguarded, is guarded at the
//! next look, and so is one whose program someone took off.

use crate::meetings::{self, Meeting};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// Where the program goes on or comes off; the kernel's, or a test's.
pub trait Cgroups {
    /// Puts the program on `path` or takes it off, as `on` says, answering
    /// whether anything changed.
    ///
    /// # Errors
    ///
    /// The cgroup isn't there, or the kernel refuses.
    fn set(&self, path: &Path, on: bool) -> io::Result<bool>;
}

impl Cgroups for crate::bpf::Guard {
    fn set(&self, path: &Path, on: bool) -> io::Result<bool> {
        Self::set(self, path, on)
    }
}

/// What the guard knows between looks.
#[derive(Default)]
pub struct Watch {
    /// The meetings from the last answer the calendar gave.
    pub meetings: Vec<Meeting>,
    /// Why the calendar couldn't be read, while it can't.
    unread: Option<String>,
    /// Whether the guard was up at the last look, unknown before the first.
    up: Option<bool>,
    /// What each cgroup's last failure said, so it's said once, not each look.
    failed: BTreeMap<PathBuf, String>,
}

impl Watch {
    /// Starts from the meetings an earlier run kept.
    pub fn from(meetings: Vec<Meeting>) -> Self {
        Self {
            meetings,
            ..Self::default()
        }
    }

    /// Takes the calendar's latest answer, answering the journal's lines and
    /// whether the meetings changed, to be kept. A calendar that can't be
    /// read leaves the meetings it last said, so a meeting already known
    /// still keeps the guard up. Why it can't is said when it starts and
    /// again each time the cause changes, so the journal never goes on
    /// naming a cause that has passed, as a refusal followed by a stall would.
    pub fn heard(&mut self, answer: Result<Vec<Meeting>, String>) -> (Vec<String>, bool) {
        let mut said = Vec::new();
        match answer {
            Ok(meetings) => {
                if self.unread.take().is_some() {
                    said.push("<5>the calendar answers again".to_owned());
                }
                let changed = meetings != self.meetings;
                self.meetings = meetings;
                (said, changed)
            }
            Err(why) => {
                match &self.unread {
                    None => said.push(format!(
                        "<4>the calendar can't be read, so the {} meetings it last said \
                         stand: {why}",
                        self.meetings.len()
                    )),
                    Some(was) if *was != why => {
                        said.push(format!("<4>the calendar still can't be read, now: {why}"));
                    }
                    Some(_) => {}
                }
                self.unread = Some(why);
                (said, false)
            }
        }
    }

    /// Looks at `now`, putting the program on each of `paths` while a
    /// meeting is near and taking it off otherwise, answering the journal's
    /// lines.
    pub fn look(&mut self, cgroups: &impl Cgroups, paths: &[PathBuf], now: u64) -> Vec<String> {
        let near = meetings::near(&self.meetings, now);
        let on = near.is_some();
        let mut said = Vec::new();
        if self.up != Some(on) {
            said.push(match near {
                Some(meeting) => {
                    format!("<5>agents may open no camera until {} ends", meeting.title)
                }
                None => "<5>agents may open a camera".to_owned(),
            });
            self.up = Some(on);
        }
        for path in paths {
            match cgroups.set(path, on) {
                Ok(changed) => {
                    self.failed.remove(path);
                    if changed {
                        let verb = if on { "guarded" } else { "let go of" };
                        said.push(format!("<6>{verb} {}", path.display()));
                    }
                }
                // A slice nothing runs in has no cgroup, and nothing in it
                // to guard; it's guarded at the first look after it's made.
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    self.failed.remove(path);
                }
                Err(error) => {
                    let why = error.to_string();
                    if self.failed.get(path) != Some(&why) {
                        let verb = if on { "guard" } else { "let go of" };
                        said.push(format!("<3>couldn't {verb} {}: {why}", path.display()));
                        self.failed.insert(path.clone(), why);
                    }
                }
            }
        }
        said
    }
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod tests;
