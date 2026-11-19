//! How long programs took to open and to end over the day, from the lines
//! the windows daemon writes as niri opens and closes their windows
//! (modules/windows, src/timed.rs): a program's first window after its
//! process started, and its process ending after its last window went.
//! Beside them, how long each of our daemons took to be ready, from the
//! line each says once it is, such as dictation's "ready in 1065 ms" or
//! meaning's "bge loaded on the NPU in 688 ms", and how long each pi took
//! to show its first session, from its own "pi: ready in 889 ms". And how
//! long each service a switch restarted was gone: from systemd stopping it
//! to its ready line, or to its start where it says none, on its boot's
//! clock, which no step of the wall clock moves, though not across a
//! sleep longer than a restart can be, leaving out a oneshot, whose run
//! serves nothing while it lasts, and a restart that followed a reload
//! something other than a switch asked for (`reloads.rs`).
//!
//! Both are our side's figures, stand-ins for what Joel waits on: an
//! opening is timed from the process's start, not from the key that asked
//! for it, and an end from the window gone, since niri never says the key
//! that asked a window to close.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::journal::{Entry, Job, Origin};
use crate::launcher::percent;
use crate::reloads::{self, Asker};

/// What the windows daemon writes its journal as.
pub const IDENTIFIER: &str = "windows";

/// The windows daemon's unit, as modules/windows/home.nix names it: its
/// openings and ends are taken only from there, since any command can
/// write itself as `windows`, but a unit a job starts can't take that name
/// while the daemon's unit file is there.
pub const UNIT: &str = "windows.service";

/// What `journalctl --grep` follows for these lines.
pub const FOLLOWED: &str = "^(opened .* ms after its process started|closed .*; its process )";

/// An opening's budget, from the process's start to its first window, as
/// docs/design.md's Latency Budgets hold it.
pub const OPENED_MS: u64 = 300;

/// An end's, from the last window gone to the process ended.
pub const ENDED_MS: u64 = 200;

/// What `journalctl --grep` follows for a daemon saying it's ready, from
/// any program: a line saying it's ready or loaded, then how long it took.
pub const READY_FOLLOWED: &str = r"\b(ready|loaded)\b.* in [0-9]+ ms\b";

/// A daemon's budget from starting to ready, as docs/design.md's Latency
/// Budgets hold it for a daemon serving again.
pub const READY_MS: u64 = 1_000;

/// What pi writes its journal as.
pub const PI: &str = "pi";

/// A pi's budget from starting to its first session shown, the spec the
/// improvement system set for it (2026-11-11).
pub const PI_MS: u64 = 3_000;

/// A service's budget from stopping to serving again, the spec the
/// improvement system set for a switch's restarts (2026-11-11).
pub const RESTARTED_MS: u64 = 5_000;

/// How long after stopping a start still counts as a restart, in
/// microseconds: a stop systemd gives up on takes 90 s.
pub const RESTART_WITHIN: i64 = 120_000_000;

/// How long after a reload something other than a switch asked for a
/// service stopping is its restart: a second, on 14 November, for the
/// tool that updates itself.
const RELOADED_WITHIN: i64 = 10_000_000;

/// How long a reload is kept after a newer one in its boot, in
/// microseconds of the boot's clock: five times the longest a stop can be
/// a reload's restart, for a follower that lags.
const RELOADS_KEPT_FOR: i64 = 5 * RESTART_WITHIN;

/// The most reloads of a kind kept at once, so a program reloading
/// systemd over and over costs health no more than this.
const MOST_RELOADS: usize = 100;

/// How long after its start a ready line is still its restart's.
const READY_WITHIN: i64 = 60_000_000;

