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
    /// Whether the stop was asked of it and may have run there, so its
    /// resume has something to undo; false while it is yet to be asked.
    pub asked: bool,
}

impl Reached {
    /// As what a stop paused keeps it.
    pub fn kept(&self) -> Value {
        json!({ "machine": self.machine, "realm": self.realm, "asked": self.asked })
    }

    /// From what a stop paused kept, as [`Reached::kept`] writes it; one
    /// kept before it was asked reads as not asked.
    pub fn from_kept(kept: &Value) -> Option<Reached> {
        Some(Reached {
            machine: kept["machine"].as_str()?.to_owned(),
            realm: kept["realm"].as_str().map(str::to_owned),
            asked: kept["asked"].as_bool().unwrap_or(false),
        })
    }
}

/// The status `mycelium run` ends with when it couldn't run the command
/// at all, as SSH's does, such as on a machine out of reach, one whose
/// mycelium is too old to run commands, or one whose link isn't proven.
/// It ends so too when the node hangs up on a run under way, which
/// [`BEGUN`] tells apart.
const DIDNT_RUN: &str = "(exit status: 255)";

/// What the script says on its error output before anything else, so a
/// run that failed after it began, as one whose link dropped, isn't taken
/// for one that never ran.
pub(super) const BEGUN: &str = "pineapple: begun there";

/// What the script ends with when the mycelium it asks there fails, so a
/// 255 of that mycelium's own can't read as the script never running.
const OWN_FAILURE: u8 = 70;

/// The folder a stop's run there starts in. Unnamed, a run asks for the
/// folder pineapple runs in here, Joel's home, which another machine may
/// hold under that name but keep from whoever runs there, such as a
/// room's own account, which can't start a shell in it. The root won't
/// do either, since a fenced run there refuses a folder holding the
/// home. The script reads and writes only by whole paths, so the shared
/// scratch folder, which every account may enter, serves on every
/// machine.
pub(super) const FOLDER: &str = "/tmp";

/// What `mycelium` is asked to run `script` on `machine`, in `realm` or
/// its own, in [`FOLDER`].
pub(super) fn run_there(machine: &str, realm: Option<&str>, script: String) -> Vec<String> {
    let mut args = asked(&["run"], realm);
    args.extend([
        "--need".to_owned(),
        format!("machine={machine}"),
        "--folder".to_owned(),
        FOLDER.to_owned(),
        "--".to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        script,
    ]);
    args
}

/// Whether `why` a run failed is mycelium saying the command never ran.
/// The script's word may share its line with the status, when nothing
/// was said after it.
pub(super) fn didnt_run(why: &str) -> bool {
    why.trim_end().ends_with(DIDNT_RUN)
        && !why.lines().any(|line| line.trim_start().starts_with(BEGUN))
}

