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
//! threads, each apart; how long Chrome's and Slack's drawing threads
//! waited, at all times, leaving out tabs the app lowered as hidden, for
//! the hourly measure cadence judges to read; and, every two seconds,
//! how long niri takes to answer on its socket, which it does from the
//! same loop that reads input and draws. Each window is tagged with the
//! system generation and how hard the work behind Joel was working, so
//! like is compared with like.
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

mod apps;
mod kernel;
mod now;
mod report;
mod window;

use std::collections::{BTreeMap, BTreeSet};
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

/// The cgroup folder of the account's manager this runs under, as
/// [`manager`] names it: the slices it watches are this account's.
fn account(cgroups: &Path, manager: &str) -> PathBuf {
    cgroups.join(manager.trim_start_matches('/'))
}

/// The account's manager in a process's cgroup, as `/proc/<pid>/cgroup`
/// names it, such as `/user.slice/user-1000.slice/user@1000.service`.
fn manager(cgroup: &str) -> Option<&str> {
    let path = cgroup.lines().find_map(|l| l.strip_prefix("0::"))?;
    let end = path.find(".service").filter(|_| path.contains("/user@"))?;
    Some(&path[..end + ".service".len()])
}

fn snapshot(
    cgroups: &Path,
    account: &Path,
    threads: &[kernel::Thread],
    unwatched: BTreeMap<String, usize>,
) -> Snapshot {
    let mut s = Snapshot {
        at: started().elapsed().as_secs_f64(),
        unwatched,
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
    // A process its app lowered, a hidden tab, is nobody's wait: counted,
    // not read. The app lowers only the tab's main thread, so that one's
    // nice, in the process's own stat, leaves out its compositor with it.
    // It's read each window, since a tab shown again is raised again, and
    // its threads' schedstat is kept as where the next window starts, so a
    // tab shown in a window is counted for all of it. One hidden in a
    // window is left out of all of it, as its nice at the end says.
    let mut lowered: BTreeMap<&Path, Option<&str>> = BTreeMap::new();
    for thread in threads {
        let mut hidden = false;
        if apps::is_app(thread)
            && let Some(process) = apps::process(thread)
        {
            hidden = lowered
                .entry(process)
                .or_insert_with(|| {
                    let stat = fs::read_to_string(process.join("stat")).unwrap_or_default();
                    kernel::nice(&stat)
                        .is_some_and(|n| n > 0)
                        .then_some(thread.label)
                })
                .is_some();
        }
        // An app's thread blocked in the kernel, on a lock, a page read back or reclaim, hangs
        // its window without waiting for a CPU, so its time blocked is read beside its waits.
        if apps::is_app(thread)
            && let Some(ns) =
                kernel::blocked(&kernel::text(&thread.schedstat.with_file_name("sched")))
        {
            s.blocked.insert(thread.clone(), ns);
        }
        let text = fs::read_to_string(&thread.schedstat).unwrap_or_default();
        if let Some(sched) = kernel::sched(&text) {
            if hidden {
                s.resting.insert(thread.clone(), sched);
            } else {
                s.threads.insert(thread.clone(), sched);
            }
        }
    }
    for label in lowered.into_values().flatten() {
        *s.lowered.entry(label.to_string()).or_default() += 1;
    }
    kernel::while_in_a_call(&mut s.threads);
    s
}

/// A read of the threads for `window::book`: when it began, each one's
/// schedstat, and which were asleep just before theirs was read.
struct Read {
    at: f64,
    sched: BTreeMap<kernel::Thread, kernel::Sched>,
    asleep: BTreeSet<kernel::Thread>,
}

/// Seconds since lag-watch started, which reads are timed by.
fn clock() -> f64 {
    started().elapsed().as_secs_f64()
}

/// The threads asleep now, each by its `stat`, read before its schedstat
/// so a wait that began after it can't be taken for one before it.
fn asleep(threads: &[kernel::Thread]) -> BTreeSet<kernel::Thread> {
    threads
        .iter()
        .filter(|t| kernel::asleep(&kernel::text(&t.schedstat.with_file_name("stat"))))
        .cloned()
        .collect()
}

/// A tick's read of the threads: which are asleep, then each schedstat.
fn reads(threads: &[kernel::Thread]) -> Read {
    let at = clock();
    let asleep = asleep(threads);
    let sched = threads
        .iter()
        .filter_map(|t| {
            let text = fs::read_to_string(&t.schedstat).ok()?;
            Some((t.clone(), kernel::sched(&text)?))
        })
        .collect();
    Read { at, sched, asleep }
}

/// A snapshot's reads of its threads, hidden tabs' included, as a tick's.
fn read_of(s: &Snapshot) -> BTreeMap<kernel::Thread, kernel::Sched> {
    let mut all = s.resting.clone();
    all.extend(s.threads.iter().map(|(k, v)| (k.clone(), *v)));
    all
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
    // One read of its own cgroup gives both the slices and the apps, so
    // they can't disagree about whose they are.
    let own = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let Some(manager) = manager(&own) else {
        return Err(
            Failure::new(Exit::Unavailable, "there are no slices to watch")
                .because("it isn't running under an account's manager")
                .try_this("systemctl --user start lag-watch"),
        );
    };
    let proc = PathBuf::from("/proc");
    let account = account(&cgroups, manager);
    started();
    let mut apps = apps::Apps::of_account(manager).reading_transient_units_from(
        env_dir("XDG_RUNTIME_DIR", ".cache").join("systemd/transient"),
    );
    let mut threads = kernel::threads(&proc, compositor(), &tuple_unit(&account), &mut apps);
    let (at, sleeping) = (clock(), asleep(&threads));
    let mut then = snapshot(&cgroups, &account, &threads, apps.unwatched());
    // Each tick's read of the threads, and what those reads booked rather
    // than waited, since the window began (`window::book`). Two reads are
    // timed from before the first began to after the second ended, so they
    // never seem closer than they were.
    let mut read = Read {
        at,
        sched: read_of(&then),
        asleep: sleeping,
    };
    let mut booked = BTreeMap::new();
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
            let after = reads(&threads);
            let passed = clock() - read.at;
            window::book(&mut booked, &read.sched, &read.asleep, &after.sched, passed);
            read = after;
            continue;
        }
        tick = 0;
        let (at, sleeping) = (clock(), asleep(&threads));
        let mut now = snapshot(&cgroups, &account, &threads, apps.unwatched());
        let after = Read {
            at,
            sched: read_of(&now),
            asleep: sleeping,
        };
        let passed = clock() - read.at;
        window::book(&mut booked, &read.sched, &read.asleep, &after.sched, passed);
        read = after;
        now.booked = std::mem::take(&mut booked);
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
            threads = kernel::threads(&proc, compositor(), &tuple_unit(&account), &mut apps);
            let (at, sleeping) = (clock(), asleep(&threads));
            then = snapshot(&cgroups, &account, &threads, apps.unwatched());
            read = Read {
                at,
                sched: read_of(&then),
                asleep: sleeping,
            };
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
    fn a_tab_its_app_lowered_is_counted_and_kept_only_as_where_the_next_window_starts() {
        let dir = std::env::temp_dir().join(format!("lag-watch-lowered-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let thread = |label: &'static str, pid: &str, tid: &str, nice: i64| {
            let at = dir.join(format!("{pid}/task/{tid}"));
            fs::create_dir_all(&at).unwrap();
            fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
            let stat = format!("{tid} (chrome) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 {nice} 30 0\n");
            fs::write(at.join("stat"), &stat).unwrap();
            if pid == tid {
                fs::write(dir.join(pid).join("stat"), &stat).unwrap();
            }
            kernel::Thread {
                label,
                name: "chrome".into(),
                schedstat: at.join("schedstat"),
                call: false,
            }
        };
        // A hidden tab: its main thread lowered, its compositor raised to
        // draw, as Chromium leaves them. A tab shown, its main thread at
        // Chromium's own nice. A tab whose stat is gone, ending. And niri,
        // whose nice is never asked.
        let hidden = thread("chrome-ui", "72", "72", 5);
        let hidden_drawing = thread("chrome-ui", "72", "73", -8);
        let shown = thread("chrome-ui", "75", "75", 0);
        let shown_drawing = thread("chrome-ui", "75", "76", -8);
        let ending = thread("slack-ui", "78", "79", -8);
        let niri = thread("niri", "74", "74", 5);
        let s = snapshot(
            &dir,
            &dir,
            // The compositor first, so its own nice is never what's asked.
            &[
                hidden_drawing.clone(),
                hidden.clone(),
                shown.clone(),
                shown_drawing.clone(),
                ending.clone(),
                niri.clone(),
            ],
            BTreeMap::new(),
        );
        let read: Vec<&kernel::Thread> = s.threads.keys().collect();
        assert_eq!(read, [&shown, &shown_drawing, &niri, &ending]);
        let resting: Vec<&kernel::Thread> = s.resting.keys().collect();
        assert_eq!(resting, [&hidden, &hidden_drawing], "the hidden tab rests");
        assert_eq!(
            s.lowered,
            BTreeMap::from([("chrome-ui".to_string(), 1)]),
            "one hidden tab, counted once"
        );
        // Shown again, the app raises the tab, and the next window reads it.
        let stat = "72 (chrome) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0\n";
        fs::write(dir.join("72/stat"), stat).unwrap();
        let s = snapshot(
            &dir,
            &dir,
            std::slice::from_ref(&hidden_drawing),
            BTreeMap::new(),
        );
        assert_eq!(s.threads.keys().collect::<Vec<_>>(), [&hidden_drawing]);
        assert!(s.lowered.is_empty(), "{:?}", s.lowered);
        assert!(s.resting.is_empty(), "{:?}", s.resting);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_app_thread_s_time_blocked_is_read_shown_or_hidden_and_no_one_else_s() {
        let dir = std::env::temp_dir().join(format!("lag-watch-blocked-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let thread = |label: &'static str, pid: &str, nice: i64, blocked: Option<&str>| {
            let at = dir.join(format!("{pid}/task/{pid}"));
            fs::create_dir_all(&at).unwrap();
            fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
            let stat = format!("{pid} (slack) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 {nice} 30 0\n");
            fs::write(at.join("stat"), &stat).unwrap();
            fs::write(dir.join(pid).join("stat"), &stat).unwrap();
            if let Some(ms) = blocked {
                let sched = format!("slack ({pid}, #threads: 9)\nstats.sum_block_runtime : {ms}\n");
                fs::write(at.join("sched"), sched).unwrap();
            }
            kernel::Thread {
                label,
                name: "slack".into(),
                schedstat: at.join("schedstat"),
                call: false,
            }
        };
        let shown = thread("slack-ui", "81", 0, Some("12.500000"));
        let hidden = thread("chrome-ui", "82", 5, Some("3.000000"));
        let unkept = thread("slack-ui", "83", 0, None);
        let niri = thread("niri", "84", 0, Some("7.000000"));
        let s = snapshot(
            &dir,
            &dir,
            &[shown.clone(), hidden.clone(), unkept, niri],
            BTreeMap::new(),
        );
        assert_eq!(
            s.blocked,
            BTreeMap::from([(shown, 12_500_000), (hidden.clone(), 3_000_000)]),
            "only the apps' threads, shown or hidden, had their time blocked read"
        );
        // Read, the hidden tab's time blocked is still left out of its window, which counts
        // only the threads a window ends with shown.
        assert!(
            s.resting.contains_key(&hidden) && !s.threads.contains_key(&hidden),
            "the tab raised past nice 0 wasn't read as hidden: resting {:?}, shown {:?}",
            s.resting.keys().map(|t| t.label).collect::<Vec<_>>(),
            s.threads.keys().map(|t| t.label).collect::<Vec<_>>()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_hidden_tab_is_left_out_on_the_paths_the_watch_finds_its_threads_by() {
        use kernel::tests::{ACCOUNT, opened, write};
        let proc = std::env::temp_dir().join(format!("lag-watch-found-{}", std::process::id()));
        let _ = fs::remove_dir_all(&proc);
        // A renderer whose main thread Chrome lowered, as it does a hidden
        // tab's, found as the watch finds it, not as a test builds it.
        opened(&proc, "72", "chrome");
        write(
            &proc.join("72/stat"),
            "72 (chrome) S 1 1 1 0 -1 0 1 0 0 0 5 6 0 0 25 5 30 0 99\n",
        );
        for (tid, comm) in [("72", "chrome"), ("73", "Compositor")] {
            write(
                &proc.join(format!("72/task/{tid}/comm")),
                &format!("{comm}\n"),
            );
            write(&proc.join(format!("72/task/{tid}/schedstat")), "10 20 3\n");
        }
        let mut apps = apps::Apps::of_account(ACCOUNT);
        let threads = kernel::threads(&proc, None, &proc.join("tuple.service"), &mut apps);
        assert_eq!(threads.len(), 2, "the renderer's two threads: {threads:?}");
        let s = snapshot(&proc, &proc, &threads, apps.unwatched());
        assert!(
            s.threads.is_empty(),
            "a hidden tab was read: {:?}",
            s.threads
        );
        assert_eq!(s.lowered, BTreeMap::from([("chrome-ui".to_string(), 1)]));
        fs::remove_dir_all(&proc).unwrap();
    }

    #[test]
    fn the_account_is_the_manager_in_the_watch_s_own_cgroup() {
        let own =
            "0::/user.slice/user-1000.slice/user@1000.service/session.slice/lag-watch.service\n";
        assert_eq!(
            manager(own),
            Some("/user.slice/user-1000.slice/user@1000.service")
        );
        // As the apps take it, a process of Joel's is found by it.
        let apps = apps::Apps::of_account(manager(own).unwrap());
        let scope = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-a.scope\n";
        assert!(
            apps.launched_by_joel(scope),
            "Joel's app isn't found by the account"
        );
        assert_eq!(manager("0::/system.slice/lag-watch.service\n"), None);
        assert_eq!(
            account(Path::new("/sys/fs/cgroup"), manager(own).unwrap()),
            Path::new("/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service")
        );
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
