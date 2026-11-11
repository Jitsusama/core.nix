//! Wi-Fi's rows, served ready to the launcher over version 1 of the shell's
//! daemon interface (RSCH-20261029-0E8050), on the second socket systemd
//! holds for wifid, /run/wifid/rows: JSON lines, each ask with an `id` its
//! answers carry, a watch answered whole once and then only with the rows
//! that changed and the keys gone. The shell reaches it as Daemon.qml does
//! every daemon, so it starts nothing and parses only the rows that moved.
//!
//! ```sh
//! echo '{"id":1,"ask":"list","kind":"row","place":"wifi","words":"'"$LAUNCHER_WORDS"'"}' \
//!   | socat - UNIX-CONNECT:/run/wifid/rows
//! ```
//!
//! A watch names the launcher's vocabulary by its path in the Nix store,
//! which every account's shell knows and anyone may read, so each account
//! sees its own words. wifid runs as root, so it reads no path outside the
//! store, and reads each once: the rows made in one vocabulary are made
//! once a change, whoever watches them.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::rows::{self, Orders, Words};
use crate::state::State;

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The one kind of record this socket keeps.
const KIND: &str = "row";

/// Where the vocabularies wifid will read live.
const STORE: &str = "/nix/store/";

/// How many lines a connection may have waiting before it's hung up on,
/// rather than told less than it asked for.
const WAITING: usize = 64;

/// The rows, by key, as they stand in one vocabulary.
pub type Made = BTreeMap<String, Value>;

/// The rows made in one vocabulary, made again as wifid's state changes
/// for as long as anyone watches them.
struct Vocabulary {
    words: Words,
    orders: Orders,
    made: watch::Sender<Arc<Made>>,
}

impl Vocabulary {
    /// Makes the rows again from the state, telling whoever watches only
    /// when one of them changed.
    fn make(&mut self, state: &State) {
        let placed = self
            .orders
            .place(rows::rows(&state.described(), &self.words));
        let made: Made = placed
            .into_iter()
            .filter_map(|row| Some((row["key"].as_str()?.to_owned(), row)))
            .collect();
        self.made.send_if_modified(|held| {
            if **held == made {
                return false;
            }
            *held = Arc::new(made);
            true
        });
    }
}

/// What every connection to the socket shares: wifid's state, and the
/// rows made in each vocabulary asked for.
pub struct Face {
    state: watch::Receiver<State>,
    /// Where vocabularies are read from, the Nix store but in tests.
    store: PathBuf,
    vocabularies: Mutex<HashMap<PathBuf, Vocabulary>>,
}

impl Face {
    pub fn new(state: watch::Receiver<State>) -> Arc<Face> {
        Face::within(state, PathBuf::from(STORE))
    }

    fn within(state: watch::Receiver<State>, store: PathBuf) -> Arc<Face> {
        Arc::new(Face {
            state,
            store,
            vocabularies: Mutex::new(HashMap::new()),
        })
    }

