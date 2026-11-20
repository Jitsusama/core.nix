//! The hour's card: each measure against its target, the one biggest
//! drag, what became of last hour's, and Joel's misses, open or fixed;
//! and what the next hour keeps of it to say so.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use super::lines::{figure, shown_lines};
use super::measures::{Class, HELD, Measure, UNRUN, UNSEEN, drag};
use super::owing::by_owner;

/// How many hours a drag may stay unmoved before the orchestrator is
/// asked to steer it.
const UNMOVED_FOR: u32 = 2;

/// What the hour remembers until the next: its drag, and the misses that
/// were open, so one that closes is told once.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default)]
    pub drag: Option<Held>,
    #[serde(default)]
    pub open: Vec<String>,
    /// The misses Joel has been told are fixed, by their words, so a miss
    /// held open an hour by a node that didn't answer isn't told twice.
    #[serde(default)]
    pub told: Vec<String>,
    /// When the hour itself last took mycelium's main, oldest first, so a
    /// train's merge waits from the fetch that saw main without it.
    #[serde(default)]
    pub fetched: Vec<i64>,
}

/// How many misses told fixed the hour keeps, newest last.
const TOLD_KEPT: usize = 300;

/// The drag as an hour named it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Held {
    pub key: String,
    pub says: String,
    pub owner: String,
    pub now: f64,
    /// How many hours in a row it hasn't moved.
    pub unmoved: u32,
}

/// The hour's card, at `at` and with the drag due at `due`, and what the
/// next hour is to remember. `said` is each miss in Joel's own words and
/// when, by its text, for telling him one is fixed.
#[must_use]
pub fn compose(
    at: &str,
    measures: &[Measure],
    kept: &Kept,
    said: &BTreeMap<String, String>,
    due: &str,
) -> (String, Kept) {
    let mut card = format!("cadence hour at {at}\n");
    for class in [
        Class::Apps,
        Class::Ship,
        Class::Miss,
        Class::Slip,
        Class::Functional,
        Class::Pains,
        Class::Cells,
        Class::Clean,
    ] {
        let of_class: Vec<&Measure> = measures.iter().filter(|m| m.class == class).collect();
        let _ = writeln!(card, "{}:", capitalised(class.name()));
        if of_class.is_empty() {
            let _ = writeln!(card, "  not read yet");
        }
        for l in shown_lines(class, &of_class) {
            let _ = writeln!(card, "  {l}");
        }
    }
    let (followed, unmoved) = follow(kept.drag.as_ref(), measures);
    let chosen = drag(measures);
    match chosen {
        Some(m) => {
            let _ = writeln!(
                card,
                "Drag: {}, {} against at most {}, to {} by {due}.",
                m.says,
                figure(m.now),
                figure(m.target.unwrap_or_default()),
                if m.class == Class::Miss {
                    "each miss's owner"
                } else if !by_owner(m).is_empty() {
                    if m.class == Class::Cells {
                        "each cell's owner"
                    } else {
                        "each file's owner"
                    }
                } else {
                    owner_or_none(&m.owner)
                }
            );
        }
        None => {
            let _ = writeln!(card, "Drag: none, every judged measure is in.");
        }
    }
    if let Some(followed) = followed {
        let _ = writeln!(card, "{followed}");
    }
    let open: Vec<String> = measures
        .iter()
        .filter(|m| m.class == Class::Miss)
        .flat_map(|m| m.behind.clone())
        .collect();
    if !open.is_empty() {
        let _ = writeln!(
            card,
            "Why didn't our checks catch these? Each stays open until its owner names the \
             check that would have, it merges, and any measure it names judges it gone:"
        );
        for miss in &open {
            let _ = writeln!(card, "  - {miss}");
        }
    }
    // An hour that couldn't judge the misses can't say which are gone, so
    // it tells Joel nothing and carries last hour's open misses whole, with
    // any it saw open since, so one that closes later is still told fixed.
    let judged = measures
        .iter()
        .all(|m| m.class != Class::Miss || m.target.is_some());
    let (open, mut told) = if judged {
        let told = tell_fixed(&mut card, kept, &open, said);
        (open, told)
    } else {
        let told = still_told(kept, &open);
        let mut carried = kept.open.clone();
        carried.extend(open.into_iter().filter(|miss| !kept.open.contains(miss)));
        (carried, told)
    };
    let next = Kept {
        drag: chosen.map(|m| Held {
            key: m.key.clone(),
            says: m.says.clone(),
            // Each miss has its own owner, the oldest's no more than the rest.
            owner: if m.class == Class::Miss {
                "each miss's owner".to_owned()
            } else {
                m.owner.clone()
            },
            now: m.now,
            unmoved: kept
                .drag
                .as_ref()
                .filter(|held| held.key == m.key)
                .map_or(0, |_| unmoved),
        }),
        open,
        // The ledger holds some forty misses; the newest few hundred told
        // are kept, so the file can't grow without end.
        told: told.split_off(told.len().saturating_sub(TOLD_KEPT)),
        fetched: kept.fetched.clone(),
    };
    (card, next)
}

