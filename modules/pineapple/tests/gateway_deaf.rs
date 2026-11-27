//! When the safe word's Slack reader over a tool gateway tells Joel it
//! can't hear Slack, and what it asks of him, since the gateway's sign-in
//! isn't Slack's.

use pineapple::deaf::{Change, Deaf, Deafness, Round, Search, UNREACHED, waits, waits_on_gateway};

const AT: u64 = 1_795_000_000_000;

#[test]
fn a_gateway_unreached_past_its_rounds_is_told_as_deaf_and_heard_again_untold() {
    let mut deaf = Deaf::through_gateway();
    let round = |n: u32| AT + u64::from(n) * 30_000;
    for n in 1..=UNREACHED + 1 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n} unasked isn't told yet"
        );
    }
    let at = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, at),
        Change::Tell(waits_on_gateway(Deafness::Unreached, at, at)),
        "a gateway unreached past its rounds is told, once the cause has held"
    );
    let item = &waits_on_gateway(Deafness::Unreached, at, at)["items"][0];
    assert!(
        item["why"]
            .as_str()
            .is_some_and(|why| why.contains("isn't answering its searches"))
            && item["doneWhen"] == serde_json::json!({"by": "person"}),
        "it says the gateway isn't answering, as a tool failing gets there too, and only \
         Joel marks it done: {item}"
    );
    assert_eq!(
        deaf.heard(Round::Unasked, round(UNREACHED + 3)),
        Change::Keep,
        "told once, not each round"
    );
    assert_eq!(
        deaf.heard(Round::Heard, round(UNREACHED + 4)),
        Change::Untell,
        "heard again, the row is taken away"
    );
    for n in UNREACHED + 5..=2 * UNREACHED + 5 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "heard again, the count starts anew, so round {n} unasked isn't told yet"
        );
    }
    let mut keyring = Deaf::new();
    let _ = keyring.heard(Round::Heard, AT);
    for n in 1..=3 * UNREACHED {
        assert_eq!(
            keyring.heard(Round::Unasked, round(n)),
            Change::Keep,
            "the keyring reader counts a round unasked neither way, round {n}"
        );
    }
}

#[test]
fn a_round_read_in_part_ends_a_run_unasked_so_an_answering_gateway_isnt_told() {
    let mut deaf = Deaf::through_gateway();
    let round = |n: u32| AT + u64::from(n) * 30_000;
    for n in 1..=3 * UNREACHED {
        let heard = if n % UNREACHED == 0 {
            Round::Partly(None)
        } else {
            Round::Unasked
        };
        assert_eq!(
            deaf.heard(heard, round(n)),
            Change::Keep,
            "round {n}: a gateway that answers a search every tenth round is reached"
        );
    }
    let after = 3 * UNREACHED;
    for n in after + 1..=after + UNREACHED + 1 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n}: the run unasked counts again from the round read in part"
        );
    }
    let at = round(after + UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, at),
        Change::Tell(waits_on_gateway(Deafness::Unreached, at, at)),
        "a whole run unasked after it is told as before"
    );
}

