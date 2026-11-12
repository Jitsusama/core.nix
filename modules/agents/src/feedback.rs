//! What Joel says of a landing, sent where it came from: a few words
//! typed on a landing's row go, as a finding on the mesh, to the quest
//! that landed it, or to its session where it names no quest, and are
//! kept, a line each, for the daily loop that reads how landings were
//! taken (80ECO2, 2026-11-11). His feedback reaches the session that can
//! act on it, rather than waiting for him to find it (WCJQSG,
//! 2026-11-11: "make it live, let me play and give feedback").

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

/// Each landing Joel may speak of, as Landed serves it, by its record's
/// key, kept as the rows are made so an ask finds what it's about.
#[derive(Clone, Default)]
pub struct Landings(Arc<Mutex<HashMap<String, Value>>>);

impl Landings {
    /// Holds the landings among `records` from now on.
    pub fn hold<'a>(&self, records: impl Iterator<Item = &'a Value>) {
        let held = records
            .filter(|record| record["kind"] == "landed")
            .filter_map(|record| {
                Some((record["key"].as_str()?.to_owned(), record["landed"].clone()))
            })
            .collect();
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = held;
    }

    /// The landing whose record is `key`.
    fn get(&self, key: &str) -> Option<Value> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.get(key).cloned()
    }
}

/// How long the node may take to send it, since Joel waits on the foot
/// saying it went, and the node answers in milliseconds.
const PATIENCE: Duration = Duration::from_secs(5);

/// The group a landing's feedback goes to: its quest's, by the quest id's
/// last part, else its session's own.
pub fn group_of(landing: &Value) -> Option<String> {
    let quest = landing["quest"].as_str().unwrap_or("");
    let short = quest.rsplit('-').next().filter(|short| !short.is_empty());
    if let Some(short) = short {
        return Some(format!("quest.{}", short.to_lowercase()));
    }
    let address = landing["session"]["address"].as_str().unwrap_or("");
    (!address.is_empty()).then(|| format!("peer.{}", address.to_lowercase()))
}

/// The finding's words: what Joel said, of which landing, with the line
/// it asked him to try, so whoever hears it needn't look the landing up.
pub fn text_of(landing: &Value, words: &str) -> String {
    let named = landing["ref"].as_str().unwrap_or("");
    let what = landing["what"].as_str().unwrap_or("");
    let mut text = format!("Joel, from the launcher, on {named} ({what}): \"{words}\"");
    let person = landing["person"].as_str().unwrap_or("");
    if !person.is_empty() {
        text.push_str(&format!(" His line for it said: \"{person}\""));
    }
    text
}

/// Sends `text` to `group` as Joel's finding through the node at
/// `socket`, answering the message's id.
fn sent(socket: &Path, group: &str, text: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    let draft = json!({"Send": {"from": "joel", "groups": [group], "intent": "finding",
        "text": text}});
    writeln!(stream, "{draft}")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let answer: Value = serde_json::from_str(&line).map_err(io::Error::other)?;
    if let Some(id) = answer["Ok"]["Sent"]["id"].as_str() {
        return Ok(id.to_owned());
    }
    let why = answer["Err"]
        .as_str()
        .map_or_else(|| answer["Refused"].to_string(), str::to_owned);
    Err(io::Error::other(why))
}

/// Keeps `line` at the end of the file at `path`, written once and never
/// again, so a reader reading on from where it left off misses none.
fn keep(path: &Path, line: &Value) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(format!("{line}\n").as_bytes())
}

fn refused(code: &str, why: &str) -> Value {
    json!({"error": {"code": code, "message": why}})
}

