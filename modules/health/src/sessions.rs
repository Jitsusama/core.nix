//! What the agents' sessions say about the tools and about Joel: a tool
//! call that failed or was refused, the same call made twice running, and
//! a line Joel typed that sounds like something hurt. Each session's log
//! is a line of JSON a message, as pi writes it.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::Value;

/// Reads the sessions' logs, a folder each place they ran, on from where
/// each was left, so a day of a gigabyte of them is read once.
pub struct Reader {
    folder: PathBuf,
    /// Each log's last tool call, so a call made twice running is heard
    /// though a tick falls between them.
    last_calls: BTreeMap<String, Option<String>>,
}

impl Reader {
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_owned(),
            last_calls: BTreeMap::new(),
        }
    }

    /// Read on in each log written to `today`, from the offsets `read`
    /// keeps, `budget` bytes in all at most, handing each of today's
    /// lines heard to `heard` with the place its session ran. A log that
    /// can't be read is left for the next time. Say whether any was read.
    pub fn read_on(
        &mut self,
        read: &mut BTreeMap<String, u64>,
        budget: u64,
        today: &impl Fn(i64) -> bool,
        mut heard: impl FnMut(&str, Heard),
    ) -> bool {
        let mut left = budget;
        let mut any = false;
        for folder in fs::read_dir(&self.folder).into_iter().flatten().flatten() {
            let place = place(&folder.file_name().to_string_lossy());
            let logs = fs::read_dir(folder.path()).into_iter().flatten().flatten();
            for log in logs.filter(|log| is_log_of(log, today)) {
                if left == 0 {
                    return any;
                }
                let key = log.path().to_string_lossy().into_owned();
                let last = self.last_calls.entry(key.clone()).or_default();
                let from = read.get(&key).copied().unwrap_or(0);
                let Ok((to, lines)) = read_lines(&log.path(), from, left, budget) else {
                    continue;
                };
                for line in lines.lines() {
                    if let Some(said) = hear(line, last).filter(|said| today(said.at)) {
                        heard(&place, said);
                    }
                }
                left -= to.saturating_sub(from).min(left);
                any |= to != from;
                read.insert(key, to);
            }
        }
        any
    }
}

/// Whether `log` is a session's log written to `today`.
fn is_log_of(log: &fs::DirEntry, today: &impl Fn(i64) -> bool) -> bool {
    let path = log.path();
    path.extension().is_some_and(|kind| kind == "jsonl")
        && log
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .and_then(|since| i64::try_from(since.as_micros()).ok())
            .is_some_and(today)
}

/// Read a log on from `from`, `budget` bytes at most, to its last whole
/// line: where that leaves it, and the lines. A log shorter than `from`
/// was begun again, and is read from its start; a line longer than a
/// whole tick's `most` is passed over.
fn read_lines(path: &Path, from: u64, budget: u64, most: u64) -> io::Result<(u64, String)> {
    let mut file = fs::File::open(path)?;
    let from = if from > file.metadata()?.len() {
        0
    } else {
        from
    };
    file.seek(SeekFrom::Start(from))?;
    let mut chunk = Vec::new();
    file.take(budget).read_to_end(&mut chunk)?;
    let whole = match chunk.iter().rposition(|&byte| byte == b'\n') {
        Some(end) => end + 1,
        None if chunk.len() as u64 == most => return Ok((from + most, String::new())),
        None => return Ok((from, String::new())),
    };
    chunk.truncate(whole);
    Ok((
        from + whole as u64,
        String::from_utf8_lossy(&chunk).into_owned(),
    ))
}

/// What one session log line said that the day counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// A tool's call failed, and the line that says why.
    Failed { tool: String, why: String },
    /// A tool refused the call, and the line that says why.
    Refused { tool: String, why: String },
    /// A tool was called again exactly as the call before it.
    Again { tool: String },
    /// Joel said something that sounds like it hurt.
    Words { text: String },
}

/// One thing said, and when, in microseconds since the epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heard {
    pub at: i64,
    pub said: Said,
}

