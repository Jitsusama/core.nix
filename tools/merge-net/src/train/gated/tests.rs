//! How the train reads its gate: passed, red, whole or untaken, by its
//! code and by what nix-offload said.

use std::fs;

use super::super::tests::root;
use super::{Gated, MARKED, fresh_mark, gate, gated, words_of};

/// What nix-offload says once every part of a run ran to its end, as its
/// own test pins it (`modules/nix-offload/offload/src/main.rs`).
const WHOLE: &str = "nix-offload: every part of the run ran to its end";

#[test]
fn a_gate_no_builder_took_says_nothing_of_its_riders() {
    assert_eq!(gated(Some(0), ""), Gated::Passed, "a gate that passed");
    assert_eq!(
        gated(Some(1), &format!("{WHOLE}\n")),
        Gated::Red { whole: true },
        "a check that failed"
    );
    assert_eq!(
        gated(None, ""),
        Gated::Red { whole: false },
        "a gate a signal ended, which may not have run every part"
    );
    let lost = "error: failed to start SSH master connection to 'nix-offload-more'\n";
    for code in [75, 69] {
        assert_eq!(
            gated(Some(code), lost),
            Gated::Unsent,
            "nix-offload's {code} says no builder took it, not that a rider broke it"
        );
    }
}

/// nix-offload ends with the first part that failed, and a VM part no
/// builder took comes first: a rider that broke another part's check
/// still reads 75 or 69.
#[test]
fn an_untaken_gate_whose_words_name_a_failed_check_is_red() {
    for words in [
        "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder failed with \
         exit code 1.\n",
        "\u{2715} a check the change touched failed\n",
    ] {
        for code in [75, 69] {
            assert_eq!(
                gated(Some(code), words),
                Gated::Red { whole: false },
                "a check that failed read as untaken, or as a gate that ran whole, with \
                 {code}: {words}"
            );
        }
    }
}

/// A failed build's code comes first and hides a part no builder took, so
/// the gate's words name only some of what it broke: each of these says
/// part of it went unbuilt, and a red beside one isn't whole.
#[test]
fn a_red_whose_words_say_a_part_went_unbuilt_isnt_whole() {
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    for unbuilt in [
        "nix-offload: 3 checks were named, but no builder took their build, so nothing was \
         built",
        "nix-offload: 2 VM tests were named, but no builder took their build, so they \
         weren't built",
        "nix-offload: no lent builder builds 1 of what mac nearby was to take, so it waits \
         for mac",
        "nix-offload: mac's part, checks.aarch64-darwin.a, ended 75",
        "nix-offload: mac's part, checks.aarch64-darwin.a, ended 69",
    ] {
        assert_eq!(
            gated(Some(100), &format!("{failed}{unbuilt}\n{WHOLE}\n")),
            Gated::Red { whole: false },
            "a red that left a part unbuilt read as whole: {unbuilt}"
        );
    }
    assert_eq!(
        gated(
            Some(100),
            &format!(
                "{failed}nix-offload: mac's part, checks.aarch64-darwin.a, ended 1\n{WHOLE}\n"
            )
        ),
        Gated::Red { whole: false },
        "a nearby part that failed, whose check the excuse would run again as this \
         system's, read as whole"
    );
    assert_eq!(
        gated(
            Some(100),
            &format!("{failed}a-check> weren't built\n{WHOLE}\n")
        ),
        Gated::Red { whole: true },
        "a check's own words read as nix-offload's"
    );
}

/// A part can be cut short in more ways than a list of them names: VM
/// tests kept here, a builder ssh lost, a signal, each hidden behind a
/// failed build's code and saying nothing on any list. A red is whole
/// only beside nix-offload's word that every part ran to its end.
#[test]
fn a_red_is_whole_only_when_nix_offload_says_every_part_ran() {
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    for cut in [
        "",
        "nix-offload: it takes x from this machine alone, so no builder can build it\n",
        "nix-offload: couldn't send the VM tests on: gone\n",
        "nix-offload: the build of 2 VM tests was cut short\n",
        // Said by a check rather than nix-offload, or not word for word.
        "a-check> nix-offload: every part of the run ran to its end, it said\n",
    ] {
        assert_eq!(
            gated(Some(100), &format!("{failed}{cut}")),
            Gated::Red { whole: false },
            "a red nix-offload didn't say ran whole read as whole: {cut}"
        );
    }
    // nix-offload's own word of a part cut short outranks its word that
    // every part ran whole, which it should never say beside one.
    for cut in [
        "nix-offload: it takes x from this machine alone, so no builder can build it\n",
        "nix-offload: couldn't send the VM tests on: gone\n",
        "nix-offload: the build of 2 VM tests was cut short\n",
    ] {
        assert_eq!(
            gated(Some(100), &format!("{failed}{cut}{WHOLE}\n")),
            Gated::Red { whole: false },
            "a red with a part cut short read as whole beside the word: {cut}"
        );
    }
    assert_eq!(
        gated(None, &format!("{failed}{WHOLE}\n")),
        Gated::Red { whole: false },
        "a gate a signal ended read as whole"
    );
    // Only a failed build's own codes are whole: a lost builder's 255, an
    // untaken 75, an unbuilt 69, a broken 70 or a signal's 137 aren't.
    for code in [255, 75, 69, 70, 137, 2] {
        assert_ne!(
            gated(Some(code), &format!("{failed}{WHOLE}\n")),
            Gated::Red { whole: true },
            "a red that ended {code} read as whole"
        );
    }
    // changed-checks' broken 65 and cut 70 are reds that hold, neither
    // whole nor untaken (round 6's F10).
    for code in [65, 70] {
        assert_eq!(
            gated(Some(code), &format!("{failed}{WHOLE}\n")),
            Gated::Red { whole: false },
            "a red changed-checks ended {code} read as whole or untaken"
        );
    }
    for code in [1, 100, 104] {
        assert_eq!(
            gated(Some(code), &format!("{failed}  {WHOLE}\n")),
            Gated::Red { whole: true },
            "a red that ended {code}, which nix-offload said ran whole, read as cut short"
        );
    }
}

