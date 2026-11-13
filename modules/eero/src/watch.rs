//! `eero watch <view>`: a view drawn again each time the home network
//! changes, and each time its ages next read differently, so what it shows
//! is never older than it says.
//!
//! It holds the daemon's socket rather than watching the record's file,
//! since the daemon tells every watch each new model, and watching the
//! network ahead reads it at the fast pace: watching is looking, and the
//! daemon should know it. When the daemon isn't listening, the page says
//! so and the watch tries again until it is.

use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use terminal_kit::Room;

use crate::listeners::KIND;
use crate::model::HomeNetwork;
use crate::views::{self, Clock, Ink, View};

/// How long to wait before trying the socket again once the daemon isn't
/// there: soon enough to catch it starting, rare enough to cost nothing.
const AGAIN: Duration = Duration::from_secs(5);

/// Where the daemon listens.
fn socket() -> Result<PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|run| PathBuf::from(run).join("eero/socket"))
        .ok_or_else(|| "XDG_RUNTIME_DIR isn't set".into())
}

/// How a watch draws: which view, as text or JSON, with what ink, and
/// and in what room: a terminal's width, drawn over the last page, or a
/// pipe's, after it.
pub struct Watch<'a> {
    pub view: &'a View,
    pub json: bool,
    pub ink: &'a Ink,
    pub room: Room,
}

/// Draws a view, then again with each change, until stopped.
pub fn watch(how: &Watch, out: &mut impl Write) -> Result<(), String> {
    let socket = socket()?;
    // What was said of the daemon's absence last, so a reader that isn't a
    // terminal is told once, not every few seconds.
    let mut said = None;
    loop {
        match UnixStream::connect(&socket) {
            Ok(mut stream) => {
                said = None;
                // A daemon that went as it was asked is waited for again.
                if look(&mut stream).is_ok() {
                    follow(how, stream, out, &Clock::system)?;
                }
            }
            Err(error) => {
                let page = absent(&socket, &error);
                if said.as_ref() != Some(&page) {
                    draw(how, out, &page)?;
                    said = Some(page);
                }
            }
        }
        thread::sleep(AGAIN);
    }
}

/// Watches the home network, and ahead, so the daemon reads it often.
fn look(stream: &mut UnixStream) -> io::Result<()> {
    for asked in [
        json!({"id": "all", "ask": "watch", "kind": KIND}),
        json!({"id": "ahead", "ask": "watch", "kind": KIND, "keys": [KIND]}),
    ] {
        stream.write_all(format!("{asked}\n").as_bytes())?;
    }
    stream.flush()
}

/// The model a line of the daemon's tells, if it tells one: the network's
/// record in a watch's answer or a change.
fn told(line: &[u8]) -> Result<Option<HomeNetwork>, String> {
    let said: Value = serde_json::from_slice(line)
        .map_err(|e| format!("eero-daemon wrote a line this eero can't read: {e}"))?;
    let records = said.get("records").or_else(|| said.get("changed"));
    let Some(record) = records
        .and_then(Value::as_array)
        .and_then(|records| records.iter().find(|record| record["key"] == KIND))
    else {
        return Ok(None);
    };
    serde_json::from_value(record["network"].clone())
        .map(Some)
        .map_err(|e| format!("eero-daemon wrote a model this eero can't read: {e}"))
}

/// What the page says while the daemon isn't listening.
fn absent(socket: &Path, error: &io::Error) -> String {
    format!(
        "eero: nothing to watch: eero-daemon isn't listening at {} ({error}); \
         start it with systemctl --user start eero-daemon\n",
        socket.display()
    )
}

/// Follows one connection to the daemon, drawing the view from each model
/// it writes, and again whenever the page's ages would read differently,
/// until the daemon goes. `clock` gives the time each page is drawn at.
pub fn follow(
    how: &Watch,
    stream: UnixStream,
    out: &mut impl Write,
    clock: &dyn Fn() -> Clock,
) -> Result<(), String> {
    let mut reading = BufReader::new(stream);
    let mut model: Option<HomeNetwork> = None;
    // Bytes of a line not yet whole, kept across a timeout so a model
    // written as the page is redrawn is still read in full.
    let mut line = Vec::new();
    draw(how, out, "eero: waiting for eero-daemon's first reading\n")?;
    loop {
        let next = match &model {
            Some(model) => {
                let now = clock();
                let page = if how.json {
                    views::json(how.view, model)
                } else {
                    views::text(how.view, model, &now, how.ink, how.room)
                };
                draw(how, out, &page?)?;
                // JSON holds no ages, so only a new model draws it again.
                (!how.json).then(|| next_change(model, &now))
            }
            None => None,
        };
        let timeout = next.map(Duration::from_secs);
        reading
            .get_ref()
            .set_read_timeout(timeout)
            .map_err(|e| format!("can't wait on eero-daemon: {e}"))?;
        match reading.read_until(b'\n', &mut line) {
            Ok(0) => {
                draw(
                    how,
                    out,
                    "eero: eero-daemon stopped; waiting for it to start\n",
                )?;
                return Ok(());
            }
            Ok(_) if line.ends_with(b"\n") => {
                if let Some(network) = told(&line)? {
                    model = Some(network);
                }
                line.clear();
            }
            // The daemon went in the middle of a line; the next read says so.
            Ok(_) => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("can't read eero-daemon: {error}")),
        }
    }
}

