//! What a stop that hands its builder over leaves for the next start: for
//! each builder, the helper still carrying its builds, and whether a run
//! outliving its caller builds there, and whether its way still takes new
//! connections, as a restart leaves it. A start that can't take that builder
//! back leaves it to finish rather than give it back mid-build: on 10
//! November a switch brought a helper that refused to take back the two
//! builders made before it, and the start gave both back, one of them
//! 9,700 objects into a kernel compile.

use std::fs;
use std::path::{Path, PathBuf};

use crate::decide;

/// The folder, in the lender's, holding a note for each builder finishing.
pub const FOLDER: &str = "finishing.d";

/// What one builder left to finish has.
#[derive(Debug, PartialEq, Eq)]
pub struct Finishing {
    /// The helper carrying its builds, while one does.
    pub helper: Option<u32>,
    /// Whether a run outliving its caller built there as it was left.
    pub outliving: bool,
    /// When the time it was asked to live runs out, in seconds since the
    /// epoch, after which nothing holds it.
    pub ends: u64,
    /// Whether its way still takes new connections, as one a restart left
    /// does until the next start holds the builder or names it nowhere.
    pub accepting: bool,
}

fn path(dir: &Path, name: &str) -> PathBuf {
    dir.join(FOLDER).join(name)
}

/// Note that the builder `name` finishes what it has.
pub fn note(dir: &Path, name: &str, finishing: &Finishing) -> std::io::Result<()> {
    fs::create_dir_all(dir.join(FOLDER))?;
    let helper = finishing
        .helper
        .map_or_else(|| "-".to_owned(), |pid| pid.to_string());
    let outliving = u8::from(finishing.outliving);
    let accepting = u8::from(finishing.accepting);
    let text = format!("{helper} {outliving} {} {accepting}\n", finishing.ends);
    fs::write(path(dir, name), text)
}

/// What the note for `name` says, if there is one it can read.
pub fn read(dir: &Path, name: &str) -> Option<Finishing> {
    let text = fs::read_to_string(path(dir, name)).ok()?;
    let mut words = text.split_whitespace();
    let helper = match words.next()? {
        "-" => None,
        pid => Some(pid.parse().ok()?),
    };
    let outliving = words.next()? == "1";
    let ends = words.next()?.parse().ok()?;
    // A note from before ways were left taking connections says nothing.
    let accepting = words.next() == Some("1");
    Some(Finishing {
        helper,
        outliving,
        ends,
        accepting,
    })
}

/// Whether the builder `name` still finishes what the last stop left it
/// at `now`: its helper runs on, or a run outliving its caller holds it,
/// until its time is up, as `decide::leaves` says for one handed over. A
/// note that holds nothing any longer goes.
pub fn still(dir: &Path, name: &str, now: u64) -> bool {
    let Some(finishing) = read(dir, name) else {
        return false;
    };
    let gone = finishing.helper.is_none_or(|pid| !carries(pid, name));
    let leaves = decide::leaves(now, finishing.ends, gone, finishing.outliving);
    if leaves {
        forget(dir, name);
    }
    !leaves
}

/// Whether the process `pid` runs, and is a helper for `name` rather than
/// another that took its pid since: not a zombie nobody reaped, and named
/// with `name` on its command line.
fn carries(pid: u32, name: &str) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 only asks whether the process exists.
    if unsafe { libc::kill(pid, 0) } != 0 {
        return false;
    }
    let zombie = fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z'))
    });
    let named = fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|line| {
        line.split(|&byte| byte == 0)
            .any(|word| word == name.as_bytes())
    });
    !zombie && named
}

/// The builders a restart left whose ways still take new connections.
pub fn accepting(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir.join(FOLDER)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| {
            read(dir, name).is_some_and(|left| {
                left.accepting && left.helper.is_some_and(|pid| carries(pid, name))
            })
        })
        .collect();
    names.sort();
    names
}

/// Tell the way to `name` a restart left taking connections to take no
/// new one and finish those it has, as a helper told to hand over does,
/// and note it no longer takes them.
pub fn hand_over(dir: &Path, name: &str) {
    let Some(mut left) = read(dir, name).filter(|left| left.accepting) else {
        return;
    };
    if let Some(pid) = left.helper.filter(|&pid| carries(pid, name))
        && let Ok(pid) = libc::pid_t::try_from(pid)
    {
        // SAFETY: a signal to the helper `carries` named, which traps it.
        unsafe { libc::kill(pid, libc::SIGUSR1) };
    }
    left.accepting = false;
    if let Err(error) = note(dir, name, &left) {
        eprintln!("nix-offload: couldn't note that {name}'s way hands over: {error}");
    }
}

/// Forget the note for `name`, once it's given back.
pub fn forget(dir: &Path, name: &str) {
    let _ = fs::remove_file(path(dir, name));
}
