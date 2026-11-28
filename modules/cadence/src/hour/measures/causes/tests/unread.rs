//! A mesh file the hour couldn't read: one the harness moved is let go
//! quietly, and one unreadable holds the mark back, so the next hour
//! reads it again.

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use super::super::heard::{Read, message_in, opened};
use super::*;

/// The mark kept in `folder`, in ms.
fn mark_in(folder: &Path) -> i64 {
    let json = fs::read(folder.join("causes.json")).expect("a reading kept");
    serde_json::from_slice::<Kept>(&json)
        .expect("a reading that parses")
        .through_ms
}

#[test]
fn a_file_moved_away_is_let_go_and_holds_nothing_back() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let written = (NOW - 7200) * 1000;
    let name = format!("{written}-SENDER-1-0000000a.json");
    let gone = root.path().join(&name);
    assert!(
        matches!(message_in(&gone, &name), Read::LeftOut),
        "a file the harness moved wasn't let go"
    );
    let (heard, through_ms) = opened(
        vec![(gone, name, written)],
        NOW * 1000,
        (NOW - WITHIN) * 1000,
    );
    assert!(
        heard.is_empty() && through_ms == NOW * 1000,
        "a file the harness moved was heard, or held the mark back to {through_ms}"
    );
}

/// Each unreadable file holds the mark to just before the earliest of
/// them, but never past the week's start, before which none counts.
#[test]
fn a_file_unreadable_holds_the_mark_to_just_before_it_within_the_week() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let shut = |ago: i64| {
        let mut m = sent("cause: x | change: core.nix#1", ago, "@1JQPPC");
        m.id = format!("{}-SENDER-1-{ago:08x}", (NOW - ago) * 1000);
        write(root.path(), &m, ago);
        let name = format!("{}.json", m.id);
        let path = root.path().join(&name);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("shut");
        (path, name, (NOW - ago) * 1000)
    };
    let since_ms = (NOW - WITHIN) * 1000;
    let (heard, through_ms) = opened(vec![shut(3600), shut(7200)], NOW * 1000, since_ms);
    assert!(
        heard.is_empty() && through_ms == (NOW - 7200) * 1000 - 1,
        "two files unreadable didn't hold the mark to just before the earlier: {through_ms}"
    );
    let (_, through_ms) = opened(vec![shut(WITHIN + 60)], NOW * 1000, since_ms);
    assert_eq!(
        through_ms, since_ms,
        "a file unreadable from before the week held the mark back past its start"
    );
}

#[test]
fn a_file_unreadable_is_held_and_read_the_next_hour() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    // Two hours before now, past the hour of slack the next reading takes
    // anyway, so only holding the mark back reads it again.
    let ago = 2 * 3600;
    let mut last = PathBuf::new();
    for n in 1..=3 {
        let line = format!("cause: flaky gate | change: core.nix#{n}");
        let mut m = sent(&line, ago, "@1JQPPC");
        m.id = format!("{}-SENDER-1-0000000{n}", (NOW - ago) * 1000);
        write(&messages, &m, ago);
        last = messages.join(format!("{}.json", m.id));
    }
    fs::set_permissions(&last, fs::Permissions::from_mode(0o000)).expect("shut");
    let first = read(home.path(), folder.path(), NOW, true);
    assert_eq!(
        first.now, 0.0,
        "a file that couldn't be read was counted: {:?}",
        first.behind
    );
    assert_eq!(
        mark_in(folder.path()),
        (NOW - ago) * 1000 - 1,
        "the mark wasn't held back to just before the file that couldn't be read"
    );
    fs::set_permissions(&last, fs::Permissions::from_mode(0o644)).expect("opened");
    let next = read(home.path(), folder.path(), NOW + 3600, true);
    assert_eq!(
        next.now, 1.0,
        "a file unreadable the hour before wasn't read again once it could be: {:?}",
        next.behind
    );
    assert_eq!(
        mark_in(folder.path()),
        (NOW + 3600) * 1000,
        "the mark stayed held back once every file was read"
    );
}
