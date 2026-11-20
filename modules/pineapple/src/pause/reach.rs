//! What a stop reaches through mycelium, in each realm this machine joins:
//! agents' starts here, the sessions each machine runs, and the other
//! machines, each asked to take the stop or its resume by a script.

use serde_json::{Value, json};

use super::system::Runs;
use super::{Plan, Took, couldnt, file_of, quoted, step};

/// A machine in a realm, as a stop asks it: `realm` none for mycelium's
/// own default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reached {
    /// The machine, by mycelium's name.
    pub machine: String,
    /// The realm it was found in.
    pub realm: Option<String>,
}

impl Reached {
    /// As what a stop paused keeps it.
    pub fn kept(&self) -> Value {
        json!({ "machine": self.machine, "realm": self.realm })
    }

    /// From what a stop paused kept, as [`Reached::kept`] writes it.
    pub fn from_kept(kept: &Value) -> Option<Reached> {
        Some(Reached {
            machine: kept["machine"].as_str()?.to_owned(),
            realm: kept["realm"].as_str().map(str::to_owned),
        })
    }
}

/// The realms the plan names, or mycelium's own default when it names
/// none.
pub fn realms(plan: &Plan) -> Vec<Option<String>> {
    if plan.realms.is_empty() {
        return vec![None];
    }
    plan.realms.iter().cloned().map(Some).collect()
}

/// mycelium's words for `act` in `realm`.
fn asked(act: &[&str], realm: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = act.iter().map(|word| (*word).to_owned()).collect();
    if let Some(realm) = realm {
        args.extend(["--realm".to_owned(), realm.to_owned()]);
    }
    args
}

/// The realms whose agents' starts here this stop refuses: each not
/// already refused, as Joel may have refused them himself. A realm whose
/// starts mycelium can't say is refused, which is harmless twice.
pub fn stop_starts(plan: &Plan, runs: &mut dyn Runs, failed: &mut Vec<String>) -> Vec<String> {
    let Some(mycelium) = &plan.mycelium else {
        return Vec::new();
    };
    let mut stopped = Vec::new();
    for realm in realms(plan) {
        let already = runs
            .run(mycelium, &asked(&["starts", "list"], realm.as_deref()))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| value.pointer("/Ok/Starts/stopped").and_then(Value::as_bool))
            .unwrap_or(false);
        if already {
            continue;
        }
        let args = asked(&["starts", "stop"], realm.as_deref());
        // Kept to be let go even when refusing failed, since letting go
        // what was never refused does no harm.
        step(runs, mycelium, &args, "refuse agents' starts", failed);
        stopped.push(realm.unwrap_or_default());
    }
    stopped
}

/// Lets agents start again in each of `realms`, as [`stop_starts`] named
/// them, and answers those it couldn't.
pub fn resume_starts(
    plan: &Plan,
    runs: &mut dyn Runs,
    realms: &[String],
    failed: &mut Vec<String>,
) -> Vec<String> {
    let Some(mycelium) = &plan.mycelium else {
        return realms.to_vec();
    };
    realms
        .iter()
        .filter(|realm| {
            let realm = Some(realm.as_str()).filter(|realm| !realm.is_empty());
            let args = asked(&["starts", "resume"], realm);
            !step(runs, mycelium, &args, "let agents start", failed)
        })
        .cloned()
        .collect()
}

/// Each session mycelium lists in each realm that answers, once, and why
/// each realm that didn't couldn't: one realm's node being down hides
/// only that realm's machines.
fn listed(plan: &Plan, runs: &mut dyn Runs) -> (Vec<(Value, Option<String>)>, Vec<String>) {
    let Some(mycelium) = &plan.mycelium else {
        return (Vec::new(), Vec::new());
    };
    let mut found: Vec<(Value, Option<String>)> = Vec::new();
    let mut failed = Vec::new();
    for realm in realms(plan) {
        let answer = runs
            .run(mycelium, &asked(&["sessions"], realm.as_deref()))
            .and_then(|text| {
                serde_json::from_str::<Value>(&text).map_err(|error| error.to_string())
            });
        let value = match answer {
            Ok(value) => value,
            Err(why) => {
                let what = realm.as_deref().map_or_else(
                    || "list the sessions".to_owned(),
                    |realm| format!("list the sessions in {realm}"),
                );
                failed.push(couldnt(&what, &why));
                continue;
            }
        };
        for one in value
            .pointer("/Ok/Sessions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            // A session that ended is listed for a while, and runs nothing.
            if one["state"] == "ended" || found.iter().any(|(seen, _)| seen["id"] == one["id"]) {
                continue;
            }
            found.push((one.clone(), realm.clone()));
        }
    }
    (found, failed)
}

