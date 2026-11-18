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
//! pains its landings never touched. A landing that isn't a reply closes
//! a pain only when the one message it names is in its thread, as a
//! landing note names the pivot it answers. A question that held several pains
//! is taken up by an answer but closed by no landing, since one rarely
//! fixes them all; and the keeper's note of new pains asks nothing of
//! anyone, so says nothing of any.

use super::heard::Heard;
use super::route::FOR_THE_KEEPER;
use super::{ASK_AT_MOST, DAY_MS, Ears, stalled};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

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
/// none can close when the messages, the core the machine runs, or the
/// git to ask of it can't be read, so a fix that never closes doesn't
/// read as one not on the machine yet.
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
        _ if !ears.messages.is_dir() => {
            Some(format!("no messages are at {}", ears.messages.display()))
        }
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
        if let Some(&known) = known.get(commit) {
            return known;
        }
        let child = Command::new(&ears.git)
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
        let git = ears.git.display();
        match git_said(child, ASK_AT_MOST) {
            // An ancestor (0), or not (1); a commit the core doesn't hold
            // (128) is another repository's, never on.
            Ok(exit @ (0 | 1 | 128)) => *known.entry(commit.to_owned()).or_insert(exit == 0),
            Ok(exit) => {
                unclosed = Some(format!("{git} said {exit} of {commit} in {core}"));
                false
            }
            Err(why) => {
                unclosed = Some(format!("{git} {why}"));
                false
            }
        }
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
    // Each question naming the pain starts its thread, since a send that
    // timed out after the node took it is sent again.
    let questions: Vec<(usize, &str)> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.from.eq_ignore_ascii_case(asker)
                && names(&m.text, &pain.entry)
                && !m.text.contains(FOR_THE_KEEPER)
        })
        .filter_map(|(at, m)| short(&m.id).map(|id| (at, id)))
        .collect();
    let Some(&(at, _)) = questions.first() else {
        return Standing::default();
    };
    let alone = questions
        .iter()
        .all(|&(at, _)| held.iter().filter(|e| names(&messages[at].text, e)).count() == 1);
    let ids: BTreeSet<&str> = messages.iter().filter_map(|m| short(&m.id)).collect();
    let mut thread: BTreeSet<&str> = questions.iter().map(|&(_, id)| id).collect();
    let mut standing = Standing::default();
    for m in &messages[at + 1..] {
        if m.from.eq_ignore_ascii_case(asker) {
            continue;
        }
        let replies =
            m.re.as_deref()
                .and_then(short)
                .is_some_and(|re| thread.contains(re));
        let named: BTreeSet<&str> = words(&m.text).filter(|w| ids.contains(w)).collect();
        let names_it = named.iter().any(|id| thread.contains(id));
        if replies || (names_it && m.to.iter().any(|to| to.starts_with('@'))) {
            standing.taken_by.get_or_insert_with(|| m.from.clone());
        }
        // A note of two landings says which fixed nothing in particular,
        // and one naming several messages which it answers.
        if let [commit] = merged_at(&m.text)[..]
            && alone
            && (replies || (names_it && named.len() == 1))
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

/// Whether text names a log entry as `#entry`, and not one that begins
/// with it.
fn names(text: &str, entry: &str) -> bool {
    let tag = format!("#{entry}");
    text.match_indices(&tag).any(|(i, _)| {
        !text[i + tag.len()..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
    })
}

/// The words of a text that could be a message's short form.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() == 8)
}

/// What a git asked of an ancestry says by its exit, within `limit`; one
/// that won't start, is killed by a signal or runs past it says why.
fn git_said(child: std::io::Result<Child>, limit: Duration) -> Result<i32, String> {
    let mut child = child.map_err(|e| format!("wouldn't run: {e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code().ok_or_else(|| format!("ended by {status}")),
            Ok(None) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("ran past {} s", limit.as_secs()));
            }
        }
    }
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
/// with. A hidden file has no time to open with, so isn't read; a link
/// and a file too big to be a message are passed over, and so is a day's
/// folder last written to before.
pub fn messages(folder: &Path, since_ms: i64) -> Vec<Message> {
    let mut found = Vec::new();
    let mut gather = |dir: &Path| {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let sent = name.split('-').next().and_then(|ms| ms.parse::<i64>().ok());
            let json = entry.path().extension().is_some_and(|ext| ext == "json");
            if !json || sent < Some(since_ms) {
                continue;
            }
            // A dir entry's type is its own, never a link's; what's read
            // is held to the limit however the file grew since.
            let plain = entry.file_type().is_ok_and(|t| t.is_file());
            let mut bytes = Vec::new();
            let read = plain
                && fs::File::open(entry.path())
                    .and_then(|f| f.take(READ_AT_MOST + 1).read_to_end(&mut bytes))
                    .is_ok_and(|n| u64::try_from(n).is_ok_and(|n| n <= READ_AT_MOST));
            if let Some(m) = read
                .then(|| serde_json::from_slice::<Message>(&bytes).ok())
                .flatten()
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
