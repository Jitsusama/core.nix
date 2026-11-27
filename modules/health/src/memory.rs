//! How much memory the programs Joel keeps many of, or keeps open all day,
//! hold, and how that grows with their age: each agent's pi session and
//! the shell. A program that only grows as it ages leaks, which no journal
//! line tells of until the machine runs short and swaps.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// The programs watched, by the last part of the path each was started by,
/// since the shell's own name in `/proc` is its wrapper's, cut short; and
/// the pi host and each conversation's own process, which are started by
/// the host's node, so they are known by the program it runs (`HOST`).
pub const WATCHED: [&str; 4] = ["pi", "quickshell", "pi-host", "pi-conversation"];

/// The harness's program a pi host's node runs, for the host and, with
/// `--child` after it, for each conversation's own process.
const HOST: &str = "pi-host.ts";

/// The ages copies are grouped by, in seconds: under an hour, under six
/// hours, under a day, and older.
pub const AGES: [u64; 3] = [3_600, 21_600, 86_400];

/// The clock ticks a second `/proc` counts a program's start in, which
/// Linux fixes at 100 for every program.
const TICKS: u64 = 100;

/// What one copy of a program held when looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held {
    /// In memory, in KiB: its own share, its Pss, where the kernel says
    /// it, so pages every copy shares, as node's own program, count once
    /// across them rather than once in each.
    pub kib: u64,
    /// Put out to swap, in KiB.
    pub swap_kib: u64,
    /// How long it had run, in seconds.
    pub age: u64,
    /// Whether `kib` is all it had resident, shared pages and all, since
    /// its share couldn't be read: on a Mac, where `ps` says only that,
    /// or for a process whose rollup this user may not read.
    pub by_rss: bool,
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

/// Ask `ps` for each watched program's copies, where there's no `/proc`
/// to look in, as on a Mac. Its copies put nothing out to swap, since `ps`
/// doesn't say what a process has there.
#[must_use]
pub fn asked() -> BTreeMap<String, Vec<Held>> {
    Command::new("/bin/ps")
        .args(["-axo", "rss=,etime=,comm="])
        .output()
        .ok()
        .filter(|said| said.status.success())
        .map(|said| from_ps(&String::from_utf8_lossy(&said.stdout)))
        .unwrap_or_default()
}

/// The watched programs' copies in what `ps -o rss=,etime=,comm=` said: a
/// line a process, its memory in KiB, how long it has run, and the path
/// it was started by, which may hold spaces.
fn from_ps(said: &str) -> BTreeMap<String, Vec<Held>> {
    let mut seen: BTreeMap<String, Vec<Held>> = BTreeMap::new();
    for line in said.lines() {
        let Some((kib, rest)) = line.trim_start().split_once(' ') else {
            continue;
        };
        let Some((ran, path)) = rest.trim_start().split_once(' ') else {
            continue;
        };
        let name = path.rsplit('/').next().unwrap_or(path);
        if !WATCHED.contains(&name) {
            continue;
        }
        let (Ok(kib), Some(age)) = (kib.parse::<u64>(), seconds_of(ran)) else {
            continue;
        };
        seen.entry(name.to_owned()).or_default().push(Held {
            kib,
            swap_kib: 0,
            age,
            by_rss: true,
        });
    }
    seen
}

/// The seconds in how long `ps` says a process has run, as `[[dd-]hh:]mm:ss`.
pub fn seconds_of(ran: &str) -> Option<u64> {
    let (days, clock) = ran.split_once('-').unwrap_or(("0", ran));
    let mut seconds = days.parse::<u64>().ok()? * 86_400;
    let parts: Vec<&str> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    for (part, worth) in parts.iter().rev().zip([1, 60, 3_600]) {
        seconds += part.parse::<u64>().ok()? * worth;
    }
    Some(seconds)
}

