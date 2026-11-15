//! Tells wifid when Joel's on a call, so wifid keeps the card on the access
//! point it's on for the call rather than move it to a quicker one and
//! stall the call while it does.
//!
//! ```text
//! wifid-calls [/run/wifid/rows]
//! ```
//!
//! It reads `pw-dump --monitor` as pause-for-calls does, counting a call as
//! modules/calls does: a program recording from a microphone for two
//! seconds, over once none has for five. It says a call on at its start and
//! again each minute while it lasts, since wifid lets a call nobody speaks
//! of lapse after ten minutes, and over at its end, as a calling act on
//! wifid's rows socket, each on a connection of its own (calling.rs in
//! wifid's daemon). It says nothing is on as it starts, so a call said on
//! before it last stopped holds nothing. It ends when PipeWire goes, for
//! systemd to start it again.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use calls::{Call, Recording};
use serde_json::{Value, json};

/// How often a call still on is said again, well inside wifid's lapse.
const AGAIN: Duration = Duration::from_secs(60);

/// How long wifid has to answer, so a wifid that's stuck holds up no more
/// than that.
const ANSWERED: Duration = Duration::from_secs(2);

/// What wifid was last told, and when.
#[derive(Default)]
struct Teller {
    said: Option<(bool, Instant)>,
}

impl Teller {
    /// What wifid is to be told now, if anything: whether a call is on when
    /// that's changed, or when nothing's been said yet, or again when a
    /// call has been on a minute since it was last said.
    fn tell(&mut self, on: bool, now: Instant) -> Option<bool> {
        if let Some((said, at)) = self.said
            && said == on
            && (!on || now.duration_since(at) < AGAIN)
        {
            return None;
        }
        self.said = Some((on, now));
        Some(on)
    }

    /// How long until a call still on is to be said again.
    fn due(&self, now: Instant) -> Option<Duration> {
        match self.said {
            Some((true, at)) => Some(AGAIN.saturating_sub(now.duration_since(at))),
            _ => None,
        }
    }
}

/// Whether a call is on as it stands, a call app reconnecting still in it.
fn on(call: Call) -> bool {
    matches!(call, Call::On | Call::Ending(_))
}

/// Tells wifid on its rows socket whether a call is on, and reads that it
/// took it.
fn tell(socket: &Path, on: bool) -> Result<(), String> {
    let stream = UnixStream::connect(socket).map_err(|why| format!("{why}"))?;
    stream
        .set_read_timeout(Some(ANSWERED))
        .and_then(|()| stream.set_write_timeout(Some(ANSWERED)))
        .map_err(|why| format!("{why}"))?;
    let act = json!({"id": 1, "ask": "act", "act": "calling", "on": on});
    writeln!(&stream, "{act}").map_err(|why| format!("{why}"))?;
    let mut line = String::new();
    BufReader::new(&stream)
        .read_line(&mut line)
        .map_err(|why| format!("{why}"))?;
    let answer: Value = serde_json::from_str(&line).map_err(|_| "an answer not JSON")?;
    if answer["done"] == true {
        return Ok(());
    }
    let error = &answer["error"];
    Err(format!(
        "{}: {}",
        error["code"].as_str().unwrap_or("no code"),
        error["message"].as_str().unwrap_or("no message")
    ))
}

/// The sooner of two waits, either of which may be none.
fn sooner(one: Option<Duration>, other: Option<Duration>) -> Option<Duration> {
    match (one, other) {
        (Some(one), Some(other)) => Some(one.min(other)),
        (one, other) => one.or(other),
    }
}

