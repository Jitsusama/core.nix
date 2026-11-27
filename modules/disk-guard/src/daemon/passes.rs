//! Passes on a disk: which are due, how each is run and told, and how a
//! disk rests after one that fell short.

use super::status::{LastPass, Passing, Status};
use super::{Aim, Guard, Rest, Watched, Why, aim, ending, wall_now};
use crate::band::{Band, Thresholds};
use crate::clock::Tick;
use crate::config::Source;
use crate::decide::{Decision, Fill};
use crate::disk_reads::{Walk, Walked};
use crate::host::{FRESH, Filesystem, Reclaim, Scan};
use crate::journal::{Journaled, Line};
use crate::pace::{Monotonic, Pacer, Probe};
use crate::pass::{End, pass};
use crate::pool::pool;
use crate::survey::Offer;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;
use std::{fs, io};

/// How long a filesystem rests after a pass that fell short, at first and
/// at most: what was kept is in use or can't be judged, and looking again
/// at once would only find the same and spend a scan saying so. The worse
/// the band, the sooner it looks again, since what's in use there may
/// stop being so any moment and the disk can't wait long.
pub(super) const FIRST_REST: f64 = 60.0;

/// How long a filling disk rests once a second pass in a row finds
/// nothing more it may clear there. Each pass reads the metadata of
/// everything it judges, and on 25 November one 17 minutes after a pass
/// that freed 0 B read 15 GB of it in 24 minutes. The first such pass
/// rests only a minute, since one that expired generations or a build that
/// ended leaves garbage only the next can take. A filling disk has a tenth
/// of its room or more, and a worse band ends the rest at once.
pub(super) const EMPTY_REST: f64 = 3600.0;

/// How many times its own length a whole survey waits before the next,
/// while no disk is worse than filling, and the longest that wait may be.
/// On 27 November surveys reading 15 GB apiece walked for 32 of 50 minutes,
/// each for a reach that ended waiting or an upkeep that freed nothing,
/// and kept the disk busy beside everything Joel ran. Spaced three times
/// their walk, at most half an hour, seven from 05:58 to 09:26 still read
/// 136 GB, walking 15 to 30 minutes each, and none freed more than 1.3 GB.
/// The walk's length stands for what it read, so a survey slowed by a busy
/// disk waits the longer for it and one that read little waits little. A
/// filling disk has a tenth of its room or more, and a worse band, or a
/// forecast of one, waits for none.
pub(super) const SURVEY_SPACING: f64 = 12.0;

/// The longest a survey waits for the one before, however long it walked.
pub(super) const SURVEY_SPACING_LONGEST: f64 = 3.0 * 3600.0;

/// The shortest walk that spaces the next survey at all: one shorter read
/// too little to be worth a wait, and twelve times it is within the minute
/// a guard looks again anyway.
pub(super) const SURVEY_SPACED_FROM: f64 = 5.0;

/// How long after a whole survey that walked `seconds` the next waits, as
/// `SURVEY_SPACING` says.
pub(super) fn survey_spacing(seconds: f64) -> f64 {
    if seconds < SURVEY_SPACED_FROM {
        return 0.0;
    }
    (SURVEY_SPACING * seconds).min(SURVEY_SPACING_LONGEST)
}

/// Ends `watched`'s rest once it reads healthy, and forgets the emptied
/// hour kept for it in `emptied`, so a restart can't bring back a rest
/// the guard that kept it had already ended; whether it forgot one, so
/// the forgetting is saved before a restart reads it.
pub(super) fn healthy_ends_rest(
    watched: &mut Watched,
    emptied: &mut BTreeMap<PathBuf, f64>,
) -> bool {
    if watched.band != Band::Healthy {
        return false;
    }
    watched.rest = None;
    emptied.remove(&watched.path).is_some()
}

/// Whether a pass due as `decision` on a disk in `band` goes past a
/// survey's spacing: one for a disk worse than filling, or forecast to
/// be, as a forecast's pass starts early so it's done before the band
/// arrives.
pub(super) fn pressed(decision: Decision, band: Band) -> bool {
    band > Band::Filling || matches!(decision, Decision::Reach(reach) if reach > Band::Filling)
}

