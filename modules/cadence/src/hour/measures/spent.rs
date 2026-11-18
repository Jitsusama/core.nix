//! What each quest spent today beside what it landed, so a quest whose
//! long chains land nothing shows on the card each hour rather than in a
//! report someone has to ask for. From 10:00 to 22:00 on 17 November, 8
//! of the 10 costliest quests that merged nothing were working sessions
//! of 500 to 750 calls. The spend is spend-watch's own figure for each
//! quest today, which QCXZEB hasn't agreed yet, so the measure has no
//! target and says so: it is read, never filed as a finding.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count};

/// The least a quest spends today for the card to name it.
const NAMED_FROM: f64 = 30.0;

/// How many changes a line names before counting the rest.
const CHANGES_NAMED: usize = 3;

/// What the measure says while spend-watch's figures are unagreed.
pub const NOT_AGREED: &str = " (spend not agreed yet)";

/// Each quest's own spend today, by its short id, from spend-watch's
/// `collection.json`, whose `families` holds it as a figure for a person,
/// such as `$148` or `about $3.89`.
fn spent_today(collection: &str) -> Option<BTreeMap<String, f64>> {
    let read: Value = serde_json::from_str(collection).ok()?;
    let families = read.get("families")?.as_object()?;
    Some(
        families
            .iter()
            .filter_map(|(quest, family)| {
                let own = family.get("own")?.as_str()?;
                let figure: String = own
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                let short = quest.rsplit('-').next().unwrap_or(quest);
                Some((short.to_owned(), figure.parse().ok()?))
            })
            .collect(),
    )
}

/// The changes each quest landed, by its short id, from the landing notes
/// the mesh sends itself on a merge, such as `D46U6P landed core.nix#5540,
/// its title. ... Said by the mesh; feedback to D46U6P.` A session's own
/// note of the same landing is said its own way, so only the mesh's
/// count, and each change once. A note opens with its sender's address,
/// which `quest_of` turns into the short id of the quest it works on: a
/// session may go by an address of its own, such as 6EAC3DA6.
#[must_use]
pub fn landed_by<'a>(
    notes: impl IntoIterator<Item = &'a str>,
    quest_of: impl Fn(&str) -> String,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut landed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for note in notes {
        if !note.contains("Said by the mesh") {
            continue;
        }
        let mut words = note.split_whitespace();
        let (Some(quest), Some("landed"), Some(change)) = (words.next(), words.next(), words.next())
        else {
            continue;
        };
        let change = change.trim_end_matches([',', '.']);
        if change.contains('#') {
            landed
                .entry(quest_of(quest))
                .or_default()
                .insert(change.to_owned());
        }
    }
    landed
}

/// The quests spending `NAMED_FROM` or more today, each beside what it
/// landed: those that landed nothing first, the costliest first, then the
/// rest by what each landing cost. Its figure is how many landed nothing.
#[must_use]
pub fn beside(
    collection: &str,
    landed: &BTreeMap<String, BTreeSet<String>>,
    owner: &str,
) -> Measure {
    let says = "quests spending $30 or more today that landed nothing";
    let Some(spent) = spent_today(collection) else {
        return Measure {
            key: "spent-landing-nothing".to_owned(),
            class: Class::Cells,
            says: format!("{says} (spend-watch unread)"),
            now: 0.0,
            target: None,
            owner: owner.to_owned(),
            behind: Vec::new(),
        };
    };
    let none = BTreeSet::new();
    let mut named: Vec<(&String, f64, &BTreeSet<String>)> = spent
        .iter()
        .filter(|(_, dollars)| **dollars >= NAMED_FROM)
        .map(|(quest, dollars)| (quest, *dollars, landed.get(quest).unwrap_or(&none)))
        .collect();
    let per_landing =
        |dollars: f64, changes: &BTreeSet<String>| dollars / count(changes.len().max(1));
    named.sort_by(|(qa, da, ca), (qb, db, cb)| {
        (!ca.is_empty())
            .cmp(&!cb.is_empty())
            .then(per_landing(*db, cb).total_cmp(&per_landing(*da, ca)))
            .then(qa.cmp(qb))
    });
    let nothing = named.iter().filter(|(_, _, c)| c.is_empty()).count();
    Measure {
        key: "spent-landing-nothing".to_owned(),
        class: Class::Cells,
        says: format!("{says}{NOT_AGREED}"),
        now: count(nothing),
        target: None,
        owner: owner.to_owned(),
        behind: named
            .into_iter()
            .map(|(quest, dollars, changes)| format!("${dollars:.0} today, {} · {quest}", what(changes)))
            .collect(),
    }
}

