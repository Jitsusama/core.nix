//! Joel's real calls on this machine, watched while they happen: how long
//! his call apps' sending threads waited for a CPU.
//!
//! The hourly test call says what a call would see of the machine, but
//! his own calls go unmeasured, and a choppy one at load 120 left no row.
//! So `call-under-load watch` asks `a-call-is-on --here` every 30 s, and
//! while a call is on reads, every 5 s, the run-queue wait of each thread
//! that sends his picture, in the apps he calls from (Chrome, Slack,
//! Tuple, Zoom), from `/proc`'s schedstat and sched. When the call ends it adds the
//! call's line to the history, marked real, which the next hour's figures
//! carry for cadence. It reads thread timings only, never what a call
//! says or shows. Each time it asks, it says when it last could tell,
//! so a watch that stopped or can't tell files a fault rather than
//! reading as a day with no calls.
//!
//! Long because about half of it is the tests beside it, which build
//! stand-in `/proc` and cgroup folders to judge whose apps are Joel's.

use crate::figures::percentile;
use crate::gate::{self, Call};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
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

/// Where the watch reads Joel's apps: `/proc`, his user manager's path in
/// the cgroup tree, its app.slice's folder, and the folder `systemd-run`
/// makes its units in, whose units in app.slice are an agent's or a
/// test's, not his.
pub struct Seat {
    pub proc: PathBuf,
    pub account: String,
    pub slice: PathBuf,
    pub transient: PathBuf,
}

/// The threads whose waits hold up what Joel sends, cut to 15 bytes as the
/// kernel keeps them. Chromium's, Meet's and Slack's: the software encoder
/// and the thread that hands its packets to the network, which the 08:35
/// test call of 23 Nov found waiting in its renderer. Tuple's, which runs
/// WebRTC's own queues rather than Chromium's: its video and audio
/// encoders' queues, which lag-watch named as the call's worst-waiting
/// threads 21 times in Joel's Tuple call from 10:00 on 24 Nov, when the
/// watch, knowing only Chromium's names, judged none of them; and WebRTC's
/// pacer, which hands Tuple's packets to the network as `WebRTC_W_and_N`
/// does Chromium's. Every thread of a capture process
/// but its watchdog counts too, since it makes the frames the encoder
/// takes.
const SENDING: [&str; 6] = [
    "CodecWorker",
    "WebRTC_W_and_N",
    "VideoCaptureThr",
    "VideoEncoderQue",
    "AudioEncoderQue",
    "TaskQueuePacedS",
];

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
/// can't name it a capture process. A process that retitled itself, as
/// Chromium's zygote children do, has joined its arguments with spaces
/// into one, and only then are spaces read as their breaks.
pub fn role(cmdline: &str) -> String {
    let whole = cmdline.trim_end_matches('\0');
    let args: Vec<&str> = if whole.contains('\0') {
        whole.split('\0').collect()
    } else {
        whole.split(' ').collect()
    };
    let kind = ["--utility-sub-type=", "--type="]
        .iter()
        .find_map(|key| args.iter().find_map(|arg| arg.strip_prefix(key)))
        .map_or("main", |kind| kind.split('.').next().unwrap_or(kind));
    clean(kind)
}

/// A capture process's thread that makes no frames: Chromium's watchdog,
/// which only looks for hung threads. Joel's call at 16:35 on 24 Nov read
/// as 2000 ms a second at its p99, all of it this thread's, while
/// lag-watch saw his call's threads waiting far less.
const WATCHING: &str = "HangWatcher";

/// Whether a thread's wait holds up what Joel sends.
pub fn sending(role: &str, comm: &str) -> bool {
    SENDING.contains(&comm) || (role == "video_capture" && comm != WATCHING)
}

/// How long a thread has waited for a CPU in all, in ns: schedstat's
/// second field.
pub fn waited(schedstat: &str) -> Option<u64> {
    schedstat.split_whitespace().nth(1)?.parse().ok()
}

/// How long a thread has waited for a CPU in all, in ns, by the fair
/// scheduler's own count in its `sched`: `wait_sum`, never the
/// `iowait_sum` beside it. The kernel prints both in ms, and prints
/// neither while `kernel.sched_schedstats` is off, when this is None.
pub fn fair_waited(sched: &str) -> Option<u64> {
    let ms: f64 = sched.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = key.trim();
        (key == "wait_sum" || key.ends_with(".wait_sum")).then(|| value.trim().parse().ok())?
    })?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ms a thread has waited, held at zero or more, are far below u64's ns"
    )]
    Some((ms.max(0.0) * 1e6).round() as u64)
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

