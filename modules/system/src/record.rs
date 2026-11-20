//! The record every view of the system reads, `system.view/1`: a health
//! for the whole, and for each part its health, what it says, the things
//! in it worth naming (a machine each, for the machines) and where each
//! surface goes deeper into it.

use serde_json::{Value, json};

/// How a part, or a thing in one, stands, from best to worst, so the worst
/// of several is their greatest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Health {
    /// Nothing about it wants a look.
    Good,
    /// Something about it is worth a look, though nothing has failed.
    Watch,
    /// Something about it has failed or can't be reached.
    Bad,
    /// Its owner's answer couldn't be had, so nothing is known of it.
    Unread,
}

impl Health {
    /// Its word, as the record and every view say it.
    pub fn word(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Watch => "watch",
            Self::Bad => "bad",
            Self::Unread => "not read",
        }
    }
}

/// Where each surface goes deeper into a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deeper {
    /// The command that says the part in full.
    pub command: &'static str,
    /// The local site that draws it, from the sites' root.
    pub site: &'static str,
    /// The launcher's place for it, with its crumbs.
    pub launcher: &'static str,
}

/// One thing in a part worth naming on its own: a machine, for now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thing {
    /// What kind of thing it is, as VKCF3Z's home addresses name kinds.
    pub kind: &'static str,
    pub name: String,
    pub health: Health,
    /// How it stands, in a short sentence's words.
    pub says: String,
    /// What in it is worth a look, worded for a row's side as a part's
    /// side is but without its name or a leading "is" or "has": its
    /// worst first, and none when nothing is.
    pub side: Vec<String>,
    /// Its home on the local sites, from their root, as the site that is
    /// home to it says (homes.rs); none when no site is.
    pub address: Option<String>,
    /// What ties it to the other parts, for a view to draw it whole: a
    /// machine's cells, network, code, figures and acts (parts/ties.rs).
    /// Null for a thing with none.
    pub ties: Value,
}

/// One of the system's four parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Its name: machines, mycelium, network or code.
    pub name: &'static str,
    pub health: Health,
    /// How it stands, its worst first; why it wasn't read, when it wasn't.
    pub says: String,
    /// What in it is worth a look, an item each, worded for a side: lower
    /// case unless a name leads, with no closing stop, and without what
    /// it holds. None when nothing is.
    pub side: Vec<String>,
    /// What it holds, counted in a few words, as "21 devices": the end of
    /// what it says, apart from what is worth a look. None when it wasn't
    /// read.
    pub holds: Option<String>,
    pub things: Vec<Thing>,
    pub deeper: Deeper,
}

/// The whole system as one machine reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    /// The machine it was read on.
    pub machine: String,
    /// When, as RFC 3339.
    pub read_at: String,
    pub parts: Vec<Part>,
}

impl System {
    /// The worst of the parts that could be read, or not read when none
    /// could.
    pub fn health(&self) -> Health {
        let read = self
            .parts
            .iter()
            .filter(|part| part.health != Health::Unread);
        read.map(|part| part.health).max().unwrap_or(Health::Unread)
    }

    /// What is worth a look in the whole: each part's side, in the parts'
    /// order.
    pub fn side(&self) -> Vec<&str> {
        self.parts
            .iter()
            .flat_map(|part| part.side.iter().map(String::as_str))
            .collect()
    }

    /// The record, as `system.view/1`.
    pub fn json(&self) -> Value {
        json!({
            "schema": "system.view/1",
            "machine": self.machine,
            "read_at": self.read_at,
            "health": self.health().word(),
            "side": self.side(),
            "parts": self.parts.iter().map(part_json).collect::<Vec<_>>(),
        })
    }

    /// One machine's record, as `system.machine/1`: its record as the
    /// daemon holds it (`records`), with where and when it was read; none
    /// when no part lists a machine by that name.
    pub fn machine(&self, name: &str) -> Option<Value> {
        let key = format!("machine:{name}");
        let mut said = self
            .records()
            .into_iter()
            .find(|record| record["key"] == key.as_str())?;
        said["schema"] = json!("system.machine/1");
        said["machine"] = json!(self.machine);
        said["read_at"] = json!(self.read_at);
        Some(said)
    }

    /// Why the machines couldn't be read, when they couldn't, so a machine
    /// asked for by name is said unanswered rather than unknown.
    pub fn machines_unread(&self) -> Option<&str> {
        self.parts
            .iter()
            .find(|part| part.name == "machines" && part.health == Health::Unread)
            .map(|part| part.says.as_str())
    }

