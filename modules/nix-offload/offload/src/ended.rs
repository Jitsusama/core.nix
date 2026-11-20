//! How a program nix-offload runs as its stand-in ends it: nix, cargo, a
//! command on a builder. Its own code passes through, as the standard's A
//! stand-in passes through says (docs/commands.md), and every other ending
//! is a code from the kit's table. A stand-in said why it failed in its
//! own words, and a gate in its line and log, so their endings are said
//! already: the kit adds no line of its own, which in a unit's journal
//! would be marked an error at every refused gate and failed build.

use terminal_kit::{Exit, Failure};

/// Done when `status` is 0; otherwise the stand-in's own code, `what`
/// failing. A status no exit code holds (above 255, or below 0) ends 1,
/// as it did before the kit.
pub fn passed(status: i32, what: impl FnOnce() -> String) -> Result<(), Failure> {
    if status == 0 {
        return Ok(());
    }
    Err(Failure::passed(u8::try_from(status).unwrap_or(1), what()).said_already())
}

/// Done when `code` is 0; otherwise the table's exit of that code, or a
/// stand-in's code passed through where the table holds none, `what`
/// failing. A gate's result ends this way, since its code is a refusal's
/// or changed-checks' own.
pub fn coded(code: u8, what: impl FnOnce() -> String) -> Result<(), Failure> {
    let failure = match Exit::of(code) {
        Some(Exit::Done) => return Ok(()),
        Some(exit) => Failure::new(exit, what()),
        None => Failure::passed(code, what()),
    };
    Err(failure.said_already())
}

/// How a lender ends, `superseded` or told to end: superseded, with the
/// table's try-again, 75, which its unit counts as a success and starts
/// it again on (home.nix's `SuccessExitStatus`). Handing over is the
/// unit's sign, not a failure, so nothing is said: a line marked as an
/// error at every switch would read as one in the day's health. This is
/// the only way a lender ends 75, since the unit counts every 75 a
/// handover.
pub fn lent(superseded: bool) -> Result<(), Failure> {
    if superseded {
        return Err(
            Failure::new(Exit::TryAgain, "the lender handed over to its successor").said_already(),
        );
    }
    Ok(())
}

/// A run that backs off to its caller with `exit`, `what` saying why on a
/// line of its own as it always has. Its callers (changed-checks, its
/// keep of main, cargo) read the code and go on, so the kit adds no line
/// of its own, which a unit's journal would mark an error.
pub fn backs_off(exit: Exit, what: impl Into<String>) -> Failure {
    let what = what.into();
    eprintln!("nix-offload: {what}");
    Failure::new(exit, what).said_already()
}

/// As [`coded`], for a status as a child gave it: one no exit code holds
/// (above 255, or below 0) ends 1, as it did before the kit. `there`'s
/// builds end this way, since a status is nix's own or one of `there`'s
/// that its callers read by number.
pub fn status(status: i32, what: impl FnOnce() -> String) -> Result<(), Failure> {
    coded(u8::try_from(status).unwrap_or(1), what)
}

/// Asked wrongly: an unknown word, or a value it can't take.
pub fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what)
}

/// Asked wrongly, saying what, and the help of the subcommand `sub` to
/// read for how it's asked.
pub fn asked_wrongly(what: impl Into<String>, sub: &str) -> Failure {
    usage(what).try_this(format!("nix-offload {sub} --help"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stand_in_that_ended_done_ends_done() {
        assert_eq!(passed(0, || "unsaid".into()), Ok(()));
    }

    #[test]
    fn a_stand_in_that_failed_ends_with_its_own_code() {
        let ended = passed(101, || "cargo test failed".into()).expect_err("101 isn't done");
        assert_eq!(ended.code(), 101);
        assert_eq!(ended.what, "cargo test failed");
    }

    #[test]
    fn a_code_the_table_holds_ends_as_its_exit_and_another_passes_through() {
        assert_eq!(coded(0, || "unsaid".into()), Ok(()));
        let refused = coded(65, || "the gate refused it".into()).expect_err("65 isn't done");
        assert_eq!(refused.exit, Exit::BadInput);
        let failed = coded(3, || "a check failed".into()).expect_err("3 isn't done");
        assert_eq!((failed.exit, failed.code()), (Exit::Failed, 3));
    }

    #[test]
    fn a_status_no_exit_holds_ends_one() {
        for status in [-1, 256, 9000] {
            let ended = passed(status, || "it failed".into()).expect_err("it isn't done");
            assert_eq!(ended.code(), 1, "{status} ended {}", ended.code());
            let ended = super::status(status, || "it failed".into()).expect_err("not done");
            assert_eq!(ended.code(), 1, "a build's {status} ended {}", ended.code());
        }
    }

    #[test]
    fn a_childs_status_keeps_its_number() {
        assert_eq!(status(0, || "unsaid".into()), Ok(()));
        for code in [64, 75, 100, 3] {
            let ended = status(code, || "nix failed".into()).expect_err("it isn't done");
            assert_eq!(
                ended.code(),
                u8::try_from(code).unwrap_or(0),
                "{code} moved"
            );
        }
    }

    #[test]
    fn what_a_stand_in_said_is_never_said_again() {
        for ended in [
            passed(101, || "cargo".into()),
            coded(65, || "refused".into()),
            coded(3, || "a check".into()),
            status(100, || "nix".into()),
        ] {
            let failure = ended.expect_err("it isn't done");
            assert!(failure.said, "{} would be said again", failure.what);
        }
    }

    #[test]
    fn a_lender_that_handed_over_ends_as_its_unit_counts_a_success() {
        let ended = lent(true).expect_err("a superseded lender ends a way its unit reads");
        assert_eq!(
            (ended.code(), ended.said),
            (75, true),
            "home.nix's SuccessExitStatus counts 75, said by nothing"
        );
    }

    #[test]
    fn a_lender_told_to_end_ends_done() {
        assert!(
            lent(false).is_ok(),
            "a lender stopped, not superseded, ends 0 as it did"
        );
    }

    #[test]
    fn a_run_that_backs_off_adds_no_line_of_the_kits() {
        let ended = backs_off(Exit::TryAgain, "no lent builder took it, so nothing ran");
        assert_eq!(
            (ended.code(), ended.said),
            (75, true),
            "its own line says why, and its caller reads 75"
        );
    }
}
