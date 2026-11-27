//! The sessions' model calls as pi met them, whatever route each took,
//! from the assistant turns in each session's log. On 26 November the
//! model relay was stopped at 18:49 and the relay's measures read nought
//! while 13 to 33 per cent of pi's calls failed through the evening on
//! "other side closed" and timeouts, as TLS to the origin took 6 to 10 s
//! through a starved WARP lane (WCJQSG, 720d7b2f). Two counts of the hour
//! say it, each held to a ceiling: the calls that failed, and their share
//! of every call; healthy hours that day ran 0 to 1 per cent of about
//! 5,000 calls.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde_json::Value;

use super::super::{Class, Measure, count};

/// What each of the two is known by opens so, which files each apart with
/// the model path's keeper, whatever leads the hour (owing.rs).
pub const PREFIX: &str = "calls:";

/// The model calls that failed in the hour.
pub const FAILED: &str = "calls:failed";
/// The failed calls as a share of every call in the hour.
pub const SHARE: &str = "calls:failed-share";

/// Who keeps the model path, and so moves each of the two.
pub const OWNER: &str = "OXUW1Y";

/// The ceilings: a healthy hour on 26 November failed under 1 per cent of
/// about 5,000 calls, so 2 per cent, or 100 calls, is a path failing.
pub const FAILED_AT_MOST: f64 = 100.0;
pub const SHARE_AT_MOST: f64 = 2.0;

/// How many causes the card shows behind each count, and the most of one
/// it carries, since a cause is another machine's error text.
const CAUSES_SHOWN: usize = 3;
const CAUSE_AT_MOST: usize = 120;

/// How much of a log is read back from its end in one step.
const STEP: u64 = 64 * 1024;

/// The most of one log read back for an hour. A busy session wrote about
/// 4 MB of log an hour on 26 November; one past this is read only this
/// far, so a log of tool output can't hold the hour up.
const READ_AT_MOST: u64 = 64 * 1024 * 1024;

/// What marks a log line as an assistant's message, as pi writes it.
const ASSISTANT: &[u8] = b"\"role\":\"assistant\"";

/// One assistant turn: when it ended, in milliseconds, and the error it
/// stopped on, if it did.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    pub at_ms: i64,
    pub error: Option<String>,
}

/// The two measures of the hour to `now`, from the logs under `sessions`.
#[must_use]
pub fn hour(sessions: &Path, now: i64) -> Vec<Measure> {
    let turns = read(sessions, now)
        .inspect_err(|e| eprintln!("cadence: the sessions' logs: {e}"))
        .ok();
    calls(turns.as_deref(), now)
}

/// Every assistant turn of the hour to `now`, from each log under
/// `sessions` (one folder for each place a session started) written in
/// the hour, each read back from its end only to the hour's start.
///
/// # Errors
///
/// When `sessions` can't be listed.
pub fn read(sessions: &Path, now: i64) -> io::Result<Vec<Turn>> {
    let since = now - 3600;
    let mut turns = Vec::new();
    for folder in fs::read_dir(sessions)?.flatten() {
        let Ok(logs) = fs::read_dir(folder.path()) else {
            continue;
        };
        for log in logs.flatten() {
            let path = log.path();
            // Only a plain file is read, so a pipe or device named like a
            // log can never hold the hour waiting on it.
            let written = fs::metadata(&path)
                .ok()
                .filter(fs::Metadata::is_file)
                .and_then(|m| m.modified().ok())
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .and_then(|at| i64::try_from(at.as_secs()).ok());
            if path.extension().is_some_and(|e| e == "jsonl") && written >= Some(since) {
                // A log that can't be read is one session's, not the hour's.
                turns.extend(back_to(&path, since * 1000).unwrap_or_default());
            }
        }
    }
    Ok(turns)
}

