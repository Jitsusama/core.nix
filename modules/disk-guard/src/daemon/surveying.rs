//! Surveys: walking a disk's sources for offers, and learning from what
//! a pass did with them.

use super::{Guard, Why};
use crate::band::Thresholds;
use crate::clock::Tick;
use crate::config::{Config, Source};
use crate::disk_reads::{Walk, Walked};
use crate::host::{Elsewhere, FRESH, Scan};
use crate::journal::Line;
use crate::pass::{End, Goal, Outcome, Report};
use crate::plan::Class;
use crate::survey::{Action, Offer, World, survey};
use crate::{finished, garbage};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{fs, io};

/// The weight each timed pass carries against those before it.
pub(super) const PASS_WEIGHT: f64 = 0.3;

/// How soon upkeep comes again when it kept something only the system
/// daemon could have vouched for, and couldn't be asked. A switch restarts
/// both daemons, so the user's upkeep can begin while the system's is down;
/// ten minutes lets it come back. Only once in a row: on 27 November the
/// system daemon, busy with its own passes, went unheard upkeep after
/// upkeep, and each came back for another walk of 15 GB.
pub(super) const UNHEARD_AGAIN: f64 = 600.0;

/// The trees whose builds are spent: every tree a finished quest lists,
/// every tree whose broker record names no holder still `running`, unless a
/// quest at work lists it, and every tree a quest lists while none of its
/// sessions is `running` in this boot, `boot`, whose clock is `clock`.
pub(super) fn spent_trees(
    config: &Config,
    running: impl Fn(u32, f64) -> bool + Copy,
    (boot, clock): (&str, &crate::snapshots::Boot),
) -> Vec<PathBuf> {
    // A session keeps when it started in clock ticks since boot.
    let alive =
        |pid: u32, ticks: u64| running(pid, clock.wall + crate::units::float(ticks) / clock.ticks);
    config
        .sources
        .values()
        .filter_map(|source| match source {
            Source::Workspaces {
                quests,
                trees,
                sessions,
                rosters,
                ..
            } => Some((quests, trees, sessions, rosters)),
            _ => None,
        })
        .flat_map(|(quests, trees, sessions, rosters)| {
            let unheld = trees.as_deref().map_or_else(Vec::new, |records| {
                crate::workspaces::unheld_trees(records, quests, rosters.as_deref(), running)
            });
            let idle = sessions.as_deref().map_or_else(Vec::new, |sessions| {
                crate::workspaces::idle_trees(quests, sessions, rosters.as_deref(), boot, alive)
            });
            crate::workspaces::finished_trees(quests)
                .into_iter()
                .chain(unheld)
                .chain(idle)
        })
        .collect()
}

/// The trees a broker's record names a holder of still `running`, which
/// nothing finishes while it holds them: on 12 November a session merged
/// its branch and went on in the tree its broker recorded it holding, and
/// twice the guard took the tree's packages within the half hour after.
pub(super) fn held_trees(
    config: &Config,
    running: impl Fn(u32, f64) -> bool + Copy,
) -> Vec<PathBuf> {
    config
        .sources
        .values()
        .filter_map(|source| match source {
            Source::Workspaces {
                trees: Some(records),
                rosters,
                ..
            } => Some((records, rosters)),
            _ => None,
        })
        .flat_map(|(records, rosters)| {
            crate::workspaces::held_trees(records, rosters.as_deref(), running)
        })
        .collect()
}

