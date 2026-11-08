//! What the daemon knows and answers, over its socket in version 1 of the
//! shell's daemons' interface (dictation's daemon.rs): hello, know,
//! search, health, and watch and unwatch, which hold nothing, since it
//! keeps no record a client follows.
//!
//! `know` tells it the launcher's things, `{"kind": "thing", "things":
//! [{"key", "text"}]}`, in place of those told before; each text not yet
//! embedded waits for the worker (main.rs). `search` asks which things
//! mean what's typed, `{"kind": "thing", "words", "limit"}`, and is
//! answered once the words are embedded, a moment later, on the same
//! connection by its id: `{"id", "records": [{"key", "kind", "near"}],
//! "whole"}`, the nearest first and none below the floor, `whole` false
//! while some things still wait to be embedded, so the shell keeps only a
//! whole answer.

use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::Sender;

use dictation::daemon::{Answers, VERSION, error, now};
use serde_json::{Value, json};

use crate::store::Store;
use crate::text::near;

/// The one kind of record this daemon answers about yet.
pub const KIND: &str = "thing";

/// How near a thing has to mean what's typed to be answered, as bge's
/// cosine: below it, a thing only shares the words' kind of language.
/// Measured over the launcher's 513 things on the NPU: words meaning one
/// of them ("copy history", Clipboard) found it at 0.54 or nearer, while
/// words meaning none ("a song about the sea") came no nearer than 0.50
/// to anything, unless they shared a word with it, which typing finds.
pub const FLOOR: f32 = 0.53;

/// How many a search answers when it doesn't say.
const SEARCHED: usize = 8;

/// One of the launcher's things, by its key, and the text meant by it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thing {
    pub key: String,
    pub text: String,
}

/// A search waiting for its words to be embedded.
pub struct Query {
    pub id: Value,
    pub words: String,
    pub limit: usize,
    pub told: Sender<String>,
}

/// What the worker does next.
pub enum Job {
    Query(Query),
    Embed(String),
}

/// Everything the threads share.
pub struct Known {
    things: Vec<Thing>,
    pub store: Store,
    queries: VecDeque<Query>,
    /// Wakes the worker when there's something new to do.
    wake: Sender<()>,
    since: u64,
    /// How the last query went, for health: how long its words took to
    /// embed, in milliseconds, and the device.
    pub last_ms: Option<u128>,
    pub device: String,
    /// Whether embedding waits on something recording, for health.
    pub held_back: bool,
}

impl Known {
    pub fn new(store: Store, wake: Sender<()>) -> Self {
        Self {
            things: Vec::new(),
            store,
            queries: VecDeque::new(),
            wake,
            since: now(),
            last_ms: None,
            device: String::new(),
            held_back: false,
        }
    }

    /// The next job: a query before anything else, since someone waits on
    /// it, then a thing's text not yet embedded.
    pub fn next(&mut self) -> Option<Job> {
        if let Some(query) = self.queries.pop_front() {
            return Some(Job::Query(query));
        }
        self.waiting()
            .next()
            .map(|text| Job::Embed(text.to_owned()))
    }

    fn waiting(&self) -> impl Iterator<Item = &str> {
        let mut seen = HashSet::new();
        self.things
            .iter()
            .map(|thing| thing.text.as_str())
            .filter(move |text| !self.store.holds(text) && seen.insert(*text))
    }

    /// The things nearest what a query means, the nearest first, none
    /// below the floor, at most `limit`.
    pub fn nearest(&self, meaning: &[f32], limit: usize) -> Vec<(String, f32)> {
        let mut found: Vec<(String, f32)> = self
            .things
            .iter()
            .filter_map(|thing| {
                let near = near(meaning, self.store.get(&thing.text)?);
                (near >= FLOOR).then(|| (thing.key.clone(), near))
            })
            .collect();
        found.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        found.truncate(limit);
        found
    }

    /// Answers a query with what it means.
    pub fn answer_query(&self, query: &Query, meaning: &[f32]) {
        let records: Vec<Value> = self
            .nearest(meaning, query.limit)
            .into_iter()
            .map(|(key, near)| json!({"key": key, "kind": KIND, "near": rounded(near)}))
            .collect();
        let whole = self.waiting().next().is_none();
        let line = json!({"id": query.id, "records": records, "whole": whole});
        let _ = query.told.send(line.to_string());
    }

    /// Takes the things told, in place of those before, and lets go of
    /// the meanings of texts no longer among them.
    fn know(&mut self, things: Vec<Thing>) {
        self.things = things;
        let texts: Vec<String> = self.things.iter().map(|t| t.text.clone()).collect();
        self.store.keep_only(texts.iter().map(String::as_str));
        let _ = self.wake.send(());
    }
}

/// A nearness to three places, as the shell needs no more.
fn rounded(near: f32) -> f64 {
    (f64::from(near) * 1000.0).round() / 1000.0
}

/// The things a `know` tells, each with a key and a text that isn't
/// empty.
fn things(said: &Value) -> Vec<Thing> {
    said["things"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|thing| {
            let key = thing["key"].as_str()?;
            let text = thing["text"].as_str()?.trim();
            (!key.is_empty() && !text.is_empty()).then(|| Thing {
                key: key.to_owned(),
                text: text.to_owned(),
            })
        })
        .collect()
}

