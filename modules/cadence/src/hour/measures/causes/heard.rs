//! Reading the mesh's messages for the causes measure: which file holds
//! which message, and who sent it.

use std::path::Path;

use super::super::misses::symbiosis;
use crate::mesh::Message;

/// Whether `message` is the one its file `name` names, sent by whom its
/// id says, so no file can stand in for another message or sender. It
/// checks that a file agrees with itself, not who wrote it: every writer
/// to the mesh is the same local user, so a forger who writes the id and
/// sender alike passes, until the mesh signs what it carries.
fn genuine(message: &Message, name: &str) -> bool {
    name.strip_suffix(".json") == Some(message.id.as_str())
        && sender_of(&message.id) == Some(message.from.as_str())
}

/// The message the mesh's file at `path`, named `name`, holds, when it is
/// genuine. One the harness moved after it was listed is let go without
/// a word, since the harness moves only what is old, read already or
/// expired. One that can't be read, or doesn't parse, is said on stderr
/// and left out. The next hour lists it again only while it was written
/// within `SLACK_MS` of that hour's mark, so one caught as it was written
/// still counts; one that stays unreadable past then is lost, said only
/// on stderr.
pub(super) fn message_in(path: &Path, name: &str) -> Option<Message> {
    let parsed = match symbiosis::opened_at_most(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        opened => opened.map_err(|error| error.to_string()).and_then(|json| {
            serde_json::from_slice::<Message>(&json).map_err(|error| error.to_string())
        }),
    };
    match parsed {
        Ok(message) => genuine(&message, name).then_some(message),
        Err(why) => {
            eprintln!(
                "cadence: the mesh's {} was left out of the causes read ({why})",
                path.display()
            );
            None
        }
    }
}

/// Who sent the message `id` names, as `<ms>-<from>-<pid>-<short id>`.
pub(super) fn sender_of(id: &str) -> Option<&str> {
    id.split('-').nth(1)
}
