//! Asking the daemon to do an act, as `eero devices <MAC> reserve` does:
//! the command never opens eero's session itself, so a change to the home
//! network goes only through the daemon's socket, which takes only the
//! acts acts.rs names and logs each with who asked (changes.rs).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::watch;

/// How long an act may take, all told: a reservation makes six calls of
/// up to the cloud's 20 s each, behind whatever read the daemon is in,
/// the longest of which makes ten. That's room for the usual worst; it
/// isn't for a slow cloud through every read falling due at once, or a
/// refresh doubling a call. Past it, the outcome is said unknown, and the
/// daemon lets the act go, logged `gone`, unless it has begun it, so
/// the act's last line in the change log says which once it ends.
const PATIENCE: Duration = Duration::from_mins(6);

/// How long a daemon that took no ask is given to say it refused the
/// connection, which it does as it closes it.
const REFUSAL: Duration = Duration::from_secs(2);

/// Why an act wasn't done, or isn't known to be.
#[derive(Debug, PartialEq, Eq)]
pub enum Undone {
    /// The daemon isn't there to ask, so nothing was asked.
    Absent(String),
    /// The daemon or eero said no, and why.
    Refused(String),
    /// eero couldn't be reached; asking again later may work.
    Unreachable(String),
    /// eero's session needs signing in again.
    SignedOut(String),
    /// It was asked, but whether it was done isn't known.
    Unknown(String),
    /// The daemon couldn't log the ask, so it did nothing: its own state
    /// folder, not eero, is at fault.
    Unlogged(String),
    /// The daemon knows no such act, as one older than this command
    /// wouldn't, so nothing was done.
    NoSuchAct(String),
    /// The daemon knows the act but couldn't read the ask's fields,
    /// which this command checks first, so the two disagree; nothing
    /// was done.
    Misasked(String),
    /// The daemon answers only its own account, so nothing was read.
    NotOurs(String),
}

/// Asks the daemon for `act`, an act's fields as acts.rs reads them, and
/// waits for its answer, which says what it freed, if anything.
pub fn act(act: &Value) -> Result<Value, Undone> {
    let socket = watch::socket().map_err(Undone::Absent)?;
    act_at(&socket, act, PATIENCE)
}

/// Asks the daemon at `socket` for `act`, waiting up to `patience` in all
/// for its answer, however much else it says meanwhile. Once the ask is
/// sent, the daemon may do it whatever comes after, so silence then is an
/// unknown outcome, never an absent daemon. A daemon that refused the
/// connection says so first, read even when the ask couldn't be sent.
fn act_at(socket: &Path, act: &Value, patience: Duration) -> Result<Value, Undone> {
    let started = Instant::now();
    let absent = |error: std::io::Error| Undone::Absent(format!("{}: {error}", socket.display()));
    let mut stream = UnixStream::connect(socket).map_err(absent)?;
    let mut asked = act.clone();
    asked["id"] = json!(1);
    asked["ask"] = json!("act");
    let sent = writeln!(stream, "{asked}");
    // An ask that couldn't be sent was never read, so only a refusal the
    // daemon wrote as it closed is waited for, briefly.
    let deadline = if sent.is_ok() {
        started + patience
    } else {
        Instant::now() + REFUSAL.min(patience)
    };
    let mut reading = BufReader::new(stream);
    loop {
        let answer = match line_by(&mut reading, deadline) {
            Ok(line) => answered(line.trim_end()),
            Err(hung_up) => return Err(unanswered(socket, hung_up, &sent)),
        };
        match (answer, &sent) {
            (Some(answer @ Err(Undone::NotOurs(_))), _) | (Some(answer), Ok(())) => return answer,
            _ => {}
        }
    }
}

/// The longest line read whole; anything longer is no answer of ours.
const LONGEST: usize = 64 * 1024;

