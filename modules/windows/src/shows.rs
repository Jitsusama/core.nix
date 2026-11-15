//! The file each Reader window shows, so what speaks of a window can say
//! which document it is rather than a title two windows may share. Reader
//! keeps it where the launcher's Recent reads it: a file named for its
//! process under the state home's reader/showing, holding the process's
//! id, a tab and the file's whole path, moved there whole as it opens
//! another file. A window's record gains it as `file`, read by niri's pid
//! for the window each time the records are worked out; it is absent for
//! other programs, for Reader's own start page, and when unreadable.
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Where readers say what they show: the state home's reader/showing.
pub fn folder() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
        })?;
    Some(state.join("reader").join("showing"))
}

/// Adds each Reader window's file to its record.
pub fn add(records: &mut [Value], folder: Option<&Path>) {
    let Some(folder) = folder else { return };
    for record in records {
        if record["app_id"] != "reader" {
            continue;
        }
        if let Some(file) = record["pid"].as_u64().and_then(|pid| shown(folder, pid)) {
            record["file"] = Value::String(file);
        }
    }
}

/// The whole path the reader of process `pid` says it shows, when it says
/// one under its own id.
fn shown(folder: &Path, pid: u64) -> Option<String> {
    let said = std::fs::read_to_string(folder.join(pid.to_string())).ok()?;
    let (by, file) = said.trim_end_matches('\n').split_once('\t')?;
    (by.parse::<u64>().ok()? == pid && Path::new(file).is_absolute()).then(|| file.to_owned())
}

#[cfg(test)]
mod tests {
    use super::add;
    use serde_json::{Value, json};

    fn folder(test: &str) -> std::path::PathBuf {
        let folder =
            std::env::temp_dir().join(format!("windows-shows-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn added(folder: &std::path::Path, record: Value) -> Value {
        let mut records = [record];
        add(&mut records, Some(folder));
        let [record] = records;
        record
    }

    #[test]
    fn a_reader_window_says_the_file_its_process_shows() {
        let folder = folder("reader");
        std::fs::write(folder.join("120"), "120\t/notes/sanderling.md\n").unwrap();
        let record = added(&folder, json!({"app_id": "reader", "pid": 120}));
        assert_eq!(record["file"], "/notes/sanderling.md");
    }

    #[test]
    fn only_a_reader_says_a_file() {
        let folder = folder("others");
        std::fs::write(folder.join("121"), "121\t/notes/now.md\n").unwrap();
        let record = added(&folder, json!({"app_id": "kitty", "pid": 121}));
        assert!(record.get("file").is_none(), "{record}");
    }

    #[test]
    fn a_reader_saying_nothing_readable_has_no_file() {
        let folder = folder("unread");
        std::fs::write(folder.join("123"), "999\t/notes/other.md\n").unwrap();
        std::fs::write(folder.join("124"), "124\tnotes/relative.md\n").unwrap();
        for pid in [122, 123, 124] {
            let record = added(&folder, json!({"app_id": "reader", "pid": pid}));
            assert!(record.get("file").is_none(), "{record}");
        }
        let record = added(&folder, json!({"app_id": "reader"}));
        assert!(record.get("file").is_none(), "{record}");
    }
}