/// The other machines a stop reaches: those the plan names, and each one
/// mycelium lists a session running on, in whichever realm, since a
/// machine running a session runs agents; so no machine's name need be
/// written down for it to stop.
pub fn others(
    plan: &Plan,
    host: &str,
    runs: &mut dyn Runs,
    failed: &mut Vec<String>,
) -> Vec<Reached> {
    let mut machines: Vec<Reached> = plan
        .machines
        .iter()
        .map(|machine| Reached {
            machine: machine.clone(),
            realm: realms(plan).into_iter().next().flatten(),
        })
        .collect();
    let (sessions, unlisted) = listed(plan, runs);
    for why in unlisted {
        failed.push(format!(
            "{why}, so the machines running only those may run on"
        ));
    }
    for (one, realm) in sessions {
        let Some(machine) = one["machine"].as_str() else {
            continue;
        };
        let reached = Reached {
            machine: machine.to_owned(),
            realm,
        };
        if machine != host && !machines.contains(&reached) {
            machines.push(reached);
        }
    }
    machines
}

/// How many sessions mycelium lists running on `machines`, in every
/// realm, or none when it can't say.
pub fn sessions(plan: &Plan, machines: &[&str], runs: &mut dyn Runs) -> Option<usize> {
    session_ids(plan, machines, runs).map(|ids| ids.len())
}

/// Each session mycelium lists running on `machines`, in every realm, by
/// the id the harness writes as it parks one, or none when it can't say
/// for every realm.
pub fn session_ids(plan: &Plan, machines: &[&str], runs: &mut dyn Runs) -> Option<Vec<String>> {
    plan.mycelium.as_ref()?;
    let (sessions, failed) = listed(plan, runs);
    if !failed.is_empty() {
        return None;
    }
    Some(
        sessions
            .iter()
            .filter(|(one, _)| machines.contains(&one["machine"].as_str().unwrap_or_default()))
            .filter_map(|(one, _)| one["id"].as_str().map(str::to_owned))
            .collect(),
    )
}

/// The script a machine runs for `act`, stop or resume, of the stop
/// `said` in `realm`: its record written whole into that machine's
/// stops, then its starts refused or let go again, leaving them refused
/// on the resume when they were refused before the stop.
#[must_use]
pub fn script(said: &Value, act: &str, realm: Option<&str>) -> String {
    let id = said.get("id").and_then(Value::as_str).unwrap_or_default();
    let (flag, mark) = match realm {
        Some(realm) => (
            format!(" --realm {}", quoted(realm)),
            format!("-{}", file_of(realm)),
        ),
        None => (String::new(), String::new()),
    };
    // A resume whose answer came too late is tried again, so the first
    // that finishes says so, and one after it lets nothing go: the starts
    // Joel refused before the stop stay refused.
    let starts = if act == "resume" {
        format!(
            "resumed=\"$d/{file}.resumed{mark}\"\n\
             if [ -e \"$resumed\" ] || [ -e \"$kept\" ]; then : > \"$resumed\"; \
             else \"$m\" starts resume{flag} >/dev/null; : > \"$resumed\"; fi\n",
            file = file_of(id),
        )
    } else {
        format!(
            "if \"$m\" starts list{flag} | grep -q '\"stopped\":true'; then : > \"$kept\"; \
             else \"$m\" starts stop{flag} >/dev/null; fi\n"
        )
    };
    format!(
        "set -e\n\
         d=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops\"\n\
         mkdir -p \"$d\"\n\
         printf '%s\\n' {record} > \"$d/{file}.new\"\n\
         mv \"$d/{file}.new\" \"$d/{file}.json\"\n\
         kept=\"$d/{file}.starts-kept{mark}\"\n\
         m=\n\
         for c in \"$(command -v mycelium || true)\" \
         \"/etc/profiles/per-user/$(id -un)/bin/mycelium\" \
         \"$HOME/.nix-profile/bin/mycelium\"; do\n\
         if [ -n \"$c\" ] && [ -x \"$c\" ]; then m=$c; break; fi\n\
         done\n\
         [ -n \"$m\" ] || {{ echo 'no mycelium here' >&2; exit 127; }}\n\
         {starts}",
        record = quoted(&said.to_string()),
        file = file_of(id),
    )
}

/// The stop `said` asked of the machine `at` reaches, for `act`.
pub fn elsewhere(plan: &Plan, runs: &mut dyn Runs, at: &Reached, said: &Value, act: &str) -> Took {
    let Some(mycelium) = &plan.mycelium else {
        return Took {
            machine: at.machine.clone(),
            failed: vec!["this machine names no mycelium to reach it".to_owned()],
            recorded: false,
        };
    };
    let mut args = asked(&["run"], at.realm.as_deref());
    args.extend([
        "--need".to_owned(),
        format!("machine={}", at.machine),
        "--".to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        script(said, act, at.realm.as_deref()),
    ]);
    let mut failed = Vec::new();
    let done = step(runs, mycelium, &args, &format!("{act} there"), &mut failed);
    Took {
        machine: at.machine.clone(),
        failed,
        recorded: done,
    }
}
