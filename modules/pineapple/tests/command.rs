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
    command.args(args).env("XDG_STATE_HOME", state);
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
    let under_pi = std::env::var_os("PI_SESSION_ID").is_some();
    let missed = pineapple(&state, &["resume", "pr:example/other#1"], false);
    let said = pineapple(&state, &["resume", "pr:example/shop#12"], false);
    if under_pi {
        // An agent running this case is under pi, whatever it clears.
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
