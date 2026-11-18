//! Who Slack knows by an email address, as a library the program
//! (main.rs) and its tests share: the harness's Slack sign-in, read and
//! never written ([`Account`]), one look-up by address ([`Slack`]), said
//! to the shell as one JSON line ([`Found`]), and who each address was
//! found to be, kept between runs ([`Kept`]), with their direct messages
//! with Joel ([`Conversation`]), which are never kept.
//!
//! It only ever reads: `users.lookupByEmail`, and where a workspace
//! refuses that to the sign-in, `search.messages` and `users.info`, then
//! `users.info` and `users.getPresence` for how they are, and
//! `search.messages` for their conversation, are the methods it calls,
//! and nothing it does sends a message or changes anything in Slack.
//!
//! What in Slack wants Joel, for Now, is [`unreads`]'s, which the
//! slack-unreads program (src/bin/slack-unreads.rs) serves for the shell
//! to watch, and searching Slack when Joel asks is
//! [`search`]'s, which that program answers over the socket it holds as
//! Slack's daemon ([`daemon`]), beside each message wanting Joel, held
//! until he's read it in Slack ([`messages`]).

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ureq::Agent;

pub mod daemon;
pub mod message_rows;
pub mod messages;
pub mod person;
pub mod posted;
pub mod rows;
pub mod search;
pub mod unreads;

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

/// How long a program rests from the keyring once its sign-in ran over:
/// long enough that a keyring's unlock prompt isn't raised again each
/// round, and with the 30 seconds it ran and pineapple's Slack reader's
/// 30-second round, four minutes, leaving one of the five a stop said in
/// Slack is to land in for its search and the enforcer.
pub const REST: Duration = Duration::from_mins(3);

/// When this program's keyring sign-in last ran over, shared by every
/// read of it, so no caller asks again during the rest.
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

    /// The sign-in `signed` holds, kept as read at `now`; or, while the
    /// keyring runs over or rests, the one kept, while it stands; or none,
    /// once neither the keyring nor the file holds one.
    pub fn heard(&mut self, signed: Signed, now: Instant) -> Option<Account> {
        match signed {
            Signed::In(account) => {
                self.last = Some((account.clone(), now));
                Some(account)
            }
            Signed::RanOver | Signed::Resting if self.stands(now) => {
                self.last.as_ref().map(|(account, _)| account.clone())
            }
            Signed::RanOver | Signed::Resting | Signed::Out => {
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
    /// It ran over less than [`REST`] ago, so the keyring wasn't asked.
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
            Signed::Resting => "Resting",
        })
    }
}

impl Account {
    /// The harness's sign-in, as `signed` reads it, or none. While the
    /// keyring runs over or rests, the one this program last read, while
    /// it stands; a run-over is said in the journal.
    pub fn read(path: &Path) -> Option<Account> {
        let signed = Account::signed(path, sign_in_program::PATIENCE, REST);
        let now = Instant::now();
        let mut kept = LAST_READ
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(signed, Signed::RanOver) {
            eprintln!("<4>{}", ran_over(kept.stands(now)));
        }
        kept.heard(signed, now)
    }

    /// The harness's sign-in, as `kept` reads it, the keyring's printed by
    /// the program `SIGN_IN_KEYRING` names, as each program's unit says,
    /// its program given `patience`, and never asked for during the `rest`
    /// after it ran over.
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
            Signed::Out | Signed::RanOver | Signed::Resting => None,
        }
    }

    /// As `kept`, the keyring's program given `patience`. One that ran over
    /// isn't read past to the file, whose sign-in is older, so the caller
    /// hears it ran over.
    fn kept_within(keyring: Option<&Path>, path: &Path, patience: Duration) -> Signed {
        use sign_in_program::SignIn;
        let printed =
            match keyring.map(|program| sign_in_program::token(program.as_os_str(), patience)) {
                Some(SignIn::Token(text)) => Some(text),
                Some(SignIn::RanOver) => return Signed::RanOver,
                Some(SignIn::Missing) | None => None,
            };
        let Some(text) = printed.or_else(|| std::fs::read_to_string(path).ok()) else {
            return Signed::Out;
        };
        serde_json::from_str::<File>(&text).map_or(Signed::Out, |file| Signed::In(file.token))
    }
}

/// What `ask` gives, unless its last answer under `at` ran over less than
/// `rest` ago; one that runs over now starts the rest. `at` is held across
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
    *at = matches!(signed, Signed::RanOver).then(Instant::now);
    signed
}

/// The line `read` says when the keyring ran over: whether the sign-in
/// last read `stands` meanwhile or Slack reads as signed out.
fn ran_over(stands: bool) -> String {
    let meanwhile = if stands {
        "the sign-in last read stands meanwhile"
    } else {
        "with no sign-in read yet, Slack reads as signed out till then"
    };
    format!(
        "the keyring's Slack sign-in ran past {} seconds and was stopped, so it isn't asked \
         again for {} minutes, and {meanwhile}",
        sign_in_program::PATIENCE.as_secs(),
        REST.as_secs().div_ceil(60)
    )
}

/// What Slack said of an address, as the shell is told it: who it is in
/// Slack and its workspace, which open them there, their name, and the
/// time zone their profile keeps, by tzdata's name; or that Slack knows
/// nobody by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Found {
    pub email: String,
    /// Their id in Slack, or none for an address Slack knows nobody by.
    pub id: Option<String>,
    pub team: String,
    pub name: String,
    /// Their zone, as "America/Toronto", or "" when Slack keeps none.
    pub zone: String,
    /// "active", "away" or "in a huddle", or "" until Slack says.
    pub presence: String,
    /// Their status's words while it's set, or "".
    pub status: String,
    /// Their status's emoji as Unicode while it's set, or "" for none or
    /// for a workspace's own picture, which has no Unicode form to show.
    pub emoji: String,
    /// Their handle, as "ada", which their conversation is found by, or
    /// "" for nobody.
    #[serde(default)]
    pub handle: String,
    /// Their title as their profile says it, "Staff Engineer", or ""
    /// where it says none; who they are, so it's kept with their name.
    /// Left out of what's said where it's "", so a person without one is
    /// said as before.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Their direct messages with Joel, while Slack finds any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Conversation>,
}