/// When the soonest build offered in a pinned snapshot, not yet spent, or
/// the soonest older build there, will have gone an hour unused. One a
/// build ran in or a session sat in, which the survey kept or couldn't
/// spend, is looked at again an hour on, since it may be done by then.
pub(super) fn snapshot_spends(
    offers: &[Offer],
    kept: &[(PathBuf, crate::activity::Use)],
    snapshots: &[PathBuf],
    now: f64,
) -> Option<f64> {
    let pinned = |dir: &Path| snapshots.iter().any(|tree| dir == tree);
    let hour = crate::trees::SNAPSHOT_UNUSED;
    let offered = offers
        .iter()
        .map(|offer| &offer.candidate)
        .filter(|candidate| {
            matches!(
                candidate.class,
                Class::Target | Class::Install | Class::Replaced
            ) && !candidate.spent
                && candidate.checkout.as_deref().is_some_and(pinned)
        })
        .map(|candidate| {
            if candidate.occupied {
                now + hour
            } else {
                now + (hour - candidate.idle).max(0.0)
            }
        });
    let busy = kept
        .iter()
        .filter(|(tree, _)| snapshots.iter().any(|snapshot| tree.starts_with(snapshot)))
        .map(|_| now + hour);
    offered.chain(busy).reduce(f64::min)
}

/// The trees a broker's records name as snapshots pinned at a commit,
/// whose builds a gate or a reader made and nobody edits.
pub(super) fn pinned_trees(config: &Config) -> Vec<PathBuf> {
    config
        .sources
        .values()
        .filter_map(|source| match source {
            Source::Workspaces {
                trees: Some(records),
                ..
            } => Some(records),
            _ => None,
        })
        .flat_map(|records| crate::workspaces::pinned_trees(records))
        .collect()
}

/// How many replaced builds a pass took from each target among `here`'s
/// offers, and their bytes.
pub(super) fn replaced_taken(
    here: &[Offer],
    actions: &[(PathBuf, Outcome)],
) -> BTreeMap<PathBuf, (u64, u64)> {
    let mut taken: BTreeMap<PathBuf, (u64, u64)> = BTreeMap::new();
    for (path, outcome) in actions {
        if *outcome != Outcome::Reclaimed {
            continue;
        }
        let offered = here.iter().find(|offer| &offer.candidate.path == path);
        if let Some(Offer {
            candidate,
            action: Action::Build { target, .. },
            ..
        }) = offered
        {
            let (count, bytes) = taken.entry(target.clone()).or_default();
            *count += 1;
            *bytes += candidate.size;
        }
    }
    taken
}

/// Whether a pass says how long reaching for room takes. One that ends
/// waiting or at a stop ends in moments, not when its work is done, and so
/// does one that reclaimed nothing, as on a disk with nothing to clear, a
/// forecast's past its target, or one whose every find was kept: learned,
/// each drew the forecast's lead toward its floor. A reach that deleted a
/// batch and then waits is left out too, on purpose: it took real time,
/// but not the time its work needs, and so is one its length cut short,
/// which would draw the lead toward that length. An asked collection
/// reaches only for generations and garbage, so it says nothing of a
/// whole reach.
pub(super) fn teaches_pass_length(goal: Goal, why: Why, report: &Report) -> bool {
    goal == Goal::Reach
        && !matches!(why, Why::Collect)
        && report
            .actions
            .iter()
            .any(|(_, outcome)| matches!(outcome, Outcome::Reclaimed))
        && !matches!(report.end, End::Waiting | End::Stopped | End::Unfinished)
}

/// Whether a delete failed for garbage that came alive since the listing.
pub(super) fn stale(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Failed(why) if garbage::stale(why))
}

/// When upkeep was last done, and whether it went unheard, once one more
/// upkeep pass of a step at `wall` ends: `retry`, so it comes again soon,
/// when the pass `kept` what only an unheard system daemon could vouch
/// for, unless the upkeep `before` the step had gone unheard already, and
/// `wall` otherwise. None leaves the retry an earlier pass of the step set.
pub(super) fn upkept_after(
    upkept: Option<f64>,
    (wall, retry): (f64, f64),
    before: bool,
    kept: bool,
) -> Option<(f64, bool)> {
    if upkept == Some(retry) {
        return None;
    }
    let again = kept && !before;
    Some((if again { retry } else { wall }, again))
}

