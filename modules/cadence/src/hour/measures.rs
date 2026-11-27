//! What the hour measures, each against its target, read from text the
//! machine already keeps, so every rule here is tested without a machine.
//! Every target is a most: lower is better for each of them.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;
use serde_json::Value;

use owner::first_owner;

use crate::tracking::{self, Tracking};

pub mod apps;
pub mod at_risk;
mod balanced;
mod broken;
mod card;
mod carried;
pub mod causes;
mod cells;
pub mod cited;
pub mod holds;
pub mod locked;
mod misses;
mod owner;
mod pains;
pub mod parked;
mod promised;
pub mod queues;
mod reports;
pub mod rounds;
#[cfg(test)]
mod slipped;
mod slips;
pub mod spent;
pub mod told_late;
#[cfg(test)]
mod tracked;
pub mod unbilled;
pub mod unproven;
pub mod unresolved;
pub mod way_out;
pub mod wifi;

pub use balanced::balanced;
pub use broken::{Main, broken, failed_in};
pub use card::{card_if_fresh, commit_to_merge, merge_to_on};
pub use carried::carried;
pub use cells::{Quest, cells, mapped, mid_resume, quest};
pub use locked::read as unlocked;
pub use misses::{
    BLIND, FINDINGS, HELD, SAID_NONE, UNREAD, UNRUN, UNSEEN, UNTESTED, checked_beside, findings,
    forget_unfixed, held, held_for_good, held_unseen, hosts_started, ledger, said, said_none,
    untested,
};
pub use pains::{keeper, pains};
pub use parked::parked;
pub use reports::{REPORTS_SINCE, Report, report, unasked};
pub use slips::slips;
pub use told_late::told_late;

const DAY: i64 = 86_400;

/// What a measure is about, in the order the drag is chosen from: how
/// his apps answer under load comes first, as Joel ranked it on 19
/// November, then a miss he found himself, then a date missed, then what
/// broke, then how fast things shipped, then how the cells doing it
/// stand, then how clean the code is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Apps,
    Miss,
    Slip,
    Functional,
    Ship,
    Cells,
    Clean,
    /// The queues the work waits in, which never lead the drag: the worst
    /// is asked apart, once a day (owing.rs).
    Queues,
    /// Joel's pains, last: the hearer routes each to its owner itself.
    Pains,
}

impl Class {
    /// The class as the card and the ledger name it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Apps => "his apps under load",
            Self::Miss => "his misses",
            Self::Slip => "slips",
            Self::Functional => "functional quality",
            Self::Ship => "ship",
            Self::Cells => "cell health",
            Self::Clean => "clean zones",
            Self::Queues => "queues",
            Self::Pains => "his pains",
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

/// `measure`, read from nothing since what it reads couldn't be read,
/// unjudged and saying `why`, so a file lost or torn isn't counted as
/// none past its target and closed as fixed.
#[must_use]
pub fn unread(mut measure: Measure, why: &str) -> Measure {
    measure.target = None;
    measure.says = format!("{} (unread: {why})", measure.says);
    measure
}

/// The measure to move this hour: the first class with one past its
/// target, and within it the one furthest past. A merge's rounds past five
/// can't move, so they are filed to its owner apart and never lead; nor
/// can a date already told late, which stays on the card for its day and
/// is filed to each owner apart, but never leads, since asking its owner
/// to steer it each hour moves nothing; nor do the queues, whose worst is asked apart once a day,
/// nor a cause found again and again, each class of which is filed with its own owner apart;
/// nor his asks unproven, each asked of its own cell apart, so a week's backlog of them can't
/// hold every later class's drag unfiled. Nor does a count of the way the calls go out (the
/// relay, pi's calls and the path), filed to its keeper apart each hour it is past, so it
/// never takes the place of what else drags.
#[must_use]
pub fn drag(measures: &[Measure]) -> Option<&Measure> {
    measures
        .iter()
        .filter(|m| !m.key.starts_with(rounds::FRUITLESS) && !m.key.starts_with(told_late::KEY))
        .filter(|m| m.key != causes::KEY && m.key != unproven::KEY && !way_out::on_the_way(&m.key))
        .filter(|m| m.class != Class::Queues)
        .filter_map(|m| m.past().map(|past| (m, past)))
        .min_by(|(a, pa), (b, pb)| a.class.cmp(&b.class).then(pb.total_cmp(pa)))
        .map(|(m, _)| m)
}

fn lines(text: &str) -> impl Iterator<Item = Value> + '_ {
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
}