/// Someone's direct messages with Joel, as one thing: where it is in
/// Slack, a link to its newest message, that message's first words,
/// which name it, and when it was said, with its last few messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    /// The direct message's id, as "D1", which opens it in Slack.
    pub channel: String,
    pub link: String,
    pub last: String,
    /// In milliseconds since the epoch, as the shell keeps time.
    pub at: u64,
    /// Oldest first, so the newest is last, as they're read.
    pub messages: Vec<Message>,
}

/// One message in a conversation, in plain words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Whether Joel said it.
    pub mine: bool,
    pub text: String,
    pub at: u64,
}

impl Found {
    /// Who they are without how they are, as it's kept: whether they're
    /// active, their status and what they said are only ever as fresh as
    /// Slack's answer.
    #[must_use]
    pub fn who(&self) -> Found {
        Found {
            presence: String::new(),
            status: String::new(),
            emoji: String::new(),
            conversation: None,
            ..self.clone()
        }
    }
}

/// The methods [`Slack::read`] asks for another module: each only reads.
pub const READS: [&str; 4] = [
    "search.messages",
    "conversations.replies",
    "conversations.info",
    "users.info",
];

/// How long knowing nobody by an address holds before Slack is asked
/// again, in seconds: someone new to Slack, or saying something there for
/// the first time, is found within a day, with no restart.
pub const NOBODY_HOLDS: u64 = 24 * 60 * 60;

/// Who each address was found to be, kept in a file of the program's own
/// so a restart of the shell, as each switch brings, forgets nobody:
/// someone known is said at once, before Slack is asked how they are.
/// It holds no presence, status or message, and is only ever a cache:
/// with no file, or one that can't be read or written, everyone is
/// simply asked about again.
pub struct Kept {
    path: Option<PathBuf>,
    by: HashMap<String, KeptOne>,
}

#[derive(Serialize, Deserialize)]
struct KeptOne {
    found: Found,
    /// When it was found, in seconds since the epoch.
    at: u64,
}

impl Kept {
    /// What the file at `path` keeps, or nothing yet.
    #[must_use]
    pub fn at(path: Option<PathBuf>) -> Kept {
        let by = path
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Kept { path, by }
    }

    /// Who `email` was found to be, unless it was nobody long enough ago
    /// to ask again.
    pub fn get(&self, email: &str, now: u64) -> Option<&Found> {
        self.by
            .get(email)
            .filter(|kept| kept.found.id.is_some() || now < kept.at + NOBODY_HOLDS)
            .map(|kept| &kept.found)
    }

    /// Who `email` is, as `held` while running says, over what's kept.
    /// Knowing nobody there lapses as it does kept, so someone who joins
    /// Slack after they were asked about is found without a restart.
    #[must_use]
    pub fn over(&self, held: Option<&Found>, email: &str, now: u64) -> Option<Found> {
        match held {
            Some(held) if held.id.is_some() => Some(held.clone()),
            _ => self.get(email, now).cloned(),
        }
    }

    /// Keeps who someone was found to be, writing the file anew whole, so
    /// it's never read half written.
    pub fn keep(&mut self, found: &Found, now: u64) {
        let who = found.who();
        // Nobody found there again holds from now, so it's asked a day on
        // rather than at every step-in once the first day is out.
        if who.id.is_some()
            && self
                .by
                .get(&who.email)
                .is_some_and(|kept| kept.found == who)
        {
            return;
        }
        self.by.insert(
            who.email.clone(),
            KeptOne {
                found: who,
                at: now,
            },
        );
        let Some(path) = &self.path else { return };
        let Ok(text) = serde_json::to_string(&self.by) else {
            return;
        };
        let written = path.with_extension("new");
        let wrote = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&written, text))
            .and_then(|()| std::fs::rename(&written, path));
        if let Err(error) = wrote {
            eprintln!("<4>couldn't keep who was found: {error}");
        }
    }
}

/// Why Slack said nothing of an address.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Slack won't take the sign-in any more: signing in again from pi
    /// mends it, and until then nobody is asked about.
    SignedOut,
    /// Slack couldn't be reached, or answered with an error of its own;
    /// asking again later may work.
    Unreachable(String),
}

/// How long Slack is left alone after refusing for its limit without
/// saying for how long.
const WAIT_UNSAID: Duration = Duration::from_secs(30);

/// The longest wait taken from Slack's word, so a garbled header never
/// silences Slack for a day.
const WAIT_MOST: Duration = Duration::from_secs(600);

/// How long Slack asked to be left alone, from a 429's `Retry-After`:
/// whole seconds, as Slack sends it, up to [`WAIT_MOST`], or
/// [`WAIT_UNSAID`] when it's missing or not a number of seconds.
pub fn retry_after(header: Option<&str>) -> Duration {
    header
        .and_then(|seconds| seconds.trim().parse::<u64>().ok())
        .map_or(WAIT_UNSAID, |seconds| {
            Duration::from_secs(seconds).min(WAIT_MOST)
        })
}

/// Slack's API, at an origin given rather than fixed, so the tests can
/// ask a stand-in on this machine.
pub struct Slack {
    agent: Agent,
    origin: String,
    /// Searches asked since [`Slack::searched`] was last asked, which a
    /// grid allows only about 20 of a minute.
    searches: Cell<u32>,
    /// Until when Slack asked, with a 429, to be asked nothing more.
    quiet_until: Cell<Option<Instant>>,
}

impl Slack {
    pub fn new(origin: impl Into<String>) -> Slack {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .http_status_as_error(false)
            .build()
            .into();
        Slack {
            agent,
            origin: origin.into(),
            searches: Cell::new(0),
            quiet_until: Cell::new(None),
        }
    }

