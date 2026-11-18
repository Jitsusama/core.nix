//! The GitHub receptor: how many open changes ask Joel's review, said to
//! the realm's mycelium node, which draws it as a cell on GitHub's outside
//! network (mycelium's PROTOCOL.md, Receptors).
//!
//! It's counted from what a refresh already read, never asked of GitHub,
//! so it adds no query and no token: the review requests ride the one
//! query every refresh sends (github.rs), on the token that reads that
//! repository already. Only counts, fixed words and the changes asking
//! Joel's review leave, each change as `owner/name#N` alone, with the
//! realm's owners so mycelium can refuse any other: the node feeds each
//! to the cell whose quest claims it (mycelium's PROTOCOL.md, Feeding). No
//! title, branch, author or login is ever an argument, so nothing a
//! change says of itself reaches the node. mycelium is run by its store
//! path, by the speaker every daemon's receptors share (../receptor-say).
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
use crate::model::Change;

/// How often the receptor is said, the refresh's own timer (home.nix),
/// which is also how late the node lets it be before reading it failing.
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(300);

/// An hour, in the record's seconds.
const HOUR: u64 = 3_600;

/// The most changes asking review that are named to mycelium, which keeps
/// no more than this for a receptor.
const MOST_ITEMS: usize = 200;

/// The owners whose repositories are the realm's, as the machine declares
/// them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owners(Vec<String>);

impl Owners {
    /// The owners `declared` lists for `realm`, as `realm=owner,owner`
    /// separated by `;`, less any it lists for another realm too. It's one
    /// word, since systemd splits an unquoted `Environment=` at whitespace,
    /// so a value holding any is part of one, a realm's owners perhaps
    /// lost, and holds nobody.
    pub fn of(declared: &str, realm: &str) -> Self {
        if declared.contains(char::is_whitespace) {
            return Self(Vec::new());
        }
        let listed: Vec<(&str, Vec<&str>)> = declared
            .split(';')
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
        let owner = key
            .strip_prefix("github:")
            .and_then(|path| path.split_once('/'))
            .map(|(owner, _)| owner);
        owner.is_some_and(|owner| self.0.iter().any(|one| one.eq_ignore_ascii_case(owner)))
    }
}

/// What the receptor says of what a refresh saw at `now`, in seconds: as
/// `today`, the open changes it read in the owners' repositories, every one
/// read today; as `hour`, those updated within the hour; and as
/// `addressed`, those asking `login`'s review, however long ago it was
/// asked, since a review waits on Joel until he gives it. Mycelium refuses
/// an `addressed` past `today`, so the two count from the same changes.
/// It's failing while any of the owners' repositories went unread, its
/// counts then of those read.
pub fn told(seen: &Seen, owners: &Owners, login: &Declared, now: u64) -> Told {
    let read: Vec<&Change> = seen
        .changes
        .iter()
        .filter(|(key, _)| owners.hold(key))
        .flat_map(|(_, changes)| changes)
        .collect();
    let within_the_hour = (now.saturating_sub(HOUR) + 1)..=now;
    let unread = seen.unread.iter().any(|why| {
        why.split_once(": ")
            .is_some_and(|(key, _)| owners.hold(key))
    });
    let addressed = count(
        read.iter()
            .filter(|change| change.review_asked_of.iter().any(|one| login.is(one))),
    );
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
        hour: count(
            read.iter()
                .filter(|change| within_the_hour.contains(&change.updated_at)),
        ),
        today: count(read.iter()),
        addressed,
    }
}

/// The changes asking `login`'s review in the owners' repositories, as
/// `owner/name#N` spelled as GitHub spells the pull request, newest first
/// and at most [`MOST_ITEMS`]. GitHub's spelling, not a clone's, so two
/// clones of one repository, or a remote typed in another case, name a
/// change once and by the name a quest's alias copies. One whose link
/// isn't GitHub's pull request of its number, whose owner isn't the
/// realm's, or whose owner or name holds more than mycelium takes in one
/// (letters, digits, `-`, `_` and `.`), is left out, so it can't make
/// mycelium refuse the counts it's said beside.
pub fn asked(seen: &Seen, owners: &Owners, login: &Declared) -> Vec<String> {
    let mut asked: Vec<(u64, String)> = seen
        .changes
        .iter()
        .filter(|(key, _)| owners.hold(key))
        .flat_map(|(_, changes)| changes)
        .filter(|change| change.review_asked_of.iter().any(|one| login.is(one)))
        .filter_map(|change| {
            let github = change.github.as_ref()?;
            let item = spelled(&github.url, github.number)?;
            owners
                .hold(&format!("github:{item}"))
                .then_some((change.updated_at, item))
        })
        .collect();
    // Each change's newest first, so the one kept of two is the newest.
    asked.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    asked.dedup_by(|a, b| a.1 == b.1);
    asked.sort_by(|a, b| b.cmp(a));
    asked
        .into_iter()
        .take(MOST_ITEMS)
        .map(|(_, item)| item)
        .collect()
}

