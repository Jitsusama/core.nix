//! What the hour measures, each against its target, read from text the
//! machine already keeps, so every rule here is tested without a machine.
//! Every target is a most: lower is better for each of them.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;
use serde_json::Value;

const DAY: i64 = 86_400;

/// What a measure is about, in the order the drag is chosen from: a miss
/// Joel found himself always comes first, then a date missed, then what
/// broke, then how fast things shipped, then how clean the code is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Miss,
    Slip,
    Functional,
    Ship,
    Clean,
}

impl Class {
    /// The class as the card and the ledger name it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Miss => "his misses",
            Self::Slip => "slips",
            Self::Functional => "functional quality",
            Self::Ship => "ship",
            Self::Clean => "clean zones",
        }
    }
}

/// One measure, its figure and target, and who owns moving it.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// What it's known by from one hour to the next.
    pub key: String,
    pub class: Class,
    /// What it measures, in a few plain words.
    pub says: String,
    pub now: f64,
    /// The most it may be, or none where no target is set yet, which the
    /// card shows as unjudged rather than in.
    pub target: Option<f64>,
    /// The mesh address that moves it.
    pub owner: String,
    /// The lines behind the figure, such as each open miss.
    pub behind: Vec<String>,
}

impl Measure {
    /// How far past its target it is, as a share of the target, a target
    /// of nought counting as one; none when it's in or unjudged.
    #[must_use]
    pub fn past(&self) -> Option<f64> {
        let target = self.target?;
        (self.now > target).then(|| (self.now - target) / target.max(1.0))
    }
}

/// The measure to move this hour: the first class with one past its
/// target, and within it the one furthest past.
#[must_use]
pub fn drag(measures: &[Measure]) -> Option<&Measure> {
    measures
        .iter()
        .filter_map(|m| m.past().map(|past| (m, past)))
        .min_by(|(a, pa), (b, pb)| a.class.cmp(&b.class).then(pb.total_cmp(pa)))
        .map(|(m, _)| m)
}

/// The first address in an owner field such as `PN2VZC (H6EDLC)`, which
/// names the one owning it before those helping.
fn first_owner(owner: &str) -> String {
    owner
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn lines(text: &str) -> impl Iterator<Item = Value> + '_ {
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
}

fn seconds(value: &Value) -> Option<i64> {
    value
        .as_str()?
        .parse::<Timestamp>()
        .ok()
        .map(Timestamp::as_second)
}

#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
const fn count(n: usize) -> f64 {
    n as f64
}

/// The misses Joel found himself, from the orchestrator's `misses.jsonl`:
/// one stays open until its owner names the check that would have caught
/// it, and that check merges.
#[must_use]
pub fn misses(text: &str) -> Measure {
    let open: Vec<(String, String)> = lines(text)
        .filter(|miss| miss["check"].is_null())
        .map(|miss| {
            let what = miss["what"]
                .as_str()
                .or_else(|| miss["words"].as_str())
                .unwrap_or_default()
                .to_owned();
            (
                what,
                first_owner(miss["owner"].as_str().unwrap_or_default()),
            )
        })
        .collect();
    Measure {
        key: "misses".to_owned(),
        class: Class::Miss,
        says: "misses Joel found himself, with no check yet to catch the next".to_owned(),
        now: count(open.len()),
        target: Some(0.0),
        // The oldest open miss's owner moves it first.
        owner: open.first().map(|(_, o)| o.clone()).unwrap_or_default(),
        behind: open
            .iter()
            .map(|(what, owner)| format!("{what} · {owner}"))
            .collect(),
    }
}

