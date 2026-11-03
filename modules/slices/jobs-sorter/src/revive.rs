//! Brings back the kernel's reclaim thread, kswapd, after it gives up.
//!
//! kswapd gives up on a node after 16 runs that reclaim nothing, and from
//! then on every program that needs a page reclaims it itself, stalling
//! as it does. Since 7.2 the give-up is cleared only once the node is
//! balanced, its free memory over the high watermark, so that throttling
//! at a cgroup's `memory.high` can't keep reviving a kswapd that can't
//! balance. But a program reclaims only what it needs, which keeps free
//! memory near the min watermark, under high, so a give-up outlasts
//! whatever caused it, until a lump of memory comes free at once. On 2
//! November one ended in the OOM killer with 20 GB of clean file cache
//! and 117 GB of swap free.
//!
//! So while the node has given up, this asks the jobs slice, agents' work,
//! to give back [`ASK`] through its `memory.reclaim`, more than the high
//! watermark of about 700 MB: reclaim that makes progress clears the
//! give-up once the node is balanced. It asks at most once every
//! [`SPACED`], so a give-up that lasts doesn't empty the jobs slice.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// What the jobs slice is asked to give back.
pub const ASK: &str = "1G";

/// The least time between two asks while a give-up lasts.
pub const SPACED: Duration = Duration::from_secs(5);

/// How often the give-up is looked for.
pub const EVERY: Duration = Duration::from_secs(1);

/// Whether any node in `zoneinfo`, the text of /proc/zoneinfo, has given
/// up reclaiming.
pub fn hopeless(zoneinfo: &str) -> bool {
    zoneinfo.lines().any(|line| {
        line.trim()
            .strip_prefix("node_unreclaimable:")
            .is_some_and(|flag| flag.trim() != "0")
    })
}

/// Asks of the jobs slice while a node has given up.
pub struct Reviver {
    reclaim: PathBuf,
    asked: Option<Instant>,
}

impl Reviver {
    /// A reviver asking of the cgroup at `jobs`.
    pub fn new(jobs: &Path) -> Reviver {
        Reviver {
            reclaim: jobs.join("memory.reclaim"),
            asked: None,
        }
    }

    /// Looks at `zoneinfo` at `now` and asks the jobs slice for [`ASK`] if
    /// a node has given up and the last ask is [`SPACED`] ago or more.
    /// Says whether it asked.
    pub fn step(&mut self, zoneinfo: &str, now: Instant) -> bool {
        if !hopeless(zoneinfo)
            || self
                .asked
                .is_some_and(|asked| now.duration_since(asked) < SPACED)
        {
            return false;
        }
        self.asked = Some(now);
        // A reclaim that falls short of the ask says so, having still
        // given back what it could, which is what clears the give-up.
        match fs::write(&self.reclaim, ASK) {
            Ok(()) => eprintln!(
                "kswapd gave up reclaiming, so the jobs slice gave back {ASK} to bring it back."
            ),
            Err(error) => eprintln!(
                "kswapd gave up reclaiming, and the jobs slice gave back less than {ASK}: {error}"
            ),
        }
        true
    }
}

/// Revives kswapd for as long as the program runs, on a thread of its own,
/// since an ask can take a second.
pub fn watch(jobs: &Path) {
    let mut reviver = Reviver::new(jobs);
    thread::spawn(move || {
        loop {
            if let Ok(zoneinfo) = fs::read_to_string("/proc/zoneinfo") {
                reviver.step(&zoneinfo, Instant::now());
            }
            thread::sleep(EVERY);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of the test's own, standing in for the jobs slice.
    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("revive-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// /proc/zoneinfo's shape, cut to what it says of a node's give-up.
    fn zoneinfo(given_up: u8) -> String {
        format!(
            "Node 0, zone      DMA\n  per-node stats\n      nr_inactive_anon 3890471\n  \
             node_unreclaimable:  {given_up}\n  start_pfn:           1\n  pages free     2785\n        \
             min      3\n        high     41\nNode 0, zone   Normal\n  pages free     19119\n        \
             min      16467\n        high     175653\n  node_unreclaimable:  {given_up}\n"
        )
    }

    #[test]
    fn a_node_that_gave_up_is_seen() {
        assert!(hopeless(&zoneinfo(1)));
        assert!(!hopeless(&zoneinfo(0)));
    }

    #[test]
    fn a_give_up_asks_the_jobs_slice_for_memory_back() {
        let folder = folder("asks");
        let mut reviver = Reviver::new(folder.as_path());
        assert!(reviver.step(&zoneinfo(1), Instant::now()));
        assert_eq!(
            fs::read_to_string(folder.as_path().join("memory.reclaim")).unwrap(),
            ASK
        );
    }

    #[test]
    fn nothing_is_asked_while_reclaim_goes_on() {
        let folder = folder("quiet");
        let mut reviver = Reviver::new(folder.as_path());
        assert!(!reviver.step(&zoneinfo(0), Instant::now()));
        assert!(!folder.as_path().join("memory.reclaim").exists());
    }

    #[test]
    fn a_give_up_that_lasts_is_asked_of_again_only_once_spaced() {
        let folder = folder("spaced");
        let mut reviver = Reviver::new(folder.as_path());
        let began = Instant::now();
        assert!(reviver.step(&zoneinfo(1), began));
        assert!(!reviver.step(&zoneinfo(1), began + Duration::from_secs(1)));
        assert!(reviver.step(&zoneinfo(1), began + SPACED));
    }
}