/// The day's openings and ends, read again from the journal on each
/// start, so never resumed.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Launches {
    /// Each program's openings, in milliseconds, as they came.
    pub opened: BTreeMap<String, Vec<u64>>,
    /// Each program's ends after its last window, in milliseconds.
    pub ended: BTreeMap<String, Vec<u64>>,
    /// How often each program still ran when the daemon stopped waiting.
    pub outlived: BTreeMap<String, u64>,
    /// How long each daemon took to be ready, as it said, in milliseconds.
    pub ready: BTreeMap<String, Vec<u64>>,
    /// How long each pi took to show its first session, in milliseconds.
    pub pi: BTreeMap<String, Vec<u64>>,
    /// How long each service was gone in a restart, in milliseconds.
    pub restarted: BTreeMap<String, Vec<u64>>,
    /// When each service started to stop, in which boot and in
    /// microseconds on the boot's clock and the wall clock, until it
    /// starts: a stop at shutdown and a start in the next boot are a
    /// reboot, not a restart.
    #[serde(skip)]
    stopping: BTreeMap<String, (String, i64, i64)>,
    /// Each service started again, until it says it's ready in that boot
    /// or starts once more.
    #[serde(skip)]
    restarting: BTreeMap<String, Restarting>,
    /// Each service that said it was ready soon after a restart.
    #[serde(skip)]
    stamped: BTreeSet<String>,
    /// The reloads systemd made lately for a switch, and for anything else,
    /// at most [`MOST_RELOADS`] of each.
    #[serde(skip)]
    reloaded: BTreeMap<bool, Vec<Reload>>,
    /// The stops and restarts a tool's reload heard late took back, until
    /// a switch's reload heard later says they were its own, at most
    /// [`MOST_RELOADS`]. A new day doesn't carry them: a switch's reload
    /// heard only after midnight leaves its restart the tool's.
    #[serde(skip)]
    taken: Vec<Taken>,
}

/// A stop a tool's reload took back: the service's, in which boot, when on
/// the boot's clock and the wall's, and the restart it timed, once its
/// start is heard, with what was waiting for its ready line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Taken {
    unit: String,
    boot: String,
    stopped: i64,
    stopped_at: i64,
    timed: Option<(u64, Restarting)>,
}

/// A restart waiting for its ready line: in which boot, when on the boot's
/// clock it stopped and started, when on the wall clock it stopped, and
/// its next job line's time, a stop or a start, once one is heard, since a
/// ready line written after that is the next program's.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Restarting {
    boot: String,
    stopped: i64,
    started: i64,
    stopped_at: i64,
    next_job: Option<i64>,
}

/// A reload systemd made: in which boot, how long into it, and by the wall
/// clock. A reload's restarts are in its own boot.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reload {
    boot: String,
    since_boot: i64,
    at: i64,
}

impl Reload {
    /// Whether a stop in `boot` at `since_boot` and `at` came within
    /// `longest` after this reload, on the boot's clock and the wall's, as
    /// [`Launches::restarts_itself`] says why.
    fn made(&self, boot: &str, since_boot: i64, at: i64, longest: i64) -> bool {
        self.boot == boot
            && (0..=longest).contains(&since_boot.saturating_sub(self.since_boot))
            && at.saturating_sub(self.at) <= longest
    }
}

/// How one of the two went over the day.
#[derive(Debug, PartialEq, Eq)]
pub struct Spread<'a> {
    /// How many were timed.
    pub count: u64,
    /// The median, in milliseconds.
    pub median_ms: u64,
    /// The p99, by nearest rank, in milliseconds.
    pub p99_ms: u64,
    /// The slowest, in milliseconds, and the program it was.
    pub slowest: (u64, &'a str),
    /// How many went past the budget.
    pub late: u64,
}

impl Spread<'_> {
    /// The share past the budget, in percent.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's count of openings is far below where a float loses whole numbers"
    )]
    pub fn late_percent(&self) -> f64 {
        100.0 * self.late as f64 / self.count.max(1) as f64
    }
}

