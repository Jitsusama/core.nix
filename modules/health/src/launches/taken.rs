//! What a tool's reload heard after the stop it caused takes back from a
//! switch's restarts, and what a switch's reload heard later still gives
//! back. The jobs' follower can run ahead of the reloads', so health
//! learns whose a stop was only once both reloads before it are heard.

use super::{Launches, MOST_RELOADS, RELOADED_WITHIN, Ready, Reload, Restarting, millis, ready};
use crate::journal::Entry;

/// A stop a tool's reload took back: the service's, in which boot, when on
/// the boot's clock and the wall's, and the restart it timed, once its
/// start is heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Taken {
    unit: String,
    boot: String,
    stopped: i64,
    stopped_at: i64,
    timed: Option<Timed>,
}

/// A restart taken back, as it would have been counted: how long its
/// service was gone, in milliseconds, when on the boot's clock it started,
/// whether a ready line timed it, and what still waits for one, as
/// `restarting` would hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Timed {
    gone: u64,
    started: i64,
    stamped: bool,
    waiting: Option<Restarting>,
}

impl Launches {
    /// Take back the stops still waiting, and the restarts waiting for
    /// their ready line, that a tool's `reload` heard after them made its
    /// own, keeping each in `taken`.
    pub(super) fn take_back(&mut self, reload: &Reload) {
        let made = |boot: &str, since_boot, at| {
            reload.made(boot, since_boot, at, RELOADED_WITHIN)
                && !self.switched_before(boot, since_boot, at)
        };
        let stops: Vec<String> = self
            .stopping
            .iter()
            .filter(|(_, (boot, since_boot, at))| made(boot, *since_boot, *at))
            .map(|(unit, _)| unit.clone())
            .collect();
        let restarts: Vec<String> = self
            .restarting
            .iter()
            .filter(|(_, waiting)| made(&waiting.boot, waiting.stopped, waiting.stopped_at))
            .map(|(unit, _)| unit.clone())
            .collect();
        for unit in stops {
            if let Some((boot, stopped, stopped_at)) = self.stopping.remove(&unit) {
                self.take(Taken {
                    unit,
                    boot,
                    stopped,
                    stopped_at,
                    timed: None,
                });
            }
        }
        // A restart waiting for its ready line was its service's last
        // timed, since each one timed replaces the one waiting.
        for unit in restarts {
            let Some(waiting) = self.restarting.remove(&unit) else {
                continue;
            };
            let Some(gone) = self.restarted.get_mut(&unit).and_then(Vec::pop) else {
                continue;
            };
            if self.restarted.get(&unit).is_some_and(Vec::is_empty) {
                self.restarted.remove(&unit);
            }
            self.take(Taken {
                unit,
                boot: waiting.boot.clone(),
                stopped: waiting.stopped,
                stopped_at: waiting.stopped_at,
                timed: Some(Timed {
                    gone,
                    started: waiting.started,
                    stamped: false,
                    waiting: Some(waiting),
                }),
            });
        }
    }

    /// Keep what a tool's reload took back, at most [`MOST_RELOADS`].
    fn take(&mut self, taken: Taken) {
        if self.taken.len() >= MOST_RELOADS {
            self.taken.remove(0);
        }
        self.taken.push(taken);
    }

    /// Time the restart a start `entry` ends whose stop a tool's reload took
    /// back, keeping it with what was taken, for a switch's reload heard
    /// later to give back. One that times nothing goes, as its stop would.
    pub(super) fn time_taken(&mut self, entry: &Entry) {
        let Some(at) = self.untimed(entry) else {
            return;
        };
        let taken = &mut self.taken[at];
        match super::timed(&taken.boot, taken.stopped, taken.stopped_at, entry) {
            Some(gone) => {
                taken.timed = Some(Timed {
                    gone: millis(gone),
                    started: entry.since_boot,
                    stamped: false,
                    waiting: Some(Restarting {
                        boot: taken.boot.clone(),
                        stopped: taken.stopped,
                        started: entry.since_boot,
                        stopped_at: taken.stopped_at,
                        next_job: None,
                    }),
                });
            }
            None => {
                self.taken.remove(at);
            }
        }
    }

    /// Let a start `entry` of the day before end the stop taken back that
    /// it started, timing nothing, as `before` does a stop still waiting.
    pub(super) fn start_taken(&mut self, entry: &Entry) {
        if let Some(at) = self
            .untimed(entry)
            .filter(|&at| self.taken[at].stopped <= entry.since_boot)
        {
            self.taken.remove(at);
        }
    }

    /// Where in `taken` the stop a start `entry` could end is: its
    /// service's, in its boot, still untimed.
    fn untimed(&self, entry: &Entry) -> Option<usize> {
        self.taken.iter().position(|taken| {
            taken.unit == entry.thing && taken.boot == entry.boot && taken.timed.is_none()
        })
    }

    /// Let a job line `entry` fence off each restart taken back that waits
    /// in its boot, as `heard` does the one waiting in `restarting`: a
    /// ready line written after it is the next program's.
    pub(super) fn job_taken(&mut self, entry: &Entry) {
        for taken in &mut self.taken {
            if let Some(waiting) = taken
                .timed
                .as_mut()
                .and_then(|timed| timed.waiting.as_mut())
                .filter(|waiting| taken.unit == entry.thing && waiting.boot == entry.boot)
            {
                waiting.next_job.get_or_insert(entry.since_boot);
            }
        }
    }

    /// Let a ready line `entry` from `unit` time a restart taken back, as
    /// `served` does the one waiting in `restarting`, for a switch's
    /// reload heard later to give back as it would have been counted.
    pub(super) fn served_taken(&mut self, unit: &str, entry: &Entry) {
        for taken in &mut self.taken {
            let Some(timed) = taken.timed.as_mut().filter(|_| taken.unit == unit) else {
                continue;
            };
            let Some(waiting) = &timed.waiting else {
                continue;
            };
            match ready(waiting, entry) {
                Ready::NotIts => {}
                Ready::Late => timed.waiting = None,
                Ready::Gone(gone) => {
                    timed.gone = gone;
                    timed.stamped = true;
                    timed.waiting = None;
                }
            }
        }
    }

    /// Give back what a tool's reload took that a switch's `reload`, heard
    /// after it, made: a stop to wait for its start again, or a restart to
    /// count as it would have been. A restart since, timed or waiting for
    /// its ready line, stays its service's last, as `served` and
    /// `take_back` read it.
    pub(super) fn give_back(&mut self, reload: &Reload) {
        let (back, kept) = std::mem::take(&mut self.taken)
            .into_iter()
            .partition(|taken| {
                reload.made(
                    &taken.boot,
                    taken.stopped,
                    taken.stopped_at,
                    super::RESTART_WITHIN,
                )
            });
        self.taken = kept;
        for taken in back {
            let Some(timed) = taken.timed else {
                self.stopping.entry(taken.unit).or_insert((
                    taken.boot,
                    taken.stopped,
                    taken.stopped_at,
                ));
                continue;
            };
            let newer = self
                .restarting
                .get(&taken.unit)
                .is_some_and(|waiting| waiting.started > timed.started);
            let all = self.restarted.entry(taken.unit.clone()).or_default();
            if newer && !all.is_empty() {
                all.insert(all.len() - 1, timed.gone);
            } else {
                all.push(timed.gone);
                match timed.waiting {
                    Some(waiting) => {
                        self.restarting.insert(taken.unit.clone(), waiting);
                    }
                    None => {
                        self.restarting.remove(&taken.unit);
                    }
                }
            }
            if timed.stamped {
                self.stamped.insert(taken.unit);
            }
        }
    }
}