/// Writes a line for Joel for each miss gone since last hour, and gives
/// back every miss told so far. A miss is its words: one handed to a new
/// owner, or noted as unread, is still open, so only words gone from the
/// open misses are fixed. One already told isn't told again, though an
/// hour the node didn't answer held it open between, nor twice in an hour
/// when two owners held the same words. One the ledger reopens, open with
/// no measure's hold on it, leaves what was told, so it's told again once
/// it closes anew.
fn tell_fixed(
    card: &mut String,
    kept: &Kept,
    open: &[String],
    said: &BTreeMap<String, String>,
) -> Vec<String> {
    let still: Vec<&str> = open.iter().map(|miss| miss_words(miss)).collect();
    let mut told = still_told(kept, open);
    for miss in &kept.open {
        let what = miss_words(miss);
        if still.contains(&what) || told.iter().any(|t| t == what) {
            continue;
        }
        told.push(what.to_owned());
        // Joel works on many things at once, so the line says which thing
        // of his it was, that it's fixed, and that nothing is asked of him.
        let thing = said
            .get(what)
            .cloned()
            .unwrap_or_else(|| format!("something you ran into, {what}"));
        let _ = writeln!(
            card,
            "For Joel: {thing}, is fixed, and a check now catches it if it comes back. \
             Nothing to do."
        );
    }
    told
}

/// The misses already told fixed that stay told: one seen open again, and
/// not held, is forgotten, so its next fix is told too.
fn still_told(kept: &Kept, open: &[String]) -> Vec<String> {
    let reopened: Vec<&str> = open
        .iter()
        // Only the part after the owner's mark says it's held, since a
        // miss's own words might hold the same phrase. One held because its
        // check doesn't run here yet was told fixed too soon, as at 21:10 on
        // 19 November, so it's told again once it runs. One held only because
        // the hour can't see what runs stays told, so a blind hour doesn't
        // tell a long-fixed miss fixed again.
        .filter(|miss| {
            !miss.rsplit_once(" · ").is_some_and(|(_, by)| {
                by.contains(HELD)
                    && by
                        .split_once(UNRUN)
                        .is_none_or(|(_, why)| why.starts_with(UNSEEN))
            })
        })
        .map(|miss| miss_words(miss))
        .collect();
    kept.told
        .iter()
        .filter(|what| !reopened.contains(&what.as_str()))
        .cloned()
        .collect()
}

