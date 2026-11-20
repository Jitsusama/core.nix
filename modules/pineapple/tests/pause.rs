//! Joel's stop on everything, as before a demo: what it pauses on this
//! machine and asks of the others, and that a resume undoes exactly that.
//! The machine is a fake one here, answering systemctl and mycelium as
//! the real ones would, and writing down everything asked of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pineapple::pause::{self, Plan, Runs, Tended, Took};
use serde_json::Value;

/// A unit on the fake machine: its name, its slice, and whether it's
/// frozen.
struct Unit {
    id: &'static str,
    slice: &'static str,
    frozen: bool,
}

/// A machine whose agents' timers, work and sessions are as given, which
/// writes down each program it's asked to run, stops, starts, freezes and
/// thaws what it's asked to, and refuses whatever `refusing` names.
#[derive(Default)]
struct Machine {
    asked: Vec<String>,
    timers: Vec<String>,
    units: Vec<Unit>,
    refusing: Option<&'static str>,
    starts_stopped: bool,
}

impl Machine {
    fn unit(&mut self, id: &str) -> Option<&mut Unit> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    fn show(&self, names: &[&str]) -> String {
        names
            .iter()
            .map(
                |name| match self.units.iter().find(|unit| unit.id == *name) {
                    Some(unit) => format!(
                        "Id={}\nActiveState=active\nFreezerState={}\nSlice={}\n",
                        unit.id,
                        if unit.frozen { "frozen" } else { "running" },
                        unit.slice
                    ),
                    None => {
                        format!("Id={name}\nActiveState=inactive\nFreezerState=running\nSlice=\n")
                    }
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Runs for Machine {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        let asked = format!("{} {}", program.to_string_lossy(), args.join(" "));
        self.asked.push(asked.clone());
        if self.refusing.is_some_and(|refused| asked.contains(refused)) {
            return Err("refused here".to_owned());
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(match words.as_slice() {
            ["--user", "list-units", "--type=timer", "--state=active", ..] => self
                .timers
                .iter()
                .map(|timer| format!("{timer} loaded active waiting"))
                .collect::<Vec<_>>()
                .join("\n"),
            [
                "--user",
                "list-units",
                "--type=service,scope",
                "--state=running",
                ..,
            ] => self
                .units
                .iter()
                .map(|unit| format!("{} loaded active running", unit.id))
                .collect::<Vec<_>>()
                .join("\n"),
            ["--user", "show", _, names @ ..] => self.show(names),
            ["--user", "stop", names @ ..] => {
                self.timers.retain(|timer| !names.contains(&timer.as_str()));
                String::new()
            }
            ["--user", "start", names @ ..] => {
                self.timers
                    .extend(names.iter().map(|name| (*name).to_owned()));
                String::new()
            }
            ["--user", verb @ ("freeze" | "thaw"), names @ ..] => {
                for name in names {
                    if let Some(unit) = self.unit(name) {
                        unit.frozen = *verb == "freeze";
                    }
                }
                String::new()
            }
            ["sessions"] => r#"{"Ok":{"Sessions":[
                {"machine":"here"},{"machine":"here"},{"machine":"mac"}]}}"#
                .to_owned(),
            ["starts", "list"] => format!(
                r#"{{"Ok":{{"Starts":{{"stopped":{},"newest":[]}}}}}}"#,
                self.starts_stopped
            ),
            _ => String::new(),
        })
    }
}

fn plan() -> Plan {
    Plan {
        systemctl: PathBuf::from("systemctl"),
        mycelium: Some(PathBuf::from("mycelium")),
        timers: vec!["cadence*.timer".to_owned(), "pain-hear.timer".to_owned()],
        machines: vec!["mac".to_owned()],
        keep: vec![
            "sccache.service".to_owned(),
            "merge-net.service".to_owned(),
            "*demo*".to_owned(),
        ],
    }
}

fn running(id: &'static str, slice: &'static str) -> Unit {
    Unit {
        id,
        slice,
        frozen: false,
    }
}

fn machine() -> Machine {
    Machine {
        timers: vec!["cadence.timer".to_owned(), "cadence-day.timer".to_owned()],
        units: vec![
            running("ox4wfv-gate.service", "jobs.slice"),
            running("jobs.service", "jobs.slice"),
            running("sccache.service", "jobs.slice"),
            running("reader-demo-1.service", "jobs.slice"),
            running("timed.service", "jobs-timed.slice"),
            running("merge-net.service", "jobs-timed.slice"),
            running("work-1-2.scope", "jobs-work-unranked.slice"),
            running("aquifer-demo-3.service", "jobs-work-unranked.slice"),
            running("awaited.service", "jobs-awaited.slice"),
            running("work-3-4.scope", "jobs-awaited.slice"),
            running("kitty-5-6.scope", "app.slice"),
            running("pineapple-slack.service", "background.slice"),
        ],
        ..Machine::default()
    }
}

/// What a stop freezes on [`machine`]: agents' work, never Joel's own
/// terminal's, the cache, the merge checks or the demo's.
const FROZEN: &str = "ox4wfv-gate.service jobs.service timed.service work-1-2.scope";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-pause-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch state folder");
    dir
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("written")).expect("json")
}

