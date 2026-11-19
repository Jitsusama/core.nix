//! Whether a miss whose check merged is still held open by a measure the
//! check names: a merge closes a miss only once that measure judges it gone.

use super::{Class, Measure, first_owner, lines, named_as};

/// The keys of the hour's measures a check can name to hold its miss.
/// Only a key with a hyphen is one, since a bare key such as broken is also
/// a word a check's prose uses. They are listed rather than read from the
/// hour, so a measure the hour didn't make still holds its miss.
const JUDGES: &[&str] = &[
    "ask-to-first-slice",
    "cells-idle",
    "cells-parked-done",
    "commit-to-merge",
    "map-cells",
    "merge-to-on",
    "mycelium-to-lock",
    "pains-open",
    "quarantine-undated",
    "quests-cells-ended",
    "quests-parked-past-start",
    "quests-without-cells",
    "reports-unasked",
];

/// What a miss's card line carries after its owner once its check merged
/// and a measure still holds it open; the card and the owner's note read it.
pub const HELD: &str = ", though its check ";

/// Why a measure holds a miss when it couldn't look: the hour's own
/// blindness, which the miss's owner can't move, so it isn't counted.
pub const BLIND: [&str; 2] = ["wasn't read this hour", "can't judge it yet"];

/// Holds open each miss whose check names one of the hour's measures by
/// its key, until that measure is judged and within its target: a merged
/// check that can't judge yet, or still finds the fault, hasn't fixed it.
/// At 02:10 on 17 November the card told Joel his 20:37 miss was fixed on
/// core #5252's merge, while its map-cells read unjudged until the map's own
/// check merged. A key is one of `JUDGES`, or any hyphenated key the hour
/// made, so a new measure holds its miss before the list names it, named
/// whole in any case; one of `JUDGES` the hour didn't make, as on a look by
/// hand with no realm, holds it too. A zone's keys, `clean:repo:measure`,
/// are named by no check. Only a miss on last hour's open lines, `was_open`,
/// is held: one closed before was judged gone, so a later drift of its
/// measure for another cause doesn't reopen it. A hold counts only while
/// its measure still finds the fault; one `BLIND` stays on the card
/// uncounted, so the hour's own outage can't become the drag.
pub fn held(text: &str, measures: &mut [Measure], was_open: &[String]) {
    let words = |line: &str| line.rsplit_once(" · ").map_or(line, |(w, _)| w).to_owned();
    let was_open: Vec<String> = was_open.iter().map(|line| words(line)).collect();
    let mut keys: Vec<String> = JUDGES.iter().map(|&key| key.to_owned()).collect();
    for m in measures.iter() {
        if m.key.contains('-') && !m.key.contains(':') && !keys.contains(&m.key) {
            keys.push(m.key.clone());
        }
    }
    let judging: Vec<(&str, &str)> = keys
        .iter()
        .map(String::as_str)
        .filter_map(|key| {
            let why = match measures.iter().find(|m| m.key == key) {
                None => BLIND[0],
                Some(m) => match m.target {
                    None => BLIND[1],
                    Some(target) if m.now > target => "still finds it",
                    Some(_) => return None,
                },
            };
            Some((key, why))
        })
        .collect();
    let Some(open) = measures.iter_mut().find(|m| m.class == Class::Miss) else {
        return;
    };
    for miss in lines(text) {
        let Some(check) = miss["check"].as_str() else {
            continue;
        };
        let what = named_as(&miss);
        let already = open.behind.iter().any(|line| words(line) == what)
            || !was_open.iter().any(|w| w == what);
        let check = check.to_lowercase();
        // A key that still finds the fault outranks one the hour couldn't
        // read, so a blind key named first doesn't mask the real hold.
        let found = judging
            .iter()
            .filter(|(key, _)| names_whole(&check, key))
            .min_by_key(|(_, why)| BLIND.contains(why));
        let (false, Some((key, why))) = (already, found) else {
            continue;
        };
        let owner = first_owner(miss["owner"].as_str().unwrap_or_default());
        open.behind
            .push(format!("{what} · {owner}{HELD}{key} {why}"));
        if BLIND.contains(why) {
            continue;
        }
        open.now += 1.0;
        if open.owner.is_empty() {
            open.owner = owner;
        }
    }
}

