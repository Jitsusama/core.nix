//! The tests of who the file for any work names, and of what a program
//! told of it hears.

use super::*;
use crate::machines::{Lending, Machines, Standing, seconds};
use crate::proof::Health;
use crate::testing::executable;
use std::net::TcpListener;

// A build that may go anywhere public, such as a machine's update
// built with no local jobs, reads the one lent with nothing
// mandatory, so it takes any piece; never a builder beside or
// standing, which only some work may reach.
#[test]
fn a_lent_builder_is_named_for_any_work_to_builds_that_ask() {
    let dir = tempfile::tempdir().expect("a folder");
    let anything = dir.path().join("machines-for-anything");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let mut machines = Machines::new(
        dir.path().join("machines"),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![Standing {
            address: address.clone(),
            line: "ssh-ng://home".to_owned(),
        }],
    )
    .beside(
        dir.path().join("machines-beside"),
        vec![Standing {
            address,
            line: "ssh-ng://work x86_64-linux - 8 1 big-parallel".to_owned(),
        }],
    )
    .for_anything(anything.clone());
    let read = |path| fs::read_to_string(path).expect("read");
    machines.show(Some(Lending {
        line: "ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark - KEY",
        store: None,
    }));
    assert_eq!(
        read(&anything),
        "ssh-ng://lent x86_64-linux /key 8 1 big-parallel,benchmark - KEY\n"
    );
    machines.show(None);
    assert_eq!(read(&anything), "", "emptied while none is lent");
    drop(listening);
}

#[test]
fn the_lent_builder_is_named_for_any_work_while_it_answers_else_a_stand_in() {
    let lines = |lines: &[&str]| lines.iter().map(|line| (*line).to_owned()).collect();
    let (second, vm): (Vec<String>, Vec<String>) = (lines(&["second"]), lines(&["vm", "vm-3"]));
    let alone = |state: State| (state, false);
    for state in [State::None, State::Unproven, State::Passing] {
        assert_eq!(
            for_anything(Some("lent"), alone(state), &second, &vm),
            ("lent\n".to_owned(), Named::Lent),
            "{state:?}"
        );
    }
    assert_eq!(
        for_anything(Some("lent"), alone(State::Failing), &second, &vm),
        ("lent\nsecond\n".to_owned(), Named::Second),
        "one failing once is still named, the second first beside it"
    );
    assert_eq!(
        for_anything(Some("lent"), alone(State::Failing), &[], &vm),
        ("lent\nvm\nvm-3\n".to_owned(), Named::Vm(2))
    );
    assert_eq!(
        for_anything(None, alone(State::Dropped), &[], &vm),
        ("vm\nvm-3\n".to_owned(), Named::Vm(2))
    );
    assert_eq!(
        for_anything(None, alone(State::None), &[], &vm),
        ("vm\nvm-3\n".to_owned(), Named::Vm(2)),
        "none lent"
    );
    assert_eq!(
        for_anything(Some("lent"), alone(State::Failing), &[], &[]),
        ("lent\n".to_owned(), Named::Unanswered)
    );
    assert_eq!(
        for_anything(None, alone(State::Dropped), &[], &[]),
        (String::new(), Named::Unanswered)
    );
    assert_eq!(
        for_anything(None, alone(State::None), &[], &[]),
        (String::new(), Named::Nothing)
    );
    assert_eq!(
        for_anything(Some("lent"), alone(State::Full), &second, &vm),
        ("second\n".to_owned(), Named::Second),
        "one with room stands in for a full one"
    );
    assert_eq!(
        for_anything(Some("lent"), alone(State::Full), &[], &vm),
        ("vm\nvm-3\n".to_owned(), Named::Vm(2))
    );
    assert_eq!(
        for_anything(Some("lent"), alone(State::Full), &[], &[]),
        ("lent\n".to_owned(), Named::Lent),
        "a full one, collecting as it builds, beats none"
    );
}

// From 14:40 on 15 November every lender's proof stalled together,
// dozens of times in ten minutes, while this machine's memory was
// full: each time no builder was said to answer, and switches were
// held, though every builder built and only the way up from here
// stalled.
#[test]
fn a_stall_every_lender_shares_keeps_the_lent_builder_named_and_tells_no_one() {
    assert_eq!(
        for_anything(Some("lent"), (State::Dropped, true), &[], &[]),
        ("lent\n".to_owned(), Named::Stalled),
        "a stall every lender shares is this machine's own"
    );
    assert_eq!(
        for_anything(None, (State::Dropped, true), &[], &[]),
        (String::new(), Named::Unanswered),
        "with none lent there's nothing to keep named"
    );
    let second = vec!["second".to_owned()];
    assert_eq!(
        for_anything(Some("lent"), (State::Dropped, true), &second, &[]),
        ("lent\nsecond\n".to_owned(), Named::Second),
        "one that answers stands in as before"
    );
    assert_eq!(told(Named::Lent, Named::Stalled), None, "no one is told");
    assert_eq!(told(Named::Stalled, Named::Lent), None);
    assert_eq!(
        told(Named::Stalled, Named::Unanswered).map(|(word, _)| word),
        Some("none"),
        "a stall that's no longer shared is told"
    );
    assert_eq!(
        told(Named::Unanswered, Named::Stalled).map(|(word, _)| word),
        Some("back"),
        "once the stall is seen to be shared, builds go again"
    );
}