    /// How long is left of the wait Slack last asked for, or none once
    /// it may be asked again.
    pub fn quiet(&self) -> Option<Duration> {
        let left = self
            .quiet_until
            .get()?
            .saturating_duration_since(Instant::now());
        if left.is_zero() {
            self.quiet_until.set(None);
            return None;
        }
        Some(left)
    }

    /// How many searches were asked since this was last asked.
    pub fn searched(&self) -> u32 {
        self.searches.replace(0)
    }

    /// Their direct messages with Joel, for someone found: a search in
    /// their conversation, newest first, taking only what's in a direct
    /// message with them, since a search names each match's channel.
    pub fn conversation(
        &self,
        account: &Account,
        found: &Found,
    ) -> Result<Option<Conversation>, Refused> {
        let Some(id) = &found.id else {
            return Ok(None);
        };
        if found.handle.is_empty() {
            return Ok(None);
        }
        let said = self.ask(
            account,
            "search.messages",
            &[
                ("query", &format!("in:@{}", found.handle)),
                ("count", "5"),
                ("sort", "timestamp"),
                ("sort_dir", "desc"),
            ],
        )?;
        conversation(&said, id, &found.name, &account.user)
    }

    /// Who Slack knows by `email` in the account's workspace.
    ///
    /// An enterprise grid refuses the look-up by address to the browser's
    /// sign-in the harness keeps, so there they're found by a message from
    /// the handle their address starts with, and taken only when Slack's
    /// profile for whoever that is gives the same address. Someone who has
    /// never said anything Joel can see isn't found that way.
    pub fn look_up(&self, account: &Account, email: &str) -> Result<Found, Refused> {
        let said = self.ask(account, "users.lookupByEmail", &[("email", email)])?;
        if !refused_to_the_sign_in(&said) {
            return found(&said, email, &account.team);
        }
        let handle = email.split_once('@').map_or(email, |(handle, _)| handle);
        let searched = self.ask(
            account,
            "search.messages",
            &[("query", &format!("from:@{handle}")), ("count", "1")],
        )?;
        let Some(id) = speaker(&searched, handle)? else {
            return found(&nobody(), email, &account.team);
        };
        let user = self.ask(account, "users.info", &[("user", &id)])?;
        if addressed(&user) == Some(email.to_lowercase()) {
            found(&user, email, &account.team)
        } else {
            found(&nobody(), email, &account.team)
        }
    }

    /// Who they are now, for someone already found: Slack asked again by
    /// their id, so a name, a zone or a status changed since is taken.
    pub fn again(&self, account: &Account, known: &Found) -> Result<Found, Refused> {
        let Some(id) = &known.id else {
            return Ok(known.clone());
        };
        let user = self.ask(account, "users.info", &[("user", id)])?;
        found(&user, &known.email, &known.team)
    }

    /// Whether someone is active in Slack now or away, as
    /// `users.getPresence` says, or "" when it says neither.
    pub fn presence(&self, account: &Account, id: &str) -> Result<String, Refused> {
        let said = self.ask(account, "users.getPresence", &[("user", id)])?;
        if said.get("ok").and_then(Value::as_bool) != Some(true) {
            return found(&said, "", "").map(|_| String::new());
        }
        Ok(match said.get("presence").and_then(Value::as_str) {
            Some(word @ ("active" | "away")) => word.to_owned(),
            _ => String::new(),
        })
    }