/// The last part of the path a process was started by; or, for a node
/// running the pi host's program, the host or a conversation's process.
/// A node is any program whose name starts so, since a runtime may name
/// its own by its version, and a conversation counted as some other
/// program is lost from its figures without a word. The host's program is
/// the first word after node's own flags, each one word as the host's
/// unit writes them, so a script naming it later is not the host, and
/// a `--child` counts only after it.
fn started_by(process: &Path) -> Option<String> {
    let started = fs::read(process.join("cmdline")).ok()?;
    let mut words = started
        .split(|&byte| byte == 0)
        .map(String::from_utf8_lossy);
    let first = words.next()?;
    let last_part = |word: &str| word.rsplit('/').next().unwrap_or(word).to_owned();
    let name = last_part(&first);
    if name.starts_with("node") {
        let mut after = words.skip_while(|word| word.starts_with('-'));
        if after
            .next()
            .is_some_and(|program| last_part(&program) == HOST)
        {
            let child = after.any(|word| word == "--child");
            return Some(if child { "pi-conversation" } else { "pi-host" }.to_owned());
        }
    }
    (!name.is_empty()).then_some(name)
}

/// The KiB `said` gives for `field`, a line of `/proc`'s `Field:  n kB`.
fn kib_in(said: &str, field: &str) -> Option<u64> {
    said.lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(|value| value.split_whitespace().next()?.parse::<u64>().ok())
}

