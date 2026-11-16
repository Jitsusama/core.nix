//! Where a job goes, from what mycelium says of the realm's machines: here
//! while this machine has room in its processor and memory, else the
//! first machine it may send to that's ready and has room in both, else
//! here after all, since a job waiting its turn on a crowded machine still
//! ends, and one sent to a crowded far machine only moves the crowd.
//!
//! Processor and memory alone decide, rather than mycelium's placement,
//! because the jobs sent are test suites and model checks, which want
//! both, and placement weighs other things: at 17:02 on 15 November it
//! named the Mac while the Mac's processor was over and this machine's
//! was not, for this machine's filling disk.

use serde_json::Value;

/// One machine, as mycelium's `machines` says it.
#[derive(Debug, Clone, PartialEq)]
pub struct Machine {
    pub name: String,
    /// `ready` when its node answers and it takes work.
    pub state: String,
    /// What it's over on: `cpu`, `memory` or `io`, as mycelium's measure
    /// of its pressure says.
    pub over: Vec<String>,
    /// Its processor's pressure, the share of time work waited for it.
    pub cpu: Option<f64>,
}

impl Machine {
    fn crowded(&self) -> bool {
        self.over.iter().any(|it| it == "cpu" || it == "memory")
    }
}

/// Why a job goes where it goes.
#[derive(Debug, Clone, PartialEq)]
pub enum Why {
    /// This machine has room in its processor and memory.
    Room,
    /// This machine is crowded, and the far one isn't.
    Crowded,
    /// This machine is crowded, and so is every machine it may send to,
    /// or none is ready.
    NoneHasRoom,
    /// It may send to no machine.
    NoMachines,
    /// mycelium couldn't say, for the reason given.
    NotHeard(String),
    /// It was told where.
    Told,
}

impl Why {
    /// The word for it in JSON.
    pub fn word(&self) -> &'static str {
        match self {
            Why::Room => "room",
            Why::Crowded => "crowded",
            Why::NoneHasRoom => "none-has-room",
            Why::NoMachines => "no-machines",
            Why::NotHeard(_) => "not-heard",
            Why::Told => "told",
        }
    }
}

/// Where a job goes: `None` is here.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub machine: Option<String>,
    pub why: Why,
}

/// The machines mycelium's `machines` answers, or what's wrong with it.
pub fn machines(text: &str) -> Result<Vec<Machine>, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let Some(list) = value.pointer("/Ok/Machines").and_then(Value::as_array) else {
        let said = value
            .pointer("/Err")
            .map_or_else(|| text.trim().to_owned(), Value::to_string);
        return Err(format!("mycelium answered no machines: {said}"));
    };
    Ok(list.iter().map(machine).collect())
}

fn machine(value: &Value) -> Machine {
    let text = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    Machine {
        name: text("/name").to_owned(),
        state: text("/state").to_owned(),
        over: value
            .pointer("/room/over")
            .and_then(Value::as_array)
            .map(|over| {
                over.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        cpu: value.pointer("/room/pressure/cpu").and_then(Value::as_f64),
    }
}

/// Where a job goes from `here`, among `machines`, sending only to those
/// in `allowed`, the first with room taken.
pub fn decide(here: &str, machines: &[Machine], allowed: &[String]) -> Place {
    let stays = |why| Place { machine: None, why };
    if allowed.is_empty() {
        return stays(Why::NoMachines);
    }
    let Some(this) = machines.iter().find(|it| it.name == here) else {
        return stays(Why::NotHeard(format!("mycelium doesn't know {here}")));
    };
    if !this.crowded() {
        return stays(Why::Room);
    }
    allowed
        .iter()
        .filter(|name| name.as_str() != here)
        .find_map(|name| machines.iter().find(|it| &it.name == name))
        .filter(|far| far.state == "ready" && !far.crowded())
        .map_or_else(
            || stays(Why::NoneHasRoom),
            |far| Place {
                machine: Some(far.name.clone()),
                why: Why::Crowded,
            },
        )
}

#[cfg(test)]
mod tests {
    use super::{Machine, Place, Why, decide, machines};

    fn machine(name: &str, state: &str, over: &[&str]) -> Machine {
        Machine {
            name: name.to_owned(),
            state: state.to_owned(),
            over: over.iter().map(|it| (*it).to_owned()).collect(),
            cpu: None,
        }
    }

    fn allowed(names: &[&str]) -> Vec<String> {
        names.iter().map(|it| (*it).to_owned()).collect()
    }

    #[test]
    fn a_job_stays_while_this_machine_has_room() {
        let realm = [
            machine("here", "ready", &["io"]),
            machine("far", "ready", &[]),
        ];
        let place = decide("here", &realm, &allowed(&["far"]));
        assert_eq!(
            place.machine, None,
            "a job left a machine with room: {place:?}"
        );
        assert_eq!(place.why, Why::Room, "it stayed for the wrong reason");
    }

    #[test]
    fn a_crowded_machine_sends_to_the_first_far_one_with_room() {
        let realm = [
            machine("here", "ready", &["cpu"]),
            machine("busy", "ready", &["cpu"]),
            machine("far", "ready", &[]),
        ];
        let place = decide("here", &realm, &allowed(&["far", "busy"]));
        assert_eq!(
            place,
            Place {
                machine: Some("far".into()),
                why: Why::Crowded
            },
            "a crowded machine didn't send to the far one with room"
        );
    }

    #[test]
    fn a_crowded_machine_keeps_a_job_no_far_one_can_take() {
        for realm in [
            vec![
                machine("here", "ready", &["cpu"]),
                machine("far", "ready", &["cpu"]),
            ],
            vec![
                machine("here", "ready", &["memory"]),
                machine("far", "ready", &["memory"]),
            ],
            vec![
                machine("here", "ready", &["cpu"]),
                machine("far", "unreached", &[]),
            ],
            vec![machine("here", "ready", &["cpu"])],
        ] {
            let place = decide("here", &realm, &allowed(&["far"]));
            assert_eq!(
                place.machine, None,
                "a job went to a machine without room: {realm:?}"
            );
            assert_eq!(
                place.why,
                Why::NoneHasRoom,
                "it stayed for the wrong reason"
            );
        }
    }

    #[test]
    fn a_machine_mycelium_doesnt_know_keeps_its_jobs() {
        let place = decide("here", &[machine("far", "ready", &[])], &allowed(&["far"]));
        assert_eq!(
            place.machine, None,
            "a job left a machine mycelium didn't know"
        );
        assert!(
            matches!(place.why, Why::NotHeard(_)),
            "it stayed as {:?}",
            place.why
        );
        let none = decide("here", &[machine("here", "ready", &["cpu"])], &[]);
        assert_eq!(
            none.why,
            Why::NoMachines,
            "a job with nowhere to go said {:?}",
            none.why
        );
    }

    #[test]
    fn mycelium_s_answer_is_read_as_it_writes_it() {
        let text = r#"{"Ok":{"Machines":[{"name":"far","state":"ready",
            "room":{"pressure":{"cpu":84.7},"over":["cpu"]}}]}}"#;
        let read = machines(text).expect("mycelium's answer wasn't read");
        assert_eq!(read[0].over, ["cpu"], "what it's over on was lost");
        assert_eq!(read[0].cpu, Some(84.7), "its processor's pressure was lost");
        let refused = machines(r#"{"Err":"no node"}"#).expect_err("an error read as machines");
        assert!(
            refused.contains("no node"),
            "mycelium's error was lost: {refused}"
        );
    }
}
