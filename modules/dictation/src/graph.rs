//! PipeWire's graph as `pw-dump --monitor` prints it, a JSON array of the
//! whole graph and then one of each change, kept so a call's two sides can
//! be found in it: what the call app records from, and what it plays.
//!
//! Each object pw-dump prints replaces the one held by its id, and one
//! printed with no info is gone.

use std::collections::HashMap;

use serde_json::Value;

/// The graph's objects by id.
#[derive(Default, Debug)]
pub struct Graph {
    objects: HashMap<u64, Value>,
}

/// A link between two ports, by ids.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Link {
    pub id: u64,
    pub output: u64,
    pub input: u64,
}

impl Graph {
    /// Takes in one of pw-dump's arrays.
    pub fn take(&mut self, array: &Value) {
        for object in array.as_array().into_iter().flatten() {
            let Some(id) = object["id"].as_u64() else {
                continue;
            };
            if object["info"].is_null() {
                self.objects.remove(&id);
            } else {
                self.objects.insert(id, object.clone());
            }
        }
    }

    fn of_type<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = (u64, &'a Value)> + 'a {
        self.objects
            .iter()
            .filter(move |(_, object)| object["type"] == kind)
            .map(|(id, object)| (*id, object))
    }

    /// The call app's recording, as modules/calls counts one: the lowest
    /// id, so the same one is found each time while several record.
    pub fn call(&self) -> Option<u64> {
        self.of_type("PipeWire:Interface:Node")
            .filter(|(_, node)| calls::records(node))
            .map(|(id, _)| id)
            .min()
    }

    /// The node named `name`, as a recorder of ours is.
    pub fn named(&self, name: &str) -> Option<u64> {
        self.of_type("PipeWire:Interface:Node")
            .find(|(_, node)| node["info"]["props"]["node.name"] == name)
            .map(|(id, _)| id)
    }

    /// Every link, by its ports.
    pub fn links(&self) -> Vec<Link> {
        self.of_type("PipeWire:Interface:Link")
            .filter_map(|(id, link)| {
                let info = &link["info"];
                Some(Link {
                    id,
                    output: info["output-port-id"].as_u64()?,
                    input: info["input-port-id"].as_u64()?,
                })
            })
            .collect()
    }

    /// A node's ports going `direction`, "input" or "output", leaving out a
    /// sink's monitor ports, which repeat what it plays.
    pub fn ports(&self, node: u64, direction: &str) -> Vec<u64> {
        let mut ports: Vec<u64> = self
            .of_type("PipeWire:Interface:Port")
            .filter(|(_, port)| {
                let info = &port["info"];
                info["direction"] == direction
                    && info["props"]["node.id"].as_u64() == Some(node)
                    && info["props"]["port.monitor"] != true
            })
            .map(|(id, _)| id)
            .collect();
        ports.sort_unstable();
        ports
    }

    /// The ports `node` records from: each output port linked into it.
    pub fn feeding(&self, node: u64) -> Vec<u64> {
        let inputs = self.ports(node, "input");
        let mut feeding: Vec<u64> = self
            .links()
            .into_iter()
            .filter(|link| inputs.contains(&link.input))
            .map(|link| link.output)
            .collect();
        feeding.sort_unstable();
        feeding.dedup();
        feeding
    }

    /// The output ports of what the program recording at `call` plays: the
    /// playback streams of its own process, or, where that process plays
    /// none, of its program, as a call app that plays from a helper process
    /// does. A notice or music, played by another program, isn't the call.
    pub fn playing(&self, call: u64) -> Vec<u64> {
        let Some(recording) = self.objects.get(&call) else {
            return Vec::new();
        };
        let props = &recording["info"]["props"];
        let streams = |key: &str| -> Vec<u64> {
            let wanted = &props[key];
            if wanted.is_null() {
                return Vec::new();
            }
            let mut ports: Vec<u64> = self
                .of_type("PipeWire:Interface:Node")
                .filter(|(_, node)| {
                    let theirs = &node["info"]["props"];
                    theirs["media.class"] == "Stream/Output/Audio" && theirs[key] == *wanted
                })
                .flat_map(|(id, _)| self.ports(id, "output"))
                .collect();
            ports.sort_unstable();
            ports
        };
        let own = streams("application.process.id");
        if own.is_empty() {
            streams("application.process.binary")
        } else {
            own
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(id: u64, class: &str, binary: &str, pid: u64, state: &str) -> Value {
        json!({
            "id": id, "type": "PipeWire:Interface:Node",
            "info": {"state": state, "props": {
                "media.class": class, "application.process.binary": binary,
                "application.process.id": pid, "node.name": format!("{binary}-{id}"),
            }},
        })
    }

    fn port(id: u64, node: u64, direction: &str) -> Value {
        json!({
            "id": id, "type": "PipeWire:Interface:Port",
            "info": {"direction": direction, "props": {"node.id": node}},
        })
    }

    fn link(id: u64, output: u64, input: u64) -> Value {
        json!({
            "id": id, "type": "PipeWire:Interface:Link",
            "info": {"output-port-id": output, "input-port-id": input},
        })
    }

    /// A headset, a call app recording from it and playing two streams, and
    /// a music player.
    fn desk() -> Graph {
        let mut graph = Graph::default();
        graph.take(&json!([
            node(30, "Audio/Source", "", 0, "running"),
            port(31, 30, "output"),
            port(32, 30, "output"),
            node(40, "Stream/Input/Audio", "chrome", 900, "running"),
            port(41, 40, "input"),
            port(42, 40, "input"),
            link(43, 31, 41),
            link(44, 32, 42),
            node(50, "Stream/Output/Audio", "chrome", 900, "running"),
            port(51, 50, "output"),
            port(52, 50, "output"),
            node(55, "Stream/Output/Audio", "chrome", 900, "running"),
            port(56, 55, "output"),
            node(60, "Stream/Output/Audio", "spotify", 700, "running"),
            port(61, 60, "output"),
        ]));
        graph
    }

    #[test]
    fn the_call_records_from_the_ports_linked_into_it() {
        let graph = desk();
        assert_eq!(graph.call(), Some(40));
        assert_eq!(graph.feeding(40), vec![31, 32]);
    }

    #[test]
    fn the_call_plays_its_own_streams_and_not_anothers() {
        assert_eq!(desk().playing(40), vec![51, 52, 56]);
    }

    #[test]
    fn a_call_playing_from_another_process_is_matched_by_its_program() {
        let mut graph = Graph::default();
        graph.take(&json!([
            node(40, "Stream/Input/Audio", "zoom", 900, "running"),
            node(50, "Stream/Output/Audio", "zoom", 901, "running"),
            port(51, 50, "output"),
            node(60, "Stream/Output/Audio", "spotify", 700, "running"),
            port(61, 60, "output"),
        ]));
        assert_eq!(graph.playing(40), vec![51]);
    }

    #[test]
    fn what_goes_is_forgotten() {
        let mut graph = desk();
        graph.take(&json!([{"id": 44, "info": null}, {"id": 55, "info": null}]));
        assert_eq!(graph.feeding(40), vec![31]);
        assert_eq!(graph.links().len(), 1);
        graph.take(&json!([{"id": 40, "info": null}]));
        assert_eq!(graph.call(), None);
    }

    #[test]
    fn a_node_is_found_by_its_name_and_a_monitor_port_is_no_output() {
        let mut graph = desk();
        let mut monitor = port(33, 30, "output");
        monitor["info"]["props"]["port.monitor"] = json!(true);
        graph.take(&json!([monitor]));
        assert_eq!(graph.ports(30, "output"), vec![31, 32]);
        assert_eq!(graph.named("chrome-40"), Some(40));
        assert_eq!(graph.named("nobody"), None);
    }
}
