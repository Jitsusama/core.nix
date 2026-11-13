//! Joel's word on a to-do, kept in `mission/settled.json` as the shell
//! kept it (Waiting.js's `settle`): `done`, by identity, when he said it,
//! in milliseconds; `notNow`, by identity, what the item said when he put
//! it off, and when, so it comes back once its owner says something new.
//! The daemon writes it as Mark It Done and Put It Off ask it to, so the
//! shell writes no file for them (Y2YBTS, 2026-11-10), and reads the file
//! afresh first, since the orchestrator may clear what Joel said. It's
//! written beside and renamed into place, so nothing reads it half
//! written, and the daemon's own watch on it hears it as any write.

use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Map, Value, json};

/// Why Joel's word couldn't be kept, in the interface's codes.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

pub fn refused(code: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// Keeps Joel's word `act`, `done` or `notNow`, on the to-do `item`,
/// which says `said` now (stream.rs's `said`), at `at` milliseconds.
pub fn settle(path: &Path, act: &str, item: &str, said: &str, at: u64) -> Result<(), Refused> {
    let field = match act {
        "done" => "done",
        "notNow" => "notNow",
        _ => return Err(refused("bad-act", format!("no act {act:?} on a to-do"))),
    };
    let mut word = match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(word)) => word,
            // What can't be read is left as it is, rather than written
            // over with all Joel said before it lost.
            _ => return Err(refused("refused", "settled.json isn't an object")),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Map::new(),
        Err(error) => return Err(refused("refused", error.to_string())),
    };
    for each in ["done", "notNow"] {
        if !word.get(each).is_some_and(Value::is_object) {
            word.insert(each.to_owned(), json!({}));
        }
    }
    let said = if field == "done" {
        json!(at)
    } else {
        json!({"said": said, "at": at})
    };
    word[field][item] = said;
    written(path, &Value::Object(word).to_string())
        .map_err(|error| refused("refused", error.to_string()))
}

/// Writes `text` whole beside `path`, then renames it into place.
pub fn written(path: &Path, text: &str) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut writing = path.as_os_str().to_owned();
    writing.push(".writing");
    fs::write(&writing, text)?;
    fs::rename(&writing, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn joels_word_is_kept_beside_what_he_said_before() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("mission/settled.json");
        settle(&path, "done", "Q:a", "[]", 5).expect("a file made new");
        fs::write(
            &path,
            r#"{"done":{"Q:a":5,"Q:z":1},"notNow":{},"cleared":true}"#,
        )
        .unwrap();
        settle(&path, "notNow", "Q:b", r#"["B"]"#, 7).expect("kept beside the rest");
        assert_eq!(
            read(&path),
            json!({"done": {"Q:a": 5, "Q:z": 1}, "notNow": {"Q:b": {"said": "[\"B\"]", "at": 7}},
                "cleared": true}),
            "what was there stays, and what he said now joins it"
        );
        assert!(
            !folder.path().join("mission/settled.json.writing").exists(),
            "written beside, then renamed into place"
        );
    }

    #[test]
    fn a_word_that_cant_be_kept_leaves_the_file_be() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settled.json");
        fs::write(&path, r#"{"done":"#).unwrap();
        assert_eq!(
            settle(&path, "done", "Q:a", "[]", 5).map_err(|refused| refused.code),
            Err("refused"),
            "a file cut short is refused"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"done":"#,
            "and left as it was"
        );
        assert_eq!(
            settle(&path, "forget", "Q:a", "[]", 5).map_err(|refused| refused.code),
            Err("bad-act"),
            "an act no to-do takes"
        );
    }
}
