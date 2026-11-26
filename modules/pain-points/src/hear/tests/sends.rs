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
    assert_eq!(
        (node.held, node.out_of_time),
        (1, 0),
        "the held send is counted as the node's"
    );
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
    assert_eq!(sent, Sent::Held, "a send 20 s long can't start 10 s short");
    assert_eq!(
        (node.held, node.out_of_time),
        (0, 1),
        "the held send is counted as the run's time"
    );
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
fn a_keeper_s_note_held_goes_with_the_next_run_s() {
    let place = Place::new("held-note");
    place.routes();
    a_day(&place);
    let note = |to: &str, text: &str| to == "ORCH01" && text.contains(route::FOR_THE_KEEPER);
    let mut held = 0;
    let hour = hear_putting(&place.ears(), CELLS, &mut |to, text, _| {
        if note(to, text) {
            held += 1;
            Sent::Held
        } else {
            Sent::Went
        }
    })
    .expect("a run");
    assert_eq!(hour.routed, 3, "the day's pains were routed: {hour:?}");
    assert_eq!(held, 1, "the keeper's one note was held: {hour:?}");
    let mut notes = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        if note(to, text) {
            notes.push(text.to_owned());
        }
        true
    })
    .expect("a run");
    assert_eq!(notes.len(), 1, "the held note went with the next run");
    let mut again = 0;
    hear(&place.ears(), CELLS, &mut |to, text| {
        again += usize::from(note(to, text));
        true
    })
    .expect("a run");
    assert_eq!(again, 0, "a note that went is said no more");
}
