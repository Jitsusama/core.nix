//! Every live stop a reader keeps here, held on each other machine running
//! agents too, so a stop said in Slack or on GitHub refuses and parks the
//! sessions there as it does here: each machine's write guard reads only
//! its own stops folder, and the readers run on this machine alone.
//!
//! What a machine was last given is kept under `carried/`, one file for
//! each stop, so a stop is written there once, and again only when its
//! record here changes: when Joel resumes, narrows or widens it. A stop
//! standing goes to each machine mycelium lists a session on, and to the
//! machines the plan names, so a machine that starts a session while it
//! stands is given it too. A resumed stop goes only to the machines that
//! were given it standing, since one that never held it has nothing to
//! end. A machine that couldn't be given it is tried again after a
//! minute, never sooner, so one out of reach costs a run a minute.
//!
//! A stop is kept there under the same file name as here, never its id:
//! a stop read from an answer that couldn't be read is kept apart from
//! the stop of the same message (src/bin/pineapple-slack/round.rs), and
//! keeping both apart here keeps them apart there. Each copy names the
//! machine whose reader heard it in `carried_from`. A stop in shadow
//! refuses nothing, so it isn't carried, and Joel's stop on everything
//! reaches the other machines through its own steps, with its starts.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use super::reach::{self, Reached};
use super::{Plan, Runs, quoted};

/// The folder under the state folder where what each machine was given
/// is kept.
pub const CARRIED: &str = "carried";

/// How long a machine that couldn't be given a stop waits to be tried
/// again, in milliseconds.
pub const AGAIN_MS: u64 = 60_000;

/// The ids Joel's stop on everything goes by (src/pause/mod.rs), which
/// reaches the other machines itself.
const STOP_ALL: &str = "stop-all-";

/// A stop given to a machine, or tried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carried {
    /// The stop's file name here, without `.json`.
    pub name: String,
    /// The machine, by mycelium's name.
    pub machine: String,
    /// Whether what it was given is the stop resumed.
    pub resumed: bool,
    /// Why it couldn't be given, said plainly; none when it was.
    pub failed: Option<String>,
}

/// Gives each machine the stops it lacks, remembering when to try again a
/// machine that couldn't take one.
#[derive(Debug, Default)]
pub struct Carrier {
    /// When each stop may be tried again on each machine, by its name and
    /// the machine's.
    again: BTreeMap<(String, String), u64>,
}

impl Carrier {
    /// Gives each other machine, from `host`, every live stop under
    /// `state` as its record stands now, at `at` in milliseconds; answers
    /// each it gave or tried, and why mycelium couldn't list the machines
    /// running sessions when it couldn't.
    pub fn carry(
        &mut self,
        plan: &Plan,
        state: &Path,
        host: &str,
        at: u64,
        runs: &mut dyn Runs,
    ) -> (Vec<Carried>, Vec<String>) {
        let stops = live(&state.join("stops"));
        let mut listing = Vec::new();
        if plan.mycelium.is_none() || stops.is_empty() {
            return (Vec::new(), listing);
        }
        let standing = stops.iter().any(|(_, record)| stands(record));
        let machines = if standing {
            reach::others(plan, host, runs, &mut listing)
        } else {
            Vec::new()
        };
        let mut carried = Vec::new();
        for (name, record) in &stops {
            let mut held = given(state, name);
            let targets: Vec<Reached> = if stands(record) {
                one_each(&machines)
            } else {
                held.iter().map(|(reached, _)| reached.clone()).collect()
            };
            let mut moved = false;
            for target in targets {
                let holds = held
                    .iter()
                    .find(|(reached, _)| reached.machine == target.machine)
                    .map(|(_, holds)| holds);
                let key = (name.clone(), target.machine.clone());
                if holds == Some(record) || self.again.get(&key).is_some_and(|again| at < *again) {
                    continue;
                }
                let failed = give(plan, runs, &target, name, record, host).err();
                if failed.is_some() {
                    self.again.insert(key, at + AGAIN_MS);
                } else {
                    self.again.remove(&key);
                    held.retain(|(reached, _)| reached.machine != target.machine);
                    held.push((target.clone(), record.clone()));
                    moved = true;
                }
                carried.push(Carried {
                    name: name.clone(),
                    machine: target.machine.clone(),
                    resumed: !stands(record),
                    failed,
                });
            }
            if moved && let Err(error) = keep_given(state, name, &held) {
                // Given again next time, which writes the same record.
                carried.push(Carried {
                    name: name.clone(),
                    machine: String::new(),
                    resumed: !stands(record),
                    failed: Some(format!("couldn't note what was given: {error}")),
                });
            }
        }
        (carried, listing)
    }
}