    /// The record split as the daemon holds it, in the shell's daemon
    /// interface: the whole's `summary`, a `part` record for each part and
    /// one for each thing in a part, a `machine` for now. Each holds only
    /// what changes with it, so a part that stands as it stood isn't told
    /// again when another moves; only the summary says when it was read.
    pub fn records(&self) -> Vec<Value> {
        let summary = json!({
            "key": "summary",
            "kind": "summary",
            "schema": "system.view/1",
            "machine": self.machine,
            "read_at": self.read_at,
            "health": self.health().word(),
            "side": self.side(),
            "parts": self.parts.iter().map(|part| part.name).collect::<Vec<_>>(),
        });
        let parts = self.parts.iter().map(|part| {
            let mut said = part_json(part);
            said["key"] = json!(format!("part:{}", part.name));
            said["kind"] = json!("part");
            said["things"] = part.things.iter().map(thing_key).collect();
            said
        });
        let things = self.parts.iter().flat_map(|part| {
            part.things.iter().map(move |thing| {
                json!({
                    "key": thing_key(thing),
                    "kind": thing.kind,
                    "name": thing.name,
                    "part": part.name,
                    "health": thing.health.word(),
                    "says": thing.says,
                    "side": thing.side,
                    "address": thing.address,
                    "ties": thing.ties,
                })
            })
        });
        std::iter::once(summary)
            .chain(parts)
            .chain(things)
            .collect()
    }
}

/// A thing's key, its kind and its name, as the daemon interface keys a
/// record and VKCF3Z's home addresses name one.
fn thing_key(thing: &Thing) -> Value {
    json!(format!("{}:{}", thing.kind, thing.name))
}