    /// What Slack answers to one of the methods that only read, for a
    /// program of another module that wants Slack's own answer, as the
    /// safe word's reader does (../pineapple): a message's text as Slack
    /// keeps it, tags by id and all, which [`plain`] turns into names.
    ///
    /// # Errors
    ///
    /// Why Slack said nothing, and [`Refused::Unreachable`] for any method
    /// not in [`READS`], since nothing here writes to Slack.
    pub fn read(
        &self,
        account: &Account,
        method: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, Refused> {
        if !READS.contains(&method) {
            return Err(Refused::Unreachable(format!(
                "{method} isn't one of the methods that only read"
            )));
        }
        self.ask(account, method, query)
    }

    /// What Slack answers to one method, asked with the sign-in.
    pub(crate) fn ask(
        &self,
        account: &Account,
        method: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, Refused> {
        // Asking while Slack asked for a wait only lengthens it, so it's
        // said busy, as Slack would, with nothing sent.
        if self.quiet().is_some() {
            return Err(Refused::Unreachable(search::BUSY.to_owned()));
        }
        if method == "search.messages" {
            self.searches.set(self.searches.get() + 1);
        }
        let mut asked = self
            .agent
            .get(format!("{}/api/{method}", self.origin))
            .header("Authorization", &format!("Bearer {}", account.token))
            .header("Cookie", &format!("d={}", account.cookie));
        for (name, value) in query {
            asked = asked.query(*name, *value);
        }
        let mut answer = asked
            .call()
            .map_err(|error| Refused::Unreachable(format!("couldn't reach Slack: {error}")))?;
        let status = answer.status().as_u16();
        if status == 429 {
            let asked = answer
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok());
            self.quiet_until
                .set(Some(Instant::now() + retry_after(asked)));
            return Err(Refused::Unreachable(search::BUSY.to_owned()));
        }
        if status != 200 {
            return Err(Refused::Unreachable(format!("Slack answered {status}")));
        }
        answer
            .body_mut()
            .read_json()
            .map_err(|error| Refused::Unreachable(format!("Slack's answer: {error}")))
    }
}

/// Slack's answer for an address it knows nobody by.
fn nobody() -> Value {
    serde_json::json!({ "ok": false, "error": "users_not_found" })
}

/// Whether Slack refused a method to this kind of sign-in, as an
/// enterprise grid refuses the look-up by address to a browser's.
pub fn refused_to_the_sign_in(said: &Value) -> bool {
    matches!(
        said.get("error").and_then(Value::as_str),
        Some("not_allowed_token_type" | "enterprise_is_restricted" | "missing_scope")
    )
}

/// Who said the message a search for `from:@handle` found, when it was
/// that handle who said it, or none when the search found nothing of
/// theirs.
pub fn speaker(said: &Value, handle: &str) -> Result<Option<String>, Refused> {
    if said.get("ok").and_then(Value::as_bool) != Some(true) {
        // Refusals read as a look-up's do: signed out, or try later.
        return found(said, "", "").map(|_| None);
    }
    let matches = said.pointer("/messages/matches").and_then(Value::as_array);
    Ok(matches.into_iter().flatten().find_map(|message| {
        let by = message.get("username").and_then(Value::as_str)?;
        let id = message.get("user").and_then(Value::as_str)?;
        by.eq_ignore_ascii_case(handle).then(|| id.to_owned())
    }))
}

/// The conversation a search in someone's direct messages found,
/// newest first as asked, or none when it found nothing said there. `id`
/// and `name` are theirs, `me` is Joel's id.
pub fn conversation(
    said: &Value,
    id: &str,
    name: &str,
    me: &str,
) -> Result<Option<Conversation>, Refused> {
    if said.get("ok").and_then(Value::as_bool) != Some(true) {
        return found(said, "", "").map(|_| None);
    }
    let names = [(id, name)];
    let matches = said.pointer("/messages/matches").and_then(Value::as_array);
    let mut newest: Option<(String, String)> = None;
    let mut messages: Vec<Message> = Vec::new();
    for one in matches.into_iter().flatten() {
        let channel = one.get("channel").unwrap_or(&Value::Null);
        let theirs = channel.get("is_im").and_then(Value::as_bool) == Some(true)
            && channel.get("name").and_then(Value::as_str) == Some(id);
        let text = plain(
            one.get("text").and_then(Value::as_str).unwrap_or_default(),
            &names,
        );
        let Some(at) = one.get("ts").and_then(Value::as_str).and_then(millis) else {
            continue;
        };
        if !theirs || text.is_empty() {
            continue;
        }
        if newest.is_none() {
            let Some(channel) = channel.get("id").and_then(Value::as_str) else {
                continue;
            };
            let link = one
                .get("permalink")
                .and_then(Value::as_str)
                .unwrap_or_default();
            newest = Some((channel.to_owned(), link.to_owned()));
        }
        let mine = !me.is_empty() && one.get("user").and_then(Value::as_str) == Some(me);
        messages.push(Message { mine, text, at });
    }
    let Some((channel, link)) = newest else {
        return Ok(None);
    };
    messages.reverse();
    let last = messages.last().map_or(String::new(), |message| {
        cut(message.text.lines().next().unwrap_or_default(), FIRST_WORDS)
    });
    let at = messages.last().map_or(0, |message| message.at);
    Ok(Some(Conversation {
        channel,
        link,
        last,
        at,
        messages,
    }))
}

/// How many characters a conversation's name, its newest message's first
/// words, runs to at most.
pub(crate) const FIRST_WORDS: usize = 60;
/// How many characters of a message the drawer shows at most.
pub(crate) const LONGEST: usize = 400;

/// A message in plain words, as Slack's markup would show it: a link by
/// its label, or itself without `mailto:`; someone mentioned by name, by
/// `names` (id and name), or as someone; a channel by its name; `@here`
/// and its like by their word; then `&lt;`, `&gt;` and `&amp;` as
/// themselves, cut at a word to [`LONGEST`].
pub fn plain(text: &str, names: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        out.push_str(&marked(&after[..end], names));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    let out = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    cut(out.trim(), LONGEST)
}

/// What one piece of Slack's markup, between `<` and `>`, shows as.
fn marked(inside: &str, names: &[(&str, &str)]) -> String {
    let (target, label) = match inside.split_once('|') {
        Some((target, label)) => (target, Some(label)),
        None => (inside, None),
    };
    if let Some(user) = target.strip_prefix('@') {
        let name = label
            .or_else(|| {
                names
                    .iter()
                    .find(|(id, _)| *id == user)
                    .map(|(_, name)| *name)
            })
            .filter(|name| !name.is_empty())
            .unwrap_or("someone");
        return format!("@{}", name.trim_start_matches('@'));
    }
    if let Some(channel) = target.strip_prefix('#') {
        return format!("#{}", label.unwrap_or(channel).trim_start_matches('#'));
    }
    if let Some(special) = target.strip_prefix('!') {
        let word = special.split('^').next().unwrap_or(special);
        return format!("@{}", label.unwrap_or(word).trim_start_matches('@'));
    }
    label.map_or_else(
        || target.strip_prefix("mailto:").unwrap_or(target).to_owned(),
        str::to_owned,
    )
}

/// `text` cut at a word to at most `most` characters.
pub(crate) fn cut(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_owned();
    }
    let upto: String = text.chars().take(most).collect();
    upto.rfind(char::is_whitespace)
        .map_or(upto.as_str(), |space| &upto[..space])
        .trim_end()
        .to_owned()
}

/// When a message was said, from Slack's `ts`, as "1730312345.123456",
/// in milliseconds since the epoch.
pub fn millis(ts: &str) -> Option<u64> {
    let (seconds, fraction) = ts.split_once('.').unwrap_or((ts, ""));
    let seconds: u64 = seconds.parse().ok()?;
    let thousandths: String = fraction.chars().chain("000".chars()).take(3).collect();
    Some(seconds * 1000 + thousandths.parse::<u64>().ok()?)
}

/// The address a profile Slack gave keeps, as written in lower case.
pub fn addressed(said: &Value) -> Option<String> {
    said.pointer("/user/profile/email")
        .and_then(Value::as_str)
        .map(str::to_lowercase)
}

