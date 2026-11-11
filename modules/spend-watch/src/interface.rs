//! The collection told over the daemon's socket, in version 1 of the
//! shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"): JSON lines, each ask with an `id` echoed in its answers.
//! A watch is answered with every record of the kinds it names, then,
//! after each pass, with only the records that pass changed and the keys
//! it let go, and nothing when it changed none.
//!
//! The shell reads the whole collection again for each pass otherwise,
//! about 228 KB every 37 s, though most of it is only drawn while the
//! launcher's Spend is (2026-11-10). So the collection and the budgets
//! are cut into records of a few kinds (`records.rs`), each kind watched
//! only while it's drawn.
//!
//! collection.json and budgets.json are still written whole for the
//! command line, and for the shell while the socket isn't there.
//!
//! Long because its tests, nearly half of it, drive the socket itself.

use crate::acts::{self, Doer};
use crate::collection::Collection;
use crate::ready::{Served, Words, in_place};
use crate::records::{BUDGETS, budget_records, cut, cut_budgets, kind_of, kinds};
use crate::standing::Standing;
use crate::status::ShellAction;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// Lines queued for a reader before it's hung up on as too slow, which
/// asks again and is told everything whole.
const QUEUED: usize = 64;

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Records by key, as they stand now.
    Get {
        id: Value,
        keys: Vec<String>,
    },
    /// Every record of these kinds, of one launcher's place where it
    /// names one.
    List {
        id: Value,
        kinds: Vec<String>,
        place: Option<String>,
    },
    /// Records of these kinds as they change, as `List` names them.
    Watch {
        id: Value,
        kinds: Vec<String>,
        place: Option<String>,
    },
    Unwatch {
        id: Value,
    },
    Health {
        id: Value,
    },
    /// A kind the collection isn't told as.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// Something to do of a line the collection holds, by its key
    /// (`acts`).
    Act {
        id: Value,
        act: String,
        line: String,
    },
    /// An ask this doesn't know, or a line that isn't one.
    Unknown {
        id: Value,
        ask: String,
    },
}

/// What a line from a client asks.
pub fn ask(line: &str) -> Ask {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Ask::Unknown {
            id: Value::Null,
            ask: String::new(),
        };
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said
        .get("ask")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let named = || kinds(&said);
    let place = said["place"].as_str().map(str::to_owned);
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: said
                .get("keys")
                .and_then(Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(Value::as_str)
                        .filter(|key| kind_of(key).is_some())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        },
        "list" => match named() {
            Ok(kinds) => Ask::List { id, kinds, place },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "watch" => match named() {
            Ok(kinds) => Ask::Watch { id, kinds, place },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "unwatch" => Ask::Unwatch { id },
        "health" => Ask::Health { id },
        "act" => Ask::Act {
            id,
            act: said["act"].as_str().unwrap_or_default().to_owned(),
            line: said["line"].as_str().unwrap_or_default().to_owned(),
        },
        _ => Ask::Unknown { id, ask: asked },
    }
}

/// An error answer, by the interface's codes.
fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// What the launcher's vocabulary says for each code an act is refused
/// with, spend's own `refused:<code>` words, so the launcher says a
/// refusal as the daemon words it; a message is for the journal alone.
pub fn refusals(vocabulary: &str) -> BTreeMap<String, String> {
    let Ok(read) = serde_json::from_str::<Value>(vocabulary) else {
        return BTreeMap::new();
    };
    let spend = read["things"].as_array().into_iter().flatten();
    spend
        .filter(|thing| thing["id"] == "spend")
        .filter_map(|thing| thing["said"].as_object())
        .flatten()
        .filter_map(|(key, said)| {
            Some((
                key.strip_prefix("refused:")?.to_owned(),
                said.as_str()?.to_owned(),
            ))
        })
        .collect()
}

/// One watch: the kinds it's told, and where.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    place: Option<String>,
    told: SyncSender<String>,
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        kind_of(key).is_some_and(|kind| self.kinds.iter().any(|wanted| wanted == kind))
            && in_place(key, self.place.as_deref())
    }
}

#[derive(Default)]
struct Held {
    records: BTreeMap<String, Value>,
    /// What the bar's piece offers, as the last pass wrote `status.json`,
    /// by each action's key.
    offered: BTreeMap<String, Value>,
    /// The launcher's rows as they were last served ready (`ready.rs`).
    served: Served,
    /// Whether a pass has said the collection yet: until it has, the
    /// state is "starting".
    passed: bool,
    watches: Vec<Watch>,
    connections: u64,
}