/// The asks list's lines, each read as its ask, but for a twin: a line
/// the keeper folded into another with the same moment and words, its id
/// marked `~`. Every measure of the asks keeps an ask by its latest line,
/// so a twin written after the ask it copies would stand in its place.
fn asks(text: &str) -> impl Iterator<Item = Value> + '_ {
    lines(text).filter(|ask| !ask["id"].as_str().is_some_and(|id| id.contains('~')))
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

/// How long Joel's asks in the orchestrator's `asks.jsonl` took to put a
/// first piece he can use in front of him, as the median in hours over
/// every ask still open or first reached in the day before `now`. An
/// open ask counts the hours it has waited so far, so one can't hide by
/// staying open. A date moved is a new line for the same ask, known by
/// when it was asked and its words, so each ask counts once. Its first
/// slice is `first` where the orchestrator wrote one, else `landed`. Only
/// asks of the kind `which` count, and only his own are held to a target.
#[must_use]
pub fn to_first_slice(text: &str, now: i64, which: Tracking) -> Measure {
    // Each ask, by when and what was asked: its owner, first reaching, and
    // how its latest line tracks it, since the list re-marks or puts down
    // an ask by writing it again.
    type Ask = (String, Option<i64>, Option<Tracking>);
    let mut by_ask: BTreeMap<(i64, String), Ask> = BTreeMap::new();
    for ask in asks(text) {
        let Some(asked) = seconds(&ask["at"]) else {
            continue;
        };
        let reached = seconds(&ask["first"]).or_else(|| seconds(&ask["landed"]));
        let words = ask["words"].as_str().unwrap_or_default().to_owned();
        let owner = first_owner(ask["owner"].as_str().unwrap_or_default());
        let entry = by_ask.entry((asked, words)).or_insert((owner, None, None));
        entry.1 = match (entry.1, reached) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        entry.2 = tracking::of(&ask);
    }
    let counted: Vec<(i64, &str, &str, bool)> = by_ask
        .iter()
        .filter(|(_, (.., latest))| *latest == Some(which))
        .filter(|((asked, _), (_, reached, _))| {
            *asked <= now && reached.is_none_or(|reached| reached > now - DAY)
        })
        .map(|((asked, words), (owner, reached, _))| {
            let until = reached.map_or(now, |reached| reached.min(now));
            (
                until - asked,
                words.as_str(),
                owner.as_str(),
                reached.is_none(),
            )
        })
        .collect();
    let mut hours: Vec<f64> = counted
        .iter()
        .map(|(waited, ..)| hours_of(*waited))
        .collect();
    hours.sort_by(f64::total_cmp);
    let mut open: Vec<&(i64, &str, &str, bool)> =
        counted.iter().filter(|(.., open)| *open).collect();
    open.sort_by_key(|(waited, ..)| std::cmp::Reverse(*waited));
    let his = which == Tracking::His;
    Measure {
        key: which.key("ask-to-first-slice"),
        class: Class::Ship,
        says: if his {
            "median hours from his ask to a first piece he can use"
        } else {
            "median hours from an ask the list set itself to its first piece"
        }
        .to_owned(),
        now: median(&hours),
        target: his.then_some(4.0),
        owner: ASKS.to_owned(),
        behind: open
            .iter()
            .take(3)
            .map(|(waited, words, owner, _)| {
                format!(
                    "{} · {owner}, open {:.0} h",
                    short(words),
                    hours_of(*waited)
                )
            })
            .collect(),
    }
}

/// Who keeps the asks ledger, and so steers how fast asks reach him.
pub const ASKS: &str = "WCJQSG";

