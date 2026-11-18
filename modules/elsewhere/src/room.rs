//! Where a job goes, from mycelium's placement: wherever `mycelium place`
//! puts work of its kind, when that's this machine or one it may send to,
//! else here, since a job waiting its turn on a crowded machine still
//! ends, and one sent somewhere it wasn't opted in to only moves trouble.
//!
//! The placement decides rather than a rule of elsewhere's own, because it
//! is the one score that sees the whole realm's load: on 15 November a
//! placer counting only its own sessions saw a fraction of one machine's
//! load (placement research, 15 November). A kind says what a machine
//! must be for it (`needs`) and how long its jobs run (`lasts`), in
//! place's own words.

use serde_json::Value;

/// Why a job goes where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// mycelium placed it on this machine.
    PlacedHere,
    /// mycelium placed it on a machine it may send to.
    Placed,
    /// mycelium placed it on the machine named, which it may not send to.
    NotAllowed(String),
    /// mycelium fits it nowhere, for the reasons given.
    Refused(String),
    /// It may send to no machine.
    NoMachines,
    /// mycelium couldn't say, for the reason given.
    NotHeard(String),
    /// It was told where.
    Told,
    /// Its kind brings back the path named, which couldn't come home.
    CantComeHome(String),
}

impl Why {
    /// The word for it in JSON.
    pub fn word(&self) -> &'static str {
        match self {
            Why::PlacedHere => "placed-here",
            Why::Placed => "placed",
            Why::NotAllowed(_) => "not-allowed",
            Why::Refused(_) => "refused",
            Why::NoMachines => "no-machines",
            Why::NotHeard(_) => "not-heard",
            Why::Told => "told",
            Why::CantComeHome(_) => "cant-come-home",
        }
    }
}

/// Where a job goes: `None` is here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub machine: Option<String>,
    pub why: Why,
}

/// What `mycelium place` answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// The machine the work goes to.
    Placed(String),
    /// No machine fits, each with why, in mycelium's words.
    Refused(Vec<String>),
}

/// The arguments asking `mycelium place` for a kind that `needs` these
/// (`kind=value` each) and whose jobs last `lasts`, when it says.
pub fn asking(needs: &[String], lasts: Option<&str>) -> Vec<String> {
    let mut args = vec!["place".to_owned()];
    for need in needs {
        args.extend(["--need".to_owned(), need.clone()]);
    }
    if let Some(lasts) = lasts {
        args.extend(["--lasts".to_owned(), lasts.to_owned()]);
    }
    args
}

/// The placement mycelium's `place` answers, or what's wrong with it.
pub fn placement(text: &str) -> Result<Placement, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if let Some(machine) = value
        .pointer("/Ok/Placement/placed/machine")
        .and_then(Value::as_str)
    {
        return Ok(Placement::Placed(machine.to_owned()));
    }
    if let Some(misses) = value
        .pointer("/Ok/Placement/refused/misses")
        .and_then(Value::as_array)
    {
        return Ok(Placement::Refused(misses.iter().map(miss).collect()));
    }
    let said = value
        .pointer("/Err")
        .map_or_else(|| text.trim().to_owned(), Value::to_string);
    Err(format!("mycelium placed nothing: {said}"))
}

/// One machine that didn't fit, as `NAME: why, why`.
fn miss(value: &Value) -> String {
    let name = value
        .pointer("/machine")
        .and_then(Value::as_str)
        .unwrap_or("a machine");
    let because: Vec<String> = value
        .pointer("/because")
        .and_then(Value::as_array)
        .map(|reasons| reasons.iter().map(Value::to_string).collect())
        .unwrap_or_default();
    format!("{name}: {}", because.join(", "))
}

