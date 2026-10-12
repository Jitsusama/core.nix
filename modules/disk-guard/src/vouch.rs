//! The system daemon vouching for processes a user daemon can't read.
//!
//! A user daemon can't read the open files of root's processes, so it
//! would judge every candidate unknown and keep it. Once a scan, it asks
//! the system daemon which processes touch anything under its roots. The
//! answer vouches, by pid and start, for every process the system daemon
//! read, and tells of those touching the asker's own files, with only
//! what judging a use needs: those files, the name each was called by,
//! and the build variables pointing there. Nothing else of another
//! account's processes leaves the system daemon.

use crate::activity::Process;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// Every process the system daemon read, by pid and start in ticks.
    pub vouched: Vec<(u32, u64)>,
    /// Those touching the asker's files beneath its roots, cut down to that.
    pub touching: Vec<Process>,
}

/// The variables that point a build at a target, as activity reads them.
const BUILD_VARIABLES: [&str; 2] = ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"];

/// What the system daemon tells `asker` of `processes` beneath `roots`,
/// where `owner` says whose a path is.
pub fn answer(
    processes: &[Process],
    asker: u32,
    roots: &[PathBuf],
    owner: impl Fn(&Path) -> Option<u32>,
) -> Answer {
    let theirs = |path: &Path| {
        path.is_absolute()
            && roots.iter().any(|root| path.starts_with(root))
            && owner(path) == Some(asker)
    };
    let mut told = Answer::default();
    for process in processes.iter().filter(|process| process.readable) {
        told.vouched.push((process.pid, process.start_ticks));
        let keep = |paths: &[PathBuf]| -> Vec<PathBuf> {
            paths.iter().filter(|path| theirs(path)).cloned().collect()
        };
        let cwd = process.cwd.clone().filter(|cwd| theirs(cwd));
        let files = keep(&process.files);
        let maps = keep(&process.maps);
        let env: Vec<(OsString, OsString)> = process
            .env
            .iter()
            .filter(|(name, value)| {
                let value = Path::new(value);
                // A variable naming the asker's target, something in it, or
                // a directory of the asker's its targets sit in.
                BUILD_VARIABLES.iter().any(|variable| name == variable)
                    && owner(value) == Some(asker)
                    && roots
                        .iter()
                        .any(|root| value.starts_with(root) || root.starts_with(value))
            })
            .cloned()
            .collect();
        let exe_here = process.exe.as_deref().is_some_and(theirs);
        if cwd.is_none() && files.is_empty() && maps.is_empty() && env.is_empty() && !exe_here {
            continue;
        }
        let exe = if exe_here {
            process.exe.clone()
        } else {
            process
                .exe
                .as_deref()
                .and_then(Path::file_name)
                .map(PathBuf::from)
        };
        let called = process
            .cmdline
            .first()
            .and_then(|word| Path::new(word).file_name())
            .map(OsString::from);
        let mode = process
            .cmdline
            .get(1)
            .filter(|word| *word == "proc-macro")
            .cloned();
        told.touching.push(Process {
            pid: process.pid,
            uid: process.uid,
            start_ticks: process.start_ticks,
            name: process.name.clone(),
            exe,
            cwd,
            cmdline: called.into_iter().chain(mode).collect(),
            env,
            files,
            maps,
            readable: true,
            write_bytes: None,
            cgroup: None,
        });
    }
    told
}

/// The asker's scan with what the system daemon vouched for in place of
/// what it couldn't read. A process it never mentioned, or whose pid now
/// belongs to a process started since, stays unreadable.
pub fn merge(local: Vec<Process>, answer: &Answer) -> Vec<Process> {
    local
        .into_iter()
        .map(|process| {
            if process.readable {
                return process;
            }
            let same = |seen: &&Process| {
                seen.pid == process.pid && seen.start_ticks == process.start_ticks
            };
            if let Some(seen) = answer.touching.iter().find(same) {
                return seen.clone();
            }
            if answer.vouched.contains(&(process.pid, process.start_ticks)) {
                return Process {
                    pid: process.pid,
                    uid: process.uid,
                    start_ticks: process.start_ticks,
                    name: process.name,
                    readable: true,
                    ..Default::default()
                };
            }
            process
        })
        .collect()
}
