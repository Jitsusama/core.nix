//! How a run ends, as the code it exits with. Only a comparison whose
//! every build ran, and one failed, ends 1: nix-offload reads 1 as a part
//! that ran to its end, and merge-net's train excuses a red only from such
//! a run (`tools/merge-net/src/train/gated.rs`). A check that no longer
//! evaluates, a build cut short and an error part way each end with a
//! code of their own, so none is read as a whole red (round 5, 23
//! November). A panic in the body ends 70 through terminal-kit's `caught`,
//! never Rust's 101 (round 6).

use terminal_kit::{Exit, Failure};

/// What a run cut short ends with: terminal-kit's bug, 70, as `Cut` and
/// `broke` end.
pub const CUT_SHORT: u8 = 70;

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
        assert_eq!(
            code(Compared::Cut),
            CUT_SHORT,
            "a cut run ended other than a sent run cut short does"
        );
        let broken = broke("nix eval failed: signal: 9".into()).exit.code();
        for (compared, ended) in [
            ("Broken", code(Compared::Broken)),
            ("Cut", code(Compared::Cut)),
            ("an error part way", broken),
        ] {
            assert!(
                ![0, 1, 69, 75].contains(&ended) && !(100..=104).contains(&ended),
                "{compared} ended {ended}, a code read as whole or untaken"
            );
        }
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
