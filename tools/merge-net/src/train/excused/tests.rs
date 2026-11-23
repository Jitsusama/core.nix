//! What excusing a red does, case by case: which checks fail alike, which
//! runs are asked, and when a red holds.

use super::*;

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The words of a gate in which each of `failed` failed to build,
/// printing the lines named, and each of `following` failed only for
/// one of them.
fn red(failed: &[(&str, &[&str])], following: &[&str]) -> String {
    let mut words = String::from("changed-checks: evaluates 3 of x86_64-linux's 9 checks\n");
    for (check, printed) in failed {
        for line in *printed {
            let _ = writeln!(words, "{}> {line}", nix_name(check));
        }
        let _ = writeln!(words, "{}", cannot_build(check));
    }
    for check in following {
        let _ = writeln!(
            words,
            "error: Cannot build '/nix/store/j5fh-{check}.drv'.\n       Reason: 1 \
                 dependency failed."
        );
    }
    words.push_str("error: Build failed due to failed dependency\n");
    words.push_str("error: build of '/nix/store/0x1c-x.drv^*' failed\n");
    words.push_str("\u{2715} a check the change touched failed\n");
    words
}

/// What nix says of `check` failing to build of itself.
fn cannot_build(check: &str) -> String {
    format!(
        "error: Cannot build '/nix/store/0x1c-{check}.drv'.\n       Reason: builder failed \
             with exit code 1.\n       Output paths:"
    )
}

/// `lines`, each printed once, as a check's failure says them.
fn said(lines: &[&str]) -> Said {
    let mut said = Said::new();
    for &line in lines {
        *said.entry(line.to_owned()).or_default() += 1;
    }
    said
}

/// A base that fails every check, printing `lines` once.
fn fails(lines: &[&str]) -> AtBase {
    AtBase::Fails(vec![said(lines)])
}

/// Asks of which each answers as its closure says.
fn asks<'a>(
    known: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
    again: &'a (dyn Fn(&str) -> Option<bool> + Sync),
    asked: &'a (dyn Fn(&str) -> Option<AtBase> + Sync),
) -> Asks<'a> {
    Asks {
        known,
        again,
        asked,
    }
}

fn nothing(_: &str) -> Option<AtBase> {
    None
}

fn unsaid(_: &str) -> Option<bool> {
    None
}

/// A check that fails again when run alone at the candidate.
const FAILED_AGAIN: fn(&str) -> Option<bool> = |_| Some(false);

/// A check that passes when run alone at the candidate.
const PASSED_AGAIN: fn(&str) -> Option<bool> = |_| Some(true);

/// A run that must not be made.
fn never(_: &[String], _: &str, _: &Path, _: &dyn Fn() -> bool) -> Option<i32> {
    panic!("a run was asked for when nothing should have run");
}

#[test]
fn a_red_whose_checks_print_what_the_base_printed_is_main_s_own() {
    let words = red(
        &[
            ("cat-on-niri", &["counted 12 frames of 30"]),
            ("launcher", &["starting niri", "no window in 9 s"]),
        ],
        &["all"],
    );
    let known = |check: &str| {
        Some(if check == "cat-on-niri" {
            fails(&["counted # frames of #"])
        } else {
            fails(&["starting niri", "no window in # s"])
        })
    };
    assert_eq!(
        excused(&words, &asks(&known, &unsaid, &nothing)),
        Ok(vec![
            ("cat-on-niri".to_owned(), Excuse::MainsOwn),
            ("launcher".to_owned(), Excuse::MainsOwn),
        ]),
        "a red failing at main's own commit alike held its riders, or asked of a check \
             that failed only for another"
    );
}

/// A count of none is another failure than a count of some, and a
/// failure printing fewer lines than main's, or more, isn't main's.
#[test]
fn a_failure_unlike_the_base_s_isn_t_main_s_own() {
    let cases: [(&[&str], &[&str]); 4] = [
        (&["counted 0 frames of 30"], &["counted 12 frames of 30"]),
        (
            &["a.nix runs 900 lines", "b.nix runs 812 lines"],
            &["a.nix runs 900 lines"],
        ),
        (
            &["a.nix runs 900 lines"],
            &["a.nix runs 900 lines", "b.nix runs 812 lines"],
        ),
        (&["x", "x"], &["x"]),
    ];
    for (candidate, base) in cases {
        let words = red(&[("cat", candidate)], &[]);
        let base_words = red(&[("cat", base)], &[]);
        let at_base = failure_of(&base_words, "cat").unwrap();
        let known = move |_: &str| Some(AtBase::Fails(vec![at_base.clone()]));
        assert!(
            excused(&words, &asks(&known, &FAILED_AGAIN, &nothing)).is_err(),
            "{candidate:?} was excused by main's {base:?}"
        );
    }
}

