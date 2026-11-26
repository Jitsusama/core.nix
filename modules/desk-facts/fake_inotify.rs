//! A stand-in inotify over folders kept in memory, so how folders are
//! waited for and heard is tried on any machine, refusing what the kernel
//! refuses and saying what it would say.

use std::cell::RefCell;
use std::ffi::c_int;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::inotify::{IN_CREATE, IN_IGNORED, IN_MASK_ADD, Kernel};
use crate::tests::event_on;
use crate::watches::Watches;

/// Every bit `inotify_add_watch` takes (the kernel's `ALL_INOTIFY_BITS`):
/// its events, `IN_UNMOUNT`, `IN_Q_OVERFLOW` and `IN_IGNORED`, and its flags.
const ALL_INOTIFY_BITS: u32 = 0xfff | 0xe000 | 0xf700_0000;

/// A bit no inotify mask may hold, as a mask the kernel won't take.
pub(crate) const NOT_INOTIFY: u32 = 0x1000;

/// What a stand-in inotify holds: the folders there, its watches, the
/// events it has to say, and how it refuses.
#[derive(Default)]
pub(crate) struct Held {
    pub(crate) made: Vec<PathBuf>,
    pub(crate) watches: Vec<(c_int, PathBuf, u32)>,
    last: c_int,
    pub(crate) events: Vec<u8>,
    /// How many watches there's room for, if not any.
    pub(crate) room: Option<usize>,
    /// What starting inotify is refused with.
    pub(crate) unopenable: Option<i32>,
    /// A folder made just as its watch is first refused, as one made
    /// between that and the watch on the folder above.
    pub(crate) racing: Option<PathBuf>,
    /// Watches let go of that it had already dropped, each of which the
    /// kernel refuses: a wasted call, and a number it might have given
    /// to another watch by then.
    pub(crate) dropped_let_go: Vec<c_int>,
}

/// A stand-in inotify, shared between the watches it's started for and the
/// test that makes and removes its folders.
#[derive(Clone, Default)]
pub(crate) struct Fake(pub(crate) Rc<RefCell<Held>>);

impl Fake {
    /// Makes a folder, heard by each watch on the one it's in that asks
    /// for what's made.
    pub(crate) fn make(&self, path: &str) {
        let path = PathBuf::from(path);
        let mut held = self.0.borrow_mut();
        let name = path
            .file_name()
            .expect("a folder made has a name")
            .to_string_lossy()
            .into_owned();
        let heard: Vec<u8> = held
            .watches
            .iter()
            .filter(|(_, watched, mask)| {
                Some(watched.as_path()) == path.parent() && mask & IN_CREATE != 0
            })
            .flat_map(|(wd, ..)| event_on(*wd, IN_CREATE, &name))
            .collect();
        held.events.extend(heard);
        held.made.push(path);
    }

    /// Removes a folder, whose watch the kernel drops, saying so.
    pub(crate) fn remove(&self, path: &str) {
        let mut held = self.0.borrow_mut();
        held.made.retain(|made| made != Path::new(path));
        if let Some(at) = held
            .watches
            .iter()
            .position(|(_, watched, _)| watched == Path::new(path))
        {
            let (wd, ..) = held.watches.remove(at);
            held.events.extend(event_on(wd, IN_IGNORED, ""));
        }
    }

    /// The folders watched and what each watch asks, sorted.
    pub(crate) fn watched(&self) -> Vec<(String, u32)> {
        let mut watched: Vec<(String, u32)> = self
            .0
            .borrow()
            .watches
            .iter()
            .map(|(_, path, mask)| (path.display().to_string(), *mask))
            .collect();
        watched.sort();
        watched
    }

    /// Watches these folders through this inotify, started each time it's
    /// asked unless it's made unopenable.
    pub(crate) fn watches(&self, folders: &[(&str, u32)]) -> io::Result<Watches<Fake>> {
        let folders: Vec<(PathBuf, u32)> = folders
            .iter()
            .map(|(path, mask)| (PathBuf::from(path), *mask))
            .collect();
        let fake = self.clone();
        let open = move || match fake.0.borrow().unopenable {
            Some(code) => Err(io::Error::from_raw_os_error(code)),
            None => Ok(fake.clone()),
        };
        Watches::with(&folders, Box::new(open))
    }
}

impl Kernel for Fake {
    fn add(&mut self, path: &Path, mask: u32) -> io::Result<c_int> {
        let mut held = self.0.borrow_mut();
        // EINVAL, as the kernel refuses a mask with a bit it doesn't take or
        // none it does.
        if mask & !ALL_INOTIFY_BITS != 0 || mask & ALL_INOTIFY_BITS == 0 {
            return Err(io::Error::from_raw_os_error(22));
        }
        if !held.made.iter().any(|made| made == path) {
            if held.racing.as_deref() == Some(path) {
                held.racing = None;
                held.made.push(path.to_owned());
            }
            return Err(io::Error::from_raw_os_error(2));
        }
        if let Some(watch) = held
            .watches
            .iter_mut()
            .find(|(_, watched, _)| watched == path)
        {
            // As the kernel does: added to only when asked to be.
            if mask & IN_MASK_ADD == 0 {
                watch.2 = 0;
            }
            watch.2 |= mask & !IN_MASK_ADD;
            return Ok(watch.0);
        }
        if held.room.is_some_and(|room| held.watches.len() >= room) {
            return Err(io::Error::from_raw_os_error(28));
        }
        held.last += 1;
        let wd = held.last;
        held.watches
            .push((wd, path.to_owned(), mask & !IN_MASK_ADD));
        Ok(wd)
    }

    fn remove(&mut self, wd: c_int) {
        let mut held = self.0.borrow_mut();
        if let Some(at) = held.watches.iter().position(|(each, ..)| *each == wd) {
            held.watches.remove(at);
            held.events.extend(event_on(wd, IN_IGNORED, ""));
        } else {
            held.dropped_let_go.push(wd);
        }
    }

    fn fd(&self) -> c_int {
        -1
    }

    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut held = self.0.borrow_mut();
        if held.events.is_empty() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let length = held.events.len().min(buffer.len());
        buffer[..length].copy_from_slice(&held.events[..length]);
        held.events.drain(..length);
        Ok(length)
    }
}
