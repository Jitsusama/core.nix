use super::*;
use crate::scratch::Scratch;

/// The flake's source as Nix copied it on the lent builder, where
/// `traced.txt` was traced: one stand-in check for each way a check reads
/// a file, each evaluated alone from `path:` and this path.
const SOURCE: &str = "/nix/store/0rnx5qbx219cyjiiaiml7jl7jkpr3sgs-source";

/// What evaluating the stand-in check `check` read, by its trace.
fn read_by(check: &str) -> Reads {
    let traced = include_str!("traced.txt");
    let section = traced
        .split("== ")
        .find_map(|section| section.strip_prefix(&format!("{check}\n")))
        .unwrap_or_else(|| panic!("traced.txt traces no {check}"));
    Reads::traced(section, SOURCE).unwrap_or_else(|| panic!("{check}'s trace wasn't read"))
}

/// Which of `files`, touched so, reach what `check` read, when no folder
/// came or went with them.
fn reaching(check: &str, files: &[(&str, Touched)]) -> Vec<String> {
    let reads = read_by(check);
    files
        .iter()
        .filter(|(file, how)| reads.reached_by(file, *how, &STAYED))
        .map(|(file, _)| (*file).to_owned())
        .collect()
}

use Touched::{Added, Changed, Removed};

/// No folder came or went.
const STAYED: fn(&str) -> bool = |_| false;

#[test]
fn a_file_read_any_way_is_reached_by_its_change_and_one_elsewhere_isnt() {
    for (check, file) in [
        ("vm-imports", "imported/a.nix"),
        ("vm-reads", "read/b.txt"),
        ("vm-fileset", "set/e.txt"),
        ("vm-src", "whole/f.txt"),
        ("vm-cleaned", "cleaned/g.txt"),
        ("vm-interpolated", "other/h.txt"),
    ] {
        assert_eq!(
            reaching(check, &[(file, Changed), ("elsewhere/x.nix", Changed)]),
            [file],
            "{check} read {file}, by its trace, and nothing else"
        );
    }
}

#[test]
fn a_folder_listed_is_reached_only_by_an_entry_coming_or_going() {
    // readDir gives a folder's names and kinds alone, so a file changed in
    // place moves nothing a listing gave. A copy of a folder reads each
    // file in it as well, which reaches it apart.
    let files = [
        ("listed/c3", Added),
        ("listed/c1", Changed),
        ("listed/c2", Removed),
        ("listed-not/c", Added),
    ];
    assert_eq!(
        reaching("vm-lists", &files),
        ["listed/c3", "listed/c2"],
        "readDir's folder"
    );
    let reads = read_by("vm-lists");
    assert!(
        reads.reached_by("listed/new/c", Added, &|folder| folder == "listed/new"),
        "a folder that came in a listed one is an entry of it"
    );
    assert!(
        !reads.reached_by("listed/old/c", Added, &STAYED),
        "a file added in a folder that was already an entry"
    );
    assert_eq!(
        reaching("vm-src", &[("whole/g.txt", Added)]),
        ["whole/g.txt"],
        "a folder as a source takes in a file added to it"
    );
    assert_eq!(
        reaching("vm-fileset", &[("set/new.txt", Added)]),
        ["set/new.txt"],
        "a fileset of a folder takes in a file added to it"
    );
}

#[test]
fn a_path_looked_up_and_not_found_is_reached_by_adding_it_or_under_it() {
    let files = [
        ("looked/missing", Added),
        ("looked/missing/inside", Added),
        ("looked/present", Changed),
    ];
    assert_eq!(
        reaching("vm-looks", &files),
        ["looked/missing", "looked/missing/inside"],
        "pathExists of a path that wasn't there"
    );
}

#[test]
fn a_folder_only_passed_through_is_reached_only_by_its_going() {
    // Importing imported/a.nix looks the folder up on the way, which a file
    // added, changed or removed beside it can't move while the folder
    // stays: on 13 November #4085 turned one file under modules/ into a
    // folder, and every check, having looked modules/ up, was evaluated
    // in each gate until a record came past it.
    let files = [
        ("imported/new.nix", Added),
        ("imported/b.nix", Changed),
        ("imported/old.nix", Removed),
    ];
    assert_eq!(
        reaching("vm-imports", &files),
        Vec::<String>::new(),
        "a file removed beside one imported reached it, though the folder stayed"
    );
    let reads = read_by("vm-imports");
    assert!(
        reads.reached_by("imported/old.nix", Removed, &|folder| folder == "imported"),
        "the last file removed from a folder looked up took the folder, unreached"
    );
}

