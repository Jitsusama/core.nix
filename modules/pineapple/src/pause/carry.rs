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
//! stands is given it too. Whatever follows (a resume, a narrowing, a
//! widening) goes to each machine that may hold the stop, one whose give
//! failed included, since a run can fail after its script wrote the stop
//! there, and one noted lacking it while it waited out its minute; one
//! never given it nor noted so is given nothing. A stop no longer kept
//! here, such as one read from an answer that couldn't be read, forgotten
//! once the answer could be, is taken away from each machine given it. A
//! machine that couldn't take what it was given is tried again after a
//! minute, never sooner, for every stop at once, so one out of reach
//! costs a run a minute.
//!
//! A stop is kept there under the same file name as here, never its id:
//! a stop read from an answer that couldn't be read is kept apart from
//! the stop of the same message (src/bin/pineapple-slack/round.rs), and
//! keeping both apart here keeps them apart there. Each copy names the
//! machine whose reader heard it in `carried_from`, and a copy is never
//! carried on, so only the machine whose reader heard a stop carries it.
//! A stop in shadow refuses nothing, so it isn't carried, and Joel's stop
//! on everything reaches the other machines through its own steps, with
//! its starts.

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

/// How long a machine that couldn't take a stop waits to be tried again,
/// in milliseconds.
pub const AGAIN_MS: u64 = 60_000;

/// The ids Joel's stop on everything goes by (src/pause/mod.rs), which
/// reaches the other machines itself.
const STOP_ALL: &str = "stop-all-";

/// What a machine is given of a stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gift {
    /// The stop, standing.
    Stop,
    /// The stop, resumed.
    Resume,
    /// The stop taken away, since it is no longer kept here.
    Removal,
}

impl Gift {
    /// What is given, as a journal line says it before the stop's name.
    #[must_use]
    pub fn said(self) -> &'static str {
        match self {
            Self::Stop => "the stop",
            Self::Resume => "the resume of",
            Self::Removal => "the end of",
        }
    }
}

/// A stop given to a machine, or tried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carried {
    /// The stop's file name here, without `.json`.
    pub name: String,
    /// The machine, by mycelium's name.
    pub machine: String,
    /// What it was given.
    pub gift: Gift,
    /// Why it couldn't be given, said plainly; none when it was.
    pub failed: Option<String>,
}

/// Gives each machine the stops it lacks, remembering when to try again a
/// machine that couldn't take one.
#[derive(Debug, Default)]
pub struct Carrier {
    /// When each machine that couldn't take a stop may be tried again, by
    /// its name.
    again: BTreeMap<String, u64>,
}

/// Where a look of the carrier stands: the plan, the state folder, this
/// machine's name and the time, in milliseconds.
#[derive(Clone, Copy)]
struct Here<'a> {
    plan: &'a Plan,
    state: &'a Path,
    host: &'a str,
    at: u64,
}