/// Their status as a profile Slack gave keeps it, while it's set: its
/// words, and its emoji as Unicode (VKCF3Z, 67f6cbac). `now` is in seconds
/// since the epoch, as Slack says when a status ends, 0 for never.
pub fn status(profile: &Value, now: u64) -> (String, String) {
    let until = profile
        .get("status_expiration")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if until != 0 && until <= now {
        return (String::new(), String::new());
    }
    let words = profile
        .get("status_text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let emoji = profile
        .pointer("/status_emoji_display_info/0/unicode")
        .and_then(Value::as_str)
        .and_then(unicode)
        .unwrap_or_default();
    (words.to_owned(), emoji)
}

/// An emoji as Slack writes its code points, in hex joined by `-`, or
/// none when one isn't a character.
fn unicode(points: &str) -> Option<String> {
    points
        .split('-')
        .map(|point| u32::from_str_radix(point, 16).ok().and_then(char::from_u32))
        .collect()
}

/// Seconds since the epoch, as Slack keeps when a status ends.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// What Slack's answer to a look-up says.
pub fn found(said: &Value, email: &str, team: &str) -> Result<Found, Refused> {
    let nobody = || Found {
        email: email.to_owned(),
        id: None,
        team: team.to_owned(),
        name: String::new(),
        zone: String::new(),
        presence: String::new(),
        status: String::new(),
        emoji: String::new(),
        handle: String::new(),
        title: String::new(),
        conversation: None,
    };
    if said.get("ok").and_then(Value::as_bool) == Some(true) {
        let user = said.get("user").unwrap_or(&Value::Null);
        let text = |pointer: &str| {
            user.pointer(pointer)
                .and_then(Value::as_str)
                .filter(|said| !said.is_empty())
        };
        let Some(id) = text("/id") else {
            return Ok(nobody());
        };
        let name = text("/profile/real_name")
            .or_else(|| text("/real_name"))
            .or_else(|| text("/profile/display_name"))
            .unwrap_or_default();
        let (status, emoji) = status(user.get("profile").unwrap_or(&Value::Null), now());
        // In a huddle says more than active, so it's said in its place.
        let huddle = text("/profile/huddle_state") == Some("in_a_huddle");
        return Ok(Found {
            email: email.to_owned(),
            id: Some(id.to_owned()),
            team: text("/team_id").unwrap_or(team).to_owned(),
            name: name.to_owned(),
            zone: text("/tz").unwrap_or_default().to_owned(),
            presence: if huddle { "in a huddle" } else { "" }.to_owned(),
            status,
            emoji,
            handle: text("/name").unwrap_or_default().to_owned(),
            title: text("/profile/title").unwrap_or_default().to_owned(),
            conversation: None,
        });
    }
    match said.get("error").and_then(Value::as_str).unwrap_or("") {
        "users_not_found" => Ok(nobody()),
        "invalid_auth" | "not_authed" | "account_inactive" | "token_revoked" | "token_expired" => {
            Err(Refused::SignedOut)
        }
        other => Err(Refused::Unreachable(format!(
            "Slack refused the look-up: {other}"
        ))),
    }
}

/// Whether an address is worth asking Slack about: one `@` with
/// something either side, and nothing that would read as more than one.
pub fn asked(email: &str) -> bool {
    let Some((name, host)) = email.split_once('@') else {
        return false;
    };
    !name.is_empty()
        && host.contains('.')
        && !host.contains('@')
        && !email.chars().any(|c| c.is_whitespace() || c == ',')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slack_saying_how_long_is_waited_that_long() {
        assert_eq!(retry_after(Some("12")), Duration::from_secs(12));
        assert_eq!(retry_after(Some(" 3 ")), Duration::from_secs(3));
    }

    #[test]
    fn slack_saying_nothing_readable_is_waited_half_a_minute() {
        assert_eq!(retry_after(None), WAIT_UNSAID);
        assert_eq!(retry_after(Some("soon")), WAIT_UNSAID);
        assert_eq!(
            retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")),
            WAIT_UNSAID
        );
    }

    #[test]
    fn slack_asking_for_hours_is_waited_ten_minutes_at_most() {
        assert_eq!(retry_after(Some("86400")), WAIT_MOST);
    }

    #[test]
    fn someone_found_is_said_by_their_id_and_real_name() {
        let said = json!({ "ok": true, "user": { "id": "U1", "team_id": "T9",
            "real_name": "Ada L", "tz": "Europe/London",
            "profile": { "real_name": "Ada Lovelace", "title": "Analyst" } } });
        assert_eq!(
            found(&said, "ada@example.com", "T1"),
            Ok(Found {
                email: "ada@example.com".into(),
                id: Some("U1".into()),
                team: "T9".into(),
                name: "Ada Lovelace".into(),
                zone: "Europe/London".into(),
                presence: String::new(),
                status: String::new(),
                emoji: String::new(),
                handle: String::new(),
                title: "Analyst".into(),
                conversation: None,
            })
        );
        let handled = json!({ "ok": true, "user": { "id": "U1", "name": "ada" } });
        assert_eq!(
            found(&handled, "ada@example.com", "T1").unwrap().handle,
            "ada"
        );
    }

    #[test]
    fn a_conversation_is_their_direct_messages_with_joel_newest_last() {
        let im = json!({ "id": "D1", "is_im": true, "name": "U1" });
        let said = json!({ "ok": true, "messages": { "matches": [
            { "channel": im, "user": "U1", "text": "See you at <!here|here> lunch",
              "ts": "1730312345.123456", "permalink": "https://slack.example/p3" },
            { "channel": { "id": "C1", "is_im": false, "name": "general" },
              "user": "U1", "text": "In a channel", "ts": "1730312300.000000" },
            { "channel": { "id": "D7", "is_im": true, "name": "U7" },
              "user": "U7", "text": "Someone else's", "ts": "1730312290.000000" },
            { "channel": im, "user": "U0", "text": "", "ts": "1730312280.000000" },
            { "channel": im, "user": "U0", "text": "Lunch, <@U1>?",
              "ts": "1730312200.5", "permalink": "https://slack.example/p1" }] } });
        assert_eq!(
            conversation(&said, "U1", "Ada Lovelace", "U0"),
            Ok(Some(Conversation {
                channel: "D1".into(),
                link: "https://slack.example/p3".into(),
                last: "See you at @here lunch".into(),
                at: 1_730_312_345_123,
                messages: vec![
                    Message {
                        mine: true,
                        text: "Lunch, @Ada Lovelace?".into(),
                        at: 1_730_312_200_500,
                    },
                    Message {
                        mine: false,
                        text: "See you at @here lunch".into(),
                        at: 1_730_312_345_123,
                    },
                ],
            }))
        );
        let none = json!({ "ok": true, "messages": { "matches": [] } });
        assert_eq!(conversation(&none, "U1", "Ada", "U0"), Ok(None));
        let refused = json!({ "ok": false, "error": "invalid_auth" });
        assert_eq!(
            conversation(&refused, "U1", "Ada", "U0"),
            Err(Refused::SignedOut)
        );
    }

    #[test]
    fn a_message_is_said_in_plain_words() {
        let names = [("U1", "Ada Lovelace")];
        let said = |text| plain(text, &names);
        assert_eq!(
            said("<https://a.example|the plan> &amp; <https://b.example>"),
            "the plan & https://b.example"
        );
        assert_eq!(
            said("write <mailto:ada@example.com|me> or <mailto:g@example.com>"),
            "write me or g@example.com"
        );
        assert_eq!(
            said("<@U1> and <@U2> and <@U3|grace>"),
            "@Ada Lovelace and @someone and @grace"
        );
        assert_eq!(
            said("in <#C1|general> <!channel> <!subteam^S1|@shell>"),
            "in #general @channel @shell"
        );
        assert_eq!(said("1 &lt; 2 &gt; 0"), "1 < 2 > 0");
        assert_eq!(
            said("an open < without its end"),
            "an open < without its end"
        );
        let long = "word ".repeat(100);
        let cut = said(&long);
        assert!(cut.len() <= LONGEST && cut.ends_with("word"), "{cut}");
    }

    #[test]
    fn slacks_times_are_milliseconds() {
        assert_eq!(millis("1730312345.123456"), Some(1_730_312_345_123));
        assert_eq!(millis("1730312345.5"), Some(1_730_312_345_500));
        assert_eq!(millis("1730312345"), Some(1_730_312_345_000));
        assert_eq!(millis("soon"), None);
    }

    #[test]
    fn a_status_is_its_words_and_its_emoji_while_it_lasts() {
        let palm = json!({ "status_text": "Away on the coast ", "status_expiration": 2000,
            "status_emoji_display_info": [{ "emoji_name": "palm_tree", "unicode": "1f334" }] });
        assert_eq!(
            status(&palm, 1000),
            ("Away on the coast".into(), "🌴".into())
        );
        assert_eq!(status(&palm, 2000), (String::new(), String::new()));
        let joined = json!({ "status_text": "Cooking", "status_expiration": 0,
            "status_emoji_display_info": [{ "unicode": "1f469-200d-1f373" }] });
        assert_eq!(
            status(&joined, u64::MAX),
            ("Cooking".into(), "👩\u{200d}🍳".into())
        );
        // A workspace's own picture has no Unicode form, so only the words.
        let own = json!({ "status_text": "Shipping", "status_emoji": ":ship-it:",
            "status_emoji_display_info": [{ "emoji_name": "ship-it",
                "display_url": "https://emoji.example/ship-it.png" }] });
        assert_eq!(status(&own, 0), ("Shipping".into(), String::new()));
        assert_eq!(status(&json!({}), 0), (String::new(), String::new()));
    }

    #[test]
    fn someone_in_a_huddle_is_said_so_in_place_of_active() {
        let said = json!({ "ok": true, "user": { "id": "U1",
            "profile": { "huddle_state": "in_a_huddle" } } });
        assert_eq!(
            found(&said, "a@example.com", "T1").unwrap().presence,
            "in a huddle"
        );
        let not = json!({ "ok": true, "user": { "id": "U1",
            "profile": { "huddle_state": "default_unset" } } });
        assert_eq!(found(&not, "a@example.com", "T1").unwrap().presence, "");
    }

    #[test]
    fn an_address_slack_knows_nobody_by_is_said_so() {
        let said = json!({ "ok": false, "error": "users_not_found" });
        let nobody = found(&said, "x@example.com", "T1").unwrap();
        assert_eq!(nobody.id, None);
        assert_eq!(nobody.team, "T1");
        assert_eq!(nobody.zone, "");
    }

    #[test]
    fn a_sign_in_slack_refuses_is_signed_out() {
        for error in ["invalid_auth", "token_revoked"] {
            let said = json!({ "ok": false, "error": error });
            assert_eq!(found(&said, "x@example.com", "T1"), Err(Refused::SignedOut));
        }
    }

    #[test]
    fn a_search_names_whoever_said_it_only_by_their_own_handle() {
        let said = json!({ "ok": true, "messages": { "matches": [
            { "user": "U7", "username": "someone.else" },
            { "user": "U2", "username": "Grace.Hopper" }] } });
        assert_eq!(speaker(&said, "grace.hopper"), Ok(Some("U2".into())));
        let none = json!({ "ok": true, "messages": { "matches": [] } });
        assert_eq!(speaker(&none, "grace.hopper"), Ok(None));
        let refused = json!({ "ok": false, "error": "invalid_auth" });
        assert_eq!(speaker(&refused, "grace.hopper"), Err(Refused::SignedOut));
    }

    #[test]
    fn only_a_grids_refusal_of_the_sign_in_turns_to_searching() {
        for error in ["not_allowed_token_type", "enterprise_is_restricted"] {
            assert!(refused_to_the_sign_in(
                &json!({ "ok": false, "error": error })
            ));
        }
        for error in ["users_not_found", "invalid_auth", "ratelimited"] {
            assert!(!refused_to_the_sign_in(
                &json!({ "ok": false, "error": error })
            ));
        }
        assert_eq!(
            addressed(
                &json!({ "ok": true, "user": { "profile": { "email": "Grace@Example.com" } } })
            ),
            Some("grace@example.com".into())
        );
    }

    #[test]
    fn who_was_found_is_kept_without_how_they_were_and_nobody_only_a_day() {
        let path = std::env::temp_dir()
            .join(format!("slack-people-kept-{}", std::process::id()))
            .join("known.json");
        let ada = Found {
            email: "ada@example.com".into(),
            id: Some("U1".into()),
            team: "T1".into(),
            name: "Ada Lovelace".into(),
            zone: "Europe/London".into(),
            presence: "active".into(),
            status: "Writing".into(),
            emoji: "📝".into(),
            handle: "ada".into(),
            title: "Analyst".into(),
            conversation: Some(Conversation {
                channel: "D1".into(),
                link: String::new(),
                last: "Hello".into(),
                at: 1,
                messages: Vec::new(),
            }),
        };
        let nobody = found(&nobody(), "x@example.com", "T1").unwrap();
        let mut kept = Kept::at(Some(path.clone()));
        kept.keep(&ada, 1000);
        kept.keep(&nobody, 1000);
        let again = Kept::at(Some(path.clone()));
        assert_eq!(again.get("ada@example.com", u64::MAX), Some(&ada.who()));
        assert_eq!(
            again.get("x@example.com", 1000 + NOBODY_HOLDS - 1),
            Some(&nobody)
        );
        assert_eq!(again.get("x@example.com", 1000 + NOBODY_HOLDS), None);
        assert_eq!(again.get("grace@example.com", 1000), None);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        // With nowhere to keep it, it's only held while the program runs.
        let mut held = Kept::at(None);
        held.keep(&ada, 1000);
        assert_eq!(held.get("ada@example.com", 1000), Some(&ada.who()));
    }

    #[test]
    fn nobody_known_while_running_lapses_after_a_day_as_kept() {
        let nobody = found(&nobody(), "x@example.com", "T1").unwrap();
        let ada = Found {
            email: "ada@example.com".into(),
            id: Some("U1".into()),
            team: "T1".into(),
            name: "Ada Lovelace".into(),
            ..nobody.clone()
        };
        let mut kept = Kept::at(None);
        kept.keep(&nobody, 1000);
        kept.keep(&ada, 1000);
        let day = 24 * 60 * 60;
        assert_eq!(
            kept.over(Some(&nobody), "x@example.com", 1000 + day - 1),
            Some(nobody.clone()),
            "nobody, within the day"
        );
        assert_eq!(
            kept.over(Some(&nobody), "x@example.com", 1000 + day),
            None,
            "asked again a day on, though it's held while running"
        );
        assert_eq!(
            kept.over(Some(&ada), "ada@example.com", u64::MAX),
            Some(ada.clone()),
            "someone found holds"
        );
        // Found to be nobody again, it holds a day from then.
        kept.keep(&nobody, 1000 + day);
        assert_eq!(
            kept.over(Some(&nobody), "x@example.com", 1000 + 2 * day - 1),
            Some(nobody),
            "nobody again, held from when it was found"
        );
    }

    #[test]
    fn a_rate_limit_is_unreachable_for_now() {
        let said = json!({ "ok": false, "error": "ratelimited" });
        assert!(matches!(
            found(&said, "x@example.com", "T1"),
            Err(Refused::Unreachable(_))
        ));
    }

    #[test]
    fn only_a_plain_address_is_asked_about() {
        assert!(asked("ada@example.com"));
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@host",
            "a@b@c.com",
            "a b@c.com",
            "a@b.com,c@d.com",
        ] {
            assert!(!asked(bad), "{bad}");
        }
    }

    /// A sign-in for `token`, as the harness keeps one.
    fn signed_in(token: &str) -> String {
        format!(r#"{{"token": {{"accessToken": "{token}", "cookie": "d", "teamId": "T1"}}}}"#)
    }

    /// A folder of the test's own, holding the file with `file` and a
    /// keyring reader that runs `reader`.
    fn kept_in(test: &str, file: &str, reader: &str) -> (PathBuf, PathBuf) {
        let folder = std::env::temp_dir().join(format!("slack-account-{test}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join("credentials.json");
        std::fs::write(&path, file).expect("the file is written");
        let keyring = folder.join("keyring");
        // Written by a shell of its own, never by this process: a test
        // forking beside this one would hold the reader open for writing
        // until its child ran, and running the reader meanwhile fails as a
        // text file busy, so the file is read over it (seen once in
        // google's like test on a builder, 2026-11-14).
        let written = std::process::Command::new("/bin/sh")
            .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 755 "$1""#, "sh"])
            .arg(&keyring)
            .arg(format!("#!/bin/sh\n{reader}"))
            .status()
            .expect("the reader's shell runs");
        assert!(written.success(), "the reader wasn't written");
        (keyring, path)
    }

    #[test]
    fn the_keyrings_sign_in_is_read_over_the_file() {
        let reader = format!("cat <<'END'\n{}\nEND", signed_in("xoxc-keyring"));
        let (keyring, path) = kept_in("keyring", &signed_in("xoxc-file"), &reader);
        let account = Account::kept(Some(&keyring), &path).expect("the keyring's is read");
        assert_eq!(
            account.token, "xoxc-keyring",
            "the file was read over the keyring"
        );
    }

    #[test]
    fn the_file_is_read_while_the_keyring_holds_no_sign_in() {
        for (test, reader) in [("refused", "exit 1"), ("empty", "exit 0")] {
            let (keyring, path) = kept_in(test, &signed_in("xoxc-file"), reader);
            let account = Account::kept(Some(&keyring), &path).expect("the file's is read");
            assert_eq!(account.token, "xoxc-file", "{test}: the file wasn't read");
        }
        let (_, path) = kept_in("unreadable", &signed_in("xoxc-file"), "");
        let nowhere = Path::new("/nonexistent/keyring");
        assert_eq!(
            Account::kept(Some(nowhere), &path).map(|account| account.token),
            Some("xoxc-file".to_owned()),
            "a keyring that can't be read hid the file"
        );
        let (keyring, _) = kept_in("neither", "", "exit 1");
        let missing = Path::new("/nonexistent/credentials.json");
        assert!(
            Account::kept(Some(&keyring), missing).is_none(),
            "a sign-in from nowhere"
        );
    }

    #[test]
    fn a_keyring_that_never_answers_runs_over_and_isnt_read_past() {
        let patience = Duration::from_millis(300);
        let (keyring, path) = kept_in("hangs", &signed_in("xoxc-file"), "exec sleep 60");
        let (said, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // The case may have given up, and then nobody listens.
            let _ = said.send(Account::kept_within(Some(&keyring), &path, patience));
        });
        let signed = heard
            .recv_timeout(patience + Duration::from_secs(5))
            .expect("a keyring that never answers is never waited on for good");
        assert!(
            matches!(signed, Signed::RanOver),
            "a keyring that never answers ran over, not {signed:?}"
        );
    }

    #[test]
    fn a_sign_in_that_ran_over_isnt_asked_for_again_until_its_rest_is_up() {
        let at = std::sync::Mutex::new(None);
        let asked = Cell::new(0);
        let ask = |answer: Signed| {
            asked.set(asked.get() + 1);
            answer
        };
        let hour = Duration::from_secs(3600);
        let first = rested(&at, hour, || ask(Signed::RanOver));
        assert!(
            matches!(first, Signed::RanOver),
            "it ran over, not {first:?}"
        );
        let resting = rested(&at, hour, || ask(Signed::Out));
        assert!(
            matches!(resting, Signed::Resting),
            "it rests, not {resting:?}"
        );
        assert_eq!(asked.get(), 1, "the keyring was asked during its rest");
        let after = rested(&at, Duration::ZERO, || ask(Signed::Out));
        assert!(
            matches!(after, Signed::Out),
            "asked once rested, not {after:?}"
        );
        let again = rested(&at, hour, || ask(Signed::Out));
        assert!(
            matches!(again, Signed::Out),
            "an answer ends the rest, not {again:?}"
        );
        assert_eq!(asked.get(), 3, "asked once its rest was up, and after");
    }

    #[test]
    fn a_keyring_running_over_is_asked_once_however_many_want_it() {
        static AT: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
        static OTHERS_ASKED: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let (asking, asked) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let first = std::thread::spawn(move || {
            rested(&AT, REST, || {
                // The case may have given up, and then nobody listens.
                let _ = asking.send(());
                // Held until the other caller has come, or a minute.
                let _ = released.recv_timeout(Duration::from_mins(1));
                Signed::RanOver
            })
        });
        asked
            .recv_timeout(Duration::from_secs(5))
            .expect("the first caller started asking");
        let (said, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let signed = rested(&AT, REST, || {
                OTHERS_ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Signed::Out
            });
            // The case may have given up, and then nobody listens.
            let _ = said.send(signed);
        });
        // Long enough for the other caller to ask, were it let.
        std::thread::sleep(Duration::from_millis(300));
        let _ = release.send(());
        let _ = first.join();
        let other = heard.recv_timeout(Duration::from_secs(5));
        assert_eq!(
            OTHERS_ASKED.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "a second caller asked the keyring while the first ran over: {other:?}"
        );
        assert!(
            matches!(other, Ok(Signed::Resting)),
            "the second caller rests once the first ran over, not {other:?}"
        );
    }

    /// A sign-in to the stand-in workspace, its token `token`.
    fn account(token: &str) -> Account {
        Account {
            token: token.to_owned(),
            cookie: "d".to_owned(),
            team: "T1".to_owned(),
            user: String::new(),
        }
    }

    #[test]
    fn a_sign_in_read_stands_while_the_keyring_runs_over_or_rests() {
        let token = |kept: Option<Account>| kept.map(|account| account.token);
        let read = Instant::now();
        let mut last = LastSignIn::new();
        let none = token(last.heard(Signed::RanOver, read));
        assert_eq!(none, None, "none read yet");
        let first = token(last.heard(Signed::In(account("xoxc-keyring")), read));
        assert_eq!(first.as_deref(), Some("xoxc-keyring"), "what's read");
        let over = token(last.heard(Signed::RanOver, read + Duration::from_secs(30)));
        assert_eq!(
            over.as_deref(),
            Some("xoxc-keyring"),
            "it stands as it runs over"
        );
        let resting = token(last.heard(Signed::Resting, read + Duration::from_mins(3)));
        assert_eq!(resting.as_deref(), Some("xoxc-keyring"), "and as it rests");
        let out = token(last.heard(Signed::Out, read + Duration::from_mins(4)));
        assert_eq!(out, None, "signed out is");
        let after = token(last.heard(Signed::Resting, read + Duration::from_mins(4)));
        assert_eq!(after, None, "and nothing stands once Joel signed out");
    }

    #[test]
    fn a_sign_in_read_stands_no_longer_than_its_bound() {
        let read = Instant::now();
        let mut last = LastSignIn::new();
        let _ = last.heard(Signed::In(account("xoxc-keyring")), read);
        let late = read + STANDS;
        assert!(
            !last.stands(late),
            "a sign-in read {STANDS:?} ago stands no longer"
        );
        let kept = last
            .heard(Signed::Resting, late)
            .map(|account| account.token);
        assert_eq!(
            kept, None,
            "a locked keyring ends Slack once its bound is up"
        );
        assert!(
            last.heard(Signed::Resting, read).is_none(),
            "and what was let go isn't brought back"
        );
    }

    #[test]
    fn a_run_over_says_whether_a_sign_in_stands_meanwhile() {
        let stands = ran_over(true);
        assert!(
            stands.ends_with("and the sign-in last read stands meanwhile"),
            "one read stands: {stands}"
        );
        let none = ran_over(false);
        assert!(
            none.ends_with("with no sign-in read yet, Slack reads as signed out till then")
                && !none.contains("stands"),
            "none read, so none is said to stand: {none}"
        );
    }
}
