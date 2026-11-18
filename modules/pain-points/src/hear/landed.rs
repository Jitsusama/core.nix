//! Whether a pain was taken up, and whether what answered it is on the
//! machine. Each pain goes to its cell as a question naming its log entry;
//! a peer that answers it, or names it, has taken the pain up, and so has
//! each peer the answer was passed to. A landing in that thread ("merged
//! at" a commit) closes the pain once a switch carries the commit, since
//! only then does Joel have the fix. A question that held several pains
//! says nothing of any one of them, so its answers take none up.
//!
//! The thread grows only through what peers sent to peers: a note to a
//! group names many messages, and following it would close pains its
//! landings never touched.

use super::heard::Heard;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

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
    let Some(at) = messages.iter().position(|m| {
        m.from == asker
            && names(&m.text, &pain.entry)
            && held.iter().filter(|e| names(&m.text, e)).count() == 1
    }) else {
        return Standing::default();
    };
    let mut thread = BTreeSet::from([short(&messages[at].id).to_owned()]);
    let mut standing = Standing::default();
    for m in &messages[at + 1..] {
        let links = m.re.as_deref().is_some_and(|re| thread.contains(short(re)))
            || thread.iter().any(|id| m.text.contains(id.as_str()));
        if m.from == asker || !links {
            continue;
        }
        standing.taken_by.get_or_insert_with(|| m.from.clone());
        if let Some(commit) = merged_at(&m.text) {
            if standing.closed_by.is_none() && on(commit) {
                standing.closed_by = Some(commit.to_owned());
            }
        } else if m.to.iter().any(|to| to.starts_with('@')) {
            thread.insert(short(&m.id).to_owned());
        }
    }
    standing
}

/// A message id's short form, its last eight letters, as peers name it.
fn short(id: &str) -> &str {
    id.rsplit('-').next().unwrap_or(id)
}

/// The commit a landing says it merged at.
fn merged_at(text: &str) -> Option<&str> {
    let (_, after) = text.split_once("merged at ")?;
    let commit = after
        .split(|c: char| !c.is_ascii_hexdigit())
        .next()
        .filter(|c| c.len() >= 7)?;
    Some(commit)
}

/// The messages sent from `since_ms` on, in the order they were sent:
/// the folder's own and its days', by the time each file's name opens
/// with, skipping what's being written.
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
            if let Some(m) = fs::read(entry.path())
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Message>(&bytes).ok())
            {
                found.push((sent, m));
            }
        }
    };
    gather(folder);
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && name.starts_with("20") {
            gather(&entry.path());
        }
    }
    found.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    found.into_iter().map(|(_, m)| m).collect()
}