/// The answer to Joel's feedback `said` (`{landing, words}`, the landing
/// by its record's key), sent through the node at `node` and kept in the
/// file at `kept`, at `at`, an RFC 3339 instant: who it went to and the
/// message's id, or why it didn't go.
pub fn answer(
    said: &Value,
    landings: &Landings,
    node: Option<&Path>,
    kept: &Path,
    at: &str,
) -> Value {
    let words = said["words"].as_str().map(str::trim).unwrap_or("");
    if words.is_empty() {
        return refused("not-an-ask", "feedback says some words");
    }
    let key = said["landing"].as_str().unwrap_or("");
    let Some(landing) = landings.get(key) else {
        return refused("not-an-ask", &format!("no landing is {key:?}"));
    };
    let Some(group) = group_of(&landing) else {
        return refused("unsent", "the landing names no quest or session to tell");
    };
    let Some(node) = node else {
        return refused("unsent", "this machine runs no node to send it through");
    };
    let id = match sent(node, &group, &text_of(&landing, words)) {
        Ok(id) => id,
        Err(why) => return refused("unsent", &format!("the node didn't send it: {why}")),
    };
    let line = json!({"at": at, "landing": key, "ref": landing["ref"], "url": landing["url"],
        "quest": landing["quest"], "address": landing["session"]["address"],
        "line": landing["person"], "words": words, "sent": id});
    // Sent is what Joel asked for, so a file that can't be written says
    // so beside it rather than taking back what went.
    let kept = keep(kept, &line).err().map(|why| why.to_string());
    let to = group
        .split_once('.')
        .map_or("", |(_, to)| to)
        .to_uppercase();
    json!({"sent": id, "to": to, "unkept": kept})
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    fn landing(quest: &str, address: &str) -> Value {
        json!({"ref": "core.nix#7", "what": "Quiet Rows", "quest": quest,
            "person": "Open Agents and look.", "url": "https://example.org/7",
            "session": {"address": address}})
    }

    fn held(one: Value) -> Landings {
        let landings = Landings::default();
        let records = [
            json!({"key": "landed:agents:s 7 at", "kind": "landed", "landed": one}),
            json!({"key": "row:agents:SKIFF", "kind": "row"}),
        ];
        landings.hold(records.iter());
        landings
    }

    /// A node that answers one ask with `answer`, handing back what it was
    /// asked.
    fn node(folder: &Path, answer: Value) -> (std::path::PathBuf, thread::JoinHandle<String>) {
        let socket = folder.join("node.sock");
        let listener = UnixListener::bind(&socket).expect("a socket");
        let asked = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a client");
            let mut line = String::new();
            BufReader::new(stream.try_clone().expect("a clone"))
                .read_line(&mut line)
                .expect("an ask");
            writeln!(stream, "{answer}").expect("an answer");
            line
        });
        (socket, asked)
    }

    fn folder(name: &str) -> std::path::PathBuf {
        let folder =
            std::env::temp_dir().join(format!("agents-feedback-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        folder
    }

    #[test]
    fn it_goes_to_the_quest_else_the_session() {
        assert_eq!(
            (
                group_of(&landing("QEST-20261106-KP0L6A", "SKIFF")),
                group_of(&landing("", "SKIFF"))
            ),
            (
                Some("quest.kp0l6a".to_owned()),
                Some("peer.skiff".to_owned())
            ),
            "a quest's group by its id's last part, else the session's own"
        );
        assert_eq!(
            group_of(&landing("", "")),
            None,
            "one naming neither goes nowhere"
        );
    }

    #[test]
    fn it_says_what_joel_said_of_which_landing() {
        assert_eq!(
            text_of(&landing("", "SKIFF"), "too dim"),
            "Joel, from the launcher, on core.nix#7 (Quiet Rows): \"too dim\" His line for it \
             said: \"Open Agents and look.\"",
            "his words, the landing and its line"
        );
    }

    #[test]
    fn it_is_sent_as_joels_finding_and_kept_a_line_each() {
        let folder = folder("sent");
        let (socket, asked) = node(&folder, json!({"Ok": {"Sent": {"id": "m-1"}}}));
        let kept = folder.join("feedback.jsonl");
        fs::write(&kept, "{\"before\":true}\n").expect("an earlier line");
        let said = json!({"ask": "feedback", "landing": "landed:agents:s 7 at",
            "words": " too dim "});
        let answer = answer(
            &said,
            &held(landing("QEST-1-KELP", "SKIFF")),
            Some(&socket),
            &kept,
            "2026-11-12T08:00:00Z",
        );
        assert_eq!(answer, json!({"sent": "m-1", "to": "KELP", "unkept": null}));
        let asked: Value = serde_json::from_str(&asked.join().expect("the node")).expect("JSON");
        assert_eq!(
            asked,
            json!({"Send": {"from": "joel", "groups": ["quest.kelp"], "intent": "finding",
                "text": text_of(&landing("", ""), "too dim")}}),
            "a finding from Joel to the quest's group"
        );
        let lines: Vec<Value> = fs::read_to_string(&kept)
            .expect("the file")
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON line"))
            .collect();
        assert_eq!(
            lines,
            vec![
                json!({"before": true}),
                json!({"at": "2026-11-12T08:00:00Z",
                "landing": "landed:agents:s 7 at", "ref": "core.nix#7",
                "url": "https://example.org/7", "quest": "QEST-1-KELP", "address": "SKIFF",
                "line": "Open Agents and look.", "words": "too dim", "sent": "m-1"})
            ],
            "kept after what was there"
        );
    }

    #[test]
    fn what_cant_go_says_why_and_keeps_nothing() {
        let folder = folder("unsent");
        let (socket, asked) = node(&folder, json!({"Err": "no such group"}));
        let kept = folder.join("feedback.jsonl");
        let landings = held(landing("", "SKIFF"));
        let ask = |landing: &str, words: &str| json!({"ask": "feedback", "landing": landing, "words": words});
        let code = |answer: Value| answer["error"]["code"].clone();
        let at = "2026-11-12T08:00:00Z";
        assert_eq!(
            [
                code(answer(
                    &ask("landed:agents:s 7 at", " "),
                    &landings,
                    Some(&socket),
                    &kept,
                    at
                )),
                code(answer(
                    &ask("landed:agents:gone", "hi"),
                    &landings,
                    Some(&socket),
                    &kept,
                    at
                )),
                code(answer(
                    &ask("landed:agents:s 7 at", "hi"),
                    &landings,
                    None,
                    &kept,
                    at
                )),
                code(answer(
                    &ask("landed:agents:s 7 at", "hi"),
                    &landings,
                    Some(&socket),
                    &kept,
                    at
                )),
            ],
            [
                json!("not-an-ask"),
                json!("not-an-ask"),
                json!("unsent"),
                json!("unsent")
            ],
            "no words, no such landing, no node, and the node refusing"
        );
        asked.join().expect("the node");
        assert!(!kept.exists(), "nothing unsent is kept");
    }
}
