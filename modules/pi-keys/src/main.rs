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
//! to $XDG_STATE_HOME/pi-keys/<UTC day>.jsonl, for cadence's daily read.
//! It reads only /proc, /sys and niri's answers, never touches a window,
//! and stops the moment a call starts.
//!
//! Usage: pi-keys sample [--minutes N] [--wait N] [--call-check PROGRAM]

mod burst;
mod model;
mod path;

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
    match args.first().map(String::as_str) {
        Some("sample") => sample(&Settings {
            minutes: number("--minutes", 10.0),
            wait: number("--wait", 60.0),
            call_check: value("--call-check").map(PathBuf::from),
        }),
        _ => {
            eprintln!("usage: pi-keys sample [--minutes N] [--wait N] [--call-check PROGRAM]");
            ExitCode::from(2)
        }
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
}

/// The threads of the window Joel is at, when it's a pi window whose
/// session is found.
struct At {
    window: u64,
    kitty: Watched,
    pi: Watched,
}

fn sample(settings: &Settings) -> ExitCode {
    let Some(socket) = niri_socket() else {
        eprintln!("pi-keys: no niri to ask, so no sample");
        return ExitCode::SUCCESS;
    };
    let Some(mut niri) = socket_pid(&socket).and_then(Watched::open) else {
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
    let begun = Instant::now();
    let ms = || begun.elapsed().as_secs_f64() * 1000.0;
    let enough = settings.minutes * 60_000.0;
    let mut frame = ask(&socket, "FocusedOutput")
        .as_deref()
        .and_then(path::frame)
        .unwrap_or(SIXTY);
    let (mut niri_spans, mut kitty_spans, mut pi_spans) = (Vec::new(), Vec::new(), Vec::new());
    let mut at_pi: Vec<(f64, f64)> = Vec::new();
    let mut since: Option<f64> = None;
    let mut at: Option<At> = None;
    let mut windows = HashSet::new();
    let (mut gaps, mut call) = (0u32, false);
    let mut focus_at = f64::NEG_INFINITY;
    let mut calls_at = 0.0;
    let mut checking: Option<Child> = None;
    let mut last = ms();
    let mut next = Instant::now();
    loop {
        next += TICK;
        match next.checked_duration_since(Instant::now()) {
            Some(wait) => thread::sleep(wait),
            None => next = Instant::now(),
        }
        let now = ms();
        let interval = now - last;
        last = now;
        if interval > GAP {
            gaps += 1;
            niri.bursts.reset();
            if let Some(at) = at.as_mut() {
                at.kitty.bursts.reset();
                at.pi.bursts.reset();
            }
            if let Some(from) = since {
                at_pi.push((from, now - interval));
                since = Some(now);
            }
        }
        if now - focus_at >= FOCUS {
            focus_at = now;
            let window = ask(&socket, "FocusedWindow")
                .as_deref()
                .and_then(path::focused);
            if at.as_ref().map(|a| a.window) != window.as_ref().map(|w| w.id) {
                at = window.as_ref().and_then(threads_of);
                if let Some(from) = since.take() {
                    at_pi.push((from, now));
                }
                if let Some(found) = at.as_ref() {
                    windows.insert(found.window);
                    since = Some(now);
                    if let Some(f) = ask(&socket, "FocusedOutput")
                        .as_deref()
                        .and_then(path::frame)
                    {
                        frame = f;
                    }
                }
            }
        }
        let take = |watched: &mut Watched, spans: &mut Vec<(f64, f64)>| {
            let Some(reading) = watched.read() else {
                return false;
            };
            if let Some(b) = watched.bursts.take(now, interval, reading)
                && b.solid()
                && b.worth()
            {
                spans.push((b.at, b.at + b.ms));
            }
            true
        };
        take(&mut niri, &mut niri_spans);
        if let Some(found) = at.as_mut() {
            let alive =
                take(&mut found.kitty, &mut kitty_spans) && take(&mut found.pi, &mut pi_spans);
            if !alive {
                eprintln!(
                    "pi-keys: thread {} or {} ended",
                    found.kitty.tid, found.pi.tid
                );
                at = None;
                if let Some(from) = since.take() {
                    at_pi.push((from, now));
                }
            }
        }
        if let Some(program) = settings.call_check.as_deref() {
            match checking.as_mut().map(Child::try_wait) {
                Some(Ok(Some(status))) => {
                    checking = None;
                    if status.code() == Some(0) {
                        call = true;
                        break;
                    }
                }
                Some(Ok(None)) => {}
                Some(Err(_)) => checking = None,
                None if now - calls_at >= CALLS => {
                    calls_at = now;
                    checking = Command::new(program)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .ok();
                }
                None => {}
            }
        }
        let held: f64 =
            at_pi.iter().map(|(a, b)| b - a).sum::<f64>() + since.map_or(0.0, |s| now - s);
        if held >= enough || now >= 3.0 * enough {
            break;
        }
    }
    if let Some(mut child) = checking {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(from) = since {
        at_pi.push((from, last));
    }
    let path = model::Path {
        niri: model::Spans::new(niri_spans),
        kitty: model::Spans::new(kitty_spans),
        pi: model::Spans::new(pi_spans),
    };
    let ended = seconds_now();
    let keys = model::keys(&path, &at_pi, frame, KEYS, &mut model::Draw::new(ended));
    let seconds: f64 = at_pi.iter().map(|(a, b)| b - a).sum::<f64>() / 1000.0;
    let Some(sum) = model::summary(&keys) else {
        eprintln!("pi-keys: no time at a pi window was sampled, so no sample");
        return ExitCode::SUCCESS;
    };
    let line = serde_json::json!({
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
        "frame_ms": round(frame, 2),
        "gaps": gaps,
        "windows": windows.len(),
        "call": call,
    });
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
        Command::new(program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
    let named: Vec<(u32, String)> = processes(scope)
        .into_iter()
        .filter_map(|pid| {
            let comm = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            Some((pid, comm.trim_end().to_owned()))
        })
        .collect();
    Some(At {
        window: window.id,
        kitty: Watched::open(window.pid?)?,
        pi: Watched::open(path::pi_of(&named)?)?,
    })
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
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("kitty-") && name.ends_with(".scope") {
                found.push(path);
            } else if name.ends_with(".slice") && depth < 3 {
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
