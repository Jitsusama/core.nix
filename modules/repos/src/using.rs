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

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// The ticks a second Linux's interface counts processor time in
/// (`USER_HZ`), the same on every architecture it runs on.
pub const TICKS: u64 = 100;

/// The furthest apart two readings may be and still measure a share,
/// three of the daemon's minutes.
pub const LONGEST: u64 = 3 * 60 * TICKS;

/// Whether `now` measures processor time since `before`.
pub fn measured(before: Option<&Sample>, now: &Sample) -> bool {
    before.is_some_and(|before| now.at > before.at)
}

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
    folders
        .iter()
        .filter(|candidate| folder.starts_with(candidate))
        .max_by_key(|candidate| candidate.components().count())
        .map(PathBuf::as_path)
}

/// A reading of `proc`, Linux's `/proc`, keeping each process whose
/// working folder lies in one of `folders`; nothing where there's no
/// `/proc`, as on a Mac.
pub fn sample(proc: &Path, folders: &[PathBuf]) -> Sample {
    let Some(at) = fs::read_to_string(proc.join("uptime"))
        .ok()
        .and_then(|said| uptime(&said))
    else {
        return Sample::default();
    };
    let Ok(entries) = fs::read_dir(proc) else {
        return Sample::default();
    };
    // A process's working folder is read first, and only one placed in a
    // folder is read further, since its memory costs the kernel a walk of
    // its pages and most of the machine's processes work nowhere in a tree.
    let processes = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse().ok()?;
            let own = entry.path();
            let working = fs::read_link(own.join("cwd")).ok()?;
            let folder = placed(folders, &working)?.to_path_buf();
            let (started, ticks) = stat(&fs::read_to_string(own.join("stat")).ok()?)?;
            let memory = fs::read_to_string(own.join("smaps_rollup"))
                .ok()
                .and_then(|said| shared(&said));
            Some(Process {
                pid,
                folder,
                started,
                ticks,
                memory,
            })
        })
        .collect();
    Sample { at, processes }
}

/// The time since boot `/proc/uptime` gives first, in ticks: its seconds
/// and their hundredths, read as written rather than through a float.
fn uptime(said: &str) -> Option<u64> {
    let first = said.split_whitespace().next()?;
    let (whole, part) = first.split_once('.').unwrap_or((first, ""));
    let hundredths: String = part.chars().chain("00".chars()).take(2).collect();
    Some(whole.parse::<u64>().ok()? * TICKS + hundredths.parse::<u64>().ok()?)
}

/// When a process started and the processor time it has taken, from its
/// `stat`: fields 22, and 14 and 15, counted from after its name, which is
/// read to its last bracket since a name may hold one.
fn stat(said: &str) -> Option<(u64, u64)> {
    let (_, rest) = said.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // Field 3, its state, is the first after the name.
    let field = |number: usize| fields.get(number - 3)?.parse::<u64>().ok();
    Some((field(22)?, field(14)? + field(15)?))
}

/// A process's share of memory, in bytes, from its `smaps_rollup`.
fn shared(said: &str) -> Option<u64> {
    said.lines().find_map(|line| {
        let kilobytes = line.strip_prefix("Pss:")?.trim().strip_suffix("kB")?.trim();
        kilobytes
            .parse::<u64>()
            .ok()
            .map(|kilobytes| kilobytes * 1024)
    })
}

/// What works in each folder `now` placed a process in, its processor
/// time measured since `before`.
pub fn using(before: Option<&Sample>, now: &Sample) -> BTreeMap<PathBuf, Use> {
    let earlier: Option<HashMap<(u32, u64), u64>> = before.map(|before| {
        let each = before.processes.iter();
        each.map(|process| ((process.pid, process.started), process.ticks))
            .collect()
    });
    let mut using: BTreeMap<PathBuf, (Use, u64)> = BTreeMap::new();
    for process in &now.processes {
        let (folder, worked) = using.entry(process.folder.clone()).or_default();
        folder.processes += 1;
        folder.memory += process.memory.unwrap_or(0);
        if let (Some(before), Some(earlier)) = (before, &earlier) {
            *worked += match earlier.get(&(process.pid, process.started)) {
                Some(ticks) => process.ticks.saturating_sub(*ticks),
                None if process.started >= before.at => process.ticks,
                None => 0,
            };
        }
    }
    let between = before
        .map(|before| now.at.saturating_sub(before.at))
        .filter(|at| *at > 0);
    using
        .into_iter()
        .map(|(path, (mut folder, worked))| {
            folder.cpu = between.map(|between| share(worked, between));
            (path, folder)
        })
        .collect()
}

/// `worked` ticks of `between` as a share of one core.
#[expect(
    clippy::cast_precision_loss,
    reason = "ticks between two readings a minute apart are far below 2^52"
)]
fn share(worked: u64, between: u64) -> f64 {
    worked as f64 / between as f64
}

#[cfg(test)]
mod tests;
