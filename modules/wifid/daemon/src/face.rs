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
//!
//! A watch may name the account's number settings the same way, as
//! `numbers`, a file in the store holding `{ "group", "point" }`; with
//! none, figures take the comma and the full stop.
//!
//! What a row's acts ask comes back on the same connection, as an `act`
//! ask (acts.rs), so the shell acts without starting anything.
//!
//! A watch whose `keys` name a network's row, as the shell's watch ahead
//! does while the network's selected or stepped into (Y2YBTS,
//! 2026-11-11), has wifid read each card's link and serve the network's
//! insides (parts.rs): its drawer on its row, and the rows a step in
//! shows, in the place's own stream. Once no watch names it, they're
//! told gone, and the link is read no more.
//!
//! Long because its cases drive a connection end to end, from a watch to
//! a look ahead and an act, through the private helpers they share.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::acts;
use ready_rows::kept::Kept;
use ready_rows::numbers::Numbers;

use crate::parts::{self, Look};
use crate::rows::{self, Words};
use crate::service::{Ask, Watching};
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

/// The rows' records, by key, as they stand in one vocabulary, each
/// `{ key, kind: "row", row }` as every daemon serves them (ready-rows).
pub type Made = BTreeMap<String, Value>;

/// Where a vocabulary's words and number settings are read from: the
/// rows made in one pair are made once, whoever watches them.
type Said = (PathBuf, Option<PathBuf>);

/// The rows made in one vocabulary, made again as wifid's state changes
/// for as long as anyone watches them.
struct Vocabulary {
    words: Words,
    parts: parts::Words,
    numbers: Numbers,
    kept: Kept,
    made: watch::Sender<Arc<Made>>,
    /// The networks watches look ahead at, by name.
    looked: BTreeMap<String, Looked>,
}

/// A network looked ahead at: by how many watches, and what's kept of it
/// while they do.
#[derive(Default)]
struct Looked {
    watchers: usize,
    look: Look,
}

impl Vocabulary {
    /// Makes the rows again from the state, each keeping its spaced order
    /// where it can, and tells whoever watches only when one of them
    /// changed. Each network looked at has its drawer on its row and its
    /// insides after every row of the list.
    fn make(&mut self, state: &State) {
        let described = state.described();
        let mut rows = rows::rows(&described, &self.words);
        let now = now();
        for (name, looked) in &mut self.looked {
            let detail = state.detail(name);
            looked.look.follow(&detail, now);
            let connected = described
                .cards
                .iter()
                .any(|card| card.network.as_deref() == Some(name.as_str()));
            let read = parts::Read {
                detail: &detail,
                connected,
                look: &looked.look,
                seen: state.seen(),
                now,
            };
            let insides = parts::insides(&read, &self.parts, &self.numbers);
            let key = rows::key(&format!("network:{name}"));
            if let Some(drawer) = insides.drawer
                && let Some(row) = rows.iter_mut().find(|row| row["key"] == key.as_str())
            {
                row["drawer"] = drawer;
            }
            rows.extend(insides.rows);
        }
        let moved = self.kept.take(rows, Vec::new());
        if moved.is_empty() {
            return;
        }
        let made: Made = self
            .kept
            .all()
            .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record.clone())))
            .collect();
        self.made.send_replace(Arc::new(made));
    }
}

/// Now, in milliseconds since the epoch, as a row's times travel.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}

/// A watch looking ahead at networks, for as long as it's kept: while it
/// is, each is made with its insides, and each card's link is read.
struct Looking {
    face: Arc<Face>,
    said: Said,
    names: Vec<String>,
    _watching: Watching,
}

impl Drop for Looking {
    fn drop(&mut self) {
        self.face.look(&self.said, &self.names, false);
    }
}

