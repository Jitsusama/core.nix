//! The launcher's openings, read from the shell's journal: over the last
//! hour, and while the test call ran set against the same span just
//! before it, so an hour says whether the call itself slows what Joel
//! opens. A call span more than one and a half times worse than the span
//! before it is hurting him, and the hour says so.

use crate::figures::percentile;
use serde::Serialize;
use std::process::Command;

/// How many times the span before the call's p99 the call's own may be
/// before the call counts as hurting Joel.
pub const WORSE: f64 = 1.5;

/// The openings in one span: how many, and their p99 in ms.
#[derive(Debug, PartialEq, Serialize)]
pub struct Span {
    pub opens: usize,
    pub p99: Option<f64>,
}

/// The call's span set against the same span just before it.
#[derive(Debug, PartialEq, Serialize)]
pub struct Beside {
    pub before: Span,
    pub during: Span,
    /// Whether the call's p99 passed `WORSE` times the span before's;
    /// false when either span had no openings, since nothing compares.
    pub worse: bool,
}

/// Each opening over the last hour, as (seconds since the epoch, ms), from
/// the line the launcher writes to the shell's journal as it's on screen.
/// Read from the shell's unit alone, as cadence reads it, so no other
/// program's line passes for one.
pub fn read() -> Vec<(f64, f64)> {
    Command::new("journalctl")
        .args(["--user", "-u", "quickshell", "--since", "-1h"])
        .args(["--no-pager", "-o", "short-unix"])
        .args(["--grep", "launcher: opening presented"])
        .output()
        .map(|out| timed(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

/// The openings in `lines` as journalctl's short-unix writes them, each a
/// line's seconds then its text holding `opening presented N ms`.
pub fn timed(lines: &str) -> Vec<(f64, f64)> {
    lines
        .lines()
        .filter_map(|line| {
            let (at, text) = line.trim_start().split_once(' ')?;
            Some((at.parse().ok()?, took(text)?))
        })
        .collect()
}

/// The ms in a line's `... opening presented N ms ...`.
fn took(text: &str) -> Option<f64> {
    let after = text.split_once("opening presented ")?.1;
    after
        .strip_suffix(" ms")
        .unwrap_or(after)
        .split(' ')
        .next()?
        .parse()
        .ok()
}

/// The openings from `started` to `ended`, both seconds since the epoch,
/// against those in the same length of time just before `started`.
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
            p99: percentile(&took, 99),
        }
    };
    let before = span(&|at| at >= from - (to - from) && at < from);
    let during = span(&|at| at >= from && at <= to);
    let worse = matches!((before.p99, during.p99), (Some(b), Some(d)) if d > WORSE * b);
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

    #[test]
    fn openings_are_read_with_their_times_from_short_unix_lines() {
        let lines = "1795303690.981234 optimus quickshell[2101]: \
                     INFO qml: launcher: opening presented 41 ms after the chord\n\
                     -- No entries --\n\
                     1795303700.5 optimus quickshell[2101]: launcher: opening presented 7 ms\n";
        assert_eq!(
            timed(lines),
            [(1_795_303_690.981_234, 41.0), (1_795_303_700.5, 7.0)]
        );
    }

    #[test]
    fn the_call_s_span_is_set_against_the_same_span_before_it() {
        // The call ran from 1000 to 1100, so the span before is 900 to 1000.
        let opens = [
            (850.0, 900.0),
            (900.0, 20.0),
            (999.0, 30.0),
            (1000.0, 40.0),
            (1100.0, 46.0),
            (1101.0, 900.0),
        ];
        let seen = beside(&opens, 1000, 1100);
        assert_eq!(
            seen.before,
            Span {
                opens: 2,
                p99: Some(30.0)
            }
        );
        assert_eq!(
            seen.during,
            Span {
                opens: 2,
                p99: Some(46.0)
            }
        );
        assert!(
            seen.worse,
            "46 ms against 30 ms before is past 1.5 times: {seen:?}"
        );
        let fine = beside(&[(950.0, 30.0), (1050.0, 45.0)], 1000, 1100);
        assert!(
            !fine.worse,
            "45 ms against 30 ms is exactly 1.5 times, not past it: {fine:?}"
        );
    }

    #[test]
    fn a_span_with_no_openings_compares_with_nothing() {
        let none_before = beside(&[(1050.0, 400.0)], 1000, 1100);
        assert_eq!(
            none_before.before,
            Span {
                opens: 0,
                p99: None
            }
        );
        assert!(
            !none_before.worse,
            "no openings before can't be worse: {none_before:?}"
        );
        let none = beside(&[], 1000, 1100);
        assert_eq!(
            none.during,
            Span {
                opens: 0,
                p99: None
            }
        );
        assert!(!none.worse, "no openings at all can't be worse: {none:?}");
    }
}
