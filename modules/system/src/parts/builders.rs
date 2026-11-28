//! The builders the machines send Nix builds to, from the line `mycelium
//! machines` writes after its machines. A builder is no machine of the
//! realm: no cell runs on it, so it has no cells, network or code to tie.
//! What it has is whether it takes builds, and that is what it's judged by.
//! Its own module because it changes for the builders' reasons (what the
//! node and nix-offload say of them), which the machines don't share.
//!
//! What the node can say bounds what this can. It hears the builders
//! nix-offload names as standing or beside a machine, and nix-offload names
//! one only while it answers, so one it leaves out, as full or failing its
//! proofs, drops out of the list rather than turning up worth a look. A
//! dialled builder is said answering, not answering or unknown, with the
//! node's reason; only a lent one is said running, with its lender's word
//! for how its proofs stand, or not running. So a dialled builder turns up
//! not answering only in the moment before nix-offload drops it, or when the
//! node can't reach one nix-offload can.

use serde_json::Value;

use super::plain_name;
use crate::record::{Health, Look, Thing};

/// The most of a node's reason a builder's line carries, in characters:
/// a reason is one plain clause, and past this it's cut at a word.
const REASON_AT_MOST: usize = 160;
/// What a cut reason ends with.
const CUT: char = '\u{2026}';
/// nix-offload's words for how a builder's proofs stand (its proof.rs),
/// which the node says in a builder's `said` where it has no reason to
/// give: a state of the proofs, read as one, and never said as a reason.
const PROOFS: [&str; 6] = ["none", "unproven", "passing", "failing", "dropped", "full"];

/// Each builder the answer names, worst first, then by name, with the
/// words for a side of what in it is worth a look. None where the answer
/// names none: one from a node too old to say them, or one that couldn't
/// read them, since the node then writes no builders' line at all. Two
/// builders whose names are drawn alike are one to the reader and one
/// record, so only the worse is kept: a fine one never hides a failing one.
/// Two as bad are joined, each saying its own words, so neither the node's
/// order nor a tie decides which the reader hears of. Builders without a
/// name are all said unnamed, so they too are one, said at its worst.
pub fn builders(said: &Value) -> Vec<(Thing, Vec<String>)> {
    let mut read: Vec<(Thing, Vec<String>)> = said
        .pointer("/Ok/Builders")
        .and_then(Value::as_array)
        .map(|all| all.iter().map(builder).collect())
        .unwrap_or_default();
    // By what each says after its health and name, so builders drawn alike
    // meet in an order of their own rather than the node's; one saying
    // what another drawn alike says adds nothing to it.
    read.sort_by(|(a, _), (b, _)| {
        b.health
            .cmp(&a.health)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.says.cmp(&b.says))
    });
    read.dedup_by(|(a, _), (b, _)| a.name == b.name && a.says == b.says);
    let mut kept: Vec<(Thing, Vec<String>)> = Vec::new();
    let mut at = std::collections::HashMap::new();
    for (thing, looks) in read {
        if let Some(&first) = at.get(&thing.name) {
            joined(&mut kept[first], thing, looks);
        } else {
            at.insert(thing.name.clone(), kept.len());
            kept.push((thing, looks));
        }
    }
    kept
}

/// `first` with what `other`, a builder drawn alike after it, says, where it
/// stands as badly: its words after the first's, and each look it raises
/// beside the first's, or the words of one they share after the first's
/// own. One better than the first adds nothing, so a fine builder never
/// softens a failing one.
fn joined(first: &mut (Thing, Vec<String>), other: Thing, words: Vec<String>) {
    let (thing, said) = first;
    if other.health != thing.health {
        return;
    }
    thing.says = format!("{}; {}", thing.says, other.says);
    for (look, word) in other.looks.into_iter().zip(words) {
        if let Some(shared) = thing.side.iter().position(|side| *side == look.side) {
            said[shared] = format!("{}; {word}", said[shared]);
        } else {
            thing.side.push(look.side.clone());
            thing.looks.push(look);
            said.push(word);
        }
    }
}

