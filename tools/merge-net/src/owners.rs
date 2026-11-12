//! Which VM tests own a file a change touches, so `merge-net needs` can
//! name them: a test owns its own files, those `checks.nix` imports for it
//! and each path they name beside them, what each .nix file among those
//! names in turn unless it builds a crate, and the folder of each module or
//! role it imports by name. A test that imports a role tests the machine
//! the role makes, not each module in it, so it owns only the role's
//! folder: otherwise a change to any module would name every desktop test.
//! On 10 November #2883 changed the slices module's disk targets, nothing
//! named the slices VM test, and it failed on main only.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::rules::matches;

/// The VM tests that own one of `files`, read from the checkout at `root`,
/// or nothing when its `checks.nix` or `flake.nix` can't be read.
pub fn in_tree(root: &Path, files: &[String]) -> Option<Vec<String>> {
    let checks = fs::read_to_string(root.join("checks.nix")).ok()?;
    let folders = folders(&fs::read_to_string(root.join("flake.nix")).ok()?);
    let read = |file: &str| fs::read_to_string(root.join(file)).ok();
    let tests: Vec<(String, Vec<String>)> = every_vm_test(&checks, read)
        .into_iter()
        .map(|(name, file)| {
            // A file it can't read still owns itself.
            let text = fs::read_to_string(root.join(&file)).unwrap_or_default();
            let paths = owned(&file, &text, &folders, read);
            (name, paths)
        })
        .collect();
    Some(owners(&tests, files))
}

/// Each VM test the root `checks.nix`, holding `checks`, names, and each
/// the folders' `checks.nix` it gathers name, with the file it imports for
/// it from the repository's root. Only a folder's file the root names is
/// read, through `read`, so one nothing gathers names no test, as in Nix.
pub fn every_vm_test(checks: &str, read: impl Fn(&str) -> Option<String>) -> Vec<(String, String)> {
    let mut found = vm_tests(checks);
    for gathered in gathered(checks) {
        let Some(text) = read(&gathered) else {
            continue;
        };
        let here = gathered.rsplit_once('/').map_or("", |(folder, _)| folder);
        found.extend(
            vm_tests(&text)
                .into_iter()
                .filter_map(|(name, file)| Some((name, joined(here, &file)?))),
        );
    }
    found
}

/// The folders' `checks.nix` that `checks` gathers, one to a line of its
/// list: `modules/zsh/checks.nix` for `./modules/zsh/checks.nix`.
fn gathered(checks: &str) -> Vec<String> {
    checks
        .lines()
        .filter_map(|line| line.trim().strip_prefix("./"))
        .filter(|path| path.ends_with("/checks.nix"))
        .map(str::to_owned)
        .collect()
}

/// Each VM test one `checks.nix` names, with the file it imports for it
/// from that file's folder.
pub fn vm_tests(checks: &str) -> Vec<(String, String)> {
    checks
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.trim().split_once(" = import ./")?;
            let file = rest.split_whitespace().next()?;
            let nix = Path::new(file)
                .extension()
                .is_some_and(|extension| extension == "nix");
            (name.starts_with("vm-") && nix).then(|| (name.to_owned(), file.to_owned()))
        })
        .collect()
}

/// The folder each module, role or hardware lives in, by its module
/// system and name (`nixosModules.slices`), as `flake.nix` lists them.
pub fn folders(flake: &str) -> BTreeMap<String, String> {
    let mut system = None;
    let mut found = BTreeMap::new();
    for line in flake.lines().map(str::trim) {
        if let Some(name) = line.strip_suffix(" = {")
            && name.ends_with("Modules")
        {
            system = Some(name.to_owned());
            continue;
        }
        let Some(system) = &system else { continue };
        let Some((name, rest)) = line.split_once(" = ") else {
            continue;
        };
        let path = rest.trim_start_matches("moduleFrom ");
        let Some(path) = path.strip_prefix("./") else {
            continue;
        };
        let mut parts = path.split('/');
        if let (Some(kind @ ("modules" | "roles" | "hardware")), Some(folder)) =
            (parts.next(), parts.next())
        {
            found.insert(format!("{system}.{name}"), format!("{kind}/{folder}"));
        }
    }
    found
}

