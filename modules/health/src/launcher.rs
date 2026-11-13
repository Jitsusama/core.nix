//! The launcher's speed over the day, from the lines the shell writes as
//! Joel uses it: how long a key, a daemon's change, a row's drawer and an
//! opening took to reach the screen, against docs/design.md's Latency
//! Budgets. These are his side's figures, what he waited for as the shell
//! timed it.
//!
//! The shell sums its keys in batches, each with its median, slowest, p99
//! and how many were within the bar, so the day's p99 can't be told
//! exactly. The largest batch's p99 is never below it, though: by nearest
//! rank at most one in a hundred of each batch is past its p99, so at most
//! one in a hundred of the day's are past the largest. That is the p99 the
//! day keeps, a ceiling, and exact where each was timed alone or where the
//! launcher's trials gave every key's time (`trials.rs` pools them).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What the shell writes its journal as.
pub const IDENTIFIER: &str = "quickshell";

/// What `journalctl --grep` follows for these lines.
pub const FOLLOWED: &str = "launcher: (opening (presented|usable) |.* presented a median )";

/// The day's launcher, a part at a time, read again from the journal on
/// each start, so never resumed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Launcher {
    /// Keys typed, from the key to the screen.
    pub keys: Timed,
    /// A daemon's changes, from its line to the screen.
    pub changes: Timed,
    /// Rows' drawers, from the key that landed on the row.
    pub drawers: Timed,
    /// Openings, from the chord to the launcher shown.
    pub shown: Timed,
    /// Openings, from the chord to the launcher taking keys.
    pub usable: Timed,
    /// Searches, from the launcher's trials.
    pub searches: Searches,
}

/// The day's searches: the openings Joel typed a letter in, and those he
/// left with nothing picked, as the shell's trials say. What he sought is
/// never kept here, only counted.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Searches {
    /// Openings he typed a letter in.
    pub typed: u64,
    /// Of those, the ones he closed with nothing picked.
    pub missed: u64,
}

impl Searches {
    /// The share of searches he left with nothing picked, in percent.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's count of searches is far below where a float loses whole numbers"
    )]
    pub fn missed_percent(&self) -> f64 {
        if self.typed == 0 {
            return 0.0;
        }
        100.0 * self.missed as f64 / self.typed as f64
    }
}

/// One part of the launcher, as the day names and says it.
pub struct Part<'a> {
    /// The name its figures take.
    pub name: &'static str,
    /// What they're called when said.
    pub words: &'static str,
    /// What one is called.
    pub one: &'static str,
    /// Its budget's median, in milliseconds.
    pub bar_ms: u64,
    /// How long it took.
    pub timed: &'a Timed,
}

/// How long one part took over the day.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timed {
    /// Each batch's median in milliseconds, counted once for each it
    /// timed, so the day's median weighs every key alike.
    pub medians: BTreeMap<u64, u64>,
    /// How many were timed.
    pub count: u64,
    /// How many batches the shell said them in.
    pub batches: u64,
    /// How many went past the bar.
    pub late: u64,
    /// The slowest, in milliseconds.
    pub slowest_ms: u64,
    /// The largest of the batches' p99s.
    pub p99_ms: u64,
    /// Each time the trials gave, in milliseconds, counted once for each
    /// it was, so the day's p99 can be told exactly.
    pub pooled: BTreeMap<u64, u64>,
}

impl Timed {
    fn add(&mut self, count: u64, median: u64, slowest: u64, p99: u64, within: u64) {
        *self.medians.entry(median).or_default() += count;
        self.count += count;
        self.batches += 1;
        self.late += count - within.min(count);
        self.slowest_ms = self.slowest_ms.max(slowest);
        self.p99_ms = self.p99_ms.max(p99);
    }

    /// Pool one time the trials gave, in milliseconds.
    pub fn pool(&mut self, took: u64) {
        *self.pooled.entry(took).or_default() += 1;
    }

    /// How many times were pooled.
    #[must_use]
    pub fn pooled_count(&self) -> u64 {
        self.pooled.values().sum()
    }

    /// Whether the p99 is exact: from every one pooled, or each timed alone.
    #[must_use]
    pub fn exact(&self) -> bool {
        !self.pooled.is_empty() || self.batches == self.count
    }