impl Launches {
    /// A new day's, from its first line at `at`, carrying on from this one
    /// what a restart across midnight needs: the stops still waiting for
    /// their start, and the reloads that say whose a stop is, each within
    /// [`RESTART_WITHIN`] before, as no later start can time an older stop.
    /// A restart already timed stays the old day's, its ready line after
    /// midnight left unheard.
    #[must_use]
    pub fn carried(&self, at: i64) -> Self {
        let recent = |stopped_at: i64| at.saturating_sub(stopped_at) <= RESTART_WITHIN;
        Self {
            stopping: self
                .stopping
                .iter()
                .filter(|(_, (_, _, stopped_at))| recent(*stopped_at))
                .map(|(unit, stop)| (unit.clone(), stop.clone()))
                .collect(),
            reloaded: self
                .reloaded
                .iter()
                .map(|(&switch, kept)| {
                    let kept = kept.iter().filter(|reload| recent(reload.at)).cloned();
                    (switch, kept.collect())
                })
                .collect(),
            ..Self::default()
        }
    }

    /// Take an entry of the day before, heard after this day began, as a
    /// follower behind the others hears it, or read just before it, as
    /// the replay leads in: its stops and reloads, for a restart across
    /// midnight, and its starts, which end a stop of theirs. What it times
    /// was the day before's, so nothing is timed.
    pub fn before(&mut self, entry: &Entry) {
        match entry.job {
            Some(Job::Stopping) => {
                self.heard(entry);
            }
            Some(Job::Started) => {
                if self
                    .stopping
                    .get(&entry.thing)
                    .is_some_and(|(boot, stopped, _)| {
                        *boot == entry.boot && *stopped <= entry.since_boot
                    })
                {
                    self.stopping.remove(&entry.thing);
                }
            }
            None if entry.origin == Origin::Systemd && reloads::asker(&entry.message).is_some() => {
                self.heard(entry);
            }
            None => {}
        }
    }

