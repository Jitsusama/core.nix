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
//! there; one that never held it is given nothing. A stop no longer kept
//! here, such as one read from an answer that couldn't be read, forgotten
//! once the answer could be, is taken away from each machine given it.
//! The machines lacking a stop are given it sixteen at a time
//! (src/pause/system.rs), so one that doesn't answer holds none of its
//! lot up past their limit, and a later lot up by that limit at most. A
//! machine that couldn't take what
//! it was given is tried again after a minute, never sooner, for every
//! stop at once, so one out of reach costs a run a minute.
//!
//! A machine is gone from the mesh while a give to it never runs there
//! and mycelium lists no session on it, as a cloud room released or a
//! Mac asleep is, so whom it lists is asked once a minute at most while
//! one may be gone, a stop standing or not. Once one has been gone for
//! an hour, it's parked for what follows a stop: noted as gone beside
//! what it may hold, and asked nothing more till mycelium lists it
//! again, when it's given what it lacks, so a room released costs a
//! listing a minute and no runs, and a Mac that wakes is given its
//! resume at once. A stop standing is never parked, since one coming
//! back with agents must be stopped. One that runs a give, or is listed
//! again, is no longer taken for gone.
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

/// How long a machine is gone from the mesh, in milliseconds, before it's
/// parked for what follows a stop: longer than a link takes to be made
/// again or a node to restart, so one briefly away is still tried each
/// minute, and short against a day of a run a minute to a room that was
/// released.
pub const GONE_MS: u64 = 60 * 60 * 1000;

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
    /// What became of the machine gone from the mesh, if it is or was.
    pub away: Option<Away>,
}

/// What a give says of a machine gone from the mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Away {
    /// Gone for [`GONE_MS`], so it's asked nothing more of the stop until
    /// mycelium lists it again.
    Parked,
    /// Listed again once parked, so it's given what it lacks.
    Back,
}

/// What a machine was given of a stop, as noted under `carried/`: null
/// for one whose give failed, which may hold it or not; and since when
/// it has been gone from the mesh, in milliseconds, once it's parked.
#[derive(Clone, Debug)]
struct Held {
    reached: Reached,
    holds: Value,
    gone_since: Option<u64>,
}

/// Why a machine couldn't take what it was given, said plainly, and
/// whether the run never began there.
struct Untaken {
    why: String,
    never_ran: bool,
}

