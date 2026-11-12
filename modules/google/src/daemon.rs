//! What this program holds, served in the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"), version 1.
//!
//! Each meeting is a `meeting:<id>` record, each piece of mail that wants
//! Joel a `mail:<id>`, each document he looked at lately a `file:<id>`,
//! each meeting held in the month before today a `held:<id>`, each piece
//! of mail he sent or read lately a `lately:<id>`, each Meet call the
//! shell follows a `call:<code>`, and how the account
//! stands, its unread count, Meet, Drive's transfers and Drive's own
//! places one `account:google`, each place's files told in its own order,
//! which records alone don't keep. A record carries its thing under a
//! field named for its kind, since meetings and documents have a `kind`
//! of their own. A record's `heard` is when it last changed, so a look at
//! Google that finds nothing new tells a watch only that the account was
//! heard, rather than every record again.
//!
//! The asks are `hello`, `health`, `get`, `list`, `watch` and `unwatch`,
//! a `list` or `watch` taking one kind or a list of them, and `act`,
//! which hands its `act` to the same asks the old socket takes
//! (ask.rs), so the shell can do everything on one connection.
//!
//! Each piece of mail that wants Joel is also a `row:mail:<id>` record,
//! each in Mail's Recent a `row:lately:<id>`, each document a
//! `row:documents:<id>`, each meeting to come a `row:calendar:<id>`,
//! each held lately in Calendar's Recent a `row:held:<id>` and each call
//! in Meet's Recent a `row:meet:<code>`, the row the launcher shows for
//! it, worded in the vocabulary this program is handed (`rows.rs`,
//! `document_rows.rs`, `calendar_rows.rs`) and wrapped and ordered as
//! every daemon's (`ready.rs`), so the shell makes no rows of its own;
//! a `list` or `watch` of rows names its `place`, one or a list of them, to hear only
//! those. Each row's drawer is a `drawer:<place>:<id>` record of its own,
//! which the row names, watched by its key only while it shows, so a
//! mail's body never rides with its row. Without a vocabulary, no rows
//! or drawers are made.
//!
//! A `list` or a `watch` answers beside its records when Google was last
//! read (`read`, in milliseconds since the epoch, null until the first
//! read since the program started) and how the account stands
//! (`sources`), so an empty list read before Google answered is never
//! taken for nothing there: a switch's gate took one for no meeting
//! across a restart (2026-11-10).

use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use jiff::Timestamp;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::ask;
use crate::drive::Drive;
use crate::files::Fetched;
use crate::ready::{self, Ready};
use crate::rows::Vocabulary;
use crate::state::{Meet, Standing, State};

/// The interface's version.
pub const VERSION: u64 = 1;

/// The kinds this daemon keeps.
const KINDS: [&str; 9] = [
    "meeting", "mail", "file", "held", "lately", "call", "account", "row", "drawer",
];

/// The one account's key.
const ACCOUNT: &str = "account:google";

/// How the account stands, its record's thing.
#[derive(Serialize)]
struct Account<'a> {
    standing: Standing,
    unread: Option<u64>,
    fetched: &'a Option<Fetched>,
    meet: &'a Meet,
    drive: &'a Drive,
}

