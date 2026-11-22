//! The launcher's openings, read from the shell's journal: over the last
//! hour, and while the test call ran set against the same span just
//! before it, so an hour says whether the call itself slows what Joel
//! opens. A call span whose openings are more than one and a half times
//! slower than the span before's is hurting him, and the hour says so once
//! each span holds enough openings to tell.

use crate::figures::percentile;
use serde::Serialize;
use std::process::Command;

/// How many times slower than the span before's the call span's openings
/// may be before the call counts as hurting Joel.
pub const WORSE: f64 = 1.5;

/// The fewest openings each span needs before the two are set against
/// each other, so the median the verdict compares is never one opening's.
pub const FEWEST: usize = 3;

/// The longest a journal's reason is kept, so one line says it.
const SAID: usize = 200;

/// The openings in one span: how many, their median and their p99 in ms.
/// The median is the middle two's mean on an even count, so half a span's
/// openings can't slow without moving it. A span a few minutes long holds
/// far under a hundred openings, so its p99 is its slowest, kept so one
/// slow opening still shows.
#[derive(Debug, PartialEq, Serialize)]
pub struct Span {
    pub opens: usize,
    pub p50: Option<f64>,
    pub p99: Option<f64>,
}

/// The call's span set against the same span just before it.
#[derive(Debug, PartialEq, Serialize)]
pub struct Beside {
    pub before: Span,
    pub during: Span,
    /// Whether the call span's median passed `WORSE` times the span
    /// before's, or null when either span had fewer than `FEWEST`
    /// openings, since then it can't be told. The medians, not the p99s,
    /// since at these counts a p99 is one opening, and one cold opening
    /// on either side would decide it. A reader gating on it refuses on
    /// null.
    pub worse: Option<bool>,
}

/// What the journal gave: each opening over the last hour, as (seconds
/// since the epoch, ms), and why it wasn't read whole, when it wasn't.
pub struct Read {
    pub opens: Vec<(f64, f64)>,
    pub unread: Option<String>,
}

/// Each opening over the last hour, from the line the launcher writes to
/// the shell's journal as it's on screen. Read from the shell's unit, as
/// cadence reads it. Not `--quiet`, which keeps journalctl from saying it
/// found no journal files at all (systemd 262, 22 Nov 2026). short-unix
/// writes the shell's lines without their colours, as `-o cat` doesn't:
/// 250 of 253 lines over three days read as openings, none as blob data.
pub fn read() -> Read {
    let out = Command::new("journalctl")
        .args(["--user", "-u", "quickshell", "--since", "-1h"])
        .args(["--no-pager", "-o", "short-unix"])
        .args(["--grep", "launcher: opening presented"])
        .output();
    match out {
        Ok(out) => from(
            out.status.code(),
            &String::from_utf8_lossy(&out.stdout),
            &String::from_utf8_lossy(&out.stderr),
        ),
        Err(error) => Read {
            opens: Vec::new(),
            unread: Some(format!("journalctl couldn't start: {error}")),
        },
    }
}

/// What journalctl's answer gave: the openings on stdout, kept even when
/// the journal wasn't read whole, beside why it wasn't.
fn from(code: Option<i32>, stdout: &str, stderr: &str) -> Read {
    Read {
        opens: timed(stdout),
        unread: answered(code, stderr).err(),
    }
}

/// Whether journalctl read the journal whole, from how it ended and what
/// it said on stderr. A grep that matches nothing exits 1 and says so on
/// stdout alone (systemd 262, 22 Nov 2026); anything on stderr, no journal
/// files or a file it skipped among them, means some of the hour may be
/// missing. A skipped file is named each hour until it's removed.
fn answered(code: Option<i32>, said: &str) -> Result<(), String> {
    let first = said.trim().lines().next().unwrap_or("");
    let first: String = first.chars().take(SAID).collect();
    match code {
        Some(0 | 1) if first.is_empty() => Ok(()),
        Some(code) if first.is_empty() => Err(format!("journalctl ended with code {code}")),
        Some(code) => Err(format!("journalctl ended with code {code}: {first}")),
        None => Err("journalctl was ended by a signal".to_string()),
    }
}

/// The openings in `lines` as journalctl's short-unix writes them, each a
/// line's seconds then the shell's `launcher: opening presented N ms`.
/// A line not starting with its seconds, a message's own later line, is
/// no opening.
pub fn timed(lines: &str) -> Vec<(f64, f64)> {
    lines
        .lines()
        .filter_map(|line| {
            let (at, text) = line.split_once(' ')?;
            Some((at.parse().ok()?, took(text)?))
        })
        .collect()
}

/// The ms in the launcher's `launcher: opening presented N ms ...`, read
/// as cadence reads it: the launcher's first word after its tag, so text
/// it logs from elsewhere can't pass for an opening. The number must be
/// followed by `ms`, so a line cut off partway is no opening, and must be
/// a time an opening could take.
fn took(text: &str) -> Option<f64> {
    let mut words = text
        .split_once("launcher: ")?
        .1
        .strip_prefix("opening presented ")?
        .split(' ');
    let ms: f64 = words.next()?.parse().ok()?;
    (words.next() == Some("ms") && ms.is_finite() && ms >= 0.0).then_some(ms)
}