/// One builder: how it stands, and what in that is worth a look, as a
/// side says it. One without a name is kept and said unnamed, since one
/// that can't be named may still be one that isn't answering.
///
/// Only a builder that should take builds and can't is worth a look: one
/// that didn't answer, or a lent one whose lender says its proofs left it
/// out or its store is full. One that rests until its lender runs it waits
/// for work, and one nobody has heard from is often so by design, as a
/// builder behind a jump host that isn't dialled so as not to wake it;
/// both are said, and neither is a look nobody could clear. A state or a
/// lender's word this build doesn't know is worth a look, said in the
/// node's own words, rather than passed as good: it may be a way of not
/// building that this build was never told of. A look carries the node's
/// reason whole, since what to do about a builder is in it. Its name is
/// made plain as any text the system shows, and a state or lender's word is
/// matched in the one form a look would show it (`plain`), so no look ever
/// names a word this build knows. The map draws a builder by the node's own
/// name, so where the plain one differs the builder is called by that too,
/// for the page to find it there; a name the map drops, an empty one, it is
/// called by none.
fn builder(said: &Value) -> (Thing, Vec<String>) {
    let name = plain_name(said);
    let called = said
        .get("name")
        .and_then(Value::as_str)
        .filter(|own| !own.is_empty() && *own != name)
        .map(str::to_owned);
    let lender = said.get("said").and_then(Value::as_str).and_then(plain);
    let proof = lender.as_deref().filter(|said| PROOFS.contains(said));
    let why = lender.clone().filter(|_| proof.is_none());
    let unknown = |words: String| (Health::Watch, words.clone(), Some(words));
    let lent = said.get("lent").and_then(Value::as_bool) == Some(true);
    let takes = match said.get("jobs").and_then(Value::as_u64) {
        Some(0) | None => "takes builds".to_owned(),
        Some(1) => "takes 1 build at a time".to_owned(),
        Some(jobs) => format!("takes {jobs} builds at once"),
    };
    let with_why = |words: &str| match &why {
        Some(why) => format!("{words} ({why})"),
        None => words.to_owned(),
    };
    // A state there but unreadable, not a string or nothing once plain, is no
    // state this build knows, so it fails closed as one; only a state the node
    // left out is unheard.
    let state = match said.get("state") {
        None | Some(Value::Null) => "unknown".to_owned(),
        Some(state) => state
            .as_str()
            .and_then(plain)
            .unwrap_or_else(|| "unreadable".to_owned()),
    };
    let (health, says, look) = match state.as_str() {
        "answering" => (Health::Good, format!("answers and {takes}"), None),
        "running" => match &why {
            Some(why) => unknown(format!("runs, its lender saying {why}")),
            None => running(lent, &takes, proof),
        },
        "not running" => (
            Health::Good,
            with_why("rests until its lender runs it"),
            None,
        ),
        "not answering" => (
            Health::Watch,
            with_why("isn't answering"),
            Some("not answering".to_owned()),
        ),
        "unknown" => (Health::Good, with_why("hasn't been heard from"), None),
        other => unknown(with_why(&format!("is {other}"))),
    };
    let looks: Vec<String> = look.iter().map(|_| says.clone()).collect();
    let thing = Thing {
        kind: "builder",
        name,
        health,
        says,
        looks: look
            .iter()
            .map(|side| Look {
                health,
                side: side.clone(),
                matters: matters(side).to_owned(),
            })
            .collect(),
        side: look.into_iter().collect(),
        called,
        address: None,
        ties: Value::Null,
    };
    (thing, looks)
}

/// Why a builder's look matters: each is a builder the machines can't send
/// builds to, which leaves the rest fewer to share them; a state this build
/// doesn't know may be one.
fn matters(side: &str) -> &'static str {
    match side {
        "not answering" => {
            "The machines can't send it builds until it answers, so the builders left \
             share them."
        }
        "left out, its proofs failing" => {
            "nix-offload sends it no builds until its proofs pass again, so the builders \
             left share them."
        }
        "its store full" => {
            "nix-offload sends it builds only while no builder with room can take them, \
             so those with room share them until its store has room again."
        }
        _ => "It stands in a state this daemon doesn't know, so it may not be taking builds.",
    }
}