#[test]
fn a_search_left_unasked_round_after_round_beside_one_answered_is_told_unread() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let tag = Round::Partly(Some(Search::Tag));
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 1 {
        assert_eq!(
            deaf.heard(tag, round(n)),
            Change::Keep,
            "round {n} with the tag's search unasked isn't told yet"
        );
    }
    let at = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(tag, at),
        Change::Tell(waits_on_gateway(Deafness::Unread(Search::Tag), at, at)),
        "a search unasked past its rounds beside one answered is told as unread"
    );
    assert_eq!(
        deaf.heard(tag, round(UNREACHED + 3)),
        Change::Keep,
        "told once, not each round"
    );
    assert_eq!(
        deaf.heard(Round::Heard, round(UNREACHED + 4)),
        Change::Untell,
        "heard again, the row is taken away"
    );
    for n in 1..=3 * UNREACHED {
        let left = if n % UNREACHED == 0 {
            Search::Phrase
        } else {
            Search::Tag
        };
        assert_eq!(
            deaf.heard(Round::Partly(Some(left)), round(n)),
            Change::Keep,
            "round {n}: the other search unasked starts the run afresh"
        );
    }
    let mut then = Deaf::through_gateway();
    let phrase = Round::Deaf(Deafness::Unread(Search::Phrase));
    let _ = then.heard(phrase, AT);
    let _ = then.heard(phrase, round(1));
    assert_eq!(
        then.heard(tag, round(2)),
        Change::Untell,
        "a round the phrase's search answered takes away the row saying it can't be read"
    );
    for n in 3..=UNREACHED + 2 {
        assert_eq!(then.heard(tag, round(n)), Change::Keep, "round {n}");
    }
    let at = round(UNREACHED + 3);
    assert_eq!(
        then.heard(tag, at),
        Change::Tell(waits_on_gateway(Deafness::Unread(Search::Tag), at, at)),
        "then the tag's alone is told as the tag's, never as both, as the phrase's answered"
    );
    let mut keyring = Deaf::new();
    let _ = keyring.heard(Round::Heard, AT);
    for n in 1..=3 * UNREACHED {
        assert_eq!(
            keyring.heard(tag, round(n)),
            Change::Keep,
            "the keyring reader counts a round read in part neither way, round {n}"
        );
    }
}

#[test]
fn a_heard_round_or_a_deaf_one_of_another_cause_starts_each_run_afresh() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let runs = [Round::Unasked, Round::Partly(Some(Search::Tag))];
    let breaks = [Round::Heard, Round::Deaf(Deafness::Refused)];
    for (run, broken_by) in runs.into_iter().flat_map(|run| breaks.map(|by| (run, by))) {
        let mut deaf = Deaf::through_gateway();
        // A reader's first round heard takes away whatever row an earlier
        // one left, so it's had before the run.
        let _ = deaf.heard(Round::Heard, round(0));
        for n in 1..=UNREACHED {
            assert_eq!(deaf.heard(run, round(n)), Change::Keep, "{run:?} round {n}");
        }
        assert_eq!(
            deaf.heard(broken_by, round(UNREACHED + 1)),
            Change::Keep,
            "{broken_by:?} once, after {run:?} rounds that told nothing, tells nothing"
        );
        for n in UNREACHED + 2..=2 * UNREACHED + 1 {
            assert_eq!(
                deaf.heard(run, round(n)),
                Change::Keep,
                "after {broken_by:?}, {run:?} counts afresh, so round {n} isn't told yet"
            );
        }
    }
}

#[test]
fn a_gateway_answering_takes_away_its_unreached_row_so_an_outage_after_is_told_anew() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 1 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n}"
        );
    }
    let first = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, first),
        Change::Tell(waits_on_gateway(Deafness::Unreached, first, first)),
        "unreached, told"
    );
    assert_eq!(
        deaf.heard(Round::Partly(None), round(UNREACHED + 3)),
        Change::Untell,
        "a round the gateway answered in part takes the unreached row away"
    );
    let after = UNREACHED + 3;
    for n in after + 1..=after + UNREACHED + 1 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n}"
        );
    }
    let again = round(after + UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, again),
        Change::Tell(waits_on_gateway(Deafness::Unreached, again, again)),
        "an outage after is told anew, as a time of its own, even were the first marked done"
    );
}

#[test]
fn an_outage_then_one_search_answered_narrows_its_row_to_the_other_rather_than_blanking_it() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 1 {
        let _ = deaf.heard(Round::Unasked, round(n));
    }
    let first = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, first),
        Change::Tell(waits_on_gateway(Deafness::Unreached, first, first)),
        "unreached, told"
    );
    let tag = Round::Partly(Some(Search::Tag));
    assert_eq!(
        deaf.heard(tag, round(UNREACHED + 3)),
        Change::Keep,
        "one round answering only the phrase leaves the row standing, never blank"
    );
    let at = round(UNREACHED + 4);
    assert_eq!(
        deaf.heard(tag, at),
        Change::Tell(waits_on_gateway(Deafness::Unread(Search::Tag), at, at)),
        "a second narrows it to the tag's, still unheard, as a time of its own"
    );
}

