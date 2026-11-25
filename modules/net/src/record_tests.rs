use std::fs;
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