/// A base that failed differently in two runs says no one failure is
/// main's.
#[test]
fn every_run_at_the_base_must_fail_alike() {
    let words = red(&[("cat", &["x"])], &[]);
    let known = |_: &str| Some(AtBase::Fails(vec![said(&["x"]), said(&["y"])]));
    assert!(
        excused(&words, &asks(&known, &FAILED_AGAIN, &nothing)).is_err(),
        "a failure one base run printed and another didn't was main's own"
    );
}

#[test]
fn a_check_that_passes_run_again_at_the_candidate_is_a_flake() {
    let words = red(&[("cat-on-niri", &["drew 4 frames"])], &[]);
    for known in [None, Some(AtBase::Passes), Some(fails(&["other"]))] {
        let base = move |_: &str| known.clone();
        assert_eq!(
            excused(&words, &asks(&base, &PASSED_AGAIN, &nothing)),
            Ok(vec![("cat-on-niri".to_owned(), Excuse::PassedAgain)]),
            "a check that passed run again at the candidate held its riders"
        );
    }
}

#[test]
fn a_check_that_fails_again_and_passes_at_the_base_holds_the_red() {
    let words = red(&[("cat", &["x"]), ("long-files", &["x"])], &[]);
    let known = |check: &str| {
        Some(if check == "long-files" {
            AtBase::Passes
        } else {
            fails(&["x"])
        })
    };
    assert!(
        excused(&words, &asks(&known, &FAILED_AGAIN, &nothing)).is_err(),
        "a check the riders broke was excused"
    );
}

/// Only a base that said nothing is run, and it is asked only once
/// the check failed again at the candidate.
#[test]
fn a_base_that_said_nothing_is_run_once_the_check_fails_again() {
    let words = red(&[("cat", &["x"])], &[]);
    let ran = AtomicUsize::new(0);
    let asked = |_: &str| {
        ran.fetch_add(1, Ordering::SeqCst);
        Some(fails(&["x"]))
    };
    assert_eq!(
        excused(&words, &asks(&nothing, &FAILED_AGAIN, &asked)),
        Ok(vec![("cat".to_owned(), Excuse::MainsOwn)]),
        "a lent run at the base failing alike didn't excuse the check"
    );
    assert_eq!(ran.load(Ordering::SeqCst), 1, "the base wasn't run once");
    let unasked = |_: &str| -> Option<AtBase> {
        panic!("the base was run when it had already answered");
    };
    let known = |_: &str| Some(AtBase::Passes);
    assert!(
        excused(&words, &asks(&known, &FAILED_AGAIN, &unasked)).is_err(),
        "a check that passed at the base was excused"
    );
    assert!(
        excused(&words, &asks(&nothing, &unsaid, &nothing)).is_err(),
        "a check no run spoke for was excused"
    );
}

/// A pytest file prints its daemons' noise beside its verdict, which
/// differs from run to run: only its failed subtests and their `E`
/// lines are compared.
#[test]
fn a_pytest_failure_is_read_by_its_failed_subtests() {
    let printed = |noise: &str, subtest: &str| {
        red(
            &[(
                "cat-on-niri",
                &[
                    noise,
                    &format!("t.py::{subtest} FAILED [ 14%]"),
                    "E       AssertionError: drew 5 frames",
                    "E       assert 5 <= 1",
                    &format!("FAILED t.py::{subtest} - AssertionError: drew 5"),
                ],
            )],
            &[],
        )
    };
    let base = failure_of(&printed("WARN broken pipe", "test_a_walk"), "cat-on-niri");
    let base = base.unwrap();
    let known = move |_: &str| Some(AtBase::Fails(vec![base.clone()]));
    assert_eq!(
        excused(
            &printed("WARN no cursor", "test_a_walk"),
            &asks(&known, &FAILED_AGAIN, &nothing)
        ),
        Ok(vec![("cat-on-niri".to_owned(), Excuse::MainsOwn)]),
        "a daemon's noise kept a pytest failure from matching main's"
    );
    assert!(
        excused(
            &printed("WARN no cursor", "test_a_hold"),
            &asks(&known, &FAILED_AGAIN, &nothing)
        )
        .is_err(),
        "a subtest main passes was excused"
    );
}