#[test]
fn an_outage_then_its_searches_answered_by_turns_takes_its_row_away() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 1 {
        let _ = deaf.heard(Round::Unasked, round(n));
    }
    let first = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, first),
        Change::Tell(waits_on_gateway(Deafness::Unreached, first, first)),
        "unreached, told"
    );
    assert_eq!(
        deaf.heard(Round::Partly(Some(Search::Tag)), round(UNREACHED + 3)),
        Change::Keep,
        "one round answering only the phrase leaves the row standing"
    );
    assert_eq!(
        deaf.heard(Round::Partly(Some(Search::Phrase)), round(UNREACHED + 4)),
        Change::Untell,
        "the next answering only the tag shows each heard, so the row goes"
    );
    for n in UNREACHED + 5..UNREACHED + 9 {
        let alone = if (n - UNREACHED) % 2 == 1 {
            Search::Tag
        } else {
            Search::Phrase
        };
        assert_eq!(
            deaf.heard(Round::Partly(Some(alone)), round(n)),
            Change::Keep,
            "round {n}: searches answered by turns tell nothing"
        );
    }
}

#[test]
fn a_search_answered_before_a_silence_is_no_turn_after_it() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 2 {
        let _ = deaf.heard(Round::Unasked, round(n));
    }
    assert_eq!(
        deaf.heard(Round::Partly(Some(Search::Tag)), round(UNREACHED + 3)),
        Change::Keep,
        "the phrase answered alone leaves the unreached row standing"
    );
    for n in UNREACHED + 4..UNREACHED + 7 {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n}: the gateway silent again"
        );
    }
    assert_eq!(
        deaf.heard(Round::Partly(Some(Search::Phrase)), round(UNREACHED + 7)),
        Change::Keep,
        "the tag answered alone after a silence leaves the row: the phrase is unheard now"
    );
    assert_eq!(
        deaf.heard(Round::Partly(Some(Search::Tag)), round(UNREACHED + 8)),
        Change::Untell,
        "the phrase answered the very next round shows each heard"
    );
}

#[test]
fn a_row_left_from_before_goes_once_its_searches_answer_by_turns() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    // A reader started anew, as after a restart in an outage, whose row
    // may still stand.
    let mut restarted = Deaf::through_gateway();
    assert_eq!(
        restarted.heard(Round::Partly(Some(Search::Tag)), round(1)),
        Change::Keep,
        "one search answered alone may leave the row's other unheard"
    );
    assert_eq!(
        restarted.heard(Round::Partly(Some(Search::Phrase)), round(2)),
        Change::Untell,
        "the other answered the very next round shows each heard"
    );
    assert_eq!(
        restarted.heard(Round::Partly(Some(Search::Tag)), round(3)),
        Change::Keep,
        "taken away once"
    );
    // A reader whose untelling wasn't written, its row still standing.
    let mut unwritten = Deaf::through_gateway();
    let _ = unwritten.heard(Round::Heard, round(1));
    unwritten.failed(&Change::Untell);
    assert_eq!(
        unwritten.heard(Round::Partly(Some(Search::Phrase)), round(2)),
        Change::Keep,
        "one search answered alone isn't enough"
    );
    assert_eq!(
        unwritten.heard(Round::Unasked, round(3)),
        Change::Keep,
        "a silence between forgets it"
    );
    assert_eq!(
        unwritten.heard(Round::Partly(Some(Search::Tag)), round(4)),
        Change::Keep,
        "the other after a silence isn't each heard in a row"
    );
    assert_eq!(
        unwritten.heard(Round::Partly(Some(Search::Phrase)), round(5)),
        Change::Untell,
        "the next round's other is"
    );
}

