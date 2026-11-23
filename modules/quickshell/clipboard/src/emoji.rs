//! Emoji's rows, ready for the launcher to draw (Y2YBTS's ready-rows.md):
//! every emoji in the list built for the launcher (launcher/emoji.nix), a
//! line of JSON each, `[emoji, name, keywords, group]`, in Unicode's order,
//! which each row's `order` keeps, so Emoji.qml reads no file and makes no
//! row. The history serves them since it's what pastes them. Each row's
//! keywords travel beside its name for the shell's own search, which finds
//! an emoji by what it means as well as what it's called (Emoji.js), and
//! nothing draws them but its drawer. They're made once, as the history
//! starts, since the list never changes while it runs. Each holds only its
//! acts beneath it, Return pasting it and y copying it, so it opens no list
//! of them (Joel, 2026-11-11 22:00), and its drawer, shown as it's
//! selected, says its group, its keywords and, folded, its code points.
//! The row holds all its drawer says, so it serves none, saying only
//! `drawer: true`, and the shell makes it as it's drawn
//! (launcher/EmojiDrawer.js): served in every row, the 1,914 drawers were
//! 1.26 of the 4.34 MB the shell's heap kept live for Emoji's records, for
//! the one drawer shown at a time (the heap lab over Joel's answer,
//! 2026-11-23).
//!
//! Each emoji sits inside its group's row, Unicode's groups being the
//! place's own rows in the order the list first names them, so Super+E
//! with nothing typed lists nine of them, and an emoji's row is made only
//! once Joel steps into its group or a search finds it (search.rs): every
//! one of 1,914 made at once held about 30 MB of the shell's (2026-11-19).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::rows::{Words, capitalised};

/// The launcher's place these rows are drawn in.
pub const PLACE: &str = "emoji";

/// How far apart rows are ordered, as ready-rows.md spaces them.
const SPACED: u64 = 1024;

/// Every record of the place, as the history serves them, from the list's
/// text: each group's row, in the order the list first names it, then
/// every emoji's, in the list's order, inside its group's. A line that
/// isn't an emoji's is left out rather than refusing the rest.
pub fn records(list: &str, words: &Words) -> Vec<Value> {
    let lines: Vec<[String; 4]> = list
        .lines()
        .filter_map(|line| serde_json::from_str::<[String; 4]>(line).ok())
        .collect();
    let mut groups: Vec<(&str, &str, usize)> = Vec::new();
    let mut at_of: BTreeMap<&str, usize> = BTreeMap::new();
    for [emoji, _, _, group] in &lines {
        let at = *at_of.entry(group).or_insert_with(|| {
            groups.push((group, emoji, 0));
            groups.len() - 1
        });
        groups[at].2 += 1;
    }
    let grouped = groups
        .iter()
        .zip(0_u64..)
        .map(|(&(group, first, count), at)| group_record(group, first, count, at, words));
    let emoji = lines
        .iter()
        .zip(0_u64..)
        .map(|([emoji, name, keywords, group], at)| {
            let key = format!("row:{PLACE}:{at}");
            let named = Named {
                emoji,
                name,
                keywords,
                group,
            };
            let mut row = row(&key, at, &named);
            row["parent"] = json!(group_key(at_of[group.as_str()]));
            row["depth"] = json!(1);
            json!({ "key": key, "kind": "row", "row": row })
        })
        .collect::<Vec<_>>();
    grouped.chain(emoji).collect()
}

/// The key of the `at`th group's row.
fn group_key(at: usize) -> String {
    format!("row:{PLACE}:group:{at}")
}

/// A group's record: its row named as Unicode writes it, marked by its
/// first emoji, saying at its right how many it holds, and stepped into
/// for them.
fn group_record(group: &str, first: &str, count: usize, at: u64, words: &Words) -> Value {
    let key = group_key(usize::try_from(at).unwrap_or(usize::MAX));
    let row = json!({
        "key": key,
        "thing": format!("emoji:group:{group}"),
        "place": PLACE,
        "kind": "emoji-group",
        "order": at * SPACED,
        "depth": 0,
        "name": group,
        "glyph": { "code": code(first) },
        "hue": "text",
        "side": [{ "said": words.figure(count) }],
        "acts": [{ "key": "verb:more", "step": "rows" }],
    });
    json!({ "key": key, "kind": "row", "row": row })
}

