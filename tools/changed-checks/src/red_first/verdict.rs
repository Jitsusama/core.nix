//! What red first found of a change, and how the run's line notes it.

use serde_json::json;

use super::Excused;

/// What red first found of a change, as the run's line notes it. Where it
/// built checks over the base, it notes how many, so a day of gated runs
/// shows what judging costs.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It wasn't judged, and why, in a word or two.
    Unjudged(&'static str),
    /// Its tests can't be laid over the base as its head has them, each
    /// as it was said.
    Apart(Vec<String>),
    /// A commit excuses its tests, saying why, or it reverts.
    Excused(Excused),
    /// This check fails with its tests laid over the base, as it should.
    Red {
        check: String,
        tests: Vec<String>,
        built: usize,
    },
    /// This check, the first of those built over the base with no builder
    /// failing it, couldn't be built there, nix's last error saying why,
    /// so it says nothing of the tests and the change isn't judged.
    Unbuilt {
        check: String,
        reason: String,
        built: usize,
    },
    /// Red first's budget ran out at this check, building it or before it
    /// began, so the change isn't judged.
    OverBudget { check: String, built: usize },
    /// Each check built over the base passed, so its tests would pass
    /// without it.
    Refused {
        checks: Vec<String>,
        tests: Vec<String>,
        built: usize,
    },
}

impl Verdict {
    /// The verdict as the run's line holds it, of the commit `head`.
    pub fn noted(&self, head: &str) -> serde_json::Value {
        match self {
            Self::Unjudged(why) => json!({ "head": head, "verdict": "unjudged", "why": why }),
            Self::Apart(said) => json!({ "head": head, "verdict": "apart", "apart": said }),
            Self::Excused(Excused::Pinned(why)) => {
                json!({ "head": head, "verdict": "excused", "excuse": "pinned", "why": why })
            }
            Self::Excused(Excused::Reverts) => {
                json!({ "head": head, "verdict": "excused", "excuse": "reverts" })
            }
            Self::Red {
                check,
                tests,
                built,
            } => json!({
                "head": head, "verdict": "red", "check": check, "tests": tests, "built": built,
            }),
            Self::Unbuilt {
                check,
                reason,
                built,
            } => json!({
                "head": head, "verdict": "unjudged", "why": "unbuilt", "check": check,
                "reason": reason, "built": built,
            }),
            Self::OverBudget { check, built } => json!({
                "head": head, "verdict": "unjudged", "why": "over budget", "check": check,
                "built": built,
            }),
            Self::Refused {
                checks,
                tests,
                built,
            } => json!({
                "head": head, "verdict": "would refuse", "checks": checks, "tests": tests,
                "built": built,
            }),
        }
    }
}
