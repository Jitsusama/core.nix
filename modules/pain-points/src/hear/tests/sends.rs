//! How a run's sends end within its time: a node that stops answering
//! holds the rest for the next run, a send that couldn't end in time
//! waits untried, and what waits spends no try.

use super::standing::runnable;
use super::*;
use std::time::{Duration, Instant};

/// A mycelium at `place` that notes the words of each ask, then runs
/// `then`, and the file it notes them in.
fn node_doing(place: &Place, then: &str) -> (PathBuf, PathBuf) {
    let mycelium = place.root.join("mycelium");
    let asked = place.root.join("asked");
    runnable(
        &mycelium,
        &format!(
            "#!{}\necho \"$*\" >> '{}'\n{then}\n",
            std::env::var("BASH").unwrap_or_else(|_| "/bin/sh".into()),
            asked.display()
        ),
    );
    (mycelium, asked)
}

/// The words of each ask the node was given.
fn asks(asked: &Path) -> Vec<String> {
    fs::read_to_string(asked)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A node at `mycelium` sending as EAR001, its sends ending in an hour.
fn an_hour_s_node(mycelium: PathBuf, limit: Duration) -> Node {
    let ends = Instant::now() + Duration::from_secs(3600);
    Node::new(mycelium, Some("EAR001".into()), ends, limit)
}

#[test]
fn a_node_that_stops_answering_is_asked_once_and_the_rest_wait() {
    let place = Place::new("silent-node");
    let (mycelium, asked) = node_doing(&place, "sleep 30");
    let mut node = an_hour_s_node(mycelium, Duration::from_secs(1));
    let started = Instant::now();
    let first = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(first, Sent::Failed, "a send that hangs spends its try");
    let second = node.send("GHI789", "another", Put::Asked);
    assert_eq!(second, Sent::Held, "a send after it waits for the next run");
    assert_eq!(node.held, 1, "the held send is counted as the node's");
    assert_eq!(asks(&asked).len(), 1, "the node was asked only once");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stopped, not waited on"
    );
}

#[test]
fn a_node_that_never_names_its_sessions_holds_the_run_s_sends() {
    let place = Place::new("stalled-sessions");
    let (mycelium, asked) = node_doing(&place, "sleep 30");
    let mut node = an_hour_s_node(mycelium, Duration::from_secs(1));
    assert_eq!(node.sessions(), "", "a stalled node names no sessions");
    let sent = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(sent, Sent::Held, "a send after it waits for the next run");
    assert_eq!(
        asks(&asked),
        ["sessions"],
        "the node was asked only for sessions"
    );
}

#[test]
fn a_send_that_could_not_end_by_the_sends_end_waits_untried() {
    let place = Place::new("late-send");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let limit = Duration::from_secs(20);
    // Sends that end ten seconds from now leave no room for one of twenty.
    let ends = Instant::now() + Duration::from_secs(10);
    let mut node = Node::new(mycelium, Some("EAR001".into()), ends, limit);
    let sent = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(
        sent,
        Sent::OutOfTime,
        "a send 20 s long can't start 10 s short"
    );
    assert_eq!(node.held, 0, "a send out of time was counted as held");
    assert!(asks(&asked).is_empty(), "the node was never asked");
}

