//! What counts as a call: a program recording from a microphone, as
//! PipeWire's graph shows it in `pw-dump`'s JSON, that isn't one of
//! PipeWire's own filters or a program that records without calling. One
//! counts as a call once it has recorded for two seconds, so a browser
//! opening the microphone for a moment is none.
//!
//! pause-for-calls and call-transcripts follow calls as they come and go
//! with this ([`Call`]), and a-call-is-on (src/main.rs) asks once, for
//! whatever mustn't cut one. On a Mac it asks Core Audio which processes
//! record ([`MacRecording`]), and counts them the same way.

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// How long a program has to record before it counts as a call.
pub const STARTS_AFTER: Duration = Duration::from_secs(2);

/// How long the recording has to stop before the call counts as over, so
/// a call app reconnecting is still the one call.
pub const ENDS_AFTER: Duration = Duration::from_secs(5);

/// A call starting or ending.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Change {
    Started,
    Ended,
}

/// Where a call stands, with when it last changed, for what follows calls
/// as they come and go.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Call {
    Off,
    Starting(Instant),
    On,
    Ending(Instant),
}

impl Call {
    /// Moves on from whether anything is recording now, saying when the
    /// call starts or ends.
    pub fn step(&mut self, recording: bool, now: Instant) -> Option<Change> {
        let (next, change) = match (*self, recording) {
            (Call::Off, true) => (Call::Starting(now), None),
            (Call::Starting(_) | Call::Off, false) => (Call::Off, None),
            (Call::Starting(since), true) if now - since >= STARTS_AFTER => {
                (Call::On, Some(Change::Started))
            }
            (Call::On, false) => (Call::Ending(now), None),
            (Call::Ending(_) | Call::On, true) => (Call::On, None),
            (Call::Ending(since), false) if now - since >= ENDS_AFTER => {
                (Call::Off, Some(Change::Ended))
            }
            (waiting, _) => (waiting, None),
        };
        *self = next;
        change
    }

    /// How long until the call starts or ends if nothing changes, for the
    /// wait between pw-dump's arrays.
    pub fn due(&self, now: Instant) -> Option<Duration> {
        match *self {
            Call::Starting(since) => Some((since + STARTS_AFTER).saturating_duration_since(now)),
            Call::Ending(since) => Some((since + ENDS_AFTER).saturating_duration_since(now)),
            Call::Off | Call::On => None,
        }
    }
}

/// Programs that record from a microphone without being a call: PipeWire's
/// own filters, dictation's recorder, the camera program's videos, the
/// shell's level meters, and the noise-cancelled microphones, which hear a
/// microphone only for the program recording from them.
pub const NOT_CALLS: [&str; 7] = [
    "noise-cancelled",
    "pipewire",
    "wireplumber",
    "pw-record",
    "pw-cat",
    "ffmpeg",
    "quickshell",
];

/// Whether a node, as pw-dump prints it, is a stream recording from a
/// microphone right now, whatever program it is.
fn captures(node: &Value) -> bool {
    let info = &node["info"];
    let props = &info["props"];
    node["type"] == "PipeWire:Interface:Node"
        && props["media.class"] == "Stream/Input/Audio"
        && info["state"] == "running"
        && props["stream.capture.sink"] != true
}

/// The client a node belongs to, the object pw-dump prints as
/// `PipeWire:Interface:Client`.
pub fn client_of(node: &Value) -> Option<u64> {
    node["info"]["props"]["client.id"].as_u64()
}

