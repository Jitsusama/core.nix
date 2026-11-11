//! The history on disk, in the daemon's own state folder, so a restart
//! loses none: every notice kept, and the id the next one gets, so a key
//! names one notice for good. Written whole to a file beside it, made to
//! reach the disk, and moved over it, so a crash or a battery running out
//! mid-write leaves the last one whole rather than an empty file.
//!
//! One that doesn't read all the same is set aside, kept for whoever
//! wants what it held, and the daemon starts with none: a daemon that
//! stopped at each start over it would leave nothing to take any notice,
//! and every one sent would be lost.
//!
//! The first time it starts, with no history of its own, it takes the
//! shell's (`notices.json` in Quickshell's state), so moving notices out
//! of the shell keeps what Joel had. The shell's ids were its server's,
//! begun again at each start, so each taken gets a new one.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::held::Holding;
use crate::notice::{Notice, Urgency};

/// What's written: the id the next notice gets, and those kept, newest
/// first.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    pub next: u32,
    pub notices: Vec<Notice>,
}

/// What was written, as it was read.
#[derive(Debug)]
pub enum Kept {
    Saved(Saved),
    /// Nothing was written yet.
    None,
    /// It didn't read, so it was set aside here, saying why, and nothing
    /// is kept now.
    SetAside(PathBuf, String),
}

/// Reads what was written. One that doesn't read is moved aside, as
/// `notices.json.unread-<ms>` at `now`, so starting over writes over
/// nothing Joel kept. One that can't be opened at all is an error, since
/// moving it would fail the same way.
pub fn read(file: &Path, now: i64) -> Result<Kept, String> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Kept::None),
        Err(why) => return Err(format!("{} can't be read: {why}", file.display())),
    };
    let why = match serde_json::from_str(&text) {
        Ok(saved) => return Ok(Kept::Saved(saved)),
        Err(why) => why.to_string(),
    };
    let aside = file.with_extension(format!("json.unread-{now}"));
    std::fs::rename(file, &aside)
        .map_err(|moved| format!("{} doesn't read ({why}) nor move: {moved}", file.display()))?;
    Ok(Kept::SetAside(aside, why))
}

/// Writes what's held, those passing unkept left out.
pub fn write(file: &Path, holding: &Holding) -> std::io::Result<()> {
    let saved = serde_json::json!({
        "next": holding.next(),
        "notices": holding.kept().collect::<Vec<_>>(),
    });
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let beside = file.with_extension("json.new");
    let mut written = std::fs::File::create(&beside)?;
    written.write_all(saved.to_string().as_bytes())?;
    // On the disk before it's moved into place, and the move on the disk
    // after, so neither a crash nor a battery running out between them
    // leaves a name over a file not yet written.
    written.sync_all()?;
    std::fs::rename(&beside, file)?;
    match file.parent() {
        Some(folder) => std::fs::File::open(folder)?.sync_all(),
        None => Ok(()),
    }
}

/// The newest of the shell's histories under Quickshell's `by-shell`
/// state folder, one for each shell it's run, if there's any.
pub fn shells(folder: &Path) -> Option<PathBuf> {
    std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("notices.json"))
        .filter_map(|file| Some((file.metadata().ok()?.modified().ok()?, file)))
        .max()
        .map(|(_, file)| file)
}

/// The shell's history as notices, newest first, each with an id from
/// `holding`. None is open, since the shell's server held them, and what
/// isn't read was missed or not as the shell said.
pub fn taken(text: &str, holding: &mut Holding) -> Result<Vec<Notice>, String> {
    let said: Vec<Value> = serde_json::from_str(text).map_err(|why| why.to_string())?;
    Ok(said
        .iter()
        .map(|one| from_shell(one, holding.new_id()))
        .collect())
}

