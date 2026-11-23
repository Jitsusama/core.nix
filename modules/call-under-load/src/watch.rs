//! Joel's real calls on this machine, watched while they happen: how long
//! his call apps' sending threads waited for a CPU.
//!
//! The hourly test call says what a call would see of the machine, but
//! his own calls go unmeasured, and a choppy one at load 120 left no row.
//! So `call-under-load watch` asks `a-call-is-on --here` every 30 s, and
//! while a call is on reads, every 5 s, the run-queue wait of each thread
//! that sends his picture, in the apps he calls from (Chrome, Slack,
//! Tuple, Zoom), from `/proc`'s schedstat. When the call ends it adds the
//! call's line to the history, marked real, which the next hour's figures
//! carry for cadence. It reads thread timings only, never what a call
//! says or shows.

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
/// How many times in a row a-call-is-on may fail to tell during a call
/// before the call is closed as unsure: three minutes, so a call that
/// ended while it couldn't tell doesn't run on into the next.
const UNSURE: u32 = 6;
/// How many times it asks before a call that can't be told isn't watched,
/// as the hour's run asks.
const ASKS: u32 = 3;
/// How long a call's figure stands when no later call replaced it.
const DAY: u64 = 24 * 3600;
/// Past this many ms per second of waiting for a CPU, a sending thread is
/// late enough to show: 0.3 s of every second is a third of its frames.
pub const LATE_MS: f64 = 300.0;
/// Past this 15-minute load, the machine is carrying more than its room.
pub const HEAVY: f64 = 80.0;
/// The apps Joel calls from, as their scopes and services are named.
const APPS: [&str; 4] = ["chrome", "slack", "tuple", "zoom"];
/// Units in his app.slice that aren't his, as lag-watch names them
/// (modules/lag-watch/src/apps.rs): the test call's, a terminal's, which
/// a pi may run in, and mycelium's and its hosts'.
const NOT_HIS: [&str; 4] = ["call-under-load", "kitty-", "mycelium-pi-", "pi-host-"];
/// Units in his app.slice whose children aren't his, as lag-watch judges
/// a Chrome's parent: those `NOT_HIS` names but his terminal, which may
/// carry his call, and those `systemd-run` makes as they're run.
const NOT_HIS_PARENT: [&str; 4] = ["call-under-load", "mycelium-pi-", "pi-host-", "run-"];

/// The threads whose waits hold up what Joel sends, as Chromium names
/// them, cut to 15 bytes: the software encoder and the thread that hands
/// its packets to the network. The 08:35 test call of 23 Nov found both
/// waiting in its renderer. Every thread of a capture process counts too,
/// since it makes the frames the encoder takes.
const SENDING: [&str; 3] = ["CodecWorker", "WebRTC_W_and_N", "VideoCaptureThr"];

/// Only letters, digits and `_.-`, at most 32, from text another process
/// wrote, before it reaches the history and cadence's card.
fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .take(32)
        .collect()
}

/// What a Chromium process is, from its command line: its utility
/// sub-type or `--type`, or `main` for the browser itself.
/// Only a whole argument counts, so a URL the browser was opened with
/// can't name it a capture process.
pub fn role(cmdline: &str) -> String {
    let kind = ["--utility-sub-type=", "--type="]
        .iter()
        .find_map(|key| {
            cmdline
                .split(['\0', ' '])
                .find_map(|arg| arg.strip_prefix(key))
        })
        .map_or("main", |kind| kind.split('.').next().unwrap_or(kind));
    clean(kind)
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

/// The pid a scope is named for, `app-<name>-<pid>.scope`, as Chrome names
/// the scope it moves itself into.
fn named_pid(unit: &str) -> Option<u32> {
    unit.strip_suffix(".scope")?
        .rsplit_once('-')?
        .1
        .parse()
        .ok()
}

/// Whether a process whose parent is in `cgroup`, its `/proc` line, was
/// started by Joel: by his user manager, his session or one of his apps.
/// An agent's Chrome moves itself into a scope in his app.slice as his
/// does, but its parent stays in the agents' or jobs' slice.
pub fn his_parent(cgroup: &str) -> bool {
    let Some(path) = cgroup.lines().find_map(|line| line.strip_prefix("0::")) else {
        return false;
    };
    let in_apps = path
        .split_once("/app.slice/")
        .is_some_and(|(_, unit)| !NOT_HIS_PARENT.iter().any(|not| unit.starts_with(not)));
    in_apps
        || path.ends_with("/init.scope")
        || path.contains("/session-")
        || path.contains("/session.slice/")
}

/// Whether a unit in his app.slice is his, read from `proc`: one not named
/// for a process is his by its name, as is one whose process or parent is
/// gone before it's read, as lag-watch leaves them.
fn his(proc: &Path, unit: &str) -> bool {
    let Some(pid) = named_pid(unit) else {
        return true;
    };
    let stat = fs::read_to_string(proc.join(format!("{pid}/stat"))).unwrap_or_default();
    let parent = stat
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().nth(1)?.parse::<u32>().ok());
    parent
        .and_then(|ppid| fs::read_to_string(proc.join(format!("{ppid}/cgroup"))).ok())
        .is_none_or(|cgroup| his_parent(&cgroup))
}