/// Every record the state makes, in its order: the meetings, the mail,
/// the documents, then the account. Each is without its `heard`, which
/// the daemon adds as it learns when one changed.
pub fn records(state: &State) -> Vec<(String, Value)> {
    fn record(kind: &str, id: &str, thing: Value) -> (String, Value) {
        let key = format!("{kind}:{id}");
        let mut fields = Map::new();
        fields.insert("key".into(), json!(key));
        fields.insert("kind".into(), json!(kind));
        fields.insert(kind.into(), thing);
        (key, Value::Object(fields))
    }
    fn as_value<T: Serialize>(thing: &T) -> Value {
        serde_json::to_value(thing).unwrap_or(Value::Null)
    }
    let mut made = Vec::new();
    // A meeting says how many are on it, as its Guests row counts them.
    let meeting_value = |meeting: &crate::calendar::Meeting| {
        let mut value = as_value(meeting);
        value["guestsCounted"] = json!(crate::calendar_rows::guests_counted(meeting));
        value
    };
    for meeting in &state.meetings {
        made.push(record("meeting", &meeting.id, meeting_value(meeting)));
    }
    // Mail done is still a mail record, its `done` saying how it went,
    // so a watch hears the one it held change rather than vanish.
    for mail in state.mail.iter().chain(&state.finished) {
        made.push(record("mail", &mail.id, as_value(mail)));
    }
    for document in &state.documents {
        made.push(record("file", &document.id, as_value(document)));
    }
    for meeting in &state.held {
        made.push(record("held", &meeting.id, meeting_value(meeting)));
    }
    for mail in &state.lately {
        made.push(record("lately", &mail.id, as_value(mail)));
    }
    for call in &state.calls {
        made.push(record("call", &call.code, as_value(call)));
    }
    let account = Account {
        standing: state.account,
        unread: state.unread,
        fetched: &state.fetched,
        meet: &state.meet,
        drive: &state.drive,
    };
    made.push(record("account", "google", as_value(&account)));
    made
}

/// The source's word for how the account stands, as the people daemon
/// says its sources' (`starting` until Google is first asked).
fn word(state: Option<&State>) -> (&'static str, &'static str) {
    match state.map(|state| state.account) {
        None => ("starting", ""),
        Some(Standing::None | Standing::SignedOut) => ("signed-out", ""),
        Some(Standing::Unreachable) => ("refused", "Google couldn't be reached"),
        Some(Standing::Ready) => ("answering", ""),
    }
}

/// A list's or a watch's answer: its records, when Google last answered
/// (none before it first has), and how the account stands.
fn listed<T: serde::Serialize>(id: &Value, records: &[T], state: Option<&State>) -> Value {
    json!({
        "id": id, "records": records, "read": state.and_then(|state| state.heard),
        "sources": {"google": word(state).0},
    })
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The kinds watched, or none for every kind.
    kinds: Option<HashSet<String>>,
    /// The keys watched, or none for every key of its kinds.
    keys: Option<HashSet<String>>,
    /// The places whose rows are watched, or none for every place's.
    places: Option<HashSet<String>>,
    told: Sender<String>,
}

impl Watch {
    fn sees(&self, key: &str) -> bool {
        let (kind, rest) = key.split_once(':').unwrap_or((key, ""));
        let place = rest.split_once(':').map_or(rest, |(place, _)| place);
        self.kinds.as_ref().is_none_or(|kinds| kinds.contains(kind))
            && self.keys.as_ref().is_none_or(|keys| keys.contains(key))
            && (!matches!(kind, "row" | "drawer")
                || self
                    .places
                    .as_ref()
                    .is_none_or(|places| places.contains(place)))
    }
}

#[derive(Default)]
struct Held {
    /// Every record, by key, with its `heard`.
    records: BTreeMap<String, Value>,
    /// The keys in the state's order.
    order: Vec<String>,
    /// The last state said, none before the first look at Google.
    state: Option<State>,
    /// The words rows are said in, none to make no rows.
    words: Option<Vocabulary>,
    /// Each place's rows as last told, which keeps their order.
    ready: Ready,
    watches: Vec<Watch>,
    since: i64,
    connections: u64,
}

/// What this program holds, as the interface serves it.
#[derive(Clone)]
pub struct Daemon {
    held: Arc<Mutex<Held>>,
}