    /// Take one entry, saying whether it was one of these: a service
    /// stopping or started, or a line `took` takes.
    pub fn heard(&mut self, entry: &Entry) -> bool {
        let unit = &entry.thing;
        // Whoever stops or starts it, a crash's restart included, a ready
        // line written after is the next program's, so it can't time the
        // restart before; one written before and heard after, as the
        // followers lag, still can. A line without the boot's stamp, at
        // 0, says nothing of when.
        if entry.job.is_some()
            && entry.since_boot > 0
            && let Some(waiting) = self
                .restarting
                .get_mut(unit)
                .filter(|waiting| waiting.boot == entry.boot)
        {
            waiting.next_job.get_or_insert(entry.since_boot);
        }
        match entry.job {
            Some(Job::Stopping) => {
                if !self.restarts_itself(&entry.boot, entry.since_boot, entry.at) {
                    self.stopping.insert(
                        unit.clone(),
                        (entry.boot.clone(), entry.since_boot, entry.at),
                    );
                }
            }
            Some(Job::Started) => {
                let Some((boot, stopped, stopped_at)) = self.stopping.remove(unit) else {
                    self.time_taken(entry);
                    return true;
                };
                if let Some(gone) = timed(&boot, stopped, stopped_at, entry) {
                    self.restarted
                        .entry(unit.clone())
                        .or_default()
                        .push(millis(gone));
                    self.restarting.insert(
                        unit.clone(),
                        Restarting {
                            boot,
                            stopped,
                            started: entry.since_boot,
                            stopped_at,
                            next_job: None,
                        },
                    );
                }
            }
            None => {
                // Who asked is systemd's to say, as a restart is.
                if let Some(asker) =
                    reloads::asker(&entry.message).filter(|_| entry.origin == Origin::Systemd)
                {
                    self.keep_reload(asker == Asker::Switch, entry);
                } else if (unit != IDENTIFIER || entry.ran_in.as_deref() == Some(UNIT))
                    && self.took(unit, &entry.message)
                {
                    self.served(entry);
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Time the restart a start `entry` ends whose stop a tool's reload took
    /// back, keeping it with what was taken, for a switch's reload heard
    /// later to give back. One that times nothing goes, as its stop would.
    fn time_taken(&mut self, entry: &Entry) {
        let Some(at) = self.taken.iter().position(|taken| {
            taken.unit == entry.thing && taken.boot == entry.boot && taken.timed.is_none()
        }) else {
            return;
        };
        let taken = &mut self.taken[at];
        match timed(&taken.boot, taken.stopped, taken.stopped_at, entry) {
            Some(gone) => {
                taken.timed = Some((
                    millis(gone),
                    Restarting {
                        boot: taken.boot.clone(),
                        stopped: taken.stopped,
                        started: entry.since_boot,
                        stopped_at: taken.stopped_at,
                        next_job: None,
                    },
                ));
            }
            None => {
                self.taken.remove(at);
            }
        }
    }

    /// Keep the reload `entry` says, a switch's or not. Heard live, a
    /// reload's follower can run ahead of the jobs', so a later reload may
    /// come before a stop written earlier: the recent ones are kept, not
    /// only the newest, for the stop to find the one before it. Another
    /// boot's are kept as they are, since a replay ordered by the wall
    /// clock can mix two boots' lines. A flood of more than [`MOST_RELOADS`]
    /// heard before a stop that lags behind them can cost the stop its
    /// reload, and its tool's restart then counts as a switch's: one
    /// misfiled restart, rather than a list a loop could grow for ever.
    ///
    /// The jobs' follower can run ahead too, so a tool's reload may come
    /// after the stop it caused, or after that restart was timed: either is
    /// then taken back as the tool's own, until a switch's reload heard
    /// after it says the stop was the switch's after all. One heard after
    /// the restart's ready line, or after the day it was timed in, comes
    /// too late, and the restart stays counted as a switch's.
    fn keep_reload(&mut self, switch: bool, entry: &Entry) {
        let kept = self.reloaded.entry(switch).or_default();
        kept.retain(|reload| {
            reload.boot != entry.boot
                || entry.since_boot.saturating_sub(reload.since_boot) <= RELOADS_KEPT_FOR
        });
        if kept.len() >= MOST_RELOADS {
            kept.remove(0);
        }
        let reload = Reload {
            boot: entry.boot.clone(),
            since_boot: entry.since_boot,
            at: entry.at,
        };
        kept.push(reload.clone());
        if switch {
            self.give_back(&reload);
        } else {
            self.take_back(&reload);
        }
    }

    /// Take back the stops still waiting, and the restarts waiting for
    /// their ready line, that a tool's `reload` heard after them made its
    /// own, keeping each in `taken`.
    fn take_back(&mut self, reload: &Reload) {
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
                timed: Some((gone, waiting)),
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

    /// Give back what a tool's reload took that a switch's `reload`, heard
    /// after it, made: a stop to wait for its start again, or a restart to
    /// count, waiting for its ready line unless another restart now is.
    fn give_back(&mut self, reload: &Reload) {
        let (back, kept) = std::mem::take(&mut self.taken)
            .into_iter()
            .partition(|taken| {
                reload.made(&taken.boot, taken.stopped, taken.stopped_at, RESTART_WITHIN)
            });
        self.taken = kept;
        for taken in back {
            match taken.timed {
                None => {
                    self.stopping.entry(taken.unit).or_insert((
                        taken.boot,
                        taken.stopped,
                        taken.stopped_at,
                    ));
                }
                Some((gone, waiting)) => {
                    self.restarted
                        .entry(taken.unit.clone())
                        .or_default()
                        .push(gone);
                    self.restarting.entry(taken.unit).or_insert(waiting);
                }
            }
        }
    }

    /// Whether a service stopping in `boot` at `since_boot` on its clock
    /// and `at` on the wall's is one something other than a switch
    /// restarts: it had systemd reload within [`RELOADED_WITHIN`] before it
    /// in its boot, and no switch did within [`RESTART_WITHIN`].
    /// A reload from the boot before counts for neither, however quick the
    /// reboot, and one over [`RESTART_WITHIN`] before the day's start isn't
    /// heard. Within the boot,
    /// both clocks must put it near enough: the boot's, which no step of
    /// the wall clock moves, to put it before the stop, and the wall
    /// clock's, since the boot's stands still while the machine sleeps, so
    /// a reload before a suspend would seem just before a stop after it. A
    /// wall clock stepped back may put the reload any way after the stop.
    /// One stepped forward past the window between the two reads as a
    /// sleep, which the journal's two clocks can't tell apart, so that
    /// rare restart counts as it did before either clock was kept.
    fn restarts_itself(&self, boot: &str, since_boot: i64, at: i64) -> bool {
        self.reloaded.get(&false).is_some_and(|kept| {
            kept.iter()
                .any(|reload| reload.made(boot, since_boot, at, RELOADED_WITHIN))
        }) && !self.switched_before(boot, since_boot, at)
    }

    /// Whether a switch's reload came within [`RESTART_WITHIN`] before a
    /// stop in `boot` at `since_boot` and `at`.
    fn switched_before(&self, boot: &str, since_boot: i64, at: i64) -> bool {
        self.reloaded.get(&true).is_some_and(|kept| {
            kept.iter()
                .any(|reload| reload.made(boot, since_boot, at, RESTART_WITHIN))
        })
    }

    /// A service saying it's ready soon after a restart started it: that
    /// restart lasted until now.
    pub fn served(&mut self, entry: &Entry) {
        let Some(unit) = &entry.ran_in else { return };
        // The day is replayed in the order it was written (`replay.rs`), so
        // a ready from the boot before comes before this boot's starts; a
        // ready from another boot still leaves this boot's restart waiting
        // for its own, so the pairing doesn't rest on that order.
        let Some(&Restarting {
            stopped,
            started,
            next_job,
            ..
        }) = self
            .restarting
            .get(unit)
            .filter(|waiting| waiting.boot == entry.boot)
        else {
            return;
        };
        if next_job.is_some_and(|next| next <= entry.since_boot) {
            return;
        }
        let after = entry.since_boot.saturating_sub(started);
        // On the boot's clock a ready line before its start was written
        // before it, by the program it stopped or by the new one beating
        // systemd's own line, and heard after, as one follower ran ahead of
        // another. The replay hears it first, leaving the restart for a
        // ready line after its start, and live is held to the same.
        if after < 0 {
            return;
        }
        self.restarting.remove(unit);
        if after > READY_WITHIN {
            return;
        }
        if let Some(gone) = self.restarted.get_mut(unit).and_then(|all| all.last_mut()) {
            *gone = millis(entry.since_boot.saturating_sub(stopped));
        }
        self.stamped.insert(unit.clone());
    }

    /// Take one line `thing` said, saying whether it was one of these: an
    /// opening or end from the windows daemon, a pi's start, or any
    /// program's ready.
    pub fn took(&mut self, thing: &str, message: &str) -> bool {
        if thing != IDENTIFIER {
            let Some(ms) = ready_in(message) else {
                return false;
            };
            let taken = if thing == PI {
                &mut self.pi
            } else {
                &mut self.ready
            };
            taken.entry(thing.to_owned()).or_default().push(ms);
            return true;
        }
        if let Some(rest) = message.strip_prefix("opened ") {
            let Some((app, ms)) = rest
                .strip_suffix(" ms after its process started")
                .and_then(|said| said.rsplit_once(' '))
            else {
                return false;
            };
            let Ok(ms) = ms.parse() else { return false };
            self.opened.entry(app.to_owned()).or_default().push(ms);
            return true;
        }
        let Some((app, how)) = message
            .strip_prefix("closed ")
            .and_then(|rest| rest.split_once("; its process "))
        else {
            return false;
        };
        if how.starts_with("was still running") {
            *self.outlived.entry(app.to_owned()).or_default() += 1;
            return true;
        }
        let Some(ms) = how
            .strip_prefix("ended ")
            .and_then(|rest| rest.strip_suffix(" ms later"))
            .and_then(|ms| ms.parse().ok())
        else {
            return false;
        };
        self.ended.entry(app.to_owned()).or_default().push(ms);
        true
    }

    /// The openings over the day, once any were timed.
    #[must_use]
    pub fn openings(&self) -> Option<Spread<'_>> {
        spread(&self.opened, OPENED_MS)
    }

    /// The ends over the day, once any were timed.
    #[must_use]
    pub fn ends(&self) -> Option<Spread<'_>> {
        spread(&self.ended, ENDED_MS)
    }

    /// How long daemons took to be ready over the day, once any said.
    #[must_use]
    pub fn readies(&self) -> Option<Spread<'_>> {
        spread(&self.ready, READY_MS)
    }

    /// How long each pi took to show its first session, once any said.
    #[must_use]
    pub fn pi_starts(&self) -> Option<Spread<'_>> {
        spread(&self.pi, PI_MS)
    }

    /// How long services were gone in a restart, once any was.
    #[must_use]
    pub fn restarts(&self) -> Option<Spread<'_>> {
        spread(&self.restarted, RESTARTED_MS)
    }

