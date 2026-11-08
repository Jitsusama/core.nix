//! What the kernel says, read from its files: how long a cgroup's tasks
//! stalled, what its memory did, and how long a thread waited to run.
//! Each reader takes the file's text, so the tests need no kernel.

use std::fs;
use std::path::{Path, PathBuf};

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

pub fn sched(text: &str) -> Option<Sched> {
    let mut n = text.split_whitespace().map(|w| w.parse::<u64>().ok());
    Some(Sched {
        run_ns: n.next()??,
        wait_ns: n.next()??,
        slices: n.next()??,
    })
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

fn text(path: &Path) -> String {
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
/// `ThreadPoolAudio` is left out: every process keeps one, idle.
pub const CHROME_CALL_THREADS: [&str; 6] = [
    "WebRTC_",
    "AudioOutputDevi",
    "AudioInputDevic",
    "AudioThread",
    "AudioWorkletThr",
    "RealtimeAudioWo",
];

/// A process's threads, by name and schedstat path.
fn tasks(process: &Path) -> Vec<(String, PathBuf)> {
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
/// threads would only say how an idle daemon waits.
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
pub fn threads(proc: &Path, compositor: Option<u32>, tuple_unit: &Path) -> Vec<Thread> {
    let mut found = tuple(proc, tuple_unit);
    let Ok(entries) = fs::read_dir(proc) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let comm = text(&entry.path().join("comm"));
        if comm.trim() == "chrome" {
            found.extend(
                tasks(&entry.path())
                    .into_iter()
                    .filter(|(name, _)| CHROME_CALL_THREADS.iter().any(|c| name.starts_with(c)))
                    .map(|(name, schedstat)| Thread {
                        label: "chrome",
                        name,
                        schedstat,
                        call: true,
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
    found.sort();
    found
}

#[cfg(test)]
mod tests {
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

    fn write(path: &Path, text: &str) {
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
        let found = threads(&root, None, &root.join("tuple.service"));
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
            threads(&root, Some(40), &tuple),
            [found("niri", "niri", root.join("40/schedstat"), false)]
        );
        assert_eq!(threads(&root, None, &tuple).len(), 3);
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
        write(&proc.join("30/comm"), "chrome\n");
        write(&proc.join("30/task/31/comm"), "WebRTC_Network\n");
        write(&proc.join("30/task/32/comm"), "ThreadPoolAudio\n");
        write(&proc.join("30/task/33/comm"), "AudioOutputDevi\n");
        write(&proc.join("30/task/34/comm"), "Compositor\n");
        let task = |pid: &str, tid: &str| proc.join(format!("{pid}/task/{tid}/schedstat"));
        assert_eq!(
            threads(&proc, None, &root.join("tuple.service")),
            [
                found("chrome", "AudioOutputDevi", task("30", "33"), true),
                found("chrome", "WebRTC_Network", task("30", "31"), true),
                found("tuple", ".tuple-wrapped", task("60", "60"), false),
                found("tuple", "AudioEncoder", task("60", "62"), true),
                found("tuple", "Signaling", task("60", "61"), false),
            ]
        );
        // Out of a call, neither has a thread worth reading.
        fs::remove_file(proc.join("60/task/62/comm")).unwrap();
        fs::remove_file(proc.join("30/task/31/comm")).unwrap();
        fs::remove_file(proc.join("30/task/33/comm")).unwrap();
        assert_eq!(threads(&proc, None, &root.join("tuple.service")), []);
        // Nor without Tuple at all.
        assert_eq!(threads(&proc, None, &root.join("gone.service")), []);
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
        let found = threads(Path::new("/proc"), None, &root.join("tuple.service"));
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
