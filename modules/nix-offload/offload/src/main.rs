//! Lends the machine a builder elsewhere while its machine queue holds
//! builds back, and gives it back once it's idle.
//!
//! Usage: `nix-offload serve --helper PATH --prefix NAME --rate DOLLARS
//! --first-start HH:MM --last-start HH:MM --deadline HH:MM [--standing
//! PATH]`, or `nix-offload stand --standing PATH` to name standing
//! builders alone and lend none, or `nix-offload report --rate DOLLARS` for
//! today's line.
//!
//! Every 20 seconds it reads the machine queue. Once a build or a VM check
//! has waited three minutes for a slot this machine doesn't have, it asks
//! the helper for a builder, names it `NAME<seconds>`, and writes the Nix
//! machines line the helper gives into /run/nix-offload/machines, which
//! nix-daemon reads at each build (`builders = @` it) and the queue reads
//! to count the builder's room. Once nothing has reached it for fifteen
//! minutes, or the deadline comes, it takes the line out first, so no build
//! is sent to a builder that's going, then gives the builder back. It
//! starts none outside the day's hours, so none is made overnight or to
//! live only minutes.
//!
//! It gives back only builders it asked for: each ask is journalled before
//! the helper runs, and a builder counts as gone only once the helper said
//! so. One not yet seen gone is asked to go again each tick, and no other
//! is lent beside it. Whatever else shares the prefix is someone else's,
//! and is only said.
//!
//! The helper says how, and this when, so this knows nothing of where
//! the builder lives:
//! - `helper up NAME SECONDS` makes it, to live at most SECONDS, the time
//!   left until the deadline, prints its machines line once Nix can use
//!   it, and stays running while it's there, holding the way to it. Before
//!   that line it may print `price DOLLARS`, the hour's price of what it
//!   made, since a fallback machine can cost more than `--rate`. It
//!   ends once the builder goes on its own, and gives the way up when told
//!   to end (SIGTERM, to its whole process group). The seconds are counted
//!   here, in the zone TZ names, so the helper needs no time zones of its
//!   own: a shell without zoneinfo reads a zone's 20:00 as UTC's.
//! - `helper down NAME` gives it back, and succeeds once it's gone,
//!   whether or not it was there.
//! - `helper alive PREFIX` names, a line each, the builders there now.
//!
//! Each ask, start, stop and leftover is a line in the journal under XDG's
//! state folder, `nix-offload/journal.jsonl`, which `report` sums.
//!
//! Builders always there, such as other machines at home, are named in
//! the file `--standing` gives, and join the machines file while they
//! answer, so Nix never waits on one that's away.

mod decide;
mod journal;
mod machines;
mod queue;

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

use decide::{Hours, Tick};
use journal::{Event, Ran};
use machines::Machines;

/// How long a job waits before a builder is asked for.
const WAIT: u64 = 180;
/// How long a builder may take to be ready before it's given up on.
const READY: Duration = Duration::from_mins(15);
/// How often the queue is read.
const TICK: Duration = Duration::from_secs(20);
/// Where nix-daemon and the queue read the builder's line.
const MACHINES: &str = "/run/nix-offload/machines";

const USED: &str = "Usage: nix-offload serve --helper PATH --prefix NAME --rate DOLLARS \
                    --first-start HH:MM --last-start HH:MM --deadline HH:MM \
                    [--standing PATH] | nix-offload stand --standing PATH \
                    | nix-offload report --rate DOLLARS";

/// Set when the service is told to end, so the builder is given back first.
static ENDING: AtomicBool = AtomicBool::new(false);

extern "C" fn ending(_: libc::c_int) {
    ENDING.store(true, Ordering::SeqCst);
}

/// What `serve` was given.
struct Settings {
    helper: PathBuf,
    prefix: String,
    hours: Hours,
    queue: PathBuf,
    standing: Option<PathBuf>,
    journal: PathBuf,
}

/// A builder lent to the machine.
struct Lent {
    name: String,
    helper: Child,
    line: String,
    host: String,
    rate: Option<u64>,
    since: u64,
    used: u64,
}