#[test]
fn a_row_left_from_before_waits_on_the_round_just_before_whatever_reads_it() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut waiting = Deaf::through_gateway();
    assert_eq!(
        waiting.heard(Round::Partly(Some(Search::Tag)), round(1)),
        Change::Keep,
        "one search answered alone"
    );
    assert_eq!(
        waiting.heard(Round::Partly(None), round(2)),
        Change::Keep,
        "a search waiting on a later page answers neither alone"
    );
    assert_eq!(
        waiting.heard(Round::Partly(Some(Search::Phrase)), round(3)),
        Change::Keep,
        "so the other after it isn't each heard in a row"
    );
    let mut unread = Deaf::through_gateway();
    let _ = unread.heard(Round::Partly(Some(Search::Tag)), round(1));
    assert_eq!(
        unread.heard(Round::Deaf(Deafness::Unreadable), round(2)),
        Change::Keep,
        "one round unreadable isn't yet told"
    );
    assert_eq!(
        unread.heard(Round::Partly(Some(Search::Phrase)), round(3)),
        Change::Keep,
        "an unreadable round between forgets the round before"
    );
    // A reader through the keyring, its searches failing by turns.
    let mut keyring = Deaf::new();
    assert_eq!(
        keyring.heard(Round::Partly(Some(Search::Tag)), round(1)),
        Change::Keep,
        "the keyring reader's one search answered alone"
    );
    assert_eq!(
        keyring.heard(Round::Partly(Some(Search::Phrase)), round(2)),
        Change::Untell,
        "the keyring reader read Slack whole across the two as well"
    );
}

#[test]
fn a_search_left_unasked_is_never_told_as_alone_while_the_gateway_answers_nothing() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    let tag = Round::Partly(Some(Search::Tag));
    for n in 1..UNREACHED {
        assert_eq!(deaf.heard(tag, round(n)), Change::Keep, "round {n}");
    }
    for n in UNREACHED..=2 * UNREACHED {
        assert_eq!(
            deaf.heard(Round::Unasked, round(n)),
            Change::Keep,
            "round {n}: with nothing answered, the tag's run isn't told as the phrase heard"
        );
    }
    let at = round(2 * UNREACHED + 1);
    assert_eq!(
        deaf.heard(Round::Unasked, at),
        Change::Tell(waits_on_gateway(Deafness::Unreached, at, at)),
        "it's told as the gateway not answering"
    );
}

#[test]
fn a_search_left_unasked_alone_counts_on_through_rounds_none_was_answered_in() {
    let round = |n: u32| AT + u64::from(n) * 30_000;
    let mut deaf = Deaf::through_gateway();
    for n in 1..=UNREACHED + 1 {
        let heard = if n % UNREACHED == 1 {
            Round::Partly(Some(Search::Tag))
        } else {
            Round::Unasked
        };
        assert_eq!(deaf.heard(heard, round(n)), Change::Keep, "round {n}");
    }
    let at = round(UNREACHED + 2);
    assert_eq!(
        deaf.heard(Round::Unasked, at),
        Change::Tell(waits_on_gateway(Deafness::Unread(Search::Tag), at, at)),
        "a tag left unasked in every round, the phrase answered once in ten, is told by the \
         twelfth, not after eleven rounds read in part"
    );
}

#[test]
fn a_reader_through_a_gateway_never_sends_joel_to_sign_in_to_slack() {
    let mut deaf = Deaf::through_gateway();
    let refused = Round::Deaf(Deafness::Refused);
    deaf.heard(refused, AT);
    let told = deaf.heard(refused, AT + 30_000);
    assert_eq!(
        told,
        Change::Tell(waits_on_gateway(
            Deafness::Refused,
            AT + 30_000,
            AT + 30_000
        )),
        "told in the gateway's words"
    );
    for why in [Deafness::Refused, Deafness::NoSignIn] {
        let item = &waits_on_gateway(why, AT, AT)["items"][0];
        let said = item["why"].as_str().unwrap_or_default();
        assert!(
            said.contains("tool gateway") && !said.contains("keyring"),
            "{why:?} names the gateway's sign-in: {said}"
        );
        assert_eq!(
            item["doneWhen"],
            serde_json::json!({"by": "person"}),
            "{why:?} is marked done by Joel alone, since Slack's sign-in mends nothing"
        );
        assert!(
            !item["steps"].to_string().contains("Sign in to Slack"),
            "{why:?} sends him to no Slack sign-in: {}",
            item["steps"]
        );
    }
    assert_eq!(
        waits_on_gateway(Deafness::Unreadable, AT, AT),
        waits(Deafness::Unreadable, AT, AT),
        "what Slack's answers say is the same either way"
    );
}
