//! The disks as the disk guard says them: each guard, the system's and the
//! account's, writes its status every minute or so, saying each
//! filesystem's free space and band and when it last beat. The day keeps
//! each filesystem's least free space and gravest band, and counts a guard
//! found stopped once, as a service's fault, until it beats again.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How old a guard's heartbeat may be before it's taken to have stopped,
/// in seconds, as `disk-guard status` takes it.
const STOPPED_AFTER: f64 = 120.0;

/// The bands, least grave first, by the words the guard writes them in.
const BANDS: [&str; 4] = ["healthy", "filling", "nearly-full", "critically-full"];

/// Each guard's status file: the account's, under its `runtime` folder
/// when it has one, then the system's, `system`.
#[must_use]
pub fn statuses(runtime: Option<&Path>, system: PathBuf) -> Vec<PathBuf> {
    runtime
        .map(|runtime| runtime.join("disk-guard/status.json"))
        .into_iter()
        .chain([system])
        .collect()
}

/// What a status file says, as far as the day reads it.
#[derive(Deserialize)]
struct Status {
    heartbeat: f64,
    #[serde(default)]
    filesystems: Vec<Said>,
}

#[derive(Deserialize)]
struct Said {
    path: String,
    band: String,
    free: u64,
    size: u64,
}

/// One filesystem over the day.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filesystem {
    /// Its size, in bytes.
    pub size: u64,
    /// The least free it was, in bytes.
    pub least_free: u64,
    /// When, in microseconds since the epoch.
    pub least_at: i64,
    /// Its gravest band.
    pub band: String,
}

impl Filesystem {
    /// How much of it was in use at most, in percent.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "a share only needs to be near")]
    pub fn most_used_percent(&self) -> f64 {
        if self.size == 0 {
            0.0
        } else {
            (self.size - self.least_free.min(self.size)) as f64 * 100.0 / self.size as f64
        }
    }
}

/// One look at the guards: each filesystem they said, and each guard's
/// status file that says it stopped.
#[derive(Debug, Default)]
pub struct Look {
    said: Vec<(String, String, u64, u64)>,
    /// The status files of guards whose heartbeat is too old.
    pub stopped: Vec<PathBuf>,
}

/// Look at each status in `statuses` at `at`, in microseconds since the
/// epoch. A status that isn't there is a guard this machine doesn't run,
/// and what a stopped one says is as old as its heartbeat, so left out.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a heartbeat's age is judged in seconds"
)]
pub fn look(statuses: &[PathBuf], at: i64) -> Look {
    let now = at as f64 / 1e6;
    let mut look = Look::default();
    for path in statuses {
        let Some(status) = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Status>(&bytes).ok())
        else {
            continue;
        };
        if now - status.heartbeat > STOPPED_AFTER {
            look.stopped.push(path.clone());
            continue;
        }
        for said in status.filesystems {
            look.said.push((said.path, said.band, said.free, said.size));
        }
    }
    look
}

/// The day's disks.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Disks {
    /// Each filesystem, by its mount.
    pub filesystems: BTreeMap<String, Filesystem>,
}

impl Disks {
    /// Keep what a look at `at` said of each filesystem.
    pub fn saw(&mut self, at: i64, look: &Look) {
        for (path, band, free, size) in &look.said {
            let kept = self.filesystems.entry(path.clone()).or_default();
            if kept.size == 0 || *free < kept.least_free {
                kept.least_free = *free;
                kept.least_at = at;
            }
            kept.size = *size;
            if grave(band) > grave(&kept.band) {
                kept.band.clone_from(band);
            }
        }
    }

    /// The most in use of any filesystem, in percent.
    #[must_use]
    pub fn most_used_percent(&self) -> f64 {
        self.filesystems
            .values()
            .map(Filesystem::most_used_percent)
            .fold(0.0, f64::max)
    }
}