/// Whether a process whose parent is in `parents`, its `/proc` cgroup
/// file, was started by Joel, as lag-watch judges a Chrome's parent: his
/// user manager, at `account` in the cgroup tree, a service of his
/// session, or a unit in his app.slice but those `NOT_HIS` names and those
/// `systemd-run` made, whose files are in `transient`. A launcher's
/// `app-*.scope` is made as it's run too, and is his. An agent's Chrome
/// moves itself into a scope in his app.slice as his does, but its parent
/// stays in the agents' or jobs' slice; one his terminal started is his
/// terminal's, which an agent's there reads the same as, and isn't watched.
pub fn his_parent(parents: &str, account: &str, transient: &Path) -> bool {
    parents
        .lines()
        .filter_map(|line| line.strip_prefix("0::"))
        .filter_map(|path| path.strip_prefix(account))
        .any(|rest| {
            rest == "/init.scope"
                || rest.starts_with("/session.slice/")
                || rest.strip_prefix("/app.slice/").is_some_and(|path| {
                    // A run's own cgroups under it, a delegated one's,
                    // are the run's.
                    let unit = path.split('/').next().unwrap_or(path);
                    let launched = unit.starts_with("app-")
                        && Path::new(unit)
                            .extension()
                            .is_some_and(|ext| ext == "scope");
                    !NOT_HIS.iter().any(|not| unit.starts_with(not))
                        && (launched || !transient.join(unit).exists())
                })
        })
}

/// Whether a unit in his app.slice is his: one not named for a process is
/// his by its name. None when its process or parent is gone before it's
/// read, or the pid it's named for is no longer in it, used again by
/// another, which is left his for now and judged again, as lag-watch
/// leaves them.
fn his(seat: &Seat, unit: &str) -> Option<bool> {
    let Some(pid) = named_pid(unit) else {
        return Some(true);
    };
    let own = fs::read_to_string(seat.proc.join(format!("{pid}/cgroup"))).ok()?;
    let in_it = own
        .lines()
        .filter_map(|line| line.strip_prefix("0::"))
        .any(|path| path.split('/').any(|part| part == unit));
    if !in_it {
        return None;
    }
    let fields = fs::read_to_string(seat.proc.join(format!("{pid}/stat"))).ok()?;
    let parent = fields
        .rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().nth(1)?.parse::<u32>().ok())?;
    let parents = fs::read_to_string(seat.proc.join(format!("{parent}/cgroup"))).ok()?;
    Some(his_parent(&parents, &seat.account, &seat.transient))
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

/// A thread's wait for a CPU in ns by both of the kernel's counts, the
/// schedstat's and the fair scheduler's where it keeps one, and when they
/// were read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Read {
    pub counted: u64,
    pub fair: Option<u64>,
    pub at: Instant,
}

impl Read {
    /// How long the thread waited for a CPU since `old`: the lesser of
    /// what each count grew by, where both were read, as lag-watch takes
    /// it. Each counts every real wait, and each counts some sleeps the
    /// other doesn't: the schedstat a delayed sleeper's move between CPUs,
    /// up to seconds; the fair count a sleeper's time still queued before
    /// it leaves, 310 ms against 125 ms over 30 s for Chrome's main thread
    /// on 26 November. None when the schedstat's count went back.
    pub fn waited_since(&self, old: &Read) -> Option<u64> {
        let counted = self.counted.checked_sub(old.counted)?;
        Some(match (self.fair, old.fair) {
            (Some(now), Some(then)) => now.checked_sub(then).map_or(counted, |f| counted.min(f)),
            _ => counted,
        })
    }
}

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
            let read = now.get(&t.id)?;
            let was = before.get(&t.id)?;
            let span = read.at.checked_duration_since(was.at)?.as_secs_f64();
            if span <= 0.0 {
                return None;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "a tick's wait in ns is far below f64's exact range"
            )]
            let ms = read.waited_since(was)? as f64 / 1e6 / span;
            Some((ms, t.name.clone()))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// What the watch keeps across a call's ticks: each process's role, read