/// A check split in parts fails as a part, which nix names before its
/// lines without the part's number, and the watch runs as the check.
#[test]
fn a_part_that_failed_is_read_as_its_check() {
    let words = format!(
        "launcher-part> t.py::test_row FAILED\n{}\n{}\nerror: Cannot build \
             '/nix/store/j5fh-launcher.drv'.\n       Reason: 1 dependency failed.\n",
        cannot_build("launcher-part-6"),
        "error: Build failed due to failed dependency"
    );
    let asked = std::sync::Mutex::new(Vec::new());
    let again = |check: &str| {
        asked.lock().unwrap().push(check.to_owned());
        Some(true)
    };
    assert_eq!(
        excused(&words, &asks(&nothing, &again, &nothing)),
        Ok(vec![("launcher".to_owned(), Excuse::PassedAgain)]),
        "a part's failure wasn't read as its check's"
    );
    assert_eq!(
        asked.into_inner().unwrap(),
        vec!["launcher".to_owned()],
        "the part, not its check, was run again"
    );
}

/// Each asks nothing: an evaluation's error, a build nix gave up on
/// for a reason the excuse can't read, a VM test, more checks than
/// `MOST`, a build that printed nothing, and one that printed more
/// than `MOST_LINES`.
#[test]
fn a_red_the_excuse_cant_read_asks_nothing() {
    let failed: Vec<(&str, &[&str])> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|&check| (check, &["x"] as &[&str]))
        .collect();
    let long: Vec<String> = (0..=MOST_LINES).map(|at| format!("line {at}")).collect();
    let long: Vec<&str> = long.iter().map(String::as_str).collect();
    let reds = [
        format!(
            "{}error: attribute 'x' missing\n",
            red(&[("cat", &["x"])], &[])
        ),
        "cat> x\nerror: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: timed out \
             after 3600 seconds of silence\n"
            .to_owned(),
        red(&[("vm-test-run-desktop-looks", &["x"])], &[]),
        red(&failed, &[]),
        red(&[("cat", &["x"]), ("quiet", &[])], &[]),
        red(&[("cat", &long)], &[]),
    ];
    for words in reds {
        let asked = AtomicBool::new(false);
        let ask = |_: &str| {
            asked.store(true, Ordering::SeqCst);
            Some(fails(&["x"]))
        };
        let again = |_: &str| {
            asked.store(true, Ordering::SeqCst);
            Some(true)
        };
        let excused = excused(&words, &asks(&ask, &again, &ask));
        assert!(excused.is_err(), "a red the excuse can't read was excused");
        assert!(
            !asked.load(Ordering::SeqCst),
            "a red that can't be excused asked of a run: {}",
            excused.unwrap_err()
        );
    }
}

#[test]
fn a_line_is_read_with_its_numbers_and_store_hashes_masked() {
    assert_eq!(
        masked("took 12.5 s in /nix/store/0123456789abcdfghijklmnpqrsvwxyz-niri/bin  "),
        "took #.# s in /nix/store/*-niri/bin",
        "a count or a hash that differs between runs failing alike kept them apart"
    );
    assert_eq!(
        masked("counted 0 of 10, 0.25 s, 007"),
        "counted 0 of #, 0.# s, #",
        "a lone 0 read as any number, or a number holding a 0 read as none"
    );
    assert_eq!(
        masked("/nix/store/short-x"),
        "/nix/store/short-x",
        "a store path with no hash lost its name"
    );
}

#[test]
fn nix_names_a_derivation_without_what_it_reads_as_a_version() {
    for (name, named) in [
        ("launcher-on-niri-part-6", "launcher-on-niri-part"),
        (
            "the-margin-cat-follows-the-keys-on-niri",
            "the-margin-cat-follows-the-keys-on-niri",
        ),
        ("niri-26.04", "niri"),
    ] {
        assert_eq!(nix_name(name), named, "nix's name for {name} was misread");
    }
}

