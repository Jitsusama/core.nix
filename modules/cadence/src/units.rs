//! The watch on the account's own units: telling of one that is broken
//! within a tick of its breaking, rather than when someone happens to
//! look. On 13 November slack-unreads restarted 44 times from 20:47, and
//! the Google daemon read signed out from 20:44, and nobody knew until
//! 22:38. Each tick a unit restarting in a loop, a unit failed, and a
//! daemon whose socket says a source is signed out, unreachable or
//! refused, or that doesn't answer at all, is told to the orchestrator and
//! the unit's owner once for each spell, with since when and its last
//! error line. `read` reads the journal, systemd and the sockets; the rest
//! is a decision over what was read, and main.rs sends.
//!
//! A process running as the account's user can write journal lines that
//! read as a unit's retries and so silence its loop, which it could do as
//! well by stopping cadence.
//!
//! Long because each thing told (a loop, a failure, a daemon's source) is
//! decided here beside the night's journal that shows it, replayed tick by
//! tick in the tests.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::holders::a_run;

pub mod journal;
mod shown;

pub use shown::leave_runs;
use shown::{SHOW, shown_from};

/// How far back restarts are counted.
pub const WITHIN: i64 = 15 * 60;

/// How many restarts within `WITHIN` make a loop. One restart is a unit
/// mending itself, as merge-net's at 20:13 on 13 November did.
pub const LOOPING: usize = 3;

/// How many of a unit's quiet retries within `WITHIN` are left out of its
/// loop count. Past them they all count, since no start limit fails a
/// build lender, so nothing else would tell of one reloaded over and over.
pub const QUIET_RETRIES: usize = LOOPING * 2;

/// How long a daemon not answering, or reading a source unreachable or
/// refused, must hold before it is told: a daemon restarting, or the
/// network coming back from sleep, mends itself within a tick. Signed out
/// never mends by itself, so it is told at once.
pub const HELD: i64 = 4 * 60;

/// The words a daemon's source says when it is broken, as daemon-serve
/// says them: `answering`, `starting`, `waiting` and the rest are well.
pub const BROKEN: [&str; 3] = ["signed-out", "unreachable", "refused"];

/// systemd's journal message for a restart it scheduled itself, after a
/// unit's process died, as opposed to one asked for by a switch or a person.
pub const RESTART_SCHEDULED: &str = "5eb03494b6584870a536b337290809b3";

/// systemd's journal message for a unit's process ending, with how it
/// ended (`EXIT_CODE`, "exited" when it exited itself) and its status.
pub const PROCESS_EXITED: &str = "98e322203f7a4ed290d09fe03c09fe15";

/// How long before the restarts counted their exits are read, so the exit
/// before the first restart counted is read too: well past the longest
/// wait before a restart of a unit naming retry statuses, repos-refresh's
/// 30 seconds, and the account's other units' 60.
const EXITS_BEFORE: i64 = 5 * 60;

/// How much longer than its unit's restart delay an exit may come before
/// the restart it pairs with: systemd waits the delay once the unit has
/// stopped, and the stop itself takes a moment.
const STOPPING: i64 = 30;

/// One restart systemd scheduled, of `unit`, at a moment in seconds; the
/// status its process exited with just before, when it exited itself
/// rather than being killed; and whether an exit of the invocation it
/// restarts was read before it, since systemd journals a failed exit or a
/// kill but not a clean one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restart {
    pub unit: String,
    pub at: i64,
    pub after: Option<u32>,
    pub exit_read: bool,
}

/// An exit of a unit's process not yet paired with its restart: the status
/// it exited with, when it exited itself; when; and the invocation of the
/// unit it ended, when its journal line names one.
struct Exit {
    status: Option<u32>,
    at: i64,
    invocation: Option<String>,
}

/// A unit in the failed state, since when, when systemd says, and whether
/// systemd-run started it, a run whose starter reads its end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub unit: String,
    pub since: Option<i64>,
    pub run: bool,
}

/// One of a daemon's sources, as its socket's health says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub word: String,
    pub reason: String,
}

/// What a daemon's socket said when asked how it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// It answered, with each of its sources.
    Answered(Vec<Source>),
    /// It couldn't be reached or said nothing, and why.
    Silent(String),
}

/// A daemon, by its socket's name, and what its socket said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Daemon {
    pub name: String,
    pub answer: Answer,
}

/// What one tick read. `retries` holds, for each unit restarted after an
/// exit with a status or with no exit read before, the statuses its
/// `RestartForceExitStatus` names. The unit retries on these by design, as
/// repos-refresh does on 69 while the network is away, so a restart after
/// one is no loop. If those retries never end, its start limit fails it,
/// and it is told as failed.
///
/// `quiet` holds the units that restart only on failure and whose retry
/// statuses are all in their `SuccessExitStatus` too, as the build
/// lenders' 75 and 100 are. systemd journals no clean exit, though it
/// journals a failure or a kill. So a restart of such a unit with no exit
/// read before it followed one of its retries, and is no loop either. The
/// lenders have no start limit to fail them, though, so past
/// `QUIET_RETRIES` such restarts of one unit within `WITHIN`, they count
/// again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    pub restarts: Vec<Restart>,
    pub retries: BTreeMap<String, Vec<u32>>,
    pub quiet: std::collections::BTreeSet<String>,
    pub failed: Vec<Failed>,
    pub daemons: Vec<Daemon>,
}

/// A spell of one thing broken: when it began and whether it was told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spell {
    pub since: i64,
    pub told: bool,
}

/// The spells under way, by key, so each is told once.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Told {
    #[serde(default)]
    pub spells: BTreeMap<String, Spell>,
}

/// A line to tell, of `unit`, keyed by its spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tell {
    pub key: String,
    pub unit: String,
    pub text: String,
}

