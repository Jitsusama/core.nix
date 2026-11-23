//! How a run ends, as the code it exits with. Only a comparison whose
//! every build ran, and one failed, ends 1: nix-offload reads 1 as a part
//! that ran to its end, and merge-net's train excuses a red only from such
//! a run (`tools/merge-net/src/train/gated.rs`). A check that no longer
//! evaluates, a build cut short, an error part way and a panic each end
//! with a code of their own, so none is read as a whole red (round 5, 23
//! November).

use terminal_kit::{Exit, Failure};

/// How a comparison ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compared {
    /// Every check it named evaluates, and each built passed.
    Passed,
    /// A check no longer evaluates, so nothing was built.
    Broken,
    /// A check it built failed.
    Failed,
    /// A build was cut short, by a signal or a lost builder, so not every
    /// check it named was built.
    Cut,
}

/// What a build said of itself: whether it passed, and whether every
/// build it started ran to its end.
#[derive(Debug, Clone, Copy)]
pub struct Built {
    pub passed: bool,
    pub whole: bool,
}

impl Compared {
    /// The verdict of a comparison whose checks all evaluated when
    /// `evaluates`, of the build it ran, if any. Passing wins over a cut,
    /// since what passed was built.
    pub fn of(evaluates: bool, build: Option<Built>) -> Self {
        match build {
            _ if !evaluates => Self::Broken,
            None | Some(Built { passed: true, .. }) => Self::Passed,
            Some(Built { whole: false, .. }) => Self::Cut,
            Some(Built { whole: true, .. }) => Self::Failed,
        }
    }

    /// How the run ends of it.
    pub fn ending(self) -> Result<(), Failure> {
        match self {
            Self::Passed => Ok(()),
            // Not 1: a check the riders broke never built, so a red beside
            // it doesn't name all the change broke.
            Self::Broken => Err(Failure::new(
                Exit::BadInput,
                "a check the change touched no longer evaluates",
            )),
            Self::Failed => Err(Failure::new(
                Exit::Failed,
                "a check the change touched failed",
            )),
            // Not 1, and not 69, which merge-net reads as no builder taking
            // it: a build that was cut short, by a signal or a lost builder.
            Self::Cut => Err(Failure::new(
                Exit::Bug,
                "a build of the checks the change touched was cut short, so not every check was \
                 built",
            )),
        }
    }
}

/// A run that couldn't go on, as it said why: never 1, since a build may
/// already have printed its failures when it stopped.
pub fn broke(why: String) -> Failure {
    Failure::new(Exit::Bug, why)
}

/// End the process at a panic, after saying it, rather than unwinding to
/// Rust's 101, which is also a code nix ends a failed build with. An abort
/// reads as a signal, so as cut short.
pub fn abort_on_panic() {
    let said = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        said(panic);
        std::process::abort();
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The code `compared` ends a run with.
    fn code(compared: Compared) -> u8 {
        compared
            .ending()
            .map_or_else(|failure| failure.exit.code(), |()| 0)
    }

    #[test]
    fn only_a_failure_every_build_ran_to_ends_one() {
        assert_eq!(code(Compared::Passed), 0, "a pass didn't end 0");
        assert_eq!(code(Compared::Failed), 1, "a whole failure didn't end 1");
        for compared in [Compared::Broken, Compared::Cut] {
            let ended = code(compared);
            assert!(
                ![0, 1, 69, 75].contains(&ended) && !(100..=104).contains(&ended),
                "{compared:?} ended {ended}, a code read as whole or untaken"
            );
        }
        assert_ne!(
            broke("nix eval failed: signal: 9".into()).exit.code(),
            1,
            "an error part way ended 1"
        );
    }

    #[test]
    fn a_verdict_is_broken_first_then_passed_then_cut() {
        let built = |passed, whole| Some(Built { passed, whole });
        let cases = [
            (false, built(true, true), Compared::Broken),
            (false, None, Compared::Broken),
            (true, None, Compared::Passed),
            (true, built(true, false), Compared::Passed),
            (true, built(false, false), Compared::Cut),
            (true, built(false, true), Compared::Failed),
        ];
        for (evaluates, build, verdict) in cases {
            assert_eq!(
                Compared::of(evaluates, build),
                verdict,
                "evaluates {evaluates}, built {build:?}"
            );
        }
    }
}
