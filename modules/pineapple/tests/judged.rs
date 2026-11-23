//! How far the Slack reader has read: which messages are new, how many
//! judged it remembers, and what it keeps across a restart.

use std::path::PathBuf;

use pineapple::judged::{Afresh, FILE, Judged, KEPT, read, read_at, write};
use pineapple::slack::Message;

/// A message in `channel` said at `ts`.
fn said(channel: &str, ts: &str) -> Message {
    Message {
        channel: channel.to_owned(),
        ts: ts.to_owned(),
        thread: None,
        link: String::new(),
        author: "U1".to_owned(),
        by_bot: false,
        edited: false,
        text: "pineapple override".to_owned(),
    }
}

/// A folder of its own for one case's state.
fn state(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "pineapple-judged-{case}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ))
}

#[test]
fn a_message_is_new_once_said_after_since_and_not_yet_judged() {
    let mut judged = Judged::starting(1_795_000_000);
    assert!(
        !judged.is_new(&said("C1", "1794999999.999999")),
        "one said before it began isn't"
    );
    assert!(
        !judged.is_new(&said("C1", "1795000000.000000")),
        "nor one said as it began"
    );
    let message = said("C1", "1795000000.000001");
    assert!(judged.is_new(&message), "one said after is");
    judged.judged(&message);
    assert!(!judged.is_new(&message), "until it's judged");
    assert!(
        judged.is_new(&said("C2", "1795000000.000001")),
        "and the same time in another conversation is another message"
    );
}

#[test]
fn only_the_newest_judged_are_remembered() {
    let mut judged = Judged::starting(1_795_000_000);
    let first = said("C1", "1795000001.000000");
    judged.judged(&first);
    for n in 0..KEPT {
        judged.judged(&said("C2", &format!("1795000002.{n:06}")));
    }
    assert_eq!(judged.keys.len(), KEPT, "it keeps at most {KEPT}");
    assert!(judged.is_new(&first), "the oldest is forgotten first");
    assert_eq!(
        judged.keys.last(),
        Some(&said("C2", &format!("1795000002.{:06}", KEPT - 1)).key()),
        "and the newest is last"
    );
}

/// When a restart would look back from, in seconds.
const FRESH: u64 = 1_795_000_600;

