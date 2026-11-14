use serde_json::json;

use super::*;

/// The MV7's own source, running or not.
fn mv7(state: &str) -> Value {
    json!({
        "id": 30, "type": "PipeWire:Interface:Node",
        "info": {"state": state, "props": {
            "media.class": "Audio/Source", "node.name": "alsa_input.mv7", "device.id": 70,
        }},
    })
}

/// Chrome recording, as a call does.
fn chrome() -> Value {
    json!({
        "id": 40, "type": "PipeWire:Interface:Node",
        "info": {"state": "running", "props": {
            "media.class": "Stream/Input/Audio", "application.process.binary": "chrome",
        }},
    })
}

fn the_mv7() -> BTreeSet<String> {
    BTreeSet::from(["alsa_input.mv7".to_owned()])
}

#[test]
fn a_microphone_running_with_no_call_is_not_heard() {
    let mut follower = Follower::default();
    follower.take(&json!([mv7("running")]));
    let now = Instant::now();
    assert_eq!(
        follower.stepped(now),
        None,
        "the MV7 was heard with no call on"
    );
    assert_eq!(
        follower.stepped(now + Duration::from_secs(10)),
        None,
        "the MV7 was heard with no call on"
    );
}

#[test]
fn a_call_on_hears_the_microphone_it_runs_until_its_over() {
    let mut follower = Follower::default();
    let start = Instant::now();
    follower.take(&json!([mv7("running"), chrome()]));
    assert_eq!(
        follower.stepped(start),
        None,
        "a moment's recording was taken for a call"
    );
    let on = start + Duration::from_secs(3);
    assert_eq!(
        follower.stepped(on),
        Some(the_mv7()),
        "the call's microphone wasn't heard"
    );
    assert_eq!(
        follower.stepped(on),
        None,
        "the same microphones were told twice"
    );
    follower.take(&json!([{"id": 40, "info": null}]));
    let gone = on + Duration::from_secs(1);
    assert_eq!(
        follower.stepped(gone),
        None,
        "a call a moment away was taken as over"
    );
    assert_eq!(
        follower.stepped(gone + Duration::from_secs(6)),
        Some(BTreeSet::new()),
        "the call's microphone was still heard after it ended"
    );
}

#[test]
fn a_microphone_running_once_a_call_is_on_is_heard_then() {
    let mut follower = Follower::default();
    let start = Instant::now();
    follower.take(&json!([mv7("suspended"), chrome()]));
    follower.stepped(start);
    let on = start + Duration::from_secs(3);
    assert_eq!(
        follower.stepped(on),
        None,
        "a suspended microphone was heard"
    );
    follower.take(&json!([mv7("running")]));
    assert_eq!(
        follower.stepped(on + Duration::from_secs(1)),
        Some(the_mv7()),
        "the microphone the call woke wasn't heard"
    );
}
