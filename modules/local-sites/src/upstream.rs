//! One watch on a daemon, however many pages read it. The first page to
//! watch a feed's kind opens a watch on the daemon's socket, asking the
//! fields and depth the feed declares; every later page, in any tab, is
//! told what that watch holds and then each change it hears, so a second
//! tab costs the daemon nothing (the node's whole `session` watch at ten
//! thousand cells is about 19 MB). The watch follows the daemon as
//! daemon-serve's `watch` does: when the daemon goes, as on a switch, each
//! page is told `broken` with why, the watch opens again after a pause,
//! and the records the daemon then sends whole stand for all it holds.
//!
//! A page is told three things, each a JSON line carrying its watch's id:
//! the daemon's own `records`, `changed` and `gone`, trimmed to what the
//! page asked; and `state`, `connecting`, `live` or `broken` (with
//! `reason` and `since`, milliseconds since the epoch), which the daemon
//! interface has no word for because a daemon's own client is never told
//! of a daemon that isn't there.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use daemon_serve::watch::{Held, Watch};
use serde_json::{Map, Value, json};

use crate::config::Kind;

/// How long to wait before watching a daemon again once it has gone, as
/// daemon-serve's `watch` waits.
const AGAIN: Duration = Duration::from_secs(2);

/// How a daemon's watch stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Connecting,
    Live,
    Broken { reason: String, since: u128 },
}

impl State {
    /// The line a page is told it in, under its watch's `id`.
    fn told(&self, id: &Value) -> Value {
        match self {
            Self::Connecting => json!({"id": id, "state": "connecting"}),
            Self::Live => json!({"id": id, "state": "live"}),
            Self::Broken { reason, since } => {
                json!({"id": id, "state": "broken", "reason": reason, "since": since})
            }
        }
    }
}

/// One page's watch: where its lines go, under which id, narrowed to
/// which keys and fields.
#[derive(Debug, Clone)]
pub struct Reader {
    pub id: Value,
    pub keys: Option<BTreeSet<String>>,
    pub fields: Option<Vec<String>>,
    pub to: Sender<Value>,
}

impl Reader {
    /// The answer `answer` as this reader is told it, or none where
    /// nothing in it is the reader's.
    fn told(&self, answer: &Value) -> Option<Value> {
        let mut told = Map::new();
        for part in ["records", "changed"] {
            if let Some(records) = answer.get(part).and_then(Value::as_array) {
                let records: Vec<Value> = records
                    .iter()
                    .filter(|record| self.reads(record.get("key").and_then(Value::as_str)))
                    .map(|record| daemon_serve::asks::trimmed(record, self.fields.as_deref()))
                    .filter(|record| {
                        part == "records" || record.as_object().is_some_and(|r| r.len() > 1)
                    })
                    .collect();
                // Records whole stand for every record held, so they're
                // told even when none are this reader's.
                if part == "records" || !records.is_empty() {
                    told.insert(part.to_owned(), Value::Array(records));
                }
            }
        }
        if let Some(gone) = answer.get("gone").and_then(Value::as_array) {
            let gone: Vec<Value> = gone
                .iter()
                .filter(|key| self.reads(key.as_str()))
                .cloned()
                .collect();
            if !gone.is_empty() {
                told.insert("gone".to_owned(), Value::Array(gone));
            }
        }
        if told.is_empty() {
            return None;
        }
        told.insert("id".to_owned(), self.id.clone());
        Some(Value::Object(told))
    }

    fn reads(&self, key: Option<&str>) -> bool {
        match (&self.keys, key) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(keys), Some(key)) => keys.contains(key),
        }
    }

    fn tell(&self, line: Value) -> bool {
        self.to.send(line).is_ok()
    }
}

/// What one daemon watch holds, and who reads it.
#[derive(Debug)]
struct Followed {
    held: Held,
    state: State,
    readers: BTreeMap<u64, Reader>,
}

/// A daemon watch's name: its feed and its kind.
type Named = (String, String);

/// Every daemon watch the server holds, by feed and kind.
#[derive(Debug, Default)]
pub struct Upstreams {
    followed: Mutex<BTreeMap<Named, Arc<Mutex<Followed>>>>,
    next: Mutex<u64>,
}

impl Upstreams {
    /// Adds `reader` to the watch of `kind` on the feed `feed` at
    /// `socket`, opening that watch where none is open, and tells the
    /// reader what it holds now; answers the number that takes it off.
    pub fn watch(
        self: &Arc<Self>,
        feed: &str,
        socket: &Path,
        kind: (&str, &Kind),
        reader: Reader,
    ) -> u64 {
        let number = {
            let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
            *next += 1;
            *next
        };
        let name = (feed.to_owned(), kind.0.to_owned());
        let mut all = self.followed.lock().unwrap_or_else(PoisonError::into_inner);
        let followed = all.entry(name.clone()).or_insert_with(|| {
            let followed = Arc::new(Mutex::new(Followed {
                held: Held::default(),
                state: State::Connecting,
                readers: BTreeMap::new(),
            }));
            follow(
                Arc::clone(self),
                name,
                socket.to_path_buf(),
                asked(kind.0, kind.1),
                Arc::clone(&followed),
            );
            followed
        });
        let mut followed = followed.lock().unwrap_or_else(PoisonError::into_inner);
        let records: Vec<Value> = followed
            .held
            .records()
            .map(|(_, record)| record.clone())
            .collect();
        if followed.state != State::Connecting
            && let Some(now) = reader.told(&json!({"records": records}))
        {
            reader.tell(now);
        }
        reader.tell(followed.state.told(&reader.id));
        followed.readers.insert(number, reader);
        number
    }