/// What a process holds, by its own share and its swap from the rollup of
/// its mappings, or by its status where the rollup can't be read; and its
/// age, from when its stat says it started. A kernel thread, holding none
/// of its own, is passed over.
fn held(process: &Path, uptime: u64) -> Option<Held> {
    let status = fs::read_to_string(process.join("status")).ok()?;
    let rss = kib_in(&status, "VmRSS:")?;
    let rollup = fs::read_to_string(process.join("smaps_rollup")).unwrap_or_default();
    let (kib, swap_kib, by_rss) = match kib_in(&rollup, "Pss:") {
        Some(pss) => (pss, kib_in(&rollup, "Swap:").unwrap_or(0), false),
        None => (rss, kib_in(&status, "VmSwap:").unwrap_or(0), true),
    };
    let stat = fs::read_to_string(process.join("stat")).ok()?;
    // Fields after the name, which may hold spaces, closed by its bracket:
    // the start is the 22nd field, the 20th after it.
    let (_, after) = stat.rsplit_once(')')?;
    let started = after.split_whitespace().nth(19)?.parse::<u64>().ok()?;
    Some(Held {
        kib,
        swap_kib,
        age: uptime.saturating_sub(started / TICKS),
        by_rss,
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
    /// What the middle copy held, in memory and swap, in KiB, by nearest
    /// rank: a typical copy, which the sum of them all hides once there
    /// are a hundred. 0 when none was running.
    pub median_kib: u64,
    /// What the copy only one in twenty held more than held, the same way:
    /// a heavy copy. 0 when none was running.
    pub p95_kib: u64,
    /// The day's looks that saw a copy running.
    pub looks: u64,
    /// The sum of those looks' middle copies, so the day's typical copy
    /// is their mean.
    pub median_sum_kib: u64,
    /// The sum of those looks' heavy copies, the same way.
    pub p95_sum_kib: u64,
    /// The copies at the last look counted by their own share, Pss, rather
    /// than by all they had resident, as `Held::by_rss` says. A day kept
    /// before shares were read has none, as every copy then was by Rss.
    pub by_pss: u64,
    /// The day's looks that counted every copy they saw by its Pss, so
    /// the typical and heavy copy, means over the looks, say how much of
    /// them was by shares: fewer than `looks` on the day a machine moved
    /// to shares, or where some copy's share couldn't be read. None for a
    /// day kept with looks before this count was, whose looks' measure
    /// isn't known, rather than 0, which would say they were all by Rss.
    /// Left out of the file then, not written as null, which a build
    /// holding a plain count can't read, so it would start the day afresh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub looks_by_pss: Option<u64>,
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
        self.by_pss = copies.iter().filter(|held| !held.by_rss).count() as u64;
        self.kib = copies.iter().map(|held| held.kib).sum();
        self.swap_kib = copies.iter().map(|held| held.swap_kib).sum();
        self.largest_kib = copies.iter().map(Held::total).max().unwrap_or(0);
        self.oldest = copies.iter().map(|held| held.age).max().unwrap_or(0);
        let together = self.kib + self.swap_kib;
        if together > self.most_kib {
            self.most_kib = together;
            self.most_at = at;
        }
        let mut each: Vec<u64> = copies.iter().map(Held::total).collect();
        each.sort_unstable();
        self.median_kib = ranked(&each, 50);
        self.p95_kib = ranked(&each, 95);
        // A day with no looks yet has none uncounted, so its count starts.
        if self.looks == 0 && self.looks_by_pss.is_none() {
            self.looks_by_pss = Some(0);
        }
        if !each.is_empty() {
            self.looks += 1;
            self.median_sum_kib += self.median_kib;
            self.p95_sum_kib += self.p95_kib;
            if let Some(counted) = self.looks_by_pss.as_mut()
                && self.by_pss == self.copies
            {
                *counted += 1;
            }
        }
        self.by_age = vec![Aged::default(); AGES.len() + 1];
        for held in copies {
            let group = AGES.iter().take_while(|&&under| held.age >= under).count();
            self.by_age[group].copies += 1;
            self.by_age[group].kib += held.total();
        }
    }
}

/// What the last look counted its copies by: their own share (Pss), all
/// they had resident (Rss), or, when it was both, how many were by Rss,
/// in brackets so the count can't read as one of the line's sizes;
/// nothing when none was running. A kept day saying more copies by Pss
/// than it saw reads as all by Pss rather than a wrapped count.
#[must_use]
pub fn measure(held: &Memory) -> String {
    match (held.copies, held.by_pss) {
        (0, _) => String::new(),
        (_, 0) => "by Rss".to_owned(),
        (all, pss) if pss >= all => "by Pss".to_owned(),
        (all, pss) => format!("by Pss ({} by Rss)", all - pss),
    }
}

/// The value at `percent` of `sorted`, by nearest rank: the smallest that
/// at least that share of them are no larger than; none when empty.
fn ranked(sorted: &[u64], percent: usize) -> u64 {
    let rank = (sorted.len() * percent).div_ceil(100).max(1);
    sorted.get(rank - 1).copied().unwrap_or(0)
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
                age: 3_600,
                by_rss: true
            }],
            "pi, an hour old; the copy holding nothing of its own passed over"
        );
        assert_eq!(seen["quickshell"][0].age, 7_200, "the shell, by its path");
        assert_eq!(seen.len(), 2, "nothing else is watched: {seen:?}");
    }

    #[test]
    fn a_copy_is_counted_by_its_own_share_where_the_kernel_says_it() {
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        fs::write(proc.path().join("uptime"), "7200.55 14000.10\n").expect("uptime");
        process(proc.path(), 10, "pi\0", Some(1_000), 0);
        fs::write(
            proc.path().join("10/smaps_rollup"),
            concat!(
                "00400000-7fff00000000 ---p 00000000 00:00 0    [rollup]\n",
                "Rss:                1000 kB\n",
                "Pss:                 700 kB\n",
                "Pss_Anon:            650 kB\n",
                "Pss_File:             50 kB\n",
                "Swap:                 40 kB\n",
                "SwapPss:              30 kB\n",
            ),
        )
        .expect("its rollup");
        process(proc.path(), 11, "pi\0", Some(2_000), 0);
        process(proc.path(), 12, "pi\0", Some(3_000), 0);
        fs::write(proc.path().join("12/smaps_rollup"), "").expect("an empty rollup");
        let mut seen = look(proc.path()).remove("pi").expect("pi");
        seen.sort_unstable_by_key(|held| held.kib);
        assert_eq!(
            seen,
            vec![
                Held {
                    kib: 700,
                    swap_kib: 40,
                    age: 7_200,
                    by_rss: false
                },
                Held {
                    kib: 2_000,
                    swap_kib: 24,
                    age: 7_200,
                    by_rss: true
                },
                Held {
                    kib: 3_000,
                    swap_kib: 24,
                    age: 7_200,
                    by_rss: true
                },
            ],
            "by its rollup's Pss and Swap, not Pss_Anon or SwapPss; by its \
             status where the rollup can't be read or says nothing"
        );
    }

    #[test]
    fn a_hosts_conversations_and_the_host_are_found_by_the_program_they_run() {
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        fs::write(proc.path().join("uptime"), "7200.55 14000.10\n").expect("uptime");
        let node = "/nix/store/y-pi-host-runtime-1.5.0/libexec/pi-host/node";
        let flags = "--disable-sigusr1\0--max-old-space-size=4096";
        process(
            proc.path(),
            20,
            &format!("{node}\0{flags}\0scripts/pi-host.ts\0--child\0"),
            Some(300_000),
            0,
        );
        process(
            proc.path(),
            21,
            &format!("{node}\0{flags}\0/srv/harness/scripts/pi-host.ts\0--child\0"),
            Some(200_000),
            0,
        );
        process(
            proc.path(),
            22,
            &format!("{node}\0{flags}\0scripts/pi-host.ts\0realm\0"),
            Some(900_000),
            0,
        );
        process(
            proc.path(),
            23,
            &format!("{node}\0server.js\0"),
            Some(50),
            0,
        );
        process(
            proc.path(),
            24,
            &format!("{node}\0server.js\0--child\0"),
            Some(60),
            0,
        );
        process(
            proc.path(),
            25,
            &format!("{node}\0scripts/not-pi-host.ts\0--child\0"),
            Some(70),
            0,
        );
        process(
            proc.path(),
            26,
            "/nix/store/z-nodejs-22/bin/node22\0scripts/pi-host.ts\0--child\0",
            Some(100_000),
            0,
        );
        process(proc.path(), 27, "vim\0scripts/pi-host.ts\0", Some(80), 0);
        process(
            proc.path(),
            28,
            &format!("{node}\0scripts/tool.js\0scripts/pi-host.ts\0--child\0"),
            Some(90),
            0,
        );
        process(
            proc.path(),
            29,
            &format!("{node}\0--child\0scripts/pi-host.ts\0realm\0"),
            Some(400_000),
            0,
        );
        let seen = look(proc.path());
        // /proc lists its processes in no order a test can count on.
        let kib = |name: &str| -> Vec<u64> {
            let mut kib: Vec<u64> = seen[name].iter().map(|held| held.kib).collect();
            kib.sort_unstable();
            kib
        };
        assert_eq!(
            kib("pi-conversation"),
            vec![100_000, 200_000, 300_000],
            "each conversation's own process, by the host's program and --child, \
             whatever its node is called"
        );
        assert_eq!(
            kib("pi-host"),
            vec![400_000, 900_000],
            "the host, by its program alone, a --child before it being node's"
        );
        assert_eq!(
            seen.len(),
            2,
            "another node program, one named like the host's, or the host's \
             program open in an editor isn't pi: {seen:?}"
        );
    }

    #[test]
    fn the_watched_programs_are_found_in_what_ps_says() {
        let said = concat!(
            "  1000 01:00:00 /nix/store/x-pi/bin/pi\n",
            "443328 04-20:03:50 pi\n",
            "   512 05:07 /Users/someone/My Tools/pi\n",
            "  2048 00:09 /Applications/Some App.app/Contents/MacOS/Some App\n",
            "    20 00:09 /bin/bash\n",
            "a line ps never says\n",
        );
        let held = |kib, age| Held {
            kib,
            swap_kib: 0,
            age,
            by_rss: true,
        };
        let seen = from_ps(said);
        assert_eq!(
            seen.get("pi"),
            Some(&vec![
                held(1_000, 3_600),
                held(443_328, 4 * 86_400 + 20 * 3_600 + 3 * 60 + 50),
                held(512, 5 * 60 + 7),
            ]),
            "each pi, by its path or its bare name, with its days, hours and minutes"
        );
        assert_eq!(seen.len(), 1, "nothing else is watched: {seen:?}");
    }

    #[test]
    fn copies_are_kept_by_age_and_the_days_most() {
        let copy = |kib, age| Held {
            kib,
            swap_kib: 1,
            age,
            by_rss: false,
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
    fn a_typical_copy_and_a_heavy_one_are_kept_each_look_and_over_the_day() {
        let copy = |kib| Held {
            kib,
            swap_kib: kib / 10,
            age: 60,
            by_rss: false,
        };
        let mut memory = Memory::default();
        // Twenty-one copies, out of order, so the middle and the heavy one
        // fall between ranks and only a sort finds them.
        let out_of_order: Vec<Held> = (1..=21).map(|n| copy(((n + 6) % 21 + 1) * 10)).collect();
        memory.saw(1, &out_of_order);
        assert_eq!(
            (memory.median_kib, memory.p95_kib),
            (121, 220),
            "the eleventh of twenty-one and the twentieth, by nearest rank, each \
             in memory and swap together"
        );
        let mut even = Memory::default();
        even.saw(1, &(1..=20).map(|n| copy(n * 10)).collect::<Vec<_>>());
        assert_eq!(
            (even.median_kib, even.p95_kib),
            (110, 209),
            "the tenth of twenty and the nineteenth, where an index rounded \
             down would take the eleventh and the twentieth"
        );
        memory.saw(2, &[copy(20)]);
        memory.saw(3, &[]);
        assert_eq!(
            (memory.median_kib, memory.p95_kib),
            (0, 0),
            "none running now"
        );
        assert_eq!(
            (memory.looks, memory.median_sum_kib, memory.p95_sum_kib),
            (2, 143, 242),
            "a look with no copy running says nothing of a copy's size"
        );
    }

    #[test]
    fn the_last_look_counts_its_copies_by_pss_and_the_day_its_looks() {
        let copy = |by_rss| Held {
            kib: 10,
            swap_kib: 0,
            age: 60,
            by_rss,
        };
        let mut memory = Memory::default();
        memory.saw(1, &[copy(true), copy(false), copy(true)]);
        assert_eq!(memory.by_pss, 1, "one of three by Pss");
        memory.saw(2, &[copy(false)]);
        assert_eq!(memory.by_pss, 1, "the last look's, not the day's");
        assert_eq!(measure(&memory), "by Pss");
        memory.saw(3, &[]);
        assert_eq!(measure(&memory), "", "none running");
        let kept: Memory = serde_json::from_str(r#"{"copies": 2, "kib": 20}"#).expect("a day");
        assert_eq!(
            measure(&kept),
            "by Rss",
            "a day kept before shares were read"
        );
        memory.saw(3, &[copy(true), copy(true)]);
        assert_eq!(measure(&memory), "by Rss");
        memory.saw(4, &[copy(true), copy(false), copy(false)]);
        assert_eq!(measure(&memory), "by Pss (1 by Rss)");
        assert_eq!(
            (memory.looks, memory.looks_by_pss),
            (4, Some(1)),
            "only the look whose one copy was by Pss counted every copy by it"
        );
        let mut resumed: Memory =
            serde_json::from_str(r#"{"copies": 1, "by_pss": 1, "looks": 3}"#).expect("a day");
        resumed.saw(5, &[copy(false)]);
        assert_eq!(
            (resumed.looks, resumed.looks_by_pss),
            (4, None),
            "a day kept with looks before the count can't say how many were by Pss"
        );
        let overcounted: Memory =
            serde_json::from_str(r#"{"copies": 2, "by_pss": 5}"#).expect("a day");
        assert_eq!(
            measure(&overcounted),
            "by Pss",
            "more by Pss than copies reads as all, not a wrapped count"
        );
    }

    #[test]
    fn a_day_of_unknown_count_reads_back_on_a_build_that_counts_plainly() {
        /// The day as a build holding the count as a plain number reads it.
        #[derive(Debug, Default, Deserialize)]
        #[serde(default)]
        struct Plain {
            looks: u64,
            looks_by_pss: u64,
        }
        let unknown = Memory {
            looks: 3,
            ..Memory::default()
        };
        let written = serde_json::to_string(&unknown).expect("a day written");
        assert!(
            !written.contains("looks_by_pss"),
            "an unknown count is left out, not null: {written}"
        );
        let plain: Plain = serde_json::from_str(&written).expect("the plain build reads it");
        assert_eq!((plain.looks, plain.looks_by_pss), (3, 0));
        let counted = Memory {
            looks: 3,
            looks_by_pss: Some(2),
            ..Memory::default()
        };
        let written = serde_json::to_string(&counted).expect("a day written");
        let again: Memory = serde_json::from_str(&written).expect("read back");
        assert_eq!(
            again.looks_by_pss,
            Some(2),
            "a known count reads back: {written}"
        );
        let plain: Plain = serde_json::from_str(&written).expect("the plain build reads it");
        assert_eq!(plain.looks_by_pss, 2);
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
