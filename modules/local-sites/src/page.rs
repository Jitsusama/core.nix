//! One page's feed: a WebSocket at `/feed/<name>`, each text message one
//! line of the daemon interface. What the page asks goes through the gate
//! (gate.rs); a watch joins the one the server holds on the daemon
//! (upstream.rs); a get, a list or a health is asked of the daemon once,
//! on a connection of its own, and its answer trimmed to what the feed
//! shows. The connection reads with a short timeout so it can also send
//! what the watches hear, since a WebSocket over a plain stream can't be
//! read on one thread and written on another.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

use crate::config::Feed;
use crate::gate::{self, Passed};
use crate::upstream::{Reader, Upstreams};

/// How long a read waits before the page's watches are heard: the most a
/// change waits on its way to the page.
const TURN: Duration = Duration::from_millis(20);

/// How long a daemon has to answer an ask asked once.
const PATIENCE: Duration = Duration::from_secs(10);

/// Serves one page's feed until it closes.
pub fn serve(
    mut socket: WebSocket<TcpStream>,
    name: &str,
    feed: &Feed,
    upstreams: &Arc<Upstreams>,
) {
    if socket.get_ref().set_read_timeout(Some(TURN)).is_err() {
        return;
    }
    let (to, heard) = mpsc::channel::<Value>();
    // The page's watches by the id it gave each: the kind and the number
    // the server's watch knows it by.
    let mut watches: BTreeMap<String, (String, u64)> = BTreeMap::new();
    loop {
        match socket.read() {
            Ok(Message::Text(line)) => {
                let answer = asked(line.as_str(), name, feed, upstreams, &to, &mut watches);
                if let Some(answer) = answer
                    && socket.send(Message::text(answer.to_string())).is_err()
                {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(why))
                if matches!(
                    why.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(_) => break,
        }
        let mut sent = true;
        while let Ok(line) = heard.try_recv() {
            if socket.write(Message::text(line.to_string())).is_err() {
                sent = false;
                break;
            }
        }
        if !sent || socket.flush().is_err() {
            break;
        }
    }
    for (kind, number) in watches.values() {
        upstreams.unwatch(name, kind, *number);
    }
}

/// What the page's `line` asks, answered now where it can be; a watch's
/// and an ask's answers come by `to`.
fn asked(
    line: &str,
    name: &str,
    feed: &Feed,
    upstreams: &Arc<Upstreams>,
    to: &Sender<Value>,
    watches: &mut BTreeMap<String, (String, u64)>,
) -> Option<Value> {
    match gate::pass(feed, line) {
        Passed::Refused(answer) => Some(answer),
        Passed::Hello { id } => Some(json!({
            "id": id,
            "version": 1,
            "domain": "local-sites",
            "feed": name,
            "kinds": feed.kinds.keys().collect::<Vec<_>>(),
        })),
        Passed::Unwatch { id } => {
            if let Some((kind, number)) = watches.remove(&id.to_string()) {
                upstreams.unwatch(name, &kind, number);
            }
            None
        }
        Passed::Watch {
            id,
            kind,
            keys,
            fields,
        } => {
            if watches.contains_key(&id.to_string()) {
                return Some(gate::error(&id, "refused", "a watch with this id is open"));
            }
            let declared = &feed.kinds[&kind];
            let reader = Reader {
                id: id.clone(),
                keys: keys.map(|keys| keys.into_iter().collect()),
                fields,
                to: to.clone(),
            };
            let number = upstreams.watch(name, &feed.socket, (&kind, declared), reader);
            watches.insert(id.to_string(), (kind, number));
            None
        }
        Passed::Once { id, asked } => {
            let (feed, to) = (feed.clone(), to.clone());
            thread::spawn(move || {
                let answer = once(&feed.socket, &asked).map_or_else(
                    |why| {
                        gate::error(
                            &id,
                            "unavailable",
                            &format!("{} doesn't answer: {why}", feed.socket.display()),
                        )
                    },
                    |answer| trimmed(&feed, answer, &id),
                );
                // The page may have closed meanwhile; nobody's left to tell.
                let _ = to.send(answer);
            });
            None
        }
    }
}

/// Asks the daemon at `socket` one thing after the hello, and answers what
/// it said to it.
fn once(socket: &Path, asked: &Value) -> io::Result<Value> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    let mut ask = asked.clone();
    ask["id"] = json!(2);
    writeln!(stream, "{}\n{ask}", json!({"id": 1, "ask": "hello"}))?;
    let mut lines = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if lines.read_line(&mut line)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "it hung up before answering",
            ));
        }
        if let Ok(answer) = serde_json::from_str::<Value>(&line)
            && answer.get("id") == Some(&json!(2))
        {
            return Ok(answer);
        }
    }
}

/// A daemon's answer under the page's id, its records trimmed to what the
/// feed shows.
fn trimmed(feed: &Feed, mut answer: Value, id: &Value) -> Value {
    answer["id"] = id.clone();
    if let Some(records) = answer.get_mut("records").and_then(Value::as_array_mut) {
        *records = records
            .iter()
            .filter_map(|record| gate::shown(feed, record))
            .collect();
    }
    answer
}