impl Answers for Known {
    fn answer(&mut self, line: &str, _connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "not a JSON line");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said["ask"].as_str().unwrap_or_default();
        let kind = said["kind"].as_str().unwrap_or(KIND);
        if matches!(asked, "know" | "search") && kind != KIND {
            let why = format!("this daemon knows only {KIND}, not {kind}");
            return error(&id, "unknown-kind", &why);
        }
        match asked {
            "hello" => json!({
                "id": id, "version": VERSION, "domain": "meaning", "state": "answering",
                "since": self.since,
            }),
            "know" => {
                self.know(things(&said));
                let waiting = self.waiting().count();
                json!({"id": id, "known": self.things.len(), "waiting": waiting})
            }
            "search" => {
                let words = said["words"].as_str().unwrap_or_default().trim();
                if words.is_empty() {
                    return json!({"id": id, "records": [], "whole": true});
                }
                let limit = said["limit"]
                    .as_u64()
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(SEARCHED);
                self.queries.push_back(Query {
                    id,
                    words: words.to_owned(),
                    limit,
                    told: told.clone(),
                });
                let _ = self.wake.send(());
                // Answered by the worker once the words are embedded.
                Value::Null
            }
            // Daemon.qml watches every daemon's kind as it connects;
            // nothing here changes by itself, so a watch holds nothing.
            "watch" => json!({"id": id, "records": []}),
            "unwatch" => json!({"id": id, "unwatched": true}),
            "health" => json!({
                "id": id, "version": VERSION, "state": "answering",
                "known": self.things.len(), "embedded": self.store.len(),
                "waiting": self.waiting().count(), "held back": self.held_back,
                "device": self.device, "last query ms": self.last_ms,
            }),
            other => error(&id, "unknown-ask", &format!("{other:?} isn't asked here")),
        }
    }

    fn gone(&mut self, _connection: u64) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{WIDTH, unit};
    use std::sync::mpsc;

    fn pointing(at: usize) -> Vec<f32> {
        let mut values = vec![0.0; WIDTH];
        values[at] = 1.0;
        values
    }

    fn known() -> (Known, mpsc::Receiver<()>, tempfile::TempDir) {
        let folder = tempfile::tempdir().unwrap();
        let store = Store::read(&folder.path().join("meanings"), "test");
        let (wake, woken) = mpsc::channel();
        (Known::new(store, wake), woken, folder)
    }

    fn ask(known: &mut Known, said: &Value) -> (Value, mpsc::Receiver<String>) {
        let (told, heard) = mpsc::channel();
        (known.answer(&said.to_string(), 1, &told), heard)
    }

    #[test]
    fn things_told_wait_to_be_embedded_each_text_once() {
        let (mut known, woken, _folder) = known();
        let said = json!({"id": 1, "ask": "know", "things": [
            {"key": "a", "text": "Brightness"}, {"key": "b", "text": "Brightness"},
            {"key": "c", "text": "  "}, {"key": "d", "text": "Wi-Fi"},
        ]});
        let (answer, _) = ask(&mut known, &said);
        assert_eq!(answer, json!({"id": 1, "known": 3, "waiting": 2}));
        assert!(woken.try_recv().is_ok());
        assert!(matches!(known.next(), Some(Job::Embed(text)) if text == "Brightness"));
    }

    #[test]
    fn a_query_goes_before_any_thing_waiting() {
        let (mut known, _woken, _folder) = known();
        ask(
            &mut known,
            &json!({"ask": "know", "things": [{"key": "a", "text": "Wi-Fi"}]}),
        );
        let (answer, _) = ask(
            &mut known,
            &json!({"id": 2, "ask": "search", "words": "dim"}),
        );
        assert!(answer.is_null());
        assert!(matches!(known.next(), Some(Job::Query(query)) if query.words == "dim"));
        assert!(matches!(known.next(), Some(Job::Embed(_))));
    }

    #[test]
    fn a_search_answers_the_nearest_above_the_floor_saying_whether_whole() {
        let (mut known, _woken, _folder) = known();
        ask(
            &mut known,
            &json!({"ask": "know", "things": [
                {"key": "near", "text": "Brightness"}, {"key": "far", "text": "Wi-Fi"},
                {"key": "later", "text": "Not yet"},
            ]}),
        );
        known.store.put(
            "Brightness",
            unit(&{
                let mut v = pointing(0);
                v[1] = 0.3;
                v
            }),
        );
        known.store.put("Wi-Fi", pointing(1));
        let (_, heard) = ask(
            &mut known,
            &json!({"id": 3, "ask": "search", "words": "dim"}),
        );
        let Some(Job::Query(query)) = known.next() else {
            panic!("no query waited");
        };
        known.answer_query(&query, &pointing(0));
        let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(line["whole"], json!(false));
        assert_eq!(line["records"].as_array().unwrap().len(), 1);
        assert_eq!(line["records"][0]["key"], json!("near"));
        assert_eq!(line["records"][0]["kind"], json!("thing"));
    }

    #[test]
    fn another_kind_is_refused_by_name() {
        let (mut known, _woken, _folder) = known();
        let (answer, _) = ask(
            &mut known,
            &json!({"id": 4, "ask": "search", "kind": "mail"}),
        );
        assert_eq!(answer["error"]["code"], json!("unknown-kind"));
    }
}
