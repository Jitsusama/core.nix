//! What Slack's daemon, slack-unreads, holds and answers over its socket,
//! in the shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"), version 1. What every daemon answers alike, `hello`,
//! `health`, `list`, `watch`, `unwatch` and `get` of what it holds, is
//! answered by daemon-serve (../daemon-serve), as every daemon's is; what
//! only Slack can say is answered here: `search` for Joel's words, and
//! `around`, a message a search found with the few around it, each asked
//! of Slack on a thread of its own, so one waiting on Slack holds up no
//! other ask.
//!
//! It holds six kinds of record, made from what slack-unreads last
//! heard from Slack (`Daemon::say`): each conversation unread, a
//! `slack-unread:<id>`; each he read last, a `slack-recent:<id>`; the
//! read mark of each conversation named so far, a `slack-seen:<id>`, by
//! which Slack's own notices are read once he's read past them (lib's
//! `unreads::seen`); each
//! message wanting him, a `slack-message:<channel>:<ts>`, done once he's
//! read it in Slack, and Slack's read mark for its conversation, a
//! `slack-read:<channel>` (lib's `messages`); and how asking Slack
//! stands, the one `slack-account:slack`, whose `messages` says
//! `whole` once every message wanting him is held. A record carries
//! its thing under a field named for its kind, as Google's daemon's do,
//! since a conversation has a `kind` of its own; and the timestamp it's
//! ordered by rather than its place, so a new message changes its own
//! record and not every one after it. A record's `heard` is when it last
//! changed, so a look at Slack that finds nothing new tells no watch
//! anything: the connection says the daemon is there, and its watchdog
//! restarts one that hangs, so nothing need be said to show it's alive.
//!
//! Handed the launcher's vocabulary (`Daemon::words`), each conversation
//! unread and each read last is also a `row:<place>:<id>` record, the
//! row Slack's Unread or Recent shows for it, and each message wanting
//! him a row of `slack-messages` with its `drawer:` record beside it
//! (rows.rs), so the shell makes no rows of its own; a `list` or `watch`
//! of rows names its `place`, one or a list, to hear only those, and a
//! drawer is watched by its key while it shows. Without a vocabulary, no
//! rows are made.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use daemon_serve::asks::{Domain, error};
use daemon_serve::serve::{self, Own, Shared};
use serde_json::{Map, Value, json};

use crate::messages::People;
use crate::rows::{self, Words};
use crate::search::{self, Failed};
use crate::unreads::{Conversation, Said, State};
use crate::{Account, Slack};

/// The kinds this daemon answers for. A `slack-message` a search finds
/// is asked of Slack each time and never held; those held are the
/// messages wanting Joel, which Search Slack's watch, kept for how Slack
/// stands (SlackSearch.qml), passes over.
const DOMAIN: Domain = Domain {
    name: "slack",
    kinds: &[
        "slack-unread",
        "slack-recent",
        "slack-seen",
        "slack-account",
        "slack-message",
        "slack-read",
        "row",
        "drawer",
    ],
    sole: &[],
    named_only: &[],
};

/// The asks only Slack answers, each of which waits on Slack.
const ITS_OWN: &[&str] = &["search", "around"];

/// Every record what Slack last said makes, in its order: what's unread,
/// those wanting Joel first, then what he read last, then each message
/// wanting him, newest first, then their conversations' read marks, then
/// the account.
/// Only while Slack answered are any conversations said, so an old count
/// is never held as a current one; the messages wanting him are said
/// through Slack refusing or out of reach too, since one unread stays
/// unread until Slack says it was read, and what holds its notice would
/// take one gone for read. With no sign-in, `said` is none, nothing but
/// the account is said, and the account says so. Each is without its `heard`, which the daemon
/// adds as it learns when one changed.
pub fn records(said: Option<&Said>) -> Vec<(String, Value)> {
    fn record(kind: &str, id: &str, thing: Value) -> (String, Value) {
        let key = format!("{kind}:{id}");
        let mut fields = Map::new();
        fields.insert("key".into(), json!(key));
        fields.insert("kind".into(), json!(kind));
        fields.insert(kind.into(), thing);
        (key, Value::Object(fields))
    }
    let shown = |one: &Conversation| {
        json!({
            "id": one.id,
            "name": one.name,
            "kind": one.kind,
            "link": one.link,
        })
    };
    let mut made = Vec::new();
    let standing = match said {
        None => json!("none"),
        Some(said) => json!(said.state),
    };
    if let Some(said) = said.filter(|said| said.state == State::Ok) {
        for one in &said.conversations {
            let mut thing = shown(one);
            thing["count"] = json!(one.count);
            thing["latest"] = json!(one.at);
            made.push(record("slack-unread", &one.id, thing));
        }
        for one in &said.recent {
            let mut thing = shown(one);
            thing["read"] = json!(one.at);
            made.push(record("slack-recent", &one.id, thing));
        }
        for one in &said.seen {
            made.push(record("slack-seen", &one.id, json!(one)));
        }
    }
    if let Some(said) = said {
        for one in &said.messages {
            let id = one.key.strip_prefix("slack-message:").unwrap_or(&one.key);
            made.push(record("slack-message", id, json!(one)));
        }
        for mark in &said.marks {
            made.push(record("slack-read", &mark.channel, json!(mark)));
        }
    }
    // Once every message wanting him is held, the account says so, and
    // only then is one missing from them known read, as after a restart.
    let mut account = json!({"standing": standing});
    if said.is_some_and(|said| said.whole) {
        account["messages"] = json!(WHOLE);
    }
    made.push(record("slack-account", "slack", account));
    made
}

