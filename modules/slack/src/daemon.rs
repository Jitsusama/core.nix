//! What Slack's daemon, slack-unreads, answers over its socket, in the
//! shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"), version 1: `hello`, `health`, `list`, `watch` and
//! `unwatch` of what it holds, `search` for Joel's words, and `get` of
//! what it holds or of a message a search found, with the few around it.
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
//! Each ask is answered on a thread of its own, so a search waiting on
//! Slack holds up no other ask.

use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::{Map, Value, json};

use crate::messages::People;
use crate::rows::{self, Words};
use crate::search::{self, Failed};
use crate::unreads::{Conversation, Said, State};
use crate::{Account, Slack};

/// The interface's version.
pub const VERSION: u64 = 1;

/// The kinds this daemon answers for. A `slack-message` a search finds
/// is asked of Slack each time and never held; those held are the
/// messages wanting Joel, which Search Slack's watch, kept for how Slack
/// stands (SlackSearch.qml), passes over.
const KINDS: [&str; 8] = [
    "slack-unread",
    "slack-recent",
    "slack-seen",
    "slack-account",
    "slack-message",
    "slack-read",
    "row",
    "drawer",
];

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
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The kinds watched, or none for every kind held.
    kinds: Option<HashSet<String>>,
    /// The keys watched, or none for every key of its kinds.
    keys: Option<HashSet<String>>,
    /// The places whose rows are watched, or none for every place's.
    places: Option<HashSet<String>>,
    told: Sender<String>,
}

impl Watch {
    fn sees(&self, key: &str) -> bool {
        seen(self.kinds.as_ref(), self.places.as_ref(), key)
            && self.keys.as_ref().is_none_or(|keys| keys.contains(key))
    }
}

/// Whether a key is of the kinds asked, none for every kind, and a row's
/// of the places asked, none for every place.
fn seen(kinds: Option<&HashSet<String>>, places: Option<&HashSet<String>>, key: &str) -> bool {
    let (kind, rest) = key.split_once(':').unwrap_or((key, ""));
    let place = rest.split_once(':').map_or(rest, |(place, _)| place);
    kinds.is_none_or(|kinds| kinds.contains(kind))
        && (kind != "row" || places.is_none_or(|places| places.contains(place)))
}

#[derive(Default)]
struct Held {
    /// Every record, by key, with its `heard`.
    records: BTreeMap<String, Value>,
    /// The keys in the order Slack's said them.
    order: Vec<String>,
    /// How asking Slack last stood.
    stands: Stands,
    /// The words rows are said in, none to make no rows.
    words: Option<Words>,
    /// Each place's rows as last made, so each keeps its order where it
    /// can and a conversation coming first is one row changed, not every
    /// one below it.
    told: rows::Told,
    watches: Vec<Watch>,
    connections: u64,
}

/// What slack-unreads holds, as the interface serves it.
#[derive(Clone)]
pub struct Daemon {
    held: Arc<Mutex<Held>>,
    asking: Arc<Asking>,
    /// Who each person is, which its rounds fill and its searches read.
    people: People,
}

struct Asking {
    credentials: PathBuf,
    origin: String,
    since: u64,
}

/// Listens on `socket`, on a thread of its own, for as long as the
/// program runs, or says why it can't and serves nothing.
pub fn listen(socket: &Path, credentials: PathBuf, origin: String) -> Daemon {
    let daemon = Daemon::new(credentials, origin);
    let listener = match bind(socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            return daemon;
        }
    };
    let serving = daemon.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let daemon = serving.clone();
            let connection = {
                let mut held = daemon.lock();
                held.connections += 1;
                held.connections
            };
            std::thread::spawn(move || daemon.serve(stream, connection));
        }
    });
    daemon
}

/// The socket systemd holds for it (slack-unreads.socket), so the shell
/// never finds it missing while the daemon starts or starts again; or, run
/// by hand, its own, in a folder only Joel reads, replacing one left behind.
fn bind(socket: &Path) -> std::io::Result<UnixListener> {
    if let Some(listener) = handed() {
        return Ok(listener);
    }
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

/// The listening socket systemd hands this process, as the first
/// descriptor after the standard three, when it says so (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    let given = unsafe { UnixListener::from_raw_fd(3) };
    // A copy closed on exec, which the descriptor as handed over isn't.
    given.try_clone().ok()
}

