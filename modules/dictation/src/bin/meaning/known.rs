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
//!
//! Where it looks at pictures, `search` of `{"kind": "picture"}` asks
//! which pictures in the folders it was given look like what's typed,
//! answered alike with each picture's path as its key. Pictures aren't
//! told: they're those in the folders, heard coming and going
//! (dictation's watch.rs), each looked at once, newest first, and kept
//! by its path, when it was written and its size, so one written over is
//! looked at again.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::UNIX_EPOCH;

use dictation::daemon::{Answers, VERSION, error, now};
use dictation::picture::readable;
use serde_json::{Value, json};

use crate::sight;
use crate::store::Store;
use crate::text::near;

/// The kind of record a launcher's thing is.
pub const KIND: &str = "thing";

/// The kind of record a picture is.
pub const PICTURE: &str = "picture";

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

/// What a search looks among.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Among {
    Things,
    Pictures,
}

/// A search waiting for its words to be embedded.
pub struct Query {
    pub id: Value,
    pub words: String,
    pub limit: usize,
    pub among: Among,
    pub told: Sender<String>,
    /// The connection that asked, so a newer search from it, or its
    /// closing, lets this one go.
    pub connection: u64,
}

/// A picture in a folder looked through: where, when it was written, and
/// what its look is kept under.
struct Shown {
    path: PathBuf,
    taken: u64,
    key: String,
}

impl Shown {
    /// The picture at `path` as it is now, if it's there and one this
    /// reads.
    fn now(path: &Path) -> Option<Self> {
        if !readable(path) {
            return None;
        }
        let data = std::fs::metadata(path).ok()?;
        let written = data.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        let taken = u64::try_from(written.as_millis()).ok()?;
        let key = format!("{}\t{taken}\t{}", path.display(), data.len());
        Some(Self {
            path: path.to_owned(),
            taken,
            key,
        })
    }
}

/// What the worker does next.
pub enum Job {
    Query(Query),
    Embed(String),
    /// A picture to look at, by its path and the key its look is kept by.
    Look(PathBuf, String),
}

/// Everything the threads share.
pub struct Known {
    things: Vec<Thing>,
    pub store: Store,
    /// The pictures in the folders looked through, the newest first.
    pictures: Vec<Shown>,
    /// What each picture looks like, by its key, where pictures are
    /// looked at.
    pub looks: Option<Store>,
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
    pub fn new(store: Store, looks: Option<Store>, wake: Sender<()>) -> Self {
        Self {
            things: Vec::new(),
            store,
            pictures: Vec::new(),
            looks,
            queries: VecDeque::new(),
            wake,
            since: now(),
            last_ms: None,
            device: String::new(),
            held_back: false,
        }
    }

    /// The next job: a query before anything else, since someone waits on
    /// it, then a thing's text not yet embedded, then a picture not yet
    /// looked at, the newest first.
    pub fn next(&mut self) -> Option<Job> {
        if let Some(query) = self.queries.pop_front() {
            return Some(Job::Query(query));
        }
        if let Some(text) = self.waiting().next() {
            return Some(Job::Embed(text.to_owned()));
        }
        self.unlooked()
            .next()
            .map(|shown| Job::Look(shown.path.clone(), shown.key.clone()))
    }

    fn unlooked(&self) -> impl Iterator<Item = &Shown> {
        self.pictures.iter().filter(|shown| {
            self.looks
                .as_ref()
                .is_some_and(|looks| !looks.holds(&shown.key))
        })
    }

    /// How many pictures wait to be looked at, for health and the log.
    pub fn pictures_waiting(&self) -> usize {
        self.unlooked().count()
    }

    /// Takes a picture written or moved into a folder looked through, in
    /// place of one at the same path.
    pub fn came(&mut self, path: &Path) {
        self.pictures.retain(|shown| shown.path != path);
        if let Some(shown) = Shown::now(path) {
            let at = self
                .pictures
                .partition_point(|before| before.taken >= shown.taken);
            self.pictures.insert(at, shown);
            let _ = self.wake.send(());
        }
        self.keep_looked();
    }

    /// Lets go of a picture removed or moved out.
    pub fn went(&mut self, path: &Path) {
        self.pictures.retain(|shown| shown.path != path);
        self.keep_looked();
    }

