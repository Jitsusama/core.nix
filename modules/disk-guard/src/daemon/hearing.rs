//! Hearing opens: marking build folders for fanotify, taking what it
//! heard, and the roots and holds that say a path is in use.

use super::{Guard, Hearing, Holding};
use crate::clock::Tick;
use crate::config::Source;
use crate::discover;
use crate::disk_reads::Walk;
use crate::opens::Seen;
use crate::replaced::Uses;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How often the targets are walked for a profile and its `deps` to watch:
/// one built since goes unheard until then, and is dated from when it's
/// first watched.
pub(super) const MARK_EVERY: f64 = 3600.0;

/// How many failed reads of opens in a row end the listening, and how
/// long it waits after each: one that passes, as running short of memory
/// does, costs a few seconds' lost events rather than an hour floor for
/// every build until the guard restarts.
pub const LISTEN_TRIES: u32 = 5;

pub(super) const LISTEN_PAUSE: Duration = Duration::from_secs(1);

/// How long a user daemon's word holds a filesystem's passes for it.
pub(super) const SPOKEN_FOR: std::time::Duration = std::time::Duration::from_secs(60);

/// Whether a user daemon spoke for the filesystem within the minute, so it
/// drives that filesystem's passes.
pub(super) fn spoken_for(hearing: Option<&Hearing>, device: u64) -> bool {
    hearing.is_some_and(|hearing| {
        let spoken = hearing
            .spoken
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        spoken
            .get(&device)
            .is_some_and(|at| at.elapsed() < SPOKEN_FOR)
    })
}

impl Guard {
    /// Hears from `watch` who opens the artifacts of the targets it finds,
    /// on a thread of its own, so a busy build's thousands of opens never
    /// wait on a step. The thread ends with the guard, after
    /// [`LISTEN_TRIES`] failed reads in a row, or on a panic, after which
    /// the guard counts itself deaf.
    pub fn listen(&mut self, watch: Arc<crate::opens::Watch>) {
        let heard = Arc::clone(&watch);
        self.listen_to(watch, move |within| heard.wait(within));
    }

