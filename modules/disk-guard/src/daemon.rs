//! The guard's loop, one step at a time: read the clocks, band and
//! forecast every filesystem a source lies on, and run a pass on each one
//! that needs it, journalling what it did and keeping what it learned.

use crate::anchor::Dir;
use crate::band::{Band, Space, Thresholds};
use crate::clock::Tick;
use crate::config::{Config, Source};
use crate::decide::{Decision, Fill, Situation, decide, spare};
use crate::finished::{self, Answers, Predicate};
use crate::garbage::{self, BATCH_BYTES, Batch, Dead, GC_TIMEOUT, Listing};
use crate::host::{FRESH, Filesystem, Reclaim, Scan};
use crate::journal::{self, Journal, Journaled, Line};
use crate::moments::{Power, Pressure};
use crate::pass::{Disk, End, Goal, Learned, Outcome, Reclaimer, pass};
use crate::plan::{Candidate, Class};
use crate::state::{self, Memory};
use crate::survey::{Action, Offer, World, named, survey};
use crate::units::Size;
use serde::Serialize;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

/// What the machine is doing, read by the caller each step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Senses {
    pub power: Power,
    /// Whether the power profile is saving power.
    pub saver: bool,
    pub pressure: Pressure,
}

/// A filesystem a source lies on, as last seen.
struct Watched {
    path: PathBuf,
    band: Band,
    fill: Fill,
    space: Option<Space>,
    /// After a pass that fell short: no other until this monotonic time,
    /// unless the band gets worse than it was.
    rest: Option<Rest>,
}

#[derive(Clone, Copy, Debug)]
struct Rest {
    until: f64,
    seconds: f64,
    band: Band,
}

/// How long a filesystem rests after a pass that fell short, at first and
/// at most: what was kept is in use or can't be judged, and looking again
/// at once would only find the same and spend a scan saying so.
const FIRST_REST: f64 = 60.0;
const LONGEST_REST: f64 = 900.0;

/// How long a pass is assumed to take before one has been timed.
const FIRST_PASS_SECONDS: f64 = 60.0;
/// The weight each timed pass carries against those before it.
const PASS_WEIGHT: f64 = 0.3;
/// How often what was learned is saved when no pass has run.
const SAVE_EVERY: f64 = 300.0;
/// How often the status file is written when nothing in it has changed,
/// so its heartbeat says the guard is alive.
const HEARTBEAT_EVERY: f64 = 30.0;

/// What the garbage's commands and candidates are named for.
const GARBAGE: &str = "garbage";
/// How old Nix's garbage listing may grow on a healthy disk, and under
/// pressure; and how long a listing that failed waits for the next.
const LISTING_FRESH: f64 = 86400.0;
const LISTING_PRESSED: f64 = 3600.0;
const LISTING_RETRY: f64 = 3600.0;
/// The most paths deleted in one go, so one refused path costs little.
const MOST_PER_BATCH: usize = 300;

/// Where a guard keeps and reads things.
#[derive(Clone, Debug, PartialEq)]
pub struct Places {
    /// Its memory and journal.
    pub state: PathBuf,
    /// Its status file, when the config asks for one.
    pub status: Option<PathBuf>,
    /// Where processes are read from: `/proc`.
    pub processes: PathBuf,
    /// The system daemon's socket, for a user daemon to ask about the
    /// processes it can't read.
    pub system: Option<PathBuf>,
}

pub struct Guard {
    config: Config,
    processes: PathBuf,
    system: Option<PathBuf>,
    memory_path: PathBuf,
    memory: Memory,
    journal: Journal,
    status_path: Option<PathBuf>,
    predicates: Vec<Predicate>,
    answers: Answers,
    watched: BTreeMap<u64, Watched>,
    last: Option<LastPass>,
    saved_at: Option<f64>,
    written: Option<(f64, Status)>,
    /// Whether what an earlier run left moved aside has been seen to.
    resumed: bool,
    /// Nix's garbage as last listed, for the system daemon, and where it's
    /// kept for the next guard.
    listing: Option<Listing>,
    listing_path: PathBuf,
    /// When a listing last began, so one that failed waits.
    listing_begun: Option<f64>,
    /// A listing under way on its own thread, and when it began.
    listing_out: Option<(f64, Listening)>,
}

/// Where a listing on its own thread sends the dead paths, or why not.
type Listening = Receiver<Result<Vec<Dead>, String>>;