/// The call apps' units in `slice`, the apps' slice, with the app each
/// is, and those that aren't his left out: the test call's own browsers,
/// which it measures itself, and agents' and terminals' work.
pub fn units(slice: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = fs::read_dir(slice)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|ext| ext == "scope" || ext == "service")
                && !NOT_HIS.iter().any(|not| name.starts_with(not))
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

/// One sending thread: who it is and where its schedstat is read.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    pub id: (u32, u32),
    pub name: String,
}

/// A thread's wait in ns, and when it was read.
type Read = (u64, Instant);

/// The tick's worst sending wait, in ms per second, and whose it was,
/// from each thread's read now and its read before, each over its own
/// span. None when no thread had both: a call whose sending threads
/// weren't found reads as unread, never as a call that waited for nothing.
pub fn worst(
    threads: &[Thread],
    now: &HashMap<(u32, u32), Read>,
    before: &HashMap<(u32, u32), Read>,
) -> Option<(f64, String)> {
    threads
        .iter()
        .filter_map(|t| {
            let (ns, at) = now.get(&t.id)?;
            let (was, then) = before.get(&t.id)?;
            let span = at.checked_duration_since(*then)?.as_secs_f64();
            if span <= 0.0 {
                return None;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a tick's wait in ns is far below f64's exact range"
            )]
            let ms = ns.checked_sub(*was)? as f64 / 1e6 / span;
            Some((ms, t.name.clone()))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// What the watch keeps across a call's ticks: each process's role, read
/// from its command line once, when it's first seen, since that read takes
/// the process's memory lock and could stall behind a page faulting under
/// the very pressure it records; the sending threads; and their last reads.
#[derive(Default)]
struct Watching {
    roles: HashMap<u32, String>,
    threads: Vec<Thread>,
    before: HashMap<(u32, u32), Read>,
}

impl Watching {
    /// Looks again for his call apps' sending threads. Threads found anew
    /// are read now, so their first span starts here.
    fn look(&mut self, proc: &Path, slice: &Path) {
        let mut threads = Vec::new();
        let mut roles = HashMap::new();
        for (unit, app) in units(slice).into_iter().filter(|(unit, _)| his(proc, unit)) {
            let procs = fs::read_to_string(slice.join(&unit).join("cgroup.procs"));
            for pid in procs
                .unwrap_or_default()
                .split_whitespace()
                .filter_map(|p| p.parse::<u32>().ok())
            {
                let role = self.roles.remove(&pid).unwrap_or_else(|| {
                    role(
                        &fs::read_to_string(proc.join(format!("{pid}/cmdline")))
                            .unwrap_or_default(),
                    )
                });
                let tasks = fs::read_dir(proc.join(format!("{pid}/task")));
                for task in tasks
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
                {
                    let comm = fs::read_to_string(proc.join(format!("{pid}/task/{task}/comm")))
                        .unwrap_or_default();
                    let comm = clean(comm.trim());
                    if sending(&role, &comm) {
                        threads.push(Thread {
                            id: (pid, task),
                            name: format!("{app}:{role}:{comm}"),
                        });
                    }
                }
                roles.insert(pid, role);
            }
        }
        // Gone processes' roles go, so a pid used again is read afresh.
        self.roles = roles;
        self.threads = threads;
        let fresh: Vec<Thread> = self
            .threads
            .iter()
            .filter(|t| !self.before.contains_key(&t.id))
            .cloned()
            .collect();
        self.before.extend(reads(proc, &fresh));
        self.before
            .retain(|id, _| self.threads.iter().any(|t| t.id == *id));
    }

    /// One tick's worst sending wait, the threads' reads kept for the next.
    fn tick(&mut self, proc: &Path) -> Option<(f64, String)> {
        let now = reads(proc, &self.threads);
        let found = worst(&self.threads, &now, &self.before);
        self.before.extend(now);
        found
    }
}

fn reads(proc: &Path, threads: &[Thread]) -> HashMap<(u32, u32), Read> {
    threads
        .iter()
        .filter_map(|t| {
            let (pid, task) = t.id;
            let text =
                fs::read_to_string(proc.join(format!("{pid}/task/{task}/schedstat"))).ok()?;
            Some((t.id, (waited(&text)?, Instant::now())))
        })
        .collect()
}

/// One tick of a call: the worst sending thread's wait in ms per second,
/// unread when none was found, whose it was, and the machine's 15-minute
/// load.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub worst_ms: Option<f64>,
    pub who: Option<String>,
    pub load15: f64,
}

/// A call's line in the history, marked real.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Real {
    pub began: u64,
    pub ended: u64,
    pub ticks: usize,
    /// Ticks with a sending thread read.
    pub read: usize,
    /// Ticks where a sending thread waited past `LATE_MS` per second or
    /// the load was past `HEAVY`.
    pub flagged: usize,
    /// The p99 across the call's read ticks of each tick's worst sending
    /// wait, in ms per second; unread when no tick was read.
    pub sending_p99: Option<f64>,
    pub worst_ms: Option<f64>,
    pub worst_who: Option<String>,
    pub load15_max: f64,
    /// Closed because a-call-is-on couldn't tell for three minutes.
    #[serde(default)]
    pub unsure: bool,
    /// Cut short by the watch stopping, and written when it started again.
    #[serde(default)]
    pub cut: bool,
}

