//! A look Joel opened or cleared, kept in its own file in the harness's
//! looks folder as the shell kept it (Looks.js's `rewritten`): every field
//! the harness wrote stays, and `state`, `how` and `resolvedAt` say what he
//! did, in his words, and when. The daemon writes it as the shell asks, so
//! the shell reads and writes no look file (Y2YBTS and FEH1HE,
//! 2026-11-13), and reads the file afresh first, since its session may
//! have written it since. The daemon's watch on the folder hears the
//! rename and tells every watch the look as it now stands.

use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value;

use crate::settle::{Refused, refused, written};

/// What Joel may make of a look from the launcher: open it, or clear it.
/// Resolving one is its session's word, and expiring it the clock's.
const STATES: &[&str] = &["opened", "cleared"];

/// Keeps that Joel made the look `id` in `folder` `state`, saying `how`,
/// at `at` milliseconds.
pub fn settle_look(
    folder: &Path,
    id: &str,
    state: &str,
    how: &Value,
    at: i64,
) -> Result<(), Refused> {
    if !STATES.contains(&state) {
        return Err(refused("bad-act", format!("no look is made {state:?}")));
    }
    // A name that would reach outside the folder, or a file the daemon
    // keeps beside one as it's written, is no look's.
    if id.is_empty() || id.starts_with('.') || id.contains(['/', '\0']) {
        return Err(refused("unknown-item", format!("no look {id:?}")));
    }
    let path = folder.join(format!("{id}.json"));
    let mut look = match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(look)) => look,
            // What can't be read is left as it is, rather than written
            // over with less than its session said.
            _ => return Err(refused("refused", format!("{id}.json isn't a look"))),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(refused("unknown-item", format!("no look {id}")));
        }
        Err(error) => return Err(refused("refused", error.to_string())),
    };
    look.insert("state".to_owned(), Value::from(state));
    look.insert("how".to_owned(), how.clone());
    look.insert("resolvedAt".to_owned(), Value::from(iso(at)));
    let text = serde_json::to_string_pretty(&Value::Object(look))
        .map_err(|error| refused("refused", error.to_string()))?;
    written(&path, &text).map_err(|error| refused("refused", error.to_string()))
}

/// `at` milliseconds as JavaScript's `toISOString` writes them, as every
/// time in a look is written.
fn iso(at: i64) -> String {
    jiff::Timestamp::from_millisecond(at).map_or_else(
        |_| String::new(),
        |time| time.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(path: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn an_opened_look_keeps_all_its_session_said_and_says_how_and_when() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("a1.json");
        let was = json!({"v": 1, "id": "a1", "kind": "pr", "title": "Fix",
            "target": "https://example.com/1", "state": "waiting", "thread": "t9",
            "choices": ["yes", "no"]});
        fs::write(&path, was.to_string()).unwrap();
        settle_look(
            folder.path(),
            "a1",
            "opened",
            &json!("opened from For You"),
            1_794_000_000_123,
        )
        .expect("a look there");
        let mut meant = was;
        meant["state"] = json!("opened");
        meant["how"] = json!("opened from For You");
        meant["resolvedAt"] = json!("2026-11-06T21:20:00.123Z");
        assert_eq!(read(&path), meant, "every field kept beside what Joel did");
        assert!(
            fs::read_to_string(&path).unwrap().contains("\n  \"id\""),
            "written as the harness and the shell write a look, two spaces deep"
        );
        assert!(
            !folder.path().join("a1.json.writing").exists(),
            "written beside, then renamed into place"
        );
    }

    #[test]
    fn a_look_that_cant_be_settled_leaves_the_folder_be() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("cut.json"), r#"{"v":"#).unwrap();
        fs::write(folder.path().join("a1.json"), r#"{"id":"a1"}"#).unwrap();
        let code = |id: &str, state: &str| {
            settle_look(folder.path(), id, state, &json!(""), 5).map_err(|refused| refused.code)
        };
        assert_eq!(code("cut", "cleared"), Err("refused"), "a file cut short");
        assert_eq!(
            fs::read_to_string(folder.path().join("cut.json")).unwrap(),
            r#"{"v":"#,
            "and left as it was"
        );
        assert_eq!(
            code("gone", "cleared"),
            Err("unknown-item"),
            "a look not there"
        );
        assert_eq!(
            code("../a1", "cleared"),
            Err("unknown-item"),
            "a name outside it"
        );
        assert_eq!(code("", "cleared"), Err("unknown-item"), "no name");
        assert_eq!(
            code("a1", "resolved"),
            Err("bad-act"),
            "its session's word, not Joel's"
        );
        assert_eq!(
            read(&folder.path().join("a1.json")),
            json!({"id": "a1"}),
            "nothing refused was written"
        );
    }
}