#[expect(clippy::cast_precision_loss, reason = "seconds stay far below 2^52")]
fn hours_of(seconds: i64) -> f64 {
    seconds as f64 / 3600.0
}

/// The middle of figures already sorted, the mean of the two middle ones
/// when there's an even count, and nought when there are none.
fn median(sorted: &[f64]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2],
        n => f64::midpoint(sorted[n / 2 - 1], sorted[n / 2]),
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

/// The machines over their room while another of their realm has room,
/// from each realm's `mycelium machines` answer (`elsewhere.rs`): work
/// that could go elsewhere waits on the one machine instead.
#[must_use]
pub fn crowded(answers: &[String], owner: &str) -> Measure {
    let read: Vec<_> = answers
        .iter()
        .filter_map(|answer| crate::elsewhere::crowded(answer))
        .collect();
    // An hour no node named its machines in judged nothing, and a quiet 0
    // would read as met: that hid the builders' line for an hour on 16
    // November.
    if read.is_empty() {
        return Measure {
            key: "crowded".to_owned(),
            class: Class::Ship,
            says: "machines over their room while another has room \
                   (no realm's node named its machines)"
                .to_owned(),
            now: 0.0,
            target: None,
            owner: owner.to_owned(),
            behind: Vec::new(),
        };
    }
    let crowded: Vec<String> = read
        .into_iter()
        .flatten()
        .map(|m| {
            format!(
                "{} is over its room ({}) while {} has room",
                m.machine,
                m.over.join(", "),
                m.roomy.join(", ")
            )
        })
        .collect();
    Measure {
        key: "crowded".to_owned(),
        class: Class::Ship,
        says: "machines over their room while another has room".to_owned(),
        now: count(crowded.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: crowded,
    }
}

/// How many first-parent merges `log`, as `%H %ct %s` lines, holds from the hour before `now`.
#[must_use]
pub fn landed(log: &str, now: i64) -> usize {
    log.lines()
        .filter_map(|line| line.split(' ').nth(1)?.parse::<i64>().ok())
        .filter(|at| *at > now - 3600 && *at <= now)
        .count()
}

/// The code-health measures Y9XISP's hourly read writes, each with its
/// target, or none where no target is set yet. Where it says which owner
/// each gained file is, by folder, the files behind it are said with
/// theirs, so a drag reaches who owns the code rather than code-health.
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
                // With nothing gained, code-health gives no one anything, so the
                // measure's own files are said, for its own owner.
                behind: m["by_owner"]
                    .as_object()
                    .filter(|o| !o.is_empty())
                    .map_or_else(
                        || strings(&m["files"]).take(3).map(str::to_owned).collect(),
                        |owners| {
                            owners
                                .iter()
                                .flat_map(|(owner, files)| {
                                    strings(files)
                                        .take(3)
                                        .map(move |f| format!("{f} · {owner}"))
                                })
                                .collect()
                        },
                    ),
            })
        })
        .collect()
}

/// The strings in a JSON list, none where it isn't one.
fn strings(list: &Value) -> impl Iterator<Item = &str> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// The measure under `key` among those a source gave, for its tests to
/// read one by one.
#[cfg(test)]
fn by_key<'a>(measures: &'a [Measure], key: &str) -> &'a Measure {
    measures
        .iter()
        .find(|m| m.key == key)
        .unwrap_or_else(|| panic!("no {key}"))
}

#[cfg(test)]
mod tests {
    use super::card::{CARD_FRESH_FOR, card_median};
    use super::*;

    const NOW: i64 = 1_794_700_000;

    fn open_ask() -> Value {
        serde_json::json!({"id": "0a1b2c3d", "at": super::slipped::at(NOW - 3 * 3600),
            "words": "fold", "owner": "F"})
    }

    #[test]
    fn a_twin_folded_into_an_ask_doesnt_take_its_wait_off_the_median() {
        use super::slipped::{list, twin};
        let open = open_ask();
        let waited = to_first_slice(
            &list(&[open.clone(), twin(&open, NOW - 60)]),
            NOW,
            Tracking::His,
        );
        assert!(
            (waited.now - 3.0).abs() < 0.01,
            "a twin closing an open ask took its wait off the median: {waited:?}"
        );
    }

