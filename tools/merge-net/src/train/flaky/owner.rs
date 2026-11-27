//! Which merge owns a check the train excused, read as the net's watch
//! reads one, but in the train's own checkout and over a bounded range.

use std::path::Path;
use std::process::Command;

use super::Red;
use crate::state::State;

/// The most merges before a red's base an owner is looked for among. Each
/// costs up to four evaluations of the whole flake on a lent builder, run
/// inside the ride before it lands, so a net that hasn't passed a head for
/// many merges looks no further back than these. Main gained up to 182
/// merges a day, and 27 in an hour, in the week to 27 November, and the
/// last passed head stands still while main is red, which is when a red
/// is main's, so unbounded a range could then run to scores of merges.
pub(super) const MOST_OWNER_MERGES: usize = 4;

/// Why a finding names no owner of its check, so the train's log tells a
/// range never read from one read in vain.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unowned {
    /// The net has passed no head yet, so there is no range to read.
    NothingPassed,
    /// git couldn't count the merges from the last passed head to the
    /// base, as when the checkout lacks either.
    Uncounted,
    /// None of the newest `looked` of the `since` merges from the last
    /// passed head to the base moved the check.
    NoMerge { looked: usize, since: usize },
}

impl Unowned {
    /// How the train's log says why no owner of `check` is named.
    pub(super) fn said(&self, check: &str) -> String {
        match self {
            Self::NothingPassed => {
                format!("no owner of {check} is named, as the net has passed no head yet")
            }
            Self::Uncounted => format!(
                "no owner of {check} is named, as git couldn't count the merges since the \
                 last passed head"
            ),
            Self::NoMerge { looked, since } if looked < since => format!(
                "no merge of the newest {looked} of {since} since the last passed head owns \
                 {check}"
            ),
            Self::NoMerge { looked, .. } => {
                format!("no merge of the {looked} since the last passed head owns {check}")
            }
        }
    }
}

/// The PR that owns `check`, failed beside `failed`, as the net's watch
/// reads an owner: the newest merge from the head it last passed to the
/// candidate's base that moved the check, at the repository `url`, under
/// the state folder `root`, looking at no more than `MOST_OWNER_MERGES`;
/// otherwise why none is named. The merges are read in the train's
/// checkout, which has just fetched the base, rather than the watch's
/// clone, which may not hold it.
pub(super) fn owner_in(
    root: &Path,
    url: &str,
    red: &Red<'_>,
    check: &str,
    failed: &[String],
) -> Result<u64, Unowned> {
    let passed = State::at(root.to_path_buf())
        .heads()
        .0
        .ok_or(Unowned::NothingPassed)?;
    let range = format!("{passed}..{}", red.base);
    let since = Command::new("git")
        .args(["rev-list", "--first-parent", "--count", &range])
        .current_dir(red.checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())
        .and_then(|ran| {
            String::from_utf8_lossy(&ran.stdout)
                .trim()
                .parse::<usize>()
                .ok()
        })
        .ok_or(Unowned::Uncounted)?;
    let from = if since > MOST_OWNER_MERGES {
        format!("{}~{MOST_OWNER_MERGES}", red.base)
    } else {
        passed
    };
    let world = crate::world::Real {
        state: root.to_path_buf(),
        checkout: red.checkout.to_path_buf(),
        url: url.to_owned(),
        system: crate::SYSTEM.to_owned(),
        booted: 0,
        branch: "main".to_owned(),
        moving: std::sync::Mutex::new(()),
        dropped: std::sync::Mutex::default(),
        follow: None,
    };
    crate::watch::owner_of(&world, &from, red.base, check, failed).ok_or(Unowned::NoMerge {
        looked: since.min(MOST_OWNER_MERGES),
        since,
    })
}