/// What became of last hour's drag, and how many hours it has now gone
/// unmoved: back in, moved down, or not moved, which past two hours asks
/// the orchestrator to steer it.
fn follow(last: Option<&Held>, measures: &[Measure]) -> (Option<String>, u32) {
    let Some(last) = last else {
        return (None, 0);
    };
    // A measure that couldn't be read this hour says nothing of the drag.
    let now = measures
        .iter()
        .find(|m| m.key == last.key && m.target.is_some());
    match now {
        Some(m) if m.past().is_none() => (
            Some(format!("Last hour's drag, {}, is back in.", last.says)),
            0,
        ),
        None => (
            Some(format!(
                "Last hour's drag, {}, wasn't read this hour.",
                last.says
            )),
            last.unmoved,
        ),
        Some(m) if m.now < last.now => (
            Some(format!(
                "Last hour's drag, {}, moved from {} to {}.",
                last.says,
                figure(last.now),
                figure(m.now)
            )),
            0,
        ),
        Some(m) => {
            let unmoved = last.unmoved + 1;
            let steer = if unmoved >= UNMOVED_FOR {
                " Orchestrator: please steer it."
            } else {
                ""
            };
            // A drag that grew hasn't moved down either, so it counts on; at
            // 01:10 on 15 November one that went from 7 to 8 read "still 8".
            let how = if m.now > last.now {
                format!("grew from {} to {}", figure(last.now), figure(m.now))
            } else {
                format!(
                    "hasn't moved in {unmoved} {}: still {}",
                    if unmoved == 1 { "hour" } else { "hours" },
                    figure(m.now)
                )
            };
            (
                Some(format!(
                    "Last hour's drag, {}, {how}, with {}.{steer}",
                    last.says,
                    owner_or_none(&last.owner)
                )),
                unmoved,
            )
        }
    }
}

