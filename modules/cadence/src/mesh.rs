//! The sessions' mesh, as the pi harness keeps it on disk: what each
//! session says of itself (presence), the messages between them, and the
//! markers of parked ones, with each pi host's roster. cadence reads
//! them to know whether the session keeping the cycle fast is there and
//! answering, and writes a question the way the harness's own folder
//! transport does: one JSON file per message, renamed into place whole.

use std::collections::{BTreeMap, BTreeSet};
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
    /// The shared pi host it runs in, when it runs in one rather than in
    /// a pi of its own.
    #[serde(default)]
    pub hosted: Option<Hosted>,
    /// What it says it is doing, its next step under `next`.
    #[serde(default)]
    pub doing: serde_json::Value,
}

/// The pi hosts' rosters, by host: the sessions each holds, or none when
/// its roster can't be read now.
#[derive(Debug, Clone, Default)]
pub struct Rosters(pub BTreeMap<String, Option<BTreeSet<String>>>);

/// How long a roster that can't be read is taken as torn mid-rewrite, in
/// seconds: three of the tick's looks, as long as a session settles.
pub const TORN_AT_MOST: i64 = 15 * 60;

impl Rosters {
    /// The hosts whose readable rosters hold `session`.
    #[must_use]
    pub fn holders(&self, session: &str) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(_, held)| held.as_ref().is_some_and(|h| h.contains(session)))
            .map(|(host, _)| host.as_str())
            .collect()
    }

    /// The hosts whose rosters can't be read now, torn mid-rewrite.
    #[must_use]
    pub fn torn(&self) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(_, held)| held.is_none())
            .map(|(host, _)| host.as_str())
            .collect()
    }

    /// Whether `host`'s roster holds `session`. One torn mid-rewrite, or
    /// a host with none, can't rule it out, so holds: a host's rosters are
    /// written whole, and a host gone for good reads as down by its unit.
    #[must_use]
    pub fn holds(&self, host: &str, session: &str) -> bool {
        self.0
            .get(host)
            .and_then(Option::as_ref)
            .is_none_or(|held| held.contains(session))
    }
}

