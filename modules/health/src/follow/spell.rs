//! A spell of the day's writes failing, as on a full disk, and what
//! health says of it: once as it starts, once at each new kind of cause
//! and once as it ends, and of a disk flapping at its edge, no more than
//! once every ten minutes, with how many short spells went unsaid before
//! it, or, once the ten minutes pass with none on or as a run ends with its
//! follower, in a notice of their own; a stop ends a run with them untold.
//! A spell lasts only as long as health's run, so a start after one says
//! the next spell at once, and a spell that outlasts a run, as one given
//! up on does when its last try fails, is never told ended.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use super::{Follower, GIVES_UP, SETTLED};
use crate::day::Day;

/// How long after a spell's end was told that a new one goes unsaid, as a
/// disk at the edge of full flaps between the two: such a spell is said
/// once this much has passed, and its end then told too. It's as long as
/// `SETTLED`, the least a run lasts, since each start says a spell at
/// once, so health's starts alone say a flapping disk no more often. A
/// told end can let a run that has had its follower end end at once,
/// though, and the start that follows says the next spell within a
/// minute. It's kept on both clocks, as the awake one stands still while
/// the machine sleeps and the wall one may be set: a wake on a disk still
/// full is said at once, as is a spell after the clock is set back, or on
/// by ten minutes or more.
const QUIET_FOR: Duration = SETTLED;

/// A spell of the day's writes failing: since when, its latest error,
/// whether it was said and the kinds of cause it was said for, when a
/// spell's end was last told, on the awake clock and the wall's, in
/// microseconds, and how many short spells since went unsaid, as `QUIET_FOR`
/// keeps them. A cause is known by its kind, as a disk full
/// or read-only, since its words name the day failing, which changes at
/// midnight or once a day held from before is written. The errors std
/// leaves unnamed share one kind, so a change between two of them, both
/// rare in writing a file, goes unsaid.
#[derive(Debug, Default)]
pub(super) struct Spell {
    since: Option<Instant>,
    latest: String,
    said: bool,
    causes: Vec<io::ErrorKind>,
    told: Option<(Instant, i64)>,
    unsaid: u32,
}

impl Spell {
    /// Whether a spell is on: the day's writes failing.
    pub(super) fn is_on(&self) -> bool {
        self.since.is_some()
    }

    /// Whether health has tried to write the day for `GIVES_UP` by `at`,
    /// and so gives up.
    pub(super) fn gave_up(&self, at: Instant) -> bool {
        self.since
            .is_some_and(|since| at.saturating_duration_since(since) >= GIVES_UP)
    }

    /// What health ends on once it has given up writing the day into
    /// `into`, with the days held from before it `lost` (`losing`) and the
    /// spell's latest error, its final try's first failure, since a first
    /// said long ago may name a day written since.
    pub(super) fn given_up(&self, into: &Path, lost: &str) -> io::Error {
        io::Error::other(format!(
            "health gave up writing the day into {} after {} minutes of trying{lost}: {}",
            into.display(),
            GIVES_UP.as_secs() / 60,
            self.latest
        ))
    }

    /// Health's last try at `at`, `wall` on the wall clock, once it
    /// `gave_up`: the `follower`'s days written once more and `tried`. One
    /// that fails is what health ends on (`given_up`); one that writes
    /// everything ends the spell, and health carries on, as it lost nothing.
    pub(super) fn last_try(
        &mut self,
        follower: &mut Follower,
        at: Instant,
        wall: i64,
        say: &dyn Fn(&str),
        notice: &dyn Fn(&str),
    ) -> io::Result<()> {
        let written = follower.write();
        self.tried(
            written.as_ref().err(),
            at,
            wall,
            &follower.into,
            say,
            notice,
        );
        let lost = losing(&follower.held);
        written.map_err(|_| self.given_up(&follower.into, &lost))
    }

    /// Take a write's `error` at `at`, `wall` on the wall clock, if any. A
    /// spell's start is said through `say`, naming the folder `into` and
    /// the error, and its end through `notice`, once each, since the next
    /// half minute's would most likely say the same; a cause of a kind not
    /// yet said within the spell is said once too, as a full disk turning
    /// read-only. A spell starting within `QUIET_FOR` of the last end told
    /// is said only once `QUIET_FOR` has passed since, with how long it has
    /// lasted and its error then, so a flapping disk is said at most once
    /// in each `QUIET_FOR`, however short its spells, and a lasting one
    /// soon. Short spells it kept unsaid are counted in the next said, or,
    /// once the quiet ends with none on, told through `notice` on their own.
    /// That keeps no new quiet, so a disk still flapping is said again as
    /// failing rather than only ever told writing again.
    pub(super) fn tried(
        &mut self,
        error: Option<&io::Error>,
        at: Instant,
        wall: i64,
        into: &Path,
        say: &dyn Fn(&str),
        notice: &dyn Fn(&str),
    ) {
        if let Some(error) = error {
            self.latest = error.to_string();
        }
        match (error, self.since) {
            (Some(error), None) => {
                self.since = Some(at);
                self.said = !self.quiet(at, wall);
                if self.said {
                    self.say(&error.to_string(), error.kind(), into, Duration::ZERO, say);
                }
            }
            (Some(error), Some(since)) => {
                let lasted = at.saturating_duration_since(since);
                if !self.said && !self.quiet(at, wall) {
                    self.said = true;
                    self.say(&error.to_string(), error.kind(), into, lasted, say);
                } else if self.said && !self.causes.contains(&error.kind()) && !self.gave_up(at) {
                    // Not on the last try, which promises no more tries:
                    // health ends on it, naming this cause (`given_up`).
                    self.causes.push(error.kind());
                    say(&unwritten(&format!("now {error}"), into, lasted, 0));
                }
            }
            (None, Some(since)) => {
                if self.said {
                    notice(&format!(
                        "health: writes the day into {} again, after trying for {}",
                        into.display(),
                        lasting(at.saturating_duration_since(since))
                    ));
                    self.told = Some((at, wall));
                } else {
                    self.unsaid = self.unsaid.saturating_add(1);
                }
                self.since = None;
                self.latest.clear();
                self.said = false;
                self.causes.clear();
            }
            (None, None) => {
                if self.unsaid > 0 && !self.quiet(at, wall) {
                    self.tell_unsaid(into, notice);
                }
            }
        }
    }