/// The script a machine runs to hold the stop `record` under `name`, given
/// it from the machine `from`: the record written whole beside its stops
/// and renamed into place, so its guard never reads half of one, naming
/// in `carried_from` the machine whose reader heard it.
#[must_use]
pub fn script(name: &str, record: &Value, from: &str) -> String {
    let mut copy = record.clone();
    copy["carried_from"] = json!(from);
    format!(
        "set -e\n\
         echo '{begun}' >&2\n\
         d=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops\"\n\
         mkdir -p \"$d\"\n\
         printf '%s\\n' {record} > \"$d/{name}.new\"\n\
         mv \"$d/{name}.new\" \"$d/{name}.json\"\n",
        begun = reach::BEGUN,
        record = quoted(&copy.to_string()),
    )
}

/// Asks mycelium to run [`script`] on the machine `at` names, from `host`.
fn give(
    plan: &Plan,
    runs: &mut dyn Runs,
    at: &Reached,
    name: &str,
    record: &Value,
    host: &str,
) -> Result<(), String> {
    let Some(mycelium) = &plan.mycelium else {
        return Err("this machine names no mycelium to reach it".to_owned());
    };
    let mut args = reach::asked(&["run"], at.realm.as_deref());
    args.extend([
        "--need".to_owned(),
        format!("machine={}", at.machine),
        "--".to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        script(name, record, host),
    ]);
    runs.run(mycelium, &args)
        .map(|_| ())
        .map_err(|why| reach::without_begun(&why))
}

/// Each live stop kept in `stops`, by its file name, beside its record:
/// none in shadow, none of Joel's stops on everything, and none under a
/// name no reader keeps one under. A record that
/// can't be read is left out, as it is wherever stops are acted on.
fn live(stops: &Path) -> Vec<(String, Value)> {
    let Ok(entries) = fs::read_dir(stops) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Value)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let record: Value = serde_json::from_str(&fs::read_to_string(&path).ok()?).ok()?;
            Some((name, record))
        })
        .filter(|(name, record)| {
            record["shadow"] == false && !name.starts_with(STOP_ALL) && plain(name)
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Whether `name` is one a record is kept under (src/record.rs), so it
/// stands in the script as it is; a file named otherwise wasn't kept by a
/// reader.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Whether a stop's record stands: not resumed.
fn stands(record: &Value) -> bool {
    record.get("resumed").is_none_or(Value::is_null)
}

/// Each machine once, in the first realm it was found in, since a machine
/// keeps one stops folder however many realms it joins.
fn one_each(machines: &[Reached]) -> Vec<Reached> {
    let mut each: Vec<Reached> = Vec::new();
    for machine in machines {
        if !each.iter().any(|one| one.machine == machine.machine) {
            each.push(machine.clone());
        }
    }
    each
}

/// What each machine was given of the stop `name`, as last noted.
fn given(state: &Path, name: &str) -> Vec<(Reached, Value)> {
    let kept = fs::read_to_string(state.join(CARRIED).join(format!("{name}.json")))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    kept.as_ref()
        .and_then(|kept| kept["machines"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|one| Some((Reached::from_kept(one)?, one.get("holds")?.clone())))
        .collect()
}

/// Notes what each machine was given of the stop `name`, written whole
/// and renamed into place.
fn keep_given(state: &Path, name: &str, given: &[(Reached, Value)]) -> io::Result<()> {
    let folder = state.join(CARRIED);
    fs::create_dir_all(&folder)?;
    let machines: Vec<Value> = given
        .iter()
        .map(|(reached, holds)| {
            let mut one = reached.kept();
            one["holds"] = holds.clone();
            one
        })
        .collect();
    let path = folder.join(format!("{name}.json"));
    let written = path.with_extension("new");
    fs::write(&written, format!("{:#}\n", json!({ "machines": machines })))?;
    fs::rename(&written, &path)
}
