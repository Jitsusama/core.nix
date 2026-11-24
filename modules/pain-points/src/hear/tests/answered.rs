//! A pain taken up by its session's own cell answering him where he said
//! it: which answers count, which pains each takes, and which cells may
//! take any.

use super::*;

/// The agent ending its turn at `stamp` as `stop`, saying it was done.
fn ended(stamp: &str, stop: &str) -> String {
    json!({"type": "message", "id": "s", "parentId": "r", "timestamp": stamp,
           "message": {"role": "assistant", "stopReason": stop, "timestamp": 0,
                       "content": [{"type": "text", "text": "Done: it opens again."}]}})
    .to_string()
}

#[test]
fn a_pain_its_session_answered_him_on_is_taken_by_that_cell() {
    let place = Place::new("answered");
    place.routes();
    let mut live = vec![user(
        "a",
        "2026-11-16T19:10:00.000Z",
        "no, that's broken again",
    )];
    live.extend(tool("bash"));
    live.extend([
        ended("2026-11-16T19:30:00.000Z", "stop"),
        user("b", "2026-11-16T19:40:00.000Z", "wtf, the bar is wrong now"),
    ]);
    live.extend(tool("edit"));
    live.push(ended("2026-11-16T19:50:00.000Z", "error"));
    place.log("src", "s-live", Path::new("/home/j/src"), &live);
    // A run whose node names no cell for the session keeps the answer for
    // the run that does.
    let unnamed = r#"{"Ok":{"Sessions":[{"id":"s-void","address":"VOID01","state":"idle"}]}}"#;
    hear(&place.ears(), unnamed, &mut |_, _| true).expect("a run with no cell for the session");
    assert!(
        place.heard().iter().all(|h| h.taken_by.is_none()),
        "a pain was taken with no cell for its session"
    );
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    let heard = place.heard();
    let taken: Vec<(&str, Option<&str>)> = heard
        .iter()
        .map(|h| (h.entry.as_str(), h.taken_by.as_deref()))
        .collect();
    assert_eq!(
        taken,
        vec![("a", Some("ABC123")), ("b", None)],
        "the pain its cell answered is taken, the one a failed turn left isn't"
    );
    assert_eq!((hour.open, hour.taken), (1, 1), "{hour:?}");
}

#[test]
fn an_answer_takes_up_only_what_it_answered_and_never_for_a_refused_cell() {
    let place = Place::new("answered-since");
    place.routes();
    let mut live = vec![user(
        "a",
        "2026-11-16T19:10:00.000Z",
        "no, that's broken again",
    )];
    live.extend(tool("bash"));
    live.extend([
        ended("2026-11-16T19:20:00.000Z", "error"),
        user("c", "2026-11-16T19:25:00.000Z", "now make the bar green"),
    ]);
    live.extend(tool("edit"));
    live.extend([
        ended("2026-11-16T19:30:00.000Z", "stop"),
        user("b", "2026-11-16T19:40:00.000Z", "wtf, the bar is wrong now"),
    ]);
    live.extend(tool("edit"));
    live.push(ended("2026-11-16T19:50:00.000Z", "stop"));
    place.log("src", "s-live", Path::new("/home/j/src"), &live);
    hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    let heard = place.heard();
    let taken: Vec<(&str, Option<&str>, Option<i64>)> = heard
        .iter()
        .map(|h| (h.entry.as_str(), h.taken_by.as_deref(), h.answered_at))
        .collect();
    // 19:50 UTC on 16 November 2026.
    let answered = Some(AT - 10 * 60_000);
    assert_eq!(
        taken,
        vec![("a", None, None), ("b", Some("ABC123"), answered)],
        "the pain a failed turn left stays open past an answer to another ask"
    );

    let void = Place::new("answered-void");
    void.routes();
    void.log("void", "s-void", &void.root.join("void"), &live);
    hear(&void.ears(), CELLS, &mut |_, _| true).expect("a run");
    let heard = void.heard();
    assert!(
        heard.len() == 2 && heard.iter().all(|h| h.void && h.taken_by.is_none()),
        "the void has no cell to take a pain up: {heard:?}"
    );

    let refused = Place::new("answered-refused");
    fs::write(
        refused.root.join("state/routes.json"),
        r#"{"from":"EAR001","fallback":"ORCH01","ledgers":"ORCH01","never":["ABC123"]}"#,
    )
    .expect("routes written");
    refused.log("src", "s-live", Path::new("/home/j/src"), &live);
    hear(&refused.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert!(
        refused.heard().iter().all(|h| h.taken_by.is_none()),
        "a cell the routes refuse took a pain up"
    );
}
