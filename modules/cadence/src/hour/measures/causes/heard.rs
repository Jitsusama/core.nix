//! Reading the mesh's messages for the causes measure: which file holds
//! which message, who sent it, and where the next hour reads on from.

use std::path::Path;

use super::super::misses::symbiosis::{self, Listed};
use crate::mesh::Message;

/// What one file listed gave.
#[derive(Debug)]
pub(super) enum Read {
    /// A genuine message, boxed so a read that gives none isn't a
    /// message's size too.
    Message(Box<Message>),
    /// Nothing to count, and nothing to read again: a file the harness
    /// moved, one that isn't a message, or one naming another.
    LeftOut,
    /// A file that couldn't be read, which the next hour reads again.
    Unread,
}

/// Whether `message` is the one its file `name` names, sent by whom its
/// id says, so no file can stand in for another message or sender. It
/// checks that a file agrees with itself, not who wrote it: every writer
/// to the mesh is the same local user, so a forger who writes the id and
/// sender alike passes, until the mesh signs what it carries.
fn genuine(message: &Message, name: &str) -> bool {
    name.strip_suffix(".json") == Some(message.id.as_str())
        && sender_of(&message.id) == Some(message.from.as_str())
}

/// The genuine messages the files `listed` hold, each with when this
/// machine wrote it, in ms, and the mark the next hour reads on from, as
/// of `now_ms`. The mark is now, unless a file couldn't be read: then it
/// is held back to just before the earliest such was written, as
/// symbiosis holds its own, so the next hour lists it again however far
/// past `SLACK_MS` of the mark it falls, rather than losing its lines for
/// good. It is held back no further than `since_ms`, the week's start,
/// since a file older than that counts nothing, so one that stays
/// unreadable costs a week's files each hour only while it could count.
pub(super) fn opened(
    listed: Vec<Listed>,
    now_ms: i64,
    since_ms: i64,
) -> (Vec<(Message, i64)>, i64) {
    let mut held: Option<i64> = None;
    let mut heard = Vec::new();
    for (path, name, written) in listed {
        match message_in(&path, &name) {
            Read::Message(message) => heard.push((*message, written)),
            Read::LeftOut => {}
            Read::Unread => {
                let at = written.min(now_ms);
                held = Some(held.map_or(at, |held| held.min(at)));
            }
        }
    }
    let through_ms = held.map_or(now_ms, |at| (at - 1).max(since_ms));
    (heard, through_ms)
}

/// What the mesh's file at `path`, named `name`, gives. One the harness
/// moved after it was listed is let go without a word, since the harness
/// moves only what is old, read already or expired. One that can't be
/// read is said on stderr and unread, so the next hour reads it again.
/// One that doesn't parse, or isn't the message it names, is left out,
/// since reading it again reads the same bytes.
pub(super) fn message_in(path: &Path, name: &str) -> Read {
    let json = match symbiosis::opened_at_most(path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Read::LeftOut,
        Err(error) => {
            eprintln!(
                "cadence: the mesh's {} couldn't be read for the causes, so the next hour \
                 reads it again ({error})",
                path.display()
            );
            return Read::Unread;
        }
    };
    match serde_json::from_slice::<Message>(&json) {
        Ok(message) if genuine(&message, name) => Read::Message(Box::new(message)),
        Ok(_) => Read::LeftOut,
        Err(error) => {
            eprintln!(
                "cadence: the mesh's {} was left out of the causes read ({error})",
                path.display()
            );
            Read::LeftOut
        }
    }
}

/// Who sent the message `id` names, as `<ms>-<from>-<pid>-<short id>`.
pub(super) fn sender_of(id: &str) -> Option<&str> {
    id.split('-').nth(1)
}
