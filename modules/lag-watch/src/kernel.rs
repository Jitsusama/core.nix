//! What the kernel says, read from its files: how long a cgroup's tasks
//! stalled, what its memory did, and how long a thread waited to run.
//! Each reader takes the file's text, so the tests need no kernel.

use std::fs;
use std::path::{Path, PathBuf};

use crate::apps::{APPS, Apps};

/// A pressure file's running totals, in microseconds: some task stalled
/// (some), or every task did (full).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stall {
    pub some: u64,
    pub full: u64,
}

/// The totals of a `cpu.pressure`, `memory.pressure` or `io.pressure` file.
pub fn stall(text: &str) -> Stall {
    let total = |kind: &str| {
        text.lines()
            .find(|l| l.starts_with(kind))
            .and_then(|l| l.split_whitespace().find_map(|w| w.strip_prefix("total=")))
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    };
    Stall {
        some: total("some"),
        full: total("full"),
    }
}

/// One field of a flat keyed file, such as memory.stat or memory.events.
pub fn field(text: &str, key: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(' ')?.trim().parse().ok())
        .unwrap_or(0)
}

/// A thread's schedstat: nanoseconds on a CPU, nanoseconds waiting for
/// one, and how many times it ran; and, where its `sched` keeps it, the
/// fair scheduler's own count of the wait (`waited`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sched {
    pub run_ns: u64,
    pub wait_ns: u64,
    pub slices: u64,
    pub fair_ns: Option<u64>,
    /// What each count, the schedstat's and the fair one, grew by in the
    /// window that ended at this read and the other didn't match
    /// (`credit_since`), kept for one window in case the other books it
    /// late.
    pub owed: (u64, u64),
}

impl Sched {
    /// How long the thread waited for a CPU since `old`: the lesser of
    /// what each count grew by, where both were read. Each counts every
    /// real wait, and each counts some sleeps the other doesn't: the
    /// schedstat a delayed sleeper's move between CPUs, up to seconds; the
    /// fair count the time a sleeper stays queued before it leaves, which
    /// put Chrome's drawing thread's at 310 ms against 125 ms over 30 s on
    /// 26 November. The lesser is the real wait, plus whatever both
    /// counted.
    ///
    /// The two credit a wait at different moments: the schedstat adds a
    /// queued thread's wait so far when it moves between CPUs, the fair
    /// count only when the wait ends, and the two are read from two files.
    /// So what one grew by and the other didn't in the window before is
    /// owed (`owed`), and is counted when the other's growth this window,
    /// past what the schedstat's own matched, books it. This window's
    /// growth is matched to this window's first, so a sleep owed can pair
    /// only with the other count's leftover, and what's owed is let go
    /// after one window. Each unit of growth is counted at most once, so
    /// over a run of windows that starts with nothing owed, while both
    /// counts are read and the fair one doesn't go back, the count is never
    /// more than the lesser of the two counts' totals; a run that starts
    /// mid-way may count what the window before it owed. Around a sleep one
    /// count booked, at most
    /// the other's own excess in the windows either side is counted with
    /// it. What's let go is lost: a wait that spans two reads, which a
    /// wait of just over one window can, loses what the schedstat booked
    /// of it before the first of them, and a wait whose early booking
    /// paid another wait's leftover loses that part. Both are
    /// under-counts, of waits moved between CPUs while queued. A fair
    /// count that went back, as when schedstats was turned off and on,
    /// leaves the schedstat's alone.
    pub fn waited_since(&self, old: &Sched) -> u64 {
        match self.matched(old) {
            Some((waited, _)) => waited,
            None => self.wait_ns.saturating_sub(old.wait_ns),
        }
    }

    /// Takes what each count is owed for the window after this read
    /// (`owed`): what it grew by since `old` that nothing matched, never
    /// what `old` was owed, which is let go.
    pub fn credit_since(&mut self, old: &Sched) {
        self.owed = self.matched(old).map_or((0, 0), |(_, owed)| owed);
    }

