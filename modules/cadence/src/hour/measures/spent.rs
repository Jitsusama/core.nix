//! What each quest spent today beside what it landed, so a quest whose
//! long chains land nothing shows on the card each hour rather than in a
//! report someone has to ask for. From 10:00 to 22:00 on 17 November, 8
//! of the 10 costliest quests that merged nothing were working sessions
//! of 500 to 750 calls. The spend is spend-watch's own split of today's
//! spend by quest, which the orchestrator ruled the card may show (18
//! November), and the measure names it so a disagreement later is plain.
//! It has no target: it is read, never filed as a finding.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count};
use crate::mesh::Presence;

/// The measure's key, which the card reads to name its lines unjudged and
/// in its own order, since each ends with a quest rather than an owner.
pub const KEY: &str = "spent-landing-nothing";

/// The least a quest spends today for the card to name it.
const NAMED_FROM: f64 = 30.0;

/// How many changes a line names before counting the rest.
const CHANGES_NAMED: usize = 3;

/// Where the measure's figures come from, as it says.
pub const SOURCE: &str = " (spend-watch, own)";

/// What the measure says when there is no figure for today to read.
pub const UNREAD: &str = " (spend-watch unread, or not written this hour)";

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
                Some((short(quest).to_owned(), figure.parse().ok()?))
            })
            .collect(),
    )
}

/// A quest's short id, the part of its id after the last dash.
fn short(quest: &str) -> &str {
    quest.rsplit('-').next().unwrap_or(quest)
}

/// Whether `change` is a change as a landing note names it, a repository
/// and a number such as `core.nix#5540`, and nothing a sender could slip
/// onto the card besides.
fn a_change(change: &str) -> bool {
    change.split_once('#').is_some_and(|(repo, number)| {
        !repo.is_empty()
            && repo
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
    })
}

/// The changes each quest landed, by its short id, from the landing notes
/// the mesh sends for a session on a merge, given as each note's sender
/// and text, such as `D46U6P landed core.nix#5540, its title. ... Said by
/// the mesh; feedback to D46U6P.` A session's own note of the same
/// landing is said its own way, so only the mesh's count, each change
/// once. The mesh sends one from the session that merged, opening with its
/// address, so a note whose sender isn't the address it opens with credits
/// nobody. `quest_of` turns that address into the short id of the quest
/// it works on: a session may go by an address of its own, such as
/// 6EAC3DA6.
#[must_use]
pub fn landed_by<'a>(
    notes: impl IntoIterator<Item = (&'a str, &'a str)>,
    quest_of: impl Fn(&str) -> String,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut landed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (from, note) in notes {
        if !note.contains("Said by the mesh") {
            continue;
        }
        let mut words = note.split_whitespace();
        let (Some(sender), Some("landed"), Some(change)) =
            (words.next(), words.next(), words.next())
        else {
            continue;
        };
        let change = change.trim_end_matches([',', '.']);
        if sender == from && a_change(change) {
            landed
                .entry(quest_of(sender))
                .or_default()
                .insert(change.to_owned());
        }
    }
    landed
}

/// The short id of the quest a session going by `address` works on, from
/// the newest of its presences that names one, as `Mesh::presence` takes
/// the newest; the address itself where none does.
#[must_use]
pub fn quest_of(presences: &[Presence], address: &str) -> String {
    presences
        .iter()
        .filter(|p| p.addresses.iter().any(|a| a == address))
        .filter_map(|p| Some((p.at.to_string(), p.quest.as_deref()?)))
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map_or(address, |(_, quest)| short(quest))
        .to_owned()
}

