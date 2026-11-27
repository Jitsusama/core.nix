//! The status the guard writes for `disk-guard status`, the bar and the
//! shell, with its heartbeat, and the previews it answers.

use super::{Guard, WRITING_WINDOW, ending, held_off, wall_now};
use crate::band::{Band, Space, Thresholds};
use crate::clock::Tick;
use crate::pass::{Disk, Goal, Learned, Outcome, Reclaimer, pass};
use crate::plan::Candidate;
use crate::units::{Size, float, whole};
use crate::writers::Frozen;
use crate::{state, writers};
use serde::Serialize;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, PoisonError};
use std::time::Duration;
use std::{fs, io};

/// How often the status file is written when nothing in it has changed,
/// so its heartbeat says the guard is alive.
pub(super) const HEARTBEAT_EVERY: f64 = 30.0;

/// How far, as a share of itself, a forecast or the time until full may
/// move before it's written ahead of the heartbeat. Each is remade from
/// the fill's trend at every step, so it moves a little every few
/// seconds; written each time, the desktop woke 26 times a minute to
/// read figures it showed no differently (2026-10-31).
pub(super) const ESTIMATE_DRIFT: f64 = 0.1;

/// The status file's shape. The second added sentences, each disk's use
/// and fill, and what was cleared, beside everything the first had.
pub const STATUS_VERSION: u32 = 2;

/// How many things cleared the status file lists.
pub(super) const CLEARED_SHOWN: usize = 20;

/// What the status file says, for the desktop's disk piece and
/// `disk-guard status`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    pub version: u32,
    /// When the guard last wrote this, in wall seconds.
    pub heartbeat: f64,
    /// One sentence for the machine as a whole: its worst disk, or how
    /// much room its tightest has.
    pub says: String,
    pub filesystems: Vec<FilesystemStatus>,
    /// The last things cleared, newest first.
    pub cleared: Vec<ClearedStatus>,
    pub last: Option<LastPass>,
    /// The pass running as this was written, if one is. `last` is the one
    /// before it, which says nothing of what this one has cleared since.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passing: Option<Passing>,
    /// The pass whose clearing failed, while its failing stands: until a
    /// pass on its disk frees some or finds it no longer filling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<LastPass>,
    /// The writers frozen now, for the desktop to offer to thaw.
    pub frozen: Vec<crate::writers::Frozen>,
    /// Who wrote most of the last minute's writes, while a disk is below
    /// healthy and one process did.
    pub writer: Option<Writing>,
    /// What stands to be told now, from any band below healthy, for
    /// whoever reads this file: the desktop is told only once a disk is nearly full.
    pub notice: Option<crate::notices::Notice>,
    /// What holds the disks, biggest first, measured while one is below
    /// healthy, and when it was measured, in wall seconds.
    pub holders: Vec<crate::holders::Holder>,
    pub holders_at: Option<f64>,
    /// How hard each disk is written and how worn it is, for a guard
    /// given its disks to watch: the machine's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disks: Vec<crate::disks::DiskStatus>,
}

/// A process writing most of what's written, as the status file says it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Writing {
    pub name: String,
    pub pid: u32,
    /// Bytes a second.
    pub rate: f64,
}