fn asked_with(machine: &Machine, words: &str) -> Vec<String> {
    machine
        .asked
        .iter()
        .filter(|asked| asked.contains(words))
        .cloned()
        .collect()
}

/// Held while a case starts a program, and while one tends: a child holds
/// a copy of every file this process has open until it runs its program,
/// the pause folder's lock among them, and a tend finding it held skips
/// its turn, as it should on a machine but not in a case.
static FORKS: Mutex<()> = Mutex::new(());

fn tend(state: &Path, at: u64, machine: &mut Machine) -> Tended {
    let _held = FORKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pause::tend(&plan(), state, "here", at, machine).expect("tends")
}

fn resumed(state: &Path, at: u64) {
    pineapple::resume::resume(&state.join("stops"), "all", at, "joel").expect("resumed");
}

#[test]
fn a_stop_says_everything_stops_and_pauses_only_agents_work() {
    let state = scratch("stop");
    let mut here = machine();
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");

    let record = json(&state.join("stops/stop-all-1.json"));
    assert_eq!(
        record["reaches"][0], "everything",
        "the stop reaches everything: {record}"
    );
    assert_eq!(record["shadow"], false, "Joel's own stop is live: {record}");
    assert_eq!(record["by_joel"], true, "Joel said it: {record}");
    assert!(
        record["resumed"].is_null(),
        "it stands until he resumes: {record}"
    );

    assert!(
        here.asked
            .iter()
            .any(|asked| asked == "mycelium starts stop"),
        "agents' starts are refused: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "systemctl --user stop "),
        vec!["systemctl --user stop cadence.timer cadence-day.timer".to_owned()],
        "only the agents' timers running are stopped: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec![format!("systemctl --user freeze {FROZEN}")],
        "agents' work is frozen, never Joel's own, the cache, the merge checks or the \
         demo's: {:?}",
        here.asked
    );
    let mac = asked_with(&here, "mycelium run --need machine=mac -- sh -c");
    assert_eq!(mac.len(), 1, "the Mac is asked to stop: {:?}", here.asked);
    assert!(
        mac[0].contains("starts stop") && mac[0].contains("stop-all-1.json"),
        "the Mac writes the same stop and refuses its starts: {}",
        mac[0]
    );
    assert_eq!(
        took,
        vec![
            Took {
                machine: "here".to_owned(),
                failed: vec![],
                recorded: true,
            },
            Took {
                machine: "mac".to_owned(),
                failed: vec![],
                recorded: true,
            },
        ],
        "both machines took it"
    );
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping 3 sessions on here and mac",
        "Joel is told in one line"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_resume_undoes_exactly_what_its_stop_paused_and_only_once() {
    let state = scratch("undo");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    assert!(
        pause::settle(&plan(), &state, "here", 2, &mut here)
            .expect("settles")
            .is_empty(),
        "nothing is undone while the stop stands: {:?}",
        here.asked
    );
    assert_eq!(
        pause::stands(&state).as_deref(),
        Some("stop-all-1"),
        "the pause stands"
    );

    resumed(&state, 3);
    let took = pause::settle(&plan(), &state, "here", 3, &mut here).expect("settles");
    assert_eq!(
        here.asked[..3],
        [
            format!("systemctl --user thaw {FROZEN}"),
            "systemctl --user start cadence.timer cadence-day.timer".to_owned(),
            "mycelium starts resume".to_owned(),
        ],
        "what was paused is undone in reverse, and nothing more"
    );
    let mac = asked_with(&here, "mycelium run --need machine=mac");
    assert!(
        mac.len() == 1 && mac[0].contains("starts resume") && mac[0].contains("\"by\":\"joel\""),
        "the Mac is given the resumed record and lets its agents start: {mac:?}"
    );
    assert_eq!(took.len(), 2, "both machines are said: {took:?}");
    assert!(
        pause::stands(&state).is_none(),
        "the pause no longer stands"
    );

    let mut again = Machine::default();
    let took = pause::settle(&plan(), &state, "here", 4, &mut again).expect("settles");
    assert!(
        took.is_empty() && again.asked.is_empty(),
        "an undone pause isn't undone again: {:?}",
        again.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn starts_joel_refused_before_the_stop_stay_refused_after_it() {
    let state = scratch("starts");
    let mut here = Machine {
        starts_stopped: true,
        ..machine()
    };
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    resumed(&state, 2);
    pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    assert!(
        asked_with(&here, "mycelium starts ")
            .iter()
            .all(|asked| asked == "mycelium starts list"),
        "starts already refused are neither refused again nor let go: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn work_started_or_thawed_while_it_stands_is_frozen_and_thawed_with_the_rest() {
    let state = scratch("hold");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    here.units
        .push(running("1dzylj-probe.service", "jobs.slice"));
    here.unit("timed.service").expect("in the fixture").frozen = false;
    let tended = tend(&state, 2, &mut here);
    assert_eq!(
        tended.froze,
        vec![
            "timed.service".to_owned(),
            "1dzylj-probe.service".to_owned()
        ],
        "only what isn't frozen: {tended:?}"
    );
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec!["systemctl --user freeze timed.service 1dzylj-probe.service".to_owned()],
        "it's frozen again: {:?}",
        here.asked
    );
    resumed(&state, 3);
    here.asked.clear();
    tend(&state, 3, &mut here);
    assert_eq!(
        here.asked.first().map(String::as_str),
        Some(format!("systemctl --user thaw {FROZEN} 1dzylj-probe.service").as_str()),
        "it's thawed with the rest, once the stop is resumed: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_step_a_resume_cannot_take_is_tried_again_after_a_minute() {
    let state = scratch("retry");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    resumed(&state, 2);
    here.refusing = Some("start cadence");
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    assert_eq!(
        pause::said("resuming", &took, None),
        "resuming every session on here and mac; here couldn't start agents' timers: \
         refused here, tried again each minute",
        "Joel is told what's left and that it's tried again"
    );
    here.asked.clear();
    here.refusing = None;
    tend(&state, 3, &mut here);
    assert!(
        here.asked.is_empty(),
        "it isn't tried again within the minute: {:?}",
        here.asked
    );
    tend(&state, 2 + 60_000, &mut here);
    assert_eq!(
        here.asked,
        vec!["systemctl --user start cadence.timer cadence-day.timer".to_owned()],
        "only what was left is tried again"
    );
    here.asked.clear();
    tend(&state, 3 + 120_000, &mut here);
    assert!(
        here.asked.is_empty(),
        "and then it's done: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_stop_that_cannot_list_agents_work_says_so() {
    let state = scratch("unlisted");
    let mut here = Machine {
        refusing: Some("list-units"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    assert_eq!(
        pause::said("stopping", &took, Some(3)),
        "stopping 3 sessions on here and mac; here couldn't list agents' timers: refused \
         here, couldn't list agents' work: refused here",
        "Joel is told what wasn't paused"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_running_sessions_stops_though_no_plan_names_it() {
    let state = scratch("unnamed");
    let mut here = machine();
    let unnamed = Plan {
        machines: vec![],
        ..plan()
    };
    let took = pause::stop(&unnamed, &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let reached: Vec<&str> = took.iter().map(|one| one.machine.as_str()).collect();
    assert_eq!(
        reached,
        ["here", "mac"],
        "the Mac runs a session, so it stops too"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_that_refuses_is_named_with_why() {
    let state = scratch("refused");
    let mut here = Machine {
        refusing: Some("machine=mac"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping 2 sessions on here; mac couldn't stop there: refused here, \
         so its agents may run on",
        "Joel is told which machine refused and why, and only its sessions are left out"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn only_a_stop_whose_record_stands_is_standing() {
    let state = scratch("stray");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    fs::write(state.join("paused/stop-all-1.new"), "{\"id\":\"half").expect("a stray");
    fs::write(state.join("paused/notes.txt"), "{\"id\":\"other\"}").expect("a stray");
    assert_eq!(
        pause::stands(&state).as_deref(),
        Some("stop-all-1"),
        "stray files are passed over"
    );
    resumed(&state, 2);
    assert!(
        pause::stands(&state).is_none(),
        "a stop resumed doesn't stand, though its pause isn't undone yet"
    );
    let _ = fs::remove_dir_all(&state);
}

/// Runs the script a machine runs for `act` of `said`, with a fake
/// mycelium on its path that writes down what it's asked and says starts
/// are already refused when `refused` is set, and answers what it asked.
fn script_run(home: &Path, said: &Value, act: &str, refused: bool) -> String {
    let _held = FORKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let bin = home.join("bin");
    fs::create_dir_all(&bin).expect("a bin folder");
    let mycelium = bin.join("mycelium");
    fs::write(
        &mycelium,
        format!(
            "#!/bin/sh\necho \"$@\" >> \"$HOME/asked\"\n\
             [ \"$1 $2\" = 'starts list' ] &&\n\
             echo '{{\"Ok\":{{\"Starts\":{{\"stopped\":{refused}}}}}}}'\n\
             exit 0\n"
        ),
    )
    .expect("a fake mycelium");
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&mycelium)
        .status()
        .expect("chmod runs");
    let ran = std::process::Command::new("sh")
        .arg("-c")
        .arg(pause::script(said, act))
        .env("HOME", home)
        .env_remove("XDG_STATE_HOME")
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.to_string_lossy(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("sh runs");
    assert!(ran.status.success(), "the script runs: {ran:?}");
    let asked = fs::read_to_string(home.join("asked")).unwrap_or_default();
    let _ = fs::remove_file(home.join("asked"));
    asked
}

#[test]
fn the_script_a_machine_runs_writes_the_stop_and_stops_its_starts() {
    let home = scratch("script");
    let said = pause::record("stop-all-1", "here", 1);
    assert_eq!(
        script_run(&home, &said, "stop", false),
        "starts list\nstarts stop\n",
        "it refuses its agents' starts"
    );
    let kept = json(&home.join(".local/state/pineapple/stops/stop-all-1.json"));
    assert_eq!(kept, said, "the machine keeps the same stop, whole");
    assert_eq!(
        script_run(&home, &said, "resume", false),
        "starts resume\n",
        "and lets them go again on the resume"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn the_script_leaves_starts_refused_there_before_it_refused() {
    let home = scratch("script-kept");
    let said = pause::record("stop-all-1", "here", 1);
    assert_eq!(
        script_run(&home, &said, "stop", true),
        "starts list\n",
        "starts already refused aren't refused again"
    );
    assert_eq!(
        script_run(&home, &said, "resume", true),
        "",
        "nor let go on the resume"
    );
    let _ = fs::remove_dir_all(&home);
}
