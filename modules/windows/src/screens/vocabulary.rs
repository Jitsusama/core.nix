//! The files Screens is said by, read once as the daemon starts: the
//! launcher's words.json and the account's number settings, each one it
//! can't read said once in the journal, Screens keeping its rows.

use std::path::Path;

use ready_rows::numbers::Numbers;
use serde_json::Value;

use super::Words;

/// The words a screen's row is said in, from the launcher's words.json,
/// each it lacks said by its id and warned of once, here.
#[must_use]
pub fn worded(path: &Path) -> Words {
    let read = std::fs::read_to_string(path).map_err(|why| why.to_string());
    let vocabulary =
        read.and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()));
    let (words, lacked) = Words::read(vocabulary.as_ref().unwrap_or(&Value::Null));
    if let Err(why) = &vocabulary {
        eprintln!(
            "<4>couldn't read the words in {}, so Screens says ids: {why}",
            path.display()
        );
    } else if !lacked.is_empty() {
        eprintln!(
            "<4>the words in {} lack {}, said by id",
            path.display(),
            lacked.join(", ")
        );
    }
    words
}

/// The account's number settings, or the comma and the full stop where
/// it's handed none, or none it can read, which it says once, here.
#[must_use]
pub fn numbered(path: Option<&Path>) -> Numbers {
    let Some(path) = path else {
        return Numbers::default();
    };
    let read = std::fs::read_to_string(path).map_err(|why| why.to_string());
    match read.and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string())) {
        Ok(settings) => Numbers::read(&settings),
        Err(why) => {
            eprintln!(
                "<4>couldn't read the number settings in {}, so Screens says the usual: {why}",
                path.display()
            );
            Numbers::default()
        }
    }
}
