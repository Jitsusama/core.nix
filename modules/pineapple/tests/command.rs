//! pineapple, Joel's own command: it lists the stops standing, refuses to
//! resume one under pi, and resumes what a scope names elsewhere, marking
//! the record rather than removing it.
//!
//! Run from an agent's shell, which is under pi, a resume is refused
//! however the variables pi sets are cleared, and the case says so; run
//! as the package builds, out of pi's reach, it resumes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-command-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("pineapple/stops")).expect("a scratch state folder");
    fs::write(
        dir.join("pineapple/stops/slack-C1-1.0.json"),
        json!({
            "id": "slack:C1:1.0",
            "shadow": true,
            "said_by": "U0OTHER001",
            "where": "https://example.slack.com/archives/C1/p1",
            "reaches": ["pr:example/shop#12"],
            "resumed": null,
        })
        .to_string(),
    )
    .expect("a stop is written");
    dir
}

fn pineapple(state: &Path, args: &[&str], pi_says: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pineapple"));
    command
        .args(args)
        .env("XDG_STATE_HOME", state)
        // Never the machine's own plan, which pauses its real agents.
        .env("PINEAPPLE_PAUSE", state.join("no-plan.json"));
    for var in ["PI_SESSION_ID", "PI_BASH_CALL", "PI_HOST_CONVERSATION"] {
        command.env_remove(var);
    }
    if pi_says {
        command.env("PI_SESSION_ID", "a-session");
    }
    command.output().expect("pineapple runs")
}

fn record(state: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(state.join("pineapple/stops/slack-C1-1.0.json")).expect("kept"),
    )
    .expect("json")
}

#[test]
fn it_lists_the_stops_standing() {
    let state = scratch("list");
    let said = pineapple(&state, &["stops"], false);
    let out = String::from_utf8_lossy(&said.stdout);
    assert!(said.status.success(), "stops answers: {said:?}");
    assert!(
        out.contains("slack:C1:1.0  (noted only: nothing was stopped)"),
        "names it, saying a shadow stop stopped nothing: {out}"
    );
    assert!(
        out.contains("stops pr:example/shop#12"),
        "says what it reaches: {out}"
    );
}

#[test]
fn it_refuses_a_resume_where_pi_says_it_runs() {
    let state = scratch("refused");
    let said = pineapple(&state, &["resume", "all"], true);
    assert!(!said.status.success(), "refused: {said:?}");
    assert!(
        String::from_utf8_lossy(&said.stderr).contains("only Joel resumes a stop"),
        "says why: {said:?}"
    );
    assert_eq!(
        record(&state)["resumed"],
        Value::Null,
        "the stop still stands"
    );
}

#[test]
fn it_resumes_what_a_scope_names_out_of_pis_reach() {
    let state = scratch("resumed");
    let under_pi =
        std::env::var_os("PI_SESSION_ID").is_some() || in_agents_cgroup("the scoped resume");
    let missed = pineapple(&state, &["resume", "pr:example/other#1"], false);
    let said = pineapple(&state, &["resume", "pr:example/shop#12"], false);
    if under_pi {
        // An agent running this case is under pi, or where only agents
        // and services run, whatever it clears.
        assert!(!said.status.success(), "refused under pi: {said:?}");
        assert_eq!(
            record(&state)["resumed"],
            Value::Null,
            "the stop still stands"
        );
        return;
    }
    assert!(
        !missed.status.success(),
        "a scope naming nothing resumes nothing: {missed:?}"
    );
    assert!(said.status.success(), "resumes: {said:?}");
    assert_eq!(
        record(&state)["resumed"]["by"],
        "joel",
        "marked, not removed"
    );
    let left = pineapple(&state, &["stops"], false);
    assert!(
        String::from_utf8_lossy(&left.stdout).contains("no stop stands"),
        "nothing stands after: {left:?}"
    );
}

/// A pause plan naming a fake systemctl and mycelium, each writing down
/// what it was asked in `asked`, and the plan's path.
fn pause_plan(state: &Path) -> PathBuf {
    let asked = state.join("asked");
    let fake = |name: &str| {
        let path = state.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\necho \"{name} $*\" >> '{}'\n\
                 [ \"$1\" = sessions ] && echo '{{\"Ok\":{{\"Sessions\":[]}}}}'\nexit 0\n",
                asked.to_string_lossy()
            ),
        )
        .expect("a fake program");
        Command::new("chmod")
            .arg("+x")
            .arg(&path)
            .status()
            .expect("chmod runs");
        path
    };
    let plan = state.join("pause.json");
    fs::write(
        &plan,
        json!({
            "systemctl": fake("systemctl"),
            "mycelium": fake("mycelium"),
            "timers": ["cadence*.timer"],
        })
        .to_string(),
    )
    .expect("a plan");
    plan
}

