//! What ties a machine to the other three parts, so a view can draw each
//! machine whole: the mycelium's cells on it, the network it reaches
//! through, the code it holds, its own figures, and what Joel can do with
//! it. The machine is the bridge between the others (Joel, 2026-11-17),
//! and its own module because each tie changes for its owner's reasons
//! and none of them changes what makes a machine worth a look.

use serde_json::{Value, json};

/// The ties of one machine, as the node says it (`said`), with its
/// sessions from `mycelium health`, the network's `net --json` and this
/// machine's `repos --json`, each where it was read.
pub fn ties(
    said: &Value,
    sessions: Option<&Value>,
    net: Option<&Value>,
    repos: Option<&Value>,
) -> Value {
    let name = said.get("name").and_then(Value::as_str).unwrap_or("");
    json!({
        "cells": sessions.map(cells),
        "network": net.map(|net| places(name, net)),
        "code": repos.and_then(|repos| code(name, repos)),
        "figures": figures(said),
        "acts": {
            "link": format!("mycelium:machine/{name}?from=system"),
            "commands": [
                format!("mycelium sprout --on {name}"),
                format!("mycelium update {name}"),
            ],
        },
    })
}

/// The mycelium's cells on a machine: the busy, the idle and the ended,
/// as the node's health counts them.
fn cells(sessions: &Value) -> Value {
    let of = |key: &str| sessions.get(key).and_then(Value::as_u64).unwrap_or(0);
    json!({"busy": of("busy"), "idle": of("idle"), "ended": of("ended")})
}

/// Where a machine meets the network: each device the network names for
/// it, by its band or a wire, through the first point on its way out.
fn places(name: &str, net: &Value) -> Value {
    let devices = net
        .get("devices")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let point = |id: &str| {
        devices
            .iter()
            .find(|device| device.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|device| device.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_owned()
    };
    devices
        .iter()
        .filter(|device| device.get("name").and_then(Value::as_str) == Some(name))
        .map(|device| {
            let eero = device.pointer("/detail/eero");
            let wired = eero.and_then(|eero| eero.get("ethernet")?.as_bool()) == Some(true);
            let radio = |key: &str| {
                (!wired)
                    .then(|| eero?.get(key).cloned())
                    .flatten()
                    .unwrap_or(Value::Null)
            };
            let by = if wired { json!("wired") } else { radio("band") };
            json!({
                "device": device.get("id"),
                "by": by,
                "point": device
                    .pointer("/path/through/0")
                    .and_then(Value::as_str)
                    .map(point),
                "bars": radio("bars"),
                "signal_dbm": radio("signal_dbm"),
                "room_mbps": device.pointer("/path/down/room_mbps"),
            })
        })
        .collect()
}

/// The code a machine holds, counted, when `repos` was read on it; none
/// for another machine, since repos reads only the one it runs on.
fn code(name: &str, repos: &Value) -> Option<Value> {
    if repos.get("machine").and_then(Value::as_str) != Some(name) {
        return None;
    }
    let repositories = repos
        .get("repositories")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let checkouts: Vec<&Value> = repositories
        .iter()
        .flat_map(|repository| listed(repository, "checkouts"))
        .collect();
    let trees = checkouts
        .iter()
        .flat_map(|checkout| listed(checkout, "trees"))
        .filter(|tree| tree.get("present").and_then(Value::as_bool) != Some(false))
        .count();
    let behind = checkouts
        .iter()
        .filter(|checkout| {
            checkout
                .pointer("/head/upstream/count/behind")
                .and_then(Value::as_u64)
                .is_some_and(|behind| behind > 0)
        })
        .count();
    let changes: usize = repositories
        .iter()
        .map(|repository| listed(repository, "changes").len())
        .sum();
    Some(json!({
        "repositories": repositories.len(),
        "checkouts": checkouts.len(),
        "trees": trees,
        "behind": behind,
        "changes": changes,
    }))
}

fn listed<'a>(said: &'a Value, key: &str) -> &'a [Value] {
    said.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A machine's own figures, as the node says them; what it didn't say is
