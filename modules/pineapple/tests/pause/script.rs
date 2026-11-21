//! The script another machine runs for a stop or its resume, against a
//! fake mycelium on its path: what it writes there, which starts it
//! refuses or lets go, and how it ends.

use std::fs;
use std::path::Path;

use pineapple::pause;
use serde_json::Value;

use super::{FORKS, json, scratch};

/// Runs the script a machine runs for `act` of `said`, with a fake
/// mycelium on its path that writes down what it's asked and says starts
/// are already refused when `refused` is set, and answers what it asked.
fn script_run(home: &Path, said: &Value, act: &str, refused: bool) -> String {
    let (asked, ended) = script_ends(home, said, act, refused, 0);
    assert_eq!(ended, Some(0), "the script runs");
    asked
}

/// As [`script_run`], with the fake mycelium's refusing or letting go of
/// starts ending as `starts_end` says, answering what it asked and how
/// the script ended.
fn script_ends(
    home: &Path,
    said: &Value,
    act: &str,
    refused: bool,
    starts_end: u8,
) -> (String, Option<i32>) {
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
             echo '{{\"Ok\":{{\"Starts\":{{\"stopped\":{refused}}}}}}}' && exit 0\n\
             exit {starts_end}\n"
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
        .arg(pause::script(said, act, None))
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
    let asked = fs::read_to_string(home.join("asked")).unwrap_or_default();
    let _ = fs::remove_file(home.join("asked"));
    (asked, ran.status.code())
}

#[test]
fn the_script_resumes_nothing_where_the_stop_never_landed() {
    let home = scratch("script-unlanded");
    let said = pause::record("stop-all-1", "here", 1);
    assert_eq!(
        script_run(&home, &said, "resume", false),
        "",
        "starts Joel refused there himself stay refused"
    );
    assert!(
        !home
            .join(".local/state/pineapple/stops/stop-all-1.json")
            .exists(),
        "and no stop is written there by its resume"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn the_script_never_ends_as_mycelium_does_a_command_it_never_ran() {
    let said = pause::record("stop-all-1", "here", 1);
    let stopping = scratch("script-ends-stop");
    let (_, stop) = script_ends(&stopping, &said, "stop", false, 255);
    let resuming = scratch("script-ends-resume");
    script_run(&resuming, &said, "stop", false);
    let (_, resume) = script_ends(&resuming, &said, "resume", false, 255);
    assert_eq!(
        (stop, resume),
        (Some(70), Some(70)),
        "a mycelium there failing to refuse or let go of starts ends the script with its own \
         code, never mycelium's 255 for a command it never ran"
    );
    let _ = fs::remove_dir_all(&stopping);
    let _ = fs::remove_dir_all(&resuming);
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
    assert_eq!(
        script_run(&home, &said, "resume", true),
        "",
        "a resume tried again, its answer having come late, lets nothing go twice"
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
    assert_eq!(
        script_run(&home, &said, "resume", true),
        "",
        "nor on one tried again"
    );
    let _ = fs::remove_dir_all(&home);
}
