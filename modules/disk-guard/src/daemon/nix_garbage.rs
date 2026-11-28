//! Nix's garbage, generations and store deduplication as the guard runs
//! them: when a listing is due, how deletes rest, and the store's floor.

use super::hearing::spoken_for;
use super::{GARBAGE, Guard, held_off};
use crate::band::{Band, Thresholds};
use crate::clock::Tick;
use crate::decide::Decision;
use crate::dedupe::Deduped;
use crate::garbage::{BATCH_BYTES, Batch, GC_TIMEOUT, Listing};
use crate::generations::Boot;
use crate::host::Deletes;
use crate::journal::Line;
use crate::pass::Outcome;
use crate::plan::{Candidate, Class};
use crate::pool::pool;
use crate::survey::{Action, Offer, named};
use crate::{dedupe, garbage, generations, state};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, mpsc};
use std::{fs, io};

/// How often Nix's own `min-free` is read again, since a person changes
/// their `nix.conf` seldom and asking costs a process.
pub(super) const MIN_FREE_EVERY: f64 = 3600.0;

/// How soon a `min-free` that couldn't be read is asked again, the first
/// time, and every time while no read has answered; each failure after
/// doubles it, up to `MIN_FREE_EVERY`, while a floor read before is kept.
pub(super) const MIN_FREE_AGAIN: f64 = 300.0;

/// How soon `min-free` is asked again after `failures` reads in a row
/// failed: five minutes, then ten, twenty and forty, then hourly. Each
/// read can hold the guard's step for its 30 s timeout, so a Nix that
/// stays slow is asked less and less rather than every five minutes.
pub(super) fn min_free_again(failures: u32) -> f64 {
    let doublings = failures.saturating_sub(1).min(4);
    (MIN_FREE_AGAIN * f64::from(1_u32 << doublings)).min(MIN_FREE_EVERY)
}

/// What linking the store's identical files is journaled and run as.
pub(super) const DEDUPE: &str = "dedupe";

/// How long after a failed run linking the store's files the next waits.
pub(super) const DEDUPE_RETRY: f64 = 86400.0;

/// The candidates a profile's generations are offered as.
pub(super) const GENERATIONS: &str = "generations";

pub(super) const YOUNG_GENERATIONS: &str = "generations:young";

/// How old Nix's garbage listing may grow on a healthy disk before a spare
/// moment lists it again, as often as upkeep comes, and how old on a
/// filling one. Then how long a listing that failed waits for the next, on a
/// healthy disk and a filling one, where a listing is all garbage can be
/// collected by.
pub(super) const LISTING_FRESH: f64 = 6.0 * 3600.0;

pub(super) const LISTING_PRESSED: f64 = 3600.0;

pub(super) const LISTING_RETRY: f64 = 3600.0;

pub(super) const LISTING_RETRY_PRESSED: f64 = 300.0;

/// How soon after the last listing began one comes again on a filling disk
/// once a root has left and the last listing has rested, so a build's
/// result deleted is garbage gone within a minute of that; and once a
/// process holding paths has ended, which happens every few seconds on a
/// busy machine, so the listing's minute of the disk's time is spent at
/// most every two. On a healthy disk neither brings a listing:
/// on 7 November a listing of 101,000 paths took 13 minutes of a core over
/// 15 and freed 190 MB with 313 GB free, so what they let go waits for the
/// next six-hourly one. What a process held in the listing kept goes once it
/// ends, either way, since that needs no listing.
pub(super) const ROOT_LEFT_GAP: f64 = 30.0;

pub(super) const HELD_ENDED_GAP: f64 = 120.0;

/// How long the garbage's own passes rest after one that left every path
/// it was offered, doubling from the first to the longest until one takes
/// some. From 21:59 to 22:07 on 16 November a machine's guard ran 16 such
/// passes, one each time a process holding the store ended, each deleting
/// the same 14,448 bytes that an evaluation registered again and freeing
/// nothing, for 9 to 49 s of waiting on the disk's commits apiece. What's
/// left is judged by the paths still there, not by the room measured,
/// which other writers move. Garbage that pass wasn't offered still goes at
/// once, as a build's result deleted on a filling disk, and a pass a band
/// brings takes the garbage too.
pub(super) const COLLECT_FIRST_REST: f64 = 120.0;

