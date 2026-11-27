//! Writers that run away with a disk: measuring who holds it, freezing
//! a runaway and thawing it once there is room.

use super::surveying::{pinned_trees, spent_trees};
use super::{Guard, WRITING_WINDOW, Watched};
use crate::band::Band;
use crate::clock::Tick;
use crate::config::Source;
use crate::disk_reads::Walk;
use crate::journal::{Journal, Line};
use crate::pass::Outcome;
use crate::pool::pool;
use crate::state::{Memory, Owners};
use crate::survey::named;
use crate::writers;
use crate::writers::{Frozen, Writers};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, PoisonError};
use std::{fs, io};

/// The least a writer must write, in bytes a second, to be what fills a
/// disk once critically full: anything slower isn't worth stopping.
pub(super) const RUNAWAY: f64 = 1024.0 * 1024.0;

/// Whether process `pid` has a file open on the filesystem `device`.
pub(super) fn writes_on(processes: &Path, pid: u32, device: u64) -> bool {
    let Ok(fds) = fs::read_dir(processes.join(pid.to_string()).join("fd")) else {
        return false;
    };
    fds.filter_map(Result::ok)
        .any(|fd| pool(&fd.path()).is_ok_and(|pool| pool == device))
}

/// Thaws each frozen process whose disk has room again, and lets go of one
/// somebody thawed already, journaling either; the rest stay frozen.
pub(super) fn thaw(
    freeze: &mut dyn crate::freeze::Freeze,
    watched: &BTreeMap<u64, Watched>,
    memory: &mut Memory,
    journal: &mut Journal,
    tick: &Tick,
) -> io::Result<()> {
    let band_of = |path: &Path| {
        watched
            .values()
            .find(|watched| watched.path == path)
            .map(|watched| watched.band)
    };
    let mut still = Vec::new();
    for frozen in std::mem::take(&mut memory.frozen) {
        let roomy = band_of(&frozen.path).is_none_or(|band| band < Band::NearlyFull);
        let (kept, outcome) = if roomy {
            match freeze.thaw(&frozen.unit) {
                Ok(()) => (false, Outcome::Reclaimed),
                Err(why) => (true, Outcome::Failed(why)),
            }
        } else if let Ok(true) = freeze.frozen(&frozen.unit) {
            (true, Outcome::Reclaimed)
        } else {
            // Thawed by somebody else, or gone: theirs to decide.
            memory.spared.push((frozen.pid, frozen.start));
            (false, Outcome::Kept("thawed already".to_owned()))
        };
        if !kept || roomy {
            journal.write(&Line::Action {
                at: tick.wall,
                band: band_of(&frozen.path).unwrap_or(Band::Healthy),
                path: named(&frozen.path, &format!("{}:{}", frozen.name, frozen.pid)),
                kind: "thaw".to_owned(),
                size: 0,
                outcome,
            })?;
        }
        if kept {
            still.push(frozen);
        }
    }
    memory.frozen = still;
    Ok(())
}

/// How often the guard looks for an owner that has ended: a tree's last
/// holder, a quest finished or parked, a boot, or a guard with new rules.
/// Reading every quest's README and a few processes takes milliseconds, and
/// a minute keeps what an owner leaves from outliving it by much more.
pub(super) const OWNERS_EVERY: f64 = 60.0;

