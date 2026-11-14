//! A standing watch on whether what Joel uses waits.
//!
//! Every measurement of the machine's responsiveness had been a script
//! run once, so a change that made the desktop wait was found only by
//! feeling it, and by then nobody knew which change it was. lag-watch
//! reads, every ten seconds and without privilege, what the kernel already
//! counts: how long the desktop's and the apps' slices stalled on the CPU,
//! memory and the disk; what their memory lost to swap; how long niri's
//! main thread and PipeWire's data loops waited for a CPU each time they
//! were ready, and in a call the worst of Tuple's and of Chrome's call
//! threads, each apart; and, every two seconds, how long niri takes to answer on
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
//! `lag-watch` and `lag-watch now` say what the last window was like, and
//! `lag-watch report` sums up the history; each answers in the form
//! `--format` asks for, as every command of ours does (docs/commands.md).
//! `lag-watch run` is the watch itself, as its service runs it.

mod kernel;
mod now;
mod report;
mod window;

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use terminal_kit::{Context, Exit, Failure, Format, Help, flags};
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

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "lag-watch",
        "whether what Joel uses waits, watched every ten seconds",
    )
    .example("lag-watch", "what the last ten seconds were like")
    .example(
        "lag-watch report --hours 4",
        "the last four hours, by generation and load",
    )
    .example("lag-watch report --json", "the last day, for a script")
    .subcommand(
        Help::new("now", "the last ten seconds, the same as a bare lag-watch")
            .example("lag-watch now", "what the last ten seconds were like")
            .example("lag-watch now --json", "the same, for a script")
            .exit(Exit::BadInput, "its status can't be read")
            .exit(Exit::Unavailable, "the watch isn't running"),
    )
    .subcommand(
        Help::new("report", "the history summed up by generation and load")
            .example("lag-watch report", "the last day")
            .example("lag-watch report --hours 4", "the last four hours")
            .option("--hours N", "how far back it reads, 24 unless given")
            .exit(Exit::BadInput, "the history can't be read"),
    )
    .subcommand(
        Help::new("run", "the watch itself, as its service runs it")
            .example("lag-watch run", "watches until stopped")
            .exit(Exit::Unavailable, "it has nothing to watch"),
    )
    .exit(Exit::BadInput, "its status can't be read")
    .exit(
        Exit::Unavailable,
        "the watch isn't running, or has nothing to watch",
    )
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let (command, rest) = match context.args.split_first() {
            Some((command, rest)) => (command.as_str(), rest),
            None => ("now", &[][..]),
        };
        match command {
            "now" => none_after(rest).and_then(|()| now(context)),
            "report" => hours(rest).and_then(|hours| answer_report(context, hours)),
            "run" => none_after(rest).and_then(|()| run()),
            word => Err(flags::unknown("command", word, &commands)),
        }
    })
}

/// Nothing more on the line, where a command takes nothing; `--verbose`
/// is the kit's.
fn none_after(rest: &[String]) -> Result<(), Failure> {
    match rest
        .iter()
        .find(|arg| !matches!(arg.as_str(), "--verbose" | "-v"))
    {
        Some(arg) => Err(Failure::new(Exit::Usage, format!("{arg} isn't taken here"))
            .because("now and run take nothing more")
            .try_this("lag-watch --help")),
        None => Ok(()),
    }
}

/// How far back `report` reads, from its `--hours N`: a day unless given.
fn hours(rest: &[String]) -> Result<u64, Failure> {
    let mut hours = 24;
    let mut args = rest.iter();
    while let Some(arg) = args.next() {
        let given = match arg.as_str() {
            "--verbose" | "-v" => continue,
            "--hours" => args.next().map(String::as_str),
            other => match other.strip_prefix("--hours=") {
                Some(given) => Some(given),
                None => return Err(flags::unknown("option", other, &["--hours"])),
            },
        };
        hours = given.and_then(|given| given.parse().ok()).ok_or_else(|| {
            Failure::new(Exit::Usage, "--hours takes a whole number of hours")
                .because(format!("{} isn't one", given.unwrap_or("nothing")))
                .try_this("lag-watch report --hours 4")
        })?;
    }
    Ok(hours)
}

