//! What ties a machine to the other three parts, so a view can draw each
//! machine whole: the mycelium's cells on it, the network it reaches
//! through, the code it holds, its own figures, and what Joel can do with
//! it. The machine is the bridge between the others (Joel, 2026-11-17),
//! and its own module because each tie changes for its owner's reasons
//! and none of them changes what makes a machine worth a look.

use serde_json::{Value, json};

/// A gigabyte, the finest a machine's bytes are said in.
const GIGABYTE: f64 = 1e9;

/// The finest a share of a processor is said in: a tenth.
const TENTHS: f64 = 10.0;

/// The longest name mycelium-link takes for a machine.
const LONGEST_NAME: usize = 64;

/// The ties of one machine, as the node says it (`said`), with its
/// sessions from `mycelium health` (`Null` for a health read that lists
/// none on it), the network's `net --json` and this machine's
/// `repos --json`, each where it was read. A figure is said only as finely
/// as a view says it, so a watch is told a machine again only when what
/// it would say of it moves, not at every reading's change.
pub fn ties(
    said: &Value,
    sessions: Option<&Value>,
    net: Option<&Value>,
    repos: Option<&Value>,
) -> Value {
    let name = said.get("name").and_then(Value::as_str).unwrap_or("");
    // Whether the node reaches it, so a view can set one out of reach
    // apart rather than read its words, and when it was last heard, said
    // only then: a machine in reach is measured every reading, and saying
    // that would tell every watch of it again each time.
    let reached = said.get("state").and_then(Value::as_str) != Some("unreachable");
    let heard = if reached {
        Value::Null
    } else {
        said.get("measured_at").cloned().unwrap_or(Value::Null)
    };
    json!({
        "reached": reached,
        "heard_at": heard,
        "cells": sessions.map(cells),
        "network": net.map(|net| places(name, net)),
        "code": repos.and_then(|repos| code(name, repos)),
        "figures": figures(said),
        "acts": acts(name),
    })
}

/// What Joel can do with a machine: the launcher's link to its row, and
/// the commands, for a name mycelium-link would take (its `named`); none
/// for any other, since the name is the node's and a page offers both.
fn acts(name: &str) -> Value {
    let shaped = name.len() <= LONGEST_NAME
        && name.starts_with(|first: char| first.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || matches!(each, '.' | '_' | '-'));
    if !shaped {
        return Value::Null;
    }
    json!({
        "link": format!("mycelium:machine/{name}?from=system"),
        "commands": [
            format!("mycelium sprout --on {name}"),
            format!("mycelium update {name}"),
        ],
    })
}

/// `value` said in whole `unit`s, or as it was where it isn't a number.
fn coarse(value: Option<&Value>, unit: f64) -> Value {
    match value.and_then(Value::as_f64) {
        // A figure here is a count of bytes, a share or a rate, all well
        // inside what an f64 carries whole.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a machine's bytes and rates are positive and far below 2^53"
        )]
        Some(number) if unit >= 1.0 => json!(((number / unit).round() * unit) as u64),
        Some(number) => json!((number / unit).round() * unit),
        None => value.cloned().unwrap_or(Value::Null),
    }
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
        .filter(|device| {
            device
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|said| !name.is_empty() && said.eq_ignore_ascii_case(name))
        })
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
                "room_mbps": coarse(device.pointer("/path/down/room_mbps"), 1.0),
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

