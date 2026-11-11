//! What the desktop's watchers say of themselves, each in a status file
//! it writes whole and renames into place: lag-watch, of whether what Joel
//! uses waits (`status:lag-watch`), and the disk guards, the account's
//! (`status:disk-guard`) and the machine's (`status:disk-guard-machine`),
//! of the disks. Each is told as a record holding what its file says and
//! whether the watcher has gone silent, so the shell reads no file and
//! keeps no timer to say a watcher stopped.
//!
//! Each file carries a heartbeat, the time it was written: a disk guard's
//! `heartbeat` and `at`, rewritten when nothing else is, and lag-watch's
//! window's `ended`. A watcher is silent once no heartbeat has come for
//! its silence, judged by the machine's running time from the read that
//! heard the heartbeat, which a suspend stops as it stops the watcher, so
//! waking gives it as long again. The heartbeat that says only when is
//! left out of the record, so a write that changes nothing else tells
//! nobody anything.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::Root;

/// The name every watcher gives its status file.
pub const FILE: &str = "status.json";
/// The keys a disk guard rewrites each time that say only when it wrote.
const BEATS: [&str; 2] = ["heartbeat", "at"];

/// A watcher whose status file is read.
#[derive(Clone, Debug)]
pub struct Watcher {
    /// The name its key says, `status:<name>`.
    pub name: &'static str,
    /// The folder its file is written in, on the machine.
    pub folder: String,
    /// How long without a heartbeat it's taken as gone.
    silence: Duration,
    /// The key its file's heartbeat is under, in seconds since the epoch.
    beat: &'static str,
}

/// The watchers of a session with this runtime folder: the account's only
/// with one, the machine's guard always.
pub fn watchers(runtime: Option<&str>) -> Vec<Watcher> {
    let mut watchers = Vec::new();
    if let Some(runtime) = runtime.filter(|runtime| !runtime.is_empty()) {
        watchers.push(Watcher {
            name: "lag-watch",
            folder: format!("{runtime}/lag-watch"),
            // A window ends every ten seconds, and one that ended long ago
            // is lag-watch gone, not a wait still going.
            silence: Duration::from_secs(30),
            beat: "ended",
        });
        watchers.push(Watcher {
            name: "disk-guard",
            folder: format!("{runtime}/disk-guard"),
            silence: Duration::from_secs(120),
            beat: "heartbeat",
        });
    }
    watchers.push(Watcher {
        name: "disk-guard-machine",
        folder: "/run/disk-guard".to_owned(),
        silence: Duration::from_secs(120),
        beat: "heartbeat",
    });
    watchers
}

/// The watchers of this session, by its `XDG_RUNTIME_DIR`.
pub fn of_session() -> Vec<Watcher> {
    watchers(std::env::var("XDG_RUNTIME_DIR").ok().as_deref())
}

/// Whether a file of this name in a watcher's folder is its status, not
/// one being written beside it.
pub fn names(name: &str) -> bool {
    name == FILE
}

/// A heartbeat heard: when it says it was written, and when it was read
/// and how old it was then, so its silence is counted on the machine's
/// running time from there.
#[derive(Debug)]
struct Heard {
    beat: f64,
    at: Instant,
    age: Duration,
    /// Whether it's been said silent, so its deadline wakes nothing again.
    silent: bool,
}

impl Heard {
    fn deadline(&self, silence: Duration) -> Instant {
        self.at + silence.saturating_sub(self.age)
    }
}

/// Each watcher's last heartbeat, kept between reads.
#[derive(Debug, Default)]
pub struct Kept {
    heard: HashMap<&'static str, Heard>,
}

impl Kept {
    /// Each watcher with a status that parses as an object, as of `now`
    /// on the clock and `epoch` since the epoch: what it says without its
    /// beats, and whether it's silent. A watcher with no file, or one that
    /// doesn't parse, says nothing, and is forgotten.
    pub fn records(
        &mut self,
        root: &Root,
        watchers: &[Watcher],
        now: Instant,
        epoch: Duration,
    ) -> Vec<Value> {
        let mut records = Vec::new();
        for watcher in watchers {
            let said = root
                .read(&format!("{}/{FILE}", watcher.folder))
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .and_then(|said| match said {
                    Value::Object(said) => Some(said),
                    _ => None,
                });
            let Some(mut said) = said else {
                self.heard.remove(watcher.name);
                continue;
            };
            let beat = said
                .get(watcher.beat)
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let heard = self.heard.entry(watcher.name).or_insert_with(|| Heard {
                beat: f64::NAN,
                at: now,
                age: Duration::ZERO,
                silent: false,
            });
            // NaN is never equal, so a first read always counts as heard.
            if heard.beat.to_bits() != beat.to_bits() {
                *heard = Heard {
                    beat,
                    at: now,
                    age: epoch
                        .saturating_sub(Duration::try_from_secs_f64(beat).unwrap_or_default()),
                    silent: false,
                };
            }
            heard.silent = now >= heard.deadline(watcher.silence);
            for beat in BEATS {
                said.remove(beat);
            }
            records.push(record(watcher.name, &said, heard.silent));
        }
        records
    }

    /// When the soonest watcher not yet silent will be, if none writes
    /// before then.
    pub fn due(&self, watchers: &[Watcher]) -> Option<Instant> {
        watchers
            .iter()
            .filter_map(|watcher| {
                let heard = self.heard.get(watcher.name)?;
                (!heard.silent).then(|| heard.deadline(watcher.silence))
            })
            .min()
    }
}

fn record(name: &str, said: &Map<String, Value>, silent: bool) -> Value {
    json!({
        "key": format!("status:{name}"),
        "kind": "status",
        "name": name,
        "said": said,
        "silent": silent,
    })
}