/// The shared pi host a conversation runs in, as its presence names it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Hosted {
    pub host: String,
    pub conversation: String,
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

    /// The next step it says it has, when it names one.
    #[must_use]
    pub fn next_step(&self) -> Option<&str> {
        self.doing["next"]
            .as_str()
            .map(str::trim)
            .filter(|next| !next.is_empty())
    }

    /// Whether its process runs: a pi of its own, or the shared pi host
    /// that holds it as one of its conversations. A hosted presence's
    /// owner is the host, a node rather than a pi, so asking only whether
    /// the owner is a pi judged every moved session dead: on 10 November
    /// cadence started a second pi on a conversation a host held, and
    /// both wrote to its one log.
    #[must_use]
    pub fn runs(&self, is_pi: impl Fn(u32) -> bool, is_host: impl Fn(u32) -> bool) -> bool {
        let Some(pid) = self.owner.as_ref().and_then(|o| o.pid) else {
            return false;
        };
        is_pi(pid) || (self.host().is_some() && is_host(pid))
    }

    /// The shared pi host it says it runs in, only when that host's
    /// conversation is this session: a presence a host wrote for another
    /// conversation says nothing of this one.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        self.hosted
            .as_ref()
            .filter(|h| h.conversation == self.session)
            .map(|h| h.host.as_str())
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

    /// Each pi host's roster, by the host's name, its file's stem: what
    /// the host resumes when it starts again, whatever a presence it left
    /// behind still says. One that can't be read is torn only while it was
    /// written in the last `TORN_AT_MOST` seconds before `now`: a host
    /// writes its roster whole, so one unreadable longer is broken, and
    /// kept as torn it would keep every standalone session from reviving.
    #[must_use]
    pub fn rostered(&self, now: i64) -> Rosters {
        #[derive(Deserialize)]
        struct Roster {
            #[serde(default)]
            sessions: Vec<serde_json::Value>,
        }
        let Ok(entries) = fs::read_dir(self.root.join("host-rosters")) else {
            return Rosters::default();
        };
        let read = |path: &Path| {
            let roster: Roster = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
            let held = roster
                .sessions
                .iter()
                .filter_map(|e| e["sessionId"].as_str());
            Some(held.map(str::to_owned).collect())
        };
        let torn_now = |path: &Path| {
            fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|at| i64::try_from(at.as_secs()).ok())
                .is_some_and(|at| (0..TORN_AT_MOST).contains(&(now - at)))
        };
        Rosters(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|e| e == "json"))
                .filter_map(|path| {
                    let host = path.file_stem()?.to_str()?.to_owned();
                    match read(&path) {
                        Some(held) => Some((host, Some(held))),
                        None => torn_now(&path).then_some((host, None)),
                    }
                })
                .collect(),
        )
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

    /// Every address a session on the mesh goes by, living or left
    /// behind, upper case, as a unit named for one carries it.
    #[must_use]
    pub fn known(&self) -> std::collections::BTreeSet<String> {
        self.all()
            .into_iter()
            .flat_map(|p| p.addresses)
            .map(|a| a.to_ascii_uppercase())
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

    /// Every message `address` sent, oldest first by id. The harness files
    /// older messages into a folder for each day, so those are read too:
    /// a card filed away would otherwise look never sent.
    #[must_use]
    pub fn sent_by(&self, address: &str) -> Vec<Message> {
        let mark = format!("-{address}-");
        let folder = self.messages();
        let days: Vec<PathBuf> = fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect();
        let mut sent: Vec<Message> = json_files(&folder)
            .chain(days.iter().flat_map(|day| json_files(day)))
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

/// When each session's log under `logs` was last written, in seconds, by
/// session id. pi keeps one log a session, in a folder for its working
/// directory, named `<started>_<session>.jsonl`, and writes to it at
/// every turn.
#[must_use]
pub fn logs_written(logs: &Path) -> BTreeMap<String, i64> {
    let mut written = BTreeMap::new();
    for folder in fs::read_dir(logs)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        for log in fs::read_dir(folder.path())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
            let name = log.file_name();
            let Some(session) = name
                .to_str()
                .and_then(|name| name.strip_suffix(".jsonl"))
                .and_then(|name| name.rsplit_once('_'))
                .map(|(_, session)| session.to_owned())
            else {
                continue;
            };
            let Some(at) = log
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|at| i64::try_from(at.as_secs()).ok())
            else {
                continue;
            };
            let newest = written.entry(session).or_insert(at);
            *newest = (*newest).max(at);
        }
    }
    written
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

    #[test]
    fn a_session_s_log_says_when_it_was_last_written() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("--home-joel-src--");
        fs::create_dir_all(&folder).unwrap();
        let log = folder.join("2026-11-09T03-32-08-305Z_01a1be5c-fff0.jsonl");
        fs::write(&log, "{}\n").unwrap();
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_794_197_847);
        fs::File::options()
            .write(true)
            .open(&log)
            .unwrap()
            .set_modified(at)
            .unwrap();
        fs::write(folder.join("notes.txt"), "").unwrap();
        let written = logs_written(dir.path());
        assert_eq!(
            written.into_iter().collect::<Vec<_>>(),
            [("01a1be5c-fff0".to_owned(), 1_794_197_847)]
        );
        assert!(logs_written(&dir.path().join("none")).is_empty());
    }

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
            hosted: None,
            doing: serde_json::Value::Null,
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
    fn a_conversation_a_live_host_holds_runs_and_one_it_left_does_not() {
        // As 1H56ZJ's presence read once the background host held it.
        let presence = |hosted: serde_json::Value| -> Presence {
            serde_json::from_value(serde_json::json!({
                "session": "01a1be5c",
                "addresses": ["1H56ZJ"],
                "state": "listening",
                "owner": {"pid": 1_784_702, "host": "work-background"},
                "hosted": hosted,
            }))
            .unwrap()
        };
        let host = |pid| pid == 1_784_702;
        let no_pi = |_| false;
        let held = presence(serde_json::json!({
            "host": "work-background",
            "conversation": "01a1be5c",
        }));
        assert!(held.runs(no_pi, host), "its host runs, so it does");
        assert!(!held.runs(no_pi, |_| false), "its host is gone, so it is");
        let unhosted = presence(serde_json::Value::Null);
        assert!(
            !unhosted.runs(no_pi, host),
            "a host owning it unsaid is a reused pid, not its host"
        );
        let another = presence(serde_json::json!({
            "host": "work-background",
            "conversation": "someone-else",
        }));
        assert!(
            !another.runs(no_pi, host),
            "the host holds another conversation"
        );
        assert!(
            unhosted.runs(|pid| pid == 1_784_702, |_| false),
            "a pi of its own runs"
        );
        assert_eq!(held.host(), Some("work-background"));
        assert_eq!(unhosted.host(), None, "it says no host");
        assert_eq!(another.host(), None, "its host holds another conversation");
    }

    #[test]
    fn each_hosts_roster_is_read_by_its_name_and_a_torn_one_rules_nothing_out() {
        let (dir, mesh) = mesh();
        let rosters = dir.path().join("host-rosters");
        fs::create_dir_all(&rosters).unwrap();
        fs::write(
            rosters.join("work-background.json"),
            r#"{"v":1,"sessions":[{"sessionId":"a","cwd":"/w"},{"cwd":"/x"}]}"#,
        )
        .unwrap();
        fs::write(rosters.join("work-attended.json"), "{\"sessi").unwrap();
        fs::write(rosters.join("work-background.json.lock"), "pid 5").unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_secs()).unwrap())
            .unwrap();
        assert!(
            mesh.rostered(now + TORN_AT_MOST).torn().is_empty(),
            "a roster unreadable this long is broken, and rules nothing out"
        );
        let rostered = mesh.rostered(now);
        assert_eq!(rostered.holders("a"), ["work-background"]);
        assert!(
            rostered.holders("b").is_empty(),
            "no readable roster holds it"
        );
        assert_eq!(rostered.torn(), ["work-attended"], "a lock is no roster");
        assert!(
            rostered.holds("work-background", "a"),
            "a roster that lists it holds it"
        );
        assert!(
            !rostered.holds("work-background", "b"),
            "a roster that reads and doesn't list it doesn't hold it"
        );
        assert!(
            rostered.holds("work-attended", "b"),
            "a torn roster, mid-rewrite, can't rule it out"
        );
        assert!(
            rostered.holds("gone-host", "b"),
            "nor can a host with no roster, whose unit says whether it's down"
        );
        fs::write(
            rosters.join("work-background.json"),
            r#"{"sessions":[{"sessionId":7},{"sessionId":"a"}]}"#,
        )
        .unwrap();
        assert!(
            mesh.rostered(now).holds("work-background", "a")
                && !mesh.rostered(now).holds("work-background", "7"),
            "one badly typed entry doesn't drop the rest"
        );
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

    #[test]
    fn what_an_address_sent_is_read_from_the_day_folders_too() {
        let (_dir, mesh) = mesh();
        let mut card = question("1-WATCHER-9-aa", "CADENCE");
        card.from = "WATCHER".into();
        mesh.send(&card).unwrap();
        let day = mesh.messages().join("2026-11-11");
        fs::create_dir_all(&day).unwrap();
        fs::rename(
            mesh.messages().join("1-WATCHER-9-aa.json"),
            day.join("1-WATCHER-9-aa.json"),
        )
        .unwrap();
        card.id = "2-WATCHER-9-bb".into();
        mesh.send(&card).unwrap();
        let ids: Vec<String> = mesh.sent_by("WATCHER").into_iter().map(|m| m.id).collect();
        assert_eq!(
            ids,
            ["1-WATCHER-9-aa", "2-WATCHER-9-bb"],
            "a message filed into its day's folder is still read, in order"
        );
    }
}
