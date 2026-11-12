//! The broker's two asks, answered: `scope`, which moves the process
//! asking into a scope of agents' work, and `status`, which says how named
//! units stand. Each scope made and each refused is one line in the
//! journal, naming the process, its cgroup, the unit and the quest, and
//! never what the process runs.

use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::peer::Peer;
use crate::{place, systemd};

/// The most units one status may name.
const UNITS_MOST: usize = 32;

/// How long a scope's start may take to move its process before the ask
/// is refused, and how often it's looked at meanwhile.
const MOVING: Duration = Duration::from_secs(2);
const LOOKED_AT: Duration = Duration::from_millis(10);

fn refused(code: &str, message: &str) -> Value {
    json!({"error": {"code": code, "message": message}})
}

/// Moves the process asking, the peer of `stream`, into a scope of its
/// own in agents' work: its quest's slice when it runs under one or names
/// one, else `jobs-work.slice`. Answered once the kernel has it there.
pub fn scope(said: &Value, stream: &UnixStream) -> Value {
    let peer = match Peer::of(stream) {
        Ok(peer) => peer,
        Err(error) => {
            eprintln!("refused a scope to a process it couldn't name: {error}");
            return refused(
                "no-peer",
                &format!("the process asking can't be named: {error}"),
            );
        }
    };
    let cgroup = peer.cgroup.trim();
    let refuse = |code: &str, message: &str| {
        eprintln!("refused a scope to pid {} in {cgroup}: {message}", peer.pid);
        refused(code, message)
    };
    let Some(origin) = place::origin(&peer.cgroup) else {
        return refuse(
            "not-an-agent",
            "only an agent's process may ask for a scope",
        );
    };
    let asked = match said.get("quest") {
        None | Some(Value::Null) => None,
        Some(Value::String(quest)) if place::is_quest(quest) => Some(quest.as_str()),
        Some(_) => return refuse("bad-quest", "a quest is lowercase letters and digits"),
    };
    let quest = place::quest_for(&origin, asked);
    let slice = place::slice_for(quest.as_deref());
    let name = place::scope_for(quest.as_deref(), peer.pid);
    let started = systemd::session()
        .and_then(|bus| systemd::start_scope(&bus, &name, &slice, peer.pidfd.as_fd()));
    if let Err(error) = started {
        return refuse("refused", &format!("systemd didn't start {name}: {error}"));
    }
    if !moved(&peer, &name) {
        return refuse("not-moved", &format!("{name} started without it"));
    }
    let quest = quest.as_deref().unwrap_or("none");
    eprintln!(
        "scoped pid {} from {cgroup} into {name} in {slice}, quest {quest}",
        peer.pid
    );
    json!({"scope": name, "slice": slice})
}

/// Whether the process is in the scope `name` by now, its cgroup read
/// while its pidfd shows it's still the same process.
fn moved(peer: &Peer, name: &str) -> bool {
    let began = Instant::now();
    let end = format!("/{name}");
    while began.elapsed() < MOVING {
        let cgroup = std::fs::read_to_string(format!("/proc/{}/cgroup", peer.pid));
        if !peer.alive() {
            return false;
        }
        if cgroup.is_ok_and(|cgroup| cgroup.trim_end().ends_with(&end)) {
            return true;
        }
        // systemd moves the process as its job runs, a few milliseconds
        // after the call answers; the wait is bounded by MOVING.
        std::thread::sleep(LOOKED_AT);
    }
    false
}

/// How each unit named stands, in the order named, or why a name was
/// refused or couldn't be asked about.
pub fn status(said: &Value) -> Value {
    let Some(units) = said.get("units").and_then(Value::as_array) else {
        return refused("bad-units", "a status names its units as a list");
    };
    if units.len() > UNITS_MOST {
        return refused(
            "bad-units",
            &format!("a status names at most {UNITS_MOST} units"),
        );
    }
    // Every name is judged before systemd is asked, so one that isn't a
    // unit's is refused as such even when systemd can't be reached.
    let bus = systemd::session();
    let told: Vec<Value> = units
        .iter()
        .map(|unit| match (unit.as_str(), &bus) {
            (Some(name), Ok(bus)) if place::is_unit(name) => match systemd::status(bus, name) {
                Ok(standing) => json!({"unit": name, "status": standing}),
                Err(error) => json!({"unit": name, "error": error.to_string()}),
            },
            (Some(name), Err(error)) if place::is_unit(name) => {
                json!({"unit": name, "error": format!("systemd can't be asked: {error}")})
            }
            _ => json!({"unit": unit, "error": "not a unit's name"}),
        })
        .collect();
    json!({"units": told})
}
