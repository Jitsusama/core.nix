//! A standing watch on whether what Joel uses waits.
//!
//! Every measurement of the machine's responsiveness had been a script
//! run once, so a change that made the desktop wait was found only by
//! feeling it, and by then nobody knew which change it was. lag-watch
//! reads, every ten seconds and without privilege, what the kernel already
//! counts: how long the desktop's and the apps' slices stalled on the CPU,
//! memory and the disk; what their memory lost to swap; how long niri's
//! main thread and PipeWire's data loops waited for a CPU each time they
//! were ready; and, every two seconds, how long niri takes to answer on
//! its socket, which it does from the same loop that reads input and
//! draws. Each window is tagged with the system generation and how hard
//! the work behind Joel was working, so like is compared with like.
//!
//! The latest window goes to $XDG_RUNTIME_DIR/lag-watch/status.json, for
//! the shell to show: its `trouble` in sentences; `worst`, one word for
//! what the wait Joel would feel most was on (cpu, memory, disk or niri),
//! and `who`, whose it was (desktop or apps), both empty when nothing
//! waited; and `since`, when the run of troubled windows began. Every
//! window to a day's file under
//! $XDG_STATE_HOME/lag-watch, kept two weeks.
//!
//! Usage: lag-watch run
//!        lag-watch now
//!        lag-watch report [--hours N]

mod kernel;
mod report;
mod window;

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use window::{Answers, Snapshot, Window};

const TICK: Duration = Duration::from_secs(2);
const TICKS_A_WINDOW: u32 = 5;
const KEPT_DAYS: u64 = 14;

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

fn runtime() -> PathBuf {
    env_dir("XDG_RUNTIME_DIR", ".cache").join("lag-watch")
}

fn history() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state").join("lag-watch")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => run(),
        Some("now") => now(),
        Some("report") => {
            let hours = args
                .iter()
                .position(|a| a == "--hours")
                .and_then(|i| args.get(i + 1))
                .and_then(|h| h.parse().ok())
                .unwrap_or(24);
            print!("{}", report::render(&read_history(hours)));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: lag-watch run | now | report [--hours N]");
            ExitCode::from(2)
        }
    }
}

/// The account's manager this runs under, from its own cgroup: the
/// slices it watches are this account's.
fn account(cgroups: &Path) -> Option<PathBuf> {
    let own = fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own.lines().find_map(|l| l.strip_prefix("0::"))?;
    let end = path.find(".service").filter(|_| path.contains("/user@"))?;
    let manager = &path[..end + ".service".len()];
    Some(cgroups.join(manager.trim_start_matches('/')))
}