    /// [`Guard::listen`], hearing through `wait` what the watch marks: the
    /// way a test hands it a read that fails or panics.
    pub fn listen_to(
        &mut self,
        watch: Arc<crate::opens::Watch>,
        mut wait: impl FnMut(Duration) -> io::Result<Vec<Seen>> + Send + 'static,
    ) {
        /// Says the guard no longer hears however the thread ends,
        /// unwinding included, and forgets what it heard and watched, so
        /// neither this guard nor the next, reading the saved record,
        /// takes the deaf stretch for heard.
        struct Deafens(Arc<AtomicBool>, std::sync::Weak<Mutex<Uses>>);
        impl Drop for Deafens {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    say!("disk-guard: hearing opens panicked, so builds keep the hour");
                }
                self.0.store(false, Ordering::SeqCst);
                if let Some(uses) = self.1.upgrade() {
                    uses.lock().unwrap_or_else(PoisonError::into_inner).deaf();
                }
            }
        }
        let uses = Arc::downgrade(&self.uses);
        self.listening.store(true, Ordering::SeqCst);
        let deafens = Deafens(Arc::clone(&self.listening), Arc::downgrade(&self.uses));
        let spawned = std::thread::Builder::new()
            .name("opens".to_owned())
            .spawn(move || {
                let _deafens = deafens;
                let mut failed = 0;
                loop {
                    let seen = wait(Duration::from_secs(1));
                    let Some(uses) = uses.upgrade() else { return };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0.0, |since| since.as_secs_f64());
                    let seen = match seen {
                        Ok(seen) => {
                            failed = 0;
                            seen
                        }
                        // A failed read may have lost opens of anything
                        // marked, and may pass, as running short of memory
                        // does, so it counts as lost events and is tried
                        // again a moment later.
                        Err(error) => {
                            failed += 1;
                            uses.lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .lost(now);
                            if failed >= LISTEN_TRIES {
                                say!(
                                    "disk-guard: stopped hearing opens ({error}), \
                                     so builds keep the hour"
                                );
                                return;
                            }
                            say!("disk-guard: reading opens failed ({error}), trying again");
                            drop(uses);
                            std::thread::sleep(LISTEN_PAUSE);
                            continue;
                        }
                    };
                    if seen.is_empty() {
                        continue;
                    }
                    let mut uses = uses.lock().unwrap_or_else(PoisonError::into_inner);
                    for seen in seen {
                        match seen {
                            Seen::Opened(path) => uses.opened(&path, now),
                            Seen::Lost => uses.lost(now),
                        }
                    }
                }
            });
        if spawned.is_ok() {
            self.opens = Some(watch);
        } else {
            self.listening.store(false, Ordering::SeqCst);
        }
    }

    /// Marks every profile and its `deps` in the targets the cargo sources
    /// hold, at first and hourly after, so a build made since is heard too.
    /// A folder that was made anew loses its mark with the old one and gets
    /// one again, heard from then. Each is dated as its mark lands, since
    /// the walk can take minutes in which an unmarked folder hears nothing.
    /// A guard that no longer hears marks nothing, as nothing would come of
    /// it.
    pub(super) fn mark(&mut self, tick: &Tick) {
        let Some(watch) = self.opens.clone() else {
            return;
        };
        if !self.listening.load(Ordering::SeqCst) {
            return;
        }
        if self.marked_at.is_some_and(|at| tick.wall - at < MARK_EVERY) {
            return;
        }
        self.marked_at = Some(tick.wall);
        let walk = Walk::start();
        let mut marked = Vec::new();
        let mut unmarked = 0_usize;
        let mut why = None;
        for source in self.config.sources.values() {
            let Source::Cargo { roots, depth, .. } = source else {
                continue;
            };
            for root in roots {
                // A stop ends the walk, as it ends a survey's: a walk of
                // every cargo root under a loaded disk can take minutes.
                let found = discover::walk_until(
                    root,
                    depth.map(|depth| depth as usize),
                    self.home.as_deref(),
                    &crate::commands::stop_waiting,
                );
                for tree in found.unwrap_or_default() {
                    if tree.kind != discover::Kind::Target {
                        continue;
                    }
                    for dir in crate::replaced::watched_dirs(&tree.path) {
                        match watch.watch(&dir) {
                            Ok(handle) => {
                                let at = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .map_or(0.0, |since| since.as_secs_f64());
                                marked.push((dir, at, handle));
                            }
                            // One gone since the walk found it needs none.
                            Err(error) if dir.is_dir() => {
                                unmarked += 1;
                                why.get_or_insert(error);
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }
        // After the walk, so a folder forgotten as it marked is forgotten
        // below before what it marked again is dated.
        let forgot = watch.tidy();
        self.journal_walk(tick.wall, "marks", walk.end());
        if let Some(why) = why {
            say!(
                "disk-guard: {unmarked} build folders couldn't be marked ({why}, the first), \
                 so their builds keep the hour"
            );
        }
        // A mark goes with its folder. Which have gone is read before the
        // lock, since a stat on a loaded disk can take seconds, in which
        // the listener would wait and the kernel's queue could overflow.
        let held: Vec<PathBuf> = self
            .uses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .heard
            .keys()
            .cloned()
            .collect();
        let gone: Vec<PathBuf> = held.into_iter().filter(|dir| !dir.is_dir()).collect();
        let mut uses = self.uses.lock().unwrap_or_else(PoisonError::into_inner);
        // A listener that ended while the walk ran forgets what was heard,
        // and these marks must not bring it back. It says it stopped before
        // it takes this lock to forget, so either this sees that, or its
        // forgetting comes after these marks.
        if !self.listening.load(Ordering::SeqCst) {
            return;
        }
        for dir in gone.iter().chain(&forgot) {
            uses.heard.remove(dir);
        }
        // One marked in this walk and forgotten after, by the tidy or as
        // another id needed a group, has no mark to hear by, so it isn't
        // dated as heard: the next walk marks it again, and until then its
        // builds keep the hour. One made anew at a forgotten folder's path
        // keeps its own mark and is dated.
        for (dir, at, handle) in marked {
            if !watch.holds(&handle) {
                continue;
            }
            uses.watching(&dir, at);
            uses.marked(&dir, at, handle);
        }
    }

    /// Hears user daemons from now on.
    pub fn hear(&mut self, hearing: Hearing) {
        self.hearing = Some(hearing);
    }

    /// Notes what holds the store, and whether anything let go of it since
    /// the last look: a root that left is garbage made, and a process that
    /// ended may have let go of a build nothing else holds.
    pub(super) fn watch_roots(&mut self) {
        if self.store().is_none() {
            return;
        }
        let nix = &self.config.nix;
        let now = Holding::read(&nix.temp_roots, &nix.roots);
        if let Some(before) = &self.holding {
            if before.roots.iter().any(|root| !now.roots.contains(root)) {
                self.root_left = true;
            }
            if before
                .processes
                .iter()
                .any(|process| !now.processes.contains(process))
            {
                self.held_ended = true;
                self.collect = true;
            }
        }
        self.holding = Some(now);
    }

    /// The folders this guard hears now, as a survey would read them.
    pub fn hears(&self) -> Vec<PathBuf> {
        let uses = self.uses_now();
        uses.heard
            .keys()
            .filter(|dir| uses.heard_since(std::slice::from_ref(*dir)).is_some())
            .cloned()
            .collect()
    }

    /// When each build's artifacts were last opened: a copy, so a long
    /// survey never holds up what hears the opens. A listener that ended
    /// forgot what it heard, and a guard with none marks nothing, so the
    /// copy hears nothing then. A guard with none still reads what was
    /// watched, as `plan` reads the daemon's record.
    pub(super) fn uses_now(&self) -> Uses {
        self.uses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// When the last lost events were, as the listener counts them: the
    /// moment from which nothing marked is heard.
    pub fn lost_at(&self) -> f64 {
        self.uses_now().unheard
    }

    /// Whether a listing of Nix's garbage is out, begun and not yet heard.
    /// A listing counts as ending at the step that hears it, so this says
    /// whether that step has come.
    pub fn lists_garbage(&self) -> bool {
        self.listing_out.is_some()
    }

    /// Forgets what earlier guards watched, for a daemon that can't hear
    /// opens: their dates stop where it starts, so an older build would
    /// otherwise look unopened for every hour it ran deaf.
    pub fn hears_nothing(&mut self) {
        self.uses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .deaf();
    }
}