impl Daemon {
    fn new(credentials: PathBuf, origin: String) -> Daemon {
        Daemon {
            held: Arc::new(Mutex::new(Held::default())),
            asking: Arc::new(Asking {
                credentials,
                origin,
                since: crate::now() * 1000,
            }),
            people: People::default(),
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
    /// stands whenever that changes.
    pub fn say(&self, said: Option<&Said>, now: u64) {
        let mut held = self.lock();
        let before = held.stands;
        let mut changed = Vec::new();
        let mut order = Vec::new();
        let mut kept = BTreeMap::new();
        let mut made = records(said);
        let Held { words, told, .. } = &mut *held;
        if let (Some(words), Some(said)) = (words.as_ref(), said) {
            made.extend(told.records(said, words));
        }
        for (key, mut record) in made {
            let heard = match held.records.get(&key) {
                Some(was) if same(was, &record) => was["heard"].as_u64().unwrap_or(now),
                _ => now,
            };
            record["heard"] = json!(heard);
            if held.records.get(&key) != Some(&record) {
                changed.push(record.clone());
            }
            order.push(key.clone());
            kept.insert(key, record);
        }
        let gone: Vec<String> = held
            .records
            .keys()
            .filter(|key| !kept.contains_key(*key))
            .cloned()
            .collect();
        held.records = kept;
        held.order = order;
        held.stands = Stands::of(said);
        let after = held.stands;
        held.watches.retain(|watch| {
            let records: Vec<&Value> = changed
                .iter()
                .filter(|record| watch.sees(record["key"].as_str().unwrap_or_default()))
                .collect();
            let went: Vec<&String> = gone.iter().filter(|key| watch.sees(key)).collect();
            let mut told = true;
            if !records.is_empty() || !went.is_empty() {
                let line = json!({"id": watch.id, "changed": records, "gone": went});
                told = watch.told.send(line.to_string()).is_ok();
            }
            if told && before != after {
                let line = json!({"id": watch.id, "sources": {"slack": after.word().0}});
                told = watch.told.send(line.to_string()).is_ok();
            }
            told
        });
    }

    /// Answers one client until it goes, each ask on a thread of its own,
    /// and its watches with it.
    fn serve(&self, stream: UnixStream, connection: u64) {
        let Ok(reading) = stream.try_clone() else {
            return;
        };
        let (told, lines) = mpsc::channel::<String>();
        let mut writing = stream;
        std::thread::spawn(move || {
            for line in lines {
                if writeln!(writing, "{line}").is_err() {
                    break;
                }
            }
        });
        for line in BufReader::new(reading).lines() {
            let Ok(line) = line else { break };
            let daemon = self.clone();
            let told = told.clone();
            std::thread::spawn(move || {
                if let Some(answer) = daemon.answer(&line, connection, &told) {
                    let _ = told.send(answer.to_string());
                }
            });
        }
        self.lock()
            .watches
            .retain(|watch| watch.connection != connection);
    }

    /// What one line asks, answered, or none once a watch's answer has
    /// been told already.
    fn answer(&self, line: &str, connection: u64, told: &Sender<String>) -> Option<Value> {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return Some(error(&Value::Null, "unknown-ask", "that isn't an ask"));
        };
        if said.get("ask").and_then(Value::as_str) == Some("watch") {
            return self.watch(&said, connection, told);
        }
        Some(self.asked(&said, connection))
    }

    /// A watch, told what's held while the lock is still held and only
    /// then added, so no change `say` tells it can come before the
    /// records it changes; or why it can't be.
    fn watch(&self, said: &Value, connection: u64, told: &Sender<String>) -> Option<Value> {
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let kinds = match kinds(said) {
            Ok(kinds) => kinds,
            Err(kind) => return Some(unknown(&id, &kind)),
        };
        let watch = Watch {
            connection,
            id: id.clone(),
            kinds,
            keys: strings(said, "keys").map(|keys| keys.into_iter().collect()),
            places: places(said),
            told: told.clone(),
        };
        let mut held = self.lock();
        let records: Vec<&Value> = held
            .order
            .iter()
            .filter(|key| watch.sees(key))
            .filter_map(|key| held.records.get(key))
            .collect();
        let answer = json!({"id": id, "records": records});
        if told.send(answer.to_string()).is_err() {
            return None;
        }
        // A watch asked again by its id replaces the one before.
        held.watches
            .retain(|was| !(was.connection == connection && was.id == id));
        held.watches.push(watch);
        None
    }

    /// What any ask but a watch is answered with.
    fn asked(&self, said: &Value, connection: u64) -> Value {
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
        match asked {
            "hello" => {
                let (state, _) = self.lock().stands.word();
                json!({
                    "id": id, "version": VERSION, "domain": "slack", "state": "answering",
                    "since": self.asking.since, "sources": {"slack": state},
                })
            }
            "health" => {
                let held = self.lock();
                let (state, reason) = held.stands.word();
                json!({
                    "id": id, "version": VERSION, "state": "answering",
                    "sources": {"slack": {"state": state, "reason": reason}},
                    "records": held.records.len(), "watching": held.watches.len(),
                })
            }
            "list" => {
                let kinds = match kinds(said) {
                    Ok(kinds) => kinds,
                    Err(kind) => return unknown(&id, &kind),
                };
                let limit = said
                    .get("limit")
                    .and_then(Value::as_u64)
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(usize::MAX);
                let places = places(said);
                let held = self.lock();
                let records: Vec<&Value> = held
                    .order
                    .iter()
                    .filter(|key| seen(kinds.as_ref(), places.as_ref(), key))
                    .filter_map(|key| held.records.get(key))
                    .take(limit)
                    .collect();
                json!({"id": id, "records": records})
            }
            "unwatch" => {
                self.lock()
                    .watches
                    .retain(|was| !(was.connection == connection && was.id == id));
                json!({"id": id, "unwatched": true})
            }
            "search" => self.search(&id, said),
            "get" => self.get(&id, said),
            other => error(&id, "unknown-ask", &format!("no ask {other:?}")),
        }
    }

    /// Slack searched for Joel's words.
    fn search(&self, id: &Value, said: &Value) -> Value {
        let words = said
            .get("words")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let Some(account) = Account::read(&self.asking.credentials) else {
            return error(id, "signed-out", "nobody is signed in to Slack");
        };
        if words.is_empty() {
            return json!({"id": id, "records": []});
        }
        match Slack::new(self.asking.origin.clone()).search(&account, words, &self.people) {
            Ok(found) => json!({"id": id, "records": found}),
            Err(failed) => refused(id, &failed),
        }
    }

    /// Each key asked for, in the order asked: one held as it's held, and
    /// a message, found by a search or held, with the few around it, asked
    /// of Slack.
    fn get(&self, id: &Value, said: &Value) -> Value {
        let keys = strings(said, "keys").unwrap_or_default();
        let messages = keys.iter().any(|key| search::named(key).is_some());
        let account = Account::read(&self.asking.credentials);
        if messages && account.is_none() {
            return error(id, "signed-out", "nobody is signed in to Slack");
        }
        let slack = Slack::new(self.asking.origin.clone());
        let mut records = Vec::new();
        for key in &keys {
            let held = self.lock().records.get(key).cloned();
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
                Err(failed) => return refused(id, &failed),
            }
        }
        json!({"id": id, "records": records})
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

/// The kinds an ask names, as one or a list, none for every kind held,
/// or the first this daemon doesn't hold.
fn kinds(said: &Value) -> Result<Option<HashSet<String>>, String> {
    let named: Vec<String> = match said.get("kind") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(kind)) => vec![kind.clone()],
        Some(kinds) => kinds
            .as_array()
            .map(|kinds| {
                kinds
                    .iter()
                    .map(|kind| kind.as_str().unwrap_or_default().to_owned())
                    .collect()
            })
            .unwrap_or_default(),
    };
    match named.iter().find(|kind| !KINDS.contains(&kind.as_str())) {
        Some(kind) => Err(kind.clone()),
        None => Ok(Some(named.into_iter().collect())),
    }
}

