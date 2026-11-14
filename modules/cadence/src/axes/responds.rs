//! How fast the machine answers Joel, as health and cadence day already
//! count it: boots, shutdowns, programs opening and ending, daemons and pi
//! starting, a switch's restarts, his apps' stalls, the minutes the day's
//! worst moments cost him in all and in each place, and pi's worst typing.
//! These change together when docs/design.md's budgets do, so they keep a
//! file apart from the other readings counted.rs takes from the same
//! sources; the launcher's own keys and drawers are launcher.rs's.

use std::collections::BTreeMap;

use super::counted::Sources;
use super::{Bar, Reading, shown};

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

/// A key typed into pi, at its worst window of the day: the wait one key
/// in a hundred passed, held to the design's budget for a key to what it
/// changed at the 99th, 100 ms. pi-keys models the keys rather than
/// timing his own, so it's a stand-in.
#[must_use]
pub fn typing(s: &Sources) -> Reading {
    Reading::new(
        "the slowest key in a hundred typed into pi {}, in its worst window today",
        s.health
            .as_ref()
            .and_then(|f| f.get("health.typing.worst-p99-ms").copied()),
        " ms",
        "85PKB2",
    )
    .bar(Bar::AtMost(100.0))
    .from("health's day file, from pi-keys' modelled keys")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pis_worst_typing_is_held_to_the_design_from_our_side() {
        let s = Sources {
            health: Some(BTreeMap::from([(
                "health.typing.worst-p99-ms".to_owned(),
                1_208.2,
            )])),
            ..Sources::default()
        };
        let keys = typing(&s);
        assert_eq!(
            keys.words(),
            "the slowest key in a hundred typed into pi 1,208 ms, in its worst window today, \
             at most 100 ms"
        );
        assert!(
            keys.out() && !keys.his,
            "a second's key was in spec, or read from his side"
        );
        assert_eq!(
            typing(&Sources::default()).words(),
            "the slowest key in a hundred typed into pi, in its worst window today: no reading yet"
        );
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