/// Reads a daemon's answer to `{"ask":"health"}`: each source by its
/// state and reason, or by its word alone, as older daemons say them.
#[must_use]
pub fn answer(line: &str) -> Answer {
    let Ok(said) = serde_json::from_str::<serde_json::Value>(line) else {
        return Answer::Silent(format!("it said something that isn't JSON: {line:.80}"));
    };
    let mut sources = Vec::new();
    if let Some(all) = said.get("sources").and_then(serde_json::Value::as_object) {
        for (name, source) in all {
            let state = source.get("state").unwrap_or(source);
            sources.push(Source {
                name: name.clone(),
                word: state.as_str().unwrap_or_default().to_owned(),
                reason: source
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
    }
    let state = said.get("state").and_then(serde_json::Value::as_str);
    if state.is_some_and(|s| s != "answering") {
        sources.push(Source {
            name: "itself".to_owned(),
            word: state.unwrap_or_default().to_owned(),
            reason: String::new(),
        });
    }
    Answer::Answered(sources)
}

/// Reads the restarts in journalctl's JSON lines, one object a line, in
/// the journal's order, each with its unit's exit before it. A line of a
/// process's exit is no restart; any other is one.
///
/// An exit pairs only with a restart of the invocation it ended, so one
/// left over from an invocation before isn't read as the next one's. When
/// both lines name their invocation, they must name the same one. When
/// either doesn't, the exit must come no longer before the restart than
/// the unit's restart delay, which `delay` gives in seconds, and
/// `STOPPING`; with no delay known, no longer than `EXITS_BEFORE`.
#[must_use]
pub fn restarts(journal: &str, mut delay: impl FnMut(&str) -> Option<i64>) -> Vec<Restart> {
    let mut unpaired: BTreeMap<String, Exit> = BTreeMap::new();
    let mut restarts = Vec::new();
    for entry in journal
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    {
        let field = |name: &str| entry.get(name).and_then(serde_json::Value::as_str);
        let Some(unit) = field("USER_UNIT").or_else(|| field("UNIT")) else {
            continue;
        };
        let Some(micros) = field("__REALTIME_TIMESTAMP").and_then(|t| t.parse::<i64>().ok()) else {
            continue;
        };
        let at = micros / 1_000_000;
        let invocation = field("USER_INVOCATION_ID").or(field("INVOCATION_ID"));
        if field("MESSAGE_ID") == Some(PROCESS_EXITED) {
            let status = field("EXIT_STATUS").and_then(|s| s.parse().ok());
            let own = field("EXIT_CODE") == Some("exited");
            let exit = Exit {
                status: status.filter(|_| own),
                at,
                invocation: invocation.map(str::to_owned),
            };
            unpaired.insert(unit.to_owned(), exit);
            continue;
        }
        let pending = unpaired.remove(unit);
        // The same invocation pairs whenever it ended; any other exit pairs
        // only within the unit's delay, so a restart line naming another
        // invocation than its exit's, as a systemd that wrote the next
        // one's would, falls back to the time rather than dropping it.
        let exit = pending.filter(|exit| {
            let same = matches!(
                (exit.invocation.as_deref(), invocation),
                (Some(ended), Some(restarted)) if ended == restarted
            );
            same || at - exit.at <= delay(unit).map_or(EXITS_BEFORE, |d| d + STOPPING)
        });
        restarts.push(Restart {
            unit: unit.to_owned(),
            at,
            after: exit.as_ref().and_then(|exit| exit.status),
            exit_read: exit.is_some(),
        });
    }
    restarts
}

/// The whole seconds in a span as `systemctl show` writes one, such as
/// `100ms`, `1.5s` or `1min 30s`, rounded up: none for `infinity`, an
/// empty value or a unit it doesn't know.
fn seconds(span: &str) -> Option<i64> {
    if span.trim().is_empty() {
        return None;
    }
    let mut micros: u64 = 0;
    for word in span.split_whitespace() {
        let digits = word
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(word.len());
        let (number, unit) = word.split_at(digits);
        let per: u64 = match unit {
            "" | "us" => 1,
            "ms" => 1_000,
            "s" => 1_000_000,
            "min" => 60_000_000,
            "h" => 3_600_000_000,
            "d" => 86_400_000_000,
            "w" => 604_800_000_000,
            _ => return None,
        };
        let (whole, part) = number.split_once('.').unwrap_or((number, ""));
        // A part of a unit is taken as a whole one, as the span is rounded
        // up anyway.
        let whole = whole
            .parse::<u64>()
            .ok()?
            .checked_add(u64::from(part.bytes().any(|b| b != b'0')))?;
        micros = micros.checked_add(whole.checked_mul(per)?)?;
    }
    i64::try_from(micros.div_ceil(1_000_000)).ok()
}

/// The value `key` has in `systemctl show`'s `Key=value` lines, or nothing.
fn value<'a>(shown: &'a str, key: &str) -> &'a str {
    shown
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_default()
}

/// The exit statuses `systemctl show -p RestartForceExitStatus --value`
/// names, the numbers alone: a signal it names is a kill, which is no
/// exit of the unit's own.
#[must_use]
pub fn retry_statuses(shown: &str) -> Vec<u32> {
    shown
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect()
}

/// A unit's retry statuses, and whether its retries are quiet, from
/// `systemctl show -p Restart -p RestartForceExitStatus -p
/// SuccessExitStatus`'s `Key=value` lines: quiet when it restarts only
/// on failure and every retry status is a success status too.
#[must_use]
pub fn retrying(shown: &str) -> (Vec<u32>, bool) {
    let statuses = retry_statuses(value(shown, "RestartForceExitStatus"));
    let succeeding = retry_statuses(value(shown, "SuccessExitStatus"));
    let quiet = value(shown, "Restart") == "on-failure"
        && !statuses.is_empty()
        && statuses.iter().all(|s| succeeding.contains(s));
    (statuses, quiet)
}

/// What the journal's lines up to `at` say of the restarts in the `WITHIN`
/// before it, with `show` giving what `systemctl show` says of a unit. The
/// lines go back `EXITS_BEFORE` further, so the exit before the first
/// restart counted is read too, and the restarts before the `WITHIN` are
/// then dropped. A unit is asked about once at most: for its restart delay
/// when an exit and a restart of it can't be paired by their invocation,
/// and for its retries when a restart of it followed an exit with a status
/// or had no exit read before it.
#[must_use]
pub fn paired(journal: &str, at: i64, show: impl Fn(&str) -> String) -> Read {
    let mut said: BTreeMap<String, String> = BTreeMap::new();
    let mut shown_of = |unit: &str| {
        said.entry(unit.to_owned())
            .or_insert_with(|| show(unit))
            .clone()
    };
    let delay = |unit: &str| seconds(value(&shown_of(unit), "RestartUSec"));
    let mut read = Read {
        restarts: restarts(journal, delay),
        ..Read::default()
    };
    read.restarts.retain(|r| r.at > at - WITHIN);
    let retried: std::collections::BTreeSet<String> = read
        .restarts
        .iter()
        .filter(|r| (r.after.is_some() || !r.exit_read) && !a_run(&r.unit))
        .map(|r| r.unit.clone())
        .collect();
    for unit in retried {
        let (statuses, quiet) = retrying(&shown_of(unit.as_str()));
        if quiet {
            read.quiet.insert(unit.clone());
        }
        read.retries.insert(unit, statuses);
    }
    read
}

/// The reader of the restarts a tick keeps: the journal's lines of a
/// restart scheduled or a process exited, looked back on as far as
/// `read_from` reads them.
pub const RESTARTS: journal::Want = journal::Want {
    name: "restarts",
    journal: journal::Journal::User,
    keep: WITHIN + EXITS_BEFORE,
    fill: &[
        "MESSAGE_ID=5eb03494b6584870a536b337290809b3",
        "MESSAGE_ID=98e322203f7a4ed290d09fe03c09fe15",
    ],
    takes: a_restart,
};

/// Whether a journal line is a restart scheduled or a process exited.
fn a_restart(fields: &serde_json::Map<String, serde_json::Value>) -> bool {
    journal::is(fields, "MESSAGE_ID", RESTART_SCHEDULED)
        || journal::is(fields, "MESSAGE_ID", PROCESS_EXITED)
}

/// Reads what `cadence units` asks of a moment: the restarts systemd
/// scheduled in the account's units in the `WITHIN` before `at`, read by
/// their match, as a look by hand is too rare for its cost to count, and
/// the rest as [`read_from`] reads it.
#[must_use]
pub fn read(at: i64, now: bool, runtime: Option<&Path>) -> Read {
    let journal = run(
        "journalctl",
        &[
            "--user",
            "-o",
            "json",
            "--no-pager",
            "--since",
            &format!("@{}", at - WITHIN - EXITS_BEFORE),
            "--until",
            &format!("@{at}"),
            &format!("MESSAGE_ID={RESTART_SCHEDULED}"),
            &format!("MESSAGE_ID={PROCESS_EXITED}"),
        ],
    );
    read_from(&journal.unwrap_or_default(), at, now, runtime)
}

/// Reads what a tick needs, given the `journal`'s JSON lines of restarts
/// and exits from `EXITS_BEFORE` before the `WITHIN` before `at` on: the
/// restarts in that `WITHIN`, and, when `now`, the units failed and each
/// daemon's socket under `runtime`, which can't be read as they were in
/// the past.
#[must_use]
pub fn read_from(journal: &str, at: i64, now: bool, runtime: Option<&Path>) -> Read {
    let systemctl_show = |unit: &str| {
        let asked = [
            "--user",
            "show",
            "-p",
            "Restart",
            "-p",
            "RestartUSec",
            "-p",
            "RestartForceExitStatus",
            "-p",
            "SuccessExitStatus",
            "--",
            unit,
        ];
        run("systemctl", &asked).unwrap_or_default()
    };
    let mut read = paired(journal, at, systemctl_show);
    if !now {
        return read;
    }
    let listed = run(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--state=failed",
            "--plain",
            "--no-legend",
            "--full",
        ],
    );
    read.failed = listed
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(|unit| {
            let shown = run("systemctl", &[&SHOW[..], &[unit]].concat()).unwrap_or_default();
            let (since, run) = shown_from(unit, &shown);
            Failed {
                unit: unit.to_owned(),
                since,
                run,
            }
        })
        .collect();
    let sockets = runtime.and_then(|r| r.join("daemons").read_dir().ok());
    for entry in sockets.into_iter().flatten().flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str());
        if let Some(name) = name.and_then(|n| n.strip_suffix(".sock")) {
            read.daemons.push(Daemon {
                name: name.to_owned(),
                answer: ask(&path),
            });
        }
    }
    read.daemons.sort_by(|a, b| a.name.cmp(&b.name));
    read
}

