//! How a program nix-offload runs as its stand-in ends it: nix, cargo, a
//! command on a builder. Its own code passes through, as the standard's A
//! stand-in passes through says (docs/commands.md), and every other ending
//! is a code from the kit's table.

use terminal_kit::{Exit, Failure};

/// Done when `status` is 0; otherwise the stand-in's own code, saying
/// `what` failed. A status no exit code holds (above 255, or below 0)
/// ends 1, as it did before the kit.
pub fn passed(status: i32, what: impl FnOnce() -> String) -> Result<(), Failure> {
    if status == 0 {
        return Ok(());
    }
    Err(Failure::passed(u8::try_from(status).unwrap_or(1), what()))
}

/// Done when `code` is 0; otherwise the table's exit of that code, or a
/// stand-in's code passed through where the table holds none, saying
/// `what` failed. A gate's result ends this way, since its code is a
/// refusal's or changed-checks' own.
pub fn coded(code: u8, what: impl FnOnce() -> String) -> Result<(), Failure> {
    match Exit::of(code) {
        Some(Exit::Done) => Ok(()),
        Some(exit) => Err(Failure::new(exit, what())),
        None => Err(Failure::passed(code, what())),
    }
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
        }
    }
}
