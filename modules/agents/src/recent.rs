//! Agents' Recent: the sessions here that closed this boot, newest first,
//! as many as Recent lists, each served ready as `recent:agents:<id>` with
//! what its row says, so the shell lists them without sorting or comparing
//! every closed session each time one changes (SessionRow.js and Recent.js
//! did it there).
//!
//! Never one Agents already shows: one running or parked again by its
//! address, a closed orchestrator the harness lists, one of Sessions to
//! Reopen, or one whose quest a live session works on, which reopening
//! would double.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::clock;
use crate::said::Said;
use crate::served::{PLACE, SPACING};
use crate::session::{Lives, Session};

/// As many as Recent lists (Recent.js's MOST).
pub const MOST: usize = 9;

/// Recent's own record, what its place says of what it holds.
pub const RECENT: &str = "recent:agents";

/// What Recent leaves out, as Agents shows it elsewhere.
pub struct Shown<'a> {
    /// The addresses of the sessions running or parked, and of every
    /// orchestrator the harness lists.
    pub addresses: HashSet<&'a str>,
    /// The quests a session running or parked works on.
    pub quests: HashSet<&'a str>,
    /// The ids of the sessions to reopen.
    pub reopening: HashSet<String>,
}

/// One closed session as Recent lists it: pi's id, the address it went by,
/// its name and folder, its quest, the folder it ran in, when it closed,
/// in milliseconds, and its note.
fn listed(one: Session) -> Option<Value> {
    let id = one.id();
    let id = if id.is_empty() {
        one.0["key"].as_str()?.strip_prefix("session:")?
    } else {
        id
    };
    let presence = one.presence()?;
    let (title, folder) = one.named();
    let said = |field: &str| presence[field].as_str().unwrap_or("").to_owned();
    let quest = presence["questInfo"]["id"]
        .as_str()
        .or_else(|| presence["quest"].as_str());
    let name = [title.as_str(), one.address(), id]
        .into_iter()
        .find(|one| !one.is_empty())
        .unwrap_or("")
        .to_owned();
    Some(json!({
        "id": id,
        "address": one.address(),
        "name": name,
        "title": title,
        "folder": folder,
        "quest": quest,
        "cwd": said("cwd"),
        "ended": clock::ms_of(&one.0["at"]),
        "note": note(one, &title, &folder),
    }))
}

