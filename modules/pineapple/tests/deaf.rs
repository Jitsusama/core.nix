//! When the safe word's Slack reader tells Joel it can't hear Slack, and
//! what it asks of him.

use pineapple::deaf::{Change, Deaf, Deafness, ROUNDS, Round, make, refuses_the_sign_in, waits};

const AT: u64 = 1_795_000_000_000;

#[test]
fn joel_is_told_once_two_rounds_running_hear_nothing() {
    let mut deaf = Deaf::new();
    let refused = Round::Deaf(Deafness::Refused);
    assert_eq!(deaf.heard(refused, AT), Change::Keep, "one round is a blip");
    assert_eq!(ROUNDS, 2, "the count this case is written for");
    assert_eq!(
        deaf.heard(refused, AT + 30_000),
        Change::Tell(waits(Deafness::Refused, AT + 30_000, AT + 30_000)),
        "two in a row tell him"
    );
    assert_eq!(
        deaf.heard(refused, AT + 60_000),
        Change::Keep,
        "and once is enough"
    );
    let changed = deaf.heard(Round::Deaf(Deafness::NoSignIn), AT + 90_000);
    assert_eq!(
        changed,
        Change::Tell(waits(Deafness::NoSignIn, AT + 30_000, AT + 90_000)),
        "a new why is said, in the same row"
    );
    assert_eq!(
        deaf.heard(Round::Heard, AT + 120_000),
        Change::Untell,
        "hearing again takes it away"
    );
    assert_eq!(deaf.heard(Round::Heard, AT + 150_000), Change::Keep, "once");
}

#[test]
fn each_time_slack_goes_unheard_is_a_row_of_its_own() {
    let mut deaf = Deaf::new();
    let refused = Round::Deaf(Deafness::Refused);
    let told = |deaf: &mut Deaf, at: u64| {
        let _ = deaf.heard(refused, at);
        match deaf.heard(refused, at + 30_000) {
            Change::Tell(row) => row,
            other => panic!("not told at {at}: {other:?}"),
        }
    };
    let first = told(&mut deaf, AT);
    let _ = deaf.heard(Round::Heard, AT + 60_000);
    let second = told(&mut deaf, AT + 90_000);
    assert_ne!(
        first["items"][0]["id"], second["items"][0]["id"],
        "For You says a name once and keeps one marked done gone, so a later time needs its own"
    );
    assert_eq!(
        second["items"][0]["raisedAt"],
        slack_people::messages::iso(AT + 120_000),
        "and says when it began, so only a sign-in after it makes it done"
    );
}

#[test]
fn a_change_that_couldnt_be_made_is_made_again() {
    let mut deaf = Deaf::new();
    let refused = Round::Deaf(Deafness::Refused);
    let _ = deaf.heard(Round::Heard, AT);
    let _ = deaf.heard(refused, AT);
    let tell = deaf.heard(refused, AT + 30_000);
    assert!(matches!(tell, Change::Tell(_)), "told: {tell:?}");
    deaf.failed(&tell);
    assert_eq!(
        deaf.heard(refused, AT + 60_000),
        Change::Tell(waits(Deafness::Refused, AT + 30_000, AT + 60_000)),
        "a row not written is told again, in the same row"
    );
    let untell = deaf.heard(Round::Heard, AT + 90_000);
    assert_eq!(untell, Change::Untell);
    deaf.failed(&untell);
    assert_eq!(
        deaf.heard(Round::Heard, AT + 120_000),
        Change::Untell,
        "and one not taken away is taken away again"
    );
}

#[test]
fn a_round_slack_couldnt_be_asked_in_counts_neither_way() {
    let mut deaf = Deaf::new();
    let refused = Round::Deaf(Deafness::Refused);
    let _ = deaf.heard(Round::Heard, AT);
    assert_eq!(deaf.heard(refused, AT), Change::Keep);
    assert_eq!(
        deaf.heard(Round::Unasked, AT),
        Change::Keep,
        "the network's down: nothing of Joel's"
    );
    assert!(
        matches!(deaf.heard(refused, AT), Change::Tell(_)),
        "nor does it end the count"
    );
    assert_eq!(
        deaf.heard(Round::Unasked, AT),
        Change::Keep,
        "nor take the row away"
    );
}

#[test]
fn what_an_earlier_reader_told_goes_once_slack_is_heard() {
    let mut deaf = Deaf::new();
    assert_eq!(
        deaf.heard(Round::Heard, AT),
        Change::Untell,
        "a row left before a restart isn't left standing"
    );
    for before in [Round::Deaf(Deafness::Refused), Round::Unasked] {
        let mut deaf = Deaf::new();
        let _ = deaf.heard(before, AT);
        assert_eq!(
            deaf.heard(Round::Heard, AT + 30_000),
            Change::Untell,
            "nor after a round of {before:?} first"
        );
    }
}

#[test]
fn only_a_refusal_of_the_sign_in_is_joels_to_fix() {
    for word in [
        "invalid_auth",
        "not_authed",
        "token_revoked",
        "token_expired",
        "account_inactive",
    ] {
        assert!(refuses_the_sign_in(word), "{word}");
    }
    for word in ["ratelimited", "fatal_error", "not ok"] {
        assert!(
            !refuses_the_sign_in(word),
            "{word} isn't fixed by signing in"
        );
    }
}

#[test]
fn the_row_says_what_isnt_heard_and_how_to_fix_it() {
    let refused = waits(Deafness::Refused, AT, AT + 30_000);
    let item = &refused["items"][0];
    assert_eq!(refused["quest"], "pineapple");
    assert_eq!(item["id"], format!("pineapple-slack-deaf-{AT}"));
    assert_eq!(item["title"], "The Safe Word Can't Hear Slack");
    let why = item["why"].as_str().unwrap_or_default();
    assert!(
        why.starts_with("Slack refused the safe word's sign-in")
            && why.contains("a stop said in Slack isn't heard")
            && why.ends_with("one said elsewhere still is"),
        "why: {why}"
    );
    assert_eq!(item["raisedAt"], slack_people::messages::iso(AT));
    assert_eq!(refused["updated"], slack_people::messages::iso(AT + 30_000));
    assert_eq!(item["ready"], true, "he can do it now");
    assert_eq!(item["doneWhen"]["signal"], "signin:slack");
    assert_eq!(item["steps"][0], "Press Sign In on Slack in Programs");
    let locked = waits(Deafness::NoSignIn, AT, AT);
    assert_eq!(
        locked["items"][0]["steps"][0], "Unlock your keyring if it asks",
        "no sign-in may be a locked keyring"
    );
}

#[test]
fn the_row_is_written_whole_and_taken_away() {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-deaf-file-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("waiting/pineapple.json");
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("the folder is made");
    let told = waits(Deafness::Refused, AT, AT);
    make(&file, &Change::Tell(told.clone())).expect("written");
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("read")).expect("JSON");
    assert_eq!(read, told, "what's written is what's told");
    let aside = folder.join("waiting/.writing");
    assert!(
        std::fs::read_dir(&aside).is_ok_and(|mut left| left.next().is_none()),
        "written aside and renamed in, so nothing is left aside"
    );
    make(&file, &Change::Untell).expect("taken away");
    assert!(!file.exists(), "gone");
    make(&file, &Change::Untell).expect("taking none away is fine");
    let _ = std::fs::remove_dir_all(&folder);
}
