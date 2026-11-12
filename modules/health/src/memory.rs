//! How much memory the programs Joel keeps many of, or keeps open all day,
//! hold, and how that grows with their age: each agent's pi session and
//! the shell. A program that only grows as it ages leaks, which no journal
//! line tells of until the machine runs short and swaps.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The programs watched, by the last part of the path each was started by,
/// since the shell's own name in `/proc` is its wrapper's, cut short.
pub const WATCHED: [&str; 2] = ["pi", "quickshell"];

/// The ages copies are grouped by, in seconds: under an hour, under six
/// hours, under a day, and older.
pub const AGES: [u64; 3] = [3_600, 21_600, 86_400];

/// The clock ticks a second `/proc` counts a program's start in, which
/// Linux fixes at 100 for every program.
const TICKS: u64 = 100;

/// What one copy of a program held when looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held {
    /// In memory, in KiB.
    pub kib: u64,
    /// Put out to swap, in KiB.
    pub swap_kib: u64,
    /// How long it had run, in seconds.
    pub age: u64,
}

impl Held {
    fn total(&self) -> u64 {
        self.kib + self.swap_kib
    }
}

/// Look in `proc` at each watched program's copies.
#[must_use]
pub fn look(proc: &Path) -> BTreeMap<String, Vec<Held>> {
    let uptime = fs::read_to_string(proc.join("uptime"))
        .ok()
        .and_then(|said| said.split(['.', ' ']).next()?.parse::<u64>().ok())
        .unwrap_or(0);
    let mut seen: BTreeMap<String, Vec<Held>> = BTreeMap::new();
    let copies = fs::read_dir(proc)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|process| {
            let path = process.path();
            let name = started_by(&path).filter(|name| WATCHED.contains(&name.as_str()))?;
            Some((name, held(&path, uptime)?))
        });
    for (name, held) in copies {
        seen.entry(name).or_default().push(held);
    }
    seen
}

/// The last part of the path a process was started by.
fn started_by(process: &Path) -> Option<String> {
    let started = fs::read(process.join("cmdline")).ok()?;
    let first = started.split(|&byte| byte == 0).next()?;
    let path = String::from_utf8_lossy(first);
    let name = path.rsplit('/').next()?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// What a process holds, from its status, and its age, from when its stat
/// says it started; a kernel thread, holding none of its own, is passed
/// over.
fn held(process: &Path, uptime: u64) -> Option<Held> {
    let status = fs::read_to_string(process.join("status")).ok()?;
    let kib_of = |field: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(field))
            .and_then(|said| said.split_whitespace().next()?.parse::<u64>().ok())
    };
    let stat = fs::read_to_string(process.join("stat")).ok()?;
    // Fields after the name, which may hold spaces, closed by its bracket:
    // the start is the 22nd field, the 20th after it.
    let (_, after) = stat.rsplit_once(')')?;
    let started = after.split_whitespace().nth(19)?.parse::<u64>().ok()?;
    Some(Held {
        kib: kib_of("VmRSS:")?,
        swap_kib: kib_of("VmSwap:").unwrap_or(0),
        age: uptime.saturating_sub(started / TICKS),
    })
}

/// A watched program's memory over the day: the last look, and the most
/// its copies held together.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Memory {
    /// When it was last looked at, in microseconds since the epoch.
    pub at: i64,
    /// Its copies running then.
    pub copies: u64,
    /// What they held in memory, in KiB.
    pub kib: u64,
    /// What they had put out to swap, in KiB.
    pub swap_kib: u64,
    /// What the largest copy held, in memory and swap, in KiB.
    pub largest_kib: u64,
    /// How long the oldest copy had run, in seconds.
    pub oldest: u64,
    /// The most the copies held together in the day, in memory and swap.
    pub most_kib: u64,
    /// When they held it.
    pub most_at: i64,
    /// The copies by age, as `AGES` groups them, youngest first.
    pub by_age: Vec<Aged>,
}

/// The copies of an age, and what they held in memory and swap, in KiB.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Aged {
    pub copies: u64,
    pub kib: u64,
}

impl Memory {
    /// Keep what a look at `at` saw of the program's copies.
    pub fn saw(&mut self, at: i64, copies: &[Held]) {
        self.at = at;
        self.copies = copies.len() as u64;
        self.kib = copies.iter().map(|held| held.kib).sum();
        self.swap_kib = copies.iter().map(|held| held.swap_kib).sum();
        self.largest_kib = copies.iter().map(Held::total).max().unwrap_or(0);
        self.oldest = copies.iter().map(|held| held.age).max().unwrap_or(0);
        let together = self.kib + self.swap_kib;
        if together > self.most_kib {
            self.most_kib = together;
            self.most_at = at;
        }
        self.by_age = vec![Aged::default(); AGES.len() + 1];
        for held in copies {
            let group = AGES.iter().take_while(|&&under| held.age >= under).count();
            self.by_age[group].copies += 1;
            self.by_age[group].kib += held.total();
        }
    }
}

