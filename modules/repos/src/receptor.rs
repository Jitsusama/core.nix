//! The GitHub receptor: how many open changes ask Joel's review, said to
//! the realm's mycelium node, which draws it as a cell on GitHub's outside
//! network (mycelium's PROTOCOL.md, Receptors).
//!
//! It's counted from what a refresh already read, never asked of GitHub,
//! so it adds no query and no token: the review requests ride the one
//! query every refresh sends (github.rs), on the token that reads that
//! repository already. Only counts and fixed words leave: no title,
//! branch, repository or login is ever an argument, so nothing a change
//! says reaches the node. mycelium is run by its store path, by the
//! speaker every daemon's receptors share (../receptor-say).
//!
//! Whose review counts, and which owners are the realm's, are declared by
//! the machine's config alone, in `REPOS_RECEPTOR_LOGIN` and
//! `REPOS_RECEPTOR_OWNERS` (home.nix), never read from a record, a
//! checkout or any file a refresh writes. Each fails closed: with no login
//! nothing is said, a repository of an owner the realm doesn't list counts
//! nowhere, and an owner listed for two realms counts in neither.

use std::ffi::OsString;

use receptor_say::{Declared, Source, Speaker, State, Told};

use crate::github::Seen;
use crate::when;

/// How often the receptor is said, the refresh's own timer (home.nix),
/// which is also how late the node lets it be before reading it failing.
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(300);

/// An hour, in the record's seconds.
const HOUR: u64 = 3_600;

/// The owners whose repositories are the realm's, as the machine declares
/// them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owners(Vec<String>);

impl Owners {
    /// The owners `declared` lists for `realm`, as `realm=owner,owner`
    /// separated by spaces, less any it lists for another realm too.
    pub fn of(declared: &str, realm: &str) -> Self {
        let listed: Vec<(&str, Vec<&str>)> = declared
            .split_whitespace()
            .filter_map(|one| one.split_once('='))
            .map(|(realm, owners)| (realm, owners.split(',').filter(|o| !o.is_empty()).collect()))
            .collect();
        let elsewhere = |owner: &str| {
            listed.iter().any(|(other, owners)| {
                *other != realm && owners.iter().any(|o| o.eq_ignore_ascii_case(owner))
            })
        };
        Self(
            listed
                .iter()
                .filter(|(one, _)| *one == realm)
                .flat_map(|(_, owners)| owners.iter().copied())
                .filter(|owner| !elsewhere(owner))
                .map(str::to_owned)
                .collect(),
        )
    }

    /// Whether the repository keyed `key` is one of these owners', its
    /// owner matched as GitHub matches one, ASCII case aside.
    pub fn hold(&self, key: &str) -> bool {
        let _ = (key, &self.0);
        false
    }
}

/// What the receptor says of what a refresh saw at `now`, the day having
/// begun at `midnight`, both in seconds: the open changes in the owners'
/// repositories asking `login`'s review, those updated within the hour,
/// and today. It's failing while any of the owners' repositories went
/// unread, its counts then of those read.
pub fn told(seen: &Seen, owners: &Owners, login: &Declared, now: u64, midnight: u64) -> Told {
    let asking: Vec<u64> = seen
        .changes
        .iter()
        .filter(|(key, _)| owners.hold(key))
        .flat_map(|(_, changes)| changes)
        .filter(|change| change.review_asked_of.iter().any(|one| login.is(one)))
        .map(|change| change.updated_at)
        .collect();
    let since = |from: u64| count(asking.iter().filter(|&&at| (from..=now).contains(&at)));
    let unread = seen.unread.iter().any(|why| {
        why.split_once(": ")
            .is_some_and(|(key, _)| owners.hold(key))
    });
    let addressed = count(asking.iter());
    let state = if unread {
        State::Failing
    } else if addressed > 0 {
        State::Reading
    } else {
        State::Quiet
    };
    Told {
        source: Source::GitHub,
        state,
        hour: since(now.saturating_sub(HOUR) + 1),
        today: since(midnight),
        addressed,
    }
}

/// How many there are, as mycelium counts.
fn count<T>(items: impl Iterator<Item = T>) -> u64 {
    u64::try_from(items.count()).unwrap_or(u64::MAX)
}

/// When `now`'s day began where the machine is, in seconds.
pub fn midnight(now: u64) -> u64 {
    let here = when::local(now);
    let into = u64::try_from(here.hour * 3_600 + here.minute * 60).unwrap_or(0);
    now - now % 60 - into.min(now - now % 60)
}

/// Where the GitHub receptor is said, for whom, and for which owners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receptor {
    speaker: Speaker,
    login: Declared,
    owners: Owners,
}

impl Receptor {
    /// The receptor the machine declares, if it declares one whole: an
    /// absolute mycelium, a realm of its letters, a GitHub login, and the
    /// owners listed for that realm.
    pub fn named(
        program: Option<OsString>,
        realm: Option<OsString>,
        login: Option<OsString>,
        owners: Option<OsString>,
    ) -> Option<Self> {
        let realm = realm?.into_string().ok()?;
        let owners = Owners::of(&owners?.into_string().ok()?, &realm);
        Some(Self {
            speaker: Speaker::named(program, Some(realm.into()))?,
            login: Declared::login(login)?,
            owners,
        })
    }

    /// The receptor the service's environment declares.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
            std::env::var_os("REPOS_RECEPTOR_LOGIN"),
            std::env::var_os("REPOS_RECEPTOR_OWNERS"),
        )
    }

    /// What mycelium is given for what a refresh saw at `now`, said for
    /// the process `pid`, which owns the refresh's timer.
    pub fn arguments(&self, seen: &Seen, now: u64, pid: u32) -> Vec<OsString> {
        let told = told(seen, &self.owners, &self.login, now, midnight(now));
        self.speaker.arguments(told, EVERY, pid)
    }

    /// Says what a refresh saw at `now`.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn say(&self, seen: &Seen, now: u64, pid: u32) -> Result<(), String> {
        let told = told(seen, &self.owners, &self.login, now, midnight(now));
        self.speaker.say(&[told], EVERY, pid)
    }
}

#[cfg(test)]
mod tests;
