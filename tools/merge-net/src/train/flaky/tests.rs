//! Whose a red candidate's failed checks are, read from its gate's words.

use super::*;

/// A gate's words, as changed-checks and nix say them, naming `built` as
/// changed and each of `failed` a derivation whose builder failed.
fn words(built: &[&str], failed: &[&str]) -> String {
    let rows = built.iter().map(|check| format!("changed  {check}\n"));
    let errors = failed.iter().map(|name| {
        format!(
            "error: Cannot build '/nix/store/2pglmw1knj57yvg0rm4wv9fk70085wq2-{name}.drv'.\n       \
             Reason: builder failed with exit code 1.\n"
        )
    });
    std::iter::once("kind     check\n".to_owned())
        .chain(rows)
        .chain(errors)
        .chain(["\u{2715} a check the change touched failed\n".to_owned()])
        .collect()
}

/// A judging whose names are none, whose untouched checks are `untouched`,
/// and whose builds at the base end as `at_base` says of each.
fn judged(
    words: &str,
    untouched: &[&str],
    at_base: &dyn Fn(&str) -> Gated,
) -> Option<Vec<(String, Blame)>> {
    let untouched: BTreeSet<String> = untouched.iter().map(|&check| check.to_owned()).collect();
    let mut built = |check: &str| at_base(check);
    let mut judging = Judging {
        named: &|_| BTreeMap::new(),
        untouched: &|_| untouched.clone(),
        at_base: &mut built,
    };
    judge(words, &mut judging)
}

#[test]
fn a_failed_derivation_is_named_once_and_one_a_dependency_stopped_not_at_all() {
    let said = "error: Cannot build '/nix/store/aaaa-the-margin-cat-on-niri.drv'.\n\
                \x20      Reason: builder failed with exit code 1.\n\
                error: Cannot build '/nix/store/bbbb-home-manager-path.drv'.\n\
                \x20      Reason: 1 dependency failed.\n\
                error: Cannot build '/nix/store/aaaa-the-margin-cat-on-niri.drv'.\n\
                \x20      Reason: builder failed with exit code 1.\n";
    assert_eq!(
        failed_derivations(said),
        ["the-margin-cat-on-niri"],
        "a derivation was named twice, or one a dependency stopped was named"
    );
}

#[test]
fn a_failed_derivation_is_placed_by_its_name_its_vm_run_or_what_nix_names_it() {
    let built: Vec<String> = ["formatting", "vm-pi", "a-theme-changes-everything"]
        .map(str::to_owned)
        .to_vec();
    let failed: Vec<String> = [
        "a-theme-changes-everything",
        "vm-test-run-vm-pi",
        "treefmt-check",
    ]
    .map(str::to_owned)
    .to_vec();
    let asked = std::cell::RefCell::new(Vec::new());
    let named = |checks: &[String]| {
        asked.borrow_mut().extend(checks.iter().cloned());
        BTreeMap::from([("treefmt-check".to_owned(), "formatting".to_owned())])
    };
    assert_eq!(
        checks_of(&failed, &built, &named),
        Some(
            ["a-theme-changes-everything", "vm-pi", "formatting"]
                .map(str::to_owned)
                .to_vec()
        )
    );
    assert_eq!(
        *asked.borrow(),
        ["formatting"],
        "nix was asked the names of checks already placed"
    );
    let unnamed = |_: &[String]| BTreeMap::new();
    assert_eq!(
        checks_of(&failed, &built, &unnamed),
        None,
        "a failed derivation no check names was passed over, so the riders were excused for it"
    );
}

#[test]
fn a_rider_whose_unrelated_check_fails_once_stays_and_one_whose_own_fails_is_split() {
    let said = words(
        &["vm-pi", "a-theme-changes-everything"],
        &["vm-test-run-vm-pi"],
    );
    let passes = |_: &str| Gated::Passed;
    let unrelated = judged(&said, &["vm-pi"], &passes).unwrap();
    assert_eq!(unrelated, [("vm-pi".to_owned(), Blame::Flaky)]);
    assert!(
        excused(&unrelated),
        "a flake no rider touches split the train"
    );
    let own = judged(&said, &[], &passes).unwrap();
    assert_eq!(own, [("vm-pi".to_owned(), Blame::Riders)]);
    assert!(
        !excused(&own),
        "a VM test a rider touches was excused for passing at the base"
    );
}

