//! `system serve`: the system daemon. It holds the system's record ready
//! on the socket systemd holds for it, or on a Mac one it listens on
//! itself, in the shell's daemon interface (docs/daemons.md), so the
//! launcher's home rows, the local sites' system page and the machine's
//! mycelium node draw from what it holds rather than each asking the four
//! owners themselves. The record is split by kind, the whole's summary, a
//! record a part and one a machine, so a watch is told only what moved.
//!
//! It asks the owners on a pace rather than following them: three of the
//! four have no feed of their own yet, and together they answer in well
//! under a second, so a pace costs little and keeps one way of reading.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Shared};
use serde_json::{Map, Value, json};
use system::kept::Notes;
use system::record::{Health, System};
use terminal_kit::{Exit, Failure, flags};

/// The daemon as its clients know it: the whole's one summary, a record
/// for each part, and one for each machine and each builder the machines
/// send builds to.
const DOMAIN: Domain = Domain {
    name: "system",
    kinds: &["summary", "part", "machine", "builder"],
    sole: &["summary"],
    named_only: &[],
};

/// How often the owners are asked. The four together answer in about a
/// tenth of a second, and a machine's state moves on the order of tens of
/// seconds, so this keeps a page within a breath of now for little. The
/// journal, asked for the firmware's error records in one to two seconds,
/// is asked once a boot, again only when it couldn't be read, and only for
/// a day after the boot (power.rs).
const PACE: Duration = Duration::from_secs(15);

/// The flags its unit may hand it.
const FLAGS: [&str; 1] = ["--socket"];

/// Holds the system's record, read again each pace, for as long as it
/// runs.
pub fn run(args: &[String]) -> Result<(), Failure> {
    let socket = socket(args)?;
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let listener = serve::listen(&socket).map_err(|why| {
        let socket = socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    loop {
        let (system, notes) = crate::read();
        shared.change(system.records(), sources(&system, &notes));
        thread::sleep(PACE);
    }
}

/// Where it serves: systemd's socket when it hands one, else the one
/// named, else the runtime folder's, as every client looks there.
fn socket(args: &[String]) -> Result<PathBuf, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("system --help");
    match args {
        [] => std::env::var_os("XDG_RUNTIME_DIR")
            .map(|folder| PathBuf::from(folder).join("daemons/system.sock"))
            .ok_or_else(|| {
                wrongly("system serve needs --socket, with no runtime folder".to_owned())
            }),
        [flag, path] if flag == "--socket" => Ok(PathBuf::from(path)),
        [flag] if flag == "--socket" => Err(wrongly("--socket needs a value".to_owned())),
        [flag, ..] if flag.starts_with('-') => Err(flags::unknown("option", flag, &FLAGS)),
        [word, ..] => Err(wrongly(format!("system serve takes no word like {word}"))),
    }
}

/// How each owner stands, by its part: answering, or not, with why. One
/// whose part stands on its last kept answer didn't answer, and says so
/// with the note saying when that answer was read.
fn sources(system: &System, notes: &Notes) -> Value {
    let sources: Map<String, Value> = system
        .parts
        .iter()
        .map(|part| {
            let said = match notes.of(part.name) {
                Some(note) => json!({"state": "not answering", "reason": note}),
                None if part.health == Health::Unread => {
                    json!({"state": "not answering", "reason": part.says})
                }
                None => json!("answering"),
            };
            (part.name.to_owned(), said)
        })
        .collect();
    Value::Object(sources)
}

#[cfg(test)]
mod tests {
    use super::*;
    use system::parts;

    fn words(said: &[&str]) -> Vec<String> {
        said.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn it_serves_on_the_socket_named() {
        let socket = socket(&words(&["--socket", "/run/user/1000/daemons/system.sock"]));
        assert_eq!(
            socket.ok(),
            Some(PathBuf::from("/run/user/1000/daemons/system.sock"))
        );
    }

    #[test]
    fn words_it_doesnt_know_are_refused_as_usage() {
        for said in [&["--socket"][..], &["--sock", "x"], &["extra"]] {
            let refused = socket(&words(said)).err();
            assert!(
                refused.is_some_and(|failure| failure.exit == Exit::Usage),
                "{said:?} is a usage error"
            );
        }
    }

    #[test]
    fn every_record_is_of_a_kind_its_clients_may_watch() {
        let said = json!({"Ok": {
            "Machines": [{"name": "laptop", "state": "ready", "disk_band": "healthy",
                           "room": {"over": []}}],
            "Builders": [{"name": "gull", "state": "not answering"}],
        }});
        let none: parts::Answer = Err(String::new());
        let read = System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-15T19:00:00Z".to_owned(),
            parts: vec![parts::machines(&Ok(said), &none, None, None)],
        };
        let kinds: Vec<String> = read
            .records()
            .iter()
            .map(|record| record["kind"].as_str().unwrap_or("").to_owned())
            .collect();
        assert!(
            kinds.iter().any(|kind| kind == "builder"),
            "no builder's record was made to check: {kinds:?}"
        );
        for kind in &kinds {
            assert!(
                DOMAIN.kinds.contains(&kind.as_str()),
                "a {kind} record is of a kind no client may watch"
            );
        }
    }

    #[test]
    fn an_owner_not_read_is_said_as_not_answering_with_why() {
        let gone: parts::Answer = Err("net isn't on this machine".to_owned());
        let read = System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-15T19:00:00Z".to_owned(),
            parts: vec![
                parts::network(&gone),
                parts::code(&Ok(json!({"repositories": []}))),
            ],
        };
        assert_eq!(
            sources(&read, &Notes::default()),
            json!({
                "network": {"state": "not answering", "reason": "net isn't on this machine"},
                "code": "answering",
            })
        );
    }

    #[test]
    fn an_owner_standing_on_its_kept_answer_is_said_as_not_answering_with_when() {
        let read = System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-15T19:00:00Z".to_owned(),
            parts: vec![parts::code(&Ok(json!({"repositories": []})))],
        };
        let note = "as read at 18:54; just now repos --json gave no answer in 20 s";
        let notes = Notes {
            repos: Some(note.to_owned()),
            ..Notes::default()
        };
        assert_eq!(
            sources(&read, &notes),
            json!({"code": {"state": "not answering", "reason": note}}),
            "an owner that missed its read isn't said to answer while its last answer stands"
        );
    }
}
