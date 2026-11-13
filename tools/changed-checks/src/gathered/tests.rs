use super::*;
use crate::reads::Touched;

#[test]
fn code_keeps_what_a_string_interpolates_and_nothing_a_comment_says() {
    let text = "a = \"x ../.. ${./b.sh} y\"; # ../c\nd = ''\n  ''${./e} ${../f}\n'';\n/* ./g */ h";

    assert_eq!(
        code(text),
        "a =  ${./b.sh}  ; \nd =  ${../f}  ;\n h",
        "a string's text, an escaped interpolation or a comment was kept as code"
    );
}

#[test]
fn paths_are_the_literals_that_copy_or_read() {
    let text = "src = ../..; f = ./x.nix; g = lib.fileset.toSource {\n  root = ../.;\n  \
                fileset = ./.;\n}; h = lib.path.removePrefix ./. file; i = a./b; \
                j = ./${name}.nix;";

    assert_eq!(
        paths(&code(text)),
        ["../..", "./x.nix", "./.", "./"],
        "a fileset's root, a path taken from, or a name with a slash counted as a path"
    );
}

#[test]
fn a_folder_filtered_for_files_of_another_kind_than_nix_names_none() {
    let text = "a = lib.fileset.fileFilter (file: file.hasExt \"qml\") ../.; \
                b = fileFilter (f: f.hasExt \"nix\") ../y; \
                c = fileFilter (f: g.hasExt \"qml\") ../z; \
                d = fileFilter (f: f.hasExt \"q${x}\") ../w; e = toString ../.;";

    assert_eq!(
        paths(&code(text)),
        ["../y", "../z", "../w", "../."],
        "a folder filtered for QML counted, or one filtered for Nix, for another's \
         kind, for a kind said by interpolation or used otherwise did not"
    );
}

#[test]
fn a_path_after_any_text_is_found() {
    let text = format!("a = [ {} ]; b = ../x;", "é ".repeat(300));

    assert_eq!(
        paths(&code(&text)),
        ["../x"],
        "a path after other letters went unseen"
    );
}

#[test]
fn a_path_resolves_from_its_folder_and_never_above_the_root() {
    assert_eq!(
        resolved("modules/x", "../..").as_deref(),
        Some(""),
        "../.. from modules/x"
    );
    assert_eq!(
        resolved("modules/x", "./checks.nix").as_deref(),
        Some("modules/x/checks.nix"),
        "a file beside it"
    );
    assert_eq!(
        resolved("modules/x", "../y/.").as_deref(),
        Some("modules/y"),
        "a sibling"
    );
    assert_eq!(resolved("", "../x"), None, "a path above the root");
}

#[test]
fn the_whole_source_is_used_as_a_path_or_for_the_outputs_holding_every_check() {
    for text in [
        "\"${self}/x\"",
        "src = self;",
        "core + \"/x\"",
        "toString self",
        "self.outPath",
        "lib.attrNames self.checks.x86_64-linux",
        "core.legacyPackages.x",
        "self.${system}",
    ] {
        assert!(
            takes_whole(&code(text), false),
            "{text} uses the source or every check"
        );
    }
    for text in ["\"${self}/x\"", "src = self;", "self.outPath"] {
        assert!(
            takes_whole(&code(text), true),
            "{text} uses the source in a file making the outputs"
        );
    }
    for text in [
        "self.checks.${system}",
        "self.legacyPackages.x",
        "self.${system}",
    ] {
        assert!(
            !takes_whole(&code(text), true),
            "{text} makes another output in a file making them"
        );
    }
    for text in [
        "core = self;",
        "self.nixosModules.x",
        "self.checksum",
        "{ self, ... }:",
        "itself",
        "inherit core;",
    ] {
        assert!(
            !takes_whole(&code(text), false),
            "{text} uses no path of the source"
        );
    }
}