    /// What `waited_since` counts and `credit_since` owes, where both
    /// counts were read and the fair one didn't go back.
    fn matched(&self, old: &Sched) -> Option<(u64, (u64, u64))> {
        let (Some(now), Some(then)) = (self.fair_ns, old.fair_ns) else {
            return None;
        };
        let fair = now.checked_sub(then)?;
        let stat = self.wait_ns.saturating_sub(old.wait_ns);
        let both = stat.min(fair);
        let (stat_left, fair_left) = (stat - both, fair - both);
        let (stat_owed, fair_owed) = old.owed;
        let stat_paid = stat_owed.min(fair_left);
        let fair_paid = fair_owed.min(stat_left);
        Some((
            both + stat_paid + fair_paid,
            (stat_left - fair_paid, fair_left - stat_paid),
        ))
    }
}

/// A thread's nice, from its `stat`: field 19. A process's `stat` holds
/// its main thread's.
pub fn nice(text: &str) -> Option<i64> {
    stat_field(text, 19)?.parse().ok()
}

/// When a process started, in clock ticks since boot, from its `stat`:
/// field 22. A pid used again is another process with another start.
pub fn started(text: &str) -> Option<u64> {
    stat_field(text, 22)?.parse().ok()
}

/// Whether a thread's `stat` says it was neither running nor ready to
/// run (field 3 other than R): asleep, so any wait it has yet to have
/// begins after this read. A `stat` that can't be read says nothing.
pub fn asleep(text: &str) -> bool {
    stat_field(text, 3).is_some_and(|state| state != "R")
}

/// A process's parent's pid, from its `stat`: field 4.
pub fn parent(text: &str) -> Option<&str> {
    stat_field(text, 4)
}

/// Field `n` of a `stat`, counted from 1, past the name, which is in
/// brackets and may hold spaces and brackets of its own.
fn stat_field(text: &str, n: usize) -> Option<&str> {
    text.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(n.checked_sub(3)?)
}

pub fn sched(text: &str) -> Option<Sched> {
    let mut n = text.split_whitespace().map(|w| w.parse::<u64>().ok());
    Some(Sched {
        run_ns: n.next()??,
        wait_ns: n.next()??,
        slices: n.next()??,
        fair_ns: None,
        owed: (0, 0),
    })
}

/// A thread's schedstat, with the fair scheduler's count of its wait from
/// its `sched` beside it where the kernel keeps one there (`waited`), so
/// `Sched::waited_since` can take the lesser. The schedstat's own wait
/// also counts some sleeps: on 26 November, in 20 minutes, it counted 23
/// sleeps of Slack's threads and a test's as waits of up to 4 s, while the
/// fair count grew by at most 2 ms on each (docs/kernel.md, The Patches).
/// None for a schedstat that can't be read.
pub fn read_sched(schedstat: &Path) -> Option<Sched> {
    read_sched_and_text(schedstat).map(|(read, _)| read)
}

/// `read_sched`, with the text of the `sched` it read, so a caller that
/// reads the `sched` for more than the wait reads it once. The schedstat is
/// read first and the `sched` after it, the order the carry's tests
/// describe, and only once the schedstat gave a read: a thread that ended
/// would otherwise cost a second failed open on every two-second read.
pub fn read_sched_and_text(schedstat: &Path) -> Option<(Sched, String)> {
    read_in_order(schedstat, text)
}

/// `read_sched_and_text`, each file read by `read`, so a test can see
/// which files it opens, and in what order.
fn read_in_order(
    schedstat: &Path,
    mut read: impl FnMut(&Path) -> String,
) -> Option<(Sched, String)> {
    let mut found = sched(&read(schedstat))?;
    let sched_text = read(&schedstat.with_file_name("sched"));
    found.fair_ns = waited(&sched_text);
    Some((found, sched_text))
}

