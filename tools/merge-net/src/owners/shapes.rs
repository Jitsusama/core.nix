//! The shapes the tree's VM tests name their paths in, each as the tree
//! writes it, so a shape the reading drops is seen here rather than as a
//! test `merge-net needs` never names.

use super::*;

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn files(tests: &[VmTest]) -> Vec<(&str, &str)> {
    tests
        .iter()
        .map(|test| (test.name.as_str(), test.file.as_str()))
        .collect()
}

fn owned_by(file: &str, text: &str, given: &[(&str, &str)]) -> Vec<String> {
    let given = given
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    owned(file, text, &given, &BTreeMap::new(), |_: &str| None)
}

#[test]
fn a_path_built_from_a_value_owns_each_file_it_could_name() {
    // 20 November: `tests = ./screens-${area}-vm-test.py;` owned only
    // `tests/vm/screens/screens-`, so a change to a screens case named
    // no VM test to run before it merged.
    let none = BTreeMap::new();
    assert_eq!(
        beside(
            "tests = ./screens-${area}-vm-test.py; f ../theme/${t}.toml;",
            &none
        ),
        words("./screens-*-vm-test.py ../theme/*.toml")
    );
    assert_eq!(
        beside("readFile ./ir-camera/${name}.py) ${./a.sh}", &none),
        words("./ir-camera/*.py ./a.sh")
    );
    assert_eq!(
        beside("./a-${ { b = 1; }.b }.py ./c-${unclosed", &none),
        words("./a-*.py ./c-"),
        "a value's own braces end it at its own closing brace, and one never closed ends it"
    );
    let tests = [(
        "vm-screens".to_owned(),
        owned_by(
            "tests/vm/screens/screens.nix",
            "tests = ./screens-${area}-vm-test.py;",
            &[],
        ),
    )];
    assert_eq!(
        owners(
            &tests,
            &words("tests/vm/screens/screens-launcher-vm-test.py tests/vm/other.py")
        ),
        words("vm-screens")
    );
}

#[test]
fn a_value_the_import_gives_owns_only_the_file_it_names() {
    let desktop = "{ area, ... }: { tests = ./desktop/desktop-${area}-vm-test.py; }";
    let tests = [
        (
            "vm-desktop-captures".to_owned(),
            owned_by("tests/desktop.nix", desktop, &[("area", "captures")]),
        ),
        (
            "vm-desktop-looks".to_owned(),
            owned_by("tests/desktop.nix", desktop, &[("area", "looks")]),
        ),
    ];
    assert_eq!(
        owners(&tests, &words("tests/desktop/desktop-captures-vm-test.py")),
        words("vm-desktop-captures"),
        "one area's cases put every desktop test before its merge"
    );
}

#[test]
fn a_path_inside_a_value_or_from_the_root_or_a_folder_is_owned() {
    assert_eq!(
        owned_by("tests/a.nix", "t = ./a-${import ./b.nix}.py;", &[]),
        words("tests/a-*.py tests/a.nix tests/b.nix"),
        "a path named inside a value built into another is its own"
    );
    assert_eq!(
        owned_by(
            "tests/vm/sound/noise.nix",
            "imports = [ \"${self}/modules/pipewire/nixos.nix\" ];",
            &[],
        ),
        words("modules/pipewire/nixos.nix tests/vm/sound/noise.nix")
    );
    let tests = [(
        "vm-desktop-looks".to_owned(),
        owned_by(
            "tests/desktop.nix",
            "warp_contract = \"${../modules/warp/contract}\"; b = \"${../x.json}\";",
            &[],
        ),
    )];
    assert_eq!(
        owners(&tests, &words("modules/warp/contract/status.json")),
        words("vm-desktop-looks")
    );
    assert_eq!(
        owners(&tests, &words("x.json/y")),
        Vec::<String>::new(),
        "a file is no folder"
    );
}