/// The median of `took`: its middle one, or the mean of its middle two.
fn median(took: &[f64]) -> Option<f64> {
    let mut sorted = took.to_vec();
    sorted.sort_by(f64::total_cmp);
    let half = sorted.len() / 2;
    match sorted.len() {
        0 => None,
        n if n % 2 == 1 => Some(sorted[half]),
        _ => Some(f64::midpoint(sorted[half - 1], sorted[half])),
    }
}

/// The openings from `started` to `ended`, both seconds since the epoch,
/// against those in the same length of time just before `started`. Ends
/// given the wrong way round make both spans empty, which reads as can't
/// tell, never as fine.
pub fn beside(opens: &[(f64, f64)], started: u64, ended: u64) -> Beside {
    let (from, to) = (seconds(started), seconds(ended));
    let span = |within: &dyn Fn(f64) -> bool| {
        let took: Vec<f64> = opens
            .iter()
            .filter(|(at, _)| within(*at))
            .map(|(_, ms)| *ms)
            .collect();
        Span {
            opens: took.len(),
            p50: median(&took),
            p99: percentile(&took, 99),
        }
    };
    let before = span(&|at| at >= from - (to - from) && at < from);
    let during = span(&|at| at >= from && at <= to);
    let worse = match (before.p50, during.p50) {
        (Some(b), Some(d)) if before.opens >= FEWEST && during.opens >= FEWEST => {
            Some(d > WORSE * b)
        }
        _ => None,
    };
    Beside {
        before,
        during,
        worse,
    }
}

