//! The sound server's devices as heard, and the records drawn from them:
//! each output and input as one `device` record, saying what the shell
//! draws its row from as PipeWire says it. Words are the shell's, so a
//! record holds the server's names and figures, never a sentence.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// A change the sound server said, as the daemon takes it in.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// A node's properties, as it first appears or as they change.
    Node {
        id: u32,
        properties: BTreeMap<String, String>,
    },
    /// How loud a node is set: each channel's volume, and whether it's
    /// muted, where its Props said them.
    Levels {
        id: u32,
        volumes: Option<Vec<f32>>,
        muted: Option<bool>,
    },
    /// A node or the default metadata going away, by its id.
    Gone { id: u32 },
    /// The default metadata saying a key, or no key for every key at
    /// once, its value none where it's let go of.
    Chose {
        key: Option<String>,
        value: Option<String>,
    },
}

/// One device as heard: its node's properties, and how loud it's set.
#[derive(Clone, Debug, Default, PartialEq)]
struct Node {
    properties: BTreeMap<String, String>,
    /// Each channel's volume as PipeWire holds it, a linear share.
    volumes: Vec<f32>,
    muted: bool,
}

/// Every device heard, by its node's id, and the output and input chosen
/// by the node names the default metadata gives them.
#[derive(Debug, Default)]
pub struct Devices {
    nodes: BTreeMap<u32, Node>,
    output: Option<String>,
    input: Option<String>,
}

/// Which way a node goes by its media class: an output (true), an input
/// (false), or neither, as a program's stream or a video node is.
pub fn direction(class: &str) -> Option<bool> {
    match class {
        "Audio/Sink" => Some(true),
        "Audio/Source" | "Audio/Source/Virtual" => Some(false),
        _ => None,
    }
}

/// The node name a default's value gives, `{"name": "…"}`.
fn named(value: Option<&str>) -> Option<String> {
    let said: Value = serde_json::from_str(value?).ok()?;
    said["name"].as_str().map(str::to_owned)
}

/// A volume as the shell shows it: the mean of each channel's cube root,
/// since PipeWire holds each share cubed, to four places so a share read
/// back the same never reads as a change.
fn shown(volumes: &[f32]) -> f64 {
    if volumes.is_empty() {
        return 0.0;
    }
    let count = f64::from(u32::try_from(volumes.len()).unwrap_or(u32::MAX));
    let roots = volumes.iter().map(|share| f64::from(share.max(0.0)).cbrt());
    (roots.sum::<f64>() / count * 10_000.0).round() / 10_000.0
}

impl Devices {
    /// Takes in one change, answering whether any record could have
    /// changed by it.
    pub fn hear(&mut self, change: Change) -> bool {
        match change {
            Change::Node { id, properties } => {
                let class = properties.get("media.class").map_or("", String::as_str);
                if direction(class).is_none() {
                    return self.nodes.remove(&id).is_some();
                }
                self.nodes.entry(id).or_default().properties = properties;
                true
            }
            Change::Levels { id, volumes, muted } => {
                let Some(node) = self.nodes.get_mut(&id) else {
                    return false;
                };
                if let Some(volumes) = volumes {
                    node.volumes = volumes;
                }
                if let Some(muted) = muted {
                    node.muted = muted;
                }
                true
            }
            Change::Gone { id } => self.nodes.remove(&id).is_some(),
            Change::Chose { key, value } => {
                match key.as_deref() {
                    Some("default.audio.sink") => self.output = named(value.as_deref()),
                    Some("default.audio.source") => self.input = named(value.as_deref()),
                    Some(_) => return false,
                    None => {
                        self.output = None;
                        self.input = None;
                    }
                }
                true
            }
        }
    }

    /// Lets go of everything heard, as the sound server going away does.
    pub fn forget(&mut self) {
        *self = Devices::default();
    }

