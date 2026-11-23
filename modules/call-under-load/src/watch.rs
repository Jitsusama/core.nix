//! Joel's real calls on this machine, watched while they happen: how long
//! his call apps' sending threads waited for a CPU.
//!
//! The hourly test call says what a call would see of the machine, but
//! his own calls go unmeasured, and a choppy one at load 120 left no row.
//! So `call-under-load watch` asks `a-call-is-on --here` every 30 s, and
//! while a call is on reads, every 5 s, the run-queue wait of each media
//! thread of the apps he calls from (Chrome, Slack, Tuple, Zoom) from
//! `/proc`'s schedstat. When the call ends it adds the call's line to the
//! history, marked real, which the next hour's figures carry for cadence.
//! It reads thread timings only, never what a call says or shows.

use crate::figures::percentile;
use crate::gate::{self, Call};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often a call's threads are read.
const TICK: Duration = Duration::from_secs(5);
/// How often it asks whether a call is on, and looks again for the call
/// apps' threads, which come and go as a call's tabs and processes do.
const ASK: Duration = Duration::from_secs(30);
/// Past this many ms per second of waiting for a CPU, a sending thread is
/// late enough to show: 0.3 s of every second is a third of its frames.
pub const LATE_MS: f64 = 300.0;
/// Past this 15-minute load, the machine is carrying more than its room.
pub const HEAVY: f64 = 80.0;
/// The apps Joel calls from, as their scopes and services are named.
const APPS: [&str; 4] = ["chrome", "slack", "tuple", "zoom"];

/// The threads whose waits are read: Chromium's media and drawing threads,
/// which Chrome, Slack and Tuple share, and every thread of a capture or
/// audio process.
const MEDIA: [&str; 11] = [
    "ThreadPoolSingl",
    "CodecWorker",
    "WebRTC_W_and_N",
    "WebRTC_Signalin",
    "Chrome_ChildIOT",
    "Chrome_IOThread",
    "VideoCaptureThr",
    "VizCompositorTh",
    "CrRendererMain",
    "Compositor",
    "Media",
];

/// The threads whose waits hold up what Joel sends: capture, the encoder
/// and the network thread that sends it.
const SENDING: [&str; 3] = ["CodecWorker", "WebRTC_W_and_N", "VideoCaptureThr"];

/// What a Chromium process is, from its command line: its `--type` or
/// utility sub-type, or `main` for the browser itself.
pub fn role(cmdline: &str) -> String {
    ["--utility-sub-type=", "--type="]
        .iter()
        .find_map(|key| cmdline.split_once(key))
        .and_then(|(_, rest)| rest.split(['\0', ' ']).next())
        .map_or("main", |kind| kind.split('.').next().unwrap_or(kind))
        .to_string()
}

/// Whether a thread is read at all.
pub fn watched(role: &str, comm: &str) -> bool {
    MEDIA.contains(&comm) || matches!(role, "video_capture" | "audio")
}

/// Whether a thread's wait holds up what Joel sends.
pub fn sending(role: &str, comm: &str) -> bool {
    SENDING.contains(&comm) || role == "video_capture"
}

/// How long a thread has waited for a CPU in all, in ns: schedstat's
/// second field.
pub fn waited(schedstat: &str) -> Option<u64> {
    schedstat.split_whitespace().nth(1)?.parse().ok()
}

/// The call apps' scopes and services in `slice`, the apps' slice, with
/// the app each is: the test call's own browsers are left out, since it
/// measures itself.
pub fn apps(slice: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = fs::read_dir(slice)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            (name.ends_with(".scope") || name.ends_with(".service"))
                && !name.starts_with("call-under-load")
        })
        .filter_map(|name| {
            let lower = name.to_lowercase();
            let app = APPS.iter().find(|app| lower.contains(*app))?;
            Some((name, (*app).to_string()))
        })
        .collect();
    found.sort();
    found
}

/// One watched thread: who it is and where its schedstat is read.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    pub id: (u32, u32),
    pub name: String,
    pub sending: bool,
}

