//! What Slack's daemon, slack-unreads, answers over its socket, in the
//! shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"), version 1: `hello`, `health`, `list`, `watch` and
//! `unwatch` of what it holds, `search` for Joel's words, and `get` of
//! what it holds or of a message a search found, with the few around it.
//!
//! It holds three kinds of record, made from what slack-unreads last
//! heard from Slack (`Daemon::say`): each conversation unread, a
//! `slack-unread:<id>`; each he read last, a `slack-recent:<id>`; and how
//! asking Slack stands, the one `slack-account:slack`. A record carries
//! its thing under a field named for its kind, as Google's daemon's do,
//! since a conversation has a `kind` of its own; and the timestamp it's
//! ordered by rather than its place, so a new message changes its own
//! record and not every one after it. A record's `heard` is when it last
//! changed, so a look at Slack that finds nothing new tells no watch
//! anything: the connection says the daemon is there, and its watchdog
//! restarts one that hangs, so nothing need be said to show it's alive.
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

use crate::search::{self, Failed};
use crate::unreads::{Conversation, Said, State};
use crate::{Account, Slack};

/// The interface's version.
pub const VERSION: u64 = 1;

/// The kinds this daemon answers for. A `slack-message` is asked of
/// Slack each time and never held, so a watch of it holds no records
/// and hears only how Slack stands, which is what Search Slack's watch
/// is for (SlackSearch.qml).
const KINDS: [&str; 4] = [
    "slack-unread",
    "slack-recent",
    "slack-account",
    "slack-message",
];

/// Every record what Slack last said makes, in its order: what's unread,
/// those wanting Joel first, then what he read last, then the account.
/// Only while Slack answered are any conversations said, so an old count
/// is never held as a current one; with no sign-in, `said` is none and
/// the account says so. Each is without its `heard`, which the daemon
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
    let shown = |one: &Conversation| json!({"id": one.id, "name": one.name, "kind": one.kind, "link": one.link});
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
    }
    made.push(record(
        "slack-account",
        "slack",
        json!({"standing": standing}),
    ));
    made
}

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
    told: Sender<String>,
}

impl Watch {
    fn sees(&self, key: &str) -> bool {
        let kind = key.split_once(':').map_or(key, |(kind, _)| kind);
        self.kinds.as_ref().is_none_or(|kinds| kinds.contains(kind))
            && self.keys.as_ref().is_none_or(|keys| keys.contains(key))
    }
}

#[derive(Default)]
struct Held {
    /// Every record, by key, with its `heard`.
    records: BTreeMap<String, Value>,
    /// The keys in the order Slack's said them.
    order: Vec<String>,
    /// How asking Slack last stood.
    stands: Stands,
    watches: Vec<Watch>,
    connections: u64,
}

/// What slack-unreads holds, as the interface serves it.
#[derive(Clone)]
pub struct Daemon {
    held: Arc<Mutex<Held>>,
    asking: Arc<Asking>,
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
        }
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
        for (key, mut record) in records(said) {
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
                let answer = daemon.answer(&line, connection, &told);
                let _ = told.send(answer.to_string());
            });
        }
        self.lock()
            .watches
            .retain(|watch| watch.connection != connection);
    }

    /// What one line asks, answered.
    fn answer(&self, line: &str, connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "that isn't an ask");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
        let watch = |kinds| Watch {
            connection,
            id: id.clone(),
            kinds,
            keys: strings(&said, "keys").map(|keys| keys.into_iter().collect()),
            told: told.clone(),
        };
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
            "list" | "watch" => {
                let kinds = match kinds(&said) {
                    Ok(kinds) => kinds,
                    Err(kind) => {
                        return error(
                            &id,
                            "unknown-kind",
                            &format!(
                                "this daemon keeps slack-unread, slack-recent, \
                                 slack-account and slack-message, not {kind}"
                            ),
                        );
                    }
                };
                let limit = said
                    .get("limit")
                    .and_then(Value::as_u64)
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(usize::MAX);
                let mut held = self.lock();
                let watch = watch(kinds);
                let records: Vec<Value> = held
                    .order
                    .iter()
                    .filter(|key| watch.sees(key))
                    .filter_map(|key| held.records.get(key))
                    .take(if asked == "list" { limit } else { usize::MAX })
                    .cloned()
                    .collect();
                if asked == "watch" {
                    // A watch asked again by its id replaces the one before.
                    held.watches
                        .retain(|was| !(was.connection == connection && was.id == id));
                    held.watches.push(watch);
                }
                json!({"id": id, "records": records})
            }
            "unwatch" => {
                self.lock()
                    .watches
                    .retain(|was| !(was.connection == connection && was.id == id));
                json!({"id": id, "unwatched": true})
            }
            "search" => self.search(&id, &said),
            "get" => self.get(&id, &said),
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
        match Slack::new(self.asking.origin.clone()).search(&account, words) {
            Ok(found) => json!({"id": id, "records": found}),
            Err(failed) => refused(id, &failed),
        }
    }

    /// Each key asked for, in the order asked: one held as it's held, and
    /// a message a search found with the few around it, asked of Slack.
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
            if let Some(record) = self.lock().records.get(key) {
                records.push(record.clone());
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

fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreads::Kind;

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
            checked: 0,
            state: State::Ok,
            conversations,
            recent,
        }
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
        let held = daemon.answer(&asked.to_string(), 1, &told);
        assert_eq!(held["records"], json!([]));
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
                json!({"id": "all", "sources": {"slack": "answering"}}),
                json!({"id": "all", "sources": {"slack": "signed-out"}})
            ]
        );
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
