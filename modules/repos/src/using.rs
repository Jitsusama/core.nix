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
//! is `utime` and `stime` in `stat`, with `cutime` and `cstime`, its
//! children's once it has waited on them, in the ticks of a hundredth of
//! a second Linux's interface always counts in. A build's compilers start
//! and end between two readings, so they're counted in the cargo that
//! waited on them, while it's placed in a folder at both readings.
//!
//! A child a reading saw was counted as it worked, and when it's waited
//! on the kernel moves its whole time into its parent, so what the
//! reading last saw of it is taken off the children's time of the nearest
//! of its ancestors still there, and only that, so the ancestor's own
//! work stands. That holds wherever the child worked, since every
//! process's `stat` is read, a folder or not, even one like `sudo` whose
//! working folder the reader may not look in. A child is gone only once
//! its id is: one that moved out of every folder, or has ended and isn't
//! yet waited on, is still there. What's left: a child waited on between
//! reading it and reading its parent, in a walk of under a second, is
//! counted twice in that reading, and one handed to another process to
//! wait on since the last reading is taken off the wrong ancestor, which
//! reads low for a minute.
//!
//! Time reaches a folder only through a process placed in it at both
//! readings, so some is never counted. A process working in no folder,
//! whether out of every tree or one like `sudo` the reader may not look
//! in, is counted for none of the time it works there, save what it took
//! since the last reading once a placed parent waits on it. And one
//! waited on by a process in no folder, as the user's service manager or
//! a terminal's server, loses what it took since the last reading, while
//! one that starts and ends between readings under such a parent is
//! counted nowhere.
//!
//! A folder's is a share of one core over the waking time between two
//! readings, no more than `LONGEST` apart, each read whole within `EVEN`: a process
//! new since the first is counted from its start, and one that only moved
//! in since isn't counted, as nothing says how long it has worked there.
//! A process whose working folder was removed works nowhere.
//!
//! Only `/proc` says any of it, so only Linux has it, and only for the
//! processes the reader may look in: Joel's own. A build the Nix daemon
//! runs works in its own folder as its own user, and is counted nowhere.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use serde::Serialize;

/// The ticks a second Linux's interface counts processor time in
/// (`USER_HZ`), the same on every architecture it runs on.
pub const TICKS: u64 = 100;

/// The furthest apart two readings may be and still measure a share,
/// three of the daemon's waking minutes, so a share is of one recent span
/// and not of the time since one long gone.
pub const LONGEST: u64 = 3 * 60 * TICKS;

/// The longest a reading may take to walk `/proc` and still measure a
/// share, two seconds: one held up by a process's memory map reads those
/// after it late, so its time isn't theirs.
pub const EVEN: u64 = 2 * TICKS;

/// Whether `now` measures processor time since `before`: both were read
/// whole within `EVEN`, and some waking time, no more than `LONGEST`,
/// passed between them.
pub fn measured(before: Option<&Sample>, now: &Sample) -> bool {
    between(before, now).is_some()
}

/// The waking ticks between two readings, where they measure a share.
fn between(before: Option<&Sample>, now: &Sample) -> Option<u64> {
    let (before, now) = (before?.awake?, now.awake?);
    Some(now.saturating_sub(before)).filter(|between| (1..=LONGEST).contains(between))
}

/// The waking time since the daemon first asked, in ticks. Rust's `Instant`
/// is Linux's monotonic clock, which stops while the machine sleeps, as
/// processor time does; the time since boot runs on.
fn awake() -> u64 {
    static FIRST: OnceLock<Instant> = OnceLock::new();
    let elapsed = FIRST.get_or_init(Instant::now).elapsed();
    u64::try_from(elapsed.as_millis() / u128::from(1000 / TICKS)).unwrap_or(u64::MAX)
}

/// One process as one reading saw it, placed in a folder or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// The process that started it, or took it on, which its time moves
    /// into once it has waited on it.
    pub parent: u32,
    /// The checkout's or tree's folder it works in, where it works in one
    /// and the reader may look.
    pub folder: Option<PathBuf>,
    /// When it started, in ticks since the machine booted, which tells a
    /// process from a later one given the same id.
    pub started: u64,
    /// The processor time it has taken itself, in ticks.
    pub own: u64,
    /// The processor time of the children it has waited on, in ticks.
    pub children: u64,
    /// Its share of memory, in bytes, where it's placed and could be read.
    pub memory: Option<u64>,
}

impl Process {
    /// All the processor time it holds, its own and its children's.
    pub fn ticks(&self) -> u64 {
        self.own + self.children
    }
}

/// One reading: when, and each process it read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sample {
    /// In ticks since boot, as each process's start is said.
    pub at: u64,
    /// In waking ticks (`awake`), where it was read whole within `EVEN`.
    pub awake: Option<u64>,
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

/// The process whose memory a reading is reading, 0 between them. A read
/// of `smaps_rollup` waits while the process holds its memory map, as one
/// faulting on a mount that hung may for good, and this names it.
static READING: AtomicU32 = AtomicU32::new(0);

/// The process whose memory a reading is held on, where one is.
pub fn reading() -> Option<u32> {
    Some(READING.load(Ordering::Relaxed)).filter(|pid| *pid != 0)
}

/// A reading of `proc`, Linux's `/proc`, of every process, placing each
/// whose working folder lies in one of `folders`; nothing where there's no
/// `/proc`, as on a Mac.
pub fn sample(proc: &Path, folders: &[PathBuf]) -> Sample {
    sample_by(proc, folders, awake)
}