pub(super) fn longest_rest(band: Band) -> f64 {
    match band {
        Band::CriticallyFull => 60.0,
        Band::NearlyFull => 300.0,
        _ => 900.0,
    }
}

/// What marks a line on the error stream as an error for systemd's
/// journal, so `journalctl -p err` finds it. A Mac's log file would show
/// it as written, so there it's left off.
pub(super) const ERROR_LEVEL: &str = if cfg!(target_os = "linux") { "<3>" } else { "" };

/// What marks a line as a warning there, as `ERROR_LEVEL` marks an error.
pub(super) const WARNING_LEVEL: &str = if cfg!(target_os = "linux") { "<4>" } else { "" };

/// Says how a pass that took `seconds` went, at `when`, and how it went
/// wrong if it did, where people look: the journal or the Mac's log file,
/// since the guard's own journal is read only by the guard.
pub(super) fn say_pass(
    report: &crate::pass::Report,
    scan: &Scan,
    seconds: f64,
    when: &str,
) -> Option<crate::pass::Failing> {
    if let Some(line) = crate::pass::told(report, seconds, when) {
        say!("{line}");
    }
    if scan.went_unheard() {
        say!(
            "{WARNING_LEVEL}{}",
            crate::pass::unheard_line(scan.unheard(), when)
        );
    }
    let failing = crate::pass::failing(report);
    if let Some(failing) = &failing {
        say!(
            "{ERROR_LEVEL}{}",
            crate::pass::failed_line(failing, report, when)
        );
    }
    failing
}

/// Says no pass is at work in `passing` and in the status `beaten` keeps
/// for the beats to write.
pub(super) fn no_pass_at_work(passing: &mut Option<Passing>, beaten: &Mutex<Option<Status>>) {
    *passing = None;
    if let Some(status) = beaten
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        status.passing = None;
    }
}

/// Where the filesystem holding `path`, the pool `device`, is mounted:
/// the highest directory above it still on that pool, which is how a
/// person knows the disk, so a Btrfs pool is named by its topmost mount
/// rather than by whichever subvolume a source lies in. Failing that,
/// `path` itself.
pub(super) fn mounted(path: &Path, device: u64) -> PathBuf {
    let Ok(mut top) = fs::canonicalize(path) else {
        return path.to_owned();
    };
    while let Some(above) = top.parent()
        && pool(above).is_ok_and(|above| above == device)
    {
        top = above.to_owned();
    }
    top
}

impl Guard {
    /// Notes every filesystem a source or the store lies on, keeping what
    /// was seen of those already watched. A user daemon watches the store
    /// when it looks after an account's generations, which free it.
    pub(super) fn watch(&mut self, tick: &Tick) {
        let mut seen = BTreeMap::new();
        let store = (self.config.system.is_some() || self.config.nix.profiles.is_some())
            .then_some(self.config.nix.store.as_path());
        for path in self
            .config
            .sources
            .values()
            .flat_map(Source::under)
            .chain(store)
        {
            let (Ok(meta), Ok(device)) = (fs::metadata(path), pool(path)) else {
                continue;
            };
            if !meta.is_dir() || seen.contains_key(&device) {
                continue;
            }
            let watched = self.watched.remove(&device).unwrap_or_else(|| {
                let at = mounted(path, device);
                let rest = self.emptied_before(tick, &at);
                Watched {
                    path: at,
                    band: Band::Healthy,
                    fill: Fill::default(),
                    space: None,
                    rest,
                    backing: crate::pool::backed_by(path),
                }
            });
            seen.insert(device, watched);
        }
        self.watched = seen;
    }

    /// The hour of rest an earlier guard gave the disk mounted at `path`,
    /// left with nothing to clear, for what's left of it now. A rest
    /// that ended, or that reads as longer than an hour, as a wall clock
    /// set back would make it, is none.
    fn emptied_before(&self, tick: &Tick, path: &Path) -> Option<Rest> {
        let left = self.memory.emptied.get(path)? - tick.wall;
        (left > 0.0 && left <= EMPTY_REST).then_some(Rest {
            until: tick.monotonic + left,
            seconds: EMPTY_REST,
            band: Band::Filling,
            unvouched: false,
            exhausted: true,
            emptied: true,
        })
    }