    /// Answers everyone who connects, for as long as wifid runs, and makes
    /// the rows again as wifid's state changes.
    pub async fn serve(self: Arc<Face>, listener: UnixListener) -> std::io::Result<()> {
        let making = Arc::clone(&self);
        tokio::spawn(async move { making.follow().await });
        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(Arc::clone(&self).answer(stream));
        }
    }

    /// Makes each vocabulary's rows again as the state changes, only while
    /// someone watches them, so nobody watching costs nothing.
    async fn follow(&self) {
        let mut state = self.state.clone();
        while state.changed().await.is_ok() {
            let now = state.borrow_and_update().clone();
            let mut vocabularies = self.held();
            for vocabulary in vocabularies.values_mut() {
                if vocabulary.made.receiver_count() > 0 {
                    vocabulary.make(&now);
                }
            }
        }
    }

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, Vocabulary>> {
        self.vocabularies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The rows made in the vocabulary at a path, as they stand now and as
    /// they change; or why it can't be read.
    fn rows_in(&self, path: &str) -> Result<watch::Receiver<Arc<Made>>, String> {
        let path = self.vocabulary_at(path)?;
        let state = self.state.borrow().clone();
        let mut vocabularies = self.held();
        if let Some(vocabulary) = vocabularies.get_mut(&path) {
            // Nobody watched while it changed, so it's made afresh first.
            if vocabulary.made.receiver_count() == 0 {
                vocabulary.make(&state);
            }
            return Ok(vocabulary.made.subscribe());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} can't be read: {error}", path.display()))?;
        let whole: Value = serde_json::from_str(&text)
            .map_err(|error| format!("{} isn't JSON: {error}", path.display()))?;
        let mut vocabulary = Vocabulary {
            words: Words::read(&whole)?,
            orders: Orders::default(),
            made: watch::Sender::new(Arc::new(Made::new())),
        };
        vocabulary.make(&state);
        let rows = vocabulary.made.subscribe();
        vocabularies.insert(path, vocabulary);
        Ok(rows)
    }

    /// The vocabulary's path, once it's found to be in the store.
    fn vocabulary_at(&self, path: &str) -> Result<PathBuf, String> {
        let found = Path::new(path)
            .canonicalize()
            .map_err(|error| format!("{path} can't be found: {error}"))?;
        if !found.starts_with(&self.store) || !found.is_file() {
            return Err(format!("{path} isn't a file in {}", self.store.display()));
        }
        Ok(found)
    }

    /// Answers one connection until it hangs up.
    async fn answer(self: Arc<Face>, stream: UnixStream) {
        let (reading, mut writing) = stream.into_split();
        let (told, mut telling) = mpsc::channel::<String>(WAITING);
        let writer = tokio::spawn(async move {
            while let Some(mut line) = telling.recv().await {
                line.push('\n');
                if writing.write_all(line.as_bytes()).await.is_err() {
                    return;
                }
            }
        });
        let mut connection = Connection {
            face: Arc::clone(&self),
            told,
            words: None,
            watches: HashMap::new(),
        };
        let mut lines = BufReader::new(reading).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(answer) = connection.answer(&line)
                && connection.told.try_send(answer.to_string()).is_err()
            {
                break;
            }
            if connection.told.is_closed() {
                break;
            }
        }
        for (_, watch) in connection.watches.drain() {
            watch.abort();
        }
        drop(connection);
        let _ = writer.await;
    }
}

/// One client's connection: where its lines go, the vocabulary it last
/// watched in, and its watches by id.
struct Connection {
    face: Arc<Face>,
    told: mpsc::Sender<String>,
    words: Option<String>,
    watches: HashMap<String, JoinHandle<()>>,
}

impl Connection {
    /// What one line asks, answered, or nothing where a watch answers it.
    fn answer(&mut self, line: &str) -> Option<Value> {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return Some(error(&Value::Null, "unknown-ask", "that line isn't JSON"));
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
        let hello =
            || json!({"id": id, "version": VERSION, "domain": "wifi", "state": "answering"});
        match asked {
            "hello" => Some(hello()),
            "health" => {
                let mut health = hello();
                health["watches"] = json!(self.watches.len());
                health["vocabularies"] = json!(self.face.held().len());
                Some(health)
            }
            "get" | "list" | "watch" => {
                if let Some(refused) = unkept(&said) {
                    return Some(error(&id, "unknown-kind", &refused));
                }
                let words = said
                    .get("words")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| self.words.clone());
                let Some(words) = words else {
                    return Some(error(
                        &id,
                        "no-words",
                        "say the launcher's words.json as words",
                    ));
                };
                let rows = match self.face.rows_in(&words) {
                    Ok(rows) => rows,
                    Err(why) => return Some(error(&id, "no-words", &why)),
                };
                self.words = Some(words);
                let keys: Option<Vec<String>> =
                    said.get("keys").and_then(Value::as_array).map(|keys| {
                        keys.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    });
                if asked == "watch" {
                    self.watch(id, rows, keys);
                    return None;
                }
                let held = rows.borrow().clone();
                let records: Vec<&Value> = held
                    .iter()
                    .filter(|(key, _)| keys.as_ref().is_none_or(|keys| keys.contains(key)))
                    .map(|(_, row)| row)
                    .collect();
                Some(json!({"id": id, "records": records}))
            }
            "unwatch" => {
                if let Some(watch) = self.watches.remove(&id.to_string()) {
                    watch.abort();
                }
                Some(json!({"id": id, "unwatched": true}))
            }
            _ => Some(error(&id, "unknown-ask", &format!("no ask {asked:?} here"))),
        }
    }

    /// Answers a watch whole now, then with each change, until it's
    /// unwatched, watched again by its id, or the connection goes.
    fn watch(
        &mut self,
        id: Value,
        mut rows: watch::Receiver<Arc<Made>>,
        keys: Option<Vec<String>>,
    ) {
        let told = self.told.clone();
        let named = id.to_string();
        // Watched again by its id, the one before says nothing more.
        if let Some(before) = self.watches.remove(&named) {
            before.abort();
        }
        let wanted = move |key: &String| keys.as_ref().is_none_or(|keys| keys.contains(key));
        // Taken as it's seen, so a change made after it is told after it.
        let mut sent = Arc::clone(&*rows.borrow_and_update());
        let first: Vec<&Value> = sent
            .iter()
            .filter(|(key, _)| wanted(key))
            .map(|(_, row)| row)
            .collect();
        if told
            .try_send(json!({"id": id, "records": first}).to_string())
            .is_err()
        {
            return;
        }
        let watching = tokio::spawn(async move {
            while rows.changed().await.is_ok() {
                let now = Arc::clone(&*rows.borrow_and_update());
                let (changed, gone) = between(&sent, &now, &wanted);
                sent = now;
                if changed.is_empty() && gone.is_empty() {
                    continue;
                }
                let mut said = json!({"id": id, "changed": changed});
                if !gone.is_empty() {
                    said["gone"] = json!(gone);
                }
                if told.try_send(said.to_string()).is_err() {
                    return;
                }
            }
        });
        self.watches.insert(named, watching);
    }
}