/// A machine's own figures, as the node says them, its bytes in whole
/// gigabytes and its processor's share in tenths, and its names plain, as
/// they reach a terminal; what it didn't say, or a name with nothing plain
/// left in it, is said as unknown.
fn figures(said: &Value) -> Value {
    let at = |pointer: &str| said.pointer(pointer).cloned().unwrap_or(Value::Null);
    let named = |pointer: &str| match said.pointer(pointer) {
        Some(Value::String(name)) => {
            let name = crate::plain::words(name);
            if name.is_empty() {
                Value::Null
            } else {
                Value::from(name)
            }
        }
        other => other.cloned().unwrap_or(Value::Null),
    };
    let bytes = |pointer: &str| coarse(said.pointer(pointer), GIGABYTE);
    json!({
        "model": named("/model_name"),
        "system": named("/system"),
        "os": named("/os"),
        "disk": bytes("/disk"),
        "disk_free": bytes("/disk_free"),
        "cores": at("/cpu/cores"),
        "cpu_busy": coarse(said.pointer("/cpu/busy"), 1.0 / TENTHS),
        "memory": bytes("/memory"),
        "memory_available": bytes("/room/memory_available"),
        "charge": at("/power/charge"),
        "on_battery": at("/power/on_battery"),
        "updated_at": at("/updated/at"),
        "update_step": step(said.pointer("/updated/step")),
        "update_why": super::why::update_why(said.get("updated").unwrap_or(&Value::Null)),
    })
}

/// The step a machine's last update reached, when it reads as one: a short
/// word of lowercase letters, since it reaches a terminal as it is.
fn step(said: Option<&Value>) -> Value {
    said.and_then(Value::as_str)
        .filter(|step| {
            (1..=STEP_AT_MOST).contains(&step.len())
                && step
                    .chars()
                    .all(|each| each.is_ascii_lowercase() || each == '-')
        })
        .map_or(Value::Null, Value::from)
}