/// What a call's ticks add up to.
pub fn summed(began: u64, ended: u64, ticks: &[Tick]) -> Real {
    let waits: Vec<f64> = ticks.iter().filter_map(|t| t.worst_ms).collect();
    let worst = ticks
        .iter()
        .filter(|t| t.worst_ms.is_some())
        .max_by(|a, b| {
            a.worst_ms
                .unwrap_or(0.0)
                .total_cmp(&b.worst_ms.unwrap_or(0.0))
        });
    Real {
        began,
        ended,
        ticks: ticks.len(),
        read: waits.len(),
        flagged: ticks
            .iter()
            .filter(|t| t.worst_ms.is_some_and(|ms| ms > LATE_MS) || t.load15 > HEAVY)
            .count(),
        sending_p99: percentile(&waits, 99),
        worst_ms: worst.and_then(|t| t.worst_ms),
        worst_who: worst.and_then(|t| t.who.clone()),
        load15_max: ticks.iter().map(|t| t.load15).fold(0.0, f64::max),
        unsure: false,
        cut: false,
    }
}

/// The history's line for a call.
pub fn line(real: &Real) -> Value {
    json!({"at": real.ended, "real": real})
}

/// The real calls the hour's figure reads, as of `now`: those that ended
/// after `since`, the last hour written, so calls in hours a meeting or a
/// call skipped still count; or, when none did, the newest of the last
/// day, whose figure stands until a later call replaces it. A call said to
/// end in the future is a clock's fault, and left out.
pub fn heard(history: &str, since: u64, now: u64) -> Vec<Real> {
    let mut all: Vec<Real> = history
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|row| serde_json::from_value(row.get("real")?.clone()).ok())
        .filter(|real: &Real| real.ended <= now + 60)
        .collect();
    all.sort_by_key(|real| real.ended);
    if all.iter().any(|real| real.ended > since) {
        all.retain(|real| real.ended > since);
    } else {
        all.retain(|real| real.ended + DAY > now);
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

/// Writes the call so far beside the history, whole or not at all, so a
/// stop mid-call leaves it to be written when the watch starts again.
fn kept(part: &Path, real: &Real) -> std::io::Result<()> {
    let tmp = part.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec(real)?)?;
    fs::rename(&tmp, part)
}