/// The last line of `unit`'s journal at warning or worse: its own, or
/// systemd's about it, such as a watchdog's kill.
#[must_use]
pub fn last_error(unit: &str) -> Option<String> {
    let args = [
        "--user",
        "-u",
        unit,
        "-p",
        "warning",
        "-n",
        "1",
        "-o",
        "cat",
        "--no-pager",
    ];
    let said = run("journalctl", &args).ok()?;
    let line = said.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.chars().take(200).collect())
}

/// Asks a daemon's socket how it stands, as daemon-serve answers any
/// client, waiting three seconds at most.
fn ask(socket: &Path) -> Answer {
    let asked = (|| -> io::Result<String> {
        let mut stream = UnixStream::connect(socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(b"{\"id\":1,\"ask\":\"health\"}\n")?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line)?;
        Ok(line)
    })();
    match asked {
        Ok(line) if !line.trim().is_empty() => answer(&line),
        Ok(_) => Answer::Silent("it hung up without answering".to_owned()),
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            Answer::Silent("no answer within three seconds".to_owned())
        }
        Err(err) => Answer::Silent(err.to_string()),
    }
}

fn run(program: &str, args: &[&str]) -> io::Result<String> {
    let out = Command::new(program).args(args).output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(io::Error::other(format!("{program} failed")))
    }
}

