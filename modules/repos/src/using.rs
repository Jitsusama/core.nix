//! What works in each checkout and tree, and what it takes: the
//! processes whose working folder lies in one, their processor time
//! between two readings and the memory they hold. Joel asked how much
//! space, processor and memory each repository takes; the space is the
//! store's weight (weight.rs), and this is the rest.
//!
//! A process is placed in the deepest checkout or tree whose folder holds
//! its working folder, so an agent's cargo in a tree counts for that tree,
//! not the checkout it was cut from where one lies inside the other. Its
//! memory is its proportional share, `Pss` in `smaps_rollup`, so forty
//! compilers sharing one library count it once between them, and it's
//! said in kilobytes whatever the machine's page size. Its processor time
//! is `utime` and `stime` in `stat`, in the ticks of a hundredth of a
//! second Linux's interface always counts in, and a folder's is a share
//! of one core between two readings: a process new since the first is
//! counted from its start, and one that only moved in since isn't
//! counted, as nothing says how long it has worked there.
//!
//! Only `/proc` says any of it, so only Linux has it, and only for the
//! processes the reader may look in: Joel's own. A build the Nix daemon
//! runs works in its own folder as its own user, and is counted nowhere.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// The ticks a second Linux's interface counts processor time in
/// (`USER_HZ`), the same on every architecture it runs on.
pub const TICKS: u64 = 100;

/// One process placed in a folder, as one reading saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// The checkout's or tree's folder it works in.
    pub folder: PathBuf,
    /// When it started, in ticks since the machine booted, which tells a
    /// process from a later one given the same id.
    pub started: u64,
    /// The processor time it has taken, in ticks.
    pub ticks: u64,
    /// Its share of memory, in bytes, where it could be read.
    pub memory: Option<u64>,
}

/// One reading: when, in ticks since boot, and each process it placed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sample {
    pub at: u64,
    pub processes: Vec<Process>,
}

/// What works in one folder.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct Use {
    /// How many processes work in it.
    pub processes: usize,
    /// Their memory, in bytes.
    pub memory: u64,
    /// Their share of one core between two readings, where there were two.
    pub cpu: Option<f64>,
}

/// The deepest of `folders` holding `folder`.
pub fn placed<'a>(folders: &'a [PathBuf], folder: &Path) -> Option<&'a Path> {
    let _ = (folders, folder);
    None
}

/// A reading of `proc`, Linux's `/proc`, keeping each process whose
/// working folder lies in one of `folders`; nothing where there's no
/// `/proc`, as on a Mac.
pub fn sample(proc: &Path, folders: &[PathBuf]) -> Sample {
    let _ = (proc, folders);
    Sample::default()
}

/// What works in each folder `now` placed a process in, its processor
/// time measured since `before`.
pub fn using(before: Option<&Sample>, now: &Sample) -> BTreeMap<PathBuf, Use> {
    let _ = (before, now);
    BTreeMap::new()
}

#[cfg(test)]
mod tests;
