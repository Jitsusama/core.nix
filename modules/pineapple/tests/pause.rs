//! Joel's stop on everything, as before a demo: what it pauses on this
//! machine and asks of the others, and that a resume undoes exactly that.
//! The machine is a fake one here, answering systemctl and mycelium as
//! the real ones would, and writing down everything asked of it.

use std::fs;
use std::path::{Path, PathBuf};

use pineapple::pause::{self, Plan, Runs, Took};
use serde_json::Value;

/// A machine whose agents' timers, work and sessions are as given, which
/// writes down each program it's asked to run, and refuses whatever
/// `refusing` names.
#[derive(Default)]
struct Machine {
    asked: Vec<String>,
    timers: Vec<&'static str>,
    units: Vec<(&'static str, &'static str)>,
    refusing: Option<&'static str>,
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
            ["--user", "list-units", "--type=timer", ..] => self
                .timers
                .iter()
                .map(|timer| format!("{timer} loaded active waiting"))
                .collect::<Vec<_>>()
                .join("\n"),
            ["--user", "list-units", ..] => self
                .units
                .iter()
                .map(|(unit, _)| format!("{unit} loaded active running"))
                .collect::<Vec<_>>()
                .join("\n"),
            ["--user", "show", ..] => self
                .units
                .iter()
                .map(|(unit, slice)| format!("Id={unit}\nSlice={slice}\n"))
                .collect::<Vec<_>>()
                .join("\n"),
            ["sessions"] => r#"{"Ok":{"Sessions":[
                {"machine":"here"},{"machine":"here"},{"machine":"mac"}]}}"#
                .to_owned(),
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
        keep: vec!["sccache.service".to_owned(), "*demo*".to_owned()],
    }
}

fn machine() -> Machine {
    Machine {
        timers: vec!["cadence.timer", "cadence-day.timer"],
        units: vec![
            ("ox4wfv-gate.service", "jobs.slice"),
            ("pi-spawner-1-a.scope", "jobs.slice"),
            ("sccache.service", "jobs.slice"),
            ("reader-demo-1.service", "jobs.slice"),
            ("work-1-2.scope", "jobs-work-unranked.slice"),
            ("work-3-4.scope", "jobs-awaited.slice"),
            ("kitty-5-6.scope", "app.slice"),
            ("pineapple-slack.service", "background.slice"),
        ],
        ..Machine::default()
    }
}

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
        asked_with(
            &here,
            "systemctl --user stop cadence.timer cadence-day.timer"
        ),
        vec!["systemctl --user stop cadence.timer cadence-day.timer".to_owned()],
        "only the agents' timers running are stopped: {:?}",
        here.asked
    );
    let froze = asked_with(&here, "--user freeze");
    assert_eq!(
        froze,
        vec![
            "systemctl --user freeze ox4wfv-gate.service pi-spawner-1-a.scope jobs-work.slice"
                .to_owned()
        ],
        "agents' work is frozen, never Joel's own, the cache or the demo's: {:?}",
        here.asked
    );
    let mac = asked_with(&here, "mycelium run --need machine=mac -- sh -c");
    assert_eq!(mac.len(), 1, "the Mac is asked to stop: {:?}", here.asked);
    assert!(
        mac[0].contains("mycelium starts stop") && mac[0].contains("stop-all-1.json"),
        "the Mac writes the same stop and refuses its starts: {}",
        mac[0]
    );
    assert_eq!(
        took,
        vec![
            Took {
                machine: "here".to_owned(),
                failed: vec![]
            },
            Took {
                machine: "mac".to_owned(),
                failed: vec![]
            },
        ],
        "both machines took it"
    );
    assert_eq!(
        pause::said(
            "stopping",
            &took,
            pause::sessions(&plan(), &took, &mut here)
        ),
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
    let mut later = Machine::default();
    assert!(
        pause::settle(&plan(), &state, "here", 2, &mut later)
            .expect("settles")
            .is_empty(),
        "nothing is undone while the stop stands: {:?}",
        later.asked
    );
    assert_eq!(
        pause::stands(&state).as_deref(),
        Some("stop-all-1"),
        "the pause stands"
    );

    pineapple::resume::resume(&state.join("stops"), "all", 3, "joel").expect("resumed");
    let took = pause::settle(&plan(), &state, "here", 3, &mut later).expect("settles");
    assert_eq!(
        later.asked[..3],
        [
            "systemctl --user thaw ox4wfv-gate.service pi-spawner-1-a.scope jobs-work.slice"
                .to_owned(),
            "systemctl --user start cadence.timer cadence-day.timer".to_owned(),
            "mycelium starts resume".to_owned(),
        ],
        "what was paused is undone in reverse, and nothing more"
    );
    let mac = asked_with(&later, "mycelium run --need machine=mac");
    assert!(
        mac.len() == 1
            && mac[0].contains("mycelium starts resume")
            && mac[0].contains("\"by\":\"joel\""),
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
fn work_started_while_it_stands_is_frozen_and_thawed_with_the_rest() {
    let state = scratch("hold");
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    here.asked.clear();
    here.units.push(("1dzylj-probe.service", "jobs.slice"));
    let froze = pause::hold(&plan(), &state, &mut here).expect("holds");
    assert_eq!(
        froze,
        vec!["1dzylj-probe.service".to_owned()],
        "only the new one: {froze:?}"
    );
    assert_eq!(
        asked_with(&here, "--user freeze"),
        vec!["systemctl --user freeze 1dzylj-probe.service".to_owned()],
        "the new work is frozen: {:?}",
        here.asked
    );
    pineapple::resume::resume(&state.join("stops"), "all", 2, "joel").expect("resumed");
    let mut later = Machine::default();
    pause::settle(&plan(), &state, "here", 2, &mut later).expect("settles");
    assert!(
        later.asked[0].ends_with("jobs-work.slice 1dzylj-probe.service"),
        "it's thawed with the rest: {:?}",
        later.asked
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
fn a_machine_that_refuses_is_named_with_what_to_type_there() {
    let state = scratch("refused");
    let mut here = Machine {
        refusing: Some("machine=mac"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    assert_eq!(
        pause::said("stopping", &took, Some(2)),
        "stopping 2 sessions on here; mac couldn't: couldn't stop there: refused here, \
         so its agents run on",
        "Joel is told which machine refused and why"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn the_script_a_machine_runs_writes_the_stop_and_stops_its_starts() {
    let home = scratch("script");
    let bin = home.join("bin");
    fs::create_dir_all(&bin).expect("a bin folder");
    let mycelium = bin.join("mycelium");
    fs::write(&mycelium, "#!/bin/sh\necho \"$@\" > \"$HOME/asked\"\n").expect("a fake mycelium");
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&mycelium)
        .status()
        .expect("chmod runs");
    let said = pause::record("stop-all-1", "here", 1);
    let ran = std::process::Command::new("sh")
        .arg("-c")
        .arg(pause::script(&said, "stop"))
        .env("HOME", &home)
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
    let kept = json(&home.join(".local/state/pineapple/stops/stop-all-1.json"));
    assert_eq!(kept, said, "the machine keeps the same stop, whole");
    assert_eq!(
        fs::read_to_string(home.join("asked"))
            .expect("mycelium was asked")
            .trim(),
        "starts stop",
        "and refuses its agents' starts"
    );
    let _ = fs::remove_dir_all(&home);
}
