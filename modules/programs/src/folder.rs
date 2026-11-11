//! A folder asked for by its path, as `folder:/home/joel/Downloads`: what's
//! in it, read by this daemon itself with no command, a file a line as
//! [`files_in`] prints them. Its key names it, so the shell can ask for a
//! folder no source names, as the picture viewer's galleries are the
//! folders of whatever its Recent lists. A folder is watched from the first
//! time it's asked for, and read again on each change in it while its key
//! is watched, and as each watch of it begins.

use std::collections::{HashMap, HashSet};
use std::path::Path;

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

/// The folders watched for their keys, by inotify's number for each.
#[derive(Debug, Default)]
pub struct Looked {
    keys: HashMap<i32, Vec<String>>,
    watching: HashSet<String>,
}

impl Looked {
    /// Watches a key's folder, if it isn't yet and can be, as one that
    /// isn't there yet can't: it's tried again as it's next asked for.
    pub fn watch(&mut self, notify: Option<&Notify>, key: &str, path: &str) {
        if self.watching.contains(key) {
            return;
        }
        let Some(watch) = notify.and_then(|notify| notify.watch(Path::new(path))) else {
            return;
        };
        self.keys.entry(watch).or_default().push(key.to_owned());
        self.watching.insert(key.to_owned());
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
        looked.watch(Some(&notify), &key, &path);
        looked.watch(Some(&notify), &key, &path);
        let watch = notify.watch(&folder).expect("the folder is watched");
        assert_eq!(looked.keys_of(watch), [key], "the folder's key, once");
    }
}
