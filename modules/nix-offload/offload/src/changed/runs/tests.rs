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

#[test]
fn a_line_two_runs_wrote_into_each_other_stays_on_the_builder() {
    // On 12 November two gates on the lent builder ended a second apart
    // and wrote their lines into each other, which no reader could parse.
    let folder = tempfile::tempdir().unwrap();
    let home = folder.path().join("changed-runs.jsonl");
    let added = kept("{\"at\":1}\n{\"at\":{\"a2t\":3}\n{\"at\":4}\n", &home).unwrap();
    assert_eq!(added, 2, "only the lines a reader can parse come home");
    assert_eq!(
        fs::read_to_string(&home).unwrap(),
        "{\"at\":1}\n{\"at\":4}\n",
        "the mixed line came home"
    );
}