#[test]
fn the_flakes_own_files_reach_every_check() {
    assert_eq!(
        reaching(
            "ordinary",
            &[
                ("flake.nix", Changed),
                ("flake.lock", Changed),
                ("imported/a.nix", Changed)
            ]
        ),
        ["flake.nix", "flake.lock"]
    );
    assert!(
        Reads::default().reached_by("flake.lock", Changed, &STAYED),
        "flake.lock reached nothing an empty record holds"
    );
}

#[test]
fn a_line_under_the_source_that_names_no_path_plainly_leaves_nothing_read() {
    let source = format!("\"{SOURCE}");
    for line in [
        format!("chdir({source}/a\") = 0"),
        format!("open(\"a.nix\", O_RDONLY) = 3</{SOURCE}/a.nix>"),
        format!("openat(AT_FDCWD</tmp>, {source}/b\\303\\251.nix\", O_RDONLY) = 3"),
        format!("openat(AT_FDCWD</tmp>, {source}/a/../b.nix\", O_RDONLY) = 3"),
    ] {
        assert_eq!(Reads::traced(&line, SOURCE), None, "{line} was read");
    }
}

#[test]
fn a_split_call_counts_as_read_and_missing_and_the_process_number_is_passed_over() {
    let trace = format!(
        "4242 newfstatat(AT_FDCWD</tmp>, \"{SOURCE}/a.nix\", <unfinished ...>\n\
         4242 <... newfstatat resumed>0x7ffd, 0) = 0\n\
         4243 openat(7<{SOURCE}/set>, \"e.txt\", O_RDONLY|O_CLOEXEC) = 8<{SOURCE}/set/e.txt>\n"
    );
    let reads = Reads::traced(&trace, SOURCE).expect("read");
    assert!(
        reads.reached_by("a.nix", Changed, &STAYED),
        "a split call's path"
    );
    assert!(
        reads.reached_by("a.nix/b", Added, &STAYED),
        "a split call's path may have been missing"
    );
    assert!(
        reads.reached_by("set/e.txt", Changed, &STAYED),
        "a path relative to a folder's descriptor"
    );
}

#[test]
fn a_record_is_kept_for_its_tree_and_read_for_no_other() {
    let reads = read_by("vm-lists");
    let kept = reads.kept("abc").to_string();
    assert_eq!(Reads::from_kept(&kept, "abc"), Some(reads));
    assert_eq!(
        Reads::from_kept(&kept, "abd"),
        None,
        "another tree's record was used"
    );
    assert_eq!(
        Reads::from_kept("{\"tree\": \"abc\"}", "abc"),
        None,
        "a part was missing"
    );
}

#[test]
fn only_what_git_diff_says_of_a_file_is_read_as_its_change() {
    assert_eq!(
        touched("A\tnew.nix\nM\tsome/old.nix\nD\tgone.nix\nT\tlink\n"),
        Some(vec![
            ("new.nix".to_owned(), Added),
            ("some/old.nix".to_owned(), Changed),
            ("gone.nix".to_owned(), Removed),
            ("link".to_owned(), Removed),
        ]),
        "a change of kind takes away the entry a listing gave"
    );
    assert_eq!(touched(""), Some(Vec::new()), "no change touches nothing");
    assert_eq!(touched("R100\ta\tb\n"), None, "a rename went unsaid");
    assert_eq!(
        touched("U\tboth.nix\n"),
        None,
        "an unmerged file went unsaid"
    );
}

#[test]
fn a_flake_input_locked_to_a_local_path_is_found() {
    let scratch = Scratch::new("local-input");
    fs::create_dir_all(&*scratch).expect("made");
    let lock = scratch.join("flake.lock");
    let locked = |locked: &str| {
        format!(
            "{{\"nodes\": {{\"root\": {{\"inputs\": {{\"x\": \"x\"}}}}, \
             \"x\": {{\"locked\": {locked}}}}}, \"root\": \"root\", \"version\": 7}}"
        )
    };
    fs::write(&lock, locked(r#"{"type": "path", "path": "./sub"}"#)).expect("written");
    assert_eq!(local_input(&lock), Ok(Some("x".to_owned())));
    fs::write(&lock, locked(r#"{"type": "git", "url": "file:///here"}"#)).expect("written");
    assert_eq!(local_input(&lock), Ok(Some("x".to_owned())));
    fs::write(
        &lock,
        locked(r#"{"type": "github", "owner": "o", "repo": "r"}"#),
    )
    .expect("written");
    assert_eq!(local_input(&lock), Ok(None));
    assert_eq!(
        local_input(&scratch.join("none.lock")),
        Ok(None),
        "no lock, no inputs"
    );
}