/// The next line the daemon says, by `deadline` however it arrives: each
/// read waits only what's left, so a line sent a little at a time can't
/// hold the wait past it. Its end is `Err(true)` when the daemon hung up
/// and `Err(false)` when the deadline passed.
fn line_by(reading: &mut BufReader<UnixStream>, deadline: Instant) -> Result<String, bool> {
    let mut line = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || reading.get_ref().set_read_timeout(Some(left)).is_err() {
            return Err(false);
        }
        let got = match reading.fill_buf() {
            Ok(got) => got,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(false),
        };
        if got.is_empty() {
            return Err(true);
        }
        let (taken, ended) = got
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or((got.len(), false), |at| (at + 1, true));
        if line.len() < LONGEST {
            line.extend_from_slice(&got[..taken]);
        }
        reading.consume(taken);
        if ended {
            return Ok(String::from_utf8_lossy(&line).into_owned());
        }
    }
}

/// What the wait for our answer came to when the daemon said no more,
/// having `hung_up` or let the deadline pass: an ask sent may have been
/// done, so it's unknown, but one that couldn't be sent was never read,
/// so the daemon is as good as absent.
fn unanswered(socket: &Path, hung_up: bool, sent: &std::io::Result<()>) -> Undone {
    match sent {
        // A daemon that hangs up on a waiting ask has stopped, as on a
        // crash or a restart, so the act may never log its end.
        Ok(()) if hung_up => Undone::Unknown(
            "the daemon went before it answered; it may have stopped partway, so the change \
             log may hold only the ask"
                .into(),
        ),
        Ok(()) => Undone::Unknown(
            "the daemon didn't answer in time; it may have been done or still be under way, \
             and its last line in the change log says which once it ends"
                .into(),
        ),
        Err(error) => Undone::Absent(format!("{}: {error}", socket.display())),
    }
}