    /// Each service a restart timed only to its start, since it said no
    /// ready line after any of the day's, with how often it restarted:
    /// what its owner adds a line to, so its restarts time what it serves.
    #[must_use]
    pub fn unstamped(&self) -> BTreeMap<&str, u64> {
        self.restarted
            .iter()
            .filter(|(unit, _)| !self.stamped.contains(*unit))
            .map(|(unit, gone)| (unit.as_str(), u64::try_from(gone.len()).unwrap_or(u64::MAX)))
            .collect()
    }

    /// How many ends the daemon stopped waiting for.
    #[must_use]
    pub fn outlived(&self) -> u64 {
        self.outlived.values().sum()
    }

    /// The day's openings and ends in a line, as `health` says them.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(opened) = self.openings() {
            parts.push(said(&opened, ("opening", "openings"), OPENED_MS));
        }
        if let Some(ended) = self.ends() {
            parts.push(said(&ended, ("end", "ends"), ENDED_MS));
        }
        if let Some(ready) = self.readies() {
            parts.push(said(&ready, ("daemon ready", "daemons ready"), READY_MS));
        }
        if let Some(pi) = self.pi_starts() {
            parts.push(said(&pi, ("pi started", "pis started"), PI_MS));
        }
        if let Some(restarted) = self.restarts() {
            parts.push(said(&restarted, ("restart", "restarts"), RESTARTED_MS));
        }
        match self.outlived() {
            0 => {}
            1 => parts.push("1 program still ran 30 s after its last window".to_owned()),
            n => parts.push(format!(
                "{n} programs still ran 30 s after their last window"
            )),
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// Each program's name and its openings and ends, the slowest first,
    /// as `health launches` shows them.
    #[must_use]
    pub fn by_program(&self) -> Vec<(String, String)> {
        let mut names: Vec<&String> = self
            .opened
            .keys()
            .chain(self.ended.keys())
            .chain(self.outlived.keys())
            .chain(self.ready.keys())
            .chain(self.pi.keys())
            .chain(self.restarted.keys())
            .collect();
        names.sort();
        names.dedup();
        let slowest = |name: &String| {
            let most = |taken: &BTreeMap<String, Vec<u64>>| {
                taken.get(name).and_then(|all| all.iter().max().copied())
            };
            most(&self.opened)
                .max(most(&self.ended))
                .max(most(&self.ready))
                .max(most(&self.pi))
                .max(most(&self.restarted))
                .unwrap_or_default()
        };
        names.sort_by_key(|name| std::cmp::Reverse(slowest(name)));
        names
            .into_iter()
            .map(|name| {
                let one = |taken: &BTreeMap<String, Vec<u64>>, words| {
                    taken.get(name).map(|all| {
                        let most = all.iter().max().copied().unwrap_or_default();
                        format!("{} {words}, the slowest {most} ms", all.len())
                    })
                };
                let mut parts: Vec<String> = [
                    one(&self.opened, "opened"),
                    one(&self.ended, "ended"),
                    one(&self.ready, "ready"),
                    one(&self.pi, "started"),
                    one(&self.restarted, "restarted"),
                ]
                .into_iter()
                .flatten()
                .collect();
                if let Some(times) = self.outlived.get(name) {
                    parts.push(format!("{times} still running 30 s on"));
                }
                (name.clone(), parts.join("; "))
            })
            .collect()
    }
}

