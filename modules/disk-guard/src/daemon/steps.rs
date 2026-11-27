//! The guard's making and its step: what each step reads, in what order
//! it asks the rest, and what it resumes after a sleep.

use super::hearing::spoken_for;
use super::{Explained, Guard, Places, Senses, held_off, loaded, predicates};
use crate::anchor::Dir;
use crate::band::{Band, Thresholds};
use crate::clock::Tick;
use crate::config::Config;
use crate::decide::{Decision, Situation, calm, decide, overloaded, powered, roomy, spare};
use crate::finished::Answers;
use crate::garbage::Listing;
use crate::journal::{Journal, Line};
use crate::pass::Outcome;
use crate::plan::Candidate;
use crate::pool::pool;
use crate::replaced::Uses;
use crate::socket::{Reply, Request};
use crate::state::Memory;
use crate::writers::Writers;
use crate::{journal, state};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;
use std::{fs, io};

/// How long a pass is assumed to take before one has been timed.
pub(super) const FIRST_PASS_SECONDS: f64 = 60.0;

/// How often what was learned is saved when no pass has run.
pub(super) const SAVE_EVERY: f64 = 300.0;

/// How long a user daemon waits for the system daemon to hear its
/// heartbeat.
pub(super) const HEARTBEAT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

impl Guard {
    pub fn new(config: Config, places: &Places) -> io::Result<Guard> {
        let state = &places.state;
        let memory_path = state.join("memory.json");
        let mut damaged = false;
        let memory = match state::load(&memory_path) {
            Ok(memory) => memory.unwrap_or_default(),
            // Begun afresh, and set aside at the first save for whoever
            // wants to know why, so a plan that never saves changes nothing.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                damaged = true;
                Memory::default()
            }
            Err(error) => return Err(error),
        };
        let predicates = predicates(&config);
        let status_path = places.status.clone().filter(|_| config.status_file);
        let listing_path = state.join("garbage.json");
        // One that can't be read is listed afresh.
        let listing: Option<Listing> = fs::read(&listing_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        // What can't be read starts afresh: an unwatched deps dates nothing.
        let answers_path = state.join("finished.json");
        let mut answers: Answers = loaded(&answers_path);
        answers.forget_gone();
        let uses_path = state.join("uses.json");
        let mut uses: Uses = loaded(&uses_path);
        uses.forget_gone();
        let home = matches!(config.scope, crate::config::Scope::User)
            .then(|| std::env::var_os("HOME").map(PathBuf::from))
            .flatten();
        // Started again, it says what held the disks when it last measured
        // until its own first measure ends, rather than nothing for minutes.
        let measured = crate::ledger::last(state, crate::holders::places(&config));
        Ok(Guard {
            config,
            processes: places.processes.clone(),
            system: places.system.clone(),
            memory_path,
            memory,
            journal: Journal::new(&state.join("journal.jsonl"), journal::LIMIT),
            status_path,
            predicates,
            answers,
            answers_path,
            watched: BTreeMap::new(),
            last: None,
            failed: None,
            passing: None,
            saved_at: None,
            written: None,
            beaten: Arc::default(),
            resumed: false,
            stepped: Instant::now(),
            listing_begun: listing.as_ref().map(|listing| listing.listed_at),
            listing_ended: listing.as_ref().and_then(Listing::ended),
            listing_failed: false,
            writers_held: false,
            room_awaited: None,
            reached_short: BTreeMap::new(),
            listing,
            listing_path,
            listing_out: None,
            hearing: None,
            collected_on: BTreeSet::new(),
            running: places.running.clone(),
            system_heard: false,
            relist: false,
            tried: BTreeSet::new(),
            stale_listing: false,
            refused: BTreeSet::new(),
            holding: None,
            owners_at: None,
            produced: Vec::new(),
            snapshot_spends: None,
            snapshots_known: None,
            budget_looked: None,
            root_left: false,
            held_ended: false,
            collect: false,
            collect_rest: None,
            deletes_rest: None,
            left_by_collect: BTreeSet::new(),
            dedupe_out: None,
            dedupe_failed: None,
            uses: Arc::new(Mutex::new(uses)),
            uses_path,
            opens: None,
            marked_at: None,
            listening: Arc::new(AtomicBool::new(false)),
            min_free: None,
            home,
            freeze: None,
            disks: None,
            stop: Box::new(|| false),
            writers: Writers::default(),
            unread: (0, None),
            damaged,
            kept: Vec::new(),
            offered: None,
            sizes: crate::survey::Sizes::default(),
            remeasure: false,
            measured: Arc::new(Mutex::new(measured)),
            measuring: None,
            began: None,
        })
    }