/// An amount of memory in KiB, in plain words: MiB under a GiB, else GiB
/// to a tenth.
#[must_use]
pub fn size(kib: u64) -> String {
    if kib < 1 << 20 {
        return format!("{} MiB", kib >> 10);
    }
    let tenths = (kib * 10) >> 20;
    format!("{}.{} GiB", tenths / 10, tenths % 10)
}

/// An age in seconds, in plain words: minutes under an hour, else hours.
#[must_use]
pub fn age(seconds: u64) -> String {
    if seconds < 3_600 {
        format!("{} min", seconds / 60)
    } else {
        format!("{} h", seconds / 3_600)
    }
}

/// What an age group is called, by its place among `AGES`.
#[must_use]
pub fn aged(group: usize) -> String {
    match (
        group.checked_sub(1).map(|before| AGES[before]),
        AGES.get(group),
    ) {
        (None, Some(&under)) => format!("under {}", age(under)),
        (Some(from), Some(&under)) => format!("{} to {}", age(from), age(under)),
        (Some(from), None) => format!("over {}", age(from)),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process in a stand-in `/proc`, started `started` ticks after boot.
    fn process(proc: &Path, pid: u32, started_by: &str, kib: Option<u64>, started: u64) {
        let path = proc.join(pid.to_string());
        fs::create_dir_all(&path).expect("a process");
        fs::write(path.join("cmdline"), started_by).expect("its command");
        let held = kib.map_or(String::new(), |kib| {
            format!("VmRSS:\t {kib} kB\nVmSwap:\t 24 kB\n")
        });
        fs::write(
            path.join("status"),
            format!("Name:\tx\n{held}Threads:\t1\n"),
        )
        .expect("status");
        let fields = format!("S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 {started} 0 0");
        fs::write(path.join("stat"), format!("{pid} (a name) {fields}\n")).expect("stat");
    }

    #[test]
    fn the_watched_programs_are_found_in_proc() {
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        fs::write(proc.path().join("uptime"), "7200.55 14000.10\n").expect("uptime");
        process(proc.path(), 10, "pi\0--resume\0", Some(1_000), 360_000);
        process(
            proc.path(),
            11,
            "/nix/store/x-quickshell/bin/quickshell\0-p\0",
            Some(500),
            0,
        );
        process(proc.path(), 12, "bash\0", Some(9), 0);
        process(proc.path(), 13, "", None, 0);
        process(proc.path(), 14, "pi\0", None, 0);
        let seen = look(proc.path());
        assert_eq!(
            seen["pi"],
            vec![Held {
                kib: 1_000,
                swap_kib: 24,
                age: 3_600
            }],
            "pi, an hour old; the copy holding nothing of its own passed over"
        );
        assert_eq!(seen["quickshell"][0].age, 7_200, "the shell, by its path");
        assert_eq!(seen.len(), 2, "nothing else is watched: {seen:?}");
    }

    #[test]
    fn copies_are_kept_by_age_and_the_days_most() {
        let copy = |kib, age| Held {
            kib,
            swap_kib: 1,
            age,
        };
        let mut memory = Memory::default();
        memory.saw(1, &[copy(10, 60), copy(20, 7_200), copy(30, 90_000)]);
        memory.saw(2, &[copy(10, 120)]);
        assert_eq!(memory.copies, 1, "the last look");
        assert_eq!((memory.most_kib, memory.most_at), (63, 1), "the day's most");
        assert_eq!(memory.largest_kib, 11, "the largest copy now");
        memory.saw(3, &[copy(10, 60), copy(20, 7_200), copy(30, 90_000)]);
        assert_eq!(
            memory.by_age,
            vec![
                Aged { copies: 1, kib: 11 },
                Aged { copies: 1, kib: 21 },
                Aged::default(),
                Aged { copies: 1, kib: 31 }
            ],
            "under an hour, to six, to a day, and older"
        );
        assert_eq!(
            memory.most_at, 1,
            "held as much, not more, so kept as first"
        );
    }

    #[test]
    fn memory_and_age_are_said_in_plain_words() {
        assert_eq!(size(961_536), "939 MiB");
        assert_eq!(size(29_465_395), "28.1 GiB");
        assert_eq!(age(1_800), "30 min");
        assert_eq!(age(79_200), "22 h");
        assert_eq!(aged(0), "under 1 h");
        assert_eq!(aged(1), "1 h to 6 h");
        assert_eq!(aged(3), "over 24 h");
    }
}
