//! Which merge owns a check the train excused, read as the net's watch
//! reads one, but in the train's own checkout and over a bounded range.

use std::path::Path;
use std::process::Command;

use super::Red;
use crate::state::State;

/// The most merges before a red's base an owner is looked for among. Each
/// costs up to four evaluations of the whole flake on a lent builder, run
/// inside the ride before it lands, so a net that hasn't passed a head for
/// many merges looks no further back than these.
pub(super) const MOST_OWNER_MERGES: usize = 4;

/// The PR that owns `check`, failed beside `failed`, as the net's watch
/// reads an owner: the newest merge from the head it last passed to the
/// candidate's base that moved the check, at the repository `url`, under
/// the state folder `root`, looking at no more than `MOST_OWNER_MERGES`;
/// none where it passed nothing yet, git can't count the range, or no
/// such merge is found. The merges are read in the train's checkout, which
/// has just fetched the base, rather than the watch's clone, which may not
/// hold it.
pub(super) fn owner_in(
    root: &Path,
    url: &str,
    red: &Red<'_>,
    check: &str,
    failed: &[String],
) -> Option<u64> {
    let passed = State::at(root.to_path_buf()).heads().0?;
    let since = format!("{passed}..{}", red.base);
    let behind = Command::new("git")
        .args(["rev-list", "--first-parent", "--count", &since])
        .current_dir(red.checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())
        .and_then(|ran| {
            String::from_utf8_lossy(&ran.stdout)
                .trim()
                .parse::<usize>()
                .ok()
        })?;
    let from = if behind > MOST_OWNER_MERGES {
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
    crate::watch::owner_of(&world, &from, red.base, check, failed)
}
