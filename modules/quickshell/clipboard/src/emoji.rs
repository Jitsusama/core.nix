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

use serde_json::{Value, json};

use crate::rows::{Words, capitalised};

/// The launcher's place these rows are drawn in.
pub const PLACE: &str = "emoji";

/// How far apart rows are ordered, as ready-rows.md spaces them.
const SPACED: u64 = 1024;

/// Every emoji's record, its row as the history serves it, from the
/// list's text, in its order: a line that isn't an emoji's is left out
/// rather than refusing the rest.
pub fn records(list: &str, words: &Words) -> Vec<Value> {
    list.lines()
        .filter_map(|line| serde_json::from_str::<[String; 4]>(line).ok())
        .zip(0_u64..)
        .map(|([emoji, name, keywords, group], at)| {
            let key = format!("row:{PLACE}:{at}");
            let named = Named {
                emoji: &emoji,
                name: &name,
                keywords: &keywords,
                group: &group,
            };
            let row = row(&key, at, &named, words);
            json!({ "key": key, "kind": "row", "row": row })
        })
        .collect()
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

/// An emoji's drawer: its group over its name, set as Unicode writes it,
/// its keywords as words where it has any, and its code points as
/// Unicode writes them, raw, folded behind za.
fn drawer(named: &Named, words: &Words) -> Value {
    let keywords = named.keywords.split_whitespace().collect::<Vec<_>>();
    let facts = if keywords.is_empty() {
        Vec::new()
    } else {
        vec![json!({
            "name": words.name("emoji:keywords"),
            "said": keywords.join(", "),
            "words": true,
        })]
    };
    let points = named
        .emoji
        .chars()
        .map(|one| format!("U+{:04X}", u32::from(one)))
        .collect::<Vec<_>>()
        .join(" ");
    json!({
        "eyebrow": named.group,
        "eyebrowNames": true,
        "title": capitalised(named.name),
        "facts": facts,
        "nerd": [{ "label": words.name("emoji:code"), "value": points }],
    })
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

/// One emoji's row, the `at`th in the list. Its name is given a capital,
/// as every row's is, and its thing keeps the name as Unicode writes it,
/// which what's learned of it is kept by. Its copy is asked of the
/// history, so the shell can't tell from its acts alone that y does it,
/// and `actsOnly` says so (ready-rows.md).
fn row(key: &str, at: u64, named: &Named, words: &Words) -> Value {
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
        "drawer": drawer(named, words),
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

    fn rows() -> Vec<Value> {
        records(LIST, &words())
            .into_iter()
            .map(|record| record["row"].clone())
            .collect()
    }

    #[test]
    fn every_emoji_is_a_row_in_the_lists_order() {
        let records = records(LIST, &words());
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
    fn its_drawer_says_its_group_keywords_and_code_points() {
        let drawer = &rows()[1]["drawer"];
        assert_eq!(drawer["eyebrow"], "People & Body");
        assert_eq!(
            drawer["eyebrowNames"], true,
            "a group is set as Unicode writes it"
        );
        assert_eq!(drawer["title"], "Woman technologist");
        assert_eq!(
            drawer["facts"],
            json!([{ "name": "Keywords", "said": "coder, software", "words": true }])
        );
        assert_eq!(
            drawer["nerd"],
            json!([{ "label": "Code Points", "value": "U+1F469 U+200D U+1F4BB" }]),
            "every code point, as Unicode writes them"
        );
    }

    #[test]
    fn one_with_no_keywords_says_none() {
        let records = records(
            "[\"🫠\", \"melting face\", \"\", \"Smileys & Emotion\"]",
            &words(),
        );
        assert_eq!(records[0]["row"]["drawer"]["facts"], json!([]));
    }
}