/// The rows that changed between two makings, and the keys gone.
fn between(
    before: &Made,
    now: &Made,
    wanted: &impl Fn(&String) -> bool,
) -> (Vec<Value>, Vec<String>) {
    let changed = now
        .iter()
        .filter(|(key, row)| wanted(key) && before.get(*key) != Some(*row))
        .map(|(_, row)| row.clone())
        .collect();
    let gone = before
        .keys()
        .filter(|key| wanted(key) && !now.contains_key(*key))
        .cloned()
        .collect();
    (changed, gone)
}

/// Why an ask names a kind or place this socket doesn't keep, if it does.
fn unkept(said: &Value) -> Option<String> {
    let kinds: Vec<&str> = match said.get("kind") {
        None | Some(Value::Null) => vec![],
        Some(Value::String(kind)) => vec![kind.as_str()],
        Some(Value::Array(kinds)) => kinds.iter().filter_map(Value::as_str).collect(),
        Some(other) => return Some(format!("no kind {other} here")),
    };
    if let Some(kind) = kinds.iter().find(|kind| **kind != KIND) {
        return Some(format!("no kind {kind} here"));
    }
    match said.get("place").and_then(Value::as_str) {
        Some(place) if place != rows::PLACE => Some(format!("no place {place} here")),
        _ => None,
    }
}

