//! What each access point of a trusted network is kept off for, and how
//! many times running its trials went back, kept on the disk so a restart
//! of wifid, or a resume, carries them to the card's next chooser
//! (choose.rs).
//!
//! A chooser alone forgets both whenever it's begun again, and on 25 Nov
//! 2026 that was twice a night: ch52's radio carried 2450.1 ms at p95 with
//! 14.6% lost on its trial at 01:42 and was kept off for an hour, the
//! first keep-off, as wifid had restarted at 22:47 Tue; it restarted again
//! at 06:51, and at 07:59 the radio was tried afresh as untried and kept
//! 14 minutes until it carried 633.9 ms. Kept here, each back counts
//! towards the longer keep-offs `SPURNED_MOST` allows, whatever starts in
//! between.
//!
//! Times are kept in the machine's own time, as an `Instant` means nothing
//! to the next wifid, so a suspend counts against a keep-off read back
//! after a resume, where a chooser running on counts awake time alone. A
//! clock set back or forward reads a keep-off shorter or longer by as much,
//! which the chooser caps at `SPURNED_MOST` (`Chooser::carry_in`).
//!
//! Only a trusted network's are kept, as which access points a machine was
//! near, and so where it has been, is said only to one trusted to know the
//! machine (learn.rs, network.rs). Any other's go with the chooser, as
//! before.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The file in wifid's state folder the keep-offs are kept in.
pub const FILE: &str = "kept-off";

/// An access point kept off or struck, as the disk keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spurn {
    pub point: String,
    pub kept: Option<Kept>,
    pub struck: Option<Struck>,
}

/// Until when an access point is kept off, and until when against a
/// failing link, in milliseconds since 1970, and the point whose trial
/// earned it, which alone frees it early.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    pub until: u64,
    pub firm: u64,
    pub from: Option<String>,
}

/// How many times running an access point's trials went back, and until
/// when the last kept it off, in milliseconds since 1970.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Struck {
    pub times: u32,
    pub until: u64,
}

/// One access point's spurn, by its network's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    network: String,
    #[serde(flatten)]
    spurn: Spurn,
}

/// What every trusted network's access points are kept off for.
pub struct KeptOff {
    file: PathBuf,
    entries: Vec<Entry>,
}

impl KeptOff {
    /// What was kept off before, kept in this file. One that can't be read
    /// is begun again empty, which leaves each point untried, as a chooser
    /// begun afresh always has.
    pub fn kept_in(file: &Path) -> KeptOff {
        let kept = fs::read(file).ok();
        let entries = kept
            .and_then(|kept| serde_json::from_slice(&kept).ok())
            .unwrap_or_default();
        KeptOff {
            file: file.to_path_buf(),
            entries,
        }
    }

    /// What this network's access points were kept off for.
    pub fn of(&self, network: &str) -> Vec<Spurn> {
        self.entries
            .iter()
            .filter(|entry| entry.network == network)
            .map(|entry| entry.spurn.clone())
            .collect()
    }

    /// Keeps what a network's access points are kept off for now in place
    /// of what was, on the disk where the network is trusted. Another's
    /// are never written, and any kept from when it was trusted go.
    pub fn keep(&mut self, network: &str, trusted: bool, spurns: Vec<Spurn>) -> io::Result<()> {
        let kept: Vec<Entry> = if trusted {
            spurns
                .into_iter()
                .map(|spurn| Entry {
                    network: network.to_owned(),
                    spurn,
                })
                .collect()
        } else {
            Vec::new()
        };
        let before: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|entry| entry.network == network)
            .collect();
        if before.len() == kept.len() && before.iter().zip(&kept).all(|(was, is)| *was == is) {
            return Ok(());
        }
        self.entries.retain(|entry| entry.network != network);
        self.entries.extend(kept);
        self.write()
    }

    /// Forgets what the access points of every network but these are kept
    /// off for.
    pub fn keep_only(&mut self, names: &[String]) -> io::Result<()> {
        let before = self.entries.len();
        self.entries.retain(|entry| names.contains(&entry.network));
        if self.entries.len() == before {
            return Ok(());
        }
        self.write()
    }

    /// Writes what's kept whole or not at all, readable by wifid alone.
    fn write(&self) -> io::Result<()> {
        let partial = self.file.with_extension("partial");
        let mut written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        written.write_all(&serde_json::to_vec(&self.entries)?)?;
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o600))?;
        fs::rename(partial, &self.file)
    }
}