/// The core the machine switched to last, from integration's switches.
pub fn core_on(switches: &str) -> Option<String> {
    switches
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|v| v["revs"]["core"].as_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(entry: &str) -> Heard {
        serde_json::from_value(serde_json::json!({
            "key": format!("s/{entry}"), "session": "s", "entry": entry, "at": 0,
            "words": "i can't show the launcher", "kinds": ["corrected"], "hits": 1,
            "carried": false
        }))
        .expect("a pain")
    }

    fn message(id: &str, from: &str, to: &str, re: Option<&str>, text: &str) -> Message {
        Message {
            id: id.into(),
            from: from.into(),
            to: vec![to.into()],
            re: re.map(Into::into),
            text: text.into(),
        }
    }

    /// The Tuple pain of 17 November, as the mesh carried it to its fix.
    fn tuple() -> Vec<Message> {
        vec![
            message(
                "1-6LU6ZM-1-186eeb39",
                "6LU6ZM",
                "@WCJQSG",
                None,
                "Joel hit something in mycelium:cell/LR3WMN#f7da717c: \"i am not able\"",
            ),
            message(
                "2-WCJQSG-1-1adf6d68",
                "WCJQSG",
                "@Y2YBTS",
                Some("186eeb39"),
                "Show the launcher in a shared screen",
            ),
            message(
                "3-X-1-aaaaaaaa",
                "X",
                "#orchestration/wcjqsg",
                None,
                "1adf6d68 and more",
            ),
            message(
                "4-Y-1-bbbbbbbb",
                "Y",
                "#orchestration/wcjqsg",
                Some("aaaaaaaa"),
                "Y landed core #1, merged at 99999999abcd",
            ),
            message(
                "5-Y2YBTS-1-a85417d8",
                "Y2YBTS",
                "#orchestration/wcjqsg",
                None,
                "Y2YBTS landed core #5342, merged at 255494bb414c (12:38), for WCJQSG's pivot \
                 1adf6d68",
            ),
        ]
    }

    #[test]
    fn a_pain_closes_once_a_switch_carries_the_landing_its_thread_led_to() {
        let mut asked = Vec::new();
        let mut on = |commit: &str| {
            asked.push(commit.to_owned());
            commit == "255494bb414c"
        };
        assert_eq!(
            standing(
                &heard("f7da717c"),
                "6LU6ZM",
                &["f7da717c"],
                &tuple(),
                &mut on
            ),
            Standing {
                taken_by: Some("WCJQSG".into()),
                closed_by: Some("255494bb414c".into())
            },
            "the pivot took it up and its landing closed it"
        );
        assert_eq!(
            asked,
            ["255494bb414c"],
            "a landing re a note to a group is no landing of this pain"
        );
    }

    #[test]
    fn a_landing_no_switch_carries_leaves_it_taken() {
        assert_eq!(
            standing(
                &heard("f7da717c"),
                "6LU6ZM",
                &["f7da717c"],
                &tuple(),
                &mut |_| false
            ),
            Standing {
                taken_by: Some("WCJQSG".into()),
                closed_by: None
            },
            "taken, not closed"
        );
    }

    #[test]
    fn an_unanswered_or_shared_question_takes_nothing_up() {
        let asked_alone = &tuple()[..1];
        assert_eq!(
            standing(
                &heard("f7da717c"),
                "6LU6ZM",
                &["f7da717c"],
                asked_alone,
                &mut |_| true
            ),
            Standing::default(),
            "nobody answered"
        );
        let mut shared = tuple();
        shared[0]
            .text
            .push_str(" and mycelium:cell/OMNOJW#0badc0de");
        assert_eq!(
            standing(
                &heard("f7da717c"),
                "6LU6ZM",
                &["f7da717c", "0badc0de"],
                &shared,
                &mut |_| true
            ),
            Standing::default(),
            "a question holding two pains says nothing of either"
        );
        assert_eq!(
            standing(
                &heard("feedf00d"),
                "6LU6ZM",
                &["feedf00d"],
                &tuple(),
                &mut |_| true
            ),
            Standing::default(),
            "a pain never asked of anyone"
        );
    }

    #[test]
    fn a_landing_names_the_commit_it_merged_at() {
        assert_eq!(
            merged_at("landed core #5486, merged at 5b05e833e36c (06:45)"),
            Some("5b05e833e36c"),
            "the commit"
        );
        assert_eq!(merged_at("merged at abc"), None, "too short to be a commit");
        assert_eq!(merged_at("landed it"), None, "no commit");
    }

    #[test]
    fn the_core_on_is_the_last_switchs() {
        let switches = concat!(
            r#"{"at":"a","generation":1,"revs":{"core":"aaa"}}"#,
            "\n",
            r#"{"at":"b","generation":2,"revs":{"core":"bbb","world":"w"}}"#,
            "\nnot json\n"
        );
        assert_eq!(core_on(switches).as_deref(), Some("bbb"), "the newest");
        assert_eq!(core_on(""), None, "no switch yet");
    }

    #[test]
    fn messages_are_read_from_the_folder_and_its_days_in_order_sent() {
        let dir = std::env::temp_dir().join(format!("landed-{}", std::process::id()));
        let day = dir.join("2026-11-17");
        fs::create_dir_all(&day).expect("a day folder");
        let write = |path: std::path::PathBuf, id: &str| {
            let text = format!(r#"{{"id":"{id}","from":"A","to":["@B"],"text":"t"}}"#);
            fs::write(path, text).expect("a message");
        };
        write(day.join("20-A-1-bbbbbbbb.json"), "20-A-1-bbbbbbbb");
        write(dir.join("30-A-1-cccccccc.json"), "30-A-1-cccccccc");
        write(dir.join("5-A-1-aaaaaaaa.json"), "5-A-1-aaaaaaaa");
        write(dir.join(".read-A.json"), "x");
        let ids: Vec<String> = messages(&dir, 10).into_iter().map(|m| m.id).collect();
        fs::remove_dir_all(&dir).expect("cleaned up");
        assert_eq!(
            ids,
            ["20-A-1-bbbbbbbb", "30-A-1-cccccccc"],
            "since 10, in order, no dotfiles"
        );
    }
}
