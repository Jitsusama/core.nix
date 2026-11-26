//! Watching the folders facts come from through inotify, and waiting for
//! those it can't watch yet: a folder missing, removed, refused for want
//! of room or by the system, and inotify itself refused at its start.

use std::ffi::c_int;
use std::io;
use std::path::{Path, PathBuf};

use crate::inotify::{
    IN_CREATE, IN_IGNORED, IN_MASK_ADD, IN_MOVED_TO, IN_Q_OVERFLOW, Inotify, Kernel,
};

/// What a watch on the folder above one missing asks: its being made.
const MADE: u32 = IN_CREATE | IN_MOVED_TO;

/// Why a folder can't be watched yet, when it may be later.
#[derive(Debug, PartialEq)]
enum Later {
    /// It isn't there, or what it's in isn't a folder, as /dev/snd isn't on
    /// a machine with no sound card till one is plugged in.
    Missing,
    /// The kernel, or this person, has no room for another watch, inotify
    /// or descriptor.
    Full,
    /// The system refuses it otherwise: its permissions, a loop of links or
    /// a failing disk, which a change to the folder or the disk may mend.
    Refused,
}

/// Whether a refusal may be mended by waiting, and why. Only a fault of the
/// program's can't be: a mask or a call the kernel won't take (EINVAL,
/// EBADF, EFAULT), which fails the hearing whole, as any other bug does.
/// Every other refusal is one folder's, so it costs no other's facts.
fn later(error: &io::Error) -> Option<Later> {
    match (error.raw_os_error(), error.kind()) {
        (Some(9 | 14 | 22), _) => None,
        // ENFILE and EMFILE: no room for another descriptor, or inotify.
        (Some(23 | 24), _) | (_, io::ErrorKind::StorageFull | io::ErrorKind::OutOfMemory) => {
            Some(Later::Full)
        }
        (_, io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => Some(Later::Missing),
        _ => Some(Later::Refused),
    }
}

/// Where a folder's watching stands.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Watched(c_int),
    /// Not watched yet, and while it's missing, watched for by this watch
    /// on the nearest folder above it that's there.
    Waiting(Option<c_int>),
}

/// A folder to watch, and what was last said of why it isn't, so a
/// refusal tried again is said once.
struct Folder {
    path: PathBuf,
    mask: u32,
    state: State,
    said: Option<String>,
}

/// Says a refusal in the journal unless it was the last said, keeping it.
fn say(said: &mut Option<String>, refusal: String, mendable: bool) {
    if said.as_ref() != Some(&refusal) {
        if mendable {
            eprintln!("<4>{refusal}, so it's tried again till it can be");
        } else {
            eprintln!("<3>{refusal}");
        }
        *said = Some(refusal);
    }
}

/// inotify over a set of folders, and those of them it can't watch yet.
///
/// A folder that's missing is watched for from the nearest folder above it
/// that's there, and one removed once watched is waited for again the same
/// way. Each is tried again whenever any is heard from, and, while one
/// waits with nothing above it watched, every few seconds (hearing's
/// `RETRY`), since nothing tells of room come free, or of a refusal lifted.
/// The watches above are heard only as wakeups, never as the folders' own
/// events.
pub(crate) struct Watches<K: Kernel = Inotify> {
    open: Box<dyn FnMut() -> io::Result<K>>,
    kernel: Option<K>,
    unopened: Option<String>,
    folders: Vec<Folder>,
    /// The watches held on folders above missing ones.
    above: Vec<c_int>,
}

impl Watches {
    /// Watches each of these folders through the kernel's inotify.
    pub(crate) fn new(watched: &[(PathBuf, u32)]) -> io::Result<Watches> {
        Watches::with(watched, Box::new(Inotify::open))
    }
}

impl<K: Kernel> Watches<K> {
    /// Watches each of these folders for what its mask names, through the
    /// inotify `open` starts, saying in the journal each it can't yet. A
    /// refusal that waiting can't mend is an error.
    pub(crate) fn with(
        asked: &[(PathBuf, u32)],
        open: Box<dyn FnMut() -> io::Result<K>>,
    ) -> io::Result<Watches<K>> {
        let folders = asked
            .iter()
            .map(|(path, mask)| Folder {
                path: path.clone(),
                mask: *mask,
                state: State::Waiting(None),
                said: None,
            })
            .collect();
        let mut watches = Watches {
            open,
            kernel: None,
            unopened: None,
            folders,
            above: Vec::new(),
        };
        match watches.try_again() {
            (_, Some(fault)) => Err(fault),
            (_, None) => Ok(watches),
        }
    }

