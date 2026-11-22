//! Asking the daemon to do an act, as `eero devices <MAC> reserve` does:
//! the command never opens eero's session itself, so a change to the home
//! network goes only through the daemon's socket, which takes only the
//! acts acts.rs names and logs each with who asked (changes.rs).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::watch;

/// How long an act may take: a reservation reads eero's reservations,
/// its home subnet, devices and eeros before it asks, and reads the
/// reservations back after, each up to the cloud's 20 s, behind whatever
/// read the daemon is in.
const PATIENCE: Duration = Duration::from_mins(3);

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
}

/// Asks the daemon for `act`, an act's fields as acts.rs reads them, and
/// waits for its answer, which says what it freed, if anything.
pub fn act(act: &Value) -> Result<Value, Undone> {
    let socket = watch::socket().map_err(Undone::Absent)?;
    act_at(&socket, act, PATIENCE)
}

/// Asks the daemon at `socket` for `act`, waiting up to `patience` for
/// its answer. Once the ask is sent, the daemon may do it whatever comes
/// after, so silence then is an unknown outcome, never an absent daemon.
fn act_at(socket: &Path, act: &Value, patience: Duration) -> Result<Value, Undone> {
    let absent = |error: std::io::Error| Undone::Absent(format!("{}: {error}", socket.display()));
    let mut stream = UnixStream::connect(socket).map_err(absent)?;
    stream.set_read_timeout(Some(patience)).map_err(absent)?;
    let mut asked = act.clone();
    asked["id"] = json!(1);
    asked["ask"] = json!("act");
    writeln!(stream, "{asked}").map_err(absent)?;
    let unknown = |what: &str| {
        Undone::Unknown(format!(
            "{what}; it may have been done, so read it again before asking again"
        ))
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            return Err(unknown("the daemon didn't answer in time"));
        };
        if let Some(answer) = answered(&line) {
            return answer;
        }
    }
    Err(unknown("the daemon went before it answered"))
}

/// The daemon's answer to our ask, if `line` is it, rather than anything
/// else it tells a listener.
fn answered(line: &str) -> Option<Result<Value, Undone>> {
    let said: Value = serde_json::from_str(line).ok()?;
    if said["id"] != json!(1) {
        return None;
    }
    if said["done"] == json!(true) {
        return Some(Ok(said));
    }
    let code = said["error"]["code"].as_str();
    let why = said["error"]["message"]
        .as_str()
        .or(code)
        .unwrap_or("the daemon answered without saying why")
        .to_owned();
    Some(Err(match code {
        Some("unreachable") => Undone::Unreachable(why),
        Some("signed-out") => Undone::SignedOut(why),
        Some("unknown") => Undone::Unknown(why),
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
        assert_eq!(
            [
                answered(r#"{"id":1,"done":true,"freed":"10.0.0.208"}"#),
                answered(refused),
                answered(unreachable),
                answered(signed_out),
                answered(unknown),
                answered(r#"{"id":1,"error":{"code":"bad-act"}}"#),
                answered(r#"{"id":2,"done":true}"#),
                answered(r#"{"change":"devices"}"#),
                answered("not json"),
            ],
            [
                Some(Ok(json!({"id": 1, "done": true, "freed": "10.0.0.208"}))),
                Some(Err(Undone::Refused("no".into()))),
                Some(Err(Undone::Unreachable("down".into()))),
                Some(Err(Undone::SignedOut("aged".into()))),
                Some(Err(Undone::Unknown("read it".into()))),
                Some(Err(Undone::Refused("bad-act".into()))),
                None,
                None,
                None,
            ],
            "an answer was missed, misread, or another line taken for it"
        );
    }

    /// What asking a daemon that says `lines` after reading the ask, then
    /// hangs up or holds on for `held`, comes to.
    fn asked_of(
        name: &str,
        lines: &'static [&'static str],
        held: Duration,
    ) -> Result<Value, Undone> {
        let folder =
            std::env::temp_dir().join(format!("eero-asking-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let socket = folder.join("socket");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the test's socket listens");
        let daemon = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the ask connects");
            let mut reading = BufReader::new(stream.try_clone().expect("the stream clones"));
            let mut ask = String::new();
            reading.read_line(&mut ask).expect("the ask is read");
            let mut writing = stream;
            for line in lines {
                writeln!(writing, "{line}").expect("the answer is written");
            }
            thread::sleep(held);
            ask
        });
        let answer = act_at(
            &socket,
            &json!({"act": "speedtest"}),
            Duration::from_millis(300),
        );
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
                Duration::ZERO
            ),
            Ok(json!({"id": 1, "done": true})),
            "a done answer after another line wasn't taken"
        );
        assert_eq!(
            asked_of(
                "refused",
                &[r#"{"id":1,"error":{"code":"refused","message":"taken"}}"#],
                Duration::ZERO
            ),
            Err(Undone::Refused("taken".into())),
            "a refusal wasn't said as the daemon said it"
        );
    }

    #[test]
    fn silence_after_the_ask_is_an_unknown_outcome_not_an_absent_daemon() {
        assert!(
            matches!(
                asked_of("gone", &[], Duration::ZERO),
                Err(Undone::Unknown(_))
            ),
            "a daemon gone after the ask read as one never asked"
        );
        assert!(
            matches!(
                asked_of("slow", &[], Duration::from_secs(1)),
                Err(Undone::Unknown(_))
            ),
            "a daemon slower than our patience read as one never asked"
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
