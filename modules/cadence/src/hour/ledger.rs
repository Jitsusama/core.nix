//! The asks list, read whole or said unread. On 25 November it was
//! written over with three lines that were no asks for four minutes, and
//! an hour reading it then would have counted none of Joel's dates as
//! slipped, so a slip read as zero would have closed as if fixed. Its
//! keeper appends to it and puts it back when it's lost, so a list
//! holding under half the asks read in it before was written over, not
//! answered, until it has stayed that short long enough to have been
//! pruned on purpose.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::measures::Measure;

/// How long the asks list may hold under half its most asks before the
/// hour takes it as pruned rather than written over: one written over is
/// put back in minutes, as it was on 25 November.
const SHORT_LASTS: i64 = 2 * 3600;

/// What the hour has seen of the asks list, kept from one hour to the
/// next by the run that sends the card.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    /// The most asks the hour has read in it.
    #[serde(default)]
    pub most: usize,
    /// Since when it has held under half that many, while it does.
    #[serde(default)]
    pub short_since: Option<i64>,
}

/// The measures read from the asks list, by key, each of Joel's and the
/// list's own, its `-derived` twin.
const FROM_ASKS: [&str; 4] = [
    "slips",
    "slips-parked",
    "ask-to-first-slice",
    "holds-undated",
];

/// The asks list at `path` as read at `now`, or empty with why it can't
/// be read whole, beside what the hour has `seen` of it, which it brings
/// up to date. A list never seen and not there holds no asks yet. One
/// gone or unreadable once seen is unread, as is one holding under half
/// the most asks read in it, until it has held that few for two hours,
/// when it's taken as pruned and counted afresh from what it holds.
pub(super) fn whole(path: &Path, seen: &mut Seen, now: i64) -> (String, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if seen.most == 0 && err.kind() == io::ErrorKind::NotFound => {
            return (String::new(), None);
        }
        Err(err) => return (String::new(), Some(format!("it can't be read: {err}"))),
    };
    let held = asks(&text);
    if held * 2 >= seen.most {
        *seen = Seen {
            most: seen.most.max(held),
            short_since: None,
        };
        return (text, None);
    }
    let since = *seen.short_since.get_or_insert(now);
    if now - since < SHORT_LASTS {
        let most = seen.most;
        let why = format!("it holds {held} asks, under half the {most} read in it");
        return (String::new(), Some(why));
    }
    eprintln!(
        "cadence: the asks list has held {held} asks, under half the {}, for two hours, \
         so it's taken as pruned",
        seen.most
    );
    *seen = Seen {
        most: held,
        short_since: None,
    };
    (text, None)
}

/// How many asks `text` holds: its lines that are records naming when
/// each was asked and in what words, so lines of anything else written
/// over it aren't counted as asks.
fn asks(text: &str) -> usize {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|ask| ask["at"].is_string() && ask["words"].is_string())
        .count()
}

/// `all` with each measure read from the asks list unjudged when the list
/// wasn't read whole, saying `why`, so no count of none stands for his
/// asks and none of them closes as if fixed. With the list whole, `all`
/// is left as it is.
pub(super) fn unless_whole(all: &mut [Measure], why: Option<&str>) {
    let Some(why) = why else {
        return;
    };
    for measure in &mut *all {
        let key = measure.key.strip_suffix("-derived").unwrap_or(&measure.key);
        if FROM_ASKS.contains(&key) {
            measure.target = None;
            measure.says = format!("{} (the asks list unread: {why})", measure.says);
        }
    }
}

#[cfg(test)]
mod tests;
