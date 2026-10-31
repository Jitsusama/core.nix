//! What Slack's daemon, slack-unreads, answers over its socket, in the
//! shell's daemon interface (RSCH-20261029-0E8050, "Daemons: One
//! Interface"), version 1: `hello`, `health`, `search` for Joel's words,
//! and `get` of a message a search found, with the few around it. Each ask is answered on a thread
//! of its own, so a search waiting on Slack holds up no other ask.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::search::{self, Failed};
use crate::{Account, Slack};

/// Listens on `socket`, on a thread of its own, for as long as the
/// program runs.
pub fn listen(socket: &Path, credentials: PathBuf, origin: String) {
    let listener = match bind(socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            return;
        }
    };
    let since = crate::now() * 1000;
    let asking = Arc::new(Asking {
        credentials,
        origin,
        since,
    });
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let asking = Arc::clone(&asking);
            std::thread::spawn(move || serve(&asking, stream));
        }
    });
}

/// The socket, in a folder only Joel reads, replacing one left behind.
fn bind(socket: &Path) -> std::io::Result<UnixListener> {
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

struct Asking {
    credentials: PathBuf,
    origin: String,
    since: u64,
}

/// Answers one client until it goes, each ask on a thread of its own.
fn serve(asking: &Arc<Asking>, stream: UnixStream) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let writing = Arc::new(Mutex::new(stream));
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let asking = Arc::clone(asking);
        let writing = Arc::clone(&writing);
        std::thread::spawn(move || {
            let said = answer(&asking, &line).to_string();
            let mut out = writing
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let _ = writeln!(out, "{said}");
        });
    }
}

/// What one line asks, answered.
fn answer(asking: &Asking, line: &str) -> Value {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return error(&Value::Null, "unknown-ask", "that isn't an ask");
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let account = Account::read(&asking.credentials);
    let standing = if account.is_some() {
        "answering"
    } else {
        "signed-out"
    };
    match said.get("ask").and_then(Value::as_str).unwrap_or("") {
        "hello" => json!({
            "id": id, "version": 1, "domain": "slack", "state": "answering",
            "since": asking.since, "sources": {"slack": standing},
        }),
        "health" => json!({
            "id": id, "version": 1, "state": "answering",
            "sources": {"slack": {"state": standing, "reason": ""}},
        }),
        "search" => {
            let words = said
                .get("words")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let Some(account) = account else {
                return error(&id, "signed-out", "nobody is signed in to Slack");
            };
            if words.is_empty() {
                return json!({"id": id, "records": []});
            }
            match Slack::new(asking.origin.clone()).search(&account, words) {
                Ok(found) => json!({"id": id, "records": found}),
                Err(failed) => refused(&id, &failed),
            }
        }
        "get" => {
            let Some(account) = account else {
                return error(&id, "signed-out", "nobody is signed in to Slack");
            };
            let keys = said.get("keys").and_then(Value::as_array);
            let slack = Slack::new(asking.origin.clone());
            let mut records = Vec::new();
            for key in keys.into_iter().flatten().filter_map(Value::as_str) {
                let Some((channel, ts)) = search::named(key) else {
                    continue;
                };
                match slack.around(&account, channel, ts) {
                    Ok(near) => records.push(json!({
                        "key": key, "kind": "slack-message", "around": near,
                    })),
                    Err(failed) => return refused(&id, &failed),
                }
            }
            json!({"id": id, "records": records})
        }
        other => error(&id, "unknown-ask", &format!("no ask {other:?}")),
    }
}

/// Why Slack said nothing, by the interface's codes: too busy is
/// too-many, which the shell says as "Slack is busy".
fn refused(id: &Value, failed: &Failed) -> Value {
    match failed {
        Failed::SignedOut => error(id, "signed-out", "Slack refused the sign-in"),
        Failed::Busy => error(id, "too-many", "Slack is busy"),
        Failed::Unreachable(why) => {
            eprintln!("<4>{why}");
            error(id, "refused", "Slack couldn't be reached")
        }
    }
}

fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}