impl Carrier {
    /// Gives each other machine, from `host`, every live stop under
    /// `state` as its record stands now, and the end of each it was given
    /// that is kept here no longer, at `at` in milliseconds; answers each
    /// it gave or tried, and why mycelium couldn't list the machines
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
        let gone = gone(state);
        let mut listing = Vec::new();
        if plan.mycelium.is_none() || (stops.is_empty() && gone.is_empty()) {
            return (Vec::new(), listing);
        }
        let machines = if stops.iter().any(|(_, record)| stands(record)) {
            reach::others(plan, host, runs, &mut listing)
        } else {
            Vec::new()
        };
        let here = Here {
            plan,
            state,
            host,
            at,
        };
        let mut carried = Vec::new();
        for (name, record) in &stops {
            carried.extend(self.give_each(here, runs, name, record, &machines));
        }
        for name in &gone {
            carried.extend(self.take_away(here, runs, name));
        }
        (carried, listing)
    }

    /// Whether `machine` couldn't take a stop within the last minute.
    fn waits(&self, machine: &str, at: u64) -> bool {
        self.again.get(machine).is_some_and(|again| at < *again)
    }

    /// Notes whether `machine` took what it was given at `at`.
    fn took(&mut self, machine: &str, at: u64, failed: bool) {
        if failed {
            self.again.insert(machine.to_owned(), at + AGAIN_MS);
        } else {
            self.again.remove(machine);
        }
    }

    /// Gives the stop `record`, kept here under `name`, to each machine
    /// lacking it as it stands: each of `machines` while it stands, and
    /// each that may hold it already, whatever it has become.
    fn give_each(
        &mut self,
        here: Here<'_>,
        runs: &mut dyn Runs,
        name: &str,
        record: &Value,
        machines: &[Reached],
    ) -> Vec<Carried> {
        let mut held = given(here.state, name);
        let mut targets: Vec<Reached> = held.iter().map(|(reached, _)| reached.clone()).collect();
        if stands(record) {
            targets.extend(machines.iter().cloned());
        }
        let gift = if stands(record) {
            Gift::Stop
        } else {
            Gift::Resume
        };
        let mut carried = Vec::new();
        let mut waiting = false;
        for target in one_each(&targets) {
            let holds = held
                .iter()
                .find(|(reached, _)| reached.machine == target.machine)
                .map(|(_, holds)| holds);
            if holds == Some(record) {
                continue;
            }
            if self.waits(&target.machine, here.at) {
                // A machine waiting out its minute after another stop is
                // noted as lacking this one as it stands, whether it holds
                // none or an older copy, as after Joel widens it, so Joel's
                // row names it until it takes it.
                let lacks = holds.is_none_or(|holds| !holds.is_null());
                if lacks {
                    held.retain(|(reached, _)| reached.machine != target.machine);
                    held.push((target.clone(), Value::Null));
                    waiting = true;
                }
                continue;
            }
            let failed = give(here.plan, runs, &target, &script(name, record, here.host)).err();
            self.took(&target.machine, here.at, failed.is_some());
            // One that answered it couldn't may still have written it, so it
            // is noted as holding something no record equals, given again
            // and given whatever follows.
            let holds = if failed.is_none() {
                record.clone()
            } else {
                Value::Null
            };
            held.retain(|(reached, _)| reached.machine != target.machine);
            held.push((target.clone(), holds));
            carried.push(Carried {
                name: name.to_owned(),
                machine: target.machine,
                gift,
                failed,
            });
        }
        if (waiting || !carried.is_empty())
            && let Err(error) = keep_given(here.state, name, &held)
        {
            carried.push(unnoted(name, gift, &error, !carried.is_empty()));
        }
        carried
    }

    /// Takes the stop kept under `name` away from each machine it was
    /// given to, since it is kept here no longer, and forgets each machine
    /// it was taken from.
    fn take_away(&mut self, here: Here<'_>, runs: &mut dyn Runs, name: &str) -> Vec<Carried> {
        let mut kept = Vec::new();
        let mut carried = Vec::new();
        for (target, holds) in given(here.state, name) {
            if self.waits(&target.machine, here.at) {
                kept.push((target, holds));
                continue;
            }
            let failed = give(here.plan, runs, &target, &taken_away(name)).err();
            self.took(&target.machine, here.at, failed.is_some());
            if failed.is_some() {
                kept.push((target.clone(), holds));
            }
            carried.push(Carried {
                name: name.to_owned(),
                machine: target.machine,
                gift: Gift::Removal,
                failed,
            });
        }
        if carried.is_empty() {
            return carried;
        }
        let noted = if kept.is_empty() {
            forget_given(here.state, name)
        } else {
            keep_given(here.state, name, &kept)
        };
        if let Err(error) = noted {
            carried.push(unnoted(name, Gift::Removal, &error, true));
        }
        carried
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

/// The script a machine runs to drop the stop it was given under `name`:
/// only a copy that names where it was carried from, never a stop of its
/// own reader's.
#[must_use]
pub fn taken_away(name: &str) -> String {
    format!(
        "set -e\n\
         echo '{begun}' >&2\n\
         f=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops/{name}.json\"\n\
         if grep -q '\"carried_from\"' \"$f\" 2>/dev/null; then rm -f \"$f\"; fi\n",
        begun = reach::BEGUN,
    )
}

/// Asks mycelium to run `script` on the machine `at` names.
fn give(plan: &Plan, runs: &mut dyn Runs, at: &Reached, script: &str) -> Result<(), String> {
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
        script.to_owned(),
    ]);
    runs.run(mycelium, &args)
        .map(|_| ())
        .map_err(|why| reach::without_begun(&why))
}