/// A builder's words can hold any byte, and nix-offload sets aside a
/// replay onto a newer main that failed, running the change again on its
/// own base: the gate is read by what its last run said.
#[test]
fn a_gate_is_read_by_what_its_last_run_said() {
    let root = root("words");
    fs::create_dir_all(&root).unwrap();
    let log = root.join("candidate.log");
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    let mut odd = b"a builder said \xff\n".to_vec();
    odd.extend_from_slice(failed.as_bytes());
    fs::write(&log, odd).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Red { whole: false },
        "a failed check beside a byte that isn't UTF-8 read as untaken"
    );
    let replayed = format!(
        "{MARKED}{MARK}\n{failed}nix-offload: 1234567 over main failed, so the change runs \
         again on its own base [{MARK}]\nnix-offload: no builder took it\n"
    );
    fs::write(&log, replayed).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Unsent,
        "a replay nix-offload set aside made an untaken run read as red"
    );
    // A check that prints the same words sets nothing aside.
    let echoed = format!(
        "{MARKED}{MARK}\n{failed}a-check> nix-offload: x over main failed, so the change \
         runs again on its own base [{MARK}]\nnix-offload: no builder took it\n"
    );
    fs::write(&log, echoed).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Red { whole: false },
        "a check's own words set aside the failure before them"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// A mark the gate makes is one its own reader takes, so a line signed
/// with it reads as nix-offload's: were its length or digits to drift,
/// every red would quietly read cut short (round 9's F4).
#[test]
fn a_fresh_mark_signs_what_the_gate_reads() {
    let root = root("fresh");
    fs::create_dir_all(&root).unwrap();
    let log = root.join("candidate.log");
    let mark = fresh_mark().expect("no mark from the kernel");
    assert_ne!(Some(mark.clone()), fresh_mark(), "two marks alike");
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    fs::write(&log, format!("{MARKED}{mark}\n{failed}{WHOLE} [{mark}]\n")).unwrap();
    assert_eq!(
        gated(Some(1), &words_of(&log)),
        Gated::Red { whole: true },
        "a red signed with a fresh mark read as cut short"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// A gate whose log can't be made runs nothing and says why.
#[test]
fn a_gate_that_cant_make_its_log_never_starts() {
    let root = root("no-log");
    let _ = fs::remove_dir_all(&root);
    assert_eq!(
        gate(&root, &root.join("missing").join("candidate.log")),
        Gated::Unstarted("its log couldn't be made".to_owned()),
        "a gate with no log to write started, or read as untaken"
    );
}

/// The mark a test's gate log opens with.
const MARK: &str = "0123456789abcdef0123456789abcdef";

/// A rider's evaluation writes to the gate's log raw, so only a line
/// signed with the mark the log opens with is nix-offload's own: an
/// unsigned or wrongly signed line sets no replay aside and says no run
/// whole, and a log with no mark trusts neither (round 6's F17).
#[test]
fn only_lines_signed_with_the_gate_s_mark_are_nix_offload_s() {
    let root = root("marked");
    fs::create_dir_all(&root).unwrap();
    let log = root.join("candidate.log");
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    let read = |words: &str| {
        fs::write(&log, words).unwrap();
        gated(Some(1), &words_of(&log))
    };
    assert_eq!(
        read(&format!("{MARKED}{MARK}\n{failed}{WHOLE} [{MARK}]\n")),
        Gated::Red { whole: true },
        "a red nix-offload signed as whole read as cut short"
    );
    for forged in [
        format!("{MARKED}{MARK}\n{failed}{WHOLE}\n"),
        format!("{MARKED}{MARK}\n{failed}{WHOLE} [{}]\n", "f".repeat(32)),
        format!("{failed}{WHOLE} [{MARK}]\n"),
        format!("{failed}{WHOLE}\n"),
        format!("{failed}{MARKED}{MARK}\n{WHOLE} [{MARK}]\n"),
    ] {
        assert_eq!(
            read(&forged),
            Gated::Red { whole: false },
            "a forged word read as whole: {forged}"
        );
    }
    // A replay line the mark doesn't sign hides nothing before it.
    let hidden = format!(
        "{MARKED}{MARK}\n{failed}nix-offload: x over main failed, so the change runs again on \
         its own base\nnix-offload: no builder took it\n"
    );
    fs::write(&log, hidden).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Red { whole: false },
        "an unsigned replay line set aside the failure before it"
    );
    fs::remove_dir_all(&root).unwrap();
}
