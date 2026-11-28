use super::super::super::by_key;
use super::*;

const RESET: &str = "model-relay: the origin reset a request on connection 0.407, kept, as one \
                     was retired lately, after 60 asked and 6 cancelled on it: http2 error: \
                     stream error received: unspecific protocol error detected; pi goes directly";
const RESET_AGAIN: &str = "model-relay: the origin reset a request on connection 2.410, already \
                           out of use, after 55 asked and 8 cancelled on it: http2 error: stream \
                           error received: unspecific protocol error detected; it goes once \
                           more, if another connection will take it";
const SENT_AGAIN: &str = "model-relay: a request reset on connection 0.406 is sent again on \
                          connection 0.407";
const UNCARRIED_AGAIN: &str = "model-relay: a request sent again from connection 0.408 couldn't \
                               be carried again, so pi goes directly: http2 error: keep-alive \
                               timed out: operation timed out";
const ANSWERED: &str = "model-relay: a request couldn't be carried, so pi is answered 502: http2 \
                        error: keep-alive timed out: operation timed out";
const ANSWERED_RESET: &str = "model-relay: a request couldn't be carried, so pi is answered 502: \
                              connection error: connection reset";

/// A reset the relay answers 502 rather than sending direct, as
/// model-relay's `said_reset` writes it: its outcome last, after a `; `.
const RESET_ANSWERED: &str = "model-relay: the origin reset a request on connection 1.12, kept, \
                              after 40 asked and 2 cancelled on it: http2 error: stream error \
                              received: refused stream before processing any application logic; \
                              pi is answered 502";

const UNTAKEN: &str = "model-relay: a request reset on connection 0.272 couldn't be sent again, \
                       as no other connection would take it; pi goes directly";
const CLOSED: &str =
    "model-relay: connection 1.432 carried 56 asked, 11 of them cancelled and 0 cut";

/// The relay's ten-minute tally, `asked` since it started.
fn tally(asked: u64) -> String {
    format!(
        "model-relay: {asked} asked, 0 refused, 745 failed, 679 of them reset or left out; 969 \
         reset or left out and sent again; pi cancelled 7214; 0 cut as it was stopped; 147 \
         connections opened, 4 taking requests; 36 broken off midway"
    )
}

#[test]
fn each_line_counts_toward_what_it_says() {
    let journal = [
        RESET,
        RESET_AGAIN,
        SENT_AGAIN,
        UNCARRIED_AGAIN,
        ANSWERED,
        ANSWERED_RESET,
        RESET_ANSWERED,
        CLOSED,
        &tally(1000),
        UNTAKEN,
        &tally(1100),
    ]
    .join("\n");
    let measures = relay(Some(&journal));
    assert_eq!(
        by_key(&measures, RESETS).now,
        3.0,
        "a reset sent direct, one sent again and one answered 502"
    );
    assert_eq!(
        by_key(&measures, DIRECT).now,
        3.0,
        "a reset given to pi, a retry and a request no connection took"
    );
    assert_eq!(
        by_key(&measures, ANSWERED_502).now,
        3.0,
        "two requests carried nowhere and a reset, each answered 502"
    );
    assert_eq!(
        by_key(&measures, SHARE).target,
        Some(SHARE_AT_MOST),
        "two tallies weigh the share"
    );
}

#[test]
fn a_reset_answered_502_is_counted_under_its_resets_own_error() {
    let journal = [RESET_ANSWERED, RESET_ANSWERED].join("\n");
    let measures = relay(Some(&journal));
    assert_eq!(
        by_key(&measures, ANSWERED_502).behind,
        [
            "2 on http2 error: stream error received: refused stream before processing any \
          application logic"
        ],
        "a reset's 502 is grouped apart from its own outcome"
    );
    assert_eq!(
        by_key(&measures, DIRECT).now,
        0.0,
        "a reset answered 502 was counted as sent direct"
    );
}

#[test]
fn a_request_no_connection_took_groups_by_its_cause_not_its_connection() {
    let other = UNTAKEN.replace("0.272", "3.41");
    let journal = [UNTAKEN, &other].join("\n");
    let behind = by_key(&relay(Some(&journal)), DIRECT).behind.clone();
    assert_eq!(behind, ["2 on as no other connection would take it"]);
}