    /// Each device as a record, keyed by its node's name, which a device
    /// keeps from one sound server to the next where its id doesn't.
    pub fn records(&self) -> Vec<Value> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| {
                let name = node.properties.get("node.name")?;
                let class = node.properties.get("media.class")?;
                let sink = direction(class)?;
                let chosen = if sink { &self.output } else { &self.input };
                let said = |key: &str| node.properties.get(key).cloned().unwrap_or_default();
                Some(json!({
                    "key": format!("device:{name}"),
                    "kind": "device",
                    "id": id,
                    "name": name,
                    "description": said("node.description"),
                    "nick": said("node.nick"),
                    "sink": sink,
                    "volume": shown(&node.volumes),
                    "volumes": node.volumes,
                    "muted": node.muted,
                    "chosen": chosen.as_deref() == Some(name.as_str()),
                    "properties": node.properties,
                }))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, name: &str, class: &str) -> Change {
        Change::Node {
            id,
            properties: BTreeMap::from([
                ("node.name".to_owned(), name.to_owned()),
                ("node.description".to_owned(), format!("{name}, described")),
                ("media.class".to_owned(), class.to_owned()),
            ]),
        }
    }

    fn chose(key: &str, name: &str) -> Change {
        Change::Chose {
            key: Some(key.to_owned()),
            value: Some(format!(r#"{{"name":"{name}"}}"#)),
        }
    }

    fn keys(devices: &Devices) -> Vec<String> {
        devices
            .records()
            .iter()
            .map(|record| record["key"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn outputs_and_inputs_are_devices_and_streams_arent() {
        let mut devices = Devices::default();
        assert!(devices.hear(node(40, "speakers", "Audio/Sink")));
        assert!(devices.hear(node(41, "microphone", "Audio/Source")));
        assert!(!devices.hear(node(42, "spotify", "Stream/Output/Audio")));
        assert_eq!(keys(&devices), ["device:speakers", "device:microphone"]);
        let records = devices.records();
        assert_eq!(records[0]["sink"], json!(true));
        assert_eq!(records[1]["sink"], json!(false));
        assert_eq!(records[0]["description"], json!("speakers, described"));
    }

    #[test]
    fn a_volume_is_said_as_the_shell_shows_it() {
        let mut devices = Devices::default();
        devices.hear(node(40, "speakers", "Audio/Sink"));
        assert!(devices.hear(Change::Levels {
            id: 40,
            volumes: Some(vec![0.125, 0.125]),
            muted: Some(true),
        }));
        let record = &devices.records()[0];
        assert_eq!(record["volume"], json!(0.5));
        assert_eq!(record["volumes"], json!([0.125, 0.125]));
        assert_eq!(record["muted"], json!(true));
        // A mute alone keeps the volumes said before it.
        devices.hear(Change::Levels {
            id: 40,
            volumes: None,
            muted: Some(false),
        });
        let record = &devices.records()[0];
        assert_eq!((record["volume"].clone(), record["muted"].clone()), (json!(0.5), json!(false)));
        assert!(!devices.hear(Change::Levels {
            id: 99,
            volumes: Some(vec![1.0]),
            muted: None,
        }));
    }

    #[test]
    fn the_chosen_are_those_the_default_metadata_names() {
        let mut devices = Devices::default();
        devices.hear(node(40, "speakers", "Audio/Sink"));
        devices.hear(node(43, "headphones", "Audio/Sink"));
        devices.hear(node(41, "microphone", "Audio/Source"));
        devices.hear(chose("default.audio.sink", "headphones"));
        devices.hear(chose("default.audio.source", "microphone"));
        assert!(!devices.hear(chose("default.video.source", "camera")));
        let chosen = |devices: &Devices| -> Vec<bool> {
            devices
                .records()
                .iter()
                .map(|record| record["chosen"].as_bool().unwrap())
                .collect()
        };
        // By id: the speakers 40, the microphone 41, the headphones 43.
        assert_eq!(keys(&devices)[2], "device:headphones");
        assert_eq!(chosen(&devices), [false, true, true]);
        devices.hear(Change::Chose {
            key: None,
            value: None,
        });
        assert_eq!(chosen(&devices), [false, false, false]);
    }

    #[test]
    fn a_node_gone_or_no_longer_a_device_is_let_go() {
        let mut devices = Devices::default();
        devices.hear(node(40, "speakers", "Audio/Sink"));
        devices.hear(node(41, "microphone", "Audio/Source"));
        assert!(devices.hear(Change::Gone { id: 40 }));
        assert!(!devices.hear(Change::Gone { id: 40 }));
        assert!(devices.hear(node(41, "microphone", "Video/Source")));
        assert_eq!(keys(&devices), Vec::<String>::new());
        devices.hear(node(44, "speakers", "Audio/Sink"));
        devices.forget();
        assert_eq!(keys(&devices), Vec::<String>::new());
    }
}