/// The dates promised in the orchestrator's `asks.jsonl` that passed in
/// the day before `now` without their ask landing by then. A date moved
/// is a new line beside the old, so each move counts as a slip.
#[must_use]
pub fn slips(text: &str, now: i64) -> Measure {
    let mut slipped: Vec<(i64, String, String)> = lines(text)
        .filter_map(|ask| {
            let expected = seconds(&ask["expected"])?;
            let landed = seconds(&ask["landed"]);
            let late = expected <= now
                && expected > now - DAY
                && landed.is_none_or(|landed| landed > expected);
            late.then(|| {
                (
                    expected,
                    ask["words"].as_str().unwrap_or_default().to_owned(),
                    first_owner(ask["owner"].as_str().unwrap_or_default()),
                )
            })
        })
        .collect();
    slipped.sort_by_key(|(expected, _, _)| *expected);
    Measure {
        key: "slips".to_owned(),
        class: Class::Slip,
        says: "dates promised that passed in the last day without landing".to_owned(),
        now: count(slipped.len()),
        target: Some(0.0),
        owner: slipped
            .first()
            .map(|(_, _, o)| o.clone())
            .unwrap_or_default(),
        behind: slipped
            .iter()
            .map(|(_, words, owner)| format!("{} · {owner}", short(words)))
            .collect(),
    }
}

/// The first few words of an ask, enough to know it by.
fn short(words: &str) -> String {
    let mut said: Vec<&str> = words.split_whitespace().take(9).collect();
    if words.split_whitespace().nth(9).is_some() {
        said.push("...");
    }
    said.join(" ")
}