    /// The day's median, in milliseconds.
    #[must_use]
    pub fn median(&self) -> Option<u64> {
        ranked(&self.medians, self.count.div_ceil(2))
    }

    /// The day's p99, in milliseconds: by nearest rank of those pooled, or
    /// where each was timed alone, and otherwise the ceiling the batches
    /// give.
    #[must_use]
    pub fn p99(&self) -> Option<u64> {
        if !self.pooled.is_empty() {
            ranked(&self.pooled, (self.pooled_count() * 99).div_ceil(100))
        } else if self.count == 0 {
            None
        } else if self.batches == self.count {
            ranked(&self.medians, (self.count * 99).div_ceil(100))
        } else {
            Some(self.p99_ms)
        }
    }

    /// The share past the bar, in percent.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's count of keys is far below where a float loses whole numbers"
    )]
    pub fn late_percent(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        100.0 * self.late as f64 / self.count as f64
    }
}

impl Launcher {
    /// Take one of the shell's lines, saying whether it was one of these.
    pub fn took(&mut self, message: &str) -> bool {
        let Some((_, said)) = message.split_once("launcher: ") else {
            return false;
        };
        if let Some(rest) = said.strip_prefix("opening presented ") {
            return once(&mut self.shown, rest, SHOWN_MS);
        }
        if let Some(rest) = said.strip_prefix("opening usable ") {
            return once(&mut self.usable, rest, USABLE_MS);
        }
        let Some((before, after)) = said.split_once(" presented a median ") else {
            return false;
        };
        let part = if before.ends_with(" keys") || before.ends_with(" key") {
            &mut self.keys
        } else if before.contains(" change in ") || before.contains(" changes in ") {
            &mut self.changes
        } else if before.contains(" drawer in ") || before.contains(" drawers in ") {
            &mut self.drawers
        } else {
            return false;
        };
        let (Some(count), Some(median)) = (number(before), number(after)) else {
            return false;
        };
        let after_words = |words: &str| after.split_once(words).and_then(|(_, rest)| number(rest));
        let slowest = after_words(" the slowest ").unwrap_or(median);
        // A batch said before the shell gave its p99 is held to its
        // slowest, which is never below it.
        let p99 = after_words(" the p99 ").unwrap_or(slowest);
        // The count just before " within ", so a figure the shell writes
        // between, such as its p99, is no count.
        let within = after
            .split_once(" within ")
            .and_then(|(before, _)| before.rsplit(' ').next())
            .and_then(|count| count.parse().ok())
            .unwrap_or(count);
        part.add(count, median, slowest, p99, within);
        true
    }

    /// Each part, in the order they're said.
    #[must_use]
    pub fn parts(&self) -> [Part<'_>; 5] {
        let part = |name, (one, words), bar_ms, timed| Part {
            name,
            words,
            one,
            bar_ms,
            timed,
        };
        [
            part("keys", ("key", "keys"), KEY_MS, &self.keys),
            part("changes", ("change", "changes"), KEY_MS, &self.changes),
            part("drawers", ("drawer", "drawers"), KEY_MS, &self.drawers),
            part(
                "opening.shown",
                ("opening shown", "openings shown"),
                SHOWN_MS,
                &self.shown,
            ),
            part(
                "opening.usable",
                ("opening taking keys", "openings taking keys"),
                USABLE_MS,
                &self.usable,
            ),
        ]
    }

    /// A line for each part timed, as `health launcher` shows it, and one
    /// for the day's searches when he typed any.
    #[must_use]
    pub fn said(&self) -> Vec<String> {
        let mut said: Vec<String> = self
            .parts()
            .into_iter()
            .filter(|part| part.timed.count > 0)
            .map(|part| {
                let timed = part.timed;
                let late = match timed.late {
                    0 => format!("none past {} ms", part.bar_ms),
                    late => format!(
                        "{late} past {} ms ({})",
                        part.bar_ms,
                        percent(timed.late_percent())
                    ),
                };
                let ceiling = if timed.exact() { "" } else { "at most " };
                // The pooled are fewer than those timed while the trials
                // only began to give each key on 12 November.
                let pooled = timed.pooled_count();
                let over = if pooled > 0 && pooled != timed.count {
                    format!(" over {pooled} pooled")
                } else {
                    String::new()
                };
                format!(
                    "{} {}: a median {} ms, the p99 {ceiling}{} ms{over}, the slowest {} ms; \
                     {late}",
                    timed.count,
                    if timed.count == 1 {
                        part.one
                    } else {
                        part.words
                    },
                    timed.median().unwrap_or_default(),
                    timed.p99().unwrap_or_default(),
                    timed.slowest_ms
                )
            })
            .collect();
        let searches = &self.searches;
        if searches.typed > 0 {
            said.push(format!(
                "{} {} typed: {} left with nothing picked ({})",
                searches.typed,
                if searches.typed == 1 {
                    "search"
                } else {
                    "searches"
                },
                searches.missed,
                percent(searches.missed_percent())
            ));
        }
        said
    }

