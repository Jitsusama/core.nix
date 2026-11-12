//! The cases for which lines of a builder's runs come home, kept beside
//! runs.rs as changed.rs keeps its own.

use super::*;
use std::fs;

#[test]
fn only_whole_lines_not_already_home_are_added() {
    let folder = tempfile::tempdir().unwrap();
    let home = folder.path().join("nix-offload/changed-runs.jsonl");

    // The first lines a run finds make the file, folder and all.
    let added = kept("{\"at\":1}\n{\"at\":2}\n", &home).unwrap();
    assert_eq!(added, 2, "both are new");

    // A run beside the first found its line too, and one of its own, and
    // the start of a line another run is still writing.
    let added = kept("{\"at\":2}\n{\"at\":3}\n{\"at\":3}\n{\"at", &home).unwrap();
    assert_eq!(added, 1, "only its own, once, comes home");
    assert_eq!(
        fs::read_to_string(&home).unwrap(),
        "{\"at\":1}\n{\"at\":2}\n{\"at\":3}\n",
        "each line is home once, and the unfinished one waits"
    );

    // Nothing found adds nothing.
    assert_eq!(kept("", &home).unwrap(), 0, "nothing was found");
}