fn part_json(part: &Part) -> Value {
    json!({
        "part": part.name,
        "health": part.health.word(),
        "says": part.says,
        "side": part.side,
        "holds": part.holds,
        "things": part.things.iter().map(|thing| json!({
            "kind": thing.kind,
            "name": thing.name,
            "health": thing.health.word(),
            "says": thing.says,
            "side": thing.side,
            "address": thing.address,
            "ties": thing.ties,
        })).collect::<Vec<_>>(),
        "deeper": {
            "command": part.deeper.command,
            "site": part.deeper.site,
            "launcher": part.deeper.launcher,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &'static str, health: Health) -> Part {
        Part {
            name,
            health,
            says: String::new(),
            side: Vec::new(),
            holds: None,
            things: Vec::new(),
            deeper: Deeper {
                command: "",
                site: "",
                launcher: "",
            },
        }
    }

    fn system(parts: Vec<Part>) -> System {
        System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-15T19:00:00Z".to_owned(),
            parts,
        }
    }

    fn machine(name: &str, health: Health) -> Thing {
        Thing {
            kind: "machine",
            name: name.to_owned(),
            health,
            says: format!("{name} says so"),
            side: Vec::new(),
            address: None,
            ties: Value::Null,
        }
    }

    #[test]
    fn one_machine_answers_its_record_where_and_when_it_was_read() {
        let mut machines = part("machines", Health::Good);
        machines.things = vec![machine("laptop", Health::Watch)];
        let system = system(vec![machines]);
        let said = system.machine("laptop").expect("the laptop is listed");
        assert_eq!(said["schema"], "system.machine/1");
        assert_eq!(said["key"], "machine:laptop");
        assert_eq!(said["health"], "watch");
        assert_eq!(said["part"], "machines", "the part that lists it");
        assert!(said.get("ties").is_some(), "its ties, as SKILL.md says");
        assert_eq!(said["machine"], system.machine);
        assert_eq!(said["read_at"], system.read_at);
        assert_eq!(system.machine("Laptop"), None, "named as the node names it");
        assert_eq!(system.machines_unread(), None);
    }

    #[test]
    fn machines_that_couldnt_be_read_say_why() {
        let mut machines = part("machines", Health::Unread);
        machines.says = "mycelium isn't answering".to_owned();
        let system = system(vec![machines]);
        assert_eq!(system.machine("laptop"), None);
        assert_eq!(system.machines_unread(), Some("mycelium isn't answering"));
    }

    #[test]
    fn a_machine_record_carries_its_ties_to_the_other_parts() {
        let mut machines = part("machines", Health::Good);
        let mut laptop = machine("laptop", Health::Good);
        laptop.ties = json!({"cells": {"busy": 1, "idle": 0, "ended": 0}});
        machines.things = vec![laptop];
        let system = system(vec![machines]);
        assert_eq!(system.records()[2]["ties"]["cells"]["busy"], 1);
        assert_eq!(
            system.json()["parts"][0]["things"][0]["ties"]["cells"]["busy"],
            1
        );
    }

    #[test]
    fn a_machine_record_carries_its_side() {
        let mut machines = part("machines", Health::Watch);
        let mut studio = machine("studio", Health::Watch);
        studio.side = vec!["code wants a look".to_owned()];
        machines.things = vec![studio, machine("laptop", Health::Good)];
        let system = system(vec![machines]);
        let records = system.records();
        assert_eq!(records[2]["side"], json!(["code wants a look"]));
        assert_eq!(records[3]["side"], json!([]), "a ready machine's is empty");
        assert_eq!(
            system.json()["parts"][0]["things"][0]["side"],
            json!(["code wants a look"])
        );
    }

    #[test]
    fn the_daemon_holds_a_summary_then_each_part_then_each_thing() {
        let mut machines = part("machines", Health::Watch);
        machines.things = vec![
            machine("studio", Health::Watch),
            machine("laptop", Health::Good),
        ];
        let records = system(vec![machines, part("code", Health::Good)]).records();
        let keys: Vec<&str> = records
            .iter()
            .filter_map(|record| record["key"].as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "summary",
                "part:machines",
                "part:code",
                "machine:studio",
                "machine:laptop"
            ]
        );
        assert_eq!(records[0]["health"], "watch", "the summary says the whole");
        assert_eq!(records[0]["parts"], json!(["machines", "code"]));
        assert_eq!(
            records[1]["things"],
            json!(["machine:studio", "machine:laptop"]),
            "a part names its things by their keys rather than saying them twice"
        );
        assert_eq!(records[3]["part"], "machines");
        assert_eq!(records[3]["says"], "studio says so");
        assert_eq!(
            records[3]["address"],
            Value::Null,
            "a thing with no home says so"
        );
    }

    #[test]
    fn a_thing_with_a_home_carries_its_address_in_every_record() {
        let mut machines = part("machines", Health::Good);
        let mut laptop = machine("laptop", Health::Good);
        laptop.address = Some("mycelium/#machine=laptop".to_owned());
        machines.things = vec![laptop];
        let system = system(vec![machines]);
        assert_eq!(
            system.json()["parts"][0]["things"][0]["address"],
            "mycelium/#machine=laptop"
        );
        assert_eq!(system.records()[2]["address"], "mycelium/#machine=laptop");
    }

    #[test]
    fn the_side_of_each_part_and_of_the_whole_is_in_every_record() {
        let mut network = part("network", Health::Watch);
        network.side = vec!["the modem is busy".to_owned()];
        let mut code = part("code", Health::Watch);
        code.side = vec!["the build is failing".to_owned()];
        let system = system(vec![part("machines", Health::Good), network, code]);
        let whole = json!(["the modem is busy", "the build is failing"]);
        assert_eq!(
            system.json()["side"],
            whole,
            "the whole's, in the parts' order"
        );
        assert_eq!(
            system.json()["parts"][1]["side"],
            json!(["the modem is busy"])
        );
        let records = system.records();
        assert_eq!(records[0]["side"], whole, "the summary says the whole's");
        assert_eq!(
            records[1]["side"],
            json!([]),
            "a part with nothing worth a look says none"
        );
        assert_eq!(records[2]["side"], json!(["the modem is busy"]));
    }

    #[test]
    fn a_part_says_what_it_holds_apart_and_none_when_unread() {
        let mut network = part("network", Health::Good);
        network.holds = Some("21 devices".to_owned());
        let system = system(vec![network, part("code", Health::Unread)]);
        assert_eq!(system.json()["parts"][0]["holds"], "21 devices");
        let records = system.records();
        assert_eq!(
            records[1]["holds"], "21 devices",
            "the part's record says it"
        );
        assert!(
            records[2]["holds"].is_null(),
            "a part not read holds nothing it can say: {}",
            records[2]
        );
    }

    #[test]
    fn only_the_summary_says_when_it_was_read() {
        let records = system(vec![part("network", Health::Good)]).records();
        assert_eq!(records[0]["read_at"], "2026-11-15T19:00:00Z");
        assert!(
            records[1].get("read_at").is_none(),
            "a part that stands as it stood mustn't be told again: {}",
            records[1]
        );
    }

    #[test]
    fn the_whole_stands_as_its_worst_read_part() {
        let whole = system(vec![
            part("machines", Health::Watch),
            part("network", Health::Unread),
            part("code", Health::Good),
        ]);
        assert_eq!(
            whole.health(),
            Health::Watch,
            "a part not read says nothing of the whole"
        );
        let bad = system(vec![
            part("machines", Health::Watch),
            part("code", Health::Bad),
        ]);
        assert_eq!(bad.health(), Health::Bad, "the worst read part wins");
    }

    #[test]
    fn nothing_read_is_said_as_not_read() {
        let none = system(vec![part("machines", Health::Unread)]);
        assert_eq!(none.health(), Health::Unread);
        assert_eq!(none.json()["health"], "not read");
    }

    #[test]
    fn the_record_names_its_schema_and_each_part_in_order() {
        let said = system(vec![
            part("machines", Health::Good),
            part("code", Health::Watch),
        ])
        .json();
        assert_eq!(said["schema"], "system.view/1");
        assert_eq!(said["parts"][0]["part"], "machines");
        assert_eq!(said["parts"][1]["health"], "watch");
        assert!(
            said["parts"][1]["things"]
                .as_array()
                .is_some_and(Vec::is_empty),
            "a part with nothing under it still says things, as none: {said}"
        );
    }
}
