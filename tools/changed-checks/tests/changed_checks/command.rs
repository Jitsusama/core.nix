//! The run as a command: its turn, a session going first, a base wrongly
//! asked, the system compared, options refused, help, and keeping main.

use std::fs;
use std::process::Command;

use super::{BASE, KEEPING, StandIn, WORKING, text};

#[test]
fn a_run_evaluating_here_waits_its_turn_while_another_holds_it() {
    let stand_in = StandIn::new("turns");
    stand_in.checks("base", BASE);
    let lock = stand_in.path("cache/changed-checks/evaluating-here.lock");
    fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
    let held = fs::File::create(&lock).expect("the lock");
    held.lock().expect("held");

    let mut running = stand_in.started(&["--system", "x86_64-linux", "--keep-base", "--here"], &[]);
    let mut said = std::io::BufReader::new(running.stderr.take().expect("its stderr"));
    let mut line = String::new();
    while !line.contains("another run evaluates on this machine, so this waits its turn") {
        line.clear();
        let read = std::io::BufRead::read_line(&mut said, &mut line).expect("read");
        assert_ne!(read, 0, "it ended without waiting its turn");
    }
    let kept = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(!kept.exists(), "nothing is evaluated while it waits");
    drop(held);

    let ran = running.wait_with_output().expect("it runs");
    let rest = std::io::read_to_string(said).expect("read");
    assert!(ran.status.success(), "{rest}");
    assert!(kept.exists(), "kept once its turn came: {rest}");
}

#[test]
fn a_keep_on_a_builder_waits_for_another_keeping_the_same_base() {
    // Two keeps of one base trace into the same files, and one took
    // another's trace on 13 November: a keep cut off by a reboot ran on
    // through its grace while the next keep of that base began beside it.
    let stand_in = StandIn::new("keeps-one-at-a-time");
    stand_in.checks("base", BASE);
    let lock = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.keeping.lock");
    fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
    let held = fs::File::create(&lock).expect("the lock");
    held.lock().expect("held");

    let mut running = stand_in.started(&KEEPING, &[("CHANGED_CHECKS_HERE", "1")]);
    let mut said = std::io::BufReader::new(running.stderr.take().expect("its stderr"));
    let mut line = String::new();
    while !line.contains("another keep of 0123456789ab runs here, so this waits for it") {
        line.clear();
        let read = std::io::BufRead::read_line(&mut said, &mut line).expect("read");
        assert_ne!(read, 0, "it ended without waiting for the other keep");
    }
    let kept = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(!kept.exists(), "nothing is kept while the other keeps it");
    drop(held);

    let ran = running.wait_with_output().expect("it runs");
    let rest = std::io::read_to_string(said).expect("read");
    assert!(ran.status.success(), "{rest}");
    assert!(kept.exists(), "kept once the other let go: {rest}");
}

#[test]
fn keeping_a_base_elsewhere_never_asks_for_a_builder_nor_goes_where_it_must() {
    let stand_in = StandIn::elsewhere("keeps-elsewhere");
    stand_in.checks("base", BASE);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");
    let keeping = ["--system", "x86_64-linux", "--keep-base"];

    // A nix-offload that can't be told never to ask isn't sent the keep,
    // and it isn't evaluated here instead.
    let ran = stand_in.run(&keeping);
    assert_eq!(ran.status.code(), Some(75), "{}", text(&ran.stderr));
    let said = text(&ran.stderr);
    assert!(said.contains("so it's left unkept"), "{said}");
    assert!(said.contains("--here keeps it on this machine"), "{said}");
    assert!(!stand_in.path("changed").exists(), "it wasn't sent");
    let kept = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(!kept.exists(), "nor kept here");

    fs::write(stand_in.path("takes-never-ask"), "").expect("written");
    let ran = stand_in.run(&keeping);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--never-ask\n--changed\n--system\nx86_64-linux\n--keep-base\n"),
        "{sent}"
    );
}