/// One thing cleared, as the status file lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ClearedStatus {
    /// When, in wall seconds.
    pub at: f64,
    pub kind: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub says: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FilesystemStatus {
    pub path: PathBuf,
    pub band: Band,
    pub free: u64,
    /// Its size, as `size`; kept for readers of the first version.
    pub total: u64,
    pub size: u64,
    pub used: u64,
    /// Seconds until the next band at the fill's trend, if it's filling.
    pub forecast: Option<f64>,
    /// Bytes an hour it's filling at over the last two hours or so,
    /// negative when space is coming back; none until the guard has
    /// watched it ten minutes.
    pub fill_per_hour: Option<f64>,
    /// When it's full at the fill's trend, in wall seconds, if it's
    /// filling, both over the trend and now. None while it's above
    /// `nix_floor`, since Nix collects there before it fills.
    pub full_at: Option<f64>,
    /// The free space Nix keeps here by collecting its own garbage, its
    /// `min-free`, when Nix's store is on this filesystem and Nix does.
    pub nix_floor: Option<u64>,
    /// When it reaches `nix_floor` at the fill's trend, in wall seconds,
    /// if it's above it and filling.
    pub floor_at: Option<f64>,
    pub says: String,
    /// Whether the last pass here, in this band or a worse one, found
    /// nothing more it may clear.
    pub exhausted: bool,
    /// Whether a writers' hold keeps passes off this disk, as it does
    /// above nearly full, so it isn't told as clearing.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub waiting: bool,
    /// Whether a writers' hold stands over this disk with no clear Joel
    /// asked for at work on it, from which each measure works out
    /// `waiting` again, since a heartbeat's measure can move the band
    /// either way.
    #[serde(skip)]
    pub held: bool,
}

impl FilesystemStatus {
    /// Takes a fresh measure, during a pass long enough that its start's
    /// would mislead: its band is worked out as the guard's watch works it
    /// out, from the band it showed. A forecast made for another band no
    /// longer holds, so it goes; when it fills is kept, from the fill the
    /// pass began with.
    pub fn measured(
        &mut self,
        space: &Space,
        bands: &crate::config::Bands,
        now: f64,
        home: Option<&Path>,
    ) {
        let band = Thresholds::new(bands, space.total).band(space, self.band);
        if band != self.band {
            self.forecast = None;
        }
        // A worse band takes more than the pass that found nothing left, and
        // its own pass hasn't run yet.
        if band > self.band {
            self.exhausted = false;
        }
        self.waiting = held_off(self.held, band);
        self.band = band;
        self.free = space.free;
        self.total = space.total;
        self.size = space.total;
        self.used = space.total.saturating_sub(space.free);
        // A forecast outlived by a long pass goes, rather than being told as
        // "full in 0 s" by every reader of the status, the bar's included.
        if self.full_at.is_some_and(|at| at <= now) {
            self.full_at = None;
        }
        // So does a time to Nix's floor, which the broker passes on as a
        // moment still to come.
        if self.floor_at.is_some_and(|at| at <= now) {
            self.floor_at = None;
        }
        self.says = crate::words::filesystem(&self.told(now), home);
    }

    /// The room it's told from at `now`: until Nix's floor while it's
    /// above it with a time to reach it, and until full while it's at or
    /// under it with a time to fill. A fresh measure that crossed the floor
    /// either way since the status was worked out has neither time, so
    /// neither is told: one at or under the floor has reached it, and one
    /// back above it doesn't fill while Nix collects. Its own sentence and
    /// the machine's both come from here, so they never disagree.
    pub fn told(&self, now: f64) -> crate::words::Room<'_> {
        let above = self.nix_floor.is_some_and(|floor| self.free > floor);
        match (self.floor_at, self.full_at) {
            (Some(at), _) if above => self.ahead(Some(at - now), true),
            (None, Some(at)) if !above => self.ahead(Some(at - now), false),
            _ => self.ahead(None, false),
        }
    }

    /// Its room, with `next` seconds until it's full, or until it reaches
    /// Nix's floor when `to_floor`.
    pub(super) fn ahead(&self, next: Option<f64>, to_floor: bool) -> crate::words::Room<'_> {
        crate::words::Room {
            path: &self.path,
            band: self.band,
            free: self.free,
            size: self.size,
            full_in: next,
            nix_floor: self.nix_floor.filter(|_| to_floor),
            cleared_all: self.exhausted,
            waiting: self.waiting,
        }
    }
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

