//! Watching the folders facts come from through inotify, and waiting for
//! those it can't watch yet: a folder missing, removed, or refused for want
//! of room, and inotify itself refused at its start.

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
}

/// Whether a refusal may be mended by waiting, and why. Any other refusal
/// is a fault of the program's, such as a mask the kernel won't take.
fn later(error: &io::Error) -> Option<Later> {
    // ENFILE and EMFILE: no room for another descriptor, or inotify.
    if matches!(error.raw_os_error(), Some(23 | 24)) {
        return Some(Later::Full);
    }
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Some(Later::Missing),
        io::ErrorKind::StorageFull | io::ErrorKind::OutOfMemory => Some(Later::Full),
        _ => None,
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
/// `RETRY`), since nothing tells of room come free. The watches above are
/// heard only as wakeups, never as the folders' own events.
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
    /// watched: refused for want of room, with nothing above it watched, or
    /// with inotify not started. One missing below a watch above is woken
    /// by that watch.
    pub(crate) fn waits(&self) -> bool {
        self.folders
            .iter()
            .any(|folder| folder.state == State::Waiting(None))
    }

    /// What the folders watched say, as inotify's events, leaving out those
    /// of the watches above folders not yet there; then tries again each
    /// folder not watched. Says too whether a folder came to be watched or
    /// was removed since, so its kind is read again for what it holds. A
    /// refusal waiting can't mend is an error, as it is at the start, so
    /// the hearing ends and is started afresh rather than leave a folder
    /// unheard.
    pub(crate) fn read(&mut self) -> io::Result<(Vec<u8>, bool)> {
        let mut said = Vec::new();
        if let Some(kernel) = &mut self.kernel {
            let mut buffer = [0; 8192];
            while let Ok(read) = kernel.read(&mut buffer) {
                if read == 0 {
                    break;
                }
                said.extend_from_slice(&buffer[..read]);
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
            match folder {
                Some(folder) if mask & IN_IGNORED != 0 => {
                    eprintln!("<4>{} was removed", folder.path.display());
                    folder.state = State::Waiting(None);
                    removed = true;
                }
                Some(_) => own.extend_from_slice(event),
                None if wd == -1 => {
                    overflowed |= mask & IN_Q_OVERFLOW != 0;
                    own.extend_from_slice(event);
                }
                // The folder a watch above was on is gone, so each folder
                // waiting on it looks again for the nearest that's there;
                // the kernel dropped that watch, so it isn't let go of
                // again, by a number the kernel may since have given out.
                None if mask & IN_IGNORED != 0 => {
                    self.above.retain(|each| *each != wd);
                    for folder in &mut self.folders {
                        if folder.state == State::Waiting(Some(wd)) {
                            folder.state = State::Waiting(None);
                        }
                    }
                }
                None => {}
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
                // can't be taken away alone.
                Some(folder) => {
                    if let Ok(again) = kernel.add(&folder.path, folder.mask) {
                        folder.state = State::Watched(again);
                    }
                }
                None => kernel.remove(*wd),
            }
        }
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
/// made or moved in, giving the watch, or none when nothing above it can be
/// watched yet.
fn watch_above(kernel: &mut impl Kernel, missing: &Path) -> io::Result<Option<c_int>> {
    for above in missing.ancestors().skip(1) {
        match kernel.add(above, MADE | IN_MASK_ADD) {
            Ok(wd) => return Ok(Some(wd)),
            Err(error) if later(&error).is_some() => {}
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
mod tests {
    use super::*;
    use crate::inotify::{IN_CLOSE_WRITE, IN_DELETE, IN_Q_OVERFLOW, names};
    use crate::tests::*;

    use crate::fake_inotify::{Fake, NOT_INOTIFY};

    #[cfg(target_os = "linux")]
    use std::fs;

    /// What the folders say, where nothing refuses one for good.
    fn told<K: Kernel>(watches: &mut Watches<K>) -> (Vec<u8>, bool) {
        watches
            .read()
            .expect("a folder was refused for good where none should be")
    }

    #[test]
    fn only_a_missing_folder_or_no_room_waits() {
        let cases = [
            (2, Some(Later::Missing), "ENOENT"),
            (20, Some(Later::Missing), "ENOTDIR"),
            (28, Some(Later::Full), "ENOSPC"),
            (12, Some(Later::Full), "ENOMEM"),
            (23, Some(Later::Full), "ENFILE"),
            (24, Some(Later::Full), "EMFILE"),
            (22, None, "EINVAL"),
            (13, None, "EACCES"),
        ];
        for (code, expected, name) in cases {
            assert_eq!(
                later(&io::Error::from_raw_os_error(code)),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn a_folder_made_later_is_watched_and_the_watch_above_let_go_of() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
        let waiting = (fake.watched(), told(&mut watches), watches.waits());
        fake.make("/a/b");
        let made = (told(&mut watches), fake.watched(), watches.waits());
        // The watch above, let go of, says so; that isn't the folder's.
        let after = told(&mut watches);
        assert_eq!(
            (waiting, made, after),
            (
                (vec![("/a".into(), MADE)], (vec![], false), false),
                ((vec![], true), vec![("/a/b".into(), IN_CLOSE_WRITE)], false),
                (vec![], false),
            ),
            "a folder made later wasn't watched for from above, without waking the loop, \
             then watched with its own mask, the watch above let go of and unheard"
        );
    }

    #[test]
    fn what_is_made_beside_a_waiting_folder_isnt_heard_as_its_own() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
        fake.make("/a/other");
        assert_eq!(
            (told(&mut watches), watches.waits()),
            ((vec![], false), false),
            "a folder made beside one waited for was heard as the one's own"
        );
    }

    #[test]
    fn a_watch_above_two_waiting_folders_is_kept_till_both_are_watched() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake
            .watches(&[("/a/b", IN_CLOSE_WRITE), ("/a/c", IN_CLOSE_WRITE)])
            .unwrap();
        fake.make("/a/b");
        let one = (told(&mut watches).1, fake.watched());
        fake.make("/a/c");
        let both = (told(&mut watches).1, fake.watched());
        assert_eq!(
            (one, both),
            (
                (
                    true,
                    vec![("/a".into(), MADE), ("/a/b".into(), IN_CLOSE_WRITE)]
                ),
                (
                    true,
                    vec![
                        ("/a/b".into(), IN_CLOSE_WRITE),
                        ("/a/c".into(), IN_CLOSE_WRITE)
                    ]
                ),
            ),
            "the watch above two folders wasn't kept till both were watched"
        );
    }

    #[test]
    fn a_folder_watched_that_is_above_a_waiting_one_keeps_its_own_watch() {
        let fake = Fake::default();
        fake.make("/a");
        let own = IN_MOVED_TO | IN_DELETE;
        let mut watches = fake
            .watches(&[("/a", own), ("/a/b", IN_CLOSE_WRITE)])
            .unwrap();
        let waiting = fake.watched();
        fake.make("/a/b");
        // Its being made is heard on the folder's own watch, so is its own.
        let (events, heard) = told(&mut watches);
        assert_eq!(
            (
                waiting,
                names(&events, |name| name == b"b"),
                heard,
                fake.watched()
            ),
            (
                vec![("/a".into(), own | MADE)],
                true,
                true,
                vec![("/a".into(), own), ("/a/b".into(), IN_CLOSE_WRITE)],
            ),
            "watching for a folder from one watched took the one's own watch, or left it \
             asking for what's made"
        );
    }

    #[test]
    fn a_folder_missing_two_deep_is_watched_for_from_each_folder_made_toward_it() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a/b/c", IN_CLOSE_WRITE)]).unwrap();
        fake.make("/a/b");
        let between = (told(&mut watches).1, fake.watched());
        fake.make("/a/b/c");
        let made = (told(&mut watches).1, fake.watched());
        assert_eq!(
            (between, made),
            (
                (false, vec![("/a/b".into(), MADE)]),
                (true, vec![("/a/b/c".into(), IN_CLOSE_WRITE)]),
            ),
            "a folder two deep wasn't watched for from the folder made between"
        );
    }

    #[test]
    fn a_folder_made_as_the_watch_above_is_made_is_watched_at_once() {
        let fake = Fake::default();
        fake.make("/a");
        fake.0.borrow_mut().racing = Some("/a/b".into());
        let watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
        assert_eq!(
            (watches.waits(), fake.watched()),
            (false, vec![("/a/b".into(), IN_CLOSE_WRITE)]),
            "a folder made before its watch above was waited for regardless"
        );
    }

    #[test]
    fn a_folder_removed_once_watched_is_waited_for_again() {
        let fake = Fake::default();
        fake.make("/a");
        fake.make("/a/b");
        let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
        fake.remove("/a/b");
        let removed = (told(&mut watches), fake.watched());
        fake.make("/a/b");
        let made = (told(&mut watches).1, fake.watched());
        assert_eq!(
            (removed, made),
            (
                ((vec![], true), vec![("/a".into(), MADE)]),
                (true, vec![("/a/b".into(), IN_CLOSE_WRITE)]),
            ),
            "a folder removed wasn't heard gone, waited for and watched again"
        );
    }

    #[test]
    fn no_room_is_tried_again_till_there_is() {
        let fake = Fake::default();
        fake.make("/a");
        fake.0.borrow_mut().room = Some(0);
        let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
        let full = (told(&mut watches).1, watches.waits());
        fake.0.borrow_mut().room = None;
        let room = (told(&mut watches).1, watches.waits(), fake.watched());
        assert_eq!(
            (full, room),
            (
                (false, true),
                (true, false, vec![("/a".into(), IN_CLOSE_WRITE)])
            ),
            "a folder refused for want of room wasn't watched once there was"
        );
    }

    #[test]
    fn inotify_that_wouldnt_start_is_started_once_it_can() {
        let fake = Fake::default();
        fake.make("/a");
        fake.0.borrow_mut().unopenable = Some(24);
        let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
        let unstarted = (told(&mut watches).1, watches.waits(), fake.watched());
        fake.0.borrow_mut().unopenable = None;
        let started = (told(&mut watches).1, watches.waits(), fake.watched());
        assert_eq!(
            (unstarted, started),
            (
                (false, true, vec![]),
                (true, false, vec![("/a".into(), IN_CLOSE_WRITE)]),
            ),
            "inotify refused for want of room wasn't started, and the folders watched, once \
             it could be"
        );
    }

    #[test]
    fn a_refusal_waiting_cant_mend_fails_naming_what_was_refused() {
        let fake = Fake::default();
        fake.make("/a");
        let refused = |watches: io::Result<Watches<Fake>>| watches.err().map(|e| e.to_string());
        let mask = refused(fake.watches(&[("/a", IN_CLOSE_WRITE), ("/a", NOT_INOTIFY)]));
        fake.0.borrow_mut().unopenable = Some(22);
        let unstarted = refused(fake.watches(&[("/a", IN_CLOSE_WRITE)]));
        assert_eq!(
            (mask.as_deref(), unstarted.as_deref()),
            (
                Some("can't watch /a: Invalid argument (os error 22)"),
                Some("Invalid argument (os error 22)")
            ),
            "a refusal that waiting can't mend was let wait, or said without its folder"
        );
    }

    #[test]
    fn a_refusal_waiting_cant_mend_met_after_the_start_fails_the_read() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a/b", IN_CLOSE_WRITE)]).unwrap();
        fake.0.borrow_mut().refusing = Some(("/a/b".into(), 13));
        fake.make("/a/b");
        assert_eq!(
            watches.read().err().map(|e| e.to_string()).as_deref(),
            Some("can't watch /a/b: Permission denied (os error 13)"),
            "a refusal waiting can't mend, met after the start, was let wait with the folder \
             unheard, or said without its folder"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_kernel_s_inotify_watches_a_folder_once_it_s_made() {
        let sys = Sys::new("watch-later");
        let later = sys.0.join("later");
        assert!(
            Watches::new(&[(sys.0.clone(), NOT_INOTIFY)]).is_err(),
            "the kernel's refusal of a mask it doesn't take was let wait"
        );
        let mut watches = Watches::new(&[(later.clone(), IN_CLOSE_WRITE)]).unwrap();
        let before = told(&mut watches);
        fs::create_dir(&later).unwrap();
        let made = told(&mut watches);
        fs::write(later.join("file"), "x").unwrap();
        let (written, _) = told(&mut watches);
        fs::create_dir(sys.0.join("other")).unwrap();
        let unheard = told(&mut watches);
        assert_eq!(
            (
                before,
                made,
                names(&written, |name| name == b"file"),
                unheard
            ),
            ((vec![], false), (vec![], true), true, (vec![], false)),
            "a folder made later wasn't heard made, watched, then heard from, with the folder \
             above let go of"
        );
    }

    #[test]
    fn a_folder_made_as_its_watch_above_is_but_refused_for_room_waits_with_nothing_above() {
        let fake = Fake::default();
        fake.make("/a");
        fake.0.borrow_mut().racing = Some("/a/b".into());
        fake.0.borrow_mut().room = Some(1);
        let own = IN_MOVED_TO;
        let mut watches = fake
            .watches(&[("/a", own), ("/a/b", IN_CLOSE_WRITE)])
            .unwrap();
        let full = (watches.waits(), fake.watched());
        fake.0.borrow_mut().room = None;
        let room = (told(&mut watches).1, watches.waits(), fake.watched());
        assert_eq!(
            (full, room),
            (
                (true, vec![("/a".into(), own)]),
                (
                    true,
                    false,
                    vec![("/a".into(), own), ("/a/b".into(), IN_CLOSE_WRITE)]
                ),
            ),
            "a folder there but refused for want of room was left to the watch above, \
             which can't wake it, or that watch kept asking for what's made"
        );
    }

    #[test]
    fn a_folder_above_one_waiting_removed_is_looked_past_for_the_next_one_up() {
        let fake = Fake::default();
        fake.make("/a");
        fake.make("/a/b");
        let mut watches = fake.watches(&[("/a/b/c", IN_CLOSE_WRITE)]).unwrap();
        let first = fake.watched();
        fake.remove("/a/b");
        let removed = (told(&mut watches), watches.waits(), fake.watched());
        fake.make("/a/b");
        let again = (told(&mut watches).1, fake.watched());
        assert_eq!(
            (
                first,
                removed,
                again,
                fake.0.borrow().dropped_let_go.clone()
            ),
            (
                vec![("/a/b".into(), MADE)],
                ((vec![], false), false, vec![("/a".into(), MADE)]),
                (false, vec![("/a/b".into(), MADE)]),
                vec![],
            ),
            "a folder waiting on a watch above that was removed wasn't watched for from \
             the next folder up, and from the one made again, or the removed watch was \
             let go of after the kernel dropped it"
        );
    }

    #[test]
    fn an_overflow_watches_each_folder_afresh_for_a_removal_it_lost() {
        let fake = Fake::default();
        fake.make("/a");
        fake.make("/a/b");
        let mut watches = fake
            .watches(&[("/a", IN_CLOSE_WRITE), ("/a/b", IN_CLOSE_WRITE)])
            .unwrap();
        // /a/b removed and made again, its removal lost in an overflow.
        fake.remove("/a/b");
        fake.0.borrow_mut().events = event_on(-1, IN_Q_OVERFLOW, "");
        fake.make("/a/b");
        let (events, _) = told(&mut watches);
        fake.0.borrow_mut().events = event_on(3, IN_CLOSE_WRITE, "file");
        let (heard, _) = told(&mut watches);
        assert_eq!(
            (
                events,
                fake.watched(),
                names(&heard, |name| name == b"file")
            ),
            (
                event_on(-1, IN_Q_OVERFLOW, ""),
                vec![
                    ("/a".into(), IN_CLOSE_WRITE),
                    ("/a/b".into(), IN_CLOSE_WRITE)
                ],
                true,
            ),
            "a folder whose removal an overflow lost stayed on its dead watch"
        );
    }

    #[test]
    fn only_the_folders_own_events_and_an_overflow_are_kept() {
        let fake = Fake::default();
        fake.make("/a");
        let mut watches = fake.watches(&[("/a", IN_CLOSE_WRITE)]).unwrap();
        let mut said = event_on(1, IN_CLOSE_WRITE, "own");
        said.extend(event_on(7, IN_CLOSE_WRITE, "stray"));
        said.extend(event_on(-1, IN_Q_OVERFLOW, ""));
        let cut = event_on(1, IN_CLOSE_WRITE, "cut");
        said.extend(&cut[..cut.len() - 1]);
        let mut kept = event_on(1, IN_CLOSE_WRITE, "own");
        kept.extend(event_on(-1, IN_Q_OVERFLOW, ""));
        kept.extend(&cut[..cut.len() - 1]);
        assert_eq!(
            watches.own_events(&said),
            (kept, false),
            "a stray watch's event was kept, or an overflow or the rest cut short dropped"
        );
    }
}
