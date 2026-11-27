//! Clears asked for: the offers a person's or a command's ask makes,
//! and the collects run for them.

use super::{Aim, GARBAGE, Guard, Why, ending, held_off};
use crate::band::{Band, Thresholds};
use crate::clock::Tick;
use crate::disk_reads::Walked;
use crate::host::{FRESH, Scan};
use crate::pass::{End, Goal};
use crate::plan::{Candidate, Class};
use crate::pool::pool;
use crate::socket::Reply;
use crate::survey::{Action, Offer, named};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

impl Guard {
    /// What a user daemon offers of the system daemon's garbage: on each
    /// filesystem it collects on, a collect asked of it, in garbage's turn
    /// once a disk is filling and measured like any command. Routine upkeep of
    /// garbage is the system daemon's own.
    pub(super) fn asked_offers(&self) -> Vec<Offer> {
        let Some(socket) = &self.system else {
            return Vec::new();
        };
        self.collected_on
            .iter()
            .filter_map(|&device| {
                let mount = &self.watched.get(&device)?.path;
                Some(Offer {
                    candidate: Candidate {
                        path: named(mount, GARBAGE),
                        class: Class::Garbage {
                            kept: true,
                            order: 0,
                        },
                        idle: 0.0,
                        size: 0,
                        occupied: false,
                        spent: false,
                        checkout: None,
                    },
                    action: Action::Ask {
                        root: mount.clone(),
                        socket: socket.clone(),
                    },
                    device,
                })
            })
            .collect()
    }

    /// Clears each folder Joel asked cleared, saying whether there were
    /// any. Each takes what the survey offers inside it that a clear takes
    /// (`crate::plan::clears`), judged for use again as it goes, paced by
    /// its disk's own band, so a clear on a healthy disk is a gentle one.
    /// An ask ends once its pass has, so until then the status says the
    /// folder is clearing.
    pub(super) fn clear_asked(&mut self, tick: &Tick) -> io::Result<bool> {
        let asks = crate::clears::asked(self.state())?;
        if asks.is_empty() {
            return Ok(false);
        }
        // Told as at work from before its survey, which walks for minutes:
        // under a writers' hold the disk otherwise says it waits, as if
        // the clear Joel asked for never started.
        // Disks are known by their pool, as a Btrfs subvolume's own device
        // number names none of them.
        let first = asks.iter().find_map(|(_, ask)| {
            let watched = self.watched.get(&pool(&ask.path).ok()?)?;
            Some((watched.path.clone(), watched.band))
        });
        if let Some((path, band)) = first {
            self.tell_passing(tick, &path, band, Why::Clear);
        } else if let Err(error) = self.write_status(tick.wall) {
            // Joel asks most as the disk fills, when the status may not
            // fit: the clear goes on, and so does every pass after it.
            say!("disk-guard: writing the status before a clear: {error}");
        }
        let folders: Vec<PathBuf> = asks.iter().map(|(_, ask)| ask.path.clone()).collect();
        let scoped = crate::survey::within(&self.config, &folders);
        let (mut scan, offers, surveyed) = self
            .offers(tick, Some(&scoped))
            .inspect_err(|_| self.stop_telling(tick))?;
        let mut ended = Ok(());
        for (file, ask) in asks {
            // A source's root may be named through a link, and an ask's
            // folder never is.
            let inside: Vec<Offer> = offers
                .iter()
                .filter(|offer| {
                    let path = &offer.candidate.path;
                    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                    resolved.starts_with(&ask.path) && crate::plan::clears(&offer.candidate)
                })
                .cloned()
                .collect();
            let cleared = self.clear_on(tick, &inside, &mut scan, surveyed);
            // Ended whatever the pass did, since one that failed would
            // fail again, each step surveying everything to.
            ended = crate::clears::done(&file).and(cleared);
            if ended.is_err() {
                break;
            }
        }
        // A clear that found nothing inside its folder ran no pass to end
        // the telling, and its disk would say it clears until the next.
        self.stop_telling(tick);
        ended?;
        self.remeasure = true;
        Ok(true)
    }

