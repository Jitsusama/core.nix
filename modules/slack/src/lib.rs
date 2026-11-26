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
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ureq::Agent;

mod account;
pub mod daemon;
pub mod message_rows;
pub mod messages;
pub mod person;
pub mod posted;
pub mod rows;
pub mod search;
pub mod unreads;

pub use account::{Account, LastSignIn, REST, STANDS, Signed, locked, no_longer_stands};

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

/// The words Slack refuses a sign-in with, each mended by signing in
/// again, kept once so every reader of Slack counts the same ones.
const SIGN_IN_REFUSALS: [&str; 5] = [
    "invalid_auth",
    "not_authed",
    "account_inactive",
    "token_revoked",
    "token_expired",
];

/// Whether Slack's error word `error` refuses the sign-in itself.
#[must_use]
pub fn refuses_the_sign_in(error: &str) -> bool {
    SIGN_IN_REFUSALS.contains(&error)
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
        error if refuses_the_sign_in(error) => Err(Refused::SignedOut),
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
#[path = "lib_tests.rs"]
mod tests;