/// Gives each machine the stops it lacks, remembering when to try again a
/// machine that couldn't take one, and since when one has been gone.
#[derive(Debug, Default)]
pub struct Carrier {
    /// When each machine that couldn't take a stop may be tried again, by
    /// its name.
    again: BTreeMap<String, u64>,
    /// When each machine was first found gone from the mesh, by its name:
    /// a give to it never ran there, and mycelium listed no session on it.
    gone_since: BTreeMap<String, u64>,
    /// The machines mycelium last listed in every realm, and when.
    listed: Option<(u64, Vec<String>)>,
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
        let held: Vec<Held> = stops
            .iter()
            .map(|(name, _)| name)
            .chain(&gone)
            .flat_map(|name| given(state, name))
            .collect();
        // Only a machine still held is remembered as gone.
        self.gone_since
            .retain(|machine, _| held.iter().any(|one| one.reached.machine == *machine));
        let machines = if stops.iter().any(|(_, record)| stands(record)) || self.unsure(&held, at) {
            self.list(plan, host, at, runs, &mut listing)
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

    /// The machines mycelium listed in every realm within the minute, or
    /// none when it wasn't asked, or not every realm answered.
    fn listed_lately(&self, at: u64) -> Option<&[String]> {
        let (when, names) = self.listed.as_ref()?;
        (at < when + AGAIN_MS).then_some(names.as_slice())
    }

    /// Whether mycelium, asked within the minute, listed `machine` as
    /// running a session; none when it can't say.
    fn listed_now(&self, machine: &str, at: u64) -> Option<bool> {
        self.listed_lately(at)
            .map(|names| names.iter().any(|name| name == machine))
    }

    /// Whether a machine `held` notes as given a stop may be gone, with
    /// mycelium not asked within the minute whom it lists: one parked, or
    /// one to be tried now whose give failed or that was found gone.
    fn unsure(&self, held: &[Held], at: u64) -> bool {
        self.listed_lately(at).is_none()
            && held.iter().any(|one| {
                let machine = &one.reached.machine;
                one.gone_since.is_some()
                    || ((one.holds.is_null()
                        || self.again.contains_key(machine)
                        || self.gone_since.contains_key(machine))
                        && !self.waits(machine, at))
            })
    }

    /// The other machines a stop reaches, as [`reach::others`] lists them;
    /// when every realm answered, each is noted as listed at `at`, and no
    /// longer taken for gone.
    fn list(
        &mut self,
        plan: &Plan,
        host: &str,
        at: u64,
        runs: &mut dyn Runs,
        listing: &mut Vec<String>,
    ) -> Vec<Reached> {
        let unlisted = listing.len();
        let machines = reach::others(plan, host, runs, listing);
        self.listed = None;
        if listing.len() == unlisted {
            let names: Vec<String> = machines.iter().map(|one| one.machine.clone()).collect();
            self.gone_since
                .retain(|machine, _| !names.contains(machine));
            self.listed = Some((at, names));
        }
        machines
    }

    /// Notes how `machine` took what it was given at `at`, and answers
    /// since when it has been gone from the mesh, once that's
    /// [`GONE_MS`] or more: the give never ran there, mycelium listed no
    /// session on it within the minute, and neither has been otherwise
    /// since it was first found gone.
    fn took(&mut self, machine: &str, at: u64, answer: &Result<(), Untaken>) -> Option<u64> {
        if answer.is_ok() {
            self.again.remove(machine);
        } else {
            self.again.insert(machine.to_owned(), at + AGAIN_MS);
        }
        let never_ran = answer.as_ref().is_err_and(|untaken| untaken.never_ran);
        if !never_ran {
            self.gone_since.remove(machine);
            return None;
        }
        if self.listed_now(machine, at) != Some(false) {
            return None;
        }
        let since = *self.gone_since.entry(machine.to_owned()).or_insert(at);
        (at >= since + GONE_MS).then_some(since)
    }

    /// Each machine parked in `held` that mycelium lists again at `at`,
    /// no longer parked, so it's given what it lacks.
    fn back(&self, held: &mut [Held], at: u64) -> Vec<String> {
        held.iter_mut()
            .filter(|one| {
                one.gone_since.is_some() && self.listed_now(&one.reached.machine, at) == Some(true)
            })
            .map(|one| {
                one.gone_since = None;
                one.reached.machine.clone()
            })
            .collect()
    }

    /// Gives the stop `record`, kept here under `name`, to each machine
    /// lacking it as it stands: each of `machines` while it stands, and
    /// each that may hold it already, whatever it has become, but one
    /// parked.
    fn give_each(
        &mut self,
        here: Here<'_>,
        runs: &mut dyn Runs,
        name: &str,
        record: &Value,
        machines: &[Reached],
    ) -> Vec<Carried> {
        let mut held = given(here.state, name);
        let back = self.back(&mut held, here.at);
        let mut targets: Vec<Reached> = held.iter().map(|one| one.reached.clone()).collect();
        if stands(record) {
            targets.extend(machines.iter().cloned());
        }
        let gift = if stands(record) {
            Gift::Stop
        } else {
            Gift::Resume
        };
        let lacking: Vec<Reached> = one_each(&targets)
            .into_iter()
            .filter(|target| {
                let noted = held
                    .iter()
                    .find(|one| one.reached.machine == target.machine);
                noted.is_none_or(|one| one.holds != *record && one.gone_since.is_none())
                    && !self.waits(&target.machine, here.at)
            })
            .collect();
        let answers = give(here.plan, runs, &lacking, &script(name, record, here.host));
        let mut carried = Vec::new();
        for (target, answer) in lacking.into_iter().zip(answers) {
            let gone = self.took(&target.machine, here.at, &answer);
            let failed = answer.err().map(|untaken| untaken.why);
            // One that answered it couldn't may still have written it, so it
            // is noted as holding something no record equals, given again
            // and given whatever follows. One long gone is parked, unless
            // the stop stands, since one coming back with agents must be
            // stopped.
            let holds = if failed.is_none() {
                record.clone()
            } else {
                Value::Null
            };
            let gone_since = gone.filter(|_| gift != Gift::Stop);
            held.retain(|one| one.reached.machine != target.machine);
            held.push(Held {
                reached: target.clone(),
                holds,
                gone_since,
            });
            carried.push(Carried {
                name: name.to_owned(),
                away: away(gone_since, &back, &target.machine),
                machine: target.machine,
                gift,
                failed,
            });
        }
        if !carried.is_empty()
            && let Err(error) = keep_given(here.state, name, &held)
        {
            carried.push(unnoted(name, gift, &error));
        }
        carried
    }

    /// Takes the stop kept under `name` away from each machine it was
    /// given to, since it is kept here no longer, and forgets each machine
    /// it was taken from; one parked keeps it till it's listed again.
    fn take_away(&mut self, here: Here<'_>, runs: &mut dyn Runs, name: &str) -> Vec<Carried> {
        let mut held = given(here.state, name);
        let back = self.back(&mut held, here.at);
        let (mut kept, taking): (Vec<Held>, Vec<Held>) = held
            .into_iter()
            .partition(|one| one.gone_since.is_some() || self.waits(&one.reached.machine, here.at));
        let targets: Vec<Reached> = taking.iter().map(|one| one.reached.clone()).collect();
        let answers = give(here.plan, runs, &targets, &taken_away(name));
        let mut carried = Vec::new();
        for (one, answer) in taking.into_iter().zip(answers) {
            let gone_since = self.took(&one.reached.machine, here.at, &answer);
            let failed = answer.err().map(|untaken| untaken.why);
            let machine = one.reached.machine.clone();
            if failed.is_some() {
                kept.push(Held { gone_since, ..one });
            }
            carried.push(Carried {
                name: name.to_owned(),
                away: away(gone_since, &back, &machine),
                machine,
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
            carried.push(unnoted(name, Gift::Removal, &error));
        }
        carried
    }
}

/// What a give says of `machine` gone from the mesh: parked, when it has
/// been gone since `gone_since`, or back, when it's among those `back`.
fn away(gone_since: Option<u64>, back: &[String], machine: &str) -> Option<Away> {
    if gone_since.is_some() {
        Some(Away::Parked)
    } else if back.iter().any(|one| one == machine) {
        Some(Away::Back)
    } else {
        None
    }
}

/// The stops folder a machine built as a mycelium node of its own keeps,
/// as a cloud room does: each session's stops folder there is linked to
/// it, so its guards read it rather than any account's state folder.
pub const NODE_STOPS: &str = "/var/lib/pineapple/stops";

/// Shell setting `d` to the stops folder a machine's guards read: the
/// node's own where it keeps one, and this account's state folder
/// otherwise. Where the node's can't be written, the script fails, so a
/// stop is never said to be held where no guard reads it.
pub(super) fn stops_folder() -> String {
    format!(
        "d={NODE_STOPS}\n\
         [ -d \"$d\" ] || d=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops\"\n"
    )
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
         {folder}\
         mkdir -p \"$d\"\n\
         printf '%s\\n' {record} > \"$d/{name}.new\"\n\
         mv \"$d/{name}.new\" \"$d/{name}.json\"\n",
        begun = reach::BEGUN,
        folder = stops_folder(),
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
         {folder}\
         f=\"$d/{name}.json\"\n\
         if grep -q '\"carried_from\"' \"$f\" 2>/dev/null; then rm -f \"$f\"; fi\n",
        begun = reach::BEGUN,
        folder = stops_folder(),
    )
}

/// Asks mycelium to run `script` on each machine `at` names, sixteen at
/// a time, so one out of reach holds the others up by a run's limit at
/// most; answers each in the order named.
fn give(
    plan: &Plan,
    runs: &mut dyn Runs,
    at: &[Reached],
    script: &str,
) -> Vec<Result<(), Untaken>> {
    if at.is_empty() {
        return Vec::new();
    }
    let Some(mycelium) = &plan.mycelium else {
        let unnamed = || Untaken {
            why: "this machine names no mycelium to reach it".to_owned(),
            never_ran: false,
        };
        return at.iter().map(|_| Err(unnamed())).collect();
    };
    let asked: Vec<Vec<String>> = at
        .iter()
        .map(|one| reach::run_there(&one.machine, one.realm.as_deref(), script.to_owned()))
        .collect();
    let each: Vec<(&Path, &[String])> = asked
        .iter()
        .map(|args| (mycelium.as_path(), args.as_slice()))
        .collect();
    let answers = runs.run_all(&each);
    begun_elsewhere(mycelium, runs, at, script, answers)
        .into_iter()
        .map(|answer| {
            answer.map(|_| ()).map_err(|why| Untaken {
                never_ran: reach::didnt_run(&why),
                why: reach::without_begun(&why),
            })
        })
        .collect()
}

/// `answers` to runs of `script` on each of `at`, with each machine that
/// refused to begin it there asked once more, all together, to begin it
/// in the scratch folder, its second answer kept in place of its first.
fn begun_elsewhere(
    mycelium: &Path,
    runs: &mut dyn Runs,
    at: &[Reached],
    script: &str,
    mut answers: Vec<Result<String, String>>,
) -> Vec<Result<String, String>> {
    let refused: Vec<usize> = answers
        .iter()
        .enumerate()
        .filter(|(_, answer)| {
            answer
                .as_ref()
                .is_err_and(|why| reach::refused_to_begin(why))
        })
        .map(|(index, _)| index)
        .collect();
    if refused.is_empty() {
        return answers;
    }
    let asked: Vec<Vec<String>> = refused
        .iter()
        .map(|&index| {
            let one = &at[index];
            reach::run_in_scratch(&one.machine, one.realm.as_deref(), script.to_owned())
        })
        .collect();
    let each: Vec<(&Path, &[String])> = asked
        .iter()
        .map(|args| (mycelium, args.as_slice()))
        .collect();
    for (index, answer) in refused.into_iter().zip(runs.run_all(&each)) {
        answers[index] = answer;
    }
    answers
}

/// What a give answers when what each machine was given couldn't be noted:
/// it is given again next time, which writes the same record.
fn unnoted(name: &str, gift: Gift, error: &io::Error) -> Carried {
    Carried {
        name: name.to_owned(),
        machine: String::new(),
        gift,
        failed: Some(format!("couldn't note what was given: {error}")),
        away: None,
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

/// What each machine was given of the stop `name`, as last noted.
fn given(state: &Path, name: &str) -> Vec<Held> {
    let kept = fs::read_to_string(state.join(CARRIED).join(format!("{name}.json")))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    kept.as_ref()
        .and_then(|kept| kept["machines"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|one| {
            Some(Held {
                reached: Reached::from_kept(one)?,
                holds: one.get("holds")?.clone(),
                gone_since: one["gone_since"].as_u64(),
            })
        })
        .collect()
}

/// Notes what each machine was given of the stop `name`, written whole
/// and renamed into place.
fn keep_given(state: &Path, name: &str, given: &[Held]) -> io::Result<()> {
    let folder = state.join(CARRIED);
    fs::create_dir_all(&folder)?;
    let machines: Vec<Value> = given
        .iter()
        .map(|held| {
            let mut one = held.reached.kept();
            one["holds"] = held.holds.clone();
            if let Some(since) = held.gone_since {
                one["gone_since"] = json!(since);
            }
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
