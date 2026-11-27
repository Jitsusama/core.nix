//! How long the node's site takes to come live, from the samples
//! cadence-site (site.sh) writes every ten minutes: each watch the page
//! opens, timed to its first records. The page lays its map once the four
//! watches it waits on have answered, the rest filling in after, so a
//! sample's wait is the slowest of those four's,
//! and the hour's median of those is held to a ceiling, an unanswered
//! watch or a failed sample counting as the probe's whole wait. On 26
//! November at 22:40 the message watch took 3.5 s and the cell watch
//! 2.8 s, where seven minutes before both took under 1 s; it came back
//! through the night, so a watch slow again is the node's to move.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::super::today;
use super::{Class, Measure};
use crate::DAY;

/// What the measure is known by from one hour to the next.
pub const KEY: &str = "site:live";

/// Who keeps the node, whose answers the watches wait on, and so moves it.
const OWNER: &str = "KP0L6A";

/// The most an hour's median wait may take, in seconds: about twice the
/// slowest watch on a quiet night, and under what the page took slow.
pub const LIVE_AT_MOST: f64 = 2.0;

/// How long site.sh waits for a watch before it counts as unanswered,
/// which an unanswered watch and a failed sample count as.
pub const PROBE_WAIT: f64 = 20.0;

/// The watches the site's page waits on before it lays its map
/// (modules/mycelium/site/site.ts's `heard`); those it watches apart fill
/// in once they come, so they're sampled but hold no page.
const WAITED_ON: [&str; 4] = ["network", "cell", "machine", "message"];

/// The fewest samples an hour judges by: the probe takes six.
const SAMPLES_AT_LEAST: usize = 3;

/// The folder under the state home site.sh writes its days in.
pub const FOLDER: &str = "cadence/site";

/// The feed home.nix names to the hour when cadence-site samples it; with
/// none, the site isn't sampled and the hour has no measure of it.
const FEED: &str = "CADENCE_SITE";

/// One sample: each watch's seconds, an unanswered one counted as the
/// whole wait, or the sample's failure.
struct Sample {
    watches: Vec<(String, f64)>,
    error: Option<String>,
}

impl Sample {
    /// How long the page waited on this sample to come live.
    fn wait(&self) -> f64 {
        self.watches
            .iter()
            .filter(|(kind, _)| WAITED_ON.contains(&kind.as_str()))
            .map(|(_, seconds)| *seconds)
            .reduce(f64::max)
            .unwrap_or(PROBE_WAIT)
    }
}

/// The hour's measure of the site, read from the last two days' files, or
/// none where home.nix names no feed.
#[must_use]
pub fn hour(state_home: &Path, now: i64) -> Option<Measure> {
    std::env::var(FEED).ok().filter(|feed| !feed.is_empty())?;
    let folder = state_home.join(FOLDER);
    let day = |at| fs::read_to_string(folder.join(format!("{}.jsonl", today(at))));
    let lines = [day(now - DAY), day(now)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    Some(site(&lines, now))
}

/// The measure over the hour to `now` from site.sh's lines, unjudged where
/// fewer than three samples were read.
#[must_use]
pub fn site(lines: &str, now: i64) -> Measure {
    let hour = now - 3600..now;
    let samples: Vec<Sample> = lines
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|v| {
            v.get("at")
                .and_then(Value::as_i64)
                .is_some_and(|at| hour.contains(&at))
        })
        .map(|v| Sample {
            watches: v
                .get("kinds")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(kind, s)| (kind.clone(), s.as_f64().unwrap_or(PROBE_WAIT)))
                .collect(),
            error: v.get("error").and_then(Value::as_str).map(str::to_owned),
        })
        .collect();
    measured(&samples)
}

/// The middle of `times`, the middle two's mean for an even count, so three
/// slow samples of six read half way rather than as the slow one.
fn median(mut times: Vec<f64>) -> f64 {
    times.sort_by(f64::total_cmp);
    let half = times.len() / 2;
    if times.len().is_multiple_of(2) {
        f64::midpoint(times[half - 1], times[half])
    } else {
        times[half]
    }
}