    /// Takes every picture in `folders` as it is now, in place of those
    /// before. A folder that isn't there holds none.
    pub fn look_through(&mut self, folders: &[PathBuf]) {
        self.pictures = folders
            .iter()
            .filter_map(|folder| std::fs::read_dir(folder).ok())
            .flatten()
            .filter_map(|entry| Shown::now(&entry.ok()?.path()))
            .collect();
        self.pictures
            .sort_by(|a, b| b.taken.cmp(&a.taken).then_with(|| a.path.cmp(&b.path)));
        self.keep_looked();
        let _ = self.wake.send(());
    }

    /// Keeps only the looks of pictures still there.
    fn keep_looked(&mut self) {
        if let Some(looks) = &mut self.looks {
            looks.keep_only(self.pictures.iter().map(|shown| shown.key.as_str()));
        }
    }

    /// The pictures nearest what words say, the nearest first, none below
    /// SigLIP's floor, at most `limit`, each by its path.
    pub fn nearest_pictures(&self, meaning: &[f32], limit: usize) -> Vec<(String, f32)> {
        let Some(looks) = &self.looks else {
            return Vec::new();
        };
        let mut found: Vec<(String, f32)> = self
            .pictures
            .iter()
            .filter_map(|shown| {
                let near = near(meaning, looks.get(&shown.key)?);
                (near >= sight::FLOOR).then(|| (shown.path.display().to_string(), near))
            })
            .collect();
        found.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        found.truncate(limit);
        found
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

    /// Lets go of the searches `connection` still waits on among the same,
    /// as a newer one from it comes, since it typed on: each is answered
    /// with nothing and as not whole, so the shell asks it again a moment
    /// later should it be typed back.
    fn passed_over(&mut self, connection: u64, among: Among) {
        let (over, kept): (VecDeque<Query>, VecDeque<Query>) = self
            .queries
            .drain(..)
            .partition(|query| query.connection == connection && query.among == among);
        self.queries = kept;
        for query in over {
            let line = json!({"id": query.id, "records": [], "whole": false});
            let _ = query.told.send(line.to_string());
        }
    }

    /// Answers a query with what it means.
    pub fn answer_query(&self, query: &Query, meaning: &[f32]) {
        let (found, kind, whole) = match query.among {
            Among::Things => (
                self.nearest(meaning, query.limit),
                KIND,
                self.waiting().next().is_none(),
            ),
            Among::Pictures => (
                self.nearest_pictures(meaning, query.limit),
                PICTURE,
                self.unlooked().next().is_none(),
            ),
        };
        let records: Vec<Value> = found
            .into_iter()
            .map(|(key, near)| json!({"key": key, "kind": kind, "near": rounded(near)}))
            .collect();
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
    fn answer(&mut self, line: &str, connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "not a JSON line");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said["ask"].as_str().unwrap_or_default();
        let kind = said["kind"].as_str().unwrap_or(KIND);
        let looking = self.looks.is_some();
        let among = match (kind, asked) {
            (KIND, _) => Among::Things,
            (PICTURE, "search") if looking => Among::Pictures,
            _ if matches!(asked, "know" | "search") => {
                let done = if asked == "know" { "told" } else { "searched" };
                let why = format!("{kind} isn't {done} here");
                return error(&id, "unknown-kind", &why);
            }
            _ => Among::Things,
        };
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
                self.passed_over(connection, among);
                self.queries.push_back(Query {
                    id,
                    words: words.to_owned(),
                    limit,
                    among,
                    told: told.clone(),
                    connection,
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
                "pictures": self.pictures.len(),
                "looked at": self.looks.as_ref().map_or(0, Store::len),
                "pictures waiting": self.pictures_waiting(),
            }),
            other => error(&id, "unknown-ask", &format!("{other:?} isn't asked here")),
        }
    }

