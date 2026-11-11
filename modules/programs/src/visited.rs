//! When zoxide last saw each folder, read from its own database as
//! `visited:<database>` names it, for kitty's page in the launcher
//! (modules/kitty/Kitty.qml): each folder still there, the one visited
//! last first, with when, in milliseconds. zoxide's `query` prints only a
//! score, so its database is read here, as bincode lays it out: a
//! version, read only at 3, zoxide's since 0.8, then how many folders,
//! each its path's length and bytes, its score, eight bytes, and when it
//! was last visited, in seconds since 1970; each number little-endian, a
//! count in eight bytes. A database of another version says none, and
//! one cut short says the folders before the cut. It's read as a get
//! names it, never otherwise, so nothing is read at rest.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

/// The one version of zoxide's database this reads.
const VERSION: u64 = 3;

/// The record a key names, or none for a key of another kind or a path
/// that isn't whole.
#[must_use]
pub fn record(key: &str) -> Option<Value> {
    let database = key
        .strip_prefix("visited:")
        .filter(|path| path.starts_with('/'))?;
    let folders: Vec<Value> = fs::read(database)
        .map(|bytes| visits(&bytes))
        .unwrap_or_default()
        .into_iter()
        .filter(|(path, _)| Path::new(path).is_dir())
        .map(|(path, at)| json!({"path": path, "at": at}))
        .collect();
    Some(json!({
        "key": key, "kind": "visited", "database": database, "folders": folders,
    }))
}

/// A database's bytes, read from the front.
struct Bytes<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, size: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(size)?;
        let taken = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(taken)
    }

    /// A little-endian number of `size` bytes.
    fn number(&mut self, size: usize) -> Option<u64> {
        let taken = self.take(size)?;
        Some(
            taken
                .iter()
                .rev()
                .fold(0, |number, byte| (number << 8) | u64::from(*byte)),
        )
    }
}

/// Each folder and when it was last visited, in milliseconds, the one
/// visited last first.
fn visits(bytes: &[u8]) -> Vec<(String, u64)> {
    let mut read = Bytes { bytes, at: 0 };
    if read.number(4) != Some(VERSION) {
        return Vec::new();
    }
    let Some(count) = read.number(8) else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    for _ in 0..count {
        let Some(one) = folder(&mut read) else {
            break;
        };
        seen.push(one);
    }
    seen.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    seen
}

fn folder(read: &mut Bytes) -> Option<(String, u64)> {
    let length = usize::try_from(read.number(8)?).ok()?;
    let path = String::from_utf8_lossy(read.take(length)?).into_owned();
    // Its score, whose frecency has no unit Joel could read.
    read.take(8)?;
    let seconds = read.number(8)?;
    Some((path, seconds.saturating_mul(1000)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn little(value: u64, size: usize) -> Vec<u8> {
        value.to_le_bytes()[..size].to_vec()
    }

    fn database(version: u64, folders: &[(&str, u64)]) -> Vec<u8> {
        let mut bytes = little(version, 4);
        bytes.extend(little(u64::try_from(folders.len()).unwrap(), 8));
        for (path, seconds) in folders {
            bytes.extend(little(u64::try_from(path.len()).unwrap(), 8));
            bytes.extend(path.as_bytes());
            bytes.extend(1.0_f64.to_le_bytes());
            bytes.extend(little(*seconds, 8));
        }
        bytes
    }

    #[test]
    fn each_folder_says_when_it_was_seen_the_last_seen_first() {
        let bytes = database(3, &[("/s/a", 1_700_000_000), ("/s/\u{e9}", 1_700_003_600)]);
        assert_eq!(
            visits(&bytes),
            [
                ("/s/\u{e9}".to_owned(), 1_700_003_600_000),
                ("/s/a".to_owned(), 1_700_000_000_000),
            ],
            "the folders weren't read as zoxide wrote them"
        );
    }

    #[test]
    fn another_version_says_none_and_one_cut_short_what_came_before() {
        let bytes = database(3, &[("/s/a", 1), ("/s/b", 2)]);
        assert!(
            visits(&database(4, &[("/s/a", 1)])).is_empty(),
            "another version's database was read"
        );
        assert_eq!(
            visits(&bytes[..bytes.len() - 3]),
            [("/s/a".to_owned(), 1000)],
            "a database cut short didn't say what came before the cut"
        );
        assert!(visits(&[]).is_empty(), "no database said a folder");
    }

    #[test]
    fn only_a_whole_path_is_a_database() {
        assert!(
            record("visited:db.zo").is_none(),
            "a part of a path was read"
        );
        assert!(record("folder:/db.zo").is_none(), "another kind was read");
        assert_eq!(
            record("visited:/nowhere/db.zo").map(|record| record["folders"].clone()),
            Some(json!([])),
            "a database that isn't there didn't say no folders"
        );
    }
}