/// Where a job goes from `here`, given mycelium's `placement`, sending
/// only to machines in `allowed`.
pub fn decide(here: &str, placement: &Placement, allowed: &[String]) -> Place {
    let stays = |why| Place { machine: None, why };
    if allowed.is_empty() {
        return stays(Why::NoMachines);
    }
    match placement {
        Placement::Placed(machine) if machine == here => stays(Why::PlacedHere),
        Placement::Placed(machine) if allowed.contains(machine) => Place {
            machine: Some(machine.clone()),
            why: Why::Placed,
        },
        Placement::Placed(machine) => stays(Why::NotAllowed(machine.clone())),
        Placement::Refused(misses) => stays(Why::Refused(misses.join("; "))),
    }
}

#[cfg(test)]
mod tests {
    use super::{Place, Placement, Why, asking, decide, placement};

    fn allowed(names: &[&str]) -> Vec<String> {
        names.iter().map(|it| (*it).to_owned()).collect()
    }

    fn placed(machine: &str) -> Placement {
        Placement::Placed(machine.to_owned())
    }

    #[test]
    fn a_job_mycelium_places_here_stays() {
        let place = decide("here", &placed("here"), &allowed(&["far"]));
        assert_eq!(
            place,
            Place {
                machine: None,
                why: Why::PlacedHere
            },
            "a job placed here didn't stay"
        );
    }

    #[test]
    fn a_job_placed_on_a_machine_it_may_send_to_goes_there() {
        let place = decide("here", &placed("far"), &allowed(&["near", "far"]));
        assert_eq!(
            place,
            Place {
                machine: Some("far".into()),
                why: Why::Placed
            },
            "a job placed on a machine it may send to didn't go"
        );
    }

    #[test]
    fn a_job_placed_where_it_may_not_go_stays() {
        let place = decide("here", &placed("stranger"), &allowed(&["far"]));
        assert_eq!(
            place,
            Place {
                machine: None,
                why: Why::NotAllowed("stranger".into())
            },
            "a job went to a machine it wasn't opted in to"
        );
    }

    #[test]
    fn a_job_mycelium_fits_nowhere_or_with_nowhere_to_go_stays() {
        let refused = decide(
            "here",
            &Placement::Refused(vec!["far: lacks".into()]),
            &allowed(&["far"]),
        );
        assert_eq!(refused.machine, None, "a refused job left: {refused:?}");
        assert_eq!(
            refused.why,
            Why::Refused("far: lacks".into()),
            "a refused job lost mycelium's reasons"
        );
        let none = decide("here", &placed("far"), &[]);
        assert_eq!(
            none,
            Place {
                machine: None,
                why: Why::NoMachines
            },
            "a job with no machine to go to was sent"
        );
    }

    #[test]
    fn a_kind_asks_in_place_s_own_words() {
        assert_eq!(
            asking(&["system=darwin".into(), "reach=home".into()], Some("2h")),
            [
                "place",
                "--need",
                "system=darwin",
                "--need",
                "reach=home",
                "--lasts",
                "2h"
            ],
            "a kind's needs and length weren't asked as place takes them"
        );
        assert_eq!(asking(&[], None), ["place"], "a plain kind asked more");
    }

    #[test]
    fn mycelium_s_answer_is_read_as_it_writes_it() {
        let text = r#"{"Ok":{"Placement":{"placed":{"machine":"far",
            "passed":[{"machine":"here","because":[{"lacks":{"system":"linux"}}]}]}}}}"#;
        assert_eq!(
            placement(text),
            Ok(placed("far")),
            "a placement wasn't read"
        );
        let refused = placement(
            r#"{"Ok":{"Placement":{"refused":{"misses":[
            {"machine":"far","because":[{"lacks":{"machine":"x"}}]}]}}}}"#,
        )
        .expect("a refusal wasn't read");
        let Placement::Refused(misses) = refused else {
            panic!("a refusal read as {refused:?}");
        };
        assert!(
            misses[0].starts_with("far: ") && misses[0].contains("lacks"),
            "a miss lost its machine or reason: {misses:?}"
        );
        let error = placement(r#"{"Err":"no node"}"#).expect_err("an error read as placed");
        assert!(
            error.contains("no node"),
            "mycelium's error was lost: {error}"
        );
    }
}
