//! Emoji's rows, ready for the launcher to draw (Y2YBTS's ready-rows.md):
//! every emoji in the list built for the launcher (launcher/emoji.nix), a
//! line of JSON each, `[emoji, name, keywords, group]`, in Unicode's order,
//! which each row's `order` keeps, so Emoji.qml reads no file and makes no
//! row. The history serves them since it's what pastes them. Each row's
//! keywords travel beside its name for the shell's own search, which finds
//! an emoji by what it means as well as what it's called (Emoji.js), and
//! nothing draws them. They're made once, as the history starts, since the
//! list never changes while it runs.

use serde_json::{Value, json};

use crate::rows::capitalised;

/// The launcher's place these rows are drawn in.
pub const PLACE: &str = "emoji";

/// How far apart rows are ordered, as ready-rows.md spaces them.
const SPACED: u64 = 1024;

/// Every emoji's record, its row as the history serves it, from the
/// list's text, in its order: a line that isn't an emoji's is left out
/// rather than refusing the rest.
pub fn records(list: &str) -> Vec<Value> {
    list.lines()
        .filter_map(|line| serde_json::from_str::<[String; 4]>(line).ok())
        .zip(0_u64..)
        .map(|([emoji, name, keywords, group], at)| {
            let key = format!("row:{PLACE}:{at}");
            let row = row(&key, at, &emoji, &name, &keywords, &group);
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

/// What an emoji's row asks for when it's pasted and when it's copied, in
/// the one verb order. Each shuts the launcher as it's asked, so a paste's
/// keys go to the window under it.
fn acts(emoji: &str) -> Value {
    json!([
        { "key": "verb:emoji:paste", "ask": { "ask": "paste", "text": emoji }, "closes": true },
        { "key": "verb:emoji:copy", "ask": { "ask": "copy", "text": emoji }, "closes": true }
    ])
}

/// One emoji's row, the `at`th in the list. Its name is given a capital,
/// as every row's is, and its thing keeps the name as Unicode writes it,
/// which what's learned of it is kept by.
fn row(key: &str, at: u64, emoji: &str, name: &str, keywords: &str, group: &str) -> Value {
    json!({
        "key": key,
        "thing": format!("emoji:{name}"),
        "place": PLACE,
        "kind": "emoji",
        "order": at * SPACED,
        "name": capitalised(name),
        "glyph": { "code": code(emoji) },
        "hue": "text",
        "keywords": keywords.split_whitespace().collect::<Vec<_>>(),
        "acts": acts(emoji),
        "held": { "emoji": emoji, "group": group },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = concat!(
        "[\"😀\", \"grinning face\", \"cheerful grin happy\", \"Smileys & Emotion\"]\n",
        "not an emoji's line\n",
        "[\"👩‍💻\", \"woman technologist\", \"coder software\", \"People & Body\"]\n",
    );

    fn rows() -> Vec<Value> {
        records(LIST)
            .into_iter()
            .map(|record| record["row"].clone())
            .collect()
    }

    #[test]
    fn every_emoji_is_a_row_in_the_lists_order() {
        let records = records(LIST);
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
                .is_some_and(|acts| acts.iter().all(|act| act["closes"] == true)),
            "each act shuts the launcher: {acts}"
        );
    }
}