/// One thing broken now: its key, its unit, when it began as far as this
/// tick knows, how long it must hold before it's told, and what it is.
struct Broken {
    key: String,
    unit: String,
    since: i64,
    hold: i64,
    what: String,
    /// The line to quote when the unit's journal has none.
    reason: Option<String>,
}

/// What to tell at `now`, minding the spells told already: each spell is
/// told once, and one no longer broken ends, so the next is told again.
/// `last_error` gives a unit's last error line from its journal; `clock`
/// writes a moment for a person.
pub fn due(
    read: &Read,
    told: &mut Told,
    now: i64,
    last_error: impl Fn(&str) -> Option<String>,
    clock: impl Fn(i64) -> String,
) -> Vec<Tell> {
    let broken = broken(read, told, now, &clock);
    told.spells
        .retain(|key, _| broken.iter().any(|b| &b.key == key));
    let mut tells = Vec::new();
    for b in broken {
        let spell = told.spells.entry(b.key.clone()).or_insert(Spell {
            since: b.since,
            told: false,
        });
        if spell.told || now - spell.since < b.hold {
            continue;
        }
        spell.told = true;
        let error = b
            .reason
            .filter(|r| !r.is_empty())
            .or_else(|| last_error(&b.unit))
            .map_or_else(
                || " Its journal holds no error line.".to_owned(),
                |line| format!(" Its last error: \"{line}\"."),
            );
        tells.push(Tell {
            text: format!(
                "{what}, since {since}.{error} Told once by cadence's unit watch; \
                 a new spell is told again.",
                what = b.what,
                since = clock(spell.since),
            ),
            key: b.key,
            unit: b.unit,
        });
    }
    tells
}

/// How many of the restarts in the `WITHIN` before `now` were left out of
/// the loop count as retries, by unit, as `cadence units` says them, such
/// as `none left out as retries` or `5 of them left out as retries
/// (nix-offload.service 3, repos-refresh.service 2)`.
#[must_use]
pub fn retries_said(read: &Read, now: i64) -> String {
    let Counted { left, .. } = counted(read, now, &BTreeMap::new());
    if left.is_empty() {
        return "none left out as retries".to_owned();
    }
    let by_unit: Vec<String> = left
        .iter()
        .map(|(unit, count)| format!("{unit} {count}"))
        .collect();
    format!(
        "{} of them left out as retries ({})",
        left.values().sum::<usize>(),
        by_unit.join(", ")
    )
}

/// The restarts in the `WITHIN` before `now` that count toward each unit's
/// loop, by unit, and how many of each unit's were left out as retries. A
/// restart after an exit with a status its unit names for retries is left
/// out. So is a quiet unit's with no exit read before it, unless more than
/// `QUIET_RETRIES` of those fall in the `WITHIN`, when they all count, or
/// its loop is among the `held` spells already told: a lender retrying
/// near the cap would otherwise cross it again and again, each crossing
/// told as a new loop.
fn counted<'a>(read: &'a Read, now: i64, held: &BTreeMap<String, Spell>) -> Counted<'a> {
    let mut looping: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    let mut left: BTreeMap<&str, usize> = BTreeMap::new();
    let mut quiet: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for r in read
        .restarts
        .iter()
        .filter(|r| r.at > now - WITHIN && r.at <= now && !a_run(&r.unit))
    {
        let listed = r.after.is_some_and(|status| {
            read.retries
                .get(&r.unit)
                .is_some_and(|s| s.contains(&status))
        });
        if listed {
            *left.entry(&r.unit).or_default() += 1;
        } else if !r.exit_read && read.quiet.contains(&r.unit) {
            quiet.entry(&r.unit).or_default().push(r.at);
        } else {
            looping.entry(&r.unit).or_default().push(r.at);
        }
    }
    for (unit, times) in quiet {
        if times.len() > QUIET_RETRIES || held.contains_key(&format!("loop:{unit}")) {
            looping.entry(unit).or_default().extend(times);
        } else {
            *left.entry(unit).or_default() += times.len();
        }
    }
    Counted { looping, left }
}

/// A window's restarts by unit: the moments of those counted toward a
/// loop, and how many were left out as retries.
struct Counted<'a> {
    looping: BTreeMap<&'a str, Vec<i64>>,
    left: BTreeMap<&'a str, usize>,
}