    /// The run's final write, `written`, handed back, with the short spells
    /// kept unsaid told through `notice` if it wrote into `into`, as a run
    /// ending with its follower within the quiet would otherwise lose them.
    /// A stop, restart or reboot ends health before this, losing them yet.
    pub(super) fn ending(
        &mut self,
        written: io::Result<()>,
        into: &Path,
        notice: &dyn Fn(&str),
    ) -> io::Result<()> {
        if written.is_ok() && self.unsaid > 0 {
            self.tell_unsaid(into, notice);
        }
        written
    }

    /// Tell the short spells kept unsaid, writing into `into` again since,
    /// and count them afresh.
    fn tell_unsaid(&mut self, into: &Path, notice: &dyn Fn(&str)) {
        notice(&format!(
            "health: writes the day into {} again, after {} failing that went unsaid",
            into.display(),
            short_spells(self.unsaid)
        ));
        self.unsaid = 0;
    }

    /// Whether a spell at `at`, `wall` on the wall clock, comes within
    /// `QUIET_FOR` of the last end told on both clocks, and so goes unsaid
    /// for now: a sleep the wall clock saw, or a wall clock set, ends the
    /// quiet.
    fn quiet(&self, at: Instant, wall: i64) -> bool {
        self.told.is_some_and(|(told, told_wall)| {
            at.saturating_duration_since(told) < QUIET_FOR
                && wall
                    .checked_sub(told_wall)
                    .and_then(|since| u64::try_from(since).ok())
                    .is_some_and(|since| Duration::from_micros(since) < QUIET_FOR)
        })
    }

    /// Say the spell for `cause`, of `kind`, its writes having failed for
    /// `lasted`, with the short spells unsaid before it, which it then
    /// counts afresh.
    fn say(
        &mut self,
        cause: &str,
        kind: io::ErrorKind,
        into: &Path,
        lasted: Duration,
        say: &dyn Fn(&str),
    ) {
        self.causes.push(kind);
        say(&unwritten(cause, into, lasted, self.unsaid));
        self.unsaid = 0;
    }
}

/// What health says of a day it can't write into `into` for `cause`, its
/// writes having failed for `lasted`, if said late, and `unsaid` short
/// spells before it gone unsaid, with the time `left`.
fn unwritten(cause: &str, into: &Path, lasted: Duration, unsaid: u32) -> String {
    let already = if lasted.is_zero() {
        String::new()
    } else {
        format!(" having tried for {},", lasting(lasted))
    };
    let before = match unsaid {
        0 => String::new(),
        spells => format!(" and {} before it went unsaid,", short_spells(spells)),
    };
    format!(
        "health: can't write the day into {}: {cause},{already}{before} so it keeps the day and \
         tries again every half minute for {}",
        into.display(),
        left(lasted)
    )
}

/// The time a spell that has `lasted` so long has left, about so long,
/// to the nearest minute, rather than up to it: the last try comes at the
/// first tick past `GIVES_UP`, which may be most of a tick later, so a
/// limit would be one health goes past. Under half a minute left, the
/// last try comes within a minute.
pub(super) fn left(lasted: Duration) -> String {
    match GIVES_UP.saturating_sub(lasted).as_secs().saturating_add(30) / 60 {
        0 => "less than a minute".to_owned(),
        1 => "about a minute".to_owned(),
        60 => "about an hour".to_owned(),
        minutes => format!("about {minutes} minutes"),
    }
}

/// So many short spells, in words.
fn short_spells(spells: u32) -> String {
    match spells {
        1 => "a short spell".to_owned(),
        spells => format!("{spells} short spells"),
    }
}

/// What health says it loses of the days `held` from before as it ends
/// unable to write them, which a start's replay of only today won't hear
/// again: nothing when none are held.
pub(super) fn losing(held: &[Day]) -> String {
    let dates: Vec<&str> = held.iter().map(|day| day.date.as_str()).collect();
    match dates.split_last() {
        None => String::new(),
        Some((last, [])) => {
            format!(", losing what it couldn't write of {last}, held from before")
        }
        Some((last, rest)) => format!(
            ", losing what it couldn't write of {} and {last}, held from before",
            rest.join(", ")
        ),
    }
}

/// How long `lasted` was, to the minute: time health was awake to try,
/// since a sleeping machine's clock stands still.
pub(super) fn lasting(lasted: Duration) -> String {
    match lasted.as_secs() / 60 {
        0 => "less than a minute".to_owned(),
        1 => "a minute".to_owned(),
        minutes => format!("{minutes} minutes"),
    }
}