/// Every watched thread of the call apps, read from `proc` and the
/// apps' slice.
fn threads(proc: &Path, slice: &Path) -> Vec<Thread> {
    let mut found = Vec::new();
    for (unit, app) in apps(slice) {
        let procs = fs::read_to_string(slice.join(&unit).join("cgroup.procs")).unwrap_or_default();
        for pid in procs
            .split_whitespace()
            .filter_map(|p| p.parse::<u32>().ok())
        {
            let cmdline =
                fs::read_to_string(proc.join(format!("{pid}/cmdline"))).unwrap_or_default();
            let role = role(&cmdline);
            let tasks = fs::read_dir(proc.join(format!("{pid}/task")));
            for task in tasks
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
            {
                let comm = fs::read_to_string(proc.join(format!("{pid}/task/{task}/comm")))
                    .unwrap_or_default();
                let comm = comm.trim();
                if watched(&role, comm) {
                    found.push(Thread {
                        id: (pid, task),
                        name: format!("{app}:{role}:{comm}"),
                        sending: sending(&role, comm),
                    });
                }
            }
        }
    }
    found
}

/// One tick of a call: the worst sending thread's wait in ms per second,
/// whose it was, and the machine's 15-minute load.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub worst_ms: f64,
    pub who: Option<String>,
    pub load15: f64,
}

/// The tick's worst sending wait, from each thread's wait in ns now and
/// at the tick before, over `span_s` seconds. A thread first seen this
/// tick has nothing to compare with, so it waits for the next.
pub fn worst(
    threads: &[Thread],
    now: &HashMap<(u32, u32), u64>,
    before: &HashMap<(u32, u32), u64>,
    span_s: f64,
) -> (f64, Option<String>) {
    threads
        .iter()
        .filter(|t| t.sending)
        .filter_map(|t| {
            let ns = now.get(&t.id)?.checked_sub(*before.get(&t.id)?)?;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a tick's wait in ns is far below f64's exact range"
            )]
            let ms = ns as f64 / 1e6 / span_s;
            Some((ms, t.name.clone()))
        })
        .fold((0.0, None), |(best, who), (ms, name)| {
            if ms > best {
                (ms, Some(name))
            } else {
                (best, who)
            }
        })
}

/// A call's line in the history, marked real.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Real {
    pub began: u64,
    pub ended: u64,
    pub ticks: usize,
    /// Ticks where a sending thread waited past `LATE_MS` per second or
    /// the load was past `HEAVY`.
    pub flagged: usize,
    /// The p99 across the call's ticks of each tick's worst sending wait,
    /// in ms per second.
    pub sending_p99: Option<f64>,
    pub worst_ms: f64,
    pub worst_who: Option<String>,
    pub load15_max: f64,
}

/// What a call's ticks add up to.
pub fn summed(began: u64, ended: u64, ticks: &[Tick]) -> Real {
    let waits: Vec<f64> = ticks.iter().map(|t| t.worst_ms).collect();
    let worst = ticks
        .iter()
        .max_by(|a, b| a.worst_ms.total_cmp(&b.worst_ms));
    Real {
        began,
        ended,
        ticks: ticks.len(),
        flagged: ticks
            .iter()
            .filter(|t| t.worst_ms > LATE_MS || t.load15 > HEAVY)
            .count(),
        sending_p99: percentile(&waits, 99),
        worst_ms: worst.map_or(0.0, |t| t.worst_ms),
        worst_who: worst.and_then(|t| t.who.clone()),
        load15_max: ticks.iter().map(|t| t.load15).fold(0.0, f64::max),
    }
}

/// The history's line for a call.
pub fn line(real: &Real) -> Value {
    json!({"at": real.ended, "real": real})
}

