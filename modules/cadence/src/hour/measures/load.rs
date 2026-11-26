//! How loaded the machine was this hour, minute by minute, against the
//! limits velocity set (25 November) for Joel's "keep this laptop lively
//! but never heavily loaded": his app and session slices barely waiting,
//! the agents' slice waiting little for a CPU, the whole machine short
//! of no resource for long, and memory left over. lag-watch keeps a
//! window every ten seconds with each slice's stalls, the machine's and
//! the memory left; six of them make each minute judged here.

use std::collections::BTreeMap;
use std::path::Path;

use jiff::Timestamp;
use serde_json::Value;

use super::super::lines::figure;
use super::super::records::tail;
use super::{Class, Measure};

/// What the measure is known by from one hour to the next.
pub const KEY: &str = "load-over";

/// Minutes past a limit an hour may hold: none.
pub const TARGET: f64 = 0.0;

/// Who set the limits and orders the load: velocity, 25 November.
const OWNER: &str = "A210E4CA";

/// Where lag-watch keeps its windows, a file a day by the UTC date.
const FOLDER: &str = "lag-watch";

/// How much of a day's file is read from its end: an hour's 360 windows
/// with room over, so a long day isn't read whole.
const READ: u64 = 4 << 20;

/// Minutes an hour must have whole to be judged: three quarters, so a
/// few missed windows are forgiven and an hour mostly unwatched, asleep
/// or with lag-watch stopped, is said unread rather than passing.
const READ_ENOUGH: u32 = 45;

/// Seconds of windows that make a minute watched.
const WATCHED: f64 = 30.0;

/// The memory the machine must keep left, in GiB.
const LEAST_GIB: f64 = 16.0;

const SAYS: &str = "minutes the machine was loaded past his limits, this hour";

/// Which of a slice's stalls a limit is on.
#[derive(Clone, Copy)]
enum Stall {
    CpuSome,
    MemoryFull,
    IoFull,
}

impl Stall {
    const fn field(self) -> &'static str {
        match self {
            Self::CpuSome => "cpu_some",
            Self::MemoryFull => "memory_full",
            Self::IoFull => "io_full",
        }
    }
}

/// One limit: lag-watch's name for the group, what the card calls it,
/// the stall, and the most it may be, in percent of the minute.
struct Limit {
    group: &'static str,
    said: &'static str,
    stall: Stall,
    most: f64,
}

const LIMITS: [Limit; 8] = [
    Limit {
        group: "app",
        said: "his apps waiting for a CPU",
        stall: Stall::CpuSome,
        most: 2.0,
    },
    Limit {
        group: "app",
        said: "his apps all stalled on memory",
        stall: Stall::MemoryFull,
        most: 0.5,
    },
    Limit {
        group: "session",
        said: "his desktop waiting for a CPU",
        stall: Stall::CpuSome,
        most: 2.0,
    },
    Limit {
        group: "session",
        said: "his desktop all stalled on memory",
        stall: Stall::MemoryFull,
        most: 0.5,
    },
    Limit {
        group: "agents",
        said: "the agents waiting for a CPU",
        stall: Stall::CpuSome,
        most: 15.0,
    },
    Limit {
        group: "machine",
        said: "the machine waiting for a CPU",
        stall: Stall::CpuSome,
        most: 40.0,
    },
    Limit {
        group: "machine",
        said: "the machine all stalled on memory",
        stall: Stall::MemoryFull,
        most: 5.0,
    },
    Limit {
        group: "machine",
        said: "the machine all stalled on the disk",
        stall: Stall::IoFull,
        most: 2.0,
    },
];

/// One minute's windows: how long they ran, each limit's stall weighted
/// by its window's length with how long it was read, and the least memory
/// left any of them saw, in MiB.
#[derive(Default)]
struct Minute {
    seconds: f64,
    stalled: [f64; LIMITS.len()],
    read: [f64; LIMITS.len()],
    least_mib: Option<f64>,
}

impl Minute {
    fn add(&mut self, window: &Value, seconds: f64) {
        self.seconds += seconds;
        for (i, limit) in LIMITS.iter().enumerate() {
            if let Some(share) = window["stalls"][limit.group][limit.stall.field()].as_f64() {
                self.stalled[i] += share * seconds;
                self.read[i] += seconds;
            }
        }
        if let Some(mib) = window["available_mib"].as_f64() {
            self.least_mib = Some(self.least_mib.map_or(mib, |least| least.min(mib)));
        }
    }

    /// Each limit's share of the minute, or none for one read in under
    /// half of it.
    fn shares(&self) -> [Option<f64>; LIMITS.len()] {
        std::array::from_fn(|i| {
            (self.read[i] >= self.seconds / 2.0).then(|| self.stalled[i] / self.read[i])
        })
    }

    fn whole(&self) -> bool {
        self.seconds >= WATCHED
            && self.least_mib.is_some()
            && self.shares().iter().all(Option::is_some)
    }
}

