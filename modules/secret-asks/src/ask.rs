//! One ask as its file says it, `<id>.ask.json` in the asks folder, held
//! to the shape the release line is built to (the shape's version 1): an
//! item, the command it's for, the tree it runs in, the variable it's
//! given as, how long it may run, and who asked and why. A file of any
//! other shape is no ask, so nothing an ask file says reaches a row or a
//! command unchecked.

use serde_json::{Value, json};

/// The shape's version this reads.
pub const VERSION: u64 = 1;
/// How long an ask waits for an answer before it ends, in milliseconds.
pub const WAITS_MS: i64 = 600_000;
/// The longest a released command may run, in minutes.
pub const LONGEST_MINUTES: u64 = 30;

/// Variables a command's start reads before it reads its own, so a value
/// given as one of them would be read by what starts the command.
const NEVER_GIVEN: &[&str] = &["PATH", "HOME", "LANG", "BASH_ENV", "ENV", "SHELL"];

/// An ask, read and checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub id: String,
    pub asked_ms: i64,
    pub session: String,
    /// The quest's id and title, when the session holds one.
    pub quest: Option<(String, String)>,
    pub item: String,
    pub why: String,
    pub argv: Vec<String>,
    pub tree: String,
    pub given: String,
    pub minutes: u64,
}

/// Whether a file's name is an ask's, and its id if so: 32 lowercase hex
/// characters, then `.ask.json`.
pub fn id_of(name: &str) -> Option<&str> {
    let id = name.strip_suffix(".ask.json")?;
    (id.len() == 32
        && id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
    .then_some(id)
}

/// Whether a variable's name may carry a value: capitals, digits and
/// underscores, not starting with a digit, and none a command's start
/// reads.
pub fn may_be_given(name: &str) -> bool {
    let mut bytes = name.bytes();
    let first = bytes.next();
    matches!(first, Some(b'A'..=b'Z' | b'_'))
        && name.len() <= 64
        && bytes.all(|byte| matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_'))
        && !NEVER_GIVEN.contains(&name)
        && !name.starts_with("LD_")
}

impl Ask {
    /// The ask a file's text says, under the id its name gives, or why it
    /// isn't one.
    ///
    /// # Errors
    ///
    /// Where the text isn't JSON, or isn't the shape's version 1.
    pub fn read(id: &str, text: &str) -> Result<Ask, String> {
        let said: Value = serde_json::from_str(text).map_err(|why| why.to_string())?;
        let text_of = |field: &str| {
            said[field]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("no {field}"))
        };
        if said["version"].as_u64() != Some(VERSION) {
            return Err("not version 1".to_owned());
        }
        if said["id"].as_str() != Some(id) {
            return Err("its id isn't its file's".to_owned());
        }
        let quest = match &said["quest"] {
            Value::Null => None,
            quest => Some((
                quest["id"].as_str().ok_or("a quest with no id")?.to_owned(),
                quest["title"]
                    .as_str()
                    .ok_or("a quest with no title")?
                    .to_owned(),
            )),
        };
        let argv: Vec<String> = said["argv"]
            .as_array()
            .ok_or("no argv")?
            .iter()
            .map(|word| {
                word.as_str()
                    .map(str::to_owned)
                    .ok_or("an argv word not text")
            })
            .collect::<Result<_, _>>()?;
        if argv.is_empty() {
            return Err("an empty argv".to_owned());
        }
        let given = text_of("given")?;
        if !may_be_given(&given) {
            return Err(format!("{given} can't carry a value"));
        }
        let minutes = said["minutes"].as_u64().ok_or("no minutes")?;
        if !(1..=LONGEST_MINUTES).contains(&minutes) {
            return Err(format!(
                "{minutes} minutes is outside 1 to {LONGEST_MINUTES}"
            ));
        }
        Ok(Ask {
            id: id.to_owned(),
            asked_ms: said["asked_ms"].as_i64().ok_or("no asked_ms")?,
            session: text_of("session")?,
            quest,
            item: text_of("item")?,
            why: text_of("why")?,
            argv,
            tree: text_of("tree")?,
            given,
            minutes,
        })
    }

    /// The ask as its file says it.
    pub fn said(&self) -> Value {
        json!({
            "version": VERSION, "id": self.id, "asked_ms": self.asked_ms,
            "session": self.session,
            "quest": self.quest.as_ref().map(|(id, title)| json!({"id": id, "title": title})),
            "item": self.item, "why": self.why, "argv": self.argv, "tree": self.tree,
            "given": self.given, "minutes": self.minutes,
        })
    }

    /// When it ends unanswered.
    pub fn ends_ms(&self) -> i64 {
        self.asked_ms + WAITS_MS
    }

    /// Whether it's still waiting at `now_ms`: not asked in the future, and
    /// not past its end.
    pub fn waits_at(&self, now_ms: i64) -> bool {
        self.asked_ms <= now_ms && now_ms < self.ends_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35";

    fn said() -> Value {
        json!({
            "version": 1, "id": ID, "asked_ms": 1_000, "session": "S1",
            "quest": {"id": "Q-1", "title": "Try the line"},
            "item": "op://Private/Release Test/password",
            "why": "To try the line.", "argv": ["secret-asks", "received"],
            "tree": "/tmp/tree", "given": "RELEASED", "minutes": 1,
        })
    }

    #[test]
    fn an_ask_reads_back_as_it_was_said() {
        let ask = Ask::read(ID, &said().to_string()).expect("the ask reads");
        assert_eq!(ask.said(), said(), "the ask didn't read back as said");
    }

    #[test]
    fn only_an_asks_name_is_one() {
        assert_eq!(
            id_of(&format!("{ID}.ask.json")),
            Some(ID),
            "an ask's name was missed"
        );
        assert_eq!(
            id_of(&format!("{ID}.answer.json")),
            None,
            "an answer was taken as an ask"
        );
        assert_eq!(
            id_of("3F0C.ask.json"),
            None,
            "a short or capital id was taken"
        );
        assert_eq!(id_of("../x.ask.json"), None, "a path was taken as an id");
    }

    #[test]
    fn a_value_is_never_given_as_what_a_start_reads() {
        for name in ["PATH", "LD_PRELOAD", "BASH_ENV", "lower", "1ST", "A-B", ""] {
            assert!(!may_be_given(name), "{name} was allowed to carry a value");
        }
        assert!(may_be_given("PACKAGE_TOKEN"), "a plain name was refused");
    }

    #[test]
    fn a_file_off_the_shape_is_no_ask() {
        let mut other = said();
        other["id"] = json!("0".repeat(32));
        assert!(
            Ask::read(ID, &other.to_string()).is_err(),
            "a mismatched id was read"
        );
        let mut long = said();
        long["minutes"] = json!(31);
        assert!(
            Ask::read(ID, &long.to_string()).is_err(),
            "31 minutes was read"
        );
        let mut given = said();
        given["given"] = json!("LD_PRELOAD");
        assert!(
            Ask::read(ID, &given.to_string()).is_err(),
            "LD_PRELOAD was read"
        );
    }

    #[test]
    fn an_ask_waits_ten_minutes_from_when_it_was_asked() {
        let ask = Ask::read(ID, &said().to_string()).expect("the ask reads");
        assert!(!ask.waits_at(999), "an ask from the future waited");
        assert!(ask.waits_at(1_000), "a fresh ask didn't wait");
        assert!(!ask.waits_at(601_000), "an ask past ten minutes waited");
    }
}
