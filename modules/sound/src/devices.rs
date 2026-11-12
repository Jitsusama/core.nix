//! The sound server's devices as heard, and the records drawn from them:
//! each output and input as one `device` record, saying what the shell
//! draws its row from as PipeWire says it. Words are the shell's, so a
//! record holds the server's names and figures, never a sentence.

use std::collections::BTreeMap;

use pipewire::node::NodeState;
use serde_json::{Value, json};

use crate::levels::{Format, Port};
use crate::streams::Streams;

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
    /// Which channel each of a node's volumes is of, by SPA's position
    /// ids, where its Props said them.
    Channels { id: u32, channels: Vec<u32> },
    /// The format a node moves its audio in, as its Format param says
    /// it, or none once it isn't set up to move any.
    Format { id: u32, format: Option<Format> },
    /// A card's properties, as it first appears or as they change: for
    /// a Bluetooth one, what `BlueZ` says it is.
    Card {
        id: u32,
        properties: BTreeMap<String, String>,
    },
    /// A node, a card, a link or the default metadata going away, by its
    /// id.
    Gone { id: u32 },
    /// A link, from the node it goes out of to the one it goes into.
    Link { id: u32, from: u32, to: u32 },
    /// What a node is doing, as its info says it, in `PipeWire`'s words
    /// (`said_state`).
    State { id: u32, state: &'static str },
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
    /// The default metadata saying where a program's stream is sent, its
    /// `target.object`, or none where it's let go of and the stream
    /// follows the default; no stream lets go of every one's, as the
    /// metadata going away does.
    Sent {
        stream: Option<u32>,
        target: Option<String>,
    },
}

impl Change {
    /// What a node's info says, its properties before its state: a first
    /// info says both, and a state told first reaches a node not yet
    /// heard, which a device left suspended never says again.
    pub fn info(
        id: u32,
        state: Option<&'static str>,
        properties: Option<BTreeMap<String, String>>,
    ) -> Vec<Change> {
        let state = state.map(|state| Change::State { id, state });
        let node = properties.map(|properties| Change::Node { id, properties });
        node.into_iter().chain(state).collect()
    }
}

/// What a stream sent nowhere is said to be sent to, as `WirePlumber` says
/// it besides letting the key go.
const NOWHERE: [&str; 2] = ["", "-1"];

/// One device as heard: its node's properties, and how loud it's set.
#[derive(Clone, Debug, Default, PartialEq)]
struct Node {
    properties: BTreeMap<String, String>,
    /// Each channel's volume as PipeWire holds it, a linear share.
    volumes: Vec<f32>,
    /// Which channel each volume is of, by SPA's position ids.
    channels: Vec<u32>,
    muted: bool,
    /// What it's doing, in `PipeWire`'s words, "" until its info says.
    state: &'static str,
    /// The format the graph moves its audio in, while it's set up to.
    format: Option<Format>,
}

