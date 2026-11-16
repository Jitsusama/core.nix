//! The machines of the realm, judged from the node's own answer: whether a
//! link reaches each, its disk, its battery and what its own health says
//! of its code. Its own module because it changes for the machines'
//! reasons (what a node reports, and what Joel can act on from a machine),
//! which the other three parts don't share.

use serde_json::Value;

use super::{Answer, and, count, side_of, unread};
use crate::record::{Deeper, Health, Part, Thing};

/// A gigabyte, as a disk's free room is said.
const GIGABYTE: f64 = 1e9;
/// A battery at or under this, off the charger, is worth a look.
const LOW_CHARGE: u64 = 20;

/// The parts of another machine's own health this takes in: those only
/// that machine reads. Its machines, mycelium and network are the same
/// whole read from every machine, each already a part here, and taking
/// them in would spread one machine's trouble to all. The first two would
/// also latch, as each machine's word fed the other's.
const ITS_OWN: [&str; 1] = ["code"];

/// The machines of the realm, from `mycelium machines`, each with its
/// sessions from `mycelium health` where that was read.
pub fn machines(machines: &Answer, health: &Answer) -> Part {
    let deeper = Deeper {
        command: "mycelium machines",
        site: "mycelium/",
        launcher: "Agents › Networks › Machines",
    };
    let list = match machines {
        Ok(said) => said
            .pointer("/Ok/Machines")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        Err(why) => return unread("machines", why, deeper),
    };
    if list.is_empty() {
        return unread("machines", "the node knows of no machine", deeper);
    }
    let sessions = health.as_ref().ok().and_then(|said| said.get("sessions"));
    // The node says this machine first, whose own parts are parts here.
    let mut read: Vec<(Thing, Vec<String>)> = list
        .iter()
        .enumerate()
        .map(|(at, said)| {
            let others = (at > 0).then(|| said.get("health")).flatten();
            machine(
                said,
                sessions.and_then(|all| all.get(name_of(said))),
                others,
            )
        })
        .collect();
    // The worst first, then by name, so the line a look wants leads.
    read.sort_by(|(a, _), (b, _)| b.health.cmp(&a.health).then_with(|| a.name.cmp(&b.name)));
    let side = side_of(
        read.iter()
            .filter(|(thing, _)| thing.health > Health::Good)
            .map(|(thing, looks)| format!("{} {}", thing.name, looks.join(", ")))
            .collect(),
    );
    let things: Vec<Thing> = read.into_iter().map(|(thing, _)| thing).collect();
    let health = things
        .iter()
        .map(|thing| thing.health)
        .max()
        .unwrap_or(Health::Good);
    let says = match things
        .iter()
        .filter(|thing| thing.health > Health::Good)
        .count()
    {
        0 => format!(
            "{} ready",
            count(things.len() as u64, "machine", "machines")
        ),
        _ => things
            .iter()
            .filter(|thing| thing.health > Health::Good)
            .map(|thing| format!("{} {}", thing.name, thing.says))
            .collect::<Vec<_>>()
            .join("; "),
    };
    let holds = count(things.len() as u64, "machine", "machines");
    Part {
        name: "machines",
        health,
        says,
        side,
        holds: Some(holds),
        things,
        deeper,
    }
}

fn name_of(said: &Value) -> &str {
    said.get("name").and_then(Value::as_str).unwrap_or("")
}

/// What a machine's line says, in the order it's found, and what in it is
/// worth a look: how bad, with the words for a row's side, which names no
/// machine and so drops the verb ("filling disk" for "has a filling
/// disk").
#[derive(Default)]
struct Said {
    line: Vec<String>,
    looks: Vec<(Health, String, String)>,
}

impl Said {
    /// Something the line says that is no look.
    fn say(&mut self, words: String) {
        self.line.push(words);
    }

    /// Something worth a look, as the line says it and as a side does.
    fn look(&mut self, health: Health, said: String, side: String) {
        self.line.push(said.clone());
        self.looks.push((health, said, side));
    }
}