#[test]
fn a_session_asking_to_go_first_waits_its_turn_and_says_so() {
    let stand_in = StandIn::elsewhere("first");
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("takes-first"), "").expect("written");
    fs::write(stand_in.path("clean"), "").expect("written");

    // A test runs outside the net's unit, as any session does.
    let ran = stand_in.run(&["--system", "x86_64-linux", "--name-only", "--first"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("--first is the net's alone, so this run waits its turn"),
        "{}",
        text(&ran.stderr)
    );
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--changed\n--system\nx86_64-linux\n--name-only\n"),
        "neither nix-offload nor the far side is told to go first: {sent}"
    );
}

#[test]
fn a_base_sharing_no_history_is_refused_as_asked_wrongly() {
    let stand_in = StandIn::new("unrelated");

    let ran = stand_in.run(&["--base", "elsewhere", "--system", "x86_64-linux"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(text(&ran.stderr).contains("elsewhere shares no history with HEAD"));
}

#[test]
fn unnamed_the_system_compared_is_the_one_with_the_most_checks() {
    // A run on a Mac compared its own checks and passed over the checks
    // about checks a repository held on Linux alone, as #1373 did.
    let stand_in = StandIn::new("busiest");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(
        stand_in.path("systems.json"),
        r#"{"aarch64-darwin": 9, "x86_64-linux": 4}"#,
    )
    .expect("written");

    let ran = stand_in.run(&["--name-only"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("comparing aarch64-darwin's checks"),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn an_option_it_doesnt_take_is_refused_naming_the_nearest() {
    let stand_in = StandIn::new("misused");

    let ran = stand_in.run(&["--bild"]);

    assert_eq!(ran.status.code(), Some(64), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("no option called --bild")
            && text(&ran.stderr).contains("did you mean --build?"),
        "it didn't name the option it meant: {}",
        text(&ran.stderr)
    );

    let ran = stand_in.run(&["keep-mian"]);

    assert_eq!(ran.status.code(), Some(64), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("did you mean keep-main?"),
        "it didn't name the command it meant: {}",
        text(&ran.stderr)
    );

    let ran = stand_in.run(&["--build", "--name-only"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(
        text(&ran.stderr).contains("--build and --name-only"),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn each_form_answers_its_own_help_and_starts_nothing() {
    let stand_in = StandIn::new("helped");

    let ran = stand_in.run(&["--help"]);

    assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
    for word in ["--name-only", "keep-main", "75"] {
        assert!(
            text(&ran.stdout).contains(word),
            "its help leaves out {word}: {}",
            text(&ran.stdout)
        );
    }

    let ran = stand_in.run(&["keep-main", "--help"]);

    assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stdout).starts_with("changed-checks keep-main")
            && text(&ran.stdout).contains("--follow PATH"),
        "keep-main's help isn't its own: {}",
        text(&ran.stdout)
    );
    assert!(
        !text(&ran.stderr).contains("keeping each head"),
        "asking keep-main's help started the keeper: {}",
        text(&ran.stderr)
    );
}

#[test]
fn keeping_main_ends_before_its_next_look_once_another_build_is_installed() {
    // On 14 November each of the 5 switches that restarted the keeper did
    // so mid-keep, and at 14:47 one threw away 9 minutes of a keep.
    let stand_in = StandIn::new("followed");
    let installed = stand_in.path("installed");
    fs::write(&installed, "").expect("written");
    let ran = Command::new(env!("CARGO_BIN_EXE_changed-checks"))
        .args(["keep-main", "--follow"])
        .arg(&installed)
        .env(
            "CHANGED_CHECKS_REPOSITORY",
            stand_in.path("no-such-repository"),
        )
        .env("XDG_STATE_HOME", stand_in.path("state"))
        .env("NIX_OFFLOAD_DIR", stand_in.path("offload"))
        .output()
        .expect("changed-checks runs");
    assert_eq!(ran.status.code(), Some(0), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("another changed-checks is installed, so this one ends"),
        "it didn't say why it ended: {}",
        text(&ran.stderr)
    );
}
