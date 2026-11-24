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
//!
//! A landing that names the pain's own entry, `#entry`, closes it whatever
//! its thread, as its owner says outright what it fixed: the one way to
//! close a pain whose thread was never seen, or whose question held
//! others.
//!
//! A pain stays open until its fix runs, however long that takes, so its
//! thread is kept with it and each run reads only the messages sent since
//! the last: a week of the mesh is some 600 megabytes, too much to read
//! again every five minutes.

use super::heard::{Heard, Landing};
use super::route::FOR_THE_KEEPER;
use super::{ASK_AT_MOST, DAY_MS, Ears, stalled};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

/// The most of a message file read: the mesh's are a few kilobytes, and a
/// run is held to 256 megabytes.
const READ_AT_MOST: u64 = 256 * 1024;

/// How long before the newest message a run read the next reads again,
/// for one whose file came after its sending, as one relayed from another
/// machine.
const LATE_AT_MOST_MS: i64 = 30 * 60_000;

/// The most of a landing's note kept, its first line, to say what fixed a
/// pain.
const SAID_AT_MOST: usize = 300;

/// Where a run's reading of the mesh is kept between runs.
const READ_FILE: &str = "read.json";

/// Where the last run's reading of the mesh ended: the sending of the
/// newest message it read, and each message it read since
/// `LATE_AT_MOST_MS` before then, so a late one is read once.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Read {
    pub at: i64,
    pub seen: Vec<String>,
}

/// What a run of `stand` found: why no pain can close, when none can, and
/// where its reading ended, to keep once the pains are kept.
#[derive(Debug, Default)]
pub struct Stood {
    pub unclosed: Option<String>,
    pub read: Option<Read>,
}

/// The last run's reading kept in `state`, if one was.
pub(super) fn last_read(state: &Path) -> Option<Read> {
    serde_json::from_slice(&fs::read(state.join(READ_FILE)).ok()?).ok()
}

/// Keeps a run's reading in `state` for the next.
pub(super) fn keep_read(state: &Path, read: &Read) -> Result<(), String> {
    let text = serde_json::to_string(read).map_err(|e| e.to_string())?;
    crate::write_whole(&state.join(READ_FILE), &text)
}

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

/// Takes up and closes, from the mesh and the machine, each pain sent and
/// still open, however old; `asker` is who sent the questions, and `read`
/// where the last run's reading ended. Says why none can close when the
/// messages, the core the machine runs, or the git to ask of it can't be
/// read, so a fix that never closes doesn't read as one not on the
/// machine yet. A first run reads from the oldest question of the last
/// day, so threads begun before it are followed.
pub(super) fn stand(
    ears: &Ears,
    asker: Option<&str>,
    kept: &mut [Heard],
    read: Option<&Read>,
) -> Stood {
    let still = |h: &Heard| h.closed_by.is_none() && !stalled::is(h) && h.routed_at.is_some();
    let Some(asker) = asker else {
        return Stood::default();
    };
    let since = kept
        .iter()
        .filter(|h| still(h))
        .filter_map(|h| h.routed_at)
        .min();
    let Some(since) = since else {
        // Nothing waits on the mesh, so nothing is read: the next
        // question is sent after now.
        let read = Read {
            at: ears.at,
            seen: Vec::new(),
        };
        return Stood {
            unclosed: None,
            read: Some(read),
        };
    };
    let first = (since - 60_000).max(ears.at - DAY_MS);
    let from = read.map_or(first, |r| r.at - LATE_AT_MOST_MS);
    let seen: BTreeSet<&str> = read
        .map(|r| r.seen.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let batch: Vec<(i64, Message)> = messages(&ears.messages, from)
        .into_iter()
        .filter(|(_, m)| !seen.contains(m.id.as_str()))
        .collect();
    let next = ears
        .messages
        .is_dir()
        .then(|| read_after(read, from, ears.at, &batch));
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
    let sent: BTreeSet<&str> = batch.iter().filter_map(|(_, m)| short(&m.id)).collect();
    let mut listed: Option<BTreeSet<String>> = None;
    let mut is_id = |word: &str| {
        sent.contains(word)
            || listed
                .get_or_insert_with(|| listed_ids(&ears.messages))
                .contains(word)
    };
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
        for (_, m) in &batch {
            follow(h, asker, &held, m, &mut is_id);
        }
        h.closed_by = closing(h, &mut on);
    }
    Stood {
        unclosed,
        read: next,
    }
}