/// from its command line once, when it's first seen, since that read takes
/// the process's memory lock and could stall behind a page faulting under
/// the very pressure it records; the sending threads; and their last reads.
/// The call apps found over the call, which tell a call whose apps' threads
/// couldn't be read from a microphone in use with no call app open. And
/// whose each unit is, judged once and kept while the watch runs, as
/// lag-watch keeps it: an agent's Chrome whose launcher has ended is taken
/// on by his user manager, and judged again would read as his.
#[derive(Default)]
struct Watching {
    roles: HashMap<u32, String>,
    threads: Vec<Thread>,
    before: HashMap<(u32, u32), Read>,
    apps: BTreeSet<String>,
    verdicts: HashMap<String, bool>,
}

impl Watching {
    /// Starts a call afresh, keeping only who each unit is.
    fn begin(&mut self) {
        *self = Self {
            verdicts: std::mem::take(&mut self.verdicts),
            ..Self::default()
        };
    }

    /// Whether `unit` is his, as first judged.
    fn judged(&mut self, seat: &Seat, unit: &str) -> bool {
        if let Some(&verdict) = self.verdicts.get(unit) {
            return verdict;
        }
        let Some(verdict) = his(seat, unit) else {
            return true;
        };
        self.verdicts.insert(unit.to_string(), verdict);
        verdict
    }

    /// His call apps' units now, each judged once while it lasts. The watch
    /// judges between calls too, so an agent's Chrome is judged while its
    /// launcher still runs, before a call it carries begins.
    fn judge(&mut self, seat: &Seat) -> Vec<(String, String)> {
        let found = units(&seat.slice);
        // A unit gone takes its verdict, so a pid used again is judged anew.
        self.verdicts
            .retain(|unit, _| found.iter().any(|(name, _)| name == unit));
        found
            .into_iter()
            .filter(|(unit, _)| self.judged(seat, unit))
            .collect()
    }

    /// The call so far, from `began`, with the call apps found over it.
    fn so_far(&self, began: u64, ticks: &[Tick]) -> Real {
        let mut real = summed(began, seconds_now(), ticks);
        real.apps = self.apps.iter().cloned().collect();
        real
    }

    /// Looks again for his call apps' sending threads. Threads found anew
    /// are read now, so their first span starts here.
    fn look(&mut self, seat: &Seat) {
        let (proc, slice) = (seat.proc.as_path(), seat.slice.as_path());
        let mut threads = Vec::new();
        let mut roles = HashMap::new();
        for (unit, app) in self.judge(seat) {
            self.apps.insert(app.clone());
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
            let dir = proc.join(format!("{pid}/task/{task}"));
            let counted = waited(&fs::read_to_string(dir.join("schedstat")).ok()?)?;
            let fair = fs::read_to_string(dir.join("sched"))
                .ok()
                .and_then(|text| fair_waited(&text));
            Some((
                t.id,
                Read {
                    counted,
                    fair,
                    at: Instant::now(),
                },
            ))
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
    /// His call apps found during the call; none when the microphone was
    /// in use with no call app open.
    #[serde(default)]
    pub apps: Vec<String>,
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
        apps: Vec::new(),
    }
}

/// Whether `history` already holds the real call that began at `began`,
/// so a call kept beside it isn't written twice when a stop came between
/// its line and the kept file's removal.
pub fn written(history: &str, began: u64) -> bool {
    history
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|row| serde_json::from_value::<Real>(row.get("real")?.clone()).ok())
        .any(|real| real.began == began)
}

/// When the watch last could tell whether a call was on, as it said in
/// `alive` beside the history; None before it first ran.
pub fn clear_at(alive: &Path) -> Option<u64> {
    let text = fs::read_to_string(alive).ok()?;
    serde_json::from_str::<Value>(&text)
        .ok()?
        .get("clear")?
        .as_u64()
}