/// What a give answers when what each machine was given couldn't be noted:
/// it is given again next time, which writes the same record.
fn unnoted(name: &str, gift: Gift, error: &io::Error, gave: bool) -> Carried {
    let why = if gave {
        format!("given, but what was given couldn't be noted: {error}")
    } else {
        format!("the machines waiting to be given it couldn't be noted: {error}")
    };
    Carried {
        name: name.to_owned(),
        machine: String::new(),
        gift,
        failed: Some(why),
    }
}

/// Each live stop kept in `stops`, by its file name, beside its record:
/// none in shadow, none of Joel's stops on everything, none carried in
/// from another machine, and none under a name no reader keeps one under.
/// A record that can't be read is left out, as it is wherever stops are
/// acted on.
fn live(stops: &Path) -> Vec<(String, Value)> {
    let mut found: Vec<(String, Value)> = names(stops)
        .into_iter()
        .filter_map(|name| {
            let text = fs::read_to_string(stops.join(format!("{name}.json"))).ok()?;
            Some((name, serde_json::from_str::<Value>(&text).ok()?))
        })
        .filter(|(name, record)| {
            record["shadow"] == false
                && record.get("carried_from").is_none()
                && !name.starts_with(STOP_ALL)
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Each stop noted as given whose record is kept here no longer.
fn gone(state: &Path) -> Vec<String> {
    let stops = state.join("stops");
    let mut found: Vec<String> = names(&state.join(CARRIED))
        .into_iter()
        .filter(|name| !stops.join(format!("{name}.json")).exists())
        .collect();
    found.sort();
    found
}

/// The name of each `.json` file in `folder`, without it, that a record
/// is kept under (src/record.rs), so it stands in a script as it is.
fn names(folder: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
        .filter_map(|path| Some(path.file_stem()?.to_string_lossy().into_owned()))
        .filter(|name| plain(name))
        .collect()
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

/// Each machine once, as first found, since a machine keeps one stops
/// folder however many realms it joins.
fn one_each(machines: &[Reached]) -> Vec<Reached> {
    let mut each: Vec<Reached> = Vec::new();
    for machine in machines {
        if !each.iter().any(|one| one.machine == machine.machine) {
            each.push(machine.clone());
        }
    }
    each
}

/// What each machine was given of the stop `name`, as last noted; null
/// for one whose give failed, which may hold it or not.
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

/// Each machine that couldn't take the stop kept here under `name` when
/// last given it, by name, so Joel's row can say his agents there may not
/// have stopped; none once each has taken it.
#[must_use]
pub fn unreached(state: &Path, name: &str) -> Vec<String> {
    let mut machines: Vec<String> = given(state, name)
        .into_iter()
        .filter(|(_, holds)| holds.is_null())
        .map(|(reached, _)| reached.machine)
        .collect();
    machines.sort();
    machines.dedup();
    machines
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

/// Forgets what each machine was given of the stop `name`, once none
/// holds it.
fn forget_given(state: &Path, name: &str) -> io::Result<()> {
    match fs::remove_file(state.join(CARRIED).join(format!("{name}.json"))) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