pub(super) const COLLECT_LONGEST_REST: f64 = 1800.0;

/// How many times its own length the guard rests after a pass's delete
/// before another short of nearly full, so deleting takes at most a fifth
/// of the store's time. Nix ends every delete reading each entry of its
/// links folder, whatever it deleted: on 21 November a machine's guard
/// made 97 deletes, one pass's three finishing five to eight minutes
/// apart, and one freeing 408 bytes nine minutes after the one before.
/// Past nearly full the batches go back to back, room coming first.
pub(super) const DELETE_REST: f64 = 4.0;

/// How many times its own length a listing rests after it ends before a
/// relist brings another on a healthy disk, so listing takes at most a
/// tenth of the guard's time, kept with the listing so a restart rests
/// too. A store of 400 GiB took 108 s and 75 s of CPU to list on 31
/// October, and roots left so often that one began as each ended, a core
/// spent to free kilobytes. A filling disk rests the same short of
/// nearly full: on 8 November one at 75% listed back to back, five to six
/// minutes of a core each with the machine's processors 45 to 55% stalled,
/// and each pass found nothing more to clear. Nearly full and past, none.
/// A listing that failed rests the same before it's tried again, nearly
/// full too: on 9 November one failing on Nix's own temporary paths, five
/// minutes of a core each time, was tried again six times in two hours.
pub(super) const LISTING_REST: f64 = 9.0;

/// The longest a filling store's listing that's due waits for the machine
/// to have room before it begins at idle priority anyway, so a machine
/// whose disk or memory never quiets still lists a filling store before
/// it's nearly full.
pub(super) const ROOM_AWAITED_LONGEST: f64 = 6.0 * 3600.0;

/// The most paths deleted in one go: no count, a batch closing only at its
/// size. Each delete costs Nix a whole look for roots and a read of every
/// entry in its links folder, five to eight minutes apiece on 21 November
/// whatever it deleted, so batches of 2,000 small paths paid it again and
/// again while a backlog drained. The paths go on Nix's standard input,
/// which no command line's length limits.
pub(super) const MOST_PER_BATCH: usize = usize::MAX;

/// Whether a store path was written since `listed_at`, and so isn't the
/// path the listing dated. The listing keeps what was deleted, so on 15
/// November a derivation deleted as two hours old, written again by the
/// next evaluation, went again within a minute of that evaluation ending,
/// by its first date: the guard broke Nix's evaluate-then-build. Its ctime
/// is when it landed, as Nix stamps every path's mtime with the epoch.
pub(super) fn written_since(meta: &fs::Metadata, listed_at: f64) -> bool {
    use std::os::unix::fs::MetadataExt;
    crate::units::wall(meta.ctime(), meta.ctime_nsec()) > listed_at
}