#[test]
fn a_test_imported_as_a_folder_from_above_or_after_a_let_is_read() {
    let admin = "
      vm-apps-come-first = import ./slices { inherit self pkgs; };
      vm-pi-conversations-come-back =
        let
          hostedPi = import ../../../modules/pi/hosted-pi.nix { inherit pkgs folder; };
        in
        import ./pi-conversations-come-back.nix {
          inherit self pkgs hostedPi;
        };
    ";
    let keys = "
      vm-desktop-answers-secret-asks = import ../../desktop.nix {
        inherit self pkgs;
        area = \"answers-secret-asks\";
      };
    ";
    let root = "
      vm-climbs-out = import ../outside.nix { };
    ++ map (file: import file given) [
      ./tests/vm/admin/checks.nix
      ./tests/vm/keys/checks.nix
    ]
    ";
    let read = |file: &str| match file {
        "tests/vm/admin/checks.nix" => Some(admin.to_owned()),
        "tests/vm/keys/checks.nix" => Some(keys.to_owned()),
        _ => None,
    };
    let tests = every_vm_test(root, read);
    assert_eq!(
        files(&tests),
        vec![
            ("vm-apps-come-first", "tests/vm/admin/slices/default.nix"),
            (
                "vm-pi-conversations-come-back",
                "tests/vm/admin/pi-conversations-come-back.nix"
            ),
            ("vm-desktop-answers-secret-asks", "tests/desktop.nix"),
        ],
        "each was dropped until 20 November, and one from outside the repository still is"
    );
    assert_eq!(
        tests[2].given.get("area").map(String::as_str),
        Some("answers-secret-asks"),
        "a test from above is given its values as any other"
    );
}