/// A closed session's note, which never says its name again: its quest's
/// title where that differs from its name, else the machine and repository
/// it ran in, as a session's drawer names them (VKCF3Z, 2026-11-08).
fn note(one: Session, title: &str, folder: &str) -> String {
    let presence = one.presence();
    let said = |path: &str| {
        presence
            .and_then(|presence| presence.pointer(path))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let quest = said("/questInfo/title");
    if !quest.is_empty() && quest != title {
        return quest.to_owned();
    }
    let repo = Some(said("/repo")).filter(|repo| !repo.is_empty());
    [one.machine(), repo.unwrap_or(folder)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// Recent's records, the newest closed first, then by id, at most MOST,
/// each once by its id.
pub fn recent<'a>(sessions: impl IntoIterator<Item = Session<'a>>, shown: &Shown) -> Vec<Value> {
    let mut closed: Vec<Value> = sessions
        .into_iter()
        .filter(|one| one.lives() == Lives::Closed)
        .filter_map(listed)
        .filter(|one| {
            let said = |field: &str| one[field].as_str().unwrap_or("");
            !shown.addresses.contains(said("address"))
                && !shown.quests.contains(said("quest"))
                && !shown.reopening.contains(said("id"))
        })
        .collect();
    let ended = |one: &Value| one["ended"].as_i64().unwrap_or(i64::MIN);
    closed.sort_by(|one, other| {
        ended(other)
            .cmp(&ended(one))
            .then_with(|| one["id"].as_str().cmp(&other["id"].as_str()))
    });
    let mut seen = HashSet::new();
    closed.retain(|one| seen.insert(one["id"].as_str().unwrap_or("").to_owned()));
    closed
        .into_iter()
        .take(MOST)
        .zip(1..)
        .map(|(one, place)| {
            json!({
                "key": format!("recent:{PLACE}:{}", one["id"].as_str().unwrap_or("")),
                "kind": "recent",
                "order": place * SPACING,
                "recent": one,
            })
        })
        .collect()
}

/// What Recent's place says of the sessions it lists, `served` as
/// `recent` serves them, as Recent.js's `said` says it: at its side how
/// many, or its words' `none`; in its drawer their names, newest first,
/// a name held more than once said once where it's newest, with how
/// often. Its key starts as each session's does, so the shell makes
/// Recent again as either moves, and it holds no `recent` of its own, so
/// the shell never lists it as one.
pub fn place(served: &[Value], words: &Said) -> Value {
    let names: Vec<&str> = served
        .iter()
        .filter_map(|one| one["recent"]["name"].as_str())
        .collect();
    let side = if names.is_empty() {
        words.word("none").to_owned()
    } else {
        names.len().to_string()
    };
    let mut often: Vec<(&str, usize)> = Vec::new();
    for name in &names {
        match often.iter_mut().find(|(one, _)| one == name) {
            Some((_, count)) => *count += 1,
            None => often.push((name, 1)),
        }
    }
    let held: Vec<String> = often
        .into_iter()
        .map(|(name, count)| match count {
            1 => name.to_owned(),
            2 => words.fill("twice", &[("name", name)]),
            _ => words.fill("times", &[("name", name), ("count", &count.to_string())]),
        })
        .collect();
    json!({"key": RECENT, "kind": "recent", "place": {"side": side, "held": held}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed(id: &str, address: &str, at: &str, presence: &Value) -> Value {
        let mut presence = presence.clone();
        presence["session"] = json!(id);
        presence["addresses"] = json!([address]);
        json!({"key": format!("session:{id}"), "standing": "closed", "here": true,
               "at": at, "machine": "skiff", "presence": presence})
    }

    fn ids(served: &[Value]) -> Vec<&str> {
        served
            .iter()
            .map(|one| one["recent"]["id"].as_str().unwrap_or(""))
            .collect()
    }

    fn nothing_shown() -> Shown<'static> {
        Shown {
            addresses: HashSet::new(),
            quests: HashSet::new(),
            reopening: HashSet::new(),
        }
    }

    #[test]
    fn the_newest_closed_come_first_and_no_more_than_recent_lists() {
        let mut records: Vec<Value> = (0..12)
            .map(|n| {
                let at = format!("2026-11-11T1{}:00:00Z", n % 10);
                closed(&format!("s-{n:02}"), &format!("A{n}"), &at, &json!({}))
            })
            .collect();
        records.push(json!({"key": "session:live", "standing": "running",
                            "presence": {"addresses": ["L"], "session": "s-live"}}));
        let served = recent(records.iter().map(Session), &nothing_shown());
        assert_eq!(
            ids(&served),
            [
                "s-09", "s-08", "s-07", "s-06", "s-05", "s-04", "s-03", "s-02", "s-01"
            ],
            "the newest nine closed, the same moment by id, none running"
        );
        assert_eq!(
            served[0]["key"],
            json!("recent:agents:s-09"),
            "keyed in Agents' place"
        );
        assert_eq!(
            served[1]["order"].as_i64(),
            Some(2 * SPACING),
            "spaced in order"
        );
    }

    #[test]
    fn one_agents_shows_elsewhere_is_left_out() {
        let records = [
            closed("s-again", "AGAIN", "2026-11-11T10:00:00Z", &json!({})),
            closed(
                "s-quest",
                "QUESTY",
                "2026-11-11T10:00:00Z",
                &json!({"questInfo": {"id": "QEST-1"}}),
            ),
            closed("s-reopen", "REOPEN", "2026-11-11T10:00:00Z", &json!({})),
            closed("s-kept", "KEPT", "2026-11-11T10:00:00Z", &json!({})),
        ];
        let shown = Shown {
            addresses: HashSet::from(["AGAIN"]),
            quests: HashSet::from(["QEST-1"]),
            reopening: HashSet::from(["s-reopen".to_owned()]),
        };
        assert_eq!(
            ids(&recent(records.iter().map(Session), &shown)),
            ["s-kept"],
            "one live again, one whose quest a live one works on, one to reopen"
        );
    }

    #[test]
    fn its_note_names_its_quest_else_where_it_ran() {
        let records = [
            closed(
                "s-a",
                "A",
                "2026-11-11T10:00:00Z",
                &json!({"questInfo": {"id": "QEST-1", "title": "Reef Survey"},
                        "title": "\u{3c0} - Reef Survey - kelp", "cwd": "/home/x/kelp"}),
            ),
            closed(
                "s-b",
                "B",
                "2026-11-11T09:00:00Z",
                &json!({"cwd": "/home/x/kelp/", "repo": "harbour/kelp"}),
            ),
            closed(
                "s-c",
                "C",
                "2026-11-11T08:00:00Z",
                &json!({"cwd": "/home/x/buoy"}),
            ),
        ];
        let served = recent(records.iter().map(Session), &nothing_shown());
        let said: Vec<_> = served
            .iter()
            .map(|one| {
                let one = &one["recent"];
                (
                    one["title"].clone(),
                    one["note"].clone(),
                    one["folder"].clone(),
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                (
                    json!("Reef Survey"),
                    json!("skiff \u{b7} kelp"),
                    json!("kelp")
                ),
                (
                    json!("B"),
                    json!("skiff \u{b7} harbour/kelp"),
                    json!("kelp")
                ),
                (json!("C"), json!("skiff \u{b7} buoy"), json!("buoy")),
            ],
            "a quest named as the row is, so where it ran; else its repo or folder"
        );
        assert_eq!(
            served[0]["recent"]["ended"],
            json!(1_794_391_200_000_i64),
            "when it closed, in milliseconds"
        );
        assert_eq!(served[0]["recent"]["cwd"], json!("/home/x/kelp"));
        assert_eq!(served[0]["recent"]["quest"], json!("QEST-1"));
    }

    fn words() -> Said {
        crate::row::Words::from_vocabulary(crate::said::LAUNCHERS)
            .expect("the launcher's words read")
            .recent
    }

    #[test]
    fn its_place_counts_them_and_names_each_once_newest_first() {
        let records = [
            closed(
                "s-a",
                "A",
                "2026-11-11T12:00:00Z",
                &json!({"title": "Tide Tables"}),
            ),
            closed(
                "s-b",
                "B",
                "2026-11-11T11:00:00Z",
                &json!({"title": "Kelp"}),
            ),
            closed(
                "s-c",
                "C",
                "2026-11-11T10:00:00Z",
                &json!({"title": "Tide Tables"}),
            ),
            closed(
                "s-d",
                "D",
                "2026-11-11T09:00:00Z",
                &json!({"title": "Tide Tables"}),
            ),
            closed(
                "s-e",
                "E",
                "2026-11-11T08:00:00Z",
                &json!({"title": "Kelp"}),
            ),
            closed("s-f", "", "2026-11-11T07:00:00Z", &json!({})),
        ];
        let served = recent(records.iter().map(Session), &nothing_shown());
        assert_eq!(
            served[0]["recent"]["name"],
            json!("Tide Tables"),
            "a row is named by its title"
        );
        assert_eq!(
            served[5]["recent"]["name"],
            json!("s-f"),
            "else its address, else its id"
        );
        let said = place(&served, &words());
        assert_eq!(said["key"], json!(RECENT), "keyed as Recent's place");
        assert_eq!(
            said["place"]["side"],
            json!("6"),
            "its side counts every one"
        );
        let held = said["place"]["held"].as_array().expect("its names");
        assert_eq!(
            held[..2],
            [json!("Tide Tables, 3 times"), json!("Kelp, twice")],
            "each name once where it's newest, with how often"
        );
        assert_eq!(
            held.get(2),
            Some(&json!("s-f")),
            "and the one named otherwise"
        );
        assert_eq!(
            place(&[], &words())["place"],
            json!({"side": "none", "held": []}),
            "holding none, its side says so"
        );
    }
}
