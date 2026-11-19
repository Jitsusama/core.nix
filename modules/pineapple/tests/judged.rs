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
    write(&state, &judged, 1_795_000_002_000).expect("kept");
    assert_eq!(
        read(&state, FRESH),
        (judged, None),
        "what's read back is what was kept"
    );
    assert_eq!(
        read_at(&state),
        Some(1_795_000_002_000),
        "with when Slack was read"
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
    for spoiled in ["{not json", r#"{"keys": []}"#, r#"{"since": 1795000000}"#] {
        std::fs::write(state.join(FILE), spoiled).expect("spoiled");
        assert_eq!(
            read(&state, FRESH),
            (Judged::starting(FRESH), Some(Afresh::Unreadable)),
            "{spoiled} can't be read"
        );
    }
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn a_since_that_would_miss_what_was_said_starts_afresh_keeping_its_keys() {
    // A clock set back since the file was kept leaves a since past ten
    // minutes ago, which would pass over everything said till then.
    let state = state("ahead");
    for since in ["1795000601.000000", "not a time"] {
        let kept = Judged {
            since: since.to_owned(),
            keys: vec![said("C1", "1795000001.000000").key()],
        };
        write(&state, &kept, 1_795_000_002_000).expect("kept");
        assert_eq!(
            read(&state, FRESH),
            (
                Judged {
                    keys: kept.keys,
                    ..Judged::starting(FRESH)
                },
                Some(Afresh::Ahead)
            ),
            "{since:?} reads from ten minutes back, with what it judged"
        );
    }
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}

#[test]
fn a_state_that_cant_be_written_says_so() {
    let state = state("unwritable");
    std::fs::write(&state, "a file, not a folder").expect("in the way");
    assert!(
        write(&state, &Judged::starting(FRESH), 1).is_err(),
        "a folder that can't be made is an error, not a silence"
    );
    std::fs::remove_file(&state).expect("the case's file goes");
}
