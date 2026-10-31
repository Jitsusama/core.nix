//! Every screenshot read, a record each, for the shell, which asks for
//! them over a socket in version 1 of its daemons' interface (dictation's
//! daemon.rs): hello, get, list, search, watch, unwatch and health.
//!
//! A record is kind "screenshot", keyed `screenshot:<path>`; its fields
//! are its path, when it was taken (the file's time), its text, and, from
//! get alone, its lines in reading order. A search gives the screenshots
//! holding every word typed, best first, each with the line that matched,
//! and leaves lines out, so the shell holds only what it shows.
//!
//! The records are kept in a file of their own, so a restart reads only
//! the pictures it hasn't, and a record goes when its picture does.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::UNIX_EPOCH;

use dictation::daemon::{self, Answers, VERSION, Watches, error, now, write_whole};
use serde_json::{Value, json};

/// The one kind of record this daemon keeps.
const KIND: &str = "screenshot";

/// How many a search answers when it doesn't say, as the people daemon.
const SEARCHED: usize = 50;

/// One screenshot read.
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub path: PathBuf,
    /// When it was taken, in milliseconds since the epoch, and its size,
    /// which together say whether the file is still the one read.
    pub taken: u64,
    pub size: u64,
    pub lines: Vec<String>,
}

/// When a file was last written, and its size, if it's there.
pub fn stamp(path: &Path) -> Option<(u64, u64)> {
    let data = std::fs::metadata(path).ok()?;
    let taken = data.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((u64::try_from(taken.as_millis()).ok()?, data.len()))
}

/// Whether a file is a picture this reads.
pub fn is_picture(path: &Path) -> bool {
    path.extension()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("png"))
}

impl Shot {
    fn key(&self) -> String {
        format!("screenshot:{}", self.path.display())
    }

    fn json(&self, lines: bool) -> Value {
        let mut record = json!({
            "key": self.key(), "kind": KIND, "path": self.path, "taken": self.taken,
            "text": self.lines.join("\n"),
        });
        if lines {
            record["lines"] = json!(self.lines);
        }
        record
    }

    fn kept(&self) -> Value {
        json!({"path": self.path, "taken": self.taken, "size": self.size, "lines": self.lines})
    }

    fn from_kept(kept: &Value) -> Option<Self> {
        Some(Self {
            path: PathBuf::from(kept["path"].as_str()?),
            taken: kept["taken"].as_u64()?,
            size: kept["size"].as_u64()?,
            lines: kept["lines"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        })
    }

    /// How well it matches the words typed, lowercased: none unless its
    /// text holds every one; better with all of them in one line; and the
    /// line that matched best.
    fn matches(&self, words: &[String]) -> Option<(bool, &str)> {
        let holds = |text: &str| {
            let text = text.to_lowercase();
            words.iter().all(|word| text.contains(word.as_str()))
        };
        if !holds(&self.lines.join("\n")) {
            return None;
        }
        if let Some(line) = self.lines.iter().find(|line| holds(line)) {
            return Some((true, line));
        }
        let first = &words[0];
        let line = self
            .lines
            .iter()
            .find(|line| line.to_lowercase().contains(first.as_str()))?;
        Some((false, line))
    }
}

/// Every screenshot read, newest first, where they're kept, and who
/// watches them.
pub struct Shelf {
    shots: Vec<Shot>,
    file: PathBuf,
    watches: Watches,
    since: u64,
    /// How many pictures wait to be read, for health.
    pub waiting: usize,
}

impl Shelf {
    /// The records kept in `file`, or none.
    pub fn read(file: &Path) -> Self {
        let kept: Value = std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        let mut shots: Vec<Shot> = kept["shots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Shot::from_kept)
            .collect();
        shots.sort_by_key(|shot| std::cmp::Reverse(shot.taken));
        Self {
            shots,
            file: file.to_owned(),
            watches: Watches::default(),
            since: now(),
            waiting: 0,
        }
    }

    /// Lets go of each record whose picture is gone or was written since,
    /// and gives the pictures in `folder` not read yet, newest first.
    pub fn unread(&mut self, folder: &Path) -> Vec<PathBuf> {
        let stale: Vec<PathBuf> = self
            .shots
            .iter()
            .filter(|shot| stamp(&shot.path) != Some((shot.taken, shot.size)))
            .map(|shot| shot.path.clone())
            .collect();
        for path in stale {
            self.forget(&path);
        }
        let mut unread: Vec<(u64, PathBuf)> = std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_picture(path) && !self.holds(path))
            .filter_map(|path| Some((stamp(&path)?.0, path)))
            .collect();
        unread.sort_by_key(|(taken, _)| std::cmp::Reverse(*taken));
        unread.into_iter().map(|(_, path)| path).collect()
    }