#[test]
fn a_send_the_node_refused_fails_and_the_next_is_still_tried() {
    let place = Place::new("refusing-node");
    let answer = place.root.join("answer");
    let (mycelium, asked) = node_doing(&place, &format!("cat '{}'", answer.display()));
    let mut node = an_hour_s_node(mycelium, Duration::from_secs(5));
    fs::write(&answer, r#"{"Err":"no such peer"}"#).expect("an answer");
    let refused = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(refused, Sent::Failed, "a refusal is a failed send");
    fs::write(&answer, r#"{"Ok":{"Sent":1}}"#).expect("an answer");
    let took = node.send("GHI789", "a note", Put::Told);
    assert_eq!(took, Sent::Went, "a node that answers is asked again");
    assert_eq!(
        asks(&asked),
        [
            "send --from EAR001 --intent question --text a finding peer.abc123",
            "send --from EAR001 --intent fyi --text a note peer.ghi789",
        ],
        "an owner asked and the keeper told, each by its address"
    );
}

#[test]
fn a_mycelium_that_cant_start_holds_the_rest_of_the_run_untried() {
    let place = Place::new("unstarted-node");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let missing = place.root.join("no-mycelium");
    let mut node = an_hour_s_node(missing.clone(), Duration::from_secs(5));
    let first = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(
        first,
        Sent::Unstarted,
        "a send never started wasn't said as such"
    );
    assert_eq!(
        node.silent,
        Some(Silence::Unstarted(std::io::ErrorKind::NotFound)),
        "the node didn't keep why it couldn't start"
    );
    // Now it would start: the rest of the run still waits.
    fs::rename(&mycelium, &missing).expect("the mycelium put in place");
    let second = node.send("GHI789", "another", Put::Asked);
    assert_eq!(
        second,
        Sent::Unstarted,
        "a send after one that couldn't start wasn't held as such"
    );
    assert!(
        asks(&asked).is_empty(),
        "the run asked the mycelium again once it couldn't start"
    );
    assert_eq!(
        node.held, 0,
        "sends that couldn't start were said as the mycelium stopped answering"
    );
}

#[test]
fn a_mycelium_that_cant_start_names_no_sessions_and_holds_the_run() {
    let place = Place::new("unstarted-sessions");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let missing = place.root.join("no-mycelium");
    let mut node = an_hour_s_node(missing.clone(), Duration::from_secs(5));
    assert_eq!(
        node.sessions(),
        "",
        "a mycelium never started named sessions"
    );
    fs::rename(&mycelium, &missing).expect("the mycelium put in place");
    let sent = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(
        sent,
        Sent::Unstarted,
        "a send after the sessions couldn't be asked was tried"
    );
    assert!(asks(&asked).is_empty(), "the run asked the mycelium again");
}

#[test]
fn a_send_as_no_one_waits_untried_and_is_counted() {
    let place = Place::new("no-one");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let ends = Instant::now() + Duration::from_secs(3600);
    let mut no_one = Node::new(mycelium, None, ends, Duration::from_secs(5));
    let sent = no_one.send("ABC123", "a finding", Put::Asked);
    assert_eq!(sent, Sent::Held, "a send as no one spent its try");
    assert_eq!(no_one.fromless, 1, "a send as no one went uncounted");
    assert!(
        asks(&asked).is_empty(),
        "the node was asked to send as no one"
    );
}

#[test]
fn a_text_no_send_can_carry_fails_and_the_next_send_goes() {
    let place = Place::new("nul-text");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let mut node = an_hour_s_node(mycelium, Duration::from_secs(5));
    let refused = node.send("ABC123", "a\0finding", Put::Asked);
    assert_eq!(
        refused,
        Sent::Failed,
        "a text no send can carry didn't spend its try"
    );
    assert_eq!(node.silent, None, "a text no send can carry held the run");
    let took = node.send("GHI789", "a note", Put::Told);
    assert_eq!(took, Sent::Went, "the send after it didn't go");
    assert_eq!(asks(&asked).len(), 1, "the node wasn't asked the one send");
}

#[test]
fn a_pain_for_a_fallback_never_sent_to_waits_with_its_tries() {
    let place = Place::new("refused-fallback");
    fs::write(
        place.root.join("state/routes.json"),
        r#"{"from":"EAR001","fallback":"VOID01","ledgers":"ORCH01","never":["VOID01"]}"#,
    )
    .expect("routes");
    a_day(&place);
    for _ in 0..5 {
        let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
        assert_eq!(
            hour.troubles,
            ["routes.json's fallback VOID01 is one never to send to, so what goes to it waits"],
            "the refused fallback wasn't the one thing said of the run: {hour:?}"
        );
    }
    let heard = place.heard();
    let void = heard
        .iter()
        .find(|h| h.words.contains("why is the launcher so slow"))
        .expect("the void's pain heard");
    assert!(
        void.routed_at.is_none() && void.tries == 0,
        "the pain for a fallback never sent to was taken as sent, or spent tries: {void:?}"
    );
}

#[test]
fn a_held_send_spends_no_try() {
    let place = Place::new("held");
    place.routes();
    a_day(&place);
    for _ in 0..5 {
        let hour = hear_putting(&place.ears(), CELLS, &mut |_, _, _| Sent::Held).expect("a run");
        assert!(
            hour.troubles.is_empty(),
            "a held send read as a failed one: {hour:?}"
        );
    }
    let mut to_owner = 0;
    hear(&place.ears(), CELLS, &mut |to, _| {
        to_owner += usize::from(to == "ABC123");
        true
    })
    .expect("a run");
    assert_eq!(to_owner, 1, "five held runs left the pain its three tries");
}

#[test]
fn sends_out_of_the_run_s_time_are_a_trouble_of_the_hour_and_keep_their_tries() {
    let place = Place::new("out-of-time-hour");
    place.routes();
    a_day(&place);
    for _ in 0..5 {
        let mut tried = 0;
        let hour = hear_putting(&place.ears(), CELLS, &mut |_, _, _| {
            tried += 1;
            Sent::OutOfTime
        })
        .expect("a run");
        // The day's live pain to its owner and the void's to the fallback.
        assert_eq!(tried, 2, "the run didn't try the day's two sends");
        assert_eq!(
            hour.troubles,
            [format!(
                "{tried} sends wait for the next run, as its time for sends ran out"
            )],
            "a run too slow to send didn't say so alone, with how many"
        );
    }
    let mut to_owner = 0;
    hear(&place.ears(), CELLS, &mut |to, _| {
        to_owner += usize::from(to == "ABC123");
        true
    })
    .expect("a run");
    assert_eq!(
        to_owner, 1,
        "five runs out of time left the pain its three tries"
    );
}

#[test]
fn a_pain_too_old_for_its_owner_keeps_its_tries_while_the_keeper_s_ask_is_untried() {
    for untried in [Sent::OutOfTime, Sent::Held, Sent::Unstarted] {
        an_old_pain_with_its_keeper(untried);
    }
}

/// A pain too old for its owner, its keeper's ask answered `untried` five
/// runs, keeps its tries and goes to the keeper with the next run's.
fn an_old_pain_with_its_keeper(untried: Sent) {
    let place = Place::new("old-untried");
    place.routes();
    place.log(
        "src",
        "s-live",
        Path::new("/home/j/src"),
        &[
            user("old", "2026-11-16T15:00:00.000Z", "status update please"),
            user("new", "2026-11-16T19:50:00.000Z", "no, that's broken"),
        ],
    );
    for _ in 0..5 {
        let keeper_out = |to: &str| if to == "ORCH01" { untried } else { Sent::Went };
        hear_putting(&place.ears(), CELLS, &mut |to, _, _| keeper_out(to)).expect("a run");
    }
    let heard = place.heard();
    let old = heard
        .iter()
        .find(|h| h.words.contains("status update please"))
        .expect("the old pain heard");
    assert!(
        old.routed_at.is_none() && old.tries == 0,
        "the old pain was taken as routed, or spent tries, while its keeper's ask was \
         {untried:?}: {old:?}"
    );
    let mut to_keeper = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        if to == "ORCH01" {
            to_keeper.push(text.to_owned());
        }
        true
    })
    .expect("a run");
    assert!(
        to_keeper
            .iter()
            .any(|text| text.contains("status update please")),
        "five runs {untried:?} spent the old pain's tries: {to_keeper:?}"
    );
}

