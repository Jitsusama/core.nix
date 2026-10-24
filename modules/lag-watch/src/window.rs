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

/// Nanoseconds, microseconds or a tally, as a float. Exact: every one here
/// is far below 2⁵³.
#[allow(clippy::cast_precision_loss)]
pub fn float(n: u64) -> f64 {
    n as f64
}

/// How many there are, as a float; as exact as `float`.
#[allow(clippy::cast_precision_loss)]
pub fn count(n: usize) -> f64 {
    n as f64
}

/// The place in a list nearest a fractional one; before the first is the
/// first.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest(at: f64) -> usize {
    at.round().max(0.0) as usize
}

/// What the kernel said at one moment.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Seconds, from a clock that only goes forward.
    pub at: f64,
    pub groups: BTreeMap<String, Group>,
    /// Each of app.slice's own units and scopes, by its name: what an app
    /// is to Joel isn't always what runs there, so a stall names its unit.
    pub apps: BTreeMap<String, Group>,
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
            t.get(nearest((count(t.len()) - 1.0) * p))
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
    /// What the wait he'd feel most was on, in one word (`Cause::word`),
    /// or empty when nothing waited: a shell showing a mark rather than
    /// sentences reads this instead of parsing them. Windows written
    /// before it was added read as empty.
    #[serde(default)]
    pub worst: String,
    /// Whose wait that was (`Who::word`), or empty when nothing waited.
    /// Work waiting on work is never trouble, so it is never named.
    #[serde(default)]
    pub who: String,
    /// When the run of windows with trouble that this one ends began, as
    /// the end of its first window in seconds since 1970; none when
    /// nothing waited. A shell can mark trouble that lasts, rather than
    /// every window's. `between` sees one window, so the watch's loop
    /// fills it in with `Window::carry`.
    #[serde(default)]
    pub since: Option<u64>,
}

impl Window {
    /// Fills in `since` from the window before's: kept while trouble goes
    /// on, begun with this window when it starts, cleared when it stops.
    pub fn carry(&mut self, before: Option<u64>) {
        self.since = if self.trouble.is_empty() {
            None
        } else {
            Some(before.unwrap_or(self.ended))
        };
    }
}

/// Whose wait a trouble line is about: the desktop's (session.slice, niri
/// and sound among it) or the apps'.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Desktop,
    Apps,
}

impl Who {
    pub fn word(self) -> &'static str {
        match self {
            Who::Desktop => "desktop",
            Who::Apps => "apps",
        }
    }
}

/// What a trouble line's wait was on. The desktop and the apps wait on a
/// CPU, memory or the disk; niri slow to answer is its own, whatever
/// held it, since that is the screen not keeping up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Cpu,
    Memory,
    Disk,
    Niri,
}

impl Cause {
    pub fn word(self) -> &'static str {
        match self {
            Cause::Cpu => "cpu",
            Cause::Memory => "memory",
            Cause::Disk => "disk",
            Cause::Niri => "niri",
        }
    }
}

/// The trouble lines found in a window, and the cause of the one Joel
/// would feel most.
#[derive(Default)]
struct Found {
    lines: Vec<String>,
    worst: Option<(u8, Who, Cause)>,
}

impl Found {
    /// Adds a line, `felt` saying how much more it's felt than others:
    /// a stall of a few percent costs a moment, a screen that doesn't
    /// answer costs a frame or more, and a program ended costs its work.
    /// Between lines felt alike, the first stays the worst.
    fn add(&mut self, who: Who, felt: u8, cause: Cause, line: String) {
        if self.worst.is_none_or(|(worst, _, _)| felt > worst) {
            self.worst = Some((felt, who, cause));
        }
        self.lines.push(line);
    }
}