/// A source whose files are `written`, each with its text, in a folder of
/// its own named for `test`.
fn source(test: &str, written: &[(&str, &str)]) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("gathered-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    for (file, text) in written {
        let path = folder.join(file);
        fs::create_dir_all(path.parent().expect("a folder")).expect("made");
        fs::write(path, text).expect("written");
    }
    folder
}

/// The files each check of the tests reads, every gathered file among them.
const READ: [&str; 4] = ["flake.nix", "checks.nix", "x/checks.nix", "y/checks.nix"];

/// What each of the checks `files` names read, each of `read`, narrowed
/// from the source `at`.
fn narrowed(at: &Path, files: &[(&str, &str)], read: &[&str]) -> BTreeMap<String, Vec<String>> {
    let files: BTreeMap<String, String> = files
        .iter()
        .map(|(name, file)| ((*name).to_owned(), (*file).to_owned()))
        .collect();
    let read: BTreeSet<String> = read.iter().map(|file| (*file).to_owned()).collect();
    let mut each: BTreeMap<&str, Reads> = files
        .keys()
        .map(|name| {
            let reads = Reads {
                read: read.clone(),
                ..Reads::default()
            };
            (name.as_str(), reads)
        })
        .collect();
    narrow(at, &files, &mut each);
    each.into_iter()
        .map(|(name, reads)| (name.to_owned(), reads.read.into_iter().collect()))
        .collect()
}

#[test]
fn a_file_a_gathered_one_imports_for_every_check_is_kept_only_by_its_own() {
    // modules/quickshell/checks.nix imports 39 files of its checks, which
    // every check then read, so a change to one reached all 882.
    let at = source(
        "sub-gathered",
        &[
            ("flake.nix", "{ }"),
            (
                "checks.nix",
                "given = import ./given.nix; folders = [ ./x/checks.nix ./q/checks.nix ];",
            ),
            ("given.nix", "{ }"),
            ("x/checks.nix", "{ }: { a = 1; }"),
            ("q/checks.nix", "{ }: map import [ ./one.nix ./two.nix ]"),
            ("q/one.nix", "{ q = 1; }"),
            ("q/two.nix", "{ r = 1; }"),
        ],
    );
    let read = [
        "checks.nix",
        "flake.nix",
        "given.nix",
        "q/checks.nix",
        "q/one.nix",
        "q/two.nix",
        "x/checks.nix",
    ];

    let each = narrowed(&at, &[("a", "x/checks.nix"), ("q", "q/checks.nix")], &read);

    let _ = fs::remove_dir_all(&at);
    assert_eq!(
        each["a"],
        ["checks.nix", "flake.nix", "given.nix", "x/checks.nix"],
        "a kept a file only another folder's checks import, or lost one checks.nix does"
    );
    assert_eq!(
        each["q"],
        [
            "checks.nix",
            "flake.nix",
            "given.nix",
            "q/checks.nix",
            "q/one.nix",
            "q/two.nix"
        ],
        "q lost a file its own checks import"
    );
}

#[test]
fn a_check_keeps_only_the_gathered_files_its_own_names() {
    let at = source(
        "narrowed",
        &[
            // The flake makes other outputs of its checks, which every
            // check reads, and from 13 November each kept every file.
            (
                "flake.nix",
                "{ devShells = { checks = self.checks.${system}; }; }",
            ),
            (
                "checks.nix",
                "{ self }: folders = [ ./x/checks.nix ./y/checks.nix ]; r = removePrefix ./. f; \
                 s = self.checks;",
            ),
            (
                "x/checks.nix",
                "{ }: { a = 1; b = lib.fileset.toSource { root = ../.; fileset = ./.; }; }",
            ),
            ("y/checks.nix", "{ }: { c = \"${../x}\"; }"),
        ],
    );

    let each = narrowed(
        &at,
        &[
            ("a", "x/checks.nix"),
            ("c", "y/checks.nix"),
            ("inline", "checks.nix"),
        ],
        &READ,
    );

    let _ = fs::remove_dir_all(&at);
    assert_eq!(
        each["a"],
        ["checks.nix", "flake.nix", "x/checks.nix"],
        "a kept a gathered file its own file doesn't name"
    );
    assert_eq!(
        each["c"],
        ["checks.nix", "flake.nix", "x/checks.nix", "y/checks.nix"],
        "c lost the folder its own file copies"
    );
    assert_eq!(
        each["inline"],
        ["checks.nix", "flake.nix", "x/checks.nix", "y/checks.nix"],
        "a check checks.nix gives lost a gathered file"
    );
}

