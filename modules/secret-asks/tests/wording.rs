//! What an ask's line says, in the launcher's own words, as both this
//! daemon and the passwords daemon serve it (each makes its rows here,
//! in row.rs): who asks and for what, the command, where and for how
//! long, and never the words or the shape of 1Password's own prompt, so
//! an agent's ask can't be taken for one, nor one for an agent's ask
//! (H6EDLC, 16 November).

use secret_asks::ask::Ask;
use secret_asks::row::{self, Held, State};
use secret_asks::tree::{Checkout, Tree};
use secret_asks::words::Words;
use serde_json::Value;

/// The launcher's vocabulary, which the daemon says its rows in.
const VOCABULARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../quickshell/launcher/words.json"
);
/// Where the asks' trees are, as `~`.
const HOME: &str = "/home/someone";

/// What 1Password's prompts say as they ask to be let in: its desktop
/// app's "Allow ... to access 1Password", Authorize and Deny, and its
/// unlocking by Touch ID, system authentication or the account password.
/// Lower case, as any word of a line is matched lower-cased.
const PROMPTS: [&str; 13] = [
    "allow",
    "access",
    "authori",
    "approve",
    "deny",
    "unlock",
    "touch id",
    "system authentication",
    "wants to",
    "requesting",
    "sign in",
    "account password",
    "secret key",
];

/// The fields of a row that are said to the machine, never drawn.
const UNDRAWN: [&str; 10] = [
    "key", "thing", "place", "kind", "state", "glyph", "hue", "digest", "as", "ask",
];

fn words() -> (Value, Words) {
    let text = std::fs::read_to_string(VOCABULARY).expect("the vocabulary reads");
    let vocabulary = serde_json::from_str(&text).expect("the vocabulary is JSON");
    let words = Words::from_vocabulary(&text).expect("the vocabulary is JSON");
    (vocabulary, words)
}

/// An ask made up to say none of the prompts' words of its own, so any
/// found is the line's.
fn held(state: State) -> Held {
    Held {
        ask: Ask {
            id: "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35".to_owned(),
            asked_ms: 1_794_459_000_000,
            session: "8IDU4E".to_owned(),
            quest: Some(("QEST-1".to_owned(), "Fire the Pottery Kiln".to_owned())),
            item: "op://Studio/Kiln Token/credential".to_owned(),
            why: "To start the kiln's firing.".to_owned(),
            argv: vec![
                "ssh".to_owned(),
                "kiln.example".to_owned(),
                "fire".to_owned(),
            ],
            tree: format!("{HOME}/src/pottery"),
            given: "KILN_TOKEN".to_owned(),
            minutes: 3,
        },
        tree: Tree {
            commit: Some("4f2a91c03b1d7e6a5b4c3d2e1f0a9b8c7d6e5f4a".to_owned()),
            uncommitted: Some(0),
            checkout: Checkout::None,
            landed: None,
            origin: None,
        },
        state,
        since_ms: 1_794_459_060_000,
    }
}

/// Every word a row draws, its fields said to the machine aside.
fn drawn(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::String(word) => found.push(word.clone()),
        Value::Array(all) => all.iter().for_each(|one| drawn(one, found)),
        Value::Object(fields) => fields
            .iter()
            .filter(|(name, _)| !UNDRAWN.contains(&name.as_str()))
            .for_each(|(_, one)| drawn(one, found)),
        _ => {}
    }
}

/// The words Asks' own things draw beside its rows: its verbs, what a
/// first Return arms, and its drawer's facts' names.
fn asks_own(vocabulary: &Value) -> Vec<String> {
    let mut found = Vec::new();
    for thing in vocabulary["things"].as_array().into_iter().flatten() {
        if thing["in"] == "asks" || thing["id"] == "asks" {
            for field in ["name", "said", "tells"] {
                drawn(&thing[field], &mut found);
            }
        }
    }
    found
}

/// Each word of `said` that carries one of the prompts' words.
fn prompting(said: &[String]) -> Vec<(String, &'static str)> {
    said.iter()
        .flat_map(|word| {
            let lower = word.to_lowercase();
            PROMPTS
                .iter()
                .filter(move |prompt| lower.contains(*prompt))
                .map(move |prompt| (word.clone(), *prompt))
        })
        .collect()
}

#[test]
fn a_waiting_line_says_who_asks_what_runs_where_and_for_how_long() {
    let (_, words) = words();
    let row = row::row(&held(State::Waiting), HOME, &words, true);
    let drawer = &row["drawer"];
    assert_eq!(
        drawer["eyebrow"], "8IDU4E \u{b7} Fire the Pottery Kiln",
        "who asks"
    );
    assert_eq!(
        drawer["title"], "op://Studio/Kiln Token/credential",
        "for what"
    );
    let fact = |name: &str| {
        drawer["facts"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|fact| fact["name"] == name)
            .unwrap_or_else(|| panic!("no {name} fact in {drawer}"))
            .clone()
    };
    assert_eq!(
        fact("Runs")["said"],
        "ssh kiln.example fire",
        "the command and its host"
    );
    assert_eq!(fact("In")["said"], "~/src/pottery at 4f2a91c03b1d", "where");
    assert_eq!(
        fact("Given As")["said"],
        "KILN_TOKEN, for 3 min",
        "how long it's given"
    );
    assert_eq!(fact("Waits Until")["as"], "moment", "how long it waits");
}

#[test]
fn no_line_carries_the_words_of_1passwords_prompt() {
    let (vocabulary, words) = words();
    let own = asks_own(&vocabulary);
    assert!(!own.is_empty(), "Asks' own words are found");
    assert_eq!(prompting(&own), [], "Asks' own words");
    for state in [
        State::Waiting,
        State::Releasing,
        State::Ran { exit: Some(0) },
        State::Ran { exit: None },
        State::Failed {
            why: "the kiln is cold".to_owned(),
        },
        State::Refused,
        State::Ended,
    ] {
        let mut said = Vec::new();
        drawn(
            &row::row(&held(state.clone()), HOME, &words, true),
            &mut said,
        );
        assert_eq!(prompting(&said), [], "the line, {state:?}");
    }
}

#[test]
fn a_waiting_line_isnt_shaped_like_1passwords_prompt() {
    let (vocabulary, words) = words();
    let row = row::row(&held(State::Waiting), HOME, &words, true);
    // 1Password's prompt is its own, under its name and mark, offering
    // Authorize or Deny. A line waiting on Joel never names it, wears
    // the launcher's passwords mark, and offers Release It or Refuse It,
    // by the launcher's verbs, in a row of its list.
    let mut said = Vec::new();
    drawn(&row, &mut said);
    let named: Vec<_> = said
        .iter()
        .filter(|word| word.to_lowercase().contains("1password"))
        .collect();
    assert_eq!(
        named,
        Vec::<&String>::new(),
        "a waiting line never names 1Password"
    );
    assert_eq!(row["kind"], "ask", "a row of Asks' list");
    assert_eq!(row["glyph"], "passwords", "the launcher's mark");
    let acts: Vec<_> = row["acts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|act| act["key"].as_str().unwrap_or("").to_owned())
        .collect();
    assert_eq!(acts, ["verb:release", "verb:refuse"], "its two acts");
    let verb = |id: &str| {
        vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|thing| thing["id"] == id)
            .map(|thing| thing["name"].clone())
    };
    assert_eq!(
        verb("verb:release"),
        Some("Release It".into()),
        "a yes's name"
    );
    assert_eq!(verb("verb:refuse"), Some("Refuse It".into()), "a no's name");
}
