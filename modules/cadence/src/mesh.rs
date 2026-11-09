//! The sessions' mesh, as the pi harness keeps it on disk: what each
//! session says of itself (presence), the messages between them, and the
//! markers of parked ones. cadence reads them to know whether the session
//! keeping the cycle fast is there and answering, and writes a question
//! the way the harness's own folder transport does: one JSON file per
//! message, renamed into place whole.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Where the harness keeps the mesh and its parked sessions.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub root: PathBuf,
}

/// What a session says of itself.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Presence {
    #[serde(default)]
    pub session: String,
    #[serde(default)]
    pub addresses: Vec<String>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub at: serde_json::Value,
    pub owner: Option<Owner>,
    #[serde(default, rename = "orchestratedBy")]
    pub orchestrated_by: Option<String>,
    #[serde(default)]
    pub quest: Option<String>,
    #[serde(default)]
    pub title: String,
    /// The groups it hears, an orchestrator's among them as
    /// `#orchestration/<its address>`.
    #[serde(default)]
    pub groups: Vec<String>,
    /// What it has spent and when, its last turn under `turns.last`.
    #[serde(default)]
    pub usage: serde_json::Value,
}

impl Presence {
    /// When its last turn ended, in seconds, by when it began and how long
    /// it took, if it has taken one.
    #[must_use]
    pub fn last_turn_ended(&self) -> Option<i64> {
        let last = &self.usage["turns"]["last"];
        let began: jiff::Timestamp = last["at"].as_str()?.parse().ok()?;
        let took = last["duration_ms"].as_i64().unwrap_or_default();
        Some((began.as_millisecond() + took) / 1000)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Owner {
    pub pid: Option<u32>,
    /// When its pi started, in milliseconds.
    #[serde(default, rename = "startedAt")]
    pub started_at: Option<i64>,
    /// The boot the pi ran in, as the kernel's boot id names it.
    #[serde(default, rename = "bootId")]
    pub boot_id: Option<String>,
}

/// A message, as far as cadence writes and reads one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub v: u32,
    pub id: String,
    pub at: String,
    pub from: String,
    pub to: Vec<String>,
    pub intent: String,
    pub text: String,
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<String>,
}

#[derive(Deserialize)]
struct Marker {
    #[serde(default)]
    session: String,
    #[serde(default)]
    addresses: Vec<String>,
    wake: Option<PathBuf>,
}

impl Mesh {
    fn messages(&self) -> PathBuf {
        self.root.join("mesh-workflow/messages")
    }

    fn presences(&self) -> PathBuf {
        self.root.join("mesh-workflow/presence")
    }

    fn parked(&self) -> PathBuf {
        self.root.join("park-workflow/parked")
    }

    /// The newest presence going by `address`, if any session does.
    #[must_use]
    pub fn presence(&self, address: &str) -> Option<Presence> {
        self.all()
            .into_iter()
            .filter(|p| p.addresses.iter().any(|a| a == address))
            .max_by_key(|p| p.at.to_string())
    }

    /// Every presence, living or left behind.
    #[must_use]
    pub fn all(&self) -> Vec<Presence> {
        json_files(&self.presences())
            .filter_map(|path| read_json::<Presence>(&path))
            .collect()
    }

    /// Whether `session` is parked, its holder waiting to start it again.
    #[must_use]
    pub fn is_parked(&self, session: &str) -> bool {
        !session.is_empty() && self.parked().join(format!("{session}.json")).exists()
    }

    /// Writes `message` into the messages folder whole, then asks the holder
    /// of any parked session going by an address it is sent to to start it
    /// again, as a sender in the harness does for a question.
    ///
    /// # Errors
    ///
    /// When the message can't be written.
    pub fn send(&self, message: &Message) -> io::Result<()> {
        let folder = self.messages();
        let staged = folder.join(format!(".{}.json.tmp", message.id));
        fs::write(&staged, serde_json::to_vec(message)?)?;
        fs::rename(&staged, folder.join(format!("{}.json", message.id)))?;
        for marker in json_files(&self.parked()).filter_map(|p| read_json::<Marker>(&p)) {
            let reached = marker
                .addresses
                .iter()
                .any(|a| message.to.contains(&format!("@{a}")));
            if let (true, Some(wake)) = (reached, marker.wake) {
                let record = serde_json::json!({
                    "session": marker.session,
                    "messageId": message.id,
                    "at": message.at,
                });
                // Unwoken, it reads the message at its next start all the same.
                let _ = fs::write(wake, format!("{record}\n"));
            }
        }
        Ok(())
    }

    /// Every message `address` sent, oldest first by id.
    #[must_use]
    pub fn sent_by(&self, address: &str) -> Vec<Message> {
        let mark = format!("-{address}-");
        let mut sent: Vec<Message> = json_files(&self.messages())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().contains(&mark))
            })
            .filter_map(|p| read_json::<Message>(&p))
            .filter(|m| m.from == address)
            .collect();
        sent.sort_by(|a, b| a.id.cmp(&b.id));
        sent
    }
}