fn main() -> ExitCode {
    let socket = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/run/wifid/rows".into());
    let Ok(mut dump) = Command::new("pw-dump")
        .arg("--monitor")
        .arg("--no-colors")
        .stdout(Stdio::piped())
        .spawn()
    else {
        eprintln!("wifid-calls: pw-dump wouldn't start");
        return ExitCode::FAILURE;
    };
    let Some(out) = dump.stdout.take() else {
        return ExitCode::FAILURE;
    };

    // pw-dump's arrays are read on a thread of their own, so the waits for
    // a call to start or end, or to be said again, can run out between
    // them.
    let (arrays, heard) = mpsc::channel();
    thread::spawn(move || {
        for array in serde_json::Deserializer::from_reader(BufReader::new(out)).into_iter::<Value>()
        {
            let Ok(array) = array else { break };
            if arrays.send(array).is_err() {
                break;
            }
        }
    });

    let (mut recording, mut call) = (Recording::default(), Call::Off);
    let mut teller = Teller::default();
    loop {
        let now = Instant::now();
        if let Some(said) = teller.tell(on(call), now)
            && let Err(why) = tell(Path::new(&socket), said)
        {
            eprintln!("wifid-calls: couldn't tell wifid: {why}");
        }
        let next = match sooner(call.due(now), teller.due(now)) {
            Some(wait) => heard.recv_timeout(wait),
            None => heard
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match next {
            Ok(array) => recording.take(&array),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        call.step(recording.any(), Instant::now());
    }
    let _ = dump.wait();
    eprintln!("wifid-calls: pw-dump ended");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;

    #[test]
    fn nothing_on_is_said_at_the_start_then_only_a_change() {
        let (mut teller, start) = (Teller::default(), Instant::now());
        assert_eq!(teller.tell(false, start), Some(false), "said at the start");
        let later = start + AGAIN * 3;
        assert_eq!(
            teller.tell(false, later),
            None,
            "no call, nothing more to say"
        );
        assert_eq!(teller.due(later), None, "and nothing to wait for");
        assert_eq!(
            teller.tell(true, later),
            Some(true),
            "a call starting is said"
        );
        assert_eq!(teller.tell(false, later), Some(false), "and ending");
    }

    #[test]
    fn a_call_on_is_said_again_each_minute() {
        let (mut teller, start) = (Teller::default(), Instant::now());
        teller.tell(true, start);
        let soon = start + Duration::from_secs(20);
        assert_eq!(teller.tell(true, soon), None, "not yet a minute on");
        assert_eq!(
            teller.due(soon),
            Some(Duration::from_secs(40)),
            "said again then"
        );
        assert_eq!(
            teller.tell(true, start + AGAIN),
            Some(true),
            "a minute on, again"
        );
    }

    #[test]
    fn a_call_app_reconnecting_is_still_on_a_call() {
        let now = Instant::now();
        assert!(on(Call::On), "on");
        assert!(on(Call::Ending(now)), "still on while it reconnects");
        assert!(!on(Call::Starting(now)), "not on until two seconds in");
        assert!(!on(Call::Off), "off");
    }

    #[test]
    fn wifid_is_told_as_its_rows_socket_takes_a_call() {
        let folder = tempfile::tempdir().expect("a folder");
        let socket = folder.path().join("rows");
        let listener = UnixListener::bind(&socket).expect("a socket");
        let wifid = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("a connection");
            let mut line = String::new();
            BufReader::new(&stream)
                .read_line(&mut line)
                .expect("a line");
            writeln!(&stream, "{}", json!({"id": 1, "done": true})).expect("answered");
            serde_json::from_str::<Value>(&line).expect("JSON")
        });
        assert_eq!(tell(&socket, true), Ok(()), "told");
        assert_eq!(
            wifid.join().expect("wifid heard"),
            json!({"id": 1, "ask": "act", "act": "calling", "on": true}),
            "a calling act of exactly its own shape"
        );
    }

    #[test]
    fn a_refusal_or_no_wifid_is_said_as_it_is() {
        let folder = tempfile::tempdir().expect("a folder");
        let socket = folder.path().join("rows");
        assert!(tell(&socket, true).is_err(), "no wifid, no telling");
        let listener = UnixListener::bind(&socket).expect("a socket");
        let wifid = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("a connection");
            let refused = json!({"id": 1, "error": {"code": "bad-ask", "message": "no"}});
            writeln!(&stream, "{refused}").expect("answered");
        });
        let refused = tell(&socket, false);
        wifid.join().expect("wifid answered");
        assert!(
            refused.as_ref().is_err_and(|why| why.contains("bad-ask")),
            "the refusal is said: {refused:?}"
        );
    }

    #[test]
    fn the_sooner_wait_is_taken() {
        let (one, two) = (Duration::from_secs(1), Duration::from_secs(2));
        assert_eq!(sooner(Some(two), Some(one)), Some(one), "both");
        assert_eq!(sooner(None, Some(two)), Some(two), "one");
        assert_eq!(sooner(None, None), None, "none");
    }
}