/// Words that, in a line Joel typed, say something slowed or failed him.
const PAINED: [&str; 24] = [
    "fuck",
    "wtf",
    "why is",
    "why are",
    "why does",
    "why do ",
    "why can't",
    "still not",
    "again?",
    "broken",
    "doesn't work",
    "does not work",
    "not working",
    "isn't working",
    "slow",
    "forever",
    "stuck",
    "lost my trust",
    "not making",
    "no sense",
    "too hard",
    "annoying",
    "frustrat",
    "confus",
];

/// What sets a refusal apart from a tool's own failure: the gate's words.
const REFUSALS: [&str; 2] = ["Blocked:", "⊘"];

/// Hear one line of a session's log. `last` is the session's last tool
/// call, kept between lines so a call made twice running is heard.
#[must_use]
pub fn hear(line: &str, last: &mut Option<String>) -> Option<Heard> {
    let entry: Value = serde_json::from_str(line).ok()?;
    if entry["type"] != "message" {
        return None;
    }
    let message = &entry["message"];
    let at = message["timestamp"].as_i64()? * 1_000;
    let said = match message["role"].as_str()? {
        "toolResult" => result(message)?,
        "assistant" => again(message, last)?,
        "user" => words(&text_of(message))?,
        _ => return None,
    };
    Some(Heard { at, said })
}

/// A tool's result, when it failed or was refused.
fn result(message: &Value) -> Option<Said> {
    let tool = message["toolName"].as_str()?.to_owned();
    let text = text_of(message);
    if REFUSALS.iter().any(|refusal| text.starts_with(refusal)) {
        let why = text.lines().next().unwrap_or_default();
        return Some(Said::Refused {
            tool,
            why: why.chars().take(400).collect(),
        });
    }
    if message["isError"] != true {
        return None;
    }
    Some(Said::Failed {
        tool,
        why: why_failed(&text).chars().take(400).collect(),
    })
}

/// The line of a failure that says why: the first that names an error,
/// else the last, where a command's exit is said.
fn why_failed(text: &str) -> &str {
    const NAMES: [&str; 8] = [
        "error",
        "cannot",
        "No such",
        "not found",
        "denied",
        "refused",
        "failed",
        "timed out",
    ];
    let lines = || text.lines().map(str::trim).filter(|line| !line.is_empty());
    lines()
        .find(|line| NAMES.iter().any(|name| line.contains(name)))
        .or_else(|| lines().next_back())
        .unwrap_or_default()
}

/// A tool call the same as the session's call before it.
fn again(message: &Value, last: &mut Option<String>) -> Option<Said> {
    let mut heard = None;
    for call in message["content"].as_array()? {
        if call["type"] != "toolCall" {
            continue;
        }
        let tool = call["name"].as_str().unwrap_or_default();
        let made = format!("{tool} {}", call["arguments"]);
        if last.as_deref() == Some(made.as_str()) {
            heard = Some(Said::Again {
                tool: tool.to_owned(),
            });
        }
        *last = Some(made);
    }
    heard
}

/// A line Joel typed, when it sounds like something hurt. A line the
/// harness wrote opens with a bracket, except where Joel steered a session
/// from outside it.
fn words(text: &str) -> Option<Said> {
    const STEERED: &str = "[Steered by joel from outside";
    let typed = if text.starts_with(STEERED) {
        text.split_once(']').map_or("", |(_, said)| said).trim()
    } else if text.starts_with('[') {
        return None;
    } else {
        text.trim()
    };
    let lower = typed.to_lowercase();
    PAINED
        .iter()
        .any(|word| lower.contains(word))
        .then(|| Said::Words {
            text: typed
                .lines()
                .next()
                .unwrap_or_default()
                .chars()
                .take(400)
                .collect(),
        })
}