#[test]
fn what_was_judged_is_kept_whole_across_a_restart() {
    let state = state("kept");
    assert_eq!(
        read(&state, FRESH),
        (Judged::starting(FRESH), Some(Afresh::Nothing)),
        "nothing kept starts afresh"
    );
    assert_eq!(read_at(&state), None, "and was never read");
    let mut judged = Judged::starting(1_795_000_000);
    judged.judged(&said("C1", "1795000001.000000"));
    judged.reached("pineapple", Some(&said("C1", "1795000001.000000")));
    write(&state, &judged, Some(1_795_000_002_000)).expect("kept");
    assert_eq!(
        read(&state, FRESH),
        (judged.clone(), None),
        "what's read back is what was kept"
    );
    assert_eq!(
        read_at(&state),
        Some(1_795_000_002_000),
        "with when Slack was read"
    );
    judged.judged(&said("C1", "1795000003.000000"));
    write(&state, &judged, read_at(&state)).expect("kept again");
    assert_eq!(
        (read(&state, FRESH), read_at(&state)),
        ((judged, None), Some(1_795_000_002_000)),
        "a round that judged without reading Slack whole keeps when it last was"
    );
    assert!(
        !state.join(FILE).with_extension("new").exists(),
        "and nothing is left beside it"
    );
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn what_cant_be_read_starts_afresh_and_says_why() {
    let state = state("afresh");
    std::fs::create_dir_all(&state).expect("the case's state");
    let spoiled: [&[u8]; 2] = [b"{not json", b"{\"since\": \"\xff\"}"];
    for spoiled in spoiled {
        std::fs::write(state.join(FILE), spoiled).expect("spoiled");
        assert_eq!(
            read(&state, FRESH),
            (Judged::starting(FRESH), Some(Afresh::Unreadable)),
            "{:?} can't be read",
            String::from_utf8_lossy(spoiled)
        );
    }
    std::fs::remove_file(state.join(FILE)).expect("the spoiled file goes");
    std::fs::create_dir(state.join(FILE)).expect("a folder in its place");
    assert_eq!(
        read(&state, FRESH),
        (Judged::starting(FRESH), Some(Afresh::Unreadable)),
        "one that's there but can't be opened can't be read, and is no first start"
    );
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn an_answer_cut_short_reaches_back_to_the_last_read_or_since_whichever_is_later() {
    let mut judged = Judged::starting(1_795_000_000);
    assert_eq!(
        judged.back_to("pineapple"),
        "1795000000.000000",
        "before any answer to it was read, nothing earlier than since is judged"
    );
    judged.reached("pineapple", Some(&said("C1", "1795000009.000100")));
    assert_eq!(
        (judged.back_to("pineapple"), judged.back_to("<@U1>")),
        ("1795000009.000100", "1795000000.000000"),
        "once read, to its newest message, and each search to its own"
    );
    judged.reached("pineapple", None);
    assert_eq!(
        judged.back_to("pineapple"),
        "1795000009.000100",
        "an answer giving nothing leaves it where it was"
    );
    judged.reached("pineapple", Some(&said("C1", "1794999999.000000")));
    assert_eq!(
        judged.back_to("pineapple"),
        "1795000000.000000",
        "one earlier than since reaches only since, as nothing before it is judged"
    );
}

#[test]
fn how_far_each_search_reached_that_cant_be_read_is_let_go() {
    let state = state("newest");
    std::fs::create_dir_all(&state).expect("the case's state");
    let kept = r#"{"since": "1795000000.000000", "keys": [],
        "newest": {"pineapple": "1795000009.000100", "<@U1>": "soon", "x": 3}}"#;
    std::fs::write(state.join(FILE), kept).expect("kept");
    let (judged, afresh) = read(&state, FRESH);
    assert_eq!(afresh, None, "how far it read is whole");
    assert_eq!(
        judged.newest.into_iter().collect::<Vec<_>>(),
        [("pineapple".to_owned(), "1795000009.000100".to_owned())],
        "only a search's newest that's a timestamp is kept"
    );
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn a_since_that_cant_be_read_starts_afresh_keeping_its_keys() {
    let state = state("since");
    std::fs::create_dir_all(&state).expect("the case's state");
    let key = said("C1", "1795000001.000000").key();
    for since in [None, Some("1795000000"), Some("\"not a time\"")] {
        let kept = match since {
            Some(since) => format!(r#"{{"since": {since}, "keys": ["{key}"]}}"#),
            None => format!(r#"{{"keys": ["{key}"]}}"#),
        };
        std::fs::write(state.join(FILE), &kept).expect("kept");
        assert_eq!(
            read(&state, FRESH),
            (
                Judged {
                    keys: vec![key.clone()],
                    ..Judged::starting(FRESH)
                },
                Some(Afresh::Unreadable)
            ),
            "{kept} reads from ten minutes back, with what it judged"
        );
    }
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn keys_that_cant_all_be_read_keep_since_and_say_so() {
    let state = state("keys");
    std::fs::create_dir_all(&state).expect("the case's state");
    let key = said("C1", "1795000001.000000").key();
    for (keys, left) in [
        (None, vec![]),
        (Some("\"spoiled\"".to_owned()), vec![]),
        (Some(format!(r#"[1, "{key}"]"#)), vec![key.clone()]),
    ] {
        let kept = match &keys {
            Some(keys) => format!(r#"{{"since": "1795000000.000000", "keys": {keys}}}"#),
            None => r#"{"since": "1795000000.000000"}"#.to_owned(),
        };
        std::fs::write(state.join(FILE), &kept).expect("kept");
        let (judged, afresh) = read(&state, FRESH);
        assert_eq!(
            (judged.since.as_str(), afresh),
            ("1795000000.000000", Some(Afresh::Unjudged)),
            "{kept} keeps how far it read, and says what it judged is partly lost"
        );
        assert_eq!(
            judged.keys, left,
            "{kept} keeps the keys that read, and only those"
        );
    }
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn a_since_later_than_ten_minutes_ago_looks_back_keeping_its_keys() {
    // A clock set back leaves a since past ten minutes ago, which would
    // pass over everything said until the clock caught up.
    let keys = vec![said("C1", "1795000001.000000").key()];
    let mut ahead = Judged {
        since: "1795000600.000001".to_owned(),
        keys: keys.clone(),
        ..Judged::starting(0)
    };
    assert!(ahead.look_back_to(FRESH), "one later looks back");
    assert_eq!(
        ahead,
        Judged {
            keys,
            ..Judged::starting(FRESH)
        },
        "to ten minutes ago, with what it judged"
    );
    assert!(!ahead.look_back_to(FRESH), "and once there, stays");
    let mut behind = Judged::starting(FRESH - 1);
    assert!(!behind.look_back_to(FRESH), "one earlier doesn't move");
    assert_eq!(behind, Judged::starting(FRESH - 1), "and is as it was");
}

#[test]
fn a_state_that_cant_be_written_says_so() {
    let state = state("unwritable");
    std::fs::write(&state, "a file, not a folder").expect("in the way");
    assert!(
        write(&state, &Judged::starting(FRESH), Some(1)).is_err(),
        "a folder that can't be made is an error, not a silence"
    );
    std::fs::remove_file(&state).expect("the case's file goes");
}