/// The places an ask's rows are narrowed to, as one or a list, none for
/// every place.
fn places(said: &Value) -> Option<HashSet<String>> {
    match said.get("place")? {
        Value::String(place) => Some(HashSet::from([place.clone()])),
        _ => strings(said, "place").map(|places| places.into_iter().collect()),
    }
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
fn refused(id: &Value, failed: &Failed) -> Value {
    match failed {
        Failed::SignedOut => error(id, "signed-out", "Slack refused the sign-in"),
        Failed::Busy => error(id, "too-many", "Slack is busy"),
        Failed::Unreachable(why) => {
            eprintln!("<4>{why}");
            error(id, "refused", "Slack couldn't be reached")
        }
    }
}

/// A kind this daemon doesn't answer for, said so.
fn unknown(id: &Value, kind: &str) -> Value {
    error(
        id,
        "unknown-kind",
        &format!(
            "this daemon keeps slack-unread, slack-recent, slack-seen, slack-account, \
             slack-message, slack-read, row and drawer, not {kind}"
        ),
    )
}

fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Done, Mark, Raised};
    use crate::unreads::{Kind, Seen};

    /// The one account's key.
    const ACCOUNT: &str = "slack-account:slack";

    fn one(id: &str, kind: Kind, count: u64, at: &str) -> Conversation {
        Conversation {
            id: id.to_owned(),
            name: "Ada".to_owned(),
            kind,
            count,
            link: format!("slack://channel?team=T1&id={id}"),
            at: at.to_owned(),
        }
    }

    fn ok(conversations: Vec<Conversation>, recent: Vec<Conversation>) -> Said {
        Said {
            state: State::Ok,
            conversations,
            recent,
            messages: Vec::new(),
            marks: Vec::new(),
            seen: Vec::new(),
            whole: false,
        }
    }

    fn message(channel: &str, ts: &str, done: Option<Done>) -> Raised {
        Raised {
            key: search::key(channel, ts),
            channel: channel.to_owned(),
            ts: ts.to_owned(),
            place: "Ada".to_owned(),
            who: "Ada".to_owned(),
            user: "U7".to_owned(),
            named: true,
            mine: false,
            first: "hi".to_owned(),
            text: "hi".to_owned(),
            at: 1_730_300_001_000,
            link: "https://ada.slack.com/archives/D1/p1730300001000100".to_owned(),
            team: "T1".to_owned(),
            done,
        }
    }

    #[test]
    fn a_message_wanting_joel_is_still_said_while_slack_doesnt_answer() {
        let mut said = ok(vec![one("D1", Kind::Dm, 1, "1")], vec![]);
        said.messages = vec![message("D1", "1730300001.000100", None)];
        said.state = State::Unreachable;
        let keys: Vec<String> = records(Some(&said))
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, ["slack-message:D1:1730300001.000100", ACCOUNT]);
    }

    #[test]
    fn the_account_says_the_messages_are_whole_only_once_they_are() {
        let mut said = ok(vec![], vec![]);
        let account = |said: Option<&Said>| records(said).pop().unwrap().1["slack-account"].clone();
        assert_eq!(account(Some(&said)), json!({"standing": "ok"}));
        said.whole = true;
        assert_eq!(
            account(Some(&said)),
            json!({"standing": "ok", "messages": "whole"})
        );
        assert_eq!(account(None), json!({"standing": "none"}));
    }

    #[test]
    fn a_message_wanting_joel_is_a_record_and_its_being_read_a_change_to_it() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        let (told, heard) = mpsc::channel();
        let asked = json!({"id": "m", "ask": "watch", "kind": ["slack-message", "slack-read"]});
        daemon.answer(&asked.to_string(), 1, &told);
        let mut said = ok(vec![one("D1", Kind::Dm, 1, "1730300001.000100")], vec![]);
        said.messages = vec![message("D1", "1730300001.000100", None)];
        said.marks = vec![Mark {
            channel: "D1".to_owned(),
            read: "1730300000.000100".to_owned(),
            at: "2024-10-30T14:53:20Z".to_owned(),
        }];
        daemon.say(Some(&said), 10);
        let done = Done {
            how: "readInSlack",
            at: "2024-10-31T18:40:00Z".to_owned(),
            millis: 1_730_400_000_000,
        };
        said.conversations.clear();
        said.messages = vec![message("D1", "1730300001.000100", Some(done))];
        said.marks[0].read = "1730300001.000100".to_owned();
        daemon.say(Some(&said), 20);
        let lines: Vec<Value> = heard
            .try_iter()
            .map(|line: String| serde_json::from_str(&line).unwrap())
            .collect();
        let keys = |line: &Value| -> Vec<String> {
            line["changed"]
                .as_array()
                .unwrap()
                .iter()
                .map(|record| record["key"].as_str().unwrap().to_owned())
                .collect()
        };
        let message = "slack-message:D1:1730300001.000100";
        assert_eq!(keys(&lines[1]), [message, "slack-read:D1"]);
        assert_eq!(lines[1]["changed"][0]["slack-message"]["done"], Value::Null);
        // Read in Slack, the message and its mark change, and nothing goes.
        assert_eq!(keys(&lines[3]), [message, "slack-read:D1"]);
        assert_eq!(
            lines[3]["changed"][0]["slack-message"]["done"],
            json!({"how": "readInSlack", "at": "2024-10-31T18:40:00Z"})
        );
        assert_eq!(lines[3]["gone"], json!([]));
    }

    #[test]
    fn each_unread_each_read_last_and_the_account_is_a_record_in_slacks_order() {
        let said = ok(
            vec![
                one("D1", Kind::Dm, 2, "1730300000.000100"),
                one("C3", Kind::Unread, 0, "1730100000.000100"),
            ],
            vec![one("C1", Kind::Channel, 0, "1730700000.000100")],
        );
        let made: Vec<Value> = records(Some(&said)).into_iter().map(|(_, r)| r).collect();
        assert_eq!(
            made,
            [
                json!({"key": "slack-unread:D1", "kind": "slack-unread", "slack-unread": {
                    "id": "D1", "name": "Ada", "kind": "dm", "count": 2,
                    "link": "slack://channel?team=T1&id=D1", "latest": "1730300000.000100"}}),
                json!({"key": "slack-unread:C3", "kind": "slack-unread", "slack-unread": {
                    "id": "C3", "name": "Ada", "kind": "unread", "count": 0,
                    "link": "slack://channel?team=T1&id=C3", "latest": "1730100000.000100"}}),
                json!({"key": "slack-recent:C1", "kind": "slack-recent", "slack-recent": {
                    "id": "C1", "name": "Ada", "kind": "channel",
                    "link": "slack://channel?team=T1&id=C1", "read": "1730700000.000100"}}),
                json!({"key": "slack-account:slack", "kind": "slack-account",
                       "slack-account": {"standing": "ok"}}),
            ]
        );
    }

    #[test]
    fn each_conversations_read_mark_is_a_record_while_slack_answers() {
        let mut said = ok(vec![], vec![]);
        said.seen = vec![Seen {
            id: "C5".to_owned(),
            name: "#aquifer".to_owned(),
            read: "1730300000.000100".to_owned(),
        }];
        let made: Vec<Value> = records(Some(&said)).into_iter().map(|(_, r)| r).collect();
        assert_eq!(
            made[0],
            json!({"key": "slack-seen:C5", "kind": "slack-seen", "slack-seen": {
                "id": "C5", "name": "#aquifer", "read": "1730300000.000100"}}),
            "a conversation's read mark is said by its name"
        );
        said.state = State::Unreachable;
        let keys: Vec<String> = records(Some(&said)).into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, [ACCOUNT], "no mark outlives Slack answering");
    }

    #[test]
    fn only_the_account_is_held_while_slack_doesnt_answer_or_nobody_signed_in() {
        let mut said = ok(vec![one("D1", Kind::Dm, 2, "")], vec![]);
        said.state = State::Unreachable;
        let keys = |made: Vec<(String, Value)>| {
            made.into_iter()
                .map(|(_, record)| record)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            keys(records(Some(&said))),
            [json!({"key": ACCOUNT, "kind": "slack-account",
                    "slack-account": {"standing": "unreachable"}})]
        );
        assert_eq!(
            keys(records(None)),
            [json!({"key": ACCOUNT, "kind": "slack-account",
                    "slack-account": {"standing": "none"}})]
        );
    }

    #[test]
    fn a_record_is_heard_when_it_changes_not_each_time_slack_is_asked() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        let first = ok(
            vec![one("D1", Kind::Dm, 1, "1"), one("D2", Kind::Dm, 1, "1")],
            vec![],
        );
        daemon.say(Some(&first), 10);
        let second = ok(
            vec![one("D1", Kind::Dm, 2, "2"), one("D2", Kind::Dm, 1, "1")],
            vec![],
        );
        daemon.say(Some(&second), 20);
        let held = daemon.lock();
        let heard = |key: &str| held.records[key]["heard"].as_u64();
        assert_eq!(
            (
                heard("slack-unread:D1"),
                heard("slack-unread:D2"),
                heard(ACCOUNT)
            ),
            (Some(20), Some(10), Some(10))
        );
    }

    #[test]
    fn a_watch_of_messages_hears_only_how_slack_stands_as_that_changes() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        let (told, heard) = mpsc::channel();
        let asked = json!({"id": "all", "ask": "watch", "kind": "slack-message"});
        // Asked before Slack is, as a shell starting the daemon by its
        // socket asks.
        // Its answer is told on the watch's own line, before anything
        // else.
        assert_eq!(daemon.answer(&asked.to_string(), 1, &told), None);
        let said = ok(vec![one("D1", Kind::Dm, 1, "1")], vec![]);
        daemon.say(Some(&said), 10);
        daemon.say(Some(&said), 20);
        daemon.say(None, 30);
        let lines: Vec<Value> = heard
            .try_iter()
            .map(|line: String| serde_json::from_str(&line).unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                json!({"id": "all", "records": []}),
                json!({"id": "all", "sources": {"slack": "answering"}}),
                json!({"id": "all", "sources": {"slack": "signed-out"}})
            ]
        );
    }

    #[test]
    fn a_watch_hears_what_was_held_before_any_change_to_it() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        daemon.say(Some(&ok(vec![one("D1", Kind::Dm, 1, "1")], vec![])), 10);
        let (told, heard) = mpsc::channel();
        let asked = json!({"id": "all", "ask": "watch", "kind": "slack-unread"});
        // A change said from another thread as the watch is asked: either
        // before it, and in what it's told is held, or after, as a change.
        let saying = daemon.clone();
        let racing = std::thread::spawn(move || {
            saying.say(Some(&ok(vec![one("D1", Kind::Dm, 2, "2")], vec![])), 20);
        });
        daemon.answer(&asked.to_string(), 1, &told);
        racing.join().unwrap();
        let lines: Vec<Value> = heard
            .try_iter()
            .map(|line: String| serde_json::from_str(&line).unwrap())
            .collect();
        let count = |line: &Value, field: &str| line[field][0]["slack-unread"]["count"].clone();
        assert!(lines[0].get("records").is_some(), "{lines:?}");
        let last = lines.last().unwrap();
        let latest = if last.get("changed").is_some() {
            count(last, "changed")
        } else {
            count(last, "records")
        };
        assert_eq!(latest, json!(2), "{lines:?}");
    }

    #[test]
    fn rows_are_heard_by_a_watch_of_their_place_once_words_are_given() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        let said = ok(
            vec![one("D1", Kind::Dm, 2, "1")],
            vec![one("C1", Kind::Channel, 0, "1")],
        );
        daemon.say(Some(&said), 10);
        let (told, heard) = mpsc::channel();
        let listed = daemon.asked(&json!({"id": 1, "ask": "list", "kind": "row"}), 1);
        assert_eq!(listed["records"], json!([]), "rows made without words");
        daemon.words(Words {
            glyph: "chat".to_owned(),
            count: "{count} to read".to_owned(),
            ..Words::default()
        });
        let watch = |asked: Value| {
            daemon.answer(&asked.to_string(), 1, &told);
            serde_json::from_str::<Value>(&heard.recv().unwrap()).unwrap()
        };
        let unread =
            watch(json!({"id": 2, "ask": "watch", "kind": "row", "place": "slack-unread"}));
        let recent =
            watch(json!({"id": 3, "ask": "watch", "kind": "row", "place": ["slack-recent"]}));
        assert_eq!(
            unread["records"],
            json!([]),
            "rows said before Slack answered again"
        );
        assert_eq!(recent["records"], json!([]));
        daemon.say(Some(&said), 20);
        let lines: Vec<Value> = heard
            .try_iter()
            .map(|line: String| serde_json::from_str(&line).unwrap())
            .collect();
        let keys: Vec<(Value, Value)> = lines
            .iter()
            .map(|line| (line["id"].clone(), line["changed"][0]["key"].clone()))
            .collect();
        assert_eq!(
            keys,
            [
                (json!(2), json!("row:slack-unread:D1")),
                (json!(3), json!("row:slack-recent:C1"))
            ],
            "{lines:?}"
        );
        assert_eq!(
            lines[0]["changed"][0]["row"]["side"],
            json!([{"said": "2 to read"}])
        );
        // Asked again with nothing new, Slack's answer tells no watch.
        daemon.say(Some(&said), 30);
        assert!(heard.try_recv().is_err(), "a row unchanged was said again");
        let listed = daemon.asked(
            &json!({"id": 4, "ask": "list", "kind": "row", "place": "slack-recent"}),
            1,
        );
        assert_eq!(listed["records"][0]["key"], "row:slack-recent:C1");
        assert_eq!(listed["records"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn a_conversation_coming_first_is_the_one_row_a_watch_hears() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        daemon.words(Words::default());
        let first = vec![one("D1", Kind::Dm, 1, "1"), one("C1", Kind::Unread, 1, "1")];
        daemon.say(Some(&ok(first.clone(), vec![])), 10);
        let (told, heard) = mpsc::channel();
        let asked = json!({"id": 1, "ask": "watch", "kind": "row", "place": "slack-unread"});
        daemon.answer(&asked.to_string(), 1, &told);
        heard.recv().unwrap();
        let mut now = first;
        now.insert(0, one("D2", Kind::Dm, 1, "2"));
        daemon.say(Some(&ok(now, vec![])), 20);
        let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        let changed: Vec<(Value, Value)> = line["changed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| (record["key"].clone(), record["row"]["order"].clone()))
            .collect();
        assert_eq!(
            changed,
            [(json!("row:slack-unread:D2"), json!(0))],
            "{line}"
        );
        assert!(
            heard.try_recv().is_err(),
            "the rows below it were said again"
        );
    }

    #[test]
    fn a_messages_drawer_is_heard_only_by_a_watch_of_its_key() {
        let daemon = Daemon::new(PathBuf::new(), String::new());
        daemon.words(Words::default());
        let mut said = ok(vec![], vec![]);
        said.messages = vec![message("D1", "1730300001.000100", None)];
        // Said through Slack out of reach too, as the message is.
        said.state = State::Unreachable;
        let (told, heard) = mpsc::channel();
        let drawer = "drawer:slack-messages:D1:1730300001.000100";
        let rows = json!({"id": "rows", "ask": "watch", "kind": "row"});
        let drawers = json!({"id": "drawer", "ask": "watch", "kind": "drawer", "keys": [drawer]});
        daemon.answer(&rows.to_string(), 1, &told);
        daemon.answer(&drawers.to_string(), 1, &told);
        daemon.say(Some(&said), 10);
        let lines: Vec<Value> = heard
            .try_iter()
            .map(|line: String| serde_json::from_str(&line).unwrap())
            .filter(|line: &Value| line.get("changed").is_some())
            .collect();
        let heard: Vec<(Value, Value)> = lines
            .iter()
            .map(|line| (line["id"].clone(), line["changed"][0]["key"].clone()))
            .collect();
        assert_eq!(
            heard,
            [
                (
                    json!("rows"),
                    json!("row:slack-messages:D1:1730300001.000100")
                ),
                (json!("drawer"), json!(drawer))
            ],
            "{lines:?}"
        );
        assert_eq!(lines[0]["changed"].as_array().map(Vec::len), Some(1));
        assert_eq!(lines[1]["changed"][0]["drawer"]["body"], "hi");
    }

    #[test]
    fn a_kind_it_doesnt_hold_is_said_so() {
        assert_eq!(kinds(&json!({})), Ok(None));
        assert_eq!(
            kinds(&json!({"kind": ["slack-unread", "slack-account"]})),
            Ok(Some(
                ["slack-unread", "slack-account"]
                    .map(str::to_owned)
                    .into_iter()
                    .collect()
            ))
        );
        assert_eq!(
            kinds(&json!({"kind": "slack-channel"})),
            Err("slack-channel".to_owned())
        );
    }
}
