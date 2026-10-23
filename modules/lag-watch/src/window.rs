//! A window of time between two snapshots: how much of it each slice
//! spent stalled, what its memory lost, how long the watched threads
//! waited for a CPU, and how long niri took to answer. And what of that
//! counts as Joel waiting.
//!
//! Every figure is per slice, never the whole machine's: the machine's
//! own pressure is mostly work waiting on work, which is what the slices
//! are for, and a watch built on it would cry wolf all day.

use crate::kernel::{Group, Sched, Stall};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What the kernel said at one moment.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Seconds, from a clock that only goes forward.
    pub at: f64,
    pub groups: BTreeMap<String, Group>,
    /// Each watched thread's schedstat, by its label and path.
    pub threads: BTreeMap<(String, String), Sched>,
}

/// A share of the window, in percent, each kind of stall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stalls {
    pub cpu_some: f64,
    pub cpu_full: f64,
    pub memory_some: f64,
    pub memory_full: f64,
    pub io_some: f64,
    pub io_full: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub swap_mib: u64,
    pub refaults: u64,
    pub sock_throttled: u64,
}

/// How long a kind of thread waited for a CPU each time it was ready, in
/// microseconds: on average across them, and for the one that waited most.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wait {
    pub mean_us: f64,
    pub worst_us: f64,
}

/// How long niri took to answer, in milliseconds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Answers {
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub asked: usize,
    pub unanswered: usize,
}

impl Answers {
    pub fn of(times_ms: &[f64], unanswered: usize) -> Answers {
        let mut t = times_ms.to_vec();
        t.sort_by(f64::total_cmp);
        let q = |p: f64| {
            t.get(((t.len() as f64 - 1.0) * p).round() as usize)
                .copied()
                .unwrap_or(0.0)
        };
        Answers {
            p50_ms: q(0.5),
            p99_ms: q(0.99),
            max_ms: t.last().copied().unwrap_or(0.0),
            asked: t.len() + unanswered,
            unanswered,
        }
    }
}

/// How hard the work behind Joel was working, from how much of the
/// window jobs and the Nix builds spent waiting for a CPU: so a window is
/// compared with one under the same load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Quiet,
    Busy,
    Loaded,
}

