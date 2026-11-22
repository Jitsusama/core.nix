//! The builders the machines send Nix builds to, from the line `mycelium
//! machines` writes after its machines. A builder is no machine of the
//! realm: no cell runs on it, so it has no cells, network or code to tie.
//! What it has is whether it takes builds, and that is what it's judged by.
//! Its own module because it changes for the builders' reasons (what the
//! node and nix-offload say of them), which the machines don't share.

use serde_json::Value;

use crate::record::{Health, Thing};

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
/// names none, as one from a node too old to say them.
pub fn builders(said: &Value) -> Vec<(Thing, Vec<String>)> {
    let mut read: Vec<(Thing, Vec<String>)> = said
        .pointer("/Ok/Builders")
        .and_then(Value::as_array)
        .map(|all| all.iter().filter_map(builder).collect())
        .unwrap_or_default();
    read.sort_by(|(a, _), (b, _)| b.health.cmp(&a.health).then_with(|| a.name.cmp(&b.name)));
    read
}

/// One builder: how it stands, and what in that is worth a look, as a
/// side says it; none for a builder without a name.
///
/// Only a builder that should take builds and can't is worth a look: one
/// that didn't answer, or one nix-offload left out after its proofs failed
/// or found its store full. One that rests until its lender runs it waits
/// for work, and one nobody has heard from is often so by design, as a
/// builder behind a jump host that isn't dialled so as not to wake it;
/// both are said, and neither is a look nobody could clear. A state this
/// build doesn't know is said in the node's own word.
fn builder(said: &Value) -> Option<(Thing, Vec<String>)> {
    let name = said.get("name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    let word = |key: &str| said.get(key).and_then(Value::as_str);
    let proof = word("said").filter(|said| PROOFS.contains(said));
    let why = word("said").filter(|_| proof.is_none()).and_then(plain);
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
    let (health, says, look) = match word("state").unwrap_or("unknown") {
        "answering" => (Health::Good, format!("answers and {takes}"), None),
        "running" => running(lent, &takes, proof),
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
        other => match plain(other).filter(|_| other != "unknown") {
            Some(state) => (Health::Good, with_why(&format!("is {state}")), None),
            None => (Health::Good, with_why("hasn't been heard from"), None),
        },
    };
    let looks: Vec<String> = look.iter().map(|_| says.clone()).collect();
    let thing = Thing {
        kind: "builder",
        name: name.to_owned(),
        health,
        says,
        side: look.into_iter().collect(),
        address: None,
        ties: Value::Null,
    };
    Some((thing, looks))
}

/// A builder its lender runs: it takes builds unless nix-offload's proofs
/// left it out, or found its store full. One failed proof is said but no
/// look, as the next may pass.
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
            "runs but is left out, its store full".to_owned(),
            Some("left out, its store full".to_owned()),
        ),
        Some("failing") => (
            Health::Good,
            format!("{runs}, though its last proof failed"),
            None,
        ),
        _ => (Health::Good, runs, None),
    }
}

/// A node's words as one plain clause of a line: the first line, with
/// what a terminal would act on as a space, no closing stop, and cut at a
/// word past `REASON_AT_MOST`; none when nothing is left.
fn plain(said: &str) -> Option<String> {
    let line: String = said
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .map(|each| if each.is_control() { ' ' } else { each })
        .collect();
    let line = line.trim().trim_end_matches(['.', ';', ':', ',', ' ']);
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
                    "runs but is left out, its store full".to_owned(),
                    vec!["left out, its store full".to_owned()]
                ),
                (
                    "osprey".to_owned(),
                    Health::Watch,
                    "runs but is left out, its proofs failing".to_owned(),
                    vec!["left out, its proofs failing".to_owned()]
                ),
                (
                    "kestrel".to_owned(),
                    Health::Good,
                    "rests until its lender runs it".to_owned(),
                    vec![]
                ),
                (
                    "swift".to_owned(),
                    Health::Good,
                    "is resting".to_owned(),
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
             and a proof's word is no reason"
        );
    }

    #[test]
    fn a_builder_is_read_leniently_and_its_reason_kept_plain() {
        let long = format!("{} tail", "word ".repeat(40));
        let said = read(&json!([
            { "state": "running" },
            { "name": "  ", "state": "running" },
            { "name": "osprey", "state": "not answering",
              "said": "refused\u{1b}[31m red\nsecond line" },
            { "name": "heron", "state": "not answering", "said": long },
            { "name": "wren", "state": "not answering", "said": " . " },
            { "name": "tern" },
        ]));
        let says: Vec<&str> = said.iter().map(|(_, _, says, _)| says.as_str()).collect();
        assert_eq!(said.len(), 4, "a builder without a name is left out");
        assert_eq!(
            says[1], "isn't answering (refused [31m red)",
            "only the first line is kept, a control character as a space"
        );
        assert!(
            says[0].ends_with("word\u{2026})") && says[0].chars().count() < 200,
            "a long reason is cut at a word: {}",
            says[0]
        );
        assert_eq!(says[2], "isn't answering", "a reason of nothing is none");
        assert_eq!(
            says[3], "hasn't been heard from",
            "a builder with no state is unheard"
        );
        assert!(
            builders(&json!({ "Ok": { "Machines": [] } })).is_empty(),
            "an answer naming no builders has none"
        );
    }
}