    /// Puts off, at `now`, every pass `due` that walks every source while
    /// the last whole survey's spacing lasts, unless a disk is worse than
    /// filling, or forecast to be within a pass, whose pass walks for them
    /// all: a forecast's pass starts early so it's done before the band
    /// arrives, which a wait would undo. Each is due again at the next
    /// step. The spacing is kept across restarts, as a switch restarts the
    /// guard, and a wall clock set back ends it.
    pub(super) fn space_surveys(&self, now: f64, due: &mut Vec<(u64, Decision, Thresholds)>) {
        let Some((ended, seconds)) = self.memory.surveyed else {
            return;
        };
        if !(0.0..survey_spacing(seconds)).contains(&(now - ended)) {
            return;
        }
        // Builds an owner left go within a minute, however soon after the
        // last survey; only upkeep that's merely due again waits.
        if self.spent_due {
            return;
        }
        let pressing = due.iter().any(|&(device, decision, _)| {
            self.watched
                .get(&device)
                .is_some_and(|watched| pressed(decision, watched.band))
        });
        if !pressing {
            due.retain(|&(_, decision, _)| decision == Decision::Collect);
        }
    }

    /// Whether what's freed on the watched filesystem `on` is freed on
    /// `device` too, as an image's builds are on the disk it's a file on.
    pub(super) fn backs(&self, device: u64, on: u64) -> bool {
        self.watched
            .get(&on)
            .is_some_and(|watched| watched.backing == Some(device))
    }

    /// Where the images on `device` that `here` offers from are mounted.
    pub(super) fn images(&self, device: u64, here: &[Offer]) -> Vec<PathBuf> {
        let images: BTreeSet<u64> = here
            .iter()
            .map(|offer| offer.device)
            .filter(|&on| on != device)
            .collect();
        images
            .iter()
            .filter_map(|on| self.watched.get(on).map(|watched| watched.path.clone()))
            .collect()
    }

    /// Whether an offer on an image is gone already: its builds are offered
    /// both to its own disk's pass and to the one beneath it, and the first
    /// of the two in a round may have taken it.
    pub(super) fn taken(&self, offer: &Offer) -> bool {
        self.watched
            .get(&offer.device)
            .is_some_and(|watched| watched.backing.is_some())
            && crate::survey::gone(&offer.candidate, &self.tried)
    }

    /// Keeps what the next reach on `device` skips, by how the pass for
    /// `decision` there `end`ed, as `pass::skipped_after` says.
    pub(super) fn note_reached_short(&mut self, device: u64, decision: Decision, end: &End) {
        let skipped = self.reached_short.remove(&device);
        if let Some(skipped) = crate::pass::skipped_after(skipped, decision, end, &self.tried) {
            self.reached_short.insert(device, skipped);
        }
    }

    /// Walks every source, keeping when the walk ended and how long it
    /// took, for the next one's spacing, and the survey for a plan to read.
    /// One a stop or a failure cut short is spaced by what it walked too:
    /// a switch is likelier than not to land in a half-hour walk, and the
    /// guard it starts would otherwise walk again at once. Its memory is
    /// saved then, as the step it ends saves nothing; not over a damaged
    /// one, which the next full save sets aside first.
    fn survey_whole(&mut self, tick: &Tick) -> io::Result<(Scan, Vec<Offer>, Walked)> {
        let started = Instant::now();
        let (scan, offers, walked) = self.offers(tick, None).inspect_err(|_| {
            let ended = tick.wall + self.stepped.elapsed().as_secs_f64();
            self.memory.surveyed = Some((ended, started.elapsed().as_secs_f64()));
            if !self.damaged
                && let Err(error) = crate::state::save(&self.memory_path, &self.memory)
            {
                say!("disk-guard: keeping when a survey cut short ended: {error}");
            }
            self.end_passing();
        })?;
        let ended = tick.wall + self.stepped.elapsed().as_secs_f64();
        self.memory.surveyed = Some((ended, walked.seconds));
        // Kept for a plan to read; one that can't be kept only means the
        // next plan surveys for itself.
        let kept = crate::surveyed::Surveyed::new(tick.wall, &offers);
        if let Err(error) = crate::surveyed::save(self.state(), &kept) {
            say!("disk-guard: keeping the survey: {error}");
        }
        Ok((scan, offers, walked))
    }