/// The pull request at `url`, numbered `number`, as `owner/name#N`: none
/// unless `url` is GitHub's link to that very pull request, of an owner
/// and a name mycelium takes.
fn spelled(url: &str, number: u64) -> Option<String> {
    let mut path = url.strip_prefix("https://github.com/")?.split('/');
    let (owner, name) = (path.next()?, path.next()?);
    let (pull, said) = (path.next()?, path.next()?);
    let sound = path.next().is_none()
        && pull == "pull"
        && number > 0
        && said.parse::<u64>().ok() == Some(number)
        && named(owner)
        && named(name);
    sound.then(|| format!("{owner}/{name}#{number}"))
}

/// Whether `word` is an owner or a repository name as mycelium takes one:
/// its letters, at most [`LONGEST_NAME`] of them and not dots alone. One
/// opening with a dash is refused too, since mycelium would read it as a
/// flag of its own and refuse the counts with it.
fn named(word: &str) -> bool {
    !word.is_empty()
        && word.len() <= LONGEST_NAME
        && !word.starts_with('-')
        && word.bytes().any(|letter| letter != b'.')
        && word
            .bytes()
            .all(|letter| letter.is_ascii_alphanumeric() || b"-_.".contains(&letter))
}

/// The longest owner or repository name mycelium takes.
const LONGEST_NAME: usize = 100;

/// How many there are, as mycelium counts.
fn count<T>(items: impl Iterator<Item = T>) -> u64 {
    u64::try_from(items.count()).unwrap_or(u64::MAX)
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
    /// owners listed for that realm, at least one of them, since a realm
    /// with none would read quiet for good rather than as undeclared.
    pub fn named(
        program: Option<OsString>,
        realm: Option<OsString>,
        login: Option<OsString>,
        owners: Option<OsString>,
    ) -> Option<Self> {
        let realm = realm?.into_string().ok()?;
        let owners = Owners::of(&owners?.into_string().ok()?, &realm);
        if owners.0.is_empty() {
            return None;
        }
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
        let told = told(seen, &self.owners, &self.login, now);
        self.speaker
            .arguments_with(told, EVERY, pid, self.items(seen, told.state))
    }

    /// The words naming the changes asking review, after the counts: each
    /// owner that may be named, the declared ones mycelium takes and those
    /// the items spell (each one of the declared, ASCII case aside), as
    /// `--owner`, then each change as `--item`. The owners are
    /// named even with no item, so mycelium clears what it kept. None is
    /// said while a repository went unread, since its changes are then
    /// missing rather than closed, and clearing them would feed every cell
    /// again once the read came back.
    fn items(&self, seen: &Seen, state: State) -> Vec<OsString> {
        if state == State::Failing {
            return Vec::new();
        }
        let asked = asked(seen, &self.owners, &self.login);
        // A declared owner mycelium wouldn't take would make it refuse the
        // counts too, so it's left out rather than said.
        let mut owners: Vec<&str> = self
            .owners
            .0
            .iter()
            .map(String::as_str)
            .filter(|owner| named(owner))
            .collect();
        owners.extend(
            asked
                .iter()
                .filter_map(|item| item.split_once('/'))
                .map(|(o, _)| o),
        );
        owners.sort_unstable();
        owners.dedup();
        let owners = owners.into_iter().flat_map(|owner| ["--owner", owner]);
        let items = asked.iter().flat_map(|item| ["--item", item.as_str()]);
        owners.chain(items).map(OsString::from).collect()
    }

    /// Says what a refresh saw at `now`.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn say(&self, seen: &Seen, now: u64, pid: u32) -> Result<(), String> {
        let told = told(seen, &self.owners, &self.login, now);
        self.speaker
            .say_with(told, EVERY, pid, self.items(seen, told.state))
    }
}

#[cfg(test)]
mod tests;
