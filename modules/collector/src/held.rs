//! What the slices with memory floors hold, kept every ten seconds as a
//! span, so their floors can be sized to what they use rather than to a
//! share of memory.
//!
//! On 6 November app.slice held 5.7 GB in memory under a 6.2 GB hard
//! floor, 2.9 of it in active use, and agents.slice 10.5 under 9.3: the
//! floors were set as shares of memory, and nothing kept what the slices
//! held over a day to say what they should be. Each reading says, in
//! bytes, what the slice's memory.stat counts as anonymous memory and
//! file cache, how much of each the kernel last found in use, what it
//! holds in swap and the floor it has.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::otlp::Span;
use crate::pressure::{manager, now, random};
use crate::store::Store;

/// How often each slice is read.
const EVERY: Duration = Duration::from_secs(10);

/// The slices with floors of their own, below the account's manager.
const FLOORED: [&str; 3] = ["session.slice", "app.slice", "agents.slice"];

/// What one slice holds, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held {
    pub anon: u64,
    pub active_anon: u64,
    pub file: u64,
    pub active_file: u64,
    pub swap: u64,
    pub floor: u64,
}

/// The places read, each by its name and folder: the account's manager,
/// whose floor covers the rest, and each floored slice below it, found
/// from `cgroup`, this process's line of /proc/self/cgroup.
pub fn places(cgroups: &Path, cgroup: &str) -> Vec<(String, PathBuf)> {
    let Some(manager) = manager(cgroup) else {
        return Vec::new();
    };
    let manager = cgroups.join(manager);
    let mut found = vec![("the account".to_owned(), manager.clone())];
    found.extend(FLOORED.map(|slice| (slice.to_owned(), manager.join(slice))));
    found
}

/// What the folder of one cgroup says it holds, or nothing when it has no
/// memory controller.
pub fn held(dir: &Path) -> Option<Held> {
    let stat = fs::read_to_string(dir.join("memory.stat")).ok()?;
    let count = |name: &str| {
        stat.lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(' '))
            .and_then(|bytes| bytes.trim().parse().ok())
    };
    let bytes = |file: &str| {
        fs::read_to_string(dir.join(file))
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0)
    };
    Some(Held {
        anon: count("anon")?,
        active_anon: count("active_anon").unwrap_or(0),
        file: count("file")?,
        active_file: count("active_file").unwrap_or(0),
        swap: bytes("memory.swap.current"),
        floor: bytes("memory.min"),
    })
}

/// The span a reading is kept as: a moment, in a trace of its own.
pub fn span(place: &str, held: &Held, at: i64, ids: (String, String)) -> Span {
    Span {
        trace: ids.0,
        id: ids.1,
        parent: None,
        name: format!("memory held by {place}"),
        service: "kernel".into(),
        start: at,
        end: at,
        status: None,
        attributes: Map::<String, Value>::from_iter([
            ("anon_bytes".into(), json!(held.anon)),
            ("active_anon_bytes".into(), json!(held.active_anon)),
            ("file_bytes".into(), json!(held.file)),
            ("active_file_bytes".into(), json!(held.active_file)),
            ("swap_bytes".into(), json!(held.swap)),
            ("floor_bytes".into(), json!(held.floor)),
        ]),
    }
}

/// Reads each place now and every ten seconds after, keeping what each
/// holds.
pub fn run(places: &[(String, PathBuf)], store: &Mutex<Store>) -> ! {
    loop {
        let at = now();
        let spans: Vec<Span> = places
            .iter()
            .filter_map(|(place, dir)| Some(span(place, &held(dir)?, at, (random(16), random(8)))))
            .collect();
        if !spans.is_empty() {
            let mut store = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = store.add(&spans, at) {
                eprintln!("{e}");
            }
        }
        thread::sleep(EVERY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "anon 6120587264\nfile 644874240\nkernel 118947840\n\
                        anon_thp 0\ninactive_anon 3006480384\nactive_anon 3114106880\n\
                        inactive_file 430252032\nactive_file 214622208\n";

    #[test]
    fn a_slice_says_what_it_holds_in_memory_and_swap_and_its_floor() {
        let dir = tempfile::tempdir().expect("a folder");
        fs::write(dir.path().join("memory.stat"), STAT).unwrap();
        fs::write(dir.path().join("memory.swap.current"), "21367615488\n").unwrap();
        fs::write(dir.path().join("memory.min"), "6690131968\n").unwrap();
        assert_eq!(
            held(dir.path()),
            Some(Held {
                anon: 6_120_587_264,
                active_anon: 3_114_106_880,
                file: 644_874_240,
                active_file: 214_622_208,
                swap: 21_367_615_488,
                floor: 6_690_131_968,
            })
        );
    }

    // anon_thp starts with "anon" too, and must not be read for it.
    #[test]
    fn a_count_is_read_by_its_whole_name() {
        let dir = tempfile::tempdir().expect("a folder");
        fs::write(
            dir.path().join("memory.stat"),
            "anon_thp 7\nanon 5\nfile 3\n",
        )
        .unwrap();
        let held = held(dir.path()).expect("a reading");
        assert_eq!((held.anon, held.file, held.swap, held.floor), (5, 3, 0, 0));
    }

    #[test]
    fn a_cgroup_without_memory_says_nothing() {
        let dir = tempfile::tempdir().expect("a folder");
        assert_eq!(held(dir.path()), None);
    }

    #[test]
    fn the_account_and_its_floored_slices_are_read() {
        let cgroup =
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/collector.service\n";
        let places = places(Path::new("/c"), cgroup);
        let names: Vec<_> = places.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["the account", "session.slice", "app.slice", "agents.slice"]
        );
        assert_eq!(
            places[2].1,
            Path::new("/c/user.slice/user-1000.slice/user@1000.service/app.slice")
        );
    }

    #[test]
    fn a_reading_is_kept_as_a_moment_by_the_kernel() {
        let held = Held {
            anon: 1,
            active_anon: 2,
            file: 3,
            active_file: 4,
            swap: 5,
            floor: 6,
        };
        let kept = span("app.slice", &held, 42, ("t".into(), "s".into()));
        assert_eq!(kept.name, "memory held by app.slice");
        assert_eq!((kept.start, kept.end), (42, 42));
        assert_eq!(kept.service, "kernel");
        assert_eq!(kept.attributes["swap_bytes"], json!(5));
        assert_eq!(kept.attributes["floor_bytes"], json!(6));
    }
}