/// What the account's `messages` says once every message wanting Joel is
/// held.
pub const WHOLE: &str = "whole";

/// How asking Slack stands, as the source's word says it.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Stands {
    /// slack-unreads hasn't looked yet.
    #[default]
    Starting,
    /// Nobody is signed in, or Slack refused the sign-in.
    SignedOut,
    Unreachable,
    Answering,
}

impl Stands {
    /// How what Slack last said, or none with no sign-in, stands.
    fn of(said: Option<&Said>) -> Stands {
        match said.map(|said| said.state) {
            None | Some(State::SignedOut) => Stands::SignedOut,
            Some(State::Unreachable) => Stands::Unreachable,
            Some(State::Ok) => Stands::Answering,
        }
    }

    /// The source's word for it and why, as the people daemon says its
    /// sources'.
    fn word(self) -> (&'static str, &'static str) {
        match self {
            Stands::Starting => ("starting", ""),
            Stands::SignedOut => ("signed-out", ""),
            Stands::Unreachable => ("refused", "Slack couldn't be reached"),
            Stands::Answering => ("answering", ""),
        }
    }

    /// Its one source, as daemon-serve is given them: health says its
    /// reason, and every other line its word alone.
    fn sources(self) -> Value {
        let (state, reason) = self.word();
        json!({"slack": {"state": state, "reason": reason}})
    }
}

#[derive(Default)]
struct Held {
    /// Every record as last said, by key, with its `heard`.
    records: BTreeMap<String, Value>,
    /// The words rows are said in, none to make no rows.
    words: Option<Words>,
    /// Each place's rows as last made, so each keeps its order where it
    /// can and a conversation coming first is one row changed, not every
    /// one below it.
    told: rows::Told,
}

impl Held {
    /// Every record what Slack said at `now` makes, in its order, each
    /// with when it last changed, which is kept for the next.
    fn heard(&mut self, said: Option<&Said>, now: u64) -> Vec<Value> {
        let mut made = records(said);
        if let (Some(words), Some(said)) = (self.words.as_ref(), said) {
            made.extend(self.told.records(said, words));
        }
        let mut kept = BTreeMap::new();
        let mut each = Vec::with_capacity(made.len());
        for (key, mut record) in made {
            let heard = match self.records.get(&key) {
                Some(was) if same(was, &record) => was["heard"].as_u64().unwrap_or(now),
                _ => now,
            };
            record["heard"] = json!(heard);
            each.push(record.clone());
            kept.insert(key, record);
        }
        self.records = kept;
        each
    }
}

/// What slack-unreads holds, as the interface serves it.
#[derive(Clone)]
pub struct Daemon {
    shared: Arc<Shared>,
    held: Arc<Mutex<Held>>,
    /// Who each person is, which its rounds fill and its searches read.
    people: People,
}

/// What Slack's own asks are asked with.
struct Asking {
    credentials: PathBuf,
    origin: String,
    people: People,
}

/// Listens on `socket`, on threads of its own, for as long as the
/// program runs, or says why it can't and serves nothing.
pub fn listen(socket: &Path, credentials: PathBuf, origin: String) -> Daemon {
    let daemon = Daemon::new(credentials, origin);
    match serve::listen(socket) {
        Ok(listener) => serve::accept(Arc::clone(&daemon.shared), listener),
        Err(why) => eprintln!("<3>couldn't listen on {}: {why}", socket.display()),
    }
    daemon
}

