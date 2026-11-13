//! The mesh's messages over the last day, as far as the axes read them:
//! each one's id, intent and what it answers, for collaboration's fyi
//! share and its questions left unanswered.
//!
//! A day holds over eleven thousand message files, and opening them all
//! took seconds on a busy machine, sixteen once with the cache cold. A
//! message's file never changes once written, so what each said is kept
//! by its file's name in one file of cadence's own, and only the files
//! new since the last reading are opened.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::DAY;
use crate::mesh;

/// A mesh message, as far as the axes and the week's review read one.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub intent: String,
    #[serde(default)]
    pub re: Option<String>,
    /// Who sent it, for the review's noise by sender.
    #[serde(default)]
    pub from: String,
    /// What it said, hashed, so a message sent again is seen without
    /// keeping the text of a week of them.
    #[serde(default, rename = "text", deserialize_with = "hashed")]
    pub said: u64,
}

/// A message's text as its hash, or a hash kept from an earlier reading.
fn hashed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Said {
        Text(String),
        Hash(u64),
    }
    Ok(match Said::deserialize(d)? {
        Said::Text(text) => fnv(&text),
        Said::Hash(hash) => hash,
    })
}

/// FNV-1a, which a toolchain update never changes, unlike std's hasher,
/// so a kept hash still matches the same text read after one.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Every message sent since `since`, in seconds, under the mesh's `root`,
/// each once. What each file said is kept at `kept`, when given, and read
/// from there rather than opened again.
#[must_use]
pub fn read(root: &Path, since: i64, kept: Option<&Path>) -> Vec<Note> {
    let folder = root.join("mesh-workflow/messages");
    let since_ms = since * 1000;
    let mut known: BTreeMap<String, Note> = kept
        .and_then(|path| fs::read(path).ok())
        .and_then(|json| serde_json::from_slice(&json).ok())
        .unwrap_or_default();
    // The mesh kept a folder a day until 12 November, then wrote each
    // message straight into the folder, named by when it was sent; a
    // message written both ways counts once. Either way a file is chosen
    // by the time in its name before it's opened.
    // A week's review reads eight days' folders; a day's reading, two.
    let by_day = (0..=8)
        .map(|n| since + n * DAY)
        .filter_map(|at| Timestamp::from_second(at).ok())
        .map(|at| folder.join(at.strftime("%Y-%m-%d").to_string()))
        .collect::<BTreeSet<PathBuf>>()
        .into_iter()
        .flat_map(|day| fs::read_dir(day).into_iter().flatten().flatten());
    let flat = fs::read_dir(&folder).into_iter().flatten().flatten();
    let mut said: BTreeMap<String, Note> = BTreeMap::new();
    for entry in by_day.chain(flat) {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if said.contains_key(&name) || mesh::stamped_at(&name).is_none_or(|ms| ms < since_ms) {
            continue;
        }
        let note = known
            .remove(&name)
            .or_else(|| serde_json::from_slice::<Note>(&fs::read(entry.path()).ok()?).ok());
        if let Some(note) = note {
            said.insert(name, note);
        }
    }
    if let Some(path) = kept {
        keep(path, &said);
    }
    said.into_values()
        .filter(|m| mesh::stamped_at(&m.id).is_some_and(|ms| ms >= since_ms))
        .map(|m| (m.id.clone(), m))
        .collect::<BTreeMap<String, Note>>()
        .into_values()
        .collect()
}

/// Writes what the day's files said to `path` whole, through a file beside
/// it, so a reading cut short leaves the last one whole. A failure only
/// costs the next reading its opens, so it's let pass.
fn keep(path: &Path, said: &BTreeMap<String, Note>) {
    let Ok(json) = serde_json::to_vec(said) else {
        return;
    };
    let beside = path.with_extension("json.new");
    let written = path
        .parent()
        .is_some_and(|parent| fs::create_dir_all(parent).is_ok())
        && fs::write(&beside, json).is_ok();
    if written {
        let _ = fs::rename(&beside, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 00:00 UTC on Thu 12 November 2026.
    const SINCE: i64 = 1_794_441_600_000;

    fn note(at: i64, tag: &str, intent: &str) -> (String, String) {
        let id = format!("{at}-AAAAAA-1-{tag}");
        (
            id.clone(),
            format!(r#"{{"id":"{id}","intent":"{intent}"}}"#),
        )
    }

    fn ids(notes: Vec<Note>) -> Vec<String> {
        let mut ids: Vec<String> = notes.into_iter().map(|m| m.id).collect();
        ids.sort();
        ids
    }

    #[test]
    fn messages_are_read_flat_and_by_day_each_once() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let folder = root.path().join("mesh-workflow/messages");
        fs::create_dir_all(folder.join("2026-11-12")).expect("the day's folder");
        let (by_day, body) = note(SINCE + 10_000, "a", "fyi");
        fs::write(folder.join(format!("2026-11-12/{by_day}.json")), &body).expect("written");
        fs::write(folder.join(format!("{by_day}.json")), &body).expect("written");
        let (flat, body) = note(SINCE + 20_000, "b", "fyi");
        fs::write(folder.join(format!("{flat}.json")), body).expect("written");
        let (old, body) = note(SINCE - 100_000, "c", "fyi");
        fs::write(folder.join(format!("{old}.json")), body).expect("written");
        // A file named for an hour before is never opened, by day or flat:
        // what it holds isn't read, so the day's thousands cost nothing.
        let (_, recent) = note(SINCE + 30_000, "d", "fyi");
        fs::write(folder.join(format!("2026-11-12/{old}.json")), recent).expect("written");

        assert_eq!(
            ids(read(root.path(), SINCE / 1000, None)),
            vec![by_day, flat]
        );
    }

    #[test]
    fn a_file_read_once_is_kept_and_never_opened_again() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let folder = root.path().join("mesh-workflow/messages");
        fs::create_dir_all(&folder).expect("the folder");
        let kept = root.path().join("cache/mesh-notes.json");
        let (first, body) = note(SINCE + 10_000, "a", "fyi");
        let file = folder.join(format!("{first}.json"));
        fs::write(&file, body).expect("written");
        let read_once = read(root.path(), SINCE / 1000, Some(&kept));
        assert_eq!(read_once[0].intent, "fyi");

        // Were it opened again, it would now say question.
        fs::write(&file, note(SINCE + 10_000, "a", "question").1).expect("rewritten");
        let (second, body) = note(SINCE + 20_000, "b", "question");
        fs::write(folder.join(format!("{second}.json")), body).expect("written");
        let read_twice = read(root.path(), SINCE / 1000, Some(&kept));
        assert_eq!(
            read_twice
                .iter()
                .map(|m| (m.id.as_str(), m.intent.as_str()))
                .collect::<Vec<_>>(),
            [(first.as_str(), "fyi"), (second.as_str(), "question")],
            "the kept file answered for the first, and only the new one was opened"
        );

        // A message gone from the folder goes from what's kept too.
        fs::remove_file(&file).expect("removed");
        assert_eq!(
            ids(read(root.path(), SINCE / 1000, Some(&kept))),
            vec![second]
        );
    }
}