/// A reading of `proc` timed by `clock`, in waking ticks.
fn sample_by(proc: &Path, folders: &[PathBuf], mut clock: impl FnMut() -> u64) -> Sample {
    let Some(at) = fs::read_to_string(proc.join("uptime"))
        .ok()
        .and_then(|said| uptime(&said))
    else {
        return Sample::default();
    };
    let began = clock();
    let Ok(entries) = fs::read_dir(proc) else {
        return Sample::default();
    };
    // Every process's stat is read, as any reaped child's time may land
    // in one working in a folder, but only one placed in a folder has its
    // memory read, since that costs the kernel a walk of its pages and most
    // of the machine's processes work nowhere in a tree.
    let processes = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse().ok()?;
            let path = entry.path();
            let (parent, started, own, children) =
                stat(&fs::read_to_string(path.join("stat")).ok()?)?;
            let folder = fs::read_link(path.join("cwd"))
                .ok()
                .filter(|working| !working.to_string_lossy().ends_with(" (deleted)"))
                .and_then(|working| placed(folders, &working).map(Path::to_path_buf));
            let memory = folder.as_ref().and_then(|_| {
                READING.store(pid, Ordering::Relaxed);
                let said = fs::read_to_string(path.join("smaps_rollup"));
                READING.store(0, Ordering::Relaxed);
                said.ok().and_then(|said| shared(&said))
            });
            Some(Process {
                pid,
                parent,
                folder,
                started,
                own,
                children,
                memory,
            })
        })
        .collect();
    let walked = clock().saturating_sub(began);
    Sample {
        at,
        awake: (walked <= EVEN).then_some(began),
        processes,
    }
}

/// The time since boot `/proc/uptime` gives first, in ticks: its seconds
/// and their hundredths, read as written rather than through a float.
fn uptime(said: &str) -> Option<u64> {
    let first = said.split_whitespace().next()?;
    let (whole, part) = first.split_once('.').unwrap_or((first, ""));
    let hundredths: String = part.chars().chain("00".chars()).take(2).collect();
    Some(whole.parse::<u64>().ok()? * TICKS + hundredths.parse::<u64>().ok()?)
}

/// A process's parent, when it started, the processor time it has taken
/// and its waited-on children's, from its `stat`: fields 4, 22, 14 and 15,
/// and 16 and 17, counted from after its name, which is read to its last
/// bracket since a name may hold one.
fn stat(said: &str) -> Option<(u32, u64, u64, u64)> {
    let (_, rest) = said.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // Field 3, its state, is the first after the name.
    let field = |number: usize| fields.get(number - 3)?.parse::<u64>().ok();
    Some((
        u32::try_from(field(4)?).ok()?,
        field(22)?,
        field(14)? + field(15)?,
        field(16)? + field(17)?,
    ))
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
    // Only one placed then too: one that moved in since worked elsewhere.
    let earlier: Option<HashMap<(u32, u64), &Process>> = before.map(|before| {
        let placed = before
            .processes
            .iter()
            .filter(|process| process.folder.is_some());
        placed
            .map(|process| ((process.pid, process.started), process))
            .collect()
    });
    let counted = before
        .map(|before| counted_already(before, now))
        .unwrap_or_default();
    let mut using: BTreeMap<PathBuf, (Use, u64)> = BTreeMap::new();
    for process in &now.processes {
        let Some(path) = &process.folder else {
            continue;
        };
        let (folder, worked) = using.entry(path.clone()).or_default();
        folder.processes += 1;
        folder.memory += process.memory.unwrap_or(0);
        if let (Some(before), Some(earlier)) = (before, &earlier) {
            let key = (process.pid, process.started);
            let counted = counted.get(&key).copied().unwrap_or(0);
            *worked += match earlier.get(&key) {
                Some(then) => {
                    process.own.saturating_sub(then.own)
                        + process
                            .children
                            .saturating_sub(then.children)
                            .saturating_sub(counted)
                }
                None if process.started >= before.at => {
                    process.own + process.children.saturating_sub(counted)
                }
                None => 0,
            };
        }
    }
    let between = between(before, now);
    using
        .into_iter()
        .map(|(path, (mut folder, worked))| {
            folder.cpu = between.map(|between| share(worked, between));
            (path, folder)
        })
        .collect()
}

/// The ticks each process still there took in from children `before` saw,
/// now gone: each gone one's last-seen time, taken from the nearest ancestor
/// still there, through any gone between. A child is gone only once no
/// process has its id and start, since one ended but not yet waited on, or
/// working outside every folder, hasn't given its parent its time.
fn counted_already(before: &Sample, now: &Sample) -> HashMap<(u32, u64), u64> {
    let working: HashSet<(u32, u64)> = now
        .processes
        .iter()
        .map(|process| (process.pid, process.started))
        .collect();
    let seen: HashMap<u32, &Process> = before
        .processes
        .iter()
        .map(|process| (process.pid, process))
        .collect();
    let mut counted = HashMap::new();
    for gone in &before.processes {
        if working.contains(&(gone.pid, gone.started)) {
            continue;
        }
        // One reading's parents can't loop, but its length bounds the walk.
        let mut parent = gone.parent;
        for _ in 0..before.processes.len() {
            let Some(ancestor) = seen.get(&parent) else {
                break;
            };
            let key = (ancestor.pid, ancestor.started);
            if working.contains(&key) {
                *counted.entry(key).or_insert(0) += gone.ticks();
                break;
            }
            parent = ancestor.parent;
        }
    }
    counted
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