impl Daemon {
    fn new(credentials: PathBuf, origin: String) -> Daemon {
        let people = People::default();
        let asking = Asking {
            credentials,
            origin,
            people: people.clone(),
        };
        let own: Own = Box::new(move |said| asking.answer(said));
        let shared = Shared::new(DOMAIN, own);
        shared.slow(ITS_OWN);
        // Said as starting until Slack is first asked, as a shell starting
        // the daemon by its socket hears.
        shared.change(Vec::new(), Stands::Starting.sources());
        Daemon {
            shared,
            held: Arc::new(Mutex::new(Held::default())),
            people,
        }
    }

    /// Who each person is, for the rounds to name people in and fill, so
    /// a search asks Slack only for those no round has named.
    #[must_use]
    pub fn people(&self) -> People {
        self.people.clone()
    }

    /// Makes ready rows from Slack's next answer on, said in these words.
    pub fn words(&self, words: Words) {
        self.lock().words = Some(words);
    }

    fn lock(&self) -> MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes what Slack said, or none with no sign-in, at `now` in
    /// milliseconds since the epoch: each record that changed is said to
    /// each watch that sees it, each that went as gone, and how Slack
    /// stands whenever that changes. It's said while what's held is still
    /// locked, so two sayings are never told in the other order.
    pub fn say(&self, said: Option<&Said>, now: u64) {
        let mut held = self.lock();
        let records = held.heard(said, now);
        self.shared.change(records, Stands::of(said).sources());
    }
}

impl Asking {
    /// One of Slack's own asks, answered without its id, or none for an
    /// ask that isn't.
    fn answer(&self, said: &Value) -> Option<Value> {
        match said.get("ask").and_then(Value::as_str)? {
            "search" => Some(self.search(said)),
            "around" => Some(self.around(said)),
            _ => None,
        }
    }

    /// Slack searched for Joel's words.
    fn search(&self, said: &Value) -> Value {
        let words = said
            .get("words")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let Some(account) = Account::read(&self.credentials) else {
            return error(&Value::Null, "signed-out", "nobody is signed in to Slack");
        };
        if words.is_empty() {
            return json!({"records": []});
        }
        match Slack::new(self.origin.clone()).search(&account, words, &self.people) {
            Ok(found) => json!({"records": found}),
            Err(failed) => refused(&failed),
        }
    }

    /// Each message asked for by its key, found by a search or held, with
    /// the few around it, asked of Slack, in the order asked.
    fn around(&self, said: &Value) -> Value {
        let keys = strings(said, "keys").unwrap_or_default();
        let named: Vec<(&String, (&str, &str))> = keys
            .iter()
            .filter_map(|key| Some((key, search::named(key)?)))
            .collect();
        if named.is_empty() {
            return json!({"records": []});
        }
        let Some(account) = Account::read(&self.credentials) else {
            return error(&Value::Null, "signed-out", "nobody is signed in to Slack");
        };
        let slack = Slack::new(self.origin.clone());
        let mut records = Vec::new();
        for (key, (channel, ts)) in named {
            match slack.around(&account, channel, ts) {
                Ok(near) => records.push(json!({
                    "key": key, "kind": "slack-message", "around": near,
                })),
                Err(failed) => return refused(&failed),
            }
        }
        json!({"records": records})
    }
}

/// Whether two records hold the same, whenever each was heard.
fn same(was: &Value, now: &Value) -> bool {
    let mut was = was.clone();
    if let Some(fields) = was.as_object_mut() {
        fields.remove("heard");
    }
    was == *now
}

/// An ask's list of words, as `keys` is.
fn strings(said: &Value, field: &str) -> Option<Vec<String>> {
    said.get(field).and_then(Value::as_array).map(|each| {
        each.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}

/// Why Slack said nothing, by the interface's codes: too busy is
/// too-many, which the shell says as "Slack is busy".
fn refused(failed: &Failed) -> Value {
    let id = Value::Null;
    match failed {
        Failed::SignedOut => error(&id, "signed-out", "Slack refused the sign-in"),
        Failed::Busy => error(&id, "too-many", "Slack is busy"),
        Failed::Unreachable(why) => {
            eprintln!("<4>{why}");
            error(&id, "refused", "Slack couldn't be reached")
        }
    }
}

#[cfg(test)]
mod tests;