/// The day's disks in a line, when any was seen: each past healthy at
/// its gravest, else the most used.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "a size only needs to be near")]
pub fn said(disks: &Disks) -> Option<String> {
    let gib = |bytes: u64| format!("{:.1} GiB", bytes as f64 / f64::from(1u32 << 30));
    let past: Vec<String> = disks
        .filesystems
        .iter()
        .filter(|(_, kept)| grave(&kept.band) > 0)
        .map(|(path, kept)| {
            format!(
                "{path} {}, {} free at least",
                kept.band,
                gib(kept.least_free)
            )
        })
        .collect();
    if !past.is_empty() {
        return Some(past.join("; "));
    }
    let (path, most) = disks
        .filesystems
        .iter()
        .max_by(|a, b| a.1.most_used_percent().total_cmp(&b.1.most_used_percent()))?;
    Some(format!(
        "all healthy; the fullest, {path}, {:.0}% used at most",
        most.most_used_percent()
    ))
}

/// How grave a band is; one the day doesn't know is as grave as none.
fn grave(band: &str) -> usize {
    BANDS.iter().position(|known| *known == band).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(into: &Path, heartbeat: f64, free: u64, band: &str) -> PathBuf {
        let path = into.join(format!("{heartbeat}.json"));
        fs::write(
            &path,
            format!(
                r#"{{"heartbeat": {heartbeat}, "says": "", "filesystems": [
                    {{"path": "/", "band": "{band}", "free": {free}, "size": 1000}}]}}"#
            ),
        )
        .expect("a status written");
        path
    }

    #[test]
    fn a_disk_keeps_its_least_free_and_gravest_band() {
        let into = tempfile::tempdir().expect("a folder");
        let mut disks = Disks::default();
        let at = |seconds: i64| seconds * 1_000_000;
        disks.saw(
            1,
            &look(&[status(into.path(), 100.0, 300, "filling")], at(100)),
        );
        disks.saw(
            2,
            &look(&[status(into.path(), 160.0, 50, "nearly-full")], at(160)),
        );
        disks.saw(
            3,
            &look(&[status(into.path(), 220.0, 400, "healthy")], at(220)),
        );
        let root = &disks.filesystems["/"];
        assert_eq!(
            (root.least_free, root.least_at, root.band.as_str()),
            (50, 2, "nearly-full"),
            "the day's least free space or gravest band was lost to a later look"
        );
        assert!(
            (disks.most_used_percent() - 95.0).abs() < 1e-9,
            "the most in use was {}",
            disks.most_used_percent()
        );
    }

    #[test]
    fn a_disk_past_healthy_is_said_before_the_fullest() {
        let mut disks = Disks::default();
        let kept = |least_free, band: &str| Filesystem {
            size: 50 << 30,
            least_free,
            least_at: 1,
            band: band.into(),
        };
        disks
            .filesystems
            .insert("/".into(), kept(40 << 30, "healthy"));
        assert_eq!(
            said(&disks).as_deref(),
            Some("all healthy; the fullest, /, 20% used at most"),
            "a healthy day's disks in words"
        );
        disks
            .filesystems
            .insert("/var/cache/builds".into(), kept(14 << 30, "filling"));
        assert_eq!(
            said(&disks).as_deref(),
            Some("/var/cache/builds filling, 14.0 GiB free at least"),
            "a filling disk wasn't said first"
        );
    }

    #[test]
    fn a_guard_whose_heart_stopped_is_said_and_one_not_there_isnt() {
        let into = tempfile::tempdir().expect("a folder");
        let old = status(into.path(), 100.0, 300, "healthy");
        let missing = into.path().join("none.json");
        // A heartbeat 121 s old, past the 120 s a guard may go quiet.
        let looked = look(&[old.clone(), missing], 221_000_000);
        assert_eq!(
            looked.stopped,
            vec![old],
            "a stale guard wasn't said stopped, or a missing one was"
        );
        assert!(
            looked.said.is_empty(),
            "a stopped guard's old word was kept"
        );
    }
}