/// A message's text, its parts joined.
fn text_of(message: &Value) -> String {
    match &message["content"] {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Where a session ran, from its log's folder: pi names it after the
/// working folder, its slashes as dashes. The account's home and its
/// `src` are left off, and a worktree is named by itself.
#[must_use]
pub fn place(folder: &str) -> String {
    let trimmed = folder.trim_matches('-');
    let mut place = trimmed
        .strip_prefix("home-")
        .and_then(|rest| rest.split_once('-').map(|(_, rest)| rest))
        .unwrap_or(if trimmed.starts_with("home-") {
            "home"
        } else {
            trimmed
        });
    if let Some((_, tree)) = place.rsplit_once("-trees-") {
        place = tree;
    }
    place.strip_prefix("src-").unwrap_or(place).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(message: &str) -> String {
        format!(r#"{{"type":"message","message":{message}}}"#)
    }

    #[test]
    fn a_failed_call_says_why() {
        let failed = line(
            r#"{"role":"toolResult","toolName":"bash","isError":true,"timestamp":2,
               "content":[{"type":"text","text":"  427 a.js\nls: cannot access 'b': No such file\n\nCommand exited with code 2"}]}"#,
        );
        assert_eq!(
            hear(&failed.replace('\n', ""), &mut None),
            Some(Heard {
                at: 2_000,
                said: Said::Failed {
                    tool: "bash".into(),
                    why: "ls: cannot access 'b': No such file".into()
                }
            })
        );
        let exited = line(
            r#"{"role":"toolResult","toolName":"bash","isError":true,"timestamp":2,
               "content":[{"type":"text","text":"done\nCommand exited with code 1"}]}"#,
        );
        let Some(Heard {
            said: Said::Failed { why, .. },
            ..
        }) = hear(&exited.replace('\n', ""), &mut None)
        else {
            panic!("an exit that names no error is still a failure");
        };
        assert_eq!(why, "Command exited with code 1", "the exit is why");
        let fine = line(
            r#"{"role":"toolResult","toolName":"bash","isError":false,"timestamp":2,"content":[]}"#,
        );
        assert_eq!(hear(&fine, &mut None), None, "a call that worked");
    }

    #[test]
    fn a_refusal_is_told_from_a_failure() {
        let refused = line(
            r#"{"role":"toolResult","toolName":"review_offer","timestamp":3,
               "content":[{"type":"text","text":"⊘ This body is not yet in shape.\nFix it"}]}"#,
        );
        assert_eq!(
            hear(&refused.replace('\n', ""), &mut None).map(|heard| heard.said),
            Some(Said::Refused {
                tool: "review_offer".into(),
                why: "⊘ This body is not yet in shape.".into()
            })
        );
    }

    #[test]
    fn the_same_call_twice_running_is_heard() {
        let call = |command: &str| {
            line(&format!(
                r#"{{"role":"assistant","timestamp":4,"content":[{{"type":"toolCall","name":"bash","arguments":{{"command":"{command}"}}}}]}}"#
            ))
        };
        let mut last = None;
        assert_eq!(hear(&call("date"), &mut last), None, "the first call");
        assert_eq!(
            hear(&call("date"), &mut last).map(|heard| heard.said),
            Some(Said::Again {
                tool: "bash".into()
            })
        );
        assert_eq!(hear(&call("ls"), &mut last), None, "another call");
    }

    #[test]
    fn joels_words_are_heard_when_they_hurt() {
        let said = |text: &str| {
            line(&format!(
                r#"{{"role":"user","timestamp":5,"content":[{{"type":"text","text":"{text}"}}]}}"#
            ))
        };
        assert_eq!(
            hear(&said("why is THREADS not keeping up"), &mut None).map(|heard| heard.said),
            Some(Said::Words {
                text: "why is THREADS not keeping up".into()
            })
        );
        assert_eq!(hear(&said("what is choose by ear about?"), &mut None), None);
        assert_eq!(
            hear(
                &said("[Automated message, no human input] it is slow"),
                &mut None
            ),
            None,
            "the harness's own words"
        );
        assert_eq!(
            hear(
                &said("[Steered by joel from outside] this is too slow"),
                &mut None
            )
            .map(|heard| heard.said),
            Some(Said::Words {
                text: "this is too slow".into()
            }),
            "his words steered in from outside"
        );
    }

    #[test]
    fn a_place_is_named_by_its_folder() {
        assert_eq!(place("--home-joel-src-core.nix--"), "core.nix");
        assert_eq!(place("--home-joel--"), "home");
        assert_eq!(
            place("--home-joel-.local-state-pi-work-trees-worktree-core.nix-main--"),
            "worktree-core.nix-main"
        );
    }
}
