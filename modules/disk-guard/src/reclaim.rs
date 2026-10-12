//! Reclaiming one candidate, under the rules no source can loosen.

use crate::activity::Use;
use crate::anchor::Dir;
use crate::locks::{Hold, hold_cargo};
use crate::pass::Outcome;
use crate::plan::Candidate;
use std::ffi::OsStr;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// What the reclaimer asks of the machine at each moment.
pub trait Looker {
    /// How the target is used right now.
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> io::Result<Use>;
    /// Who waits on the locks the guard holds on these inodes.
    fn waiters(&mut self, inodes: &[u64]) -> io::Result<Vec<i32>>;
}

/// What survives a target's removal: benchmark baselines, which nothing
/// regenerates.
const KEEP: [&str; 1] = ["criterion"];

/// Why a use keeps the target, if it does. A tree someone sits in goes only
/// when the plan took it as occupied.
fn refusal(used: Use, planned_occupied: bool) -> Option<String> {
    match used {
        Use::Free => None,
        Use::Busy(pid) => Some(format!("process {pid} works in it")),
        Use::Unknown(pid) => Some(format!("process {pid} can't be read")),
        Use::Occupied(_) if planned_occupied => None,
        Use::Occupied(pid) => Some(format!("process {pid} sits in its tree")),
    }
}

/// Reclaims a cargo target under `root`, which `root_path` names. It's
/// judged, then cargo's locks are taken, and everything but the
/// benchmarks is moved aside into a trash inside it. With the locks still
/// held, a build waiting on one, or one started since the judgement, puts
/// everything back. Then the locks go and the trash is emptied. When the
/// disk is too full even to make the trash, the target is deleted in place
/// under the locks. A build that opens the lock between the move and the
/// check finds an empty target and rebuilds it.
pub fn target(
    root: &Dir,
    root_path: &Path,
    candidate: &Candidate,
    checkout: Option<&Path>,
    looker: &mut impl Looker,
) -> Outcome {
    let Ok(relative) = candidate.path.strip_prefix(root_path) else {
        return Outcome::Failed(format!("outside its root {}", root_path.display()));
    };
    let judged = |looker: &mut _| match Looker::judge(looker, &candidate.path, checkout) {
        Ok(used) => refusal(used, candidate.occupied),
        Err(error) => Some(format!("its use can't be judged: {error}")),
    };
    if let Some(reason) = judged(looker) {
        return Outcome::Kept(reason);
    }
    let dir = match root.child(relative) {
        Ok(dir) => dir,
        Err(error) => return Outcome::Failed(format!("opening it: {error}")),
    };
    let held = match hold_cargo(&dir) {
        Ok(Hold::Held(held)) => held,
        Ok(Hold::Busy(lock)) => return Outcome::Kept(format!("cargo holds {}", lock.display())),
        Err(error) => return Outcome::Failed(format!("taking its locks: {error}")),
    };
    let detached = match dir.detach(&KEEP) {
        Ok(detached) => detached,
        Err(error) if error.raw_os_error() == Some(libc::ENOSPC) => {
            let removed = remove_in_place(&dir, &KEEP);
            drop(held);
            return match removed {
                Ok(()) => Outcome::Reclaimed,
                Err(error) => Outcome::Failed(format!("deleting it in place: {error}")),
            };
        }
        Err(error) => return Outcome::Failed(format!("moving it aside: {error}")),
    };
    let reason = match looker.waiters(&held.inodes()) {
        Err(error) => Some(format!("its lock's waiters can't be read: {error}")),
        Ok(waiters) if !waiters.is_empty() => Some("a build waits on its lock".to_owned()),
        Ok(_) => judged(looker),
    };
    if let Some(reason) = reason {
        return match detached.restore() {
            Ok(()) => Outcome::Kept(reason),
            Err(error) => Outcome::Failed(format!("{reason}, and putting it back failed: {error}")),
        };
    }
    drop(held);
    let trash = detached.name().to_owned();
    match detached.empty() {
        Ok(()) => Outcome::Reclaimed,
        Err(error) => Outcome::Failed(format!("emptying {}: {error}", trash.to_string_lossy())),
    }
}

/// Deletes a target's contents where they lie, all but `keep`, for a disk
/// too full to make a trash in. Every profile's fingerprints go first, so
/// a removal that's stopped part way, the guard killed or its machine
/// losing power, leaves cargo rebuilding what's left rather than trusting
/// it. Profiles sit one level down, or two under a target triple.
pub fn remove_in_place(dir: &Dir, keep: &[&str]) -> io::Result<()> {
    let fingerprints = OsStr::new(".fingerprint");
    let mut failed = None;
    let mut note = |result: io::Result<()>| {
        if let Err(error) = result {
            failed.get_or_insert(error);
        }
    };
    let names = dir.names()?;
    let kept = |name: &[u8]| keep.iter().any(|kept| kept.as_bytes() == name);
    for name in names.iter().filter(|name| !kept(name)) {
        let Ok(Some(profile)) = dir.entry(OsStr::from_bytes(name)) else {
            continue;
        };
        note(profile.remove_entry(fingerprints));
        for inner in profile.names().unwrap_or_default() {
            if let Ok(Some(nested)) = profile.entry(OsStr::from_bytes(&inner)) {
                note(nested.remove_entry(fingerprints));
            }
        }
    }
    for name in names.iter().filter(|name| !kept(name)) {
        note(dir.remove_entry(OsStr::from_bytes(name)));
    }
    failed.map_or(Ok(()), Err)
}