    /// Surveys once and runs each filesystem's pass over what lies on it.
    pub(super) fn passes(
        &mut self,
        tick: &Tick,
        due: &[(u64, Decision, Thresholds)],
    ) -> io::Result<()> {
        self.tried.clear();
        // Garbage alone needs no survey, which walks every source.
        let (mut scan, offers, surveyed) = if due
            .iter()
            .all(|(_, decision, _)| *decision == Decision::Collect)
        {
            let me = std::process::id();
            let scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
            (scan, self.garbage_offers(tick), None)
        } else {
            // A survey can walk for most of an hour before any judging, so
            // the first disk it's for is told as at work from its start.
            let first = due.iter().find_map(|&(device, decision, _)| {
                let watched = self.watched.get(&device)?;
                (decision != Decision::Collect).then(|| (watched.path.clone(), aim(decision, None)))
            });
            if let Some((path, aimed)) = first {
                self.tell_passing(tick, &path, aimed.band, aimed.why);
            }
            let (scan, offers, walked) = self.survey_whole(tick)?;
            (scan, offers, Some(walked))
        };
        for &(device, decision, thresholds) in due {
            if !self.watched.contains_key(&device) {
                continue;
            }
            // A disk short of room is offered too what an image on it holds
            // that the band would take: on one laptop on 12 November the
            // cargo build disk held 28 GiB of the system disk, and its own
            // passes came only once the image itself filled.
            let reaching = matches!(decision, Decision::Reach(_));
            // Only a reach that its length can cut skips what a cut one
            // judged: one at a worse band runs on, sees every offer, and so
            // never calls a disk exhausted over offers it wasn't shown.
            let judged = match decision {
                Decision::Reach(band) if band <= Band::Filling => self.reached_short.get(&device),
                _ => None,
            };
            let here: Vec<Offer> = offers
                .iter()
                .filter(|offer| {
                    offer.device == device || reaching && self.backs(device, offer.device)
                })
                .filter(|offer| !self.taken(offer))
                .cloned()
                .collect();
            let afresh = crate::pass::judged_all(&here, judged, |offer| &offer.candidate.path);
            let here = crate::pass::left_to_judge(&here, judged, |offer| &offer.candidate.path);
            if afresh {
                self.reached_short.remove(&device);
            }
            let offered = if decision == Decision::Collect {
                self.collect_offers(tick.wall)
            } else {
                Vec::new()
            };
            let aimed = aim(decision, surveyed);
            let end = self.pass_on(tick, device, &here, aimed, thresholds, &mut scan);
            if decision == Decision::Collect {
                self.rest_collect(tick.wall + self.stepped.elapsed().as_secs_f64(), &offered);
            }
            self.note_reached_short(device, decision, &end);
            let Decision::Reach(band) = decision else {
                continue;
            };
            let unheard = scan.went_unheard();
            // From when the pass ended, on the step's clock: one ran 24
            // minutes on 25 November, and a rest counted from its start was
            // over before it ended.
            let since = self.stepped.elapsed().as_secs_f64();
            let ended = tick.monotonic + since;
            if let Some(watched) = self.watched.get_mut(&device) {
                let unvouched = unheard && !watched.rest.is_some_and(|rest| rest.unvouched);
                // A pass that gave way to a disk shorter of room, to a
                // stop or to its own length fell short of nothing; its disk
                // is due again.
                let fell_short = !matches!(end, End::Reached | End::Stopped | End::Unfinished);
                // One that couldn't ask the system daemon found nothing
                // left only for want of its word, which comes back in a
                // minute or two.
                let exhausted = matches!(end, End::Exhausted) && !unheard;
                // Filling only: a healthy disk's rest goes at its next
                // measure, so an hour promised there would never hold. In
                // a row only when its rest ended within the longest rest
                // before this pass: one after hours of holding off judges
                // what those hours built.
                let emptied = exhausted
                    && watched.rest.is_some_and(|rest| {
                        rest.exhausted && tick.monotonic - rest.until <= longest_rest(band)
                    })
                    && band <= Band::Filling
                    && watched.band == Band::Filling;
                watched.rest = fell_short.then(|| {
                    let seconds = if emptied {
                        EMPTY_REST
                    } else {
                        watched.rest.map_or(FIRST_REST, |rest| {
                            (rest.seconds * 2.0).min(longest_rest(band))
                        })
                    };
                    Rest {
                        until: ended + seconds,
                        seconds,
                        band,
                        unvouched,
                        exhausted,
                        emptied,
                    }
                });
                if emptied {
                    let until = tick.wall + since + EMPTY_REST;
                    self.memory.emptied.insert(watched.path.clone(), until);
                } else {
                    self.memory.emptied.remove(&watched.path);
                }
            }
        }
        Ok(())
    }

