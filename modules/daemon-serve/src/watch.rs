//! What another daemon holds, heard over its socket in version 1 of the
//! daemon interface (mycelium's PROTOCOL.md, The Daemon Interface): a
//! watch's records whole, then each change by key with only the fields
//! that differ, a dropped one as null, and the keys gone. The records are
//! held mended, as the shell's Daemon.qml holds them, so a change says
//! only which keys moved and what they now read. Each is followed on a
//! thread of its own ([`follow`]), its answers handed to the daemon's one
//! loop.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use serde_json::{Map, Value, json};

/// Every record a watch has heard, by key, as each now reads.
#[derive(Debug, Default)]
pub struct Held {
    records: BTreeMap<String, Value>,
}

/// What one answer moved: the keys it made or changed, and those gone.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Moved {
    pub changed: Vec<String>,
    pub gone: Vec<String>,
}

impl Moved {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.gone.is_empty()
    }
}

impl Held {
    /// A record as it now reads.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.records.get(key)
    }

    /// Every record, by key.
    pub fn records(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.records
            .iter()
            .map(|(key, record)| (key.as_str(), record))
    }

    /// Takes one answer to a watch: its records whole, which stand for
    /// every record it holds, those it no longer names gone; then each
    /// change mended into the record it names; then the keys gone. An
    /// answer to anything else moves nothing.
    pub fn took(&mut self, answer: &Value) -> Moved {
        let mut moved = Moved::default();
        if let Some(records) = answer.get("records").and_then(Value::as_array) {
            let named: Vec<_> = records.iter().filter_map(key_of).collect();
            moved.gone = self
                .records
                .keys()
                .filter(|key| !named.contains(&key.as_str()))
                .cloned()
                .collect();
            for key in &moved.gone {
                self.records.remove(key);
            }
            for record in records {
                if let Some(key) = key_of(record) {
                    self.records.insert(key.to_owned(), record.clone());
                    moved.changed.push(key.to_owned());
                }
            }
        }
        for change in answer
            .get("changed")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(key), Some(fields)) = (key_of(change), change.as_object()) else {
                continue;
            };
            let record = self
                .records
                .entry(key.to_owned())
                .or_insert_with(|| json!({}));
            mend(record, fields);
            moved.changed.push(key.to_owned());
        }
        for key in answer
            .get("gone")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(key) = key.as_str()
                && self.records.remove(key).is_some()
            {
                moved.gone.push(key.to_owned());
            }
        }
        moved
    }
}

fn key_of(record: &Value) -> Option<&str> {
    record.get("key").and_then(Value::as_str)
}

/// A change's fields laid over its record, a null taking the field away.
fn mend(record: &mut Value, fields: &Map<String, Value>) {
    let Some(record) = record.as_object_mut() else {
        return;
    };
    for (field, value) in fields {
        if value.is_null() {
            record.remove(field);
        } else {
            record.insert(field.clone(), value.clone());
        }
    }
}

/// A watch held open on a daemon's socket.
pub struct Watch {
    lines: BufReader<UnixStream>,
    pub held: Held,
}

impl Watch {
    /// Opens a watch of one kind on the daemon at `socket`, after the
    /// hello every client of the interface opens with.
    pub fn open(socket: &Path, kind: &str) -> io::Result<Self> {
        Self::asking(socket, &json!({ "kind": kind }))
    }

    /// Opens a watch asking what `asked` says, such as a place's rows
    /// alone (`{"kind": "row", "place": "displays"}`), after the hello.
    pub fn asking(socket: &Path, asked: &Value) -> io::Result<Self> {
        const STUB: bool = true;
        let mut stream = UnixStream::connect(socket)?;
        let hello = json!({"id": 1, "ask": "hello"});
        let mut watch = json!({"id": 2, "ask": "watch"});
        if let (Some(watch), Some(asked), false) = (watch.as_object_mut(), asked.as_object(), STUB) {
            for (field, value) in asked {
                watch.entry(field.clone()).or_insert_with(|| value.clone());
            }
        }
        writeln!(stream, "{hello}\n{watch}")?;
        Ok(Self {
            lines: BufReader::new(stream),
            held: Held::default(),
        })
    }

    /// Waits for the next answer to the watch and takes it, passing over
    /// any that move nothing; an end of the connection is an error, so
    /// whoever holds the watch opens it again.
    pub fn heard(&mut self) -> io::Result<Moved> {
        loop {
            let answer = self.answer()?;
            let moved = self.held.took(&answer);
            if !moved.is_empty() {
                return Ok(moved);
            }
        }
    }

    /// Waits for the next answer to the watch, as it was said, passing
    /// over the hello's, for whoever holds the records elsewhere.
    pub fn answer(&mut self) -> io::Result<Value> {
        loop {
            let mut line = String::new();
            if self.lines.read_line(&mut line)? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the daemon hung up",
                ));
            }
            let Ok(answer) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if answer.get("id") != Some(&json!(2)) {
                continue;
            }
            if let Some(error) = answer.get("error") {
                return Err(io::Error::other(format!("the watch was refused: {error}")));
            }
            return Ok(answer);
        }
    }
}

/// How long to wait before opening a watch again once its daemon has gone,
/// as while the node starts again on a switch: the one clock a daemon
/// following another keeps, and only while that one is away.
const AGAIN: Duration = Duration::from_secs(2);