/// Whether `text` names `key` as a whole word, not inside a longer one.
fn names_whole(text: &str, key: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    text.match_indices(key).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(part)
            && !text[at + key.len()..].chars().next().is_some_and(part)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::super::misses;
    use super::super::tests::{change, miss, repos};
    use super::*;
    use crate::hour::measures::{cells, mapped, pains, unasked};

    /// Last hour's open misses, as the card kept them, with `miss`'s open.
    fn was_open() -> Vec<String> {
        vec!["the cat waits · XY9H5C".to_owned()]
    }

    fn judged(key: &str, now: f64, target: Option<f64>) -> Measure {
        Measure {
            key: key.to_owned(),
            class: Class::Cells,
            says: "cells the map draws wrongly".to_owned(),
            now,
            target,
            owner: "WCJQSG".to_owned(),
            behind: Vec::new(),
        }
    }

    /// At 02:10 on 17 November the card told Joel his 20:37 miss was fixed
    /// once core #5252 merged, while map-cells, the measure that would catch
    /// it, read unjudged until the map's own check merged.
    #[test]
    fn a_merged_check_whose_measure_cannot_judge_or_still_finds_it_keeps_its_miss_open() {
        let text = miss("core #5252 map-cells: each cell the map draws wrongly is a line");
        let merged = BTreeSet::from([change("core", 5252)]);
        // A measure blind for the hour's own reasons holds the miss on the
        // card but doesn't count it, so an outage can't become the drag.
        let cases = [
            (None, 0.0, "can't judge it yet", 0.0),
            (Some(0.0), 2.0, "still finds it", 1.0),
        ];
        for (target, now, why, counted) in cases {
            let mut hour = vec![
                misses(&text, &repos(), &merged, &BTreeSet::new()),
                judged("map-cells", now, target),
            ];
            held(&text, &mut hour, &was_open());
            assert_eq!(
                hour[0].behind,
                [format!(
                    "the cat waits · XY9H5C, though its check map-cells {why}"
                )],
                "a miss read fixed while the measure that catches it read {now} of {target:?}"
            );
            assert!(
                (hour[0].now - counted).abs() < 0.5,
                "a miss held because its measure {why} counted {} rather than {counted}",
                hour[0].now
            );
        }
        let mut hour = vec![
            misses(&text, &repos(), &merged, &BTreeSet::new()),
            judged("map-cells", 0.0, Some(0.0)),
        ];
        held(&text, &mut hour, &was_open());
        assert!(
            hour[0].behind.is_empty(),
            "a merged check whose measure judges and reads within target kept its miss: {:?}",
            hour[0].behind
        );
    }

    #[test]
    fn a_measure_it_names_that_the_hour_never_read_keeps_its_miss_open() {
        // Row 38's check, as the orchestrator wrote it.
        let text = miss(
            "core #5252 (ebbfb6a3d617) reports-unasked: a report in draft or build of an \
             active quest, 2 h to 3 d old, with no asking look naming it, is a line on the \
             hourly card; live from the next switch",
        );
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![misses(&text, &repos(), &merged, &BTreeSet::new())];
        held(&text, &mut hour, &was_open());
        assert_eq!(
            hour[0].behind,
            ["the cat waits · XY9H5C, though its check reports-unasked wasn't read this hour"],
            "a miss read fixed though the measure it names wasn't on the card"
        );
        let shouted = miss("core #5252 Map-Cells");
        let mut hour = vec![
            misses(&shouted, &repos(), &merged, &BTreeSet::new()),
            judged("map-cells", 0.0, None),
        ];
        held(&shouted, &mut hour, &was_open());
        assert_eq!(
            hour[0].behind.len(),
            1,
            "a key written in capitals didn't hold its miss"
        );
    }

    #[test]
    fn a_measure_the_hour_makes_but_the_list_lacks_still_holds_its_miss() {
        let text = miss("core #5252 pains-untaken: a pain no owner took is a line");
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![
            misses(&text, &repos(), &merged, &BTreeSet::new()),
            judged("pains-untaken", 2.0, Some(0.0)),
        ];
        held(&text, &mut hour, &was_open());
        assert_eq!(
            hour[0].behind,
            ["the cat waits · XY9H5C, though its check pains-untaken still finds it"],
            "a new measure's key, missing from the list, let its miss close on merge"
        );
    }

    /// A miss closed before, told fixed or closed before the card told
    /// anything, was judged gone; its measure drifting later for another
    /// cause doesn't reopen it, nor does an hour.json from before `told`.
    #[test]
    fn a_miss_not_open_last_hour_stays_closed_when_its_measure_drifts_later() {
        let text = miss("core #5252 map-cells");
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![
            misses(&text, &repos(), &merged, &BTreeSet::new()),
            judged("map-cells", 2.0, Some(0.0)),
        ];
        held(&text, &mut hour, &["another miss · WCJQSG".to_owned()]);
        assert!(
            hour[0].behind.is_empty(),
            "a miss closed before last hour was reopened by its measure's drift: {:?}",
            hour[0].behind
        );
        // Held last hour, it's open last hour, and stays held.
        let held_line = "the cat waits · XY9H5C, though its check map-cells still finds it";
        held(&text, &mut hour, &[held_line.to_owned()]);
        assert_eq!(
            hour[0].behind,
            [held_line],
            "a miss held last hour wasn't held again"
        );
    }

    /// A check naming a measure the hour couldn't read and one that still
    /// finds the fault is held by the one that finds it, counted and owned.
    #[test]
    fn a_key_that_still_finds_the_fault_outranks_a_blind_one() {
        let text = miss("core #5252 cells-idle and map-cells");
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![
            misses(&text, &repos(), &merged, &BTreeSet::new()),
            judged("cells-idle", 0.0, None),
            judged("map-cells", 2.0, Some(0.0)),
        ];
        held(&text, &mut hour, &was_open());
        assert_eq!(
            (hour[0].behind.clone(), hour[0].now),
            (
                vec![
                    "the cat waits · XY9H5C, though its check map-cells still finds it".to_owned()
                ],
                1.0
            ),
            "a blind key masked the one that still finds the fault"
        );
    }

    #[test]
    fn a_zones_key_holds_no_miss() {
        let text = miss("core #5252 clean:core-nix:long-files");
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![
            misses(&text, &repos(), &merged, &BTreeSet::new()),
            judged("clean:core-nix:long-files", 9.0, Some(0.0)),
        ];
        held(&text, &mut hour, &was_open());
        assert!(
            hour[0].behind.is_empty(),
            "a zone's key, which no check names, held a miss: {:?}",
            hour[0].behind
        );
    }

    #[test]
    fn every_key_the_cell_and_pain_measures_make_is_one_a_check_can_name() {
        let made: Vec<Measure> = cells(&[], None, "", &jiff::tz::TimeZone::UTC, 0, "")
            .into_iter()
            .chain([
                mapped(None, "", ""),
                unasked(&[], None, None, 0, ""),
                pains(None, 0, ""),
            ])
            .collect();
        for m in made {
            assert!(
                JUDGES.contains(&m.key.as_str()),
                "{} is made by the hour but a check naming it wouldn't be held on an hour \
                 that couldn't make it",
                m.key
            );
        }
    }

    #[test]
    fn only_a_key_with_a_hyphen_named_whole_holds_a_miss_and_an_open_one_once() {
        let words = miss(
            "core #5252 fixes what broken cells did; map-cellsx; xmap-cells; map-cells_; \
             reports-unasked:",
        );
        let merged = BTreeSet::from([change("core", 5252)]);
        let mut hour = vec![
            misses(&words, &repos(), &merged, &BTreeSet::new()),
            judged("broken", 3.0, Some(0.0)),
            judged("map-cells", 3.0, Some(0.0)),
            judged("reports-unasked", 0.0, Some(0.0)),
        ];
        held(&words, &mut hour, &was_open());
        assert!(
            hour[0].behind.is_empty(),
            "a plain word, a key inside a longer word, or a judged key held a miss: {:?}",
            hour[0].behind
        );
        let open = miss("core #9 map-cells");
        let mut hour = vec![
            misses(&open, &repos(), &BTreeSet::new(), &BTreeSet::new()),
            judged("map-cells", 0.0, None),
        ];
        held(&open, &mut hour, &was_open());
        assert_eq!(
            hour[0].behind,
            ["the cat waits · XY9H5C"],
            "a miss already open was held a second time"
        );
    }
}
