//! Holds a day against the days before it: a figure past its bar, or
//! worse than its median over them by a fifth and by its floor, and a
//! crash or failure of something not seen in them, each becomes a finding
//! the day it appears. The note says them in a few plain lines.

use serde::{Deserialize, Serialize};

use crate::day::{Day, KINDS, MEASURES, median};

/// How many days before are held against a day.
pub const BEFORE: usize = 7;
/// Fewer days before than this, and a figure is said but not judged
/// against them, since a median of one or two days is that day's luck.
const ENOUGH: usize = 3;
/// How much worse than its median a figure must be to be a regression.
const WORSE: f64 = 1.2;

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
    for measure in MEASURES {
        let Some(&now) = day.figures.get(measure.key) else {
            continue;
        };
        judged.read += 1;
        let past: Vec<f64> = before
            .iter()
            .filter_map(|d| d.figures.get(measure.key).copied())
            .collect();
        let middle = (past.len() >= ENOUGH).then(|| median(&past)).flatten();
        if middle.is_some() {
            judged.held += 1;
        }
        let said = |value: f64| format!("{}{}", round(value), measure.unit);
        if let Some(bar) = measure.bar.filter(|bar| now > *bar) {
            let words = format!(
                "{} was {}, past its bar of {}",
                measure.says,
                said(now),
                said(bar)
            );
            judged.worse.push(Finding {
                key: measure.key.to_owned(),
                bar: Some(bar),
                before: middle,
                now,
                words,
            });
        } else if let Some(middle) = middle {
            if now > middle * WORSE && now - middle >= measure.floor {
                let words = format!(
                    "{} went from {} to {}",
                    measure.says,
                    said(middle),
                    said(now)
                );
                judged.worse.push(Finding {
                    key: measure.key.to_owned(),
                    bar: None,
                    before: Some(middle),
                    now,
                    words,
                });
            } else if now * WORSE < middle && middle - now >= measure.floor {
                let words = format!(
                    "{} went from {} to {}",
                    measure.says,
                    said(middle),
                    said(now)
                );
                judged.better.push(words);
            }
        }
    }
    for kind in KINDS {
        let Some(named) = day.named.get(kind.key) else {
            continue;
        };
        if before.len() < ENOUGH {
            continue;
        }
        for (name, &times) in named {
            let seen = before
                .iter()
                .any(|d| d.named.get(kind.key).is_some_and(|n| n.contains_key(name)));
            if !seen {
                let words = format!("{} {name}, new, {}", kind.says, times_said(times));
                judged.worse.push(Finding {
                    key: format!("{}:{name}", kind.key),
                    bar: None,
                    before: Some(0.0),
                    now: f64::from(times),
                    words,
                });
            }
        }
    }
    judged
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
