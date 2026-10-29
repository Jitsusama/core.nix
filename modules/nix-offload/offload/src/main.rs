//! Lends the machine a builder elsewhere while its machine queue holds
//! builds back, and gives it back once it's idle.
//!
//! Usage: `nix-offload serve --helper PATH --prefix NAME --rate DOLLARS
//! --last-start HH:MM --deadline HH:MM`, or `nix-offload report --rate
//! DOLLARS` for today's line.
//!
//! Every 20 seconds it reads the machine queue. Once a build or a VM check
//! has waited three minutes for a slot this machine doesn't have, it asks
//! the helper for a builder, names it `NAME<seconds>`, and writes the Nix
//! machines line the helper gives into /run/nix-offload/machines, which
//! nix-daemon reads at each build (`builders = @` it) and the queue reads
//! to count the builder's room. Once nothing has used it, and nothing has
//! wanted it, for fifteen minutes, or the deadline comes, it empties the
//! file first, so no build is sent to a builder that's going, then gives
//! the builder back. It starts none after the last start, so none is
//! made to live minutes, and gives back any it finds left over when it
//! starts and after each deadline, so none is kept overnight.
//!
//! The helper says how, and this when, so this knows nothing of where
//! the builder lives:
//! - `helper up NAME SECONDS` makes it, to live at most SECONDS, the time
//!   left until the deadline, prints its machines line once Nix can use
//!   it, and stays running while it's there, holding the way to it; it
//!   gives that up when told to end (SIGTERM). The seconds are counted
//!   here, in the zone TZ names, so the helper needs no time zones of its
//!   own: a shell without zoneinfo reads a zone's 20:00 as UTC's.
//! - `helper down NAME` gives it back, and succeeds once it's gone,
//!   whether or not it was there.
//! - `helper alive PREFIX` names, a line each, the builders there now.
//!
//! Each start, stop and leftover is a line in the journal under XDG's
//! state folder, `nix-offload/journal.jsonl`, which `report` sums.

mod journal;
mod queue;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

use journal::Event;

/// How long a job waits before a builder is asked for.
const WAIT: u64 = 180;
/// How long a builder sits unused before it's given back.
const IDLE: u64 = 15 * 60;
/// How long a builder may take to be ready before it's given up on.
const READY: Duration = Duration::from_mins(15);
/// How long after a builder fails to be ready before another is asked.
const AFTER_FAILING: u64 = 30 * 60;
/// How often the queue is read.
const TICK: Duration = Duration::from_secs(20);
/// Where nix-daemon and the queue read the builder's line.
const MACHINES: &str = "/run/nix-offload/machines";

const USED: &str = "Usage: nix-offload serve --helper PATH --prefix NAME --rate DOLLARS \
                    --last-start HH:MM --deadline HH:MM | nix-offload report --rate DOLLARS";

/// Set when the service is told to end, so the builder is given back first.
static ENDING: AtomicBool = AtomicBool::new(false);

extern "C" fn ending(_: libc::c_int) {
    ENDING.store(true, Ordering::SeqCst);
}

/// What `serve` was given.
struct Settings {
    helper: PathBuf,
    prefix: String,
    last_start: u32,
    deadline: u32,
    queue: PathBuf,
    machines: PathBuf,
    journal: PathBuf,
}

/// A builder lent to the machine.
struct Lent {
    name: String,
    helper: Child,
    host: String,
    since: u64,
    used: u64,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let given = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|at| arguments.get(at + 1))
            .cloned()
    };
    let Some(rate) = given("--rate").as_deref().and_then(journal::price) else {
        eprintln!("nix-offload: --rate takes a price, such as 0.68\n{USED}");
        return ExitCode::from(64);
    };
    let journal = state().join("nix-offload/journal.jsonl");
    match arguments.first().map(String::as_str) {
        Some("report") => {
            let today = journal::day(&fs::read_to_string(&journal).unwrap_or_default(), today());
            println!("{}", journal::said(&today, rate));
            ExitCode::SUCCESS
        }
        Some("serve") => {
            let minutes = |flag| given(flag).as_deref().and_then(minute_of);
            let (Some(helper), Some(prefix), Some(last_start), Some(deadline)) = (
                given("--helper"),
                given("--prefix"),
                minutes("--last-start"),
                minutes("--deadline"),
            ) else {
                eprintln!("nix-offload: serve needs each of its settings\n{USED}");
                return ExitCode::from(64);
            };
            if last_start > deadline {
                eprintln!("nix-offload: the last start comes after the deadline\n{USED}");
                return ExitCode::from(64);
            }
            let runtime = env::var_os("XDG_RUNTIME_DIR").map_or_else(
                || PathBuf::from("/run/user").join(unsafe { libc::getuid() }.to_string()),
                PathBuf::from,
            );
            serve(&Settings {
                helper: helper.into(),
                prefix,
                last_start,
                deadline,
                queue: runtime.join("jobs/queue.json"),
                machines: MACHINES.into(),
                journal,
            });
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USED}");
            ExitCode::from(64)
        }
    }
}

