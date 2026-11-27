//! What's watched, and the places' rows made of what's held: which kinds
//! and trees a watch holds, which places' rows are made, and each
//! place's rows and headline made again, kept and handed over.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::time::Duration;

use crate::answering::sources;
use crate::{DISK, Daemon, FIRMWARE, HEAT, MACHINE, PLACES, POWER, Place};
use machine::interface::{APART, KINDS, MADE, kind_of};
use machine::{clock_ms, firmware_rows, heat_rows, machine_rows, rows, tree};
use serde_json::{Value, json};

/// How lately the programs must have been read for Machine's rows to say
/// them: two reads' time, so one read a little late doesn't lose them.
const USING_FRESH: Duration = Duration::from_secs(2);
/// The kinds some watch holds, of every place or by a key, as the rows
/// of a place watched are said by `makes`.
pub(crate) fn watched(daemon: &Daemon) -> HashSet<String> {
    let served = &daemon.served;
    let mut kinds: HashSet<String> = served
        .keys_watched()
        .iter()
        .filter_map(|key| kind_of(key))
        .map(str::to_owned)
        .collect();
    let whole = KINDS.iter().filter(|kind| served.watches(kind, ""));
    kinds.extend(whole.map(|kind| (*kind).to_owned()));
    kinds
}

/// The pids whose trees some watch names by key.
pub(crate) fn watched_trees(daemon: &Daemon) -> BTreeSet<u32> {
    let keys = daemon.served.keys_watched();
    keys.iter().filter_map(|key| tree::pid_of(key)).collect()
}

/// Whether some watch holds a place's rows or its headline.
pub(crate) fn makes(daemon: &Daemon, place: Place) -> bool {
    let served = &daemon.served;
    daemon.words.is_some()
        && (served.watches("row", place.name)
            || served.watches("head", place.name)
            || served.keys_watched().iter().any(|key| place.holds(key)))
}

/// The places whose rows some watch holds.
pub(crate) fn making(daemon: &Daemon) -> Vec<Place> {
    PLACES
        .into_iter()
        .filter(|place| makes(daemon, *place))
        .collect()
}

/// The kinds read for these: each as it is, but a made kind, which is
/// read as what these places' rows are made of.
pub(crate) fn sources_of(kinds: &HashSet<String>, places: &[Place]) -> HashSet<String> {
    let mut read: HashSet<String> = kinds
        .iter()
        .filter(|kind| !MADE.contains(&kind.as_str()))
        .cloned()
        .collect();
    for place in places {
        read.extend(place.sources.iter().map(|kind| (*kind).to_owned()));
    }
    read
}

/// The kinds read while what's watched is: those watched, and what the
/// places watched are made of, as fwupd is asked for Firmware's rows.
pub(crate) fn reads(daemon: &Daemon) -> HashSet<String> {
    sources_of(&watched(daemon), &making(daemon))
}

/// Lets a place's reads go once nothing watches its rows, so a shut
/// launcher keeps none, and the next watch starts afresh.
pub(crate) fn let_go(daemon: &mut Daemon) {
    let held = daemon.held.len();
    for place in PLACES {
        if makes(daemon, place) {
            continue;
        }
        if place == HEAT {
            daemon.heat.forget();
        } else if place == POWER {
            daemon.power.forget();
        } else if place == DISK {
            daemon.disk.forget();
        } else {
            // How an act ended is said until the launcher shuts, as
            // Firmware kept it; one still going is kept.
            daemon.acting.ended.clear();
        }
        daemon.kept.remove(place.name);
        daemon.held.retain(|key, _| !place.holds(key));
    }
    if !telemetry_asked(daemon) {
        daemon.telemetry.dropped();
    }
    if daemon.held.len() != held {
        hand_over(daemon);
    }
}

/// Hands every record held, and how the sources stand, to what serves
/// them, which tells each watch what moved of what it wants.
pub(crate) fn hand_over(daemon: &Daemon) {
    let records = daemon.held.values().cloned().collect();
    daemon.served.change(records, sources(daemon));
}