/// Follows a daemon's watch of `kind` at `socket`, handing each answer to
/// `to` as `heard` names it, opening it again each time it ends, until
/// the loop it hands to has gone.
pub fn follow<Heard: Send + 'static>(
    socket: PathBuf,
    kind: &'static str,
    heard: fn(Value) -> Heard,
    to: Sender<Heard>,
) {
    follow_asking(socket, json!({ "kind": kind }), heard, to);
}

/// Follows a daemon's watch asking what `asked` says, as [`follow`]
/// follows one of a kind.
pub fn follow_asking<Heard: Send + 'static>(
    socket: PathBuf,
    asked: Value,
    heard: fn(Value) -> Heard,
    to: Sender<Heard>,
) {
    thread::spawn(move || {
        loop {
            if let Ok(mut watch) = Watch::asking(&socket, &asked) {
                while let Ok(answer) = watch.answer() {
                    if to.send(heard(answer)).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(AGAIN);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    #[test]
    fn a_watch_holds_its_records_mended_by_each_change() {
        let mut held = Held::default();
        let whole = json!({"id": 2, "records": [
            {"key": "session:a", "title": "A", "step": "edit"},
            {"key": "session:b", "title": "B"}
        ]});
        let moved = held.took(&whole);
        assert_eq!(
            moved.changed,
            ["session:a", "session:b"],
            "the records whole"
        );
        let change = json!({"id": 2, "changed": [{"key": "session:a", "step": null, "told": "1"}],
            "gone": ["session:b", "session:never"]});
        let moved = held.took(&change);
        assert_eq!(moved.changed, ["session:a"], "the key a change names");
        assert_eq!(moved.gone, ["session:b"], "only a key it held is gone");
        let a = held.get("session:a").expect("a is held");
        assert_eq!(
            a,
            &json!({"key": "session:a", "title": "A", "told": "1"}),
            "a null takes its field away"
        );
    }

    #[test]
    fn records_whole_again_stand_for_all_it_holds() {
        let mut held = Held::default();
        held.took(&json!({"records": [{"key": "a"}, {"key": "b"}]}));
        let moved = held.took(&json!({"records": [{"key": "b", "said": 1}]}));
        assert_eq!(moved.gone, ["a"], "one no longer named is gone");
        assert_eq!(held.records().count(), 1);
    }

    #[test]
    fn an_answer_to_anything_else_moves_nothing() {
        let mut held = Held::default();
        assert!(
            held.took(&json!({"id": 1, "version": 1, "domain": "node"}))
                .is_empty(),
            "a hello's answer moved something"
        );
    }

    #[test]
    fn over_its_socket_a_watch_hears_each_change_and_its_end() {
        let folder =
            std::env::temp_dir().join(format!("daemon-serve-watch-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket");
        let socket = folder.join("node.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the stand-in daemon listens");
        let daemon = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the watch connects");
            let mut asks = BufReader::new(stream.try_clone().expect("its stream"));
            let mut said = Vec::new();
            for _ in 0..2 {
                let mut line = String::new();
                asks.read_line(&mut line).expect("an ask");
                said.push(line);
            }
            let mut stream = stream;
            let answers = [
                json!({"id": 1, "version": 1, "domain": "node"}),
                json!({"id": 2, "records": [{"key": "session:a", "title": "A"}]}),
                json!({"id": 2, "changed": []}),
                json!({"id": 2, "changed": [{"key": "session:a", "title": "Again"}]}),
            ];
            for answer in answers {
                writeln!(stream, "{answer}").expect("an answer");
            }
            said
        });
        let mut watch = Watch::open(&socket, "session").expect("the watch opens");
        assert_eq!(watch.heard().expect("the records").changed, ["session:a"]);
        assert_eq!(
            watch.heard().expect("a change").changed,
            ["session:a"],
            "an answer moving nothing is passed over"
        );
        let title = watch
            .held
            .get("session:a")
            .and_then(|a| a.get("title"))
            .cloned();
        assert_eq!(title, Some(json!("Again")));
        let said = daemon.join().expect("the stand-in daemon ends");
        assert!(
            said[0].contains("\"hello\""),
            "it opens with hello: {}",
            said[0]
        );
        assert!(
            said[1].contains("\"watch\"") && said[1].contains("\"session\""),
            "then watches its kind: {}",
            said[1]
        );
        assert!(
            watch.heard().is_err(),
            "the daemon hanging up ends the watch"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_watch_asks_for_what_its_given_but_its_own_id() {
        let folder =
            std::env::temp_dir().join(format!("daemon-serve-asking-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket");
        let socket = folder.join("windows.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the stand-in daemon listens");
        let daemon = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the watch connects");
            let mut asks = BufReader::new(stream);
            let mut said = Vec::new();
            for _ in 0..2 {
                let mut line = String::new();
                asks.read_line(&mut line).expect("an ask");
                said.push(line);
            }
            said
        });
        let _watch = Watch::asking(
            &socket,
            &json!({"kind": "row", "place": "displays", "id": 9}),
        )
        .expect("the watch opens");
        let said = daemon.join().expect("the stand-in daemon ends");
        let watch: Value = serde_json::from_str(&said[1]).expect("the watch is JSON");
        assert_eq!(
            watch,
            json!({"id": 2, "ask": "watch", "kind": "row", "place": "displays"}),
            "its place asked for, its id kept as the watch's"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }
}