/// The hour to `now` judged from lag-watch's windows beside `state_home`.
#[must_use]
pub fn hour(state_home: &Path, now: i64) -> Measure {
    load(&read(state_home, now), now)
}

/// lag-watch's windows in the hour to `now`, from the end of each day's
/// file the hour touches, today's and, early in the day, yesterday's.
#[must_use]
pub fn read(state_home: &Path, now: i64) -> String {
    let mut days: Vec<String> = [now - 3600, now]
        .iter()
        .filter_map(|&at| Timestamp::from_second(at).ok())
        .map(|at| at.strftime("%Y-%m-%d").to_string())
        .collect();
    days.dedup();
    let folder = state_home.join(FOLDER);
    days.iter()
        .filter_map(|day| tail(&folder.join(format!("{day}.jsonl")), READ))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The hour to `now` judged from lag-watch's windows in `text`: the
/// minutes past any limit, each limit past named under it with its
/// minutes and its worst. A window is put in the minute it ended in, so
/// six make one; an hour with fewer than `READ_ENOUGH` minutes whole, its
/// windows, each limit's group and the memory left all read, is said
/// unread, naming what fell short, and never judged.
#[must_use]
pub fn load(text: &str, now: i64) -> Measure {
    let mut minutes: BTreeMap<i64, Minute> = BTreeMap::new();
    for line in text.lines() {
        let Ok(window) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(ended) = window["ended"].as_i64() else {
            continue;
        };
        let seconds = window["seconds"].as_f64().unwrap_or(0.0);
        if ended <= now - 3600 || ended > now || seconds <= 0.0 {
            continue;
        }
        minutes
            .entry(ended.div_euclid(60))
            .or_default()
            .add(&window, seconds);
    }
    let whole: Vec<&Minute> = minutes.values().filter(|m| m.whole()).collect();
    if let Some(why) = short(&minutes, whole.len()) {
        return Measure {
            key: KEY.to_owned(),
            class: Class::Apps,
            says: format!("{SAYS}, unread: {why}"),
            now: 0.0,
            target: None,
            owner: OWNER.to_owned(),
            behind: Vec::new(),
        };
    }
    let mut past = [(0_u32, 0.0_f64); LIMITS.len()];
    let mut short_of_memory = (0_u32, f64::INFINITY);
    let mut over = 0_u32;
    for each in whole {
        let mut any = false;
        for (i, share) in each.shares().into_iter().enumerate() {
            if let Some(share) = share.filter(|&share| share > LIMITS[i].most) {
                past[i] = (past[i].0 + 1, past[i].1.max(share));
                any = true;
            }
        }
        let left = each.least_mib.unwrap_or(f64::INFINITY) / 1024.0;
        if left < LEAST_GIB {
            short_of_memory = (short_of_memory.0 + 1, short_of_memory.1.min(left));
            any = true;
        }
        over += u32::from(any);
    }
    let mut behind: Vec<(u32, String)> = LIMITS
        .iter()
        .zip(past)
        .filter(|(_, (spent, _))| *spent > 0)
        .map(|(limit, (spent, worst))| {
            let line = format!(
                "{} over {}%: {spent} min, worst {}%",
                limit.said,
                figure(limit.most),
                figure(worst)
            );
            (spent, line)
        })
        .collect();
    if short_of_memory.0 > 0 {
        let (spent, least) = short_of_memory;
        let line = format!(
            "memory left under {} GiB: {spent} min, least {} GiB",
            figure(LEAST_GIB),
            figure(least)
        );
        behind.push((spent, line));
    }
    behind.sort_by_key(|a| std::cmp::Reverse(a.0));
    Measure {
        key: KEY.to_owned(),
        class: Class::Apps,
        says: SAYS.to_owned(),
        now: f64::from(over),
        target: Some(TARGET),
        owner: OWNER.to_owned(),
        behind: behind.into_iter().map(|(_, line)| line).collect(),
    }
}

/// Why an hour with `whole` minutes whole of `minutes` can't be judged,
/// or none when it can: too few minutes watched at all, else the limit
/// read in fewest, else the memory left.
fn short(minutes: &BTreeMap<i64, Minute>, whole: usize) -> Option<String> {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    if count(whole) >= READ_ENOUGH {
        return None;
    }
    let watched: Vec<&Minute> = minutes.values().filter(|m| m.seconds >= WATCHED).collect();
    if count(watched.len()) < READ_ENOUGH {
        return Some(format!(
            "{} of 60 minutes watched by lag-watch",
            watched.len()
        ));
    }
    let fewest = (0..LIMITS.len())
        .map(|i| {
            (
                watched.iter().filter(|m| m.shares()[i].is_some()).count(),
                i,
            )
        })
        .min();
    if let Some((read, i)) = fewest.filter(|&(read, _)| count(read) < READ_ENOUGH) {
        return Some(format!("{} read in {read} of 60 minutes", LIMITS[i].said));
    }
    let left = watched.iter().filter(|m| m.least_mib.is_some()).count();
    Some(format!("the memory left read in {left} of 60 minutes"))
}

#[cfg(test)]
mod tests;
