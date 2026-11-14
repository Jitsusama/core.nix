//! Each session's talks with Joel as the launcher shows them under its
//! field (SessionTalks.js picked them there until this served them, due
//! Sat 21 Nov for MY3HPU): by address, the newest first, every one still
//! open, then the newest few of the rest. A talk the session hasn't taken
//! yet has no record, since its first line is written as it's taken
//! (the harness's talks.ts), so until then it's its steer, waiting in the
//! session's inbox as a file of its own, served as "sent". That's mostly a
//! session parked, which takes its inbox once it's back; a running one
//! takes a steer within moments. Only this machine's inboxes are read.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use inotify::{Inotify, WatchDescriptor, WatchMask};
use serde_json::{Value, json};

use crate::clock;
use crate::talks::Talk;

/// What the served records are, one per address.
pub const KIND: &str = "talks";
/// How many talks done with are shown after the open ones, as Super+S's
/// line keeps its own (TalkLine.js's KEPT_TALKS).
const KEPT: usize = 3;
/// The stages after which a talk is done with: heard, ended, or never
/// delivered.
const CLOSED: [&str; 3] = ["read", "over", "failed"];

/// The inboxes watched: their root, for an inbox made, and each inbox,
/// for a steer written into it or taken out.
pub struct Inboxes {
    inotify: Inotify,
    root: PathBuf,
    watching: HashMap<WatchDescriptor, PathBuf>,
}

impl Inboxes {
    /// Watches the inboxes under `root`, and `root` for those made later,
    /// making it first where no session has yet, as the harness makes it
    /// whenever one looks, so a daemon started first still hears them.
    pub fn open(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        let mut inboxes = Self {
            inotify: Inotify::init()?,
            root: root.to_owned(),
            watching: HashMap::new(),
        };
        inboxes.watch();
        Ok(inboxes)
    }

    /// Adds a watch on the root and each inbox in it that none watches yet.
    fn watch(&mut self) {
        let mask = WatchMask::MOVED_TO | WatchMask::MOVED_FROM | WatchMask::DELETE;
        let folders = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir());
        for folder in std::iter::once(self.root.clone()).chain(folders) {
            if self.watching.values().any(|watched| *watched == folder) {
                continue;
            }
            let mask = if folder == self.root {
                WatchMask::CREATE | WatchMask::MOVED_TO
            } else {
                mask
            };
            if let Ok(watch) = self.inotify.watches().add(&folder, mask) {
                self.watching.insert(watch, folder);
            }
        }
    }

    /// Waits until a steer lands in an inbox or leaves one, or an inbox
    /// is made, which is watched in turn.
    pub fn heard(&mut self) -> io::Result<()> {
        let mut buffer = [0; 4096];
        let events = self.inotify.read_events_blocking(&mut buffer)?.count();
        self.watch();
        if events == 0 {
            return Err(io::Error::other("the inboxes' watch ended"));
        }
        Ok(())
    }
}

/// The talks waiting in each inbox under `root`, by its address in
/// capitals: each steer carrying a talk's id and words, and no act, the
/// moment it was written its file name's first part where it says none.
pub fn waiting(root: &Path) -> HashMap<String, Vec<Talk>> {
    let mut by: HashMap<String, Vec<Talk>> = HashMap::new();
    for inbox in fs::read_dir(root).into_iter().flatten().flatten() {
        let address = inbox.file_name().to_string_lossy().to_uppercase();
        for steer in fs::read_dir(inbox.path()).into_iter().flatten().flatten() {
            let name = steer.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            let Some(said) = fs::read(steer.path())
                .ok()
                .and_then(|text| serde_json::from_slice::<Value>(&text).ok())
            else {
                // Taken between the listing and the read, or not a steer.
                continue;
            };
            if let Some(talk) = talk_of(&said, stem) {
                by.entry(address.clone()).or_default().push(talk);
            }
        }
    }
    by
}

/// A steer's talk, sent and not yet taken, or none for an act or words
/// with no talk's id.
fn talk_of(steer: &Value, stem: &str) -> Option<Talk> {
    let id = steer["id"].as_str().filter(|id| !id.is_empty())?;
    let words = steer["text"].as_str().map(str::trim).unwrap_or("");
    if words.is_empty() || steer.get("act").is_some_and(|act| !act.is_null()) {
        return None;
    }
    let at = steer["at"].as_str().map_or_else(
        || {
            stem.split('-')
                .next()
                .and_then(|ms| ms.parse().ok())
                .map(clock::text_of)
                .unwrap_or_default()
        },
        str::to_owned,
    );
    Some(Talk {
        id: id.to_owned(),
        at,
        words: words.to_owned(),
        stage: "sent".to_owned(),
        ..Talk::default()
    })
}

/// A record per address with any talk, its talks as the launcher shows
/// them: those of `told` (talks::by_address) and those `waiting` not yet
/// taken, each once, the newest first, every open one, then the newest
/// `KEPT` of the rest.
pub fn records<S: std::hash::BuildHasher>(
    told: &HashMap<String, Vec<Talk>, S>,
    waiting: &HashMap<String, Vec<Talk>, S>,
) -> Vec<Value> {
    let mut addresses: Vec<&String> = told.keys().chain(waiting.keys()).collect();
    addresses.sort();
    addresses.dedup();
    addresses
        .into_iter()
        .map(|address| {
            let kept = told.get(address).map_or(&[][..], Vec::as_slice);
            let sent = waiting.get(address).map_or(&[][..], Vec::as_slice);
            let talks: Vec<Value> = shown(kept, sent).iter().map(|talk| said(talk)).collect();
            json!({"key": format!("{KIND}:{address}"), "kind": KIND, "address": address,
                "talks": talks})
        })
        .collect()
}