    /// The descriptor poll waits on, or -1, which poll passes over, while
    /// inotify wouldn't start.
    pub(crate) fn fd(&self) -> c_int {
        self.kernel.as_ref().map_or(-1, Kernel::fd)
    }

    /// Whether a folder waits with nothing to wake it once it can be
    /// watched: refused for want of room or by the system, with nothing
    /// above it watched, or with inotify not started. One missing below a
    /// watch above is woken by that watch.
    pub(crate) fn waits(&self) -> bool {
        self.folders
            .iter()
            .any(|folder| folder.state == State::Waiting(None))
    }

    /// What the folders watched say, as inotify's events, leaving out those
    /// of the watches above folders not yet there; then tries again each
    /// folder not watched. Says too whether a folder came to be watched or
    /// was removed since, so its kind is read again for what it holds. A
    /// refusal waiting can't mend is an error, as it is at the start, and so
    /// is inotify failing a read, which ends at nothing more to read, so
    /// the hearing ends and is started afresh rather than leave a folder
    /// unheard.
    pub(crate) fn read(&mut self) -> io::Result<(Vec<u8>, bool)> {
        let mut said = Vec::new();
        if let Some(kernel) = &mut self.kernel {
            let mut buffer = [0; 8192];
            loop {
                match kernel.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => said.extend_from_slice(&buffer[..read]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => {
                        return Err(io::Error::new(
                            error.kind(),
                            format!("can't read inotify: {error}"),
                        ));
                    }
                }
            }
        }
        let (events, removed) = self.own_events(&said);
        match self.try_again() {
            (_, Some(fault)) => Err(fault),
            (watched, None) => Ok((events, removed || watched)),
        }
    }

    /// The events of the folders watched, and of none, as an overflow is,
    /// with any cut short kept whole; and whether a folder watched was
    /// removed, which waits for it again. An overflow may have lost a
    /// folder's removal, so each folder is watched afresh after one.
    fn own_events(&mut self, said: &[u8]) -> (Vec<u8>, bool) {
        let (mut own, mut removed, mut overflowed) = (Vec::new(), false, false);
        let mut rest = said;
        while !rest.is_empty() {
            let length = rest.get(12..16).map_or(usize::MAX, |bytes| {
                u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize
            });
            let Some((event, after)) = rest.split_at_checked(length.saturating_add(16)) else {
                own.extend_from_slice(rest);
                break;
            };
            let word = |at: usize| [event[at], event[at + 1], event[at + 2], event[at + 3]];
            let (wd, mask) = (c_int::from_ne_bytes(word(0)), u32::from_ne_bytes(word(4)));
            let folder = self
                .folders
                .iter_mut()
                .find(|folder| folder.state == State::Watched(wd));
            if mask & IN_IGNORED != 0 {
                if let Some(folder) = folder {
                    eprintln!("<4>{} was removed", folder.path.display());
                    folder.state = State::Waiting(None);
                    removed = true;
                }
                // A folder's own watch may be one above too. Each folder
                // waiting on it looks again for the nearest that's there;
                // the kernel dropped that watch, so it isn't let go of
                // again, by a number the kernel may since have given out.
                self.above.retain(|each| *each != wd);
                for folder in &mut self.folders {
                    if folder.state == State::Waiting(Some(wd)) {
                        folder.state = State::Waiting(None);
                    }
                }
            } else if folder.is_some() {
                own.extend_from_slice(event);
            } else if wd == -1 {
                overflowed |= mask & IN_Q_OVERFLOW != 0;
                own.extend_from_slice(event);
            }
            rest = after;
        }
        if overflowed {
            // A watch still there is given its own number back.
            for folder in &mut self.folders {
                folder.state = State::Waiting(None);
            }
        }
        (own, removed)
    }

