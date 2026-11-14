//! What Joel says of a landing, sent where it came from: a few words
//! typed on a landing's row go, as a talk, to the session that landed it
//! while it runs or is parked on any machine, else to whoever is on its
//! quest or orchestrates it, and are kept, a line each, for the daily
//! loop that reads how landings were taken (80ECO2, 2026-11-11). His
//! feedback reaches the session that can act on it, rather than waiting
//! for him to find it (WCJQSG, 2026-11-11: "make it live, let me play and
//! give feedback").
//!
//! A talk rather than a finding, since the harness tells every peer a
//! finding wants no reply, so his words went into a void (Joel,
//! 2026-11-13 19:30: "saying to an agent feels like the void"); a talk
//! is answered, and how it went and the answer are served on the
//! landing's drawer as `said` (`told`). Only with nobody to hear it does
//! it go as a finding to the quest's group, where a session coming to
//! the quest reads it.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::acts;
use crate::row::Words;
use crate::talks::Talk;
use crate::tell::{Heard, Routes};

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
        let _ = write!(text, " His line for it said: \"{person}\"");
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

/// Who hears Joel's words on `landing`: the session that landed it while
/// `routes` can reach it, else whoever is on its quest or orchestrates
/// it, never the void, which a landing naming no quest would reach.
fn hearer(landing: &Value, routes: &Routes) -> Option<Heard> {
    let address = landing["session"]["address"].as_str().unwrap_or("");
    let quest = landing["quest"].as_str().unwrap_or("").trim();
    (!address.is_empty())
        .then(|| routes.by_address(address))
        .flatten()
        .or_else(|| (!quest.is_empty()).then(|| routes.heard(quest)).flatten())
}

/// Where Joel's words go: as a talk to a session that hears them, or with
/// nobody to, as a finding to the quest's group.
enum Goes {
    Talk(Heard),
    Finding(String),
}

/// Says `text` to the session at `address` as Joel's talk, through the
/// node at `node`, as Talk does (acts.rs), answering the talk's id, by
/// which its records say how it went.
fn talked(node: &Path, address: &str, text: &str, now_ms: u128) -> Result<String, String> {
    let talk = json!({"ask": "talk", "address": address, "text": text});
    let steer = acts::steer_of(&talk, now_ms).ok_or("a talk is an act")??;
    let id = steer["steer"]["id"].as_str().unwrap_or("").to_owned();
    let answer = acts::done(Ok(steer), node);
    match answer["error"]["message"].as_str() {
        Some(why) => Err(why.to_owned()),
        None => Ok(id),
    }
}

/// The answer to Joel's feedback `said` (`{landing, words}`, the landing
/// by its record's key), sent through the node at `node` and kept in the
/// file at `kept`, at `now_ms`, in milliseconds since the epoch, with the
/// line kept, which the drawer's `said` is made from. The answer says how
/// it went (`by`: `talk` or `finding`), who it went to and the talk's or
/// message's id, or why it didn't go. Who it went to is the session's
/// name, as `routes` names it for a talk and the landing does for a
/// finding, since Joel reads it; it's empty for a session that has ended.
/// Why it didn't go is a code the shell words: `no-one`, `no-mesh`, or
/// `refused` with the node's reason.
pub fn answer(
    said: &Value,
    landings: &Landings,
    routes: &Routes,
    node: Option<&Path>,
    kept: &Path,
    now_ms: u128,
) -> (Value, Option<Value>) {
    let words = said["words"].as_str().map_or("", str::trim);
    if words.is_empty() {
        return (refused("not-an-ask", "feedback says some words"), None);
    }
    let key = said["landing"].as_str().unwrap_or("");
    let Some(landing) = landings.get(key) else {
        return (
            refused("not-an-ask", &format!("no landing is {key:?}")),
            None,
        );
    };
    let goes = match (hearer(&landing, routes), group_of(&landing)) {
        (Some(heard), _) => Goes::Talk(heard),
        (None, Some(group)) => Goes::Finding(group),
        (None, None) => {
            let why = "the landing names no quest or session to tell";
            return (refused("no-one", why), None);
        }
    };
    let Some(node) = node else {
        return (
            refused("no-mesh", "this machine runs no node to send it through"),
            None,
        );
    };
    let text = text_of(&landing, words);
    let went = match goes {
        Goes::Talk(heard) => {
            talked(node, &heard.to, &text, now_ms).map(|id| ("talk", id, heard.to, heard.name))
        }
        Goes::Finding(group) => sent(node, &group, &text)
            .map(|id| {
                let title = landing["session"]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned();
                ("finding", id, group, title)
            })
            .map_err(|why| why.to_string()),
    };
    let (by, id, heard, to) = match went {
        Ok(went) => went,
        Err(why) => return (refused("refused", &why), None),
    };
    let line = json!({"at": u64::try_from(now_ms / 1000).unwrap_or(0), "landing": key,
        "ref": landing["ref"], "url": landing["url"], "quest": landing["quest"],
        "address": landing["session"]["address"], "line": landing["person"], "words": words,
        "by": by, "sent": id, "heard": heard, "to": to});
    // Sent is what Joel asked for, so a file that can't be written says
    // so beside it rather than taking back what went.
    let unkept = keep(kept, &line).err().map(|why| why.to_string());
    let answer = json!({"by": by, "sent": id, "to": to, "unkept": unkept});
    (answer, Some(line))
}