#[test]
fn a_stall_is_shared_only_while_no_lender_passes_and_another_stalls() {
    let root = tempfile::tempdir().unwrap();
    let at = 10_000;
    let health = |name: &str, state: &str, stalled: bool| {
        let dir = root.path().join(name);
        fs::create_dir_all(&dir).unwrap();
        let said = serde_json::json!({
            "state": state, "stalled": stalled,
            "lastSuccess": at - 600, "lastFailure": at - 20,
        });
        fs::write(dir.join("health.json"), said.to_string()).unwrap();
        dir
    };
    let second = health("more", "dropped", true);
    let vm = vec![health("vm", "failing", true), root.path().join("vm-9")];
    assert!(
        stalled_alike(&second, &vm, at),
        "every lender stalls at once"
    );
    assert!(
        !stalled_alike(&second, &vm, at + 600),
        "a stall said long ago isn't shared now"
    );
    let vm_passing = vec![health("vm-2", "passing", false)];
    fs::write(
        vm_passing[0].join("health.json"),
        serde_json::json!({"state": "passing", "lastSuccess": at}).to_string(),
    )
    .unwrap();
    assert!(
        !stalled_alike(&second, &vm_passing, at),
        "one carrying its copies shows the way up works"
    );
    let refused = health("refused", "dropped", false);
    assert!(
        !stalled_alike(&refused, &[], at),
        "one that didn't build fails for its own reasons"
    );
}

/// A lender's folder whose builder passed its last proof just now,
/// naming `line` for any work.
fn passing(dir: &Path, line: &str) {
    fs::create_dir_all(dir).expect("a folder");
    let mut health = Health::of("builder");
    health.heard(true, seconds());
    proof::write(dir, Some(&health), None, seconds());
    fs::write(dir.join("machines-for-anything"), format!("{line}\n")).expect("write");
    fs::write(dir.join("store"), "ssh-ng://far\n").expect("write");
}

#[test]
fn a_lane_kept_for_runs_naming_it_stands_in_for_nothing() {
    let dir = tempfile::tempdir().expect("a folder");
    let folder = |name: &str| dir.path().join(name);
    let anything = folder("machines-for-anything");
    let mut machines = Machines::new(
        folder("machines"),
        folder("machines-with-standing"),
        folder("store"),
        folder("lending"),
        vec![],
    )
    .for_anything(anything.clone())
    .stand_ins_from(folder("more"), vec![folder("vm"), folder("vm-3")]);
    passing(
        &folder("vm"),
        "ssh-ng://vm x86_64-linux /k 4 1 big-parallel - K",
    );
    passing(
        &folder("vm-3"),
        "ssh-ng://vm-3 x86_64-linux /k 4 1 big-parallel - K",
    );
    fs::write(folder("vm-3").join(there::NAMED_ONLY), "").expect("write");
    let line = "ssh-ng://lent x86_64-linux /k 8 1 big-parallel - K";
    machines.proven(Some(line), State::Failing);
    machines.show(Some(Lending { line, store: None }));
    assert_eq!(
        fs::read_to_string(&anything).expect("read"),
        format!("{line}\nssh-ng://vm x86_64-linux /k 4 1 big-parallel - K\n"),
        "only the lane that isn't named-only stands in"
    );
}