/// Says, whole or not at all, when the watch last could tell.
fn alive(path: &Path, clear: u64) {
    let tmp = path.with_extension("tmp");
    let said = json!({"at": seconds_now(), "clear": clear}).to_string();
    if let Err(error) = fs::write(&tmp, said).and_then(|()| fs::rename(&tmp, path)) {
        eprintln!("when it last could tell couldn't be said: {error}");
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
    // The newest read call of the last day stands until a later read one
    // replaces it, so a call since that the watch couldn't read hides no
    // figure.
    let read = |real: &Real| real.sending_p99.is_some();
    let fresh = all.iter().any(|real| real.ended > since && read(real));
    let standing = all
        .iter()
        .rposition(|real| !fresh && read(real) && real.ended + DAY > now);
    let mut index = 0;
    all.retain(|real| {
        let keep = real.ended > since || Some(index) == standing;
        index += 1;
        keep
    });
    all
}

/// When the watch last could tell, after `answer`, asked `now`: an answer
/// it can't give leaves it where it was.
fn cleared(answer: &Call, clear: u64, now: u64) -> u64 {
    if matches!(answer, Call::Unknown(_)) {
        clear
    } else {
        now
    }
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
fn one_call(
    seat: &Seat,
    watching: &mut Watching,
    part: &Path,
    said: &Path,
    clear: &mut u64,
) -> Real {
    let began = seconds_now();
    watching.begin();
    watching.look(seat);
    let mut asked = Instant::now();
    let mut unsure = 0;
    let mut ticks = Vec::new();
    loop {
        thread::sleep(TICK);
        let (worst_ms, who) = watching.tick(&seat.proc).unzip();
        ticks.push(Tick {
            worst_ms,
            who,
            load15: load15(&seat.proc),
        });
        if asked.elapsed() < ASK {
            continue;
        }
        asked = Instant::now();
        let answer = gate::a_call();
        *clear = cleared(&answer, *clear, seconds_now());
        alive(said, *clear);
        match answer {
            Call::Off => return watching.so_far(began, &ticks),
            Call::On => unsure = 0,
            Call::Unknown(why) => {
                unsure += 1;
                if unsure >= UNSURE {
                    eprintln!("closing the call as unsure: {why}");
                    let mut real = watching.so_far(began, &ticks);
                    real.unsure = true;
                    return real;
                }
            }
        }
        watching.look(seat);
        if let Err(error) = kept(part, &watching.so_far(began, &ticks)) {
            eprintln!("the call so far couldn't be kept: {error}");
        }
    }
}

/// Writes the call a stop cut short, kept in `part`, to `history`, marked
/// cut, unless its line was written before the stop; then drops `part`.
fn recover(part: &Path, history: &Path) -> std::io::Result<()> {
    if let Some(mut real) = fs::read(part)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Real>(&bytes).ok())
    {
        let lines = fs::read_to_string(history).unwrap_or_default();
        if !written(&lines, real.began) {
            real.cut = true;
            crate::hour::append(history, &line(&real))?;
        }
    }
    // Left behind, it's found written at the next start, and dropped then.
    let _ = fs::remove_file(part);
    Ok(())
}

/// The watch's hold on `lock`, so only one watch writes the history: one
/// run by hand beside the service's would write each call twice, and take
/// the other's call in progress for one a stop cut short. None when
/// another watch holds it.
pub fn alone(lock: &Path) -> std::io::Result<Option<File>> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(error)) => Err(error),
    }
}