/// The paths a test whose file is `file`, holding `text`, owns, as the
/// patterns `rules::matches` reads.
/// A file among them is read with `read`, by its path from the root.
pub fn owned(
    file: &str,
    text: &str,
    folders: &BTreeMap<String, String>,
    read: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut paths = vec![file.to_owned()];
    named(file, text, &read, &mut paths);
    let imported: Vec<&String> = names(text)
        .iter()
        .filter_map(|name| folders.get(name))
        .collect();
    let roles = imported.iter().any(|folder| folder.starts_with("roles/"));
    paths.extend(
        imported
            .into_iter()
            .filter(|folder| !roles || folder.starts_with("roles/"))
            .map(|folder| format!("{folder}/*")),
    );
    paths.sort();
    paths.dedup();
    paths
}

/// The VM tests, of `tests` with what each owns, that own one of `files`.
pub fn owners(tests: &[(String, Vec<String>)], files: &[String]) -> Vec<String> {
    let mut found: Vec<String> = tests
        .iter()
        .filter(|(_, paths)| {
            files
                .iter()
                .any(|file| paths.iter().any(|path| matches(path, file)))
        })
        .map(|(name, _)| name.clone())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Into `found`, the paths `text`, which `file` holds, names beside it, and
/// what each .nix file among them names in turn, since a file a test reads
/// may join others: on 12 November #3599 split the launcher's words into
/// a folder words.nix joins, and no desktop test owned them any more. A
/// file that builds a crate, naming its Cargo.lock, passes nothing on: the
/// crate's own checks cover its code, and the net its VM tests after the
/// merge, where owning it would put every desktop test before each merge.
fn named(file: &str, text: &str, read: &impl Fn(&str) -> Option<String>, found: &mut Vec<String>) {
    let here = file.rsplit_once('/').map_or("", |(folder, _)| folder);
    for path in beside(text).iter().filter_map(|path| joined(here, path)) {
        if found.contains(&path) {
            continue;
        }
        found.push(path.clone());
        let nix = Path::new(&path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("nix"));
        let more = nix.then(|| read(&path)).flatten();
        if let Some(more) = more.filter(|more| !more.contains("Cargo.lock")) {
            named(&path, &more, read, found);
        }
    }
}

/// The relative paths `text` names, `./x` and `../x`.
fn beside(text: &str) -> Vec<String> {
    let is_path = |c: char| c.is_ascii_alphanumeric() || "._/-".contains(c);
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("./") {
        let start = rest[..at].rfind(|c: char| !is_path(c)).map_or(0, |i| i + 1);
        let end = at
            + rest[at..]
                .find(|c: char| !is_path(c))
                .unwrap_or(rest.len() - at);
        let path = &rest[start..end];
        if path.starts_with("./") || path.starts_with("../") {
            found.push(path.trim_end_matches('.').to_owned());
        }
        rest = &rest[end.max(at + 2)..];
    }
    found
}

/// `path` taken from the folder `here`, with its `.` and `..` worked out,
/// or nothing when it climbs out of the repository.
fn joined(here: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = here.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// The modules and roles `text` imports by name, `nixosModules.slices`
/// for `self.nixosModules.slices`.
fn names(text: &str) -> Vec<String> {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    text.match_indices("self.")
        .filter_map(|(at, _)| {
            let rest = &text[at + "self.".len()..];
            let (system, rest) = rest.split_once('.')?;
            let name: String = rest.chars().take_while(|&c| is_name(c)).collect();
            (system.ends_with("Modules")
                && system.chars().all(char::is_alphanumeric)
                && !name.is_empty())
            .then(|| format!("{system}.{name}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAKE: &str = "
      homeModules = {
        slices = ./modules/slices/home.nix;
        base = moduleFrom ./roles/base/home.nix { inherit (self) homeModules; };
      };
      nixosModules = {
        graphical = moduleFrom ./roles/graphical/nixos.nix {
          inherit (self) nixosModules;
        };
        memory = ./modules/memory/nixos.nix;
        slices = ./modules/slices/nixos.nix;
      };
      checks = import ./checks.nix;
    ";

    const CHECKS: &str = "
    vm-apps-come-first = import ./tests/slices.nix { inherit self pkgs; };
    vm-desktop-captures = import ./tests/desktop.nix {
      area = \"captures\";
    };
    fast-thing = import ./tests/fast.nix { };
    ";

    const SLICES: &str = "
      tests = ./slices-vm-test.py;
      wakes = \"${./slices-foreground-wakes.sh}\";
      import ./lib/pytest-vm-test.nix { inherit pkgs; } {
        nodes.machine = { imports = [ self.nixosModules.slices ]; };
      }
    ";

    const DESKTOP: &str = "
      imports = [ self.nixosModules.graphical self.nixosModules.slices ];
      home = [ self.homeModules.base ];
      script = ../modules/quickshell/tests/x.qml;
      vocabulary = \"${(import ../modules/quickshell/launcher/words.nix).file}\";
      guard = pkgs.callPackage ../modules/disk-guard/package.nix { };
    ";

    const WORDS: &str = "
      frame = ./words/frame.json;
      places = { on = ./words/on.json; };
      file = builtins.toFile \"words.json\" (builtins.toJSON (read ./words.nix));
    ";

    const GUARD: &str = "
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;
      helpers = ../daemon-serve/package.nix;
    ";

    /// The files the stand-in tests name, by their paths from the root.
    fn read_named(file: &str) -> Option<String> {
        match file {
            "modules/quickshell/launcher/words.nix" => Some(WORDS.to_owned()),
            "modules/disk-guard/package.nix" => Some(GUARD.to_owned()),
            _ => None,
        }
    }

    const GATHERING: &str = "
    vm-apps-come-first = import ./tests/slices.nix { inherit self pkgs; };
    ++ map (file: import file given) [
      ./modules/zsh/checks.nix
      ./tests/vm/keys/checks.nix
    ]
    ";

    const KEYS: &str = "
    onLinux {
      vm-commits-are-signed = import ./signing.nix { inherit self pkgs; };
    }
    ";

    const SIGNING: &str = "
      tests = ./signing-vm-test.py;
      import ../../lib/pytest-vm-test.nix { inherit pkgs; } { }
    ";

    fn read_area(file: &str) -> Option<String> {
        match file {
            "tests/vm/keys/checks.nix" => Some(KEYS.to_owned()),
            "tests/vm/loose/checks.nix" => Some("vm-loose = import ./loose.nix { };".to_owned()),
            _ => None,
        }
    }

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn owning() -> Vec<(String, Vec<String>)> {
        let folders = folders(FLAKE);
        vm_tests(CHECKS)
            .into_iter()
            .map(|(name, file)| {
                let text = if file == "tests/slices.nix" {
                    SLICES
                } else {
                    DESKTOP
                };
                let paths = owned(&file, text, &folders, read_named);
                (name, paths)
            })
            .collect()
    }

    #[test]
    fn checks_name_each_vm_test_with_its_file() {
        assert_eq!(
            vm_tests(CHECKS),
            vec![
                (
                    "vm-apps-come-first".to_owned(),
                    "tests/slices.nix".to_owned()
                ),
                (
                    "vm-desktop-captures".to_owned(),
                    "tests/desktop.nix".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_folder_the_root_gathers_names_its_vm_tests_from_there() {
        assert_eq!(
            every_vm_test(GATHERING, read_area),
            vec![
                (
                    "vm-apps-come-first".to_owned(),
                    "tests/slices.nix".to_owned()
                ),
                (
                    "vm-commits-are-signed".to_owned(),
                    "tests/vm/keys/signing.nix".to_owned()
                ),
            ],
            "tests/vm/loose/checks.nix is gathered by nothing, so names nothing"
        );
    }

    #[test]
    fn a_change_to_a_gathered_tests_file_names_that_test() {
        let folders = folders(FLAKE);
        let tests: Vec<(String, Vec<String>)> = every_vm_test(GATHERING, read_area)
            .into_iter()
            .map(|(name, file)| {
                let text = if file == "tests/slices.nix" {
                    SLICES
                } else {
                    SIGNING
                };
                let paths = owned(&file, text, &folders, read_named);
                (name, paths)
            })
            .collect();
        assert_eq!(
            owners(&tests, &words("tests/vm/keys/signing-vm-test.py")),
            words("vm-commits-are-signed")
        );
        assert_eq!(
            owners(&tests, &words("tests/lib/pytest-vm-test.nix")),
            words("vm-apps-come-first vm-commits-are-signed")
        );
    }

    #[test]
    fn the_flake_names_each_folder_by_its_module_system() {
        let folders = folders(FLAKE);
        assert_eq!(folders["nixosModules.slices"], "modules/slices");
        assert_eq!(folders["homeModules.slices"], "modules/slices");
        assert_eq!(folders["nixosModules.graphical"], "roles/graphical");
        assert_eq!(folders["homeModules.base"], "roles/base");
        assert_eq!(folders.len(), 5, "checks.nix is no module: {folders:?}");
    }

    #[test]
    fn a_test_owns_its_files_and_the_modules_it_imports() {
        let paths = owned("tests/slices.nix", SLICES, &folders(FLAKE), read_named);
        assert_eq!(
            paths,
            words(
                "modules/slices/* tests/lib/pytest-vm-test.nix tests/slices-foreground-wakes.sh \
                 tests/slices-vm-test.py tests/slices.nix"
            )
        );
    }

    #[test]
    fn a_test_of_a_role_owns_the_role_not_its_modules() {
        let paths = owned("tests/desktop.nix", DESKTOP, &folders(FLAKE), |_: &str| {
            None
        });
        assert_eq!(
            paths,
            words(
                "modules/disk-guard/package.nix modules/quickshell/launcher/words.nix \
                 modules/quickshell/tests/x.qml roles/base/* roles/graphical/* tests/desktop.nix"
            )
        );
    }

    #[test]
    fn a_file_a_test_names_passes_on_what_it_names_unless_it_builds_a_crate() {
        let tests = owning();
        assert_eq!(
            owners(&tests, &words("modules/quickshell/launcher/words/on.json")),
            words("vm-desktop-captures"),
            "a place's words, which words.nix joins, named no desktop test, as after #3599"
        );
        assert_eq!(
            owners(
                &tests,
                &words("modules/disk-guard/Cargo.lock modules/daemon-serve/package.nix")
            ),
            Vec::<String>::new(),
            "a crate's own files put the desktop tests before its merge"
        );
    }

    #[test]
    fn a_change_to_a_module_names_the_test_that_imports_it() {
        let tests = owning();
        assert_eq!(
            owners(&tests, &words("modules/slices/nixos.nix docs/glossary.md")),
            words("vm-apps-come-first"),
            "as #2883's change should have"
        );
        assert_eq!(
            owners(&tests, &words("tests/slices-vm-test.py")),
            words("vm-apps-come-first")
        );
        assert_eq!(
            owners(&tests, &words("roles/graphical/nixos.nix")),
            words("vm-desktop-captures")
        );
        assert_eq!(
            owners(&tests, &words("tests/lib/pytest-vm-test.nix")),
            words("vm-apps-come-first")
        );
        assert_eq!(
            owners(&tests, &words("modules/kitty/home.nix")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_path_that_climbs_out_of_the_repository_is_dropped() {
        assert_eq!(joined("tests", "../../x"), None);
        assert_eq!(
            joined("tests/vm", "./a/../b.py"),
            Some("tests/vm/b.py".to_owned())
        );
        assert_eq!(
            beside("a ./x.py, \"${../y/z.sh}\" and 1.5./"),
            words("./x.py ../y/z.sh")
        );
    }
}