fn from_shell(said: &Value, id: u32) -> Notice {
    let word = |name: &str| said[name].as_str().unwrap_or("").to_owned();
    let maybe = |name: &str| said[name].as_str().map(str::to_owned);
    let yes = |name: &str| said[name].as_bool().unwrap_or(false);
    let mut hints = serde_json::Map::new();
    if let Some(waits) = said["waits"].as_str().filter(|waits| !waits.is_empty()) {
        hints.insert("x-waiting".into(), Value::String(waits.to_owned()));
    }
    let urgency = match said["urgency"].as_str() {
        Some("low") => Urgency::Low,
        Some("critical") => Urgency::Critical,
        _ => Urgency::Normal,
    };
    // The shell kept a notice's words plain and styled; the styled are
    // the nearer to what was sent.
    let body = said["styled"]
        .as_str()
        .or(said["body"].as_str())
        .unwrap_or("")
        .to_owned();
    let urgent = yes("urgent");
    let held = yes("held");
    Notice {
        id,
        at: said["at"].as_i64().unwrap_or(0),
        app: word("app"),
        entry: word("entry"),
        summary: word("summary"),
        body,
        urgency,
        category: word("category"),
        hints,
        expire: -1,
        picture: word("picture"),
        // The shell kept no level, only what it made of one; this says as
        // much, held back counted, and nothing reads it of one closed.
        level: if urgent {
            "urgent"
        } else if held {
            "count"
        } else {
            "card"
        }
        .to_owned(),
        urgent,
        held,
        missed: yes("missed"),
        read: yes("read"),
        read_how: maybe("readHow"),
        read_at: said["readAt"].as_i64(),
        withdrawn: yes("withdrawn"),
        left: maybe("left").or_else(|| Some("dismissed".to_owned())),
        left_at: said["leftAt"].as_i64(),
        ..Notice::default()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("notices-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        folder
    }

    #[test]
    fn what_was_written_is_read_back_but_not_what_passes_unkept() {
        let folder = folder("store");
        let file = folder.join("notices").join("notices.json");
        assert!(
            matches!(read(&file, 0), Ok(Kept::None)),
            "a history never written reads as none"
        );
        let mut holding = Holding::default();
        let kept = holding.new_id();
        let passing = holding.new_id();
        holding.arrive(Notice {
            id: kept,
            summary: "kept".into(),
            ..Notice::default()
        });
        holding.arrive(Notice {
            id: passing,
            transient: true,
            ..Notice::default()
        });
        write(&file, &holding).expect("written");
        let Ok(Kept::Saved(saved)) = read(&file, 0) else {
            panic!("what was written reads back");
        };
        assert_eq!(saved.next, 3);
        assert_eq!(saved.notices.len(), 1);
        assert_eq!(saved.notices[0].summary, "kept");
        assert!(
            !folder.join("notices").join("notices.json.new").exists(),
            "the file written first is moved into place, not left beside it"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_history_that_doesnt_read_is_set_aside_and_started_over() {
        let folder = folder("unread");
        let file = folder.join("notices.json");
        // Torn by a crash, or left empty by a battery running out.
        for (torn, now) in [("{ not json", 7), ("", 8)] {
            std::fs::write(&file, torn).expect("written");
            let Ok(Kept::SetAside(aside, _)) = read(&file, now) else {
                panic!("{torn:?} is set aside");
            };
            assert_eq!(aside, folder.join(format!("notices.json.unread-{now}")));
            assert_eq!(
                std::fs::read_to_string(&aside).ok().as_deref(),
                Some(torn),
                "kept whole"
            );
            assert!(!file.exists(), "{torn:?} no longer stands as the history");
        }
        write(&file, &Holding::default()).expect("written anew");
        assert!(
            matches!(read(&file, 9), Ok(Kept::Saved(_))),
            "the history begun again reads"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_shells_history_is_taken_each_with_an_id_of_its_own() {
        let shell = r#"[
            {"id": "kept:1:0", "at": 2000, "app": "Slack", "entry": "slack",
             "summary": "Ana", "body": "hi there", "styled": "<b>hi</b> there",
             "picture": "file:///a.png", "urgency": "low", "category": "im.received",
             "low": true, "urgent": false, "held": false, "missed": false,
             "read": true, "readHow": "readInSlack", "readAt": 2500,
             "left": "expired", "leftAt": 2100},
            {"id": 7, "at": 1000, "app": "timers", "summary": "Tea", "body": "",
             "urgency": "critical", "urgent": true, "missed": true, "read": false,
             "waits": "todo:3"}
        ]"#;
        let mut holding = Holding::new(Vec::new(), 5);
        let taken = taken(shell, &mut holding).expect("it's taken");
        assert_eq!(taken.len(), 2);
        let (slack, tea) = (&taken[0], &taken[1]);
        assert_eq!((slack.id, tea.id), (5, 6));
        assert_eq!(slack.body, "<b>hi</b> there");
        assert_eq!(slack.urgency, Urgency::Low);
        assert_eq!(slack.read_how.as_deref(), Some("readInSlack"));
        assert_eq!(slack.left.as_deref(), Some("expired"));
        assert_eq!((slack.live, tea.live), (false, false));
        assert_eq!(tea.urgency, Urgency::Critical);
        // Urgent, missed and read.
        assert_eq!((tea.urgent, tea.missed, tea.read), (true, true, false));
        assert_eq!(tea.hint("x-waiting"), "todo:3");
        assert_eq!(
            tea.left.as_deref(),
            Some("dismissed"),
            "one still open there left with it"
        );
    }

    #[test]
    fn the_newest_shells_history_is_the_one_taken() {
        let folder = folder("shells");
        for (shell, at) in [("old", 1_000_000), ("new", 2_000_000)] {
            let file = folder.join(shell).join("notices.json");
            std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
            std::fs::write(&file, "[]").expect("written");
            let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(at);
            File::options()
                .write(true)
                .open(&file)
                .and_then(|open| open.set_modified(when))
                .expect("dated");
        }
        assert_eq!(
            shells(&folder),
            Some(folder.join("new").join("notices.json"))
        );
        assert_eq!(shells(&folder.join("none")), None);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