/// The lines kept in the file at `path` of words sent as talks, the ones
/// a landing's drawer says how they went, read as the daemon starts.
pub fn kept(path: &Path) -> Vec<Value> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["by"] == "talk")
        .collect()
}

/// What Joel said of each landing among `records` as talks, as `lines`
/// keep them, served on its drawer as `said`, newest first: his words,
/// when in milliseconds, who heard them, how it went in Talks' own word
/// as `talks` (by id) say, `sent` before a record of it comes, and the
/// reply once it's replied. While the newest is on its way, the landing
/// says so as `saying`, the word its row's side leads with, as an Agents
/// row says a talk waiting.
pub fn told<S: std::hash::BuildHasher>(
    records: &mut [Value],
    lines: &[Value],
    talks: &HashMap<String, Talk, S>,
    words: &Words,
) {
    let mut drawers: HashMap<String, Vec<Value>> = HashMap::new();
    for record in records.iter_mut().filter(|one| one["kind"] == "landed") {
        let key = record["key"].as_str().unwrap_or("");
        let mut said: Vec<(&Value, &str, &str)> = lines
            .iter()
            .filter(|line| line["landing"] == key)
            .map(|line| {
                let talk = talks.get(line["sent"].as_str().unwrap_or(""));
                let stage = talk.map_or("", |talk| talk.stage.as_str());
                let reply = talk.map_or("", |talk| talk.reply.as_str());
                (line, if stage.is_empty() { "sent" } else { stage }, reply)
            })
            .collect();
        if said.is_empty() {
            continue;
        }
        said.sort_by_key(|(line, _, _)| std::cmp::Reverse(line["at"].as_u64().unwrap_or(0)));
        let word = |stage: &str| {
            let word = words.talk_stages.word(stage);
            if word.is_empty() { stage } else { word }.to_owned()
        };
        let (_, newest, _) = said[0];
        if !matches!(newest, "replied" | "failed" | "read" | "over") {
            record["landed"]["saying"] = json!(word(newest));
        }
        let drawer = record["landed"]["drawer"].as_str().unwrap_or("").to_owned();
        let said = said
            .into_iter()
            .map(|(line, stage, reply)| {
                json!({"words": line["words"],
                    "at": line["at"].as_u64().unwrap_or(0).saturating_mul(1000),
                    "to": line["to"], "stage": word(stage),
                    "reply": if stage == "replied" { reply } else { "" }})
            })
            .collect();
        drawers.insert(drawer, said);
    }
    for record in records
        .iter_mut()
        .filter(|one| one["kind"] == crate::landed::DRAWER)
    {
        if let Some(said) = record["key"].as_str().and_then(|key| drawers.remove(key)) {
            record["drawer"]["said"] = json!(said);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    fn landing(quest: &str, address: &str) -> Value {
        json!({"ref": "core.nix#7", "what": "Quiet Rows", "quest": quest,
            "person": "Open Agents and look.", "url": "https://example.org/7",
            "session": {"address": address, "title": "The Kelp Quest"}})
    }

    fn held(one: &Value) -> Landings {
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

    /// A node that answers a steer after its hello, as acts.rs asks it,
    /// handing back the steer.
    fn steering(folder: &Path) -> (std::path::PathBuf, thread::JoinHandle<Value>) {
        let socket = folder.join("steer.sock");
        let _ = fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket");
        let asked = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a client");
            let mut lines = BufReader::new(stream.try_clone().expect("a clone")).lines();
            let _hello = lines.next();
            let ask = lines.next().expect("a steer").expect("a line");
            let answer = json!({"id": 2, "steered": {"machine": "skiff"}});
            writeln!(stream, "{answer}").expect("an answer");
            serde_json::from_str(&ask).expect("the steer is JSON")
        });
        (socket, asked)
    }

    fn hearing(sessions: &[Value]) -> Routes {
        let routes = Routes::default();
        let quests = [json!({"id": "QEST-1-KELP", "address": "KELP", "orchestrator": "HARBOUR"})];
        routes.hold(
            sessions.iter().map(crate::session::Session),
            quests.iter(),
            std::iter::empty(),
        );
        routes
    }

    fn running(address: &str, quest: &str, title: &str) -> Value {
        json!({"standing": "running", "presence": {"addresses": [address], "owns": [quest],
            "questInfo": {"id": quest, "title": title}}})
    }

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
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
    fn it_is_said_to_the_session_that_landed_it_else_whoever_is_on_its_quest() {
        let folder = folder("talked");
        let kept = folder.join("feedback.jsonl");
        let said = json!({"ask": "feedback", "landing": "landed:agents:s 7 at",
            "words": "too dim"});
        let landings = held(&landing("QEST-1-KELP", "SKIFF"));
        let both = hearing(&[
            running("SKIFF", "QEST-1-REEF", "The Skiff"),
            running("BUOY", "QEST-1-KELP", "The Buoy"),
        ]);
        let (socket, asked) = steering(&folder);
        let (went, line) = answer(&said, &landings, &both, Some(&socket), &kept, 7_000);
        assert_eq!(
            went,
            json!({"by": "talk", "sent": "talk-7000", "to": "The Skiff", "unkept": null}),
            "a talk to the session that landed it, named as Joel reads it"
        );
        assert_eq!(
            asked.join().expect("the node"),
            json!({"id": 2, "ask": "steer", "address": "SKIFF", "steer": {
                "text": text_of(&landing("", ""), "too dim"), "id": "talk-7000",
                "from": "joel"}}),
            "steered as Talk says it, as Joel, with the landing it's about"
        );
        let line = line.expect("a line kept");
        assert_eq!(
            (&line["at"], &line["by"], &line["sent"], &line["heard"]),
            (
                &json!(7),
                &json!("talk"),
                &json!("talk-7000"),
                &json!("SKIFF")
            ),
            "kept with the talk's id, by which its drawer says how it went"
        );
        assert_eq!(
            super::kept(&kept),
            vec![line],
            "read back as the daemon starts"
        );
        let (socket, asked) = steering(&folder);
        let quest = hearing(&[running("BUOY", "QEST-1-KELP", "The Buoy")]);
        let (went, _) = answer(&said, &landings, &quest, Some(&socket), &kept, 8_000);
        assert_eq!(
            (
                &went["by"],
                &went["to"],
                &asked.join().expect("the node")["address"]
            ),
            (&json!("talk"), &json!("The Buoy"), &json!("BUOY")),
            "with its session gone, whoever is on its quest hears it"
        );
    }

    #[test]
    fn the_drawer_says_how_each_talk_went_and_the_row_while_its_on_its_way() {
        let records = || {
            vec![
                json!({"key": "landed:agents:k", "kind": "landed",
                    "landed": {"drawer": "landed-drawer:agents:k"}}),
                json!({"key": "landed-drawer:agents:k", "kind": "landed-drawer",
                    "drawer": {"title": "Quiet Rows"}}),
                json!({"key": "landed:agents:other", "kind": "landed",
                    "landed": {"drawer": "landed-drawer:agents:other"}}),
            ]
        };
        let line = |at: u64, words: &str| {
            json!({"at": at, "landing": "landed:agents:k", "words": words, "by": "talk",
                "sent": format!("talk-{at}"), "heard": "SKIFF", "to": "The Skiff"})
        };
        let lines = [line(5, "too dim"), line(9, "better")];
        let talk = |id: &str, stage: &str, reply: &str| {
            let talk = Talk {
                id: id.to_owned(),
                stage: stage.to_owned(),
                reply: reply.to_owned(),
                ..Talk::default()
            };
            (id.to_owned(), talk)
        };
        let words = words();
        let mut said = records();
        let talks: HashMap<String, Talk> = [talk("talk-5", "replied", "Brightened.")].into();
        told(&mut said, &lines, &talks, &words);
        assert_eq!(
            said[1]["drawer"]["said"],
            json!([
                {"words": "better", "at": 9000, "to": "The Skiff", "stage": "sent",
                    "reply": ""},
                {"words": "too dim", "at": 5000, "to": "The Skiff", "stage": "replied",
                    "reply": "Brightened."}
            ]),
            "newest first, sent before a record of it comes, the reply once replied"
        );
        assert_eq!(
            said[0]["landed"]["saying"], "sent",
            "its row says it's on its way"
        );
        assert!(
            said[2]["landed"].get("saying").is_none(),
            "a landing nothing was said of says nothing"
        );
        let mut said = records();
        let talks: HashMap<String, Talk> = [talk("talk-9", "failed", "")].into();
        told(&mut said, &lines, &talks, &words);
        assert_eq!(
            (
                said[0]["landed"].get("saying"),
                &said[1]["drawer"]["said"][0]["stage"]
            ),
            (None, &json!("not delivered")),
            "one that went no further says so in its drawer, its row back as it was"
        );
        let talks: HashMap<String, Talk> = [talk("talk-9", "replying", "Half")].into();
        let mut said = records();
        told(&mut said, &lines, &talks, &words);
        assert_eq!(
            (
                &said[0]["landed"]["saying"],
                &said[1]["drawer"]["said"][0]["reply"]
            ),
            (&json!("replying"), &json!("")),
            "a reply half written is the talk's, not yet the drawer's"
        );
    }

    #[test]
    fn it_goes_as_a_finding_only_with_nobody_to_hear_it() {
        let folder = folder("sent");
        let (socket, asked) = node(&folder, json!({"Ok": {"Sent": {"id": "m-1"}}}));
        let kept = folder.join("feedback.jsonl");
        fs::write(&kept, "{\"before\":true}\n").expect("an earlier line");
        let said = json!({"ask": "feedback", "landing": "landed:agents:s 7 at",
            "words": " too dim "});
        let (answer, line) = answer(
            &said,
            &held(&landing("QEST-1-KELP", "SKIFF")),
            &Routes::default(),
            Some(&socket),
            &kept,
            1_794_470_400_000,
        );
        assert_eq!(
            answer,
            json!({"by": "finding", "sent": "m-1", "to": "The Kelp Quest", "unkept": null}),
            "sent to the landing's quest, named as Joel reads it"
        );
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
                json!({"at": 1_794_470_400,
                "landing": "landed:agents:s 7 at", "ref": "core.nix#7",
                "url": "https://example.org/7", "quest": "QEST-1-KELP", "address": "SKIFF",
                "line": "Open Agents and look.", "words": "too dim", "by": "finding",
                "sent": "m-1", "heard": "quest.kelp", "to": "The Kelp Quest"})
            ],
            "kept after what was there"
        );
        assert_eq!(
            line.as_ref(),
            lines.get(1),
            "the line kept is the one answered"
        );
    }

    #[test]
    fn what_cant_go_says_why_and_keeps_nothing() {
        let folder = folder("unsent");
        let (socket, asked) = node(&folder, json!({"Err": "no such group"}));
        let kept = folder.join("feedback.jsonl");
        let landings = held(&landing("", "SKIFF"));
        let ask =
            |key: &str, words: &str| json!({"ask": "feedback", "landing": key, "words": words});
        let code = |said: &Value, landings: &Landings, node: Option<&Path>| {
            let (answer, line) = answer(
                said,
                landings,
                &Routes::default(),
                node,
                &kept,
                1_794_470_400_000,
            );
            assert!(line.is_none(), "what didn't go keeps no line");
            answer["error"]["code"].clone()
        };
        assert_eq!(
            [
                code(&ask("landed:agents:s 7 at", " "), &landings, Some(&socket)),
                code(&ask("landed:agents:gone", "hi"), &landings, Some(&socket)),
                code(&ask("landed:agents:s 7 at", "hi"), &landings, None),
                code(&ask("landed:agents:s 7 at", "hi"), &landings, Some(&socket)),
            ],
            [
                json!("not-an-ask"),
                json!("not-an-ask"),
                json!("no-mesh"),
                json!("refused")
            ],
            "no words, no such landing, no node, and the node refusing"
        );
        asked.join().expect("the node");
        assert_eq!(
            code(
                &ask("landed:agents:s 7 at", "hi"),
                &held(&landing("", "")),
                Some(&socket)
            ),
            json!("no-one"),
            "a landing naming no quest or session has no one to tell"
        );
        assert!(!kept.exists(), "nothing unsent is kept");
    }
}