/// One machine: whether a link reaches it, what it's short of, its disk,
/// its battery, what its own health says of its own parts and its
/// sessions, each said only when it's worth saying, with the words of
/// what is worth a look in it.
///
/// Being short of room is said but never worth a look: agents' builds
/// hold the processor nearly all day, which isn't Joel's to act on, and
/// memory waits for a day measured to tell a held shortage from a spike.
fn machine(said: &Value, sessions: Option<&Value>, own: Option<&Value>) -> (Thing, Vec<String>) {
    let mut words = Said::default();
    if said.get("state").and_then(Value::as_str) == Some("unreachable") {
        words.look(
            Health::Bad,
            "is out of reach".to_owned(),
            "out of reach".to_owned(),
        );
    }
    let over: Vec<&str> = said
        .pointer("/room/over")
        .and_then(Value::as_array)
        .map(|over| {
            over.iter()
                .filter_map(Value::as_str)
                .map(resource)
                .collect()
        })
        .unwrap_or_default();
    if !over.is_empty() {
        words.say(format!("is short of {}", and(&over)));
    }
    let free = said
        .get("disk_free")
        .and_then(Value::as_f64)
        .map(|bytes| format!("{:.0} GB free", bytes / GIGABYTE));
    let disk = |band: &str| {
        free.as_ref().map_or(format!("{band} disk"), |free| {
            format!("{band} disk, {free}")
        })
    };
    let band = match said.get("disk_band").and_then(Value::as_str) {
        Some("filling") => Some((Health::Watch, disk("filling"))),
        Some("nearly-full" | "critically-full") => Some((Health::Bad, disk("nearly full"))),
        _ => None,
    };
    if let Some((health, side)) = band {
        words.look(health, format!("has a {side}"), side);
    }
    let on_battery = said.pointer("/power/on_battery").and_then(Value::as_bool) == Some(true);
    if let Some(charge) = said.pointer("/power/charge").and_then(Value::as_u64)
        && on_battery
        && charge <= LOW_CHARGE
    {
        let side = format!("on battery at {charge}%");
        words.look(Health::Watch, format!("is {side}"), side);
    }
    for part in ITS_OWN {
        match own.and_then(|own| own.pointer(&format!("/parts/{part}"))?.as_str()) {
            Some("watch") => words.look(
                Health::Watch,
                format!("says its {part} wants a look"),
                format!("{part} wants a look"),
            ),
            Some("bad") => words.look(
                Health::Bad,
                format!("says its {part} is failing"),
                format!("{part} failing"),
            ),
            _ => {}
        }
    }
    let Said {
        mut line,
        mut looks,
    } = words;
    let health = looks
        .iter()
        .map(|(health, ..)| *health)
        .max()
        .unwrap_or(Health::Good);
    let named: Vec<String> = looks.iter().map(|(_, said, _)| said.clone()).collect();
    // The side says the worst first; a sort that keeps ties in place keeps
    // the rest in the order they were found.
    looks.sort_by(|(a, ..), (b, ..)| b.cmp(a));
    let side = side_of(looks.into_iter().map(|(.., side)| side).collect());
    if line.is_empty() {
        line.push("is ready".to_owned());
    }
    if let Some(sessions) = sessions {
        line.push(format!("runs {}", sessions_said(sessions)));
    }
    let thing = Thing {
        kind: "machine",
        name: name_of(said).to_owned(),
        health,
        says: line.join(", "),
        side,
        address: None,
    };
    (thing, named)
}

/// A resource a node says a machine is short of, in Joel's words for it.
fn resource(said: &str) -> &str {
    match said {
        "cpu" => "processor",
        "io" => "disk time",
        other => other,
    }
}