#[test]
fn a_cause_is_cut_short_before_it_reaches_the_card() {
    let long = format!(
        "model-relay: a request couldn't be carried, so pi is answered 502: {}",
        "x".repeat(500)
    );
    let behind = by_key(&relay(Some(&long)), ANSWERED_502).behind.clone();
    assert_eq!(
        behind[0].len(),
        "1 on ".len() + CAUSE_AT_MOST,
        "cut: {behind:?}"
    );
}

#[test]
fn each_is_past_its_ceiling_only_above_it() {
    let at = |line: &str, n: usize| vec![line; n].join("\n");
    let at_most = [
        at(RESET_AGAIN, 400),
        at(UNCARRIED_AGAIN, 150),
        at(ANSWERED, 50),
    ]
    .join("\n");
    let measures = relay(Some(&at_most));
    for key in [RESETS, DIRECT, ANSWERED_502] {
        assert_eq!(
            by_key(&measures, key).past(),
            None,
            "{key} at its ceiling is not past it"
        );
    }
    let past = [
        at(RESET_AGAIN, 401),
        at(UNCARRIED_AGAIN, 151),
        at(ANSWERED, 51),
    ]
    .join("\n");
    let measures = relay(Some(&past));
    for key in [RESETS, DIRECT, ANSWERED_502] {
        assert!(
            by_key(&measures, key).past().is_some(),
            "{key} one above its ceiling is past it"
        );
    }
}

#[test]
fn the_share_weighs_the_resets_between_tallies_against_what_was_asked_between_them() {
    let journal = [
        RESET_AGAIN.to_owned(),
        tally(1000),
        [RESET_AGAIN; 3].join("\n"),
        tally(1100),
        RESET_AGAIN.to_owned(),
    ]
    .join("\n");
    let share = by_key(&relay(Some(&journal)), SHARE).clone();
    assert_eq!(share.now, 3.0, "3 resets of 100 asked between the tallies");
    assert!(share.past().is_some(), "3% is past 2%");
    let at_most = [tally(1000), [RESET_AGAIN; 2].join("\n"), tally(1100)].join("\n");
    assert_eq!(
        by_key(&relay(Some(&at_most)), SHARE).past(),
        None,
        "2% is the most"
    );
}

#[test]
fn a_relay_started_again_counts_its_tally_from_nought() {
    let journal = [tally(5000), RESET_AGAIN.to_owned(), tally(40), tally(50)].join("\n");
    let share = by_key(&relay(Some(&journal)), SHARE).clone();
    assert_eq!(share.now, 2.0, "1 reset of 50 asked");
}

#[test]
fn an_hour_of_fewer_than_two_tallies_leaves_the_share_unjudged() {
    let journal = [RESET, CLOSED, &tally(1000)].join("\n");
    let share = by_key(&relay(Some(&journal)), SHARE).clone();
    assert_eq!(share.target, None);
}

#[test]
fn an_hour_with_no_relay_line_is_unjudged_not_green() {
    let measures = relay(Some("\n"));
    assert_eq!(measures.len(), 4, "all four: {measures:?}");
    for m in &measures {
        assert_eq!(m.target, None, "{} read green with the relay idle", m.key);
        assert!(
            m.says.contains("carried nothing"),
            "it says why: {}",
            m.says
        );
    }
}

#[test]
fn a_journal_unread_leaves_every_one_unjudged() {
    let measures = relay(None);
    assert_eq!(measures.len(), 4, "all four: {measures:?}");
    assert!(
        measures.iter().all(|m| m.target.is_none()),
        "unread is unjudged: {measures:?}"
    );
    assert!(
        measures.iter().all(|m| m.key.starts_with(PREFIX)),
        "each is the relay's: {measures:?}"
    );
}

#[test]
fn behind_each_count_its_commonest_causes_come_first() {
    let journal = [ANSWERED, ANSWERED, ANSWERED_RESET].join("\n");
    let behind = by_key(&relay(Some(&journal)), ANSWERED_502).behind.clone();
    assert_eq!(
        behind,
        [
            "2 on http2 error: keep-alive timed out: operation timed out",
            "1 on connection error: connection reset",
        ]
    );
}

#[test]
fn the_journal_is_the_users_relay_for_the_hour() {
    let args = journal(7200);
    assert_eq!(
        args[..4],
        ["--user", "--quiet", "-u", "model-relay-agents.service"]
    );
    assert!(
        args.contains(&"--since=@3600".to_owned()),
        "read from the hour's start: {args:?}"
    );
    assert!(
        args.contains(&"--until=@7200".to_owned()),
        "read to the hour's end: {args:?}"
    );
}
