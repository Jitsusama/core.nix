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
use crate::pass::{End, Goal, pass};
use crate::state::{self, Memory};
use crate::survey::{Offer, World, survey};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
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
}

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
            if decision != Decision::Rest {
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
            });
            seen.insert(meta.dev(), watched);
        }
        self.watched = seen;
    }

    /// Surveys once and runs each filesystem's pass over what lies on it.
    fn passes(&mut self, tick: &Tick, due: &[(u64, Decision, Thresholds)]) -> io::Result<()> {
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
            let candidates: Vec<_> = here.iter().map(|offer| offer.candidate.clone()).collect();
            let (band, goal) = match decision {
                Decision::Reach(band) => (band, Goal::Reach),
                _ => (Band::Healthy, Goal::Upkeep),
            };
            let started = Instant::now();
            let mut reclaim = Reclaim::new(&here, &mut scan);
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
            let end = match &report.end {
                End::Reached => "reached".to_owned(),
                End::Exhausted => "exhausted".to_owned(),
                End::Stalled => "stalled".to_owned(),
                End::Failed(why) => format!("failed: {why}"),
            };
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
        }
        Ok(())
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
