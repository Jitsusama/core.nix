//! Programs' streams as the sound server says them, and the links that
//! join each to a device or to a filter in front of one: each a `stream`
//! record saying what it is, how loud it's set, whether it's muted and
//! which nodes it's linked to, by their names, as Sound.qml read them from
//! Quickshell's link groups, and the format it moves its audio in. Words
//! are the shell's, so a record holds the server's names and figures,
//! never a sentence.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::devices::shown;
use crate::levels::Format;

/// Whether a node is a program's stream by its media class: one playing
/// (true), one recording (false), or neither.
pub fn playing(class: &str) -> Option<bool> {
    match class {
        "Stream/Output/Audio" => Some(true),
        "Stream/Input/Audio" => Some(false),
        _ => None,
    }
}

/// One stream as heard: its node's properties, and how loud it's set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stream {
    pub properties: BTreeMap<String, String>,
    /// Each channel's volume as PipeWire holds it, a linear share.
    pub volumes: Vec<f32>,
    pub muted: bool,
    /// The format it moves its audio in, while it's set up to move any.
    pub format: Option<Format>,
    /// Whether it's running, kept only of a filter's half, since nothing
    /// else of a stream's state is shown.
    pub running: bool,
}

impl Stream {
    fn said(&self, key: &str) -> &str {
        self.properties.get(key).map_or("", String::as_str)
    }

    /// Whether it's a program's own, as Sound.qml's `own` said: the peaks
    /// the shell reads to draw a Level open a stream of their own, as
    /// another program's level meter does, and a filter's own halves carry
    /// the link group they share; none of those plays or records anything.
    fn own(&self) -> bool {
        self.said("node.name") != "quickshell-peak-monitor"
            && self.said("node.link-group").is_empty()
            && self.said("stream.monitor") != "true"
    }
}

/// Every stream heard, by its node's id, and every link, by its own id,
/// as the node it goes out of and the node it goes into.
#[derive(Debug, Default)]
pub struct Streams {
    streams: BTreeMap<u32, Stream>,
    links: BTreeMap<u32, (u32, u32)>,
}

impl Streams {
    /// Takes in a node's properties, answering whether a stream changed:
    /// a node that's no stream, or no longer one, is let go.
    pub fn node(&mut self, id: u32, properties: BTreeMap<String, String>) -> bool {
        let class = properties.get("media.class").map_or("", String::as_str);
        if playing(class).is_none() {
            return self.streams.remove(&id).is_some();
        }
        self.streams.entry(id).or_default().properties = properties;
        true
    }

    /// Takes in how loud a stream is set, where it's one.
    pub fn levels(&mut self, id: u32, volumes: Option<Vec<f32>>, muted: Option<bool>) -> bool {
        let Some(stream) = self.streams.get_mut(&id) else {
            return false;
        };
        if let Some(volumes) = volumes {
            stream.volumes = volumes;
        }
        if let Some(muted) = muted {
            stream.muted = muted;
        }
        true
    }

    /// Takes in the format a stream moves its audio in, answering whether
    /// it changed.
    pub fn format(&mut self, id: u32, format: Option<Format>) -> bool {
        let Some(stream) = self.streams.get_mut(&id) else {
            return false;
        };
        std::mem::replace(&mut stream.format, format) != stream.format
    }

    /// Takes in whether a filter's half is running, answering whether it
    /// changed; any other node's state is let be, so a program pausing
    /// works nothing out again.
    pub fn running(&mut self, id: u32, running: bool) -> bool {
        let Some(stream) = self.streams.get_mut(&id) else {
            return false;
        };
        if stream.said("node.link-group").is_empty() {
            return false;
        }
        std::mem::replace(&mut stream.running, running) != running
    }

    /// The nodes a filter's halves in this link group are linked to while
    /// they run, as Sound.qml's `flowing` read an active link: a playing
    /// half's link goes into its device, a recording half's comes out of
    /// it.
    pub fn flows(&self, group: &str) -> Vec<u32> {
        let halves: Vec<u32> = self
            .streams
            .iter()
            .filter(|(_, stream)| stream.running && stream.said("node.link-group") == group)
            .map(|(id, _)| *id)
            .collect();
        self.links
            .values()
            .filter_map(
                |(from, to)| match (halves.contains(from), halves.contains(to)) {
                    (true, false) => Some(*to),
                    (false, true) => Some(*from),
                    _ => None,
                },
            )
            .collect()
    }

    /// Takes in a link, from the node it goes out of to the one it goes
    /// into.
    pub fn link(&mut self, id: u32, from: u32, to: u32) -> bool {
        self.links.insert(id, (from, to)) != Some((from, to))
    }

    /// Lets go of a stream or a link gone, answering whether it was one.
    pub fn gone(&mut self, id: u32) -> bool {
        let stream = self.streams.remove(&id).is_some();
        self.links.retain(|_, (from, to)| *from != id && *to != id);
        self.links.remove(&id).is_some() || stream
    }

    /// The stream of this id, as an act finds it.
    pub fn stream(&self, id: u32) -> Option<&Stream> {
        self.streams.get(&id)
    }