    /// Tells the status a pass is at work on `path` for `why` in `band`,
    /// from when the first of its steps began, its survey or its judging,
    /// and writes it, telling none if the write fails. The pass goes on
    /// whatever the write did: a disk too full for the status file is the
    /// one most in need of its pass, and saying so is only a courtesy.
    pub(super) fn tell_passing(&mut self, tick: &Tick, path: &Path, band: Band, why: Why) {
        let since = self
            .passing
            .as_ref()
            .filter(|passing| passing.path == path)
            .map_or_else(wall_now, |passing| passing.since);
        self.passing = Some(Passing {
            since,
            path: path.to_owned(),
            band,
            why: why.word().to_owned(),
        });
        if let Err(error) = self.write_status(tick.wall) {
            say!("disk-guard: telling the pass at work: {error}");
            self.end_passing();
        }
    }

    /// Says no pass is at work, in the status the beats write as well, so
    /// a step that fails before its next status leaves none told.
    pub(super) fn end_passing(&mut self) {
        no_pass_at_work(&mut self.passing, &self.beaten);
    }

    /// Ends the telling of a clear and writes the status again, since a
    /// held disk's waiting is judged by whether a pass is told on it: the
    /// status the clear left says it neither waits nor clears.
    pub(super) fn stop_telling(&mut self, tick: &Tick) {
        self.end_passing();
        if let Err(error) = self.write_status(tick.wall) {
            say!("disk-guard: writing the status after a clear: {error}");
        }
    }

