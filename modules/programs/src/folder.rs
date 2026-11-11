//! A folder asked for by its path, as `folder:/home/joel/Downloads`: what's
//! in it, read by this daemon itself with no command, a file a line as
//! [`files_in`] prints them. Its key names it, so the shell can ask for a
//! folder no source names, as the picture viewer's galleries are the
//! folders of whatever its Recent lists. A folder is watched from the first
//! time it's asked for, and read again on each change in it while its key
//! is watched, and as each watch of it begins.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::notify::Notify;
use crate::source::files_in;

/// The path a folder's key names, only a whole one, since a path from
/// nowhere in particular would be read from wherever this daemon started.
#[must_use]
pub fn path_of(key: &str) -> Option<&str> {
    key.strip_prefix("folder:")
        .filter(|path| path.starts_with('/'))
}

/// A folder's record: its path and what's in it, newest first, none for
/// one that isn't there.
#[must_use]
pub fn record(key: &str, path: &str) -> Value {
    let printed = files_in(&[path.to_owned()]);
    let lines: Vec<&str> = printed.lines().collect();
    json!({"key": key, "kind": "folder", "path": path, "lines": lines})
}

/// The folders watched for their keys, by inotify's number for each, with
/// the one file in it a key hears, where it hears only one. A key can be
/// heard by many folders, as the repositories under one are by each `.git`
/// (repositories.rs), and a folder can be heard for many keys.
#[derive(Debug, Default)]
pub struct Looked {
    keys: HashMap<i32, Vec<(String, Option<String>)>>,
    watching: HashSet<(String, PathBuf)>,
}

impl Looked {
    /// Watches a folder for a key, if it isn't yet and can be, as one that
    /// isn't there yet can't: it's tried again as the key is next read.
    pub fn watch(&mut self, notify: Option<&Notify>, key: &str, path: &Path) {
        self.hear(notify, key, path, None);
    }

    /// Watches the folder a file is in for a key, hearing only that file,
    /// as zsh's history sits in a home folder that changes for many other
    /// reasons.
    pub fn watch_file(&mut self, notify: Option<&Notify>, key: &str, file: &Path) {
        let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
            return;
        };
        self.hear(
            notify,
            key,
            folder,
            Some(name.to_string_lossy().into_owned()),
        );
    }

    fn hear(&mut self, notify: Option<&Notify>, key: &str, folder: &Path, name: Option<String>) {
        let heard = name
            .as_ref()
            .map_or_else(|| folder.to_path_buf(), |name| folder.join(name));
        let pair = (key.to_owned(), heard);
        if self.watching.contains(&pair) {
            return;
        }
        let Some(watch) = notify.and_then(|notify| notify.watch(folder)) else {
            return;
        };
        let keys = self.keys.entry(watch).or_default();
        let held = (key.to_owned(), name);
        if !keys.contains(&held) {
            keys.push(held);
        }
        self.watching.insert(pair);
    }

    /// The keys a change of this name in this watch's folder is heard for:
    /// each hearing the folder whole, and each hearing only the file so
    /// named.
    pub fn keys_of<'a>(&'a self, watch: i32, name: &'a str) -> impl Iterator<Item = &'a String> {
        self.keys
            .get(&watch)
            .into_iter()
            .flatten()
            .filter(move |(_, only)| only.as_deref().is_none_or(|only| only == name))
            .map(|(key, _)| key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_whole_path_is_a_folder() {
        assert_eq!(
            path_of("folder:/home/joel"),
            Some("/home/joel"),
            "a whole path"
        );
        assert_eq!(path_of("folder:Downloads"), None, "a path from nowhere");
        assert_eq!(path_of("listed:shared:taken"), None, "another kind");
    }

    #[test]
    fn a_folder_that_isnt_there_holds_nothing() {
        let key = "folder:/nowhere/at/all";
        assert_eq!(
            record(key, "/nowhere/at/all"),
            json!({"key": key, "kind": "folder", "path": "/nowhere/at/all", "lines": []}),
            "a missing folder's record"
        );
    }

    #[test]
    fn a_folder_is_watched_once_for_each_key() {
        let notify = Notify::new().expect("inotify is there");
        let folder = std::env::temp_dir();
        let path = folder.to_string_lossy();
        let key = format!("folder:{path}");
        let mut looked = Looked::default();
        looked.watch(Some(&notify), &key, &folder);
        looked.watch(Some(&notify), &key, &folder);
        let watch = notify.watch(&folder).expect("the folder is watched");
        let keys: Vec<&String> = looked.keys_of(watch, "any").collect();
        assert_eq!(keys, [&key], "the folder's key, once");
    }

    #[test]
    fn a_key_is_heard_by_each_of_its_folders() {
        let notify = Notify::new().expect("inotify is there");
        let one = std::env::temp_dir().join(format!("looked-one-{}", std::process::id()));
        let two = std::env::temp_dir().join(format!("looked-two-{}", std::process::id()));
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&two).unwrap();
        let mut looked = Looked::default();
        looked.watch(Some(&notify), "repositories:/src", &one);
        looked.watch(Some(&notify), "repositories:/src", &two);
        looked.watch(Some(&notify), "folder:/two", &two);
        let first = notify.watch(&one).expect("the first folder is watched");
        let second = notify.watch(&two).expect("the second folder is watched");
        assert_eq!(
            of(&looked, first, "any"),
            ["repositories:/src"],
            "the first folder's key"
        );
        assert_eq!(
            of(&looked, second, "any"),
            ["repositories:/src", "folder:/two"],
            "the second folder's two keys"
        );
        looked.watch_file(Some(&notify), "reached:/two/history", &two.join("history"));
        assert_eq!(
            of(&looked, second, "history"),
            ["repositories:/src", "folder:/two", "reached:/two/history"],
            "its file's key too as the file changes"
        );
        assert_eq!(
            of(&looked, second, "elsewhere"),
            ["repositories:/src", "folder:/two"],
            "not the file's key as another changes"
        );
        std::fs::remove_dir_all(&one).unwrap();
        std::fs::remove_dir_all(&two).unwrap();
    }

    /// The keys a change of this name in this watch's folder is heard for.
    fn of(looked: &Looked, watch: i32, name: &str) -> Vec<String> {
        looked.keys_of(watch, name).cloned().collect()
    }
}
