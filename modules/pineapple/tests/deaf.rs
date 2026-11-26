//! When the safe word's Slack reader tells Joel it can't hear Slack, and
//! what it asks of him.

use std::ffi::OsStr;
use std::path::Path;
use std::slice;

use pineapple::deaf::{
    Change, Deaf, Deafness, MEET, ROUNDS, Round, Search, Waiting, keep_missed, make, missed,
    refuses_the_sign_in, waiting, waits,
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
    let no_sign_in = Round::Deaf(Deafness::NoSignIn);
    assert_eq!(
        deaf.heard(no_sign_in, AT + 90_000),
        Change::Keep,
        "a new why for one round may be a blip"
    );
    assert_eq!(
        deaf.heard(no_sign_in, AT + 120_000),
        Change::Tell(waits(Deafness::NoSignIn, AT + 120_000, AT + 120_000)),
        "two of it are said as a row of its own, as a sign-in marks the old one done"
    );
    assert_eq!(
        deaf.heard(Round::Heard, AT + 150_000),
        Change::Untell,
        "hearing again takes it away"
    );
    assert_eq!(deaf.heard(Round::Heard, AT + 180_000), Change::Keep, "once");
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
    let _ = deaf.heard(Round::Deaf(Deafness::NoSignIn), AT + 60_000);
    let rewrite = deaf.heard(Round::Deaf(Deafness::NoSignIn), AT + 90_000);
    assert!(matches!(rewrite, Change::Tell(_)), "retold: {rewrite:?}");
    deaf.failed(&rewrite);
    assert_eq!(
        deaf.heard(Round::Heard, AT + 120_000),
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
        Change::Keep,
        "a new why held once"
    );
    assert_eq!(
        deaf.heard(both, AT + 120_000),
        Change::Tell(waits(Deafness::Unreadable, AT + 120_000, AT + 120_000)),
        "the other going deaf too is a row of its own, so one marked done can't hide it"
    );
    let _ = deaf.heard(tag, AT + 150_000);
    assert_eq!(
        deaf.heard(tag, AT + 180_000),
        Change::Tell(waits(
            Deafness::Unread(Search::Tag),
            AT + 180_000,
            AT + 180_000
        )),
        "one search coming back narrows it to a row of its own, so a whole row marked done \
         can't hide it"
    );
    let phrase = Round::Deaf(Deafness::Unread(Search::Phrase));
    let _ = deaf.heard(phrase, AT + 210_000);
    assert_eq!(
        deaf.heard(phrase, AT + 240_000),
        Change::Tell(waits(
            Deafness::Unread(Search::Phrase),
            AT + 240_000,
            AT + 240_000
        )),
        "the other search going deaf in its place is a row of its own"
    );
    assert_eq!(deaf.heard(Round::Heard, AT + 270_000), Change::Untell);
}

#[test]
fn a_why_that_flips_each_round_is_no_row_of_its_own() {
    let mut deaf = Deaf::new();
    let tag = Round::Deaf(Deafness::Unread(Search::Tag));
    let both = Round::Deaf(Deafness::Unreadable);
    let _ = deaf.heard(tag, AT);
    assert!(
        matches!(deaf.heard(tag, AT + 30_000), Change::Tell(_)),
        "two of one why are told"
    );
    for round in 2..12 {
        let why = if round % 2 == 0 { both } else { tag };
        assert_eq!(
            deaf.heard(why, AT + round * 30_000),
            Change::Keep,
            "a why held for a round at a time leaves the row as it was, so each flip \
             isn't a notice: round {round}"
        );
    }
}

#[test]
fn a_search_deaf_every_round_is_told_while_its_partner_flickers() {
    let mut deaf = Deaf::new();
    let tag = Round::Deaf(Deafness::Unread(Search::Tag));
    let _ = deaf.heard(tag, AT);
    assert!(
        matches!(deaf.heard(tag, AT + 30_000), Change::Tell(_)),
        "two rounds with the tag's search deaf tell Joel"
    );
    assert!(
        deaf.tells(Search::Tag) && !deaf.tells(Search::Phrase),
        "the row says the tag's search can't be read, and only that"
    );
    // The phrase goes deaf for good, while the tag flickers back and forth.
    let phrase = Round::Deaf(Deafness::Unread(Search::Phrase));
    let both = Round::Deaf(Deafness::Unreadable);
    assert_eq!(deaf.heard(phrase, AT + 60_000), Change::Keep, "held once");
    assert_eq!(
        deaf.heard(both, AT + 90_000),
        Change::Tell(waits(Deafness::Unreadable, AT + 90_000, AT + 90_000)),
        "the phrase deaf two rounds running is told, though no one why held twice"
    );
    assert!(deaf.tells(Search::Phrase), "and the row now says so");
}