    /// The candidate a source offers at `path`, or the nearest one holding
    /// it, and how it's in use now. Nothing is taken.
    pub fn explain(&mut self, tick: &Tick, path: &Path) -> io::Result<Explained> {
        self.memory.awake.observe(tick);
        self.watch(tick);
        let (mut scan, offers, _) = self.offers(tick, None)?;
        let holds = |held: &Path| path.starts_with(held);
        let offered = offers
            .into_iter()
            .map(|offer| offer.candidate)
            .filter(|candidate| holds(&candidate.path))
            .max_by_key(|candidate| candidate.path.as_os_str().len());
        let kept = self
            .kept
            .iter()
            .filter(|(tree, _)| holds(tree))
            .max_by_key(|(tree, _)| tree.as_os_str().len())
            .cloned();
        // The nearest of either is what decides it.
        let deeper = |kept: &PathBuf, candidate: &Candidate| {
            kept.as_os_str().len() > candidate.path.as_os_str().len()
        };
        Ok(match (offered, kept) {
            (Some(candidate), Some((tree, _))) if !deeper(&tree, &candidate) => {
                let used = crate::reclaim::Looker::judge(
                    &mut scan,
                    &candidate.path,
                    candidate.checkout.as_deref(),
                )?;
                Explained::Offered { candidate, used }
            }
            (_, Some((path, used))) => Explained::Kept { path, used },
            (Some(candidate), None) => {
                let used = crate::reclaim::Looker::judge(
                    &mut scan,
                    &candidate.path,
                    candidate.checkout.as_deref(),
                )?;
                Explained::Offered { candidate, used }
            }
            (None, None) => Explained::Nothing,
        })
    }

    /// How many processes the last survey couldn't read, even vouched for,
    /// and why the system daemon couldn't be asked, if it couldn't: a tree
    /// one of them might use is kept, so a plan doesn't show it.
    pub fn unread(&self) -> (usize, Option<String>) {
        self.unread.clone()
    }

    /// Freezes a runaway writer through `freeze`, when nothing else is
    /// left to take; without one, nothing is ever frozen.
    pub fn freeze_with(&mut self, freeze: Box<dyn crate::freeze::Freeze>) {
        self.freeze = Some(freeze);
    }

    /// Reads `disks` at every step and says how hard each is written, and
    /// how worn, in the status.
    pub fn watch_disks(&mut self, disks: Box<dyn crate::disks::Disks>) {
        self.disks = Some(crate::disks::Watch::new(disks));
    }

    /// Ends a survey as soon as `stop` says the daemon was told to stop,
    /// rather than once its walks are done, and acts on nothing it found:
    /// a stop otherwise waits out every walk, holding up the switch that
    /// sent it.
    pub fn stop_when(&mut self, stop: Box<dyn Fn() -> bool>) {
        self.stop = stop;
    }