fn share(now: Stall, then: Stall, seconds: f64) -> (f64, f64) {
    let pct = |a: u64, b: u64| float(a.saturating_sub(b)) / (seconds * 1e6) * 100.0;
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
            entry.2 = entry.2.max(float(wait) / float(slices) / 1000.0);
        }
    }
    let waits: BTreeMap<String, Wait> = waits
        .into_iter()
        .map(|(label, (wait, slices, worst))| {
            let mean_us = if slices > 0 {
                float(wait) / float(slices) / 1000.0
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
        worst: String::new(),
        who: String::new(),
        since: None,
    };
    let found = trouble(&window, then, now);
    window.trouble = found.lines;
    if let Some((_, who, cause)) = found.worst {
        window.worst = cause.word().to_string();
        window.who = who.word().to_string();
    }
    window
}

/// The app whose tasks all stalled longest on one kind, from `full` of
/// the pressure `of` picks, with its share of the window in percent; none
/// when no app's did.
fn most(then: &Snapshot, now: &Snapshot, of: fn(&Group) -> Stall) -> Option<(String, f64)> {
    let seconds = (now.at - then.at).max(0.001);
    now.apps
        .iter()
        .filter_map(|(name, g)| {
            let old = then.apps.get(name)?;
            let (_, full) = share(of(g), of(old), seconds);
            (full > 0.0).then(|| (name.clone(), full))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// ", most in <unit> (<share>)", for a trouble line, or nothing.
fn naming(most: Option<(String, f64)>, digits: usize) -> String {
    most.map_or_else(String::new, |(name, pct)| {
        format!(", most in {name} ({pct:.digits$}%)")
    })
}

/// One 120 Hz frame, 8.3 ms, as a share of a ten-second window in percent.
const FRAME_OF_A_WINDOW: f64 = 8.3 / 10_000.0 * 100.0;

/// What in a window counts as Joel waiting. Each threshold is from what
/// these were measured at with the machine quiet and under sixteen
/// spinning threads: session.slice's CPU stall read 0.00% while jobs
/// stalled 23%; a wakeup in session waited at most 0.75 ms and in app
/// 1.3 ms; niri's main thread waited 23 us a turn on average across a
/// day of builds; one 120 Hz frame is 8.3 ms.
///
/// The desktop's memory stalls count from a frame's worth of the window
/// on. In a day's 457 windows with any, the median was 0.6 ms and nine in
/// ten were under 7; through the 35 of a frame or more, niri still
/// answered within a millisecond in 33. A stall too short to cost a frame
/// is no wait of Joel's, and niri late for one is counted on its own.
fn trouble(w: &Window, then: &Snapshot, now: &Snapshot) -> Found {
    let mut found = Found::default();
    let s = |name: &str| w.stalls.get(name).copied().unwrap_or_default();
    if s("session").cpu_some > 1.0 {
        found.add(
            Who::Desktop,
            2,
            Cause::Cpu,
            format!(
                "the desktop waited for a CPU {:.1}% of the time",
                s("session").cpu_some
            ),
        );
    }
    if s("session").memory_full >= FRAME_OF_A_WINDOW {
        found.add(
            Who::Desktop,
            4,
            Cause::Memory,
            format!(
                "the desktop stalled on memory {:.2}% of the time",
                s("session").memory_full
            ),
        );
    }
    if s("app").memory_full > 0.1 {
        found.add(
            Who::Apps,
            3,
            Cause::Memory,
            format!(
                "apps stalled on memory {:.2}% of the time{}",
                s("app").memory_full,
                naming(most(then, now, |g| g.memory), 2)
            ),
        );
    }
    if s("app").io_full > 1.0 {
        found.add(
            Who::Apps,
            3,
            Cause::Disk,
            format!(
                "apps stalled on the disk {:.1}% of the time{}",
                s("app").io_full,
                naming(most(then, now, |g| g.io), 1)
            ),
        );
    }
    lost(w, then, now, &mut found);
    turns(w, &mut found);
    found
}

/// Programs ended for memory, the worst a wait comes to, and the
/// desktop's memory sent to swap.
fn lost(w: &Window, then: &Snapshot, now: &Snapshot, found: &mut Found) {
    let ended = |name: &str| {
        let was = then.groups.get(name).map_or(0, |g| g.killed);
        now.groups
            .get(name)
            .map_or(0, |g| g.killed.saturating_sub(was))
    };
    if ended("session") > 0 {
        found.add(
            Who::Desktop,
            7,
            Cause::Memory,
            format!(
                "the desktop lost {} programs to the kernel's OOM killer",
                ended("session")
            ),
        );
    }
    if ended("app") > 0 {
        let unit = now
            .apps
            .iter()
            .find(|(name, g)| {
                then.apps
                    .get(*name)
                    .is_some_and(|old| g.killed > old.killed)
            })
            .map_or_else(String::new, |(name, _)| format!(", in {name}"));
        found.add(
            Who::Apps,
            7,
            Cause::Memory,
            format!(
                "apps lost {} programs to the kernel's OOM killer{unit}",
                ended("app")
            ),
        );
    }
    if let (Some(now), Some(old)) = (w.memory.get("session"), then.groups.get("session"))
        && now.swap_mib > old.swap >> 20
    {
        found.add(
            Who::Desktop,
            3,
            Cause::Memory,
            format!("the desktop went to swap, {} MiB now", now.swap_mib),
        );
    }
}

/// niri's and sound's turns for a CPU, and how niri answered.
fn turns(w: &Window, found: &mut Found) {
    if let Some(niri) = w.waits.get("niri")
        && niri.mean_us > 50.0
    {
        found.add(
            Who::Desktop,
            2,
            Cause::Cpu,
            format!("niri waited {:.0} us a turn for a CPU", niri.mean_us),
        );
    }
    if let Some(pw) = w.waits.get("pipewire")
        && pw.worst_us > 100.0
    {
        found.add(
            Who::Desktop,
            2,
            Cause::Cpu,
            format!("sound waited {:.0} us a turn for a CPU", pw.worst_us),
        );
    }
    if w.niri.max_ms > 50.0 {
        found.add(
            Who::Desktop,
            5,
            Cause::Niri,
            format!("niri took {:.0} ms to answer", w.niri.max_ms),
        );
    } else if w.niri.p99_ms > 8.3 {
        found.add(
            Who::Desktop,
            4,
            Cause::Niri,
            format!("niri took over a frame to answer, {:.1} ms", w.niri.p99_ms),
        );
    }
    if w.niri.unanswered > 0 {
        found.add(
            Who::Desktop,
            6,
            Cause::Niri,
            format!(
                "niri didn't answer {} of {} times",
                w.niri.unanswered, w.niri.asked
            ),
        );
    }
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
        assert_eq!((w.worst.as_str(), w.who.as_str()), ("", ""));
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
        assert_eq!(w.worst, "cpu");
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
        assert_eq!(w.worst, "memory");
    }

    #[test]
    fn the_desktop_stalling_on_memory_is_trouble_from_a_frame_on() {
        let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
        // 6 ms of 10 s, under one 120 Hz frame: most windows' stalls are
        // this size, and niri answered within a millisecond through them.
        let under = snapshot(10.0, &[("session", group(0, 6_000, 0))], 0, 0);
        let w = between(&then, &under, quiet_niri(), 0, "gen");
        assert!(w.trouble.is_empty(), "{:?}", w.trouble);
        let frame = snapshot(10.0, &[("session", group(0, 9_000, 0))], 0, 0);
        let w = between(&then, &frame, quiet_niri(), 0, "gen");
        assert_eq!(
            w.trouble,
            ["the desktop stalled on memory 0.09% of the time"]
        );
    }

    #[test]
    fn a_program_the_kernel_ended_for_memory_is_trouble_and_named() {
        let mut then = snapshot(
            0.0,
            &[("app", group(0, 0, 0)), ("jobs", group(0, 0, 0))],
            0,
            0,
        );
        let mut now = then.clone();
        now.at = 10.0;
        let killed = |n| Group {
            killed: n,
            ..Group::default()
        };
        then.apps.insert("chrome.scope".to_string(), killed(0));
        now.apps.insert("chrome.scope".to_string(), killed(2));
        now.groups.insert("app".to_string(), killed(2));
        // Work's programs ended are no wait of Joel's.
        now.groups.insert("jobs".to_string(), killed(5));
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(
            w.trouble,
            ["apps lost 2 programs to the kernel's OOM killer, in chrome.scope"]
        );
        assert_eq!(w.worst, "memory");
    }

    #[test]
    fn the_worst_is_what_joel_would_feel_most_not_what_came_first() {
        let then = snapshot(
            0.0,
            &[("session", group(0, 0, 0)), ("app", group(0, 0, 0))],
            0,
            0,
        );
        let mut now = snapshot(
            10.0,
            &[("session", group(300_000, 0, 0)), ("app", group(0, 0, 0))],
            0,
            0,
        );
        now.groups.get_mut("app").unwrap().io = Stall {
            some: 200_000,
            full: 200_000,
        };
        // The desktop's CPU wait comes first, but a disk stall of 2% is felt more.
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(w.trouble.len(), 2, "{:?}", w.trouble);
        assert_eq!(w.worst, "disk");
        // niri not answering is felt more than either.
        let w = between(&then, &now, Answers::of(&[0.5, 0.5], 1), 0, "gen");
        assert_eq!(w.worst, "niri");
        assert_eq!(w.who, "desktop");
        // And a program ended for memory more than anything.
        now.groups.get_mut("app").unwrap().killed = 1;
        let w = between(&then, &now, Answers::of(&[0.5, 0.5], 1), 0, "gen");
        assert_eq!((w.worst.as_str(), w.who.as_str()), ("memory", "apps"));
    }

    #[test]
    fn trouble_that_lasts_says_since_when() {
        let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
        let busy = snapshot(10.0, &[("session", group(300_000, 0, 0))], 0, 0);
        let quiet = snapshot(10.0, &[("session", group(0, 0, 0))], 0, 0);
        let mut since = None;
        let mut seen = Vec::new();
        for (ended, now) in [(100, &quiet), (110, &busy), (120, &busy), (130, &quiet)] {
            let mut w = between(&then, now, quiet_niri(), ended, "gen");
            w.carry(since);
            since = w.since;
            seen.push(since);
        }
        assert_eq!(seen, [None, Some(110), Some(110), None]);
    }

    #[test]
    fn an_app_s_stall_names_the_unit_that_stalled_most() {
        let mut then = snapshot(0.0, &[("app", group(0, 0, 0))], 0, 0);
        let mut now = snapshot(10.0, &[("app", group(0, 225_000, 0))], 0, 0);
        for (name, stalled) in [
            ("dev.up.backend-dev.service", 220_000),
            ("kitty-1.scope", 5_000),
        ] {
            then.apps.insert(name.to_string(), group(0, 0, 0));
            now.apps.insert(name.to_string(), group(0, stalled, 0));
        }
        // One that came in the window has nothing to compare with.
        now.apps
            .insert("new.scope".to_string(), group(0, 900_000, 0));
        let w = between(&then, &now, quiet_niri(), 0, "gen");
        assert_eq!(
            w.trouble,
            [
                "apps stalled on memory 2.25% of the time, most in dev.up.backend-dev.service (2.20%)"
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
        assert_eq!(w.worst, "niri");
    }

    #[test]
    fn answers_give_their_quantiles() {
        let a = Answers::of(&[3.0, 1.0, 2.0], 0);
        assert_eq!((a.p50_ms, a.max_ms, a.asked), (2.0, 3.0, 3));
        assert_eq!(Answers::of(&[], 2).asked, 2);
    }
}