    // A closed connection's searches go unanswered, since nothing would
    // hear them.
    fn gone(&mut self, connection: u64) {
        self.queries.retain(|query| query.connection != connection);
    }
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
        let looks = Store::read(&folder.path().join("looks"), "sight");
        let (wake, woken) = mpsc::channel();
        (Known::new(store, Some(looks), wake), woken, folder)
    }

    /// A picture's file written into `folder`, a second apart from the
    /// one written before it, since the newest are looked at first.
    fn shot(folder: &Path, name: &str, second: u64) -> PathBuf {
        let path = folder.join(name);
        std::fs::write(&path, name).unwrap();
        let when = UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000 + second);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(when)
            .unwrap();
        path
    }

    fn looked_at(known: &mut Known) -> Vec<PathBuf> {
        let mut looked = Vec::new();
        while let Some(Job::Look(path, key)) = known.next() {
            known
                .looks
                .as_mut()
                .unwrap()
                .put(&key, pointing(looked.len()));
            looked.push(path);
        }
        looked
    }

    #[test]
    fn pictures_are_looked_at_once_each_the_newest_first_and_again_once_written_over() {
        let (mut known, _woken, folder) = known();
        let shots = folder.path().join("Screenshots");
        std::fs::create_dir(&shots).unwrap();
        let old = shot(&shots, "old.png", 1);
        let new = shot(&shots, "new.jpg", 2);
        shot(&shots, "a recording.mp4", 3);
        known.look_through(&[shots.clone(), folder.path().join("Camera")]);
        assert_eq!(looked_at(&mut known), [new.clone(), old.clone()]);
        assert!(known.next().is_none());
        shot(&shots, "old.png", 4);
        known.came(&old);
        assert_eq!(looked_at(&mut known), [old.clone()]);
        known.went(&new);
        assert_eq!(known.looks.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn a_picture_search_answers_paths_above_sights_floor() {
        let (mut known, _woken, folder) = known();
        let shot_of = shot(folder.path(), "chart.png", 1);
        shot(folder.path(), "cat.png", 2);
        known.look_through(&[folder.path().to_owned()]);
        // The cat, the newest, is looked at first, pointing one way; the
        // chart the other.
        looked_at(&mut known);
        let (answer, heard) = ask(
            &mut known,
            &json!({"id": 5, "ask": "search", "kind": "picture", "words": "a chart"}),
        );
        assert!(answer.is_null());
        let Some(Job::Query(query)) = known.next() else {
            panic!("no query waited");
        };
        assert_eq!(query.among, Among::Pictures);
        let mut asked = pointing(1);
        asked[0] = 0.05;
        known.answer_query(&query, &unit(&asked));
        let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(line["whole"], json!(true));
        assert_eq!(line["records"].as_array().unwrap().len(), 1);
        assert_eq!(
            line["records"][0]["key"],
            json!(shot_of.display().to_string())
        );
        assert_eq!(line["records"][0]["kind"], json!("picture"));
    }

    #[test]
    fn pictures_are_refused_where_none_are_looked_at() {
        let folder = tempfile::tempdir().unwrap();
        let store = Store::read(&folder.path().join("meanings"), "test");
        let (wake, _woken) = mpsc::channel();
        let mut known = Known::new(store, None, wake);
        let (answer, _) = ask(
            &mut known,
            &json!({"id": 6, "ask": "search", "kind": "picture", "words": "a cat"}),
        );
        assert_eq!(answer["error"]["code"], json!("unknown-kind"));
    }

    fn ask(known: &mut Known, said: &Value) -> (Value, mpsc::Receiver<String>) {
        ask_on(known, said, 1)
    }

    fn ask_on(known: &mut Known, said: &Value, connection: u64) -> (Value, mpsc::Receiver<String>) {
        let (told, heard) = mpsc::channel();
        (known.answer(&said.to_string(), connection, &told), heard)
    }

    fn searched(words: &str) -> Value {
        json!({"id": words, "ask": "search", "words": words})
    }

    #[test]
    fn a_newer_search_lets_its_connections_older_one_go_answered_as_not_whole() {
        let (mut known, _woken, _folder) = known();
        let (_, older) = ask_on(&mut known, &searched("too"), 1);
        let (_, beside) = ask_on(&mut known, &searched("dim"), 2);
        ask_on(&mut known, &searched("too bright"), 1);
        let line: Value = serde_json::from_str(&older.recv().unwrap()).unwrap();
        assert_eq!(line, json!({"id": "too", "records": [], "whole": false}));
        let waiting: Vec<String> = known.queries.iter().map(|q| q.words.clone()).collect();
        assert_eq!(waiting, ["dim", "too bright"]);
        assert!(beside.try_recv().is_err());
    }

    #[test]
    fn a_closed_connections_searches_go_unanswered() {
        let (mut known, _woken, _folder) = known();
        ask_on(&mut known, &searched("too"), 1);
        ask_on(&mut known, &searched("dim"), 2);
        known.gone(1);
        let waiting: Vec<String> = known.queries.iter().map(|q| q.words.clone()).collect();
        assert_eq!(waiting, ["dim"]);
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
