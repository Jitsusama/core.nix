//! The launcher's speed, read from health's day file, which reads it from
//! the lines the shell writes as Joel uses it: each timed from his chord or
//! key to niri presenting the frame that showed it, so all are his side
//! (Y2YBTS and 80ECO2, 2026-11-12). Each is held to docs/design.md's
//! Latency Budgets as health holds it: an opening shown in 50 ms and
//! taking his keys in 30 ms, and a key, a daemon's change or a row's
//! drawer drawn in 50 ms at the median and 100 ms at the 99th.
//!
//! Openings are timed one at a time, so their 99th is exact and judged.
//! Keys, changes and drawers are summed in batches, whose 99ths can't be
//! pooled: health keeps the slowest batch's, only a ceiling. So their
//! median is judged, and their 99th is said and left unread. Since
//! 12 November the shell keeps each key's time, and health pools them
//! into an exact 99th, saying how many keys it pooled (80ECO2): a day
//! with that count judges the keys' 99th; one without only says it.

use super::counted::Sources;
use super::{Bar, Reading, shown};

const FROM: &str = "health's day file, from the shell's presented frames";

/// The launcher's readings: its openings shown and taking keys at the
/// 99th percentile, its keys at the median and the 99th, and a daemon's
/// changes and a row's drawers at the median.
#[must_use]
pub fn readings(s: &Sources) -> Vec<Reading> {
    let figure = |key: String| s.health.as_ref().and_then(|f| f.get(&key).copied());
    let late = |part: &str, bar: f64| {
        figure(format!("health.launcher.{part}.late-percent")).map(|late| {
            format!(
                "{} took longer than {}",
                shown(late, "%"),
                shown(bar, " ms")
            )
        })
    };
    let ceiling = |part: &str| {
        figure(format!("health.launcher.{part}.p99-ms")).map(|ceiling| {
            format!(
                "the 99th at most {}, the slowest batch's, since batches can't be pooled",
                shown(ceiling, " ms")
            )
        })
    };
    let opening = |part: &'static str, says: &'static str, bar: f64| {
        let median = figure(format!("health.launcher.opening.{part}.median-ms"));
        Reading::new(
            says,
            figure(format!("health.launcher.opening.{part}.p99-ms")),
            " ms",
            // The opening is H6EDLC's; the keys once it's open are Y2YBTS's
            // (Y2YBTS, 2026-11-12).
            "H6EDLC",
        )
        .bar(Bar::AtMost(bar))
        .detail(median.map(|median| {
            let late = late(&format!("opening.{part}"), bar).unwrap_or_default();
            format!("a median {}; {late}", shown(median, " ms"))
        }))
        .from(FROM)
        .his()
    };
    // Keys and drawers are Y2YBTS's; a daemon's changes reach the screen at
    // the pace ZINAWZ's refresh sets, so they're its (Y2YBTS, 2026-11-12).
    let middle = |part: &'static str, says: &'static str, owner: &'static str| {
        Reading::new(
            says,
            figure(format!("health.launcher.{part}.median-ms")),
            " ms",
            owner,
        )
        .bar(Bar::AtMost(50.0))
        .detail(late(part, 50.0).map(|late| match ceiling(part) {
            Some(ceiling) if part != "keys" => format!("{late}; {ceiling}"),
            _ => late,
        }))
        .from(FROM)
        .his()
    };
    vec![
        opening(
            "usable",
            "the slowest opening in a hundred to take his keys {} after the chord",
            30.0,
        ),
        opening(
            "shown",
            "the slowest opening in a hundred to show the launcher {} after the chord",
            50.0,
        ),
        middle(
            "keys",
            "the launcher's middle key {} today, from his key to his screen",
            "Y2YBTS",
        ),
        slowest_key(s),
        middle(
            "changes",
            "the launcher's middle change from a daemon {} to his screen today",
            "ZINAWZ",
        ),
        middle(
            "drawers",
            "the launcher's middle drawer {} from his key today",
            "Y2YBTS",
        ),
    ]
}

/// How often he typed into the launcher and left with nothing picked, as
/// health counts it from the trials (80ECO2, 12 November): his side, and
/// polish and fit's first reading, since a search that finds nothing is
/// the launcher not fitting what he asked of it. The bar of 5% is ours,
/// set on the day the count began, leaving room for a search he changed
/// his mind about; it only tightens.
#[must_use]
pub fn missed(s: &Sources) -> Reading {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let typed = figure("health.launcher.searches.typed").filter(|n| *n > 0.0);
    let missed = figure("health.launcher.searches.missed");
    Reading::new(
        "the launcher's searches he left with nothing picked today {}",
        typed
            .zip(missed)
            .map(|(typed, missed)| 100.0 * missed / typed),
        "%",
        "Y2YBTS",
    )
    .bar(Bar::AtMost(5.0))
    .detail(
        typed
            .zip(missed)
            .map(|(typed, missed)| format!("{} of {}", shown(missed, ""), shown(typed, ""))),
    )
    .from("health's day file, from the launcher's trials")
    .his()
}

