//! Readings another program already counts, read from what it writes
//! rather than counted again: code-health's regressions, which cadence
//! day judges into its findings ledger under keys starting `code.`; the
//! lines where Joel sounded pained, which health counts in its day file;
//! the security musts past due and holes open, which ISXEG8 files in the
//! ledger; the asks for a secret, which health counts from secret-asks's
//! journal; boot, shutdown, disk, memory and launches, from health's day
//! file; and his apps' stalls, from cadence day's own figures. The daily
//! measure's readings keep a file of their own, measure.rs, and so do the
//! launcher's, launcher.rs. Each stays unread until its program has written a day.

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

/// His programs' crashes today, as health splits them since 12 November:
/// a core dump from his apps' or his desktop's slice is his, and any other
/// ours, such as an agent's test that ran sway 39 times that day
/// (80ECO2). Any crash of his is a pain point, so the bar is none; ours
/// are said beside it, and judged on cadence's day as a stand-in.
#[must_use]
pub fn crashes(s: &Sources) -> Reading {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    Reading::new(
        "{} crashes of his programs today",
        figure("health.crashes.his"),
        "",
        "80ECO2",
    )
    .bar(Bar::AtMost(0.0))
    .detail(figure("health.crashes.ours").map(|ours| format!("ours {}", shown(ours, ""))))
    .from("health's day file, by the slice each ran in")
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
    let spread = |name: &str, timed: &str| {
        figure(&format!("health.launches.{name}.median-ms"))
            .zip(figure(&format!("health.launches.{name}.slowest-ms")))
            .map(|(median, slowest)| {
                format!(
                    "a median {}, the slowest {}; timed from {timed}",
                    shown(median, " ms"),
                    shown(slowest, " ms")
                )
            })
    };
    let program = |says: &'static str, name: &str, budget: f64, timed: &str| {
        Reading::new(
            says,
            figure(&format!("health.launches.{name}.p99-ms")),
            " ms",
            "80ECO2",
        )
        .bar(Bar::AtMost(budget))
        .detail(spread(name, timed))
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
        // pi's start is kept apart from the daemons' since #3799, as its
        // resumes run to seconds; the harness owns making it quicker.
        Reading::new(
            "the slowest pi in a hundred to show its session {}",
            figure("health.launches.pi.p99-ms"),
            " ms",
            "85PKB2",
        )
        .bar(Bar::AtMost(3_000.0))
        .detail(spread("pi", "pi starting"))
        .from("health's day file, from pi's ready line"),
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

/// How long his apps and his desktop waited, from the stalls cadence day
/// keeps for app.slice and session.slice (niri, the shell and what runs
/// them): each one's CPU wait and memory stalls, as a share of the day.
/// They are his side's, since the kernel counts them on his own programs
/// rather than on a builder or a model of him. 80ECO2, who keeps them,
/// holds all four to 1% (2026-11-12).
#[must_use]
pub fn stalls(day: Option<&BTreeMap<String, f64>>) -> Vec<Reading> {
    let figure = |key: &str| day.and_then(|d| d.get(key).copied());
    let stall = |says: &'static str, key: &str, from: &'static str| {
        Reading::new(says, figure(key), "%", "80ECO2")
            .bar(Bar::AtMost(1.0))
            .from(from)
            .his()
    };
    let apps = "cadence's day, from app.slice's pressure";
    let desktop = "cadence's day, from session.slice's pressure";
    vec![
        stall(
            "his apps waiting for the CPU {} of the day",
            "pressure.cpu.app",
            apps,
        ),
        stall(
            "his apps stalled for memory {} of the day",
            "pressure.memory.app",
            apps,
        ),
        stall(
            "his desktop waiting for the CPU {} of the day",
            "pressure.cpu.session",
            desktop,
        ),
        stall(
            "his desktop stalled for memory {} of the day",
            "pressure.memory.session",
            desktop,
        ),
    ]
}

/// The day's worst moments in each place he works, as the pain-points
/// detector placed them and health sums them per place (80ECO2, from 13
/// November), so the week's review can hold each place's pains to a
/// reading from his side, each with the owner of that place.
#[must_use]
pub fn places(s: &Sources) -> Vec<Reading> {
    vec![
        lost(
            s,
            "time lost to pi in the day's worst moments {}",
            "pi",
            "85PKB2",
        ),
        lost(
            s,
            "time lost to the launcher in the day's worst moments {}",
            "launcher",
            "Y2YBTS",
        ),
        lost(
            s,
            "time lost to chrome in the day's worst moments {}",
            "chrome",
            "80ECO2",
        ),
    ]
}

/// One place's worst moments. A place health wrote no figure for, on a
/// day it wrote the total, lost nothing.
fn lost(s: &Sources, says: &'static str, place: &str, owner: &'static str) -> Reading {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let total = figure("health.feel.lost-minutes");
    Reading::new(
        says,
        figure(&format!("health.feel.{place}.lost-minutes")).or(total.map(|_| 0.0)),
        " min",
        owner,
    )
    .bar(Bar::AtMost(0.0))
    .from("health's day file, from its feel of his tasks")
    .his()
}

/// The agents he stopped or corrected today, as the pain-points detector
/// signs them on his sessions and health counts them (80ECO2, from 13
/// November). Most of the week's pained lines that named no place were
/// him correcting an agent's judgement (the first blind-spot review,
/// 2026-11-13), so this is collaboration read from his side, and every
/// one is a pain point: the bar is none. 1JQPPC, who routes his pains,
/// owns it.
#[must_use]
pub fn corrected(s: &Sources) -> Reading {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let stopped = figure("health.feel.stopped");
    let corrections = figure("health.feel.corrections");
    let both = stopped.zip(corrections);
    Reading::new(
        "agents he stopped or corrected today {}",
        both.map(|(s, c)| s + c),
        "",
        "1JQPPC",
    )
    .bar(Bar::AtMost(0.0))
    .detail(both.map(|(s, c)| {
        format!(
            "{} stopped mid-turn, {} corrected",
            shown(s, ""),
            shown(c, "")
        )
    }))
    .from("health's day file, from its feel of his tasks")
    .his()
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
            .map(|json| healths(&json)),
    }
}