/// A builder its lender runs: it takes builds unless nix-offload's proofs
/// left it out. One found full still takes them while no builder with room
/// can stand in, as nix-offload's `for_anything` names it, so it's said as
/// building last. One failed proof is said but no look, as the next may pass.
fn running(lent: bool, takes: &str, proof: Option<&str>) -> (Health, String, Option<String>) {
    let runs = if lent {
        format!("is lent and {takes}")
    } else {
        format!("runs and {takes}")
    };
    match proof {
        Some("dropped") => (
            Health::Watch,
            "runs but is left out, its proofs failing".to_owned(),
            Some("left out, its proofs failing".to_owned()),
        ),
        Some("full") => (
            Health::Watch,
            "runs with its store full, so it builds only when no other can".to_owned(),
            Some("its store full".to_owned()),
        ),
        Some("failing") => (
            Health::Good,
            format!("{runs}, though its last proof failed"),
            None,
        ),
        _ => (Health::Good, runs, None),
    }
}

/// A node's words as one plain clause of a line: the first line, made
/// plain as every text the system shows is (`plain::words`, which trims it
/// too), with no closing stop, and cut at a word past `REASON_AT_MOST`;
/// none when nothing is left. A state and a lender's word are read in this
/// form as well as said in it, so what is matched is what a look shows.
fn plain(said: &str) -> Option<String> {
    let line = crate::plain::words(said.lines().next().unwrap_or(""));
    let line = line.trim_end_matches(['.', ';', ':', ',', ' ']);
    if line.is_empty() {
        return None;
    }
    if line.chars().count() <= REASON_AT_MOST {
        return Some(line.to_owned());
    }
    let kept: String = line.chars().take(REASON_AT_MOST - 1).collect();
    let at_word = kept.rfind(' ').map_or(kept.as_str(), |at| &kept[..at]);
    Some(format!(
        "{}{CUT}",
        at_word.trim_end_matches(['.', ';', ':', ',', ' '])
    ))
}

#[cfg(test)]
mod tests {
    use super::builders;
    use crate::record::Health;
    use serde_json::{Value, json};

    fn read(builders_said: &Value) -> Vec<(String, Health, String, Vec<String>)> {
        builders(&json!({ "Ok": { "Builders": builders_said } }))
            .into_iter()
            .map(|(thing, looks)| {
                assert_eq!(thing.kind, "builder", "a builder is no machine");
                assert_eq!(
                    looks.len(),
                    thing.side.len(),
                    "each look has its side's words"
                );
                let sides: Vec<&str> = thing.looks.iter().map(|look| look.side.as_str()).collect();
                assert_eq!(sides, thing.side, "the looks say the side's looks");
                for look in &thing.looks {
                    assert!(look.matters.ends_with('.'), "{look:?} says why it matters");
                    assert_eq!(
                        look.health, thing.health,
                        "{look:?} isn't as bad as the builder it is the one look of"
                    );
                }
                (thing.name, thing.health, thing.says, thing.side)
            })
            .collect()
    }

    #[test]
    fn a_builder_that_takes_builds_is_good_and_says_how_many() {
        assert_eq!(
            read(&json!([
                { "name": "osprey", "state": "running", "lent": true, "jobs": 16,
                  "said": "passing" },
                { "name": "heron", "state": "answering", "jobs": 1 },
                { "name": "wren", "state": "running" },
            ])),
            [
                (
                    "heron".to_owned(),
                    Health::Good,
                    "answers and takes 1 build at a time".to_owned(),
                    vec![]
                ),
                (
                    "osprey".to_owned(),
                    Health::Good,
                    "is lent and takes 16 builds at once".to_owned(),
                    vec![]
                ),
                (
                    "wren".to_owned(),
                    Health::Good,
                    "runs and takes builds".to_owned(),
                    vec![]
                ),
            ],
            "each is said by name, as it stands"
        );
    }