#[test]
fn builders_that_passed_stand_in_for_one_lent_that_fails_until_it_passes_again() {
    let dir = tempfile::tempdir().expect("a folder");
    let folder = |name: &str| dir.path().join(name);
    let anything = folder("machines-for-anything");
    let mut machines = Machines::new(
        folder("machines"),
        folder("machines-with-standing"),
        folder("store"),
        folder("lending"),
        vec![],
    )
    .for_anything(anything.clone())
    .stand_ins_from(
        folder("more"),
        vec![folder("vm"), folder("vm-2"), folder("vm-3")],
    );
    passing(
        &folder("vm"),
        "ssh-ng://vm x86_64-linux /k 4 1 big-parallel - K",
    );
    passing(
        &folder("vm-2"),
        "ssh-ng://vm-2 x86_64-linux /k 4 1 big-parallel - K",
    );
    // Warming, so its tests would build the desktop first.
    fs::write(
        folder("vm-2").join(there::WARMING),
        format!("{} vm-2\n", std::process::id()),
    )
    .expect("write");
    // Failed its last proof.
    passing(
        &folder("vm-3"),
        "ssh-ng://vm-3 x86_64-linux /k 4 1 big-parallel - K",
    );
    let mut failed = Health::of("vm-3");
    failed.heard(false, seconds());
    proof::write(&folder("vm-3"), Some(&failed), None, seconds());
    let read = |path| fs::read_to_string(path).expect("read");
    let line = "ssh-ng://lent x86_64-linux /k 8 1 big-parallel - K";
    let lent = Some(Lending { line, store: None });
    machines.proven(Some(line), State::Passing);
    machines.show(lent);
    assert_eq!(read(&anything), format!("{line}\n"));
    machines.proven(Some(line), State::Failing);
    machines.show(lent);
    assert_eq!(
        read(&anything),
        format!("{line}\nssh-ng://vm x86_64-linux /k 4 1 big-parallel - K\n"),
        "the warm VM builder that passed stands in"
    );
    machines.proven(None, State::Dropped);
    machines.show(None);
    assert_eq!(
        read(&anything),
        "ssh-ng://vm x86_64-linux /k 4 1 big-parallel - K\n"
    );
    passing(
        &folder("more"),
        "ssh-ng://more x86_64-linux /k 8 1 big-parallel - K",
    );
    machines.show(None);
    assert_eq!(
        read(&anything),
        "ssh-ng://more x86_64-linux /k 8 1 big-parallel - K\n",
        "the second lent builder stands in first"
    );
    // A new builder stands, its line not yet proven.
    let next = "ssh-ng://next x86_64-linux /k 8 1 big-parallel - K";
    machines.show(Some(Lending {
        line: next,
        store: None,
    }));
    assert_eq!(read(&anything), format!("{next}\n"), "named alone again");
    fs::remove_file(folder("more").join(proof::HEALTH)).expect("remove");
    fs::remove_file(folder("vm").join(proof::HEALTH)).expect("remove");
    machines.proven(None, State::Dropped);
    machines.show(None);
    assert_eq!(read(&anything), "", "no builder answers at all");
}

// Whoever keeps the machine's builds moving hears the moment no
// builder answers, rather than finding a switch stalled, and hears
// once more when one does; once each, however many ticks pass.
#[test]
fn no_builder_answering_is_told_once_and_one_answering_again_once() {
    let dir = tempfile::tempdir().expect("a folder");
    let folder = |name: &str| dir.path().join(name);
    let (program, told) = (folder("tell"), folder("told"));
    executable(
        &program,
        &format!("#!/bin/sh\necho \"$1: $2\" >> '{}'\n", told.display()),
    );
    let mut machines = Machines::new(
        folder("machines"),
        folder("machines-with-standing"),
        folder("store"),
        folder("lending"),
        vec![],
    )
    .for_anything(folder("machines-for-anything"))
    .stand_ins_from(folder("more"), vec![folder("vm")])
    .telling(Some(program));
    let heard = || fs::read_to_string(&told).unwrap_or_default();
    let line = "ssh-ng://lent x86_64-linux /k 8 1 big-parallel - K";
    machines.proven(Some(line), State::Passing);
    machines.show(Some(Lending { line, store: None }));
    assert_eq!(heard(), "", "nothing to tell while the lent one answers");
    machines.proven(None, State::Dropped);
    machines.show(None);
    machines.show(None);
    assert_eq!(
        heard(),
        "none: no builder answers: the lent one failed 2 proofs in a row and none \
         stands in, so another is asked for\n",
        "told once"
    );
    let next = "ssh-ng://next x86_64-linux /k 8 1 big-parallel - K";
    machines.show(Some(Lending {
        line: next,
        store: None,
    }));
    machines.show(Some(Lending {
        line: next,
        store: None,
    }));
    assert_eq!(
        heard().lines().skip(1).collect::<Vec<_>>(),
        ["back: a builder answers again: a newly lent one takes any work"],
        "told once more"
    );
}

#[test]
fn each_way_back_from_no_builder_answering_says_which() {
    assert_eq!(
        told(Named::Lent, Named::Unanswered).map(|(word, _)| word),
        Some("none")
    );
    assert_eq!(told(Named::Unanswered, Named::Unanswered), None);
    assert_eq!(told(Named::Lent, Named::Second), None, "a stand-in answers");
    assert_eq!(
        told(Named::Unanswered, Named::Second),
        Some((
            "back",
            "a builder answers again: the second lent builder stands in".into()
        ))
    );
    assert_eq!(
        told(Named::Unanswered, Named::Vm(2)),
        Some((
            "back",
            "a builder answers again: 2 VM builders stand in".into()
        ))
    );
    assert_eq!(
        told(Named::Unanswered, Named::Nothing),
        Some(("back", "none is lent, and no build waits on one".into()))
    );
}