/// The quests spending `NAMED_FROM` or more today, each beside what it
/// landed: those that landed nothing first, the costliest first, then the
/// rest by what each landing cost. Its figure is how many landed nothing.
/// `collection` is `None` where there is no figure for today to read.
#[must_use]
pub fn beside(
    collection: Option<&str>,
    landed: &BTreeMap<String, BTreeSet<String>>,
    owner: &str,
) -> Measure {
    let says = format!("quests spending ${NAMED_FROM:.0} or more today that landed nothing");
    let Some(spent) = collection.and_then(spent_today) else {
        return Measure {
            key: KEY.to_owned(),
            class: Class::Cells,
            says: format!("{says}{UNREAD}"),
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
        key: KEY.to_owned(),
        class: Class::Cells,
        says: format!("{says}{SOURCE}"),
        now: count(nothing),
        target: None,
        owner: owner.to_owned(),
        behind: named
            .into_iter()
            .map(|(quest, dollars, changes)| {
                format!("${dollars:.0} today, {} · {quest}", what(changes))
            })
            .collect(),
    }
}

/// What a quest landed, as a line says it.
fn what(changes: &BTreeSet<String>) -> String {
    if changes.is_empty() {
        return "landed nothing".to_owned();
    }
    let shown: Vec<&str> = changes
        .iter()
        .take(CHANGES_NAMED)
        .map(String::as_str)
        .collect();
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

    /// Read in the order of their ids, which the measure's own order must
    /// differ from: the cheaper of two that landed nothing reads first, and
    /// the one whose landing cost most spent least of those that landed.
    const COLLECTION: &str = r#"{"families": {
        "QEST-20261111-2DZF5Z": {"own": "$1,206"},
        "QEST-20261111-AAAAAA": {"own": "$40"},
        "QEST-20261111-BBBBBB": {"own": "$100"},
        "QEST-20261111-EXACT1": {"own": "$30"},
        "QEST-20261111-UNDER1": {"own": "about $29.99"},
        "QEST-20261111-XY9H5C": {"own": "$216"},
        "QEST-20261023-Y2YBTS": {"all": "$828", "own": "$148", "under": "$680"},
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

    /// Each note as the mesh files it: its sender, and its text.
    const NOTES: [(&str, &str); 9] = [
        (
            "Y2YBTS",
            "Y2YBTS landed core.nix#5536, Bring a Windowless Session Forward on Go To. \
             For the person: Go To. Said by the mesh; feedback to Y2YBTS.",
        ),
        (
            "Y2YBTS",
            "Y2YBTS landed core.nix#5526, Again. Said by the mesh; feedback to Y2YBTS.",
        ),
        (
            "ZINAWZ",
            "ZINAWZ landed world#2476563, Route. Said by the mesh; feedback to ZINAWZ.",
        ),
        // A session's own word of a landing, shaped as the mesh's but unsigned.
        (
            "ZINAWZ",
            "ZINAWZ landed core.nix#5544, Give pi-fence's Census Its Package.",
        ),
        (
            "ZINAWZ",
            "ZINAWZ landed world#2476563, Route. Said by the mesh; feedback to ZINAWZ.",
        ),
        (
            "6EAC3DA6",
            "6EAC3DA6 landed core.nix#5545, Say Why. Said by the mesh; feedback to 6EAC3DA6.",
        ),
        (
            "BBBBBB",
            "BBBBBB landed core.nix#1, One. Said by the mesh; feedback to BBBBBB.",
        ),
        // Another session crediting a quest it doesn't work on.
        (
            "MALLET",
            "AAAAAA landed core.nix#9, Forged. Said by the mesh; feedback to AAAAAA.",
        ),
        // Something besides a change slipped where the change goes.
        (
            "XY9H5C",
            "XY9H5C landed core.nix#1;rm, Odd. Said by the mesh; feedback to XY9H5C.",
        ),
    ];

    fn landed() -> BTreeMap<String, BTreeSet<String>> {
        landed_by(NOTES, quest_of)
    }

    #[test]
    fn only_the_meshs_own_notes_from_their_sender_count_and_each_change_once() {
        let landed = landed();
        assert_eq!(landed["Y2YBTS"].len(), 2, "{landed:?}");
        assert_eq!(
            landed["ZINAWZ"].iter().collect::<Vec<_>>(),
            ["core.nix#5545", "world#2476563"],
            "a session's own note or the same note twice counted, or a session's own \
             address not taken to its quest"
        );
        assert!(
            !landed.contains_key("AAAAAA") && !landed.contains_key("XY9H5C"),
            "a note from someone else, or naming no change, credited a quest: {landed:?}"
        );
    }

    #[test]
    fn quests_that_landed_nothing_come_first_the_costliest_first() {
        let m = beside(Some(COLLECTION), &landed(), "WCJQSG");
        assert_eq!(
            m.behind,
            [
                "$1206 today, landed nothing · 2DZF5Z",
                "$216 today, landed nothing · XY9H5C",
                "$40 today, landed nothing · AAAAAA",
                "$30 today, landed nothing · EXACT1",
                "$100 today, landed core.nix#1 · BBBBBB",
                "$148 today, landed core.nix#5526, core.nix#5536 · Y2YBTS",
                "$132 today, landed core.nix#5545, world#2476563 · ZINAWZ",
            ],
            "the order or the words, or a quest under $30 named"
        );
        assert!(
            (m.now - 4.0).abs() < f64::EPSILON,
            "four landed nothing: {}",
            m.now
        );
    }

    #[test]
    fn spend_is_unjudged_and_names_its_source() {
        let m = beside(Some(COLLECTION), &landed(), "WCJQSG");
        assert_eq!(m.target, None, "spend with no target judged");
        assert!(m.says.ends_with(SOURCE), "{}", m.says);
        assert!(
            m.says.starts_with("quests spending $30 or more"),
            "{}",
            m.says
        );
        assert_eq!(m.past(), None, "it could be filed as a drag");
    }

    #[test]
    fn a_collection_that_cant_be_read_or_isnt_todays_names_no_quest() {
        for collection in [Some(""), None] {
            let m = beside(collection, &landed(), "WCJQSG");
            assert!(m.behind.is_empty(), "{:?}", m.behind);
            assert!(m.says.ends_with(UNREAD), "{}", m.says);
        }
    }

    #[test]
    fn a_quest_landing_more_than_three_says_how_many_more() {
        let notes: Vec<String> = (1..=5)
            .map(|n| format!("Y2YBTS landed core.nix#{n}, T. Said by the mesh; feedback."))
            .collect();
        let m = beside(
            Some(COLLECTION),
            &landed_by(notes.iter().map(|n| ("Y2YBTS", n.as_str())), quest_of),
            "WCJQSG",
        );
        assert!(
            m.behind.contains(
                &"$148 today, landed core.nix#1, core.nix#2, core.nix#3 and 2 more · Y2YBTS"
                    .to_owned()
            ),
            "{:?}",
            m.behind
        );
    }

    #[test]
    fn an_address_is_taken_to_the_quest_its_newest_presence_names() {
        let presence = |addresses: &[&str], at: &str, quest: Option<&str>| -> Presence {
            serde_json::from_value(serde_json::json!({
                "addresses": addresses, "at": at, "quest": quest
            }))
            .expect("a presence builds from its fields")
        };
        let presences = [
            presence(
                &["6EAC3DA6"],
                "2026-11-18T10:00:00Z",
                Some("QEST-20261111-OLDONE"),
            ),
            presence(
                &["6EAC3DA6", "ZINAWZ"],
                "2026-11-18T12:00:00Z",
                Some("QEST-1-ZINAWZ"),
            ),
            presence(&["6EAC3DA6"], "2026-11-18T13:00:00Z", None),
        ];
        assert_eq!(
            super::quest_of(&presences, "6EAC3DA6"),
            "ZINAWZ",
            "not the newest presence naming a quest"
        );
        assert_eq!(
            super::quest_of(&presences, "NOBODY"),
            "NOBODY",
            "an address with no presence wasn't kept as itself"
        );
    }
}