impl Guard {
    /// Takes a listing of Nix's garbage that has finished, keeping it for
    /// the next guard, and makes upkeep due when routine would take some.
    pub(super) fn heard_garbage(&mut self, now: f64) {
        let Some((begun, out)) = &self.listing_out else {
            return;
        };
        let heard = match out.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(heard) => heard,
            Err(TryRecvError::Disconnected) => Err("the listing stopped".to_owned()),
        };
        let begun = *begun;
        self.listing_out = None;
        // Heard only at the step after it ended, a listing seems to take as
        // long as any pass between, so it counts no longer than a listing
        // runs: a quick one heard after a three-hour pass would rest a
        // filling store's next listing for a day, through restarts too.
        let took = (now - begun).clamp(0.0, garbage::LIST_TIMEOUT.as_secs_f64());
        self.listing_ended = Some((now, took));
        self.listing_failed = heard.is_err();
        match heard {
            Ok(dead) => {
                let listing = Listing {
                    listed_at: begun,
                    took: Some(took),
                    dead,
                };
                // Kept for this run though the disk can't hold it: thrown
                // away, it would be listed again five minutes on.
                if let Err(error) = state::save(&self.listing_path, &listing) {
                    say!("disk-guard: saving the listing of Nix's garbage: {error}");
                }
                self.collect = true;
                self.stale_listing = false;
                if self.batches(&listing, now).iter().any(|batch| !batch.kept) {
                    self.upkeep_due();
                }
                self.listing = Some(listing);
            }
            // Tried again once it has rested, an hour on at most short of
            // pressure; until then the listing before stands.
            Err(why) => say!("disk-guard: listing Nix's garbage: {why}"),
        }
    }

    /// Has the listing's garbage go in a pass of its own, once something
    /// let some of it go and no other pass is due on the store's disk: a
    /// healthy disk keeps none of it, unless told to as a rebuild cache.
    pub(super) fn collect_garbage(&mut self, now: f64, due: &mut Vec<(u64, Decision, Thresholds)>) {
        if !self.collect {
            return;
        }
        self.collect = false;
        let Some((_, device)) = self.store() else {
            return;
        };
        if due.iter().any(|(on, _, _)| *on == device) {
            return;
        }
        let Some(watched) = self.watched.get(&device) else {
            return;
        };
        if (watched.band < Band::NearlyFull && self.deletes_resting(now))
            || held_off(self.writers_held, watched.band)
        {
            // Looked at again once the rest is over.
            self.collect = true;
            return;
        }
        // On a filling disk a user daemon speaking for the disk orders what
        // goes, its own caches against the store's garbage; healthy, the
        // garbage is the store's alone.
        if watched.band > Band::Healthy && spoken_for(self.hearing.as_ref(), device) {
            return;
        }
        let Some(space) = watched.space else {
            return;
        };
        let Some(listing) = &self.listing else {
            return;
        };
        let resting = self.collect_rest.is_some_and(|(until, _)| now < until);
        let offered = |batch: &crate::garbage::Batch| {
            !batch.kept
                && (!resting
                    || batch
                        .paths
                        .iter()
                        .any(|path| !self.left_by_collect.contains(path)))
        };
        if self.batches(listing, now).iter().any(offered) {
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            due.push((device, Decision::Collect, thresholds));
        } else if resting {
            // What let go meanwhile waits for the rest's end.
            self.collect = true;
        }
    }

    /// The paths the garbage's own pass is offered, as it begins.
    pub(super) fn collect_offers(&self, now: f64) -> Vec<String> {
        self.listing.as_ref().map_or_else(Vec::new, |listing| {
            self.batches(listing, now)
                .into_iter()
                .filter(|batch| !batch.kept)
                .flat_map(|batch| batch.paths)
                .collect()
        })
    }

    /// Whether Nix's deletes still rest after the last at `now`.
    pub(super) fn deletes_resting(&self, now: f64) -> bool {
        self.deletes_rest.is_some_and(|until| now < until)
    }

    /// Rests Nix's deletes [`DELETE_REST`] times as long as a pass's last
    /// took, from when the pass ended, and has the garbage looked at again
    /// once a pass kept some for that rest or for another writer, as the
    /// garbage's own rest allows, whatever kind of pass it was.
    pub(super) fn rest_deletes(&mut self, tick: &Tick, deletes: Deletes) {
        if let Some(took) = deletes.last {
            let ended = tick.wall + self.stepped.elapsed().as_secs_f64();
            self.deletes_rest = Some(ended + DELETE_REST * took);
        }
        self.collect |= deletes.again;
    }

    /// Rests the garbage's own passes once one left every path it was
    /// offered, and ends the rest once one takes some.
    pub(super) fn rest_collect(&mut self, now: f64, offered: &[String]) {
        let left: BTreeSet<String> = offered
            .iter()
            .filter(|path| Path::new(path).symlink_metadata().is_ok())
            .cloned()
            .collect();
        if left.len() < offered.len() {
            self.collect_rest = None;
            self.left_by_collect.clear();
            return;
        }
        let seconds = self.collect_rest.map_or(COLLECT_FIRST_REST, |(_, was)| {
            (was * 2.0).min(COLLECT_LONGEST_REST)
        });
        self.collect_rest = Some((now + seconds, seconds));
        self.left_by_collect = left;
    }

    /// Begins listing Nix's garbage on its own thread when it's due, once
    /// the last listing, kept or failed, has rested its own length nine
    /// times over short of nearly full: soon after a root leaves or a
    /// process holding paths ends on a filling disk, six hours on at a spare
    /// moment, or a day on whenever there's power, at idle priority; and
    /// when the store's filesystem is filling and the listing is an hour
    /// old. Short of nearly full, a listing begins only while the machine
    /// is `roomy`, or a filling store's once it has waited
    /// `ROOM_AWAITED_LONGEST` for room, at idle priority, and while it's
    /// overloaded only a filling store's does; one asked for meanwhile
    /// waits.
    pub(super) fn list_garbage(&mut self, now: f64, spare: bool, overloaded: bool, roomy: bool) {
        if self.listing_out.is_some() {
            return;
        }
        let Some((_, device)) = self.store() else {
            return;
        };
        let band = self
            .watched
            .get(&device)
            .map_or(Band::Healthy, |watched| watched.band);
        let pressed = band > Band::Healthy;
        if overloaded && !pressed || held_off(self.writers_held, band) {
            return;
        }
        let age = |at: Option<f64>| at.map_or(f64::INFINITY, |at| now - at);
        let listed = age(self.listing.as_ref().map(|listing| listing.listed_at));
        let retry = if pressed {
            LISTING_RETRY_PRESSED
        } else {
            LISTING_RETRY
        };
        let since = age(self.listing_begun);
        // A listing that failed rests at any band: on 24 November a nearly
        // full laptop's ran past its half hour three times running, each
        // holding Nix's lock for all of it while every build waited.
        let rested = band >= Band::NearlyFull && !self.listing_failed
            || self
                .listing_ended
                .is_none_or(|(ended, took)| now - ended >= LISTING_REST * took);
        let let_go =
            self.root_left && since >= ROOT_LEFT_GAP || self.held_ended && since >= HELD_ENDED_GAP;
        let due = rested
            && (self.relist
                || pressed && let_go
                || since >= retry
                    && if pressed {
                        listed >= LISTING_PRESSED
                    } else {
                        spare && listed >= LISTING_FRESH
                    });
        if !due {
            self.room_awaited = None;
            return;
        }
        // Short of nearly full a listing takes only what the rest leaves:
        // on 8 November one took half a processor at load 74, and on 23
        // November a filling store's ran 10 to 15 minutes at io full 17 to
        // 36 over the minute. Near full it can't wait on that, since idle
        // priority on a busy machine may never finish, and a filling
        // store's waits no longer than `ROOM_AWAITED_LONGEST`.
        let idle = band < Band::NearlyFull;
        let awaited = now - *self.room_awaited.get_or_insert(now);
        if idle && !roomy && !(pressed && awaited >= ROOM_AWAITED_LONGEST) {
            return;
        }
        self.room_awaited = None;
        let nix = self.config.nix.bin.clone();
        let store = self.config.nix.store.clone();
        let (tell, out) = mpsc::channel();
        let listing = std::thread::Builder::new()
            .name(GARBAGE.to_owned())
            .spawn(move || {
                // A guard that stopped meanwhile has no one to hear it.
                let _ = tell.send(garbage::list(&nix, &store, idle));
            });
        self.listing_begun = Some(now);
        self.relist = false;
        self.root_left = false;
        self.held_ended = false;
        match listing {
            Ok(_) => self.listing_out = Some((now, out)),
            Err(error) => say!("disk-guard: listing Nix's garbage: {error}"),
        }
    }

    /// Links the store's identical files on a thread, in a spare moment
    /// while it's healthy and a run is due, and stops a run once the store
    /// leaves healthy or the power stops being spare. Not once the disk
    /// stalls: its own reads stall it. Each run is journaled, and only one
    /// that finished counts as done.
    pub(super) fn dedupe(
        &mut self,
        tick: &Tick,
        spare_moment: bool,
        powered: bool,
    ) -> io::Result<()> {
        let Some(system) = &self.config.system else {
            return Ok(());
        };
        let store = self.config.nix.store.clone();
        let Some(watched) = pool(&store)
            .ok()
            .and_then(|device| self.watched.get(&device))
        else {
            return Ok(());
        };
        let (band, space) = (watched.band, watched.space);
        let in_hours = system
            .dedupe
            .hours
            .is_none_or(|hours| hours.hold_at(tick.wall));
        if let Some((begun, stop, out)) = &self.dedupe_out {
            if band > Band::Healthy || !powered || !in_hours {
                stop.store(true, Ordering::Relaxed);
            }
            let ended = match out.try_recv() {
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => Err("its thread ended unheard".to_owned()),
                Ok(ended) => ended,
            };
            let begun = *begun;
            self.dedupe_out = None;
            let (outcome, size) = match ended {
                Ok(Some(freed)) => {
                    self.memory.deduped = Some(Deduped { at: begun, freed });
                    self.saved_at = None;
                    (Outcome::Reclaimed, freed)
                }
                Ok(None) => (Outcome::Kept("stopped as the moment ended".to_owned()), 0),
                Err(why) => {
                    self.dedupe_failed = Some(tick.wall);
                    (Outcome::Failed(why), 0)
                }
            };
            return self.journal.write(&Line::Action {
                at: tick.wall,
                band,
                path: store,
                kind: DEDUPE.to_owned(),
                size,
                outcome,
            });
        }
        let failed_lately = self
            .dedupe_failed
            .is_some_and(|at| tick.wall - at < DEDUPE_RETRY);
        let Some(space) = space else {
            return Ok(());
        };
        let every = system.dedupe.every.0.as_secs_f64();
        let due = dedupe::due(self.memory.deduped.as_ref(), tick.wall, every, space.total);
        if band > Band::Healthy || !spare_moment || !in_hours || failed_lately || !due {
            return Ok(());
        }
        let (systemd_run, nix) = (
            system.dedupe.systemd_run.clone(),
            self.config.nix.bin.clone(),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let (tell, out) = mpsc::channel();
        let running = std::thread::Builder::new()
            .name(DEDUPE.to_owned())
            .spawn(move || {
                // A guard that stopped meanwhile has no one to hear it.
                let _ = tell.send(dedupe::optimise(systemd_run.as_deref(), &nix, &stopping));
            });
        match running {
            Ok(_) => self.dedupe_out = Some((tick.wall, stop, out)),
            Err(error) => say!("disk-guard: linking the store's files: {error}"),
        }
        Ok(())
    }

    /// Whether the store's runtime roots can be read, without which the
    /// store isn't touched (config.rs says why).
    pub(super) fn sees_processes(&self) -> bool {
        self.config.nix.runtime_roots
    }

    /// The garbage config and the store's device, for a system daemon
    /// that can see what running programs hold.
    pub(super) fn store(&self) -> Option<(&crate::config::Garbage, u64)> {
        if !self.sees_processes() {
            return None;
        }
        let garbage = &self.config.system.as_ref()?.garbage;
        Some((garbage, pool(&self.config.nix.store).ok()?))
    }

    /// The listing's garbage that is still there, in the batches and the
    /// order it goes in.
    pub(super) fn batches(&self, listing: &Listing, now: f64) -> Vec<Batch> {
        self.batches_and_links(listing, now).0
    }

    /// The listing's batches, with the files running programs hold known
    /// only by Nix's hard link to them, which only a walk of a batch's
    /// paths can place, so it's left for the delete of each.
    pub(super) fn batches_and_links(
        &self,
        listing: &Listing,
        now: f64,
    ) -> (Vec<Batch>, BTreeSet<crate::held::File>) {
        let Some((garbage, _)) = self.store() else {
            return Default::default();
        };
        let Ok(space) = crate::sys::space(&self.config.nix.store) else {
            return Default::default();
        };
        let keep = garbage.keep.0.as_secs_f64();
        let mut roots = crate::garbage::temp_roots(&self.config.nix.temp_roots);
        let running = crate::garbage::runtime_roots(&self.processes, &self.config.nix.store);
        roots.extend(running.paths);
        let held = listing.held_back(&roots);
        let batches = listing.batches(
            |dead| {
                !held.contains(dead.path.as_str())
                    && !self.refused.contains(&dead.path)
                    && Path::new(&dead.path)
                        .symlink_metadata()
                        .is_ok_and(|meta| !written_since(&meta, listing.listed_at))
            },
            now,
            keep,
            garbage.budget.of(space.total),
            MOST_PER_BATCH,
        );
        (batches, running.links)
    }

    /// What Nix's garbage offers: the listing's batches in the policy's
    /// order; or with no listing yet, once nearly full, hourly steps of
    /// `nix store gc`, which collects in no order of its own.
    pub(super) fn garbage_offers(&self, tick: &Tick) -> Vec<Offer> {
        let Some((_, device)) = self.store() else {
            // A user daemon no system daemon answers collects alone, as any
            // of Nix's clients may: the nix-daemon refuses only to ignore
            // liveness. Not where processes can't be read, either: the
            // guard can't then hold back what they use for Nix to check.
            let alone = self.config.system.is_none() && !self.system_heard && self.sees_processes();
            return match pool(&self.config.nix.store) {
                Ok(device) if alone => vec![self.gc_offer(tick, device)],
                _ => Vec::new(),
            };
        };
        let store = &self.config.nix.store;
        let candidate = |path, class, idle, size| Candidate {
            path,
            class,
            idle,
            size,
            occupied: false,
            spent: false,
            checkout: None,
        };
        let Some(listing) = &self.listing else {
            return vec![self.gc_offer(tick, device)];
        };
        if self.stale_listing {
            return vec![self.gc_offer(tick, device)];
        }
        let (batches, links) = self.batches_and_links(listing, tick.wall);
        batches
            .into_iter()
            .enumerate()
            .map(|(order, batch)| Offer {
                candidate: candidate(
                    named(store, &format!("{GARBAGE}-{order}")),
                    Class::Garbage {
                        kept: batch.kept,
                        order: u16::try_from(order).unwrap_or(u16::MAX),
                    },
                    (tick.wall - batch.newest).max(0.0),
                    batch.size,
                ),
                action: Action::Collect {
                    root: store.clone(),
                    nix: self.config.nix.bin.clone(),
                    roots: self.config.nix.temp_roots.clone(),
                    // Nix keeps its database beside its temp roots, in the
                    // state folder it was built with.
                    database: self.config.nix.temp_roots.with_file_name("db"),
                    paths: batch.paths,
                    links: links.clone(),
                },
                device,
            })
            .collect()
    }

    /// Reads Nix's own `min-free` once an hour. A read that fails keeps
    /// the last one that didn't and is asked again sooner, since the
    /// floor decides what a person is told: one slow `nix config show` on
    /// a loaded Mac would otherwise tell its disk fills for an hour. With
    /// a floor kept it backs off from five minutes on (`min_free_again`);
    /// with none, as after a switch restarts the guard under load, it's
    /// asked every five minutes until one answers. A run of failures is
    /// logged where it starts and where it ends, so the journal says
    /// whether the guard tells from an old floor, or none.
    pub(super) fn read_min_free(&mut self, now: f64) {
        if self
            .min_free
            .is_some_and(|(at, _, _)| (0.0..MIN_FREE_EVERY).contains(&(now - at)))
        {
            return;
        }
        let (kept, failures) = self
            .min_free
            .map_or((None, 0), |(_, kept, failures)| (kept, failures));
        self.min_free = Some(match garbage::min_free(&self.config.nix.bin) {
            Ok(read) => {
                if failures > 0 {
                    say!("disk-guard: read Nix's min-free again after {failures} failed");
                }
                (now, Some(read), 0)
            }
            Err(why) => {
                if failures == 0 {
                    match kept {
                        Some(floor) => say!(
                            "disk-guard: reading Nix's min-free: {why}; keeping the last read, \
                             {floor} bytes"
                        ),
                        None => say!(
                            "disk-guard: reading Nix's min-free: {why}; telling without Nix's \
                             floor until it answers"
                        ),
                    }
                }
                let failures = failures.saturating_add(1);
                let again = if kept.is_some() {
                    min_free_again(failures)
                } else {
                    MIN_FREE_AGAIN
                };
                (now - MIN_FREE_EVERY + again, kept, failures)
            }
        });
    }

    /// The filesystem Nix's store is on and the free space Nix keeps there,
    /// its own `min-free`, when Nix collects by itself.
    pub(super) fn nix_floor(&self) -> Option<(u64, u64)> {
        let min_free = self.min_free?.1.filter(|&min| min > 0)?;
        Some((pool(&self.config.nix.store).ok()?, min_free))
    }

    /// Whether the store's free space is short of `ABOVE_MIN_FREE` over
    /// Nix's own `min-free`, under which Nix soon collects by itself. A
    /// `min-free` of nothing is Nix never collecting by itself.
    pub(super) fn near_min_free(&self) -> bool {
        let Some((_, Some(min_free), _)) = self.min_free.filter(|&(_, min, _)| min > Some(0))
        else {
            return false;
        };
        crate::sys::space(&self.config.nix.store)
            .is_ok_and(|space| space.free < min_free.saturating_add(garbage::ABOVE_MIN_FREE))
    }

    /// Hourly steps of `nix store gc` once nearly full, which collects in
    /// no order of its own: for want of a listing, or of a system daemon.
    /// Near Nix's own `min-free` they come every few minutes, in any band a
    /// pass runs in, since an hour's step fell behind builds and Nix's own
    /// collections then held every build up.
    pub(super) fn gc_offer(&self, tick: &Tick, device: u64) -> Offer {
        let store = &self.config.nix.store;
        let near = self.near_min_free();
        let ran = self.memory.ran.get(GARBAGE).map_or(f64::INFINITY, |&at| {
            self.memory.awake.since(at.min(tick.wall), tick.wall)
        });
        let max = BATCH_BYTES.to_string();
        let nix = self.config.nix.bin.to_string_lossy();
        Offer {
            candidate: Candidate {
                path: named(store, GARBAGE),
                class: Class::Producer {
                    every: near.then_some(garbage::NEAR_MIN_FREE_EVERY.as_secs_f64()),
                    from: Some(if near {
                        Band::Healthy
                    } else {
                        Band::NearlyFull
                    }),
                },
                idle: ran,
                size: 0,
                occupied: false,
                spent: false,
                checkout: None,
            },
            action: Action::Run {
                root: store.clone(),
                source: GARBAGE.to_owned(),
                argv: std::iter::once(&*nix)
                    .chain(crate::garbage::NIX_COMMAND)
                    .chain(["store", "gc", "--max", &max])
                    .map(str::to_owned)
                    .collect(),
                timeout: GC_TIMEOUT,
            },
            device,
        }
    }

    /// A profile's generations that may go, each profile offering those
    /// routine takes and, apart, those only a filling disk takes, in garbage's
    /// group ahead of it and measured as commands, though what they free
    /// shows only as garbage. The system profile's are judged against
    /// the boot partition's entries; no other profile boots.
    pub(super) fn generation_offers(&self, tick: &Tick) -> Vec<Offer> {
        let nix = &self.config.nix;
        let Some(dir) = &nix.profiles else {
            return Vec::new();
        };
        let (Ok(device), Ok(profiles)) = (pool(&nix.store), generations::profiles(dir)) else {
            return Vec::new();
        };
        let in_use: Vec<PathBuf> = self
            .running
            .iter()
            .filter_map(|link| fs::read_link(link).ok())
            .collect();
        let mut offers = Vec::new();
        for profile in profiles {
            let boot = match &self.config.system {
                Some(system) if profile.path == dir.join("system") => {
                    match generations::entries(&system.boot) {
                        Ok(numbers) => Boot::Entries(numbers),
                        Err(_) => Boot::Unreadable,
                    }
                }
                _ => Boot::NotBooted,
            };
            let (routine, young) = generations::expired(
                &profile,
                &in_use,
                &boot,
                usize::try_from(nix.generations.keep).unwrap_or(usize::MAX),
                nix.generations.young.0.as_secs_f64(),
                tick.wall,
            );
            let Some(root) = profile.path.parent() else {
                continue;
            };
            for (numbers, kept, name) in [
                (routine, false, GENERATIONS),
                (young, true, YOUNG_GENERATIONS),
            ] {
                if numbers.is_empty() {
                    continue;
                }
                offers.push(Offer {
                    candidate: Candidate {
                        path: named(&profile.path, name),
                        class: Class::Generations { kept },
                        idle: 0.0,
                        size: 0,
                        occupied: false,
                        spent: false,
                        checkout: None,
                    },
                    action: Action::Expire {
                        root: root.to_owned(),
                        nix: nix.bin.clone(),
                        profile: profile.path.clone(),
                        numbers,
                    },
                    device,
                });
            }
        }
        offers
    }
}