/// The networks a watch's keys name by their rows, as
/// `row:wifi:network:Lantern`.
fn looked_at(keys: &[String]) -> Vec<String> {
    let network = rows::key("network:");
    let mut names: Vec<String> = keys
        .iter()
        .filter_map(|key| key.strip_prefix(&network))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// What every connection to the socket shares: wifid's state, and the
/// rows made in each vocabulary asked for.
pub struct Face {
    state: watch::Receiver<State>,
    /// Where vocabularies are read from, the Nix store but in tests.
    store: PathBuf,
    vocabularies: Mutex<HashMap<Said, Vocabulary>>,
    /// Where wifid's main loop takes what a row's act asks.
    asks: mpsc::Sender<Ask>,
    /// How many look at a network, which the main loop reads each card's
    /// link for (service.rs).
    watching: watch::Sender<u32>,
}

impl Face {
    pub fn new(
        state: watch::Receiver<State>,
        asks: mpsc::Sender<Ask>,
        watching: watch::Sender<u32>,
    ) -> Arc<Face> {
        Face::within(state, PathBuf::from(STORE), asks, watching)
    }

    fn within(
        state: watch::Receiver<State>,
        store: PathBuf,
        asks: mpsc::Sender<Ask>,
        watching: watch::Sender<u32>,
    ) -> Arc<Face> {
        Arc::new(Face {
            state,
            store,
            vocabularies: Mutex::new(HashMap::new()),
            asks,
            watching,
        })
    }

    /// Counts watches looking at networks in or out, and makes the rows
    /// again, so a network's insides come as it's looked at and go once
    /// nobody does.
    fn look(&self, said: &Said, names: &[String], looking: bool) {
        let state = self.state.borrow().clone();
        let mut vocabularies = self.held();
        let Some(vocabulary) = vocabularies.get_mut(said) else {
            return;
        };
        for name in names {
            if looking {
                vocabulary.looked.entry(name.clone()).or_default().watchers += 1;
            } else if let Some(looked) = vocabulary.looked.get_mut(name) {
                looked.watchers = looked.watchers.saturating_sub(1);
                if looked.watchers == 0 {
                    vocabulary.looked.remove(name);
                }
            }
        }
        vocabulary.make(&state);
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

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<Said, Vocabulary>> {
        self.vocabularies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The rows made in the vocabulary at a path, said in the number
    /// settings at another if any, as they stand now and as they change;
    /// or why they can't be read.
    fn rows_in(
        &self,
        words: &str,
        numbers: Option<&str>,
    ) -> Result<(Said, watch::Receiver<Arc<Made>>), String> {
        let said = (
            self.in_store(words)?,
            numbers.map(|numbers| self.in_store(numbers)).transpose()?,
        );
        let state = self.state.borrow().clone();
        let mut vocabularies = self.held();
        if let Some(vocabulary) = vocabularies.get_mut(&said) {
            // Nobody watched while it changed, so it's made afresh first.
            if vocabulary.made.receiver_count() == 0 {
                vocabulary.make(&state);
            }
            let rows = vocabulary.made.subscribe();
            return Ok((said, rows));
        }
        let whole = read(&said.0)?;
        let numbers = match &said.1 {
            Some(path) => Numbers::read(&read(path)?),
            None => Numbers::default(),
        };
        let mut vocabulary = Vocabulary {
            words: Words::read(&whole)?,
            parts: parts::Words::read(&whole)?,
            numbers,
            kept: Kept::default(),
            made: watch::Sender::new(Arc::new(Made::new())),
            looked: BTreeMap::new(),
        };
        vocabulary.make(&state);
        let rows = vocabulary.made.subscribe();
        vocabularies.insert(said.clone(), vocabulary);
        Ok((said, rows))
    }

    /// A file's path, once it's found to be in the store.
    fn in_store(&self, path: &str) -> Result<PathBuf, String> {
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
            numbers: None,
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

/// A file in the store, read whole as JSON.
fn read(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{} can't be read: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{} isn't JSON: {error}", path.display()))
}

/// One client's connection: where its lines go, the vocabulary and number
/// settings it last watched in, and its watches by id.
struct Connection {
    face: Arc<Face>,
    told: mpsc::Sender<String>,
    words: Option<String>,
    numbers: Option<String>,
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
                let numbers = said
                    .get("numbers")
                    .and_then(Value::as_str)
                    .filter(|numbers| !numbers.is_empty())
                    .map(str::to_owned)
                    .or_else(|| self.numbers.clone());
                let (made_in, rows) = match self.face.rows_in(&words, numbers.as_deref()) {
                    Ok(rows) => rows,
                    Err(why) => return Some(error(&id, "no-words", &why)),
                };
                self.words = Some(words);
                self.numbers = numbers;
                let keys: Option<Vec<String>> =
                    said.get("keys").and_then(Value::as_array).map(|keys| {
                        keys.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    });
                if asked == "watch" {
                    self.watch(id, rows, keys, made_in);
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
            // Answered as the main loop gets to it, while the connection
            // asks on.
            "act" => {
                let asks = self.face.asks.clone();
                tokio::spawn(acts::act(asks, id.clone(), said, self.told.clone()));
                None
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
    /// unwatched, watched again by its id, or the connection goes. One
    /// whose keys name networks looks at them for as long as it lasts.
    fn watch(
        &mut self,
        id: Value,
        mut rows: watch::Receiver<Arc<Made>>,
        keys: Option<Vec<String>>,
        said: Said,
    ) {
        let told = self.told.clone();
        let named = id.to_string();
        // Watched again by its id, the one before says nothing more.
        if let Some(before) = self.watches.remove(&named) {
            before.abort();
        }
        let networks = looked_at(keys.as_deref().unwrap_or_default());
        // Looked at before it's answered, so its first answer holds the
        // network's drawer as far as it's read.
        let looking = (!networks.is_empty()).then(|| {
            self.face.look(&said, &networks, true);
            Looking {
                face: Arc::clone(&self.face),
                said,
                names: networks,
                _watching: Watching::new(&self.face.watching),
            }
        });
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
            // Kept as long as the watch is, and let go as it ends.
            let _looking = looking;
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
pub(crate) fn error(id: &Value, code: &str, message: &str) -> Value {
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
        let (face, state, store, words, _watching) = looking_face();
        (face, state, store, words)
    }

    /// A face as `face()` is, with how many look at a network.
    fn looking_face() -> (
        Arc<Face>,
        watch::Sender<State>,
        tempfile::TempDir,
        String,
        watch::Receiver<u32>,
    ) {
        let store = tempfile::tempdir().expect("a store");
        let words = store.path().join("words.json");
        let mut things = vec![
            json!({"id": "wifi:radio", "name": "Wi-Fi Off", "glyph": "offline",
                "states": {"off": {"name": "Wi-Fi On", "glyph": "wireless"}}}),
            json!({"id": "wifi:sign-in", "name": "Sign in to {network}", "glyph": "signIn"}),
            json!({"id": "wifi:network", "said": {"connected": "connected", "saved": "saved",
                "bars": "{n} of 4 bars", "open": "open", "password": "a password",
                "account": "an account"}}),
        ];
        things.extend(
            parts::tests::things()
                .into_iter()
                .filter(|thing| thing["id"] != "wifi:network"),
        );
        let vocabulary = json!({ "things": things });
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
        let (asks, _) = mpsc::channel(1);
        let watching = watch::Sender::new(0);
        let looked = watching.subscribe();
        (
            Face::within(state, root, asks, watching),
            sending,
            store,
            words,
            looked,
        )
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
    async fn an_act_on_the_socket_is_done_by_the_main_loop() {
        let (_state, state) = watch::channel(State::default());
        let (asks, mut taken) = mpsc::channel(1);
        let face = Face::within(state, PathBuf::from(STORE), asks, watch::Sender::new(0));
        let mut client = connect(&face);
        client
            .ask(json!({"id": 9, "ask": "act", "act": "scan"}))
            .await;
        let Some(Ask::Scan { done }) = taken.recv().await else {
            panic!("the main loop was asked no scan");
        };
        done.send(Ok(())).expect("answered");
        assert_eq!(client.heard().await, json!({"id": 9, "done": true}));
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
        assert_eq!(whole["records"][1]["kind"], "row");
        assert_eq!(whole["records"][1]["row"]["name"], "Wi-Fi Off");
        state.send_modify(|state| state.radio(false));
        let change = client.heard().await;
        assert_eq!(keys(&change, "changed"), ["row:wifi:radio"]);
        assert_eq!(change["changed"][0]["row"]["name"], "Wi-Fi On");
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

    /// Lantern in the air at one access point, which the card's joined
    /// and its driver has read.
    fn heard_lantern(state: &watch::Sender<State>) {
        state.send_modify(|state| {
            state.polled(
                "wlan0",
                Some(crate::detail::Polled::read("/ap/1".into(), &HashMap::new())),
            );
            state.heard(
                "/ap/1",
                Some(crate::air::Heard {
                    name: "Lantern".into(),
                    security: crate::state::Security::Personal,
                    signal: -52,
                    bssid: "aa:aa".into(),
                    frequency: 5180,
                    age: 0,
                    radio: crate::radio::Radio::default(),
                }),
            );
        });
    }

    #[tokio::test]
    async fn a_network_looked_ahead_at_is_served_with_its_insides_until_nobody_looks() {
        let (face, state, _store, words, looked) = looking_face();
        heard_lantern(&state);
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
        assert_eq!(
            keys(&whole, "records"),
            ["row:wifi:network:Lantern", "row:wifi:radio"],
            "nothing inside a network until one looks"
        );
        assert_eq!(*looked.borrow(), 0, "no link is read while nobody looks");
        client
            .ask(json!({"id": "ahead", "ask": "watch", "kind": "row",
                "keys": ["row:wifi:network:Lantern"]}))
            .await;
        let ahead = client.heard().await;
        let change = client.heard().await;
        let (ahead, change) = if ahead["id"] == "ahead" {
            (ahead, change)
        } else {
            (change, ahead)
        };
        assert_eq!(
            keys(&ahead, "records"),
            ["row:wifi:network:Lantern"],
            "the watch ahead hears only the row it names"
        );
        assert_eq!(*looked.borrow(), 1, "the link is read while it's looked at");
        let inside = keys(&change, "changed");
        assert!(
            inside.contains(&"row:wifi:network:Lantern:heard".to_owned()),
            "the place's watch hears the network's insides: {inside:?}"
        );
        assert!(
            inside.contains(&"row:wifi:network:Lantern:heard:point-aa:aa".to_owned()),
            "and its access points: {inside:?}"
        );
        client.ask(json!({"id": "ahead", "ask": "unwatch"})).await;
        let mut gone = Vec::new();
        let mut unwatched = false;
        while gone.is_empty() || !unwatched {
            let said = client.heard().await;
            unwatched |= said["unwatched"] == true;
            if let Some(keys) = said["gone"].as_array() {
                gone.extend(keys.iter().filter_map(Value::as_str).map(str::to_owned));
            }
        }
        assert!(
            gone.contains(&"row:wifi:network:Lantern:heard".to_owned()),
            "once nobody looks, its insides are told gone: {gone:?}"
        );
        assert!(
            !gone.contains(&"row:wifi:network:Lantern".to_owned()),
            "and the network's own row stays"
        );
        assert_eq!(*looked.borrow(), 0, "and its link is read no more");
    }

    #[test]
    fn a_watchs_keys_name_the_networks_it_looks_at() {
        let keys = [
            "row:wifi:network:Lantern",
            "row:wifi:radio",
            "row:wifi:network:Lantern",
            "row:wifi:network:Harbour Wi-Fi",
        ]
        .map(str::to_owned);
        assert_eq!(looked_at(&keys), ["Harbour Wi-Fi", "Lantern"]);
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
