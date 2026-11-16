//! The daemon asked in ways its unit never asks it: for its help, with a
//! flag it doesn't know, short of one it needs, or handed levels it can't
//! read. Each ends at once in the terminal kit's shape, with the code
//! docs/commands.md gives it, before it takes the bus or listens.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// What the daemon says and ends with, run with `args`.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_notices"))
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("notices ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A folder of its own for one test, so none reads another's files.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("notices-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    folder
}

/// Every flag its unit hands it, each naming a file in `folder`, `levels`
/// among them.
fn unit(folder: &Path, levels: &str) -> Vec<String> {
    [
        "--socket",
        "notices.sock",
        "--state",
        "state",
        "--levels",
        levels,
        "--senders",
        "senders.json",
        "--quiet",
        "quiet",
        "--vocabulary",
        "words.json",
        "--marks",
        "marks.json",
    ]
    .chunks(2)
    .flat_map(|pair| {
        [
            pair[0].to_owned(),
            folder.join(pair[1]).display().to_string(),
        ]
    })
    .collect()
}

#[test]
fn its_help_opens_with_its_name_and_names_what_its_unit_hands_it() {
    let output = ran(&["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("notices"), "help opened {help:?}");
    for flag in [
        "--socket",
        "--state",
        "--levels",
        "--senders",
        "--quiet",
        "--vocabulary",
        "--marks",
        "--addressed",
        "--shells",
    ] {
        assert!(help.contains(flag), "help never named {flag}: {help}");
    }
    assert!(
        output.stderr.is_empty(),
        "help said {:?} on stderr",
        said(&output.stderr)
    );
}

#[test]
fn a_flag_it_doesnt_know_is_refused_with_the_nearest() {
    let folder = folder("unknown");
    let mut args = unit(&folder, "levels.json");
    args[0] = "--sockt".to_owned();
    let output = ran(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a wrong flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--sockt"),
        "the refusal never named the flag: {why}"
    );
    assert!(
        why.contains("--socket"),
        "the refusal never named the nearest: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "a refusal said {:?} on stdout",
        said(&output.stdout)
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_run_short_of_what_it_needs_is_refused_as_asked_wrongly() {
    let folder = folder("short");
    let args = unit(&folder, "levels.json");
    let short: Vec<&str> = args.iter().map(String::as_str).take(12).collect();
    let output = ran(&short);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a short run ended {:?}",
        output.status
    );
    assert!(
        why.contains("--marks"),
        "the refusal never named what was missing: {why}"
    );
    assert!(
        why.contains("notices --help"),
        "the refusal never said where to look: {why}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_flag_with_no_value_is_refused_as_asked_wrongly() {
    let folder = folder("bare");
    let mut args = unit(&folder, "levels.json");
    args.push("--shells".to_owned());
    let output = ran(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a bare flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--shells"),
        "the refusal never named the flag: {why}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn levels_it_cant_read_end_it_as_its_configuration() {
    let folder = folder("levels");
    let args = unit(&folder, "no-levels.json");
    let output = ran(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(78),
        "unread levels ended {:?}",
        output.status
    );
    assert!(
        why.contains("no-levels.json"),
        "the failure never named the file: {why}"
    );
    assert!(
        !folder.join("notices.sock").exists(),
        "it listened before reading its levels"
    );
    let _ = std::fs::remove_dir_all(&folder);
}
