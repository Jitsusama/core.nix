//! Samples, a few times a day, what a key typed into the pi window Joel is
//! at waits on.
//!
//! Typing in pi felt sluggish while the machine sat mostly idle, and the
//! cause was found only by measuring once, by hand, for hours. pi-keys
//! keeps the measure going cheaply: run by a timer, it waits for a pi
//! window to hold the focus, then for ten minutes at pi windows reads the
//! three threads a key passes every 2 ms, without privilege: niri's main
//! thread, the kitty drawing the window and pi's main thread. From their
//! bursts it models what a key waited (src/model.rs), and appends one line
//! to $XDG_STATE_HOME/pi-keys/<UTC day>.jsonl, for cadence's daily read,
//! with what it ran beside: the CPU's contention, and which windows, pi
//! and pi packages it read (src/context.rs), and the keys again by the
//! install each window ran and by window (src/split.rs).
//! It reads only /proc, /sys and niri's answers, never touches a window,
//! and stops the moment a call starts.
//!
//! Usage: pi-keys sample [--minutes N] [--wait N] [--call-check PROGRAM]

mod burst;
mod context;
mod model;
mod path;
mod split;

use burst::{Bursts, Reading};
use path::Window;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::FileExt;
use std::os::unix::net::UnixStream;
use std::path::{Path as FsPath, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often each thread is read. A key's wait is told from bursts, and a
/// frame at 60 Hz is 16.7 ms, so 2 ms sees each frame's work in eight.
const TICK: Duration = Duration::from_millis(2);
/// A tick this late, in milliseconds, is the sampler's own stall: what it
/// would show is the sampler's, so it's left out.
const GAP: f64 = 500.0;
/// How often the focus is asked, in milliseconds.
const FOCUS: f64 = 1000.0;
/// How often a call is looked for, in milliseconds; the check takes two
/// seconds, run beside the sampling.
const CALLS: f64 = 10_000.0;
/// How many keys a sample models.
const KEYS: usize = 10_000;
/// The frame when niri doesn't say, in milliseconds.
const SIXTY: f64 = 1000.0 / 60.0;

struct Settings {
    minutes: f64,
    wait: f64,
    call_check: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let number =
        |name: &str, default: f64| value(name).and_then(|v| v.parse().ok()).unwrap_or(default);
    if args.first().map(String::as_str) == Some("sample") {
        sample(&Settings {
            minutes: number("--minutes", 10.0),
            wait: number("--wait", 60.0),
            call_check: value("--call-check").map(PathBuf::from),
        })
    } else {
        eprintln!("usage: pi-keys sample [--minutes N] [--wait N] [--call-check PROGRAM]");
        ExitCode::from(2)
    }
}

/// A thread read every tick, by files held open and read again from
/// their start, which costs a tenth of opening them each time.
struct Watched {
    tid: u32,
    schedstat: File,
    stat: File,
    bursts: Bursts,
}

impl Watched {
    fn open(tid: u32) -> Option<Watched> {
        Some(Watched {
            tid,
            schedstat: File::open(format!("/proc/{tid}/task/{tid}/schedstat")).ok()?,
            stat: File::open(format!("/proc/{tid}/task/{tid}/stat")).ok()?,
            bursts: Bursts::default(),
        })
    }

    fn read(&self) -> Option<Reading> {
        let mut buf = [0u8; 1024];
        let n = self.schedstat.read_at(&mut buf, 0).ok()?;
        let (run, wait) = burst::schedstat(std::str::from_utf8(&buf[..n]).ok()?)?;
        let n = self.stat.read_at(&mut buf, 0).ok()?;
        let (state, majflt) = burst::stat(std::str::from_utf8(&buf[..n]).ok()?)?;
        Some(Reading {
            run,
            wait,
            state,
            majflt,
        })
    }

    /// Reads the thread at `now`, keeping in `spans` a burst it ends that a
    /// key would wait out. False when the thread has gone.
    fn take(&mut self, now: f64, interval: f64, spans: &mut Vec<(f64, f64, model::Why)>) -> bool {
        let Some(reading) = self.read() else {
            return false;
        };
        if let Some(b) = self.bursts.take(now, interval, reading)
            && b.solid()
            && b.worth()
        {
            spans.push((b.at, b.at + b.ms, model::Why::of(b.cpu, b.wait, b.d)));
        }
        true
    }
}

/// The threads of the window Joel is at, when it's a pi window whose
/// session is found.
struct At {
    window: u64,
    kitty: Watched,
    pi: Watched,
}

/// A call check run beside the sampling every [`CALLS`], never waited on.
struct Calls {
    program: Option<PathBuf>,
    running: Option<Child>,
    started: f64,
}

impl Calls {
    /// Whether the check that last finished found a call on, starting the
    /// next when it's due.
    fn on(&mut self, now: f64) -> bool {
        let Some(program) = self.program.as_deref() else {
            return false;
        };
        match self.running.as_mut().map(Child::try_wait) {
            Some(Ok(Some(status))) => {
                self.running = None;
                status.code() == Some(0)
            }
            Some(Err(_)) => {
                self.running = None;
                false
            }
            None if now - self.started >= CALLS => {
                self.started = now;
                self.running = quietly(program).spawn().ok();
                false
            }
            Some(Ok(None)) | None => false,
        }
    }
}

impl Drop for Calls {
    fn drop(&mut self) {
        if let Some(mut child) = self.running.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn quietly(program: &FsPath) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// What a sample has seen so far.
struct Sampler {
    socket: PathBuf,
    niri: Watched,
    at: Option<At>,
    frame: f64,
    spans: [Vec<(f64, f64, model::Why)>; 3],
    at_pi: Vec<split::Held>,
    /// When the stretch at a pi window began, its window and its pi.
    since: Option<(f64, u64, u32)>,
    windows: HashSet<u64>,
    /// The pi processes of the windows read.
    pis: HashSet<u32>,
    gaps: u32,
    focus_asked: f64,
}

impl Sampler {
    /// Leaves out what a late tick would show, as the sampler's own stall.
    fn gap(&mut self, now: f64, interval: f64) {
        self.gaps += 1;
        self.niri.bursts.reset();
        if let Some(at) = self.at.as_mut() {
            at.kitty.bursts.reset();
            at.pi.bursts.reset();
        }
        if let Some((from, window, pi)) = self.since {
            self.at_pi.push(split::Held {
                from,
                to: now - interval,
                window,
                pi,
            });
            self.since = Some((now, window, pi));
        }
    }

    /// Ends the span at a pi window, if one is open.
    fn away(&mut self, now: f64) {
        if let Some((from, window, pi)) = self.since.take() {
            self.at_pi.push(split::Held {
                from,
                to: now,
                window,
                pi,
            });
        }
    }

    /// Follows the focus to another window, watching its threads when
    /// it's a pi window.
    fn refocus(&mut self, now: f64) {
        self.focus_asked = now;
        let window = ask(&self.socket, "FocusedWindow")
            .as_deref()
            .and_then(path::focused);
        if self.at.as_ref().map(|a| a.window) == window.as_ref().map(|w| w.id) {
            return;
        }
        self.away(now);
        self.at = window.as_ref().and_then(threads_of);
        if let Some(found) = self.at.as_ref() {
            self.windows.insert(found.window);
            self.pis.insert(found.pi.tid);
            self.since = Some((now, found.window, found.pi.tid));
            if let Some(frame) = ask(&self.socket, "FocusedOutput")
                .as_deref()
                .and_then(path::frame)
            {
                self.frame = frame;
            }
        }
    }

    fn tick(&mut self, now: f64, interval: f64) {
        if interval > GAP {
            self.gap(now, interval);
        }
        if now - self.focus_asked >= FOCUS {
            self.refocus(now);
        }
        let [niri, kitty, pi] = &mut self.spans;
        self.niri.take(now, interval, niri);
        if let Some(found) = self.at.as_mut()
            && !(found.kitty.take(now, interval, kitty) && found.pi.take(now, interval, pi))
        {
            eprintln!(
                "pi-keys: thread {} or {} ended",
                found.kitty.tid, found.pi.tid
            );
            self.at = None;
            self.away(now);
        }
    }

    /// Milliseconds spent at pi windows so far.
    fn held(&self, now: f64) -> f64 {
        self.at_pi.iter().map(|h| h.to - h.from).sum::<f64>()
            + self.since.map_or(0.0, |(s, _, _)| now - s)
    }

    /// The sample's line, or None when no time at a pi window was read.
    fn line(self, ended: u64, call: bool, cpu_waited: f64) -> Option<serde_json::Value> {
        let mut window_ids: Vec<u64> = self.windows.iter().copied().collect();
        window_ids.sort_unstable();
        let mut pis = HashSet::new();
        let mut packages = HashSet::new();
        let root = env_dir("XDG_DATA_HOME", ".local/share").join("pi-packages");
        for &pid in &self.pis {
            pis.extend(
                fs::read(format!("/proc/{pid}/environ"))
                    .ok()
                    .as_deref()
                    .and_then(context::pi_version),
            );
            packages.extend(context::loaded_by(&root, pid));
        }
        let sorted = |set: HashSet<String>| {
            let mut list: Vec<String> = set.into_iter().collect();
            list.sort();
            list
        };
        let load1 = fs::read_to_string("/proc/loadavg")
            .ok()
            .as_deref()
            .and_then(context::load1);
        let [niri, kitty, pi] = self.spans;
        let path = model::Path {
            niri: model::Spans::told(niri),
            kitty: model::Spans::told(kitty),
            pi: model::Spans::told(pi),
        };
        let stretches: Vec<(f64, f64)> = self.at_pi.iter().map(|h| (h.from, h.to)).collect();
        let keys = model::keys(
            &path,
            &stretches,
            self.frame,
            KEYS,
            &mut model::Draw::new(ended),
        );
        let sum = model::summary(&keys)?;
        let seconds = stretches.iter().map(|(a, b)| b - a).sum::<f64>() / 1000.0;
        // The same keys again, by the install each window's pi loaded, so
        // installs compare under the one load the sample saw.
        let installs = |pid| context::loaded_by(&root, pid);
        let figures = |seconds: f64, summary: &model::Summary| {
            serde_json::json!({
                "seconds_at_pi": round(seconds, 1),
                "p50_ms": round(summary.p50, 1),
                "p99_ms": round(summary.p99, 1),
                "over_33ms": round(summary.late, 4),
                "pi_over_16ms": round(summary.lost_at[2], 4),
                "pi_over_16ms_by": held_by(summary),
            })
        };
        // The same keys again, by the install each window's pi loaded, so
        // installs compare under the one load the sample saw, and by
        // window, so one long session's slow keys show as its own.
        let by_install: Vec<serde_json::Value> =
            split::installs(&path, &self.at_pi, installs, self.frame, KEYS, ended)
                .into_iter()
                .map(|each| {
                    let mut line = figures(each.seconds, &each.summary);
                    line["packages"] = serde_json::json!(each.of);
                    line
                })
                .collect();
        let by_window: Vec<serde_json::Value> =
            split::windows(&path, &self.at_pi, installs, self.frame, KEYS, ended)
                .into_iter()
                .map(|each| {
                    let (window, packages) = each.of;
                    let mut line = figures(each.seconds, &each.summary);
                    line["window"] = serde_json::json!(window);
                    line["packages"] = serde_json::json!(packages);
                    line
                })
                .collect();
        Some(serde_json::json!({
            "ended": ended,
            "keys": sum.keys,
            "modelled": true,
            "seconds_at_pi": round(seconds, 1),
            "p50_ms": round(sum.p50, 1),
            "p99_ms": round(sum.p99, 1),
            "max_ms": round(sum.max, 1),
            "over_33ms": round(sum.late, 4),
            "waits_over_16ms": {
                "niri": round(sum.lost_at[0], 4),
                "kitty": round(sum.lost_at[1], 4),
                "pi": round(sum.lost_at[2], 4),
            },
            // Whether pi ran its own work, waited for a CPU or sat in D
            // through the keys it held, so the fix goes to pi's code or
            // to its place in the queue.
            "pi_over_16ms_by": held_by(&sum),
            "frame_ms": round(self.frame, 2),
            "gaps": self.gaps,
            "windows": self.windows.len(),
            "window_ids": window_ids,
            "pi": sorted(pis),
            "packages": sorted(packages),
            "by_install": by_install,
            "by_window": by_window,
            "cpu_waited": round(cpu_waited, 4),
            "load1": load1,
            "call": call,
        }))
    }
}

fn sample(settings: &Settings) -> ExitCode {
    let Some(socket) = niri_socket() else {
        eprintln!("pi-keys: no niri to ask, so no sample");
        return ExitCode::SUCCESS;
    };
    let Some(niri) = socket_pid(&socket).and_then(Watched::open) else {
        eprintln!("pi-keys: niri's thread can't be read, so no sample");
        return ExitCode::SUCCESS;
    };
    if !wait_for_pi(&socket, settings) {
        eprintln!(
            "pi-keys: no pi window held the focus for {} minutes, so no sample",
            settings.wait
        );
        return ExitCode::SUCCESS;
    }
    let frame = ask(&socket, "FocusedOutput")
        .as_deref()
        .and_then(path::frame)
        .unwrap_or(SIXTY);
    let mut sampler = Sampler {
        socket,
        niri,
        at: None,
        frame,
        spans: [Vec::new(), Vec::new(), Vec::new()],
        at_pi: Vec::new(),
        since: None,
        windows: HashSet::new(),
        pis: HashSet::new(),
        gaps: 0,
        focus_asked: f64::NEG_INFINITY,
    };
    let mut calls = Calls {
        program: settings.call_check.clone(),
        running: None,
        started: 0.0,
    };
    let enough = settings.minutes * 60_000.0;
    let waited_from = cpu_waited_us();
    let begun = Instant::now();
    let ms = || begun.elapsed().as_secs_f64() * 1000.0;
    let (mut last, mut next, mut call) = (ms(), Instant::now(), false);
    loop {
        next += TICK;
        match next.checked_duration_since(Instant::now()) {
            Some(wait) => thread::sleep(wait),
            None => next = Instant::now(),
        }
        let now = ms();
        sampler.tick(now, now - last);
        last = now;
        if calls.on(now) {
            call = true;
            break;
        }
        if sampler.held(now) >= enough || now >= 3.0 * enough {
            break;
        }
    }
    sampler.away(last);
    // The share of the sample some task waited for a CPU: near 0 on an
    // idle machine, near 1 when every moment had something queued.
    let cpu_waited = match (waited_from, cpu_waited_us()) {
        (Some(from), Some(to)) if last > 0.0 => {
            u32::try_from(to.saturating_sub(from)).map_or(1.0, f64::from) / (last * 1000.0)
        }
        _ => 0.0,
    };
    let ended = seconds_now();
    let Some(line) = sampler.line(ended, call, cpu_waited.min(1.0)) else {
        eprintln!("pi-keys: no time at a pi window was sampled, so no sample");
        return ExitCode::SUCCESS;
    };
    eprintln!("pi-keys: {line}");
    match keep(ended, &line.to_string()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pi-keys: couldn't keep the sample: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Waits up to the settings' minutes for a pi window, whose session is
/// found, to hold the focus while no call is on.
fn wait_for_pi(socket: &FsPath, settings: &Settings) -> bool {
    let until = Instant::now() + Duration::from_secs_f64(settings.wait * 60.0);
    loop {
        let pi = ask(socket, "FocusedWindow")
            .as_deref()
            .and_then(path::focused)
            .is_some_and(|window| threads_of(&window).is_some());
        if pi && !in_a_call(settings) {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        thread::sleep(Duration::from_secs(2));
    }
}

/// Whether the call check says a call is on: it exits 0 then.
fn in_a_call(settings: &Settings) -> bool {
    settings.call_check.as_deref().is_some_and(|program| {
        quietly(program)
            .status()
            .is_ok_and(|status| status.code() == Some(0))
    })
}

/// The kitty and pi a pi window's keys pass, found as the jobs-sorter
/// finds its session's scope.
fn threads_of(window: &Window) -> Option<At> {
    if !window.of_pi() {
        return None;
    }
    let scopes = kitty_scopes(&account()?);
    let started: Vec<(String, Vec<String>)> = scopes
        .iter()
        .filter_map(|scope| {
            let unit = scope.file_name()?.to_str()?.to_owned();
            let apps = processes(scope)
                .iter()
                .filter_map(|pid| fs::read(format!("/proc/{pid}/environ")).ok())
                .filter_map(|environ| path::app_id(&environ))
                .collect();
            Some((unit, apps))
        })
        .collect();
    let unit = path::scope_of(window, &started)?;
    let scope = scopes
        .iter()
        .find(|s| s.file_name().and_then(|n| n.to_str()) == Some(unit))?;
    Some(At {
        window: window.id,
        kitty: Watched::open(window.pid?)?,
        pi: Watched::open(path::pi_below(&processes(scope), &running())?)?,
    })
}

/// How long some task has waited for a CPU since boot, in microseconds.
fn cpu_waited_us() -> Option<u64> {
    context::waited(&fs::read_to_string("/proc/pressure/cpu").ok()?)
}

/// Every process running, by pid, parent and name, from /proc.
fn running() -> Vec<path::Process> {
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = fs::read_to_string(entry.path().join("stat")).ok()?;
            // The name sits in parentheses and may hold spaces, so the
            // parent is read after its last closing one.
            let (head, tail) = stat.rsplit_once(')')?;
            let name = head.split_once('(')?.1.to_owned();
            let parent = tail.split_whitespace().nth(1)?.parse().ok()?;
            Some(path::Process { pid, parent, name })
        })
        .collect()
}

fn processes(scope: &FsPath) -> Vec<u32> {
    fs::read_to_string(scope.join("cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .filter_map(|pid| pid.parse().ok())
        .collect()
}

/// The account's manager this runs under, from its own cgroup.
fn account() -> Option<PathBuf> {
    let own = fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own.lines().find_map(|l| l.strip_prefix("0::"))?;
    let end = path.find(".service").filter(|_| path.contains("/user@"))?;
    Some(
        PathBuf::from("/sys/fs/cgroup")
            .join(path[..end + ".service".len()].trim_start_matches('/')),
    )
}

/// Every kitty's scope under the manager, in whichever slice the
/// jobs-sorter has put it.
fn kitty_scopes(under: &FsPath) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut slices = vec![(under.to_path_buf(), 0)];
    while let Some((dir, depth)) = slices.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            let kind = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            let kitty = entry.file_name().to_string_lossy().starts_with("kitty-");
            if kitty && kind == "scope" {
                found.push(path);
            } else if kind == "slice" && depth < 3 {
                slices.push((path, depth + 1));
            }
        }
    }
    found
}

/// Asks niri one thing and gives its one-line answer.
fn ask(socket: &FsPath, request: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .ok()?;
    stream
        .write_all(format!("\"{request}\"\n").as_bytes())
        .ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    Some(line)
}

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

/// niri's socket, as niri names it to the session, or the first one in
/// the runtime directory, since a timer's service isn't given it.
fn niri_socket() -> Option<PathBuf> {
    if let Some(s) = std::env::var_os("NIRI_SOCKET") {
        return Some(PathBuf::from(s));
    }
    let mut found: Vec<PathBuf> = fs::read_dir(env_dir("XDG_RUNTIME_DIR", ".cache"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| socket_pid(p).is_some())
        .collect();
    found.sort();
    found.into_iter().next()
}

/// The pid of the niri a socket belongs to, which niri writes into its
/// name: `niri.<display>.<pid>.sock`.
fn socket_pid(socket: &FsPath) -> Option<u32> {
    let name = socket.file_name()?.to_str()?;
    let stem = name.strip_prefix("niri.")?.strip_suffix(".sock")?;
    stem.rsplit_once('.')?.1.parse().ok()
}

fn round(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

/// The share of keys pi held past a frame, by what held it.
fn held_by(summary: &model::Summary) -> serde_json::Value {
    let [ran, queued, disk] = summary.pi_lost_by;
    serde_json::json!({
        "running": round(ran, 4),
        "waiting_for_cpu": round(queued, 4),
        "disk": round(disk, 4),
    })
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Appends a sample's line to its UTC day's file.
fn keep(ended: u64, line: &str) -> std::io::Result<()> {
    let dir = env_dir("XDG_STATE_HOME", ".local/state").join("pi-keys");
    fs::create_dir_all(&dir)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(format!("{}.jsonl", day(ended))))?;
    writeln!(file, "{line}")
}

/// A day's name, YYYY-MM-DD in UTC, from seconds since 1970.
fn day(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).expect("a u64 of seconds is under 2^47 days");
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_named_in_utc() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(1_794_441_599), "2026-11-11");
        assert_eq!(day(1_794_441_600), "2026-11-12");
        assert_eq!(day(1_835_438_400), "2028-02-29");
    }

    #[test]
    fn niri_s_pid_is_read_from_its_socket_s_name() {
        assert_eq!(
            socket_pid(FsPath::new("/run/user/1000/niri.wayland-1.2561.sock")),
            Some(2561)
        );
        assert_eq!(socket_pid(FsPath::new("/run/user/1000/wayland-1")), None);
    }
}