/// A time of the machine's own, in milliseconds since 1970; none before.
pub fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| {
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    })
}

/// A time in milliseconds since 1970, as the machine's own.
pub fn time_of(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALL: &str = "02:00:00:00:01:00";
    const DESK: &str = "02:00:00:00:02:00";

    fn spurn(point: &str, times: u32) -> Spurn {
        Spurn {
            point: point.to_owned(),
            kept: Some(Kept {
                until: 1_000_000,
                firm: 900_000,
                from: Some(DESK.to_owned()),
            }),
            struck: Some(Struck {
                times,
                until: 1_000_000,
            }),
        }
    }

    #[test]
    fn a_trusted_network_s_keep_offs_are_kept_for_when_wifid_next_starts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut kept_off = KeptOff::kept_in(&file);

        kept_off.keep("Home", true, vec![spurn(HALL, 2)]).unwrap();

        assert_eq!(KeptOff::kept_in(&file).of("Home"), vec![spurn(HALL, 2)]);
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600,
            "the keep-offs are readable by others than wifid"
        );
    }

    #[test]
    fn another_network_s_keep_offs_are_never_written_and_go_once_it_isn_t_trusted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut kept_off = KeptOff::kept_in(&file);

        kept_off
            .keep("Roadside Inn", false, vec![spurn(HALL, 1)])
            .unwrap();
        assert!(
            !file.exists(),
            "an untrusted network's keep-offs were written"
        );

        kept_off.keep("Home", true, vec![spurn(HALL, 1)]).unwrap();
        kept_off.keep("Home", false, vec![spurn(HALL, 1)]).unwrap();
        assert_eq!(KeptOff::kept_in(&file).of("Home"), Vec::new());
    }

    #[test]
    fn each_network_s_keep_offs_are_kept_apart() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut kept_off = KeptOff::kept_in(&file);

        kept_off.keep("Home", true, vec![spurn(HALL, 1)]).unwrap();
        kept_off.keep("Cabin", true, vec![spurn(DESK, 3)]).unwrap();
        kept_off.keep("Home", true, Vec::new()).unwrap();

        let read = KeptOff::kept_in(&file);
        assert_eq!(read.of("Home"), Vec::new());
        assert_eq!(read.of("Cabin"), vec![spurn(DESK, 3)]);
    }

    #[test]
    fn keep_offs_unchanged_are_not_written_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut kept_off = KeptOff::kept_in(&file);
        kept_off.keep("Home", true, vec![spurn(HALL, 1)]).unwrap();
        fs::remove_file(&file).unwrap();

        kept_off.keep("Home", true, vec![spurn(HALL, 1)]).unwrap();

        assert!(!file.exists(), "the same keep-offs were written again");
    }

    #[test]
    fn a_forgotten_network_s_keep_offs_go() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut kept_off = KeptOff::kept_in(&file);
        kept_off.keep("Home", true, vec![spurn(HALL, 1)]).unwrap();
        kept_off.keep("Cabin", true, vec![spurn(DESK, 1)]).unwrap();

        kept_off.keep_only(&["Cabin".to_owned()]).unwrap();

        let read = KeptOff::kept_in(&file);
        assert_eq!(read.of("Home"), Vec::new());
        assert_eq!(read.of("Cabin"), vec![spurn(DESK, 1)]);
    }

    #[test]
    fn a_file_that_can_t_be_read_is_begun_again_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        fs::write(&file, b"{ not what wifid wrote").unwrap();

        assert_eq!(KeptOff::kept_in(&file).of("Home"), Vec::new());
    }

    #[test]
    fn a_time_is_kept_to_the_millisecond() {
        let time = UNIX_EPOCH + Duration::from_millis(1_795_630_401_123);

        assert_eq!(time_of(millis(time)), time);
        assert_eq!(millis(UNIX_EPOCH - Duration::from_secs(1)), 0);
    }
}