/// The real calls the hour's figure reads: those that ended after
/// `since`, or, when none did, the newest the history holds, so a bad
/// call's figure stands until a call shows better rather than going
/// unread with its fault open. Empty when no call was ever watched.
pub fn heard(history: &str, since: u64) -> Vec<Real> {
    let mut all: Vec<Real> = history
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|row| serde_json::from_value(row.get("real")?.clone()).ok())
        .collect();
    all.sort_by_key(|real| real.ended);
    if all.iter().any(|real| real.ended > since) {
        all.retain(|real| real.ended > since);
    } else {
        let older = all.len().saturating_sub(1);
        all.drain(..older);
    }
    all
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn load15(proc: &Path) -> f64 {
    fs::read_to_string(proc.join("loadavg"))
        .ok()
        .and_then(|text| text.split_whitespace().nth(2)?.parse().ok())
        .unwrap_or(0.0)
}

fn waits(proc: &Path, threads: &[Thread]) -> HashMap<(u32, u32), u64> {
    threads
        .iter()
        .filter_map(|t| {
            let (pid, task) = t.id;
            let text =
                fs::read_to_string(proc.join(format!("{pid}/task/{task}/schedstat"))).ok()?;
            Some((t.id, waited(&text)?))
        })
        .collect()
}

/// One call, read from its first tick until `a-call-is-on` says it's
/// over: an answer it can't give leaves the call watched, since a call
/// under the heaviest load is the one most worth reading.
fn one_call(proc: &Path, slice: &Path) -> Real {
    let began = seconds_now();
    let mut found = threads(proc, slice);
    let mut before = waits(proc, &found);
    let mut last = Instant::now();
    let mut asked = Instant::now();
    let mut ticks = Vec::new();
    loop {
        thread::sleep(TICK.saturating_sub(last.elapsed()));
        let now = waits(proc, &found);
        let span = last.elapsed().as_secs_f64();
        last = Instant::now();
        let (worst_ms, who) = worst(&found, &now, &before, span);
        ticks.push(Tick {
            worst_ms,
            who,
            load15: load15(proc),
        });
        before = now;
        if asked.elapsed() >= ASK {
            asked = Instant::now();
            if gate::a_call() == Call::Off {
                return summed(began, seconds_now(), &ticks);
            }
            // Threads found anew start from now; those already read keep
            // the tick's own start, so their next span stays whole.
            found = threads(proc, slice);
            for (id, ns) in waits(proc, &found) {
                before.entry(id).or_insert(ns);
            }
        }
    }
}