impl Daemon {
    /// Serves the interface on the socket systemd hands this process, or
    /// else at `path`, in a folder only Joel reads, replacing a socket
    /// left behind. An `act` is sent on `asks`, beside whatever else asks
    /// this program.
    pub fn listen(path: &Path, asks: Sender<String>, now: i64) -> std::io::Result<Daemon> {
        let listener = match handed() {
            Some(listener) => listener,
            None => bound(path)?,
        };
        let daemon = Daemon {
            held: Arc::new(Mutex::new(Held {
                since: now,
                ..Held::default()
            })),
        };
        let serving = daemon.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let daemon = serving.clone();
                let asks = asks.clone();
                let connection = {
                    let mut held = daemon.lock();
                    held.connections += 1;
                    held.connections
                };
                thread::spawn(move || daemon.serve(stream, connection, &asks));
            }
        });
        Ok(daemon)
    }

    /// Makes ready rows from now on, said in these words.
    pub fn words(&self, words: Vocabulary) {
        self.lock().words = Some(words);
    }

    fn lock(&self) -> MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes the state as it now stands, at `now` in milliseconds since
    /// the epoch: each record that changed is said to each watch that
    /// sees it, and each that went as gone, and how the account stands
    /// whenever that changes.
    pub fn say(&self, state: &State, now: i64) {
        let mut guard = self.lock();
        let held = &mut *guard;
        let before = word(held.state.as_ref());
        let mut made = records(state);
        if let Some(words) = &held.words {
            made.extend(held.ready.made(state, words, now));
        }
        let mut changed = Vec::new();
        let mut order = Vec::with_capacity(made.len());
        let mut kept = BTreeMap::new();
        for (key, mut record) in made {
            // The account is heard each time Google answers; everything
            // else when it changes.
            let heard = if key == ACCOUNT {
                state.heard.unwrap_or(now)
            } else {
                match held.records.get(&key) {
                    Some(was) if same(was, &record) => was["heard"].as_i64().unwrap_or(now),
                    _ => now,
                }
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
        held.state = Some(state.clone());
        let after = word(held.state.as_ref());
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
            if told && before.0 != after.0 {
                let line = json!({"id": watch.id, "sources": {"google": after.0}});
                told = watch.told.send(line.to_string()).is_ok();
            }
            told
        });
    }

    /// Answers one client until it goes.
    fn serve(&self, stream: UnixStream, connection: u64, asks: &Sender<String>) {
        let Ok(reading) = stream.try_clone() else {
            return;
        };
        let (told, lines) = mpsc::channel::<String>();
        let mut writing = stream;
        thread::spawn(move || {
            for line in lines {
                if writeln!(writing, "{line}").is_err() {
                    break;
                }
            }
        });
        for line in BufReader::new(reading).lines() {
            let Ok(line) = line else { break };
            let answer = self.answer(&line, connection, &told, asks);
            if !answer.is_null() && told.send(answer.to_string()).is_err() {
                break;
            }
        }
        self.lock()
            .watches
            .retain(|watch| watch.connection != connection);
    }

    /// What one ask is answered with, or nothing once a watch's answer is
    /// sent.
    fn answer(
        &self,
        line: &str,
        connection: u64,
        told: &Sender<String>,
        asks: &Sender<String>,
    ) -> Value {
        let said: Value = serde_json::from_str(line).unwrap_or(Value::Null);
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said.get("ask").and_then(Value::as_str).unwrap_or_default();
        let kinds = match kinds(&said) {
            Ok(kinds) => kinds,
            Err(kind) => {
                return error(
                    &id,
                    "unknown-kind",
                    &format!(
                        "this daemon keeps meeting, mail, file, held, lately, call, \
                         account, row and drawer, not {kind}"
                    ),
                );
            }
        };
        let mut held = self.lock();
        match asked {
            "hello" => {
                let (state, _) = word(held.state.as_ref());
                json!({
                    "id": id, "version": VERSION, "domain": "google", "state": "answering",
                    "since": held.since, "sources": {"google": state},
                })
            }
            "health" => {
                let (state, reason) = word(held.state.as_ref());
                json!({
                    "id": id, "version": VERSION, "state": "answering",
                    "sources": {"google": {"state": state, "reason": reason}},
                    "records": held.records.len(), "watching": held.watches.len(),
                })
            }
            "get" => {
                let records: Vec<&Value> = strings(&said, "keys")
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|key| held.records.get(key))
                    .collect();
                json!({"id": id, "records": records})
            }
            "list" => {
                let limit = said
                    .get("limit")
                    .and_then(Value::as_u64)
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(usize::MAX);
                let watch = Watch {
                    connection,
                    id: id.clone(),
                    kinds,
                    keys: None,
                    places: places(&said),
                    told: told.clone(),
                };
                let records: Vec<&Value> = held
                    .order
                    .iter()
                    .filter(|key| watch.sees(key))
                    .filter_map(|key| held.records.get(key))
                    .take(limit)
                    .collect();
                listed(&id, &records, held.state.as_ref())
            }
            "watch" => {
                let watch = Watch {
                    connection,
                    id: id.clone(),
                    kinds,
                    keys: strings(&said, "keys").map(|keys| keys.into_iter().collect()),
                    places: places(&said),
                    told: told.clone(),
                };
                let records: Vec<Value> = held
                    .order
                    .iter()
                    .filter(|key| watch.sees(key))
                    .filter_map(|key| held.records.get(key))
                    .cloned()
                    .collect();
                // A watch asked again by its id replaces the one before.
                held.watches
                    .retain(|was| !(was.connection == connection && was.id == id));
                held.watches.push(watch);
                // Sent while Google's next answer waits on the lock, so
                // whatever it changes reaches the watch after these, and
                // the shell never takes them over that change.
                let answer = listed(&id, &records, held.state.as_ref());
                let _ = told.send(answer.to_string());
                Value::Null
            }
            "unwatch" => {
                held.watches
                    .retain(|was| !(was.connection == connection && was.id == id));
                json!({"id": id, "unwatched": true})
            }
            "act" => {
                drop(held);
                act(&id, &said, asks)
            }
            "search" => search(&held, &id, &said),
            _ => error(&id, "unknown-ask", &format!("no ask {asked:?}")),
        }
    }
}