fn snapshot(cgroups: &Path, account: &Path, threads: &[(&'static str, PathBuf)]) -> Snapshot {
    let mut s = Snapshot {
        at: started().elapsed().as_secs_f64(),
        ..Snapshot::default()
    };
    for (name, dir) in [
        ("session", account.join("session.slice")),
        ("app", account.join("app.slice")),
        ("jobs", account.join("jobs.slice")),
        ("build", cgroups.join("build.slice")),
    ] {
        if let Some(g) = kernel::group(&dir) {
            s.groups.insert(name.to_string(), g);
        }
    }
    for entry in fs::read_dir(account.join("app.slice"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        let unit = path
            .extension()
            .is_some_and(|e| e == "service" || e == "scope" || e == "slice");
        if unit && let Some(g) = kernel::group(&path) {
            s.apps
                .insert(entry.file_name().to_string_lossy().to_string(), g);
        }
    }
    for (label, path) in threads {
        let text = fs::read_to_string(path).unwrap_or_default();
        if let Some(sched) = kernel::sched(&text) {
            s.threads
                .insert((label.to_string(), path.display().to_string()), sched);
        }
    }
    s
}

fn started() -> Instant {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// niri's socket, as niri names it to the session, or the first one in
/// the runtime directory.
fn niri_socket() -> Option<PathBuf> {
    if let Some(s) = std::env::var_os("NIRI_SOCKET") {
        return Some(PathBuf::from(s));
    }
    let dir = env_dir("XDG_RUNTIME_DIR", ".cache");
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            n.starts_with("niri.") && p.extension().is_some_and(|e| e == "sock")
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// The pid of the niri a socket belongs to, which niri writes into its
/// name: `niri.<display>.<pid>.sock`.
fn socket_pid(socket: &Path) -> Option<u32> {
    let name = socket.file_name()?.to_str()?;
    let stem = name.strip_prefix("niri.")?.strip_suffix(".sock")?;
    stem.rsplit_once('.')?.1.parse().ok()
}

/// The niri drawing the desktop: the one answering on the session's socket.
fn compositor() -> Option<u32> {
    niri_socket().as_deref().and_then(socket_pid)
}

/// Asks niri its version and times the answer, in milliseconds.
fn ask_niri(socket: &Path) -> Option<f64> {
    let at = Instant::now();
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .ok()?;
    stream.write_all(b"\"Version\"\n").ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    line.contains("Ok")
        .then(|| at.elapsed().as_secs_f64() * 1000.0)
}

fn system() -> String {
    fs::read_link("/run/current-system")
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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

fn keep(window: &Window) {
    let dir = runtime();
    let _ = fs::create_dir_all(&dir);
    let text = serde_json::to_string_pretty(window).expect("a window serialises");
    let fresh = dir.join("status.json.new");
    if fs::write(&fresh, text)
        .and_then(|()| fs::rename(&fresh, dir.join("status.json")))
        .is_err()
    {
        eprintln!("couldn't write {}", dir.display());
    }
    let dir = history();
    let _ = fs::create_dir_all(&dir);
    let line = serde_json::to_string(window).expect("a window serialises");
    let path = dir.join(format!("{}.jsonl", day(window.ended)));
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "{line}");
        }
        Err(e) => eprintln!("couldn't write {}: {e}", path.display()),
    }
}

/// Lets go of days older than two weeks.
fn prune() {
    let oldest = day(seconds_now().saturating_sub(KEPT_DAYS * 86_400));
    let Ok(entries) = fs::read_dir(history()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let history = entry.path().extension().is_some_and(|e| e == "jsonl");
        if history && name < format!("{oldest}.jsonl") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn read_history(hours: u64) -> Vec<Window> {
    let since = seconds_now().saturating_sub(hours * 3600);
    let mut names: Vec<PathBuf> = fs::read_dir(history())
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    names.sort();
    let mut all = Vec::new();
    for path in names
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
    {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        all.extend(
            text.lines()
                .filter_map(|l| serde_json::from_str::<Window>(l).ok())
                .filter(|w| w.ended >= since),
        );
    }
    all
}

/// The latest window, in a few lines.
fn now() -> ExitCode {
    let Ok(text) = fs::read_to_string(runtime().join("status.json")) else {
        eprintln!("lag-watch isn't running");
        return ExitCode::FAILURE;
    };
    let Ok(w) = serde_json::from_str::<Window>(&text) else {
        eprintln!("lag-watch's status can't be read");
        return ExitCode::FAILURE;
    };
    let age = seconds_now().saturating_sub(w.ended);
    println!("{:?} work behind, {age} s ago", w.band);
    if w.trouble.is_empty() {
        println!("nothing waited");
    }
    for line in &w.trouble {
        println!("  {line}");
    }
    let wait = |l: &str| w.waits.get(l).map_or(0.0, |x| x.mean_us);
    println!(
        "niri answers in {:.1} ms (p99 {:.1}); niri waits {:.0} us a turn, sound {:.0} us",
        w.niri.p50_ms,
        w.niri.p99_ms,
        wait("niri"),
        wait("pipewire")
    );
    ExitCode::SUCCESS
}

fn run() -> ExitCode {
    let cgroups = PathBuf::from("/sys/fs/cgroup");
    let Some(account) = account(&cgroups) else {
        eprintln!("not under an account's manager, so no slices to watch");
        return ExitCode::FAILURE;
    };
    let proc = PathBuf::from("/proc");
    started();
    let mut threads = kernel::threads(&proc, compositor());
    let mut then = snapshot(&cgroups, &account, &threads);
    let mut answers: Vec<f64> = Vec::new();
    let mut unanswered = 0;
    let mut tick = 0u32;
    let mut windows = 0u64;
    let mut last: Option<Window> = None;
    loop {
        thread::sleep(TICK);
        match niri_socket().and_then(|s| ask_niri(&s)) {
            Some(ms) => answers.push(ms),
            None => unanswered += 1,
        }
        tick += 1;
        if tick < TICKS_A_WINDOW {
            continue;
        }
        tick = 0;
        let now = snapshot(&cgroups, &account, &threads);
        let mut w = window::between(
            &then,
            &now,
            Answers::of(&answers, unanswered),
            seconds_now(),
            &system(),
        );
        w.carry(last.as_ref());
        keep(&w);
        last = Some(w);
        answers.clear();
        unanswered = 0;
        windows += 1;
        // Threads come and go with their programs: looked for again each
        // minute, with the snapshot taken again so no window spans two.
        if windows.is_multiple_of(6) {
            threads = kernel::threads(&proc, compositor());
            then = snapshot(&cgroups, &account, &threads);
        } else {
            then = now;
        }
        if windows.is_multiple_of(360) {
            prune();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_named_in_utc() {
        assert_eq!(day(0), "1970-01-01");
        // 19:26 UTC on 23 October 2026, and 23:00, which is the 24th.
        assert_eq!(day(1_792_783_560), "2026-10-23");
        assert_eq!(day(1_792_800_000), "2026-10-24");
        assert_eq!(day(951_782_400), "2000-02-29");
    }

    #[test]
    fn the_compositor_is_the_niri_its_socket_names() {
        assert_eq!(
            socket_pid(Path::new("/run/user/1000/niri.wayland-1.2005.sock")),
            Some(2005)
        );
        assert_eq!(socket_pid(Path::new("/run/user/1000/niri.sock")), None);
        assert_eq!(socket_pid(Path::new("/run/user/1000/other")), None);
    }
}