/// Everything broken now, each with when it began.
fn broken(read: &Read, told: &Told, now: i64, clock: &impl Fn(i64) -> String) -> Vec<Broken> {
    let mut broken = Vec::new();
    let Counted { looping, .. } = counted(read, now, &told.spells);
    for (unit, mut times) in looping {
        times.sort_unstable();
        let key = format!("loop:{unit}");
        // A loop under way goes on while it restarts at all, so one
        // slowing to two restarts in fifteen minutes isn't told twice.
        if times.len() < LOOPING && !told.spells.contains_key(&key) {
            continue;
        }
        let (first, last) = (times[0], times[times.len() - 1]);
        broken.push(Broken {
            key,
            unit: unit.to_owned(),
            since: first,
            hold: 0,
            what: format!(
                "{unit} is restarting in a loop: {n} restarts in the last 15 minutes, \
                 the last at {last}",
                n = times.len(),
                last = clock(last),
            ),
            reason: None,
        });
    }
    for f in read.failed.iter().filter(|f| !a_run(&f.unit)) {
        broken.push(Broken {
            key: format!("failed:{}", f.unit),
            unit: f.unit.clone(),
            since: f.since.unwrap_or(now),
            hold: 0,
            what: format!("{} has failed and stays down", f.unit),
            reason: None,
        });
    }
    for d in &read.daemons {
        let unit = format!("{}.service", d.name);
        let (what, hold, reason) = match &d.answer {
            Answer::Silent(why) => (
                format!("The {} daemon's socket doesn't answer ({why})", d.name),
                HELD,
                None,
            ),
            Answer::Answered(sources) => {
                let bad: Vec<&Source> = sources
                    .iter()
                    .filter(|s| BROKEN.contains(&s.word.as_str()))
                    .collect();
                if bad.is_empty() {
                    continue;
                }
                let said: Vec<String> = bad
                    .iter()
                    .map(|s| format!("{} {}", s.name, s.word))
                    .collect();
                let hold = if bad.iter().any(|s| s.word == "signed-out") {
                    0
                } else {
                    HELD
                };
                (
                    format!("The {} daemon reads {}", d.name, said.join(", ")),
                    hold,
                    bad.iter().map(|s| s.reason.clone()).find(|r| !r.is_empty()),
                )
            }
        };
        broken.push(Broken {
            key: format!("daemon:{}", d.name),
            unit,
            since: now,
            hold,
            what,
            reason,
        });
    }
    broken
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(at: i64) -> String {
        format!("t{at}")
    }

    /// slack-unreads' restarts on 13 November from 20:47:42, as its
    /// journal holds them: its watchdog killed it every 3.5 minutes.
    const SLACK: [i64; 8] = [
        1_794_620_862,
        1_794_621_072,
        1_794_621_283,
        1_794_621_494,
        1_794_621_704,
        1_794_621_914,
        1_794_622_125,
        1_794_622_335,
    ];

    /// The tick before slack-unreads' first restart, 20:45:00.
    const TICK: i64 = 1_794_620_700;

    fn of(unit: &str, times: &[i64]) -> Vec<Restart> {
        times
            .iter()
            .map(|&at| Restart {
                unit: unit.to_owned(),
                at,
                after: None,
                exit_read: false,
            })
            .collect()
    }

    /// slack-unreads' journal's last error that night; other units' hold none.
    fn watchdog(unit: &str) -> Option<String> {
        (unit == "slack-unreads.service")
            .then(|| "slack-unreads.service: Failed with result 'watchdog'.".to_owned())
    }

    #[test]
    fn tonights_slack_loop_is_told_at_the_tick_after_its_third_restart() {
        let mut told = Told::default();
        let mut heard = Vec::new();
        // Each five-minute tick from 20:45 to 21:30, over what the
        // journal held by then, with merge-net's one restart of 20:13.
        for tick in (0..10).map(|n| TICK + n * 300) {
            let mut all = of("slack-unreads.service", &SLACK);
            all.extend(of("merge-net.service", &[1_794_620_013]));
            all.retain(|r| r.at <= tick);
            let read = Read {
                restarts: all,
                ..Read::default()
            };
            for tell in due(&read, &mut told, tick, watchdog, clock) {
                heard.push((tick, tell));
            }
        }
        assert_eq!(heard.len(), 1, "told once for the spell: {heard:?}");
        let (at, tell) = &heard[0];
        assert_eq!(
            *at,
            TICK + 600,
            "told at 20:55, the tick after the third restart"
        );
        assert_eq!(tell.unit, "slack-unreads.service", "{tell:?}");
        assert_eq!(
            tell.text,
            "slack-unreads.service is restarting in a loop: 3 restarts in the last 15 \
             minutes, the last at t1794621283, since t1794620862. Its last error: \
             \"slack-unreads.service: Failed with result 'watchdog'.\". Told once by \
             cadence's unit watch; a new spell is told again.",
            "the unit, since when and its last error line"
        );
    }

    #[test]
    fn a_loop_that_stops_ends_its_spell_and_a_new_one_is_told_again() {
        let mut told = Told::default();
        let first = Read {
            restarts: of("u.service", &[100, 200, 300]),
            ..Read::default()
        };
        assert_eq!(
            due(&first, &mut told, 400, watchdog, clock).len(),
            1,
            "the first loop"
        );
        assert!(
            due(&Read::default(), &mut told, 400 + WITHIN, watchdog, clock).is_empty(),
            "quiet for fifteen minutes"
        );
        assert!(told.spells.is_empty(), "the spell ended: {:?}", told.spells);
        let again = Read {
            restarts: of("u.service", &[2000, 2100, 2200]),
            ..Read::default()
        };
        assert_eq!(
            due(&again, &mut told, 2300, watchdog, clock).len(),
            1,
            "a new loop"
        );
    }

    #[test]
    fn a_loop_slowing_to_fewer_restarts_isnt_told_twice() {
        let mut told = Told::default();
        let read = Read {
            restarts: of("u.service", &[100, 200, 300, 1100]),
            ..Read::default()
        };
        assert_eq!(
            due(&read, &mut told, 400, watchdog, clock).len(),
            1,
            "the loop"
        );
        assert!(
            due(&read, &mut told, 1150, watchdog, clock).is_empty(),
            "two restarts left in the window keep the spell"
        );
    }

    #[test]
    fn a_failed_unit_is_told_once_from_when_it_failed() {
        let mut told = Told::default();
        let read = Read {
            failed: vec![Failed {
                unit: "notices.service".into(),
                since: Some(50),
                run: false,
            }],
            ..Read::default()
        };
        let tells = due(&read, &mut told, 300, |_| None, clock);
        assert_eq!(tells.len(), 1, "{tells:?}");
        assert_eq!(
            tells[0].text,
            "notices.service has failed and stays down, since t50. Its journal holds no \
             error line. Told once by cadence's unit watch; a new spell is told again.",
            "the unit and since when"
        );
        assert!(
            due(&read, &mut told, 600, |_| None, clock).is_empty(),
            "once a spell"
        );
    }

    #[test]
    fn a_run_the_net_or_pi_started_is_never_told() {
        let mut told = Told::default();
        let failed = |unit: &str| Failed {
            unit: unit.into(),
            since: Some(50),
            run: false,
        };
        // Two kinds of unit failed on 17 November that no one held, each
        // a run its starter already reads the end of, beside a service.
        let read = Read {
            failed: vec![
                failed(
                    "merge-net-rerun-worktree-github-Jitsusama-core-nix-jitsusama-repos-serve.service",
                ),
                failed("run-p178036-i50400995.service"),
                failed("camera.service"),
            ],
            restarts: of("run-p958215-i38676451.service", &[100, 200, 300]),
            ..Read::default()
        };
        let tells = due(&read, &mut told, 400, |_| None, clock);
        let units: Vec<&str> = tells.iter().map(|t| t.unit.as_str()).collect();
        assert_eq!(units, ["camera.service"], "only the service that stays");
    }

    #[test]
    fn a_daemon_signed_out_is_told_at_once_with_its_reason() {
        let mut told = Told::default();
        let read = Read {
            daemons: vec![Daemon {
                name: "google".into(),
                answer: answer(concat!(
                    r#"{"sources":{"google":{"state":"signed-out","reason":"token refused"}},"#,
                    r#""state":"answering"}"#,
                )),
            }],
            ..Read::default()
        };
        let tells = due(&read, &mut told, 1000, watchdog, clock);
        assert_eq!(tells.len(), 1, "{tells:?}");
        assert_eq!(tells[0].unit, "google.service", "the daemon's unit");
        assert!(
            tells[0].text.starts_with(concat!(
                "The google daemon reads google signed-out, since t1000. ",
                "Its last error: \"token refused\"."
            )),
            "{}",
            tells[0].text
        );
    }

    #[test]
    fn a_daemon_silent_or_unreachable_is_told_only_once_it_holds() {
        let mut told = Told::default();
        let read = Read {
            daemons: vec![
                Daemon {
                    name: "people".into(),
                    answer: Answer::Silent("connection refused".into()),
                },
                Daemon {
                    name: "slack-unreads".into(),
                    answer: answer(r#"{"sources":{"slack":"unreachable"},"state":"answering"}"#),
                },
            ],
            ..Read::default()
        };
        assert!(
            due(&read, &mut told, 1000, |_| None, clock).is_empty(),
            "a daemon restarting mends within a tick"
        );
        let tells = due(&read, &mut told, 1300, |_| None, clock);
        assert_eq!(tells.len(), 2, "both held five minutes: {tells:?}");
        assert!(
            tells[0].text.starts_with(
                "The people daemon's socket doesn't answer (connection refused), since t1000."
            ),
            "{}",
            tells[0].text
        );
        assert!(
            tells[1]
                .text
                .starts_with("The slack-unreads daemon reads slack unreachable, since t1000."),
            "{}",
            tells[1].text
        );
    }

    #[test]
    fn a_daemon_answering_well_is_not_told() {
        let mut told = Told::default();
        let line = concat!(
            r#"{"sources":{"udisks":{"state":"starting","reason":""},"#,
            r#""kernel":"answering"},"state":"answering"}"#,
        );
        let read = Read {
            daemons: vec![Daemon {
                name: "machine".into(),
                answer: answer(line),
            }],
            ..Read::default()
        };
        assert!(
            due(&read, &mut told, 1000, |_| None, clock).is_empty(),
            "starting is well"
        );
        assert_eq!(
            answer("not json"),
            Answer::Silent("it said something that isn't JSON: not json".into()),
            "an answer that isn't JSON is no answer"
        );
    }

    #[test]
    fn restarts_are_read_from_journalctls_lines() {
        let journal = concat!(
            r#"{"__REALTIME_TIMESTAMP":"1794620862403331","USER_UNIT":"slack-unreads.service"}"#,
            "\n",
            r#"{"__REALTIME_TIMESTAMP":"1794620013434938","UNIT":"merge-net.service"}"#,
            "\nnot json\n",
        );
        assert_eq!(
            restarts(journal, |_| None),
            [
                Restart {
                    unit: "slack-unreads.service".into(),
                    at: 1_794_620_862,
                    after: None,
                    exit_read: false,
                },
                Restart {
                    unit: "merge-net.service".into(),
                    at: 1_794_620_013,
                    after: None,
                    exit_read: false,
                },
            ],
            "each line's unit and moment, in seconds"
        );
    }

    /// A journal line, as journalctl writes them: a process's exit, or a
    /// restart scheduled after it, at a moment in seconds.
    fn exit(unit: &str, code: &str, status: &str, at: i64) -> String {
        serde_json::json!({
            "__REALTIME_TIMESTAMP": format!("{at}000000"), "USER_UNIT": unit,
            "MESSAGE_ID": PROCESS_EXITED, "EXIT_CODE": code, "EXIT_STATUS": status,
        })
        .to_string()
    }

    fn restart(unit: &str, at: i64) -> String {
        serde_json::json!({
            "__REALTIME_TIMESTAMP": format!("{at}000000"), "USER_UNIT": unit,
            "MESSAGE_ID": RESTART_SCHEDULED,
        })
        .to_string()
    }

    /// A journal line naming the invocation of its unit it was written in,
    /// as systemd's lines of a unit do.
    fn of_invocation(line: &str, invocation: &str) -> String {
        let mut entry: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
        entry["USER_INVOCATION_ID"] = invocation.into();
        entry.to_string()
    }

    #[test]
    fn a_restart_carries_the_status_its_unit_exited_with_just_before() {
        // Two units' exits come before either's restart, as when the
        // network drops, so each must keep its own.
        let journal = [
            exit("repos-refresh.service", "exited", "69", 50),
            exit("pi-sweep.service", "killed", "15", 60),
            restart("repos-refresh.service", 100),
            // Restarted again with no exit read before it.
            restart("repos-refresh.service", 200),
            restart("pi-sweep.service", 300),
        ]
        .join("\n");
        let after: Vec<(String, Option<u32>, bool)> = restarts(&journal, |_| None)
            .into_iter()
            .map(|r| (r.unit, r.after, r.exit_read))
            .collect();
        assert_eq!(
            after,
            [
                ("repos-refresh.service".to_owned(), Some(69), true),
                ("repos-refresh.service".to_owned(), None, false),
                ("pi-sweep.service".to_owned(), None, true),
            ],
            "a restart didn't carry its own exit's status, took another unit's, or a kill's \
             signal was read as one"
        );
    }

    #[test]
    fn retry_statuses_read_the_numbers_and_quiet_ones_are_clean_on_failure_alone() {
        assert_eq!(
            (
                retry_statuses("75 100"),
                retry_statuses("69 SIGHUP\n"),
                retry_statuses("")
            ),
            (vec![75, 100], vec![69], vec![]),
            "a unit's retry statuses were misread"
        );
        let shown = |restart: &str, force: &str, success: &str| {
            format!(
                "Restart={restart}\nRestartForceExitStatus={force}\nSuccessExitStatus={success}\n"
            )
        };
        assert_eq!(
            [
                retrying(&shown("on-failure", "75 100", "75 100")),
                retrying(&shown("on-failure", "69", "")),
                retrying(&shown("always", "75 100", "75 100")),
                retrying(&shown("on-failure", "75 100", "75")),
                retrying(&shown("on-failure", "", "")),
            ],
            [
                (vec![75, 100], true),
                (vec![69], false),
                (vec![75, 100], false),
                (vec![75, 100], false),
                (vec![], false),
            ],
            "a unit's retries were read quiet though systemd journals some of their exits, \
             or not quiet though it journals none"
        );
    }

    #[test]
    fn a_unit_retrying_on_its_own_listed_status_is_no_loop_and_any_other_still_is() {
        let restarted = |unit: &str, after: Option<u32>| -> Vec<Restart> {
            [TICK + 60, TICK + 120, TICK + 180]
                .iter()
                .map(|&at| Restart {
                    unit: unit.to_owned(),
                    at,
                    after,
                    exit_read: after.is_some(),
                })
                .collect()
        };
        let mut restarts = restarted("repos-refresh.service", Some(69));
        // Another unit's listed status is no retry of this one's.
        restarts.extend(restarted("nix-offload-more.service", Some(69)));
        restarts.extend(restarted("slack-unreads.service", None));
        // A lender's retries leave no exit, as systemd journals no clean
        // one; a kill of it is journalled, and counts.
        let mut quiet = restarted("nix-offload.service", None);
        let mut killed = restarted("nix-offload-vm.service", None);
        for r in &mut killed {
            r.exit_read = true;
        }
        // Two listed retries and one other restart: only one counts.
        quiet[1].exit_read = true;
        restarts.extend(quiet);
        restarts.extend(killed);
        let lender = |unit: &str| (unit.to_owned(), vec![75, 100]);
        let read = Read {
            restarts,
            retries: [
                ("repos-refresh.service".to_owned(), vec![69]),
                lender("nix-offload-more.service"),
                lender("nix-offload.service"),
                lender("nix-offload-vm.service"),
            ]
            .into(),
            quiet: [
                "nix-offload.service",
                "nix-offload-more.service",
                "nix-offload-vm.service",
            ]
            .map(str::to_owned)
            .into(),
            ..Read::default()
        };
        let told: Vec<String> = due(&read, &mut Told::default(), TICK + 200, watchdog, clock)
            .into_iter()
            .map(|tell| tell.unit)
            .collect();
        assert_eq!(
            told,
            [
                "nix-offload-more.service",
                "nix-offload-vm.service",
                "slack-unreads.service"
            ],
            "a retry on a listed status, or a quiet one, was told as a loop, or a loop on \
             another status or a kill wasn't"
        );
    }

    /// What `systemctl show` says of a build lender: restarted only on
    /// failure, after systemd's default 100ms since it sets no delay, and
    /// retrying on clean exits.
    const LENDER: &str = "Restart=on-failure\nRestartUSec=100ms\n\
                          RestartForceExitStatus=75 100\nSuccessExitStatus=75 100\n";

    fn lender(times: &[i64]) -> Read {
        Read {
            restarts: of("nix-offload.service", times),
            retries: [("nix-offload.service".to_owned(), vec![75, 100])].into(),
            quiet: ["nix-offload.service".to_owned()].into(),
            ..Read::default()
        }
    }

    #[test]
    fn a_lenders_quiet_retries_are_told_as_a_loop_once_past_twice_a_loops_count() {
        let times: Vec<i64> = (1..=7).map(|n| TICK + n * 60).collect();
        assert_eq!(times.len(), QUIET_RETRIES + 1, "one restart past the cap");
        let now = TICK + 8 * 60;
        let capped = lender(&times[..QUIET_RETRIES]);
        assert!(
            due(&capped, &mut Told::default(), now, watchdog, clock).is_empty(),
            "a lender's quiet retries up to the cap were told as a loop"
        );
        assert_eq!(
            retries_said(&capped, now),
            "6 of them left out as retries (nix-offload.service 6)",
            "cadence units didn't say the retries it left out"
        );
        let past = lender(&times);
        let tells = due(&past, &mut Told::default(), now, watchdog, clock);
        assert_eq!(tells.len(), 1, "one more wasn't told: {tells:?}");
        assert!(
            tells[0]
                .text
                .starts_with("nix-offload.service is restarting in a loop: 7 restarts"),
            "past the cap, every quiet retry counts: {}",
            tells[0].text
        );
        assert_eq!(
            retries_said(&past, now),
            "none left out as retries",
            "quiet retries past the cap were still said to be left out"
        );
    }

    #[test]
    fn a_lender_told_looping_stays_one_spell_while_it_dips_under_the_cap() {
        let seven: Vec<i64> = (1..=7).map(|n| TICK + n * 120).collect();
        let now = TICK + 7 * 120 + 10;
        let mut told = Told::default();
        assert_eq!(
            due(&lender(&seven), &mut told, now, watchdog, clock).len(),
            1,
            "seven quiet retries in the window weren't told as a loop"
        );
        let six = lender(&seven[1..]);
        assert!(
            due(&six, &mut told, now, watchdog, clock).is_empty(),
            "a loop already told was told again as it dipped under the cap"
        );
        assert!(
            told.spells.contains_key("loop:nix-offload.service"),
            "a lender dipping to the cap ended its loop, so the next crossing reads as new"
        );
        assert!(
            due(&lender(&seven), &mut told, now, watchdog, clock).is_empty(),
            "a lender crossing the cap again was told as a new loop"
        );
    }

    #[test]
    fn a_unit_whose_retries_are_journalled_counts_a_restart_with_no_exit_read() {
        // repos-refresh's 69 is no success status, so systemd journals its
        // exit, and a restart with none read before it is no retry.
        let times = [TICK + 60, TICK + 120, TICK + 180];
        let read = Read {
            restarts: of("repos-refresh.service", &times),
            retries: [("repos-refresh.service".to_owned(), vec![69])].into(),
            ..Read::default()
        };
        let told: Vec<String> = due(&read, &mut Told::default(), TICK + 200, watchdog, clock)
            .into_iter()
            .map(|tell| tell.unit)
            .collect();
        assert_eq!(
            told,
            ["repos-refresh.service"],
            "restarts with no exit read were left out though the unit's retries aren't quiet"
        );
    }

    #[test]
    fn a_lender_restarted_with_no_exit_read_is_asked_about_and_one_killed_isnt() {
        let (quiet, killed) = ("nix-offload.service", "nix-offload-vm.service");
        let at = TICK + WITHIN;
        let journal = [
            restart(quiet, at - 120),
            of_invocation(&exit(killed, "killed", "9", at - 200), "a"),
            of_invocation(&restart(killed, at - 140), "a"),
        ]
        .join("\n");
        let asked = std::cell::RefCell::new(Vec::new());
        let read = paired(&journal, at, |unit| {
            asked.borrow_mut().push(unit.to_owned());
            LENDER.to_owned()
        });
        assert_eq!(
            asked.into_inner(),
            [quiet],
            "a restart with no exit read wasn't asked about, or one after a kill was"
        );
        assert_eq!(
            read.quiet,
            std::collections::BTreeSet::from([quiet.to_owned()]),
            "a lender restarted with no exit read wasn't read as quiet"
        );
        assert_eq!(
            read.retries,
            BTreeMap::from([(quiet.to_owned(), vec![75, 100])]),
            "a unit's retries were read for a restart that can't be one"
        );
    }

    #[test]
    fn an_exit_before_the_window_pairs_with_a_restart_in_it_and_a_restart_before_is_dropped() {
        let refresh = "repos-refresh.service";
        let at = TICK + WITHIN;
        let journal = [
            of_invocation(&exit(refresh, "exited", "69", at - WITHIN - 60), "r"),
            restart("merge-net.service", at - WITHIN - 30),
            of_invocation(&restart(refresh, at - WITHIN + 30), "r"),
        ]
        .join("\n");
        let shown = "Restart=on-failure\nRestartUSec=30s\nRestartForceExitStatus=69\n";
        let read = paired(&journal, at, |_| shown.to_owned());
        assert_eq!(
            read.restarts,
            [Restart {
                unit: refresh.into(),
                at: at - WITHIN + 30,
                after: Some(69),
                exit_read: true,
            }],
            "an exit read before the window didn't pair, or a restart before it was kept"
        );
    }

    #[test]
    fn a_stale_kill_leaves_a_later_quiet_restart_quiet() {
        let (lost, stale, fresh) = (
            "nix-offload.service",
            "nix-offload-more.service",
            "nix-offload-vm.service",
        );
        let at = TICK + WITHIN;
        let journal = [
            // A kill whose own restart was never read, and a restart of the
            // invocation after it.
            of_invocation(&exit(lost, "killed", "9", at - 200), "k"),
            of_invocation(&restart(lost, at - 100), "l"),
            // Lines naming no invocation, paired by the lender's delay
            // and half a minute: a kill a minute before its restart, past
            // that though inside the five minutes kept when no delay is
            // read, and one twenty seconds before, past the delay alone.
            exit(stale, "killed", "9", at - 120),
            restart(stale, at - 60),
            exit(fresh, "killed", "9", at - 80),
            restart(fresh, at - 60),
        ]
        .join("\n");
        let asked = std::cell::RefCell::new(BTreeMap::<String, usize>::new());
        let read = paired(&journal, at, |unit| {
            *asked.borrow_mut().entry(unit.to_owned()).or_default() += 1;
            LENDER.to_owned()
        });
        assert!(
            asked.into_inner().values().all(|&n| n == 1),
            "a unit was asked of systemctl more than once in a tick"
        );
        let exits_read: Vec<(&str, bool)> = read
            .restarts
            .iter()
            .map(|r| (r.unit.as_str(), r.exit_read))
            .collect();
        assert_eq!(
            exits_read,
            [(lost, false), (stale, false), (fresh, true)],
            "a kill of an invocation before was paired with a later restart, or one just \
             before its restart wasn't"
        );
        assert_eq!(
            retries_said(&read, at),
            "2 of them left out as retries (nix-offload-more.service 1, nix-offload.service 1)",
            "a quiet restart after a stale kill was counted"
        );
    }

    #[test]
    fn an_exit_whose_restart_names_another_invocation_pairs_by_the_delay() {
        let lender = "nix-offload.service";
        let at = TICK + WITHIN;
        let journal = [
            of_invocation(&exit(lender, "exited", "75", at - 70), "a"),
            of_invocation(&restart(lender, at - 60), "b"),
        ]
        .join("\n");
        let read = paired(&journal, at, |_| LENDER.to_owned());
        assert_eq!(
            read.restarts.iter().map(|r| r.after).collect::<Vec<_>>(),
            [Some(75)],
            "an exit just before a restart naming the next invocation wasn't paired"
        );
    }

    #[test]
    fn a_span_systemctl_writes_is_read_in_whole_seconds_rounded_up() {
        let spans = ["100ms", "1.5s", "1min 30s", "0", "infinity", "", "2y"];
        assert_eq!(
            spans.map(seconds),
            [Some(1), Some(2), Some(90), Some(0), None, None, None],
            "a restart delay as systemctl show writes it was misread"
        );
    }
}