/// The assistant turns of the log at `path` from `since_ms` on, read back
/// from its end a step at a time until a turn before it.
fn back_to(path: &Path, since_ms: i64) -> io::Result<Vec<Turn>> {
    let mut file = File::open(path)?;
    let mut end = file.seek(SeekFrom::End(0))?;
    let mut read = 0;
    // The line that runs on past the steps read so far, as its pieces from
    // the log's end back, so each piece is copied once, into its whole line
    // when that line's start is found, however many steps it spans.
    let mut later: Vec<Vec<u8>> = Vec::new();
    let mut turns = Vec::new();
    while end > 0 && read < READ_AT_MOST {
        let step = STEP.min(end);
        end -= step;
        file.seek(SeekFrom::Start(end))?;
        let mut chunk = vec![0; usize::try_from(step).unwrap_or(0)];
        file.read_exact(&mut chunk)?;
        read += step;
        // What comes before the step's first newline runs on into the step
        // before, unless this is the log's start.
        let (first, rest) = if end == 0 {
            (&[][..], &chunk[..])
        } else if let Some(at) = chunk.iter().position(|b| *b == b'\n') {
            (&chunk[..at], &chunk[at + 1..])
        } else {
            later.push(chunk);
            continue;
        };
        let mut lines: Vec<&[u8]> = rest.split(|b| *b == b'\n').collect();
        let mut last = lines.pop().unwrap_or_default().to_vec();
        for piece in later.drain(..).rev() {
            last.extend_from_slice(&piece);
        }
        for line in std::iter::once(&last[..]).chain(lines.into_iter().rev()) {
            if let Some(turn) = turn(line) {
                if turn.at_ms < since_ms {
                    return Ok(turns);
                }
                turns.push(turn);
            }
        }
        later.push(first.to_vec());
    }
    Ok(turns)
}

/// The turn a log line holds, where it's an assistant's message.
fn turn(line: &[u8]) -> Option<Turn> {
    // Most lines are tool output and other entries; only an assistant's
    // message is parsed.
    if !line.windows(ASSISTANT.len()).any(|w| w == ASSISTANT) {
        return None;
    }
    let entry: Value = serde_json::from_slice(line).ok()?;
    let message = entry.get("message")?;
    let error = (message.get("stopReason")?.as_str()? == "error").then(|| {
        message
            .get("errorMessage")
            .and_then(Value::as_str)
            .and_then(|e| e.lines().next())
            .unwrap_or("an unsaid error")
            .to_owned()
    });
    Some(Turn {
        at_ms: message.get("timestamp")?.as_i64()?,
        error,
    })
}

/// The two measures of the hour to `now`, from its turns, or each
/// unjudged where the logs couldn't be read. The share is unjudged in an
/// hour no session called a model in, as it has nothing to weigh.
#[must_use]
pub fn calls(turns: Option<&[Turn]>, now: i64) -> Vec<Measure> {
    let Some(turns) = turns else {
        return [FAILED, SHARE]
            .into_iter()
            .map(|key| unjudged(key, ": the sessions' logs unread"))
            .collect();
    };
    let hour = (now - 3600) * 1000..now * 1000;
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    let mut all = 0;
    for turn in turns.iter().filter(|t| hour.contains(&t.at_ms)) {
        all += 1;
        if let Some(error) = &turn.error {
            *why.entry(error.clone()).or_default() += 1;
        }
    }
    let failed: usize = why.values().sum();
    let behind = causes(&why);
    let share = if all == 0 {
        unjudged(SHARE, ": no session called a model this hour")
    } else {
        Measure {
            key: SHARE.to_owned(),
            class: Class::Functional,
            says: format!("per cent of the sessions' {all} model calls this hour that failed"),
            now: (count(failed) * 1000.0 / count(all)).round() / 10.0,
            target: Some(SHARE_AT_MOST),
            owner: OWNER.to_owned(),
            behind: behind.clone(),
        }
    };
    vec![
        Measure {
            key: FAILED.to_owned(),
            class: Class::Functional,
            says: "the sessions' model calls that failed this hour".to_owned(),
            now: count(failed),
            target: Some(FAILED_AT_MOST),
            owner: OWNER.to_owned(),
            behind,
        },
        share,
    ]
}

/// The measure `key` unjudged, saying `why`.
fn unjudged(key: &str, why: &str) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Functional,
        says: format!(
            "the sessions' model calls {}{why}",
            key.trim_start_matches(PREFIX)
        ),
        now: 0.0,
        target: None,
        owner: OWNER.to_owned(),
        behind: Vec::new(),
    }
}

/// The commonest causes, most first, each cut short.
fn causes(why: &BTreeMap<String, usize>) -> Vec<String> {
    let mut most: Vec<(&String, &usize)> = why.iter().collect();
    most.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    most.into_iter()
        .take(CAUSES_SHOWN)
        .map(|(cause, n)| {
            let cause: String = cause.chars().take(CAUSE_AT_MOST).collect();
            format!("{n} on {cause}")
        })
        .collect()
}

#[cfg(test)]
mod tests;
