//! Which changes run their VM tests before they merge: `must-run.txt`
//! names each area, the paths that are it and the tests it runs.

/// The areas, as `must-run.txt` holds them, built into the program so it
/// answers the same on any branch it was built from.
const MUST_RUN: &str = include_str!("../must-run.txt");

/// An area whose changes run their VM tests before merging.
#[derive(Debug, PartialEq, Eq)]
pub struct Area {
    pub name: String,
    pub paths: Vec<String>,
    pub tests: Vec<String>,
}

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
            let area = Area {
                name: name.trim().to_owned(),
                paths: words(paths),
                tests: words(tests),
            };
            if area.paths.is_empty() || area.tests.is_empty() {
                return Err(format!("an area needs paths and tests: {line}"));
            }
            Ok(area)
        })
        .collect()
}

/// What a change has to run first: each area it touches, with the files
/// that touch it, and the VM tests among `known` the areas name, sorted.
/// With no names known, the tests are the areas' patterns.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Needs {
    pub touched: Vec<(String, Vec<String>)>,
    pub tests: Vec<String>,
}

/// Say which areas `files` touch, and which of `known`'s VM tests they run.
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
    fn a_change_elsewhere_needs_nothing_first() {
        let needs = needs(&areas(), &words("modules/kitty/home.nix"), None);
        assert_eq!(needs, Needs::default());
    }

    #[test]
    fn with_no_names_known_the_patterns_stand() {
        let needs = needs(&areas(), &words("flake.lock"), None);
        assert_eq!(needs.tests, words("vm-*"));
    }
}
