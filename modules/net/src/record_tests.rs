use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::Record;

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("net-record-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the test");
    folder
}

#[test]
fn a_record_is_written_whole_and_said_nothing_of() {
    let folder = folder("whole");
    let file = folder.join("network.json");
    let mut record = Record::at(file.clone());
    assert_eq!(record.keep("{}\n"), None, "a kept record was spoken of");
    assert_eq!(
        fs::read_to_string(&file).expect("the record"),
        "{}\n",
        "the record isn't what was kept"
    );
    assert!(
        !folder.join("network.json.new").exists(),
        "the partial file outlived a write"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_record_that_cant_be_written_is_said_once_and_leaves_no_partial() {
    let folder = folder("refused");
    // A folder where the record goes can't be renamed over, so the
    // partial file is written and the rename fails, as a full disk fails
    // partway through.
    let file = folder.join("network.json");
    fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
    let mut record = Record::at(file);
    let first = record.keep("{}\n");
    assert!(
        first
            .as_deref()
            .is_some_and(|said| said.contains("can't write the record")),
        "the first failure wasn't said: {first:?}"
    );
    assert_eq!(
        record.keep("{}\n"),
        None,
        "a failure was said twice in one spell"
    );
    assert!(
        !folder.join("network.json.new").exists(),
        "a failed write left its partial file holding the space"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_record_kept_after_a_failure_says_so() {
    let folder = folder("again");
    let file = folder.join("network.json");
    fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
    let mut record = Record::at(file.clone());
    assert!(record.keep("{}\n").is_some(), "the failure wasn't said");
    fs::remove_dir_all(&file).expect("the folder taken away");
    let again = record.keep("{}\n");
    assert!(
        again
            .as_deref()
            .is_some_and(|said| said.contains("kept again")),
        "the record kept again wasn't said: {again:?}"
    );
    assert_eq!(
        record.keep("{}\n"),
        None,
        "a kept record was spoken of after the spell"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_folder_cleared_away_is_made_again() {
    let folder = folder("cleared");
    let file = folder.join("net").join("network.json");
    let mut record = Record::at(file.clone());
    assert_eq!(record.keep("{}\n"), None, "the first record was spoken of");
    fs::remove_dir_all(folder.join("net")).expect("the folder cleared");
    assert_eq!(
        record.keep("{}\n"),
        None,
        "a record whose folder was cleared away couldn't be kept"
    );
    assert!(file.exists(), "the record wasn't written again");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_failure_that_changes_cause_is_said_again_and_the_last_record_stays_whole() {
    let folder = folder("changed");
    let file = folder.join("network.json");
    let mut record = Record::at(file.clone());
    assert_eq!(
        record.keep("last\n"),
        None,
        "the first record was spoken of"
    );
    // A folder that can't be written to refuses the partial file itself,
    // as a full disk refuses it before a byte is written.
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o500)).expect("the folder shut");
    if fs::write(folder.join("probe"), "").is_ok() {
        // Run as root, which no folder's mode refuses: nothing to show.
        let _ = fs::remove_file(folder.join("probe"));
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).expect("the folder open");
        let _ = fs::remove_dir_all(&folder);
        return;
    }
    let refused = record.keep("next\n");
    assert!(
        refused
            .as_deref()
            .is_some_and(|said| said.contains("can't write the record")),
        "a refused write wasn't said: {refused:?}"
    );
    assert_eq!(
        fs::read_to_string(&file).expect("the last record"),
        "last\n",
        "a refused write cost the last record"
    );
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).expect("the folder open");
    fs::remove_file(&file).expect("the record taken away");
    fs::create_dir_all(file.join("in-the-way")).expect("a folder in the record's place");
    let changed = record.keep("next\n");
    assert!(
        changed.is_some(),
        "a failure of another cause went unsaid in the same spell"
    );
    let _ = fs::remove_dir_all(&folder);
}
