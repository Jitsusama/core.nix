//! Readings another program already counts, read from what it writes
//! rather than counted again: code-health's regressions, which cadence
//! day judges into its findings ledger under keys starting `code.`; the
//! lines where Joel sounded pained, which health counts in its day file;
//! the turns making one tool call, which the daily measure counts; and
//! the security musts past due and holes open, which ISXEG8 files in the
//! ledger; the asks for a secret, which health counts from secret-asks's
//! journal; and boot, shutdown, disk and memory, from health's day file.
//! Each stays unread until its program has written a day.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Bar, Reading, open, shown};
use crate::findings::Entry;

/// Everything these readings are read from.
#[derive(Debug, Default)]
pub struct Sources {
    /// The ledger's open findings, when cadence day has kept one.
    pub findings: Option<Vec<Entry>>,
    /// Whether code-health has written a day yet.
    pub code_health: bool,
    /// Today's figures from health's day file, when it wrote one.
    pub health: Option<BTreeMap<String, f64>>,
}

/// The code-health regressions found and not yet fixed: the open
/// findings cadence day keyed `code.<repo>.<measure>`.
#[must_use]
pub fn regressions(s: &Sources) -> Reading {
    let open = s
        .findings
        .as_ref()
        .filter(|_| s.code_health)
        .map(|all| all.iter().filter(|f| f.key.starts_with("code.")).count());
    Reading::new("{} code-health regressions", open.map(count), "", "Y9XISP")
        .bar(Bar::AtMost(0.0))
        .detail(Some(
            "found worse than the days before and still open".to_owned(),
        ))
        .from("cadence day's findings, from code-health's figures")
}

/// ISXEG8's musts open past their date: the security entries it filed with
/// `cadence owe`, holes aside, whose date is before `now`. Cadence day's
/// own findings carry no kind and are left out, so an asks figure judged
/// late isn't counted as a must as well.
#[must_use]
pub fn musts(s: &Sources, now: i64) -> Reading {
    let late = s.findings.as_ref().map(|all| {
        all.iter()
            .filter(|f| f.axis == "security" && f.due < now)
            .filter(|f| f.kind.as_deref().is_some_and(|kind| kind != "hole"))
            .count()
    });
    Reading::new("{} security musts past due", late.map(count), "", "ISXEG8")
        .bar(Bar::AtMost(0.0))
        .from("the ledger's musts, as ISXEG8 files them")
}

/// The security holes known and not yet closed, as ISXEG8 files them with
/// kind "hole". An entry says what kind of hole and where, never a secret,
/// a path to one or a command line, so naming them here shows nothing.
#[must_use]
pub fn holes(s: &Sources) -> Reading {
    let open = s.findings.as_ref().map(|all| {
        all.iter()
            .filter(|f| f.axis == "security" && f.kind.as_deref() == Some("hole"))
            .count()
    });
    Reading::new(
        "{} security holes known and open",
        open.map(count),
        "",
        "ISXEG8",
    )
    .bar(Bar::AtMost(0.0))
    .from("the ledger's holes, as ISXEG8 files them")
}

/// The asks for a secret today, as health counts them from secret-asks:
/// an ask is worth its interruption only while asks stay rare, read before
/// they're answered, asked once, and never one a minted token would have
/// spared. The first three are what Joel saw and did, so they're his side.
#[must_use]
pub fn asks(s: &Sources) -> Vec<Reading> {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let from = "health's day file, from secret-asks";
    vec![
        Reading::new(
            "{} asks for a secret shown to Joel today",
            figure("health.asks.shown"),
            "",
            "ISXEG8",
        )
        .bar(Bar::AtMost(3.0))
        .from(from)
        .his(),
        Reading::new(
            "{} asks said yes to before they could be read",
            figure("health.asks.unread-yeses"),
            "",
            "ISXEG8",
        )
        .bar(Bar::AtMost(0.0))
        .from(from)
        .his(),
        Reading::new(
            "{} asks for an item and command already asked for today",
            figure("health.asks.again"),
            "",
            "ISXEG8",
        )
        .bar(Bar::AtMost(0.0))
        .from(from)
        .his(),
        Reading::new(
            "{} of asks a minted token would have spared",
            figure("health.asks.avoidable-percent"),
            "%",
            "ISXEG8",
        )
        .bar(Bar::AtMost(0.0))
        .from(from),
    ]
}