    /// The day's launcher in a line, as `health` opens with it: how many
    /// of what Joel did went past its bar, and the part most did in.
    #[must_use]
    pub fn summed(&self) -> Option<String> {
        let parts = self.parts();
        let count: u64 = parts.iter().map(|part| part.timed.count).sum();
        if count == 0 {
            return None;
        }
        let late: u64 = parts.iter().map(|part| part.timed.late).sum();
        let worst = parts
            .iter()
            .filter(|part| part.timed.late > 0)
            .max_by(|a, b| a.timed.late_percent().total_cmp(&b.timed.late_percent()));
        Some(match worst {
            None => format!("all {count} timed within their budgets"),
            Some(worst) => format!(
                "{late} of {count} timed past their budgets; worst {}, {} of {} past {} ms, \
                 the slowest {} ms",
                worst.words,
                worst.timed.late,
                worst.timed.count,
                worst.bar_ms,
                worst.timed.slowest_ms
            ),
        })
    }
}

/// A key's budget, from the key to what it changed drawn.
const KEY_MS: u64 = 50;

/// An opening's, from the chord to the launcher shown, as cadence holds it.
const SHOWN_MS: u64 = 50;

/// An opening's, from the chord to the launcher taking the keys.
const USABLE_MS: u64 = 30;

/// The value at `rank` of times counted in `each`, counting from one.
fn ranked(each: &BTreeMap<u64, u64>, rank: u64) -> Option<u64> {
    let mut seen = 0;
    each.iter().find_map(|(&took, &count)| {
        seen += count;
        (seen >= rank.max(1)).then_some(took)
    })
}

/// One opening, from "N ms from the chord".
fn once(part: &mut Timed, rest: &str, bar: u64) -> bool {
    let Some(took) = number(rest) else {
        return false;
    };
    part.add(1, took, took, took, u64::from(took <= bar));
    true
}

/// The first whole number in `text`.
fn number(text: &str) -> Option<u64> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|part| !part.is_empty())?
        .parse()
        .ok()
}