/// What the status file says, for the desktop's disk piece and
/// `disk-guard status`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    pub version: u32,
    /// When the guard last wrote this, in wall seconds.
    pub heartbeat: f64,
    pub filesystems: Vec<FilesystemStatus>,
    pub last: Option<LastPass>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FilesystemStatus {
    pub path: PathBuf,
    pub band: Band,
    pub free: u64,
    pub total: u64,
    /// Seconds until the next band at the present fill, if it's filling.
    pub forecast: Option<f64>,
}

/// What a pass on one filesystem would take, in order, done nowhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub path: PathBuf,
    pub band: Band,
    pub free: u64,
    pub total: u64,
    pub target: u64,
    pub end: String,
    pub taken: Vec<crate::plan::Candidate>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LastPass {
    pub at: f64,
    pub path: PathBuf,
    pub band: Band,
    pub end: String,
    pub freed: u64,
}

impl Guard {
    pub fn new(config: Config, places: &Places) -> io::Result<Guard> {
        let state = &places.state;
        let memory_path = state.join("memory.json");
        let memory = match state::load(&memory_path) {
            Ok(memory) => memory.unwrap_or_default(),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                // Kept aside for whoever wants to know why, and begun afresh.
                fs::rename(&memory_path, state.join("memory.json.damaged"))?;
                Memory::default()
            }
            Err(error) => return Err(error),
        };
        let predicates = config
            .sources
            .values()
            .filter_map(|source| match source {
                Source::Finished {
                    remote,
                    command,
                    timeout,
                } => Some(Predicate {
                    remote: regex_lite::Regex::new(remote).ok()?,
                    command: command.iter().map(OsString::from).collect(),
                    timeout: timeout.0,
                }),
                _ => None,
            })
            .collect();
        let status_path = places.status.clone().filter(|_| config.status_file);
        let listing_path = state.join("garbage.json");
        // One that can't be read is listed afresh.
        let listing: Option<Listing> = fs::read(&listing_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        Ok(Guard {
            config,
            processes: places.processes.clone(),
            system: places.system.clone(),
            memory_path,
            memory,
            journal: Journal::new(&state.join("journal.jsonl"), journal::LIMIT),
            status_path,
            predicates,
            answers: Answers::default(),
            watched: BTreeMap::new(),
            last: None,
            saved_at: None,
            written: None,
            resumed: false,
            listing_begun: listing.as_ref().map(|listing| listing.listed_at),
            listing,
            listing_path,
            listing_out: None,
        })
    }

