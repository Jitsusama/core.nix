//! When the safe word's Slack reader tells Joel it can't hear Slack, and
//! what it asks of him.

use std::ffi::OsStr;
use std::path::Path;

use pineapple::deaf::{
    Change, Deaf, Deafness, ROUNDS, Round, Search, Waiting, make, refuses_the_sign_in, waiting,
    waits,
};

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
fn a_row_left_by_a_tell_that_failed_after_it_is_still_taken_away() {
    let mut deaf = Deaf::new();
    let _ = deaf.heard(Round::Heard, AT);
    let refused = Round::Deaf(Deafness::Refused);
    let _ = deaf.heard(refused, AT);
    let told = deaf.heard(refused, AT + 30_000);
    assert!(matches!(told, Change::Tell(_)), "told: {told:?}");
    // Written; then a new why, whose rewrite fails.
    let rewrite = deaf.heard(Round::Deaf(Deafness::NoSignIn), AT + 60_000);
    assert!(matches!(rewrite, Change::Tell(_)), "retold: {rewrite:?}");
    deaf.failed(&rewrite);
    assert_eq!(
        deaf.heard(Round::Heard, AT + 90_000),
        Change::Untell,
        "the first row still stands, so hearing Slack takes it away"
    );
}

#[test]
fn a_round_slack_couldnt_be_asked_in_doesnt_count_toward_telling() {
    let mut deaf = Deaf::new();
    let _ = deaf.heard(Round::Heard, AT);
    assert_eq!(deaf.heard(Round::Unasked, AT), Change::Keep);
    assert_eq!(
        deaf.heard(Round::Deaf(Deafness::Refused), AT),
        Change::Keep,
        "one refusal beside a blip is still only one"
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
    let unread = &waits(Deafness::Unreadable, AT, AT)["items"][0];
    assert!(
        unread["why"]
            .as_str()
            .is_some_and(|why| why.starts_with("Slack's answers to the safe word can't be read")),
        "an answer that can't be read says so: {unread}"
    );
    assert_eq!(
        (&unread["doneWhen"], &unread["steps"][0]),
        (
            &serde_json::json!({"by": "person"}),
            &serde_json::json!(
                "Ask an agent to read the reader's journal, which says what it couldn't read"
            )
        ),
        "no sign-in mends it, so only Joel's word marks it done"
    );
    for whole in [Deafness::Refused, Deafness::NoSignIn, Deafness::Unreadable] {
        let item = &waits(whole, AT, AT)["items"][0];
        let why = item["why"].as_str().unwrap_or_default();
        assert!(
            why.ends_with("so a stop said in Slack isn't heard; one said elsewhere still is"),
            "{whole:?} says what is still heard: {why}"
        );
        assert_eq!(item["title"], "The Safe Word Can't Hear Slack");
    }
    let half = &waits(Deafness::Unread(Search::Tag), AT, AT)["items"][0];
    assert_eq!(
        half["title"], "The Safe Word Can't Hear Part of Slack",
        "one search deaf isn't said as all of Slack"
    );
}

#[test]
fn answers_that_cant_be_read_two_rounds_running_are_told() {
    let mut deaf = Deaf::new();
    let unread = Round::Deaf(Deafness::Unreadable);
    assert_eq!(deaf.heard(unread, AT), Change::Keep, "one may be a blip");
    assert_eq!(
        deaf.heard(unread, AT + 30_000),
        Change::Tell(waits(Deafness::Unreadable, AT + 30_000, AT + 30_000)),
        "two in a row are no quiet Slack"
    );
}

#[test]
fn one_search_that_cant_be_read_is_told_by_name_and_as_it_changes() {
    for (search, named, still) in [
        (
            Search::Phrase,
            "for pineapple",
            "one that tags you still is",
        ),
        (Search::Tag, "for your tag", "one saying it still is"),
    ] {
        let item = &waits(Deafness::Unread(search), AT, AT)["items"][0];
        let why = item["why"].as_str().unwrap_or_default();
        assert!(
            why.contains(&format!("search of Slack {named} can't be read")) && why.ends_with(still),
            "it says which search is deaf, and what's still heard: {why}"
        );
        assert_eq!(
            item["raisedAt"],
            slack_people::messages::iso(AT),
            "and since when"
        );
        assert_eq!(item["doneWhen"], serde_json::json!({"by": "person"}));
    }
    let mut deaf = Deaf::new();
    let tag = Round::Deaf(Deafness::Unread(Search::Tag));
    let both = Round::Deaf(Deafness::Unreadable);
    assert_eq!(deaf.heard(tag, AT), Change::Keep, "one round is a blip");
    assert_eq!(
        deaf.heard(tag, AT + 30_000),
        Change::Tell(waits(
            Deafness::Unread(Search::Tag),
            AT + 30_000,
            AT + 30_000
        )),
        "two in a row are told"
    );
    assert_eq!(deaf.heard(tag, AT + 60_000), Change::Keep, "told once");
    assert_eq!(
        deaf.heard(both, AT + 90_000),
        Change::Tell(waits(Deafness::Unreadable, AT + 90_000, AT + 90_000)),
        "the other going deaf too is a row of its own, so one marked done can't hide it"
    );
    assert_eq!(
        deaf.heard(tag, AT + 120_000),
        Change::Tell(waits(
            Deafness::Unread(Search::Tag),
            AT + 90_000,
            AT + 120_000
        )),
        "one search coming back narrows the row, as the same time of not hearing"
    );
    let phrase = Round::Deaf(Deafness::Unread(Search::Phrase));
    assert_eq!(
        deaf.heard(phrase, AT + 150_000),
        Change::Tell(waits(
            Deafness::Unread(Search::Phrase),
            AT + 150_000,
            AT + 150_000
        )),
        "the other search going deaf in its place is a row of its own"
    );
    assert_eq!(deaf.heard(Round::Heard, AT + 180_000), Change::Untell);
}

#[test]
fn a_row_after_one_search_deaf_is_never_the_one_marked_done() {
    for after in [Deafness::Refused, Deafness::NoSignIn, Deafness::Unreadable] {
        let mut deaf = Deaf::new();
        let tag = Round::Deaf(Deafness::Unread(Search::Tag));
        deaf.heard(tag, AT);
        let Change::Tell(half) = deaf.heard(tag, AT + 30_000) else {
            panic!("two half rounds are told");
        };
        let Change::Tell(whole) = deaf.heard(Round::Deaf(after), AT + 60_000) else {
            panic!("{after:?} after a half round is told");
        };
        assert_ne!(
            half["items"][0]["id"], whole["items"][0]["id"],
            "{after:?} is named apart from the half row Joel may have marked done"
        );
    }
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

#[test]
fn the_row_goes_where_a_whole_name_says_and_a_part_name_is_said() {
    let home = Path::new("/home/joel/.local/state");
    let own = home.join("mission/waiting/pineapple.json");
    assert_eq!(
        waiting(Some(OsStr::new("/run/named/pineapple.json")), home),
        Waiting {
            file: "/run/named/pineapple.json".into(),
            refused: None,
        },
        "a whole path is where the row goes"
    );
    assert_eq!(
        waiting(None, home),
        Waiting {
            file: own.clone(),
            refused: None,
        },
        "none named is For You's own, and nothing to say"
    );
    for part in ["", "named/pineapple.json"] {
        assert_eq!(
            waiting(Some(OsStr::new(part)), home),
            Waiting {
                file: own.clone(),
                refused: Some(part.into()),
            },
            "{part:?} names nothing, and is kept to be said"
        );
    }
}