impl Band {
    fn of(work_cpu_some: f64) -> Band {
        match work_cpu_some {
            s if s < 10.0 => Band::Quiet,
            s if s < 50.0 => Band::Busy,
            _ => Band::Loaded,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Window {
    /// When it ended, in seconds since 1970.
    pub ended: u64,
    pub seconds: f64,
    /// The system generation it ran on, so a change can be told by its
    /// effect.
    pub system: String,
    pub band: Band,
    pub stalls: BTreeMap<String, Stalls>,
    pub memory: BTreeMap<String, Memory>,
    pub waits: BTreeMap<String, Wait>,
    pub niri: Answers,
    /// What counted as Joel waiting, in words.
    pub trouble: Vec<String>,
}

fn share(now: Stall, then: Stall, seconds: f64) -> (f64, f64) {
    let pct = |a: u64, b: u64| a.saturating_sub(b) as f64 / (seconds * 1e6) * 100.0;
    (pct(now.some, then.some), pct(now.full, then.full))
}

/// The slices holding the work behind Joel, read for the band: his own
/// slices, session and app, are the ones the trouble is about.
pub const WORK: [&str; 2] = ["jobs", "build"];

/// The window between two snapshots, given niri's answers in it.
pub fn between(then: &Snapshot, now: &Snapshot, niri: Answers, ended: u64, system: &str) -> Window {
    let seconds = (now.at - then.at).max(0.001);
    let mut stalls = BTreeMap::new();
    let mut memory = BTreeMap::new();
    for (name, g) in &now.groups {
        let Some(old) = then.groups.get(name) else {
            continue;
        };
        let (cpu_some, cpu_full) = share(g.cpu, old.cpu, seconds);
        let (memory_some, memory_full) = share(g.memory, old.memory, seconds);
        let (io_some, io_full) = share(g.io, old.io, seconds);
        stalls.insert(
            name.clone(),
            Stalls {
                cpu_some,
                cpu_full,
                memory_some,
                memory_full,
                io_some,
                io_full,
            },
        );
        memory.insert(
            name.clone(),
            Memory {
                swap_mib: g.swap >> 20,
                refaults: g.refaults.saturating_sub(old.refaults),
                sock_throttled: g.sock_throttled.saturating_sub(old.sock_throttled),
            },
        );
    }
    let mut waits: BTreeMap<String, (u64, u64, f64)> = BTreeMap::new();
    for (key, s) in &now.threads {
        let Some(old) = then.threads.get(key) else {
            continue;
        };
        let (wait, slices) = (
            s.wait_ns.saturating_sub(old.wait_ns),
            s.slices.saturating_sub(old.slices),
        );
        let entry = waits.entry(key.0.clone()).or_default();
        entry.0 += wait;
        entry.1 += slices;
        if slices > 0 {
            entry.2 = entry.2.max(wait as f64 / slices as f64 / 1000.0);
        }
    }
    let waits: BTreeMap<String, Wait> = waits
        .into_iter()
        .map(|(label, (wait, slices, worst))| {
            let mean_us = if slices > 0 {
                wait as f64 / slices as f64 / 1000.0
            } else {
                0.0
            };
            (
                label,
                Wait {
                    mean_us,
                    worst_us: worst,
                },
            )
        })
        .collect();
    let work: f64 = WORK
        .iter()
        .filter_map(|w| stalls.get(*w))
        .map(|s| s.cpu_some)
        .fold(0.0, f64::max);
    let mut window = Window {
        ended,
        seconds,
        system: system.to_string(),
        band: Band::of(work),
        stalls,
        memory,
        waits,
        niri,
        trouble: Vec::new(),
    };
    window.trouble = trouble(&window, then);
    window
}

/// What in a window counts as Joel waiting. Each threshold is from what
/// these were measured at with the machine quiet and under sixteen
/// spinning threads: session.slice's CPU stall read 0.00% while jobs
/// stalled 23%; a wakeup in session waited at most 0.75 ms and in app
/// 1.3 ms; niri's main thread waited 23 us a turn on average across a
/// day of builds; one 120 Hz frame is 8.3 ms.
fn trouble(w: &Window, then: &Snapshot) -> Vec<String> {
    let mut found = Vec::new();
    let s = |name: &str| w.stalls.get(name).copied().unwrap_or_default();
    if s("session").cpu_some > 1.0 {
        found.push(format!(
            "the desktop waited for a CPU {:.1}% of the time",
            s("session").cpu_some
        ));
    }
    if s("session").memory_full > 0.0 {
        found.push(format!(
            "the desktop stalled on memory {:.2}% of the time",
            s("session").memory_full
        ));
    }
    if s("app").memory_full > 0.1 {
        found.push(format!(
            "apps stalled on memory {:.2}% of the time",
            s("app").memory_full
        ));
    }
    if s("app").io_full > 1.0 {
        found.push(format!(
            "apps stalled on the disk {:.1}% of the time",
            s("app").io_full
        ));
    }
    if let (Some(now), Some(old)) = (w.memory.get("session"), then.groups.get("session"))
        && now.swap_mib > old.swap >> 20
    {
        found.push(format!(
            "the desktop went to swap, {} MiB now",
            now.swap_mib
        ));
    }
    if let Some(niri) = w.waits.get("niri")
        && niri.mean_us > 50.0
    {
        found.push(format!(
            "niri waited {:.0} us a turn for a CPU",
            niri.mean_us
        ));
    }
    if let Some(pw) = w.waits.get("pipewire")
        && pw.worst_us > 100.0
    {
        found.push(format!(
            "sound waited {:.0} us a turn for a CPU",
            pw.worst_us
        ));
    }
    if w.niri.max_ms > 50.0 {
        found.push(format!("niri took {:.0} ms to answer", w.niri.max_ms));
    } else if w.niri.p99_ms > 8.3 {
        found.push(format!(
            "niri took over a frame to answer, {:.1} ms",
            w.niri.p99_ms
        ));
    }
    if w.niri.unanswered > 0 {
        found.push(format!(
            "niri didn't answer {} of {} times",
            w.niri.unanswered, w.niri.asked
        ));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(cpu_some: u64, memory_full: u64, swap: u64) -> Group {
        Group {
            cpu: Stall {
                some: cpu_some,
                full: 0,
            },
            memory: Stall {
                some: memory_full,
                full: memory_full,
            },
            swap,
            ..Group::default()
        }
    }

    fn snapshot(at: f64, groups: &[(&str, Group)], niri_wait: u64, niri_slices: u64) -> Snapshot {
        let mut s = Snapshot {
            at,
            ..Snapshot::default()
        };
        for (name, g) in groups {
            s.groups.insert(name.to_string(), *g);
        }
        s.threads.insert(
            ("niri".into(), "/proc/10/schedstat".into()),
            Sched {
                run_ns: 0,
                wait_ns: niri_wait,
                slices: niri_slices,
            },
        );
        s
    }

    fn quiet_niri() -> Answers {
        Answers::of(&[0.4, 0.5, 0.6, 0.5, 0.5], 0)
    }

    #[test]
    fn work_waiting_on_work_is_no_trouble() {
        let then = snapshot(
            0.0,
            &[("session", group(0, 0, 0)), ("jobs", group(0, 0, 0))],
            0,
            0,
        );
        // jobs stalled 9 s of 10, the desktop 10 ms; niri waited 20 us a turn.
        let now = snapshot(
            10.0,
            &[
                ("session", group(10_000, 0, 0)),
                ("jobs", group(9_000_000, 0, 0)),
            ],
            20_000_000,
            1_000,
        );
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(w.band, Band::Loaded);
        assert!(w.trouble.is_empty(), "{:?}", w.trouble);
        assert!((w.waits["niri"].mean_us - 20.0).abs() < 1e-9);
    }

    #[test]
    fn the_desktop_waiting_is_trouble() {
        let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
        let now = snapshot(
            10.0,
            &[("session", group(300_000, 0, 0))],
            90_000_000,
            1_000,
        );
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(w.band, Band::Quiet);
        assert_eq!(
            w.trouble,
            [
                "the desktop waited for a CPU 3.0% of the time",
                "niri waited 90 us a turn for a CPU"
            ]
        );
    }

    #[test]
    fn apps_stalling_on_memory_and_the_desktop_swapping_are_trouble() {
        let then = snapshot(
            0.0,
            &[("session", group(0, 0, 0)), ("app", group(0, 0, 0))],
            0,
            0,
        );
        let now = snapshot(
            10.0,
            &[
                ("session", group(0, 0, 64 << 20)),
                ("app", group(0, 50_000, 0)),
            ],
            0,
            0,
        );
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(
            w.trouble,
            [
                "apps stalled on memory 0.50% of the time",
                "the desktop went to swap, 64 MiB now"
            ]
        );
    }

    #[test]
    fn niri_slow_to_answer_is_trouble() {
        let then = snapshot(0.0, &[], 0, 0);
        let now = snapshot(10.0, &[], 0, 0);
        let slow = Answers::of(&[0.5, 0.5, 0.5, 0.5, 12.0], 1);
        let w = between(&then, &now, slow, 0, "gen");
        assert_eq!(
            w.trouble,
            [
                "niri took over a frame to answer, 12.0 ms",
                "niri didn't answer 1 of 6 times"
            ]
        );
    }

    #[test]
    fn answers_give_their_quantiles() {
        let a = Answers::of(&[3.0, 1.0, 2.0], 0);
        assert_eq!((a.p50_ms, a.max_ms, a.asked), (2.0, 3.0, 3));
        assert_eq!(Answers::of(&[], 2).asked, 2);
    }
}
