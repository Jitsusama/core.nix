//! The sound server's devices as heard, and the records drawn from them:
//! each output and input as one `device` record, saying what the shell
//! draws its row from as PipeWire says it. Words are the shell's, so a
//! record holds the server's names and figures, never a sentence.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::levels::Port;

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
    /// A card's properties, as it first appears or as they change: for
    /// a Bluetooth one, what `BlueZ` says it is.
    Card {
        id: u32,
        properties: BTreeMap<String, String>,
    },
    /// A node, a card or the default metadata going away, by its id.
    Gone { id: u32 },
    /// A card saying which of its routes plays one of its profile's
    /// devices, as its Route param does.
    Route { card: u32, index: i32, device: i32 },
    /// A card saying one route it has, a port, as its `EnumRoute` param
    /// does.
    Offer { card: u32, port: Port },
    /// A card saying the profile it's in, as its Profile param does.
    Profile { card: u32, index: i32 },
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
    /// The route each card plays each of its profile's devices through,
    /// by the card's id and the profile's device.
    routes: BTreeMap<(u32, i32), i32>,
    /// What each card says it is, by its id: its form factor and icon,
    /// as `BlueZ` gives a Bluetooth one's.
    forms: BTreeMap<u32, String>,
    /// Each route each card has, by the card's id and the route's index:
    /// the profile's devices it can play, and whether nothing is plugged
    /// into it.
    offers: BTreeMap<(u32, i32), Port>,
    /// The profile each card is in, by its id.
    profiles: BTreeMap<u32, i32>,
}