/// How long a daemon's line says it took to be ready: the first "in N
/// ms" after the word ready or loaded, or none when it says neither.
fn ready_in(message: &str) -> Option<u64> {
    let words: Vec<&str> = message
        .split_whitespace()
        .map(|word| word.trim_matches(|letter: char| !letter.is_ascii_alphanumeric()))
        .collect();
    let said = words
        .iter()
        .position(|word| matches!(*word, "ready" | "loaded"))?;
    words[said..].windows(3).find_map(|three| match three {
        ["in", ms, "ms"] => ms.parse().ok(),
        _ => None,
    })
}

/// How long a service stopped in `boot` at `stopped` on its clock and
/// `stopped_at` on the wall's was gone until the start `entry`, in
/// microseconds, or none when that start ends no restart. A restart is
/// timed on the boot's clock, which no step of the wall clock moves; a
/// start before its stop on it was heard out of order, or read without the
/// boot's stamp. That clock stands still while the machine sleeps, though,
/// so the wall clock bounds it too, as `restarts_itself` does: a service
/// stopped before a sleep and started after it wasn't restarted. One
/// stepped forward past the window reads as a sleep, which the two clocks
/// can't tell apart. A stop read without the boot's stamp, at 0, can't
/// time one either, and a oneshot's run ending says "Finished" under a
/// start's message id: it served nothing, so nothing was gone.
fn timed(boot: &str, stopped: i64, stopped_at: i64, entry: &Entry) -> Option<i64> {
    let gone = entry.since_boot.saturating_sub(stopped);
    (boot == entry.boot
        && !entry.message.starts_with("Finished ")
        && stopped > 0
        && (0..=RESTART_WITHIN).contains(&gone)
        && entry.at.saturating_sub(stopped_at) <= RESTART_WITHIN)
        .then_some(gone)
}

