//! The guard's loop, one step at a time: read the clocks, band and
//! forecast every filesystem a source lies on, and run a pass on each one
//! that needs it, journalling what it did and keeping what it learned.

use crate::band::{Band, Space, Thresholds};
use crate::clock::Tick;
use crate::config::{Config, Source};
use crate::decide::{Decision, Fill, Situation, decide, spare};
use crate::finished::{self, Answers, Predicate};
use crate::host::{FRESH, Filesystem, Reclaim, Scan};
use crate::journal::{self, Journal, Line};
use crate::moments::{Power, Pressure};
use crate::pass::{Disk, End, Goal, Learned, Outcome, Reclaimer, pass};
use crate::plan::Candidate;
use crate::state::{self, Memory};
use crate::survey::{Offer, World, survey};
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
}

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
        })
    }

    pub fn step(&mut self, tick: &Tick, senses: &Senses) -> io::Result<()> {
        self.memory.awake.observe(tick);
        self.watch();
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

    /// Notes every filesystem a source lies on, keeping what was seen of
    /// those already watched.
    fn watch(&mut self) {
        let mut seen = BTreeMap::new();
        for path in self.config.sources.values().flat_map(paths) {
            let Ok(meta) = fs::metadata(path) else {
                continue;
            };
            if !meta.is_dir() || seen.contains_key(&meta.dev()) {
                continue;
            }
            let watched = self.watched.remove(&meta.dev()).unwrap_or(Watched {
                path: path.to_owned(),
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
        let offers = survey(
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
            },
        );
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
        let mut reclaim = Reclaim::new(here, scan);
        let report = pass(
            &mut Filesystem::new(&path),
            &mut reclaim,
            &candidates,
            &thresholds,
            band,
            goal,
            &mut self.memory.learned,
        );
        if goal == Goal::Reach {
            let seconds = started.elapsed().as_secs_f64();
            self.memory.pass_seconds = Some(match self.memory.pass_seconds {
                Some(before) => before + PASS_WEIGHT * (seconds - before),
                None => seconds,
            });
        } else {
            self.memory.upkept = Some(tick.wall);
        }
        for (taken, outcome) in &report.actions {
            let Some(candidate) = candidates.iter().find(|c| &c.path == taken) else {
                continue;
            };
            self.journal.write(&Line::Action {
                at: tick.wall,
                band: report.band,
                path: taken.clone(),
                kind: candidate.class.kind().to_owned(),
                size: candidate.size,
                outcome: outcome.clone(),
            })?;
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
        | Source::NixBuilds { path } => vec![path.as_path()],
        Source::Podman { .. } | Source::Producer { .. } | Source::Finished { .. } => Vec::new(),
    }
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