/// An emoji's mark as ready-rows' `code` says one: its code points in
/// hexadecimal, a space apart, so a joined or toned emoji keeps every one.
fn code(emoji: &str) -> String {
    emoji
        .chars()
        .map(|one| format!("{:X}", u32::from(one)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// An emoji as the list gives it.
struct Named<'a> {
    emoji: &'a str,
    name: &'a str,
    keywords: &'a str,
    group: &'a str,
}

/// What an emoji's row asks for when it's pasted and when it's copied, in
/// the one verb order. Each shuts the launcher as it's asked, so a paste's
/// keys go to the window under it.
fn acts(emoji: &str) -> Value {
    json!([
        { "key": "verb:emoji:paste", "ask": { "ask": "paste", "text": emoji }, "closes": true },
        { "key": "verb:emoji:copy", "ask": { "ask": "copy", "text": emoji }, "closes": true },
        { "key": "verb:details", "step": "drawer" }
    ])
}

/// One emoji's row, the `at`th in the list, its drawer the shell's to
/// make from it. Its name is given a capital, as every row's is, and its
/// thing keeps the name as Unicode writes it, which what's learned of it
/// is kept by. Its copy is asked of the history, so the shell can't tell
/// from its acts alone that y does it, and `actsOnly` says so
/// (ready-rows.md).
fn row(key: &str, at: u64, named: &Named) -> Value {
    json!({
        "key": key,
        "thing": format!("emoji:{}", named.name),
        "place": PLACE,
        "kind": "emoji",
        "order": at * SPACED,
        "name": capitalised(named.name),
        "glyph": { "code": code(named.emoji) },
        "hue": "text",
        "keywords": named.keywords.split_whitespace().collect::<Vec<_>>(),
        "acts": acts(named.emoji),
        "actsOnly": true,
        "drawer": true,
        "held": { "emoji": named.emoji, "group": named.group },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::words;

    const LIST: &str = concat!(
        "[\"😀\", \"grinning face\", \"cheerful grin happy\", \"Smileys & Emotion\"]\n",
        "not an emoji's line\n",
        "[\"👩‍💻\", \"woman technologist\", \"coder software\", \"People & Body\"]\n",
    );

    /// The emoji's rows, without their groups'.
    fn rows() -> Vec<Value> {
        records(LIST, &words())
            .into_iter()
            .map(|record| record["row"].clone())
            .filter(|row| row["kind"] == "emoji")
            .collect()
    }

    #[test]
    fn every_emoji_is_a_row_in_the_lists_order() {
        let records: Vec<Value> = records(LIST, &words())
            .into_iter()
            .filter(|record| record["row"]["kind"] == "emoji")
            .collect();
        assert_eq!(
            records.len(),
            2,
            "the line that isn't an emoji's is left out"
        );
        assert_eq!(records[0]["key"], "row:emoji:0");
        assert_eq!(records[0]["kind"], "row");
        assert_eq!(records[1]["row"]["key"], "row:emoji:1");
        assert!(
            records[0]["row"]["order"].as_u64() < records[1]["row"]["order"].as_u64(),
            "the list's order is kept: {records:?}"
        );
    }

    #[test]
    fn each_group_is_a_row_holding_its_emoji_in_the_order_first_named() {
        let list = concat!(
            "[\"😀\", \"grinning face\", \"\", \"Smileys & Emotion\"]\n",
            "[\"👩‍💻\", \"woman technologist\", \"\", \"People & Body\"]\n",
            "[\"😃\", \"grinning face with big eyes\", \"\", \"Smileys & Emotion\"]\n",
        );
        let rows: Vec<Value> = records(list, &words())
            .into_iter()
            .map(|record| record["row"].clone())
            .collect();
        let groups: Vec<&Value> = rows
            .iter()
            .filter(|row| row["kind"] == "emoji-group")
            .collect();
        assert_eq!(groups.len(), 2, "one row a group: {groups:?}");
        assert_eq!(groups[0]["key"], "row:emoji:group:0");
        assert_eq!(groups[0]["name"], "Smileys & Emotion");
        assert_eq!(groups[1]["name"], "People & Body");
        assert!(
            groups[0]["order"].as_u64() < groups[1]["order"].as_u64(),
            "in the order the list first names them: {groups:?}"
        );
        assert_eq!(
            groups[0]["side"],
            json!([{ "said": "2" }]),
            "it says how many it holds"
        );
        assert_eq!(
            groups[0]["glyph"],
            json!({ "code": "1F600" }),
            "marked by its first"
        );
        assert_eq!(
            groups[0]["acts"],
            json!([{ "key": "verb:more", "step": "rows" }])
        );
        assert!(
            groups.iter().all(|group| group.get("parent").is_none()),
            "{groups:?}"
        );
        let inside = |key: &str| {
            rows.iter()
                .filter(|row| row["parent"] == key)
                .map(|row| row["name"].as_str().unwrap_or("").to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            inside("row:emoji:group:0"),
            ["Grinning face", "Grinning face with big eyes"]
        );
        assert_eq!(inside("row:emoji:group:1"), ["Woman technologist"]);
        assert!(
            rows.iter()
                .filter(|row| row["kind"] == "emoji")
                .all(|row| row["depth"] == 1),
            "each emoji sits a step in: {rows:?}"
        );
    }

    #[test]
    fn a_row_is_named_and_marked_by_its_emoji() {
        let rows = rows();
        assert_eq!(rows[0]["name"], "Grinning face");
        assert_eq!(rows[0]["thing"], "emoji:grinning face");
        assert_eq!(rows[0]["glyph"], json!({ "code": "1F600" }));
        assert_eq!(
            rows[1]["glyph"],
            json!({ "code": "1F469 200D 1F4BB" }),
            "a joined emoji keeps every code point"
        );
        assert_eq!(rows[0]["keywords"], json!(["cheerful", "grin", "happy"]));
        assert_eq!(rows[0]["held"]["group"], "Smileys & Emotion");
    }

    #[test]
    fn an_emoji_pastes_and_copies_itself_shutting_the_launcher() {
        let acts = &rows()[1]["acts"];
        assert_eq!(acts[0]["key"], "verb:emoji:paste");
        assert_eq!(acts[0]["ask"], json!({ "ask": "paste", "text": "👩‍💻" }));
        assert_eq!(acts[1]["key"], "verb:emoji:copy");
        assert_eq!(acts[1]["ask"], json!({ "ask": "copy", "text": "👩‍💻" }));
        assert!(
            acts.as_array()
                .is_some_and(|acts| acts[..2].iter().all(|act| act["closes"] == true)),
            "each act shuts the launcher: {acts}"
        );
        assert_eq!(
            acts[2],
            json!({ "key": "verb:details", "step": "drawer" }),
            "and its details are its drawer"
        );
    }

    #[test]
    fn a_row_holds_only_its_acts_so_it_opens_no_list_of_them() {
        assert!(
            rows().iter().all(|row| row["actsOnly"] == true),
            "the shell reads an asked copy as an act of its own"
        );
    }

    #[test]
    fn a_row_holds_what_its_drawer_says_and_serves_none() {
        let records = records(LIST, &words());
        assert!(
            records.iter().all(|record| record["kind"] == "row"),
            "nothing but rows is served: {records:?}"
        );
        let rows = rows();
        assert!(
            rows.iter().all(|row| row["drawer"] == true),
            "each says it has a drawer, which the shell makes: {rows:?}"
        );
        assert_eq!(rows[1]["name"], "Woman technologist");
        assert_eq!(rows[1]["held"]["group"], "People & Body");
        assert_eq!(rows[1]["held"]["emoji"], "👩‍💻");
        assert_eq!(rows[1]["keywords"], json!(["coder", "software"]));
    }
}