/// The keys' 99th: exact, and judged, where health pooled every key's
/// time; otherwise only the slowest batch's, said and left unread.
fn slowest_key(s: &Sources) -> Reading {
    let figure = |key: &str| s.health.as_ref().and_then(|f| f.get(key).copied());
    let p99 = figure("health.launcher.keys.p99-ms");
    let pooled = figure("health.launcher.keys.pooled").filter(|n| *n > 0.0);
    let reading = |p99| {
        Reading::new(
            "the launcher's slowest key in a hundred today {}",
            p99,
            " ms",
            "Y2YBTS",
        )
        .bar(Bar::AtMost(100.0))
        .from(FROM)
        .his()
    };
    match pooled {
        Some(keys) => reading(p99).detail(Some(format!(
            "of {} keys, each timed from his key to his screen",
            shown(keys, "")
        ))),
        None => reading(None).detail(p99.map(|ceiling| {
            format!(
                "the 99th at most {}, the slowest batch's, since batches can't be pooled; \
                 an exact figure waits on health pooling each key's time",
                shown(ceiling, " ms")
            )
        })),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn today() -> Sources {
        let figures = [
            ("opening.usable", 249.0, 7_473.0, 100.0),
            ("opening.shown", 45.0, 2_904.0, 36.8),
            ("keys", 52.0, 1_614.0, 46.3),
            ("changes", 50.0, 2_282.0, 57.4),
            ("drawers", 70.0, 453.0, 79.3),
        ];
        Sources {
            health: Some(
                figures
                    .iter()
                    .flat_map(|(part, median, p99, late)| {
                        [
                            (format!("health.launcher.{part}.median-ms"), *median),
                            (format!("health.launcher.{part}.p99-ms"), *p99),
                            (format!("health.launcher.{part}.late-percent"), *late),
                        ]
                    })
                    .collect::<BTreeMap<_, _>>(),
            ),
            ..Sources::default()
        }
    }

    #[test]
    fn every_part_of_the_launcher_is_held_to_its_budget() {
        let read = readings(&today());
        let words: Vec<String> = read.iter().map(Reading::words).collect();
        assert_eq!(
            words,
            [
                "the slowest opening in a hundred to take his keys 7,473 ms after the chord, \
                 at most 30 ms",
                "the slowest opening in a hundred to show the launcher 2,904 ms after the chord, \
                 at most 50 ms",
                "the launcher's middle key 52 ms today, from his key to his screen, at most 50 ms",
                "the launcher's slowest key in a hundred today: no reading yet",
                "the launcher's middle change from a daemon 50 ms to his screen today, \
                 at most 50 ms",
                "the launcher's middle drawer 70 ms from his key today, at most 50 ms",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [true, true, true, false, false, true]
        );
        assert!(
            read.iter().all(|r| r.his),
            "all timed from his key to his screen"
        );
        assert_eq!(
            read.iter().map(|r| r.owner).collect::<Vec<_>>(),
            ["H6EDLC", "H6EDLC", "Y2YBTS", "Y2YBTS", "ZINAWZ", "Y2YBTS"]
        );
        assert_eq!(
            read[0].detail.as_deref(),
            Some("a median 249 ms; 100% took longer than 30 ms")
        );
        assert_eq!(
            read[2].detail.as_deref(),
            Some("46% took longer than 50 ms"),
            "a key's ceiling is said on its own line"
        );
        assert_eq!(
            read[3].detail.as_deref(),
            Some(
                "the 99th at most 1,614 ms, the slowest batch's, since batches can't be \
                 pooled; an exact figure waits on health pooling each key's time"
            ),
            "a ceiling is said, never judged as the 99th itself"
        );
        assert_eq!(
            read[5].detail.as_deref(),
            Some(
                "79% took longer than 50 ms; the 99th at most 453 ms, the slowest batch's, \
                 since batches can't be pooled"
            )
        );
        assert!(
            readings(&Sources::default())
                .iter()
                .all(|r| r.figure().is_none()),
            "a day with no launcher reads as unread, not as fast"
        );
    }

    #[test]
    fn a_search_left_with_nothing_picked_counts_against_polish() {
        let mut s = today();
        let figures = s.health.as_mut().expect("today's figures");
        figures.insert("health.launcher.searches.typed".to_owned(), 80.0);
        figures.insert("health.launcher.searches.missed".to_owned(), 6.0);
        let read = missed(&s);
        assert_eq!(
            read.words(),
            "the launcher's searches he left with nothing picked today 7.5%, at most 5%"
        );
        assert_eq!(read.detail.as_deref(), Some("6 of 80"));
        assert!(read.out() && read.his, "a miss is his, and past 5% is out");
        assert!(
            missed(&today()).figure().is_none(),
            "a day before the count reads unread, not as none missed"
        );
    }

    #[test]
    fn the_keys_99th_is_judged_once_health_pooled_every_key() {
        let mut pooled = today();
        let figures = pooled.health.as_mut().expect("today's figures");
        figures.insert("health.launcher.keys.p99-ms".to_owned(), 141.0);
        figures.insert("health.launcher.keys.pooled".to_owned(), 2_210.0);
        let read = slowest_key(&pooled);
        assert_eq!(
            read.words(),
            "the launcher's slowest key in a hundred today 141 ms, at most 100 ms"
        );
        assert!(read.out(), "an exact 99th past its budget is out");
        assert_eq!(
            read.detail.as_deref(),
            Some("of 2,210 keys, each timed from his key to his screen")
        );
        figures_without_pooling(&mut pooled);
        assert!(
            slowest_key(&pooled).figure().is_none(),
            "a count of none is no pooling"
        );
    }

    fn figures_without_pooling(s: &mut Sources) {
        if let Some(figures) = s.health.as_mut() {
            figures.insert("health.launcher.keys.pooled".to_owned(), 0.0);
        }
    }
}
