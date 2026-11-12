//! The fixture the launcher's own checks serve from a stand-in
//! (fixture.json): two asks, one waiting and one that ran, as the daemon
//! serves them, in the launcher's own words, and its hello. Holding it to
//! what the daemon makes keeps the shell's checks drawing what's real.

use ready_rows::kept::Kept;
use secret_asks::ask::Ask;
use secret_asks::row::{self, Held, State};
use secret_asks::tree::Tree;
use secret_asks::words::Words;
use serde_json::{Value, json};

/// The launcher's vocabulary, which the daemon says its rows in.
const VOCABULARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../quickshell/launcher/words.json"
);
/// Where the asks' trees are, as `~`.
const HOME: &str = "/home/someone";

fn held(id: &str, asked_ms: i64, state: State, uncommitted: u64) -> Held {
    Held {
        ask: Ask {
            id: id.to_owned(),
            asked_ms,
            session: "8IDU4E".to_owned(),
            quest: Some(("QEST-1".to_owned(), "Release the Package Index".to_owned())),
            item: "op://Private/Release Test/password".to_owned(),
            why: "To try the release line.".to_owned(),
            argv: vec!["secret-asks".to_owned(), "received".to_owned()],
            tree: format!("{HOME}/src/package-index"),
            given: "RELEASED".to_owned(),
            minutes: 1,
        },
        tree: Tree {
            commit: Some("4f2a91c03b1d7e6a5b4c3d2e1f0a9b8c7d6e5f4a".to_owned()),
            uncommitted: Some(uncommitted),
        },
        state,
        since_ms: asked_ms + 60_000,
    }
}

/// The records the daemon serves for the fixture's two asks.
fn served() -> Vec<Value> {
    let text = std::fs::read_to_string(VOCABULARY).expect("the vocabulary reads");
    let words = Words::from_vocabulary(&text).expect("the vocabulary is JSON");
    let waiting = held(
        "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35",
        1_794_459_000_000,
        State::Waiting,
        2,
    );
    let ran = held(
        "9b1e4d7c2a5f48e3b6d0c9a8f7e6d5c4",
        1_794_458_400_000,
        State::Ran { exit: Some(0) },
        0,
    );
    let rows = vec![
        row::row(&waiting, HOME, &words),
        row::row(&ran, HOME, &words),
    ];
    let summary = row::summary(&[&waiting, &ran], &words);
    let mut kept = Kept::default();
    kept.take(rows, vec![summary]);
    kept.all().cloned().collect()
}

#[test]
fn the_fixture_is_what_the_daemon_serves() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixture.json")).expect("the fixture is JSON");
    let served = json!(served());
    assert!(
        fixture["records"] == served,
        "the fixture's records differ from what's served, which are:\n{}",
        serde_json::to_string_pretty(&served).unwrap_or_default()
    );
}