/// A node's state in `PipeWire`'s own words, as `pw_node_state_as_string`
/// says them and Quickshell's node reads them: running, idle, suspended.
pub fn said_state(state: &NodeState) -> &'static str {
    match state {
        NodeState::Creating => "creating",
        NodeState::Suspended => "suspended",
        NodeState::Idle => "idle",
        NodeState::Running => "running",
        NodeState::Error(_) => "error",
    }
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
    /// Where each program's stream is sent, by the stream's id: a
    /// device's node name or its object serial, as `WirePlumber` takes
    /// either. A stream not here follows the default.
    sent: BTreeMap<u32, String>,
    /// Programs' streams, and the links joining them to devices.
    pub streams: Streams,
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
pub(crate) fn shown(volumes: &[f32]) -> f64 {
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
                    let stream = self.streams.node(id, properties);
                    return self.nodes.remove(&id).is_some() || stream;
                }
                self.nodes.entry(id).or_default().properties = properties;
                true
            }
            Change::Levels { id, volumes, muted } => {
                let Some(node) = self.nodes.get_mut(&id) else {
                    return self.streams.levels(id, volumes, muted);
                };
                if let Some(volumes) = volumes {
                    node.volumes = volumes;
                }
                if let Some(muted) = muted {
                    node.muted = muted;
                }
                true
            }
            // Only a device's are said, in its Channels.
            Change::Channels { id, channels } => self.nodes.get_mut(&id).is_some_and(|node| {
                std::mem::replace(&mut node.channels, channels) != node.channels
            }),
            // A device's is said in its drawer, a stream's in its
            // program's.
            Change::Format { id, format } => match self.nodes.get_mut(&id) {
                Some(node) => std::mem::replace(&mut node.format, format) != node.format,
                None => self.streams.format(id, format),
            },
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
                // A stream gone takes where it was sent with it.
                let sent = self.sent.remove(&id).is_some();
                let stream = self.streams.gone(id);
                self.nodes.remove(&id).is_some() || sent || stream
            }
            Change::Link { id, from, to } => self.streams.link(id, from, to),
            // A filter's half running is sound passing through it.
            Change::State { id, state } => match self.nodes.get_mut(&id) {
                Some(node) => std::mem::replace(&mut node.state, state) != state,
                None => self.streams.running(id, state == "running"),
            },
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
            Change::Sent { stream, target } => {
                let Some(stream) = stream else {
                    let had = !self.sent.is_empty();
                    self.sent.clear();
                    return had;
                };
                match target.filter(|target| !NOWHERE.contains(&target.as_str())) {
                    Some(target) => self.sent.insert(stream, target.clone()) != Some(target),
                    None => self.sent.remove(&stream).is_some(),
                }
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

    /// Where each program's stream is sent, as one record: each target by
    /// the stream's id, which the shell matches to the streams it lists.
    pub fn targets(&self) -> Value {
        let targets: serde_json::Map<String, Value> = self
            .sent
            .iter()
            .map(|(stream, target)| (stream.to_string(), json!(target)))
            .collect();
        json!({"key": "targets:default", "kind": "targets", "targets": targets})
    }

    /// Each device as a record, keyed by its node's name, which a device
    /// keeps from one sound server to the next where its id doesn't, then
    /// each program's stream, saying the devices it's linked to by name
    /// and where it's sent, its `target`, "" while it follows the default.
    pub fn records(&self) -> Vec<Value> {
        let mut records = self.devices();
        let named = |id: u32| {
            let node = self.nodes.get(&id)?;
            node.properties.get("node.name").map(String::as_str)
        };
        records.extend(self.streams.records(named).into_iter().map(|mut record| {
            let id = record["id"].as_u64().and_then(|id| u32::try_from(id).ok());
            let sent = id.and_then(|id| self.sent.get(&id));
            record["target"] = json!(sent.map_or("", String::as_str));
            record
        }));
        records
    }

    /// The devices a filter's sound passes to or from now, by their
    /// names: those its other half, a stream in its link group, is linked
    /// to while it runs. Any other node's are none.
    fn flows(&self, node: &Node) -> Vec<&str> {
        let group = node.properties.get("node.link-group");
        group.map_or_else(Vec::new, |group| {
            self.streams
                .flows(group)
                .iter()
                .filter_map(|id| self.nodes.get(id)?.properties.get("node.name"))
                .map(String::as_str)
                .collect()
        })
    }

    fn devices(&self) -> Vec<Value> {
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
                    "channels": node.channels,
                    "muted": node.muted,
                    "chosen": chosen.as_deref() == Some(name.as_str()),
                    "properties": node.properties,
                    "form": self.form(node),
                    "unplugged": self.unplugged(node),
                    "ports": self.ported(node),
                    "flows": self.flows(node),
                    "state": node.state,
                    "format": node.format.as_ref().map(|format| {
                        json!({"rate": format.rate, "sample": format.sample})
                    }),
                }))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;