    #[test]
    fn only_a_builder_that_should_build_and_cant_is_worth_a_look() {
        assert_eq!(
            read(&json!([
                { "name": "tern", "state": "unknown",
                  "said": "it is reached through a jump host, which isn't dialled." },
                { "name": "gull", "state": "not answering", "said": "connection refused" },
                { "name": "kestrel", "state": "not running", "lent": true,
                  "said": "passing" },
                { "name": "osprey", "state": "running", "said": "dropped" },
                { "name": "heron", "state": "running", "said": "full" },
                { "name": "wren", "state": "running", "said": "failing" },
                { "name": "swift", "state": "resting" },
                { "name": "lark", "state": "running", "lent": true, "said": "melting" },
            ])),
            [
                (
                    "gull".to_owned(),
                    Health::Watch,
                    "isn't answering (connection refused)".to_owned(),
                    vec!["not answering".to_owned()]
                ),
                (
                    "heron".to_owned(),
                    Health::Watch,
                    "runs with its store full, so it builds only when no other can".to_owned(),
                    vec!["its store full".to_owned()]
                ),
                (
                    "lark".to_owned(),
                    Health::Watch,
                    "runs, its lender saying melting".to_owned(),
                    vec!["runs, its lender saying melting".to_owned()]
                ),
                (
                    "osprey".to_owned(),
                    Health::Watch,
                    "runs but is left out, its proofs failing".to_owned(),
                    vec!["left out, its proofs failing".to_owned()]
                ),
                (
                    "swift".to_owned(),
                    Health::Watch,
                    "is resting".to_owned(),
                    vec!["is resting".to_owned()]
                ),
                (
                    "kestrel".to_owned(),
                    Health::Good,
                    "rests until its lender runs it".to_owned(),
                    vec![]
                ),
                (
                    "tern".to_owned(),
                    Health::Good,
                    "hasn't been heard from (it is reached through a jump host, which isn't \
                     dialled)"
                        .to_owned(),
                    vec![]
                ),
                (
                    "wren".to_owned(),
                    Health::Good,
                    "runs and takes builds, though its last proof failed".to_owned(),
                    vec![]
                ),
            ],
            "the worst first, then by name; a builder resting or unheard is said, not a look, \
             a proof's word is no reason, and a state or a lender's word this build doesn't \
             know is a look"
        );
    }

    #[test]
    fn a_state_there_but_unreadable_is_worth_a_look() {
        for state in [
            json!(7),
            json!(""),
            json!("\u{7}"),
            json!({ "Running": {} }),
        ] {
            let said = read(&json!([{ "name": "kite", "state": state }]));
            assert_eq!(
                said[0].1,
                Health::Watch,
                "{state} is no state this build reads"
            );
            assert_eq!(
                said[0].3,
                vec!["is unreadable".to_owned()],
                "{state} is a look"
            );
        }
    }

    #[test]
    fn a_builder_is_read_leniently_and_its_reason_kept_plain() {
        let long = format!("{} tail", "word ".repeat(40));
        let said = read(&json!([
            { "state": "running" },
            { "name": "  ", "state": "running" },
            { "name": "osprey", "state": "not answering",
              "said": "refused\u{1b}[31m red\u{202e} der\nsecond line" },
            { "name": "heron", "state": "not answering", "said": long },
            { "name": "gull", "state": "not answering", "said": "über ".repeat(40) },
            { "name": "wren", "state": "not answering", "said": " . " },
            { "name": "tern" },
        ]));
        let says: Vec<&str> = said.iter().map(|(_, _, says, _)| says.as_str()).collect();
        assert_eq!(
            said.len(),
            6,
            "two builders without a name are kept as one: {said:?}"
        );
        assert_eq!(
            (said[5].0.as_str(), said[5].2.as_str()),
            ("unnamed", "runs and takes builds"),
            "a builder without a name is said unnamed, as it stands"
        );
        assert_eq!(
            says[2], "isn't answering (refused red der)",
            "only the first line is kept, an escape gone whole and a turn of the text gone"
        );
        assert_eq!(
            says[1],
            format!("isn't answering ({}word\u{2026})", "word ".repeat(30)),
            "a long reason is cut at the last word that fits"
        );
        assert_eq!(
            says[0],
            format!("isn't answering ({}über\u{2026})", "über ".repeat(30)),
            "a reason is cut by characters, not bytes"
        );
        assert_eq!(says[3], "isn't answering", "a reason of nothing is none");
        assert_eq!(
            says[4], "hasn't been heard from",
            "a builder with no state is unheard"
        );
        assert!(
            builders(&json!({ "Ok": { "Machines": [] } })).is_empty(),
            "an answer naming no builders has none"
        );
    }

