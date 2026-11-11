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
//!
//! A disk guard writes every few seconds, and each write moves its
//! measures a little: the bytes free and used, the fill's rate and when
//! it would be full, the busiest writer's rate, and the notice that says
//! that rate. Told each time, they woke the shell twelve times a minute
//! for figures Joel reads to a tenth of a GiB and a forecast in hours
//! (RO9YDD, 2026-11-11: the largest steady stream into the shut shell).
//! So a guard's record that changes only in those is told at most once a
//! minute; anything else it says, its band, its words, what it cleared,
//! or its falling silent, is told at once, and carries the new figures
//! with it. The free bytes Joel reads are in the guard's words too, so a
//! tenth of a GiB gone is told as it goes.
//!
//! A guard that falls silent may only be restarting, as every switch
//! restarts it, so its unit is asked how it stands, once, as it falls
//! silent, and its record says so while it stays silent: whether it
//! stopped, and the `Result` that says why (Y2YBTS, 2026-11-12: the
//! shell spawns no `systemctl` to learn it). It's asked again only after
//! it's been heard from.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::Root;

/// The name every watcher gives its status file.
pub const FILE: &str = "status.json";
/// The keys a disk guard rewrites each time that say only when it wrote.
const BEATS: [&str; 2] = ["heartbeat", "at"];
/// What a disk guard measures afresh in each write, told at most once
/// a minute alone: each file system's, then the writer's and the
/// notice's, each under its object.
const MEASURES: [(&str, &[&str]); 3] = [
    (
        "filesystems",
        &["free", "used", "forecast", "fill_per_hour", "full_at"],
    ),
    ("writer", &["rate"]),
    ("notice", &["body"]),
];
/// How long a guard's measures alone are held before they're told.
const MEASURED_EVERY: Duration = Duration::from_secs(60);

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
    /// Whether what it measures in each write is held, as a guard's is.
    measures: bool,
    /// The unit it runs as, asked of once it falls silent, or none for a
    /// watcher whose silence says enough alone.
    pub unit: Option<Unit>,
}

/// A watcher's systemd unit: its name, and whether it's the account's, on
/// the session's bus, or the machine's, on the system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    pub name: &'static str,
    pub account: bool,
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
            // Each window is what Lag reads, so each is told.
            measures: false,
            // Lag says only that it's gone quiet.
            unit: None,
        });
        watchers.push(Watcher {
            name: "disk-guard",
            folder: format!("{runtime}/disk-guard"),
            silence: Duration::from_secs(120),
            beat: "heartbeat",
            measures: true,
            unit: Some(Unit {
                name: "disk-guard.service",
                account: true,
            }),
        });
    }
    watchers.push(Watcher {
        name: "disk-guard-machine",
        folder: "/run/disk-guard".to_owned(),
        silence: Duration::from_secs(120),
        beat: "heartbeat",
        measures: true,
        unit: Some(Unit {
            name: "disk-guard.service",
            account: false,
        }),
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
    /// What its unit said as it fell silent, if it was asked.
    unit: Option<Value>,
}

impl Heard {
    fn deadline(&self, silence: Duration) -> Instant {
        self.at + silence.saturating_sub(self.age)
    }
}

/// A guard's record as it was last told: what it said without its
/// measures, and when, so one that changes only in them is held.
#[derive(Debug)]
struct Told {
    steady: Value,
    record: Value,
    at: Instant,
}

/// Each watcher's last heartbeat, and each guard's record last told,
/// kept between reads, with how a unit is asked how it stands.
#[derive(Debug)]
pub struct Kept {
    heard: HashMap<&'static str, Heard>,
    told: HashMap<&'static str, Told>,
    ask: fn(Unit) -> Value,
}

/// Asks no unit anything, so a record says only that it's silent.
fn unasked(_: Unit) -> Value {
    Value::Null
}

impl Default for Kept {
    fn default() -> Kept {
        Kept::asking(unasked)
    }
}

impl Kept {
    /// Kept between reads, asking a silent watcher's unit with `ask`.
    pub fn asking(ask: fn(Unit) -> Value) -> Kept {
        Kept {
            heard: HashMap::new(),
            told: HashMap::new(),
            ask,
        }
    }

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
                self.told.remove(watcher.name);
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
                unit: None,
            });
            // NaN is never equal, so a first read always counts as heard.
            if heard.beat.to_bits() != beat.to_bits() {
                *heard = Heard {
                    beat,
                    at: now,
                    age: epoch
                        .saturating_sub(Duration::try_from_secs_f64(beat).unwrap_or_default()),
                    silent: false,
                    unit: None,
                };
            }
            let silent = now >= heard.deadline(watcher.silence);
            if silent && !heard.silent {
                heard.unit = watcher.unit.map(self.ask).filter(|said| !said.is_null());
            }
            heard.silent = silent;
            for beat in BEATS {
                said.remove(beat);
            }
            let mut record = record(watcher.name, &said, heard.silent);
            if let Some(unit) = &heard.unit {
                record["unit"] = unit.clone();
            }
            if !watcher.measures {
                records.push(record);
                continue;
            }
            // The steady record holds its silence, so falling silent is
            // told at once.
            let steady = unmeasured(&record);
            let told = self
                .told
                .get(watcher.name)
                .filter(|told| told.steady == steady && now < told.at + MEASURED_EVERY);
            if let Some(told) = told {
                records.push(told.record.clone());
                continue;
            }
            self.told.insert(
                watcher.name,
                Told {
                    steady,
                    record: record.clone(),
                    at: now,
                },
            );
            records.push(record);
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

