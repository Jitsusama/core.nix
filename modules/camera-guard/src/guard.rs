//! What the guard does each time it looks: takes what the calendar said,
//! decides whether a meeting is near, and puts the program on the agents'
//! cgroups or takes it off, saying in the journal what changed.
//!
//! Each look sets every cgroup afresh rather than remembering which it set,
//! so a slice systemd made again, empty and unguarded, is guarded at the
//! next look, and so is one whose program someone took off.

use crate::meetings::{self, Meeting};
use std::collections::{BTreeMap, BTreeSet};
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

/// The most causes said in one stretch of an unreadable calendar.
const CAUSES: usize = 8;

/// The longest cause said, in characters. A refusal carries the daemon's
/// own text, which could run to the whole answer, and root's journal keeps
/// every line.
const BRIEF: usize = 200;

/// `why`, without its control characters, as a meeting's title is read,
/// and cut to `BRIEF` characters.
fn brief(why: &str) -> String {
    let plain: String = why.chars().filter(|c| !c.is_control()).collect();
    if plain.chars().count() <= BRIEF {
        plain
    } else {
        format!("{}...", plain.chars().take(BRIEF).collect::<String>())
    }
}

/// Whether the meetings heard are still to be kept, and what the last
/// failure to keep them said, so each failure is said once and the journal
/// hears when keeping works again.
#[derive(Default)]
pub struct Keeping {
    /// Whether the next answer must be kept even if unchanged.
    owed: bool,
    /// What the last failed keep said. Apart from the start's read, so a
    /// keep failing as the read did is still said.
    failed: Option<String>,
    /// Whether the start's read failed and no keep has worked since, so
    /// the first that does is said.
    unread: bool,
}

impl Keeping {
    /// After the start's read of the kept file failed, so the next answer
    /// writes it afresh.
    pub fn unread() -> Self {
        Self {
            owed: true,
            failed: None,
            unread: true,
        }
    }

    /// Whether the next answer must be kept even if its meetings are the same.
    pub fn owed(&self) -> bool {
        self.owed
    }

    /// Takes how a keep went, answering the journal's line, if any.
    pub fn kept(&mut self, how: Result<(), String>) -> Option<String> {
        match how {
            Ok(()) => {
                self.owed = false;
                let was_failing = self.failed.take().is_some();
                let was_unread = std::mem::take(&mut self.unread);
                (was_failing || was_unread)
                    .then(|| "<5>the meetings are kept for the next run again".to_owned())
            }
            Err(why) => {
                self.owed = true;
                let why = brief(&why);
                if self.failed.as_ref() == Some(&why) {
                    return None;
                }
                let line = format!("<4>the meetings couldn't be kept for the next run: {why}");
                self.failed = Some(why);
                Some(line)
            }
        }
    }
}

/// What the guard knows between looks.
#[derive(Default)]
pub struct Watch {
    /// The meetings from the last answer the calendar gave.
    pub meetings: Vec<Meeting>,
    /// Each cause said while the calendar can't be read, so a daemon that
    /// alternates between two, stalling then failing, is heard once for
    /// each rather than twice a minute.
    unread: Option<BTreeSet<String>>,
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
    /// again for each new cause, so the journal never names only a cause
    /// that has passed, as a refusal followed by a stall would. A cause
    /// already said this stretch isn't said again, even when it comes back
    /// after another, so a daemon cycling between causes is heard once for
    /// each; and past `CAUSES` the journal says once that it stops, since a
    /// cause's text is the daemon's and may vary without end.
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
                let why = brief(&why);
                let first = self.unread.is_none();
                let causes = self.unread.get_or_insert_with(BTreeSet::new);
                if first {
                    said.push(format!(
                        "<4>the calendar can't be read, so the {} meetings it last said \
                         stand: {why}",
                        self.meetings.len()
                    ));
                } else if !causes.contains(&why) {
                    match causes.len() {
                        n if n < CAUSES => {
                            said.push(format!("<4>the calendar still can't be read, now: {why}"));
                        }
                        n if n == CAUSES => said.push(
                            "<4>the calendar still can't be read, and its further causes go \
                             unsaid until it answers"
                                .to_owned(),
                        ),
                        _ => {}
                    }
                }
                // One past `CAUSES` marks the stretch as muted, so the set stays
                // bounded and the muting is said once.
                if causes.len() <= CAUSES {
                    causes.insert(why);
                }
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