#[test]
fn a_keeper_s_note_not_tried_goes_with_the_next_run_s() {
    for untried in [Sent::Held, Sent::OutOfTime, Sent::Unstarted] {
        a_keeper_s_note_answered(untried);
    }
}

/// The keeper's one note, answered `untried` by its first run, goes with
/// the next and is said no more after; one out of time, or that the
/// mycelium couldn't start, is the hour's trouble, and one held, as the
/// node stopped answering, is not.
fn a_keeper_s_note_answered(untried: Sent) {
    let place = Place::new("untried-note");
    place.routes();
    a_day(&place);
    let note = |to: &str, text: &str| to == "ORCH01" && text.contains(route::FOR_THE_KEEPER);
    let mut untried_sends = 0;
    let hour = hear_putting(&place.ears(), CELLS, &mut |to, text, _| {
        if note(to, text) {
            untried_sends += 1;
            untried
        } else {
            Sent::Went
        }
    })
    .expect("a run");
    assert_eq!(hour.routed, 3, "the day's pains were routed: {hour:?}");
    assert_eq!(
        untried_sends, 1,
        "the keeper's one note wasn't {untried:?}: {hour:?}"
    );
    let troubles: &[&str] = match untried {
        Sent::OutOfTime => &["1 sends wait for the next run, as its time for sends ran out"],
        Sent::Unstarted => &["1 sends wait for the next run, as the mycelium couldn't be started"],
        _ => &[],
    };
    assert_eq!(
        hour.troubles, troubles,
        "a note {untried:?} wasn't the hour's trouble it should be: {hour:?}"
    );
    let mut notes = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        if note(to, text) {
            notes.push(text.to_owned());
        }
        true
    })
    .expect("a run");
    assert_eq!(
        notes.len(),
        1,
        "the note {untried:?} didn't go with the next run"
    );
    let mut again = 0;
    hear(&place.ears(), CELLS, &mut |to, text| {
        again += usize::from(note(to, text));
        true
    })
    .expect("a run");
    assert_eq!(again, 0, "a note {untried:?} that went was said again");
}

#[test]
fn a_keeper_s_note_held_past_a_day_is_said_no_more() {
    let place = Place::new("held-note");
    place.routes();
    a_day(&place);
    let note = |to: &str, text: &str| to == "ORCH01" && text.contains(route::FOR_THE_KEEPER);
    hear_putting(&place.ears(), CELLS, &mut |to, text, _| {
        if note(to, text) {
            Sent::Held
        } else {
            Sent::Went
        }
    })
    .expect("a run");
    let mut later = place.ears();
    later.at += DAY_MS + 60_000;
    let mut notes = Vec::new();
    hear(&later, CELLS, &mut |to, text| {
        if note(to, text) {
            notes.push(text.to_owned());
        }
        true
    })
    .expect("a run");
    assert!(
        notes.is_empty(),
        "a note held over a day went as new once the keeper took sends: {notes:?}"
    );
}
