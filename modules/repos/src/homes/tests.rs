use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;

use repos::model::{Checkout, Head, Record, Repository, SCHEMA, Tree, TreeKind};
use serde_json::{Value, json};

use super::{Homes, of, uri};

fn tree(path: &str, present: bool) -> Tree {
    Tree {
        path: path.to_owned(),
        kind: TreeKind::Worktree,
        head: Head {
            branch: Some("joel/say-trees".to_owned()),
            commit: Some("bbbb".to_owned()),
            upstream: None,
        },
        present,
        written_at: None,
    }
}

fn repository(key: &str, name: &str, owner: Option<&str>, trees: Vec<Tree>) -> Repository {
    Repository {
        key: key.to_owned(),
        name: name.to_owned(),
        owner: owner.map(str::to_owned),
        remotes: Vec::new(),
        trunk: Some("main".to_owned()),
        written_at: None,
        checkouts: vec![Checkout {
            path: format!("/home/joel/src/{name}"),
            bare: false,
            head: None,
            written_at: None,
            trees,
            weight: None,
        }],
        changes: Vec::new(),
        read_through: None,
    }
}

/// core.nix, owned, with a tree on the disk and one gone from it, and
/// notes, owned by nobody, with a space in its name.
fn record() -> Record {
    Record {
        schema: SCHEMA.to_owned(),
        read_at: 0,
        machine: "laptop".to_owned(),
        roots: vec!["/home/joel/src".to_owned()],
        sources: Vec::new(),
        repositories: vec![
            repository(
                "github:Jitsusama/core.nix",
                "core.nix",
                Some("Jitsusama"),
                vec![
                    tree("/home/joel/trees/say trees", true),
                    tree("/home/joel/trees/pinned", false),
                ],
            ),
            repository("local:~/src/my notes", "my notes", None, Vec::new()),
        ],
    }
}

/// A folder of its own for `name`, empty.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("repos-homes-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a folder for the homes");
    folder
}