/// The lines Joel typed today that sounded pained, as health counts them.
/// Each is a pain point the loop learns from, so the bar is none.
#[must_use]
pub fn pained(s: &Sources) -> Reading {
    Reading::new(
        "{} lines where Joel sounded pained today",
        s.health
            .as_ref()
            .and_then(|figures| figures.get("health.words.errors").copied()),
        "",
        "1JQPPC",
    )
    .bar(Bar::AtMost(0.0))
    .from("health's day file")
    .his()
}

/// What the machine held, from health's day file: the fullest disk, held to
/// the disk guard's nearly-full band, and the most pi and the shell held at
/// once. The memory peaks carry no bar until the split of resources sets
/// one, so they're shown and never in spec.
#[must_use]
pub fn held(s: &Sources) -> Vec<Reading> {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let from = "health's day file";
    vec![
        Reading::new(
            "the fullest disk {} in use",
            figure("health.disk.most-used-percent"),
            "%",
            "80ECO2",
        )
        .bar(Bar::AtMost(90.0))
        .from(from),
        Reading::new(
            "the most pi held at once {}",
            figure("health.memory.pi.most-mib"),
            " MiB",
            "80ECO2",
        )
        .from(from),
        Reading::new(
            "the most the shell held at once {}",
            figure("health.memory.quickshell.most-mib"),
            " MiB",
            "80ECO2",
        )
        .from(from),
    ]
}

/// The launcher's keys today, each timed from his key to its frame, held to
/// docs/design.md's budget: drawn in 50 ms at the median and 100 ms at the
/// 99th percentile. From lines like `launcher: "fi" took 31 ms to its
/// frame`, a drawer's left out. The shell journals only a key slower than
/// a frame at 120 Hz, about half of his, so of the `timed` keys its trials
/// counted, those missing from the journal are taken at that frame: left
/// out, they'd make the middle key look slower than it was.
#[must_use]
pub fn keys(journal: &str, timed: usize) -> Vec<Reading> {
    let mut took: Vec<u32> = journal
        .lines()
        .filter_map(|line| {
            let after = line.split("launcher: \"").nth(1)?;
            let ms = after.rsplit_once("\" took ")?.1;
            ms.split_whitespace().next()?.parse().ok()
        })
        .collect();
    took.sort_unstable();
    let unsaid = timed.saturating_sub(took.len());
    let all = took.len() + unsaid;
    // The nearest rank: the key that many in a hundred took no longer than,
    // the unsaid keys first, since none took past a frame.
    let rank = |percent: usize| {
        (all > 0).then(|| match (all * percent).div_ceil(100) {
            rank if rank <= unsaid => f64::from(UNSAID_MS),
            rank => f64::from(took[rank - unsaid - 1]),
        })
    };
    let of = Some(match unsaid {
        0 => format!("of {all} keys"),
        _ => format!("of {all} keys, {unsaid} drawn within a frame"),
    });
    vec![
        Reading::new(
            "the launcher's middle key {} today",
            rank(50),
            " ms",
            "Y2YBTS",
        )
        .bar(Bar::AtMost(50.0))
        .detail(of.clone())
        .from("the shell's journal and its trials")
        .his(),
        Reading::new(
            "the launcher's slowest key in a hundred {} today",
            rank(99),
            " ms",
            "Y2YBTS",
        )
        .bar(Bar::AtMost(100.0))
        .detail(of)
        .from("the shell's journal and its trials")
        .his(),
    ]
}

/// The slowest a key the shell doesn't journal took: a frame at 120 Hz, as
/// modules/quickshell/launcher/Trials.js's `FELT_MS` has it.
const UNSAID_MS: u32 = 8;

/// How many keys the launcher timed since `since`, in milliseconds, from
/// its trial records, one opening a line, each counting its keys timed.
#[must_use]
pub fn timed(trials: &str, since: i64) -> usize {
    trials
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|trial| trial["at"].as_i64().is_some_and(|at| at >= since))
        .filter_map(|trial| trial["timed"]["count"].as_u64())
        .map(|count| usize::try_from(count).unwrap_or(usize::MAX))
        .sum()
}