    /// Runs each producer whose watched files said something new, saying
    /// whether there were any. Only they are surveyed, so tec's cache
    /// moving on every 11 minutes no longer walks every source, as it
    /// did with each move on 29 October.
    pub(super) fn produce(&mut self, tick: &Tick) -> io::Result<bool> {
        // What they said waits for the hold's end, kept as news.
        if self.produced.is_empty() || self.writers_held {
            return Ok(false);
        }
        let mut producers = self.config.clone();
        let names = std::mem::take(&mut self.produced);
        producers.sources.retain(|name, _| names.contains(name));
        // Its news is already taken, so a failed write would lose it.
        if let Err(error) = self.write_status(tick.wall) {
            say!("disk-guard: writing the status before a producer: {error}");
        }
        let (mut scan, offers, surveyed) = self.offers(tick, Some(&producers))?;
        let devices: BTreeSet<u64> = offers.iter().map(|offer| offer.device).collect();
        for device in devices {
            let Some(watched) = self.watched.get(&device) else {
                continue;
            };
            let space = crate::sys::space(&watched.path)?;
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            let here: Vec<Offer> = offers
                .iter()
                .filter(|offer| offer.device == device)
                .cloned()
                .collect();
            let aim = Aim {
                band: Band::Healthy,
                goal: Goal::Produce,
                why: Why::Producers,
                surveyed: Some(surveyed),
            };
            self.pass_on(tick, device, &here, aim, thresholds, &mut scan);
        }
        Ok(true)
    }

    /// A clear's passes over `inside`, one on each filesystem it spans,
    /// offered by the survey `surveyed`.
    pub(super) fn clear_on(
        &mut self,
        tick: &Tick,
        inside: &[Offer],
        scan: &mut Scan,
        surveyed: Walked,
    ) -> io::Result<()> {
        let devices: BTreeSet<u64> = inside.iter().map(|offer| offer.device).collect();
        for device in devices {
            let Some(watched) = self.watched.get(&device) else {
                continue;
            };
            let band = watched.band;
            let space = crate::sys::space(&watched.path)?;
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            let here: Vec<Offer> = inside
                .iter()
                .filter(|offer| offer.device == device)
                .cloned()
                .collect();
            let aim = Aim {
                band,
                goal: Goal::Clear,
                why: Why::Clear,
                surveyed: Some(surveyed),
            };
            self.pass_on(tick, device, &here, aim, thresholds, scan);
        }
        Ok(())
    }

    /// Answers each collect asked for since the last step.
    pub(super) fn collects(&mut self, tick: &Tick) {
        let asked: Vec<_> = match &self.hearing {
            Some(hearing) => hearing.collects.try_iter().collect(),
            None => return,
        };
        for (path, band, responder) in asked {
            let reply = match self.collect(tick, &path, band) {
                Ok(end) => Reply::Collected(ending(&end)),
                Err(why) => Reply::Error(why),
            };
            responder.send(&reply);
        }
    }

    /// A pass of only the system's share of the store on the filesystem
    /// holding `path`, its generations and its garbage, in `band`, for the
    /// user daemon driving its order.
    pub(super) fn collect(&mut self, tick: &Tick, path: &Path, band: Band) -> Result<End, String> {
        // Answered as the deletes' rest is, so the user daemon's pass waits
        // with the store's garbage rather than counting a failure and taking
        // dearer files.
        if held_off(self.writers_held, band) {
            return Ok(End::Waiting);
        }
        let device = pool(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mount = match (self.store(), self.watched.get(&device)) {
            (Some((_, store)), Some(watched)) if store == device => watched.path.clone(),
            _ => return Err(format!("no garbage is collected on {}", path.display())),
        };
        let space = crate::sys::space(&mount).map_err(|error| error.to_string())?;
        let thresholds = Thresholds::new(&self.config.bands, space.total);
        let mut here = self.generation_offers(tick);
        here.extend(self.garbage_offers(tick));
        // Neither takes a process to judge, so the scan is never read.
        let me = std::process::id();
        let mut scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
        Ok(self.pass_on(
            tick,
            device,
            &here,
            Aim {
                band,
                goal: Goal::Reach,
                why: Why::Collect,
                surveyed: None,
            },
            thresholds,
            &mut scan,
        ))
    }
}
