//! The harness's Slack sign-in, read and never written: from the
//! keyring's program while it holds one, else the credentials file, with
//! the rest taken after the keyring runs over or is locked, and the
//! sign-in last read standing meanwhile, at most [`STANDS`].

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

/// The harness's Slack sign-in, as its credentials file keeps it: the
/// token Slack's own client uses, with the cookie it goes with, and the
/// workspace it's for.
#[derive(Clone, Debug, Deserialize)]
pub struct Account {
    #[serde(rename = "accessToken")]
    pub token: String,
    pub cookie: String,
    #[serde(rename = "teamId")]
    pub team: String,
    /// Joel's own id in Slack, which tells his messages from theirs.
    #[serde(rename = "userId", default)]
    pub user: String,
}

#[derive(Deserialize)]
struct File {
    token: Account,
}

/// How long a program rests from the keyring once its sign-in ran over
/// or was locked: long enough that a keyring's unlock prompt isn't raised
/// again each round, and with the 30 seconds it ran and pineapple's Slack reader's
/// 30-second round, four minutes, leaving one of the five a stop said in
/// Slack is to land in for its search and the enforcer.
pub const REST: Duration = Duration::from_mins(3);

/// When this program's keyring sign-in last ran over or was locked,
/// shared by every read of it, so no caller asks again during the rest.
static RAN_OVER_AT: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);

/// The sign-in this program's `read` last read.
static LAST_READ: std::sync::Mutex<LastSignIn> = std::sync::Mutex::new(LastSignIn::new());

/// How long a sign-in read stands while the keyring runs over or rests:
/// long past one rest, so a keyring's unlock prompt goes unnoticed, and
/// short of a keyring locked on purpose leaving Slack open for good.
pub const STANDS: Duration = Duration::from_mins(15);

/// The sign-in a program last read, which stands while the keyring runs
/// over or rests, at most [`STANDS`] after it was read: a locked keyring
/// doesn't end a sign-in.
pub struct LastSignIn {
    last: Option<(Account, Instant)>,
}

impl Default for LastSignIn {
    fn default() -> Self {
        LastSignIn::new()
    }
}

impl LastSignIn {
    /// None read yet.
    #[must_use]
    pub const fn new() -> LastSignIn {
        LastSignIn { last: None }
    }

    /// Whether a sign-in read stands at `now`.
    #[must_use]
    pub fn stands(&self, now: Instant) -> bool {
        self.last
            .as_ref()
            .is_some_and(|(_, read)| now.saturating_duration_since(*read) < STANDS)
    }

    /// Whether a sign-in was read and no longer stands at `now`, so the
    /// next run-over or rest lets it go: the moment to say so.
    #[must_use]
    pub fn lapsed(&self, now: Instant) -> bool {
        self.last.is_some() && !self.stands(now)
    }

    /// The sign-in `signed` holds, kept as read at `now`; or, while the
    /// keyring runs over or rests, the one kept, while it stands; or none,
    /// once neither the keyring nor the file holds one.
    pub fn heard(&mut self, signed: Signed, now: Instant) -> Option<Account> {
        match signed {
            Signed::In(account) => {
                self.last = Some((account.clone(), now));
                Some(account)
            }
            Signed::RanOver | Signed::Locked | Signed::Resting if self.stands(now) => {
                self.last.as_ref().map(|(account, _)| account.clone())
            }
            Signed::RanOver | Signed::Locked | Signed::Resting | Signed::Out => {
                self.last = None;
                None
            }
        }
    }
}

/// What the harness's sign-in came to.
pub enum Signed {
    /// Joel's sign-in.
    In(Account),
    /// None, in the keyring or the file.
    Out,
    /// The keyring's program ran past [`sign_in_program::PATIENCE`] and
    /// was stopped, just now.
    RanOver,
    /// The keyring's program said its keyring is locked, its unlock prompt
    /// dismissed or with no prompter to show it, just now.
    Locked,
    /// It ran over or was locked less than [`REST`] ago, so the keyring
    /// wasn't asked.
    Resting,
}

/// Which it came to, never the sign-in itself, so no part of one reaches
/// a journal or a failing case's words.
impl std::fmt::Debug for Signed {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Signed::In(_) => "In(..)",
            Signed::Out => "Out",
            Signed::RanOver => "RanOver",
            Signed::Locked => "Locked",
            Signed::Resting => "Resting",
        })
    }
}

impl Account {
    /// The harness's sign-in, as `signed` reads it, or none. While the
    /// keyring runs over or rests, the one this program last read, while
    /// it stands; a run-over or a locked keyring is said in the journal.
    pub fn read(path: &Path) -> Option<Account> {
        let keyring = std::env::var_os("SIGN_IN_KEYRING").map(PathBuf::from);
        let (account, said) = Account::read_from(
            keyring.as_deref(),
            path,
            &RAN_OVER_AT,
            &LAST_READ,
            sign_in_program::PATIENCE,
        );
        if let Some(said) = said {
            eprintln!("<4>{said}");
        }
        account
    }

