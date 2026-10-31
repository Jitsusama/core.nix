//! What counts as a call: a program recording from a microphone, as
//! PipeWire's graph shows it in `pw-dump`'s JSON, that isn't one of
//! PipeWire's own filters or a program that records without calling. One
//! counts as a call once it has recorded for two seconds, so a browser
//! opening the microphone for a moment is none.
//!
//! pause-for-calls follows calls as they come and go with this, and
//! a-call-is-on (src/main.rs) asks once, for whatever mustn't cut one.

use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

/// How long a program has to record before it counts as a call.
pub const STARTS_AFTER: Duration = Duration::from_secs(2);

/// Programs that record from a microphone without being a call: PipeWire's
/// own filters, dictation's recorder, the camera program's videos, and the
/// shell's level meters.
pub const NOT_CALLS: [&str; 6] = [
    "pipewire",
    "wireplumber",
    "pw-record",
    "pw-cat",
    "ffmpeg",
    "quickshell",
];

/// Whether a node, as pw-dump prints it, is a program recording from a
/// microphone right now.
pub fn records(node: &Value) -> bool {
    let info = &node["info"];
    let props = &info["props"];
    let text = |key: &str| props[key].as_str().unwrap_or_default();
    node["type"] == "PipeWire:Interface:Node"
        && text("media.class") == "Stream/Input/Audio"
        && info["state"] == "running"
        && props["stream.capture.sink"] != true
        && !text("application.process.binary").is_empty()
        && !NOT_CALLS.contains(&text("application.process.binary"))
}

/// The nodes recording from a microphone, kept up to date from pw-dump's
/// arrays: each object it prints is the whole of what changed, and one
/// printed with no info is gone.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Recording(HashSet<u64>);

impl Recording {
    /// Takes in one of pw-dump's arrays.
    pub fn take(&mut self, array: &Value) {
        for node in array.as_array().into_iter().flatten() {
            let Some(id) = node["id"].as_u64() else {
                continue;
            };
            if records(node) {
                self.0.insert(id);
            } else if node["info"].is_null() || node["type"] == "PipeWire:Interface:Node" {
                self.0.remove(&id);
            }
        }
    }

    /// Whether anything records.
    pub fn any(&self) -> bool {
        !self.0.is_empty()
    }

    /// Whether a node that records here records in `later` too, the same
    /// node, as a call that went on does.
    pub fn goes_on_in(&self, later: &Recording) -> bool {
        !self.0.is_disjoint(&later.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(id: u64, binary: &str, state: &str) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Node",
            "info": {
                "state": state,
                "props": {
                    "media.class": "Stream/Input/Audio",
                    "application.process.binary": binary,
                }
            }
        })
    }

    fn of(array: &Value) -> Recording {
        let mut recording = Recording::default();
        recording.take(array);
        recording
    }

    #[test]
    fn a_call_app_recording_is_seen_and_goes_when_it_stops() {
        let mut recording = Recording::default();
        recording.take(&json!([node(40, "slack", "running")]));
        assert!(recording.any());
        recording.take(&json!([node(40, "slack", "idle")]));
        assert!(!recording.any(), "an idle stream isn't recording");
        recording.take(&json!([node(41, "chrome", "running")]));
        recording.take(&json!([{ "id": 41, "info": null }]));
        assert!(!recording.any(), "a node gone isn't recording");
    }

    #[test]
    fn what_records_without_being_a_call_is_left_out() {
        let mut recording = Recording::default();
        recording.take(&json!([
            node(1, "pipewire", "running"),
            node(2, "pw-record", "running"),
            node(3, "ffmpeg", "running"),
            node(4, "", "running"),
        ]));
        let mut monitor = node(5, "pavucontrol", "running");
        monitor["info"]["props"]["stream.capture.sink"] = json!(true);
        recording.take(&json!([monitor]));
        let mut playing = node(6, "spotify", "running");
        playing["info"]["props"]["media.class"] = json!("Stream/Output/Audio");
        recording.take(&json!([playing]));
        assert!(!recording.any());
    }

    #[test]
    fn another_kind_of_object_leaves_a_node_be() {
        let mut recording = Recording::default();
        recording.take(&json!([node(7, "zoom", "running")]));
        recording.take(&json!([{ "id": 7, "type": "PipeWire:Interface:Link", "info": {} }]));
        assert!(recording.any());
    }

    #[test]
    fn only_the_same_node_recording_later_goes_on() {
        let first = of(&json!([node(8, "zoom", "running")]));
        assert!(first.goes_on_in(&of(&json!([node(8, "zoom", "running")]))));
        assert!(
            !first.goes_on_in(&of(&json!([node(9, "chrome", "running")]))),
            "another node is another moment of recording"
        );
        assert!(!first.goes_on_in(&Recording::default()));
    }
}
