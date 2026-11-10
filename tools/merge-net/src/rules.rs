//! Which changes run their VM tests before they merge: `must-run.txt`
//! names each area, the paths that are it and the tests it runs.

use crate::watch::Changed;

/// The areas, as `must-run.txt` holds them, built into the program so it
/// answers the same on any branch it was built from.
const MUST_RUN: &str = include_str!("../must-run.txt");

/// An area whose changes run their VM tests before merging.
#[derive(Debug, PartialEq, Eq)]
pub struct Area {
    pub name: String,
    pub paths: Vec<String>,
    pub tests: Vec<String>,
    /// Whether a change to it is also sent through one VM test end to end,
    /// for what runs VM tests, whose change moves none of them.
    pub there: bool,
}

/// The word among an area's tests that sets `there`.
const THERE: &str = "there";

/// The areas `must-run.txt` names.
pub fn areas() -> Vec<Area> {
    parse(MUST_RUN).expect("must-run.txt reads as areas, which its own test holds")
}

/// Read areas from lines of `name: path... => test...`, skipping comments
/// and blank lines, or say which line doesn't read.
pub fn parse(text: &str) -> Result<Vec<Area>, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, rest) = line
                .split_once(':')
                .ok_or_else(|| format!("no area named before a colon: {line}"))?;
            let (paths, tests) = rest
                .split_once("=>")
                .ok_or_else(|| format!("no `=>` before its tests: {line}"))?;
            let words = |side: &str| {
                side.split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            };
            let mut tests = words(tests);
            let there = tests.iter().any(|test| test == THERE);
            tests.retain(|test| test != THERE);
            let area = Area {
                name: name.trim().to_owned(),
                paths: words(paths),
                tests,
                there,
            };
            if area.paths.is_empty() || (area.tests.is_empty() && !area.there) {
                return Err(format!("an area needs paths and tests: {line}"));
            }
            Ok(area)
        })
        .collect()
}

/// What a change has to run first: each area it touches, with the files
/// that touch it, the VM tests among `known` the areas name, sorted, and
/// the areas to send through one VM test. With no names known, the tests
/// are the areas' patterns.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Needs {
    pub touched: Vec<(String, Vec<String>)>,
    pub tests: Vec<String>,
    pub there: Vec<String>,
}

/// Say which areas `files` touch, and which of `known`'s VM tests they run.
/// Given the VM tests whose derivation moved from main as `known`, only
/// those run, since the rest would give main's result.
pub fn needs(areas: &[Area], files: &[String], known: Option<&[String]>) -> Needs {
    let mut found = Needs::default();
    let mut patterns: Vec<&str> = Vec::new();
    for area in areas {
        let hit: Vec<String> = files
            .iter()
            .filter(|file| area.paths.iter().any(|path| matches(path, file)))
            .cloned()
            .collect();
        if !hit.is_empty() {
            found.touched.push((area.name.clone(), hit));
            if area.there {
                found.there.push(area.name.clone());
            }
            patterns.extend(area.tests.iter().map(String::as_str));
        }
    }
    found.tests = match known {
        Some(names) => names
            .iter()
            .filter(|name| patterns.iter().any(|pattern| matches(pattern, name)))
            .cloned()
            .collect(),
        None => patterns.iter().map(|&pattern| pattern.to_owned()).collect(),
    };
    found.tests.sort();
    found.tests.dedup();
    found
}

/// What a change touching `files` runs first, given `said`, what
/// `changed-checks --name-only` printed for it, when it could compare: the
/// VM tests the areas name whose derivation moved from main, or broke.
/// Without it, every VM test in `known` the areas name runs.
pub fn first(
    areas: &[Area],
    files: &[String],
    said: Option<&str>,
    known: Option<&[String]>,
) -> Needs {
    let moved = said.map(moved);
    needs(areas, files, moved.as_deref().or(known))
}

/// The VM tests `said`, what `changed-checks --name-only` printed, names as
/// moved from main or broken.
pub fn moved(said: &str) -> Vec<String> {
    let changed = Changed::parse(said);
    let mut moved = changed.vm_tests();
    moved.extend(
        changed
            .broken
            .into_iter()
            .filter(|name| name.starts_with("vm-")),
    );
    moved
}

/// A test as `needs` names it: alone, or, when `list` (main's quarantine
/// list) holds it, saying it still runs but its failure blocks nothing.
pub fn named(test: &str, list: &serde_json::Value) -> String {
    let held = list.as_array().and_then(|entries| {
        entries
            .iter()
            .find(|entry| entry["test"].as_str() == Some(test))
    });
    let Some(held) = held else {
        return test.to_owned();
    };
    let reason = held["reason"].as_str().unwrap_or("no reason given");
    let owner = held["owner_pr"]
        .as_u64()
        .map_or_else(String::new, |pr| format!("; #{pr} owns it"));
    format!(
        "{test}  (quarantined on main: {reason}{owner}; it runs, but its failure blocks nothing)"
    )
}

