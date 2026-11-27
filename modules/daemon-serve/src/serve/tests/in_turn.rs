//! The cases of asks answered in turn: off their connection's reading
//! thread, so one that waits holds up no ask after it, yet each after
//! those in turn before it, and none of another connection's.

use std::sync::mpsc::Sender;

use super::*;

/// What answers `start`, held until what's sent on the channel lets it
/// go, as a check of who asked waits on a slow user manager, and every
/// other ask at once, each done put in the list in the order done.
fn held_at_start() -> (Own, Sender<()>, Arc<Mutex<Vec<String>>>) {
    let (letting, waiting) = channel::<()>();
    let waiting = Mutex::new(waiting);
    let done = Arc::new(Mutex::new(Vec::new()));
    let doing = Arc::clone(&done);
    let own: Own = Box::new(move |said| {
        let ask = said["ask"].as_str()?.to_owned();
        if ask == "start" {
            // Past the client's own patience, so a case that waits on it
            // fails rather than passing late.
            let _ = waiting
                .lock()
                .map(|waiting| waiting.recv_timeout(Duration::from_secs(30)));
        }
        doing.lock().expect("the order is held").push(ask);
        Some(json!({"done": true}))
    });
    (own, letting, done)
}

#[test]
fn an_ask_in_turn_holds_up_no_ask_after_it_but_those_in_turn() {
    let (own, letting, done) = held_at_start();
    let (shared, mut client, _folder) = served("in-turn", own);
    shared.in_turn(&["start", "end"]);
    client.send(&json!({"id": 1, "ask": "start"}));
    client.send(&json!({"id": 2, "ask": "end"}));
    client.send(&json!({"id": 3, "ask": "get", "keys": ["row:agents:O"]}));
    assert_eq!(
        client.heard(),
        json!({"id": 3, "records": []}),
        "the get is answered while the start waits"
    );
    assert_eq!(
        *done.lock().expect("the order is held"),
        Vec::<String>::new(),
        "the end waits its turn behind the start"
    );
    letting.send(()).expect("the start is let go");
    assert_eq!(client.heard()["id"], 1, "the start is answered once done");
    assert_eq!(client.heard()["id"], 2, "and the end after it");
    assert_eq!(
        *done.lock().expect("the order is held"),
        ["start", "end"],
        "the end is done only once the start is"
    );
}

#[test]
fn an_ask_in_turn_holds_up_none_of_another_connections() {
    let (own, letting, _done) = held_at_start();
    let (shared, mut client, folder) = served("in-turn-apart", own);
    shared.in_turn(&["start", "end"]);
    client.send(&json!({"id": 1, "ask": "start"}));
    let mut other = Client::connect(&folder.0.join("daemon.sock"));
    assert_eq!(
        other.ask(&json!({"id": 1, "ask": "end"})),
        json!({"id": 1, "done": true}),
        "another connection's turn is its own"
    );
    letting.send(()).expect("the start is let go");
    assert_eq!(client.heard()["id"], 1, "the start is answered once done");
}

#[test]
fn an_ask_both_slow_and_in_turn_is_answered_apart() {
    let (own, letting, _done) = held_at_start();
    let (shared, mut client, _folder) = served("in-turn-slow", own);
    shared.in_turn(&["start", "end"]);
    shared.slow(&["end"]);
    client.send(&json!({"id": 1, "ask": "start"}));
    client.send(&json!({"id": 2, "ask": "end"}));
    assert_eq!(
        client.heard()["id"],
        2,
        "the slow one isn't held behind the start"
    );
    letting.send(()).expect("the start is let go");
    assert_eq!(client.heard()["id"], 1, "the start is answered once done");
}
