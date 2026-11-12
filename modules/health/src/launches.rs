//! How long programs took to open and to end over the day, from the lines
//! the windows daemon writes as niri opens and closes their windows
//! (modules/windows, src/timed.rs): a program's first window after its
//! process started, and its process ending after its last window went.
//! Beside them, how long each of our daemons took to be ready, from the
//! line each says once it is, such as dictation's "ready in 1065 ms" or
//! meaning's "bge loaded on the NPU in 688 ms".
//!
//! Both are our side's figures, stand-ins for what Joel waits on: an
//! opening is timed from the process's start, not from the key that asked
//! for it, and an end from the window gone, since niri never says the key
//! that asked a window to close.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::launcher::percent;

/// What the windows daemon writes its journal as.
pub const IDENTIFIER: &str = "windows";

/// What `journalctl --grep` follows for these lines.
pub const FOLLOWED: &str = "^(opened .* ms after its process started|closed .*; its process )";

/// An opening's budget, from the process's start to its first window, as
/// docs/design.md's Latency Budgets hold it.
pub const OPENED_MS: u64 = 300;

/// An end's, from the last window gone to the process ended.
pub const ENDED_MS: u64 = 200;

/// What `journalctl --grep` follows for a daemon saying it's ready, from
/// any program: a line saying it's ready or loaded, then how long it took.
pub const READY_FOLLOWED: &str = r"\b(ready|loaded)\b.* in [0-9]+ ms\b";

/// A daemon's budget from starting to ready, as docs/design.md's Latency
/// Budgets hold it for a daemon serving again.
pub const READY_MS: u64 = 1_000;

/// The day's openings and ends, read again from the journal on each
/// start, so never resumed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Launches {
    /// Each program's openings, in milliseconds, as they came.
    pub opened: BTreeMap<String, Vec<u64>>,
    /// Each program's ends after its last window, in milliseconds.
    pub ended: BTreeMap<String, Vec<u64>>,
    /// How often each program still ran when the daemon stopped waiting.
    pub outlived: BTreeMap<String, u64>,
    /// How long each daemon took to be ready, as it said, in milliseconds.
    pub ready: BTreeMap<String, Vec<u64>>,
}

/// How one of the two went over the day.
#[derive(Debug, PartialEq, Eq)]
pub struct Spread<'a> {
    /// How many were timed.
    pub count: u64,
    /// The median, in milliseconds.
    pub median_ms: u64,
    /// The p99, by nearest rank, in milliseconds.
    pub p99_ms: u64,
    /// The slowest, in milliseconds, and the program it was.
    pub slowest: (u64, &'a str),
    /// How many went past the budget.
    pub late: u64,
}

impl Spread<'_> {
    /// The share past the budget, in percent.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's count of openings is far below where a float loses whole numbers"
    )]
    pub fn late_percent(&self) -> f64 {
        100.0 * self.late as f64 / self.count.max(1) as f64
    }
}

impl Launches {
    /// Take one line `thing` said, saying whether it was one of these: an
    /// opening or end from the windows daemon, or any program's ready.
    pub fn took(&mut self, thing: &str, message: &str) -> bool {
        if thing != IDENTIFIER {
            let Some(ms) = ready_in(message) else {
                return false;
            };
            self.ready.entry(thing.to_owned()).or_default().push(ms);
            return true;
        }
        if let Some(rest) = message.strip_prefix("opened ") {
            let Some((app, ms)) = rest
                .strip_suffix(" ms after its process started")
                .and_then(|said| said.rsplit_once(' '))
            else {
                return false;
            };
            let Ok(ms) = ms.parse() else { return false };
            self.opened.entry(app.to_owned()).or_default().push(ms);
            return true;
        }
        let Some((app, how)) = message
            .strip_prefix("closed ")
            .and_then(|rest| rest.split_once("; its process "))
        else {
            return false;
        };
        if how.starts_with("was still running") {
            *self.outlived.entry(app.to_owned()).or_default() += 1;
            return true;
        }
        let Some(ms) = how
            .strip_prefix("ended ")
            .and_then(|rest| rest.strip_suffix(" ms later"))
            .and_then(|ms| ms.parse().ok())
        else {
            return false;
        };
        self.ended.entry(app.to_owned()).or_default().push(ms);
        true
    }