/// The records the last pass told, and every watch of them, shared by the
/// loop that passes and each connection.
pub struct Hub {
    held: Mutex<Held>,
    /// When the daemon started, in milliseconds since the epoch.
    since: u64,
    /// What does an act, given once the daemon can.
    doer: OnceLock<Doer>,
    /// The launcher's words the rows served ready say.
    words: Words,
    /// The launcher's words for an act refused, by its code.
    refusals: BTreeMap<String, String>,
}

impl Hub {
    pub fn new(since: u64) -> Hub {
        Hub {
            held: Mutex::new(Held::default()),
            since,
            doer: OnceLock::new(),
            words: Words::default(),
            refusals: BTreeMap::new(),
        }
    }

    /// The hub, its ready rows saying the launcher's words.
    #[must_use]
    pub fn worded(self, words: Words) -> Hub {
        Hub { words, ..self }
    }

    /// The hub, its refusals saying the launcher's words (`refusals`).
    #[must_use]
    pub fn refusing(self, refusals: BTreeMap<String, String>) -> Hub {
        Hub { refusals, ..self }
    }

    /// An act refused, worded where the vocabulary words its code.
    fn refused(&self, id: &Value, code: &str, message: &str) -> Value {
        let mut answer = error(id, code, message);
        if let Some(said) = self.refusals.get(code) {
            answer["error"]["said"] = json!(said);
        }
        answer
    }

    /// Gives the hub what does its acts; only the first is kept.
    pub fn doing(&self, doer: Doer) {
        let _ = self.doer.set(doer);
    }

    /// Holds what the bar's piece offers, as a pass wrote it.
    pub fn offer(&self, actions: &[ShellAction]) {
        let offered = actions.iter().map(|one| (one.key.clone(), json!(one)));
        self.lock().offered = offered.collect();
    }

