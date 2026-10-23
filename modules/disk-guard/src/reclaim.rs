//! Reclaiming one candidate, under the rules no source can loosen.

use crate::activity::Use;
use crate::anchor::Dir;
use crate::locks::{Hold, hold_cargo, hold_session};
use crate::pass::Outcome;
use crate::plan::Candidate;
use std::ffi::OsStr;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::SystemTime;

/// What the reclaimer asks of the machine at each moment.
pub trait Looker {
    /// How the target is used right now.
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> io::Result<Use>;
    /// Something was just moved aside, so the judgement after it must look
    /// afresh rather than at what was seen before.
    fn moved(&mut self);
    /// Who waits on the locks the guard holds on these inodes.
    fn waiters(&mut self, inodes: &[u64]) -> io::Result<Vec<i32>>;
}

/// What survives a target's removal: benchmark baselines, which nothing
/// regenerates.
pub(crate) const KEEP: [&str; 1] = ["criterion"];

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
    looker.moved();
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

/// Reclaims one crate's incremental directory. Renaming it aside, as a
/// target is, would break a rustc starting in it, since rustc works by
/// path. So every session's lock is held first, and any session rustc
/// holds, is still `-working` or is too young to collect keeps the lot.
/// Then each session goes and then its lock, the order rustc's own
/// collection uses, and the directory goes only if nothing is left in it,
/// so a session rustc started meanwhile survives.
/// Reclaims an older build of a crate inside `target`: its libraries and
/// dep-info in `deps`, and the fingerprint beside them, so cargo builds it
/// afresh if it's ever wanted again. Cargo's locks on the target are held
/// throughout, so no build reads or writes it meanwhile; one holding them
/// keeps it.
pub fn build(root: &Dir, root_path: &Path, target: &Path, candidate: &Candidate) -> Outcome {
    let (Ok(target_relative), Ok(relative)) = (
        target.strip_prefix(root_path),
        candidate.path.strip_prefix(target),
    ) else {
        return Outcome::Failed(format!("outside its root {}", root_path.display()));
    };
    let (Some(deps), Some(profile), Some(stem)) = (
        relative.parent(),
        relative.parent().and_then(Path::parent),
        relative.file_name().and_then(|stem| stem.to_str()),
    ) else {
        return Outcome::Failed("not a build's name".to_owned());
    };
    let Some((name, digest)) = stem.rsplit_once('-') else {
        return Outcome::Failed("not a build's name".to_owned());
    };
    let target_dir = match root.child(target_relative) {
        Ok(dir) => dir,
        Err(error) => return Outcome::Failed(format!("opening its target: {error}")),
    };
    let held = match hold_cargo(&target_dir) {
        Ok(Hold::Held(held)) => held,
        Ok(Hold::Busy(lock)) => return Outcome::Kept(format!("cargo holds {}", lock.display())),
        Err(error) => return Outcome::Failed(format!("taking its locks: {error}")),
    };
    let ours = |file: &[u8]| {
        let file = String::from_utf8_lossy(file);
        file == format!("{name}-{digest}.d")
            || ["rlib", "rmeta", "so", "a", "dylib"]
                .iter()
                .any(|suffix| file == format!("lib{name}-{digest}.{suffix}"))
    };
    let removed = target_dir.child(deps).and_then(|deps| {
        for file in deps.names()?.iter().filter(|file| ours(file)) {
            deps.remove_entry(OsStr::from_bytes(file))?;
        }
        Ok(())
    });
    if let Err(error) = removed {
        return Outcome::Failed(format!("removing its artifacts: {error}"));
    }
    // Its package's name may differ from the crate's; the digest doesn't.
    let suffix = format!("-{digest}");
    let fingerprints = target_dir.child(&profile.join(".fingerprint"));
    if let Ok(fingerprints) = fingerprints {
        let names = fingerprints.names().unwrap_or_default();
        for fingerprint in names
            .iter()
            .filter(|name| name.ends_with(suffix.as_bytes()))
        {
            let fingerprint = OsStr::from_bytes(fingerprint);
            let emptied = match fingerprints.entry(fingerprint) {
                Ok(Some(dir)) => remove_in_place(&dir, &[]),
                Ok(None) => Ok(()),
                Err(error) => Err(error),
            };
            if let Err(error) = emptied.and_then(|()| fingerprints.remove_if_empty(fingerprint)) {
                return Outcome::Failed(format!("removing its fingerprint: {error}"));
            }
        }
    }
    drop(held);
    Outcome::Reclaimed
}

