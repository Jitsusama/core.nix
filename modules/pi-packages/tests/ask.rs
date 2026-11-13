//! What the command line asks for: help is never taken as a hold's reason,
//! and a reason that reads as a flag is refused rather than held.

use std::path::Path;
use std::time::Duration;

use pi_packages::ask::{Asked, asked};
use pi_packages::run::Pace;

const FILE: &str = "/home/someone/pi-packages.json";

#[test]
fn help_is_asked_for_first_or_after_hold() {
    for words in [
        &["--help"][..],
        &["-h"],
        &["help"],
        &["hold", "--help"],
        &["hold", "--config", FILE, "--help"],
        &["hold", "--config", FILE, "-h"],
        &["hold", "--config", FILE, "--for", "90m", "--help"],
    ] {
        assert_eq!(asked(words), Ok(Asked::Help), "{words:?}");
    }
}

#[test]
fn a_hold_whose_reason_reads_as_a_flag_is_refused() {
    for words in [
        &["hold", "--config", FILE, "--forever", "night"][..],
        &["hold", "--config", FILE, "-x"],
        &["hold", "--config", FILE, "--for", "90m", "--now"],
    ] {
        let said = asked(words).expect_err(&format!("{words:?} was taken"));
        assert!(said.contains("flag"), "{words:?} said: {said}");
    }
}

#[test]
fn a_hold_says_why_and_may_say_for_how_long() {
    assert_eq!(
        asked(&["hold", "--config", FILE, "a", "switch", "-", "now"]),
        Ok(Asked::Hold {
            file: Path::new(FILE),
            why: "a switch - now".to_owned(),
            length: None,
        }),
        "a hold until released"
    );
    assert_eq!(
        asked(&["hold", "--config", FILE, "--for", "2h", "night"]),
        Ok(Asked::Hold {
            file: Path::new(FILE),
            why: "night".to_owned(),
            length: Some(Duration::from_mins(120)),
        }),
        "a hold for a time"
    );
    assert!(
        asked(&["hold", "--config", FILE, "--for", "soon", "night"]).is_err(),
        "a length that is no length"
    );
    assert!(
        asked(&["hold", "--config", FILE]).is_err(),
        "a hold with no reason"
    );
    assert!(
        asked(&["hold", "--config", FILE, "--for", "2h"]).is_err(),
        "a hold for a time with no reason"
    );
}

#[test]
fn the_other_commands_are_read_as_before() {
    let file = Path::new(FILE);
    assert_eq!(
        asked(&["update", "--config", FILE]),
        Ok(Asked::Update {
            file,
            pace: Pace::Spaced
        }),
        "update"
    );
    assert_eq!(
        asked(&["update", "--config", FILE, "--now"]),
        Ok(Asked::Update {
            file,
            pace: Pace::Now
        }),
        "update --now"
    );
    assert_eq!(
        asked(&["sweep", "--config", FILE]),
        Ok(Asked::Sweep(file)),
        "sweep"
    );
    assert_eq!(
        asked(&["status", "--config", FILE]),
        Ok(Asked::Status(file)),
        "status"
    );
    assert_eq!(
        asked(&["release", "--config", FILE]),
        Ok(Asked::Release(file)),
        "release"
    );
    assert!(asked(&["reboot"]).is_err(), "an unknown command");
}