/// Microseconds as whole milliseconds.
fn millis(micros: i64) -> u64 {
    u64::try_from(micros / 1_000).unwrap_or_default()
}

/// One of them over the day, or none when none was timed.
fn spread(taken: &BTreeMap<String, Vec<u64>>, bar_ms: u64) -> Option<Spread<'_>> {
    let mut all: Vec<u64> = taken.values().flatten().copied().collect();
    all.sort_unstable();
    let count = u64::try_from(all.len()).ok()?;
    let ranked = |rank: u64| {
        let at = usize::try_from(rank.max(1) - 1).ok()?;
        all.get(at).copied()
    };
    let slowest = taken
        .iter()
        .flat_map(|(name, each)| each.iter().map(move |&ms| (ms, name.as_str())))
        .max_by_key(|&(ms, _)| ms)?;
    Some(Spread {
        count,
        median_ms: ranked(count.div_ceil(2))?,
        p99_ms: ranked((count * 99).div_ceil(100))?,
        slowest,
        late: u64::try_from(all.iter().filter(|&&ms| ms > bar_ms).count()).ok()?,
    })
}

/// One of them in words.
fn said(spread: &Spread, (one, many): (&str, &str), bar_ms: u64) -> String {
    let late = match spread.late {
        0 => format!("none past {bar_ms} ms"),
        late => format!(
            "{late} past {bar_ms} ms ({})",
            percent(spread.late_percent())
        ),
    };
    format!(
        "{} {}: a median {} ms, the slowest {} ms ({}); {late}",
        spread.count,
        if spread.count == 1 { one } else { many },
        spread.median_ms,
        spread.slowest.0,
        spread.slowest.1
    )
}

#[cfg(test)]
mod tests;