/// What a quest landed, as a line says it.
fn what(changes: &BTreeSet<String>) -> String {
    if changes.is_empty() {
        return "landed nothing".to_owned();
    }
    let shown: Vec<&str> = changes.iter().take(CHANGES_NAMED).map(String::as_str).collect();
    let more = changes.len().saturating_sub(CHANGES_NAMED);
    if more == 0 {
        format!("landed {}", shown.join(", "))
    } else {
        format!("landed {} and {more} more", shown.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLECTION: &str = r#"{"families": {
        "QEST-20261023-Y2YBTS": {"all": "$828", "own": "$148", "under": "$680"},
        "QEST-20261111-XY9H5C": {"own": "$216"},
        "QEST-20261111-2DZF5Z": {"own": "$1,206"},
        "QEST-20261111-SMALL1": {"own": "about $3.89"},
        "QEST-20261111-ZINAWZ": {"own": "$132"}
    }}"#;

    /// Every sender works on its own quest but one, a session going by
    /// an address of its own.
    fn quest_of(address: &str) -> String {
        if address == "6EAC3DA6" {
            "ZINAWZ".to_owned()
        } else {
            address.to_owned()
        }
    }

    fn landed() -> BTreeMap<String, BTreeSet<String>> {
        landed_by(
            [
            "Y2YBTS landed core.nix#5536, Bring a Windowless Session Forward on Go To. \
             For the person: Go To. Said by the mesh; feedback to Y2YBTS.",
            "Y2YBTS landed core.nix#5526, Again. Said by the mesh; feedback to Y2YBTS.",
            "ZINAWZ landed world#2476563, Route. Said by the mesh; feedback to ZINAWZ.",
            // A session's own word of a landing the mesh already said.
            "ZINAWZ landed core #5544 \"Give pi-fence's Census the Package It Loads\"",
            "ZINAWZ landed world#2476563, Route. Said by the mesh; feedback to ZINAWZ.",
            "6EAC3DA6 landed core.nix#5545, Say Why. Said by the mesh; feedback to 6EAC3DA6.",
        ],
            quest_of,
        )
    }

    #[test]
    fn only_the_meshs_own_notes_count_and_each_change_once() {
        let landed = landed();
        assert_eq!(landed["Y2YBTS"].len(), 2, "{landed:?}");
        assert_eq!(
            landed["ZINAWZ"].iter().collect::<Vec<_>>(),
            ["core.nix#5545", "world#2476563"],
            "a session's own note or the same note twice counted, or a session's own \
             address not taken to its quest"
        );
    }

    #[test]
    fn quests_that_landed_nothing_come_first_the_costliest_first() {
        let m = beside(COLLECTION, &landed(), "WCJQSG");
        assert_eq!(
            m.behind,
            [
                "$1206 today, landed nothing · 2DZF5Z",
                "$216 today, landed nothing · XY9H5C",
                "$148 today, landed core.nix#5526, core.nix#5536 · Y2YBTS",
                "$132 today, landed core.nix#5545, world#2476563 · ZINAWZ",
            ],
            "the order or the words, or a quest under $30 named"
        );
        assert!((m.now - 2.0).abs() < f64::EPSILON, "two landed nothing: {}", m.now);
    }

    #[test]
    fn spend_is_unjudged_and_says_it_isnt_agreed() {
        let m = beside(COLLECTION, &landed(), "WCJQSG");
        assert_eq!(m.target, None, "an unagreed spend judged");
        assert!(m.says.ends_with(NOT_AGREED), "{}", m.says);
        assert_eq!(m.past(), None, "it could be filed as a drag");
    }

    #[test]
    fn a_collection_that_cant_be_read_names_no_quest() {
        let m = beside("", &landed(), "WCJQSG");
        assert!(m.behind.is_empty(), "{:?}", m.behind);
        assert!(m.says.ends_with("(spend-watch unread)"), "{}", m.says);
    }

    #[test]
    fn a_quest_landing_more_than_three_says_how_many_more() {
        let notes: Vec<String> = (1..=5)
            .map(|n| format!("Y2YBTS landed core.nix#{n}, T. Said by the mesh; feedback."))
            .collect();
        let m = beside(COLLECTION, &landed_by(notes.iter().map(String::as_str), quest_of), "WCJQSG");
        assert!(
            m.behind
                .contains(&"$148 today, landed core.nix#1, core.nix#2, core.nix#3 and 2 more · Y2YBTS".to_owned()),
            "{:?}",
            m.behind
        );
    }
}