/// A guard's record without what it measures afresh in each write. A
/// guard writes again within seconds, so measures held past their minute
/// are told with its next write, and nothing wakes for them alone.
fn unmeasured(record: &Value) -> Value {
    let mut steady = record.clone();
    for (under, keys) in MEASURES {
        let Some(said) = steady.get_mut("said").and_then(|said| said.get_mut(under)) else {
            continue;
        };
        // Each file system's, or the one object's.
        let each: Vec<&mut Value> = if let Value::Array(all) = said {
            all.iter_mut().collect()
        } else {
            vec![said]
        };
        for one in each {
            if let Value::Object(fields) = one {
                for key in keys {
                    fields.remove(*key);
                }
            }
        }
    }
    steady
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
    fn a_guards_measures_alone_are_told_once_a_minute_and_anything_else_at_once() {
        let guard = |heartbeat: u32, free: u32, band: &str, rate: u32| {
            json!({
                "heartbeat": heartbeat,
                "says": "filling",
                "filesystems": [{ "path": "/", "band": band, "free": free,
                    "used": 1000 - free, "forecast": free, "fill_per_hour": 1,
                    "full_at": heartbeat + free }],
                "writer": { "name": "nix-daemon", "rate": rate },
                "notice": { "summary": "filling", "body": format!("at {rate} a second") }
            })
            .to_string()
        };
        let (dir, root) = root_with(
            "measured",
            &[(
                "/run/disk-guard/status.json",
                &guard(1000, 500, "filling", 7),
            )],
        );
        let file = dir.join("run/disk-guard/status.json");
        let watchers = watchers(None);
        let mut kept = Kept::default();
        let start = Instant::now();
        let at = |seconds: u64| start + Duration::from_secs(seconds);
        let epoch = |seconds: u64| Duration::from_secs(1000 + seconds);
        let first = kept.records(&root, &watchers, at(0), epoch(0));
        std::fs::write(&file, guard(1005, 499, "filling", 8)).expect("a write");
        let measured = kept.records(&root, &watchers, at(5), epoch(5));
        assert_eq!(
            measured, first,
            "what's measured afresh in a write isn't told within the minute"
        );
        std::fs::write(&file, guard(1061, 498, "filling", 9)).expect("a write");
        let minute = kept.records(&root, &watchers, at(61), epoch(61));
        assert_eq!(
            minute[0]["said"]["filesystems"][0]["free"], 498,
            "a minute on, the measures are told: {minute:?}"
        );
        std::fs::write(&file, guard(1066, 497, "short", 9)).expect("a write");
        let banded = kept.records(&root, &watchers, at(66), epoch(66));
        assert_eq!(
            banded[0]["said"]["filesystems"][0],
            json!({ "path": "/", "band": "short", "free": 497, "used": 503,
                "forecast": 497, "fill_per_hour": 1, "full_at": 1563 }),
            "a new band is told at once, with the measures beside it"
        );
        let silent = kept.records(&root, &watchers, at(200), epoch(200));
        assert_eq!(
            silent[0]["silent"], true,
            "falling silent is told at once: {silent:?}"
        );
    }

    #[test]
    fn lag_watchs_every_window_is_told() {
        let (dir, root) = root_with(
            "windows",
            &[(
                "/run/user/1000/lag-watch/status.json",
                r#"{"ended": 1000, "niri": {"p99_ms": 1}}"#,
            )],
        );
        let watchers = watchers(Some("/run/user/1000"));
        let mut kept = Kept::default();
        let start = Instant::now();
        kept.records(&root, &watchers, start, Duration::from_secs(1000));
        std::fs::write(
            dir.join("run/user/1000/lag-watch/status.json"),
            r#"{"ended": 1010, "niri": {"p99_ms": 2}}"#,
        )
        .expect("a window");
        let records = kept.records(
            &root,
            &watchers,
            start + Duration::from_secs(10),
            Duration::from_secs(1010),
        );
        assert_eq!(
            records[0]["said"]["niri"]["p99_ms"], 2,
            "each window is Lag's to read: {records:?}"
        );
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

    #[test]
    fn a_guard_fallen_silent_says_how_its_unit_stands_asked_once_until_heard_again() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static ASKED: AtomicUsize = AtomicUsize::new(0);
        fn ask(unit: Unit) -> Value {
            ASKED.fetch_add(1, Ordering::SeqCst);
            json!({"unit": unit.name, "account": unit.account, "stopped": true})
        }
        let (dir, root) = root_with(
            "unit",
            &[(
                "/run/disk-guard/status.json",
                r#"{"heartbeat": 1000, "filesystems": []}"#,
            )],
        );
        let watchers = watchers(None);
        let mut kept = Kept::asking(ask);
        let start = Instant::now();
        let heard = kept.records(&root, &watchers, start, Duration::from_secs(1000));
        assert!(
            heard[0].get("unit").is_none(),
            "one heard asks nothing: {heard:?}"
        );
        let at = |seconds: u64| start + Duration::from_secs(seconds);
        let silent = kept.records(&root, &watchers, at(120), Duration::from_secs(1120));
        assert_eq!(
            silent[0]["unit"],
            json!({"unit": "disk-guard.service", "account": false, "stopped": true}),
            "the machine's guard is asked on the system's bus: {silent:?}"
        );
        let still = kept.records(&root, &watchers, at(210), Duration::from_secs(1210));
        assert_eq!(still[0]["unit"], silent[0]["unit"], "{still:?}");
        assert_eq!(
            ASKED.load(Ordering::SeqCst),
            1,
            "asked once while it stays silent"
        );
        std::fs::write(
            dir.join("run/disk-guard/status.json"),
            r#"{"heartbeat": 1300, "filesystems": []}"#,
        )
        .expect("a heartbeat");
        let back = kept.records(&root, &watchers, at(300), Duration::from_secs(1300));
        assert!(
            back[0].get("unit").is_none(),
            "heard again, it's running: {back:?}"
        );
        kept.records(&root, &watchers, at(420), Duration::from_secs(1420));
        assert_eq!(
            ASKED.load(Ordering::SeqCst),
            2,
            "silent again, it's asked again"
        );
    }

    fn record_of(name: &str, said: Value, silent: bool) -> Value {
        let Value::Object(said) = said else {
            unreachable!("a status is an object")
        };
        record(name, &said, silent)
    }
}
