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
}

/// One of the system's four parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Its name: machines, mycelium, network or code.
    pub name: &'static str,
    pub health: Health,
    /// How it stands, its worst first; why it wasn't read, when it wasn't.
    pub says: String,
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

    /// The record, as `system.view/1`.
    pub fn json(&self) -> Value {
        json!({
            "schema": "system.view/1",
            "machine": self.machine,
            "read_at": self.read_at,
            "health": self.health().word(),
            "parts": self.parts.iter().map(part_json).collect::<Vec<_>>(),
        })
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
        "things": part.things.iter().map(|thing| json!({
            "kind": thing.kind,
            "name": thing.name,
            "health": thing.health.word(),
            "says": thing.says,
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
        }
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
