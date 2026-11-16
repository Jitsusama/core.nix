//! A page's list whose command is refused while the page watches it, as
//! open-now is while niri stalls drawing nothing, is read again until it
//! answers, so the page doesn't stand without what it holds until its
//! next opening.

use std::time::{Duration, Instant};

use serde_json::json;

use super::Running;

const LISTED: &str = "listed:stalling:open";

#[test]
fn a_page_list_refused_while_watched_is_read_again_until_it_answers() {
    let running = Running::new("stalling");
    std::fs::write(running.path("lists/stalling"), "one\n").unwrap();
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": [LISTED]}));
    assert_eq!(client.changed(LISTED)["lines"], json!(["one"]));
    client.ask(&json!({"id": 1, "ask": "unwatch"}));
    // The next opening's read is refused, as a stalled machine has it, and
    // what it last said is kept.
    std::fs::remove_file(running.path("lists/stalling")).unwrap();
    client.ask(&json!({"id": 2, "ask": "watch", "keys": [LISTED]}));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !running.path("lists/stalling-refused").exists() {
        assert!(Instant::now() < deadline, "the watch's read never ran");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Once it answers again, the page still watching hears it unasked.
    std::fs::write(running.path("lists/stalling"), "two\n").unwrap();
    assert_eq!(
        client.changed(LISTED)["lines"],
        json!(["two"]),
        "a refused read left the page with what it said before"
    );
}