    #[test]
    fn a_twin_folded_into_a_parked_ask_doesnt_unpark_it() {
        use super::slipped::{list, twin};
        let mut ruled = open_ask();
        ruled["parked"] = "a ruling".into();
        let held = parked(
            &list(&[ruled.clone(), twin(&ruled, NOW - 60)]),
            Tracking::His,
        );
        assert!(
            (held.now - 1.0).abs() < 0.01,
            "a twin closing a parked ask took it off the parked: {held:?}"
        );
    }

    #[test]
    fn a_machine_crowded_beside_one_with_room_is_past_its_target() {
        let answer = r#"{"Ok":{"Machines":[
            {"name":"laptop","room":{"over":["memory","cpu"],"takes":{"builds":0}}},
            {"name":"mac","room":{"over":[],"takes":{"builds":10,"vm_tests":6}}}
        ]}}"#;
        let m = crowded(&[answer.to_owned()], "OWNER");
        assert_eq!(
            (m.now, m.target, m.behind),
            (
                1.0,
                Some(0.0),
                vec!["laptop is over its room (memory, cpu) while mac has room".to_owned()]
            ),
            "a machine over its room beside one with room wasn't counted and named"
        );
    }

    #[test]
    fn an_hour_no_node_named_its_machines_in_is_unjudged_and_says_so() {
        let builders_only = r#"{"Ok":{"Builders":[{"name":"lent"}]}}"#;
        let m = crowded(&[builders_only.to_owned(), "not json".to_owned()], "OWNER");
        assert_eq!(
            (
                m.target,
                m.says.contains("no realm's node named its machines")
            ),
            (None, true),
            "an hour with no machines read passed as none crowded: {} at {:?}",
            m.says,
            m.target
        );
    }

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
    fn a_date_told_late_never_leads_however_far_past_it_is() {
        let measures = [
            measure(told_late::KEY, Class::Slip, 3.0, Some(0.0)),
            measure("dates-told-late-derived", Class::Slip, 5.0, Some(0.0)),
            measure("far", Class::Clean, 30.0, Some(10.0)),
        ];
        assert_eq!(
            drag(&measures).map(|m| m.key.as_str()),
            Some("far"),
            "a date told late, which no one can move now, led the hour"
        );
    }

    #[test]
    fn his_asks_unproven_never_lead_so_a_broken_main_is_still_filed() {
        let measures = [
            measure(unproven::KEY, Class::Functional, 300.0, Some(0.0)),
            measure("broken-on-main", Class::Functional, 1.0, Some(0.0)),
        ];
        assert_eq!(
            drag(&measures).map(|m| m.key.as_str()),
            Some("broken-on-main"),
            "his asks unproven, filed apart already, took the drag from a broken main"
        );
    }

    #[test]
    fn a_count_of_the_way_out_never_leads_however_far_past_it_is() {
        let measures = [
            measure("relay:answered-502", Class::Functional, 704.0, Some(50.0)),
            measure("path:tls:forge.example", Class::Functional, 20.0, Some(1.0)),
            measure("broken", Class::Functional, 4.0, Some(0.0)),
        ];
        assert_eq!(
            drag(&measures).map(|m| m.key.as_str()),
            Some("broken"),
            "the relay, filed apart, took broken main's place as the lead"
        );
    }

    #[test]
    fn an_ask_counts_until_its_first_slice_and_an_open_one_counts_its_age() {
        let ask = |words: &str, asked: i64, first: Option<i64>, landed: Option<i64>| {
            serde_json::json!({
                "at": at(asked), "words": words, "owner": "Y2YBTS (H6EDLC)",
                "expected": at(NOW + 3600), "first": first.map(at),
                "landed": landed.map(at),
            })
            .to_string()
        };
        let text = [
            ask("landed whole", NOW - 2 * 3600, None, Some(NOW - 3600)),
            ask(
                "a slice first",
                NOW - 10 * 3600,
                Some(NOW - 7 * 3600),
                Some(NOW - 60),
            ),
            ask("still open", NOW - 30 * 3600, None, None),
            ask("still open", NOW - 30 * 3600, None, None), // its date moved
            ask("long done", NOW - 3 * DAY, None, Some(NOW - 2 * DAY)),
        ]
        .join("\n");
        let m = to_first_slice(&text, NOW, Tracking::His);
        assert!(
            (m.now - 3.0).abs() < 0.01,
            "the median of 1, 3 and 30 hours, got {}",
            m.now
        );
        assert_eq!(m.target, Some(4.0), "within hours");
        assert_eq!(m.class, Class::Ship, "it's how fast things ship");
        assert_eq!(m.owner, "WCJQSG", "the asks ledger's keeper steers it");
        assert_eq!(
            m.behind,
            ["still open · Y2YBTS, open 30 h"],
            "the oldest open asks, each once"
        );
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
    fn a_card_the_tick_stopped_drawing_measures_nothing() {
        let card = "Merge to on (target median 18): none.\n  not on yet: #5452 133 min\n";
        assert_eq!(
            card_if_fresh(card.to_owned(), Some(1_000), 1_000 + CARD_FRESH_FOR),
            card,
            "a card three ticks old is still read"
        );
        let stale = card_if_fresh(card.to_owned(), Some(1_000), 1_001 + CARD_FRESH_FOR);
        assert!(
            merge_to_on(&stale, "80ECO2").is_none(),
            "one older says nothing of merges, rather than its old waits"
        );
        assert!(
            card_if_fresh(card.to_owned(), None, 1_000).is_empty()
                && card_if_fresh(card.to_owned(), Some(1_001), 1_000).is_empty(),
            "nor one with no time drawn, or drawn after now"
        );
    }

    #[test]
    fn with_no_switch_merge_to_on_reads_the_merges_still_waiting() {
        let card = "Card at 21:55\nMerge to on (target median 18): none.\n  not on yet: #4534 2 \
                    min, #4533 14 min, #4518 27 min, #4495 144 min, #4356 145 min\n  \
                    switches: 0, last at none\n";
        let measure = merge_to_on(card, "80ECO2").expect("waits are read");
        assert_eq!(
            (measure.now, measure.target),
            (27.0, Some(18.0)),
            "the waits' median, against the card's target"
        );
        assert_eq!(
            measure.behind,
            [
                "#4356 waiting 145 min",
                "#4495 waiting 144 min",
                "#4518 waiting 27 min"
            ],
            "the three longest waits"
        );
        let switched = "Merge to on (target median 18): median 37.9, p90 47.7, max 47.7 min \
                        (n 6).\n  not on yet: #4534 2 min\n";
        let measure = merge_to_on(switched, "80ECO2").expect("a median is read");
        assert_eq!(
            (measure.now, measure.target),
            (37.9, Some(18.0)),
            "the card's own median where a switch landed"
        );
        assert_eq!(measure.behind, ["#4534 waiting 2 min"], "what still waits");
        assert!(
            merge_to_on("Merge to on (target median 18): none.\n", "80ECO2").is_none(),
            "nothing merged, nothing to read"
        );
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

    #[test]
    fn code_health_files_by_owner_are_each_said_with_their_owner() {
        let text = r#"{"measures":[{"repo":"core.nix","measure":"suppressions","now":12,
            "target":10,"owner":"Y9XISP","files":["net/a.rs","net/b.rs","x.rs"],
            "by_owner":{"PN2VZC":["net/a.rs","net/b.rs"],"Y9XISP":["x.rs"]}}]}"#;
        assert_eq!(
            clean(text)[0].behind,
            ["net/a.rs · PN2VZC", "net/b.rs · PN2VZC", "x.rs · Y9XISP"],
            "each file names its own owner, so each hears of its own"
        );
        let nothing_gained = r#"{"measures":[{"repo":"core.nix","measure":"suppressions",
            "now":12,"target":10,"owner":"Y9XISP","files":["x.rs"],"by_owner":{}}]}"#;
        assert_eq!(
            clean(nothing_gained)[0].behind,
            ["x.rs"],
            "with no file gained there's no one to give it to, so its files are said bare"
        );
    }
}
