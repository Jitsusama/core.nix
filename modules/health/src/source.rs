//! Where the day's lines come from: systemd's journals on Linux, the
//! unified log on a Mac. The follower takes one source and asks it for its
//! lines, how to read each, when each boot before this one ended, and where
//! the processes and the system's disk guard are to look at; nothing else in
//! health knows which system it runs on.

use std::collections::BTreeMap;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use jiff::Zoned;
use jiff::tz::TimeZone;

use crate::follow;
use crate::journal::{self, Entry};
use crate::reports;
use crate::unified;

/// What the day's lines are read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// systemd's journals, through journalctl.
    Journal,
    /// A Mac's unified log, through `log`.
    UnifiedLog,
}

impl Source {
    /// The source of the system health runs on.
    #[must_use]
    pub const fn here() -> Self {
        if cfg!(target_os = "macos") {
            Self::UnifiedLog
        } else {
            Self::Journal
        }
    }

    /// The entry a line of this source's holds, or none.
    #[must_use]
    pub fn parse(self, line: &str) -> Option<Entry> {
        match self {
            Self::Journal => journal::parse(line),
            Self::UnifiedLog => unified::parse(line),
        }
    }

    /// Start each of this source's followers from the day's start in
    /// `zone`, their lines sent on, saying how many there are.
    ///
    /// # Errors
    ///
    /// When one can't start.
    pub fn start(self, zone: &TimeZone, lines: &Sender<Option<String>>) -> io::Result<usize> {
        let asked = match self {
            Self::Journal => journal::followed()
                .iter()
                .map(|asked| journal::following(asked))
                .collect::<Vec<_>>(),
            Self::UnifiedLog => {
                let now = Zoned::now().with_time_zone(zone.clone());
                let midnight = format!("{} 00:00:00", now.date());
                let now = now.strftime("%Y-%m-%d %H:%M:%S").to_string();
                unified::followed(&midnight, &now)
                    .iter()
                    .map(|asked| {
                        let mut log = Command::new(unified::LOG);
                        log.args(asked);
                        log
                    })
                    .collect()
            }
        };
        let count = asked.len();
        for command in asked {
            spawn(command, lines.clone())?;
        }
        Ok(count)
    }

    /// When each boot before this one ended. A Mac's log marks no boot's
    /// stages, so there none is timed.
    ///
    /// # Errors
    ///
    /// When journalctl can't be asked.
    pub fn ends(self) -> io::Result<BTreeMap<String, i64>> {
        match self {
            Self::Journal => {
                let boots = Command::new("journalctl")
                    .args(["--list-boots", "--output=json", "--no-pager"])
                    .output()?;
                Ok(follow::ends_of(&String::from_utf8_lossy(&boots.stdout)))
            }
            Self::UnifiedLog => Ok(BTreeMap::new()),
        }
    }

    /// Where each process can be looked at, which only Linux has: a Mac's
    /// memory is asked of `ps` instead, and the agents that died there
    /// aren't read yet.
    #[must_use]
    pub fn processes(self) -> Option<&'static Path> {
        match self {
            Self::Journal => Some(Path::new("/proc")),
            Self::UnifiedLog => None,
        }
    }

    /// Whether the watched programs' memory is asked of `ps`, as on a Mac,
    /// which has no `/proc` to look in.
    #[must_use]
    pub const fn asks_ps(self) -> bool {
        matches!(self, Self::UnifiedLog)
    }

    /// Where each group of processes says what processor time it used,
    /// which only Linux's cgroups say.
    #[must_use]
    pub fn cgroups(self) -> Option<&'static Path> {
        match self {
            Self::Journal => Some(Path::new("/sys/fs/cgroup")),
            Self::UnifiedLog => None,
        }
    }

    /// Where crash reports are written, under the account's `home`, which
    /// only a Mac has: Linux's core dumps are said in the journal.
    #[must_use]
    pub fn reports(self, home: Option<&Path>) -> Vec<PathBuf> {
        match self {
            Self::Journal => Vec::new(),
            Self::UnifiedLog => reports::folders(home),
        }
    }

    /// Where the system's disk guard says how the disks stand.
    #[must_use]
    pub fn guard_status(self) -> PathBuf {
        match self {
            Self::Journal => PathBuf::from("/run/disk-guard/status.json"),
            Self::UnifiedLog => PathBuf::from("/var/run/disk-guard/status.json"),
        }
    }
}

/// Start `command`, its output's lines sent on, and a `None` when it ends.
fn spawn(mut command: Command, lines: Sender<Option<String>>) -> io::Result<()> {
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let out = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("no output"))?;
    thread::spawn(move || {
        let mut out = BufReader::new(out);
        let mut line = Vec::new();
        // A line that isn't UTF-8 is read as far as it goes; journalctl
        // writes such a message as bytes, so it's rare.
        while out.read_until(b'\n', &mut line).is_ok_and(|read| read > 0) {
            if lines
                .send(Some(String::from_utf8_lossy(&line).into_owned()))
                .is_err()
            {
                break;
            }
            line.clear();
        }
        let _ = child.wait();
        let _ = lines.send(None);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_source_reads_its_own_lines() {
        let journal = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","MESSAGE":"m"}"#;
        let unified = r#"{"timestamp":"2026-11-12 05:55:36.0-0500","messageType":"Error","eventType":"logEvent","processImagePath":"/usr/libexec/d","eventMessage":"m"}"#;
        assert!(
            Source::Journal.parse(journal).is_some() && Source::Journal.parse(unified).is_none(),
            "the journal reads the journal's lines alone"
        );
        assert!(
            Source::UnifiedLog.parse(unified).is_some()
                && Source::UnifiedLog.parse(journal).is_none(),
            "the unified log reads its own lines alone"
        );
    }
}