pub fn variant(root: &Dir, root_path: &Path, candidate: &Candidate, now: SystemTime) -> Outcome {
    let Ok(relative) = candidate.path.strip_prefix(root_path) else {
        return Outcome::Failed(format!("outside its root {}", root_path.display()));
    };
    let (Some(parent), Some(name)) = (relative.parent(), relative.file_name()) else {
        return Outcome::Failed("not a variant's directory".to_owned());
    };
    let opened = root.child(parent).and_then(|parent| {
        let found = parent.entry(name)?;
        Ok((found.ok_or(io::ErrorKind::NotFound)?, parent))
    });
    let (dir, parent) = match opened {
        Ok(opened) => opened,
        Err(error) => return Outcome::Failed(format!("opening it: {error}")),
    };
    let names = match dir.names() {
        Ok(names) => names,
        Err(error) => return Outcome::Failed(format!("reading it: {error}")),
    };
    let sessions: Vec<&[u8]> = names
        .iter()
        .map(Vec::as_slice)
        .filter(|name| name.starts_with(b"s-") && !name.ends_with(b".lock"))
        .collect();
    if sessions.iter().any(|name| name.ends_with(b"-working")) {
        return Outcome::Kept("rustc is compiling it".to_owned());
    }
    let mut held = Vec::new();
    for session in &sessions {
        match hold_session(&dir, OsStr::from_bytes(session), now) {
            Ok(Hold::Held(locked)) => held.push(locked),
            Ok(Hold::Busy(what)) => {
                return Outcome::Kept(format!("rustc holds {}", what.display()));
            }
            Err(error) => return Outcome::Failed(format!("taking its locks: {error}")),
        }
    }
    for session in &sessions {
        let lock = session
            .rsplitn(2, |&byte| byte == b'-')
            .nth(1)
            .unwrap_or(session);
        let lock = [lock, b".lock"].concat();
        let removed = dir.remove_entry(OsStr::from_bytes(session));
        if let Err(error) = removed.and_then(|()| dir.remove_entry(OsStr::from_bytes(&lock))) {
            return Outcome::Failed(format!("removing a session: {error}"));
        }
    }
    drop(held);
    match parent.remove_if_empty(name) {
        Ok(_) => Outcome::Reclaimed,
        Err(error) => Outcome::Failed(format!("removing its directory: {error}")),
    }
}

/// Reclaims one thing in a trash that `trash` opens and `trash_path`
/// names: the thing under `files/`, then its `.trashinfo`, so an
/// interrupted removal leaves an info file that's tidied next time rather
/// than a file nothing dates.
pub fn trashed(trash: &Dir, trash_path: &Path, candidate: &Candidate) -> Outcome {
    let relative = candidate.path.strip_prefix(trash_path.join("files"));
    let name = match relative.map(|relative| (relative.parent(), relative.file_name())) {
        Ok((Some(parent), Some(name))) if parent.as_os_str().is_empty() => name,
        _ => return Outcome::Failed("not in the trash's files".to_owned()),
    };
    let mut info = name.to_owned();
    info.push(".trashinfo");
    let removed = trash
        .child(Path::new("files"))
        .and_then(|files| files.remove_entry(name))
        .and_then(|()| match trash.child(Path::new("info")) {
            Ok(dir) => dir.remove_entry(&info),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        });
    match removed {
        Ok(()) => Outcome::Reclaimed,
        Err(error) => Outcome::Failed(format!("removing it: {error}")),
    }
}

/// Reclaims one entry under `root`, which `root_path` names: a scratch
/// run, a temp leftover, a workspace or a cache. It's judged first, and a
/// file or symlink is then unlinked, never followed. A directory's
/// contents are moved aside into a trash inside it and it's judged again,
/// so a process that arrived meanwhile gets everything back; then the
/// trash is emptied and the directory removed if nothing new is in it.
pub fn entry(
    root: &Dir,
    root_path: &Path,
    candidate: &Candidate,
    looker: &mut impl Looker,
) -> Outcome {
    let Ok(relative) = candidate.path.strip_prefix(root_path) else {
        return Outcome::Failed(format!("outside its root {}", root_path.display()));
    };
    let (Some(parent), Some(name)) = (relative.parent(), relative.file_name()) else {
        return Outcome::Failed("its root itself".to_owned());
    };
    let checkout = candidate.checkout.as_deref();
    let judged = |looker: &mut _| match Looker::judge(looker, &candidate.path, checkout) {
        Ok(used) => refusal(used, candidate.occupied),
        Err(error) => Some(format!("its use can't be judged: {error}")),
    };
    if let Some(reason) = judged(looker) {
        return Outcome::Kept(reason);
    }
    let parent = match root.child(parent) {
        Ok(parent) => parent,
        Err(error) => return Outcome::Failed(format!("opening where it lies: {error}")),
    };
    let failed = |doing: &str, error: io::Error| Outcome::Failed(format!("{doing}: {error}"));
    match parent.kind(name) {
        None => return Outcome::Reclaimed,
        Some(rustix::fs::FileType::Directory) => {}
        Some(_) => {
            return match parent.remove_entry(name) {
                Ok(()) => Outcome::Reclaimed,
                Err(error) => failed("removing it", error),
            };
        }
    }
    let dir = match parent.entry(name) {
        Ok(Some(dir)) => dir,
        Ok(None) => return Outcome::Failed("on another filesystem".to_owned()),
        Err(error) => return failed("opening it", error),
    };
    let detached = match dir.detach(&[]) {
        Ok(detached) => detached,
        Err(error) if error.raw_os_error() == Some(libc::ENOSPC) => {
            // Too full to make the trash: it was judged free just now.
            drop(dir);
            return match parent.remove_entry(name) {
                Ok(()) => Outcome::Reclaimed,
                Err(error) => failed("removing it in place", error),
            };
        }
        Err(error) => return failed("moving it aside", error),
    };
    looker.moved();
    if let Some(reason) = judged(looker) {
        return match detached.restore() {
            Ok(()) => Outcome::Kept(reason),
            Err(error) => Outcome::Failed(format!("{reason}, and putting it back failed: {error}")),
        };
    }
    if let Err(error) = detached.empty() {
        return failed("emptying it", error);
    }
    drop(dir);
    match parent.remove_if_empty(name) {
        Ok(_) => Outcome::Reclaimed,
        Err(error) => failed("removing it", error),
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
