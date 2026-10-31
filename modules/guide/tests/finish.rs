//! `guide finish`: what's left once the person is signed in and online,
//! GitHub's steps first, then each one the machine's repo adds, which the
//! person runs in another terminal while the guide watches for it to say
//! it's done.

use guide::finish::{self, Added};
use guide::github::Account;
use guide::installed::FACTS;
use guide::stand_in::StandIn;
use guide::{Program, Standing, Step};

const DONE: &str = "/nix/store/abc-enrolled/bin/enrolled";

fn alice() -> Account {
    Account {
        name: "alice".into(),
        home: "/home/alice".into(),
        runtime: "/run/user/1000".into(),
        bus: None,
    }
}

fn enrol() -> Added {
    Added {
        id: "enrol".into(),
        title: "Enrol this machine with work".into(),
        command: "enroll".into(),
        about: "It signs you in through the browser.".into(),
        done: DONE.into(),
    }
}

fn step(added: Added) -> Box<dyn Step> {
    finish::steps(&alice(), vec![added])
        .pop()
        .expect("the machine's step comes last")
}

#[test]
fn a_step_the_machine_adds_is_done_once_its_program_says_so() {
    let mut stand_in = StandIn::new();
    stand_in.answers(
        Program::Carried(DONE),
        &[],
        0,
        "enroll says every part is set up\n",
        "",
    );
    assert_eq!(
        step(enrol()).observe(&mut stand_in).unwrap(),
        Standing::Done {
            evidence: "enroll says every part is set up".into()
        }
    );
}

#[test]
fn a_step_not_done_says_what_to_type_and_what_is_left() {
    let mut stand_in = StandIn::new();
    stand_in.answers(
        Program::Carried(DONE),
        &[],
        1,
        "WARP isn't registered yet\n",
        "",
    );
    let standing = step(enrol()).observe(&mut stand_in).unwrap();
    let Standing::Blocked { why, remedy } = &standing else {
        panic!("{standing:?}");
    };
    assert_eq!(why, "WARP isn't registered yet");
    assert!(
        remedy.starts_with("In another terminal, type enroll."),
        "{remedy}"
    );
    assert!(
        remedy.contains("It signs you in through the browser."),
        "{remedy}"
    );
}

#[test]
fn a_program_that_says_nothing_still_leaves_a_reason() {
    let mut stand_in = StandIn::new();
    stand_in.answers(Program::Carried(DONE), &[], 1, "", "");
    let standing = step(enrol()).observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why == "enroll isn't finished yet"),
        "{standing:?}"
    );
}

#[test]
fn the_check_runs_as_the_account_not_as_root() {
    let mut stand_in = StandIn::new();
    stand_in.answers(Program::Carried(DONE), &[], 0, "done\n", "");
    step(enrol()).observe(&mut stand_in).unwrap();
    assert_eq!(stand_in.calls, [DONE]);
}

#[test]
fn githubs_steps_come_first_then_the_machines_in_their_order() {
    let other = Added {
        id: "backup".into(),
        ..enrol()
    };
    let ids: Vec<&str> = finish::steps(&alice(), vec![enrol(), other])
        .iter()
        .map(|step| step.id())
        .collect();
    assert_eq!(
        ids,
        [
            "github-code",
            "github-sign-in",
            "github-keys",
            "enrol",
            "backup"
        ]
    );
}

#[test]
fn the_machines_steps_are_read_from_what_core_says_about_it() {
    let mut stand_in = StandIn::new();
    stand_in.file(
        FACTS,
        br#"{"disk_name": "system", "once_signed_in": [
            {"id": "enrol", "title": "Enrol this machine with work", "command": "enroll",
             "about": "It signs you in through the browser.",
             "done": "/nix/store/abc-enrolled/bin/enrolled"}]}"#,
    );
    let facts = guide::installed::facts(&mut stand_in).unwrap();
    assert_eq!(facts.once_signed_in, vec![enrol()]);
}

#[test]
fn a_machine_whose_repo_adds_nothing_has_only_githubs_steps() {
    let mut stand_in = StandIn::new();
    stand_in.file(FACTS, br#"{"disk_name": "system"}"#);
    let facts = guide::installed::facts(&mut stand_in).unwrap();
    assert!(facts.once_signed_in.is_empty());
}