/// Lend a builder while the queue wants one, until told to end.
fn serve(settings: &Settings) {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = ending as *const () as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
    }
    write_machines(&settings.machines, "");
    give_back_left(settings);
    let mut lent: Option<Lent> = None;
    let mut not_before = 0;
    let mut checked_day = local_day();
    while !ENDING.load(Ordering::SeqCst) {
        let now = now();
        let minute = local_minute();
        let wants =
            fs::read_to_string(&settings.queue).is_ok_and(|queue| queue::wants(&queue, now, WAIT));
        lent = match lent.take() {
            None if wants && minute < settings.last_start && now >= not_before => {
                let until_deadline = u64::from(settings.deadline - minute) * 60;
                let lent = lend(settings, now, until_deadline);
                if lent.is_none() {
                    not_before = now + AFTER_FAILING;
                }
                lent
            }
            None => None,
            Some(mut builder) => {
                if wants || in_use(Path::new("/proc"), &builder.host) {
                    builder.used = now;
                }
                let gone = builder.helper.try_wait().ok().flatten().is_some();
                if gone {
                    give_back(settings, builder, Event::Lost);
                    None
                } else if now - builder.used >= IDLE || minute >= settings.deadline {
                    give_back(settings, builder, Event::Stopped);
                    None
                } else {
                    Some(builder)
                }
            }
        };
        // After the deadline each day, nothing should be left: say so if
        // anything is.
        let day = local_day();
        if lent.is_none() && minute >= settings.deadline && day != checked_day {
            give_back_left(settings);
            checked_day = day;
        }
        thread::sleep(TICK);
    }
    if let Some(builder) = lent {
        give_back(settings, builder, Event::Stopped);
    }
}

/// Ask the helper for a builder, and give Nix its line once it's ready.
fn lend(settings: &Settings, now: u64, left: u64) -> Option<Lent> {
    let name = format!("{}{now}", settings.prefix);
    eprintln!("nix-offload: asking for {name}, since the queue holds builds back");
    let mut helper = Command::new(&settings.helper)
        .args(["up", &name, &left.to_string()])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| eprintln!("nix-offload: running the helper: {error}"))
        .ok()?;
    let (said, heard) = mpsc::channel();
    let out = helper.stdout.take()?;
    thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(out).read_line(&mut line);
        let _ = said.send(line);
    });
    let line = heard.recv_timeout(READY).unwrap_or_default();
    let line = line.trim();
    let host = host_of(line);
    match host {
        Some(host) if helper.try_wait().ok().flatten().is_none() => {
            write_machines(&settings.machines, &format!("{line}\n"));
            let _ = journal::note(&settings.journal, now, Event::Started, &name, None);
            eprintln!("nix-offload: {name} is ready; Nix may send it builds");
            Some(Lent {
                name,
                helper,
                host,
                since: now,
                used: now,
            })
        }
        _ => {
            eprintln!("nix-offload: {name} wasn't ready in time, so it's given back");
            end(&mut helper);
            down(settings, &name);
            let _ = journal::note(&settings.journal, self::now(), Event::Failed, &name, None);
            None
        }
    }
}

/// Take the builder from Nix, then give it back, noting how long it ran.
fn give_back(settings: &Settings, mut builder: Lent, event: Event) {
    write_machines(&settings.machines, "");
    end(&mut builder.helper);
    down(settings, &builder.name);
    let now = now();
    let ran = now - builder.since;
    let _ = journal::note(&settings.journal, now, event, &builder.name, Some(ran));
    eprintln!(
        "nix-offload: gave {} back after {} minutes",
        builder.name,
        ran / 60
    );
}

/// Give back any builder left over, as one that outlived a crash would be.
fn give_back_left(settings: &Settings) {
    let Ok(alive) = Command::new(&settings.helper)
        .args(["alive", &settings.prefix])
        .output()
    else {
        eprintln!("nix-offload: couldn't ask the helper what's left");
        return;
    };
    for name in String::from_utf8_lossy(&alive.stdout).lines() {
        let name = name.trim();
        if name.starts_with(&settings.prefix) {
            eprintln!("nix-offload: {name} was left over, so it's given back");
            down(settings, name);
            let _ = journal::note(&settings.journal, now(), Event::Left, name, None);
        }
    }
}

