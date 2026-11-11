//! Whether the machine is held: a hold sent to every session, as before a
//! reboot, asks that nothing new starts until its sender ends it. The net
//! is no session, so it reads the mesh's messages itself, from the folder
//! `MERGE_NET_MESH` names, or the harness's under XDG's state. A hold
//! ends when its sender answers it or sends one in its place, or when the
//! machine boots after it was sent. The harness files older messages into
//! a folder for each day and may leave a hold above its answer, so both
//! levels are read: on 11 November one Monday hold, whose end had gone
//! into its day's folder, stood again and held the net for half an hour.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The groups a hold for the whole machine goes to.
const EVERYONE: [&str; 2] = ["#machine", "#all"];

/// The mesh's messages folder.
pub fn messages() -> PathBuf {
    if let Some(folder) = std::env::var_os("MERGE_NET_MESH").filter(|folder| !folder.is_empty()) {
        return PathBuf::from(folder);
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|folder| !folder.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        );
    state.join("pi/agentic-harness.pi/mesh-workflow/messages")
}

/// When the machine booted, in milliseconds since the epoch, from
/// /proc/stat's `btime`; none where it can't be read.
pub fn booted() -> Option<u64> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let seconds: u64 = stat
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    Some(seconds * 1000)
}

/// The text of the first hold on the whole machine still standing among
/// the messages in `folder` sent since `since` (milliseconds, as each
/// file's name begins).
pub fn standing(folder: &Path, since: u64) -> Option<String> {
    let entries: Vec<fs::DirEntry> = fs::read_dir(folder).ok()?.filter_map(Result::ok).collect();
    let days: Vec<fs::DirEntry> = entries
        .iter()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|day| fs::read_dir(day.path()).ok())
        .flat_map(|day| day.filter_map(Result::ok))
        .collect();
    let mut sent: Vec<(u64, Value)> = days
        .into_iter()
        .chain(entries)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let at: u64 = name.split('-').next()?.parse().ok()?;
            (at >= since).then_some((at, entry.path()))
        })
        .filter_map(|(at, path)| {
            let text = fs::read_to_string(path).ok()?;
            Some((at, serde_json::from_str(&text).ok()?))
        })
        .collect();
    sent.sort_by_key(|(at, _)| *at);
    let messages: Vec<Value> = sent.into_iter().map(|(_, message)| message).collect();
    still_held(&messages)
}

/// The first hold on the whole machine among `messages` that no later one
/// from its sender answers or replaces.
pub fn still_held(messages: &[Value]) -> Option<String> {
    messages.iter().find_map(|hold| {
        let everyone = hold["to"].as_array().is_some_and(|to| {
            to.iter()
                .any(|group| EVERYONE.iter().any(|name| group == name))
        });
        if hold["intent"] != "hold" || !everyone {
            return None;
        }
        let ended = messages.iter().any(|later| {
            later["from"] == hold["from"]
                && (later["re"] == hold["id"] || later["supersedes"] == hold["id"])
        });
        (!ended).then(|| hold["text"].as_str().unwrap_or_default().to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hold(id: &str, from: &str, to: &str) -> Value {
        json!({ "id": id, "from": from, "to": [to], "intent": "hold", "text": id })
    }

    #[test]
    fn a_hold_on_everyone_stands_until_its_sender_answers_it() {
        let held = hold("h1", "A", "#machine");
        assert_eq!(
            still_held(std::slice::from_ref(&held)),
            Some("h1".to_owned())
        );
        let other = json!({ "id": "x", "from": "B", "re": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held.clone(), other]), Some("h1".to_owned()));
        let ended = json!({ "id": "e", "from": "A", "re": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held.clone(), ended]), None);
        let replaced = json!({ "id": "r", "from": "A", "supersedes": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held, replaced]), None);
    }

    #[test]
    fn a_hold_on_one_session_leaves_the_net_alone() {
        assert_eq!(still_held(&[hold("h", "A", "@B")]), None);
        assert_eq!(still_held(&[hold("h", "A", "#all")]), Some("h".to_owned()));
    }

    #[test]
    fn only_holds_sent_since_the_boot_are_read() {
        let folder = std::env::temp_dir().join(format!("merge-net-holds-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let write = |name: &str, message: &Value| {
            fs::write(folder.join(name), message.to_string()).unwrap();
        };
        write("1000-A-1-aa.json", &hold("old", "A", "#machine"));
        assert_eq!(standing(&folder, 500), Some("old".to_owned()));
        assert_eq!(standing(&folder, 2000), None);
        write("3000-A-1-bb.json", &hold("new", "A", "#machine"));
        assert_eq!(standing(&folder, 2000), Some("new".to_owned()));
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn an_answer_filed_in_a_day_folder_still_ends_its_hold() {
        let folder = std::env::temp_dir().join(format!("merge-net-days-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("2026-11-09")).unwrap();
        let held = hold("1000-A-1-aa", "A", "#all");
        fs::write(folder.join("1000-A-1-aa.json"), held.to_string()).unwrap();
        assert_eq!(
            standing(&folder, 500),
            Some("1000-A-1-aa".to_owned()),
            "a hold with no answer stands"
        );
        let ended =
            json!({ "id": "2000-A-1-bb", "from": "A", "re": "1000-A-1-aa", "intent": "fyi" });
        fs::write(
            folder.join("2026-11-09/2000-A-1-bb.json"),
            ended.to_string(),
        )
        .unwrap();
        assert_eq!(
            standing(&folder, 500),
            None,
            "the answer in the day's folder ends the hold above it"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}
