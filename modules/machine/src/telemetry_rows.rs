//! The Processor's Held Back and Kept From Resting, from its own
//! telemetry ([`crate::telemetry`]), ready for the launcher beside its
//! other parts (`processor_rows.rs`): what held it back over the last
//! second, the whole processor's limit where one did and else its most
//! held cluster's, each core held a step in; and what kept the package
//! from resting over the last ten seconds, each part that did a step in.
//! They're what the launcher's `Machine.qml` and `Machine.js` made of
//! the shell's own `ProcessorTelemetry.qml`, moved here so the shell
//! words, averages and sorts nothing (Joel, 2026-11-10: "the daemons
//! need to be the data layer").
//!
//! Both stand while the telemetry is asked, held or not, so the rows
//! under them never move under Joel's selection (VKCF3Z). Kept From
//! Resting holds its place saying it's still reading until the first
//! second comes, and stays there saying it can't be read if the reader
//! refuses; on a machine with no reader it's left out.

use serde_json::Value;

use crate::machine_rows::{Say, number, percent};
use crate::rows::{Made, bar};
use crate::telemetry::{Heard, Standing};

/// The least share the reader says a part kept the package from resting
/// for, a twentieth, and so the least a part is named for.
const RESTLESS: f64 = 0.05;
/// Each part the reader says kept the package from resting, by its
/// name, and its own row's id under Kept From Resting.
const KEEPERS: [(&str, &str); 9] = [
    ("npu", "npu"),
    ("imageProcessor", "image-processor"),
    ("media", "media-engine"),
    ("graphics", "graphics"),
    ("display", "display-engine"),
    ("chipset", "chipset"),
    ("cores", "cores"),
    ("internal", "power-controller"),
    ("other", "other"),
];

/// Held Back and Kept From Resting, as the telemetry stands.
pub(crate) fn rows(say: &Say, heard: &Heard) -> Vec<Made> {
    let mut rows = Vec::new();
    if let Some(reading) = heard.reading() {
        rows.push(held_back(say, reading));
    }
    rows.extend(kept_from_resting(say, heard));
    rows
}

/// A limit that held something back, and for what share of the second.
#[derive(Clone, Debug, PartialEq)]
struct Hold {
    share: f64,
    by: String,
}

/// The limit that held most of these, the first where two held as long.
fn strongest(causes: &Value) -> Option<Hold> {
    causes
        .as_array()?
        .iter()
        .map(|cause| Hold {
            share: number(&cause["share"]),
            by: cause["by"].as_str().unwrap_or("other").to_owned(),
        })
        .reduce(|most, next| if most.share >= next.share { most } else { next })
}

/// The CPU numbers a list such as "0-3,8" names, as the reader names a
/// cluster's.
fn cpus(listed: &str) -> Vec<u64> {
    let mut each = Vec::new();
    for range in listed.split(',').map(str::trim) {
        let (first, last) = range.split_once('-').unwrap_or((range, range));
        if let (Ok(first), Ok(last)) = (first.trim().parse::<u64>(), last.trim().parse::<u64>()) {
            each.extend(first..=last);
        }
    }
    each
}

/// What held the processor back over the last second: a step in to each
/// core held, by its number, with what held its cluster most, where any
/// was. A limit under a twentieth isn't said by the reader, so a core
/// missing wasn't held.
fn held_back(say: &Say, reading: &Value) -> Made {
    let mut cores: Vec<(u64, Hold)> = Vec::new();
    for cluster in reading["clusters"].as_array().into_iter().flatten() {
        if let Some(most) = strongest(&cluster["heldBack"]) {
            let listed = cluster["cpus"].as_str().unwrap_or_default();
            cores.extend(cpus(listed).into_iter().map(|n| (n, most.clone())));
        }
    }
    cores.sort_by_key(|(n, _)| *n);
    let most = strongest(&reading["heldBack"]).or_else(|| {
        cores
            .iter()
            .map(|(_, hold)| hold.clone())
            .reduce(|most, next| if most.share >= next.share { most } else { next })
    });
    let share = most.as_ref().map_or(0.0, |most| most.share);
    let note = most.as_ref().map_or_else(
        || say.says("cpu:held-back", "none", &[]),
        |most| {
            let limit = limit(say, &most.by);
            say.says("cpu:held-back", "note", &[("limit", &limit)])
        },
    );
    let made = say.figure(
        "cpu:held-back",
        percent(share),
        Some(bar(share, false)),
        note,
    );
    if cores.is_empty() {
        return made;
    }
    let inside = cores
        .iter()
        .map(|(n, hold)| {
            let n = n.to_string();
            let limit = limit(say, &hold.by);
            Made {
                name: say.words.name("machine:cpu:held-back-core", &[("n", &n)]),
                ..say.figure(
                    &format!("cpu:held-back-core:{n}"),
                    percent(hold.share),
                    Some(bar(hold.share, false)),
                    say.says("cpu:held-back", "core", &[("limit", &limit)]),
                )
            }
        })
        .collect();
    say.holder(made, "cpu", inside)
}

