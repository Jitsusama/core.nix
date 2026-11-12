//! What Slack's daemon, slack-unreads, answers over its socket, in the
//! shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"), version 1, served by daemon-serve as every daemon's is:
//! `hello`, `health`, `list`, `watch` and `unwatch` of what it holds,
//! and two asks of its own, `search` for Joel's words and `get` of what
//! it holds or of a message a search found, with the few around it.
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
//!
//! A search and a get each wait on Slack, so each is answered on a
//! thread of its own, and holds up no other ask.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};

use daemon_serve::asks::{Domain, error};
use daemon_serve::serve::{self, Shared};
use serde_json::{Map, Value, json};

use crate::messages::People;
use crate::rows::{self, Words};
use crate::search::{self, Failed};
use crate::unreads::{Conversation, Said, State};
use crate::{Account, Slack};

/// Slack's daemon as its clients know it. A `slack-message` a search
/// finds is asked of Slack each time and never held; those held are the
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

/// The asks that wait on Slack, each answered on a thread of its own;
/// `get` is the daemon's to answer, since a message a search found is
/// held nowhere.
const ASKED_OF_SLACK: &[&str] = &["search", "get"];

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

    /// The daemon's one source as daemon-serve takes it: its word, and,
    /// refused, why, which health says.
    fn sources(self) -> Value {
        let slack = match self {
            Stands::Starting => json!("starting"),
            Stands::SignedOut => json!("signed-out"),
            Stands::Unreachable => {
                json!({"state": "refused", "reason": "Slack couldn't be reached"})
            }
            Stands::Answering => json!("answering"),
        };
        json!({ "slack": slack })
    }
}

/// What making the records keeps between Slack's answers.
#[derive(Default)]
struct Making {
    /// The words rows are said in, none to make no rows.
    words: Option<Words>,
    /// Each place's rows as last made, so each keeps its order where it
    /// can and a conversation coming first is one row changed, not every
    /// one below it.
    told: rows::Told,
}

/// What slack-unreads holds, as the interface serves it.
#[derive(Clone)]
pub struct Daemon {
    serving: Arc<Shared>,
    making: Arc<Mutex<Making>>,
    /// Who each person is, which its rounds fill and its searches read.
    people: People,
}

/// What a search or a get asks Slack with.
struct Asking {
    credentials: PathBuf,
    origin: String,
    people: People,
    /// What serves the daemon, for a get of what it holds; weak, since
    /// what serves it holds this.
    serving: Arc<OnceLock<Weak<Shared>>>,
}

/// Serves on `socket`, on threads of their own, for as long as the
/// program runs, or says why it can't and serves nothing.
pub fn listen(socket: &Path, credentials: PathBuf, origin: String) -> Daemon {
    let people = People::default();
    let held = Arc::new(OnceLock::new());
    let asking = Asking {
        credentials,
        origin,
        people: people.clone(),
        serving: Arc::clone(&held),
    };
    let serving = Shared::new(DOMAIN, Box::new(move |said| asking.answer(said)));
    let _ = held.set(Arc::downgrade(&serving));
    serving.answers(&["get"]);
    serving.slow(ASKED_OF_SLACK);
    serving.change(Vec::new(), Stands::Starting.sources());
    match serve::listen(socket) {
        Ok(listener) => serve::accept(Arc::clone(&serving), listener),
        Err(why) => eprintln!("<3>couldn't listen on {}: {why}", socket.display()),
    }
    Daemon {
        serving,
        making: Arc::default(),
        people,
    }
}

impl Daemon {
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

    fn lock(&self) -> MutexGuard<'_, Making> {
        self.making.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes what Slack said, or none with no sign-in, at `now` in
    /// milliseconds since the epoch: each record that changed is said to
    /// each watch that sees it, each that went as gone, and how Slack
    /// stands whenever that changes. A record unchanged keeps when it was
    /// first heard so.
    pub fn say(&self, said: Option<&Said>, now: u64) {
        let mut making = self.lock();
        let mut made = records(said);
        let Making { words, told } = &mut *making;
        if let (Some(words), Some(said)) = (words.as_ref(), said) {
            made.extend(told.records(said, words));
        }
        let records = made
            .into_iter()
            .map(|(key, mut record)| {
                let heard = match self.serving.held(&key) {
                    Some(was) if same(&was, &record) => was["heard"].as_u64().unwrap_or(now),
                    _ => now,
                };
                record["heard"] = json!(heard);
                record
            })
            .collect();
        self.serving.change(records, Stands::of(said).sources());
    }
}

impl Asking {
    /// One of the daemon's own asks, answered without its id.
    fn answer(&self, said: &Value) -> Option<Value> {
        match said["ask"].as_str()? {
            "search" => Some(self.search(said)),
            "get" => Some(self.get(said)),
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

    /// Each key asked for, in the order asked: one held as it's held, and
    /// a message, found by a search or held, with the few around it, asked
    /// of Slack.
    fn get(&self, said: &Value) -> Value {
        let keys = strings(said, "keys").unwrap_or_default();
        let messages = keys.iter().any(|key| search::named(key).is_some());
        let account = Account::read(&self.credentials);
        if messages && account.is_none() {
            return error(&Value::Null, "signed-out", "nobody is signed in to Slack");
        }
        let serving = self.serving.get().and_then(Weak::upgrade);
        let slack = Slack::new(self.origin.clone());
        let mut records = Vec::new();
        for key in &keys {
            let held = serving.as_ref().and_then(|serving| serving.held(key));
            if let (Some(record), None) = (held, search::named(key)) {
                records.push(record);
                continue;
            }
            let (Some((channel, ts)), Some(account)) = (search::named(key), account.as_ref())
            else {
                continue;
            };
            match slack.around(account, channel, ts) {
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
    match failed {
        Failed::SignedOut => error(&Value::Null, "signed-out", "Slack refused the sign-in"),
        Failed::Busy => error(&Value::Null, "too-many", "Slack is busy"),
        Failed::Unreachable(why) => {
            eprintln!("<4>{why}");
            error(&Value::Null, "refused", "Slack couldn't be reached")
        }
    }
}

#[cfg(test)]
mod tests;