/// The daemon's answer to our ask, if `line` is it, rather than anything
/// else it tells a listener.
fn answered(line: &str) -> Option<Result<Value, Undone>> {
    let said: Value = serde_json::from_str(line).ok()?;
    let code = said["error"]["code"].as_str();
    // A refused connection is told before its ask is read, so under no id.
    if code == Some("not-ours") {
        return Some(Err(Undone::NotOurs(
            "eero-daemon answers only its own account".into(),
        )));
    }
    if said["id"] != json!(1) {
        return None;
    }
    if said["done"] == json!(true) {
        return Some(Ok(said));
    }
    let why = said["error"]["message"]
        .as_str()
        .or(code)
        .unwrap_or("the daemon answered without saying why")
        .to_owned();
    Some(Err(match code {
        Some("unreachable") => Undone::Unreachable(why),
        Some("signed-out") => Undone::SignedOut(why),
        Some("unknown") => Undone::Unknown(why),
        Some("unlogged") => Undone::Unlogged(why),
        Some("bad-act") => Undone::NoSuchAct(why),
        Some("bad-ask") => Undone::Misasked(why),
        _ => Undone::Refused(why),
    }))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    #[test]
    fn only_the_answer_to_our_ask_ends_the_wait() {
        let refused = r#"{"id":1,"error":{"code":"refused","message":"no"}}"#;
        let unreachable = r#"{"id":1,"error":{"code":"unreachable","message":"down"}}"#;
        let signed_out = r#"{"id":1,"error":{"code":"signed-out","message":"aged"}}"#;
        let unknown = r#"{"id":1,"error":{"code":"unknown","message":"read it"}}"#;
        let unlogged = r#"{"id":1,"error":{"code":"unlogged","message":"disk full"}}"#;
        assert_eq!(
            [
                answered(r#"{"id":1,"done":true,"held":"10.0.0.208"}"#),
                answered(refused),
                answered(unreachable),
                answered(signed_out),
                answered(unknown),
                answered(unlogged),
                answered(r#"{"id":1,"error":{"code":"bad-act"}}"#),
                answered(r#"{"id":1,"error":{"code":"bad-ask"}}"#),
                answered(NOT_OURS),
                answered(r#"{"id":2,"done":true}"#),
                answered(r#"{"change":"devices"}"#),
                answered("not json"),
            ],
            [
                Some(Ok(json!({"id": 1, "done": true, "held": "10.0.0.208"}))),
                Some(Err(Undone::Refused("no".into()))),
                Some(Err(Undone::Unreachable("down".into()))),
                Some(Err(Undone::SignedOut("aged".into()))),
                Some(Err(Undone::Unknown("read it".into()))),
                Some(Err(Undone::Unlogged("disk full".into()))),
                Some(Err(Undone::NoSuchAct("bad-act".into()))),
                Some(Err(Undone::Misasked("bad-ask".into()))),
                Some(Err(Undone::NotOurs(
                    "eero-daemon answers only its own account".into()
                ))),
                None,
                None,
                None,
            ],
            "an answer was missed, misread, or another line taken for it"
        );
    }

    /// Patience no answer in a test should need, which a timely one ends
    /// long before.
    const AMPLE: Duration = Duration::from_secs(10);

    /// What asking a daemon that says `lines` after reading the ask, then
    /// hangs up or, where it `holds`, holds on until the ask is given up,
    /// comes to. One that holds on is given the short patience its test is
    /// about, and lets go only once the asker has, so a stalled thread
    /// can't hang it up first; one that answers or hangs up ends the wait
    /// at once, so it's given ample, since a loaded builder can stall a
    /// thread past a short one.
    fn asked_of(name: &str, lines: &'static [&'static str], holds: bool) -> Result<Value, Undone> {
        let folder =
            std::env::temp_dir().join(format!("eero-asking-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let socket = folder.join("socket");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the test's socket listens");
        let (given_up, let_go) = std::sync::mpsc::channel::<()>();
        let daemon = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the ask connects");
            let mut reading = BufReader::new(stream.try_clone().expect("the stream clones"));
            let mut ask = String::new();
            reading.read_line(&mut ask).expect("the ask is read");
            let mut writing = stream;
            for line in lines {
                writeln!(writing, "{line}").expect("the answer is written");
            }
            if holds {
                let _ = let_go.recv();
            }
            ask
        });
        let patience = if holds {
            Duration::from_millis(300)
        } else {
            AMPLE
        };
        let answer = act_at(&socket, &json!({"act": "speedtest"}), patience);
        let _ = given_up.send(());
        let ask = daemon.join().expect("the daemon's thread ends");
        assert_eq!(
            serde_json::from_str::<Value>(&ask).ok(),
            Some(json!({"act": "speedtest", "id": 1, "ask": "act"})),
            "the ask wasn't sent as the daemon reads one"
        );
        let _ = std::fs::remove_dir_all(&folder);
        answer
    }

    #[test]
    fn an_act_is_done_or_refused_as_the_daemon_answers_it() {
        assert_eq!(
            asked_of(
                "done",
                &[r#"{"change":"x"}"#, r#"{"id":1,"done":true}"#],
                false
            ),
            Ok(json!({"id": 1, "done": true})),
            "a done answer after another line wasn't taken"
        );
        assert_eq!(
            asked_of(
                "refused",
                &[r#"{"id":1,"error":{"code":"refused","message":"taken"}}"#],
                false
            ),
            Err(Undone::Refused("taken".into())),
            "a refusal wasn't said as the daemon said it"
        );
    }

    use crate::listeners::NOT_OURS;

    /// A socket in a folder of the test's own, and the folder.
    fn socket(name: &str) -> (std::path::PathBuf, UnixListener) {
        let folder =
            std::env::temp_dir().join(format!("eero-asking-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let socket = folder.join("socket");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the test's socket listens");
        (socket, listener)
    }

    #[test]
    fn a_daemon_saying_other_things_is_waited_for_no_longer_than_our_patience() {
        let (socket, listener) = socket("chatty");
        let daemon = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the ask connects");
            let until = Instant::now() + Duration::from_secs(3);
            while Instant::now() < until && writeln!(stream, r#"{{"change":"devices"}}"#).is_ok() {
                thread::sleep(Duration::from_millis(50));
            }
        });
        let started = Instant::now();
        let answer = act_at(
            &socket,
            &json!({"act": "speedtest"}),
            Duration::from_millis(300),
        );
        let waited = started.elapsed();
        assert!(
            matches!(answer, Err(Undone::Unknown(_))) && waited < Duration::from_secs(2),
            "a daemon saying other things held the ask {waited:?}, to {answer:?}"
        );
        let _ = daemon.join();
        let _ = std::fs::remove_dir_all(socket.parent().expect("the socket's folder"));
    }

    #[test]
    fn a_line_said_a_little_at_a_time_holds_the_wait_no_longer_than_our_patience() {
        let (socket, listener) = socket("trickle");
        let daemon = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the ask connects");
            let until = Instant::now() + Duration::from_secs(3);
            while Instant::now() < until && write!(stream, "x").is_ok() {
                thread::sleep(Duration::from_millis(50));
            }
        });
        let started = Instant::now();
        let answer = act_at(
            &socket,
            &json!({"act": "speedtest"}),
            Duration::from_millis(300),
        );
        let waited = started.elapsed();
        assert!(
            matches!(&answer, Err(Undone::Unknown(_))) && waited < Duration::from_secs(2),
            "a line said a little at a time held the ask {waited:?}, to {answer:?}"
        );
        let _ = daemon.join();
        let _ = std::fs::remove_dir_all(socket.parent().expect("the socket's folder"));
    }

    #[test]
    fn an_ask_never_sent_is_said_never_read_and_one_sent_unknown() {
        let socket = Path::new("/run/eero/socket");
        let broken = || Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert_eq!(
            [
                unanswered(socket, true, &broken()),
                unanswered(socket, false, &broken()),
            ],
            [
                Undone::Absent("/run/eero/socket: broken pipe".into()),
                Undone::Absent("/run/eero/socket: broken pipe".into()),
            ],
            "an ask that couldn't be sent was said as one that may have been done"
        );
        for hung_up in [true, false] {
            assert!(
                matches!(unanswered(socket, hung_up, &Ok(())), Undone::Unknown(_)),
                "an ask sent and unanswered was said as never read"
            );
        }
        // A daemon that hung up stopped, so it never logs the act's end.
        assert!(
            matches!(
                unanswered(socket, true, &Ok(())),
                Undone::Unknown(why) if why.contains("may hold only the ask")
            ),
            "a daemon that hung up was said to log the act's end"
        );
    }

    #[test]
    fn a_daemon_refusing_the_connection_is_said_to_have_read_nothing() {
        let (socket, listener) = socket("not-ours");
        let daemon = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the ask connects");
            writeln!(stream, "{NOT_OURS}").expect("the refusal is written");
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });
        let answer = act_at(&socket, &json!({"act": "speedtest"}), AMPLE);
        assert!(
            matches!(answer, Err(Undone::NotOurs(_))),
            "a refused connection was said as {answer:?}, not as never read"
        );
        let _ = daemon.join();
        let _ = std::fs::remove_dir_all(socket.parent().expect("the socket's folder"));
    }

    #[test]
    fn silence_after_the_ask_is_an_unknown_outcome_not_an_absent_daemon() {
        assert!(
            matches!(asked_of("gone", &[], false), Err(Undone::Unknown(_))),
            "a daemon gone after the ask read as one never asked"
        );
        let slow = asked_of("slow", &[], true);
        assert!(
            matches!(&slow, Err(Undone::Unknown(why)) if why.contains("didn't answer in time")),
            "a daemon slower than our patience wasn't given up on at it: {slow:?}"
        );
        let nowhere = std::env::temp_dir().join(format!("eero-asking-none-{}", std::process::id()));
        assert!(
            matches!(
                act_at(
                    &nowhere,
                    &json!({"act": "speedtest"}),
                    Duration::from_millis(300)
                ),
                Err(Undone::Absent(_))
            ),
            "no daemon at all wasn't said absent"
        );
    }
}
