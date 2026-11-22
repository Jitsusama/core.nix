//! The launcher's openings, read from the shell's journal: over the last
//! hour, and while the test call ran set against the same span just
//! before it, so an hour says whether the call itself slows what Joel
//! opens. A call span more than one and a half times worse than the span
//! before it is hurting him, and the hour says so once each span holds
//! enough openings to tell.

use crate::figures::percentile;
use serde::Serialize;
use std::process::Command;

/// How many times the span before the call's p99 the call's own may be
/// before the call counts as hurting Joel.
pub const WORSE: f64 = 1.5;

/// The fewest openings each span needs before the two are set against
/// each other. A span's p99 over fewer is its one slowest opening, so a
/// single cold opening during the call would read as the call hurting him.
pub const FEWEST: usize = 3;

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
    /// Whether the call's p99 passed `WORSE` times the span before's, or
    /// null when either span had fewer than `FEWEST` openings, since then
    /// it can't be told. A reader gating on it refuses on null.
    pub worse: Option<bool>,
}

/// Each opening over the last hour, as (seconds since the epoch, ms), from
/// the line the launcher writes to the shell's journal as it's on screen,
/// or why the journal couldn't be read. Read from the shell's unit alone,
/// as cadence reads it, so no other program's line passes for one.
pub fn read() -> Result<Vec<(f64, f64)>, String> {
    let out = Command::new("journalctl")
        .args(["--user", "-u", "quickshell", "--since", "-1h"])
        .args(["--no-pager", "-o", "short-unix"])
        .args(["--grep", "launcher: opening presented"])
        .output()
        .map_err(|error| format!("journalctl couldn't start: {error}"))?;
    let said = String::from_utf8_lossy(&out.stderr);
    // A grep that matches nothing exits 1 and says so on stdout alone;
    // a journal it couldn't read says why on stderr.
    let read = out.status.success() || (out.status.code() == Some(1) && said.trim().is_empty());
    if !read {
        return Err(format!("journalctl exited {}: {}", out.status, said.trim()));
    }
    Ok(timed(&String::from_utf8_lossy(&out.stdout)))
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
            p99: percentile(&took, 99),
        }
    };
    let before = span(&|at| at >= from - (to - from) && at < from);
    let during = span(&|at| at >= from && at <= to);
    let worse = match (before.p99, during.p99) {
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

    #[test]
    fn openings_are_read_with_their_times_from_short_unix_lines() {
        let lines = "1795303690.981234 laptop quickshell[2101]: \
                     INFO qml: launcher: opening presented 41 ms after the chord\n\
                     -- No entries --\n\
                     \x20 1795303700.5 laptop quickshell[2101]: launcher: opening presented 7 ms\n\
                     1795303701.0 laptop quickshell[2101]: launcher: opening presented late\n";
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
                p99: Some(30.0)
            }
        );
        assert_eq!(
            seen.during,
            Span {
                opens: 3,
                p99: Some(46.0)
            }
        );
        assert_eq!(
            seen.worse,
            Some(true),
            "46 ms against 30 ms before is past 1.5 times"
        );
        let fine = beside(
            &[
                (910.0, 30.0),
                (930.0, 30.0),
                (950.0, 30.0),
                (1010.0, 45.0),
                (1030.0, 45.0),
                (1050.0, 45.0),
            ],
            1000,
            1100,
        );
        assert_eq!(
            fine.worse,
            Some(false),
            "45 ms against 30 ms is exactly 1.5 times, not past it"
        );
    }

    #[test]
    fn too_few_openings_on_either_side_can_t_be_told() {
        let one_each = beside(&[(950.0, 20.0), (1050.0, 400.0)], 1000, 1100);
        assert_eq!(one_each.worse, None, "one cold opening is not a verdict");
        let none_during = beside(&[(910.0, 30.0), (930.0, 30.0), (950.0, 30.0)], 1000, 1100);
        assert_eq!(none_during.during.opens, 0);
        assert_eq!(none_during.worse, None, "nothing during compares");
        let none = beside(&[], 1000, 1100);
        assert_eq!(
            none.during,
            Span {
                opens: 0,
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
    fn the_run_carries_the_spans_in_the_shape_its_readers_take() {
        let seen = beside(&[(950.0, 30.0)], 1000, 1100);
        assert_eq!(
            serde_json::to_value(&seen).unwrap(),
            json!({
                "before": {"opens": 1, "p99": 30.0},
                "during": {"opens": 0, "p99": null},
                "worse": null,
            })
        );
    }
}