#[test]
fn a_check_that_fails_at_the_base_too_is_mains_and_one_untaken_there_the_riders() {
    let said = words(&["formatting"], &["formatting"]);
    let main = judged(&said, &[], &|_| Gated::Red).unwrap();
    assert_eq!(main, [("formatting".to_owned(), Blame::Main)]);
    assert!(excused(&main), "main's own red split the train");
    let untaken = judged(&said, &["formatting"], &|_| Gated::Unsent).unwrap();
    assert_eq!(
        untaken,
        [("formatting".to_owned(), Blame::Riders)],
        "a build at the base no builder took excused the riders"
    );
}

#[test]
fn a_red_is_excused_only_when_none_of_its_failures_is_the_riders() {
    let said = words(
        &["vm-pi", "formatting"],
        &["vm-test-run-vm-pi", "formatting"],
    );
    let at_base = |check: &str| {
        if check == "formatting" {
            Gated::Passed
        } else {
            Gated::Red
        }
    };
    let judged = judged(&said, &[], &at_base).unwrap();
    assert_eq!(
        judged,
        [
            ("vm-pi".to_owned(), Blame::Main),
            ("formatting".to_owned(), Blame::Riders),
        ]
    );
    assert!(
        !excused(&judged),
        "main's red beside the riders' own excused the riders'"
    );
    assert!(!excused(&[]), "a red naming no failed check was excused");
}

#[test]
fn once_a_failure_is_the_riders_nothing_after_it_is_built_again() {
    let said = words(&["a", "b", "c"], &["a", "b", "c"]);
    let asked = std::cell::RefCell::new(Vec::new());
    let at_base = |check: &str| {
        asked.borrow_mut().push(check.to_owned());
        if check == "a" {
            Gated::Red
        } else {
            Gated::Passed
        }
    };
    let judged = judged(&said, &[], &at_base).unwrap();
    assert_eq!(
        judged.iter().map(|(_, blamed)| *blamed).collect::<Vec<_>>(),
        [Blame::Main, Blame::Riders, Blame::Riders]
    );
    assert_eq!(
        *asked.borrow(),
        ["a", "b"],
        "a check was built again once the red was the riders'"
    );
}

#[test]
fn a_check_is_red_at_the_base_only_where_a_builder_failed_there() {
    assert_eq!(at_base(Some(0), ""), Gated::Passed);
    assert_eq!(at_base(Some(1), &words(&[], &["formatting"])), Gated::Red);
    let missing = "error: flake 'github:someone/core/1234' does not provide attribute \
                   'checks.x86_64-linux.vm-new'\n";
    assert_eq!(
        at_base(Some(1), missing),
        Gated::Unsent,
        "a check the riders added, missing at the base, was blamed on main"
    );
    assert_eq!(at_base(Some(75), ""), Gated::Unsent);
}

#[test]
fn words_naming_no_failed_derivation_say_nothing_of_whose_the_red_is() {
    let said = words(&["vm-pi"], &[]);
    assert_eq!(judged(&said, &["vm-pi"], &|_| Gated::Passed), None);
    let broken = words(&["vm-pi"], &["vm-test-run-vm-pi"])
        + "\u{2715} a check the change touched no longer evaluates\n";
    assert_eq!(
        judged(&broken, &["vm-pi"], &|_| Gated::Passed),
        None,
        "a check that no longer evaluates was passed over beside a flake"
    );
}

/// A tree of the test's own, gone once it ends.
struct Tree(std::path::PathBuf);

impl Tree {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("merge-net-flaky-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn only_a_vm_test_owning_none_of_the_files_is_untouched() {
    let tree = Tree::new("untouched");
    tree.file("flake.nix", "checks = import ./checks.nix;");
    tree.file(
        "checks.nix",
        "vmHostPkgs = pkgs;\n\
         vm-desktop-looks = import ./tests/desktop.nix { area = \"looks\"; };\n\
         vm-pi = import ./tests/pi.nix { };",
    );
    tree.file(
        "tests/desktop.nix",
        "{ area, ... }: { tests = ./desktop/desktop-${area}-vm-test.py; }",
    );
    tree.file("tests/pi.nix", "{ }: { tests = ./pi-vm-test.py; }");
    let checks: Vec<String> = ["vm-desktop-looks", "vm-pi", "formatting"]
        .map(str::to_owned)
        .to_vec();
    let files = |files: &[&str]| {
        files
            .iter()
            .map(|&file| file.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        untouched(&tree.0, &files(&["tests/pi-vm-test.py"]), &checks),
        BTreeSet::from(["vm-desktop-looks".to_owned()]),
        "a VM test owning a rider's file, or a check no reading places, counted as untouched"
    );
    assert_eq!(
        untouched(&tree.0, &files(&["flake.lock"]), &checks),
        BTreeSet::new(),
        "a change to flake.lock, which every test reads, left a test untouched"
    );
}