/// Seconds since the epoch as a float, which holds them exactly until 2106.
fn seconds(at: u64) -> f64 {
    f64::from(u32::try_from(at).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Openings at `at` s, each taking its ms.
    fn opened(at: &[f64], ms: f64) -> Vec<(f64, f64)> {
        at.iter().map(|at| (*at, ms)).collect()
    }

    #[test]
    fn openings_are_read_with_their_times_from_short_unix_lines() {
        let lines = "1795303690.981234 laptop quickshell[2101]: \
                     INFO qml: launcher: opening presented 41 ms after the chord\n\
                     -- No entries --\n\
                     1795303700.5 laptop quickshell[2101]: launcher: opening presented 7 ms\n\
                     1795303701.0 laptop quickshell[2101]: launcher: opening presented late\n\
                     1795303701.1 laptop quickshell[2101]: launcher: opening presented 4\n\
                     1795303701.2 laptop quickshell[2101]: launcher: opening presented inf ms\n\
                     1795303701.3 laptop quickshell[2101]: launcher: opening presented -5 ms\n\
                     1795303701.4 laptop quickshell[2101]: launcher: opening presented NaN ms\n\
                     \x20 1795303702.0 launcher: opening presented 900 ms\n\
                     1795303703.0 laptop quickshell[2101]: launcher: asked from outside to show \
                     the network x launcher: opening presented 900 ms\n";
        assert_eq!(
            timed(lines),
            [(1_795_303_690.981_234, 41.0), (1_795_303_700.5, 7.0)],
            "a later line of a message, a line cut off, a time no opening takes, or the \
             phrase in text from elsewhere, is no opening"
        );
    }

    #[test]
    fn a_journal_read_in_part_keeps_its_openings_beside_why() {
        let stdout = "1000.0 laptop quickshell[1]: launcher: opening presented 41 ms\n\
                      1001.0 laptop quickshell[1]: launcher: opening presented 7 ms\n";
        let read = from(
            Some(0),
            stdout,
            "Journal file x.journal~ is truncated, ignoring file.\n",
        );
        assert_eq!(read.opens, [(1000.0, 41.0), (1001.0, 7.0)]);
        assert!(
            read.unread
                .as_deref()
                .is_some_and(|why| why.contains("truncated")),
            "the skipped file is said: {:?}",
            read.unread
        );
        let whole = from(Some(0), stdout, "");
        assert_eq!((whole.opens.len(), whole.unread), (2, None));
    }

    #[test]
    fn an_even_span_s_median_is_its_middle_two_s_mean() {
        let mut split = opened(&[910.0, 930.0, 950.0], 30.0);
        split.extend([
            (1010.0, 20.0),
            (1020.0, 20.0),
            (1030.0, 200.0),
            (1040.0, 200.0),
        ]);
        let seen = beside(&split, 1000, 1100);
        assert_eq!(seen.during.p50, Some(110.0));
        assert_eq!(
            seen.worse,
            Some(true),
            "half the call's openings ten times slower is worse"
        );
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    }

    #[test]
    fn the_call_s_span_is_set_against_the_same_span_before_it() {
        // The call ran from 1000 to 1100, so the span before is 900 to 1000.
        let opens = [
            (850.0, 900.0),
            (900.0, 20.0),
            (950.0, 25.0),
            (999.0, 30.0),
            (1000.0, 40.0),
            (1050.0, 42.0),
            (1100.0, 46.0),
            (1101.0, 900.0),
        ];
        let seen = beside(&opens, 1000, 1100);
        assert_eq!(
            seen.before,
            Span {
                opens: 3,
                p50: Some(25.0),
                p99: Some(30.0)
            }
        );
        assert_eq!(
            seen.during,
            Span {
                opens: 3,
                p50: Some(42.0),
                p99: Some(46.0)
            }
        );
        assert_eq!(
            seen.worse,
            Some(true),
            "a median of 42 ms against 25 before is past 1.5 times"
        );
        let mut even = opened(&[910.0, 930.0, 950.0], 30.0);
        even.extend(opened(&[1010.0, 1030.0, 1050.0], 45.0));
        assert_eq!(
            beside(&even, 1000, 1100).worse,
            Some(false),
            "45 ms against 30 ms is exactly 1.5 times, not past it"
        );
    }

    #[test]
    fn one_slow_opening_on_either_side_doesn_t_decide_it() {
        let mut cold = opened(&[910.0, 930.0, 950.0], 30.0);
        cold.extend([(1010.0, 20.0), (1030.0, 20.0), (1050.0, 400.0)]);
        let seen = beside(&cold, 1000, 1100);
        assert_eq!(seen.during.p99, Some(400.0), "the slow opening still shows");
        assert_eq!(
            seen.worse,
            Some(false),
            "one cold opening during isn't the call"
        );
        let mut slowed = vec![(910.0, 30.0), (930.0, 30.0), (950.0, 400.0)];
        slowed.extend([(1010.0, 200.0), (1030.0, 210.0), (1050.0, 220.0)]);
        assert_eq!(
            beside(&slowed, 1000, 1100).worse,
            Some(true),
            "one slow opening before doesn't hide a call that slowed every one"
        );
    }

    #[test]
    fn too_few_openings_on_either_side_can_t_be_told() {
        let full = opened(&[910.0, 930.0, 950.0], 30.0);
        let thin = opened(&[1010.0, 1050.0], 400.0);
        let short_during: Vec<_> = full.iter().chain(&thin).copied().collect();
        assert_eq!(
            beside(&short_during, 1000, 1100).worse,
            None,
            "two openings during, however slow, can't be told"
        );
        let thin_before = opened(&[910.0, 950.0], 30.0);
        let full_during = opened(&[1010.0, 1030.0, 1050.0], 400.0);
        let short_before: Vec<_> = thin_before.iter().chain(&full_during).copied().collect();
        assert_eq!(
            beside(&short_before, 1000, 1100).worse,
            None,
            "two openings before can't be told"
        );
        let none = beside(&[], 1000, 1100);
        assert_eq!(
            none.during,
            Span {
                opens: 0,
                p50: None,
                p99: None
            }
        );
        assert_eq!(none.worse, None, "no openings at all can't be told");
        let backwards = beside(&[(950.0, 30.0), (1050.0, 30.0)], 1100, 1000);
        assert_eq!(
            (
                backwards.before.opens,
                backwards.during.opens,
                backwards.worse
            ),
            (0, 0, None),
            "ends the wrong way round read as can't tell"
        );
    }

    #[test]
    fn the_journal_counts_as_read_only_when_journalctl_said_nothing_on_stderr() {
        assert_eq!(answered(Some(0), ""), Ok(()));
        assert_eq!(
            answered(Some(1), "\n"),
            Ok(()),
            "a grep that matched nothing"
        );
        assert_eq!(
            answered(Some(1), "No journal files were found.\nsecond\n"),
            Err("journalctl ended with code 1: No journal files were found.".to_string())
        );
        assert_eq!(
            answered(
                Some(0),
                "Journal file x.journal~ is truncated, ignoring file.\n"
            ),
            Err(
                "journalctl ended with code 0: Journal file x.journal~ is truncated, \
                 ignoring file."
                    .to_string()
            ),
            "a skipped file may hold some of the hour"
        );
        assert_eq!(
            answered(Some(2), ""),
            Err("journalctl ended with code 2".to_string())
        );
        assert_eq!(
            answered(None, ""),
            Err("journalctl was ended by a signal".to_string())
        );
        let long = "x".repeat(500);
        assert_eq!(
            answered(Some(1), &long),
            Err(format!(
                "journalctl ended with code 1: {}",
                "x".repeat(SAID)
            )),
            "a long reason is cut to one short line"
        );
        assert_eq!(
            answered(Some(1), &"é".repeat(300)),
            Err(format!(
                "journalctl ended with code 1: {}",
                "é".repeat(SAID)
            )),
            "the cut falls between characters, never inside one"
        );
    }

    #[test]
    fn the_run_carries_the_spans_in_the_shape_its_readers_take() {
        let seen = beside(&[(950.0, 30.0)], 1000, 1100);
        assert_eq!(
            serde_json::to_value(&seen).unwrap(),
            json!({
                "before": {"opens": 1, "p50": 30.0, "p99": 30.0},
                "during": {"opens": 0, "p50": null, "p99": null},
                "worse": null,
            })
        );
    }
}
