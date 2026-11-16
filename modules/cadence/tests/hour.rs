//! `cadence hour` over a stand-in account holding one of Joel's misses,
//! whose check names a repository the run was given no checkout of: only
//! the run that sends the card may keep the hour or owe anyone its drag,
//! since a look by hand, missing the timer's checkouts, reads differently.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// One miss whose check merged in World, which a run without World's
/// checkout can't see, so it reads the miss as open.
const MISS: &str = r#"{"at": "2026-11-14T11:15:00-05:00", "words": "the notice stayed up", "what": "the backup notice stayed up after later runs succeeded", "owner": "88LQC0", "check": "world #2450001"}"#;

/// `cadence hour` with `args` over the account in `folder`, with nothing it
/// reads by command able to run, so only the files there count.
fn hour(folder: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cadence"))
        .arg("hour")
        .args(args)
        .env("HOME", folder)
        .env("XDG_STATE_HOME", folder.join("state"))
        .env("XDG_DATA_HOME", folder.join("data"))
        .env("PATH", "")
        .output()
        .expect("cadence runs")
}

fn account() -> tempfile::TempDir {
    let account = tempfile::tempdir().expect("a stand-in account");
    let mission = account.path().join("state/mission");
    fs::create_dir_all(&mission).expect("a mission folder");
    fs::write(mission.join("misses.jsonl"), format!("{MISS}\n")).expect("the misses");
    fs::create_dir_all(
        account
            .path()
            .join("state/pi/agentic-harness.pi/mesh-workflow/messages"),
    )
    .expect("the mesh's messages folder");
    account
}

/// Every file under `folder`, by its path inside it.
fn files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(at) = folders.pop() {
        for entry in fs::read_dir(&at).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if let Ok(inside) = path.strip_prefix(folder) {
                found.push(inside.display().to_string());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_look_by_hand_prints_the_card_and_keeps_and_tells_nothing() {
    let account = account();
    let before = files(account.path());

    let ran = hour(account.path(), &[]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let card = String::from_utf8_lossy(&ran.stdout);
    assert!(
        card.contains("the backup notice stayed up after later runs succeeded · 88LQC0"),
        "the look didn't print the open miss: {card}"
    );
    assert_eq!(
        files(account.path()),
        before,
        "a look by hand kept the hour, owed its drag or told someone, which only the \
         run sending the card may"
    );
}

#[test]
fn the_run_sending_the_card_keeps_the_hour_and_owes_its_drag() {
    let account = account();

    let ran = hour(account.path(), &["--to", "WCJQSG"]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let kept = files(account.path());
    for wanted in ["state/cadence/hour.json", "state/cadence/findings.jsonl"] {
        assert!(
            kept.iter().any(|file| file == wanted),
            "the run sending the card didn't keep {wanted}: {kept:?}"
        );
    }
    let ledger = fs::read_to_string(account.path().join("state/cadence/findings.jsonl"))
        .expect("the ledger");
    assert!(
        ledger.contains("\"owner\":\"88LQC0\""),
        "the drag wasn't owed to the miss's owner: {ledger}"
    );
    assert!(
        kept.iter()
            .any(|file| file.starts_with("state/pi/agentic-harness.pi/mesh-workflow/messages/")),
        "the card and the owner's finding weren't sent: {kept:?}"
    );
}
