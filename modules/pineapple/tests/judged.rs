//! How far the Slack reader has read: which messages are new, how many
//! judged it remembers, and what it keeps across a restart.

use std::path::PathBuf;

use pineapple::judged::{FILE, Judged, KEPT, read, read_at, write};
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

#[test]
fn what_was_judged_is_kept_whole_across_a_restart() {
    let state = state("kept");
    assert_eq!(read(&state), None, "nothing kept reads as none");
    assert_eq!(read_at(&state), None, "and as never read");
    let mut judged = Judged::starting(1_795_000_000);
    judged.judged(&said("C1", "1795000001.000000"));
    write(&state, &judged, 1_795_000_002_000).expect("kept");
    assert_eq!(
        read(&state),
        Some(judged),
        "what's read back is what was kept"
    );
    assert_eq!(
        read_at(&state),
        Some(1_795_000_002_000),
        "with when Slack was read"
    );
    assert!(
        !state.join(FILE).with_extension("new").exists(),
        "written aside and renamed in, so nothing is left aside"
    );
    std::fs::write(state.join(FILE), "{not json").expect("spoiled");
    assert_eq!(read(&state), None, "what can't be read reads as none");
    std::fs::remove_dir_all(&state).expect("the case's state goes");
}
