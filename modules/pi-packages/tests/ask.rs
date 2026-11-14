//! What the command line asks for: help is never taken as a hold's reason,
//! a reason that reads as a flag is refused rather than held, and the
//! settings may be named anywhere, as the account's command names them
//! first. The kit answers `--help` and `-h` before these words are read.

use std::path::Path;
use std::time::Duration;

use pi_packages::ask::{Asked, asked};
use pi_packages::run::Pace;
use terminal_kit::Exit;

const FILE: &str = "/home/someone/pi-packages.json";

/// The words as the account's command passes them, settings first.
fn told(words: &[&'static str]) -> Vec<&'static str> {
    let mut line = vec!["--config", FILE];
    line.extend_from_slice(words);
    line
}

#[test]
fn help_is_asked_for_by_name_or_after_hold() {
    for words in [&["help"][..], &["hold", "help"], &["hold", "help", "me"]] {
        assert_eq!(asked(&told(words)), Ok(Asked::Help), "{words:?}");
        assert_eq!(asked(words), Ok(Asked::Help), "{words:?} with no settings");
    }
}

#[test]
fn a_hold_whose_reason_reads_as_a_flag_is_refused() {
    for words in [
        &["hold", "--forever", "night"][..],
        &["hold", "-x"],
        &["hold", "--for", "90m", "--now"],
    ] {
        let failure = asked(&told(words)).expect_err(&format!("{words:?} was taken"));
        assert_eq!(
            failure.exit,
            Exit::Usage,
            "{words:?} said: {}",
            failure.what
        );
    }
}

#[test]
fn a_hold_says_why_and_may_say_for_how_long() {
    assert_eq!(
        asked(&told(&["hold", "a", "switch", "-", "now"])),
        Ok(Asked::Hold {
            file: Path::new(FILE),
            why: "a switch - now".to_owned(),
            length: None,
        }),
        "a hold until released"
    );
    assert_eq!(
        asked(&told(&["hold", "--for", "2h", "night"])),
        Ok(Asked::Hold {
            file: Path::new(FILE),
            why: "night".to_owned(),
            length: Some(Duration::from_mins(120)),
        }),
        "a hold for a time"
    );
    assert_eq!(
        asked(&told(&["hold", "--for=90m", "--", "-x", "marks", "it"])),
        Ok(Asked::Hold {
            file: Path::new(FILE),
            why: "-x marks it".to_owned(),
            length: Some(Duration::from_mins(90)),
        }),
        "a reason past -- may open with a dash"
    );
    for words in [
        &["hold", "--for", "soon", "night"][..],
        &["hold"],
        &["hold", "--for", "2h"],
    ] {
        assert!(asked(&told(words)).is_err(), "{words:?} was taken");
    }
}

#[test]
fn the_other_commands_are_read_as_before() {
    let file = Path::new(FILE);
    assert_eq!(asked(&told(&[])), Ok(Asked::Status(file)), "a bare line");
    assert_eq!(asked(&told(&["status"])), Ok(Asked::Status(file)), "status");
    assert_eq!(
        asked(&told(&["update"])),
        Ok(Asked::Update {
            file,
            pace: Pace::Spaced
        }),
        "update"
    );
    assert_eq!(
        asked(&told(&["update", "--now"])),
        Ok(Asked::Update {
            file,
            pace: Pace::Now
        }),
        "update --now"
    );
    assert_eq!(
        asked(&["update", "--config", FILE]),
        Ok(Asked::Update {
            file,
            pace: Pace::Spaced
        }),
        "the settings after the command, as the unit names them"
    );
    assert_eq!(asked(&told(&["sweep"])), Ok(Asked::Sweep(file)), "sweep");
    assert_eq!(
        asked(&told(&["release"])),
        Ok(Asked::Release(file)),
        "release"
    );
}

#[test]
fn a_line_asked_wrongly_is_a_usage_failure() {
    for words in [
        &["reboot"][..],
        &["sweep", "--now"],
        &["status", "--for", "2h"],
        &["release", "everything"],
    ] {
        let failure = asked(&told(words)).expect_err(&format!("{words:?} was taken"));
        assert_eq!(
            failure.exit,
            Exit::Usage,
            "{words:?} said: {}",
            failure.what
        );
    }
    let failure = asked(&["status"]).expect_err("no settings were named");
    assert_eq!(failure.exit, Exit::Usage, "{}", failure.what);
}