    /// Takes the reader `number` off the watch of `kind` on `feed`.
    pub fn unwatch(&self, feed: &str, kind: &str, number: u64) {
        let all = self.followed.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(followed) = all.get(&(feed.to_owned(), kind.to_owned())) {
            followed
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .readers
                .remove(&number);
        }
    }

    /// Drops the watch `name` once nobody reads it, answering whether it
    /// did, so its thread ends rather than holding a socket for nobody.
    fn let_go(&self, name: &(String, String), followed: &Mutex<Followed>) -> bool {
        let mut all = self.followed.lock().unwrap_or_else(PoisonError::into_inner);
        let empty = followed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .readers
            .is_empty();
        if empty {
            all.remove(name);
        }
        empty
    }
}

/// What the server asks the daemon for a kind: its declared fields and
/// depth, so every page shares one watch.
fn asked(kind: &str, declared: &Kind) -> Value {
    let mut asked = json!({"kind": kind});
    if let Some(fields) = &declared.fields {
        asked["fields"] = json!(fields);
    }
    if let Some(deep) = &declared.deep {
        asked["deep"] = json!(deep);
    }
    asked
}

fn follow(
    upstreams: Arc<Upstreams>,
    name: (String, String),
    socket: PathBuf,
    asked: Value,
    followed: Arc<Mutex<Followed>>,
) {
    thread::spawn(move || {
        loop {
            let reason = match Watch::asking(&socket, &asked) {
                Err(why) => format!("{} doesn't answer: {why}", socket.display()),
                Ok(mut watch) => loop {
                    match watch.answer() {
                        Ok(answer) => {
                            let mut now = followed.lock().unwrap_or_else(PoisonError::into_inner);
                            now.held.took(&answer);
                            if now.state != State::Live {
                                now.state = State::Live;
                                let state = State::Live;
                                now.readers
                                    .retain(|_, reader| reader.tell(state.told(&reader.id)));
                            }
                            now.readers.retain(|_, reader| {
                                reader.told(&answer).is_none_or(|line| reader.tell(line))
                            });
                        }
                        Err(why) => break why.to_string(),
                    }
                    if upstreams.let_go(&name, &followed) {
                        return;
                    }
                },
            };
            {
                let mut now = followed.lock().unwrap_or_else(PoisonError::into_inner);
                let broken = State::Broken {
                    reason,
                    since: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |since| since.as_millis()),
                };
                if !matches!(now.state, State::Broken { .. }) {
                    now.state = broken;
                    let state = now.state.clone();
                    now.readers
                        .retain(|_, reader| reader.tell(state.told(&reader.id)));
                }
            }
            if upstreams.let_go(&name, &followed) {
                return;
            }
            thread::sleep(AGAIN);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn reader(keys: Option<&[&str]>, fields: Option<&[&str]>) -> (Reader, mpsc::Receiver<Value>) {
        let (to, heard) = mpsc::channel();
        let reader = Reader {
            id: json!(7),
            keys: keys.map(|keys| keys.iter().map(|key| (*key).to_owned()).collect()),
            fields: fields.map(|fields| fields.iter().map(|field| (*field).to_owned()).collect()),
            to,
        };
        (reader, heard)
    }

    #[test]
    fn a_reader_is_told_an_answer_under_its_own_id() {
        let (reader, _) = reader(None, None);
        let told = reader.told(&json!({"id": 2, "changed": [{"key": "session:a", "step": null}]}));
        assert_eq!(
            told,
            Some(json!({"id": 7, "changed": [{"key": "session:a", "step": null}]})),
            "a dropped field stays null, so the page drops it too"
        );
    }

    #[test]
    fn a_reader_narrowed_to_keys_hears_only_those() {
        let (reader, _) = reader(Some(&["session:a"]), None);
        let answer =
            json!({"changed": [{"key": "session:b", "title": "B"}], "gone": ["session:b"]});
        assert_eq!(reader.told(&answer), None, "nothing of its");
        let whole = json!({"records": [{"key": "session:a"}, {"key": "session:b"}]});
        assert_eq!(
            reader.told(&whole),
            Some(json!({"id": 7, "records": [{"key": "session:a"}]}))
        );
    }

    #[test]
    fn a_reader_narrowed_to_fields_hears_no_change_without_one() {
        let (reader, _) = reader(None, Some(&["title"]));
        let answer = json!({"changed": [{"key": "session:a", "step": "edit"}]});
        assert_eq!(reader.told(&answer), None);
        let empty = json!({"records": []});
        assert_eq!(
            reader.told(&empty),
            Some(json!({"id": 7, "records": []})),
            "records whole, even none, stand for all"
        );
    }

    #[test]
    fn a_broken_state_says_why_and_since_when() {
        let broken = State::Broken {
            reason: "gone".to_owned(),
            since: 5,
        };
        assert_eq!(
            broken.told(&json!(3)),
            json!({"id": 3, "state": "broken", "reason": "gone", "since": 5})
        );
    }
}