/// The measure from the hour's samples.
fn measured(samples: &[Sample]) -> Measure {
    if samples.len() < SAMPLES_AT_LEAST {
        return Measure {
            key: KEY.to_owned(),
            class: Class::Functional,
            says: format!(
                "the node's site coming live: {} of its samples this hour, too few to judge",
                samples.len()
            ),
            now: 0.0,
            target: None,
            owner: OWNER.to_owned(),
            behind: Vec::new(),
        };
    }
    let waits: Vec<f64> = samples.iter().map(Sample::wait).collect();
    let worst = waits.iter().copied().fold(0.0, f64::max);
    let mut kinds: Vec<&str> = samples
        .iter()
        .flat_map(|s| s.watches.iter().map(|(kind, _)| kind.as_str()))
        .filter(|kind| WAITED_ON.contains(kind))
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    // The watch that held the page longest, by its own median over the
    // samples that opened it, so a red names which answer to look at.
    let slowest = kinds
        .iter()
        .map(|kind| {
            let times = samples
                .iter()
                .flat_map(|s| &s.watches)
                .filter(|(k, _)| k == kind)
                .map(|(_, seconds)| *seconds)
                .collect();
            (*kind, median(times))
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b));
    let mut behind = Vec::new();
    if let Some((kind, seconds)) = slowest {
        behind.push(format!("slowest watch {kind}, median {seconds:.2} s"));
    }
    behind.push(format!("worst {worst:.2} s"));
    let failed: Vec<&Sample> = samples.iter().filter(|s| s.watches.is_empty()).collect();
    if let Some(first) = failed.first() {
        let error: String = first
            .error
            .as_deref()
            .unwrap_or("an unsaid error")
            .chars()
            .take(120)
            .collect();
        behind.push(format!(
            "{} of {} failed, as {error}",
            failed.len(),
            samples.len()
        ));
    }
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: format!(
            "median seconds the node's site took to come live this hour, every watch \
             it waits on answered, of {} samples",
            samples.len()
        ),
        now: (median(waits) * 100.0).round() / 100.0,
        target: Some(LIVE_AT_MOST),
        owner: OWNER.to_owned(),
        behind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_795_723_200;

    /// A sample `ago` seconds before the hour's end, of each watch's
    /// seconds, none for one unanswered.
    fn sampled(ago: i64, watches: &[(&str, Option<f64>)]) -> String {
        let kinds: serde_json::Map<String, Value> = watches
            .iter()
            .map(|(kind, s)| ((*kind).to_owned(), s.map_or(Value::Null, Value::from)))
            .collect();
        serde_json::json!({"at": NOW - ago, "kinds": kinds}).to_string()
    }

    fn quick(ago: i64) -> String {
        sampled(ago, &[("cell", Some(0.5)), ("message", Some(0.9))])
    }

    #[test]
    fn a_quick_hour_is_judged_by_its_slowest_watch_and_in() {
        let m = site(&[quick(60), quick(660), quick(1260)].join("\n"), NOW);
        assert_eq!(m.target, Some(LIVE_AT_MOST), "three samples weren't judged");
        assert!(
            (m.now - 0.9).abs() < 1e-9,
            "the wait wasn't the slowest watch's: {m:?}"
        );
        assert_eq!(m.past(), None, "a quick hour read as past: {m:?}");
        assert_eq!(
            m.behind[0], "slowest watch message, median 0.90 s",
            "the slowest watch wasn't named"
        );
    }

    #[test]
    fn one_slow_watch_holds_the_page_and_is_named_behind_it() {
        let slow = |ago| sampled(ago, &[("cell", Some(0.5)), ("message", Some(3.1))]);
        let m = site(&[slow(60), slow(660), quick(1260)].join("\n"), NOW);
        assert!(
            (m.now - 3.1).abs() < 1e-9,
            "two slow samples of three didn't make the median: {m:?}"
        );
        assert!(m.past().is_some(), "a slow hour read as in: {m:?}");
        assert_eq!(
            m.behind[0], "slowest watch message, median 3.10 s",
            "the slow watch wasn't named"
        );
    }

    #[test]
    fn an_unanswered_watch_or_a_failed_sample_counts_the_whole_wait() {
        let unanswered = sampled(60, &[("cell", Some(0.5)), ("message", None)]);
        let failed = serde_json::json!({"at": NOW - 660, "error": "no hello"}).to_string();
        let m = site(&[unanswered, failed, quick(1260)].join("\n"), NOW);
        assert!(
            (m.now - PROBE_WAIT).abs() < 1e-9,
            "an unanswered watch and a failure didn't count the wait: {m:?}"
        );
        assert!(
            m.behind.contains(&"1 of 3 failed, as no hello".to_owned()),
            "the failure wasn't said: {:?}",
            m.behind
        );
    }

    #[test]
    fn an_even_hour_takes_the_middle_twos_mean_not_its_worst() {
        let waited =
            |ago, seconds| sampled(ago, &[("cell", Some(0.2)), ("message", Some(seconds))]);
        let lines = [
            waited(60, 0.5),
            waited(660, 1.5),
            waited(1260, 2.5),
            waited(1860, 9.0),
        ];
        let m = site(&lines.join("\n"), NOW);
        assert!(
            (m.now - 2.0).abs() < 1e-9,
            "four samples didn't take the mean of the middle two: {m:?}"
        );
        assert!(
            m.behind.contains(&"worst 9.00 s".to_owned()),
            "the worst wasn't said beside the median: {:?}",
            m.behind
        );
    }

    #[test]
    fn a_watch_the_page_doesnt_wait_on_holds_no_sample() {
        let apart = |ago| sampled(ago, &[("message", Some(0.4)), ("tie", Some(6.0))]);
        let m = site(&[apart(60), apart(660), apart(1260)].join("\n"), NOW);
        assert!(
            (m.now - 0.4).abs() < 1e-9,
            "a watch apart held the page: {m:?}"
        );
        assert_eq!(
            m.behind[0], "slowest watch message, median 0.40 s",
            "a watch apart was named as the slowest"
        );
    }

    #[test]
    fn too_few_samples_in_the_hour_leave_it_unjudged() {
        let outside = quick(3700);
        let m = site(&[quick(60), quick(660), outside].join("\n"), NOW);
        assert_eq!(m.target, None, "two samples in the hour were judged: {m:?}");
        assert!(
            m.says.contains("2 of its samples"),
            "the count wasn't said: {m:?}"
        );
    }
}
