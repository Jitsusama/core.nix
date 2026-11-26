//! The running of an update's programs, tested beside it so `run.rs` reads
//! as how a command is run, watched and told alone.

use super::*;

#[test]
fn a_log_past_its_most_keeps_its_last_part() {
    let folder = std::env::temp_dir().join(format!("update-to-main-log-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let log = folder.join("activation.log");
    let mut said = vec![b'a'; usize::try_from(LOG_MOST).unwrap()];
    said.extend(b"the failed switch said this");
    fs::write(&log, &said).unwrap();
    let (file, began) = locked(&log, "activating").unwrap();
    drop(file);
    let kept = fs::read(&log).unwrap();
    let _ = fs::remove_dir_all(&folder);
    assert_eq!(
        began, LOG_KEPT,
        "the part a command adds begins past what was kept"
    );
    assert_eq!(
        kept[..],
        said[said.len() - usize::try_from(LOG_KEPT).unwrap()..],
        "the log didn't keep its last part"
    );
}

#[test]
fn a_build_that_goes_quiet_is_stopped() {
    let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
    sh.args([
        "-c",
        "echo 'building x on y'; echo 'x> compiling'; sleep 30",
    ]);
    let started = Instant::now();
    let failed = built(
        sh,
        "building",
        std::iter::empty::<&str>(),
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it waited for the end"
    );
    assert_eq!(
        failed.why,
        "building said nothing for 0 minutes and its pieces did no work, so it was stopped"
    );
    assert_eq!(
        failed.lines,
        ["building x on y"],
        "the log's lines are left out"
    );
}

#[test]
fn a_quiet_build_whose_pieces_work_runs_on_but_not_for_ever() {
    let quiet = |then: &str, done: &dyn Fn(libc::pid_t) -> Option<u64>| {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args([
            "-c",
            &format!("echo 'these 2 derivations will be built:'; {then}"),
        ]);
        built_watching(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_millis(500),
            done,
        )
    };
    let work = std::sync::atomic::AtomicU64::new(0);
    let working = |_| Some(work.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1);
    assert!(quiet("sleep 1.6; echo done", &working).is_ok());
    // Its pieces stopped working, so it's stopped at the next look.
    let started = Instant::now();
    let failed = quiet("sleep 30", &|_| Some(7)).unwrap_err();
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it waited for the end"
    );
    assert!(failed.why.contains("did no work"), "{}", failed.why);
    // Working however long, it's quiet no more than the most.
    let started = Instant::now();
    let failed = quiet("sleep 30", &working).unwrap_err();
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(2500) && took < Duration::from_secs(10),
        "{took:?}"
    );
    assert!(failed.why.contains("said nothing for"), "{}", failed.why);
    assert!(!failed.why.contains("did no work"), "{}", failed.why);
}

#[test]
fn a_quiet_build_on_a_builder_it_can_t_read_waits_the_longest() {
    let quiet = |then: &str| {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args([
            "-c",
            &format!("echo 'these 2 derivations will be built:'; {then}"),
        ]);
        built_watching(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_millis(200),
            &|_| None,
        )
    };
    // A kernel's link may say nothing for many minutes on a busy
    // builder, which can't be told from a stuck one, so it runs on.
    assert_eq!(quiet("sleep 1.6; echo done").err().map(|f| f.why), None);
    // Still, it's quiet no longer than a copy back may be, the most.
    let started = Instant::now();
    let failed = quiet("sleep 30").unwrap_err();
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(3000) && took < Duration::from_secs(10),
        "{took:?}"
    );
    assert_eq!(
        failed.why,
        "building said nothing for 0 minutes and its builders can't say what its pieces \
         did, so it was stopped"
    );
}

#[test]
fn a_build_that_keeps_talking_runs_to_its_end() {
    let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
    sh.args([
        "-c",
        "for i in 1 2 3 4; do echo \"x> $i\" >&2; sleep 0.4; done; echo done",
    ]);
    let said = built(
        sh,
        "building",
        std::iter::empty::<&str>(),
        Duration::from_secs(1),
    );
    assert_eq!(said.unwrap(), "done");
}