    /// The openings over the day, once any were timed.
    #[must_use]
    pub fn openings(&self) -> Option<Spread<'_>> {
        spread(&self.opened, OPENED_MS)
    }

    /// The ends over the day, once any were timed.
    #[must_use]
    pub fn ends(&self) -> Option<Spread<'_>> {
        spread(&self.ended, ENDED_MS)
    }

    /// How long daemons took to be ready over the day, once any said.
    #[must_use]
    pub fn readies(&self) -> Option<Spread<'_>> {
        spread(&self.ready, READY_MS)
    }

    /// How many ends the daemon stopped waiting for.
    #[must_use]
    pub fn outlived(&self) -> u64 {
        self.outlived.values().sum()
    }

    /// The day's openings and ends in a line, as `health` says them.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(opened) = self.openings() {
            parts.push(said(&opened, ("opening", "openings"), OPENED_MS));
        }
        if let Some(ended) = self.ends() {
            parts.push(said(&ended, ("end", "ends"), ENDED_MS));
        }
        if let Some(ready) = self.readies() {
            parts.push(said(&ready, ("daemon ready", "daemons ready"), READY_MS));
        }
        match self.outlived() {
            0 => {}
            1 => parts.push("1 program still ran 30 s after its last window".to_owned()),
            n => parts.push(format!(
                "{n} programs still ran 30 s after their last window"
            )),
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// Each program's openings and ends, the slowest first, as `health
    /// launches` shows them.
    #[must_use]
    pub fn by_program(&self) -> Vec<String> {
        let mut names: Vec<&String> = self
            .opened
            .keys()
            .chain(self.ended.keys())
            .chain(self.outlived.keys())
            .chain(self.ready.keys())
            .collect();
        names.sort();
        names.dedup();
        let slowest = |name: &String| {
            let most = |taken: &BTreeMap<String, Vec<u64>>| {
                taken.get(name).and_then(|all| all.iter().max().copied())
            };
            most(&self.opened)
                .max(most(&self.ended))
                .max(most(&self.ready))
                .unwrap_or_default()
        };
        names.sort_by_key(|name| std::cmp::Reverse(slowest(name)));
        names
            .into_iter()
            .map(|name| {
                let one = |taken: &BTreeMap<String, Vec<u64>>, words| {
                    taken.get(name).map(|all| {
                        let most = all.iter().max().copied().unwrap_or_default();
                        format!("{} {words}, the slowest {most} ms", all.len())
                    })
                };
                let mut parts: Vec<String> = [
                    one(&self.opened, "opened"),
                    one(&self.ended, "ended"),
                    one(&self.ready, "ready"),
                ]
                .into_iter()
                .flatten()
                .collect();
                if let Some(times) = self.outlived.get(name) {
                    parts.push(format!("{times} still running 30 s on"));
                }
                format!("{name:<24} {}", parts.join("; "))
            })
            .collect()
    }
}

/// How long a daemon's line says it took to be ready: the first "in N
/// ms" after the word ready or loaded, or none when it says neither.
fn ready_in(message: &str) -> Option<u64> {
    let words: Vec<&str> = message
        .split_whitespace()
        .map(|word| word.trim_matches(|letter: char| !letter.is_ascii_alphanumeric()))
        .collect();
    let said = words
        .iter()
        .position(|word| matches!(*word, "ready" | "loaded"))?;
    words[said..].windows(3).find_map(|three| match three {
        ["in", ms, "ms"] => ms.parse().ok(),
        _ => None,
    })
}

/// One of the three over the day, or none when none was timed.
fn spread(taken: &BTreeMap<String, Vec<u64>>, bar_ms: u64) -> Option<Spread<'_>> {
    let mut all: Vec<u64> = taken.values().flatten().copied().collect();
    all.sort_unstable();
    let count = u64::try_from(all.len()).ok()?;
    let ranked = |rank: u64| {
        let at = usize::try_from(rank.max(1) - 1).ok()?;
        all.get(at).copied()
    };
    let slowest = taken
        .iter()
        .flat_map(|(name, each)| each.iter().map(move |&ms| (ms, name.as_str())))
        .max_by_key(|&(ms, _)| ms)?;
    Some(Spread {
        count,
        median_ms: ranked(count.div_ceil(2))?,
        p99_ms: ranked((count * 99).div_ceil(100))?,
        slowest,
        late: u64::try_from(all.iter().filter(|&&ms| ms > bar_ms).count()).ok()?,
    })
}