/// Whether `text` matches `pattern`, whose `*` stands for any run of
/// characters, a `/` among them.
pub fn matches(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_areas_built_in_all_read() {
        let areas = areas();
        assert!(areas.iter().any(|area| area.name == "flake.lock"));
        assert!(areas.iter().all(|area| !area.tests.is_empty()));
    }

    #[test]
    fn a_star_stands_for_any_run_a_slash_among_them() {
        assert!(matches("modules/kernel/*", "modules/kernel/nixos.nix"));
        assert!(matches("hardware/*", "hardware/a/kernel/local.nix"));
        assert!(matches("flake.lock", "flake.lock"));
        assert!(!matches("flake.lock", "flake.nix"));
        assert!(matches("vm-boot-*", "vm-boot-asks-in-the-theme"));
        assert!(!matches("vm-boot-*", "vm-reboot-cannot-hang"));
        assert!(matches("tests/plymouth*", "tests/plymouth-vm-test.py"));
        assert!(matches("a*b*c", "a-b-c"));
        assert!(!matches("a*b*c", "a-c-b"));
        assert!(!matches("ab*ba", "aba"));
    }

    #[test]
    fn a_line_without_its_tests_is_refused() {
        assert!(parse("boot: modules/plymouth/*").is_err());
        assert!(parse("boot: => vm-boot-*").is_err());
        assert!(parse("# a comment\n\nboot: a/* => vm-a\n").is_ok());
    }

    #[test]
    fn a_change_to_the_kernel_names_the_boot_tests_known() {
        let areas = parse("kernel: modules/kernel/* => vm-boot-* vm-reboot-cannot-hang").unwrap();
        let known = words("vm-boot-asks-in-the-theme vm-desktop-looks vm-reboot-cannot-hang");
        let needs = needs(
            &areas,
            &words("modules/kernel/nixos.nix modules/kitty/home.nix"),
            Some(&known),
        );
        assert_eq!(
            needs.touched,
            vec![("kernel".to_owned(), words("modules/kernel/nixos.nix"))]
        );
        assert_eq!(
            needs.tests,
            words("vm-boot-asks-in-the-theme vm-reboot-cannot-hang")
        );
    }

    #[test]
    fn only_the_tests_that_moved_from_main_run_first() {
        let areas = parse("kernel: modules/kernel/* => vm-boot-* vm-reboot-cannot-hang").unwrap();
        let known = words("vm-boot-a vm-boot-b vm-boot-c vm-reboot-cannot-hang");
        let files = words("modules/kernel/nixos.nix");
        let said = "changed  vm-boot-a\nchanged  formatting\nbroken   vm-boot-c\n";
        let found = first(&areas, &files, Some(said), Some(&known));
        assert_eq!(found.tests, words("vm-boot-a vm-boot-c"));
        let found = first(&areas, &files, Some("changed  formatting\n"), Some(&known));
        assert_eq!(found.touched.len(), 1, "touched, all the same");
        assert!(found.tests.is_empty(), "none moved, so none runs");
        let found = first(&areas, &files, None, Some(&known));
        assert_eq!(found.tests, known, "with nothing compared, all run");
    }

    #[test]
    fn a_change_to_what_runs_vm_tests_sends_one_through_it() {
        let offload = areas()
            .into_iter()
            .find(|area| area.name == "nix-offload")
            .unwrap();
        assert!(offload.there);
        assert_eq!(offload.tests, words("vm-*"));
        let needs = needs(
            &[offload],
            &words("modules/nix-offload/offload/src/main.rs"),
            Some(&[]),
        );
        assert_eq!(needs.tests, [] as [std::string::String; 0]);
        assert_eq!(needs.there, words("nix-offload"));
        assert!(parse("a: a/* => there").is_ok());
    }

    #[test]
    fn a_change_elsewhere_needs_nothing_first() {
        let needs = needs(&areas(), &words("modules/kitty/home.nix"), None);
        assert_eq!(needs, Needs::default());
    }

    #[test]
    fn a_test_main_quarantines_is_named_as_blocking_nothing() {
        let list = serde_json::json!([
            { "test": "vm-boot-a", "reason": "failed then passed at abc", "owner_pr": 7 },
        ]);
        assert_eq!(
            named("vm-boot-a", &list),
            "vm-boot-a  (quarantined on main: failed then passed at abc; #7 owns it; \
             it runs, but its failure blocks nothing)"
        );
        assert_eq!(named("vm-boot-b", &list), "vm-boot-b");
        assert_eq!(named("vm-boot-a", &serde_json::json!(null)), "vm-boot-a");
    }

    #[test]
    fn with_no_names_known_the_patterns_stand() {
        let needs = needs(&areas(), &words("flake.lock"), None);
        assert_eq!(needs.tests, words("vm-*"));
    }
}