/// A share, to a tenth under ten percent and whole above.
pub(crate) fn percent(share: f64) -> String {
    if share < 10.0 {
        format!("{share:.1}%")
    } else {
        format!("{share:.0}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Day;
    use crate::day::tests::entry;

    /// The shell's lines as its journal holds them, colours and all.
    const SAID: [&str; 9] = [
        "\u{1b}[32m  INFO\u{1b}[97m qml\u{1b}[0m: launcher: opening presented 65 ms from the chord",
        "  INFO qml: launcher: opening usable 312 ms from the chord, its longest stall 105 ms",
        "  INFO qml: launcher: opening usable 20 ms from the chord, its longest stall 5 ms",
        "  INFO qml: launcher: 3 keys presented a median 60 ms from the key, the slowest 71 ms, \
         0 within 50 ms: a median 2 ms before the shell heard it, 10 in the shell",
        "  INFO qml: launcher: 1 keys presented a median 20 ms from the key, the slowest 20 ms, \
         the p99 20 ms, 1 within 50 ms",
        "  INFO qml: launcher: 3 keys landed a median 79 ms from the key, the slowest 89 ms, \
         0 within 50 ms",
        "  INFO qml: launcher: 2 changes in wifi presented a median 12 ms from the line, the \
         slowest 13 ms, the p99 13 ms, 2 within 50 ms, from rows 2: a median 0 ms before",
        "  INFO qml: launcher: 10 drawers in home presented a median 42 ms from the key, the \
         slowest 134 ms, the p99 134 ms, 6 within 50 ms: a median 20 ms before it followed",
        "  INFO qml: launcher: home and 7 places kept as the shell started, 7054 rows made",
    ];

    fn read() -> Launcher {
        let mut launcher = Launcher::default();
        for line in SAID {
            launcher.took(line);
        }
        launcher
    }

    #[test]
    fn the_shells_lines_are_timed_a_part_each() {
        let launcher = read();
        assert_eq!(
            (
                launcher.keys.count,
                launcher.keys.late,
                launcher.keys.slowest_ms
            ),
            (4, 3, 71),
            "the keys presented were misread, or the keys landed were read too"
        );
        assert_eq!(
            (launcher.keys.median(), launcher.keys.p99()),
            (Some(60), Some(71)),
            "the keys' median doesn't weigh each key, or their p99 is below a batch's"
        );
        assert_eq!(
            (
                launcher.drawers.count,
                launcher.drawers.late,
                launcher.changes.count
            ),
            (10, 4, 2),
            "a drawer or a change was misread"
        );
        assert_eq!(
            (
                launcher.shown.slowest_ms,
                launcher.usable.late,
                launcher.usable.p99()
            ),
            (65, 1, Some(312)),
            "an opening was misread, or the p99 of openings timed alone isn't by rank"
        );
        assert!(
            !Launcher::default().took(SAID[8]),
            "a line that times nothing was taken"
        );
    }

    #[test]
    fn the_keys_p99_is_exact_once_the_trials_gave_every_key() {
        let mut launcher = read();
        for took in 1..=100 {
            launcher.keys.pool(took);
        }
        assert_eq!(
            launcher.keys.p99(),
            Some(99),
            "the keys' p99 is still the batches' ceiling though every key was pooled"
        );
        assert!(
            launcher.said()[0].contains(", the p99 99 ms over 100 pooled,"),
            "an exact p99 was said as a ceiling, or without how many it's over: {:?}",
            launcher.said()
        );
    }

    #[test]
    fn the_searches_left_with_nothing_picked_are_said_with_their_share() {
        let launcher = Launcher {
            searches: Searches {
                typed: 20,
                missed: 11,
            },
            ..Launcher::default()
        };
        assert_eq!(
            launcher.said(),
            ["20 searches typed: 11 left with nothing picked (55%)"],
            "the day's searches weren't said, or their misses' share was wrong"
        );
    }

    #[test]
    fn the_day_keeps_the_shells_timings_apart_from_its_warnings() {
        let mut day = Day::new("2026-11-12");
        day.take(&entry(IDENTIFIER, 6, SAID[4]));
        day.take(&entry(IDENTIFIER, 4, "launcher: a row has no act"));
        assert_eq!(
            (day.launcher.keys.count, day.launcher.keys.late),
            (1, 0),
            "the shell's timing of keys wasn't kept as the launcher's"
        );
        assert_eq!(
            day.things.get(IDENTIFIER).map(|thing| thing.lines),
            Some(1),
            "the shell's warning wasn't counted as its own, or its timing was"
        );
    }

    #[test]
    fn each_part_says_how_many_went_past_its_budget() {
        let launcher = read();
        assert_eq!(
            launcher.said(),
            [
                "4 keys: a median 60 ms, the p99 at most 71 ms, the slowest 71 ms; \
                 3 past 50 ms (75%)",
                "2 changes: a median 12 ms, the p99 at most 13 ms, the slowest 13 ms; \
                 none past 50 ms",
                "10 drawers: a median 42 ms, the p99 at most 134 ms, the slowest 134 ms; \
                 4 past 50 ms (40%)",
                "1 opening shown: a median 65 ms, the p99 65 ms, the slowest 65 ms; \
                 1 past 50 ms (100%)",
                "2 openings taking keys: a median 20 ms, the p99 312 ms, the slowest 312 ms; \
                 1 past 30 ms (50%)",
            ],
            "a part was said wrongly"
        );
        assert_eq!(
            launcher.summed().as_deref(),
            Some(
                "9 of 19 timed past their budgets; worst openings shown, 1 of 1 past 50 ms, \
                 the slowest 65 ms"
            ),
            "the day's launcher wasn't summed"
        );
    }
}
