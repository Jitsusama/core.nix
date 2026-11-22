//! Asking the daemon to do an act, as `eero devices <MAC> reserve` does:
//! the command never opens eero's session itself, so a change to the home
//! network goes only through the daemon's socket, which takes only the
//! acts acts.rs names and logs each with who asked (changes.rs).

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde_json::{Value, json};

use crate::watch;

/// How long an act may take, since eero reads each change back after it.
const PATIENCE: Duration = Duration::from_secs(60);

/// Why an act wasn't done.
#[derive(Debug, PartialEq, Eq)]
pub enum Undone {
    /// The daemon isn't there to ask.
    Absent(String),
    /// The daemon or eero said no, and why.
    Refused(String),
}

/// Asks the daemon for `act`, an act's fields as acts.rs reads them, and
/// waits for its answer.
pub fn act(act: &Value) -> Result<(), Undone> {
    let socket = watch::socket().map_err(Undone::Absent)?;
    let lost = |error: io::Error| Undone::Absent(format!("{}: {error}", socket.display()));
    let mut stream = UnixStream::connect(&socket).map_err(lost)?;
    stream.set_read_timeout(Some(PATIENCE)).map_err(lost)?;
    let mut asked = act.clone();
    asked["id"] = json!(1);
    asked["ask"] = json!("act");
    writeln!(stream, "{asked}").map_err(lost)?;
    for line in BufReader::new(stream).lines() {
        if let Some(answer) = answered(&line.map_err(lost)?) {
            return answer;
        }
    }
    Err(Undone::Absent("the daemon went before it answered".into()))
}

/// The daemon's answer to our ask, if `line` is it, rather than anything
/// else it tells a listener.
fn answered(line: &str) -> Option<Result<(), Undone>> {
    let said: Value = serde_json::from_str(line).ok()?;
    if said["id"] != json!(1) {
        return None;
    }
    if said["done"] == json!(true) {
        return Some(Ok(()));
    }
    let why = said["error"]["message"]
        .as_str()
        .or_else(|| said["error"]["code"].as_str())
        .unwrap_or("the daemon answered without saying why");
    Some(Err(Undone::Refused(why.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_answer_to_our_ask_ends_the_wait() {
        let refused = r#"{"id":1,"error":{"code":"refused","message":"no"}}"#;
        assert_eq!(
            [
                answered(r#"{"id":1,"done":true}"#),
                answered(refused),
                answered(r#"{"id":1,"error":{"code":"bad-act"}}"#),
                answered(r#"{"id":2,"done":true}"#),
                answered(r#"{"change":"devices"}"#),
                answered("not json"),
            ],
            [
                Some(Ok(())),
                Some(Err(Undone::Refused("no".into()))),
                Some(Err(Undone::Refused("bad-act".into()))),
                None,
                None,
                None,
            ],
            "an answer was missed, misread, or another line taken for it"
        );
    }
}