/// How fast the machine answers Joel, from health's day file: the slowest
/// boot to a ready launcher with his password left out, the slowest
/// shutdown, and the minutes the day's worst moments cost him, each timed
/// from his key to what he waited for, so each is his side. The bars are
/// the ones docs/design.md holds the machine to; no minute lost is the
/// bar for the last, since he asked for end to end perfection. Programs
/// opening and ending follow, held to design.md's 300 ms and 200 ms at the
/// 99th percentile, as for keys; the windows daemon times them from the
/// process starting and its last window gone, since niri never says the
/// key that asked, so they are our side's. Restarts close the list, our
/// side's too: a daemon serving again within a second of starting, from
/// its own ready line, and the slowest unit a switch restarted back within
/// 5 s, the specs the improvement system set for them (2026-11-11).
#[must_use]
pub fn responds(s: &Sources) -> Vec<Reading> {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let from = "health's day file";
    let program = |says: &'static str, name: &str, budget: f64, timed: &str| {
        let spread = figure(&format!("health.launches.{name}.median-ms"))
            .zip(figure(&format!("health.launches.{name}.slowest-ms")))
            .map(|(median, slowest)| {
                format!(
                    "a median {}, the slowest {}; timed from {timed}",
                    shown(median, " ms"),
                    shown(slowest, " ms")
                )
            });
        Reading::new(
            says,
            figure(&format!("health.launches.{name}.p99-ms")),
            " ms",
            "80ECO2",
        )
        .bar(Bar::AtMost(budget))
        .detail(spread)
        .from("health's day file, from the windows daemon")
    };
    vec![
        Reading::new(
            "the slowest boot {} to a ready launcher, the password left out",
            figure("health.boot.seconds"),
            " s",
            "80ECO2",
        )
        .bar(Bar::AtMost(30.0))
        .from(from)
        .his(),
        Reading::new(
            "the slowest shutdown {}",
            figure("health.shutdown.seconds"),
            " s",
            "80ECO2",
        )
        .bar(Bar::AtMost(10.0))
        .from(from)
        .his(),
        Reading::new(
            "time lost in the day's worst moments {}",
            figure("health.feel.lost-minutes"),
            " min",
            "80ECO2",
        )
        .bar(Bar::AtMost(0.0))
        .from("health's day file, from its feel of his tasks")
        .his(),
        program(
            "the slowest program in a hundred to its first window {}",
            "opened",
            300.0,
            "the program starting",
        ),
        program(
            "the slowest program in a hundred to end once closed {}",
            "ended",
            200.0,
            "its last window closing",
        ),
        Reading::new(
            "the slowest daemon in a hundred to serve again {}",
            figure("health.launches.ready.p99-ms"),
            " ms",
            "80ECO2",
        )
        .bar(Bar::AtMost(1_000.0))
        .from("health's day file, from each program's ready line"),
        Reading::new(
            "the slowest restart in a switch {}",
            figure("health.switch.restart.slowest-ms"),
            " ms",
            "80ECO2",
        )
        .bar(Bar::AtMost(5_000.0))
        .detail(figure("health.switch.restart.median-ms").map(|median| {
            format!(
                "a median {}; timed from its stopping to its ready",
                shown(median, " ms")
            )
        }))
        .from("health's day file"),
    ]
}