impl Guard {
    /// What reads this machine's processes for `me`, vouched for by the
    /// system daemon under every source's roots when there is one.
    pub(super) fn scan(&self, me: u32) -> Scan {
        let scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
        let Some(socket) = &self.system else {
            return scan;
        };
        let roots: Vec<PathBuf> = self
            .config
            .sources
            .values()
            .flat_map(Source::under)
            .map(Path::to_owned)
            .collect();
        scan.vouched_by(socket, &roots)
    }

    /// Every process, scanned for the pass to judge by, and what every
    /// source offers.
    /// What the sources offer now, or with `part`, only what its sources
    /// offer, for a clear or a producer with news: a survey of that part
    /// leaves what the holders say of the rest as the last whole one did.
    /// With it, how long the survey took and what it read from disk, for
    /// each pass it serves to journal, since a pass's own leave out this
    /// walk.
    pub(super) fn offers(
        &mut self,
        tick: &Tick,
        part: Option<&Config>,
    ) -> io::Result<(Scan, Vec<Offer>, Walked)> {
        let walk = Walk::start();
        let (scan, offers) = self.survey(tick, part)?;
        Ok((scan, offers, walk.end()))
    }

    /// What `offers` answers, untimed.
    pub(super) fn survey(
        &mut self,
        tick: &Tick,
        part: Option<&Config>,
    ) -> io::Result<(Scan, Vec<Offer>)> {
        let me = std::process::id();
        let mut scan = self.scan(me);
        let processes = scan.processes()?.to_vec();
        self.unread = (
            processes.iter().filter(|process| !process.readable).count(),
            scan.unheard().map(str::to_owned),
        );
        let boot = crate::sys::boot(Path::new("/proc/stat"))?;
        let uses = self.uses_now();
        let (predicates, answers) = (&self.predicates, &mut self.answers);
        // A tree a finished quest cut is finished with it, whatever its
        // branch says: its builds were the quest's. So is one nobody holds
        // any more, its builds having been its holders'. A pid holds its
        // tree only while it runs as the process that started then; one
        // that can't be read can't be ruled out, so holds it. A tree held
        // is never finished, whatever its branch or its quest says.
        let running = |pid: u32, started: f64| {
            processes.iter().any(|process| {
                let since = boot.wall + crate::units::float(process.start_ticks) / boot.ticks;
                process.pid == pid && (!process.readable || (since - started).abs() <= 2.0)
            })
        };
        let quests_over = spent_trees(&self.config, running, (&tick.boot, &boot));
        let held = held_trees(&self.config, running);
        let snapshots = pinned_trees(&self.config);
        // A build's checkout is its sources' git top, which is the path a
        // snapshot's record names; a checkout nested inside one is its own.
        let pinned = |dir: &Path| snapshots.iter().any(|tree| dir == tree);
        let mut finished = |dir: &Path| {
            if held.iter().any(|tree| dir.starts_with(tree)) {
                return false;
            }
            if quests_over.iter().any(|tree| dir.starts_with(tree)) {
                return true;
            }
            let Some(checkout) = finished::read(dir) else {
                return false;
            };
            let Some(predicate) = finished::predicate_for(predicates, &checkout.remote) else {
                return false;
            };
            // Asked afresh each time, before what's remembered of its head:
            // a merged head stays merged while someone goes on editing.
            if finished::changed(dir) {
                return false;
            }
            answers.finished(dir, &checkout.head, tick.wall, || {
                finished::ask(predicate, dir, &checkout)
            })
        };
        let mut world = World {
            processes: &processes,
            me,
            // SAFETY: geteuid only reads the process's own credentials.
            uid: unsafe { libc::geteuid() },
            home: self.home.as_deref(),
            boot,
            awake: &self.memory.awake,
            now: tick.wall,
            finished: &mut finished,
            pinned: &pinned,
            ran: &self.memory.ran,
            said: &self.memory.said,
            uses: &uses,
            kept: Vec::new(),
            counted: std::cell::RefCell::default(),
            sizes: std::cell::RefCell::new(std::mem::take(&mut self.sizes)),
            walked: 0,
            stop: &*self.stop,
        };
        let mut offers = survey(part.unwrap_or(&self.config), &mut world);
        let kept = std::mem::take(&mut world.kept);
        let mut sizes = world.sizes.take();
        drop(world);
        // Only a whole survey asks for every size it would, so only one
        // may forget the rest.
        if part.is_none() && !(self.stop)() {
            sizes.ended();
            self.snapshot_spends = snapshot_spends(&offers, &kept, &snapshots, tick.wall);
        }
        self.sizes = sizes;
        // A walk a stop cut short found only part of what's there, and a
        // checkout it never reached would look gone, so nothing is taken.
        if (self.stop)() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "told to stop during the survey, so it takes nothing",
            ));
        }
        // Read afresh each survey, so a keep holds from the next step. One
        // that won't read stops the survey rather than risk what it keeps.
        // The configuration's own keeps count even before a daemon has
        // written them, so a plan made from it honours them too.
        let keeps =
            crate::keeps::with_declared(crate::keeps::load(self.state())?, &self.config.keep, 0.0);
        // A keep names its folder with every link resolved, and a source's
        // root may be named through one, so each offer is resolved too.
        offers.retain(|offer| {
            let path = &offer.candidate.path;
            let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            !crate::keeps::touches(&keeps, &resolved)
        });
        if part.is_some() {
            return Ok((scan, offers));
        }
        self.kept = kept;
        self.offered = Some(crate::holders::Offers::new(
            offers
                .iter()
                .map(|offer| crate::holders::Offered {
                    path: offer.candidate.path.clone(),
                    size: offer.candidate.size,
                    now: crate::plan::clears(&offer.candidate),
                })
                .collect(),
        ));
        offers.extend(self.generation_offers(tick));
        offers.extend(self.garbage_offers(tick));
        offers.extend(self.asked_offers());
        Ok((scan, offers))
    }

    /// The watched filesystems but `device`'s, as their last measure left
    /// them, for a pass there to give way to.
    pub(super) fn elsewhere(&self, device: u64) -> Vec<Elsewhere> {
        self.watched
            .iter()
            .filter(|&(&other, _)| other != device)
            .filter_map(|(_, watched)| {
                let space = watched.space?;
                Some(Elsewhere {
                    path: watched.path.clone(),
                    thresholds: Thresholds::new(&self.config.bands, space.total),
                    band: watched.band,
                })
            })
            .collect()
    }

    /// Whether a pass, with the system daemon unheard, kept something only
    /// it could vouch for: kept by the survey before it was offered, or by
    /// the pass.
    pub(super) fn kept_unvouched(&self, scan: &Scan, report: &crate::pass::Report) -> bool {
        scan.went_unheard()
            && (self
                .kept
                .iter()
                .any(|(_, used)| matches!(used, crate::activity::Use::Unknown(_)))
                || report
                    .actions
                    .iter()
                    .any(|(_, outcome)| matches!(outcome, Outcome::Kept(_))))
    }

    /// Makes upkeep due at the next spare moment, for a reason of its own,
    /// so the one it brings gets its own retry if it goes unheard, however
    /// the upkeep before it went.
    pub(super) fn upkeep_due(&mut self) {
        self.memory.upkept = None;
        self.memory.upkeep_unheard = false;
    }

    /// Learns what a pass that `started` teaches: how long a pass takes,
    /// from a reach that reclaimed, or when upkeep was last done, from an
    /// upkeep pass that got through what was due.
    pub(super) fn learn_from_pass(
        &mut self,
        tick: &Tick,
        (goal, why): (Goal, Why),
        report: &Report,
        scan: &Scan,
        started: Instant,
    ) {
        if teaches_pass_length(goal, why, report) {
            let seconds = started.elapsed().as_secs_f64();
            self.memory.pass_seconds = Some(match self.memory.pass_seconds {
                Some(before) => before + PASS_WEIGHT * (seconds - before),
                None => seconds,
            });
        } else if goal == Goal::Upkeep && !matches!(report.end, End::Unfinished | End::Stopped) {
            // Only a pass that got through what was due counts as upkeep:
            // one cut short by its length or a stop leaves the rest due.
            let retry = tick.wall - crate::decide::UPKEEP_EVERY + UNHEARD_AGAIN;
            // Every disk's upkeep in a step comes again if any of them kept
            // what only the unheard system daemon could vouch for, each
            // judged against the upkeep before the step, so a later disk's
            // keep isn't lost to an earlier disk that kept nothing, and a
            // retry that went unheard again isn't retried.
            let before = match self.upkeep_judged {
                Some((at, before)) if at.to_bits() == tick.wall.to_bits() => before,
                _ => self.memory.upkeep_unheard,
            };
            self.upkeep_judged = Some((tick.wall, before));
            self.spent_due = false;
            let kept = self.kept_unvouched(scan, report);
            if let Some((upkept, unheard)) =
                upkept_after(self.memory.upkept, (tick.wall, retry), before, kept)
            {
                self.memory.upkept = Some(upkept);
                self.memory.upkeep_unheard = unheard;
            }
        }
    }

    /// Journals a walk no pass took, `step` naming it, and tells of a
    /// journal that won't take it, as a pass's line does.
    pub(super) fn journal_walk(&mut self, at: f64, step: &'static str, walked: Walked) {
        let line = Line::Walk {
            at,
            step,
            seconds: walked.seconds,
            read: walked.read,
        };
        if let Err(error) = self.journal.write(&line) {
            say!("disk-guard: journalling a walk: {error}");
        }
    }

    /// Journals how many replaced builds a pass took from each target among
    /// `here`'s offers, and their bytes.
    pub(super) fn journal_replaced(&mut self, here: &[Offer], report: &Report, at: f64) {
        for (target, (count, bytes)) in replaced_taken(here, &report.actions) {
            let from = target.display().to_string();
            let line = Line::Replaced {
                at,
                band: report.band,
                target,
                count,
                bytes,
            };
            if let Err(error) = self.journal.write(&line) {
                say!("disk-guard: journalling the builds taken from {from}: {error}");
            }
        }
    }

    /// Notes what came of each action a pass took, whatever it was, so a
    /// failing command waits its window too.
    pub(super) fn note_actions(
        &mut self,
        here: &[Offer],
        actions: &[(PathBuf, Outcome)],
        wall: f64,
    ) {
        for (taken, outcome) in actions {
            self.tried.insert(taken.clone());
            if *outcome == Outcome::Reclaimed
                && let Some(offered) = &mut self.offered
            {
                offered.forget(taken);
            }
            let offered = here.iter().find(|offer| &offer.candidate.path == taken);
            match offered.map(|offer| &offer.action) {
                Some(Action::Run { source, .. }) => {
                    self.memory.ran.insert(source.clone(), wall);
                    if let Some(Source::Producer { watch, .. }) = self.config.sources.get(source)
                        && !watch.is_empty()
                    {
                        self.memory
                            .said
                            .insert(source.clone(), crate::survey::said(watch));
                    }
                }
                // What they kept is garbage now, which no listing holds.
                Some(Action::Expire { .. }) if *outcome == Outcome::Reclaimed => {
                    self.relist = true;
                }
                // The listing's other batches would meet the same, so the
                // store is listed again, a listing under way being new
                // enough, and none of them is offered meanwhile.
                Some(Action::Collect { .. }) if stale(outcome) => {
                    self.stale_listing = true;
                    if self.listing_out.is_none() {
                        self.relist = true;
                    }
                }
                _ => {}
            }
        }
    }
}