/// An error answer, by the interface's codes.
fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
    use tokio::net::unix::OwnedReadHalf;
    use tokio::time::timeout;

    use super::*;
    use crate::link::Link;

    struct Client {
        lines: Lines<BufReader<OwnedReadHalf>>,
        writing: tokio::net::unix::OwnedWriteHalf,
    }

    impl Client {
        async fn ask(&mut self, said: Value) {
            let line = format!("{said}\n");
            self.writing
                .write_all(line.as_bytes())
                .await
                .expect("asked");
        }

        async fn heard(&mut self) -> Value {
            let line = timeout(Duration::from_secs(5), self.lines.next_line())
                .await
                .expect("an answer in time")
                .expect("a line")
                .expect("the connection still open");
            serde_json::from_str(&line).expect("JSON")
        }
    }

    /// A face over a state with one card on Lantern, and a store holding
    /// the launcher's words.
    fn face() -> (Arc<Face>, watch::Sender<State>, tempfile::TempDir, String) {
        let store = tempfile::tempdir().expect("a store");
        let words = store.path().join("words.json");
        let vocabulary = json!({"things": [
            {"id": "wifi:radio", "name": "Wi-Fi Off", "glyph": "offline",
                "states": {"off": {"name": "Wi-Fi On", "glyph": "wireless"}}},
            {"id": "wifi:sign-in", "name": "Sign in to {network}", "glyph": "signIn"},
            {"id": "wifi:network", "said": {"connected": "connected", "saved": "saved",
                "bars": "{n} of 4 bars", "open": "open", "password": "a password",
                "account": "an account"}},
        ]});
        std::fs::write(&words, vocabulary.to_string()).expect("written");
        let mut state = State::default();
        state.follow(&Link {
            index: 3,
            name: "wlan0".into(),
            ssid: Some("Lantern".into()),
            routable: true,
            dns: Vec::new(),
            portal: None,
        });
        let (sending, state) = watch::channel(state);
        let root = store.path().canonicalize().expect("the store is there");
        let words = words.to_string_lossy().into_owned();
        (Face::within(state, root), sending, store, words)
    }

    fn connect(face: &Arc<Face>) -> Client {
        let (ours, theirs) = UnixStream::pair().expect("a pair");
        tokio::spawn(Arc::clone(face).answer(theirs));
        let (reading, writing) = ours.into_split();
        Client {
            lines: BufReader::new(reading).lines(),
            writing,
        }
    }

    fn keys(said: &Value, field: &str) -> Vec<String> {
        said[field]
            .as_array()
            .expect("a list")
            .iter()
            .map(|row| row["key"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    #[tokio::test]
    async fn a_watch_is_answered_whole_then_only_with_what_changed() {
        let (face, state, _store, words) = face();
        tokio::spawn({
            let face = Arc::clone(&face);
            async move { face.follow().await }
        });
        let mut client = connect(&face);
        client
            .ask(
                json!({"id": "all", "ask": "watch", "kind": "row", "place": "wifi",
                "words": words}),
            )
            .await;
        let whole = client.heard().await;
        assert_eq!(whole["id"], "all");
        assert_eq!(
            keys(&whole, "records"),
            ["row:wifi:network:Lantern", "row:wifi:radio"]
        );
        assert_eq!(whole["records"][1]["name"], "Wi-Fi Off");
        state.send_modify(|state| state.radio(false));
        let change = client.heard().await;
        assert_eq!(keys(&change, "changed"), ["row:wifi:radio"]);
        assert_eq!(change["changed"][0]["name"], "Wi-Fi On");
        assert_eq!(change["gone"], json!(["row:wifi:network:Lantern"]));
    }

    #[tokio::test]
    async fn a_change_nothing_draws_tells_nobody() {
        let (face, state, _store, words) = face();
        tokio::spawn({
            let face = Arc::clone(&face);
            async move { face.follow().await }
        });
        let mut client = connect(&face);
        client
            .ask(json!({"id": "all", "ask": "watch", "kind": "row", "words": words}))
            .await;
        client.heard().await;
        state.send_modify(|state| state.wired(true));
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        client.ask(json!({"id": 2, "ask": "hello"})).await;
        assert_eq!(client.heard().await["id"], 2);
    }

    #[tokio::test]
    async fn a_get_uses_the_words_its_connection_watched_in() {
        let (face, _state, _store, words) = face();
        let mut client = connect(&face);
        client
            .ask(json!({"id": "all", "ask": "watch", "kind": "row", "words": words}))
            .await;
        client.heard().await;
        client
            .ask(json!({"id": 5, "ask": "get", "keys": ["row:wifi:radio"]}))
            .await;
        let got = client.heard().await;
        assert_eq!(keys(&got, "records"), ["row:wifi:radio"]);
    }

    #[tokio::test]
    async fn words_outside_the_store_are_never_read() {
        let (face, _state, _store, _words) = face();
        let mut client = connect(&face);
        client
            .ask(json!({"id": 1, "ask": "list", "kind": "row", "words": "/etc/shadow"}))
            .await;
        assert_eq!(client.heard().await["error"]["code"], "no-words");
        client
            .ask(json!({"id": 2, "ask": "list", "kind": "row"}))
            .await;
        assert_eq!(client.heard().await["error"]["code"], "no-words");
    }

    #[tokio::test]
    async fn another_kind_or_place_is_refused() {
        let (face, _state, _store, words) = face();
        let mut client = connect(&face);
        client
            .ask(json!({"id": 1, "ask": "watch", "kind": "window", "words": words}))
            .await;
        assert_eq!(client.heard().await["error"]["code"], "unknown-kind");
        client
            .ask(
                json!({"id": 2, "ask": "watch", "kind": "row", "place": "mail",
                "words": words}),
            )
            .await;
        assert_eq!(client.heard().await["error"]["code"], "unknown-kind");
    }

    #[tokio::test]
    async fn the_hello_says_the_interface_and_domain() {
        let (face, _state, _store, _words) = face();
        let mut client = connect(&face);
        client
            .ask(json!({"id": "hello", "ask": "hello", "version": 1}))
            .await;
        let hello = client.heard().await;
        assert_eq!(hello["version"], 1);
        assert_eq!(hello["domain"], "wifi");
        assert_eq!(hello["state"], "answering");
    }
}