/// One of the three in words.
fn said(spread: &Spread, (one, many): (&str, &str), bar_ms: u64) -> String {
    let late = match spread.late {
        0 => format!("none past {bar_ms} ms"),
        late => format!(
            "{late} past {bar_ms} ms ({})",
            percent(spread.late_percent())
        ),
    };
    format!(
        "{} {}: a median {} ms, the slowest {} ms ({}); {late}",
        spread.count,
        if spread.count == 1 { one } else { many },
        spread.median_ms,
        spread.slowest.0,
        spread.slowest.1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Day;
    use crate::day::tests::entry;

    fn day(lines: &[&str]) -> Launches {
        let mut launches = Launches::default();
        for line in lines {
            assert!(
                launches.took(IDENTIFIER, line),
                "a windows daemon's line wasn't taken: {line}"
            );
        }
        launches
    }

    #[test]
    fn openings_and_ends_are_read_by_program_and_other_lines_left() {
        let launches = day(&[
            "opened foot 212 ms after its process started",
            "opened google-chrome 1840 ms after its process started",
            "opened foot 190 ms after its process started",
            "closed foot; its process ended 14 ms later",
            "closed zoom; its process was still running 30 s later",
        ]);
        assert_eq!(
            launches.opened.get("foot"),
            Some(&vec![212, 190]),
            "a program's openings weren't kept by its name"
        );
        assert_eq!(
            launches.ended.get("foot"),
            Some(&vec![14]),
            "an end wasn't kept"
        );
        assert_eq!(
            launches.outlived(),
            1,
            "a process outliving the wait wasn't counted"
        );
        let mut other = Launches::default();
        assert!(
            !other.took(IDENTIFIER, "niri's event stream ended; asking again"),
            "another of the daemon's lines was taken for a timing"
        );
        assert_eq!(other, Launches::default(), "another line changed the day");
    }

    #[test]
    fn a_daemon_saying_its_ready_is_kept_by_its_name_and_other_timings_left() {
        let mut launches = Launches::default();
        for (thing, line) in [
            ("dictation", "dictation: ready in 1065 ms"),
            ("meaning", "meaning: bge loaded on the NPU in 688 ms"),
            (
                "noise-cancelled",
                "noise-cancelled: the model loaded in 398 ms, cleaning with deepfilternet",
            ),
        ] {
            assert!(
                launches.took(thing, line),
                "a ready line wasn't taken: {line}"
            );
        }
        for (thing, line) in [
            (
                "jobs-sorter",
                "Moved timeout (process group 3, 2 processes) from kitty-1.scope to jobs: \
                 6810 ms of CPU in 1040 ms.",
            ),
            (
                "call-transcripts",
                "call-transcripts: mel 33 ms, encoder 325 ms on NPU",
            ),
            ("dictation", "dictation: ready; the model is already loaded"),
        ] {
            assert!(
                !launches.took(thing, line),
                "a line that isn't a ready was taken: {line}"
            );
        }
        assert_eq!(
            launches.ready,
            BTreeMap::from([
                ("dictation".to_owned(), vec![1065]),
                ("meaning".to_owned(), vec![688]),
                ("noise-cancelled".to_owned(), vec![398]),
            ]),
            "each daemon's ready wasn't kept by its name"
        );
        assert_eq!(
            launches.said().as_deref(),
            Some(
                "3 daemons ready: a median 688 ms, the slowest 1065 ms (dictation); 1 past \
                 1000 ms (33%)"
            ),
            "the day's readies were said wrongly"
        );
        assert_eq!(
            launches.by_program().first().map(String::as_str),
            Some("dictation                1 ready, the slowest 1065 ms"),
            "the slowest daemon wasn't said first"
        );
    }

    #[test]
    fn the_day_says_its_median_slowest_and_share_past_each_budget() {
        let launches = day(&[
            "opened foot 212 ms after its process started",
            "opened google-chrome 1840 ms after its process started",
            "opened foot 190 ms after its process started",
            "closed foot; its process ended 14 ms later",
            "closed zoom; its process was still running 30 s later",
        ]);
        let opened = launches.openings().expect("openings were timed");
        assert_eq!(
            (opened.count, opened.median_ms, opened.p99_ms, opened.late),
            (3, 212, 1840, 1),
            "the openings' spread was misread"
        );
        assert_eq!(
            opened.slowest,
            (1840, "google-chrome"),
            "the slowest wasn't named"
        );
        assert_eq!(
            launches.said().as_deref(),
            Some(
                "3 openings: a median 212 ms, the slowest 1840 ms (google-chrome); 1 past \
                 300 ms (33%); 1 end: a median 14 ms, the slowest 14 ms (foot); none past \
                 200 ms; 1 program still ran 30 s after its last window"
            ),
            "the day's line was said wrongly"
        );
        assert_eq!(
            launches.by_program(),
            [
                "google-chrome            1 opened, the slowest 1840 ms",
                "foot                     2 opened, the slowest 212 ms; 1 ended, the slowest 14 ms",
                "zoom                     1 still running 30 s on",
            ],
            "each program wasn't said, the slowest first"
        );
        assert_eq!(
            Launches::default().said(),
            None,
            "a day with none said something"
        );
    }

    #[test]
    fn the_day_takes_the_daemons_timings_and_counts_its_warnings_as_ever() {
        let mut day = Day::new("2026-11-12");
        day.take(&entry(
            IDENTIFIER,
            6,
            "opened foot 212 ms after its process started",
        ));
        day.take(&entry(
            IDENTIFIER,
            4,
            "niri's event stream ended; asking again",
        ));
        day.take(&entry("dictation", 6, "dictation: ready in 1065 ms"));
        assert_eq!(
            day.launches.ready.get("dictation"),
            Some(&vec![1065]),
            "the day didn't take a daemon's ready"
        );
        assert_eq!(
            day.launches.opened.get("foot"),
            Some(&vec![212]),
            "the day didn't take the daemon's opening"
        );
        assert_eq!(
            day.things.get(IDENTIFIER).map(|thing| thing.lines),
            Some(1),
            "the daemon's warning wasn't counted, or its timing was"
        );
    }
}