/// Ask the helper to give a builder back, saying so when it can't.
fn down(settings: &Settings, name: &str) {
    let gone = Command::new(&settings.helper)
        .args(["down", name])
        .status()
        .is_ok_and(|status| status.success());
    if !gone {
        eprintln!("nix-offload: the helper couldn't give {name} back; the next check will");
    }
}

/// Tell the helper to end and wait for it.
fn end(helper: &mut Child) {
    if let Ok(pid) = libc::pid_t::try_from(helper.id()) {
        // SAFETY: a signal to our own child, which we still hold.
        unsafe { libc::kill(pid, libc::SIGTERM) };
    }
    let _ = helper.wait();
}

/// Write what Nix and the queue read, whole or not at all.
fn write_machines(path: &Path, text: &str) {
    let writing = path.with_extension("writing");
    let written = fs::write(&writing, text).and_then(|()| fs::rename(&writing, path));
    if let Err(error) = written {
        eprintln!("nix-offload: writing {}: {error}", path.display());
    }
}

/// The ssh host a machines line names, which Nix's build hook reaches it
/// by: `ssh-ng://builder@host x86_64-linux ...` names `host`.
fn host_of(line: &str) -> Option<String> {
    let uri = line.split_whitespace().next()?;
    let (_, rest) = uri.split_once("://")?;
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// Whether an ssh reaching `host` runs: Nix's build hook holds one for
/// each build it sends there.
fn in_use(proc: &Path, host: &str) -> bool {
    let Ok(entries) = fs::read_dir(proc) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let Ok(command) = fs::read(entry.path().join("cmdline")) else {
            return false;
        };
        let mut words = command.split(|byte| *byte == 0);
        let is_ssh = words
            .next()
            .is_some_and(|program| program.rsplit(|byte| *byte == b'/').next() == Some(b"ssh"));
        is_ssh && words.any(|word| word == host.as_bytes())
    })
}

/// Minutes into the day of a time written `HH:MM`.
fn minute_of(written: &str) -> Option<u32> {
    let (hours, minutes) = written.split_once(':')?;
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then_some(hours * 60 + minutes)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The local time now, in the zone TZ names.
fn local() -> libc::tm {
    let now = libc::time_t::try_from(now()).unwrap_or(0);
    // SAFETY: localtime_r writes only the tm it's given.
    unsafe {
        let mut tm = std::mem::zeroed();
        libc::localtime_r(&raw const now, &raw mut tm);
        tm
    }
}

fn local_minute() -> u32 {
    let tm = local();
    u32::try_from(tm.tm_hour * 60 + tm.tm_min).unwrap_or(0)
}

fn local_day() -> i32 {
    let tm = local();
    tm.tm_year * 1000 + tm.tm_yday
}

/// The start of today, local time, in seconds since the epoch.
fn today() -> u64 {
    let tm = local();
    let into = u64::try_from(tm.tm_hour * 3600 + tm.tm_min * 60 + tm.tm_sec).unwrap_or(0);
    now().saturating_sub(into)
}

fn state() -> PathBuf {
    env::var_os("XDG_STATE_HOME").map_or_else(
        || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"),
        PathBuf::from,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lines_host_is_what_ssh_reaches() {
        let line = "ssh-ng://builder@offload x86_64-linux /etc/nix/key 32 1 kvm - AAAA";
        assert_eq!(host_of(line).as_deref(), Some("offload"));
        assert_eq!(
            host_of("ssh-ng://offload x86_64-linux").as_deref(),
            Some("offload")
        );
        assert_eq!(host_of("").as_deref(), None);
        assert_eq!(host_of("offload x86_64-linux").as_deref(), None);
    }

    #[test]
    fn a_builder_is_in_use_while_an_ssh_reaches_it() {
        let proc = tempfile::tempdir().expect("a folder");
        let process = |pid: &str, command: &[&str]| {
            let dir = proc.path().join(pid);
            fs::create_dir(&dir).expect("made");
            fs::write(dir.join("cmdline"), command.join("\0")).expect("written");
        };
        process("1", &["/run/current-system/sw/bin/nix-daemon", "--daemon"]);
        process("2", &["sshd", "offload"]);
        assert!(!in_use(proc.path(), "offload"));
        process(
            "3",
            &[
                "/nix/store/x-openssh/bin/ssh",
                "-x",
                "offload",
                "nix-daemon",
                "--stdio",
            ],
        );
        assert!(in_use(proc.path(), "offload"));
        assert!(!in_use(proc.path(), "elsewhere"));
    }

    #[test]
    fn times_of_day_are_read_as_minutes() {
        assert_eq!(minute_of("20:00"), Some(1200));
        assert_eq!(minute_of("19:30"), Some(1170));
        assert_eq!(minute_of("24:00"), None);
        assert_eq!(minute_of("noon"), None);
    }
}