    /// Runs a pass over what lies on the watched filesystem `device` and
    /// journals it.
    pub(super) fn pass_on(
        &mut self,
        tick: &Tick,
        device: u64,
        here: &[Offer],
        Aim {
            band,
            goal,
            why,
            surveyed,
        }: Aim,
        thresholds: Thresholds,
        scan: &mut Scan,
    ) -> End {
        let Some((path, spaced)) = self.watched.get(&device).map(|watched| {
            let forecast = matches!(why, Why::Reach).then_some(band);
            let spaced = crate::band::spaces_deletes(watched.band, forecast);
            (watched.path.clone(), spaced)
        }) else {
            return End::Exhausted;
        };
        // Told before it starts, since each beat as it works writes the
        // status again as it was, and a pass can judge for hours.
        self.tell_passing(tick, &path, band, why);
        let candidates: Vec<_> = here.iter().map(|offer| offer.candidate.clone()).collect();
        let started = Instant::now();
        let walk = Walk::start();
        let since = self.stepped;
        let gentle = band < Band::NearlyFull;
        let pacer = Rc::new(RefCell::new(self.pacer(&path, band, thresholds)));
        let images = self.images(device, here);
        let reclaimer = Reclaim::new(here, scan)
            .during(band)
            .spacing_deletes(
                spaced,
                self.deletes_resting(tick.wall + since.elapsed().as_secs_f64()),
            )
            .paced(Rc::clone(&pacer))
            .giving_way(self.elsewhere(device));
        let mut reclaim = Journaled {
            inner: if gentle {
                reclaimer.gently()
            } else {
                reclaimer
            },
            journal: &mut self.journal,
            roots: here
                .iter()
                .map(|offer| (offer.candidate.path.clone(), offer.action.root().to_owned()))
                .collect(),
            at: tick.wall,
            since,
            band,
        };
        // A pass that can wait waits for the filesystem's own commits; one
        // that can't asks for them. Either beats while it waits.
        let disk = Filesystem::new(&path)
            .after(images)
            .paced(Rc::clone(&pacer));
        let mut disk = if gentle { disk.patient() } else { disk };
        let report = pass(
            &mut disk,
            &mut reclaim,
            &candidates,
            &thresholds,
            band,
            goal,
            &mut self.memory.learned,
        );
        // Only these two fields, as what reclaims still holds the journal.
        no_pass_at_work(&mut self.passing, &self.beaten);
        self.refused.extend(reclaim.inner.take_refused());
        let deletes = reclaim.inner.deletes();
        self.rest_deletes(tick, deletes);
        self.learn_from_pass(tick, (goal, why), &report, scan, started);
        self.note_actions(here, &report.actions, tick.wall);
        let end = ending(&report.end);
        let seconds = started.elapsed().as_secs_f64();
        let failing = say_pass(&report, scan, seconds, &crate::clock::stamp(tick.wall));
        // A journal on a full disk is told of and the step goes on, as each
        // action's line already does, so the other disks still get their
        // passes and this one's rest is still noted.
        let line = Line::Pass {
            at: tick.wall,
            band: report.band,
            end: end.clone(),
            freed: report.freed,
            seconds,
            why: why.word(),
            offered: here.len(),
            judged: report.actions.len(),
            surveyed: surveyed.map(|walked| walked.seconds),
            read: walk.end().read,
            survey_read: surveyed.and_then(|walked| walked.read),
        };
        if let Err(error) = self.journal.write(&line) {
            say!("disk-guard: journalling a pass: {error}");
        }
        let last = LastPass {
            at: tick.wall,
            path,
            band: report.band,
            end,
            freed: report.freed,
            failing,
        };
        if last.failing.is_some() {
            self.failed = Some(last.clone());
        } else if self.failed.as_ref().is_some_and(|failed| {
            failed.path == last.path && (last.freed > 0 || last.band < Band::Filling)
        }) {
            self.failed = None;
        }
        self.last = Some(last);
        // After the pass's own line, and told of rather than returned when
        // refused, as a full disk's journal does: the pass has done its
        // work, so they cost it neither its record nor the rest of the step.
        self.journal_replaced(here, &report, tick.wall);
        report.end
    }

    /// The pacer for a pass in `band` on the filesystem at `path`. Until
    /// the disk is nearly full it rests between batches, feeling the disk through a probe
    /// beside the guard's state when that lies on the same filesystem, and
    /// hurries once the disk becomes nearly full; from then on it never
    /// rests. Either keeps the status file's heartbeat.
    pub(super) fn pacer(&self, path: &Path, band: Band, thresholds: Thresholds) -> Pacer {
        let clock = Box::new(Monotonic::new());
        if band >= Band::NearlyFull {
            return Pacer::hurried(clock, self.beating());
        }
        let watched = path.to_owned();
        let pressed = move || {
            crate::sys::space(&watched)
                .is_ok_and(|space| thresholds.band(&space, band) >= Band::NearlyFull)
        };
        Pacer::paced(
            self.config.pacing,
            clock,
            self.probe(path),
            self.beating(),
            Box::new(pressed),
        )
    }

    /// A probe in the guard's state directory, when that lies on the same
    /// filesystem as `path`. Only on Linux: a Mac's sync flushes the drive's
    /// own cache, which a probe once a second has no business doing.
    pub(super) fn probe(&self, path: &Path) -> Option<Box<dyn Probe>> {
        if cfg!(not(target_os = "linux")) {
            return None;
        }
        let state = self.memory_path.parent()?;
        if pool(state).ok()? != pool(path).ok()? {
            return None;
        }
        match crate::pace::SyncedWrite::open(state, "probe") {
            Ok(probe) => Some(Box::new(probe)),
            Err(error) => {
                say!("disk-guard: opening its probe: {error}");
                None
            }
        }
    }
}