/// One call, read from its first tick until `a-call-is-on` says it's
/// over, or can't tell for three minutes running. A single answer it
/// can't give leaves the call watched, since a call under the heaviest
/// load is the one most worth reading.
fn one_call(proc: &Path, slice: &Path, part: &Path) -> Real {
    let began = seconds_now();
    let mut watching = Watching::default();
    watching.look(proc, slice);
    let mut asked = Instant::now();
    let mut unsure = 0;
    let mut ticks = Vec::new();
    loop {
        thread::sleep(TICK);
        let (worst_ms, who) = watching.tick(proc).unzip();
        ticks.push(Tick {
            worst_ms,
            who,
            load15: load15(proc),
        });
        if asked.elapsed() < ASK {
            continue;
        }
        asked = Instant::now();
        match gate::a_call() {
            Call::Off => return summed(began, seconds_now(), &ticks),
            Call::On => unsure = 0,
            Call::Unknown(why) => {
                unsure += 1;
                if unsure >= UNSURE {
                    eprintln!("closing the call as unsure: {why}");
                    let mut real = summed(began, seconds_now(), &ticks);
                    real.unsure = true;
                    return real;
                }
            }
        }
        watching.look(proc, slice);
        if let Err(error) = kept(part, &summed(began, seconds_now(), &ticks)) {
            eprintln!("the call so far couldn't be kept: {error}");
        }
    }
}