    #[test]
    fn a_name_a_state_and_a_lender_s_word_are_read_as_plain_words() {
        let said = read(&json!([
            { "name": "os\u{202e}prey\u{1b}[31m", "state": " answering " },
            { "name": " kite\u{200b}", "state": "running\n", "lent": true,
              "said": " passing " },
            { "name": "\u{200b}\u{1b}[0m", "state": "running" },
        ]));
        let named: Vec<(&str, &str)> = said
            .iter()
            .map(|(name, _, says, _)| (name.as_str(), says.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("kite", "is lent and takes builds"),
                ("osprey", "answers and takes builds"),
                ("unnamed", "runs and takes builds"),
            ],
            "a name is shown as plain words, one with none left said unnamed, and a padded \
             state or lender's word is the word this build knows"
        );
        assert!(
            said.iter().all(|(_, health, _, _)| *health == Health::Good),
            "no padded known word is worth a look: {said:?}"
        );
    }

    #[test]
    fn a_known_word_with_hidden_characters_in_it_is_still_that_word() {
        let said = read(&json!([
            { "name": "osprey", "state": "\u{200b}answer\u{1b}[0ming" },
            { "name": "kite", "state": "run\u{200b}ning", "lent": true,
              "said": "pass\u{2060}ing" },
        ]));
        let named: Vec<(&str, Health, &str)> = said
            .iter()
            .map(|(name, health, says, _)| (name.as_str(), *health, says.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("kite", Health::Good, "is lent and takes builds"),
                ("osprey", Health::Good, "answers and takes builds"),
            ],
            "a state or lender's word is matched as it reads, its hidden characters gone"
        );
    }

    #[test]
    fn a_known_word_is_matched_in_the_form_a_look_would_show_it() {
        let said = read(&json!([
            { "name": "osprey", "state": "answering ." },
            { "name": "kite", "state": "running;", "lent": true,
              "said": " pass\u{200b}ing." },
            { "name": "tern", "state": "unknown." },
        ]));
        let named: Vec<(&str, Health, &str)> = said
            .iter()
            .map(|(name, health, says, _)| (name.as_str(), *health, says.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("kite", Health::Good, "is lent and takes builds"),
                ("osprey", Health::Good, "answers and takes builds"),
                ("tern", Health::Good, "hasn't been heard from"),
            ],
            "a state or lender's word that a look would show as a known word is that word, \
             so no look names a word this build knows"
        );
    }

    #[test]
    fn a_state_and_a_lender_s_word_are_read_by_their_first_line_as_a_reason_is() {
        let said = read(&json!([
            { "name": "osprey", "state": "answering\nsince noon" },
            { "name": "kite", "state": "running", "said": "dropped\nsince noon" },
        ]));
        let named: Vec<(&str, Health, &str)> = said
            .iter()
            .map(|(name, health, says, _)| (name.as_str(), *health, says.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                (
                    "kite",
                    Health::Watch,
                    "runs but is left out, its proofs failing"
                ),
                ("osprey", Health::Good, "answers and takes builds"),
            ],
            "a state's or a lender's second line is no part of its word, as it is none of a \
             reason's"
        );
    }

    #[test]
    fn a_padded_unknown_is_unheard_as_unknown_is() {
        for state in [" unknown ", "\u{200b}unknown\u{1b}[0m", "unknown"] {
            let said = read(&json!([{ "name": "tern", "state": state }]));
            assert_eq!(
                (said[0].1, said[0].2.as_str(), said[0].3.len()),
                (Health::Good, "hasn't been heard from", 0),
                "{state:?} is the node's unknown, a builder unheard by design, not a look"
            );
        }
    }

    #[test]
    fn builders_whose_names_read_alike_are_one_said_at_its_worst() {
        let failing = json!({ "name": "kite\u{200b}", "state": "not answering" });
        let fine = json!({ "name": "kite", "state": "answering" });
        for builders in [json!([failing, fine]), json!([fine, failing])] {
            let said = read(&builders);
            let named: Vec<(&str, Health)> = said
                .iter()
                .map(|(name, health, _, _)| (name.as_str(), *health))
                .collect();
            assert_eq!(
                named,
                vec![("kite", Health::Watch)],
                "two builders drawn as one name are one, and a fine one never hides a failing \
                 one: {builders}"
            );
        }
    }