    /// An act asked of the line `line` names, a row's or an action the
    /// bar's piece offers, done off the lock, since bringing a window
    /// forward waits on kitty and niri. A hub with nothing to do it, as
    /// one serving only what was written (`daemon::serve`), refuses every
    /// act as not answering.
    fn act(&self, id: &Value, act: &str, line: &str) -> Value {
        let Some(doer) = self.doer.get() else {
            return self.refused(id, "not-answering", "spend-watch can't act yet");
        };
        let held = {
            let held = self.lock();
            held.records
                .values()
                .find(|record| record["line"]["key"] == line)
                .map(|record| record["line"].clone())
                .or_else(|| held.offered.get(line).cloned())
        };
        let doing = match acts::asked(act, line, held.as_ref()) {
            Ok(doing) => doing,
            Err(refusal) => return self.refused(id, refusal.code, &refusal.message),
        };
        match doer(doing) {
            Ok(()) => json!({"id": id, "done": true}),
            Err(why) => self.refused(id, "not-done", &why),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Held> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Holds what the daemon wrote last, `collection.json`'s text, until
    /// its first pass, so a shell watching meanwhile is told those figures
    /// rather than none: reading the history again before that pass can
    /// take minutes. A collection of another shape is passed over.
    pub fn seed(&self, written: &str) {
        let Ok(whole) = serde_json::from_str::<Value>(written) else {
            return;
        };
        if whole["v"].as_u64() != Some(u64::from(crate::collection::VERSION)) {
            return;
        }
        let mut held = self.lock();
        if !held.passed {
            let budgets: Vec<(String, Value)> = std::mem::take(&mut held.records)
                .into_iter()
                .filter(|(key, _)| kind_of(key).is_some_and(|kind| BUDGETS.contains(&kind)))
                .collect();
            let served = held.served.take(&whole, &self.words);
            held.records = served;
            held.records.extend(cut(whole));
            held.records.extend(budgets);
        }
    }

    /// Holds what the daemon wrote last of the budgets, `budgets.json`'s
    /// text, until its first pass, as `seed` holds the collection.
    pub fn seed_budgets(&self, written: &str) {
        let Ok(whole) = serde_json::from_str::<Value>(written) else {
            return;
        };
        if whole["v"].as_u64() != Some(u64::from(crate::standing::VERSION)) {
            return;
        }
        let mut held = self.lock();
        if !held.passed {
            held.records.extend(cut_budgets(&whole));
        }
    }

    /// Says what was written is what it answers with, as a first pass
    /// would, for a hub serving only that (`daemon::serve`).
    pub fn answering(&self) {
        self.lock().passed = true;
    }

    /// Takes what a pass made, telling each watch what changed of its
    /// kinds and what went. The first pass tells each connection watching
    /// that the daemon is answering now, as hello would, since what it
    /// was told before was the last run's and said as such.
    pub fn publish(&self, collection: &Collection, budgets: Option<&Standing>) {
        let whole = serde_json::to_value(collection).unwrap_or_default();
        self.tell(whole, budgets.map(budget_records).unwrap_or_default());
    }

    /// Takes the files as they were written again, `collection.json`'s
    /// text and `budgets.json`'s, telling each watch what changed as a
    /// pass would, for a hub serving only what was written
    /// (`daemon::serve`), so a fixture that writes them anew is heard. A
    /// collection of another shape is passed over, and budgets that can't
    /// be read are none. A collection taken away is an empty one, so what
    /// it held is told gone, as when a fixture removes it to show none.
    pub fn rewritten(&self, collection: &str, budgets: &str) {
        let whole = if collection.is_empty() {
            json!({})
        } else {
            let Ok(whole) = serde_json::from_str::<Value>(collection) else {
                return;
            };
            if whole["v"].as_u64() != Some(u64::from(crate::collection::VERSION)) {
                return;
            }
            whole
        };
        let standing = serde_json::from_str::<Value>(budgets)
            .ok()
            .filter(|said| said["v"].as_u64() == Some(u64::from(crate::standing::VERSION)));
        self.tell(
            whole,
            standing.as_ref().map(cut_budgets).unwrap_or_default(),
        );
    }

    /// Tells each watch what changed between the records held and those
    /// `whole` and `budgets` make, and holds those.
    fn tell(&self, whole: Value, budgets: BTreeMap<String, Value>) {
        let mut held = self.lock();
        let mut now = held.served.take(&whole, &self.words);
        now.extend(cut(whole));
        now.extend(budgets);
        let first = !held.passed;
        held.passed = true;
        let was = std::mem::take(&mut held.records);
        let mut greeted = BTreeSet::new();
        held.watches.retain(|watch| {
            if first && greeted.insert(watch.connection) {
                let line = json!({
                    "id": "hello", "version": VERSION, "domain": "spend",
                    "state": "answering", "since": self.since,
                });
                if watch.told.try_send(line.to_string()).is_err() {
                    let _ = watch.stream.shutdown(Shutdown::Both);
                    return false;
                }
            }
            let changed: Vec<&Value> = now
                .iter()
                .filter(|(key, record)| watch.wants(key) && was.get(*key) != Some(*record))
                .map(|(_, record)| record)
                .collect();
            let gone: Vec<&String> = was
                .keys()
                .filter(|key| watch.wants(key) && !now.contains_key(*key))
                .collect();
            if changed.is_empty() && gone.is_empty() {
                return true;
            }
            let line = json!({"id": watch.id, "changed": changed, "gone": gone}).to_string();
            // A reader this far behind is let go, and told everything whole
            // when it asks again, rather than held up to.
            if watch.told.try_send(line).is_err() {
                let _ = watch.stream.shutdown(Shutdown::Both);
                return false;
            }
            true
        });
        held.records = now;
    }

    fn state(held: &Held) -> &'static str {
        if held.passed { "answering" } else { "starting" }
    }

    /// Serves one connection whose first line was `first`, then each line
    /// `reading` holds, until it closes.
    pub fn serve(&self, first: &str, reading: impl BufRead, stream: UnixStream) {
        let Ok(hangup) = stream.try_clone().map(Arc::new) else {
            return;
        };
        let connection = {
            let mut held = self.lock();
            held.connections += 1;
            held.connections
        };
        let (told, lines) = mpsc::sync_channel::<String>(QUEUED);
        let mut writing = stream;
        let writer = std::thread::spawn(move || {
            for line in lines {
                if writeln!(writing, "{line}").is_err() {
                    break;
                }
            }
        });
        let asks = std::iter::once(first.to_owned()).chain(reading.lines().map_while(Result::ok));
        for line in asks {
            let Some(answer) = self.answer(ask(&line), connection, &told, &hangup) else {
                continue;
            };
            if told.send(answer.to_string()).is_err() {
                break;
            }
        }
        self.lock()
            .watches
            .retain(|watch| watch.connection != connection);
        drop(told);
        let _ = writer.join();
    }

    /// What one ask is answered with, or none where the answer is already
    /// queued to be said.
    fn answer(
        &self,
        asked: Ask,
        connection: u64,
        told: &SyncSender<String>,
        hangup: &Arc<UnixStream>,
    ) -> Option<Value> {
        if let Ask::Act { id, act, line } = &asked {
            return Some(self.act(id, act, line));
        }
        let mut held = self.lock();
        Some(match asked {
            Ask::Hello { id } => json!({
                "id": id, "version": VERSION, "domain": "spend", "state": Hub::state(&held),
                "since": self.since,
            }),
            Ask::Get { id, keys } => {
                let found: Vec<&Value> = keys.iter().filter_map(|k| held.records.get(k)).collect();
                json!({"id": id, "records": found})
            }
            Ask::List { id, kinds, place } => {
                json!({"id": id, "records": of(&held, &kinds, place.as_deref())})
            }
            Ask::Watch { id, kinds, place } => {
                // A watch asked again by its id, with more kinds or fewer,
                // replaces the one before and is told its kinds whole.
                held.watches
                    .retain(|watch| !(watch.connection == connection && watch.id == id));
                // Queued under the lock that publishing takes, so no pass's
                // change is said before the whole it moved on from.
                let first = json!({"id": id, "records": of(&held, &kinds, place.as_deref())});
                if told.try_send(first.to_string()).is_err() {
                    let _ = hangup.shutdown(Shutdown::Both);
                    return None;
                }
                held.watches.push(Watch {
                    connection,
                    id,
                    kinds,
                    place,
                    told: told.clone(),
                    stream: Arc::clone(hangup),
                });
                return None;
            }
            Ask::Unwatch { id } => {
                held.watches
                    .retain(|watch| !(watch.connection == connection && watch.id == id));
                json!({"id": id, "unwatched": true})
            }
            Ask::Health { id } => json!({
                "id": id, "version": VERSION, "domain": "spend", "state": Hub::state(&held),
                "since": self.since, "watches": held.watches.len(),
            }),
            Ask::UnknownKind { id, kind } => {
                error(&id, "unknown-kind", &format!("no kind {kind} here"))
            }
            Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
            // Answered above, off the lock.
            Ask::Act { .. } => return None,
        })
    }
}

/// Every record held of these kinds, of `place` where it's named, by key.
fn of<'a>(held: &'a Held, kinds: &[String], place: Option<&str>) -> Vec<&'a Value> {
    held.records
        .iter()
        .filter(|(key, _)| kind_of(key).is_some_and(|kind| kinds.iter().any(|k| k == kind)))
        .filter(|(key, _)| in_place(key, place))
        .map(|(_, record)| record)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::records;
    use crate::report::{Family, Helper};
    use std::io::{BufReader, Read};
    use std::time::Duration;

    fn collection() -> Collection {
        let mut collection = crate::collection::held(1);
        collection.sessions = BTreeMap::from([
            ("a".to_owned(), "$1.00".to_owned()),
            ("b".to_owned(), "$2.00".to_owned()),
        ]);
        collection.helpers = BTreeMap::from([(
            "a".to_owned(),
            vec![Helper {
                on: "councils".into(),
                spent: "$0.50".into(),
            }],
        )]);
        collection.families = BTreeMap::from([(
            "QEST-1".to_owned(),
            Family {
                all: "$3.00".into(),
                own: "$1.00".into(),
                under: "$2.00".into(),
            },
        )]);
        collection.today = "$3.00 today".into();
        collection
    }

    /// A connection to the hub, served on a thread of its own, and what
    /// it's told, line by line.
    struct Client {
        stream: UnixStream,
        heard: BufReader<UnixStream>,
    }

    impl Client {
        fn to(hub: &Arc<Hub>, first: &str) -> Client {
            let (ours, theirs) = UnixStream::pair().expect("a pair of sockets");
            let hub = Arc::clone(hub);
            let first = first.to_owned();
            std::thread::spawn(move || {
                let reading = BufReader::new(theirs.try_clone().expect("a reading half"));
                hub.serve(&first, reading, theirs);
            });
            ours.set_read_timeout(Some(Duration::from_secs(5)))
                .expect("a read timeout");
            let heard = BufReader::new(ours.try_clone().expect("a reading half"));
            Client {
                stream: ours,
                heard,
            }
        }

        fn say(&mut self, line: &str) {
            writeln!(self.stream, "{line}").expect("said");
        }

        fn next(&mut self) -> Value {
            let mut line = String::new();
            self.heard.read_line(&mut line).expect("a line");
            serde_json::from_str(&line).expect("a line of JSON")
        }

        /// Whether anything more is said within a little while.
        fn silent(&mut self) -> bool {
            self.stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("a read timeout");
            let mut byte = [0];
            matches!(
                self.heard.read(&mut byte),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
            )
        }
    }

    fn keys(answer: &Value, field: &str) -> Vec<String> {
        answer[field]
            .as_array()
            .expect("a list of records")
            .iter()
            .map(|record| {
                record["key"]
                    .as_str()
                    .or_else(|| record.as_str())
                    .expect("a key")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn a_session_is_one_record_of_its_three_figures() {
        let made = records(&collection());
        assert_eq!(
            made["session:a"],
            json!({"key": "session:a", "kind": "session", "spent": "$1.00",
                "helpers": [{"on": "councils", "spent": "$0.50"}]}),
            "session a's spend and helpers should be one record"
        );
        assert_eq!(
            made["day:all"]["spent"], "$3.00 today",
            "today's figure should be day:all's"
        );
        assert_eq!(
            made["quest:QEST-1"]["all"], "$3.00",
            "a quest's family should be its record's fields"
        );
        assert_eq!(
            made["row:findings:held"]["order"], 0,
            "a row should keep its place in its list"
        );
        assert!(
            made.contains_key("report:summary") && made.contains_key("words:periods"),
            "each report and word list should be a record: {:?}",
            made.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_watch_is_told_its_kinds_whole_and_then_only_what_changed() {
        let hub = Arc::new(Hub::new(0));
        let mut was = collection();
        hub.publish(&was, None);
        let mut client = Client::to(
            &hub,
            r#"{"id":"all","ask":"watch","kind":["session","day"]}"#,
        );
        let whole = client.next();
        assert_eq!(
            keys(&whole, "records"),
            ["day:agents", "day:all", "session:a", "session:b"],
            "the watch should be told its kinds whole, and no report"
        );
        was.sessions.insert("b".into(), "$2.50".into());
        was.reports.clear();
        hub.publish(&was, None);
        let change = client.next();
        assert_eq!(
            change["changed"],
            json!([{"key": "session:b", "kind": "session", "spent": "$2.50"}]),
            "only the session that changed should be told, and no report that went"
        );
        assert_eq!(change["gone"], json!([]), "nothing it watches went");
        let said = change.to_string().len();
        assert!(
            said < 120,
            "one session's change should be a short line, not {said} bytes: {change}"
        );
        hub.publish(&was, None);
        assert!(
            client.silent(),
            "a pass that changed nothing should tell nothing"
        );
    }

    #[test]
    fn a_watch_asked_again_wider_is_told_the_new_kinds_whole() {
        let hub = Arc::new(Hub::new(0));
        hub.publish(&collection(), None);
        let mut client = Client::to(&hub, r#"{"id":"all","ask":"watch","kind":"session"}"#);
        client.next();
        client.say(r#"{"id":"all","ask":"watch","kind":["session","report","words"]}"#);
        let wider = client.next();
        let told = keys(&wider, "records");
        assert!(
            told.contains(&"report:summary".to_owned())
                && told.contains(&"words:cuts".to_owned())
                && told.contains(&"session:a".to_owned()),
            "the wider watch should be told every kind it names whole: {told:?}"
        );
        let mut changed = collection();
        changed.sessions.insert("a".into(), "$9.00".into());
        hub.publish(&changed, None);
        let change = client.next();
        assert_eq!(
            keys(&change, "changed"),
            ["session:a"],
            "the wider watch should replace the first, told once a change"
        );
        assert!(client.silent(), "the first watch should be gone");
    }

    #[test]
    fn what_a_pass_let_go_is_told_as_gone() {
        let hub = Arc::new(Hub::new(0));
        hub.publish(&collection(), None);
        let mut client = Client::to(&hub, r#"{"id":"all","ask":"watch","kind":"session"}"#);
        client.next();
        let mut fewer = collection();
        fewer.sessions.remove("b");
        hub.publish(&fewer, None);
        assert_eq!(
            client.next()["gone"],
            json!(["session:b"]),
            "a session no longer in the collection should be told gone"
        );
    }

    #[test]
    fn hello_says_starting_until_a_pass_has_told_the_collection() {
        let hub = Arc::new(Hub::new(7));
        let mut client = Client::to(&hub, r#"{"id":"hello","ask":"hello","version":1}"#);
        let hello = client.next();
        assert_eq!(
            (
                hello["state"].as_str(),
                hello["domain"].as_str(),
                hello["since"].as_u64()
            ),
            (Some("starting"), Some("spend"), Some(7)),
            "before any pass the daemon should say it's starting: {hello}"
        );
        hub.publish(&collection(), None);
        client.say(r#"{"id":"again","ask":"hello"}"#);
        assert_eq!(
            client.next()["state"],
            "answering",
            "after a pass the daemon should say it's answering"
        );
    }

    #[test]
    fn what_was_written_last_is_told_until_the_first_pass() {
        let hub = Arc::new(Hub::new(0));
        let written = serde_json::to_string(&collection()).expect("a collection as text");
        hub.seed(r#"{"v":1,"sessions":{"old":"$1"}}"#);
        hub.seed(&written);
        let mut client = Client::to(&hub, r#"{"id":"all","ask":"watch","kind":"session"}"#);
        let keys: Vec<String> = client.next()["records"]
            .as_array()
            .expect("records")
            .iter()
            .filter_map(|record| record["key"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(
            keys,
            records(&collection())
                .into_keys()
                .filter(|key| key.starts_with("session:"))
                .collect::<Vec<_>>(),
            "a watch before the first pass should be told what was written last"
        );
        hub.publish(&collection(), None);
        assert_eq!(
            client.next()["state"],
            "answering",
            "the first pass should tell a watch the daemon is answering now"
        );
        assert!(
            client.silent(),
            "a first pass that says what was written should tell nothing more"
        );
        hub.seed(r#"{"v":2}"#);
        client.say(r#"{"id":"all","ask":"watch","kind":"session"}"#);
        assert_eq!(
            client.next()["records"].as_array().map(Vec::len),
            Some(keys.len()),
            "once a pass has told the collection, what was written is passed over"
        );
    }

    // What was written, served as it stands, tells a watch what a pass
    // making it tells, answering, so a fixture writing the files stands in
    // for the daemon (daemon.rs's serve); and with nothing to do them, it
    // refuses every act.
    #[test]
    fn what_was_written_is_told_as_a_pass_tells_it() {
        let made = collection();
        let written = Arc::new(Hub::new(1));
        written.seed(&serde_json::to_string(&made).expect("the collection, written"));
        written.answering();
        let passed = Arc::new(Hub::new(1));
        passed.publish(&made, None);
        let watched = r#"{"id":"all","ask":"watch","kind":["session","quest","row","summary"]}"#;
        let told = |hub: &Arc<Hub>| Client::to(hub, watched).next();
        let first = told(&written);
        assert!(
            first["records"].as_array().is_some_and(|all| all.len() > 3),
            "the written collection should tell its records: {first}"
        );
        assert_eq!(
            first,
            told(&passed),
            "what was written should tell what a pass does"
        );
        let mut client = Client::to(&written, r#"{"id":1,"ask":"hello"}"#);
        assert_eq!(
            client.next()["state"],
            "answering",
            "it answers with what was written"
        );
        client.say(r#"{"id":2,"ask":"act","act":"run","line":"held"}"#);
        assert_eq!(
            client.next()["error"]["code"],
            "not-answering",
            "with nothing to do it, every act is refused as not answering"
        );
    }

    #[test]
    fn what_is_written_again_is_told_as_it_changes() {
        let hub = Arc::new(Hub::new(1));
        hub.seed(&serde_json::to_string(&collection()).expect("the collection, written"));
        hub.answering();
        let mut client = Client::to(&hub, r#"{"id":"all","ask":"watch","kind":"summary"}"#);
        client.next();
        let mut again = serde_json::to_value(collection()).expect("the collection, as a value");
        again["today"] = json!("$9 today");
        hub.rewritten(&again.to_string(), "");
        let told = client.next();
        let said: Vec<&str> = told["changed"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| record["summary"]["said"].as_str())
            .collect();
        assert_eq!(
            said,
            ["$9 today"],
            "a rewrite should tell what it changed: {told}"
        );
        hub.rewritten(&again.to_string(), "");
        hub.rewritten("{}", "");
        assert!(
            client.silent(),
            "the same written again, or another shape, should tell nothing"
        );
    }

    #[test]
    fn a_refusal_says_the_launchers_words_for_it() {
        let vocabulary = r#"{"things": [{"id": "spend", "said": {
            "down": "not answering", "refused:not-answering": "the watch can't act yet"}}]}"#;
        let hub = Arc::new(Hub::new(1).refusing(refusals(vocabulary)));
        hub.answering();
        let mut client = Client::to(&hub, r#"{"id":1,"ask":"act","act":"run","line":"a"}"#);
        assert_eq!(
            client.next()["error"],
            json!({"code": "not-answering", "message": "spend-watch can't act yet",
                "said": "the watch can't act yet"}),
            "a refusal should say the vocabulary's words, its message kept for the journal"
        );
    }

    #[test]
    fn a_collection_taken_away_is_told_gone() {
        let hub = Arc::new(Hub::new(1));
        hub.seed(&serde_json::to_string(&collection()).expect("the collection, written"));
        hub.answering();
        let mut client = Client::to(&hub, r#"{"id":"all","ask":"watch","kind":"session"}"#);
        client.next();
        hub.rewritten("", "");
        let told = client.next();
        assert_eq!(
            told["gone"],
            json!(["session:a", "session:b"]),
            "a collection removed should tell what it held gone: {told}"
        );
    }

    fn budget(name: &str, standing: &str) -> crate::standing::Budget {
        crate::standing::Budget {
            name: name.into(),
            label: name.into(),
            kind: None,
            whose: None,
            default: false,
            undeclared: false,
            providers: Vec::new(),
            spent: BTreeMap::new(),
            value: None,
            allowances: Vec::new(),
            standing: standing.into(),
            nearest: None,
            health: None,
            words: String::new(),
            said: crate::standing::BudgetSaid::default(),
        }
    }

    fn standing(at: &str, mine: &str) -> Standing {
        Standing {
            v: crate::standing::VERSION,
            at: at.into(),
            default: "me".into(),
            total: crate::standing::Total {
                periods: BTreeMap::new(),
                words: String::new(),
            },
            budgets: vec![budget("me", mine), budget("team", "$1 of $50")],
            sessions: BTreeMap::from([("a".to_owned(), "me".to_owned())]),
            session: None,
        }
    }

    #[test]
    fn the_budgets_are_told_as_records_and_then_only_the_one_that_moved() {
        let hub = Arc::new(Hub::new(0));
        let written = serde_json::to_string(&standing("then", "$1 of $40")).expect("budgets");
        hub.seed(&serde_json::to_string(&collection()).expect("a collection as text"));
        hub.seed_budgets(&written);
        let mut client = Client::to(
            &hub,
            r#"{"id":"all","ask":"watch","kind":["budget","drew"]}"#,
        );
        let whole = client.next();
        assert_eq!(
            keys(&whole, "records"),
            ["budget:me", "budget:team", "drew:a"],
            "before the first pass a watch should be told the budgets written last"
        );
        assert_eq!(
            whole["records"][2]["budget"], "me",
            "a session's record should name the budget it drew on"
        );
        hub.publish(&collection(), Some(&standing("now", "$2 of $40")));
        assert_eq!(
            client.next()["id"],
            "hello",
            "the first pass says it's answering"
        );
        let change = client.next();
        assert_eq!(
            keys(&change, "changed"),
            ["budget:me"],
            "only the budget that moved should be told, not its time: {change}"
        );
        assert_eq!(
            change["changed"][0]["budget"]["standing"], "$2 of $40",
            "the budget should be told whole as it stands now"
        );
        hub.publish(&collection(), Some(&standing("later", "$2 of $40")));
        assert!(
            client.silent(),
            "a pass that moved only the time should tell nothing"
        );
    }

    #[test]
    fn a_kind_it_isnt_told_as_is_refused() {
        assert_eq!(
            ask(r#"{"id":2,"ask":"watch","kind":"person"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "person".into()
            },
            "a kind spend-watch doesn't tell should be refused"
        );
    }
}