    pub fn step(&mut self, tick: &Tick, senses: &Senses) -> io::Result<()> {
        self.stepped = Instant::now();
        // Once: what it resumes it journals as ended, so looking again
        // would only re-read the journal every step to find nothing.
        if !self.resumed {
            self.resumed = true;
            self.resume(tick)?;
        }
        self.memory.awake.observe(tick);
        self.watch(tick);
        if let Some(disks) = &mut self.disks {
            disks.observe(tick);
        }
        self.mark(tick);
        self.heed_holds();
        self.read_min_free(tick.wall);
        self.heartbeat();
        self.heard_garbage(tick.wall);
        self.watch_owners(tick);
        // An overloaded machine spares nothing for routine work; a disk past
        // healthy is still answered, since its bands don't ask for a spare
        // moment.
        let overload = overloaded(senses.pressure);
        let spare_moment =
            spare(senses.power, senses.saver, &self.memory.awake, tick.wall) && !overload;
        let mut due = Vec::new();
        let heard = self.system_heard;
        let writers_held = self.writers_held;
        for (&device, watched) in &mut self.watched {
            let Ok(space) = crate::sys::space(&watched.path) else {
                watched.space = None;
                continue;
            };
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            watched.band = thresholds.band(&space, watched.band);
            watched.fill.observe(tick.monotonic, space.free);
            watched.space = Some(space);
            if watched.band == Band::Healthy {
                watched.rest = None;
            }
            let decision = decide(&Situation {
                band: watched.band,
                free: space.free,
                thresholds,
                fill: &watched.fill,
                pass_seconds: self.memory.pass_seconds.unwrap_or(FIRST_PASS_SECONDS),
                upkept: self.memory.upkept,
                now: tick.wall,
                spare: spare_moment,
            });
            // The spell a cut reach belonged to is over, and what it judged
            // kept may be free to clear when the disk fills again; not while
            // a healthy disk still reaches, as it does when forecast to fill.
            if watched.band == Band::Healthy && !matches!(decision, Decision::Reach(_)) {
                self.reached_short.remove(&device);
            }
            let resting = watched.rest.is_some_and(|rest| {
                tick.monotonic < rest.until
                    && !(rest.unvouched && heard)
                    && matches!(decision, Decision::Reach(band) if band <= rest.band)
            });
            // A user daemon on this filesystem asks for the garbage in its
            // turn; the system's own upkeep goes on.
            let driven =
                matches!(decision, Decision::Reach(_)) && spoken_for(self.hearing.as_ref(), device);
            let waits = held_off(writers_held, watched.band);
            if decision != Decision::Rest && !resting && !driven && !waits {
                due.push((device, decision, thresholds));
            }
        }
        // The disk shortest of room passes first, so a long pass on another
        // never keeps it waiting.
        due.sort_by_key(|&(device, ..)| {
            Reverse(self.watched.get(&device).map(|watched| watched.band))
        });
        // Once the bands are read, and before a pass: on a restart every
        // band reads healthy until this step reads it, so the first measure
        // waited out a whole pass, and passes took 20 and 29 minutes on 26
        // October.
        self.measure_holders(tick.wall);
        self.watch_roots();
        self.list_garbage(tick.wall, spare_moment, overload, roomy(senses.pressure));
        self.collect_garbage(tick.wall, &mut due);
        // Deduplicating reads every file not yet linked and relinks its
        // twins, and nothing is lost by putting it off, so it alone still
        // waits for a calm disk.
        let calm_moment = spare_moment && calm(senses.pressure) && !self.writers_held;
        self.dedupe(tick, calm_moment, powered(senses.power, senses.saver))?;
        self.collects(tick);
        let asked = self.clear_asked(tick)?;
        let produced = self.produce(tick)?;
        let passed = asked || produced || !due.is_empty();
        if passed {
            // So a pass's heartbeat starts from what the disks are now. A
            // disk too full to hold the status is the one the pass is for,
            // so a status unwritten stops nothing but the telling.
            if let Err(error) = self.write_status(tick.wall) {
                say!("disk-guard: writing the status before a pass: {error}");
            }
            self.passes(tick, &due)?;
        }
        let frozen = self.memory.frozen.len();
        self.runaways(tick)?;
        let passed = passed || frozen != self.memory.frozen.len();
        if passed || self.saved_at.is_none_or(|at| tick.wall - at >= SAVE_EVERY) {
            if self.damaged {
                let aside = self.memory_path.with_file_name("memory.json.damaged");
                fs::rename(&self.memory_path, aside)?;
                self.damaged = false;
            }
            state::save(&self.memory_path, &self.memory)?;
            let uses = self
                .uses
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            state::save(&self.uses_path, &uses)?;
            state::save(&self.answers_path, &self.answers)?;
            self.saved_at = Some(tick.wall);
        }
        self.write_status(tick.wall)
    }

    /// Empties what a run that died mid reclaim left moved aside: the trash
    /// in each directory it began and never ended. Never put back, since
    /// its paths have been missing since, and outputs restored beside a
    /// build that has moved on could pass for fresh.
    pub(super) fn resume(&mut self, tick: &Tick) -> io::Result<()> {
        for (path, root) in self.journal.unfinished()? {
            let emptied = (|| -> io::Result<()> {
                let relative = path
                    .strip_prefix(&root)
                    .map_err(|_| io::Error::other("outside its root"))?;
                let dir = match Dir::open(&root).and_then(|root| root.child(relative)) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                    opened => opened?,
                };
                for left in dir.leftovers()? {
                    left.empty()?;
                }
                Ok(())
            })();
            self.journal.write(&Line::Action {
                at: tick.wall,
                band: Band::Healthy,
                path,
                kind: "resumed".to_owned(),
                size: 0,
                outcome: match emptied {
                    Ok(()) => Outcome::Reclaimed,
                    Err(error) => Outcome::Failed(format!("emptying what was left: {error}")),
                },
            })?;
        }
        Ok(())
    }

    /// Tells the system daemon which filesystems this user daemon watches,
    /// learning those it collects garbage on, whose passes this one then
    /// drives. A system daemon that doesn't answer collects on none, and
    /// passes alone a minute later.
    pub(super) fn heartbeat(&mut self) {
        let Some(socket) = &self.system else {
            self.system_heard = false;
            return;
        };
        let paths: Vec<PathBuf> = self.watched.values().map(|w| w.path.clone()).collect();
        if paths.is_empty() {
            self.collected_on.clear();
            return;
        }
        let watching = Request::Watching { paths };
        let heard = crate::socket::ask(socket, &watching, HEARTBEAT_TIMEOUT);
        self.system_heard = heard.is_ok();
        self.collected_on = match heard {
            Ok(Reply::Collects(held)) => held.iter().filter_map(|path| pool(path).ok()).collect(),
            _ => BTreeSet::new(),
        };
    }
}
