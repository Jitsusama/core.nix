//! Holds a day against the days before it: a figure past its bar, or
//! worse than its median over them by a fifth and by its floor, and a
//! crash or failure of something not seen in them, each becomes a finding
//! the day it appears; so does a file every change queues on, once it
//! queues a fifth and its floor more than it did. The note says them in a
//! few plain lines.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::day::{Day, KINDS, WORSE, measures, median};

/// How many days before are held against a day.
pub const BEFORE: usize = 7;
/// Fewer days before than this, and a figure is said but not judged
/// against them, since a median of one or two days is that day's luck.
const ENOUGH: usize = 3;

/// Something the day found that someone should look into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// The measure's key, or the kind and name: `crashes:biome`.
    pub key: String,
    /// The bar it passed, when that's why it's a finding.
    pub bar: Option<f64>,
    /// Its median over the days before, when there were enough.
    pub before: Option<f64>,
    pub now: f64,
    /// What the note says of it.
    pub words: String,
}

/// What a day came to against the days before it.
#[derive(Debug, Default, PartialEq)]
pub struct Judged {
    pub worse: Vec<Finding>,
    /// Figures better than their median by a fifth and their floor.
    pub better: Vec<String>,
    /// How many figures were read, and how many had days enough to judge.
    pub read: usize,
    pub held: usize,
}

/// Judges `day` against `before`, the days ahead of it, any order.
#[must_use]
pub fn judge(day: &Day, before: &[Day]) -> Judged {
    let mut judged = Judged::default();
    for (measure, key, whose, now) in measures().flat_map(|measure| {
        measure
            .read(&day.figures)
            .into_iter()
            .map(move |(key, whose, now)| (measure, key, whose, now))
    }) {
        judged.read += 1;
        let past: Vec<f64> = before
            .iter()
            .filter_map(|d| d.figures.get(key).copied())
            .collect();
        let says = whose.map_or_else(
            || measure.says.to_owned(),
            |whose| format!("{whose}'s {}", measure.says),
        );
        let worse = measure.worse();
        let middle = (past.len() >= ENOUGH).then(|| median(&past)).flatten();
        if middle.is_some() {
            judged.held += 1;
        }
        let said = |value: f64| format!("{}{}", round(value), measure.unit);
        if let Some(bar) = measure.bar.filter(|bar| now > *bar) {
            let words = format!("{says} was {}, past its bar of {}", said(now), said(bar));
            judged.worse.push(Finding {
                key: key.to_owned(),
                bar: Some(bar),
                before: middle,
                now,
                words,
            });
        } else if let Some(middle) = middle {
            if now > middle * worse && now - middle >= measure.floor {
                let words = format!("{says} went from {} to {}", said(middle), said(now));
                judged.worse.push(Finding {
                    key: key.to_owned(),
                    bar: None,
                    before: Some(middle),
                    now,
                    words,
                });
            } else if now * worse < middle && middle - now >= measure.floor {
                let words = format!("{says} went from {} to {}", said(middle), said(now));
                judged.better.push(words);
            }
        }
    }
    judge_names(day, before, &mut judged);
    judged
}

/// Holds the day's names of each kind against the days before: a name
/// not seen in them is new, and for a kind with a floor, a name seen
/// before is worse or better by its count, as a figure is by its median,
/// a day that read the kind without it counting as none.
fn judge_names(day: &Day, before: &[Day], judged: &mut Judged) {
    if before.len() < ENOUGH {
        return;
    }
    for kind in KINDS {
        let Some(today) = day.named.get(kind.key) else {
            continue;
        };
        let past: Vec<&BTreeMap<String, u32>> = before
            .iter()
            .filter_map(|d| d.named.get(kind.key))
            .collect();
        let mut names: BTreeSet<&str> = today.keys().map(String::as_str).collect();
        if kind.floor.is_some() {
            names.extend(past.iter().flat_map(|n| n.keys().map(String::as_str)));
        }
        for name in names {
            let times = today.get(name).copied().unwrap_or(0);
            let counts: Vec<f64> = past
                .iter()
                .map(|n| n.get(name).copied().map_or(0.0, f64::from))
                .collect();
            let key = format!("{}:{name}", kind.key);
            if counts.iter().all(|&count| count == 0.0) {
                let words = format!("{} {name}, new, {}", kind.says, times_said(times));
                judged.worse.push(Finding {
                    key,
                    bar: None,
                    before: Some(0.0),
                    now: f64::from(times),
                    words,
                });
                continue;
            }
            let Some(floor) = kind.floor.filter(|_| counts.len() >= ENOUGH) else {
                continue;
            };
            let Some(middle) = median(&counts) else {
                continue;
            };
            let now = f64::from(times);
            let words = format!(
                "{} {name} went from {} to {}",
                kind.says,
                round(middle),
                round(now)
            );
            if now > middle * WORSE && now - middle >= floor {
                judged.worse.push(Finding {
                    key,
                    bar: None,
                    before: Some(middle),
                    now,
                    words,
                });
            } else if now * WORSE < middle && middle - now >= floor {
                judged.better.push(words);
            }
        }
    }
}

