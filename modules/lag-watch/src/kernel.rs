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

/// Every thread of the watched kinds now running under `proc`, by label,
/// with their schedstat paths.
pub fn threads(proc: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(proc) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let comm = text(&entry.path().join("comm"));
        let Some(w) = WATCHED.iter().find(|w| w.process == comm.trim()) else {
            continue;
        };
        if w.thread.is_empty() {
            found.push((w.label, entry.path().join("schedstat")));
            continue;
        }
        let Ok(tasks) = fs::read_dir(entry.path().join("task")) else {
            continue;
        };
        for task in tasks.flatten() {
            if text(&task.path().join("comm")).trim().starts_with(w.thread) {
                found.push((w.label, task.path().join("schedstat")));
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let found = threads(&root);
        assert_eq!(
            found,
            [
                ("niri", root.join("10/schedstat")),
                ("pipewire", root.join("20/task/21/schedstat"))
            ]
        );
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
        let g = group(&root.join("app.slice")).unwrap();
        assert_eq!((g.cpu.some, g.swap, g.refaults), (10, 4096, 7));
        assert_eq!(group(&root.join("gone.slice")), None);
    }
}
