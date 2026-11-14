//! What a reader writes down: every decision, one line each in a log, and
//! for each stop a record of its own that only Joel ends. While the safe
//! word is in shadow, each stop's record says so, and nothing that acts on
//! records takes one marked shadow for a stop; it only says what it would
//! have refused.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use crate::decide::Decision;

/// Where a decision came from, as the log and a record name it.
#[derive(Clone, Debug)]
pub struct Source<'a> {
    /// The key naming the saying, as `slack:<channel>:<ts>`.
    pub key: &'a str,
    /// Where its platform shows it.
    pub link: &'a str,
    /// Who said it, as the platform names them.
    pub author: &'a str,
    /// What it says, as the platform gave it.
    pub text: &'a str,
    /// When it was said, as the platform says it.
    pub said_at: &'a str,
}

/// A decision's line in the log, saying what it would have done in shadow.
#[must_use]
pub fn line(source: &Source<'_>, decision: &Decision, heard_at: u64, shadow: bool) -> Value {
    json!({
        "heard_at": heard_at,
        "key": source.key,
        "link": source.link,
        "author": source.author,
        "said_at": source.said_at,
        "verdict": decision.verdict.to_string(),
        "by_joel": decision.by_joel,
        "reaches": decision.reaches.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "fell_back": decision.fell_back,
        "tell": decision.tell.to_string(),
        "reply": decision.reply.to_string(),
        "shadow": shadow,
        "text": source.text,
    })
}

/// A stop's own record, which stands until Joel resumes it.
#[must_use]
pub fn stop(source: &Source<'_>, decision: &Decision, heard_at: u64, shadow: bool) -> Value {
    json!({
        "id": source.key,
        "shadow": shadow,
        "said_by": source.author,
        "where": source.link,
        "words": source.text,
        "said_at": source.said_at,
        "heard_at": heard_at,
        "by_joel": decision.by_joel,
        "reaches": decision.reaches.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "fell_back": decision.fell_back,
        "resumed": Value::Null,
    })
}

/// Adds a line to the log at `path`, making its folder if it has none.
///
/// # Errors
///
/// Why the line couldn't be written.
pub fn append(path: &Path, line: &Value) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut log = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(log, "{line}")
}

/// Writes a record whole, through a file beside it renamed into place, so
/// nothing reading the folder finds one half written. A record already
/// there is left as it stands, since a stop said again is the same stop.
///
/// # Errors
///
/// Why the record couldn't be written.
pub fn keep(folder: &Path, name: &str, record: &Value) -> std::io::Result<bool> {
    fs::create_dir_all(folder)?;
    let path = folder.join(format!("{}.json", file_name(name)));
    if path.exists() {
        return Ok(false);
    }
    let written = path.with_extension("new");
    fs::write(&written, format!("{record:#}\n"))?;
    fs::rename(&written, &path)?;
    Ok(true)
}

/// A key as a file's name: anything but letters, digits, `.` and `-`
/// becomes `-`, so a key can't reach outside its folder.
fn file_name(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_stays_inside_its_folder() {
        assert_eq!(file_name("slack:C1:1.2"), "slack-C1-1.2", "colons go");
        assert_eq!(file_name("../../x"), "..-..-x", "slashes go");
    }
}