fn stop_all(state: &Path, args: &[&str], pi_says: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pineapple"));
    command
        .args(args)
        .env("XDG_STATE_HOME", state)
        .env("PINEAPPLE_PAUSE", pause_plan(state));
    for var in ["PI_SESSION_ID", "PI_BASH_CALL", "PI_HOST_CONVERSATION"] {
        command.env_remove(var);
    }
    if pi_says {
        command.env("PI_SESSION_ID", "a-session");
    }
    command.output().expect("pineapple runs")
}

/// Whether this process runs where the command refuses, by the rule the
/// command reads, which its own cases in tests/pause.rs pin; said when it
/// does, so a case it cuts short isn't read as proof of the rest.
fn in_agents_cgroup(case: &str) -> bool {
    let cgroup = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let why = pineapple::pause::agents_cgroup(&cgroup);
    if let Some(why) = &why {
        eprintln!("{case} checks only the refusal here: {why}");
    }
    why.is_some()
}

fn everything_stops(state: &Path) -> Vec<Value> {
    fs::read_dir(state.join("pineapple/stops"))
        .expect("the stops")
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(|stop| stop["reaches"][0] == "everything")
        .collect()
}

#[test]
fn it_refuses_to_stop_all_where_pi_says_it_runs() {
    let state = scratch("stop-refused");
    let said = stop_all(&state, &["stop", "all"], true);
    assert_eq!(said.status.code(), Some(77), "refused: {said:?}");
    assert!(
        String::from_utf8_lossy(&said.stderr).contains("only Joel stops every agent"),
        "says why: {said:?}"
    );
    assert!(everything_stops(&state).is_empty(), "nothing was stopped");
    assert!(!state.join("asked").exists(), "nothing was paused");
}

#[test]
fn it_stops_all_and_resuming_undoes_it_out_of_pis_reach() {
    let state = scratch("stop-all");
    let said = stop_all(&state, &["stop", "all"], false);
    if std::env::var_os("PI_SESSION_ID").is_some() || in_agents_cgroup("stop all and resume") {
        // An agent running this case is under pi, or in a part of the
        // user's manager only agents and services run in, whatever it
        // clears; run it in a scope of its own beyond pi to see the rest.
        assert_eq!(said.status.code(), Some(77), "refused for agents: {said:?}");
        return;
    }
    let out = String::from_utf8_lossy(&said.stdout);
    assert!(said.status.success(), "stops: {said:?}");
    assert!(
        out.starts_with("stopping 0 sessions on "),
        "says so in one line: {out}"
    );
    assert!(
        out.contains("0 parked here; nothing starts"),
        "says how many parked here: {out}"
    );
    assert!(
        out.contains("nothing starts until pineapple resume all"),
        "says how it ends: {out}"
    );
    let stops = everything_stops(&state);
    assert!(
        stops.len() == 1 && stops[0]["by_joel"] == true && stops[0]["resumed"].is_null(),
        "one live stop on everything, Joel's: {stops:?}"
    );
    let asked = fs::read_to_string(state.join("asked")).expect("the machine was asked");
    assert!(
        asked.contains("mycelium starts stop"),
        "agents' starts are refused: {asked}"
    );

    let again = stop_all(&state, &["stop", "all"], false);
    assert!(
        again.status.success()
            && String::from_utf8_lossy(&again.stdout).contains("already stopped"),
        "a second stop says the first stands: {again:?}"
    );
    assert_eq!(everything_stops(&state).len(), 1, "and makes no second");

    let resumed = stop_all(&state, &["resume", "all"], false);
    assert!(resumed.status.success(), "resumes: {resumed:?}");
    assert!(
        String::from_utf8_lossy(&resumed.stdout).contains("resuming every session on "),
        "says what it undid: {resumed:?}"
    );
    let asked = fs::read_to_string(state.join("asked")).expect("the machine was asked");
    assert!(
        asked.contains("mycelium starts resume"),
        "agents may start again: {asked}"
    );
}