    /// Each program's own stream as a record, keyed by its id, saying the
    /// nodes it's linked to by the names `named` gives their ids: a
    /// stream playing, the nodes it goes into, and one recording, those it
    /// comes out of.
    pub fn records<'a>(&self, named: impl Fn(u32) -> Option<&'a str>) -> Vec<Value> {
        self.streams
            .iter()
            .filter(|(_, stream)| stream.own())
            .filter_map(|(id, stream)| {
                let sink = playing(stream.said("media.class"))?;
                let ends: Vec<&str> = self
                    .links
                    .values()
                    .filter_map(|(from, to)| match (sink, *from == *id, *to == *id) {
                        (true, true, _) => named(*to),
                        (false, _, true) => named(*from),
                        _ => None,
                    })
                    .collect();
                Some(json!({
                    "key": format!("stream:{id}"),
                    "kind": "stream",
                    "id": id,
                    "name": stream.said("node.name"),
                    "sink": sink,
                    "volume": shown(&stream.volumes),
                    "volumes": stream.volumes,
                    "muted": stream.muted,
                    "properties": stream.properties,
                    "ends": ends,
                    "format": stream.format.as_ref().map(|format| {
                        json!({"rate": format.rate, "sample": format.sample})
                    }),
                }))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(name: &str, class: &str, more: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut properties = BTreeMap::from([
            ("node.name".to_owned(), name.to_owned()),
            ("media.class".to_owned(), class.to_owned()),
        ]);
        for (key, value) in more {
            properties.insert((*key).to_owned(), (*value).to_owned());
        }
        properties
    }

    fn names(id: u32) -> Option<&'static str> {
        match id {
            40 => Some("speakers"),
            41 => Some("microphone"),
            _ => None,
        }
    }

    // A stream playing names the device it goes into, one recording the
    // device it comes out of; a link to a node that's no device names
    // nothing.
    #[test]
    fn a_stream_says_the_devices_its_linked_to() {
        let mut streams = Streams::default();
        streams.node(90, stream("spotify", "Stream/Output/Audio", &[]));
        streams.node(91, stream("meet", "Stream/Input/Audio", &[]));
        streams.link(100, 90, 40);
        streams.link(101, 41, 91);
        streams.link(102, 90, 77);
        let records = streams.records(names);
        let ends: Vec<&Value> = records.iter().map(|record| &record["ends"]).collect();
        assert_eq!(ends, [&json!(["speakers"]), &json!(["microphone"])]);
        assert_eq!(records[0]["key"], "stream:90");
        assert_eq!(records[0]["sink"], true);
        assert_eq!(records[1]["sink"], false);
    }

    // The shell's own peaks, a level meter's monitor and a filter's own
    // halves record and play nothing, so they're no program's.
    #[test]
    fn only_a_programs_own_streams_are_records() {
        let mut streams = Streams::default();
        let output = "Stream/Output/Audio";
        let input = "Stream/Input/Audio";
        streams.node(90, stream("quickshell-peak-monitor", input, &[]));
        streams.node(91, stream("meter", input, &[("stream.monitor", "true")]));
        streams.node(
            92,
            stream("tuning", output, &[("node.link-group", "filter-1")]),
        );
        streams.node(93, stream("spotify", output, &[]));
        let records = streams.records(names);
        let keys: Vec<&Value> = records.iter().map(|record| &record["key"]).collect();
        assert_eq!(keys, [&json!("stream:93")]);
    }

    // A stream's levels are its own, read as the shell shows them; a
    // stream gone takes its links with it.
    #[test]
    fn a_stream_says_how_loud_it_is_until_its_gone() {
        let mut streams = Streams::default();
        assert!(
            !streams.node(40, stream("speakers", "Audio/Sink", &[])),
            "a device is no stream"
        );
        streams.node(90, stream("spotify", "Stream/Output/Audio", &[]));
        streams.link(100, 90, 40);
        assert!(
            streams.levels(90, Some(vec![0.125, 0.125]), Some(true)),
            "a stream's levels are heard"
        );
        let records = streams.records(names);
        assert_eq!(records[0]["volume"], json!(0.5));
        assert_eq!(records[0]["muted"], json!(true));
        assert!(streams.gone(90), "a stream goes");
        assert_eq!(streams.records(names), Vec::<Value>::new());
        assert!(!streams.gone(100), "its link went with it");
    }

    // A stream says the format it moves its audio in while it has one,
    // and is told of it again only as it changes.
    #[test]
    fn a_stream_says_its_format_while_it_has_one() {
        let mut streams = Streams::default();
        streams.node(90, stream("spotify", "Stream/Output/Audio", &[]));
        let float = Format {
            rate: 48_000,
            sample: "F32LE".into(),
        };
        assert!(streams.format(90, Some(float.clone())), "a format is heard");
        assert!(
            !streams.format(90, Some(float)),
            "the same format again changes nothing"
        );
        let said = &streams.records(names)[0]["format"];
        assert_eq!(said, &json!({"rate": 48_000, "sample": "F32LE"}));
        assert!(streams.format(90, None), "a format let go of is heard");
        assert_eq!(streams.records(names)[0]["format"], Value::Null);
        assert!(!streams.format(77, None), "no stream, no format");
    }

    // A filter's sound passes to its device only while its half runs; a
    // program's stream running or not changes nothing, since none of its
    // state is shown.
    #[test]
    fn a_filter_flows_to_its_device_only_while_its_half_runs() {
        let mut streams = Streams::default();
        let group = [("node.link-group", "filter-1")];
        streams.node(92, stream("tuning", "Stream/Output/Audio", &group));
        streams.node(93, stream("spotify", "Stream/Output/Audio", &[]));
        streams.link(100, 92, 40);
        streams.link(101, 93, 40);
        assert!(
            streams.flows("filter-1").is_empty(),
            "a half not running passes nothing"
        );
        assert!(streams.running(92, true), "a half starting is heard");
        assert_eq!(streams.flows("filter-1"), [40]);
        assert!(
            streams.flows("filter-2").is_empty(),
            "another group's flows nowhere"
        );
        assert!(!streams.running(92, true), "running again changes nothing");
        assert!(
            !streams.running(93, true),
            "a program's stream state is let be"
        );
        assert!(streams.running(92, false), "a half stopping is heard");
        assert!(
            streams.flows("filter-1").is_empty(),
            "a half stopped passes nothing"
        );
    }
}