/// A limit as a note says it after "by": the whole processor's by the
/// names Intel gives them, a cluster's by the kind of limit it was, and
/// one the words don't know by the reader's own name for it.
fn limit(say: &Say, by: &str) -> String {
    let said = say.says("cpu:held-back", &format!("limit-{by}"), &[]);
    if said.is_empty() { by.to_owned() } else { said }
}

/// A part that kept the package from resting, by the reader's name: its
/// own row's id under Kept From Resting.
fn keeper(by: &str) -> &'static str {
    KEEPERS
        .iter()
        .find(|(name, _)| *name == by)
        .map_or("other", |(_, id)| id)
}

/// What kept the package from resting over the last readings: the share
/// of that time anything did, on average, and a step in to each part's
/// share of it, longest first, leaving out those under a twentieth.
/// Averaged, a share near a twentieth doesn't jump each second.
fn kept_from_resting(say: &Say, heard: &Heard) -> Option<Made> {
    let id = "cpu:kept-from-resting";
    let rested = heard.rested();
    if rested.is_empty() {
        return match heard.standing() {
            Standing::Asking => Some(Made {
                copies: String::new(),
                ..say.figure(id, "\u{2026}".to_owned(), None, String::new())
            }),
            Standing::Refused => Some(Made {
                copies: String::new(),
                ..say.figure(id, String::new(), None, say.says(id, "refused", &[]))
            }),
            Standing::Heard | Standing::Missing => None,
        };
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "ten readings at most, far under f64's exact integers"
    )]
    let readings = rested.len() as f64;
    let share = rested.iter().map(|each| number(&each["kept"])).sum::<f64>() / readings;
    let mut parts: Vec<(String, f64)> = Vec::new();
    for reading in rested {
        for part in reading["by"].as_array().into_iter().flatten() {
            let by = part["by"].as_str().unwrap_or("other");
            let share = number(&part["share"]);
            match parts.iter_mut().find(|(name, _)| name == by) {
                Some((_, sum)) => *sum += share,
                None => parts.push((by.to_owned(), share)),
            }
        }
    }
    let mut parts: Vec<(String, f64)> = parts
        .into_iter()
        .map(|(by, sum)| (by, sum / readings))
        .filter(|(_, share)| *share >= RESTLESS)
        .collect();
    parts.sort_by(|a, b| b.1.total_cmp(&a.1));
    let note = match parts.first() {
        Some((by, _)) => {
            let part = say.says(&format!("cpu:kept-by-{}", keeper(by)), "by", &[]);
            say.says(id, "by", &[("keeper", &part)])
        }
        None if share < RESTLESS => say.says(id, "quiet", &[]),
        None => say.says(id, "spread", &[]),
    };
    let made = say.figure(id, percent(share), Some(bar(share, false)), note);
    if parts.is_empty() {
        return Some(made);
    }
    let inside = parts
        .iter()
        .map(|(by, share)| {
            say.figure(
                &format!("cpu:kept-by-{}", keeper(by)),
                percent(*share),
                Some(bar(*share, false)),
                String::new(),
            )
        })
        .collect();
    Some(say.holder(made, "cpu", inside))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::words::Words;

    /// The words these cases say: the launcher's own (`Words::launchers`).
    fn words() -> Words {
        Words::launchers(",", ".")
    }

    fn said(made: &Made) -> &str {
        made.side[0]["said"].as_str().unwrap_or_default()
    }

    fn ids(made: &[Made]) -> Vec<&str> {
        made.iter().map(|made| made.id.as_str()).collect()
    }

    #[test]
    fn a_cpu_list_names_each_cpu() {
        assert_eq!(cpus("0-3,8"), [0, 1, 2, 3, 8]);
        assert_eq!(cpus(" 4 "), [4]);
        assert!(cpus("").is_empty(), "none listed");
    }

    #[test]
    fn held_back_says_the_whole_processors_limit_and_each_core_its_clusters() {
        let words = words();
        let say = Say { words: &words };
        let mut heard = Heard::default();
        heard.heard(Some(json!({
            "heldBack": [{ "by": "pl1", "share": 0.4 }],
            "clusters": [
                { "cpus": "2-3", "heldBack": [{ "by": "heat", "share": 0.2 },
                    { "by": "power", "share": 0.1 }] },
                { "cpus": "0", "heldBack": [{ "by": "heat", "share": 0.3 }] },
                { "cpus": "4-7", "heldBack": [] }
            ]
        })));
        let rows = rows(&say, &heard);
        assert_eq!(
            ids(&rows),
            ["cpu:held-back"],
            "nothing said of resting is left out"
        );
        let held = &rows[0];
        assert_eq!(said(held), "40%");
        assert_eq!(held.note, "by its power limit, PL1, over the last second");
        assert_eq!(
            held.glyph, "processor",
            "a holder wears the processor's glyph"
        );
        assert_eq!(
            held.inside
                .iter()
                .map(|core| (core.name.as_str(), said(core), core.note.as_str()))
                .collect::<Vec<_>>(),
            [
                ("Core 0", "30%", "by heat"),
                ("Core 2", "20%", "by heat"),
                ("Core 3", "20%", "by heat")
            ],
            "each core held, by its number, with its cluster's strongest limit"
        );
    }

    #[test]
    fn held_back_by_nothing_still_stands_and_a_cluster_says_when_the_whole_doesnt() {
        let words = words();
        let say = Say { words: &words };
        let mut heard = Heard::default();
        heard.heard(Some(json!({ "heldBack": [], "clusters": [] })));
        let idle = &rows(&say, &heard)[0];
        assert_eq!(
            (said(idle), idle.note.as_str()),
            ("0%", "nothing holds it back now"),
            "it stands, so the rows under it never move"
        );
        assert!(
            idle.inside.is_empty(),
            "nothing holding it back names no core"
        );
        heard.heard(Some(json!({ "heldBack": [], "clusters": [
            { "cpus": "1", "heldBack": [{ "by": "current", "share": 0.25 }] }
        ] })));
        let cluster = &rows(&say, &heard)[0];
        assert_eq!(said(cluster), "25%");
        assert_eq!(
            cluster.note, "by current, over the last second",
            "a limit the words don't know by the reader's own name"
        );
    }

    #[test]
    fn kept_from_resting_is_the_mean_of_the_last_seconds_each_part_longest_first() {
        let words = words();
        let say = Say { words: &words };
        let mut heard = Heard::default();
        for (npu, graphics) in [(0.2, 0.3), (0.4, 0.3), (0.0, 0.3)] {
            heard.heard(Some(
                json!({ "keptFromResting": { "kept": npu + graphics, "by": [
                { "by": "npu", "share": npu }, { "by": "graphics", "share": graphics },
                { "by": "media", "share": 0.01 }
            ] } }),
            ));
        }
        let rows = rows(&say, &heard);
        let kept = rows
            .iter()
            .find(|row| row.id == "cpu:kept-from-resting")
            .expect("kept from resting");
        assert_eq!(said(kept), "50%", "the three seconds' mean");
        assert_eq!(
            kept.note, "by the graphics",
            "the part that kept it longest"
        );
        assert_eq!(
            ids(&kept.inside),
            ["cpu:kept-by-graphics", "cpu:kept-by-npu"],
            "longest first, and none under a twentieth"
        );
        assert_eq!(said(&kept.inside[1]), "20%");
    }

    #[test]
    fn kept_from_resting_holds_its_place_while_asked_and_says_a_refusal() {
        let words = words();
        let say = Say { words: &words };
        let mut heard = Heard::default();
        let asking = rows(&say, &heard);
        assert_eq!(ids(&asking), ["cpu:kept-from-resting"]);
        assert_eq!(said(&asking[0]), "\u{2026}", "its place held");
        assert_eq!(asking[0].copies, "", "a placeholder copies nothing");
        heard.heard(None);
        assert!(
            rows(&say, &heard).is_empty(),
            "a machine with no reader says none"
        );
        heard.heard(Some(
            json!({ "keptFromResting": { "kept": 0.01, "by": [] } }),
        ));
        let quiet = rows(&say, &heard);
        assert_eq!(quiet[1].note, "nothing keeps it from resting now");
        heard.heard(None);
        let refused = rows(&say, &heard);
        assert_eq!(
            ids(&refused),
            ["cpu:kept-from-resting"],
            "Held Back goes with its reading"
        );
        assert_eq!(
            (said(&refused[0]), refused[0].note.as_str()),
            ("", "can't be read"),
            "it stays where it was, saying why"
        );
    }
}
