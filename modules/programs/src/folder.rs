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

/// The folders watched for their keys, by inotify's number for each. A
/// key can be heard by many folders, as the repositories under one are by
/// each `.git` (repositories.rs), and a folder can be heard for many keys.
#[derive(Debug, Default)]
pub struct Looked {
    keys: HashMap<i32, Vec<String>>,
    watching: HashSet<(String, PathBuf)>,
}

impl Looked {
    /// Watches a folder for a key, if it isn't yet and can be, as one that
    /// isn't there yet can't: it's tried again as the key is next read.
    pub fn watch(&mut self, notify: Option<&Notify>, key: &str, path: &Path) {
        let pair = (key.to_owned(), path.to_path_buf());
        if self.watching.contains(&pair) {
            return;
        }
        let Some(watch) = notify.and_then(|notify| notify.watch(path)) else {
            return;
        };
        let keys = self.keys.entry(watch).or_default();
        if !keys.iter().any(|held| held == key) {
            keys.push(key.to_owned());
        }
        self.watching.insert(pair);
    }

    /// The keys whose folder this watch is.
    #[must_use]
    pub fn keys_of(&self, watch: i32) -> &[String] {
        self.keys.get(&watch).map_or(&[], Vec::as_slice)
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
        assert_eq!(looked.keys_of(watch), [key], "the folder's key, once");
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
            looked.keys_of(first),
            ["repositories:/src"],
            "the first folder's key"
        );
        assert_eq!(
            looked.keys_of(second),
            ["repositories:/src", "folder:/two"],
            "the second folder's two keys"
        );
        std::fs::remove_dir_all(&one).unwrap();
        std::fs::remove_dir_all(&two).unwrap();
    }
}