/// The talks shown of one session's, newest first: every open one, then
/// the newest of the rest.
fn shown<'a>(told: &'a [Talk], waiting: &'a [Talk]) -> Vec<&'a Talk> {
    let mut all: Vec<&Talk> = told
        .iter()
        .chain(
            waiting
                .iter()
                .filter(|one| !told.iter().any(|kept| kept.id == one.id)),
        )
        .collect();
    let at = |talk: &Talk| clock::ms_of(&json!(talk.at)).unwrap_or(0);
    all.sort_by_key(|talk| std::cmp::Reverse(at(talk)));
    let (open, done): (Vec<&Talk>, Vec<&Talk>) = all
        .into_iter()
        .partition(|talk| !CLOSED.contains(&talk.stage.as_str()));
    open.into_iter()
        .chain(done.into_iter().take(KEPT))
        .collect()
}

/// A talk as the launcher reads one, Orchestrators.qml's fold.
fn said(talk: &Talk) -> Value {
    json!({"id": talk.id, "at": talk.at, "words": talk.words, "stage": talk.stage,
        "after": talk.after, "reply": talk.reply, "why": talk.why, "came": talk.came})
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("agents-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        folder
    }

    fn talk(id: &str, at: &str, stage: &str) -> Talk {
        Talk {
            id: id.to_owned(),
            at: at.to_owned(),
            words: format!("said {id}"),
            stage: stage.to_owned(),
            ..Talk::default()
        }
    }

    #[test]
    fn a_talk_waiting_in_an_inbox_is_sent_and_nothing_else_there_is() {
        let root = folder("inboxes");
        let inbox = root.join("skiff");
        fs::create_dir_all(&inbox).expect("an inbox");
        let steers = [
            (
                "1794470400000-aa.json",
                json!({"text": "too dim", "id": "talk-1", "from": "joel"}),
            ),
            (
                "1794470400001-bb.json",
                json!({"text": "later", "id": "talk-2", "at": "2026-11-14T09:00:00.000Z"}),
            ),
            (
                "1794470400002-cc.json",
                json!({"text": "", "act": "pause", "id": "talk-3"}),
            ),
            ("1794470400003-dd.json", json!({"text": "no id"})),
            (
                "1794470400004-ee.json.bad",
                json!({"text": "set aside", "id": "talk-5"}),
            ),
        ];
        for (name, steer) in steers {
            fs::write(inbox.join(name), steer.to_string()).expect("a steer");
        }
        let waiting = waiting(&root);
        let mut talks = waiting["SKIFF"].clone();
        talks.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(
            talks
                .iter()
                .map(|one| (one.id.as_str(), one.at.as_str(), one.stage.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("talk-1", "2026-11-12T08:00:00.000Z", "sent"),
                ("talk-2", "2026-11-14T09:00:00.000Z", "sent"),
            ],
            "talks alone, sent, when their file was named unless they say"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn each_session_shows_its_open_talks_then_the_newest_done_with() {
        let at = |minute: u32| format!("2026-11-14T09:{minute:02}:00.000Z");
        let told: HashMap<String, Vec<Talk>> = [(
            "SKIFF".to_owned(),
            vec![
                talk("read-1", &at(1), "read"),
                talk("read-2", &at(2), "read"),
                talk("failed-3", &at(3), "failed"),
                talk("over-4", &at(4), "over"),
                talk("replied-5", &at(5), "replied"),
                talk("reading-0", &at(0), "reading"),
            ],
        )]
        .into();
        let waiting: HashMap<String, Vec<Talk>> = [
            (
                "SKIFF".to_owned(),
                vec![
                    talk("sent-6", &at(6), "sent"),
                    talk("replied-5", &at(5), "sent"),
                ],
            ),
            ("REEF".to_owned(), vec![talk("sent-7", &at(7), "sent")]),
        ]
        .into();
        let records = records(&told, &waiting);
        let ids = |record: &Value| -> Vec<String> {
            record["talks"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|one| one["id"].as_str().unwrap_or("").to_owned())
                .collect()
        };
        assert_eq!(
            (records[1]["key"].as_str(), ids(&records[1])),
            (
                Some("talks:SKIFF"),
                vec![
                    "sent-6",
                    "replied-5",
                    "reading-0",
                    "over-4",
                    "failed-3",
                    "read-2"
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            ),
            "open ones first, newest first, then the three newest done with"
        );
        assert_eq!(
            records[1]["talks"][1]["stage"], "replied",
            "a talk taken is its record's, never its steer's"
        );
        assert_eq!(
            (records[0]["address"].as_str(), ids(&records[0])),
            (Some("REEF"), vec!["sent-7".to_owned()]),
            "a session whose only talk waits is served too"
        );
    }

    #[test]
    fn a_steer_written_or_taken_is_heard_even_in_an_inbox_made_later() {
        let root = folder("inboxes-heard");
        // Written outside the inboxes, as a writer's temporary file is in
        // the harness, so only its rename in is heard.
        let aside = folder("inboxes-aside");
        let mut inboxes = Inboxes::open(&root).expect("watched");
        let heard = thread::spawn(move || {
            inboxes.heard().expect("an inbox made");
            inboxes.heard().expect("a steer landed");
        });
        thread::sleep(Duration::from_millis(50));
        let inbox = root.join("KELP");
        fs::create_dir_all(&inbox).expect("an inbox");
        thread::sleep(Duration::from_millis(50));
        fs::write(aside.join("steer.tmp"), "{}").expect("written");
        fs::rename(aside.join("steer.tmp"), inbox.join("1-a.json")).expect("renamed in");
        heard.join().expect("both heard");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&aside);
    }
}