    /// Whether a picture's record is the one for the file as it is.
    pub fn holds(&self, path: &Path) -> bool {
        let now = stamp(path);
        self.shots
            .iter()
            .any(|shot| shot.path == path && now == Some((shot.taken, shot.size)))
    }

    /// Keeps a screenshot read, in place of its record before.
    pub fn put(&mut self, shot: Shot) {
        self.shots.retain(|held| held.path != shot.path);
        let at = self
            .shots
            .iter()
            .position(|held| held.taken < shot.taken)
            .unwrap_or(self.shots.len());
        self.watches.tell(&shot.key(), &shot.json(false));
        self.shots.insert(at, shot);
        self.save();
    }

    /// Lets go of a screenshot's record, its picture gone.
    pub fn forget(&mut self, path: &Path) {
        let Some(at) = self.shots.iter().position(|shot| shot.path == path) else {
            return;
        };
        let key = self.shots.remove(at).key();
        self.watches
            .tell(&key, &json!({"key": key, "kind": KIND, "gone": true}));
        self.save();
    }

    fn save(&self) {
        let kept = json!({"shots": self.shots.iter().map(Shot::kept).collect::<Vec<_>>()});
        let written = self
            .file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| write_whole(&self.file, &kept.to_string()));
        if let Err(why) = written {
            eprintln!("screen-text: keeping {}: {why}", self.file.display());
        }
    }

    /// The screenshots holding every word typed, best first: every word
    /// in one line, then newest.
    fn search(&self, words: &str, limit: usize) -> Vec<Value> {
        let words: Vec<String> = words.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(bool, &Shot, &str)> = self
            .shots
            .iter()
            .filter_map(|shot| shot.matches(&words).map(|(one, line)| (one, shot, line)))
            .collect();
        // Newest first already; a stable sort keeps it within each rank.
        found.sort_by_key(|(one, _, _)| !*one);
        found
            .into_iter()
            .take(limit)
            .map(|(_, shot, line)| {
                let mut record = shot.json(false);
                record["line"] = json!(line);
                record
            })
            .collect()
    }
}