/// Seconds until the page's quickest-changing age reads differently: the
/// reading's, since it's the newest time any page shows.
fn next_change(model: &HomeNetwork, clock: &Clock) -> u64 {
    model
        .read
        .parse()
        .map_or(60, |read| views::changes_in(clock.age(read)))
}

/// Writes a page. On a terminal it's drawn over the last one, each line
/// cleared to its end and everything below it cleared, which doesn't
/// blank the screen between pages as clearing it first would. Anywhere
/// else each page follows the last, for a reader to take in turn.
fn draw(how: &Watch, out: &mut impl Write, page: &str) -> Result<(), String> {
    let written = if how.room.cells().is_some() {
        let mut over = String::from("\x1b[H");
        for line in page.lines() {
            over.push_str(line);
            over.push_str("\x1b[K\n");
        }
        over.push_str("\x1b[J");
        out.write_all(over.as_bytes())
    } else {
        out.write_all(page.as_bytes())
    };
    written
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::MODEL;

    fn clock() -> Clock {
        Clock {
            now: "2026-11-07T22:55:50Z".parse().unwrap(),
            zone: jiff::tz::TimeZone::fixed(jiff::tz::offset(-5)),
        }
    }

    /// What a watch writes from what the daemon writes, until it goes.
    fn watched(view: &View, json: bool, daemon: &str) -> String {
        let (ours, mut theirs) = UnixStream::pair().unwrap();
        theirs.write_all(daemon.as_bytes()).unwrap();
        drop(theirs);
        let ink = Ink::plain();
        let how = Watch {
            view,
            json,
            ink: &ink,
            room: Room::Unbounded,
        };
        let mut out = Vec::new();
        follow(&how, ours, &mut out, &clock).unwrap();
        String::from_utf8(out).unwrap()
    }

    /// A change the daemon tells, as one line.
    fn line(model: &HomeNetwork) -> String {
        let record = json!({"key": KIND, "kind": KIND, "network": model});
        format!("{}\n", json!({"id": "all", "changed": [record]}))
    }

    #[test]
    fn each_model_the_daemon_writes_draws_the_view_again() {
        let model: HomeNetwork = serde_json::from_str(MODEL).unwrap();
        let mut renamed = model.clone();
        renamed.name = "cabin".into();
        let daemon = format!("{}{}", line(&model), line(&renamed));

        let out = watched(&View::Firmware, false, &daemon);

        let heads: Vec<&str> = out
            .lines()
            .filter(|line| line.contains(" · firmware · "))
            .collect();
        assert_eq!(
            heads,
            [
                "home · firmware · read 1 min 7 s ago",
                "cabin · firmware · read 1 min 7 s ago"
            ]
        );
        assert!(out.starts_with("eero: waiting for eero-daemon's first reading\n"));
        assert!(out.ends_with("eero: eero-daemon stopped; waiting for it to start\n"));
    }

    #[test]
    fn a_json_watch_writes_the_slice_of_each_model() {
        let model: HomeNetwork = serde_json::from_str(MODEL).unwrap();

        let out = watched(&View::Events, true, &line(&model));

        let slice = out
            .split_once('\n')
            .and_then(|(_, rest)| rest.rsplit_once("eero: "))
            .map(|(slice, _)| slice)
            .unwrap();
        let read: serde_json::Value = serde_json::from_str(slice).unwrap();
        assert_eq!(read["events"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn a_model_this_eero_cant_read_ends_the_watch_saying_so() {
        let (ours, mut theirs) = UnixStream::pair().unwrap();
        let record = json!({"key": KIND, "kind": KIND, "network": {"read": 5}});
        let answer = json!({"id": "all", "records": [record]});
        theirs
            .write_all(format!("{{\"id\":\"hello\"}}\n{answer}\n").as_bytes())
            .unwrap();
        drop(theirs);
        let ink = Ink::plain();
        let how = Watch {
            view: &View::Status,
            json: false,
            ink: &ink,
            room: Room::Unbounded,
        };

        let refused = follow(&how, ours, &mut Vec::new(), &clock).unwrap_err();

        assert!(
            refused.starts_with("eero-daemon wrote a model this eero can't read"),
            "{refused}"
        );
    }

    #[test]
    fn a_watch_asks_for_the_network_and_looks_at_it() {
        let (mut ours, theirs) = UnixStream::pair().unwrap();

        look(&mut ours).unwrap();
        drop(ours);

        let asked: Vec<Value> = BufReader::new(theirs)
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect();
        assert_eq!(
            asked,
            [
                json!({"id": "all", "ask": "watch", "kind": KIND}),
                json!({"id": "ahead", "ask": "watch", "kind": KIND, "keys": [KIND]}),
            ],
            "a watch watches the network, and ahead, so the daemon reads it often"
        );
    }

    #[test]
    fn on_a_terminal_a_page_is_drawn_over_the_last() {
        let ink = Ink::plain();
        let how = Watch {
            view: &View::Status,
            json: false,
            ink: &ink,
            room: Room::Cells(80),
        };
        let mut out = Vec::new();

        draw(&how, &mut out, "one\ntwo\n").unwrap();

        assert_eq!(out, b"\x1b[Hone\x1b[K\ntwo\x1b[K\n\x1b[J");
    }
}