/// The quarantined tests in the merge net's `quarantine.json` with no
/// owner or no date to come back by, each of which counts as a slip.
#[must_use]
pub fn undated(quarantine: &str, today: &str, owner: &str) -> Measure {
    let list: Vec<Value> = serde_json::from_str(quarantine).unwrap_or_default();
    // An ISO date compares as its text, so one before today is past.
    let undated: Vec<&Value> = list
        .iter()
        .filter(|q| q["owner_pr"].is_null() || q["due"].as_str().is_none_or(|due| due < today))
        .collect();
    let tests: BTreeSet<String> = undated
        .iter()
        .map(|q| q["test"].as_str().unwrap_or_default().to_owned())
        .collect();
    let all = list.len();
    Measure {
        key: "quarantine-undated".to_owned(),
        class: Class::Slip,
        says: format!(
            "quarantined tests with no owner, or no date to come back or past it, of {all}"
        ),
        now: count(undated.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: tests.into_iter().collect(),
    }
}

/// The merges the net reverted, or quarantined a test for when the revert
/// wouldn't apply, in the day before `now`, from cadence's `reverted.json`,
/// which keeps when each owner was told, by `ended-in:merge`.
#[must_use]
pub fn reverts(text: &str, now: i64, owner: &str) -> Measure {
    let kept: Value = serde_json::from_str(text).unwrap_or_default();
    let recent: Vec<String> = kept["reverts"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, at)| at.as_i64().is_some_and(|at| at > now - DAY && at <= now))
        .filter_map(|(key, _)| key.split(':').nth(1).map(|undone| format!("#{undone}")))
        .collect();
    Measure {
        key: "reverts".to_owned(),
        class: Class::Functional,
        says: "merges the net reverted or quarantined a test for, in the last day".to_owned(),
        now: count(recent.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: recent,
    }
}

/// The checks still failing on main that last passed before a merge in
/// the day before `now`, from the net's `results.jsonl`: a check whose
/// newest result failed where the one before it passed. A stalled run
/// proves nothing either way, so it's left out.
#[must_use]
pub fn broken(text: &str, now: i64, owner: &str) -> Measure {
    let mut by_check: BTreeMap<String, Vec<(i64, bool)>> = BTreeMap::new();
    for result in lines(text).filter(|r| r["stalled"] != Value::Bool(true)) {
        let (Some(check), Some(at), Some(passed)) = (
            result["check"].as_str(),
            result["at"].as_i64(),
            result["passed"].as_bool(),
        ) else {
            continue;
        };
        by_check
            .entry(check.to_owned())
            .or_default()
            .push((at, passed));
    }
    let failing: Vec<String> = by_check
        .into_iter()
        .filter_map(|(check, mut runs)| {
            runs.sort_by_key(|(at, _)| *at);
            let [.., (_, before), (at, last)] = runs.as_slice() else {
                return None;
            };
            (*before && !last && *at > now - DAY).then_some(check)
        })
        .collect();
    Measure {
        key: "broken".to_owned(),
        class: Class::Functional,
        says: "checks a merge broke in the last day, still failing".to_owned(),
        now: count(failing.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: failing,
    }
}

/// The median the tick's card gives on the line opening with `label`,
/// such as `Merge to on (target median 18): median 37.9, ...`, and the
/// target it names.
#[must_use]
pub fn card_median(card: &str, label: &str) -> Option<(f64, f64)> {
    let line = card.lines().find(|line| line.starts_with(label))?;
    let number = |after: &str| -> Option<f64> {
        let rest = &line[line.find(after)? + after.len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(rest.len());
        rest[..end].parse().ok()
    };
    Some((number("): median ")?, number("target median ")?))
}

/// How many first-parent merges `log`, as `%H %ct %s` lines, holds from
/// the hour before `now`.
#[must_use]
pub fn landed(log: &str, now: i64) -> usize {
    log.lines()
        .filter_map(|line| line.split(' ').nth(1)?.parse::<i64>().ok())
        .filter(|at| *at > now - 3600 && *at <= now)
        .count()
}

/// The code-health measures Y9XISP's hourly read writes, each with its
/// target, or none where no target is set yet.
#[must_use]
pub fn clean(text: &str) -> Vec<Measure> {
    let read: Value = serde_json::from_str(text).unwrap_or_default();
    read["measures"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let repo = m["repo"].as_str()?;
            let measure = m["measure"].as_str()?;
            Some(Measure {
                key: format!("clean:{repo}:{measure}"),
                class: Class::Clean,
                says: format!("{measure} in {repo}"),
                now: m["now"].as_f64()?,
                target: m["target"].as_f64(),
                owner: m["owner"].as_str().unwrap_or("Y9XISP").to_owned(),
                behind: m["files"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .take(3)
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_700_000;

    fn at(seconds: i64) -> String {
        Timestamp::from_second(seconds)
            .map(|t| t.to_string())
            .unwrap_or_default()
    }

    fn measure(key: &str, class: Class, now: f64, target: Option<f64>) -> Measure {
        Measure {
            key: key.to_owned(),
            class,
            says: key.to_owned(),
            now,
            target,
            owner: "OWNER".to_owned(),
            behind: Vec::new(),
        }
    }

    #[test]
    fn an_open_miss_of_joels_outranks_everything_past_its_target() {
        let measures = [
            measure("ship", Class::Ship, 90.0, Some(18.0)),
            measure("misses", Class::Miss, 1.0, Some(0.0)),
            measure("broken", Class::Functional, 4.0, Some(0.0)),
        ];
        assert_eq!(
            drag(&measures).map(|m| m.key.as_str()),
            Some("misses"),
            "a miss comes first"
        );
    }

    #[test]
    fn within_a_class_the_drag_is_the_one_furthest_past_and_unjudged_ones_never_are() {
        let measures = [
            measure("near", Class::Clean, 11.0, Some(10.0)),
            measure("far", Class::Clean, 30.0, Some(10.0)),
            measure("unjudged", Class::Clean, 900.0, None),
            measure("in", Class::Slip, 0.0, Some(0.0)),
        ];
        assert_eq!(
            drag(&measures).map(|m| m.key.as_str()),
            Some("far"),
            "furthest past wins"
        );
        assert_eq!(
            drag(&measures[2..]),
            None,
            "nothing past its target is no drag"
        );
    }

    #[test]
    fn a_miss_stays_open_until_a_check_is_named_for_it() {
        let text = concat!(
            r#"{"words":"w","what":"the map had no network","owner":"KNURBN (VKCF3Z)","check":null}"#,
            "\n",
            r#"{"words":"w","what":"History paused","owner":"RCTRF8","check":"core#4500"}"#,
            "\n",
        );
        let m = misses(text);
        assert!((m.now - 1.0).abs() < 0.5, "one open, got {}", m.now);
        assert_eq!(m.owner, "KNURBN", "its first owner moves it");
        assert_eq!(
            m.behind,
            ["the map had no network · KNURBN"],
            "it says what it was"
        );
    }

    #[test]
    fn a_date_passed_without_landing_is_a_slip_and_each_move_counts() {
        let ask = |expected: i64, landed: Option<i64>| {
            serde_json::json!({
                "words": "fix it", "owner": "Y2YBTS", "expected": at(expected),
                "landed": landed.map(at),
            })
            .to_string()
        };
        let text = [
            ask(NOW - 7200, None),            // moved: the old line
            ask(NOW - 600, None),             // moved again, also passed
            ask(NOW + 3600, None),            // the date it has now
            ask(NOW - 900, Some(NOW - 1000)), // landed in time
            ask(NOW - 900, Some(NOW - 100)),  // landed late
            ask(NOW - 2 * DAY, None),         // passed before the day
        ]
        .join("\n");
        let m = slips(&text, NOW);
        assert!(
            (m.now - 3.0).abs() < 0.5,
            "two moves and a late landing, got {}",
            m.now
        );
        assert_eq!(m.owner, "Y2YBTS", "the oldest slip's owner");
    }

    #[test]
    fn a_quarantine_without_a_date_to_come_back_counts() {
        let text = r#"[{"test":"a","owner_pr":1,"due":"2026-11-20"},{"test":"b","owner_pr":2},
            {"test":"c","due":"2026-11-20"},{"test":"d","owner_pr":3,"due":"2026-11-13"},
            {"test":"e","owner_pr":4,"due":"2026-11-14"}]"#;
        let m = undated(text, "2026-11-14", "80ECO2");
        assert!(
            (m.now - 3.0).abs() < 0.5,
            "undated, unowned and past, got {}",
            m.now
        );
        assert_eq!(
            m.behind,
            ["b", "c", "d"],
            "named by test; due today isn't past"
        );
    }

    #[test]
    fn reverts_count_from_the_last_day_only() {
        let text = format!(
            r#"{{"reverts":{{"4459:4426":{},"4000:3999":{}}}}}"#,
            NOW - 60,
            NOW - 2 * DAY
        );
        let m = reverts(&text, NOW, "80ECO2");
        assert_eq!(m.behind, ["#4426"], "the undone merge, from today");
    }

    #[test]
    fn a_check_failing_after_it_passed_is_broken_and_a_stall_proves_nothing() {
        let result = |check: &str, at: i64, passed: bool, stalled: bool| {
            serde_json::json!({"check": check, "at": at, "passed": passed, "stalled": stalled})
                .to_string()
        };
        let text = [
            result("broke", NOW - 900, true, false),
            result("broke", NOW - 60, false, false),
            result("always", NOW - 900, false, false),
            result("always", NOW - 60, false, false),
            result("stalled", NOW - 900, true, false),
            result("stalled", NOW - 60, false, true),
            result("mended", NOW - 900, false, false),
            result("mended", NOW - 60, true, false),
        ]
        .join("\n");
        assert_eq!(
            broken(&text, NOW, "D5QEKR").behind,
            ["broke"],
            "only the one a merge broke"
        );
    }

    #[test]
    fn the_cards_median_and_target_are_read_as_it_prints_them() {
        let card = "Card at 18:05\nMerge to on (target median 18): median 37.9, p90 47.7, max \
                    47.7 min (n 6).\n";
        assert_eq!(
            card_median(card, "Merge to on"),
            Some((37.9, 18.0)),
            "median and target"
        );
        assert_eq!(card_median(card, "Last commit"), None, "a line it lacks");
    }

    #[test]
    fn landings_count_the_hour_before_now() {
        let log = format!(
            "a {} one\nb {} two\nc {} three\n",
            NOW - 60,
            NOW - 3000,
            NOW - 4000
        );
        assert_eq!(landed(&log, NOW), 2, "two in the hour");
    }

    #[test]
    fn code_health_reads_with_its_targets_and_none_is_unjudged() {
        let text = r#"{"measures":[{"repo":"core.nix","measure":"long-files","now":12,
            "target":10,"owner":"Y9XISP","files":["a.rs"]},{"repo":"harness",
            "measure":"suppressions","now":40,"target":null}]}"#;
        let read = clean(text);
        assert_eq!(read.len(), 2, "both read");
        assert!(read[0].past().is_some(), "past its target");
        assert_eq!(read[1].past(), None, "no target is unjudged");
    }
}