impl Answers for Shelf {
    fn answer(&mut self, line: &str, connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "not a JSON line");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said["ask"].as_str().unwrap_or_default();
        let kind = said["kind"].as_str().unwrap_or(KIND);
        if matches!(asked, "list" | "watch" | "search") && kind != KIND {
            let why = format!("this daemon keeps only {KIND}, not {kind}");
            return error(&id, "unknown-kind", &why);
        }
        let keys = daemon::keys(&said);
        let named = |shot: &&Shot| keys.as_ref().is_none_or(|keys| keys.contains(&shot.key()));
        match asked {
            "hello" => json!({
                "id": id, "version": VERSION, "domain": "screenshots", "state": "answering",
                "since": self.since,
            }),
            "get" => {
                let keys = keys.clone().unwrap_or_default();
                let got: Vec<Value> = self
                    .shots
                    .iter()
                    .filter(|shot| keys.contains(&shot.key()))
                    .map(|shot| shot.json(true))
                    .collect();
                json!({"id": id, "records": got})
            }
            "list" => {
                let listed: Vec<Value> = self
                    .shots
                    .iter()
                    .take(daemon::limit(&said))
                    .map(|shot| shot.json(false))
                    .collect();
                json!({"id": id, "records": listed})
            }
            "search" => {
                let words = said["words"].as_str().unwrap_or_default();
                let limit = said["limit"]
                    .as_u64()
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(SEARCHED);
                json!({"id": id, "records": self.search(words, limit)})
            }
            "watch" => {
                let held: Vec<Value> = self
                    .shots
                    .iter()
                    .filter(named)
                    .map(|shot| shot.json(false))
                    .collect();
                self.watches.watch(connection, &id, keys, told);
                json!({"id": id, "records": held})
            }
            "unwatch" => {
                self.watches.unwatch(connection, &id);
                json!({"id": id, "unwatched": true})
            }
            "health" => json!({
                "id": id, "version": VERSION, "state": "answering",
                "screenshots": self.shots.len(), "waiting": self.waiting,
                "watching": self.watches.len(),
            }),
            other => error(&id, "unknown-ask", &format!("no ask {other:?}")),
        }
    }

    fn gone(&mut self, connection: u64) {
        self.watches.gone(connection);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn shelf(folder: &Path) -> Shelf {
        Shelf::read(&folder.join("records.json"))
    }

    /// A picture in `folder`, and its record as read.
    fn shot(folder: &Path, name: &str, lines: &[&str]) -> Shot {
        let path = folder.join(name);
        std::fs::write(&path, name).unwrap();
        let (taken, size) = stamp(&path).unwrap();
        Shot {
            path,
            taken,
            size,
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        }
    }

    fn ask(shelf: &mut Shelf, line: &str) -> Value {
        let (told, _heard) = mpsc::channel();
        shelf.answer(line, 1, &told)
    }

    #[test]
    fn a_search_finds_every_word_and_ranks_one_line_holding_them_first() {
        let folder = tempfile::tempdir().unwrap();
        let mut shelf = shelf(folder.path());
        shelf.put(shot(
            folder.path(),
            "a.png",
            &["Move the review", "to Thursday"],
        ));
        shelf.put(shot(folder.path(), "b.png", &["Review on Thursday at ten"]));
        shelf.put(shot(folder.path(), "c.png", &["Lunch"]));
        let found = ask(
            &mut shelf,
            r#"{"id": 1, "ask": "search", "words": "thurs REVIEW"}"#,
        );
        let records = found["records"].as_array().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0]["key"].as_str().unwrap().ends_with("b.png"));
        assert_eq!(records[0]["line"], "Review on Thursday at ten");
        assert_eq!(records[1]["line"], "to Thursday");
        assert!(records[0].get("lines").is_none());
    }

    #[test]
    fn get_gives_the_lines_and_a_record_goes_with_its_picture() {
        let folder = tempfile::tempdir().unwrap();
        let mut shelf = shelf(folder.path());
        let read = shot(folder.path(), "a.png", &["one", "two"]);
        let key = read.key();
        shelf.put(read.clone());
        let got = ask(
            &mut shelf,
            &json!({"id": 2, "ask": "get", "keys": [key]}).to_string(),
        );
        assert_eq!(got["records"][0]["lines"], json!(["one", "two"]));
        assert_eq!(got["records"][0]["text"], "one\ntwo");
        std::fs::remove_file(&read.path).unwrap();
        assert!(shelf.unread(folder.path()).is_empty());
        let listed = ask(&mut shelf, r#"{"id": 3, "ask": "list"}"#);
        assert_eq!(listed["records"], json!([]));
    }

    #[test]
    fn a_restart_reads_only_the_pictures_it_hasnt() {
        let folder = tempfile::tempdir().unwrap();
        let mut shelf = shelf(folder.path());
        shelf.put(shot(folder.path(), "read.png", &["kept"]));
        shot(folder.path(), "new.png", &[]);
        std::fs::write(folder.path().join("notes.txt"), "not a picture").unwrap();
        let mut again = Shelf::read(&folder.path().join("records.json"));
        assert_eq!(again.unread(folder.path()), [folder.path().join("new.png")]);
        let listed = ask(&mut again, r#"{"id": 4, "ask": "list"}"#);
        assert_eq!(listed["records"][0]["text"], "kept");
    }

    #[test]
    fn a_watch_hears_a_screenshot_read_and_gone() {
        let folder = tempfile::tempdir().unwrap();
        let mut shelf = shelf(folder.path());
        let (told, heard) = mpsc::channel();
        shelf.answer(r#"{"id": 5, "ask": "watch"}"#, 1, &told);
        let read = shot(folder.path(), "a.png", &["hello"]);
        shelf.put(read.clone());
        shelf.forget(&read.path);
        let first: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(first["changed"][0]["text"], "hello");
        let then: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(then["changed"][0]["gone"], true);
    }

    #[test]
    fn another_kind_is_refused_and_hello_names_the_domain() {
        let folder = tempfile::tempdir().unwrap();
        let mut shelf = shelf(folder.path());
        let refused = ask(
            &mut shelf,
            r#"{"id": 1, "ask": "search", "kind": "person"}"#,
        );
        assert_eq!(refused["error"]["code"], "unknown-kind");
        assert_eq!(
            ask(&mut shelf, r#"{"id": 2, "ask": "hello"}"#)["domain"],
            "screenshots"
        );
    }
}