/// The failure for an answer that couldn't be written.
fn unwritten(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

/// The history over the last `hours`, summed up in the form asked for.
fn answer_report(context: &Context, hours: u64) -> Result<(), Failure> {
    let generations = report::summary(&read_history(hours));
    match context.format() {
        Format::Text => context.say(&report::text(&generations, &context.out, context.room)),
        _ => context.answer(&report::json(&generations, hours)),
    }
    .map_err(|error| unwritten(&error))
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

fn snapshot(cgroups: &Path, account: &Path, threads: &[kernel::Thread]) -> Snapshot {
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
    for thread in threads {
        let text = fs::read_to_string(&thread.schedstat).unwrap_or_default();
        if let Some(sched) = kernel::sched(&text) {
            s.threads.insert(thread.clone(), sched);
        }
    }
    kernel::while_in_a_call(&mut s.threads);
    s
}

/// Tuple's daemon's unit, whose cgroup names its processes.
fn tuple_unit(account: &Path) -> PathBuf {
    account.join("app.slice/tuple.service")
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

/// Writes `name` in `dir` whole or not at all: made in a `.writing` folder
/// beside it, then renamed in. The shell watches `dir`, so a file made and
/// written there woke it for each step; made aside, only the rename is
/// heard.
fn write_aside(dir: &Path, name: &str, text: &str) -> std::io::Result<()> {
    let writing = dir.join(".writing");
    fs::create_dir_all(&writing)?;
    let fresh = writing.join(format!("{name}.new"));
    fs::write(&fresh, text)?;
    fs::rename(&fresh, dir.join(name))
}

fn keep(window: &Window) {
    let dir = runtime();
    let text = serde_json::to_string_pretty(window).expect("a window serialises");
    if write_aside(&dir, "status.json", &text).is_err() {
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

/// The latest window, in the form asked for.
fn now(context: &Context) -> Result<(), Failure> {
    let status = runtime().join("status.json");
    let text = fs::read_to_string(&status).map_err(|error| {
        Failure::new(Exit::Unavailable, "lag-watch isn't running")
            .because(format!("{} can't be read: {error}", status.display()))
            .try_this("systemctl --user start lag-watch")
    })?;
    let window = serde_json::from_str::<Window>(&text).map_err(|error| {
        Failure::new(Exit::BadInput, "lag-watch's status can't be read")
            .because(format!("{}: {error}", status.display()))
            .try_this("systemctl --user restart lag-watch")
    })?;
    let age = seconds_now().saturating_sub(window.ended);
    match context.format() {
        Format::Text => context.say(&now::text(&window, age, &context.out, context.room)),
        _ => context.answer(&now::json(&window, age)),
    }
    .map_err(|error| unwritten(&error))
}

/// The watch, a window every ten seconds until it's stopped.
fn run() -> Result<(), Failure> {
    let cgroups = PathBuf::from("/sys/fs/cgroup");
    let Some(account) = account(&cgroups) else {
        return Err(
            Failure::new(Exit::Unavailable, "there are no slices to watch")
                .because("it isn't running under an account's manager")
                .try_this("systemctl --user start lag-watch"),
        );
    };
    let proc = PathBuf::from("/proc");
    started();
    let mut threads = kernel::threads(&proc, compositor(), &tuple_unit(&account));
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
            threads = kernel::threads(&proc, compositor(), &tuple_unit(&account));
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

    // The status folder only ever holds the status and the folder it's
    // written in, and that's left empty.
    #[test]
    fn the_status_is_written_aside() {
        let dir = std::env::temp_dir().join(format!("lag-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_aside(&dir, "status.json", "{}").unwrap();
        write_aside(&dir, "status.json", "{\"a\":1}").unwrap();
        let mut names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, [".writing", "status.json"]);
        assert_eq!(fs::read_dir(dir.join(".writing")).unwrap().count(), 0);
        assert_eq!(
            fs::read_to_string(dir.join("status.json")).unwrap(),
            "{\"a\":1}"
        );
        fs::remove_dir_all(&dir).unwrap();
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
