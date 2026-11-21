//! What a client asks and what the reader says back, a JSON line each.
//!
//! A client asks once, as it connects:
//! `{"folder": "/abs/path", "suffix": ".json"}`. The reader answers with
//! `{"put": NAME, "text": TEXT}` for each file there whose name ends in
//! the suffix, then `{"ready": true}`, then a put or a
//! `{"gone": NAME}` for each change. A refusal is `{"no": WHY}`, and the
//! connection ends after it.

use std::path::PathBuf;

use serde_json::{Value, json};

/// The longest ask read, in bytes: a path and a suffix.
pub const LONGEST_ASK: usize = 4096;

/// A client's ask: the folder to follow and the files in it it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub folder: PathBuf,
    pub suffix: String,
}

/// Read an ask from its line, or say why it isn't one.
pub fn ask(line: &str) -> Result<Ask, String> {
    let value: Value =
        serde_json::from_str(line.trim()).map_err(|_| "the ask isn't JSON".to_string())?;
    let folder = value
        .get("folder")
        .and_then(Value::as_str)
        .ok_or("the ask names no folder")?;
    let suffix = value
        .get("suffix")
        .and_then(Value::as_str)
        .ok_or("the ask names no suffix")?;
    let folder = PathBuf::from(folder);
    if !folder.is_absolute() {
        return Err("the folder isn't an absolute path".into());
    }
    // A suffix with a slash would never match a name in the folder, and
    // an empty one would take the writers' half-written files too.
    if suffix.is_empty() || suffix.contains('/') {
        return Err("the suffix is empty or names a folder".into());
    }
    Ok(Ask {
        folder,
        suffix: suffix.to_string(),
    })
}

/// A line the reader says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A file there now, with what it holds.
    Put { name: String, text: String },
    /// A file no longer there, or no longer readable.
    Gone { name: String },
    /// Every file there has been said; changes follow.
    Ready,
    /// The ask is refused, and why.
    No { why: String },
}

impl Line {
    /// The line as it goes on the wire, without its newline.
    pub fn text(&self) -> String {
        match self {
            Line::Put { name, text } => json!({ "put": name, "text": text }),
            Line::Gone { name } => json!({ "gone": name }),
            Line::Ready => json!({ "ready": true }),
            Line::No { why } => json!({ "no": why }),
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_ask() {
        assert_eq!(
            ask(r#"{"folder": "/state/presence", "suffix": ".json"}"#),
            Ok(Ask {
                folder: PathBuf::from("/state/presence"),
                suffix: ".json".into()
            })
        );
    }

    #[test]
    fn refuses_what_isnt_an_ask() {
        for (line, why) in [
            ("nope", "the ask isn't JSON"),
            (r#"{"suffix": ".json"}"#, "the ask names no folder"),
            (r#"{"folder": "/a"}"#, "the ask names no suffix"),
            (
                r#"{"folder": "a", "suffix": ".json"}"#,
                "the folder isn't an absolute path",
            ),
            (
                r#"{"folder": "/a", "suffix": ""}"#,
                "the suffix is empty or names a folder",
            ),
            (
                r#"{"folder": "/a", "suffix": "x/.json"}"#,
                "the suffix is empty or names a folder",
            ),
        ] {
            assert_eq!(ask(line), Err(why.to_string()), "{line}");
        }
    }

    #[test]
    fn says_each_line_as_json() {
        assert_eq!(
            Line::Put {
                name: "a.json".into(),
                text: "{\"x\":\"\\n\"}\n".into()
            }
            .text(),
            r#"{"put":"a.json","text":"{\"x\":\"\\n\"}\n"}"#
        );
        assert_eq!(
            Line::Gone {
                name: "a.json".into()
            }
            .text(),
            r#"{"gone":"a.json"}"#
        );
        assert_eq!(Line::Ready.text(), r#"{"ready":true}"#);
        assert_eq!(Line::No { why: "x".into() }.text(), r#"{"no":"x"}"#);
    }
}