    #[test]
    fn builders_drawn_alike_and_as_bad_are_one_whatever_the_node_s_order() {
        let refused = json!({ "name": "kite\u{200b}", "state": "not answering",
                              "said": "refused" });
        let timed_out = json!({ "name": "kite", "state": "not answering",
                                "said": "timed out" });
        let full = json!({ "name": " kite", "state": "running", "said": "full" });
        let answers = json!({ "name": "heron", "state": "answering" });
        let runs = json!({ "name": "heron\u{200b}", "state": "running" });
        let refused_says = "isn't answering (refused)";
        let both_refused = format!("{refused_says}; isn't answering (timed out)");
        let full_says = "runs with its store full, so it builds only when no other can";
        for (pair, says, side, words) in [
            (
                [&refused, &timed_out],
                both_refused.clone(),
                vec!["not answering"],
                vec![both_refused.clone()],
            ),
            (
                [&refused, &full],
                format!("{refused_says}; {full_says}"),
                vec!["not answering", "its store full"],
                vec![refused_says.to_owned(), full_says.to_owned()],
            ),
            (
                [&answers, &runs],
                "answers and takes builds; runs and takes builds".to_owned(),
                vec![],
                vec![],
            ),
        ] {
            let [one, other] = pair;
            let either = [json!([one, other]), json!([other, one])].map(|listed| {
                builders(&json!({ "Ok": { "Builders": listed } }))
                    .into_iter()
                    .map(|(thing, looks)| (thing.says, thing.side, looks))
                    .collect::<Vec<_>>()
            });
            let side: Vec<String> = side.into_iter().map(str::to_owned).collect();
            let joined = vec![(says, side, words)];
            assert_eq!(
                either,
                [joined.clone(), joined],
                "two builders drawn alike and as bad are one saying both, in either order: \
                 {one}, {other}"
            );
        }
    }

    #[test]
    fn a_builder_without_a_name_is_kept_and_said_unnamed() {
        for nameless in [
            json!({ "name": "\u{200b}", "state": "not answering" }),
            json!({ "name": "", "state": "not answering" }),
            json!({ "state": "not answering" }),
        ] {
            let said = read(&json!([nameless, { "name": "osprey", "state": "answering" }]));
            let named: Vec<(&str, Health, String)> = said
                .iter()
                .map(|(name, health, _, side)| (name.as_str(), *health, side.join(", ")))
                .collect();
            assert_eq!(
                named,
                vec![
                    ("unnamed", Health::Watch, "not answering".to_owned()),
                    ("osprey", Health::Good, String::new()),
                ],
                "a builder that can't be named is still listed, and one not answering is a look \
                 as a named one is: {nameless}"
            );
        }
    }

    #[test]
    fn a_builder_is_called_by_the_node_s_own_name_where_it_is_shown_by_another() {
        let said = builders(&json!({ "Ok": { "Builders": [
            { "name": "kite\u{200b}", "state": "answering" },
            { "name": "osprey", "state": "answering" },
            { "name": "\u{200b}", "state": "not answering" },
        ] } }));
        let called: Vec<(&str, Option<&str>)> = said
            .iter()
            .map(|(thing, _)| (thing.name.as_str(), thing.called.as_deref()))
            .collect();
        assert_eq!(
            called,
            vec![
                ("unnamed", Some("\u{200b}")),
                ("kite", Some("kite\u{200b}")),
                ("osprey", None),
            ],
            "a builder shown by a plain name the map doesn't know is called by the node's own"
        );
        for nameless in [json!(""), json!(7), Value::Null] {
            let said = builders(&json!({ "Ok": { "Builders": [{ "name": nameless }] } }));
            assert_eq!(
                said[0].0.called, None,
                "a builder named {nameless} gives the map no name to find it by"
            );
        }
    }

    #[test]
    fn builders_without_a_name_are_one_said_at_its_worst() {
        let failing = json!({ "name": "\u{200b}", "state": "not answering" });
        let fine = json!({ "state": "answering" });
        for builders in [json!([failing, fine]), json!([fine, failing])] {
            let said = read(&builders);
            let named: Vec<(&str, Health)> = said
                .iter()
                .map(|(name, health, _, _)| (name.as_str(), *health))
                .collect();
            assert_eq!(
                named,
                vec![("unnamed", Health::Watch)],
                "two builders without a name are one, and a fine one never hides a failing \
                 one: {builders}"
            );
        }
    }
}