/// What the helper said once its builder was made: its price, if it gave
/// one, and its machines line.
fn heard_from(helper: impl BufRead) -> (Option<u64>, String) {
    let mut rate = None;
    for line in helper.lines() {
        let Ok(line) = line else { break };
        match line.strip_prefix("price ") {
            Some(price) => rate = journal::price(price.trim()),
            None => return (rate, line),
        }
    }
    (rate, String::new())
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
    if arguments.first().map(String::as_str) == Some("stand") {
        let Some(standing) = given("--standing") else {
            eprintln!("nix-offload: stand needs --standing\n{USED}");
            return ExitCode::from(64);
        };
        stand(&read_standing(Path::new(&standing)));
        return ExitCode::SUCCESS;
    }
    let Some(rate) = given("--rate").as_deref().and_then(journal::price) else {
        eprintln!("nix-offload: --rate takes a price, such as 0.68\n{USED}");
        return ExitCode::from(64);
    };
    let journal = state().join("nix-offload/journal.jsonl");
    match arguments.first().map(String::as_str) {
        Some("report") => {
            let journal = fs::read_to_string(&journal).unwrap_or_default();
            println!("{}", journal::said(&journal::day(&journal, today(), rate)));
            ExitCode::SUCCESS
        }
        Some("serve") => {
            let seconds = |flag| given(flag).as_deref().and_then(second_of);
            let (Some(helper), Some(prefix), Some(first), Some(last), Some(deadline)) = (
                given("--helper"),
                given("--prefix"),
                seconds("--first-start"),
                seconds("--last-start"),
                seconds("--deadline"),
            ) else {
                eprintln!("nix-offload: serve needs each of its settings\n{USED}");
                return ExitCode::from(64);
            };
            if !(first <= last && last <= deadline) {
                eprintln!("nix-offload: the day's hours must run first, last, deadline\n{USED}");
                return ExitCode::from(64);
            }
            let runtime = env::var_os("XDG_RUNTIME_DIR").map_or_else(
                || PathBuf::from("/run/user").join(unsafe { libc::getuid() }.to_string()),
                PathBuf::from,
            );
            // Two would each give back what the other lent.
            let Ok(lock) = fs::File::create(runtime.join("nix-offload.lock")) else {
                eprintln!("nix-offload: can't make its lock in {}", runtime.display());
                return ExitCode::FAILURE;
            };
            if lock.try_lock().is_err() {
                eprintln!("nix-offload: another serve is running");
                return ExitCode::FAILURE;
            }
            serve(&Settings {
                helper: helper.into(),
                prefix,
                hours: Hours {
                    first,
                    last,
                    deadline,
                },
                queue: runtime.join("jobs/queue.json"),
                standing: given("--standing").map(PathBuf::from),
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

/// Hear a stop, so what's lent is given back first.
fn hear_stops() {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = ending as *const () as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
    }
}

fn read_standing(path: &Path) -> Vec<machines::Standing> {
    machines::standing(&fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("nix-offload: reading {}: {error}", path.display());
        String::new()
    }))
}

/// Name the standing builders that answer, lending none, until told to end.
fn stand(standing: &[machines::Standing]) {
    hear_stops();
    let mut machines = Machines::new(MACHINES.into(), standing.to_vec());
    while !ENDING.load(Ordering::SeqCst) {
        machines.show(None);
        thread::sleep(TICK);
    }
}

/// Lend a builder while the queue wants one, until told to end.
fn serve(settings: &Settings) {
    hear_stops();
    let standing = settings
        .standing
        .as_deref()
        .map_or_else(Vec::new, read_standing);
    let mut machines = Machines::new(MACHINES.into(), standing);
    machines.show(None);
    let mut lent: Option<Lent> = None;
    let mut not_before = 0;
    let mut said_on = None;
    while !ENDING.load(Ordering::SeqCst) {
        let tick = Tick {
            now: now(),
            into_day: into_day(),
            wants: fs::read_to_string(&settings.queue)
                .is_ok_and(|queue| queue::wants(&queue, now(), WAIT)),
        };
        let unreturned = give_back_unreturned(settings, lent.as_ref());
        lent = match lent.take() {
            None => match decide::lend(settings.hours, tick, not_before, unreturned) {
                Some(seconds) => {
                    let lent = lend(settings, &mut machines, tick.now, seconds);
                    if lent.is_none() {
                        not_before = decide::next_after(Event::Failed, tick.wants, now());
                    }
                    lent
                }
                None => None,
            },
            Some(mut builder) => {
                if in_use(Path::new("/proc"), &builder.host) {
                    builder.used = tick.now;
                }
                let gone = builder.helper.try_wait().ok().flatten().is_some();
                match decide::give_back(settings.hours, tick, builder.used, gone) {
                    Some(event) => {
                        give_back(settings, &mut machines, builder, event);
                        not_before = decide::next_after(event, tick.wants, now());
                        None
                    }
                    None => Some(builder),
                }
            }
        };
        // Once a day, after the deadline, say what's there that isn't this
        // one's to give back.
        let day = local_day();
        if tick.into_day >= settings.hours.deadline && said_on != Some(day) {
            say_others(settings);
            said_on = Some(day);
        }
        machines.show(lent.as_ref().map(|builder| builder.line.as_str()));
        thread::sleep(TICK);
    }
    if let Some(builder) = lent {
        give_back(settings, &mut machines, builder, Event::Stopped);
    }
}

/// Ask the helper for a builder, and give Nix its line once it's ready.
fn lend(settings: &Settings, machines: &mut Machines, now: u64, left: u64) -> Option<Lent> {
    let name = format!("{}{now}", settings.prefix);
    eprintln!("nix-offload: asking for {name}, since the queue holds builds back");
    if let Err(error) = journal::note(&settings.journal, now, Event::Asked, &name, None) {
        // Unjournalled, it couldn't be known as this one's to give back.
        eprintln!("nix-offload: journalling the ask: {error}; none is asked for");
        return None;
    }
    let mut helper = Command::new(&settings.helper)
        .args(["up", &name, &left.to_string()])
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| eprintln!("nix-offload: running the helper: {error}"))
        .ok()?;
    let (said, heard) = mpsc::channel();
    let out = helper.stdout.take()?;
    thread::spawn(move || {
        let _ = said.send(heard_from(BufReader::new(out)));
    });
    // In short waits, so a stop is heard while the builder is being made.
    let asked = Instant::now();
    let (mut rate, mut line) = (None, String::new());
    while asked.elapsed() < READY && !ENDING.load(Ordering::SeqCst) {
        match heard.recv_timeout(Duration::from_secs(1)) {
            Ok(said) => {
                (rate, line) = said;
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let line = line.trim();
    match host_of(line) {
        Some(host) if helper.try_wait().ok().flatten().is_none() => {
            machines.show(Some(line));
            let _ = journal::note(&settings.journal, self::now(), Event::Started, &name, None);
            eprintln!("nix-offload: {name} is ready; Nix may send it builds");
            Some(Lent {
                name,
                helper,
                line: line.to_owned(),
                host,
                rate,
                since: now,
                used: self::now(),
            })
        }
        _ => {
            eprintln!("nix-offload: {name} wasn't ready, so it's given back");
            end(&mut helper);
            if down(settings, &name) {
                let ran = Ran {
                    seconds: self::now() - now,
                    rate,
                };
                let _ = journal::note(
                    &settings.journal,
                    self::now(),
                    Event::Failed,
                    &name,
                    Some(ran),
                );
            }
            None
        }
    }
}

/// Take the builder from Nix, then give it back, noting how long it ran.
fn give_back(settings: &Settings, machines: &mut Machines, mut builder: Lent, event: Event) {
    machines.show(None);
    end(&mut builder.helper);
    if down(settings, &builder.name) {
        let now = now();
        let ran = Ran {
            seconds: now - builder.since,
            rate: builder.rate,
        };
        let _ = journal::note(&settings.journal, now, event, &builder.name, Some(ran));
        eprintln!(
            "nix-offload: gave {} back after {} minutes",
            builder.name,
            ran.seconds / 60
        );
    }
}

/// Ask again for every builder this asked for and hasn't seen gone, but
/// the one lent, and say how many are still not gone.
fn give_back_unreturned(settings: &Settings, lent: Option<&Lent>) -> usize {
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let mut still = 0;
    for name in journal::unreturned(&journal) {
        if lent.is_some_and(|builder| builder.name == name) {
            continue;
        }
        eprintln!("nix-offload: {name} isn't yet seen gone, so it's given back");
        if down(settings, &name) {
            let _ = journal::note(&settings.journal, now(), Event::Left, &name, None);
        } else {
            still += 1;
        }
    }
    still
}

/// Say any builder under the prefix that isn't this one's, leaving it be.
fn say_others(settings: &Settings) {
    let Ok(alive) = Command::new(&settings.helper)
        .args(["alive", &settings.prefix])
        .output()
    else {
        eprintln!("nix-offload: couldn't ask the helper what's there");
        return;
    };
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let ours = journal::unreturned(&journal);
    for name in String::from_utf8_lossy(&alive.stdout)
        .lines()
        .map(str::trim)
    {
        if !name.is_empty() && !ours.iter().any(|known| known == name) {
            eprintln!("nix-offload: {name} is there after the deadline, and isn't this one's");
        }
    }
}

/// Ask the helper to give a builder back: whether it's gone.
fn down(settings: &Settings, name: &str) -> bool {
    let gone = Command::new(&settings.helper)
        .args(["down", name])
        .status()
        .is_ok_and(|status| status.success());
    if !gone {
        eprintln!("nix-offload: the helper couldn't give {name} back; it's asked again next tick");
    }
    gone
}

/// Tell the helper and everything it started to end, and wait for it.
fn end(helper: &mut Child) {
    if let Ok(pid) = libc::pid_t::try_from(helper.id()) {
        // SAFETY: a signal to our own child's process group, which it leads.
        unsafe { libc::kill(-pid, libc::SIGTERM) };
    }
    let _ = helper.wait();
}

/// The ssh host a machines line names, which Nix's build hook reaches it
/// by: `ssh-ng://builder@host x86_64-linux ...` names `host`.
fn host_of(line: &str) -> Option<String> {
    let uri = line.split_whitespace().next()?;
    let (_, rest) = uri.split_once("://")?;
    let rest = rest.split(['?', '/']).next().unwrap_or(rest);
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// Whether an ssh reaching `host` runs: Nix's build hook holds one for
/// each build it sends there, naming it `host` or `user@host`.
fn in_use(proc: &Path, host: &str) -> bool {
    let Ok(entries) = fs::read_dir(proc) else {
        return false;
    };
    let with_user = format!("@{host}");
    entries.flatten().any(|entry| {
        let Ok(command) = fs::read(entry.path().join("cmdline")) else {
            return false;
        };
        let mut words = command.split(|byte| *byte == 0);
        let is_ssh = words
            .next()
            .is_some_and(|program| program.rsplit(|byte| *byte == b'/').next() == Some(b"ssh"));
        is_ssh && words.any(|word| word == host.as_bytes() || word.ends_with(with_user.as_bytes()))
    })
}

/// Seconds into the day of a time written `HH:MM`.
fn second_of(written: &str) -> Option<u32> {
    let (hours, minutes) = written.split_once(':')?;
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then_some(hours * 3600 + minutes * 60)
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

fn into_day() -> u32 {
    let tm = local();
    u32::try_from(tm.tm_hour * 3600 + tm.tm_min * 60 + tm.tm_sec).unwrap_or(0)
}

fn local_day() -> i32 {
    let tm = local();
    tm.tm_year * 1000 + tm.tm_yday
}

/// The start of today, local time, in seconds since the epoch.
fn today() -> u64 {
    now().saturating_sub(u64::from(into_day()))
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
        assert_eq!(
            host_of("ssh-ng://offload?remote-store=x x86_64-linux").as_deref(),
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
                "builder@offload",
                "nix-daemon",
                "--stdio",
            ],
        );
        assert!(in_use(proc.path(), "offload"));
        assert!(!in_use(proc.path(), "elsewhere"));
    }

    #[test]
    fn times_of_day_are_read_as_seconds() {
        assert_eq!(second_of("20:00"), Some(72_000));
        assert_eq!(second_of("19:30"), Some(70_200));
        assert_eq!(second_of("24:00"), None);
        assert_eq!(second_of("noon"), None);
    }

    #[test]
    fn the_helper_may_price_its_builder_before_its_line() {
        let line = "ssh-ng://nix-offload x86_64-linux /key 5 1 kvm - aG9zdA==";
        assert_eq!(
            heard_from(format!("price 0.89\n{line}\nlater\n").as_bytes()),
            (Some(890), line.to_owned())
        );
        assert_eq!(
            heard_from(format!("{line}\n").as_bytes()),
            (None, line.to_owned())
        );
        assert_eq!(
            heard_from("price 0.89\n".as_bytes()),
            (Some(890), String::new())
        );
    }
}