    pub fn step(&mut self, tick: &Tick, senses: &Senses) -> io::Result<()> {
        // Once: what it resumes it journals as ended, so looking again
        // would only re-read the journal every step to find nothing.
        if !self.resumed {
            self.resumed = true;
            self.resume(tick)?;
        }
        self.memory.awake.observe(tick);
        self.watch();
        self.heard_garbage(tick.wall)?;
        let spare = spare(
            senses.power,
            senses.saver,
            &self.memory.awake,
            tick.wall,
            senses.pressure,
        );
        let mut due = Vec::new();
        for (&device, watched) in &mut self.watched {
            let Ok(space) = crate::linux::space(&watched.path) else {
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
                spare,
            });
            let resting = watched.rest.is_some_and(|rest| {
                tick.monotonic < rest.until
                    && matches!(decision, Decision::Reach(band) if band <= rest.band)
            });
            if decision != Decision::Rest && !resting {
                due.push((device, decision, thresholds));
            }
        }
        self.list_garbage(tick.wall, spare);
        let passed = !due.is_empty();
        if passed {
            self.passes(tick, &due)?;
        }
        if passed || self.saved_at.is_none_or(|at| tick.wall - at >= SAVE_EVERY) {
            state::save(&self.memory_path, &self.memory)?;
            self.saved_at = Some(tick.wall);
        }
        self.write_status(tick.wall)
    }

    /// Empties what a run that died mid reclaim left moved aside: the trash
    /// in each directory it began and never ended. Never put back, since
    /// its paths have been missing since, and outputs restored beside a
    /// build that has moved on could pass for fresh.
    fn resume(&mut self, tick: &Tick) -> io::Result<()> {
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

    /// Takes a listing of Nix's garbage that has finished, keeping it for
    /// the next guard, and makes upkeep due when routine would take some.
    fn heard_garbage(&mut self, now: f64) -> io::Result<()> {
        let Some((begun, out)) = &self.listing_out else {
            return Ok(());
        };
        let heard = match out.try_recv() {
            Err(TryRecvError::Empty) => return Ok(()),
            Ok(heard) => heard,
            Err(TryRecvError::Disconnected) => Err("the listing stopped".to_owned()),
        };
        let begun = *begun;
        self.listing_out = None;
        match heard {
            Ok(dead) => {
                let listing = Listing {
                    listed_at: begun,
                    dead,
                };
                state::save(&self.listing_path, &listing)?;
                if self.batches(&listing, now).iter().any(|batch| !batch.kept) {
                    self.memory.upkept = None;
                }
                self.listing = Some(listing);
            }
            // Tried again in an hour; until then the listing before stands.
            Err(why) => eprintln!("disk-guard: listing Nix's garbage: {why}"),
        }
        Ok(())
    }

    /// Begins listing Nix's garbage on its own thread when it's due: daily
    /// at a spare moment, at idle priority; and at once when the store's
    /// filesystem is under pressure and the listing is an hour old.
    fn list_garbage(&mut self, now: f64, spare: bool) {
        if self.listing_out.is_some() {
            return;
        }
        let Some((garbage, device)) = self.store() else {
            return;
        };
        let pressed = self
            .watched
            .get(&device)
            .is_some_and(|watched| watched.band > Band::Healthy);
        let age = |at: Option<f64>| at.map_or(f64::INFINITY, |at| now - at);
        let listed = age(self.listing.as_ref().map(|listing| listing.listed_at));
        let due = age(self.listing_begun) >= LISTING_RETRY
            && if pressed {
                listed >= LISTING_PRESSED
            } else {
                spare && listed >= LISTING_FRESH
            };
        if !due {
            return;
        }
        let nix = garbage.nix.clone();
        let (tell, out) = mpsc::channel();
        let listing = std::thread::Builder::new()
            .name(GARBAGE.to_owned())
            .spawn(move || {
                // A guard that stopped meanwhile has no one to hear it.
                let _ = tell.send(garbage::list(&nix, !pressed));
            });
        self.listing_begun = Some(now);
        match listing {
            Ok(_) => self.listing_out = Some((now, out)),
            Err(error) => eprintln!("disk-guard: listing Nix's garbage: {error}"),
        }
    }

    /// The garbage config and the store's device, for a system daemon.
    fn store(&self) -> Option<(&crate::config::Garbage, u64)> {
        let garbage = &self.config.system.as_ref()?.garbage;
        let meta = fs::metadata(&garbage.store).ok()?;
        Some((garbage, meta.dev()))
    }

    /// The listing's garbage that is still there, in the batches and the
    /// order it goes in.
    fn batches(&self, listing: &Listing, now: f64) -> Vec<Batch> {
        let Some((garbage, _)) = self.store() else {
            return Vec::new();
        };
        let Ok(space) = crate::linux::space(&garbage.store) else {
            return Vec::new();
        };
        let there: Vec<Dead> = listing
            .dead
            .iter()
            .filter(|dead| Path::new(&dead.path).symlink_metadata().is_ok())
            .cloned()
            .collect();
        let keep = garbage.keep.0.as_secs_f64();
        garbage::batches(
            &there,
            now,
            keep,
            garbage.budget.of(space.total),
            MOST_PER_BATCH,
        )
    }

    /// What Nix's garbage offers: the listing's batches in the policy's
    /// order; or with no listing yet, from urgent on, hourly steps of
    /// `nix store gc`, which collects in no order of its own.
    fn garbage_offers(&self, tick: &Tick) -> Vec<Offer> {
        let Some((garbage, device)) = self.store() else {
            return Vec::new();
        };
        let store = &garbage.store;
        let nix = garbage.nix.to_string_lossy().into_owned();
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
            let ran = self.memory.ran.get(GARBAGE).map_or(f64::INFINITY, |&at| {
                self.memory.awake.since(at.min(tick.wall), tick.wall)
            });
            let class = Class::Producer {
                every: None,
                from: Some(Band::Urgent),
            };
            let max = BATCH_BYTES.to_string();
            return vec![Offer {
                candidate: candidate(named(store, GARBAGE), class, ran, 0),
                action: Action::Run {
                    root: store.clone(),
                    source: GARBAGE.to_owned(),
                    argv: [nix.as_str(), "store", "gc", "--max", &max]
                        .map(str::to_owned)
                        .to_vec(),
                    timeout: GC_TIMEOUT,
                },
                device,
            }];
        };
        self.batches(listing, tick.wall)
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
                    nix: garbage.nix.clone(),
                    paths: batch.paths,
                },
                device,
            })
            .collect()
    }

    /// Notes every filesystem a source or the store lies on, keeping what
    /// was seen of those already watched.
    fn watch(&mut self) {
        let mut seen = BTreeMap::new();
        let store = self
            .config
            .system
            .as_ref()
            .map(|system| system.garbage.store.as_path());
        for path in self.config.sources.values().flat_map(paths).chain(store) {
            let Ok(meta) = fs::metadata(path) else {
                continue;
            };
            if !meta.is_dir() || seen.contains_key(&meta.dev()) {
                continue;
            }
            let watched = self.watched.remove(&meta.dev()).unwrap_or_else(|| Watched {
                path: mounted(path, meta.dev()),
                band: Band::Healthy,
                fill: Fill::default(),
                space: None,
                rest: None,
            });
            seen.insert(meta.dev(), watched);
        }
        self.watched = seen;
    }

    /// Surveys once and runs each filesystem's pass over what lies on it.
    fn passes(&mut self, tick: &Tick, due: &[(u64, Decision, Thresholds)]) -> io::Result<()> {
        let (mut scan, offers) = self.offers(tick)?;
        for &(device, decision, thresholds) in due {
            let Some(path) = self
                .watched
                .get(&device)
                .map(|watched| watched.path.clone())
            else {
                continue;
            };
            let here: Vec<Offer> = offers
                .iter()
                .filter(|offer| offer.device == device)
                .cloned()
                .collect();
            let end = self.pass_on(tick, path, &here, decision, thresholds, &mut scan)?;
            let Decision::Reach(band) = decision else {
                continue;
            };
            if let Some(watched) = self.watched.get_mut(&device) {
                watched.rest = (end != End::Reached).then(|| {
                    let seconds = watched
                        .rest
                        .map_or(FIRST_REST, |rest| (rest.seconds * 2.0).min(LONGEST_REST));
                    Rest {
                        until: tick.monotonic + seconds,
                        seconds,
                        band,
                    }
                });
            }
        }
        Ok(())
    }

    /// Every process, scanned for the pass to judge by, and what every
    /// source offers.
    fn offers(&mut self, tick: &Tick) -> io::Result<(Scan, Vec<Offer>)> {
        let me = std::process::id();
        let mut scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
        if let Some(socket) = &self.system {
            let roots: Vec<PathBuf> = self
                .config
                .sources
                .values()
                .flat_map(paths)
                .map(Path::to_owned)
                .collect();
            scan = scan.vouched_by(socket, &roots);
        }
        let processes = scan.processes()?.to_vec();
        let boot = crate::linux::boot(Path::new("/proc/stat"))?;
        let (predicates, answers) = (&self.predicates, &mut self.answers);
        let mut finished = |dir: &Path| {
            let Some(checkout) = finished::read(dir) else {
                return false;
            };
            let Some(predicate) = finished::predicate_for(predicates, &checkout.remote) else {
                return false;
            };
            answers.finished(dir, &checkout.head, tick.wall, || {
                finished::ask(predicate, dir, &checkout)
            })
        };
        let mut offers = survey(
            &self.config,
            &mut World {
                processes: &processes,
                me,
                // SAFETY: geteuid only reads the process's own credentials.
                uid: unsafe { libc::geteuid() },
                boot,
                awake: &self.memory.awake,
                now: tick.wall,
                finished: &mut finished,
                ran: &self.memory.ran,
            },
        );
        offers.extend(self.garbage_offers(tick));
        Ok((scan, offers))
    }

    /// Runs one filesystem's pass over what lies there and journals it.
    fn pass_on(
        &mut self,
        tick: &Tick,
        path: PathBuf,
        here: &[Offer],
        decision: Decision,
        thresholds: Thresholds,
        scan: &mut Scan,
    ) -> io::Result<End> {
        let candidates: Vec<_> = here.iter().map(|offer| offer.candidate.clone()).collect();
        let (band, goal) = match decision {
            Decision::Reach(band) => (band, Goal::Reach),
            _ => (Band::Healthy, Goal::Upkeep),
        };
        let started = Instant::now();
        let gentle = band < Band::Urgent;
        let pressure = self.processes.join("pressure");
        let reclaimer = Reclaim::new(here, scan);
        let mut reclaim = Journaled {
            inner: Yielding {
                inner: if gentle {
                    reclaimer.gently()
                } else {
                    reclaimer
                },
                busy: move || {
                    gentle
                        && !crate::decide::calm(
                            crate::linux::pressure(&pressure).unwrap_or_default(),
                        )
                },
                yielded: false,
            },
            journal: &mut self.journal,
            roots: here
                .iter()
                .map(|offer| (offer.candidate.path.clone(), offer.action.root().to_owned()))
                .collect(),
            at: tick.wall,
            band,
        };
        let report = pass(
            &mut Filesystem::new(&path),
            &mut reclaim,
            &candidates,
            &thresholds,
            band,
            goal,
            &mut self.memory.learned,
        );
        let yielded = reclaim.inner.yielded;
        if goal == Goal::Reach {
            let seconds = started.elapsed().as_secs_f64();
            self.memory.pass_seconds = Some(match self.memory.pass_seconds {
                Some(before) => before + PASS_WEIGHT * (seconds - before),
                None => seconds,
            });
        } else if !yielded {
            self.memory.upkept = Some(tick.wall);
        }
        // Whatever came of it, so a failing command waits its window too.
        for (taken, _) in &report.actions {
            let offered = here.iter().find(|offer| &offer.candidate.path == taken);
            if let Some(Action::Run { source, .. }) = offered.map(|offer| &offer.action) {
                self.memory.ran.insert(source.clone(), tick.wall);
            }
        }
        let end = ending(&report.end);
        self.journal.write(&Line::Pass {
            at: tick.wall,
            band: report.band,
            end: end.clone(),
            freed: report.freed,
        })?;
        self.last = Some(LastPass {
            at: tick.wall,
            path,
            band: report.band,
            end,
            freed: report.freed,
        });
        Ok(report.end)
    }

    /// What a pass would take now on each filesystem, in `band` and with
    /// `free` bytes free when given, taking nothing. It runs the pass
    /// itself against a disk that frees what learning predicts, so it
    /// shows the order a pass goes in and where it would stop; whether
    /// each would be found in use is only known when it's tried.
    pub fn preview(
        &mut self,
        tick: &Tick,
        band: Option<Band>,
        free: Option<Size>,
    ) -> io::Result<Vec<Preview>> {
        self.memory.awake.observe(tick);
        self.watch();
        let (_, offers) = self.offers(tick)?;
        let mut previews = Vec::new();
        for (&device, watched) in &self.watched {
            let Ok(space) = crate::linux::space(&watched.path) else {
                continue;
            };
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            // A band asked for alone is pretended by free space just short
            // of it, so a plan for urgent shows what urgent would take.
            let short_of = match band {
                Some(Band::Pressure) => Some(thresholds.pressure),
                Some(Band::Urgent) => Some(thresholds.urgent),
                Some(Band::Critical) => Some(thresholds.critical),
                Some(Band::Healthy) | None => None,
            };
            let pretended = match (free, short_of) {
                (Some(free), _) => free.of(space.total),
                (None, Some(start)) => space.free.min(start.saturating_sub(1)),
                (None, None) => space.free,
            };
            let space = Space {
                free: pretended,
                ..space
            };
            let band = band.unwrap_or_else(|| thresholds.band(&space, Band::Healthy));
            let goal = if band == Band::Healthy {
                Goal::Upkeep
            } else {
                Goal::Reach
            };
            let candidates: Vec<_> = offers
                .iter()
                .filter(|offer| offer.device == device)
                .map(|offer| offer.candidate.clone())
                .collect();
            let freed = Rc::new(Cell::new(space.free));
            let mut learned = self.memory.learned.clone();
            let report = pass(
                &mut Pretend {
                    space,
                    free: freed.clone(),
                },
                &mut Pretending {
                    learned: self.memory.learned.clone(),
                    free: freed,
                    total: space.total,
                },
                &candidates,
                &thresholds,
                band,
                goal,
                &mut learned,
            );
            let taken = report
                .actions
                .iter()
                .filter_map(|(path, _)| candidates.iter().find(|c| &c.path == path).cloned())
                .collect();
            previews.push(Preview {
                path: watched.path.clone(),
                band,
                free: space.free,
                total: space.total,
                target: thresholds.target,
                end: ending(&report.end),
                taken,
            });
        }
        Ok(previews)
    }

    pub fn status(&self, now: f64) -> Status {
        let bands = &self.config.bands;
        let filesystems = self
            .watched
            .values()
            .filter_map(|watched| {
                let space = watched.space?;
                let thresholds = Thresholds::new(bands, space.total);
                let next = match watched.band {
                    Band::Healthy => Some(thresholds.pressure),
                    Band::Pressure => Some(thresholds.urgent),
                    Band::Urgent => Some(thresholds.critical),
                    Band::Critical => None,
                };
                Some(FilesystemStatus {
                    path: watched.path.clone(),
                    band: watched.band,
                    free: space.free,
                    total: space.total,
                    forecast: next.and_then(|start| watched.fill.until(space.free, start)),
                })
            })
            .collect();
        Status {
            version: state::VERSION,
            heartbeat: now,
            filesystems,
            last: self.last.clone(),
        }
    }

    fn write_status(&mut self, now: f64) -> io::Result<()> {
        let Some(path) = self.status_path.clone() else {
            return Ok(());
        };
        let status = self.status(now);
        let unchanged = self.written.as_ref().is_some_and(|(at, written)| {
            now - at < HEARTBEAT_EVERY
                && Status {
                    heartbeat: now,
                    ..written.clone()
                } == status
        });
        if unchanged {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        state::save(&path, &status)?;
        self.written = Some((now, status));
        Ok(())
    }
}