/// said as unknown.
fn figures(said: &Value) -> Value {
    let at = |pointer: &str| said.pointer(pointer).cloned().unwrap_or(Value::Null);
    json!({
        "model": at("/model_name"),
        "system": at("/system"),
        "os": at("/os"),
        "disk": at("/disk"),
        "disk_free": at("/disk_free"),
        "cores": at("/cpu/cores"),
        "cpu_busy": at("/cpu/busy"),
        "memory": at("/memory"),
        "memory_available": at("/room/memory_available"),
        "charge": at("/power/charge"),
        "on_battery": at("/power/on_battery"),
        "updated_at": at("/updated/at"),
        "update_step": at("/updated/step"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laptop() -> Value {
        json!({"name": "laptop", "state": "ready", "system": "linux",
               "os": "NixOS 26.11 (Zokor)", "model_name": "Example 14",
               "cpu": {"cores": 16, "busy": 0.89}, "memory": 66_898_141_184_u64,
               "disk": 1_023_117_623_296_u64, "disk_free": 169_270_915_072_u64,
               "power": {"on_battery": false, "charge": 79},
               "room": {"memory_available": 30_279_519_437_u64, "over": ["memory"]},
               "updated": {"at": "2026-11-17T22:45:18Z", "step": "done"}})
    }

    fn net() -> Value {
        let point = |id: &str, name: &str| json!({"id": id, "name": name, "roles": ["node"]});
        json!({"devices": [
            point("eero:1", "Hall"),
            point("eero:2", "Den"),
            {"id": "mac:a8", "name": "laptop",
             "path": {"through": ["eero:1", "eero:2", "modem"],
                      "down": {"room_mbps": 183.7}},
             "detail": {"eero": {"ethernet": false, "band": "2.4 GHz", "bars": 5,
                                 "signal_dbm": -36}}},
            {"id": "mac:d0", "name": "desk",
             "path": {"through": ["eero:2", "modem"], "down": {"room_mbps": 369.85}},
             "detail": {"eero": {"ethernet": true, "band": null, "bars": 5,
                                 "signal_dbm": 0}}},
            {"id": "mac:ff", "name": "kettle", "path": {"through": ["eero:2"]}},
        ]})
    }

    fn repos() -> Value {
        let tree = |present: bool| json!({"present": present});
        let checkout = |behind: u64, trees: Vec<Value>| {
            json!({"head": {"upstream": {"count": {"ahead": 0, "behind": behind}}},
                   "trees": trees})
        };
        json!({"machine": "laptop", "repositories": [
            {"checkouts": [checkout(100, vec![tree(true), tree(false)]),
                           checkout(0, vec![tree(true)])],
             "changes": [{}, {}]},
            {"checkouts": [{"head": {"upstream": null}, "trees": []}], "changes": [{}]},
        ]})
    }

    #[test]
    fn a_machine_carries_its_cells_as_busy_idle_and_ended() {
        let sessions = json!({"busy": 45, "idle": 27, "ended": 50, "unreachable": 0});
        let ties = ties(&laptop(), Some(&sessions), None, None);
        assert_eq!(ties["cells"], json!({"busy": 45, "idle": 27, "ended": 50}));
        let unread = super::ties(&laptop(), None, None, None);
        assert_eq!(unread["cells"], Value::Null, "unread health says no cells");
    }

    #[test]
    fn a_machine_reaches_the_network_through_each_device_named_for_it() {
        let ties = ties(&laptop(), None, Some(&net()), None);
        assert_eq!(
            ties["network"],
            json!([{"device": "mac:a8", "by": "2.4 GHz", "point": "Hall",
                    "bars": 5, "signal_dbm": -36, "room_mbps": 183.7}])
        );
        let mut desk = laptop();
        desk["name"] = json!("desk");
        let wired = super::ties(&desk, None, Some(&net()), None);
        assert_eq!(
            wired["network"],
            json!([{"device": "mac:d0", "by": "wired", "point": "Den",
                    "bars": null, "signal_dbm": null, "room_mbps": 369.85}]),
            "a wire has no bars or signal to say"
        );
        let unread = super::ties(&laptop(), None, None, None);
        assert_eq!(unread["network"], Value::Null, "unread net says no network");
    }

    #[test]
    fn this_machines_code_is_counted_and_anothers_is_not_read_from_here() {
        let ties = ties(&laptop(), None, None, Some(&repos()));
        assert_eq!(
            ties["code"],
            json!({"repositories": 2, "checkouts": 3, "trees": 2, "behind": 1,
                   "changes": 3}),
            "a tree whose folder is gone isn't counted as one"
        );
        let mut studio = laptop();
        studio["name"] = json!("studio");
        let other = super::ties(&studio, None, None, Some(&repos()));
        assert_eq!(
            other["code"],
            Value::Null,
            "repos reads only its own machine"
        );
    }

    #[test]
    fn a_machine_carries_its_own_figures() {
        let ties = ties(&laptop(), None, None, None);
        assert_eq!(
            ties["figures"],
            json!({"model": "Example 14", "system": "linux",
                   "os": "NixOS 26.11 (Zokor)",
                   "disk": 1_023_117_623_296_u64, "disk_free": 169_270_915_072_u64,
                   "cores": 16, "cpu_busy": 0.89,
                   "memory": 66_898_141_184_u64, "memory_available": 30_279_519_437_u64,
                   "charge": 79, "on_battery": false,
                   "updated_at": "2026-11-17T22:45:18Z", "update_step": "done"})
        );
        let bare = super::ties(
            &json!({"name": "spare", "state": "unreachable"}),
            None,
            None,
            None,
        );
        assert_eq!(
            bare["figures"]["disk"],
            Value::Null,
            "what the node didn't say is said as unknown"
        );
    }

    #[test]
    fn a_machines_acts_go_through_the_launcher_and_its_commands() {
        let ties = ties(&laptop(), None, None, None);
        assert_eq!(
            ties["acts"],
            json!({"link": "mycelium:machine/laptop?from=system",
                   "commands": ["mycelium sprout --on laptop", "mycelium update laptop"]})
        );
    }
}