/// The day's note, in at most ten lines: the findings past their date in
/// `late`, what got worse, what got better, and how much was read.
#[must_use]
pub fn note(day: &Day, judged: &Judged, before: usize, late: &[String]) -> String {
    let mut lines = vec![format!(
        "Day to {}: {} worse, {} better, of {} figures read ({} held against {} days before).",
        day.date,
        judged.worse.len(),
        judged.better.len(),
        judged.read,
        judged.held,
        before
    )];
    let room = 8;
    let worse = judged
        .worse
        .iter()
        .map(|f| format!("- Worse: {}.", f.words));
    let better = judged.better.iter().map(|b| format!("- Better: {b}."));
    let late = late.iter().map(|l| format!("- {l}."));
    let all: Vec<String> = late.chain(worse).chain(better).collect();
    let shown = all.len().min(room);
    lines.extend(all.iter().take(shown).cloned());
    if all.len() > shown {
        lines.push(format!(
            "- And {} more in day/{}.json.",
            all.len() - shown,
            day.date
        ));
    }
    lines.join("\n") + "\n"
}

fn times_said(times: u32) -> String {
    if times == 1 {
        "once".to_owned()
    } else {
        format!("{times} times")
    }
}

/// A figure as the note says it: whole past ten, else to one place.
fn round(value: f64) -> String {
    if value.abs() >= 10.0 || value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn day(figures: &[(&str, f64)], crashes: &[&str]) -> Day {
        let mut named = BTreeMap::new();
        named.insert(
            "crashes".to_owned(),
            crashes.iter().map(|c| ((*c).to_owned(), 1)).collect(),
        );
        Day {
            date: "2026-11-12".to_owned(),
            from: 0,
            to: 0,
            figures: figures.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
            named,
        }
    }

    #[test]
    fn a_figure_past_its_bar_is_found_with_no_days_before() {
        let judged = judge(&day(&[("oom.machine", 1.0)], &[]), &[]);
        assert_eq!(
            judged.worse.len(),
            1,
            "a machine-wide kill on the first day wasn't found: {judged:?}"
        );
        assert!(
            judged.worse[0].words.contains("past its bar of 0"),
            "the finding didn't say its bar: {:?}",
            judged.worse[0].words
        );
    }

    #[test]
    fn a_figure_is_worse_only_past_a_fifth_and_its_floor() {
        let before: Vec<Day> = [10.0, 11.0, 12.0]
            .iter()
            .map(|m| day(&[("switch.minutes.median", *m)], &[]))
            .collect();
        let small = judge(&day(&[("switch.minutes.median", 13.5)], &[]), &before);
        assert!(
            small.worse.is_empty(),
            "a move under a fifth of the median was a regression: {small:?}"
        );
        let large = judge(&day(&[("switch.minutes.median", 20.0)], &[]), &before);
        assert_eq!(
            large.worse[0].words, "a switch's median length went from 11 min to 20 min",
            "a switch twice its median wasn't said so"
        );
        let floored = judge(
            &day(&[("pressure.cpu.app", 0.5)], &[]),
            &[
                day(&[("pressure.cpu.app", 0.2)], &[]),
                day(&[("pressure.cpu.app", 0.2)], &[]),
                day(&[("pressure.cpu.app", 0.2)], &[]),
            ],
        );
        assert!(
            floored.worse.is_empty(),
            "a move inside its floor was a regression: {floored:?}"
        );
    }

    #[test]
    fn a_repositorys_count_is_worse_at_any_rise_and_its_time_past_a_fifth() {
        let before: Vec<Day> = (0..3)
            .map(|_| {
                day(
                    &[
                        ("code.core.long-files", 60.0),
                        ("code.core.edit-to-merge", 60.0),
                    ],
                    &[],
                )
            })
            .collect();
        let a_tenth = judge(
            &day(
                &[
                    ("code.core.long-files", 66.0),
                    ("code.core.edit-to-merge", 66.0),
                ],
                &[],
            ),
            &before,
        );
        let found: Vec<&str> = a_tenth.worse.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            found,
            ["code.core.long-files"],
            "a tenth more long files wasn't found, or a tenth longer to merge was"
        );
        let a_third = judge(&day(&[("code.core.edit-to-merge", 80.0)], &[]), &before);
        let said = "core's median time from a change's first edit to its merge went from \
                    60 min to 80 min";
        assert_eq!(
            a_third.worse[0].words, said,
            "a third longer to merge wasn't said so"
        );
    }

    #[test]
    fn too_few_days_before_are_said_and_not_judged() {
        let before = vec![day(&[("switch.minutes.median", 5.0)], &["tec"])];
        let judged = judge(
            &day(&[("switch.minutes.median", 50.0)], &["biome"]),
            &before,
        );
        assert!(
            judged.worse.is_empty(),
            "one day before was enough to judge: {judged:?}"
        );
        assert_eq!(
            (judged.read, judged.held),
            (1, 0),
            "a figure read with one day before wasn't counted read and not held: {judged:?}"
        );
    }

    #[test]
    fn a_crash_of_a_program_not_seen_before_is_new() {
        let before: Vec<Day> = (0..3).map(|_| day(&[], &["tec"])).collect();
        let judged = judge(&day(&[], &["tec", "biome"]), &before);
        let keys: Vec<&str> = judged.worse.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["crashes:biome"],
            "a crash seen before was new, or a new one wasn't"
        );
    }

    fn queued(counts: &[(&str, u32)]) -> Day {
        let mut day = day(&[], &[]);
        day.named.insert(
            "code.queued-on".to_owned(),
            counts.iter().map(|(f, n)| ((*f).to_owned(), *n)).collect(),
        );
        day
    }

    #[test]
    fn a_file_queued_on_every_day_is_worse_past_a_fifth_and_its_floor() {
        let file = "tests/launcher-on-niri-test.py";
        // A day it held nothing counts as none, so its median is 4.
        let before = vec![
            queued(&[(file, 4)]),
            queued(&[(file, 5)]),
            queued(&[]),
            queued(&[(file, 4)]),
        ];
        let more = judge(&queued(&[(file, 11)]), &before);
        let words: Vec<&str> = more.worse.iter().map(|f| f.words.as_str()).collect();
        assert_eq!(
            words,
            ["changes held back or run again on tests/launcher-on-niri-test.py went from 4 to 11"],
            "a file holding nearly three times its usual wasn't said worse"
        );
        assert_eq!(
            more.worse[0].key,
            format!("code.queued-on:{file}"),
            "the finding wasn't keyed by its kind and file"
        );
        let inside = judge(&queued(&[(file, 6)]), &before);
        assert!(
            inside.worse.is_empty(),
            "a rise of two, inside the floor of three, was worse: {inside:?}"
        );
        let gone = judge(&queued(&[]), &before);
        assert_eq!(
            gone.better,
            ["changes held back or run again on tests/launcher-on-niri-test.py went from 4 to 0"],
            "a file that stopped holding changes back wasn't said better"
        );
        let crashing: Vec<Day> = (0..3).map(|_| day(&[], &["tec"])).collect();
        let mut often = day(&[], &[]);
        often.named.insert(
            "crashes".to_owned(),
            BTreeMap::from([("tec".to_owned(), 20)]),
        );
        assert!(
            judge(&often, &crashing).worse.is_empty(),
            "a kind with no floor was judged on its count, not only on new names"
        );
    }

    #[test]
    fn better_figures_are_said_as_better() {
        let before: Vec<Day> = (0..3)
            .map(|_| day(&[("switch.minutes.median", 30.0)], &[]))
            .collect();
        let judged = judge(&day(&[("switch.minutes.median", 10.0)], &[]), &before);
        let note = note(&day(&[], &[]), &judged, before.len(), &[]);
        assert!(
            note.contains("- Better: a switch's median length went from 30 min to 10 min."),
            "a shorter switch wasn't said as better: {note}"
        );
    }

    #[test]
    fn the_note_keeps_to_ten_lines() {
        let worse = (0..20)
            .map(|n| Finding {
                key: format!("k{n}"),
                bar: None,
                before: None,
                now: 1.0,
                words: format!("thing {n}"),
            })
            .collect();
        let judged = Judged {
            worse,
            ..Judged::default()
        };
        let note = note(&day(&[], &[]), &judged, 7, &["Late: x".to_owned()]);
        assert_eq!(
            note.lines().count(),
            10,
            "the note ran past ten lines: {note}"
        );
        assert!(
            note.ends_with("- And 13 more in day/2026-11-12.json.\n"),
            "the note didn't say where the rest are: {note}"
        );
        assert!(
            note.lines().nth(1) == Some("- Late: x."),
            "a finding past its date didn't come first: {note}"
        );
    }
}