/// How long it's been since the epoch, for a heartbeat's age.
pub fn since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture machine of its own for each case, holding these files.
    fn root_with(name: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, Root) {
        let folder =
            std::env::temp_dir().join(format!("machine-status-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for (path, text) in files {
            let path = folder.join(path.trim_start_matches('/'));
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
            std::fs::write(path, text).expect("its file");
        }
        let root = Root::new(&folder);
        (folder, root)
    }

    #[test]
    fn only_a_status_file_is_a_watchers() {
        assert!(names("status.json"), "the status itself");
        assert!(!names(".writing"), "lag-watch's file being written");
        assert!(!names(".status"), "a guard's file being written");
    }

    #[test]
    fn a_session_without_a_runtime_folder_has_only_the_machines_guard() {
        let named = |runtime: Option<&str>| -> Vec<&str> {
            watchers(runtime)
                .iter()
                .map(|watcher| watcher.name)
                .collect()
        };
        assert_eq!(
            named(Some("/run/user/1000")),
            ["lag-watch", "disk-guard", "disk-guard-machine"]
        );
        assert_eq!(named(None), ["disk-guard-machine"]);
        assert_eq!(named(Some("")), ["disk-guard-machine"]);
    }

    #[test]
    fn what_a_watcher_says_is_told_without_its_beats_and_silent_once_its_heartbeat_is_late() {
        let (_dir, root) = root_with(
            "late",
            &[
                (
                    "/run/user/1000/lag-watch/status.json",
                    r#"{"ended": 1000, "trouble": []}"#,
                ),
                (
                    "/run/user/1000/disk-guard/status.json",
                    r#"{"heartbeat": 1000, "at": 1000, "filesystems": []}"#,
                ),
                ("/run/disk-guard/status.json", "not yet"),
            ],
        );
        let watchers = watchers(Some("/run/user/1000"));
        let mut kept = Kept::default();
        let start = Instant::now();
        // Ten seconds after both wrote.
        let records = kept.records(&root, &watchers, start, Duration::from_secs(1010));
        assert_eq!(
            records,
            [
                record_of("lag-watch", json!({"ended": 1000, "trouble": []}), false),
                record_of("disk-guard", json!({"filesystems": []}), false),
            ],
            "a file that doesn't parse says nothing"
        );
        assert_eq!(
            kept.due(&watchers),
            Some(start + Duration::from_secs(20)),
            "lag-watch's thirty seconds run out first"
        );
        // Twenty seconds on with no new heartbeat, lag-watch is silent.
        let later = start + Duration::from_secs(20);
        let records = kept.records(&root, &watchers, later, Duration::from_secs(1030));
        assert_eq!(records[0]["silent"], true, "{records:?}");
        assert_eq!(records[1]["silent"], false, "{records:?}");
        assert_eq!(
            kept.due(&watchers),
            Some(start + Duration::from_secs(110)),
            "only the guard is still to fall silent"
        );
    }

    #[test]
    fn a_heartbeat_alone_tells_nothing_new_and_gives_the_watcher_as_long_again() {
        let (dir, root) = root_with(
            "beaten",
            &[(
                "/run/disk-guard/status.json",
                r#"{"heartbeat": 1000, "at": 1000, "filesystems": []}"#,
            )],
        );
        let watchers = watchers(None);
        let mut kept = Kept::default();
        let start = Instant::now();
        let first = kept.records(&root, &watchers, start, Duration::from_secs(1000));
        std::fs::write(
            dir.join("run/disk-guard/status.json"),
            r#"{"heartbeat": 1100, "at": 1100, "filesystems": []}"#,
        )
        .expect("a new heartbeat");
        let beaten = start + Duration::from_secs(100);
        let second = kept.records(&root, &watchers, beaten, Duration::from_secs(1100));
        assert_eq!(first, second, "only the heartbeat changed");
        assert_eq!(kept.due(&watchers), Some(beaten + Duration::from_secs(120)));
    }

    #[test]
    fn a_suspend_counts_no_silence() {
        let (_dir, root) = root_with(
            "asleep",
            &[(
                "/run/disk-guard/status.json",
                r#"{"heartbeat": 1000, "filesystems": []}"#,
            )],
        );
        let watchers = watchers(None);
        let mut kept = Kept::default();
        let start = Instant::now();
        kept.records(&root, &watchers, start, Duration::from_secs(1000));
        // An hour asleep moves the epoch but not the running clock.
        let records = kept.records(&root, &watchers, start, Duration::from_secs(4600));
        assert_eq!(records[0]["silent"], false, "{records:?}");
    }

    #[test]
    fn a_watcher_whose_file_goes_says_nothing() {
        let (dir, root) = root_with(
            "gone",
            &[(
                "/run/disk-guard/status.json",
                r#"{"heartbeat": 1000, "filesystems": []}"#,
            )],
        );
        let watchers = watchers(None);
        let mut kept = Kept::default();
        let start = Instant::now();
        kept.records(&root, &watchers, start, Duration::from_secs(1000));
        std::fs::remove_file(dir.join("run/disk-guard/status.json")).expect("removed");
        let records = kept.records(&root, &watchers, start, Duration::from_secs(1000));
        assert!(records.is_empty(), "a file gone says nothing: {records:?}");
        assert_eq!(kept.due(&watchers), None, "nothing is left to fall silent");
    }

    fn record_of(name: &str, said: Value, silent: bool) -> Value {
        let Value::Object(said) = said else {
            unreachable!("a status is an object")
        };
        record(name, &said, silent)
    }
}