/// Watches for Joel's calls until it's stopped, adding each call's line
/// to `history` as it ends; `slice` is the apps' slice in the cgroup tree.
pub fn watch(proc: &Path, slice: &Path, history: &Path) -> std::io::Result<()> {
    loop {
        if gate::a_call() == Call::On {
            let real = one_call(proc, slice);
            crate::hour::append(history, &line(&real))?;
        }
        thread::sleep(ASK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_is_named_by_its_type() {
        assert_eq!(role("chrome\0--type=renderer\0--lang=en"), "renderer");
        assert_eq!(
            role(
                "chrome --type=utility --utility-sub-type=video_capture.mojom.VideoCaptureService"
            ),
            "video_capture"
        );
        assert_eq!(
            role("/opt/google/chrome/chrome"),
            "main",
            "the browser has no type"
        );
    }

    #[test]
    fn only_media_threads_are_read_and_only_sending_ones_judged() {
        assert!(watched("renderer", "CodecWorker"));
        assert!(
            watched("video_capture", "chrome"),
            "a capture process is read whole"
        );
        assert!(!watched("renderer", "DedicatedWorker"));
        assert!(sending("renderer", "CodecWorker"));
        assert!(sending("video_capture", "Chrome_ChildIOT"));
        assert!(!sending("renderer", "Compositor"), "drawing isn't sending");
    }

    #[test]
    fn a_thread_s_wait_is_schedstat_s_second_field() {
        assert_eq!(waited("123 456789 10\n"), Some(456_789));
        assert_eq!(waited("123"), None, "a short line reads as nothing");
    }

    #[test]
    fn the_call_apps_are_found_and_the_test_call_left_out() {
        let slice = std::env::temp_dir().join(format!("watch-apps-{}", std::process::id()));
        for unit in [
            "app-com.google.Chrome-12.scope",
            "app-slack-3.scope",
            "tuple.service",
            "call-under-load.service",
            "app-org.gnome.Nautilus-4.scope",
        ] {
            fs::create_dir_all(slice.join(unit)).unwrap();
        }
        let found = apps(&slice);
        fs::remove_dir_all(&slice).unwrap();
        assert_eq!(
            found,
            [
                ("app-com.google.Chrome-12.scope", "chrome"),
                ("app-slack-3.scope", "slack"),
                ("tuple.service", "tuple"),
            ]
            .map(|(u, a)| (u.to_string(), a.to_string())),
            "the test call's own browsers and other apps were taken"
        );
    }

    #[test]
    fn a_tick_s_worst_is_the_slowest_sending_thread_per_second() {
        let thread = |tid, name: &str, sending| Thread {
            id: (1, tid),
            name: name.to_string(),
            sending,
        };
        let threads = [
            thread(1, "chrome:renderer:CodecWorker", true),
            thread(2, "chrome:renderer:Compositor", false),
            thread(3, "chrome:renderer:WebRTC_W_and_N", true),
            thread(4, "chrome:video_capture:chrome", true),
        ];
        let before = HashMap::from([((1, 1), 0), ((1, 2), 0), ((1, 3), 1_000_000_000)]);
        let now = HashMap::from([
            ((1, 1), 1_500_000_000),
            ((1, 2), 4_000_000_000),
            ((1, 3), 1_250_000_000),
            ((1, 4), 9_000_000_000),
        ]);
        let (ms, who) = worst(&threads, &now, &before, 5.0);
        assert!(
            (ms - 300.0).abs() < 1e-9,
            "1.5 s over 5 s is 300 ms/s, got {ms}"
        );
        assert_eq!(
            who.as_deref(),
            Some("chrome:renderer:CodecWorker"),
            "a drawing thread or one first seen this tick was judged"
        );
    }

    #[test]
    fn a_call_sums_to_its_ticks_p99_worst_and_flagged_ticks() {
        let tick = |worst_ms, load15| Tick {
            worst_ms,
            who: Some(format!("w{worst_ms}")),
            load15,
        };
        let ticks = [
            tick(10.0, 50.0),
            tick(350.0, 60.0),
            tick(20.0, 95.0),
            tick(5.0, 40.0),
        ];
        let real = summed(100, 120, &ticks);
        assert_eq!(real.ticks, 4);
        assert_eq!(real.flagged, 2, "a late tick and a heavy one are flagged");
        assert_eq!(real.sending_p99, Some(350.0));
        assert_eq!(real.worst_who.as_deref(), Some("w350"));
        assert!((real.load15_max - 95.0).abs() < 1e-9);
        let row = line(&real);
        assert_eq!(row["at"], 120);
        assert_eq!(
            row["real"]["began"], 100,
            "the line carries the call whole: {row}"
        );
        assert!(
            row["run"].is_null(),
            "a real call's line must not read as a test run, whose minutes the hour leaves out"
        );
        assert!(
            crate::history::ran(&row.to_string()).is_empty(),
            "a real call was taken for a test run"
        );
    }

    #[test]
    fn the_hour_reads_its_own_calls_or_else_the_newest() {
        let call = |ended| line(&summed(ended - 60, ended, &[]));
        let history = [
            call(1_000).to_string(),
            r#"{"at": 1500, "run": {"started": 1400}, "measures": {}}"#.to_string(),
            call(2_000).to_string(),
            "not json".to_string(),
            call(3_000).to_string(),
        ]
        .join("\n");
        let ended = |calls: Vec<Real>| calls.iter().map(|c| c.ended).collect::<Vec<_>>();
        assert_eq!(ended(heard(&history, 1_500)), [2_000, 3_000]);
        assert_eq!(
            ended(heard(&history, 5_000)),
            [3_000],
            "an hour with no call of its own dropped the last call's figure"
        );
        assert!(heard("", 0).is_empty(), "no call watched yet reads as none");
    }
}
