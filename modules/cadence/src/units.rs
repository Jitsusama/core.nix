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

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::findings::owner_of;

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

/// One restart systemd scheduled, of `unit`, at a moment in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restart {
    pub unit: String,
    pub at: i64,
}

/// A unit in the failed state, and since when, when systemd says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub unit: String,
    pub since: Option<i64>,
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

/// What one tick read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    pub restarts: Vec<Restart>,
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

/// Reads the restarts in journalctl's JSON lines, one object a line.
#[must_use]
pub fn restarts(journal: &str) -> Vec<Restart> {
    journal
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|entry| {
            let unit = entry.get("USER_UNIT").or_else(|| entry.get("UNIT"))?;
            let micros: i64 = entry.get("__REALTIME_TIMESTAMP")?.as_str()?.parse().ok()?;
            Some(Restart {
                unit: unit.as_str()?.to_owned(),
                at: micros / 1_000_000,
            })
        })
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
            &format!("@{}", at - WITHIN),
            "--until",
            &format!("@{at}"),
            &format!("MESSAGE_ID={RESTART_SCHEDULED}"),
        ],
    );
    let mut read = Read {
        restarts: restarts(&journal.unwrap_or_default()),
        ..Read::default()
    };
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
        .map(|unit| Failed {
            unit: unit.to_owned(),
            since: changed(unit),
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

/// When systemd says `unit` last changed state, which for one failed is
/// when it failed.
fn changed(unit: &str) -> Option<i64> {
    let args = [
        "--user",
        "show",
        "--timestamp=unix",
        "-p",
        "StateChangeTimestamp",
        "--value",
    ];
    let said = run("systemctl", &[&args[..], &[unit]].concat()).ok()?;
    said.trim().strip_prefix('@')?.parse().ok()
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

/// The owner of `unit`: whoever `owners` names for `unit:<unit>`, or,
/// failing that, for `unstamped:<unit>`, since the session that owns a
/// unit's readiness owns the unit.
#[must_use]
pub fn owner(unit: &str, owners: &BTreeMap<String, String>) -> Option<String> {
    owner_of(&format!("unit:{unit}"), owners)
        .or_else(|| owner_of(&format!("unstamped:{unit}"), owners))
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
    for r in read
        .restarts
        .iter()
        .filter(|r| r.at > now - WITHIN && r.at <= now)
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
    for f in &read.failed {
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
                    at: 1_794_620_862
                },
                Restart {
                    unit: "merge-net.service".into(),
                    at: 1_794_620_013
                },
            ],
            "each line's unit and moment, in seconds"
        );
    }

    #[test]
    fn a_units_owner_is_its_own_or_its_readiness_owner() {
        let owners: BTreeMap<String, String> = [
            ("unit:google.service", "RTOIMQ"),
            ("unstamped:google.service", "OTHER1"),
            ("unstamped:slack-unreads.service", "RCTRF8"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        assert_eq!(
            owner("google.service", &owners).as_deref(),
            Some("RTOIMQ"),
            "its own"
        );
        assert_eq!(
            owner("slack-unreads.service", &owners).as_deref(),
            Some("RCTRF8"),
            "its readiness owner"
        );
        assert_eq!(owner("people.service", &owners), None, "none named");
    }
}
