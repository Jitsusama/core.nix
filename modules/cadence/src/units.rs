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

mod shown;

pub use shown::leave_runs;
use shown::{SHOW, shown_from};

/// How far back restarts are counted.
pub const WITHIN: i64 = 15 * 60;

/// How many restarts within `WITHIN` make a loop. One restart is a unit
/// mending itself, as merge-net's at 20:13 on 13 November did.
pub const LOOPING: usize = 3;

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
/// before the first restart counted is read too: the longest wait before
/// a restart among the account's units is repos-refresh's 30 seconds.
const EXITS_BEFORE: i64 = 5 * 60;

/// One restart systemd scheduled, of `unit`, at a moment in seconds, and
/// the status its process exited with just before, when it exited itself
/// rather than being killed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restart {
    pub unit: String,
    pub at: i64,
    pub after: Option<u32>,
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

/// What one tick read. `retries` are, for each unit restarted after an
/// exit of its own, the statuses its `RestartForceExitStatus` names: the
/// unit retries on these by design, as repos-refresh does on 69 while the
/// network is away, so a restart after one is no loop, and its start limit
/// fails it, told as failed, if the retries never end.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    pub restarts: Vec<Restart>,
    pub retries: BTreeMap<String, Vec<u32>>,
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
/// the journal's order, each with the status of its unit's exit before
/// it. A line of a process's exit is no restart; any other is one.
#[must_use]
pub fn restarts(journal: &str) -> Vec<Restart> {
    let mut exited: BTreeMap<String, Option<u32>> = BTreeMap::new();
    let mut restarts = Vec::new();
    for entry in journal
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    {
        let field = |name: &str| entry.get(name).and_then(serde_json::Value::as_str);
        let Some(unit) = field("USER_UNIT").or_else(|| field("UNIT")) else {
            continue;
        };
        if field("MESSAGE_ID") == Some(PROCESS_EXITED) {
            let status = field("EXIT_STATUS").and_then(|s| s.parse().ok());
            let own = field("EXIT_CODE") == Some("exited");
            exited.insert(unit.to_owned(), status.filter(|_| own));
            continue;
        }
        let Some(micros) = field("__REALTIME_TIMESTAMP").and_then(|t| t.parse::<i64>().ok()) else {
            continue;
        };
        restarts.push(Restart {
            unit: unit.to_owned(),
            at: micros / 1_000_000,
            after: exited.remove(unit).flatten(),
        });
    }
    restarts
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

/// Reads what a tick needs: the restarts systemd scheduled in the
/// account's units in the `WITHIN` before `at`, and, when `now`, the units
/// failed and each daemon's socket under `runtime`, which can't be read as
/// they were in the past.
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
    let mut read = Read {
        restarts: restarts(&journal.unwrap_or_default()),
        ..Read::default()
    };
    let retried: Vec<String> = read
        .restarts
        .iter()
        .filter(|r| r.after.is_some())
        .map(|r| r.unit.clone())
        .collect();
    for unit in retried {
        if read.retries.contains_key(&unit) {
            continue;
        }
        let show = [
            "--user",
            "show",
            "-p",
            "RestartForceExitStatus",
            "--value",
            &unit,
        ];
        let shown = run("systemctl", &show).unwrap_or_default();
        read.retries.insert(unit, retry_statuses(&shown));
    }
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

/// Everything broken now, each with when it began.
fn broken(read: &Read, told: &Told, now: i64, clock: &impl Fn(i64) -> String) -> Vec<Broken> {
    let mut broken = Vec::new();
    let mut looping: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    let retry = |r: &Restart| {
        r.after.is_some_and(|status| {
            read.retries
                .get(&r.unit)
                .is_some_and(|s| s.contains(&status))
        })
    };
    for r in read
        .restarts
        .iter()
        .filter(|r| r.at > now - WITHIN && r.at <= now && !a_run(&r.unit) && !retry(r))
    {
        looping.entry(&r.unit).or_default().push(r.at);
    }
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
            restarts(journal),
            [
                Restart {
                    unit: "slack-unreads.service".into(),
                    at: 1_794_620_862,
                    after: None,
                },
                Restart {
                    unit: "merge-net.service".into(),
                    at: 1_794_620_013,
                    after: None,
                },
            ],
            "each line's unit and moment, in seconds"
        );
    }

    /// A journal line, as journalctl writes them: a process's exit, or a
    /// restart scheduled after it.
    fn exit(unit: &str, code: &str, status: &str) -> String {
        serde_json::json!({
            "__REALTIME_TIMESTAMP": "1794620800000000", "USER_UNIT": unit,
            "MESSAGE_ID": PROCESS_EXITED, "EXIT_CODE": code, "EXIT_STATUS": status,
        })
        .to_string()
    }

    fn restart(unit: &str, seconds: i64) -> String {
        serde_json::json!({
            "__REALTIME_TIMESTAMP": format!("{seconds}000000"), "USER_UNIT": unit,
            "MESSAGE_ID": RESTART_SCHEDULED,
        })
        .to_string()
    }

    #[test]
    fn a_restart_carries_the_status_its_unit_exited_with_just_before() {
        let journal = [
            exit("repos-refresh.service", "exited", "69"),
            restart("repos-refresh.service", 100),
            // Restarted again with no exit read before it.
            restart("repos-refresh.service", 200),
            exit("pi-sweep.service", "killed", "15"),
            restart("pi-sweep.service", 300),
        ]
        .join("\n");
        let after: Vec<(String, Option<u32>)> = restarts(&journal)
            .into_iter()
            .map(|r| (r.unit, r.after))
            .collect();
        assert_eq!(
            after,
            [
                ("repos-refresh.service".to_owned(), Some(69)),
                ("repos-refresh.service".to_owned(), None),
                ("pi-sweep.service".to_owned(), None),
            ],
            "a restart didn't carry its own exit's status, or a kill's signal was read as one"
        );
        assert_eq!(
            (
                retry_statuses("75 100"),
                retry_statuses("69 SIGHUP\n"),
                retry_statuses("")
            ),
            (vec![75, 100], vec![69], vec![]),
            "a unit's retry statuses were misread"
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
                })
                .collect()
        };
        let mut restarts = restarted("repos-refresh.service", Some(69));
        restarts.extend(restarted("nix-offload-more.service", Some(1)));
        restarts.extend(restarted("slack-unreads.service", None));
        let read = Read {
            restarts,
            retries: [
                ("repos-refresh.service".to_owned(), vec![69]),
                ("nix-offload-more.service".to_owned(), vec![75, 100]),
            ]
            .into(),
            ..Read::default()
        };
        let told: Vec<String> = due(&read, &mut Told::default(), TICK + 200, watchdog, clock)
            .into_iter()
            .map(|tell| tell.unit)
            .collect();
        assert_eq!(
            told,
            ["nix-offload-more.service", "slack-unreads.service"],
            "a retry on a listed status was told as a loop, or a loop on another wasn't"
        );
    }
}