/// A machine's sessions by how they stand: the busy and the idle, since
/// the ended ones are history.
fn sessions_said(sessions: &Value) -> String {
    let of = |key: &str| sessions.get(key).and_then(Value::as_u64).unwrap_or(0);
    let (busy, idle) = (of("busy"), of("idle"));
    match (busy, idle) {
        (0, 0) => "no session".to_owned(),
        (busy, 0) => format!("{} busy", count(busy, "session", "sessions")),
        (0, idle) => format!("{} idle", count(idle, "session", "sessions")),
        (busy, idle) => format!("{busy} busy and {idle} idle sessions"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::parts::tests::{NO_LOOK, said_health};

    fn laptop(over: &[&str], band: &str) -> Value {
        json!({"name": "laptop", "state": "ready", "disk_free": 278_819_094_528_u64,
               "disk_band": band, "room": {"over": over},
               "power": {"on_battery": false, "charge": 76}})
    }

    /// What `mycelium machines` answers for these machines.
    fn realm(machines: &[Value]) -> Value {
        json!({"Ok": {"Machines": machines}})
    }

    #[test]
    fn a_machine_short_of_room_leads_with_what_it_lacks() {
        let studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy",
                            "room": {"over": []}});
        let part = machines(
            &Ok(realm(&[studio, laptop(&["memory", "cpu"], "filling")])),
            &Ok(said_health()),
        );
        assert_eq!(part.health, Health::Watch);
        assert_eq!(
            part.things[0].name, "laptop",
            "the worst machine comes first"
        );
        assert_eq!(
            part.things[0].says,
            "is short of memory and processor, has a filling disk, 279 GB free, \
             runs 55 busy and 33 idle sessions"
        );
        assert_eq!(
            part.things[1].says,
            "is ready, runs 2 busy and 1 idle sessions"
        );
        assert!(
            part.says.starts_with("laptop is short of memory"),
            "the part says only the machines worth a look: {}",
            part.says
        );
        assert_eq!(
            part.side,
            ["laptop has a filling disk, 279 GB free"],
            "a side leaves out what the machine runs and what it's short of"
        );
        assert_eq!(
            part.things[0].side,
            ["filling disk, 279 GB free"],
            "a machine's own side is worded for a row's side"
        );
        assert!(part.things[1].side.is_empty(), "a ready machine has none");
    }

    #[test]
    fn a_machine_short_of_room_alone_is_ready() {
        let part = machines(
            &Ok(realm(&[laptop(&["memory", "cpu", "io"], "healthy")])),
            &Err(String::new()),
        );
        assert_eq!(
            part.health,
            Health::Good,
            "agents' builds fill the processor nearly all day, which isn't Joel's to act on"
        );
        assert_eq!(part.says, "1 machine ready");
        assert_eq!(part.side, NO_LOOK);
        assert_eq!(
            part.things[0].says, "is short of memory, processor and disk time",
            "what it's short of is still said"
        );
        assert!(part.things[0].side.is_empty(), "{:?}", part.things[0].side);
    }

    #[test]
    fn a_machines_side_says_its_worst_first() {
        let mut studio = laptop(&[], "healthy");
        studio["name"] = json!("studio");
        studio["power"] = json!({"on_battery": true, "charge": 12});
        studio["health"] = json!({"parts": {"code": "bad"}});
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio])),
            &Err(String::new()),
        );
        assert_eq!(part.things[0].name, "studio");
        assert_eq!(
            part.things[0].says,
            "is on battery at 12%, says its code is failing"
        );
        assert_eq!(
            part.things[0].side,
            ["code failing", "on battery at 12%"],
            "the bad before the worth a look"
        );
    }

    #[test]
    fn a_side_of_many_machines_says_how_many_more() {
        let away = |name: &str| json!({"name": name, "state": "unreachable"});
        let many: Vec<Value> = ["a", "b", "c", "d", "e"].into_iter().map(away).collect();
        let part = machines(&Ok(realm(&many)), &Err(String::new()));
        assert_eq!(
            part.side,
            [
                "a is out of reach",
                "b is out of reach",
                "and 3 more worth a look"
            ]
        );
    }

    #[test]
    fn machines_all_well_are_counted() {
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy")])),
            &Err("no node".into()),
        );
        assert_eq!(part.health, Health::Good);
        assert_eq!(part.says, "1 machine ready");
        assert_eq!(part.holds.as_deref(), Some("1 machine"));
        assert_eq!(part.side, NO_LOOK, "nothing is worth a look");
        assert_eq!(
            part.things[0].says, "is ready",
            "with no health, sessions go unsaid"
        );
    }

    #[test]
    fn a_machine_out_of_reach_or_nearly_full_is_bad() {
        let away = json!({"name": "spare", "state": "unreachable"});
        let part = machines(
            &Ok(realm(&[away, laptop(&[], "nearly-full")])),
            &Ok(said_health()),
        );
        assert_eq!(part.health, Health::Bad);
        assert_eq!(part.things[0].name, "laptop", "two bad machines go by name");
        assert!(
            part.things[0].says.starts_with("has a nearly full disk"),
            "the disk leads what the laptop says: {}",
            part.things[0].says
        );
        assert_eq!(part.things[1].says, "is out of reach");
    }

    #[test]
    fn a_low_battery_counts_only_off_the_charger() {
        let mut low = laptop(&[], "healthy");
        low["power"] = json!({"on_battery": true, "charge": 12});
        let part = machines(&Ok(realm(std::slice::from_ref(&low))), &Err(String::new()));
        assert_eq!(part.things[0].says, "is on battery at 12%");
        low["power"]["on_battery"] = json!(false);
        let charging = machines(&Ok(realm(&[low])), &Err(String::new()));
        assert_eq!(charging.health, Health::Good);
    }

    #[test]
    fn another_machine_says_what_its_own_code_is() {
        let mut studio = json!({"name": "studio", "state": "ready", "disk_band": "healthy"});
        studio["health"] = json!({"health": "bad", "parts": {"code": "watch", "network": "good"}});
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio.clone()])),
            &Err(String::new()),
        );
        assert_eq!(part.health, Health::Watch, "its code is worth a look");
        assert_eq!(part.things[0].name, "studio");
        assert_eq!(part.things[0].says, "says its code wants a look");
        studio["health"]["parts"]["code"] = json!("bad");
        let part = machines(
            &Ok(realm(&[laptop(&[], "healthy"), studio])),
            &Err(String::new()),
        );
        assert_eq!(part.health, Health::Bad);
        assert_eq!(part.things[0].says, "says its code is failing");
    }

    #[test]
    fn the_realms_own_parts_and_this_machines_health_are_never_taken_in() {
        let mut here = laptop(&[], "healthy");
        here["health"] = json!({"health": "bad", "parts": {"code": "bad"}});
        let mut studio = json!({"name": "studio", "state": "ready"});
        studio["health"] = json!({"health": "bad", "parts": {
            "machines": "bad", "mycelium": "bad", "network": "bad", "code": "good"}});
        let part = machines(&Ok(realm(&[here, studio])), &Err(String::new()));
        assert_eq!(
            part.health,
            Health::Good,
            "this machine's code is its own part, and the rest are the same whole \
             read from every machine: {}",
            part.says
        );
    }
}
