//! What the shell asks spend-watch to do over its socket, of a line the
//! collection holds: run what the line offers, or mark its finding
//! expected. The shell spawns nothing for an act (Y2YBTS's data-layer
//! rule, 2026-11-10), so the daemon that made the line does it.
//!
//! An act names its line by key and never says what to run, so the socket
//! runs only what a pass put in a line.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// What one act asks done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Brings a session's window forward, as `spend-watch go` does.
    Go(String),
    /// Starts a program the line offers, such as a feed's sign-in.
    Launch(Vec<String>),
    /// Marks a finding expected, by its key.
    Expect(String),
}

/// An act refused before it's done, by the interface's codes.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

fn refused(code: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        code,
        message: message.into(),
    }
}

/// Does an act, saying why where it couldn't be done.
pub type Doer = Box<dyn Fn(Act) -> Result<(), String> + Send + Sync>;

/// What an act asks of the line it names, held as a pass told it, or none.
pub fn asked(act: &str, key: &str, line: Option<&Value>) -> Result<Act, Refusal> {
    let line = line.ok_or_else(|| refused("no-line", format!("no line {key} here")))?;
    let words = |field: &str| -> Vec<String> {
        let said = line[field]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        said.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    match act {
        "run" => {
            let run = words("run");
            match run.as_slice() {
                [] => Err(refused("bad-act", format!("{key} has nothing to run"))),
                [program, go, session] if program == "spend-watch" && go == "go" => {
                    Ok(Act::Go(session.clone()))
                }
                _ => Ok(Act::Launch(run)),
            }
        }
        // The line's expect is the command line's own ask, `spend-watch
        // findings expect <key>`, so its last word is the finding's key.
        "expect" => match words("expect").last() {
            Some(finding) => Ok(Act::Expect(finding.clone())),
            None => Err(refused("bad-act", format!("{key} is expected already"))),
        },
        _ => Err(refused("unknown-act", format!("no act {act:?} here"))),
    }
}

/// How long niri has to say it started a program.
const NIRI_PATIENCE: Duration = Duration::from_secs(2);

/// Starts a program through niri's own socket, as the launcher does, so it
/// runs in Joel's session rather than under spend-watch's unit, which a
/// restart would take it down with.
pub fn launch(command: &[String]) -> Result<(), String> {
    let socket = std::env::var_os("NIRI_SOCKET").ok_or("niri isn't running")?;
    let mut stream = UnixStream::connect(&socket).map_err(|error| format!("niri: {error}"))?;
    stream
        .set_read_timeout(Some(NIRI_PATIENCE))
        .map_err(|error| error.to_string())?;
    let asked = serde_json::json!({"Action": {"Spawn": {"command": command}}});
    writeln!(stream, "{asked}").map_err(|error| format!("niri: {error}"))?;
    let mut said = String::new();
    BufReader::new(&stream)
        .read_line(&mut said)
        .map_err(|error| format!("niri: {error}"))?;
    match serde_json::from_str::<Value>(&said) {
        Ok(answer) if answer.get("Ok").is_some() => Ok(()),
        _ => Err(format!("niri said {}", said.trim())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::Hub;
    use serde_json::json;
    use std::net::Shutdown;
    use std::sync::{Arc, Mutex};

    fn burn() -> Value {
        json!({
            "key": "finding:burn:s1",
            "run": ["spend-watch", "go", "s1"],
            "expect": ["spend-watch", "findings", "expect", "burn:s1"]
        })
    }

    #[test]
    fn an_act_does_only_what_its_line_offers() {
        let key = "finding:burn:s1";
        assert_eq!(
            asked("run", key, Some(&burn())),
            Ok(Act::Go("s1".into())),
            "going to a session should be done in the daemon"
        );
        assert_eq!(
            asked("expect", key, Some(&burn())),
            Ok(Act::Expect("burn:s1".into())),
            "marking a finding expected should name the finding"
        );
        let login = ["kitty", "--hold", "x", "login"];
        let sign_in = json!({"key": "signin:x", "run": login});
        assert_eq!(
            asked("run", "signin:x", Some(&sign_in)),
            Ok(Act::Launch(login.map(str::to_owned).to_vec())),
            "any other run should be started as a program"
        );
        let code = |act: &str, line: Option<&Value>| asked(act, key, line).map_err(|no| no.code);
        assert_eq!(code("run", None), Err("no-line"));
        assert_eq!(code("expect", Some(&sign_in)), Err("bad-act"));
        assert_eq!(code("run", Some(&json!({"key": key}))), Err("bad-act"));
        assert_eq!(code("forget", Some(&burn())), Err("unknown-act"));
    }

    #[test]
    fn the_socket_does_an_act_of_a_line_it_holds() {
        let hub = Arc::new(Hub::new(0));
        let written = json!({"v": crate::collection::VERSION, "findings": [burn()]});
        hub.seed(&written.to_string());
        let done = Arc::new(Mutex::new(Vec::new()));
        let doing = Arc::clone(&done);
        hub.doing(Box::new(move |act| {
            doing.lock().expect("the acts done").push(act);
            Ok(())
        }));
        let (mut ours, theirs) = UnixStream::pair().expect("a pair of sockets");
        let reading = BufReader::new(theirs.try_clone().expect("a reading half"));
        let serving = Arc::clone(&hub);
        let first = r#"{"id":1,"ask":"act","act":"run","line":"finding:burn:s1"}"#;
        let served = std::thread::spawn(move || serving.serve(first, reading, theirs));
        let second = r#"{"id":2,"ask":"act","act":"run","line":"finding:gone"}"#;
        writeln!(ours, "{second}").expect("asked");
        ours.shutdown(Shutdown::Write).expect("done asking");
        let answers: Vec<Value> = BufReader::new(&ours)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();
        served.join().expect("served");
        assert_eq!(
            answers,
            [
                json!({"id": 1, "done": true}),
                json!({
                    "id": 2,
                    "error": {"code": "no-line", "message": "no line finding:gone here"}
                })
            ],
            "a line held should be acted on, and one not held refused"
        );
        assert_eq!(
            *done.lock().expect("the acts done"),
            [Act::Go("s1".into())],
            "the act should be what the line offers"
        );
    }
}