#[test]
fn a_nested_set_in_the_call_gives_the_call_nothing() {
    let checks = "
      vm-a = import ./a.nix {
        screen = {
          area = \"left\";
        };
        area = \"captures\";
      };
      vm-b = import ./b.nix { screen = { area = \"left\"; }; };
    ";
    let tests = vm_tests(checks);
    let given: Vec<Option<&str>> = tests
        .iter()
        .map(|test| test.given.get("area").map(String::as_str))
        .collect();
    assert_eq!(
        given,
        vec![Some("captures"), None],
        "a nested set's area is not the call's, and one on a single line gives none"
    );
    let listed = vm_tests(
        "vm-c = import ./c.nix {\n  screens = [\n    { area = \"left\"; }\n  ];\n  \
         both = { a = 1; }; area = \"right\";\n};",
    );
    assert_eq!(
        listed.first().map(|test| test.given.get("area")),
        Some(None),
        "a set inside a list, or beside one on its line, is not the call's own"
    );
}

#[test]
fn a_test_bound_without_an_import_leaves_the_next_test_its_own() {
    let checks = "
      vm-built =
        pkgs.testers.runNixOSTest {
          name = \"built\";
        };
      vm-next =
        import ./next.nix { };
      vm-after = import ./after.nix { };
      vm-bare =
        somethingElse;
      other = import ./not-a-test.nix { };
      vm-let =
        let
          folder = \"x\";
          made = import ./made.nix { };
        in
        import ./let.nix { };
    ";
    assert_eq!(
        files(&vm_tests(checks)),
        vec![
            ("vm-next", "next.nix"),
            ("vm-after", "after.nix"),
            ("vm-let", "let.nix"),
        ],
        "a search past a test's own value took the next one's import and dropped it"
    );
}

#[test]
fn a_value_the_file_binds_again_owns_each_file_it_could_name() {
    let given = BTreeMap::from([("area".to_owned(), "captures".to_owned())]);
    let owns = |text: &str| {
        owned("tests/d.nix", text, &given, &BTreeMap::new(), |_: &str| {
            None
        })
    };
    assert_eq!(
        owns("{ area, ... }: { t = ./d-${area}.py; same = area == \"x\"; }"),
        words("tests/d-captures.py tests/d.nix")
    );
    assert_eq!(
        owns("{ area, ... }: map (area: ./d-${area}.py) areas"),
        words("tests/d-*.py tests/d.nix"),
        "a function's own area is any of them"
    );
    assert_eq!(
        owns("let\n  area = \"looks\";\nin ./d-${area}.py"),
        words("tests/d-*.py tests/d.nix"),
        "a let's own area is not the one given"
    );
    for text in [
        "{ area, ... }: let inherit (other) area; in ./d-${area}.py",
        "{ area, ... }: let inherit (other)\n  area\n  ;\nin ./d-${area}.py",
        "{ area, ... }: map ({ area, ... }: ./d-${area}.py) areas",
        "{ area, ... }: let area = \"looks\"; in ./d-${area}.py",
        "{\n  # the area\n  area,\n}: map ({ area, ... }: ./d-${area}.py) areas",
        "{ area, ... }: let inherit (a.${b}) area; in ./d-${area}.py",
        "{ ... }: with other; ./d-${area}.py",
    ] {
        assert_eq!(
            owns(text),
            words("tests/d-*.py tests/d.nix"),
            "{text} binds its area again"
        );
    }
    assert_eq!(
        owns(
            "{\n  area,\n}: {\n  again = import ./d.nix {\n    inherit\n      area\n      \
             ;\n  };\n  t = ./d-${area}.py;\n}"
        ),
        words("tests/d-captures.py tests/d.nix"),
        "a plain inherit passes the same area on"
    );
}

#[test]
fn a_folder_imported_or_called_owns_its_files() {
    let read = |file: &str| match file {
        "tests/stand-ins/stand-in-lid/default.nix" => {
            Some("src = ./stand-in-lid.c; kbuild = ./Kbuild;".to_owned())
        }
        _ => None,
    };
    let lid = owned(
        "tests/vm/devices/lid.nix",
        "(config.boot.kernelPackages.callPackage ../../stand-ins/stand-in-lid { })",
        &BTreeMap::new(),
        &BTreeMap::new(),
        read,
    );
    let slices = owned(
        "tests/vm/admin/slices/default.nix",
        "tests = ./slices-vm-test.py; contract = \"${self}/modules/warp/contract\";",
        &BTreeMap::new(),
        &BTreeMap::new(),
        |_: &str| None,
    );
    let tests = [
        ("vm-lid".to_owned(), lid),
        ("vm-apps-come-first".to_owned(), slices),
    ];
    assert_eq!(
        owners(&tests, &words("tests/stand-ins/stand-in-lid/Makefile")),
        words("vm-lid")
    );
    assert_eq!(
        owners(
            &tests,
            &words("tests/vm/admin/slices/slices-foreground-wakes.sh")
        ),
        words("vm-apps-come-first"),
        "a test imported as a folder owns all of it"
    );
    assert_eq!(
        owners(&tests, &words("modules/warp/contract/status.json")),
        words("vm-apps-come-first"),
        "a folder named from the root owns its files"
    );
    assert_eq!(
        beside("src = ./.; b = ./bin;", &BTreeMap::new()),
        words("./ ./bin"),
        "a folder neither called nor copied, a crate's `src = ./.`, owns only its name"
    );
}

/// A repository on disk, removed when the test passes and kept, said
/// where, when it fails, so what it held can still be read.
struct Tree(std::path::PathBuf);

impl Tree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("owners-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Self(root)
    }

    fn file(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        let folder = path.parent().expect("a file sits in a folder");
        fs::create_dir_all(folder).expect("the test's folder is made");
        fs::write(path, text).expect("the test's file is written");
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("the tree is kept at {}", self.0.display());
        } else {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

#[test]
fn a_value_checks_gives_reaches_what_the_tree_names() {
    let tree = Tree::new("given");
    tree.file("flake.nix", "checks = import ./checks.nix;");
    tree.file(
        "checks.nix",
        "vm-desktop-looks = import ./tests/desktop.nix { area = \"looks\"; };\n\
         ++ map (file: import file given) [\n  ./tests/vm/keys/checks.nix\n]",
    );
    tree.file(
        "tests/vm/keys/checks.nix",
        "vm-desktop-captures = import ../../desktop.nix { area = \"captures\"; };",
    );
    tree.file(
        "tests/desktop.nix",
        "{ area, ... }: { tests = ./desktop/desktop-${area}-vm-test.py; }",
    );
    let tests = |files: &str| in_tree(&tree.0, &words(files)).map(|owning| owning.tests);
    assert_eq!(
        tests("tests/desktop/desktop-captures-vm-test.py"),
        Some(words("vm-desktop-captures")),
        "one area's cases put every desktop test before its merge"
    );
    assert_eq!(
        tests("tests/desktop.nix"),
        Some(words("vm-desktop-captures vm-desktop-looks"))
    );
    assert_eq!(
        tests("tests/vm/keys/checks.nix"),
        Some(words("vm-desktop-captures")),
        "the area a call gives is part of its test, so a change to it names that test"
    );
    assert_eq!(tests("checks.nix"), Some(words("vm-desktop-looks")));
}

#[test]
fn a_vm_test_the_reading_cant_find_is_named_for_any_change() {
    let tree = Tree::new("unread");
    tree.file("flake.nix", "checks = import ./checks.nix;");
    tree.file(
        "checks.nix",
        "vm-a = import ./tests/a.nix { };\n\
         vm-odd =\n  builtins.seq x (import ./tests/odd.nix { });\n\
         vm-built = pkgs.callPackage ./built.nix { };\n\
         ++ map (file: import file given) [\n  ./tests/vm/x/checks.nix\n]",
    );
    tree.file("tests/a.nix", "{ }");
    tree.file(
        "tests/vm/x/checks.nix",
        "vm-far =\n  builtins.seq x (import ./far.nix { });",
    );
    assert_eq!(
        in_tree(&tree.0, &words("modules/zsh/home.nix")),
        Some(Owning {
            tests: words("vm-far vm-odd"),
            unread: words("vm-far vm-odd"),
        }),
        "a test the reading drops would go unnamed and fail on main only"
    );
    assert_eq!(
        in_tree(&tree.0, &[]),
        Some(Owning {
            tests: Vec::new(),
            unread: Vec::new(),
        }),
        "a change touching nothing names nothing"
    );
}

#[test]
fn a_bracket_in_a_comment_or_string_or_a_nested_let_hides_no_test() {
    let checks = "
      vm-a = import ./a.nix {
        # the screens (left and right
        name = \"a { b\";
        area = \"captures\";
      };
      vm-b =
        let
          made =
            let
              x = 1;
            in
            x;
        in
        import ./b.nix { };
      vm-c = import ./c.nix { };
    ";
    let tests = vm_tests(checks);
    assert_eq!(
        files(&tests),
        vec![("vm-a", "a.nix"), ("vm-b", "b.nix"), ("vm-c", "c.nix")],
        "an unclosed bracket in a comment once hid every test after it"
    );
    assert_eq!(
        tests[0].given.get("area").map(String::as_str),
        Some("captures")
    );
    assert_eq!(
        given::nesting("[ ( { ${x} ] \"(\" # ["),
        2,
        "lists and brackets count"
    );
}

#[test]
fn a_call_holding_a_let_or_an_indented_string_or_merged_onto_gives_nothing() {
    for checks in [
        "vm-a = import ./a.nix {\n  area = let\n    area = \"left\";\n  in area;\n};",
        "vm-a = import ./a.nix {\n  script = ''\n    area = \"left\";\n  '';\n};",
        "vm-a = import ./a.nix {\n  area = \"left\";\n} // { area = \"right\"; };",
        "vm-a = import ./a.nix {\n  area = \"left\";\n}\n// { area = \"right\"; };",
    ] {
        assert_eq!(
            vm_tests(checks).first().map(|test| test.given.len()),
            Some(0),
            "{checks} may give another area than it reads as"
        );
    }
}
