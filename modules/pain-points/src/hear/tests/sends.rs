//! How a run's sends end within its time: a node that stops answering
//! holds the rest for the next run, a send that couldn't end in time
//! waits untried, and what waits spends no try and is no trouble.

use super::standing::runnable;
use super::*;
use std::time::{Duration, Instant};

/// A mycelium at `place` that notes each send it's asked, then runs
/// `then`, and the file it notes them in.
fn node_doing(place: &Place, then: &str) -> (PathBuf, PathBuf) {
    let mycelium = place.root.join("mycelium");
    let asked = place.root.join("asked");
    runnable(
        &mycelium,
        &format!(
            "#!{}\necho \"$1\" >> {}\n{then}\n",
            std::env::var("BASH").unwrap_or_else(|_| "/bin/sh".into()),
            asked.display()
        ),
    );
    (mycelium, asked)
}

/// How many sends the node was asked.
fn sends_asked(asked: &Path) -> usize {
    fs::read_to_string(asked)
        .unwrap_or_default()
        .lines()
        .count()
}

#[test]
fn a_node_that_stops_answering_is_asked_once_and_the_rest_wait() {
    let place = Place::new("silent-node");
    let (mycelium, asked) = node_doing(&place, "sleep 30");
    let limit = Duration::from_millis(200);
    let mut node = Node::new(mycelium, Some("EAR001".into()), Instant::now(), limit);
    let started = Instant::now();
    let first = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(first, Sent::Failed, "a send that hangs spends its try");
    let second = node.send("GHI789", "another", Put::Asked);
    assert_eq!(second, Sent::Held, "a send after it waits for the next run");
    assert_eq!(node.held, 1, "the held send is counted for the journal");
    assert_eq!(sends_asked(&asked), 1, "the node was asked only once");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stopped, not waited on"
    );
}

#[test]
fn a_send_that_could_not_end_by_the_sends_end_waits_untried() {
    let place = Place::new("late-send");
    let (mycelium, asked) = node_doing(&place, r#"echo '{"Ok":{"Sent":1}}'"#);
    let limit = Duration::from_secs(20);
    let begun = Instant::now()
        .checked_sub(Duration::from_secs(170))
        .expect("a run begun 170 s ago");
    let mut node = Node::new(mycelium, Some("EAR001".into()), begun, limit);
    let sent = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(sent, Sent::Held, "a send 20 s long can't start 170 s in");
    assert_eq!(sends_asked(&asked), 0, "the node was never asked");
}

#[test]
fn a_send_the_node_refused_fails_and_the_next_is_still_tried() {
    let place = Place::new("refusing-node");
    let answer = place.root.join("answer");
    let (mycelium, asked) = node_doing(&place, &format!("cat {}", answer.display()));
    let limit = Duration::from_secs(5);
    let mut node = Node::new(mycelium, Some("EAR001".into()), Instant::now(), limit);
    fs::write(&answer, r#"{"Err":"no such peer"}"#).expect("an answer");
    let refused = node.send("ABC123", "a finding", Put::Asked);
    assert_eq!(refused, Sent::Failed, "a refusal is a failed send");
    fs::write(&answer, r#"{"Ok":{"Sent":1}}"#).expect("an answer");
    let took = node.send("ABC123", "a finding", Put::Told);
    assert_eq!(took, Sent::Went, "a node that answers is asked again");
    assert_eq!(sends_asked(&asked), 2, "both sends were tried");
}

#[test]
fn a_held_send_spends_no_try_and_is_no_trouble() {
    let place = Place::new("held");
    place.routes();
    a_day(&place);
    for _ in 0..5 {
        let hour = hear_putting(&place.ears(), CELLS, &mut |_, _, _| Sent::Held).expect("a run");
        assert!(
            !hour
                .troubles
                .iter()
                .any(|t| t.contains("failed") || t.contains("gave up")),
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
