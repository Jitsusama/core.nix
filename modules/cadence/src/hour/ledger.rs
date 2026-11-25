//! The asks list, read whole or said unread. On 25 November it was
//! written over with three lines that were no asks for four minutes, and
//! an hour reading it then would have counted none of Joel's dates as
//! slipped, so a slip read as zero would have closed as if fixed. Its
//! keeper only appends to it, so a list holding under half the lines read
//! in it before was written over, not answered, and a missing one says
//! nothing of his asks either.

use std::fs;
use std::path::Path;

use super::measures::Measure;

/// Where cadence keeps, in its folder, the most lines it has read in the
/// asks list.
const MOST: &str = "asks-most-lines";

/// The measures read from the asks list, by key, each of Joel's and the
/// list's own, its `-derived` twin.
const FROM_ASKS: [&str; 4] = [
    "slips",
    "slips-parked",
    "ask-to-first-slice",
    "holds-undated",
];

/// The asks list at `path`, with why it can't be read whole when it
/// can't: missing, unreadable, or holding under half the most lines read
/// in it before, as kept in `folder`. Unread, the list reads as empty. A
/// whole read that holds more lines than any before keeps their count.
pub(super) fn whole(path: &Path, folder: &Path) -> (String, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => return (String::new(), Some(format!("it can't be read: {err}"))),
    };
    let held = text.lines().filter(|line| !line.trim().is_empty()).count();
    let kept = folder.join(MOST);
    let most = fs::read_to_string(&kept)
        .ok()
        .and_then(|most| most.trim().parse::<usize>().ok())
        .unwrap_or_default();
    if held * 2 < most {
        let why = format!("it holds {held} lines, under half the {most} read in it before");
        return (String::new(), Some(why));
    }
    if held > most
        && let Err(err) = fs::write(&kept, held.to_string())
    {
        eprintln!("cadence: the asks list's {held} lines weren't kept: {err}");
    }
    (text, None)
}

/// `all` with each measure read from the asks list said unread when the
/// list wasn't read whole, for `why`, so no count of none stands for his
/// asks and none of them is told fixed. With the list whole, `all` is
/// left as it is.
pub(super) fn unless_whole(all: &mut [Measure], why: Option<&str>) {
    let Some(why) = why else {
        return;
    };
    for measure in all.iter_mut() {
        let key = measure.key.strip_suffix("-derived").unwrap_or(&measure.key);
        if FROM_ASKS.contains(&key) {
            measure.now = 0.0;
            measure.target = None;
            measure.behind = vec![format!("the asks list wasn't read whole: {why}")];
        }
    }
}

#[cfg(test)]
mod tests;
