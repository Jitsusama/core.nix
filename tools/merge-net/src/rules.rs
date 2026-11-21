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

/// The list of flake inputs whose lock may move alone with no VM test run
/// first. It is read from a change's merge base, never built in as the
/// areas are, so a change editing it beside the lock can't let itself
/// through.
pub const LOCK_AFTER: &str = "tools/merge-net/lock-after.txt";

/// The lock file, whose area this list lets a change leave untouched.
pub const LOCK: &str = "flake.lock";

/// The inputs a `lock-after.txt` of `text` names, one a line, skipping
/// comments and blank lines.
pub fn lock_after(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// The nodes of a lock file that differ between `base` and `head`, those
/// one holds and the other doesn't among them, read from each file's
/// `nodes`, so a change of its layout alone moves none. `None` when either
/// doesn't read as a lock.
pub fn lock_moved(base: &str, head: &str) -> Option<Vec<String>> {
    let (base, head) = (nodes(base)?, nodes(head)?);
    let mut moved: Vec<String> = base
        .keys()
        .chain(head.keys())
        .filter(|name| base.get(*name) != head.get(*name))
        .cloned()
        .collect();
    moved.sort();
    moved.dedup();
    Some(moved)
}

/// A lock's `nodes`, or `None` when it doesn't read as one.
fn nodes(text: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("nodes")?
        .as_object()
        .cloned()
}

/// The nodes of the lock of `text` that the root's inputs `after` name, as
/// the root's own `inputs` resolves each, so a node keyed apart from its
/// input's name is still found, and an input that follows another names
/// none. `None` when it doesn't read, or when any node other than the root
/// and those reads one of them, by its key or by a follow through its
/// input, since moving it would move what that node builds.
fn let_through(text: &str, after: &[String]) -> Option<Vec<String>> {
    let nodes = nodes(text)?;
    let lock: serde_json::Value = serde_json::from_str(text).ok()?;
    let root = lock.get("root")?.as_str()?;
    let inputs = nodes.get(root)?.get("inputs")?.as_object()?;
    let keys: Vec<String> = after
        .iter()
        .filter_map(|name| inputs.get(name)?.as_str().map(str::to_owned))
        .collect();
    let reads_one = |input: &serde_json::Value| match input {
        serde_json::Value::String(key) => keys.contains(key),
        serde_json::Value::Array(path) => path
            .first()
            .and_then(serde_json::Value::as_str)
            .is_some_and(|first| after.iter().any(|name| name == first)),
        _ => true,
    };
    let read = nodes
        .iter()
        .filter(|(key, _)| *key != root && !keys.contains(key))
        .filter_map(|(_, node)| node.get("inputs")?.as_object())
        .any(|inputs| inputs.values().any(reads_one));
    (!read).then_some(keys)
}

/// Whether a lock moving from `base` to `head` moves only the nodes of
/// inputs `after` names, and at least one, so its VM tests may run after
/// the merge. A lock that doesn't read, moves any other node, the root's
/// own list of inputs among them, or has another node reading one of
/// those, on either side, runs them first.
pub fn lock_runs_after(base: &str, head: &str, after: &[String]) -> bool {
    let (Some(before), Some(now)) = (let_through(base, after), let_through(head, after)) else {
        return false;
    };
    lock_moved(base, head).is_some_and(|moved| {
        !moved.is_empty()
            && moved
                .iter()
                .all(|name| before.contains(name) && now.contains(name))
    })
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
/// Without it, every VM test in `known` the areas name runs. Each other
/// check that moved touches the areas as `check:` and its name, as the
/// example laptop's initrd does.
pub fn first(
    areas: &[Area],
    files: &[String],
    said: Option<&str>,
    known: Option<&[String]>,
) -> Needs {
    let mut touching = files.to_vec();
    if let Some(said) = said {
        touching.extend(
            Changed::parse(said)
                .moved
                .into_iter()
                .filter(|name| !name.starts_with("vm-"))
                .map(|name| format!("{CHECK}{name}")),
        );
    }
    let moved = said.map(moved);
    needs(areas, &touching, moved.as_deref().or(known))
}

/// How an area's path names a check that moved rather than a file.
const CHECK: &str = "check:";

/// What `changed-checks` is asked to say moved from `base`. Sent to a
/// builder, as `nix-offload there --run` sets `elsewhere`, it compares
/// there, since that machine's nix-offload sends nothing on: on 11 November
/// `merge-net needs` run on a builder couldn't compare, and named all 31
/// VM tests of a change that moved none.
pub fn changed_checks_asks<'a>(base: &'a str, system: &'a str) -> Vec<&'a str> {
    asked_of_changed_checks(
        base,
        system,
        std::env::var_os("WORK_RUNS_ELSEWHERE").is_some(),
    )
}

fn asked_of_changed_checks<'a>(base: &'a str, system: &'a str, elsewhere: bool) -> Vec<&'a str> {
    let mut asked = vec!["--base", base, "--name-only", "--system", system];
    if elsewhere {
        asked.push("--here");
    }
    asked
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
/// list) holds it, saying it still runs but its failure blocks nothing,
/// with each owner's subtests where its entries name them.
pub fn named(test: &str, list: &serde_json::Value) -> String {
    let held: Vec<&serde_json::Value> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["test"].as_str() == Some(test))
        .collect();
    let why = |entry: &serde_json::Value| {
        let reason = entry["reason"].as_str().unwrap_or("no reason given");
        let owner = entry["owner_pr"]
            .as_u64()
            .map_or_else(String::new, |pr| format!("{pr} owns it"));
        (reason.to_owned(), owner)
    };
    let subtests = |entry: &serde_json::Value| -> Vec<String> {
        entry["subtests"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|subtest| subtest.as_str().map(str::to_owned))
            .collect()
    };
    if held.is_empty() {
        return test.to_owned();
    }
    if let Some(whole) = held.iter().find(|entry| subtests(entry).is_empty()) {
        let (reason, owner) = why(whole);
        let owner = if owner.is_empty() {
            owner
        } else {
            format!("; #{owner}")
        };
        return format!(
            "{test}  (quarantined on main: {reason}{owner}; it runs, but its failure blocks \
             nothing)"
        );
    }
    let each: Vec<String> = held
        .iter()
        .map(|entry| {
            let (reason, owner) = why(entry);
            let owner = if owner.is_empty() {
                owner
            } else {
                format!(", #{owner}")
            };
            format!("{}, {reason}{owner}", subtests(entry).join(", "))
        })
        .collect();
    format!(
        "{test}  (quarantined on main: {}; it runs, but a failure in those alone blocks nothing)",
        each.join("; ")
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

    /// A lock holding a root and the nodes named, each at `rev` beside it.
    fn lock(nodes: &[(&str, &str)]) -> String {
        let mut all = serde_json::Map::new();
        let inputs: serde_json::Map<String, serde_json::Value> = nodes
            .iter()
            .map(|(name, _)| ((*name).to_owned(), serde_json::json!(name)))
            .collect();
        all.insert("root".to_owned(), serde_json::json!({ "inputs": inputs }));
        for (name, rev) in nodes {
            all.insert(
                (*name).to_owned(),
                serde_json::json!({ "locked": { "rev": rev, "narHash": format!("sha256-{rev}") } }),
            );
        }
        serde_json::json!({ "nodes": all, "root": "root", "version": 7 }).to_string()
    }

    #[test]
    fn a_lock_moving_only_mycelium_runs_its_vm_tests_after() {
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a"), ("nixpkgs", "n")]);
        let head = lock(&[("mycelium", "b"), ("nixpkgs", "n")]);
        assert_eq!(lock_moved(&base, &head), Some(words("mycelium")));
        assert!(
            lock_runs_after(&base, &head, &after),
            "a lock moving only mycelium should run its VM tests after"
        );
    }

    #[test]
    fn a_lock_moving_mycelium_and_nixpkgs_runs_its_vm_tests_first() {
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a"), ("nixpkgs", "n")]);
        let head = lock(&[("mycelium", "b"), ("nixpkgs", "m")]);
        assert_eq!(lock_moved(&base, &head), Some(words("mycelium nixpkgs")));
        assert!(
            !lock_runs_after(&base, &head, &after),
            "a lock moving nixpkgs too should run its VM tests first"
        );
    }

    #[test]
    fn a_lock_gaining_an_input_runs_its_vm_tests_first() {
        // The new node and the root's list of inputs both move, and
        // neither is one that may move alone.
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a")]);
        let head = lock(&[("mycelium", "b"), ("extra", "e")]);
        assert_eq!(lock_moved(&base, &head), Some(words("extra mycelium root")));
        assert!(
            !lock_runs_after(&base, &head, &after),
            "a lock gaining an input should run its VM tests first"
        );
    }

    #[test]
    fn a_lock_that_does_not_read_runs_its_vm_tests_first() {
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a")]);
        assert_eq!(lock_moved(&base, "{ not a lock"), None);
        assert!(
            !lock_runs_after(&base, "{ not a lock", &after),
            "a lock that doesn't parse should run its VM tests first"
        );
        assert!(
            !lock_runs_after(&base, "{\"version\": 7}", &after),
            "a lock with no nodes should run its VM tests first"
        );
    }

    #[test]
    fn a_lock_whose_nodes_stay_runs_its_vm_tests_first() {
        // Only its layout moved, so nothing says the change is one of the
        // inputs let through.
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a")]);
        let spaced = serde_json::to_string_pretty(
            &serde_json::from_str::<serde_json::Value>(&base).unwrap(),
        )
        .unwrap();
        assert_eq!(lock_moved(&base, &spaced), Some(Vec::new()));
        assert!(
            !lock_runs_after(&base, &spaced, &after),
            "a lock whose layout alone moved should run its VM tests first"
        );
    }

    #[test]
    fn the_inputs_let_through_are_mycelium() {
        assert_eq!(
            lock_after(include_str!("../lock-after.txt")),
            words("mycelium")
        );
    }

    /// A lock whose root names `inputs`, holding `nodes` as given.
    fn shaped(inputs: &serde_json::Value, nodes: &[(&str, serde_json::Value)]) -> String {
        let mut all = serde_json::Map::new();
        all.insert("root".to_owned(), serde_json::json!({ "inputs": inputs }));
        for (key, node) in nodes {
            all.insert((*key).to_owned(), node.clone());
        }
        serde_json::json!({ "nodes": all, "root": "root", "version": 7 }).to_string()
    }

    fn at(rev: &str) -> serde_json::Value {
        serde_json::json!({ "locked": { "rev": rev } })
    }

    #[test]
    fn an_input_is_found_by_the_node_its_root_names() {
        let after = words("mycelium");
        let inputs = serde_json::json!({ "mycelium": "mycelium_2", "nixpkgs": "nixpkgs" });
        let base = shaped(&inputs, &[("mycelium_2", at("a")), ("nixpkgs", at("n"))]);
        let head = shaped(&inputs, &[("mycelium_2", at("b")), ("nixpkgs", at("n"))]);
        assert!(
            lock_runs_after(&base, &head, &after),
            "a node keyed apart from its input's name should still be let through"
        );
        let keyed = serde_json::json!({ "mycelium": "nixpkgs", "nixpkgs": "mycelium" });
        let base = shaped(&keyed, &[("mycelium", at("a")), ("nixpkgs", at("n"))]);
        let head = shaped(&keyed, &[("mycelium", at("b")), ("nixpkgs", at("n"))]);
        assert!(
            !lock_runs_after(&base, &head, &after),
            "a node keyed mycelium that the root reads as another input should run first"
        );
    }

    #[test]
    fn a_node_reading_a_let_through_input_runs_its_vm_tests_first() {
        let after = words("mycelium");
        let inputs = serde_json::json!({ "mycelium": "mycelium", "tool": "tool" });
        let tool = |reads: serde_json::Value| serde_json::json!({ "locked": { "rev": "t" }, "inputs": { "m": reads } });
        for (reads, how) in [
            (serde_json::json!("mycelium"), "by its key"),
            (serde_json::json!(["mycelium", "nixpkgs"]), "by a follow"),
        ] {
            let base = shaped(
                &inputs,
                &[("mycelium", at("a")), ("tool", tool(reads.clone()))],
            );
            let head = shaped(&inputs, &[("mycelium", at("b")), ("tool", tool(reads))]);
            assert!(
                !lock_runs_after(&base, &head, &after),
                "another node reading mycelium {how} should run its VM tests first"
            );
        }
    }

    #[test]
    fn a_let_through_node_following_another_input_is_let_through() {
        // mycelium's own nixpkgs follows the root's, as core's lock has it.
        let after = words("mycelium");
        let inputs = serde_json::json!({ "mycelium": "mycelium", "nixpkgs": "nixpkgs" });
        let mycelium = |rev: &str| serde_json::json!({ "locked": { "rev": rev }, "inputs": { "nixpkgs": ["nixpkgs"] } });
        let base = shaped(
            &inputs,
            &[("mycelium", mycelium("a")), ("nixpkgs", at("n"))],
        );
        let head = shaped(
            &inputs,
            &[("mycelium", mycelium("b")), ("nixpkgs", at("n"))],
        );
        assert!(
            lock_runs_after(&base, &head, &after),
            "mycelium following the root's nixpkgs should still be let through"
        );
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
    fn changed_checks_compares_where_needs_was_sent() {
        let asked = asked_of_changed_checks("abc", "x86_64-linux", true);
        assert!(
            asked.contains(&"--here"),
            "sent to a builder, changed-checks wasn't asked to compare there: {asked:?}"
        );
        let asked = asked_of_changed_checks("abc", "x86_64-linux", false);
        assert_eq!(
            asked,
            words("--base abc --name-only --system x86_64-linux"),
            "on the machine, changed-checks was asked to compare here rather than send it"
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
    fn a_change_that_moves_a_machines_initrd_names_the_tests_that_boot_it() {
        // #4513, at a6b6ca22: the kit moved, which hibernate-unlock reads in
        // the initrd, and needs named only the tests owning its daemons.
        let files = words(
            "modules/terminal-kit/src/lib.rs modules/night-shift/src/main.rs \
             modules/network-revive/src/main.rs modules/v4l2loopback/holders/src/main.rs",
        );
        let said = "changed  example-laptop\nchanged  example-laptop-initrd\n\
                    changed  vm-hibernation-comes-back\n\
                    changed  vm-boot-says-it-wakes-from-hibernation\n\
                    changed  vm-loopback-says-who-holds-it\nchanged  vm-desktop-looks\n";
        let found = first(&areas(), &files, Some(said), None);
        assert_eq!(
            found.tests,
            words("vm-boot-says-it-wakes-from-hibernation vm-hibernation-comes-back"),
            "a change that moved the example laptop's initrd didn't name the tests that boot it"
        );
        let said = "changed  example-laptop\nchanged  vm-hibernation-comes-back\n";
        let found = first(&areas(), &files, Some(said), None);
        assert!(
            found.tests.is_empty(),
            "a change that left the initrd as it was named boot tests: {:?}",
            found.tests
        );
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
    fn a_test_two_owners_hold_names_each_with_its_subtests() {
        let list = serde_json::json!([
            { "test": "vm-boot-a", "reason": "r1", "owner_pr": 7, "subtests": ["test_p"] },
            { "test": "vm-boot-a", "reason": "r2", "owner_pr": 8, "subtests": ["test_q"] },
        ]);
        assert_eq!(
            named("vm-boot-a", &list),
            "vm-boot-a  (quarantined on main: test_p, r1, #7 owns it; test_q, r2, #8 owns it; \
             it runs, but a failure in those alone blocks nothing)",
            "the second owner's entry wasn't named"
        );
    }

    #[test]
    fn with_no_names_known_the_patterns_stand() {
        let needs = needs(&areas(), &words("flake.lock"), None);
        assert_eq!(needs.tests, words("vm-*"));
    }
}