impl Guard {
    /// Starts measuring what holds the disks, when one is below healthy or
    /// holds more than its budget, and a measure is due, the disks having
    /// moved since the last or it being hours old, unless one still runs.
    pub(super) fn measure_holders(&mut self, now: f64) {
        if self
            .measuring
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return;
        }
        if let Some(thread) = self.measuring.take()
            && let Ok(walked) = thread.join()
        {
            // Stamped when it began, as every other walk is, so it sorts
            // beside the pass it ran with rather than at this later step.
            let at = self.began.as_mut().map_or(now, |began| {
                began.took = walked.seconds;
                began.at
            });
            self.journal_walk(at, "holders", walked);
        }
        let budget = self.config.budget;
        let pressed = self.watched.values().any(|watched| {
            watched.band != Band::Healthy
                || budget
                    .zip(watched.space.as_ref())
                    .is_some_and(|(budget, space)| {
                        space.total.saturating_sub(space.free) > budget.of(space.total)
                    })
        });
        let used = self
            .watched
            .iter()
            .filter_map(|(disk, watched)| {
                let space = watched.space.as_ref()?;
                Some((*disk, space.total.saturating_sub(space.free)))
            })
            .collect();
        if !pressed || !(self.remeasure || crate::holders::due(self.began.as_ref(), now, &used)) {
            return;
        }
        self.remeasure = false;
        self.began = Some(crate::holders::LastMeasure {
            at: now,
            used,
            took: 0.0,
        });
        let places = crate::holders::places(&self.config);
        let measured = Arc::clone(&self.measured);
        let state = self.state().to_owned();
        let started = std::thread::Builder::new()
            .name("holders".to_owned())
            .spawn(move || {
                // At idle priority, so sizing never slows what the person
                // or a build does. A Mac's only way lowers the whole
                // process, passes too, so there it runs as it is.
                #[cfg(target_os = "linux")]
                crate::commands::idle();
                let walk = Walk::start();
                let sized = crate::holders::measure(places);
                // A ledger that won't write leaves the holders unsaid only
                // in how much they grew.
                let grew = crate::ledger::record(&state, now, &sized).unwrap_or_else(|error| {
                    say!("disk-guard: keeping the ledger: {error}");
                    crate::ledger::Grown::new()
                });
                *measured.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, sized, grew));
                walk.end()
            });
        match started {
            Ok(thread) => self.measuring = Some(thread),
            Err(error) => say!("disk-guard: measuring what holds the disks: {error}"),
        }
    }

    /// Freezes what's filling a critically full disk with nothing left to
    /// take, and thaws what was frozen once its disk has room again, or
    /// lets it go when somebody thawed it by hand. Only the account's own
    /// processes, outside its sessions, and never the guard.
    pub(super) fn runaways(&mut self, tick: &Tick) -> io::Result<()> {
        // Who writes is sampled while any disk is short, freezing or not,
        // so the status and the notices can name what fills one.
        let pressed = self
            .watched
            .values()
            .any(|watched| watched.band > Band::Healthy);
        let samples = if pressed {
            // SAFETY: geteuid only reads the process's own credentials.
            let me = unsafe { libc::geteuid() };
            let samples: Vec<_> = writers::sample(&self.processes)
                .into_iter()
                .filter(|sample| sample.uid == me)
                .collect();
            self.writers.observe(samples.clone(), tick.wall);
            Some(samples)
        } else {
            self.writers = Writers::default();
            None
        };
        let Some(freeze) = self.freeze.as_mut() else {
            return Ok(());
        };
        // First what's frozen: thawed with room, let go when thawed already.
        thaw(
            freeze.as_mut(),
            &self.watched,
            &mut self.memory,
            &mut self.journal,
            tick,
        )?;
        let Some(samples) = samples else {
            return Ok(());
        };
        // SAFETY: geteuid only reads the process's own credentials.
        let me = unsafe { libc::geteuid() };
        // A process spared is spared while it lives; a pid reused is another.
        self.memory.spared.retain(|&(pid, start)| {
            samples
                .iter()
                .any(|sample| sample.pid == pid && sample.start == start)
        });
        let Some(writer) = self.writers.dominant(WRITING_WINDOW, tick.wall) else {
            return Ok(());
        };
        let freezable = writer.rate >= RUNAWAY
            && writer.pid != std::process::id()
            && !self.memory.spared.contains(&(writer.pid, writer.start))
            && !self
                .memory
                .frozen
                .iter()
                .any(|frozen| frozen.pid == writer.pid)
            && writer.cgroup.as_deref().is_some_and(|cgroup| {
                cgroup.starts_with(&format!("/user.slice/user-{me}.slice/user@{me}.service/"))
                    && !cgroup.contains("/session.slice/")
                    && !cgroup.contains("disk-guard")
            });
        if !freezable {
            return Ok(());
        }
        // A critically full disk whose last pass found nothing left, and that
        // the writer has a file open on.
        let Some((path, band)) = self
            .watched
            .iter()
            .filter(|(_, watched)| {
                watched.band == Band::CriticallyFull
                    && watched
                        .rest
                        .is_some_and(|rest| rest.band == Band::CriticallyFull)
            })
            .find(|(device, _)| writes_on(&self.processes, writer.pid, **device))
            .map(|(_, watched)| (watched.path.clone(), watched.band))
        else {
            return Ok(());
        };
        let unit = crate::freeze::unit_for(writer.pid);
        let pids = writers::descendants(&samples, writer.pid);
        let description = format!(
            "{} (pid {}), frozen by disk-guard for filling {}",
            writer.name,
            writer.pid,
            path.display()
        );
        let outcome = match freeze.freeze(&unit, &pids, &description) {
            Ok(()) => {
                self.memory.frozen.push(Frozen {
                    unit,
                    pid: writer.pid,
                    start: writer.start,
                    name: writer.name.clone(),
                    path: path.clone(),
                    at: tick.wall,
                });
                Outcome::Reclaimed
            }
            Err(why) => Outcome::Failed(why),
        };
        self.journal.write(&Line::Action {
            at: tick.wall,
            band,
            path: named(&path, &format!("{}:{}", writer.name, writer.pid)),
            kind: "freeze".to_owned(),
            size: writer.written,
            outcome,
        })
    }

    /// Notes whether any writers' hold's mark stands, as another program
    /// leaves one while the disk's writes must wait for what Joel is using,
    /// and says so in the log as a hold begins and ends. Only its presence
    /// counts, never what it says.
    pub(super) fn heed_holds(&mut self) {
        let mark = self
            .config
            .writer_holds
            .iter()
            .find(|mark| fs::symlink_metadata(mark).is_ok());
        let held = mark.is_some();
        if held != self.writers_held {
            match mark {
                Some(mark) => say!(
                    "disk-guard: {} stands, so no pass, listing or collection starts short of \
                     nearly full",
                    mark.display()
                ),
                None => say!("disk-guard: the writers' hold is over"),
            }
        }
        self.writers_held = held;
    }

    /// Makes upkeep due at once when an owner has ended since the last look:
    /// a tree's last holder or the quest it was kept for, a boot, which
    /// ends every program and so everything they left in temp, or a guard
    /// with new rules, which may judge more spent. What an owner leaves is
    /// spent the moment it ends, so waiting six hours for routine upkeep
    /// only keeps bytes nobody will read. A producer whose watched files
    /// said something new, as a cache gaining a version ends an old one's
    /// use, is run on its own instead, since only it has news.
    pub(super) fn watch_owners(&mut self, tick: &Tick) {
        if self
            .owners_at
            .is_some_and(|at| tick.monotonic - at < OWNERS_EVERY)
        {
            return;
        }
        self.owners_at = Some(tick.monotonic);
        let Ok(boot) = crate::sys::boot(Path::new("/proc/stat")) else {
            return;
        };
        let processes = &self.processes;
        let running = |pid: u32, started: f64| {
            crate::sys::started(processes, pid).is_some_and(|ticks| {
                let since = boot.wall + crate::units::float(ticks) / boot.ticks;
                (since - started).abs() <= 2.0
            })
        };
        let seen = Owners {
            boot: tick.boot.clone(),
            guard: std::env::current_exe().ok(),
            spent: spent_trees(&self.config, running, (&tick.boot, &boot))
                .into_iter()
                .filter(|tree| tree.exists())
                .collect(),
        };
        self.produced = self
            .config
            .sources
            .iter()
            .filter(|(name, source)| match source {
                Source::Producer { watch, .. } => crate::survey::changed(
                    watch,
                    self.memory.ran.get(*name).copied(),
                    self.memory.said.get(*name).copied(),
                ),
                _ => false,
            })
            .map(|(name, _)| name.clone())
            .collect();
        // A snapshot cut since the last look, or every one at the guard's
        // start, may hold a build no survey has timed yet, so it's looked
        // at again an hour on.
        let snapshots = pinned_trees(&self.config);
        let cut = self
            .snapshots_known
            .as_ref()
            .is_none_or(|known| snapshots.iter().any(|tree| !known.contains(tree)));
        if cut && !snapshots.is_empty() {
            let at = tick.wall + crate::trees::SNAPSHOT_UNUSED;
            self.snapshot_spends = Some(self.snapshot_spends.map_or(at, |soon| soon.min(at)));
        }
        self.snapshots_known = Some(snapshots);
        // A snapshot's build an hour unused is spent as surely as one whose
        // holder ended, though nothing about its holder changed.
        let snapshot_spent = self.snapshot_spends.is_some_and(|at| tick.wall >= at);
        if snapshot_spent {
            self.snapshot_spends = None;
            self.upkeep_due();
            self.spent_due = true;
        }
        // A builds disk passing its budget changes no holder, and on a
        // healthy root nothing else would bring upkeep for six hours.
        let looked = self
            .budget_looked
            .is_some_and(|at| tick.wall - at < crate::survey::BUDGET_LOOKS_EVERY);
        if !looked && crate::survey::past_budget(&self.config) {
            self.budget_looked = Some(tick.wall);
            self.upkeep_due();
        }
        let ended = self.memory.owners.as_ref().is_none_or(|before| {
            before.boot != seen.boot
                || before.guard != seen.guard
                || seen.spent.iter().any(|tree| !before.spent.contains(tree))
        });
        // A guard's own restart makes upkeep due too, but spends nothing, so
        // only trees spent since, or a new boot, go past a survey's spacing.
        let spent = self.memory.owners.as_ref().is_some_and(|before| {
            before.boot != seen.boot || seen.spent.iter().any(|tree| !before.spent.contains(tree))
        });
        if ended {
            self.upkeep_due();
            self.spent_due |= spent;
        }
        self.memory.owners = Some(seen);
    }
}
