//! The tests of the machines files whole: who is named to every build and
//! to a build that asks for the standing builders, as the lists are read
//! again, and the builder being made.

use super::*;
use std::net::TcpListener;

#[test]
fn a_standing_builder_is_named_only_to_builds_that_ask_while_it_answers() {
    let dir = tempfile::tempdir().expect("a folder");
    let every = dir.path().join("machines");
    let asked = dir.path().join("machines-with-standing");
    let store = dir.path().join("store");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let mut machines = Machines::new(
        every.clone(),
        asked.clone(),
        store.clone(),
        dir.path().join("lending"),
        vec![Standing {
            address,
            line: "ssh-ng://home".to_owned(),
        }],
    );
    let read = |path| fs::read_to_string(path).expect("read");
    machines.show(Some(Lending {
        line: "ssh-ng://lent",
        store: Some("ssh-ng://lent?ssh-key=/k"),
    }));
    assert_eq!(read(&every), "ssh-ng://lent\n");
    assert_eq!(read(&asked), "ssh-ng://home\nssh-ng://lent\n");
    assert_eq!(read(&store), "ssh-ng://lent?ssh-key=/k\n");
    machines.show(None);
    assert_eq!(read(&every), "");
    assert_eq!(read(&asked), "ssh-ng://home\n");
    assert_eq!(read(&store), "");
    // Gone: nothing listens on port 0, while a port a dropped listener
    // freed can be taken at once by another test binding one.
    drop(listening);
    machines.standing[0].address = "127.0.0.1:0".to_owned();
    machines.show(None);
    assert_eq!(read(&every), "");
    assert_eq!(read(&asked), "");
}

// A machine whose owner lets it take any of this one's work, such as a
// second machine for the same work, is named to every build while it
// answers, for its big work alone as a lent one is, and never beside
// the standing builders, which take only what asks for them.
/// How a builder beside `line` names stood, passing with room, as the
/// lender before said: one unproven is named nowhere.
fn passed(line: &str) -> Vec<Health> {
    let mut health = Health::of(&proof::named(line));
    health.heard(
        proof::Proven {
            passed: true,
            free: Some(proof::FLOOR),
            ..proof::Proven::default()
        },
        10,
    );
    vec![health]
}

#[test]
fn a_builder_beside_is_named_to_every_build_for_its_big_work_while_it_answers() {
    let dir = tempfile::tempdir().expect("a folder");
    let every = dir.path().join("machines");
    let asked = dir.path().join("machines-with-standing");
    let alone = dir.path().join("machines-beside");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let mut machines = Machines::new(
        every.clone(),
        asked.clone(),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(
        alone.clone(),
        vec![Standing {
            address,
            line: "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark".to_owned(),
        }],
    )
    .going_on_from(passed("ssh-ng://work"));
    let read = |path| fs::read_to_string(path).expect("read");
    machines.show(Some(Lending {
        line: "ssh-ng://lent",
        store: None,
    }));
    assert_eq!(
        read(&every),
        "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark big-parallel -\n\
         ssh-ng://lent\n"
    );
    assert_eq!(read(&asked), "ssh-ng://lent\n");
    // Named alone, as itself, for a build that may go there and
    // nowhere else.
    assert_eq!(
        read(&alone),
        "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark\n"
    );
    drop(listening);
    machines.beside[0].address = "127.0.0.1:0".to_owned();
    machines.show(None);
    assert_eq!(read(&every), "");
    assert_eq!(read(&alone), "");
}

#[test]
fn builders_standing_and_beside_are_read_again_each_time_they_are_named() {
    let dir = tempfile::tempdir().expect("a folder");
    let asked = dir.path().join("machines-with-standing");
    let alone = dir.path().join("machines-beside");
    let (standing, beside) = (dir.path().join("standing"), dir.path().join("beside"));
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    fs::write(&standing, format!("{address} ssh-ng://home\n")).expect("write");
    fs::write(&beside, "").expect("write");
    let mut machines = Machines::new(
        dir.path().join("machines"),
        asked.clone(),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(alone.clone(), vec![])
    .going_on_from(passed("ssh-ng://work"))
    .read_from(Some(standing.clone()), Some(beside.clone()));
    let read = |path| fs::read_to_string(path).expect("read");
    machines.show(None);
    assert_eq!(read(&asked), "ssh-ng://home\n");
    assert_eq!(read(&alone), "");
    // A switch puts new lists in place, with the lender still running.
    fs::write(&standing, "").expect("write");
    fs::write(&beside, format!("{address} ssh-ng://work\n")).expect("write");
    machines.show(None);
    assert_eq!(read(&asked), "");
    assert_eq!(read(&alone), "ssh-ng://work\n");
    fs::remove_file(&beside).expect("remove");
    machines.show(None);
    assert_eq!(
        read(&alone),
        "ssh-ng://work\n",
        "one unread stays as it was"
    );
}

#[test]
fn the_builder_being_made_is_named_until_it_stands_or_wont() {
    let dir = tempfile::tempdir().expect("a folder");
    let lending = dir.path().join("lending");
    let mut machines = Machines::new(
        dir.path().join("machines"),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        lending.clone(),
        Vec::new(),
    );
    let read = |path| fs::read_to_string(path).expect("read");
    machines.asking(Some("lend-1"));
    assert_eq!(read(&lending), "lend-1\n");
    machines.asking(None);
    assert_eq!(read(&lending), "", "emptied, not removed, once it stands");
}