/// Whether the session a presence describes is there: running, or parked
/// with its holder waiting to start it again.
#[must_use]
pub fn alive(presence: Option<&Presence>, process_exists: impl Fn(u32) -> bool) -> bool {
    presence.is_some_and(|p| {
        p.state == "parked"
            || p.owner
                .as_ref()
                .and_then(|o| o.pid)
                .is_some_and(process_exists)
    })
}

/// The stamp a message's id starts with, in milliseconds, as the harness
/// stamps them: `<ms>-<from>-<pid>-<hex>`.
#[must_use]
pub fn stamped_at(id: &str) -> Option<i64> {
    id.split('-').next()?.parse().ok()
}

fn json_files(folder: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh() -> (tempfile::TempDir, Mesh) {
        let dir = tempfile::tempdir().unwrap();
        for folder in [
            "mesh-workflow/messages",
            "mesh-workflow/presence",
            "park-workflow/parked",
        ] {
            fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        let mesh = Mesh {
            root: dir.path().to_owned(),
        };
        (dir, mesh)
    }

    fn question(id: &str, to: &str) -> Message {
        Message {
            v: 1,
            id: id.into(),
            at: "2026-11-07T02:00:00.000Z".into(),
            from: "CADENCE".into(),
            to: vec![format!("@{to}")],
            intent: "question".into(),
            text: "a target slipped".into(),
            pid: 1,
            re: None,
        }
    }

    #[test]
    fn a_sent_message_lands_whole_under_its_id() {
        let (_dir, mesh) = mesh();
        mesh.send(&question("1-CADENCE-1-aa", "WATCHER")).unwrap();
        let landed = mesh.root.join("mesh-workflow/messages/1-CADENCE-1-aa.json");
        let read: Message = serde_json::from_slice(&fs::read(landed).unwrap()).unwrap();
        assert_eq!(read, question("1-CADENCE-1-aa", "WATCHER"));
    }

    #[test]
    fn a_question_to_a_parked_session_wakes_it() {
        let (dir, mesh) = mesh();
        let wake = dir.path().join("wake");
        let marker = serde_json::json!({"session": "s1", "addresses": ["WATCHER"], "wake": wake});
        fs::write(
            mesh.root.join("park-workflow/parked/s1.json"),
            marker.to_string(),
        )
        .unwrap();
        mesh.send(&question("1-CADENCE-1-aa", "WATCHER")).unwrap();
        assert!(fs::read_to_string(wake).unwrap().contains("1-CADENCE-1-aa"));
    }

    #[test]
    fn the_newest_presence_by_an_address_is_found() {
        let (_dir, mesh) = mesh();
        let folder = mesh.root.join("mesh-workflow/presence");
        let one =
            r#"{"session":"old","addresses":["WATCHER"],"state":"listening","at":"2026-11-06"}"#;
        let two =
            r#"{"session":"new","addresses":["WATCHER","AB12"],"state":"busy","at":"2026-11-07"}"#;
        fs::write(folder.join("a.json"), one).unwrap();
        fs::write(folder.join("b.json"), two).unwrap();
        assert_eq!(mesh.presence("WATCHER").unwrap().session, "new");
        assert!(mesh.presence("NOBODY").is_none());
    }

    #[test]
    fn a_parked_session_is_alive_and_a_gone_process_isnt() {
        let parked = Presence {
            session: "s".into(),
            addresses: vec![],
            state: "parked".into(),
            at: serde_json::Value::Null,
            owner: None,
            orchestrated_by: None,
            quest: None,
            title: String::new(),
            groups: vec![],
            usage: serde_json::Value::Null,
        };
        assert!(alive(Some(&parked), |_| false));
        let gone = Presence {
            state: "listening".into(),
            owner: Some(Owner {
                pid: Some(7),
                started_at: None,
                boot_id: None,
            }),
            ..parked
        };
        assert!(!alive(Some(&gone), |_| false));
        assert!(alive(Some(&gone), |pid| pid == 7));
        assert!(!alive(None, |_| true));
    }

    #[test]
    fn what_an_address_sent_is_read_back_in_order() {
        let (_dir, mesh) = mesh();
        let mut answer = question("2-WATCHER-9-bb", "CADENCE");
        answer.from = "WATCHER".into();
        answer.re = Some("1-CADENCE-1-aa".into());
        mesh.send(&answer).unwrap();
        mesh.send(&question("1-CADENCE-1-aa", "WATCHER")).unwrap();
        let sent = mesh.sent_by("WATCHER");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].re.as_deref(), Some("1-CADENCE-1-aa"));
        assert_eq!(stamped_at(&sent[0].id), Some(2));
    }
}