#[test]
fn a_check_filtering_a_folder_for_qml_keeps_none_of_its_nix_files() {
    // Quickshell's checks found every QML file under modules/, so from 13
    // November each of its 336 kept every folder's checks.nix.
    let filtered = "lib.fileset.fileFilter (file: file.hasExt \"qml\") ../.";
    let at = source(
        "filtered",
        &[
            ("flake.nix", "{ }"),
            ("checks.nix", "folders = [ ./x/checks.nix ./y/checks.nix ];"),
            (
                "x/checks.nix",
                &format!("{{ }}: {{ a = lib.fileset.toList ({filtered}); }}"),
            ),
            (
                "z/checks.nix",
                &format!("{{ }}: {{ b = map (f: readFile (dir + f)) ({filtered}); dir = ../.; }}"),
            ),
            ("y/checks.nix", "{ }: { c = 1; }"),
        ],
    );
    let read = [
        "flake.nix",
        "checks.nix",
        "x/checks.nix",
        "y/checks.nix",
        "z/checks.nix",
    ];

    let mut each = narrowed(
        &at,
        &[
            ("a", "x/checks.nix"),
            ("b", "z/checks.nix"),
            ("c", "y/checks.nix"),
        ],
        &read,
    );

    let _ = fs::remove_dir_all(&at);
    assert_eq!(
        each["a"],
        ["checks.nix", "flake.nix", "x/checks.nix"],
        "a check filtering a folder for QML kept a checks.nix under it"
    );
    assert_eq!(
        each["b"],
        [
            "checks.nix",
            "flake.nix",
            "x/checks.nix",
            "y/checks.nix",
            "z/checks.nix"
        ],
        "a check also using the folder another way lost a checks.nix under it"
    );
    // The filter's folders are listed, so a file added to one still
    // reaches the check, as the filter may take it.
    let reads = Reads {
        read: each.remove("a").expect("a").into_iter().collect(),
        listed: BTreeSet::from(["y".to_owned()]),
        ..Reads::default()
    };
    assert!(
        !reads.reached_by("y/checks.nix", Touched::Changed, &|_| false),
        "a change to a checks.nix the filter can't take reached the check"
    );
    assert!(
        reads.reached_by("y/new.qml", Touched::Added, &|_| true),
        "a file added where the filter looks didn't reach the check"
    );
}

#[test]
fn a_check_whose_files_use_the_whole_source_or_cant_be_read_keeps_them_all() {
    let at = source(
        "whole",
        &[
            ("flake.nix", "{ }"),
            ("checks.nix", "folders = [ ./x/checks.nix ];"),
            ("x/checks.nix", "{ core }: { a = \"${core}/y\"; }"),
            (
                "y/checks.nix",
                "{ self }: { c = attrNames self.checks.x86_64-linux; }",
            ),
        ],
    );

    let each = narrowed(
        &at,
        &[
            ("a", "x/checks.nix"),
            ("b", "z/checks.nix"),
            ("c", "y/checks.nix"),
        ],
        &READ,
    );

    let _ = fs::remove_dir_all(&at);
    let all = ["checks.nix", "flake.nix", "x/checks.nix", "y/checks.nix"];
    assert_eq!(
        each["a"], all,
        "a check using the whole source lost a gathered file"
    );
    assert_eq!(
        each["b"], all,
        "a check whose own file can't be read lost a gathered file"
    );
    assert_eq!(
        each["c"], all,
        "a check of which checks exist lost a gathered file"
    );
}