    /// As `read`, the keyring's program `keyring` given `patience`, resting
    /// under `at` and keeping the last sign-in in `last`: the sign-in, and
    /// the line to say of how the keyring answered, if any.
    fn read_from(
        keyring: Option<&Path>,
        path: &Path,
        at: &std::sync::Mutex<Option<Instant>>,
        last: &std::sync::Mutex<LastSignIn>,
        patience: Duration,
    ) -> (Option<Account>, Option<String>) {
        let signed = rested(at, REST, || Account::kept_within(keyring, path, patience));
        let now = Instant::now();
        let mut kept = last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let waits = matches!(signed, Signed::RanOver | Signed::Locked | Signed::Resting);
        let said = if waits && kept.lapsed(now) {
            Some(no_longer_stands())
        } else {
            match signed {
                Signed::RanOver => Some(ran_over(kept.stands(now))),
                Signed::Locked => Some(locked(kept.stands(now), REST)),
                Signed::In(_) | Signed::Out | Signed::Resting => None,
            }
        };
        (kept.heard(signed, now), said)
    }

    /// The harness's sign-in, as `kept` reads it, the keyring's printed by
    /// the program `SIGN_IN_KEYRING` names, as each program's unit says,
    /// its program given `patience`, and never asked for during the `rest`
    /// after it ran over or was locked.
    pub fn signed(path: &Path, patience: Duration, rest: Duration) -> Signed {
        let keyring = std::env::var_os("SIGN_IN_KEYRING").map(PathBuf::from);
        rested(&RAN_OVER_AT, rest, || {
            Account::kept_within(keyring.as_deref(), path, patience)
        })
    }

    /// The harness's sign-in: the keyring's, printed by the program
    /// `keyring`, where pi has kept it since moving sign-ins there, while
    /// it holds one; else the file at `path`, as pi kept it before. None
    /// when neither holds one: then the program says nothing of anyone.
    pub fn kept(keyring: Option<&Path>, path: &Path) -> Option<Account> {
        match Account::kept_within(keyring, path, sign_in_program::PATIENCE) {
            Signed::In(account) => Some(account),
            Signed::Out | Signed::RanOver | Signed::Locked | Signed::Resting => None,
        }
    }

    /// As `kept`, the keyring's program given `patience`. One that ran over
    /// isn't read past to the file, whose sign-in is older, so the caller
    /// hears it ran over, nor is one that's locked.
    fn kept_within(keyring: Option<&Path>, path: &Path, patience: Duration) -> Signed {
        use sign_in_program::SignIn;
        let printed =
            match keyring.map(|program| sign_in_program::token(program.as_os_str(), patience)) {
                Some(SignIn::Token(text)) => Some(text),
                Some(SignIn::RanOver) => return Signed::RanOver,
                Some(SignIn::Locked) => return Signed::Locked,
                Some(SignIn::Missing) | None => None,
            };
        let Some(text) = printed.or_else(|| std::fs::read_to_string(path).ok()) else {
            return Signed::Out;
        };
        serde_json::from_str::<File>(&text).map_or(Signed::Out, |file| Signed::In(file.token))
    }
}

/// What `ask` gives, unless its last answer under `at` ran over or was
/// locked less than `rest` ago; one that does now starts the rest, so a
/// dismissed unlock prompt isn't raised again each round. `at` is held across
/// the ask, so a locked keyring is asked once however many callers want
/// it, each prompting for its unlock; the others wait, at most a sign-in's
/// patience, then rest.
fn rested(
    at: &std::sync::Mutex<Option<Instant>>,
    rest: Duration,
    ask: impl FnOnce() -> Signed,
) -> Signed {
    let mut at = at.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if at.is_some_and(|when| when.elapsed() < rest) {
        return Signed::Resting;
    }
    let signed = ask();
    *at = matches!(signed, Signed::RanOver | Signed::Locked).then(Instant::now);
    signed
}

/// The line `read` says when the keyring ran over: whether the sign-in
/// last read `stands` meanwhile or Slack reads as signed out.
fn ran_over(stands: bool) -> String {
    let meanwhile = if stands {
        "the sign-in last read stands meanwhile"
    } else {
        "with no sign-in standing, Slack reads as signed out till then"
    };
    format!(
        "the keyring's Slack sign-in ran past {} seconds and was stopped, so it isn't asked \
         again for {} minutes, and {meanwhile}",
        sign_in_program::PATIENCE.as_secs(),
        REST.as_secs().div_ceil(60)
    )
}

/// The line said when the keyring's program said its keyring is locked:
/// whether the sign-in last read `stands` meanwhile, and the `rest` till
/// it's asked again.
#[must_use]
pub fn locked(stands: bool, rest: Duration) -> String {
    let meanwhile = if stands {
        "the sign-in last read stands meanwhile"
    } else {
        "with no sign-in standing, Slack reads as signed out till then"
    };
    format!(
        "the keyring holding the Slack sign-in is locked, its unlock prompt dismissed or not \
         shown, so it isn't asked again for {} minutes, and {meanwhile}",
        rest.as_secs().div_ceil(60)
    )
}

/// The line said as a sign-in read [`STANDS`] ago is let go while the
/// keyring still runs over or rests, since nothing else says Slack has
/// stopped being read.
#[must_use]
pub fn no_longer_stands() -> String {
    format!(
        "the Slack sign-in read {} minutes ago no longer stands while the keyring runs over \
         or rests, so Slack isn't read till the keyring answers",
        STANDS.as_secs() / 60
    )
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