#[test]
fn answers_kept_at_once_for_a_base_are_all_read_back() {
    let folder = super::super::tests::root("excused-kept");
    let state = State::at(folder.join("state"));
    std::thread::scope(|scope| {
        scope.spawn(|| keep(&folder, "b1", "cat", &fails(&["x", "x", "y"])));
        scope.spawn(|| keep(&folder, "b1", "long-files", &AtBase::Passes));
    });
    assert_eq!(
        known(&folder, &state, "b1", "cat"),
        Some(fails(&["x", "x", "y"])),
        "a kept failure wasn't read back, or lost a line or its count"
    );
    assert_eq!(
        known(&folder, &state, "b1", "long-files"),
        Some(AtBase::Passes),
        "a pass kept at the same time as a failure wasn't read back"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn what_the_train_keeps_goes_once_main_is_long_past_it() {
    let folder = super::super::tests::root("excused-aged");
    keep(&folder, "b1", "cat", &AtBase::Passes);
    fs::create_dir_all(again_folder(&folder, "c1")).unwrap();
    age_out(&folder, SystemTime::now());
    assert!(
        kept(&folder, "b1", "cat").is_some() && again_folder(&folder, "c1").exists(),
        "a fresh answer was aged out"
    );
    age_out(
        &folder,
        SystemTime::now() + KEPT_FOR + Duration::from_secs(60),
    );
    assert!(
        kept(&folder, "b1", "cat").is_none(),
        "an answer for a base three days gone was kept"
    );
    assert!(
        !again_folder(&folder, "c1").exists(),
        "a candidate's runs again from three days ago were kept"
    );
    let _ = fs::remove_dir_all(&folder);
}

/// A watch at `folder` that ran `check` at `b1` once each as `runs`
/// says, a failed run's log holding its words, or gone when it is none.
fn watch_of(folder: &Path, runs: &[(Ran, Option<&str>)]) -> State {
    let state = State::at(folder.join("state"));
    state.make().unwrap();
    for (at, (ran, words)) in runs.iter().enumerate() {
        let log = folder.join(format!("cat-{at}.log"));
        if let Some(words) = words {
            fs::write(&log, words).unwrap();
        }
        state.record("b1", "cat", *ran, Some(1), &log, 60).unwrap();
    }
    state
}

/// A failed run whose words don't say the check itself failed, as a
/// dependency's failure, or whose log is gone, is left out rather
/// than losing the runs beside it.
#[test]
fn the_watch_s_failed_runs_at_the_base_are_each_read() {
    let folder = super::super::tests::root("excused-watched");
    let first = format!(
        "cat> counted 3 frames\ncat> slow\n{}\n",
        cannot_build("cat")
    );
    let followed = "error: Cannot build '/nix/store/0x1c-cat.drv'.\n       Reason: 1 \
                        dependency failed.\n";
    let state = watch_of(
        &folder,
        &[
            (Ran::Failed, Some(&first)),
            (Ran::Failed, Some(followed)),
            (Ran::Failed, None),
        ],
    );
    assert_eq!(
        known(&folder, &state, "b1", "cat"),
        Some(fails(&["counted # frames", "slow"])),
        "a failed run beside one that said nothing of the check wasn't read"
    );
    let _ = fs::remove_dir_all(&folder);
}

/// A pass anywhere it is known, the watch's or a lent run's, says the
/// check isn't main's own, whatever failed beside it.
#[test]
fn a_pass_the_base_ever_had_wins() {
    let folder = super::super::tests::root("excused-passes");
    let failed = format!("cat> x\n{}\n", cannot_build("cat"));
    let state = watch_of(
        &folder,
        &[(Ran::Failed, Some(&failed)), (Ran::Passed, None)],
    );
    assert_eq!(
        known(&folder, &state, "b1", "cat"),
        Some(AtBase::Passes),
        "a check the watch saw pass at the base read as failing there"
    );
    let _ = fs::remove_dir_all(&folder);
    let state = watch_of(&folder, &[(Ran::Failed, Some(&failed))]);
    keep(&folder, "b1", "cat", &AtBase::Passes);
    assert_eq!(
        known(&folder, &state, "b1", "cat"),
        Some(AtBase::Passes),
        "a kept pass lost to the watch's failure"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_mac_s_comparison_asks_nothing() {
    let folder = super::super::tests::root("excused-mac");
    let state = State::at(folder.join("state"));
    let check = crate::macs::check("b1");
    keep(&folder, "b1", &check, &AtBase::Passes);
    assert_eq!(
        known(&folder, &state, "b1", &check),
        None,
        "a Mac's check was read"
    );
    assert_eq!(
        ran(&folder, "o/r", "b1", &check, &never, &|| false),
        None,
        "a Mac's check was run"
    );
    let _ = fs::remove_dir_all(&folder);
}

/// A runner that appends `words` to its log, as nix-offload's file
/// is opened, and exits `code`, after checking it was asked to build
/// `wanted` at `commit`.
fn appending<'a>(
    wanted: &'a [String],
    commit: &'a str,
    words: &'a str,
    code: Option<i32>,
) -> impl Fn(&[String], &str, &Path, &dyn Fn() -> bool) -> Option<i32> + Sync + 'a {
    move |args, at, log, _| {
        assert_eq!(args, wanted, "the run was asked for something else");
        assert_eq!(at, commit, "the run was asked at another commit");
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap();
        std::io::Write::write_all(&mut file, words.as_bytes()).unwrap();
        code
    }
}

#[test]
fn a_lent_run_at_the_base_reads_its_own_words_alone() {
    let folder = super::super::tests::root("excused-lent");
    let wanted: Vec<String> = ["there", "--lent-only", "-L", "github:o/r/b1"]
        .into_iter()
        .map(str::to_owned)
        .chain([format!("checks.{SYSTEM}.cat")])
        .collect();
    let failing = format!("cat> counted 4 frames\n{}\n", cannot_build("cat"));
    let cases: [(&str, Option<i32>, Option<AtBase>); 4] = [
        ("", Some(0), Some(AtBase::Passes)),
        (&failing, Some(1), Some(fails(&["counted # frames"]))),
        ("", Some(1), None),
        ("", Some(75), None),
    ];
    for (words, code, answer) in cases {
        let run = appending(&wanted, "b1", words, code);
        assert_eq!(
            ran(&folder, "o/r", "b1", "cat", &run, &|| false),
            answer,
            "a lent run that exited {code:?} was misread, or read an earlier run's words"
        );
    }
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_check_runs_again_from_the_candidate_s_checkout() {
    let folder = super::super::tests::root("excused-again");
    let checkout = folder.join("checkout");
    let wanted: Vec<String> = ["there", "--lent-only", "-L"]
        .into_iter()
        .map(str::to_owned)
        .chain([
            checkout.display().to_string(),
            format!("checks.{SYSTEM}.cat"),
        ])
        .collect();
    for (code, answer) in [
        (Some(0), Some(true)),
        (Some(1), Some(false)),
        (Some(100), Some(false)),
        (Some(75), None),
        (Some(69), None),
        (None, None),
    ] {
        let run = appending(&wanted, "c1", "", code);
        assert_eq!(
            again(&folder, &checkout, "c1", "cat", &run, &|| false),
            answer,
            "a run again that exited {code:?} was misread"
        );
    }
    let _ = fs::remove_dir_all(&folder);
}

/// A net whose base would excuse the red, given the candidate's log,
/// its train folder and the red to excuse.
fn excusable(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = super::super::tests::root(name);
    let train = super::super::folder(&root);
    keep(&train, "b1", "cat", &fails(&["x"]));
    let log = root.join("candidate.log");
    fs::write(&log, red(&[("cat", &["x"])], &[])).unwrap();
    (root, train, log)
}

#[test]
fn a_net_told_no_excuse_excuses_nothing() {
    let (root, train, log) = excusable("excused-off");
    let red = Red {
        root: &root,
        slug: "o/r",
        checkout: &root,
        base: "b1",
        candidate: "c1",
        log: &log,
    };
    assert_eq!(
        excuse(&red, &never),
        Some(vec!["cat".to_owned()]),
        "a red its base excuses wasn't excused"
    );
    fs::write(train.join("no-excuse"), "").unwrap();
    assert_eq!(
        excuse(&red, &never),
        None,
        "a red was excused with excuses turned off"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn excuses_turned_off_while_it_asks_let_its_runs_go() {
    let (root, train, log) = excusable("excused-turned-off");
    fs::write(&log, red(&[("cat", &["y"])], &[])).unwrap();
    let run = |_: &[String], _: &str, _: &Path, dropped: &dyn Fn() -> bool| {
        assert!(
            !dropped(),
            "a run was let go before excuses were turned off"
        );
        fs::write(train.join("no-excuse"), "").unwrap();
        assert!(
            dropped(),
            "a run wasn't let go once excuses were turned off"
        );
        Some(0)
    };
    let red = Red {
        root: &root,
        slug: "o/r",
        checkout: &root,
        base: "b1",
        candidate: "c1",
        log: &log,
    };
    assert_eq!(
        excuse(&red, &run),
        None,
        "a red was excused after excuses were turned off"
    );
    let _ = fs::remove_dir_all(&root);
}