/// Watches for Joel's calls until it's stopped, adding each call's line
/// to `history` as it ends, and saying in `said` when it last could tell;
/// the caller holds `alone`'s lock.
pub fn watch(seat: &Seat, history: &Path, said: &Path) -> std::io::Result<()> {
    let part = history.with_file_name("watching.json");
    recover(&part, history)?;
    let mut told = None;
    // A watch starting counts as its last clear moment, so one that can't
    // tell from its start counts its minutes from there.
    let mut clear = clear_at(said).unwrap_or_else(seconds_now);
    let mut watching = Watching::default();
    loop {
        let mut asked = gate::a_call();
        for _ in 1..ASKS {
            if !matches!(asked, Call::Unknown(_)) {
                break;
            }
            thread::sleep(TICK);
            asked = gate::a_call();
        }
        clear = cleared(&asked, clear, seconds_now());
        alive(said, clear);
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
            let real = one_call(seat, &mut watching, &part, said, &mut clear);
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
        } else {
            watching.judge(seat);
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
        assert_eq!(
            role("chrome\0--user-data-dir=/a --type=video_capture\0--type=renderer\0"),
            "renderer",
            "a space inside an argument broke it into a type"
        );
        assert_eq!(
            role("chrome --type=renderer --lang=en\0"),
            "renderer",
            "a retitled line, ending in its one NUL, wasn't read across its spaces"
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
        assert!(
            !sending("video_capture", "HangWatcher"),
            "a capture process's watchdog, which makes no frames, was judged as sending"
        );
        assert!(!sending("renderer", "Compositor"), "drawing isn't sending");
        // Tuple's encoders, as the kernel cuts their queues' names, and
        // the threads it shows that receive.
        assert!(
            sending("main", "VideoEncoderQue"),
            "Tuple's video encoder sends"
        );
        assert!(
            sending("main", "AudioEncoderQue"),
            "Tuple's audio encoder sends"
        );
        assert!(
            sending("main", "TaskQueuePacedS"),
            "Tuple's pacer, which hands its packets to the network, sends"
        );
        for receiving in ["IncomingVideoSt", "VideoDecoderQue"] {
            assert!(
                !sending("main", receiving),
                "Tuple's {receiving} was judged as sending"
            );
        }
    }

    #[test]
    fn a_thread_s_wait_is_schedstat_s_second_field() {
        assert_eq!(waited("123 456789 10\n"), Some(456_789));
        assert_eq!(waited("123"), None, "a short line reads as nothing");
    }

    #[test]
    fn a_thread_s_fair_wait_is_its_sched_s_wait_sum_in_ns() {
        let sched = "se.sum_exec_runtime   :   1.463270\n\
                     iowait_sum            : 900.000000\n\
                     wait_sum              :  12.250000\n";
        assert_eq!(
            fair_waited(sched),
            Some(12_250_000),
            "the wait_sum line, in ms, wasn't read as ns"
        );
        assert_eq!(
            fair_waited("se.statistics.wait_sum : 2.000000\n"),
            Some(2_000_000),
            "an older kernel's keyed wait_sum wasn't read"
        );
        assert_eq!(
            fair_waited("iowait_sum : 900.000000\n"),
            None,
            "the io wait was read as a wait for a CPU"
        );
    }

    #[test]
    fn a_wait_is_the_lesser_of_what_both_counts_grew_by() {
        let then = Instant::now();
        let read = |counted, fair| Read {
            counted,
            fair,
            at: then,
        };
        assert_eq!(
            read(4_000_000_000, Some(2_000_000)).waited_since(&read(0, Some(0))),
            Some(2_000_000),
            "a sleeper's move the schedstat counted read as a wait"
        );
        assert_eq!(
            read(125_000_000, Some(310_000_000)).waited_since(&read(0, Some(0))),
            Some(125_000_000),
            "a sleeper's queued time the fair count kept read as a wait"
        );
        assert_eq!(
            read(5_000_000, None).waited_since(&read(0, Some(0))),
            Some(5_000_000),
            "with no fair count the schedstat's wasn't taken alone"
        );
        assert_eq!(
            read(5_000_000, Some(1)).waited_since(&read(0, Some(9))),
            Some(5_000_000),
            "a fair count that went back cut the wait"
        );
        assert_eq!(
            read(0, Some(9)).waited_since(&read(5, Some(0))),
            None,
            "a schedstat that went back read as a wait"
        );
    }

    #[test]
    fn a_tick_reads_both_counts_from_each_thread_s_task_folder() {
        let proc = std::env::temp_dir().join(format!("watch-reads-{}", std::process::id()));
        let task = |pid: u32, tid: u32| {
            let dir = proc.join(format!("{pid}/task/{tid}"));
            fs::create_dir_all(&dir).unwrap();
            dir
        };
        let both = task(7, 8);
        fs::write(both.join("schedstat"), "1 4000000000 3\n").unwrap();
        fs::write(both.join("sched"), "wait_sum : 2.000000\n").unwrap();
        let alone = task(7, 9);
        fs::write(alone.join("schedstat"), "1 5000000 3\n").unwrap();
        task(7, 10);
        let thread = |tid| Thread {
            id: (7, tid),
            name: format!("t{tid}"),
        };
        let read = reads(&proc, &[thread(8), thread(9), thread(10)]);
        fs::remove_dir_all(&proc).unwrap();
        let counts = |tid| read.get(&(7, tid)).map(|r| (r.counted, r.fair));
        assert_eq!(
            counts(8),
            Some((4_000_000_000, Some(2_000_000))),
            "the sched file beside the schedstat wasn't read"
        );
        assert_eq!(
            counts(9),
            Some((5_000_000, None)),
            "a thread with no sched file wasn't read by its schedstat alone"
        );
        assert_eq!(counts(10), None, "a thread with no schedstat was read");
    }

    #[test]
    fn a_scope_is_his_when_his_manager_session_or_apps_started_it() {
        let root = std::env::temp_dir().join(format!("watch-parent-{}", std::process::id()));
        let transient = root.join("transient");
        fs::create_dir_all(&transient).unwrap();
        fs::write(transient.join("agent-run.service"), "").unwrap();
        fs::write(transient.join("app-gio-x-7.scope"), "").unwrap();
        let account = "/user.slice/user-1000.slice/user@1000.service";
        let parent = |path: &str| his_parent(&format!("0::{path}\n"), account, &transient);
        let (mut his_ok, mut not_ok) = (Vec::new(), Vec::new());
        for his in [
            "/init.scope",
            "/app.slice/app-niri-x-9.scope",
            "/app.slice/app-org.gnome.Nautilus-4.scope",
            "/app.slice/tuple.service",
            "/app.slice/tuple.service/delegated/x.scope",
            "/app.slice/app-gio-x-7.scope",
            "/session.slice/xdg-desktop-portal.service",
        ] {
            his_ok.push((his, parent(&format!("{account}{his}"))));
        }
        for not in [
            "/agents.slice/pi-host-work.service",
            "/jobs.slice/jobs-work.slice/x.scope",
            "/app.slice/agent-run.service",
            "/app.slice/agent-run.service/delegated/x.scope",
            "/app.slice/call-under-load.service",
            "/app.slice/kitty-70-0.scope",
            "/background.slice/x.service/init.scope",
        ] {
            not_ok.push((not, !parent(&format!("{account}{not}"))));
        }
        let other = !parent("/user.slice/user-1001.slice/user@1001.service/init.scope");
        let session = !parent("/user.slice/user-1000.slice/session-2.scope");
        fs::remove_dir_all(&root).unwrap();
        for (his, ok) in his_ok {
            assert!(ok, "{his} wasn't Joel's");
        }
        for (not, ok) in not_ok {
            assert!(ok, "{not} was taken as Joel's");
        }
        assert!(other, "another account's manager was taken as Joel's");
        assert!(
            session,
            "a login session's scope, as lag-watch leaves out, was taken as his"
        );
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
            "app-com.google.Chrome-90.scope",
            "app-slack-56.scope",
            "tuple.service",
            "call-under-load.service",
            "kitty-70-0.scope",
            "app-org.gnome.Nautilus-4.scope",
        ] {
            fs::create_dir_all(slice.join(unit)).unwrap();
        }
        let manager = "/user.slice/user-1000.slice/user@1000.service";
        let agents = format!("{manager}/agents.slice/pi-host-work.service");
        let started = |pid: u32, parent: u32, cgroup: &str| {
            fs::create_dir_all(proc.join(pid.to_string())).unwrap();
            fs::create_dir_all(proc.join(parent.to_string())).unwrap();
            fs::write(
                proc.join(format!("{pid}/stat")),
                format!("{pid} (chrome (x)) S {parent} 1 1"),
            )
            .unwrap();
            fs::write(
                proc.join(format!("{pid}/cgroup")),
                format!("0::{manager}/app.slice/app-com.google.Chrome-{pid}.scope\n"),
            )
            .unwrap();
            fs::write(
                proc.join(format!("{parent}/cgroup")),
                format!("0::{cgroup}\n"),
            )
            .unwrap();
        };
        started(12, 2, &format!("{manager}/init.scope"));
        started(34, 3, &agents);
        // Its scope's pid ended and an agent's process took the number up:
        // its parent says nothing of who opened the scope.
        started(90, 4, &agents);
        fs::write(proc.join("90/cgroup"), format!("0::{agents}\n")).unwrap();
        let seat = Seat {
            proc: proc.clone(),
            account: manager.to_string(),
            slice: slice.clone(),
            transient: root.join("transient"),
        };
        let named = |units: Vec<(String, String)>| -> Vec<String> {
            units
                .into_iter()
                .map(|(unit, app)| format!("{unit} {app}"))
                .collect()
        };
        let mut watching = Watching::default();
        // Judged between calls, while the agent's launcher still runs.
        let found = named(watching.judge(&seat));
        // The agent's launcher ends, his user manager takes its Chrome on,
        // and a call begins.
        started(34, 1, &format!("{manager}/init.scope"));
        watching.begin();
        watching.look(&seat);
        let apps = watching.so_far(1, &[]).apps;
        let again = named(watching.judge(&seat));
        // Its scope goes, and one of its name comes back, his.
        let scope = slice.join("app-com.google.Chrome-34.scope");
        fs::remove_dir_all(&scope).unwrap();
        watching.judge(&seat);
        fs::create_dir_all(&scope).unwrap();
        let anew = named(watching.judge(&seat));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            again, found,
            "an agent's Chrome was taken as his once its launcher had ended"
        );
        assert_eq!(
            found,
            [
                "app-com.google.Chrome-12.scope chrome",
                "app-com.google.Chrome-90.scope chrome",
                "app-slack-56.scope slack",
                "tuple.service tuple",
            ],
            "the wrong units were taken for Joel's call apps"
        );
        assert!(
            anew.contains(&"app-com.google.Chrome-34.scope chrome".to_string()),
            "a gone unit's verdict was kept for a later one of its name"
        );
        assert_eq!(
            apps,
            ["chrome", "slack", "tuple"],
            "the call apps found didn't reach the call's line"
        );
    }

    #[test]
    fn an_answer_it_can_t_give_leaves_when_it_last_could_tell() {
        assert_eq!(
            [Call::On, Call::Off, Call::Unknown("timed out".to_string())]
                .map(|answer| cleared(&answer, 100, 200)),
            [200, 200, 100],
            "an answer it couldn't give moved when it last could tell"
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
        let read = |counted, s| Read {
            counted,
            fair: None,
            at: at(s),
        };
        let before = HashMap::from([((1, 1), read(0, 0)), ((1, 3), read(1_000_000_000, 4))]);
        let now = HashMap::from([
            ((1, 1), read(1_500_000_000, 5)),
            ((1, 3), read(1_250_000_000, 5)),
            ((1, 4), read(9_000_000_000, 5)),
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
        let late = HashMap::from([((1, 3), read(1_000_000_000, 1))]);
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
        let tick = || Tick {
            worst_ms: Some(10.0),
            who: Some("chrome:renderer:WebRTC_W_and_N".to_string()),
            load15: 1.0,
        };
        let call = |ended| line(&summed(ended - 60, ended, &[tick()])).to_string();
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
        let unread = line(&summed(4_400, 4_500, &[])).to_string();
        assert_eq!(
            ended(heard(&format!("{history}\n{unread}"), 4_000, 5_000)),
            [3_000, 4_500],
            "a call since the last hour the watch couldn't read hid the standing figure"
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

    #[test]
    fn a_call_a_stop_cut_short_is_written_once_marked_cut() {
        let root = std::env::temp_dir().join(format!("watch-recover-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let (part, history) = (root.join("watching.json"), root.join("history.jsonl"));
        let test_run = r#"{"at": 30, "run": {"started": 10}, "measures": {}}"#;
        fs::write(&history, format!("{test_run}\n")).unwrap();
        kept(&part, &summed(10, 20, &[])).unwrap();
        recover(&part, &history).unwrap();
        let once = fs::read_to_string(&history).unwrap();
        let dropped = !part.exists();
        // Stopped again after its line, before the kept file went.
        kept(&part, &summed(10, 20, &[])).unwrap();
        recover(&part, &history).unwrap();
        let twice = fs::read_to_string(&history).unwrap();
        fs::remove_dir_all(&root).unwrap();
        let calls = heard(&once, 0, 100);
        assert_eq!(
            calls.iter().map(|c| (c.began, c.cut)).collect::<Vec<_>>(),
            [(10, true)],
            "a test run that began when the call did hid it, or it wasn't marked cut"
        );
        assert!(dropped, "the kept call stayed to be written again");
        assert_eq!(twice, once, "a call already written was written again");
    }

    #[test]
    fn only_one_watch_writes_the_history() {
        let root = std::env::temp_dir().join(format!("watch-alone-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let lock = root.join("watch.lock");
        let first = alone(&lock).unwrap();
        let second = alone(&lock).unwrap();
        drop(first);
        let after = alone(&lock).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(
            second.is_none(),
            "a second watch was let write beside the first"
        );
        assert!(after.is_some(), "the lock outlived the watch that held it");
    }

    #[test]
    fn the_watch_says_when_it_last_could_tell() {
        let root = std::env::temp_dir().join(format!("watch-alive-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let said = root.join("watch.json");
        let before = clear_at(&said);
        alive(&said, 1_234);
        let after = clear_at(&said);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(before, None, "a watch that never ran said a time");
        assert_eq!(after, Some(1_234), "the time it last could tell was lost");
    }
}