/// One device as an act finds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Device<'a> {
    pub id: u32,
    pub name: &'a str,
    pub sink: bool,
    /// Each channel's volume as PipeWire holds it, a linear share.
    pub volumes: &'a [f32],
    /// The card playing it, the route it plays it through and the
    /// profile's device it is, where a card plays it.
    pub route: Option<(u32, i32, i32)>,
    /// The ports it can be played through or listened with, by index.
    pub ports: Vec<i32>,
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
            Change::Card { id, properties } => {
                let said = |key: &str| properties.get(key).map_or("", String::as_str);
                let form = format!(
                    "{} {}",
                    said("device.form-factor"),
                    said("device.icon-name")
                );
                let form = form.trim().to_owned();
                if self.forms.get(&id).map_or("", String::as_str) == form {
                    return false;
                }
                self.forms.insert(id, form);
                self.nodes.values().any(|node| {
                    node.properties
                        .get("device.id")
                        .is_some_and(|card| *card == id.to_string())
                })
            }
            Change::Gone { id } => {
                self.routes.retain(|(card, _), _| *card != id);
                self.offers.retain(|(card, _), _| *card != id);
                self.profiles.remove(&id);
                self.forms.remove(&id);
                self.nodes.remove(&id).is_some()
            }
            // A route moves the port its device says is chosen.
            Change::Route {
                card,
                index,
                device,
            } => self.routes.insert((card, device), index) != Some(index),
            Change::Offer { card, port } => {
                self.offers.insert((card, port.index), port.clone()) != Some(port)
            }
            Change::Profile { card, index } => self.profiles.insert(card, index) != Some(index),
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

    /// The device a record's key names, as an act finds it.
    pub fn device(&self, key: &str) -> Option<Device<'_>> {
        let name = key.strip_prefix("device:")?;
        let (id, node) = self
            .nodes
            .iter()
            .find(|(_, node)| node.properties.get("node.name").is_some_and(|n| n == name))?;
        let said = |key: &str| node.properties.get(key).map(String::as_str);
        let card: Option<u32> = said("device.id").and_then(|card| card.parse().ok());
        let device: Option<i32> = said("card.profile.device").and_then(|d| d.parse().ok());
        let route = card.zip(device).and_then(|(card, device)| {
            let index = self.routes.get(&(card, device))?;
            Some((card, *index, device))
        });
        Some(Device {
            id: *id,
            name: said("node.name")?,
            sink: direction(said("media.class")?)?,
            volumes: &node.volumes,
            route,
            ports: self.ports(node).iter().map(|port| port.index).collect(),
        })
    }

    /// The card a node is one of the devices of, and which of its
    /// profile's devices it is.
    fn card_device(node: &Node) -> Option<(u32, i32)> {
        let said = |key: &str| node.properties.get(key).map_or("", String::as_str);
        let card: u32 = said("device.id").parse().ok()?;
        let device: i32 = said("card.profile.device").parse().ok()?;
        Some((card, device))
    }

    /// The ports a node's card offers it, as `wpctl` and Sound.qml read
    /// them: those going its way that can play its profile's device, in
    /// the profile the card is in. A device no card plays has none.
    fn ports(&self, node: &Node) -> Vec<&Port> {
        let Some((card, device)) = Devices::card_device(node) else {
            return Vec::new();
        };
        let sink = node
            .properties
            .get("media.class")
            .map(String::as_str)
            .and_then(direction);
        let profile = self.profiles.get(&card);
        self.offers
            .range((card, i32::MIN)..=(card, i32::MAX))
            .map(|(_, port)| port)
            .filter(|port| port.devices.contains(&device))
            .filter(|port| port.sink.is_none_or(|way| Some(way) == sink))
            .filter(|port| {
                profile.is_none_or(|profile| {
                    port.profiles.is_empty() || port.profiles.contains(profile)
                })
            })
            .collect()
    }

    /// What a node's card says it is, or "" where it says nothing.
    fn form(&self, node: &Node) -> &str {
        let card = node
            .properties
            .get("device.id")
            .and_then(|id| id.parse().ok());
        card.and_then(|card| self.forms.get(&card))
            .map_or("", String::as_str)
    }

    /// Whether a node's card says nothing is plugged in where it plays:
    /// every route that can play its profile's device says so, and one
    /// at least does. A device no card plays, as a USB or Bluetooth one
    /// with no routes, is never unplugged.
    fn unplugged(&self, node: &Node) -> bool {
        let ports = self.ports(node);
        !ports.is_empty() && ports.iter().all(|port| port.unplugged())
    }

    /// Each port a node's card offers it, saying the one chosen.
    fn ported(&self, node: &Node) -> Vec<Value> {
        let chosen = Devices::card_device(node).and_then(|at| self.routes.get(&at));
        self.ports(node)
            .iter()
            .map(|port| {
                json!({
                    "index": port.index,
                    "name": port.name,
                    "available": port.available,
                    "chosen": chosen == Some(&port.index),
                })
            })
            .collect()
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
                    "form": self.form(node),
                    "unplugged": self.unplugged(node),
                    "ports": self.ported(node),
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
        assert!(
            devices.hear(node(40, "speakers", "Audio/Sink")),
            "an output is a device"
        );
        assert!(
            devices.hear(node(41, "microphone", "Audio/Source")),
            "an input is a device"
        );
        assert!(
            !devices.hear(node(42, "spotify", "Stream/Output/Audio")),
            "a program's stream isn't a device"
        );
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
        let heard = devices.hear(Change::Levels {
            id: 40,
            volumes: Some(vec![0.125, 0.125]),
            muted: Some(true),
        });
        assert!(heard, "a device's levels change its record");
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
        assert_eq!(
            (record["volume"].clone(), record["muted"].clone()),
            (json!(0.5), json!(false))
        );
        let heard = devices.hear(Change::Levels {
            id: 99,
            volumes: Some(vec![1.0]),
            muted: None,
        });
        assert!(!heard, "levels of a node not heard change nothing");
    }

    #[test]
    fn the_chosen_are_those_the_default_metadata_names() {
        let mut devices = Devices::default();
        devices.hear(node(40, "speakers", "Audio/Sink"));
        devices.hear(node(43, "headphones", "Audio/Sink"));
        devices.hear(node(41, "microphone", "Audio/Source"));
        devices.hear(chose("default.audio.sink", "headphones"));
        devices.hear(chose("default.audio.source", "microphone"));
        assert!(
            !devices.hear(chose("default.video.source", "camera")),
            "a video default chooses no sound device"
        );
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
    fn an_act_finds_the_route_a_card_plays_its_device_through() {
        let mut devices = Devices::default();
        let Change::Node { mut properties, .. } = node(40, "speakers", "Audio/Sink") else {
            unreachable!("node says a node");
        };
        properties.insert("device.id".into(), "7".into());
        properties.insert("card.profile.device".into(), "1".into());
        devices.hear(Change::Node { id: 40, properties });
        let routed = |devices: &Devices| devices.device("device:speakers").and_then(|d| d.route);
        assert_eq!(routed(&devices), None);
        let route = Change::Route {
            card: 7,
            index: 2,
            device: 1,
        };
        assert!(
            devices.hear(route.clone()),
            "a route moves its device's port"
        );
        assert!(!devices.hear(route), "said again, it moves nothing");
        assert_eq!(routed(&devices), Some((7, 2, 1)));
        devices.hear(Change::Gone { id: 7 });
        assert_eq!(routed(&devices), None);
        assert_eq!(devices.device("device:headphones"), None);
    }

    // What a card says it is, as BlueZ says a Bluetooth one is, reaches
    // the records of the devices it plays, and only a change to it moves
    // them.
    #[test]
    fn a_device_says_what_its_card_is() {
        let mut devices = Devices::default();
        let Change::Node { mut properties, .. } = node(40, "headphones", "Audio/Sink") else {
            unreachable!("node says a node");
        };
        properties.insert("device.id".into(), "7".into());
        devices.hear(Change::Node { id: 40, properties });
        let card = |form: &str| Change::Card {
            id: 7,
            properties: BTreeMap::from([
                ("device.form-factor".to_owned(), form.to_owned()),
                ("device.icon-name".to_owned(), "audio-headphones".to_owned()),
            ]),
        };
        assert!(
            devices.hear(card("headphone")),
            "a card saying what it is moves its device"
        );
        assert_eq!(devices.records()[0]["form"], "headphone audio-headphones");
        assert!(
            !devices.hear(card("headphone")),
            "said again, it moves nothing"
        );
        let other = Change::Card {
            id: 8,
            properties: BTreeMap::new(),
        };
        assert!(!devices.hear(other), "a card playing no device moves none");
        devices.hear(Change::Gone { id: 7 });
        assert_eq!(devices.records()[0]["form"], "");
    }

    // The laptop's card says its headphones' jack has nothing in it while
    // none are plugged in, and can't tell of its speakers, which are
    // always there.
    #[test]
    fn a_device_says_whether_anything_is_plugged_in_where_it_plays() {
        let mut devices = Devices::default();
        for (id, name, device) in [(40, "speakers", "0"), (41, "headphones", "1")] {
            let Change::Node { mut properties, .. } = node(id, name, "Audio/Sink") else {
                unreachable!("node says a node");
            };
            properties.insert("device.id".into(), "7".into());
            properties.insert("card.profile.device".into(), device.into());
            devices.hear(Change::Node { id, properties });
        }
        devices.hear(node(42, "usb-desk", "Audio/Sink"));
        let offer = |index: i32, unplugged: bool| Change::Offer {
            card: 7,
            port: Port {
                index,
                devices: vec![index],
                available: if unplugged { "no" } else { "unknown" },
                ..Port::default()
            },
        };
        let unplugged = |devices: &Devices| -> Vec<bool> {
            let records = devices.records();
            records
                .iter()
                .map(|record| record["unplugged"] == true)
                .collect()
        };
        assert_eq!(unplugged(&devices), [false, false, false]);
        assert!(
            devices.hear(offer(0, false)),
            "a route offered moves its device"
        );
        assert!(
            devices.hear(offer(1, true)),
            "an empty jack moves its device"
        );
        assert_eq!(unplugged(&devices), [false, true, false]);
        assert!(
            !devices.hear(offer(1, true)),
            "said again, it moves nothing"
        );
        devices.hear(offer(1, false));
        assert_eq!(unplugged(&devices), [false, false, false]);
        devices.hear(offer(1, true));
        devices.hear(Change::Gone { id: 7 });
        assert_eq!(unplugged(&devices), [false, false, false]);
    }

    // The laptop's analog output plays its speakers or its headphones,
    // one port each way, and offers a microphone port too, which isn't
    // the output's; a port of another profile isn't either.
    #[test]
    fn a_device_says_the_ports_its_card_offers_it() {
        let mut devices = Devices::default();
        let Change::Node { mut properties, .. } = node(40, "analog", "Audio/Sink") else {
            unreachable!("node says a node");
        };
        properties.insert("device.id".into(), "7".into());
        properties.insert("card.profile.device".into(), "0".into());
        devices.hear(Change::Node { id: 40, properties });
        let port = |index: i32, sink: bool, name: &str, available: &'static str| Change::Offer {
            card: 7,
            port: Port {
                index,
                sink: Some(sink),
                name: name.into(),
                devices: vec![0],
                profiles: vec![if index == 3 { 2 } else { 1 }],
                available,
            },
        };
        devices.hear(port(0, true, "Speakers", "unknown"));
        devices.hear(port(1, true, "Headphones", "no"));
        devices.hear(port(2, false, "Microphone", "unknown"));
        devices.hear(port(3, true, "Line Out", "yes"));
        devices.hear(Change::Profile { card: 7, index: 1 });
        assert!(
            devices.hear(Change::Route {
                card: 7,
                index: 0,
                device: 0
            }),
            "a port chosen moves its device"
        );
        assert_eq!(
            devices.records()[0]["ports"],
            json!([
                {"index": 0, "name": "Speakers", "available": "unknown", "chosen": true},
                {"index": 1, "name": "Headphones", "available": "no", "chosen": false},
            ])
        );
        let found = devices.device("device:analog").map(|device| device.ports);
        assert_eq!(found, Some(vec![0, 1]));
    }

    #[test]
    fn a_node_gone_or_no_longer_a_device_is_let_go() {
        let mut devices = Devices::default();
        devices.hear(node(40, "speakers", "Audio/Sink"));
        devices.hear(node(41, "microphone", "Audio/Source"));
        assert!(
            devices.hear(Change::Gone { id: 40 }),
            "a device gone is let go"
        );
        assert!(
            !devices.hear(Change::Gone { id: 40 }),
            "a device gone twice changes nothing"
        );
        assert!(
            devices.hear(node(41, "microphone", "Video/Source")),
            "a device become a video node is let go"
        );
        assert_eq!(keys(&devices), Vec::<String>::new());
        devices.hear(node(44, "speakers", "Audio/Sink"));
        devices.forget();
        assert_eq!(keys(&devices), Vec::<String>::new());
    }
}