/// How long a thread has waited for a CPU in all, in nanoseconds, by the
/// fair scheduler's own count in its `sched`: `wait_sum`, never the
/// `iowait_sum` beside it. The kernel keeps it only while
/// `kernel.sched_schedstats` is on; none where it isn't kept.
pub fn waited(text: &str) -> Option<u64> {
    sched_ms(text, |key| key == "wait_sum" || key.ends_with(".wait_sum"))
}

/// How long a thread has been blocked in the kernel, in nanoseconds, from
/// its `sched`: asleep where nothing but the kernel could wake it, on a
/// lock, a page read back or reclaim, which no wait for a CPU shows. The
/// kernel keeps it only while `kernel.sched_schedstats` is on, and prints
/// it in milliseconds; none where it isn't kept.
pub fn blocked(text: &str) -> Option<u64> {
    sched_ms(text, |key| key.ends_with("sum_block_runtime"))
}

/// The longest name a thread can give itself, in bytes: the kernel's
/// `TASK_COMM_LEN`, 16, less the nul that ends it. `sched_stats` tells the
/// kernel's row of dashes from one a name forges only by its length, so that
/// guard holds only while names stay this short.
const NAME_MOST: usize = 15;

/// The lines of a `sched` past its header. The header opens with the
/// thread's name, which the thread sets itself and may break into lines
/// of its own, such as one reading `wait_sum: 9e9`, and ends at a line of
/// dashes longer than any name can hold. So only what follows that line
/// is the kernel's; a `sched` with no such line has none.
fn sched_stats(text: &str) -> std::str::Lines<'_> {
    let mut lines = text.lines();
    lines
        .by_ref()
        .find(|line| line.trim().len() > NAME_MOST && line.trim().bytes().all(|b| b == b'-'));
    lines
}

/// The first line of a `sched`'s stats whose key `named` takes, which the
/// kernel prints in milliseconds, in nanoseconds.
fn sched_ms(text: &str, named: impl Fn(&str) -> bool) -> Option<u64> {
    let ms: f64 = sched_stats(text).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        named(key.trim()).then(|| value.trim().parse().ok())?
    })?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "milliseconds a thread has lived, held at zero or more, are far below 2⁶⁴ ns"
    )]
    Some((ms.max(0.0) * 1e6).round() as u64)
}

/// What one cgroup holds: its stalls of each kind and its memory's harm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Group {
    pub cpu: Stall,
    pub memory: Stall,
    pub io: Stall,
    /// Bytes of its memory in swap.
    pub swap: u64,
    /// Pages it faulted back in that it had lately, anon and file.
    pub refaults: u64,
    /// Times the kernel held back its sockets for memory.
    pub sock_throttled: u64,
    /// Programs in it or below it the kernel's OOM killer ended.
    pub killed: u64,
}

/// A file under /proc or a cgroup as text, or none when it can't be read.
/// A thread's `sched` and `stat` carry the name it gave itself, which the
/// kernel cuts byte by byte and may leave short of UTF-8, so its bytes are
/// taken as they come rather than losing the whole file to one bad name.
pub fn text(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Reads a cgroup, or None for one that isn't there.
pub fn group(dir: &Path) -> Option<Group> {
    if !dir.is_dir() {
        return None;
    }
    let stat = text(&dir.join("memory.stat"));
    let events = text(&dir.join("memory.events.local"));
    Some(Group {
        cpu: stall(&text(&dir.join("cpu.pressure"))),
        memory: stall(&text(&dir.join("memory.pressure"))),
        io: stall(&text(&dir.join("io.pressure"))),
        swap: text(&dir.join("memory.swap.current"))
            .trim()
            .parse()
            .unwrap_or(0),
        refaults: field(&stat, "workingset_refault_anon") + field(&stat, "workingset_refault_file"),
        sock_throttled: field(&events, "sock_throttled"),
        // The cgroup's own file counts its descendants' kills, as a
        // slice's must, since a slice runs nothing itself.
        killed: field(&text(&dir.join("memory.events")), "oom_kill"),
    })
}

/// A thread worth watching: a process by its name, and within it the
/// threads named so, or its main thread for an empty name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watched {
    pub process: &'static str,
    pub thread: &'static str,
    pub label: &'static str,
}