/// A search answered only inside a meeting's Move, for a time typed
/// there, as of the clock now; any other is refused, for the shell to
/// match among its rows itself (ready-rows.md).
fn search(held: &Held, id: &Value, said: &Value) -> Value {
    let now = Timestamp::now().as_millisecond();
    let found = held
        .state
        .as_ref()
        .zip(held.words.as_ref())
        .and_then(|(state, words)| ready::typed(state, words, said, now));
    match found {
        Some(mut found) => {
            found["id"] = id.clone();
            found
        }
        None => error(id, "unknown-ask", "this daemon searches only within a Move"),
    }
}

/// An act asked of the account, handed to what holds it, or why it can't
/// be: one this program doesn't know, or no account held to do it.
fn act(id: &Value, said: &Value, asks: &Sender<String>) -> Value {
    let act = said.get("act").map(Value::to_string).unwrap_or_default();
    if ask::read(&act).is_none() {
        return error(id, "unknown-ask", "no such act");
    }
    if asks.send(act).is_err() {
        return error(id, "not-ready", "the account isn't being held");
    }
    json!({"id": id, "taken": true})
}

/// Whether two records hold the same, whenever each was heard.
fn same(was: &Value, now: &Value) -> bool {
    let mut was = was.clone();
    if let Some(fields) = was.as_object_mut() {
        fields.remove("heard");
    }
    was == *now
}

/// The kinds an ask names, as one or a list, none for every kind, or the
/// first this daemon doesn't keep.
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