/// The program a node is part of, empty when nothing says. A stream made
/// through PipeWire's own API, as Tuple's is, names its program only on its
/// client, so the client's name stands in for the node's. A program Nix
/// wrapped is named for its wrapper, `.quickshell-wrapped`, so that is
/// taken back to the program's own name.
pub fn program(node: &Value, client: Option<&Value>) -> String {
    let binary = |object: &Value| {
        object["info"]["props"]["application.process.binary"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    let named = Some(binary(node))
        .filter(|name| !name.is_empty())
        .or_else(|| client.map(binary))
        .unwrap_or_default();
    let unwrapped = named
        .strip_suffix("-wrapped")
        .map_or(named.as_str(), |name| {
            name.strip_prefix('.').unwrap_or(name)
        });
    unwrapped.to_owned()
}

/// Whether a node, as pw-dump prints it beside its client, is a program
/// recording from a microphone right now.
pub fn records(node: &Value, client: Option<&Value>) -> bool {
    let program = program(node, client);
    captures(node) && !program.is_empty() && !NOT_CALLS.contains(&program.as_str())
}

/// The nodes recording from a microphone, kept up to date from pw-dump's
/// arrays: each object it prints is the whole of what changed, and one
/// printed with no info is gone. The clients are kept too, since a node
/// may name its program only there, and may come before its client.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Recording {
    streams: HashMap<u64, Value>,
    clients: HashMap<u64, Value>,
}

impl Recording {
    /// Takes in one of pw-dump's arrays.
    pub fn take(&mut self, array: &Value) {
        for object in array.as_array().into_iter().flatten() {
            let Some(id) = object["id"].as_u64() else {
                continue;
            };
            if object["info"].is_null() {
                self.streams.remove(&id);
                self.clients.remove(&id);
            } else if object["type"] == "PipeWire:Interface:Client" {
                self.clients.insert(id, object.clone());
            } else if captures(object) {
                self.streams.insert(id, object.clone());
            } else if object["type"] == "PipeWire:Interface:Node" {
                self.streams.remove(&id);
            }
        }
    }

    /// The nodes recording that are calling programs.
    fn calling(&self) -> HashSet<u64> {
        self.streams
            .iter()
            .filter(|(_, node)| records(node, client_of(node).and_then(|c| self.clients.get(&c))))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Whether anything records.
    pub fn any(&self) -> bool {
        !self.calling().is_empty()
    }

    /// Whether a node that records here records in `later` too, the same
    /// node, as a call that went on does.
    pub fn goes_on_in(&self, later: &Recording) -> bool {
        !self.calling().is_disjoint(&later.calling())
    }
}

/// Programs on a Mac that record from a microphone without being a call,
/// by bundle id: Siri and its notifications listening for their names,
/// the daemon answering it, and Sound Recognition, as a Mac listed them
/// on 11 November.
pub const NOT_CALLS_ON_A_MAC: [&str; 4] = [
    "com.apple.CoreSpeech",
    "com.apple.SiriNCService",
    "com.apple.assistantd",
    "com.apple.accessibility.heard",
];

/// The processes recording from a microphone on a Mac, as Core Audio lists
/// them: each one's process id and bundle id. One with no bundle id is a
/// command-line recorder, which, as a program unnamed is on Linux, is no
/// call.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct MacRecording {
    calling: HashSet<(i32, String)>,
}

impl MacRecording {
    /// Takes in a process Core Audio says records from a microphone.
    pub fn hears(&mut self, pid: i32, bundle: &str) {
        if !bundle.is_empty() && !NOT_CALLS_ON_A_MAC.contains(&bundle) {
            self.calling.insert((pid, bundle.to_owned()));
        }
    }

    /// Whether anything that calls records.
    pub fn any(&self) -> bool {
        !self.calling.is_empty()
    }

    /// Whether a process that records here records in `later` too, the
    /// same process, as a call that went on does.
    pub fn goes_on_in(&self, later: &MacRecording) -> bool {
        !self.calling.is_disjoint(&later.calling)
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

    fn native(id: u64, client: u64) -> Value {
        let mut stream = node(id, "", "running");
        stream["info"]["props"] = json!({
            "media.class": "Stream/Input/Audio",
            "node.name": "TupleAudio",
            "client.id": client,
        });
        stream
    }

    fn client(id: u64, binary: &str) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Client",
            "info": { "props": { "application.process.binary": binary } }
        })
    }

    #[test]
    fn a_native_stream_is_named_by_its_client() {
        let mut recording = Recording::default();
        recording.take(&json!([native(60, 59)]));
        assert!(!recording.any(), "a stream nothing names isn't a call yet");
        recording.take(&json!([client(59, "tuple")]));
        assert!(
            recording.any(),
            "Tuple's stream, named by its client, is a call"
        );
        recording.take(&json!([{ "id": 59, "info": null }]));
        assert!(
            !recording.any(),
            "a stream whose client went is named by nothing"
        );
    }

    #[test]
    fn a_native_stream_of_what_records_without_calling_is_left_out() {
        let of_client = |binary: &str| of(&json!([native(60, 59), client(59, binary)]));
        assert!(
            !of_client("pipewire").any(),
            "PipeWire's own filter isn't a call"
        );
        assert!(
            !of_client("pw-cat").any(),
            "a transcript's recorder isn't a call"
        );
        assert!(
            !of_client(".quickshell-wrapped").any(),
            "the shell's meter, named for Nix's wrapper, isn't a call"
        );
    }

    #[test]
    fn a_wrapped_program_is_named_for_itself() {
        assert_eq!(program(&node(1, ".zoom-wrapped", "running"), None), "zoom");
        assert_eq!(program(&node(1, "slack", "running"), None), "slack");
    }

    #[test]
    fn another_kind_of_object_leaves_a_node_be() {
        let mut recording = Recording::default();
        recording.take(&json!([node(7, "zoom", "running")]));
        recording.take(&json!([{ "id": 7, "type": "PipeWire:Interface:Link", "info": {} }]));
        assert!(recording.any());
    }

    #[test]
    fn a_call_starts_after_two_seconds_and_ends_five_after_it_stops() {
        let start = Instant::now();
        let at = |secs: f64| start + Duration::from_secs_f64(secs);
        let mut call = Call::Off;
        assert_eq!(call.step(true, at(0.0)), None);
        assert_eq!(call.due(at(0.5)), Some(Duration::from_secs_f64(1.5)));
        assert_eq!(call.step(true, at(1.9)), None);
        assert_eq!(call.step(true, at(2.0)), Some(Change::Started));
        assert_eq!(call.step(true, at(60.0)), None, "it starts once");
        assert_eq!(call.step(false, at(61.0)), None);
        assert_eq!(call.step(false, at(65.9)), None);
        assert_eq!(call.step(false, at(66.0)), Some(Change::Ended));
        assert_eq!(call, Call::Off);
    }

    #[test]
    fn a_moment_of_recording_is_no_call() {
        let start = Instant::now();
        let mut call = Call::Off;
        call.step(true, start);
        call.step(false, start + Duration::from_secs(1));
        assert_eq!(call.step(false, start + Duration::from_secs(9)), None);
        assert_eq!(call, Call::Off);
    }

    #[test]
    fn a_call_app_reconnecting_keeps_the_call() {
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        let mut call = Call::Off;
        call.step(true, at(0));
        assert_eq!(call.step(true, at(2)), Some(Change::Started));
        call.step(false, at(10));
        assert_eq!(call.step(true, at(13)), None);
        assert_eq!(call.step(false, at(20)), None);
        assert_eq!(call.step(false, at(24)), None);
        assert_eq!(call.step(false, at(25)), Some(Change::Ended));
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

    fn on_a_mac(processes: &[(i32, &str)]) -> MacRecording {
        let mut recording = MacRecording::default();
        for (pid, bundle) in processes {
            recording.hears(*pid, bundle);
        }
        recording
    }

    #[test]
    fn a_call_app_recording_on_a_mac_is_a_call() {
        assert!(
            on_a_mac(&[(53_522, "us.zoom.xos")]).any(),
            "Zoom recording is a call"
        );
        assert!(!on_a_mac(&[]).any(), "nothing recording is no call");
    }

    #[test]
    fn what_records_on_a_mac_without_calling_is_left_out() {
        let listening = on_a_mac(&[
            (1394, "com.apple.CoreSpeech"),
            (82_762, "com.apple.SiriNCService"),
            (1101, "com.apple.assistantd"),
            (3091, "com.apple.accessibility.heard"),
            (40_606, ""),
        ]);
        assert!(
            !listening.any(),
            "Siri, Sound Recognition and a command-line recorder are no call: {listening:?}"
        );
    }

    #[test]
    fn only_the_same_process_recording_later_goes_on_on_a_mac() {
        let first = on_a_mac(&[(53_703, "com.google.Chrome.helper")]);
        assert!(
            first.goes_on_in(&on_a_mac(&[(53_703, "com.google.Chrome.helper")])),
            "the same process still recording goes on"
        );
        assert!(
            !first.goes_on_in(&on_a_mac(&[(53_704, "com.google.Chrome.helper")])),
            "another process is another moment of recording"
        );
    }
}