#[test]
fn a_copy_back_may_stay_quiet_longer_but_not_for_ever() {
    let copying = |then: &str| {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args([
            "-c",
            &format!("echo \"copying path '/nix/store/a-x' from 'ssh-ng://builder'...\"; {then}"),
        ]);
        built(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
        )
    };
    assert_eq!(
        copying("sleep 1.6; echo done").unwrap(),
        "copying path '/nix/store/a-x' from 'ssh-ng://builder'...\ndone"
    );
    let started = Instant::now();
    let failed = copying("sleep 30").unwrap_err();
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it waited for the end"
    );
    assert!(failed.why.contains("said nothing"), "{}", failed.why);
    assert!(!is_copy_back(
        "copying path '/nix/store/a-x' to 'ssh-ng://builder'..."
    ));
}

#[test]
fn a_piece_building_may_stay_quiet_longer_but_waiting_for_one_may_not() {
    let after = |first: &str| {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args(["-c", &format!("echo \"{first}\"; sleep 1.6; echo done")]);
        built(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
        )
    };
    // A compile that says nothing for a while still works.
    assert!(after("building '/nix/store/a-x.drv' on 'ssh-ng://builder'...").is_ok());
    assert!(after("building '/nix/store/a-x.drv'...").is_ok());
    // Before any piece builds, a quiet build is one waiting on a
    // builder that doesn't answer.
    let failed = after("these 2 derivations will be built:").unwrap_err();
    assert!(failed.why.contains("said nothing"), "{}", failed.why);
}

#[test]
fn a_build_log_line_is_told_from_nix_s_own() {
    assert!(is_log("linux-7.3.1> CC kernel/fork.o"));
    assert!(!is_log(
        "building '/nix/store/a-b.drv' on 'ssh-ng://builder'..."
    ));
    assert!(!is_log("error: builder failed > 1"));
}

#[test]
fn said_gives_both_streams_and_output_only_what_was_printed() {
    let both = || {
        // The shell the integration tests' stand-ins run on, which a
        // build's sandbox has where it has no /bin/sh.
        let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let mut sh = Command::new(shell);
        sh.arg("-c").arg("echo printed; echo said >&2");
        sh
    };
    assert_eq!(output(both(), "saying", [""; 0]).unwrap(), "printed");
    assert_eq!(said(both(), "saying", [""; 0]).unwrap(), "printed\nsaid");
}

#[test]
fn a_tail_keeps_the_last_lines_that_say_something() {
    let said = (1..=30).fold(String::new(), |mut said, n| {
        said.push_str(&n.to_string());
        said.push_str("\n\n");
        said
    });
    let kept = tail(&said);
    assert_eq!(kept.len(), TAIL);
    assert_eq!(kept.first().map(String::as_str), Some("11"));
    assert_eq!(kept.last().map(String::as_str), Some("30"));
}

/// One of Nix's errors for a piece, its reason and its output.
fn cannot_build(piece: &str, reason: &str) -> String {
    format!(
        "error: Cannot build '/nix/store/{piece}.drv'.\n       Reason: {reason}.\n       \
         Output paths:\n         /nix/store/{piece}\n"
    )
}

#[test]
fn a_tail_starts_at_the_piece_that_failed_not_those_waiting_on_it() {
    let mut said = "building the system\n".to_owned();
    said.push_str(&cannot_build("guard", "builder failed with exit code 101"));
    said.push_str("       Last 25 log lines:\n       > test a_guard_holds ... FAILED\n");
    for piece in ["guard-user", "home-path", "system-path", "unit", "system"] {
        said.push_str(&cannot_build(piece, "1 dependency failed"));
    }
    let kept = tail(&said);
    assert_eq!(
        kept.first().map(String::as_str),
        Some("error: Cannot build '/nix/store/guard.drv'.")
    );
    assert!(kept.iter().any(|l| l.contains("a_guard_holds ... FAILED")));
    assert_eq!(kept.len(), TAIL);
}

#[test]
fn a_tail_starts_at_an_older_nix_builder_failing() {
    let said = "error: builder for '/nix/store/guard.drv' failed with exit code 1;\n       \
                last 25 log lines:\n       > it broke\n\
                error: 1 dependencies of derivation '/nix/store/system.drv' failed to build\n";
    let kept = tail(said);
    assert_eq!(kept.len(), 4);
    assert!(kept[0].starts_with("error: builder for '/nix/store/guard.drv'"));
}

#[test]
fn a_tail_of_only_waiting_pieces_keeps_the_last_lines() {
    let said = (0..8).fold(String::new(), |said, n| {
        said + &cannot_build(&format!("piece{n}"), "1 dependency failed")
    });
    let kept = tail(&said);
    assert_eq!(kept.len(), TAIL);
    assert_eq!(
        kept.last().map(String::as_str).map(str::trim),
        Some("/nix/store/piece7")
    );
}