/// `why` a run failed, without the script's word that it began.
pub(super) fn without_begun(why: &str) -> String {
    why.lines()
        .map(|line| {
            line.trim_start()
                .strip_prefix(BEGUN)
                .map_or(line, str::trim_start)
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
pub(super) fn asked(act: &[&str], realm: Option<&str>) -> Vec<String> {
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
            asked: false,
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
            asked: false,
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
    // A resume of a stop that never reached this machine has nothing to
    // undo here, and letting its starts go would undo Joel's own refusal.
    // So too for a realm whose starts the stop never refused, though it
    // reached another: only a stop made before each noted its realms says
    // nothing of which it refused, and is let go whole. Its note comes
    // before its record, so no stop cut off between them reads as older.
    // A resume looks for mycelium only once it has starts to let go, so
    // a machine with none, which the stop refused nothing on, ends well.
    let (unreached, note) = if act == "resume" {
        (
            format!("[ -e \"$d/{}.json\" ] || exit 0\n", file_of(id)),
            "",
        )
    } else {
        (String::new(), ": > \"$noted\"\n")
    };
    let starts = if act == "resume" {
        format!(
            "resumed=\"$d/{file}.resumed{mark}\"\n\
             if [ -e \"$resumed\" ] || [ -e \"$kept\" ] || \
             {{ [ -e \"$noted\" ] && [ ! -e \"$stopped\" ]; }}; then : > \"$resumed\"; \
             else found; \"$m\" starts resume{flag} >/dev/null || exit {OWN_FAILURE}; \
             : > \"$resumed\"; fi\n",
            file = file_of(id),
        )
    } else {
        format!(
            "found\n\
             if \"$m\" starts list{flag} | grep -q '\"stopped\":true'; then : > \"$kept\"; \
             else : > \"$stopped\"; \
             \"$m\" starts stop{flag} >/dev/null || exit {OWN_FAILURE}; fi\n"
        )
    };
    format!(
        "set -e\n\
         echo '{BEGUN}' >&2\n\
         d=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops\"\n\
         mkdir -p \"$d\"\n\
         kept=\"$d/{file}.starts-kept{mark}\"\n\
         stopped=\"$d/{file}.stopped{mark}\"\n\
         noted=\"$d/{file}.realms-noted\"\n\
         {unreached}\
         {note}\
         printf '%s\\n' {record} > \"$d/{file}.new\"\n\
         mv \"$d/{file}.new\" \"$d/{file}.json\"\n\
         m=\n\
         for c in \"$(command -v mycelium || true)\" \
         \"/etc/profiles/per-user/$(id -un)/bin/mycelium\" \
         \"$HOME/.nix-profile/bin/mycelium\"; do\n\
         if [ -n \"$c\" ] && [ -x \"$c\" ]; then m=$c; break; fi\n\
         done\n\
         found() {{ [ -n \"$m\" ] || {{ echo 'no mycelium here' >&2; exit 127; }}; }}\n\
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
            reached: false,
            realm: at.realm.clone(),
        };
    };
    let args = run_there(
        &at.machine,
        at.realm.as_deref(),
        script(said, act, at.realm.as_deref()),
    );
    let (failed, reached) = match runs.run(mycelium, &args) {
        Ok(_) => (Vec::new(), true),
        Err(why) => (
            vec![couldnt(&format!("{act} there"), &without_begun(&why))],
            !didnt_run(&why),
        ),
    };
    Took {
        machine: at.machine.clone(),
        recorded: failed.is_empty(),
        failed,
        reached,
        realm: at.realm.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::{BEGUN, didnt_run, without_begun};
    use crate::pause::{Runs, System};

    /// How a program ending as `line` says ends, run as the stop runs
    /// mycelium, fails.
    fn failed(line: &str) -> String {
        let mut system = System {
            limit: Duration::from_secs(10),
        };
        system
            .run(Path::new("sh"), &["-c".to_owned(), line.to_owned()])
            .expect_err("the program fails")
    }

    #[test]
    fn only_a_run_mycelium_never_began_reads_as_never_run() {
        let never = failed("echo 'mycelium: too old to run commands' >&2; exit 255");
        assert!(didnt_run(&never), "a 255 before the script began: {never}");
        let dropped = failed(&format!(
            "echo '{BEGUN}' >&2; echo 'mycelium: the node hung up' >&2; exit 255"
        ));
        assert!(
            !didnt_run(&dropped),
            "a 255 after the script began may have stopped it: {dropped}"
        );
        assert_eq!(
            without_begun(&dropped),
            "mycelium: the node hung up (exit status: 255)",
            "Joel is told why, not that it began"
        );
        let quiet = failed(&format!("echo '{BEGUN}' >&2; exit 255"));
        assert!(
            !didnt_run(&quiet),
            "a 255 after the script began reads so with nothing said after it: {quiet}"
        );
        assert_eq!(
            without_begun(&quiet),
            "(exit status: 255)",
            "and its status is kept"
        );
        let refused = failed("echo 'refused' >&2; exit 1");
        assert!(
            !didnt_run(&refused),
            "any other end may have run: {refused}"
        );
    }
}