/// The longest a step's name may be.
const STEP_AT_MOST: usize = 32;

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
            {"id": "mac:3c", "name": "Laptop",
             "path": {"through": ["switch:9"], "down": {"room_mbps": 941.2}},
             "detail": {"eero": {"ethernet": true}}},
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
        let unlisted = super::ties(&laptop(), Some(&Value::Null), None, None);
        assert_eq!(
            unlisted["cells"],
            json!({"busy": 0, "idle": 0, "ended": 0}),
            "a health read that lists none on it says none"
        );
    }

    #[test]
    fn a_machine_reaches_the_network_through_each_device_named_for_it() {
        let ties = ties(&laptop(), None, Some(&net()), None);
        assert_eq!(
            ties["network"],
            json!([{"device": "mac:a8", "by": "2.4 GHz", "point": "Hall",
                    "bars": 5, "room_mbps": 184},
                   {"device": "mac:3c", "by": "wired", "point": "switch:9",
                    "bars": null, "room_mbps": 941}]),
            "each device named for it, whatever its case, in the network's order, a point \
             the network doesn't list said by its id"
        );
        let mut desk = laptop();
        desk["name"] = json!("desk");
        let wired = super::ties(&desk, None, Some(&net()), None);
        assert_eq!(
            wired["network"],
            json!([{"device": "mac:d0", "by": "wired", "point": "Den",
                    "bars": null, "room_mbps": 370}]),
            "a wire has no bars to say"
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
    fn an_update_step_is_carried_only_when_it_reads_as_one() {
        let step_of = |step: Value| {
            let mut machine = laptop();
            machine["updated"] = json!({"step": step});
            super::ties(&machine, None, None, None)["figures"]["update_step"].clone()
        };
        assert_eq!(
            step_of(json!("refused")),
            json!("refused"),
            "a step is carried"
        );
        for (step, what) in [
            (
                json!("\u{1b}[2Jdone"),
                "a step that would act on a terminal",
            ),
            (json!("Done"), "a step not in update-to-main's form"),
            (
                json!("x".repeat(STEP_AT_MOST + 1)),
                "a step too long to be one",
            ),
            (json!(4), "a step that isn't words"),
        ] {
            assert_eq!(step_of(step), Value::Null, "{what} is not carried");
        }
    }

    #[test]
    fn a_machine_carries_its_own_figures() {
        let ties = ties(&laptop(), None, None, None);
        assert_eq!(
            ties["figures"],
            json!({"model": "Example 14", "system": "linux",
                   "os": "NixOS 26.11 (Zokor)",
                   "disk": 1_023_000_000_000_u64, "disk_free": 169_000_000_000_u64,
                   "cores": 16, "cpu_busy": 0.9,
                   "memory": 67_000_000_000_u64, "memory_available": 30_000_000_000_u64,
                   "charge": 79, "on_battery": false,
                   "updated_at": "2026-11-17T22:45:18Z", "update_step": "done",
                   "update_why": null})
        );
        let mut odd = laptop();
        odd["model_name"] = json!("\u{1b}[2JExample\u{202e} 14");
        odd["os"] = json!("NixOS\t26.11");
        let odd = super::ties(&odd, None, None, None);
        assert_eq!(
            (&odd["figures"]["model"], &odd["figures"]["os"]),
            (&json!("Example 14"), &json!("NixOS 26.11")),
            "a machine's names reach a terminal plain"
        );
        let mut blank = laptop();
        blank["model_name"] = json!("\u{1b}[2J");
        blank["os"] = json!("\u{202e}");
        let blank = super::ties(&blank, None, None, None);
        assert_eq!(
            (&blank["figures"]["model"], &blank["figures"]["os"]),
            (&Value::Null, &Value::Null),
            "a name with nothing plain in it is said as unknown, not as nothing"
        );
        let mut mac = laptop();
        mac["updated"] = json!({"at": "2026-11-17T23:00:00Z", "step": "refused",
                                "why": "the Mac is over its room; it waits."});
        assert_eq!(
            super::ties(&mac, None, None, None)["figures"]["update_why"],
            json!("the Mac is over its room, it waits"),
            "an update put off carries why, as one clause of the machine's line"
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
    fn a_machine_out_of_reach_says_so_and_when_it_was_last_heard() {
        let mut here = laptop();
        here["measured_at"] = json!("2026-11-23T22:45:09Z");
        let ties = ties(&here, None, None, None);
        assert_eq!(ties["reached"], json!(true));
        assert_eq!(
            ties["heard_at"],
            Value::Null,
            "one in reach is measured every reading, so when isn't said"
        );
        let away = super::ties(
            &json!({"name": "spare", "state": "unreachable",
                    "measured_at": "2026-11-23T19:02:00Z"}),
            None,
            None,
            None,
        );
        assert_eq!(away["reached"], json!(false));
        assert_eq!(away["heard_at"], json!("2026-11-23T19:02:00Z"));
        let never = super::ties(
            &json!({"name": "spare", "state": "unreachable"}),
            None,
            None,
            None,
        );
        assert_eq!(never["reached"], json!(false));
        assert_eq!(
            never["heard_at"],
            Value::Null,
            "one never heard has no time to say"
        );
    }

    #[test]
    fn a_figure_moving_less_than_it_is_said_leaves_the_ties_as_they_were() {
        let mut later = laptop();
        later["disk_free"] = json!(169_301_000_000_u64);
        later["cpu"]["busy"] = json!(0.91);
        later["room"]["memory_available"] = json!(30_112_000_000_u64);
        let mut moved = net();
        moved["devices"][2]["path"]["down"]["room_mbps"] = json!(183.9);
        moved["devices"][2]["detail"]["eero"]["signal_dbm"] = json!(-41);
        assert_eq!(
            ties(&laptop(), None, Some(&net()), None),
            ties(&later, None, Some(&moved), None),
            "a watch is told a machine again only when what it says of it moves"
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
        for misshaped in [
            "",
            "-h",
            ".hidden",
            "lap top",
            "lap;rm",
            "lap/top",
            "\u{e9}t\u{e9}",
        ] {
            let mut machine = laptop();
            machine["name"] = json!(misshaped);
            let refused = super::ties(&machine, None, None, None);
            assert_eq!(
                refused["acts"],
                Value::Null,
                "{misshaped:?} names no machine"
            );
        }
        let long = "a".repeat(65);
        let mut machine = laptop();
        machine["name"] = json!(long);
        assert_eq!(super::ties(&machine, None, None, None)["acts"], Value::Null);
        let nameless = super::ties(&json!({"state": "ready"}), None, None, None);
        assert_eq!(
            nameless["acts"],
            Value::Null,
            "a machine with no name has no acts"
        );
    }
}
