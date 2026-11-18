//! Whether a pain was taken up, and whether what answered it is on the
//! machine. Each pain goes to its cell as a question naming its log entry.
//! A peer that replies to it, or names it to a peer, has taken the pain
//! up; a reply to that reply carries the thread on, as when the
//! orchestrator passes a pain to the cell that fixes it. A landing in the
//! thread ("merged at" one commit) closes the pain once a switch carries
//! the commit, since only then does Joel have the fix.
//!
//! The thread grows only by replies, and a note to a group takes nothing
//! up: a status note names many messages, and following one would close
//! pains its landings never touched. A question that held several pains
//! is taken up by an answer but closed by no landing, since one rarely
//! fixes them all; and the keeper's note of new pains asks nothing of
//! anyone, so says nothing of any.

use super::heard::Heard;
use super::route::FOR_THE_KEEPER;
use super::{ASK_AT_MOST, DAY_MS, Ears, ended_within, stalled};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

/// The most of a message file read: the mesh's are a few kilobytes, and a
/// run is held to 256 megabytes.
const READ_AT_MOST: u64 = 256 * 1024;

/// A message on the mesh, as its file says it.
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    #[serde(default)]
    pub to: Vec<String>,
    #[serde(default)]
    pub re: Option<String>,
    #[serde(default)]
    pub text: String,
}

/// Who took a pain up, and the commit that closed it, if one has.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub taken_by: Option<String>,
    pub closed_by: Option<String>,
}

/// Takes up and closes, from the mesh and the machine, each pain of the
/// last day still open to it; `asker` is who sent the questions. Says why
/// none can close when the core the machine runs, or the git to ask of
/// it, can't be read, so a fix that never closes doesn't read as one not
/// on the machine yet.
pub(super) fn stand(ears: &Ears, asker: Option<&str>, kept: &mut [Heard]) -> Option<String> {
    let still = |h: &Heard| h.closed_by.is_none() && !stalled::is(h) && h.at >= ears.at - DAY_MS;
    let since = kept
        .iter()
        .filter(|h| still(h))
        .filter_map(|h| h.routed_at)
        .min();
    let (Some(asker), Some(since)) = (asker, since) else {
        return None;
    };
    let messages = messages(&ears.messages, since - 60_000);
    let core = core_on(&fs::read_to_string(&ears.switches).unwrap_or_default());
    let mut unclosed = match &core {
        None => Some(format!(
            "no switch's core is in {}",
            ears.switches.display()
        )),
        Some(_) if !ears.core.is_dir() => Some(format!("no core is at {}", ears.core.display())),
        Some(_) => None,
    };
    let entries: Vec<String> = kept.iter().map(|h| h.entry.clone()).collect();
    let held: Vec<&str> = entries.iter().map(String::as_str).collect();
    let mut known = BTreeMap::new();
    let mut on = |commit: &str| {
        let (Some(core), None) = (&core, &unclosed) else {
            return false;
        };
        *known.entry(commit.to_owned()).or_insert_with(|| {
            let git = Command::new(&ears.git)
                .arg("--git-dir")
                .arg(&ears.core)
                .args(["merge-base", "--is-ancestor", commit, core])
                // The core's own settings alone, never the user's.
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            match git {
                Ok(mut child) => ended_within(&mut child, ASK_AT_MOST),
                Err(e) => {
                    unclosed = Some(format!("{} wouldn't run: {e}", ears.git.display()));
                    false
                }
            }
        })
    };
    for h in kept.iter_mut().filter(|h| still(h)) {
        let found = standing(h, asker, &held, &messages, &mut on);
        h.taken_by = h.taken_by.take().or(found.taken_by);
        h.closed_by = found.closed_by;
    }
    unclosed
}