/// The threads that draw the screen and play sound: niri's main thread,
/// which reads input too, and PipeWire's data loops, which move the sound.
pub const WATCHED: [Watched; 2] = [
    Watched {
        process: "niri",
        thread: "",
        label: "niri",
    },
    Watched {
        process: "pipewire",
        thread: "data-loop",
        label: "pipewire",
    },
];

/// A thread found to watch: what it's watched as, its own name, where its
/// schedstat is, and whether it carries a call.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Thread {
    pub label: &'static str,
    pub name: String,
    pub schedstat: PathBuf,
    pub call: bool,
}

/// The threads WebRTC starts for a call, as the kernel keeps their names,
/// cut to 15 bytes: a call's audio encoder (`ChannelSend`), its pacer
/// (`TaskQueuePacedSender`), its event log, the video it sends and
/// receives (`VideoStreamEncoder`, `VideoReceiveStream2`), the sound
/// device's timer (`AudioDeviceBuffer`), and PipeWire's loop for the
/// stream it shares. Tuple's daemon holds none of them between calls; it
/// keeps its own and WebRTC's factory threads, which idle.
pub const CALL_THREADS: [&str; 8] = [
    "AudioEncoder",
    "TaskQueuePacedS",
    "rtc_event_log",
    "EncoderQueue",
    "DecodingQueue",
    "IncomingVideoSt",
    "AudioDeviceBuff",
    "data-loop",
];

/// What Chromium names the threads that carry WebRTC or sound, cut as
/// the kernel cuts them: WebRTC's own (`WebRTC_Signaling`,
/// `WebRTC_Network`, `WebRTC_Worker`), the sound devices a page plays and
/// records through (`AudioOutputDevice`, `AudioInputDevice`), the audio
/// service's `AudioThread`, and a page's audio worklet. Chromium's
/// `ThreadPoolAudio` is left out: every process keeps one, idle. They
/// count only while a page records the microphone (`CHROME_IN_A_CALL`):
/// WebRTC's own run in any tab holding a peer connection, and the sound
/// devices in any page playing sound, neither of which is a call.
pub const CHROME_CALL_THREADS: [&str; 6] = [
    "WebRTC_",
    "AudioOutputDevi",
    "AudioInputDevic",
    "AudioThread",
    "AudioWorkletThr",
    "RealtimeAudioWo",
];

/// The thread a Chrome page records the microphone through, which is
/// Chrome in a call. Chromium runs WebRTC's own per-call work on its
/// thread pool, unnamed, so this is the one name a call alone gives.
const CHROME_IN_A_CALL: &str = "AudioInputDevic";