#[test]
fn a_row_that_failed_to_write_tells_nothing() {
    let mut deaf = Deaf::new();
    let tag = Round::Deaf(Deafness::Unread(Search::Tag));
    let _ = deaf.heard(tag, AT);
    let told = deaf.heard(tag, AT + 30_000);
    assert!(
        matches!(told, Change::Tell(_)) && deaf.tells(Search::Tag),
        "deaf two rounds, a row is to be written"
    );
    deaf.failed(&told);
    assert!(
        !deaf.tells(Search::Tag),
        "a row that couldn't be written doesn't count as Joel told"
    );
}

#[test]
fn a_row_for_another_why_is_never_the_one_marked_done() {
    let every = [
        Deafness::Refused,
        Deafness::NoSignIn,
        Deafness::Unreadable,
        Deafness::Unread(Search::Phrase),
        Deafness::Unread(Search::Tag),
    ];
    for before in every {
        for after in every.into_iter().filter(|after| *after != before) {
            let mut deaf = Deaf::new();
            deaf.heard(Round::Deaf(before), AT);
            let Change::Tell(first) = deaf.heard(Round::Deaf(before), AT + 30_000) else {
                panic!("two rounds of {before:?} are told");
            };
            deaf.heard(Round::Deaf(after), AT + 60_000);
            let Change::Tell(then) = deaf.heard(Round::Deaf(after), AT + 90_000) else {
                panic!("{after:?} after {before:?} is told");
            };
            assert_ne!(
                first["items"][0]["id"], then["items"][0]["id"],
                "{after:?} is named apart from the {before:?} row Joel may have marked done"
            );
        }
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

const MINUTE: u64 = 60_000;

#[test]
fn what_was_never_read_is_said_as_minutes_before_the_row() {
    let tag = missed(
        Search::Tag,
        Some(AT - 41 * MINUTE + 1),
        Some(AT - 13 * MINUTE),
        AT,
    );
    assert_eq!(
        tag["why"],
        "The safe word never read what tagged you in Slack without saying pineapple from 41 to \
         13 minutes before this was raised, as more was said than it could read, so a stop said \
         then wasn't heard",
        "a span begun part way into a minute is said from the whole one"
    );
    assert_eq!(
        tag["id"],
        format!(
            "pineapple-slack-missed-tag-{}-{}",
            AT - 41 * MINUTE + 1,
            AT - 13 * MINUTE
        ),
        "named for its span, so written again it's the same row"
    );
    assert_eq!(
        tag["doneWhen"],
        serde_json::json!({"by": "person"}),
        "no round reads it again, so only Joel says he has looked"
    );
    let phrase = missed(Search::Phrase, Some(AT - MINUTE), Some(AT - 10), AT);
    assert_eq!(
        phrase["why"],
        "The safe word never read what said pineapple in Slack without tagging you in the 1 \
         minute before this was raised, as more was said than it could read, so a stop said \
         then wasn't heard",
        "one ending this minute is said up to the row"
    );
    assert_eq!(
        phrase["id"],
        format!("pineapple-slack-missed-phrase-{}-{}", AT - MINUTE, AT - 10),
        "each search's rows named apart"
    );
    assert!(
        missed(Search::Tag, None, Some(AT - MINUTE), AT)["why"]
            .as_str()
            .is_some_and(|why| why.contains(
                "before this was raised, since when it last read can't be told, as more"
            )),
        "and one with no last read it can tell is said with no minutes"
    );
}

#[test]
fn each_span_never_read_is_kept_a_week_beside_the_others() {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-file-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(&file, "not JSON").expect("a file it can't read");
    let week = 7 * 24 * 60 * MINUTE;
    let row = |search, at| missed(search, Some(at - MINUTE), Some(at), at);
    assert!(
        keep_missed(&file, row(Search::Tag, AT), AT).is_err()
            && std::fs::read_to_string(&file).is_ok_and(|kept| kept == "not JSON"),
        "a file it can't read may hold rows, so it isn't written over"
    );
    std::fs::write(&file, r#"{"v":2}"#).expect("a file holding no rows it can read");
    assert!(
        keep_missed(&file, row(Search::Tag, AT), AT).is_err()
            && std::fs::read_to_string(&file).is_ok_and(|kept| kept == r#"{"v":2}"#),
        "nor is one that reads but holds no rows, as it may be a newer shape"
    );
    std::fs::remove_file(&file).expect("taken away");
    let first = AT - week - 1;
    keep_missed(&file, row(Search::Tag, first), first).expect("written where none was");
    keep_missed(&file, row(Search::Phrase, AT - week), AT - week).expect("written");
    let ending_sooner = missed(Search::Tag, Some(AT - MINUTE), Some(AT - 5), AT - 5);
    keep_missed(&file, ending_sooner, AT - 5).expect("written");
    keep_missed(&file, row(Search::Tag, AT), AT).expect("written in its place");
    keep_missed(&file, row(Search::Tag, AT), AT).expect("written again");
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("read")).expect("JSON");
    let ids: Vec<_> = read["items"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["id"].clone())
        .collect();
    assert_eq!(
        ids,
        [
            format!(
                "pineapple-slack-missed-phrase-{}-{}",
                AT - week - MINUTE,
                AT - week
            ),
            format!("pineapple-slack-missed-tag-{}-{AT}", AT - MINUTE),
        ],
        "one whose span ended a week ago stays, one older goes, and the newest joins them \
         once, however often it's written, in place of one begun as it was"
    );
    assert_eq!(read["quest"], "pineapple", "in the shape For You reads");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The rows `file` holds, read back.
fn rows_in(file: &Path) -> Vec<serde_json::Value> {
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).expect("read")).expect("JSON");
    read["items"].as_array().expect("rows").clone()
}

#[test]
fn a_busy_stretch_read_on_round_after_round_is_said_in_one_row() {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-meet-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    assert_eq!(MEET, 10 * MINUTE, "the nearness this case is written for");
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let far = tag(100 * MINUTE, 90 * MINUTE);
    keep_missed(&file, far.clone(), AT - 90 * MINUTE).expect("written");
    let phrase = missed(
        Search::Phrase,
        Some(AT - 30 * MINUTE),
        Some(AT - 25 * MINUTE),
        AT,
    );
    keep_missed(&file, phrase.clone(), AT - 25 * MINUTE).expect("written");
    keep_missed(&file, tag(30 * MINUTE, 25 * MINUTE), AT - 25 * MINUTE).expect("written");
    keep_missed(&file, tag(26 * MINUTE, 20 * MINUTE), AT - 20 * MINUTE).expect("written");
    let overlapping = rows_in(&file);
    assert_eq!(
        (
            overlapping.len(),
            &overlapping[2]["id"],
            &overlapping[2]["whole"]
        ),
        (
            3,
            &serde_json::json!(format!(
                "pineapple-slack-missed-tag-{}-{}",
                AT - 30 * MINUTE,
                AT - 20 * MINUTE
            )),
            &serde_json::json!(true)
        ),
        "spans that overlap are one, said whole: {overlapping:?}"
    );
    keep_missed(&file, tag(15 * MINUTE, 12 * MINUTE), AT - 12 * MINUTE).expect("written");
    let rows = rows_in(&file);
    assert_eq!(
        rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [
            far["id"].clone(),
            phrase["id"].clone(),
            serde_json::json!(format!(
                "pineapple-slack-missed-tag-{}-{}",
                AT - 30 * MINUTE,
                AT - 12 * MINUTE
            )),
        ],
        "one five minutes past joins them, while one farther off and the other search's \
         stay their own"
    );
    assert_eq!(
        rows[2]["why"],
        "The safe word never read parts of what tagged you in Slack without saying pineapple \
         in the 18 minutes before this was raised, as more was said than it could read, so a \
         stop said then wasn't heard",
        "a read stretch lies between, so it's said as parts, raised as the newest was"
    );
    assert_eq!(rows[2]["whole"], false, "and kept so as written again");
    keep_missed(&file, tag(13 * MINUTE, 10 * MINUTE), AT - 10 * MINUTE).expect("written");
    let again = rows_in(&file);
    assert!(
        again.len() == 3 && again[2]["whole"] == false,
        "a row once said as parts stays so, though the next overlaps it: {again:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// What `kept` rows, then `row` kept at `AT`, leave in a file of their own.
fn kept_after(
    name: &str,
    kept: &[serde_json::Value],
    row: serde_json::Value,
) -> Vec<serde_json::Value> {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-{name}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    let earlier = serde_json::json!({"v": 2, "items": kept});
    std::fs::write(&file, earlier.to_string()).expect("the earlier rows are written");
    keep_missed(&file, row, AT).expect("written");
    let rows = rows_in(&file);
    let _ = std::fs::remove_dir_all(&folder);
    rows
}

#[test]
fn spans_meet_up_to_ten_minutes_apart_and_an_unknown_start_meets_only_itself() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let earlier = tag(30 * MINUTE, 20 * MINUTE);
    let met = kept_after(
        "edge",
        slice::from_ref(&earlier),
        tag(10 * MINUTE, 5 * MINUTE),
    );
    assert!(
        met.len() == 1 && met[0]["whole"] == false,
        "exactly ten minutes apart they meet, as parts: {met:?}"
    );
    let apart = kept_after(
        "past",
        slice::from_ref(&earlier),
        tag(10 * MINUTE - 1, 5 * MINUTE),
    );
    assert_eq!(
        apart.len(),
        2,
        "a millisecond farther they don't: {apart:?}"
    );
    let touching = kept_after(
        "touch",
        slice::from_ref(&earlier),
        tag(20 * MINUTE, 10 * MINUTE),
    );
    assert!(
        touching.len() == 1 && touching[0]["whole"] == true,
        "one beginning where the other ends is said whole: {touching:?}"
    );
    let mut legacy = earlier.clone();
    legacy.as_object_mut().expect("a row").remove("whole");
    let old = kept_after("legacy", &[legacy], tag(25 * MINUTE, 10 * MINUTE));
    assert!(
        old.len() == 1 && old[0]["whole"] == true,
        "a row written before rows met is taken as whole: {old:?}"
    );

    let days = 3 * 24 * 60 * MINUTE;
    let dated = tag(days + 10 * MINUTE, days);
    let unknown = missed(Search::Tag, None, Some(AT - MINUTE), AT);
    let both = kept_after("unknown", slice::from_ref(&dated), unknown.clone());
    assert_eq!(
        both.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [dated["id"].clone(), unknown["id"].clone()],
        "one whose start can't be told takes in no dated row"
    );
    let again = kept_after("unknown-again", slice::from_ref(&unknown), unknown.clone());
    assert_eq!(again.len(), 1, "though it's the same row written again");
    let after = kept_after(
        "after-unknown",
        &[missed(
            Search::Tag,
            None,
            Some(AT - 20 * MINUTE),
            AT - 20 * MINUTE,
        )],
        tag(15 * MINUTE, 10 * MINUTE),
    );
    assert_eq!(
        after.len(),
        2,
        "nor does a dated one take in a row whose start can't be told: {after:?}"
    );
}

#[test]
fn rows_written_before_rows_met_are_folded_into_one() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let rows = kept_after(
        "chain",
        &[
            tag(60 * MINUTE, 45 * MINUTE),
            tag(46 * MINUTE, 30 * MINUTE),
            tag(31 * MINUTE, 15 * MINUTE),
        ],
        tag(16 * MINUTE, 0),
    );
    assert_eq!(
        rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [serde_json::json!(format!(
            "pineapple-slack-missed-tag-{}-{AT}",
            AT - 60 * MINUTE
        ))],
        "the first two meet only once those after them are taken, and are taken too: \
         {rows:?}"
    );
    assert_eq!(
        rows[0]["whole"], true,
        "each overlapping the next, it's whole"
    );
}