/// How a pain stands from the thread of the question `asker` sent of it.
/// `held` are the entries of every pain kept, to tell a question that held
/// this one alone; `messages` are in the order they were sent; `on` says
/// whether the machine runs a commit.
pub fn standing(
    pain: &Heard,
    asker: &str,
    held: &[&str],
    messages: &[Message],
    on: &mut dyn FnMut(&str) -> bool,
) -> Standing {
    let names = |text: &str, entry: &str| text.contains(&format!("#{entry}"));
    let Some((at, first)) = messages.iter().enumerate().find_map(|(at, m)| {
        let question = m.from.eq_ignore_ascii_case(asker)
            && names(&m.text, &pain.entry)
            && !m.text.contains(FOR_THE_KEEPER);
        question.then(|| short(&m.id).map(|id| (at, id)))?
    }) else {
        return Standing::default();
    };
    let alone = held.iter().filter(|e| names(&messages[at].text, e)).count() == 1;
    let mut thread = BTreeSet::from([first]);
    let mut standing = Standing::default();
    for m in &messages[at + 1..] {
        if m.from.eq_ignore_ascii_case(asker) {
            continue;
        }
        let replies =
            m.re.as_deref()
                .and_then(short)
                .is_some_and(|re| thread.contains(re));
        let names_it = thread.iter().any(|id| m.text.contains(id));
        if replies || (names_it && m.to.iter().any(|to| to.starts_with('@'))) {
            standing.taken_by.get_or_insert_with(|| m.from.clone());
        }
        // A note of two landings says which fixed nothing in particular.
        if let [commit] = merged_at(&m.text)[..]
            && alone
            && (replies || names_it)
            && standing.closed_by.is_none()
            && on(commit)
        {
            standing.closed_by = Some(commit.to_owned());
        }
        if let (true, Some(id)) = (replies, short(&m.id)) {
            thread.insert(id);
        }
    }
    standing
}

/// A message id's short form, its last eight hex letters, as peers name
/// it; an id of another shape is none, since a short form of a letter or
/// two would be found in every message.
fn short(id: &str) -> Option<&str> {
    let last = id.rsplit('-').next()?;
    (last.len() == 8 && last.bytes().all(|b| b.is_ascii_hexdigit())).then_some(last)
}

/// Whether text is shaped as a commit, so nothing else reaches git.
fn commit_like(text: &str) -> bool {
    (7..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Each commit a message says something merged at; a time said the same
/// way ("merged at 20:28") is no commit.
fn merged_at(text: &str) -> Vec<&str> {
    text.match_indices("merged at ")
        .filter_map(|(i, said)| {
            text[i + said.len()..]
                .split(|c: char| !c.is_ascii_hexdigit())
                .next()
                .filter(|c| commit_like(c))
        })
        .collect()
}

/// The messages sent from `since_ms` on, in the order they were sent:
/// the folder's own and its days', by the time each file's name opens
/// with. What's being written, a link and a file too big to be a message
/// are passed over, and so is a day's folder last written to before.
pub fn messages(folder: &Path, since_ms: i64) -> Vec<Message> {
    let mut found = Vec::new();
    let mut gather = |dir: &Path| {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let sent = name.split('-').next().and_then(|ms| ms.parse::<i64>().ok());
            let json = entry.path().extension().is_some_and(|ext| ext == "json");
            if name.starts_with('.') || !json || sent < Some(since_ms) {
                continue;
            }
            // A dir entry's type and size are its own, never a link's.
            let plain = entry.file_type().is_ok_and(|t| t.is_file())
                && entry.metadata().is_ok_and(|m| m.len() <= READ_AT_MOST);
            if let Some(m) = plain
                .then(|| fs::read(entry.path()).ok())
                .flatten()
                .and_then(|bytes| serde_json::from_slice::<Message>(&bytes).ok())
            {
                found.push((sent, m));
            }
        }
    };
    gather(folder);
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let written = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_millis()).ok());
        let day = entry.file_type().is_ok_and(|t| t.is_dir()) && name.starts_with("20");
        if day && written >= Some(since_ms) {
            gather(&entry.path());
        }
    }
    found.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    found.into_iter().map(|(_, m)| m).collect()
}

/// The core the machine switched to last, from integration's switches; a
/// rev not shaped as a commit is passed over.
pub fn core_on(switches: &str) -> Option<String> {
    switches
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|v| {
            v["revs"]["core"]
                .as_str()
                .filter(|c| commit_like(c))
                .map(str::to_owned)
        })
}

#[cfg(test)]
mod tests;
