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

/// Chrome recording from `from`'s one port, as pw-dump prints the wiring.
fn wired_from(from: u64) -> Value {
    json!([
        {"id": 31, "type": "PipeWire:Interface:Port",
         "info": {"direction": "output", "props": {"node.id": 30}}},
        {"id": 36, "type": "PipeWire:Interface:Port",
         "info": {"direction": "output", "props": {"node.id": 35}}},
        {"id": 35, "type": "PipeWire:Interface:Node", "info": {"state": "running", "props": {
            "media.class": "Audio/Source", "node.name": "noise-cancelled.alsa_input.mv7",
        }}},
        {"id": 41, "type": "PipeWire:Interface:Port",
         "info": {"direction": "input", "props": {"node.id": 40}}},
        {"id": 45, "type": "PipeWire:Interface:Link",
         "info": {"output-port-id": from, "input-port-id": 41}},
    ])
}

#[test]
fn the_call_is_said_once_on_and_again_when_what_it_records_from_changes() {
    let mut follower = Follower::default();
    let start = Instant::now();
    follower.take(&json!([mv7("running"), chrome()]));
    follower.take(&wired_from(31));
    follower.stepped(start);
    assert_eq!(
        follower.recording(),
        None,
        "a moment's recording was said as a call"
    );
    follower.stepped(start + Duration::from_secs(3));
    assert_eq!(
        follower.recording().as_deref(),
        Some("the call in chrome records from alsa_input.mv7"),
        "the call's app and microphone weren't said"
    );
    assert_eq!(follower.recording(), None, "the same call was said twice");
    follower.take(&wired_from(36));
    follower.stepped(start + Duration::from_secs(4));
    assert_eq!(
        follower.recording().as_deref(),
        Some("the call in chrome records from noise-cancelled.alsa_input.mv7"),
        "the call moving to the twin wasn't said"
    );
    follower.take(&json!([{"id": 45, "info": null}]));
    follower.stepped(start + Duration::from_secs(5));
    assert_eq!(
        follower.recording(),
        None,
        "a call let go of its microphone was said to record from nothing"
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