/// A pass at work: since when, on which disk, in what band and why. One
/// reaching for room a disk can't give judges every offer it was handed,
/// and on 23 November one ran from 08:51 to past 23:00 while the status
/// still told the pass before it, which freed nothing, and its notice to
/// delete something by hand.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Passing {
    pub since: f64,
    pub path: PathBuf,
    pub band: Band,
    pub why: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LastPass {
    pub at: f64,
    pub path: PathBuf,
    pub band: Band,
    pub end: String,
    pub freed: u64,
    /// How it went wrong, if it's a pass that ought to be told.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failing: Option<crate::pass::Failing>,
}

/// Whether a status says something the one written didn't, beyond its
/// beats: the heartbeat, each disk's fill rate, and its forecast and time
/// until full or until Nix's floor while each is within `ESTIMATE_DRIFT`
/// of what was written.
/// A forecast that comes or goes is news, and so is anything said in
/// words, which is how a reader sees a forecast cross into another span.
pub fn news(written: &Status, status: &Status, now: f64) -> bool {
    let near = |was: Option<f64>, is: Option<f64>| match (was, is) {
        (None, None) => true,
        (Some(was), Some(is)) => (was - is).abs() <= ESTIMATE_DRIFT * was.abs().max(is.abs()),
        _ => false,
    };
    let left = |full_at: Option<f64>| full_at.map(|at| at - now);
    let drifted = written.filesystems.len() != status.filesystems.len()
        || written
            .filesystems
            .iter()
            .zip(&status.filesystems)
            .any(|(was, is)| {
                !near(was.forecast, is.forecast)
                    || !near(left(was.full_at), left(is.full_at))
                    || !near(left(was.floor_at), left(is.floor_at))
            });
    if drifted {
        return true;
    }
    // The status as written, carrying the estimates it had then, and the
    // same disks' counts, which move every step and are said at each
    // beat; a disk coming or going, or its health read, is news.
    let same_disks = written.disks.len() == status.disks.len()
        && written
            .disks
            .iter()
            .zip(&status.disks)
            .all(|(was, is)| was.name == is.name);
    let disks = if same_disks {
        written
            .disks
            .iter()
            .zip(&status.disks)
            .map(|(was, is)| crate::disks::DiskStatus {
                written_this_boot: was.written_this_boot,
                written_per_hour: was.written_per_hour,
                rated_wear_in: was.rated_wear_in,
                ..is.clone()
            })
            .collect()
    } else {
        status.disks.clone()
    };
    let steady = Status {
        heartbeat: written.heartbeat,
        disks,
        filesystems: written
            .filesystems
            .iter()
            .zip(&status.filesystems)
            .map(|(was, is)| FilesystemStatus {
                forecast: was.forecast,
                fill_per_hour: was.fill_per_hour,
                full_at: was.full_at,
                floor_at: was.floor_at,
                ..is.clone()
            })
            .collect(),
        ..status.clone()
    };
    steady != *written
}

/// Marks a status's holders with the keeps and clears in `state` as they
/// are now, since `disk-guard keeps` and `clear` change them between
/// steps. Keeps that won't read leave the holders as they were.
pub(super) fn mark(status: &mut Status, state: &Path, home: Option<&Path>) {
    if let Ok(keeps) = crate::keeps::load(state) {
        let clearing = crate::clears::clearing(state);
        crate::holders::mark(&mut status.holders, &keeps, &clearing, home);
    }
}

/// Writes the status file a guard left again with a fresh heartbeat and
/// all else as it said, or nothing when there's none or none that reads.
pub(super) fn restamp(path: &Path, now: f64) -> io::Result<()> {
    match state::load::<serde_json::Map<String, serde_json::Value>>(path) {
        Ok(Some(mut left)) => {
            left.insert("heartbeat".to_owned(), now.into());
            // A guard only starting has no pass at work, whatever the one
            // before was doing as it left.
            left.remove("passing");
            // Nor is a moment the last guard told still to come once it
            // has passed, which the broker would pass on as if it were.
            if let Some(serde_json::Value::Array(filesystems)) = left.get_mut("filesystems") {
                for filesystem in filesystems
                    .iter_mut()
                    .filter_map(serde_json::Value::as_object_mut)
                {
                    for moment in ["full_at", "floor_at"] {
                        let at = filesystem.get(moment).and_then(serde_json::Value::as_f64);
                        if at.is_some_and(|at| at <= now) {
                            filesystem.insert(moment.to_owned(), serde_json::Value::Null);
                        }
                    }
                }
            }
            state::save(path, &left)
        }
        _ => Ok(()),
    }
}