fn names_in(folder: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("the folder")
        .map(|each| {
            each.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn read(file: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(file).expect("the homes")).expect("homes as JSON")
}

#[test]
fn each_repository_and_each_tree_on_the_disk_is_a_home_at_its_place_on_the_page() {
    let homes = of(&record());
    assert_eq!(
        homes,
        json!({"things": [
            {
                "kind": "repository",
                "key": "github:Jitsusama/core.nix",
                "name": "core.nix",
                "names": ["core.nix"],
                "address": "code/#repo=Jitsusama%2Fcore.nix",
            },
            {
                "kind": "repository",
                "key": "local:~/src/my notes",
                "name": "my notes",
                "names": ["my notes"],
                "address": "code/#repo=my%20notes",
            },
            {
                "kind": "tree",
                "key": "/home/joel/trees/say trees",
                "name": "/home/joel/trees/say trees",
                "names": ["/home/joel/trees/say trees"],
                "address": "code/#tree=%2Fhome%2Fjoel%2Ftrees%2Fsay%20trees",
            },
        ]})
    );
}

#[test]
fn an_address_keeps_only_letters_digits_and_four_marks_as_they_are() {
    assert_eq!(uri("Az09-_.~"), "Az09-_.~");
    assert_eq!(uri("a b/c#d?e&f"), "a%20b%2Fc%23d%3Fe%26f");
    assert_eq!(uri("café"), "caf%C3%A9");
}

#[test]
fn the_homes_are_written_whole_for_anyone_to_read_and_nothing_else_is_left() {
    let folder = folder("whole");
    let file = folder.join("homes.json");
    let mut homes = Homes::at(file.clone());
    assert!(
        homes.write(&record()).expect("written"),
        "nothing was written"
    );
    assert_eq!(read(&file), of(&record()));
    // The writing begins private, so this holds only if it's opened to readers once whole.
    let mode = fs::metadata(&file).expect("the homes").permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o644,
        "the homes can't be read by every reader"
    );
    assert_eq!(
        names_in(&folder),
        ["homes.json"],
        "a writing left something behind"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn the_homes_go_by_their_keys_so_a_reading_that_only_reorders_them_changes_none() {
    // A reading lists the newest written first, so every commit reorders them.
    let mut both = record();
    both.repositories[0].checkouts[0].trees[1].present = true;
    let mut reordered = both.clone();
    reordered.repositories.reverse();
    reordered.repositories[1].checkouts[0].trees.reverse();
    assert_eq!(of(&reordered), of(&both));
}

#[test]
fn homes_unchanged_on_the_disk_are_not_written_again() {
    let folder = folder("unchanged");
    let file = folder.join("homes.json");
    let mut homes = Homes::at(file.clone());
    homes.write(&record()).expect("written");
    let first = fs::metadata(&file).expect("the homes").ino();
    assert!(
        !homes.write(&record()).expect("read"),
        "the same homes were written again"
    );
    let then = fs::metadata(&file).expect("the homes").ino();
    assert_eq!(then, first, "the same homes were put in place again");
    let mut more = record();
    more.repositories.pop();
    assert!(
        homes.write(&more).expect("written"),
        "changed homes weren't written"
    );
    assert_eq!(read(&file), of(&more));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn homes_another_writer_changed_or_removed_are_written_again_at_the_next_reading() {
    let folder = folder("clobbered");
    let file = folder.join("homes.json");
    let mut homes = Homes::at(file.clone());
    homes.write(&record()).expect("written");
    fs::write(&file, "x").expect("changed");
    assert!(
        homes.write(&record()).expect("written"),
        "changed homes stayed"
    );
    assert_eq!(read(&file), of(&record()));
    fs::remove_file(&file).expect("removed");
    assert!(
        homes.write(&record()).expect("written"),
        "removed homes stayed gone"
    );
    assert_eq!(read(&file), of(&record()));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_writing_left_half_done_goes_at_the_next_and_its_link_is_never_followed() {
    let folder = folder("left");
    let file = folder.join("homes.json");
    let elsewhere = folder.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("a folder elsewhere");
    let target = elsewhere.join("kept");
    fs::write(&target, "kept").expect("a file elsewhere");
    // A daemon killed mid-writing, or a link put where the next writes.
    symlink(&target, folder.join(".homes.json.writing")).expect("a link left");
    let mut homes = Homes::at(file.clone());
    assert!(
        homes.write(&record()).expect("written"),
        "nothing was written"
    );
    assert_eq!(read(&file), of(&record()));
    assert_eq!(
        fs::read_to_string(&target).expect("kept"),
        "kept",
        "the link was followed"
    );
    assert_eq!(
        names_in(&folder),
        ["elsewhere", "homes.json"],
        "the half-done writing stayed"
    );
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_writing_that_fails_leaves_nothing_and_is_tried_again_at_the_next() {
    let folder = folder("fails");
    let file = folder.join("homes.json");
    let mut homes = Homes::at(file.clone());
    homes.write(&record()).expect("written");
    // A folder full where the file goes takes no rename, even by root.
    fs::remove_file(&file).expect("the homes gone");
    fs::create_dir_all(file.join("in the way")).expect("a folder in the way");
    let mut fewer = record();
    fewer.repositories.pop();
    assert!(
        homes.write(&fewer).is_err(),
        "a rename onto a full folder succeeded"
    );
    assert_eq!(
        names_in(&folder),
        ["homes.json"],
        "a failed writing left its file"
    );
    fs::remove_dir_all(&file).expect("the folder out of the way");
    // The homes last written are written again, since the file no longer holds them.
    assert!(
        homes.write(&record()).expect("written"),
        "the homes weren't tried again"
    );
    assert_eq!(read(&file), of(&record()));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_failure_is_said_once_until_the_homes_are_written_again() {
    let folder = folder("said");
    let file = folder.join("homes.json");
    fs::create_dir_all(file.join("in the way")).expect("a folder in the way");
    let mut homes = Homes::at(file.clone());
    let said = homes.keep(&record()).expect("the failure said");
    assert!(
        said.starts_with(&format!(
            "repos: couldn't write Code's homes to {}",
            file.display()
        )) && said.ends_with("; the last stay"),
        "the failure said {said:?}"
    );
    assert_eq!(
        homes.keep(&record()),
        None,
        "the same failure was said again"
    );
    fs::remove_dir_all(&file).expect("the folder out of the way");
    assert_eq!(
        homes.keep(&record()),
        None,
        "a writing that worked said something"
    );
    assert_eq!(read(&file), of(&record()));
    fs::remove_file(&file).expect("the homes gone");
    fs::create_dir_all(file.join("in the way")).expect("a folder in the way");
    let mut fewer = record();
    fewer.repositories.pop();
    assert!(
        homes.keep(&fewer).is_some(),
        "a failure after a writing wasn't said"
    );
    let _ = fs::remove_dir_all(&folder);
}