/// A process's threads, by name and schedstat path.
pub fn tasks(process: &Path) -> Vec<(String, PathBuf)> {
    fs::read_dir(process.join("task"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|task| {
            let name = text(&task.path().join("comm"));
            let name = name.strip_suffix('\n').unwrap_or(&name);
            (!name.is_empty()).then(|| (name.to_string(), task.path().join("schedstat")))
        })
        .collect()
}

/// Every thread of Tuple's daemon, the processes of its unit's cgroup,
/// but only while one of them carries a call: so between calls nothing
/// of Tuple's is read. The cgroup names them without asking systemd.
fn tuple(proc: &Path, unit: &Path) -> Vec<Thread> {
    let found: Vec<Thread> = text(&unit.join("cgroup.procs"))
        .split_whitespace()
        .flat_map(|pid| tasks(&proc.join(pid)))
        .map(|(name, schedstat)| Thread {
            label: "tuple",
            call: CALL_THREADS.iter().any(|c| name.starts_with(c)),
            name,
            schedstat,
        })
        .collect();
    if found.iter().any(|t| t.call) {
        found
    } else {
        Vec::new()
    }
}

/// A call's threads whose program no longer carries it are let go: once
/// its last call thread can't be read, the call is over, and Tuple's idle
/// threads would only say how an idle daemon waits, Chrome's how a tab
/// playing sound does.
pub fn while_in_a_call<V>(read: &mut std::collections::BTreeMap<Thread, V>) {
    let calling: Vec<&'static str> = read.keys().filter(|t| t.call).map(|t| t.label).collect();
    read.retain(|t, _| !CALLERS.contains(&t.label) || calling.contains(&t.label));
}

/// The programs a call's threads are watched in, each recorded apart.
pub const CALLERS: [&str; 2] = ["tuple", "chrome"];

/// Every thread of the watched kinds now running under `proc`, by label,
/// with their schedstat paths, and a call's threads: Tuple's daemon's,
/// from its unit's cgroup at `tuple_unit`, and Chrome's that carry WebRTC
/// or sound. Once the niri drawing the desktop is known by its pid, no
/// other niri counts: a nested one previewing a shell runs with the jobs
/// and waits as they do, and a `niri msg` client draws nothing, so either
/// would be read as the desktop waiting.
///
/// The apps' drawing threads (`apps::APPS`) are found in the same pass,
/// each process looked into once, as `Apps` keeps them.
pub fn threads(
    proc: &Path,
    compositor: Option<u32>,
    tuple_unit: &Path,
    apps: &mut Apps,
) -> Vec<Thread> {
    let mut found = tuple(proc, tuple_unit);
    let mut chrome = Vec::new();
    let Ok(entries) = fs::read_dir(proc) else {
        return found;
    };
    // The apps' processes looked into this pass: one whose pid now names
    // another program is let go at once.
    let mut looked = std::collections::BTreeSet::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let comm = text(&entry.path().join("comm"));
        let mut joels = false;
        if let Some((_, label)) = APPS.iter().find(|(app, _)| *app == comm.trim()) {
            looked.insert(name.clone());
            if let Some(drawing) = apps.of(&name, &entry.path(), label) {
                joels = true;
                found.extend(drawing);
            }
        }
        if comm.trim() == "chrome" {
            // A Chrome Joel didn't start, an agent's or a test's with a fake
            // microphone, carries no call of his.
            if !joels && !apps.may_call(&text(&entry.path().join("cgroup")), proc) {
                continue;
            }
            chrome.extend(
                tasks(&entry.path())
                    .into_iter()
                    .filter(|(name, _)| CHROME_CALL_THREADS.iter().any(|c| name.starts_with(c)))
                    .map(|(name, schedstat)| Thread {
                        label: "chrome",
                        call: name.starts_with(CHROME_IN_A_CALL),
                        name,
                        schedstat,
                    }),
            );
            continue;
        }
        let Some(w) = WATCHED.iter().find(|w| w.process == comm.trim()) else {
            continue;
        };
        if w.label == "niri" && compositor.is_some_and(|pid| name != pid.to_string()) {
            continue;
        }
        if w.thread.is_empty() {
            found.push(Thread {
                label: w.label,
                name: comm.trim().to_string(),
                schedstat: entry.path().join("schedstat"),
                call: false,
            });
            continue;
        }
        for (name, schedstat) in tasks(&entry.path()) {
            if name.starts_with(w.thread) {
                found.push(Thread {
                    label: w.label,
                    name,
                    schedstat,
                    call: false,
                });
            }
        }
    }
    if chrome.iter().any(|t| t.call) {
        found.extend(chrome);
    }
    apps.keep_only(&looked);
    found.sort();
    found
}

#[cfg(test)]
#[path = "kernel_tests.rs"]
pub mod tests;