/// A disk for a preview, whose free space is what was pretended plus
/// what the pretend reclaimer has freed.
pub(super) struct Pretend {
    pub(super) space: Space,
    pub(super) free: Rc<Cell<u64>>,
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
pub(super) struct Pretending {
    pub(super) learned: Learned,
    pub(super) free: Rc<Cell<u64>>,
    pub(super) total: u64,
}

impl Reclaimer for Pretending {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let freed = whole(float(candidate.size) * self.learned.ratio(candidate.class));
        self.free
            .set(self.free.get().saturating_add(freed).min(self.total));
        Outcome::Reclaimed
    }
}

impl Guard {
    /// What a pass would take now on each filesystem, in `band` and with
    /// `free` bytes free when given, taking nothing. It runs the pass
    /// itself against a disk that frees what learning predicts, so it
    /// shows the order a pass goes in and where it would stop; whether
    /// each would be found in use is only known when it's tried.
    ///
    /// Unless `fresh`, it reads the survey the daemon's last pass kept,
    /// answering when that was, since a survey walks every source and on
    /// a full machine takes minutes.
    pub fn preview(
        &mut self,
        tick: &Tick,
        band: Option<Band>,
        free: Option<Size>,
        fresh: bool,
    ) -> io::Result<(Option<f64>, Vec<Preview>)> {
        self.memory.awake.observe(tick);
        self.watch();
        let kept = if fresh {
            None
        } else {
            crate::surveyed::load(self.state())
        };
        let (surveyed, offers): (_, Vec<(u64, Candidate)>) = match kept {
            Some(kept) => (
                Some(kept.at),
                kept.offered
                    .into_iter()
                    .map(|offered| (offered.device, offered.candidate))
                    .collect(),
            ),
            None => (
                None,
                self.offers(tick, None)?
                    .1
                    .into_iter()
                    .map(|offer| (offer.device, offer.candidate))
                    .collect(),
            ),
        };
        let mut previews = Vec::new();
        for (&device, watched) in &self.watched {
            let Ok(space) = crate::sys::space(&watched.path) else {
                continue;
            };
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            // A band asked for alone is pretended by free space just short
            // of it, so a plan for nearly full shows what that band would take.
            let short_of = match band {
                Some(Band::Filling) => Some(thresholds.filling),
                Some(Band::NearlyFull) => Some(thresholds.nearly_full),
                Some(Band::CriticallyFull) => Some(thresholds.critically_full),
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
                .filter(|(on, _)| *on == device)
                .map(|(_, candidate)| candidate.clone())
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
            // Looked up by path, since a plan can take tens of thousands
            // of candidates and a search of them each would be quadratic.
            // The first offer of a path is the one taken, as before.
            let mut by_path: HashMap<&Path, &crate::plan::Candidate> = HashMap::new();
            for candidate in &candidates {
                by_path.entry(candidate.path.as_path()).or_insert(candidate);
            }
            let taken = report
                .actions
                .iter()
                .filter_map(|(path, _)| by_path.get(path.as_path()).map(|&c| c.clone()))
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
        Ok((surveyed, previews))
    }

    /// Whether no pass is at work on the disk at `path` now. Under a
    /// writers' hold the only one that can be on a filling disk is a
    /// clear Joel asked for, or one begun nearly full.
    pub(super) fn unpassed(&self, path: &Path) -> bool {
        self.passing
            .as_ref()
            .is_none_or(|passing| passing.path != path)
    }

    pub fn status(&self, now: f64) -> Status {
        let bands = &self.config.bands;
        let home = self.home.as_deref();
        let nix_floor = self.nix_floor();
        let filesystems = self
            .watched
            .iter()
            .filter_map(|(&device, watched)| {
                let space = watched.space?;
                let thresholds = Thresholds::new(bands, space.total);
                let next = match watched.band {
                    Band::Healthy => Some(thresholds.filling),
                    Band::Filling => Some(thresholds.nearly_full),
                    Band::NearlyFull => Some(thresholds.critically_full),
                    Band::CriticallyFull => None,
                };
                // What a person is told comes from the trend, not the
                // two-minute rate passes are planned by, which a churning
                // disk swings either way by tens of gigabytes an hour.
                // Nix's store's disk counts to the floor Nix keeps while
                // above it: a straight line to empty told a Mac's builds
                // filled it in a day while Nix collected at 50 GiB.
                let ahead = watched.fill.forecast(space.free, device, nix_floor, now);
                let mut status = FilesystemStatus {
                    path: watched.path.clone(),
                    band: watched.band,
                    free: space.free,
                    total: space.total,
                    size: space.total,
                    used: space.total.saturating_sub(space.free),
                    // Nor is it told a band Nix's floor keeps it out of.
                    forecast: next
                        .and_then(|start| watched.fill.band_in(space.free, start, ahead.nix_floor)),
                    fill_per_hour: watched.fill.trend().map(|trend| trend * 3600.0),
                    full_at: ahead.full_at,
                    nix_floor: ahead.nix_floor,
                    floor_at: ahead.floor_at,
                    says: String::new(),
                    exhausted: self.last.as_ref().is_some_and(|last| {
                        last.path == watched.path
                            && self.unpassed(&watched.path)
                            && last.end == "exhausted"
                            && watched.band != Band::Healthy
                            && last.band >= watched.band
                    }) || (watched.band == Band::Filling
                        && self.unpassed(&watched.path)
                        && watched.rest.is_some_and(|rest| rest.emptied)),
                    // A clear Joel asked for runs under a hold, and is
                    // told as the clear it is.
                    held: self.writers_held && self.unpassed(&watched.path),
                    waiting: false,
                };
                status.waiting = held_off(status.held, status.band);
                status.says = crate::words::filesystem(&status.told(now), home);
                Some(status)
            })
            .collect::<Vec<_>>();
        // The machine's sentence is its worst filesystem's, so the store's
        // counts to Nix's floor there too.
        let rooms: Vec<_> = filesystems.iter().map(|status| status.told(now)).collect();
        let says = crate::words::machine(&rooms, home);
        let cleared = self
            .journal
            .cleared()
            .into_iter()
            .take(CLEARED_SHOWN)
            .map(|one| ClearedStatus {
                says: crate::words::cleared(&one.kind, &one.path, one.bytes, home),
                at: one.at,
                kind: one.kind,
                path: one.path,
                bytes: one.bytes,
            })
            .collect();
        let mut status = Status {
            version: STATUS_VERSION,
            heartbeat: now,
            says,
            filesystems,
            cleared,
            last: self.last.clone(),
            passing: self.passing.clone(),
            failed: self.failed.clone(),
            frozen: self.memory.frozen.clone(),
            writer: self
                .writers
                .dominant(WRITING_WINDOW, now)
                .map(|writer| Writing {
                    name: writer.name,
                    pid: writer.pid,
                    rate: writer.rate,
                }),
            notice: None,
            holders: Vec::new(),
            holders_at: None,
            disks: self
                .disks
                .as_ref()
                .map_or_else(Vec::new, crate::disks::Watch::status),
        };
        status.notice = crate::notices::standing(&status);
        if let Some((at, measured, grew)) =
            &*self.measured.lock().unwrap_or_else(PoisonError::into_inner)
        {
            status.holders =
                crate::holders::holders(measured, self.offered.as_ref(), &self.kept, grew, home);
            status.holders_at = Some(*at);
        }
        mark(&mut status, self.state(), home);
        status
    }

    /// The folder the guard keeps its memory, journal and Joel's keeps in.
    pub(super) fn state(&self) -> &Path {
        self.memory_path.parent().unwrap_or(Path::new("/"))
    }

    /// Says the guard is alive: the status last written, with a fresh
    /// heartbeat and each filesystem's free space as it is now, so the
    /// desktop neither calls it silent nor shows stale room. A pass's
    /// pacer calls it as it works, and `beat_always`'s thread every
    /// `HEARTBEAT_EVERY`. Until this guard has written a status of its
    /// own it stamps the one the last guard left, so a restart, whose
    /// first step can take minutes, never reads as a guard that stopped.
    pub fn beating(&self) -> Box<dyn FnMut() + Send> {
        let Some(path) = self.status_path.clone() else {
            return Box::new(|| {});
        };
        let beaten = Arc::clone(&self.beaten);
        let bands = self.config.bands.clone();
        let home = self.home.clone();
        let state = self.state().to_owned();
        Box::new(move || {
            let now = wall_now();
            let mut beaten = beaten.lock().unwrap_or_else(PoisonError::into_inner);
            let saved = match beaten.as_mut() {
                None => restamp(&path, now),
                Some(status) => {
                    let beaten_for = now - status.heartbeat;
                    status.heartbeat = now;
                    for filesystem in &mut status.filesystems {
                        // A band's forecast is a span from when it was
                        // made, so it counts down beat by beat through a
                        // long pass rather than standing still.
                        filesystem.forecast = filesystem
                            .forecast
                            .map(|left| left - beaten_for)
                            .filter(|left| *left > 0.0);
                        if let Ok(space) = crate::sys::space(&filesystem.path) {
                            filesystem.measured(&space, &bands, now, home.as_deref());
                        }
                    }
                    let rooms: Vec<_> = status
                        .filesystems
                        .iter()
                        .map(|filesystem| filesystem.told(now))
                        .collect();
                    status.says = crate::words::machine(&rooms, home.as_deref());
                    mark(status, &state, home.as_deref());
                    state::share(&path, status)
                }
            };
            if let Err(error) = saved {
                say!("disk-guard: writing its status: {error}");
            }
        })
    }

    /// Beats on a thread of its own for as long as the process runs, so
    /// the status file's heartbeat says what the unit's state does: that
    /// the guard is running, however long a step, a delete Nix holds up
    /// or a restart's first look takes. The desktop calls a guard silent
    /// two minutes after its last heartbeat.
    pub fn beat_always(&self) -> io::Result<()> {
        let mut beat = self.beating();
        std::thread::Builder::new()
            .name("heartbeat".to_owned())
            .spawn(move || {
                loop {
                    beat();
                    std::thread::sleep(Duration::from_secs_f64(HEARTBEAT_EVERY));
                }
            })?;
        Ok(())
    }

    pub(super) fn write_status(&mut self, now: f64) -> io::Result<()> {
        let Some(path) = self.status_path.clone() else {
            return Ok(());
        };
        let mut status = self.status(now);
        // Stamped as it's written, not as its step began: a step can run
        // for minutes, and a status written old reads as a guard stopped.
        status.heartbeat = wall_now();
        let unchanged = self.written.as_ref().is_some_and(|(at, written)| {
            now - at < HEARTBEAT_EVERY && !news(written, &status, now)
        });
        if unchanged {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // One writer at a time, since a beat saves through the same file.
        let mut beaten = self.beaten.lock().unwrap_or_else(PoisonError::into_inner);
        state::share(&path, &status)?;
        *beaten = Some(status.clone());
        drop(beaten);
        self.written = Some((now, status));
        Ok(())
    }
}