/// Where a reading from `from` of `batch` at `now` ends, after `read`'s:
/// at the newest sending either read, or `LATE_AT_MOST_MS` before now
/// where that is later, so a quiet mesh isn't read again from an old
/// start, with each message read since `LATE_AT_MOST_MS` before it.
fn read_after(read: Option<&Read>, from: i64, now: i64, batch: &[(i64, Message)]) -> Read {
    let before = read.map_or(from, |r| r.at).max(now - LATE_AT_MOST_MS);
    let at = batch.last().map_or(before, |(sent, _)| (*sent).max(before));
    let mut seen: Vec<String> = read.map(|r| r.seen.clone()).unwrap_or_default();
    seen.extend(batch.iter().map(|(_, m)| m.id.clone()));
    seen.retain(|id| sent_of(id).is_some_and(|sent| sent >= at - LATE_AT_MOST_MS));
    seen.sort_unstable();
    seen.dedup();
    Read { at, seen }
}

/// When a message was sent, as its id opens with it.
fn sent_of(id: &str) -> Option<i64> {
    id.split('-').next()?.parse().ok()
}

/// The short form of every message kept in `folder` and its days', for a
/// landing naming one read on an earlier run.
fn listed_ids(folder: &Path) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut list = |dir: &Path| {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".json").and_then(short) {
                ids.insert(id.to_owned());
            }
        }
    };
    list(folder);
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().is_ok_and(|t| t.is_dir()) && name.starts_with("20") {
            list(&entry.path());
        }
    }
    ids
}

/// Follows one message, `m`, in `pain`'s thread. A question `asker` sent
/// naming it joins the thread, since a send that timed out after the node
/// took it is sent again; `held` are the entries of every pain kept, to
/// tell a question that held this one alone. Another's message replying
/// in the thread, or naming it to a peer, takes it up; one naming one
/// landing keeps it to close the pain once a switch carries it. `is_id`
/// says whether a word is a message's short form.
fn follow(
    pain: &mut Heard,
    asker: &str,
    held: &[&str],
    m: &Message,
    is_id: &mut dyn FnMut(&str) -> bool,
) {
    if m.from.eq_ignore_ascii_case(asker) {
        if let Some(id) = short(&m.id)
            && names(&m.text, &pain.entry)
            && !m.text.contains(FOR_THE_KEEPER)
        {
            add(&mut pain.thread, id);
            pain.shared |= held.iter().filter(|e| names(&m.text, e)).count() != 1;
        }
        return;
    }
    let by_entry = names(&m.text, &pain.entry);
    if pain.thread.is_empty() && !by_entry {
        return;
    }
    let in_thread = |word: &str| pain.thread.iter().any(|id| id == word);
    let replies = m.re.as_deref().and_then(short).is_some_and(in_thread);
    let names_it = words(&m.text).any(in_thread);
    // A note of two landings says which fixed nothing in particular, and
    // one naming several messages which it answers.
    let named = if names_it {
        let named: BTreeSet<&str> = words(&m.text)
            .filter(|&w| in_thread(w) || is_id(w))
            .collect();
        named.len()
    } else {
        0
    };
    if replies || ((names_it || by_entry) && m.to.iter().any(|to| to.starts_with('@'))) {
        pain.taken_by.get_or_insert_with(|| m.from.clone());
    }
    if let [commit] = merged_at(&m.text)[..]
        && (by_entry || replies || (names_it && named == 1))
        && !pain.landed.iter().any(|l| l.commit == commit)
    {
        pain.landed.push(Landing {
            commit: commit.to_owned(),
            said: said_of(&m.text),
            by_entry,
        });
    }
    if let (true, Some(id)) = (replies, short(&m.id)) {
        add(&mut pain.thread, id);
    }
}

/// Adds `id` to a thread once.
fn add(thread: &mut Vec<String>, id: &str) {
    if !thread.iter().any(|t| t == id) {
        thread.push(id.to_owned());
    }
}

/// The first line of a landing's note, as much as is kept.
fn said_of(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default().trim();
    first.chars().take(SAID_AT_MOST).collect()
}

/// The commit that closes `pain`: the first landing kept that `on` says
/// the machine runs. A question that held other pains is closed only by a
/// landing naming its entry, since one rarely fixes them all.
fn closing(pain: &Heard, on: &mut dyn FnMut(&str) -> bool) -> Option<String> {
    pain.landed
        .iter()
        .filter(|l| l.by_entry || !pain.shared)
        .find(|l| on(l.commit.as_str()))
        .map(|l| l.commit.clone())
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
    let ids: BTreeSet<&str> = messages.iter().filter_map(|m| short(&m.id)).collect();
    let mut pain = pain.clone();
    for m in messages {
        follow(&mut pain, asker, held, m, &mut |w| ids.contains(w));
    }
    Standing {
        closed_by: closing(&pain, on),
        taken_by: pain.taken_by,
    }
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
pub fn messages(folder: &Path, since_ms: i64) -> Vec<(i64, Message)> {
    let mut found = Vec::new();
    let mut gather = |dir: &Path| {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let sent = name.split('-').next().and_then(|ms| ms.parse::<i64>().ok());
            let json = entry.path().extension().is_some_and(|ext| ext == "json");
            let Some(sent) = sent.filter(|&sent| json && sent >= since_ms) else {
                continue;
            };
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
    found
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