/// Whether the processor's telemetry is asked: while Heat's rows are
/// watched, for its temperatures, or `telemetry` is, for what held the
/// processor back and kept it from resting.
pub(crate) fn telemetry_asked(daemon: &Daemon) -> bool {
    makes(daemon, HEAT) || watched(daemon).contains("telemetry")
}

/// The records of one kind held.
fn held_of(held: &BTreeMap<String, Value>, kind: &str) -> Vec<Value> {
    held.iter()
        .filter(|(key, _)| kind_of(key) == Some(kind))
        .map(|(_, record)| record.clone())
        .collect()
}

/// A place's rows and headline as they read now, from what's held.
fn ready(daemon: &mut Daemon, place: Place) -> rows::Ready {
    let asked = telemetry_asked(daemon);
    let Daemon {
        held,
        heat,
        power,
        disk,
        acting,
        words,
        telemetry,
        read_at,
        ..
    } = daemon;
    let Some(words) = words.as_ref() else {
        return rows::Ready::default();
    };
    if place == POWER {
        return power.read(&held_of(held, "supply"), clock_ms(), words);
    }
    if place == FIRMWARE {
        return firmware_rows::read(&held_of(held, "firmware"), acting, words);
    }
    if place == DISK {
        let read = read_at.get("disk").copied();
        return disk.read(&held_of(held, "disk"), read, words);
    }
    if place == MACHINE {
        let using = held_of(held, "using");
        let fresh = read_at
            .get("using")
            .is_some_and(|at| at.elapsed() < USING_FRESH);
        let memory = held.get("memory:main");
        let boot = held_of(held, "boot").into_iter().next();
        let said = machine_rows::Held {
            processor: held.get("processor:0"),
            memory,
            boot: boot.as_ref(),
            using: fresh.then_some(using.as_slice()),
            telemetry: asked.then_some(&*telemetry),
        };
        return machine_rows::read(said, clock_ms(), words);
    }
    let (sensors, zones, cooling) = (
        held_of(held, "sensor"),
        held_of(held, "zone"),
        held_of(held, "cooling"),
    );
    let processor = held_of(held, "processor").into_iter().next();
    let temperatures = telemetry.temperatures();
    let said = heat_rows::Held {
        sensors: &sensors,
        zones: &zones,
        cooling: &cooling,
        processor: processor.as_ref(),
        telemetry: &temperatures,
    };
    heat.read(said, clock_ms(), words)
}

/// Makes a place's rows and headline again from what's held, keeps them,
/// and says those that changed and the keys of those gone.
pub(crate) fn made(daemon: &mut Daemon, place: Place) -> (Vec<Value>, Vec<String>) {
    let mut ready = ready(daemon, place);
    let head = ready
        .head
        .map(|head| json!({"key": place.head(), "kind": "head", "head": head}));
    let mut beside: Vec<Value> = head.into_iter().collect();
    beside.extend(ready.rows.iter_mut().filter_map(drawn_apart));
    let moved = daemon
        .kept
        .entry(place.name)
        .or_default()
        .take(ready.rows, beside);
    for key in &moved.gone {
        daemon.held.remove(key);
    }
    for record in &moved.changed {
        if let Some(key) = record["key"].as_str() {
            daemon.held.insert(key.to_owned(), record.clone());
        }
    }
    (moved.changed, moved.gone)
}

/// A row's drawer as a record of its own, `drawer:<place>:<id>`, the row
/// left naming it by that key (ReadyRows.js's drawerServed), so a read
/// that moves only a drawer's counts changes no row and is told only to
/// a watch naming the drawer (`APART`); none where the row has no drawer.
fn drawn_apart(row: &mut Value) -> Option<Value> {
    let id = row["key"].as_str()?.strip_prefix("row:")?;
    let key = format!("{APART}:{id}");
    let fields = row.as_object_mut()?;
    if !fields.get("drawer").is_some_and(Value::is_object) {
        return None;
    }
    let drawer = fields.insert("drawer".to_owned(), json!(key))?;
    Some(json!({"key": key, "kind": APART, "drawer": drawer}))
}
