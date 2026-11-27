//! The asks list, read whole or said unread. On 25 November it was
//! written over with three lines that were no asks for four minutes, and
//! an hour reading it then would have counted none of Joel's dates as
//! slipped, so a slip read as zero would have closed as if fixed. Its
//! keeper updates its records in place and adds new ones, dropping none,
//! and puts it back when it's lost, so a list holding fewer asks than
//! were read in it before was written over, not answered, until it has
//! stayed that short long enough to have been pruned on purpose.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::measures::Measure;
use crate::whole::text_lines;

/// How long the asks list may hold fewer than its most asks before the
/// hour takes it as pruned rather than written over: one written over is
/// put back in minutes, as it was on 25 November.
const SHORT_LASTS: i64 = 2 * 3600;

/// What the hour has seen of the asks list, kept from one hour to the
/// next by the run that sends the card.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    /// The most ask records the hour has read in it.
    #[serde(default)]
    pub most: usize,
    /// Since when it has held fewer than that many, while it does.
    #[serde(default)]
    pub short_since: Option<i64>,
}

/// The measures read from the asks list, by key, each of Joel's and the
/// list's own, its `-derived` twin: each reads an unread list's empty text
/// as none of his dates slipped, at risk or told late.
const FROM_ASKS: [&str; 6] = [
    "slips",
    "slips-parked",
    "ask-to-first-slice",
    "holds-undated",
    "dates-at-risk",
    "dates-told-late",
];

/// The asks list at `path` as read at `now`, or empty with why it can't
/// be read whole, beside what the hour has `seen` of it, which it brings
/// up to date. A list never seen and not there holds no asks yet. One
/// gone or unreadable once seen is unread until it can be read again,
/// since its keeper puts it back. One holding fewer than the most asks
/// read in it is unread too, as its keeper updates records in place and
/// adds new ones but drops none, until it has held that few for two hours,
/// when it's taken as pruned and counted afresh from what it holds. One
/// holding none is never taken as pruned, as no pruning keeps nothing,
/// and the hours it held none don't count toward pruning it after, so an
/// ask added to a list written over starts the two hours afresh. A line
/// that isn't text, or isn't a record, leaves it unread, naming the first,
/// since the measures read each ask from its latest record, and leaving
/// one out could close a slip as fixed. Its keeper writes the list whole
/// and moves it into place, so no read catches a line half written, and
/// one cut short, the last included, is a tear its keeper must mend. No
/// hour the list is unread counts toward pruning it, as none saw it short,
/// and each says why in the journal.
pub(super) fn whole(path: &Path, seen: &mut Seen, now: i64) -> (String, Option<String>) {
    let why = match fs::read(path).map(|bytes| text_lines(&bytes)) {
        Ok((text, None)) => match torn(&text) {
            None => return short(text, seen, now),
            Some(line) => format!("its line {line} isn't a record"),
        },
        // The lines before the first that isn't text are numbered as
        // written, so one of them that isn't a record is named first.
        Ok((text, Some(line))) => match torn(&text).filter(|torn| *torn < line) {
            Some(torn) => format!("its line {torn} isn't a record"),
            None => format!("its line {line} isn't text"),
        },
        Err(err) if seen.most == 0 && err.kind() == io::ErrorKind::NotFound => {
            return (String::new(), None);
        }
        Err(err) => format!("it can't be read: {err}"),
    };
    seen.short_since = None;
    eprintln!("cadence: the asks list is unread, as {why}, so what it feeds is unjudged");
    (String::new(), Some(why))
}

/// The number of the first line of `text` that isn't a record, counted
/// from one, so a write torn within its text, and any record written on
/// after it, isn't passed over unsaid. Every record is an object; blank
/// lines, as after the last newline, are none of his asks.
fn torn(text: &str) -> Option<usize> {
    let record = |line: &str| serde_json::from_str::<Value>(line).is_ok_and(|v| v.is_object());
    text.split('\n')
        .position(|line| !line.trim().is_empty() && !record(line))
        .map(|n| n + 1)
}

/// `text`, the asks list read whole, beside why it's unread when it holds
/// fewer than the most ask records `seen` in it, as `whole` says.
fn short(text: String, seen: &mut Seen, now: i64) -> (String, Option<String>) {
    let held = asks(&text);
    if held >= seen.most {
        *seen = Seen {
            most: held,
            short_since: None,
        };
        return (text, None);
    }
    if held == 0 {
        seen.short_since = None;
        let most = seen.most;
        eprintln!(
            "cadence: the asks list holds no ask records, where {most} were read in it, so \
             it's said unread until its keeper puts it back, or an ask added to it has held \
             short for two hours"
        );
        let why = format!("it holds no ask records, where {most} were read in it");
        return (String::new(), Some(why));
    }
    let since = *seen.short_since.get_or_insert(now);
    if now - since < SHORT_LASTS {
        let most = seen.most;
        let why = format!("it holds {held} ask records, fewer than the {most} read in it");
        return (String::new(), Some(why));
    }
    eprintln!(
        "cadence: the asks list has held {held} ask records, fewer than the {}, for two \
         hours, so it's taken as pruned",
        seen.most
    );
    *seen = Seen {
        most: held,
        short_since: None,
    };
    (text, None)
}

/// How many ask records `text` holds: its lines naming when an ask was
/// asked and in what words, so lines of anything else written over it
/// aren't counted. This counts records, one a line, not asks, as the
/// card's reason says.
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