/// The paths a source lies under, which say what filesystems it's on.
fn paths(source: &Source) -> Vec<&Path> {
    match source {
        Source::Cargo { roots, .. } | Source::Installs { roots, .. } => {
            roots.iter().map(PathBuf::as_path).collect()
        }
        Source::Cache { path, .. }
        | Source::Scratch { path, .. }
        | Source::Snapshots { path, .. }
        | Source::Workspaces { path, .. }
        | Source::Temp { path }
        | Source::Trash { path }
        | Source::Logs { path }
        | Source::Coredumps { path }
        | Source::NixBuilds { path }
        | Source::Producer { path, .. }
        | Source::Podman { path, .. } => vec![path.as_path()],
        Source::Finished { .. } => Vec::new(),
    }
}

/// A reclaimer for a pass that can wait, which gives way once the disk or
/// memory gets busy, keeping all that's left for a calmer moment. Asked
/// between candidates, so a delete under way finishes. Urgent and
/// critical passes never give way.
struct Yielding<R, B> {
    inner: R,
    busy: B,
    yielded: bool,
}

impl<R: Reclaimer, B: FnMut() -> bool> Reclaimer for Yielding<R, B> {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        if self.yielded || (self.busy)() {
            self.yielded = true;
            return Outcome::Kept("the machine got busy; left for a calmer moment".to_owned());
        }
        self.inner.reclaim(candidate)
    }
}