/// Watches for Joel's calls until it's stopped, adding each call's line
/// to `history` as it ends; `slice` is the apps' slice in the cgroup tree.
/// A call a stop cut short is written first, marked cut.
pub fn watch(proc: &Path, slice: &Path, history: &Path) -> std::io::Result<()> {
    let part = history.with_file_name("watching.json");
    if let Some(mut real) = fs::read(&part)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Real>(&bytes).ok())
    {
        real.cut = true;
        crate::hour::append(history, &line(&real))?;
    }
    let _ = fs::remove_file(&part);
    let mut told = None;
    loop {
        let mut asked = gate::a_call();
        for _ in 1..ASKS {
            if !matches!(asked, Call::Unknown(_)) {
                break;
            }
            thread::sleep(TICK);
            asked = gate::a_call();
        }
        // Said once each time it changes, so the journal shows a watch
        // that can't tell apart from one that hears no calls.
        let now = match &asked {
            Call::Unknown(why) => format!("can't tell whether a call is on: {why}"),
            Call::On => "a call is on; watching its sending threads".to_string(),
            Call::Off => "no call is on".to_string(),
        };
        if told.as_ref() != Some(&now) {
            eprintln!("{now}");
            told = Some(now);
        }
        if asked == Call::On {
            let real = one_call(proc, slice, &part);
            crate::hour::append(history, &line(&real))?;
            let _ = fs::remove_file(&part);
            eprintln!(
                "the call ended: {} of {} ticks read, {} flagged, worst {:?} ms/s by {}",
                real.read,
                real.ticks,
                real.flagged,
                real.worst_ms,
                real.worst_who.as_deref().unwrap_or("none")
            );
            told = None;
        }
        thread::sleep(ASK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_is_named_by_its_type_and_only_safe_text_kept() {
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
        assert_eq!(
            role("x\0--type=a\u{1b}[31m;b"),
            "a31mb",
            "a terminal escape got through"
        );
        assert_eq!(
            role("chrome\0https://x.test/?--type=video_capture"),
            "main",
            "a URL named the browser"
        );
    }

    #[test]
    fn only_sending_threads_are_judged() {
        assert!(sending("renderer", "CodecWorker"), "the encoder sends");
        assert!(
            sending("renderer", "WebRTC_W_and_N"),
            "WebRTC's worker sends"
        );
        assert!(
            sending("video_capture", "Chrome_ChildIOT"),
            "a capture process counts whole"
        );
        assert!(!sending("renderer", "Compositor"), "drawing isn't sending");
    }

    #[test]
    fn a_thread_s_wait_is_schedstat_s_second_field() {
        assert_eq!(waited("123 456789 10\n"), Some(456_789));
        assert_eq!(waited("123"), None, "a short line reads as nothing");
    }

    #[test]
    fn a_scope_is_his_when_his_manager_session_or_apps_started_it() {
        let user = "0::/user.slice/user-1000.slice";
        assert!(
            his_parent(&format!("{user}/user@1000.service/init.scope\n")),
            "his manager started it"
        );
        assert!(
            his_parent(&format!("{user}/session-2.scope\n")),
            "his session started it"
        );
        for his in [
            "app.slice/app-niri-x-9.scope",
            "app.slice/kitty-70-0.scope",
            "session.slice/xdg-desktop-portal.service",
        ] {
            assert!(
                his_parent(&format!("{user}/user@1000.service/{his}\n")),
                "{his} wasn't Joel's"
            );
        }
        for not in [
            "user@1000.service/agents.slice/pi-host-work.service",
            "user@1000.service/jobs.slice/jobs-work.slice/x.scope",
            "user@1000.service/app.slice/run-u12.service",
            "user@1000.service/app.slice/call-under-load.service",
        ] {
            assert!(
                !his_parent(&format!("{user}/{not}\n")),
                "{not} was taken as Joel's"
            );
        }
        assert_eq!(
            named_pid("app-com.google.Chrome-2349662.scope"),
            Some(2_349_662)
        );
        assert_eq!(named_pid("tuple.service"), None);
    }

    #[test]
    fn an_agent_s_chrome_in_his_slice_is_left_out() {
        let root = std::env::temp_dir().join(format!("watch-his-{}", std::process::id()));
        let proc = root.join("proc");
        let slice = root.join("app.slice");
        for unit in [
            "app-com.google.Chrome-12.scope",
            "app-com.google.Chrome-34.scope",
            "app-slack-56.scope",
            "tuple.service",
            "call-under-load.service",
            "kitty-70-0.scope",
            "app-org.gnome.Nautilus-4.scope",
        ] {
            fs::create_dir_all(slice.join(unit)).unwrap();
        }
        let started = |pid: u32, parent: u32, cgroup: &str| {
            fs::create_dir_all(proc.join(pid.to_string())).unwrap();
            fs::create_dir_all(proc.join(parent.to_string())).unwrap();
            fs::write(
                proc.join(format!("{pid}/stat")),
                format!("{pid} (chrome (x)) S {parent} 1 1"),
            )
            .unwrap();
            fs::write(
                proc.join(format!("{parent}/cgroup")),
                format!("0::{cgroup}\n"),
            )
            .unwrap();
        };
        let manager = "/user.slice/user-1000.slice/user@1000.service";
        started(12, 2, &format!("{manager}/init.scope"));
        started(
            34,
            3,
            &format!("{manager}/agents.slice/pi-host-work.service"),
        );
        let found: Vec<String> = units(&slice)
            .into_iter()
            .filter(|(unit, _)| his(&proc, unit))
            .map(|(unit, app)| format!("{unit} {app}"))
            .collect();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            found,
            [
                "app-com.google.Chrome-12.scope chrome",
                "app-slack-56.scope slack",
                "tuple.service tuple",
            ],
            "the wrong units were taken for Joel's call apps"
        );
    }

    #[test]
    fn a_tick_s_worst_is_the_slowest_sending_thread_over_its_own_span() {
        let thread = |task, name: &str| Thread {
            id: (1, task),
            name: name.to_string(),
        };
        let threads = [
            thread(1, "chrome:renderer:CodecWorker"),
            thread(3, "chrome:renderer:WebRTC_W_and_N"),
            thread(4, "chrome:video_capture:chrome"),
        ];
        let then = Instant::now();
        let at = |s: u64| then + Duration::from_secs(s);
        let before = HashMap::from([((1, 1), (0, at(0))), ((1, 3), (1_000_000_000, at(4)))]);
        let now = HashMap::from([
            ((1, 1), (1_500_000_000, at(5))),
            ((1, 3), (1_250_000_000, at(5))),
            ((1, 4), (9_000_000_000, at(5))),
        ]);
        let (ms, who) = worst(&threads, &now, &before).unwrap();
        assert!(
            (ms - 300.0).abs() < 1e-9,
            "1.5 s over 5 s is 300 ms/s, got {ms}"
        );
        assert_eq!(
            who, "chrome:renderer:CodecWorker",
            "a thread first seen this tick was judged"
        );
        let late = HashMap::from([((1, 3), (1_000_000_000, at(1)))]);
        let (ms, _) = worst(&threads[1..2], &now, &late).unwrap();
        assert!(
            (ms - 62.5).abs() < 1e-9,
            "a thread read late got a span it wasn't read over"
        );
        assert_eq!(
            worst(&threads, &now, &HashMap::new()),
            None,
            "no sending thread read twice read as a wait"
        );
    }

    #[test]
    fn a_call_sums_to_its_ticks_p99_worst_and_flagged_ticks() {
        let tick = |worst_ms: Option<f64>, load15| Tick {
            worst_ms,
            who: worst_ms.map(|ms| format!("w{ms}")),
            load15,
        };
        let ticks = [
            tick(Some(10.0), 50.0),
            tick(Some(350.0), 60.0),
            tick(Some(20.0), 95.0),
            tick(None, 40.0),
        ];
        let real = summed(100, 120, &ticks);
        assert_eq!((real.ticks, real.read), (4, 3));
        assert_eq!(real.flagged, 2, "a late tick and a heavy one are flagged");
        assert_eq!(real.sending_p99, Some(350.0));
        assert_eq!(real.worst_who.as_deref(), Some("w350"));
        assert!(
            (real.load15_max - 95.0).abs() < 1e-9,
            "the call's load is its heaviest tick's"
        );
        let unread = summed(100, 120, &[tick(None, 40.0)]);
        assert_eq!(
            (unread.sending_p99, unread.worst_ms),
            (None, None),
            "a call with no sending thread found read as one that waited for nothing"
        );
        let row = line(&real);
        assert_eq!(row["at"], 120);
        assert_eq!(
            row["real"]["began"], 100,
            "the line carries the call whole: {row}"
        );
        assert!(
            crate::history::ran(&row.to_string()).is_empty(),
            "a real call was taken for a test run, whose minutes the hour leaves out"
        );
    }

    #[test]
    fn the_hour_reads_calls_since_the_last_hour_or_else_the_day_s_newest() {
        let call = |ended| line(&summed(ended - 60, ended, &[])).to_string();
        let history = [
            call(1_000),
            r#"{"at": 1500, "run": {"started": 1400}, "measures": {}}"#.to_string(),
            call(2_000),
            "not json".to_string(),
            call(3_000),
            call(90_000),
        ]
        .join("\n");
        let ended = |calls: Vec<Real>| calls.iter().map(|c| c.ended).collect::<Vec<_>>();
        assert_eq!(
            ended(heard(&history, 1_500, 5_000)),
            [2_000, 3_000],
            "calls since the last hour written, a skipped hour's among them, weren't all read"
        );
        assert_eq!(
            ended(heard(&history, 4_000, 5_000)),
            [3_000],
            "an hour with no call of its own dropped the last call's figure"
        );
        assert!(
            heard(&history, 4_000, 3_000 + DAY + 1).is_empty(),
            "a call more than a day old still stood"
        );
        assert_eq!(
            ended(heard(&history, 0, 89_000)),
            [1_000, 2_000, 3_000],
            "a call said to end in the future was read"
        );
        assert!(
            heard("", 0, 0).is_empty(),
            "no call watched yet reads as none"
        );
    }

    #[test]
    fn a_call_a_stop_cut_short_is_kept_whole() {
        let root = std::env::temp_dir().join(format!("watch-kept-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let part = root.join("watching.json");
        let real = summed(10, 20, &[]);
        kept(&part, &real).unwrap();
        let back: Real = serde_json::from_slice(&fs::read(&part).unwrap()).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            back, real,
            "the call so far didn't read back as it was kept"
        );
    }
}