/// The share of the last day's turns that made exactly one tool call, from
/// the newest measure's "Turns Doing One Thing" table, whose first row is
/// the last day: `| ending | turns | one | none | several |`.
#[must_use]
pub fn one_call(measure: Option<&str>) -> Reading {
    let latest = measure.and_then(|text| {
        let (_, section) = text.split_once("## Turns Doing One Thing")?;
        let section = section.split("\n## ").next().unwrap_or_default();
        section
            .lines()
            .filter(|line| line.starts_with("| 20"))
            .find_map(|row| {
                let cells: Vec<f64> = row
                    .split('|')
                    .map(str::trim)
                    .skip(2)
                    .filter_map(|cell| cell.replace(',', "").parse().ok())
                    .collect();
                match cells.as_slice() {
                    [turns, one, ..] if *turns > 0.0 => Some((*one, *turns)),
                    _ => None,
                }
            })
    });
    Reading::new(
        "turns making one tool call {}",
        latest.map(|(one, turns)| one / turns * 100.0),
        "%",
        "QCXZEB",
    )
    .bar(Bar::AtMost(60.0))
    .detail(
        latest.map(|(one, turns)| {
            format!("{} of {} in the last day", shown(one, ""), shown(turns, ""))
        }),
    )
    .from("the measure's turns doing one thing")
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Reads the ledger in `folder`, whether code-health has written, and
/// health's file for the day `now` falls in: files only.
#[must_use]
pub fn gather(folder: &Path, state_home: &Path, now: i64) -> Sources {
    let date = Timestamp::from_second(now)
        .map(|t| {
            t.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default();
    Sources {
        findings: folder
            .join("findings.jsonl")
            .exists()
            .then(|| open::gather(folder)),
        code_health: fs::read_dir(state_home.join("code-health"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "json")),
        health: fs::read_to_string(state_home.join(format!("health/{date}.json")))
            .ok()
            .map(|json| crate::machine::figures(&json)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(key: &str) -> Entry {
        on(key, "quality", 0, "null")
    }

    /// A ledger entry; `kind` is a filed entry's, or null for cadence's own.
    fn on(key: &str, axis: &str, due: i64, kind: &str) -> Entry {
        serde_json::from_str(&format!(
            r#"{{"key":"{key}","first":"2026-11-12","last":"2026-11-12","last_to":0,
            "due":{due},"words":"rose","axis":"{axis}","evidence":"","fell_when":"",
            "open":true,"late":false,"kind":{kind}}}"#
        ))
        .expect("a finding parses")
    }

    #[test]
    fn musts_are_the_filed_security_musts_past_their_date() {
        let s = Sources {
            findings: Some(vec![
                on("security:polkit", "security", 100, r#""must""#),
                on("security:asks", "security", 300, r#""must""#),
                on("security:fence", "security", 100, r#""hole""#),
                on("health.asks.again", "security", 100, "null"),
                on("crashes:biome", "quality", 100, r#""must""#),
            ]),
            ..Sources::default()
        };
        assert_eq!(
            musts(&s, 200).words(),
            "1 security must past due, at most 0"
        );
        assert_eq!(
            holes(&s).words(),
            "1 security hole known and open, at most 0"
        );
        assert_eq!(musts(&Sources::default(), 200).figure(), None);
    }

    #[test]
    fn regressions_are_the_open_code_findings() {
        let s = Sources {
            findings: Some(vec![
                finding("code.core.nix.long-files"),
                finding("crashes:biome"),
                finding("code.mycelium.suppressions"),
            ]),
            code_health: true,
            health: None,
        };
        assert_eq!(
            regressions(&s).words(),
            "2 code-health regressions, at most 0"
        );
    }

    #[test]
    fn regressions_wait_for_code_health_and_the_ledger() {
        let before = Sources {
            findings: Some(vec![]),
            code_health: false,
            health: None,
        };
        assert_eq!(regressions(&before).figure(), None);
        let no_ledger = Sources {
            findings: None,
            code_health: true,
            health: None,
        };
        assert_eq!(regressions(&no_ledger).figure(), None);
    }

    #[test]
    fn one_call_turns_are_the_last_days_share() {
        let measure = "# Measure\n\n## Turns Doing One Thing\n\n79 of 100 turns made one.\n\n\
            | Day ending | Turns | One call | None | Two or more |\n|---|---|---|---|---|\n\
            | 2026-11-12 | 1000 | 800 | 50 | 150 |\n| 2026-11-11 | 900 | 600 | 0 | 300 |\n\n\
            | Tool | One-call turns | Right after its own |\n|---|---|---|\n| bash | 400 | 200 |\n\n\
            ## Next\n";
        let reading = one_call(Some(measure));
        assert_eq!(
            reading.words(),
            "turns making one tool call 80%, at most 60%"
        );
        assert_eq!(one_call(Some("# Measure\n")).figure(), None);
        assert_eq!(one_call(None).figure(), None);
    }

    #[test]
    fn asks_come_from_healths_figures() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.asks.shown".to_owned(), 4.0),
                ("health.asks.unread-yeses".to_owned(), 1.0),
            ])),
            ..Sources::default()
        };
        let words: Vec<String> = asks(&s).iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "4 asks for a secret shown to Joel today, at most 3",
                "1 ask said yes to before they could be read, at most 0",
                "asks for an item and command already asked for today: no reading yet",
                "of asks a minted token would have spared: no reading yet",
            ]
        );
    }

    #[test]
    fn what_the_machine_held_comes_from_healths_figures() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.disk.most-used-percent".to_owned(), 93.0),
                ("health.memory.pi.most-mib".to_owned(), 4096.0),
            ])),
            ..Sources::default()
        };
        let read = held(&s);
        let words: Vec<String> = read.iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "the fullest disk 93% in use, at most 90%",
                "the most pi held at once 4,096 MiB",
                "the most the shell held at once: no reading yet",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [true, false, false],
            "only the disk has a bar to be past"
        );
    }

    #[test]
    fn launcher_keys_are_held_to_their_median_and_99th_percentile() {
        let journal = r#"qml: launcher: "fi" took 31 ms to its frame, 4 ms searching
qml: launcher: "fir" took 74 ms to its frame, 60 ms searching
qml: launcher: "a \" took 9" took 120 ms to its frame
qml: launcher: drawer for "Firefox" took 80 ms to its frame
"#;
        let read = keys(journal, 0);
        let words: Vec<String> = read.iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "the launcher's middle key 74 ms today, at most 50 ms",
                "the launcher's slowest key in a hundred 120 ms today, at most 100 ms",
            ]
        );
        assert!(
            keys("", 0).iter().all(|r| r.figure().is_none()),
            "a day with no keys reads as unread, not as fast"
        );
        // Seven keys timed, three slow enough for the journal: the other
        // four took a frame at most, so the middle key is one of them.
        let read = keys(journal, 7);
        assert_eq!(
            read.iter().map(Reading::figure).collect::<Vec<_>>(),
            [Some(8.0), Some(120.0)],
            "the keys the journal leaves out weren't counted as fast"
        );
        assert_eq!(
            read[0].detail.as_deref(),
            Some("of 7 keys, 4 drawn within a frame")
        );
    }

    #[test]
    fn the_keys_timed_today_are_counted_from_the_trials() {
        let trials = r#"{"at":5000,"timed":{"count":6,"felt":[]}}
{"at":100,"timed":{"count":9,"felt":[]}}
not a trial
{"at":7000,"keys":{}}
{"at":8000,"timed":{"count":2,"felt":[]}}"#;
        assert_eq!(
            timed(trials, 1000),
            8,
            "only today's openings, by their keys timed"
        );
    }

    #[test]
    fn responsiveness_comes_from_healths_figures() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.boot.seconds".to_owned(), 27.0),
                ("health.shutdown.seconds".to_owned(), 12.0),
                ("health.launches.opened.p99-ms".to_owned(), 420.0),
                ("health.launches.opened.median-ms".to_owned(), 90.0),
                ("health.launches.opened.slowest-ms".to_owned(), 1_300.0),
                ("health.launches.ended.p99-ms".to_owned(), 150.0),
                ("health.launches.ready.p99-ms".to_owned(), 1_065.0),
                ("health.switch.restart.slowest-ms".to_owned(), 4_200.0),
                ("health.switch.restart.median-ms".to_owned(), 900.0),
            ])),
            ..Sources::default()
        };
        let read = responds(&s);
        let words: Vec<String> = read.iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "the slowest boot 27 s to a ready launcher, the password left out, at most 30 s",
                "the slowest shutdown 12 s, at most 10 s",
                "time lost in the day's worst moments: no reading yet",
                "the slowest program in a hundred to its first window 420 ms, at most 300 ms",
                "the slowest program in a hundred to end once closed 150 ms, at most 200 ms",
                "the slowest daemon in a hundred to serve again 1,065 ms, at most 1,000 ms",
                "the slowest restart in a switch 4,200 ms, at most 5,000 ms",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [false, true, false, true, false, true, false],
            "only the shutdown, the openings and the daemons are past their bars"
        );
        assert_eq!(
            read[6].detail.as_deref(),
            Some("a median 900 ms; timed from its stopping to its ready"),
            "a switch's restarts say their median and where they're timed from"
        );
        assert_eq!(
            read[3].detail.as_deref(),
            Some("a median 90 ms, the slowest 1,300 ms; timed from the program starting"),
            "an opening says its spread and that it's timed from our side"
        );
    }

    #[test]
    fn pained_lines_come_from_healths_figure() {
        let s = Sources {
            health: Some(BTreeMap::from([("health.words.errors".to_owned(), 1.0)])),
            ..Sources::default()
        };
        assert_eq!(
            pained(&s).words(),
            "1 line where Joel sounded pained today, at most 0"
        );
        assert_eq!(pained(&Sources::default()).figure(), None);
    }
}
