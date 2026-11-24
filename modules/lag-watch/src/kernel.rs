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
/// one, and how many times it ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sched {
    pub run_ns: u64,
    pub wait_ns: u64,
    pub slices: u64,
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
    })
}

/// How long a thread has been blocked in the kernel, in nanoseconds, from
/// its `sched`: asleep where nothing but the kernel could wake it, on a
/// lock, a page read back or reclaim, which no wait for a CPU shows. The
/// kernel keeps it only while `kernel.sched_schedstats` is on, and prints
/// it in milliseconds; none where it isn't kept.
pub fn blocked(text: &str) -> Option<u64> {
    sched_ns(text, "sum_block_runtime")
}

/// How much of a thread's time blocked in the kernel was spent waiting on
/// the disk, in nanoseconds, from its `sched`'s `iowait_sum`, so a block
/// reads apart as io or something else (a lock, reclaim); none where it
/// isn't kept.
pub fn io_waited(text: &str) -> Option<u64> {
    sched_ns(text, "iowait_sum")
}

/// The milliseconds the `sched` line keyed `name` holds, in nanoseconds.
/// The key is `name` itself, as kernels since 6.x print it, or `name` after
/// a dot, as `stats.` or `se.statistics.` before them; never another key
/// that only ends in it.
fn sched_ns(text: &str, name: &str) -> Option<u64> {
    let ms: f64 = text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = key.trim();
        let named = key.strip_suffix(name)?;
        (named.is_empty() || named.ends_with('.')).then(|| value.trim().parse().ok())?
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

pub fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
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
pub mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn a_pressure_file_gives_its_totals() {
        let text = "some avg10=1.20 avg60=0.80 avg300=0.50 total=123456\n\
                    full avg10=0.10 avg60=0.00 avg300=0.00 total=789\n";
        assert_eq!(
            stall(text),
            Stall {
                some: 123_456,
                full: 789
            }
        );
        assert_eq!(
            stall("some avg10=0.00 avg60=0.00 avg300=0.00 total=5\n").full,
            0
        );
    }

    #[test]
    fn a_keyed_file_gives_the_field_named_and_not_one_it_starts() {
        let text = "workingset_refault_anon 12\nworkingset_refault_anon_x 99\nanon 4096\n";
        assert_eq!(field(text, "workingset_refault_anon"), 12);
        assert_eq!(field(text, "anon"), 4096);
        assert_eq!(field(text, "file"), 0);
    }

    #[test]
    fn a_thread_is_asleep_unless_its_stat_says_it_runs_or_is_ready() {
        assert!(
            asleep("7 (Chrome_ChildIOT) S 1 7 7 0 -1\n"),
            "a sleeping thread counted as ready"
        );
        assert!(
            asleep("7 (a (b) c) D 1 7 7 0 -1\n"),
            "a thread in disk sleep, named with brackets, counted as ready"
        );
        assert!(
            !asleep("7 (CrBrowserMain) R 1 7 7 0 -1\n"),
            "a ready thread counted as asleep"
        );
        assert!(!asleep(""), "an unread stat counted as asleep");
    }

    #[test]
    fn a_schedstat_gives_run_wait_and_slices() {
        assert_eq!(
            sched("1621278039012 163383489014 6999273\n"),
            Some(Sched {
                run_ns: 1_621_278_039_012,
                wait_ns: 163_383_489_014,
                slices: 6_999_273
            })
        );
        assert_eq!(sched(""), None);
    }

    #[test]
    fn a_sched_file_gives_the_time_blocked_only_while_it_is_kept() {
        let kept = "Chrome_ChildIOT (7, #threads: 30)\n\
                    ----------------------------------------\n\
                    se.sum_exec_runtime          :          1.019739\n\
                    stats.sum_sleep_runtime      :       9000.000000\n\
                    stats.sum_block_runtime      :        842.500125\n\
                    stats.block_max              :        300.000000\n";
        assert_eq!(
            blocked(kept),
            Some(842_500_125),
            "the time blocked wasn't read from its line"
        );
        let unkept = "cat (7, #threads: 1)\n\
                      se.sum_exec_runtime          :          1.019739\n";
        assert_eq!(blocked(unkept), None, "a file without it read as blocked");
    }

    #[test]
    fn a_sched_file_gives_the_time_waited_on_the_disk_only_while_it_is_kept() {
        let kept = "chrome (7, #threads: 30)\n\
                    stats.sum_block_runtime      :        842.500125\n\
                    stats.iowait_sum             :        600.250000\n\
                    stats.iowait_count           :              12\n";
        assert_eq!(
            io_waited(kept),
            Some(600_250_000),
            "the time waited on the disk wasn't read from its line"
        );
        let unkept = "cat (7, #threads: 1)\n\
                      se.sum_exec_runtime          :          1.019739\n";
        assert_eq!(io_waited(unkept), None, "a file without it read as io");
    }

    #[test]
    fn a_sched_line_is_read_by_its_whole_key_bare_or_after_a_dot() {
        // A key only ending in the name is another key, wherever it sits.
        let text = "chrome (7, #threads: 30)\n\
                    stats.nr_iowait_sum          :       9999.000000\n\
                    sum_block_runtime            :        842.500125\n\
                    iowait_sum                   :        600.250000\n";
        assert_eq!(
            (blocked(text), io_waited(text)),
            (Some(842_500_125), Some(600_250_000)),
            "a key only ending in the name was read, or a bare key wasn't"
        );
    }

    /// Writes `text` to a stand-in kernel file, making its folders.
    pub fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn the_watched_threads_are_found_by_their_names() {
        let root = std::env::temp_dir().join(format!("lag-watch-proc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        write(&root.join("10/comm"), "niri\n");
        write(&root.join("10/schedstat"), "1 2 3\n");
        write(&root.join("20/comm"), "pipewire\n");
        write(&root.join("20/task/20/comm"), "pipewire\n");
        write(&root.join("20/task/21/comm"), "data-loop.0\n");
        write(&root.join("30/comm"), "chrome\n");
        write(&root.join("self/comm"), "niri\n");
        let found = seen(&root, None, &root.join("tuple.service"));
        assert_eq!(
            found,
            [
                self::found("niri", "niri", root.join("10/schedstat"), false),
                self::found(
                    "pipewire",
                    "data-loop.0",
                    root.join("20/task/21/schedstat"),
                    false
                )
            ]
        );
    }

    #[test]
    fn only_the_niri_drawing_the_desktop_is_watched_once_it_is_known() {
        let root = std::env::temp_dir().join(format!("lag-watch-nested-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        write(&root.join("10/comm"), "niri\n");
        write(&root.join("40/comm"), "niri\n");
        write(&root.join("50/comm"), "niri\n");
        let tuple = root.join("tuple.service");
        assert_eq!(
            seen(&root, Some(40), &tuple),
            [found("niri", "niri", root.join("40/schedstat"), false)]
        );
        assert_eq!(seen(&root, None, &tuple).len(), 3);
    }

    /// The account the tests' processes are Joel's in.
    pub const ACCOUNT: &str = "/user.slice/user-1000.slice/user@1000.service";

    /// A process named `comm`, in the scope of an app Joel opened.
    pub fn opened(proc: &Path, pid: &str, comm: &str) {
        write(&proc.join(pid).join("comm"), &format!("{comm}\n"));
        let scope = format!("{ACCOUNT}/app.slice/app-niri-a-70.scope");
        write(&proc.join(pid).join("cgroup"), &format!("0::{scope}\n"));
    }

    /// The threads found in a first look, with no app's process known yet.
    fn seen(proc: &Path, compositor: Option<u32>, unit: &Path) -> Vec<Thread> {
        threads(proc, compositor, unit, &mut Apps::of_account(ACCOUNT))
    }

    #[test]
    fn a_thread_s_nice_and_start_are_read_past_a_name_holding_spaces_and_brackets() {
        let stat = "72 (chrome (a) b) S 70 1 1 0 -1 4194560 1 0 0 0 5 6 0 0 25 5 30 0 99\n";
        assert_eq!(nice(stat), Some(5));
        assert_eq!(started(stat), Some(99));
        assert_eq!(
            nice("72 (chrome) S 70 1 1 0 -1 4194560 1 0 0 0 5 6 0 0 12 -8"),
            Some(-8)
        );
        assert_eq!(nice("72 (chrome) S 70"), None);
        assert_eq!(started("72 (chrome) S 70"), None);
        assert_eq!(nice(""), None);
    }

    fn found(label: &'static str, name: &str, schedstat: PathBuf, call: bool) -> Thread {
        Thread {
            label,
            name: name.to_string(),
            schedstat,
            call,
        }
    }

    #[test]
    fn a_call_s_threads_are_found_while_it_carries_one() {
        let root = std::env::temp_dir().join(format!("lag-watch-call-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let proc = root.join("proc");
        write(&root.join("tuple.service/cgroup.procs"), "60\n");
        write(&proc.join("60/comm"), ".tuple-wrapped\n");
        write(&proc.join("60/task/60/comm"), ".tuple-wrapped\n");
        write(&proc.join("60/task/61/comm"), "Signaling\n");
        write(&proc.join("60/task/62/comm"), "AudioEncoder\n");
        opened(&proc, "30", "chrome");
        write(&proc.join("30/task/31/comm"), "WebRTC_Network\n");
        write(&proc.join("30/task/32/comm"), "ThreadPoolAudio\n");
        write(&proc.join("30/task/33/comm"), "AudioOutputDevi\n");
        write(&proc.join("30/task/34/comm"), "ThreadPoolForeg\n");
        let task = |pid: &str, tid: &str| proc.join(format!("{pid}/task/{tid}/schedstat"));
        // A tab with a peer connection playing sound is no call of
        // Chrome's until a page records the microphone, in any of its
        // processes.
        let chrome = |proc: &Path| -> Vec<Thread> {
            seen(proc, None, &root.join("tuple.service"))
                .into_iter()
                .filter(|t| t.label == "chrome")
                .collect()
        };
        assert_eq!(chrome(&proc), []);
        // Nor while only a Chrome Joel didn't open records one, a test's.
        write(&proc.join("50/comm"), "chrome\n");
        write(
            &proc.join("50/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/call-under-load.service\n"),
        );
        write(&proc.join("50/task/51/comm"), "AudioInputDevic\n");
        assert_eq!(chrome(&proc), []);
        // Nor an agent's, in the jobs slice.
        write(&proc.join("55/comm"), "chrome\n");
        write(
            &proc.join("55/cgroup"),
            &format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n"),
        );
        write(&proc.join("55/task/56/comm"), "AudioInputDevic\n");
        assert_eq!(chrome(&proc), []);
        // A Chrome Joel started from a terminal carries his call, though
        // its drawing isn't read.
        write(&proc.join("45/comm"), "chrome\n");
        write(
            &proc.join("45/cgroup"),
            &format!("0::{ACCOUNT}/app.slice/kitty-70-0.scope\n"),
        );
        write(&proc.join("45/task/46/comm"), "WebRTC_Network\n");
        opened(&proc, "40", "chrome");
        write(&proc.join("40/task/41/comm"), "AudioInputDevic\n");
        assert_eq!(
            seen(&proc, None, &root.join("tuple.service")),
            [
                found("chrome", "AudioInputDevic", task("40", "41"), true),
                found("chrome", "AudioOutputDevi", task("30", "33"), false),
                found("chrome", "WebRTC_Network", task("30", "31"), false),
                found("chrome", "WebRTC_Network", task("45", "46"), false),
                found("tuple", ".tuple-wrapped", task("60", "60"), false),
                found("tuple", "AudioEncoder", task("60", "62"), true),
                found("tuple", "Signaling", task("60", "61"), false),
            ]
        );
        // Out of a call, neither has a thread worth reading.
        fs::remove_file(proc.join("60/task/62/comm")).unwrap();
        fs::remove_file(proc.join("40/task/41/comm")).unwrap();
        assert_eq!(seen(&proc, None, &root.join("tuple.service")), []);
        // Nor without Tuple at all.
        assert_eq!(seen(&proc, None, &root.join("gone.service")), []);
    }

    #[test]
    fn a_call_s_threads_count_only_while_one_carrying_it_is_read() {
        let at = |n: &str| PathBuf::from(format!("/proc/{n}/schedstat"));
        let mut read = BTreeMap::new();
        read.insert(
            found("tuple", "Signaling", at("61"), false),
            Sched::default(),
        );
        read.insert(found("niri", "niri", at("10"), false), Sched::default());
        read.insert(
            found("chrome", "AudioThread", at("31"), true),
            Sched::default(),
        );
        while_in_a_call(&mut read);
        let labels: Vec<&str> = read.keys().map(|t| t.label).collect();
        assert_eq!(labels, ["chrome", "niri"]);
    }

    // A stand-in for Tuple's daemon: this test's own process, whose thread
    // names itself as WebRTC's encoder does (prctl's PR_SET_NAME, which
    // std's thread names are set by), read from the kernel's own /proc.
    #[test]
    fn a_stand_in_s_named_threads_are_read_from_the_kernel() {
        let root = std::env::temp_dir().join(format!("lag-watch-stand-in-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        write(
            &root.join("tuple.service/cgroup.procs"),
            &format!("{}\n", std::process::id()),
        );
        let (named, go) = std::sync::mpsc::channel();
        let (stop, stopped) = std::sync::mpsc::channel::<()>();
        let encoder = std::thread::Builder::new()
            .name("AudioEncoder".into())
            .spawn(move || {
                named.send(()).unwrap();
                let _ = stopped.recv();
            })
            .unwrap();
        go.recv().unwrap();
        let found = seen(Path::new("/proc"), None, &root.join("tuple.service"));
        let carrying = found
            .iter()
            .find(|t| t.label == "tuple" && t.name == "AudioEncoder")
            .expect("the encoder is found by its name");
        let read = sched(&text(&carrying.schedstat));
        stop.send(()).unwrap();
        encoder.join().unwrap();
        assert!(carrying.call);
        assert!(read.is_some_and(|s| s.slices > 0), "{read:?}");
        assert!(found.iter().filter(|t| t.label == "tuple").count() > 1);
    }

    #[test]
    fn a_cgroup_is_read_whole_or_not_at_all() {
        let root = std::env::temp_dir().join(format!("lag-watch-cg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        write(
            &root.join("app.slice/cpu.pressure"),
            "some avg10=0 avg60=0 avg300=0 total=10\n",
        );
        write(&root.join("app.slice/memory.swap.current"), "4096\n");
        write(
            &root.join("app.slice/memory.stat"),
            "workingset_refault_anon 3\nworkingset_refault_file 4\n",
        );
        write(
            &root.join("app.slice/memory.events"),
            "low 0\nhigh 0\nmax 0\noom 2\noom_kill 1\noom_group_kill 0\n",
        );
        let g = group(&root.join("app.slice")).unwrap();
        assert_eq!((g.cpu.some, g.swap, g.refaults, g.killed), (10, 4096, 7, 1));
        assert_eq!(group(&root.join("gone.slice")), None);
    }
}