/// Every figure in health's day file, as `health.<figure>`. Cadence day's
/// own reader keeps only the figures it holds a bar for, which leaves out
/// programs opening, daemons and pi starting, and switch restarts, all of
/// which responsiveness reads; so the axes read the file whole.
fn healths(json: &str) -> BTreeMap<String, f64> {
    let Ok(written) = serde_json::from_str::<serde_json::Value>(json) else {
        return BTreeMap::new();
    };
    written
        .get("figures")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, figure)| Some((format!("health.{key}"), figure.as_f64()?)))
        .collect()
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
    fn only_his_programs_crashes_are_judged_his() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.crashes.his".to_owned(), 1.0),
                ("health.crashes.ours".to_owned(), 69.0),
            ])),
            ..Sources::default()
        };
        let read = crashes(&s);
        assert_eq!(read.words(), "1 crash of his programs today, at most 0");
        assert_eq!(read.detail.as_deref(), Some("ours 69"));
        assert!(read.out() && read.his, "a crash of his is out, on his side");
        assert!(
            crashes(&Sources::default()).figure().is_none(),
            "a day before the split reads unread, not as none"
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
                ("health.launches.pi.p99-ms".to_owned(), 4_000.0),
                ("health.launches.pi.median-ms".to_owned(), 1_800.0),
                ("health.launches.pi.slowest-ms".to_owned(), 28_000.0),
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
                "the slowest pi in a hundred to show its session 4,000 ms, at most 3,000 ms",
                "the slowest restart in a switch 4,200 ms, at most 5,000 ms",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [false, true, false, true, false, true, true, false],
            "only the shutdown, the openings, the daemons and pi are past their bars"
        );
        assert_eq!(
            read[6].detail.as_deref(),
            Some("a median 1,800 ms, the slowest 28,000 ms; timed from pi starting"),
            "pi's start says its spread and that it's timed from our side"
        );
        assert_eq!(
            read[6].owner, "85PKB2",
            "pi's start is the harness's to make quicker"
        );
        assert_eq!(
            read[7].detail.as_deref(),
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
    fn each_place_says_its_worst_moments_and_a_quiet_one_lost_none() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.feel.lost-minutes".to_owned(), 16.3),
                ("health.feel.pi.lost-minutes".to_owned(), 15.5),
                ("health.feel.launcher.lost-minutes".to_owned(), 0.8),
            ])),
            ..Sources::default()
        };
        let places: Vec<(Option<f64>, &str, bool)> =
            places(&s).iter().map(|r| (r.now, r.owner, r.his)).collect();
        assert_eq!(
            places,
            [
                (Some(15.5), "85PKB2", true),
                (Some(0.8), "Y2YBTS", true),
                (Some(0.0), "80ECO2", true),
            ],
            "chrome had no worst moment today, so it lost none rather than reading unread"
        );
    }

    #[test]
    fn the_agents_he_stopped_or_corrected_are_his_side() {
        let s = Sources {
            health: Some(BTreeMap::from([
                ("health.feel.stopped".to_owned(), 2.0),
                ("health.feel.corrections".to_owned(), 3.0),
            ])),
            ..Sources::default()
        };
        let read = corrected(&s);
        assert_eq!(
            read.words(),
            "agents he stopped or corrected today 5, at most 0"
        );
        assert_eq!(
            read.detail.as_deref(),
            Some("2 stopped mid-turn, 3 corrected")
        );
        assert!(read.his && read.out());
        assert_eq!(
            corrected(&Sources::default()).now,
            None,
            "before health counts them, it reads unread, not none"
        );
    }

    #[test]
    fn healths_figures_are_read_whole() {
        let figures = healths(
            r#"{"date": "2026-11-12", "figures": {"boot.seconds": 27.0,
                "launches.pi.p99-ms": 28572, "switch.restart.slowest-ms": 4200}}"#,
        );
        assert_eq!(
            figures,
            BTreeMap::from([
                ("health.boot.seconds".to_owned(), 27.0),
                ("health.launches.pi.p99-ms".to_owned(), 28_572.0),
                ("health.switch.restart.slowest-ms".to_owned(), 4_200.0),
            ]),
            "a figure the axes read was lost because cadence day holds no bar for it"
        );
        assert!(
            healths("not json").is_empty(),
            "an unread file gives no figures"
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

    #[test]
    fn his_apps_stalls_come_from_cadences_day() {
        let day = BTreeMap::from([
            ("pressure.cpu.app".to_owned(), 1.4),
            ("pressure.memory.app".to_owned(), 0.07),
            ("pressure.cpu.session".to_owned(), 0.48),
            ("pressure.memory.session".to_owned(), 1.2),
            ("pressure.cpu.jobs".to_owned(), 13.0),
        ]);
        let read = stalls(Some(&day));
        let words: Vec<String> = read.iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "his apps waiting for the CPU 1.4% of the day, at most 1%",
                "his apps stalled for memory 0.1% of the day, at most 1%",
                "his desktop waiting for the CPU 0.5% of the day, at most 1%",
                "his desktop stalled for memory 1.2% of the day, at most 1%",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [true, false, false, true],
            "only the apps' CPU wait and the desktop's memory stalls are past their bars"
        );
        assert!(
            stalls(None).iter().all(|r| r.figure().is_none()),
            "no day kept yet leaves all four unread"
        );
    }
}