/// Where the filesystem holding `path`, on `device`, is mounted: the
/// highest directory above it still on that device, which is how a
/// person knows the disk. A Btrfs subvolume counts as its own, as the
/// kernel reports it. Failing that, `path` itself.
fn mounted(path: &Path, device: u64) -> PathBuf {
    let Ok(mut top) = fs::canonicalize(path) else {
        return path.to_owned();
    };
    while let Some(above) = top.parent()
        && fs::metadata(above).is_ok_and(|meta| meta.dev() == device)
    {
        top = above.to_owned();
    }
    top
}

/// How a pass ended, in the journal's words.
fn ending(end: &End) -> String {
    match end {
        End::Reached => "reached".to_owned(),
        End::Exhausted => "exhausted".to_owned(),
        End::Stalled => "stalled".to_owned(),
        End::Failed(why) => format!("failed: {why}"),
    }
}

/// A disk for a preview, whose free space is what was pretended plus
/// what the pretend reclaimer has freed.
struct Pretend {
    space: Space,
    free: Rc<Cell<u64>>,
}

impl Disk for Pretend {
    fn space(&mut self) -> io::Result<Space> {
        Ok(Space {
            free: self.free.get(),
            ..self.space
        })
    }

    fn settle(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn now(&self) -> f64 {
        0.0
    }
}

/// Frees what learning predicts each candidate would, taking nothing.
struct Pretending {
    learned: Learned,
    free: Rc<Cell<u64>>,
    total: u64,
}

impl Reclaimer for Pretending {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let freed = (candidate.size as f64 * self.learned.ratio(candidate.class)) as u64;
        self.free
            .set(self.free.get().saturating_add(freed).min(self.total));
        Outcome::Reclaimed
    }
}