fn strings(said: &Value, field: &str) -> Option<Vec<String>> {
    said.get(field)?.as_array().map(|all| {
        all.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}

fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// Its own socket, when run by hand, in a folder only Joel reads,
/// replacing one left behind.
fn bound(path: &Path) -> std::io::Result<UnixListener> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(path);
    UnixListener::bind(path)
}

/// The listening socket systemd hands this process (google.socket), as
/// the first descriptor after the standard three, when it says so
/// (`sd_listen_fds`), so the shell never finds it missing while this
/// program starts or starts again.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows;

    #[test]
    fn a_signed_out_account_is_one_record_of_how_it_stands() {
        let made = records(&State::without(Standing::SignedOut));
        assert_eq!(made.len(), 1);
        let (key, record) = &made[0];
        assert_eq!(key, ACCOUNT);
        assert_eq!(record["kind"], "account");
        assert_eq!(record["account"]["standing"], "signedOut");
    }

    #[test]
    fn a_watch_is_answered_before_any_change_said_after_it() {
        let daemon = Daemon {
            held: Arc::new(Mutex::new(Held::default())),
        };
        let (told, lines) = mpsc::channel();
        let (asks, _) = mpsc::channel();
        let answered = daemon.answer(r#"{"id": 1, "ask": "watch"}"#, 1, &told, &asks);
        // Google answers as the answer is on its way: what the watch is
        // told of it must come after the records it was answered with, or
        // the shell would take those older records over the change.
        daemon.say(&State::without(Standing::SignedOut), 1);
        if !answered.is_null() {
            told.send(answered.to_string()).unwrap();
        }
        let first: Value = serde_json::from_str(&lines.recv().unwrap()).unwrap();
        assert_eq!(
            first,
            json!({"id": 1, "records": [], "read": null, "sources": {"google": "starting"}})
        );
        let next: Value = serde_json::from_str(&lines.recv().unwrap()).unwrap();
        assert_eq!(next["changed"][0]["key"], ACCOUNT, "{next}");
    }

    #[test]
    fn mails_rows_are_heard_by_a_watch_of_their_place_once_words_are_given() {
        let daemon = Daemon {
            held: Arc::new(Mutex::new(Held::default())),
        };
        let mut state = State::without(Standing::Ready);
        state.mail = vec![crate::mail::Mail {
            id: "m1".into(),
            from: "Ada Lovelace".into(),
            address: "ada@example.com".into(),
            to: String::new(),
            cc: String::new(),
            others: Vec::new(),
            subject: "Engines".into(),
            invitation: None,
            sent: false,
            went: Vec::new(),
            at: 1_000,
            page: "https://mail.google.com/mail/u/0/#inbox/m1".into(),
            body: "On the engine.".into(),
            attachments: Vec::new(),
            done: None,
        }];
        let (told, lines) = mpsc::channel();
        let (asks, _) = mpsc::channel();
        daemon.say(&state, 1);
        let unworded = daemon.answer(
            r#"{"id": 1, "ask": "list", "kind": "row"}"#,
            1,
            &told,
            &asks,
        );
        assert_eq!(unworded["records"], json!([]), "rows made without words");
        daemon.words(Vocabulary {
            mail: Some(rows::Words {
                untitled: "Mail Without a Subject".into(),
                glyph: "mail".into(),
                ..rows::Words::default()
            }),
            ..Vocabulary::default()
        });
        let ask = |line: &str| {
            daemon.answer(line, 1, &told, &asks);
            serde_json::from_str::<Value>(&lines.recv().unwrap()).unwrap()
        };
        let mail = ask(r#"{"id": 2, "ask": "watch", "kind": "row", "place": "mail"}"#);
        let documents = ask(r#"{"id": 3, "ask": "watch", "kind": "row", "place": ["documents"]}"#);
        assert_eq!(mail["records"], json!([]), "rows said before the next look");
        daemon.say(&state, 2);
        let heard: Value = serde_json::from_str(&lines.recv().unwrap()).unwrap();
        assert_eq!(
            heard["id"], 2,
            "a watch of documents heard mail's rows: {heard}"
        );
        assert_eq!(heard["changed"][0]["key"], "row:mail:m1");
        assert_eq!(heard["changed"][0]["row"]["thing"], "mail:m1");
        assert_eq!(heard["changed"][0]["row"]["name"], "Engines");
        assert!(lines.try_recv().is_err(), "more was said than mail's row");
        assert_eq!(documents["records"], json!([]));
        // Its drawer is heard only by a watch of its key.
        let drawer =
            ask(r#"{"id": 4, "ask": "watch", "kind": "drawer", "keys": ["drawer:mail:m1"]}"#);
        assert_eq!(drawer["records"][0]["drawer"]["title"], "Engines");
        assert_eq!(drawer["records"][0]["drawer"]["body"], "On the engine.");
        assert!(
            !heard.to_string().contains("On the engine.")
                && heard["changed"][0]["row"]["drawer"] == "drawer:mail:m1",
            "a row carried its drawer: {heard}"
        );
    }

    fn document(id: &str, title: &str) -> crate::drive::Document {
        crate::drive::Document {
            id: id.into(),
            title: title.into(),
            kind: crate::drive::Is::Document,
            link: format!("https://docs.google.com/document/d/{id}/edit"),
            viewed: Some(5_000),
            changed: Some(4_000),
            owner: Some("Ada Lovelace".into()),
            mime: "application/vnd.google-apps.document".into(),
            size: None,
            created: None,
            changed_by: None,
            changed_by_me: None,
            shared_with_me: None,
            shared_by: None,
            shared: false,
            starred: false,
            drive: None,
            about: None,
            across: None,
            down: None,
            lasts: None,
        }
    }

    #[test]
    fn documents_rows_reach_their_places_watch_and_a_drawer_watch_hears_only_its_changes() {
        let daemon = Daemon {
            held: Arc::new(Mutex::new(Held::default())),
        };
        let (words, lacking) = Vocabulary::read(&rows::marked::vocabulary());
        assert_eq!(
            lacking,
            Vec::<String>::new(),
            "the launcher's words lack some"
        );
        daemon.words(words);
        let mut state = State::without(Standing::Ready);
        state.documents = vec![document("a", "Plan"), document("b", "Budget")];
        let (told, lines) = mpsc::channel();
        let (asks, _) = mpsc::channel();
        daemon.say(&state, 1);
        let ask = |line: &str| {
            daemon.answer(line, 1, &told, &asks);
            serde_json::from_str::<Value>(&lines.recv().unwrap()).unwrap()
        };
        let listed = ask(r#"{"id": 1, "ask": "watch", "kind": "row", "place": "documents"}"#);
        let keys: Vec<&str> = listed["records"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| record["key"].as_str())
            .collect();
        assert_eq!(keys, ["row:documents:a", "row:documents:b"], "{listed}");
        let drawn = ask(r#"{"id": 2, "ask": "watch", "kind": "drawer",
                "keys": ["drawer:documents:a", "drawer:documents:b"]}"#);
        assert_eq!(drawn["records"][1]["drawer"]["facts"][1]["name"], "Owner");
        state.documents[1].owner = Some("Grace Hopper".into());
        daemon.say(&state, 2);
        let heard: Value = serde_json::from_str(&lines.recv().unwrap()).unwrap();
        assert_eq!(
            heard["id"], 2,
            "the row watch heard a drawer's change: {heard}"
        );
        let changed: Vec<&str> = heard["changed"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| record["key"].as_str())
            .collect();
        assert_eq!(changed, ["drawer:documents:b"], "{heard}");
        assert!(lines.try_recv().is_err(), "more was said than b's drawer");
    }

    #[test]
    fn a_kind_is_one_or_a_list_and_never_one_it_doesnt_keep() {
        assert_eq!(kinds(&json!({})), Ok(None));
        assert_eq!(
            kinds(&json!({"kind": "mail"})),
            Ok(Some(HashSet::from(["mail".to_owned()])))
        );
        assert_eq!(
            kinds(&json!({"kind": ["mail", "meeting"]})),
            Ok(Some(HashSet::from([
                "mail".to_owned(),
                "meeting".to_owned()
            ])))
        );
        assert_eq!(
            kinds(&json!({"kind": ["mail", "person"]})),
            Err("person".to_owned())
        );
    }
}