fn owner_or_none(owner: &str) -> &str {
    if owner.is_empty() { "no owner" } else { owner }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// A miss's own words, as its card line holds them: only its owner, and a
/// note after it, follow the last mark, since a miss's words may hold one.
fn miss_words(line: &str) -> &str {
    line.rsplit_once(" · ").map_or(line, |(what, _)| what)
}

#[cfg(test)]
mod tests {
    use super::super::measure;
    use super::*;

    /// No miss in Joel's own words.
    fn none() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn held(key: &str, now: f64, unmoved: u32) -> Kept {
        Kept {
            drag: Some(Held {
                key: key.to_owned(),
                says: format!("{key} said"),
                owner: "OWNER".to_owned(),
                now,
                unmoved,
            }),
            ..Kept::default()
        }
    }

    #[test]
    fn clean_zones_show_only_what_is_past_and_count_the_rest() {
        let measures = [
            measure("long", Class::Clean, 12.0, Some(10.0)),
            measure("crowded", Class::Clean, 2.0, Some(2.0)),
            measure("notes", Class::Clean, 87.0, None),
            measure("lints", Class::Clean, 9.0, None),
        ];
        let (card, _) = compose(
            "18:10",
            &measures,
            &Kept::default(),
            &none(),
            "18:10 tomorrow",
        );
        assert!(
            card.contains(
                "Clean zones:\n  ✕ long said: 12, at most 10\n  1 more within target, 2 unjudged\n"
            ),
            "a clean measure in or unjudged was listed, or the rest weren't counted: {card}"
        );
        let (all_in, _) = compose(
            "18:10",
            &measures[1..2],
            &Kept::default(),
            &none(),
            "18:10 tomorrow",
        );
        assert!(
            all_in.contains("Clean zones:\n  all 1 within target\n"),
            "a clean zone with nothing past didn't say so: {all_in}"
        );
    }

    #[test]
    fn the_card_names_the_drag_and_its_owner_and_marks_each_measure() {
        let measures = [
            measure("merge-to-on", Class::Ship, 37.9, Some(18.0)),
            measure("landed", Class::Ship, 12.0, None),
        ];
        let (card, next) = compose(
            "18:10",
            &measures,
            &Kept::default(),
            &none(),
            "18:10 tomorrow",
        );
        assert!(
            card.contains("✕ merge-to-on said: 37.9, at most 18"),
            "{card}"
        );
        assert!(card.contains("· landed said: 12, unjudged"), "{card}");
        assert!(
            card.contains("Drag: merge-to-on said, 37.9 against at most 18, to OWNER"),
            "{card}"
        );
        assert!(
            card.contains("Clean zones:\n  not read yet"),
            "unread is said: {card}"
        );
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(0),
            "a new drag starts unmoved"
        );
    }

    #[test]
    fn his_apps_under_load_lead_the_card_ahead_of_every_class() {
        let measures = [
            measure("merge-to-on", Class::Ship, 37.9, Some(18.0)),
            measure("apps:launcher", Class::Apps, 127.0, Some(50.0)),
        ];
        let (card, _) = compose("18:10", &measures, &Kept::default(), &none(), "");
        assert!(
            card.starts_with(
                "cadence hour at 18:10\nHis apps under load:\n  ✕ apps:launcher said: 127, \
                 at most 50\n"
            ),
            "{card}"
        );
        assert!(card.contains("Drag: apps:launcher said"), "{card}");
    }

    #[test]
    fn a_drag_unmoved_two_hours_asks_the_orchestrator_to_steer_it() {
        let measures = [measure("slips", Class::Slip, 3.0, Some(0.0))];
        let (card, next) = compose("19:10", &measures, &held("slips", 3.0, 1), &none(), "");
        assert!(card.contains("hasn't moved in 2 hours"), "{card}");
        assert!(card.contains("Orchestrator: please steer it."), "{card}");
        assert_eq!(next.drag.map(|d| d.unmoved), Some(2), "it counts on");
    }

    #[test]
    fn a_drag_that_grew_says_so_and_counts_as_unmoved() {
        let grew = [measure("slips", Class::Slip, 5.0, Some(0.0))];
        let (card, next) = compose("01:10", &grew, &held("slips", 3.0, 0), &none(), "");
        assert!(
            card.contains("Last hour's drag, slips said, grew from 3 to 5, with OWNER."),
            "a drag that got worse was said to hold still: {card}"
        );
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(1),
            "growing is no move down"
        );
    }

    #[test]
    fn a_drag_that_moved_or_came_back_in_starts_over() {
        let moved = [measure("slips", Class::Slip, 2.0, Some(0.0))];
        let (card, next) = compose("19:10", &moved, &held("slips", 3.0, 1), &none(), "");
        assert!(card.contains("moved from 3 to 2"), "{card}");
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(0),
            "moving starts it over"
        );
        let back = [measure("slips", Class::Slip, 0.0, Some(0.0))];
        let (card, next) = compose("20:10", &back, &held("slips", 2.0, 0), &none(), "");
        assert!(card.contains("is back in"), "{card}");
        assert_eq!(next.drag, None, "nothing past its target is no drag");
        let unread = [measure("slips", Class::Slip, 0.0, None)];
        let (card, _) = compose("21:10", &unread, &held("slips", 2.0, 0), &none(), "");
        assert!(
            card.contains("Last hour's drag, slips said, wasn't read this hour."),
            "a drag its hour couldn't read was said to be back in: {card}"
        );
    }

    #[test]
    fn a_cells_drag_goes_to_each_cells_owner() {
        let mut idle = measure("cells-idle", Class::Cells, 1.0, Some(0.0));
        idle.behind = vec!["AAAAAA idle 30 min: write it · ORCH".to_owned()];
        let (card, _) = compose("02:10", &[idle], &Kept::default(), &none(), "x");
        assert!(
            card.contains("to each cell's owner by x"),
            "a cells drag was said to go to files' owners: {card}"
        );
    }

    #[test]
    fn a_miss_of_joels_that_closes_is_said_for_him_once() {
        let mut open = measure("misses", Class::Miss, 1.0, Some(0.0));
        open.behind = vec!["the map had no network · KNURBN".to_owned()];
        let kept = Kept {
            drag: None,
            open: vec![
                "the map had no network · KNURBN".to_owned(),
                "History · Reader paused · RCTRF8".to_owned(),
            ],
            ..Kept::default()
        };
        let said = BTreeMap::from([(
            "History · Reader paused".to_owned(),
            "what you ran into on 16 Nov at 13:38, \"history stopped\"".to_owned(),
        )]);
        let (card, next) = compose("19:10", &[open.clone()], &kept, &said, "");
        assert!(
            card.contains(
                "For Joel: what you ran into on 16 Nov at 13:38, \"history stopped\", is fixed, \
                 and a check now catches it if it comes back. Nothing to do."
            ),
            "a closed miss wasn't said in Joel's own words and when: {card}"
        );
        let (unsaid, _) = compose("19:10", &[open], &kept, &none(), "");
        assert!(
            unsaid.contains(
                "For Joel: something you ran into, History · Reader paused, is fixed, and a \
                 check now catches it if it comes back. Nothing to do."
            ),
            "a miss without Joel's words wasn't said whole, only its owner cut: {unsaid}"
        );
        assert_eq!(
            card.matches("For Joel").count(),
            1,
            "the one miss that closed wasn't told once, or one still open was told: {card}"
        );
        assert!(
            card.contains("Why didn't our checks catch these?"),
            "{card}"
        );
        assert_eq!(
            next.open,
            ["the map had no network · KNURBN"],
            "only the open one is kept"
        );
    }

    /// A node that doesn't answer for an hour holds a fixed miss open again;
    /// once it answers, Joel isn't told a second time.
    #[test]
    fn a_miss_told_fixed_once_is_not_told_again_when_it_closes_anew() {
        let open = measure("misses", Class::Miss, 0.0, Some(0.0));
        let kept = Kept {
            drag: None,
            open: vec!["the map drew cells alone · WCJQSG".to_owned()],
            ..Kept::default()
        };
        let said = BTreeMap::from([(
            "the map drew cells alone".to_owned(),
            "what you ran into".to_owned(),
        )]);
        let (first, next) = compose("03:10", std::slice::from_ref(&open), &kept, &said, "");
        assert!(first.contains("For Joel"), "{first}");
        let again = Kept {
            open: kept.open.clone(),
            ..next
        };
        let (second, after) = compose("05:10", &[open], &again, &said, "");
        assert!(
            !second.contains("For Joel"),
            "a miss already told fixed was told again: {second}"
        );
        assert_eq!(
            after.told,
            ["the map drew cells alone"],
            "what was told wasn't kept"
        );
    }

    /// Two misses in the same words are told fixed once; one the ledger
    /// reopens, open with no hold on it, is told again once it closes anew.
    #[test]
    fn a_miss_is_told_fixed_once_an_hour_and_again_after_it_reopens() {
        let none = measure("misses", Class::Miss, 0.0, Some(0.0));
        let kept = Kept {
            drag: None,
            open: vec![
                "the map drew cells alone · WCJQSG".to_owned(),
                "the map drew cells alone · H2VYOB".to_owned(),
            ],
            ..Kept::default()
        };
        let (first, next) = compose(
            "03:10",
            std::slice::from_ref(&none),
            &kept,
            &BTreeMap::new(),
            "",
        );
        assert_eq!(
            (first.matches("For Joel").count(), next.told.len()),
            (1, 1),
            "two misses in the same words were told fixed twice: {first}"
        );
        let mut reopened = measure("misses", Class::Miss, 1.0, Some(0.0));
        reopened.behind = vec!["the map drew cells alone · WCJQSG".to_owned()];
        let (_, open) = compose("04:10", &[reopened], &next, &BTreeMap::new(), "");
        let (third, _) = compose(
            "05:10",
            std::slice::from_ref(&none),
            &open,
            &BTreeMap::new(),
            "",
        );
        assert!(
            third.contains("For Joel"),
            "a miss the ledger reopened was never told fixed again: {third}"
        );
        // An hour a measure holds it, its check merged, isn't a reopening.
        let mut holding = measure("misses", Class::Miss, 1.0, Some(0.0));
        holding.behind = vec![format!(
            "the map drew cells alone · WCJQSG{HELD}map-cells wasn't read this hour"
        )];
        let (_, through) = compose("04:10", &[holding], &next, &BTreeMap::new(), "");
        let (fourth, _) = compose(
            "05:10",
            std::slice::from_ref(&none),
            &through,
            &BTreeMap::new(),
            "",
        );
        assert!(
            !fourth.contains("For Joel"),
            "a miss held an hour by its measure was told fixed a second time: {fourth}"
        );
        // One told fixed on its merge, then held as not running here, as at
        // 21:10 on 19 November, is told again once it runs.
        let mut unrun = measure("misses", Class::Miss, 1.0, Some(0.0));
        unrun.behind = vec![format!(
            "the map drew cells alone · WCJQSG{HELD}core #5800{UNRUN}the last switch \
             doesn't carry it"
        )];
        let (_, waiting) = compose("04:10", &[unrun], &next, &BTreeMap::new(), "");
        let (fifth, _) = compose("05:10", &[none], &waiting, &BTreeMap::new(), "");
        assert!(
            fifth.contains("For Joel"),
            "a miss told fixed before it ran wasn't told again once it ran: {fifth}"
        );
    }

    /// An hour that couldn't read the misses ledger whole says nothing of
    /// which are gone, so it tells Joel nothing and keeps last hour's open.
    #[test]
    fn an_hour_the_ledger_wasnt_read_whole_tells_nothing_and_keeps_what_was_open() {
        let mut unread = measure("misses", Class::Miss, 1.0, None);
        unread.behind = vec!["the launcher hid in a share · D46U6P".to_owned()];
        let kept = Kept {
            drag: None,
            open: vec!["the map drew cells alone · WCJQSG".to_owned()],
            told: vec!["History paused".to_owned()],
            ..Kept::default()
        };
        let (card, next) = compose("06:10", &[unread], &kept, &BTreeMap::new(), "");
        assert!(
            !card.contains("For Joel") && next.told == kept.told,
            "an unread ledger told a miss fixed: {card} {next:?}"
        );
        // One seen open since last hour is carried too, so it's told fixed
        // when it closes.
        assert_eq!(
            next.open,
            [
                "the map drew cells alone · WCJQSG",
                "the launcher hid in a share · D46U6P"
            ],
            "an unread ledger dropped a miss open last hour or since: {next:?}"
        );
    }

    #[test]
    fn an_unread_hour_forgets_telling_of_a_miss_seen_open_again() {
        let mut unread = measure("misses", Class::Miss, 1.0, None);
        unread.behind = vec!["the launcher hid in a share · D46U6P".to_owned()];
        let kept = Kept {
            drag: None,
            told: vec!["the launcher hid in a share".to_owned()],
            ..Kept::default()
        };
        let (_, next) = compose("06:10", &[unread], &kept, &BTreeMap::new(), "");
        assert!(
            next.told.is_empty(),
            "a miss open again kept its telling, so its next fix goes untold: {next:?}"
        );
    }

    #[test]
    fn a_miss_still_open_under_new_words_after_its_owner_is_not_said_fixed() {
        let mut open = measure("misses", Class::Miss, 2.0, Some(0.0));
        open.behind = vec![
            "the map had no network · PN2VZC".to_owned(),
            "the notice stayed up · 88LQC0, though cadence couldn't read world's main to see \
             its check"
                .to_owned(),
        ];
        let kept = Kept {
            drag: None,
            open: vec![
                "the map had no network · KNURBN".to_owned(),
                "the notice stayed up · 88LQC0".to_owned(),
            ],
            ..Kept::default()
        };
        let (card, _) = compose("12:10", &[open], &kept, &none(), "");
        assert!(
            !card.contains("For Joel"),
            "a miss handed to a new owner, or noted as unread, was told fixed: {card}"
        );
    }

    #[test]
    fn a_clean_drag_with_files_owned_names_each_files_owner() {
        let mut suppressions = measure(
            "clean:core.nix:suppressions",
            Class::Clean,
            12.0,
            Some(10.0),
        );
        suppressions.behind = vec!["net/a.rs · PN2VZC".to_owned()];
        let (card, _) = compose("03:10", &[suppressions], &Kept::default(), &none(), "");
        assert!(
            card.contains("to each file's owner by"),
            "a clean drag whose files have owners named one owner: {card}"
        );
    }
}
