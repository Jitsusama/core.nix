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

fn closed(asked: u64) -> String {
    format!("model-relay: connection 1.432 carried {asked} asked, 11 of them cancelled and 0 cut")
}

fn by_key<'a>(measures: &'a [Measure], key: &str) -> &'a Measure {
    measures
        .iter()
        .find(|m| m.key == key)
        .unwrap_or_else(|| panic!("no {key}"))
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
        &closed(56),
        &closed(44),
    ]
    .join("\n");
    let measures = relay(Some(&journal));
    assert_eq!(by_key(&measures, RESETS).now, 2.0);
    assert_eq!(
        by_key(&measures, DIRECT).now,
        2.0,
        "a reset given to pi, and a retry"
    );
    assert_eq!(by_key(&measures, ANSWERED_502).now, 2.0);
    assert_eq!(by_key(&measures, SHARE).now, 2.0, "2 resets of 100 carried");
}

#[test]
fn each_is_past_its_ceiling_only_above_it() {
    let at = |line: &str, n: usize| vec![line; n].join("\n");
    let journal = [at(RESET_AGAIN, 400), at(ANSWERED, 51)].join("\n");
    let measures = relay(Some(&journal));
    assert_eq!(
        by_key(&measures, RESETS).past(),
        None,
        "400 is the most, not past it"
    );
    assert!(
        by_key(&measures, ANSWERED_502).past().is_some(),
        "51 is past 50"
    );
    assert_eq!(by_key(&measures, DIRECT).past(), None);
}

#[test]
fn the_share_weighs_resets_against_what_closed_connections_carried() {
    let journal = [[RESET_AGAIN; 3].join("\n"), closed(100)].join("\n");
    let share = by_key(&relay(Some(&journal)), SHARE).clone();
    assert_eq!(share.now, 3.0);
    assert!(share.past().is_some(), "3% is past 2%");
}

#[test]
fn an_hour_no_connection_closed_in_leaves_the_share_unjudged() {
    let share = by_key(&relay(Some(RESET)), SHARE).clone();
    assert_eq!(share.target, None);
}

#[test]
fn an_hour_with_no_relay_line_is_judged_in() {
    let measures = relay(Some(""));
    for key in [RESETS, DIRECT, ANSWERED_502] {
        let m = by_key(&measures, key);
        assert_eq!((m.now, m.past()), (0.0, None), "{key}");
        assert!(m.target.is_some(), "{key} is judged");
    }
}

#[test]
fn a_journal_unread_leaves_every_one_unjudged() {
    let measures = relay(None);
    assert_eq!(measures.len(), 4);
    assert!(measures.iter().all(|m| m.target.is_none()));
    assert!(measures.iter().all(|m| m.key.starts_with(PREFIX)));
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
    assert_eq!(args[..3], ["--user", "-u", "model-relay-agents.service"]);
    assert!(args.contains(&"--since=@3600".to_owned()));
    assert!(args.contains(&"--until=@7200".to_owned()));
}
