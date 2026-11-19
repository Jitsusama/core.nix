//! A spell of the day's writes failing, as on a full disk, and what
//! health says of it: once as it starts and once as it ends, and of a
//! disk flapping at its edge, no more than once every ten minutes. A
//! spell lasts only as long as health's run, so a start after one says
//! the next spell at once, and a spell that outlasts a run, as one given
//! up on does, is never told ended.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use super::{GIVES_UP, SETTLED};
use crate::day::Day;

/// How long after a spell's end was told that a new one goes unsaid, as a
/// disk at the edge of full flaps between the two: such a spell is said
/// once this much has passed, and its end then told too. It's as long as
/// `SETTLED`, the least a run lasts, since each start says a spell at
/// once, so health's starts alone say a flapping disk no more often. A
/// told end can let a run that has had its follower end end at once,
/// though, and the start that follows says the next spell within a
/// minute.
const QUIET_FOR: Duration = SETTLED;

/// A spell of the day's writes failing: since when and for what, first
/// and latest, while they do, whether it was said, and when a spell's end
/// was last told. The two causes differ once the day failing does, as
/// when a day held from before is written and today goes on failing.
#[derive(Debug, Default)]
pub(super) struct Spell {
    since: Option<Instant>,
    cause: String,
    latest: String,
    said: bool,
    told: Option<Instant>,
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
    /// `into`, with the days held from before it `lost` (`losing`) and
    /// what failed last: its `final_try`'s error, else the spell's latest,
    /// since a first said long ago may name a day written since.
    pub(super) fn given_up(
        &self,
        into: &Path,
        lost: &str,
        final_try: Option<&io::Error>,
    ) -> io::Error {
        let cause = final_try.map_or_else(|| self.latest.clone(), ToString::to_string);
        io::Error::other(format!(
            "health gave up writing the day into {} after {} minutes of trying{lost}: {cause}",
            into.display(),
            GIVES_UP.as_secs() / 60,
        ))
    }

    /// Take a write's `error` at `at`, if any. A spell's start is said
    /// through `say`, naming the folder `into` and the spell's first
    /// error, and its end through `notice`, once each, since the next
    /// half minute's would most likely say the same. A spell starting
    /// within `QUIET_FOR` of the last
    /// end told is said only once `QUIET_FOR` has passed since, with how
    /// long it has lasted, so a flapping disk is said at most once in each
    /// `QUIET_FOR`, however short its spells, and a lasting one soon.
    pub(super) fn tried(
        &mut self,
        error: Option<&io::Error>,
        at: Instant,
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
                self.cause = error.to_string();
                self.said = self
                    .told
                    .is_none_or(|told| at.saturating_duration_since(told) >= QUIET_FOR);
                if self.said {
                    say(&unwritten(&self.cause, into, Duration::ZERO));
                }
            }
            (Some(_), Some(since)) => {
                let quiet = self
                    .told
                    .is_some_and(|told| at.saturating_duration_since(told) < QUIET_FOR);
                if !self.said && !quiet {
                    self.said = true;
                    say(&unwritten(
                        &self.cause,
                        into,
                        at.saturating_duration_since(since),
                    ));
                }
            }
            (None, Some(since)) => {
                if self.said {
                    notice(&format!(
                        "health: writes the day into {} again, after trying for {}",
                        into.display(),
                        lasting(at.saturating_duration_since(since))
                    ));
                    self.told = Some(at);
                }
                self.since = None;
                self.cause.clear();
                self.latest.clear();
                self.said = false;
            }
            (None, None) => {}
        }
    }
}

/// What health says of a day it can't write into `into` for `cause`, its
/// writes having failed for `lasted`, if said late, with the time left.
fn unwritten(cause: &str, into: &Path, lasted: Duration) -> String {
    let already = if lasted.is_zero() {
        String::new()
    } else {
        format!(" having tried for {},", lasting(lasted))
    };
    format!(
        "health: can't write the day into {}: {cause},{already} so it keeps the day and tries \
         again every half minute for up to {} minutes",
        into.display(),
        GIVES_UP.saturating_sub(lasted).as_secs() / 60
    )
}

/// What health says it loses of the days `held` from before as it ends
/// unable to write them, which a start's replay of only today won't hear
/// again: nothing when none are held.
pub(super) fn losing(held: &[Day]) -> String {
    let dates: Vec<&str> = held.iter().map(|day| day.date.as_str()).collect();
    match dates.split_last() {
        None => String::new(),
        Some((last, [])) => {
            format!(", losing what {last}, held from before, saw and couldn't write")
        }
        Some((last, rest)) => format!(
            ", losing what {} and {last}, held from before, saw and couldn't write",
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
