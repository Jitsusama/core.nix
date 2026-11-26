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
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::{Message, WebSocket};

use crate::config::Feed;
use crate::gate::{self, Passed};
use crate::upstream::{Reader, Upstreams};

/// How long a read waits before the page's watches are heard: the most a
/// change waits on its way to the page.
const TURN: Duration = Duration::from_millis(20);

/// How long a daemon has to answer an ask asked once, in all.
const PATIENCE: Duration = Duration::from_secs(10);

/// The longest line an ask's answer is read to, as daemon-serve reads one.
const LONGEST_LINE: usize = 16 << 20;

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
/// it said to it, or that it didn't in time: a daemon telling line after
/// line of something else can't hold the asker past its patience either.
pub fn once(socket: &Path, asked: &Value) -> io::Result<Value> {
    once_within(socket, asked, PATIENCE)
}

/// As `once`, with `patience` in all.
fn once_within(socket: &Path, asked: &Value, patience: Duration) -> io::Result<Value> {
    let deadline = Instant::now() + patience;
    let mut stream = UnixStream::connect(socket)?;
    stream.set_write_timeout(Some(patience))?;
    let mut ask = asked.clone();
    ask["id"] = json!(2);
    writeln!(stream, "{}\n{ask}", json!({"id": 1, "ask": "hello"}))?;
    let mut lines = BufReader::new(stream);
    loop {
        let line = line_by(&mut lines, deadline)?;
        // A daemon outside the interface ends this ask in the same words
        // it ends a watch in, rather than leaving it to time out.
        if let Some(answer) = daemon_serve::watch::spoken(&line)?
            && answer.get("id") == Some(&json!(2))
        {
            return Ok(answer);
        }
    }
}

/// The next line `lines` tells by `deadline`, whose time left bounds each
/// read, so a daemon trickling a line a byte at a time can't hold the asker
/// past it either; and no longer than daemon-serve takes one.
fn line_by(lines: &mut BufReader<UnixStream>, deadline: Instant) -> io::Result<String> {
    let mut line = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "it didn't answer in time",
            ));
        }
        lines.get_ref().set_read_timeout(Some(left))?;
        let read = lines.fill_buf()?;
        if read.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "it hung up before answering",
            ));
        }
        let (taken, ended) = read
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or((read.len(), false), |at| (at + 1, true));
        line.extend_from_slice(&read[..taken]);
        lines.consume(taken);
        if line.len() > LONGEST_LINE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "it told a line past what's read",
            ));
        }
        if ended {
            return String::from_utf8(line)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "it told no text"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    /// How long each case gives the daemon in all.
    const BRIEF: Duration = Duration::from_millis(300);

    /// A stand-in daemon at a fresh socket, which reads the ask and then
    /// tells `told` a part at a time, `apart` between each.
    fn telling(name: &str, told: Vec<Vec<u8>>, apart: Duration) -> std::path::PathBuf {
        let socket =
            std::env::temp_dir().join(format!("local-sites-once-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the stand-in listens");
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut ask = [0; 256];
            let _ = stream.read(&mut ask);
            for part in told {
                if stream.write_all(&part).is_err() {
                    return;
                }
                thread::sleep(apart);
            }
        });
        socket
    }

    #[test]
    fn a_daemon_trickling_or_telling_else_is_given_up_on_in_time() {
        let trickled = br#"{"id": 2, "ok": true}"#.iter().map(|byte| vec![*byte]).collect();
        let other = (0..100)
            .map(|_| b"{\"id\": 1, \"ok\": true}\n".to_vec())
            .collect();
        for (name, told) in [("trickled", trickled), ("other", other)] {
            let socket = telling(name, told, Duration::from_millis(40));
            let asked = Instant::now();
            let answer = once_within(&socket, &json!({"ask": "steer"}), BRIEF);
            let took = asked.elapsed();
            let _ = std::fs::remove_file(&socket);
            assert!(answer.is_err(), "{name}: it answered {answer:?}");
            assert!(took < BRIEF * 3, "{name}: it was held {took:?}");
        }
    }

    #[test]
    fn an_answer_told_in_parts_is_read_whole() {
        let told = vec![br#"{"id": 2, "o"#.to_vec(), b"k\": true}\n".to_vec()];
        let socket = telling("parts", told, Duration::from_millis(10));
        let answer = once_within(&socket, &json!({"ask": "steer"}), BRIEF);
        let _ = std::fs::remove_file(&socket);
        assert_eq!(
            answer.expect("it answered")["ok"],
            json!(true),
            "its answer was misread"
        );
    }
}