    /// Starts inotify if it isn't, and watches each folder not yet watched
    /// that can be, letting go of each watch above that's no longer waited
    /// on. Says whether any came to be watched, and the first refusal that
    /// waiting can't mend; every refusal is said in the journal.
    fn try_again(&mut self) -> (bool, Option<io::Error>) {
        if self.kernel.is_none() {
            match (self.open)() {
                Ok(kernel) => {
                    if self.unopened.take().is_some() {
                        eprintln!("<6>inotify has started");
                    }
                    self.kernel = Some(kernel);
                }
                Err(error) => {
                    let mendable = later(&error).is_some();
                    let refusal = format!("can't start inotify: {error}");
                    say(&mut self.unopened, refusal, mendable);
                    return (false, (!mendable).then_some(error));
                }
            }
        }
        let Some(kernel) = self.kernel.as_mut() else {
            return (false, None);
        };
        let (mut watched, mut fault) = (false, None);
        for folder in &mut self.folders {
            if !matches!(folder.state, State::Waiting(_)) {
                continue;
            }
            match watch(kernel, folder, &mut self.above) {
                Ok(true) => {
                    if folder.said.take().is_some() {
                        eprintln!("<6>{} is watched now", folder.path.display());
                    }
                    watched = true;
                }
                Ok(false) => {}
                Err(error) => {
                    say(&mut folder.said, error.to_string(), false);
                    if fault.is_none() {
                        fault = Some(error);
                    }
                }
            }
        }
        let mut waited_on: Vec<c_int> = self
            .folders
            .iter()
            .filter_map(|folder| match folder.state {
                State::Waiting(above) => above,
                State::Watched(_) => None,
            })
            .collect();
        waited_on.sort_unstable();
        waited_on.dedup();
        self.above.sort_unstable();
        self.above.dedup();
        let mut unset = Vec::new();
        for wd in &self.above {
            if waited_on.contains(wd) {
                continue;
            }
            match self
                .folders
                .iter_mut()
                .find(|folder| folder.state == State::Watched(*wd))
            {
                // A folder's own watch, added to for one below it, is set
                // back to the folder's own mask, since what was added
                // can't be taken away alone. Refused, it hears what was
                // added till it's set back on a later try, which costs a
                // wakeup, not a fact, unless the refusal is the program's.
                Some(folder) => match kernel.add(&folder.path, folder.mask) {
                    Ok(again) => {
                        if folder.said.take().is_some() {
                            eprintln!("<6>{}'s watch is set back", folder.path.display());
                        }
                        folder.state = State::Watched(again);
                    }
                    Err(error) => {
                        let mendable = later(&error).is_some();
                        let refusal =
                            format!("can't set {}'s watch back: {error}", folder.path.display());
                        say(&mut folder.said, refusal, mendable);
                        if mendable {
                            unset.push(*wd);
                        } else if fault.is_none() {
                            fault = Some(error);
                        }
                    }
                },
                None => kernel.remove(*wd),
            }
        }
        waited_on.extend(unset);
        self.above = waited_on;
        (watched, fault)
    }
}

/// Watches a folder that waits, saying whether it now is. One missing is
/// watched for from the nearest folder above that's there, as far as that
/// has come since last time, and tried once more after, for one made in
/// between. Each watch above it makes is kept in `held`, to be let go of
/// once nothing waits on it. A refusal that waiting can't mend is an error
/// naming the folder.
fn watch(kernel: &mut impl Kernel, folder: &mut Folder, held: &mut Vec<c_int>) -> io::Result<bool> {
    let path = folder.path.clone();
    let refused = |error: io::Error| {
        io::Error::new(
            error.kind(),
            format!("can't watch {}: {error}", path.display()),
        )
    };
    let mut error = match kernel.add(&folder.path, folder.mask | IN_MASK_ADD) {
        Ok(wd) => {
            folder.state = State::Watched(wd);
            return Ok(true);
        }
        Err(error) => error,
    };
    let mut above = None;
    if later(&error) == Some(Later::Missing) {
        above = watch_above(kernel, &path).map_err(refused)?;
        held.extend(above);
        match kernel.add(&folder.path, folder.mask | IN_MASK_ADD) {
            Ok(wd) => {
                folder.state = State::Watched(wd);
                return Ok(true);
            }
            Err(again) => error = again,
        }
    }
    if later(&error).is_none() {
        return Err(refused(error));
    }
    // Only a folder still missing is woken by the watch above it.
    let missing = later(&error) == Some(Later::Missing);
    folder.state = State::Waiting(above.filter(|_| missing));
    let refusal = format!("can't watch {} yet: {error}", path.display());
    say(&mut folder.said, refusal, true);
    Ok(false)
}

/// Watches the nearest folder above a missing one that's there for what is
/// made or moved in, giving the watch, or none when it can't be watched
/// yet. Only a folder missing is climbed past: a watch further up than one
/// refused wouldn't hear what's made below it, so the folder is tried
/// again on the clock instead.
fn watch_above(kernel: &mut impl Kernel, missing: &Path) -> io::Result<Option<c_int>> {
    for above in missing.ancestors().skip(1) {
        match kernel.add(above, MADE | IN_MASK_ADD) {
            Ok(wd) => return Ok(Some(wd)),
            Err(error) if later(&error) == Some(Later::Missing) => {}
            Err(error) if later(&error).is_some() => return Ok(None),
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("watching {} for it: {error}", above.display()),
                ));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
