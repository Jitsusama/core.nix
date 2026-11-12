//! Whether two changes to one JSON file, to check-descriptions.nix, to a
//! folder's checks.nix, to a list of modules or to a pytest file
//! (`apart/cases.rs`) touch different keys, lines or cases, so the gate
//! can count them as two files. It changes with how such files are
//! compared, not with what the gate asks of a branch.

mod cases;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

pub use cases::{cases_apart, is_cases};

/// Whether a JSON file both sides changed was changed apart: what the
/// branch changed from `base` and what main changed since touch no key in
/// common, and the rebased `head` differs from each side only where the
/// other changed it. Two such changes stand as two files would, which the
/// gate already lets merge with no new run, the net catching how they
/// meet. On 9 November #2571 was refused four times in two hours, each
/// time for main moving words.json, a 300 KB file of the launcher's words
/// that a quarter of main's merges change, during its own run; each of the
/// 23 merges to it that day changed different words from the one before.
/// A file any side leaves unreadable isn't apart.
#[must_use]
pub fn apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    apart_read(
        &|text| serde_json::from_str::<Value>(text).ok(),
        base,
        branch,
        main,
        head,
    )
}

/// Whether check-descriptions.nix, which both sides changed, was changed
/// `apart`, read as keyed by each check's name. Its check holds only that
/// the names it describes are the names checked, in any order, so changes
/// at different names meet as two files would. On 10 November #2763 was
/// rebased six times in half an hour, each time for main adding another
/// check's description during the build of the one list check it needed.
/// A file read otherwise than as entries isn't apart.
#[must_use]
pub fn descriptions_apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    apart_read(&descriptions, base, branch, main, head)
}

/// check-descriptions.nix as an object of each entry's lines by its
/// check's name, with every line outside an entry under a key no check
/// can have. An entry opens at `  <name> = ''` and closes at `  '';`. One
/// left open, or a name given twice, reads as nothing.
fn descriptions(text: &str) -> Option<Value> {
    let mut entries = serde_json::Map::new();
    let mut outside = String::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        if let Some((name, said)) = &mut open {
            if line == "  '';" {
                let said = Value::String(std::mem::take(said));
                if entries.insert(std::mem::take(name), said).is_some() {
                    return None;
                }
                open = None;
            } else {
                said.push_str(line);
                said.push('\n');
            }
        } else if let Some(name) = entry(line) {
            open = Some((name.to_owned(), String::new()));
        } else {
            outside.push_str(line);
            outside.push('\n');
        }
    }
    if open.is_some() || entries.contains_key(OUTSIDE) {
        return None;
    }
    entries.insert(OUTSIDE.to_owned(), Value::String(outside));
    Some(Value::Object(entries))
}

/// The key the lines outside every entry are kept under, which a check's
/// name, of letters, digits and dashes, can't be.
const OUTSIDE: &str = " outside";

/// The check's name, when `line` opens an entry.
fn entry(line: &str) -> Option<&str> {
    let name = line.strip_prefix("  ")?.strip_suffix(" = ''")?;
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
    .then_some(name)
}

/// Whether a folder's checks.nix, which both sides changed, was changed
/// `apart`, read as keyed by each check it gives. A check is built from
/// its own entry and what the file keeps outside the entries, its
/// arguments and `let` helpers, so changes to different checks' entries
/// meet as two files would, while either side changing what's outside
/// them may reach every check, and isn't apart. On 10 November #2807
/// added a check to modules/quickshell/checks.nix while #2822 changed
/// another there, and #2807 ran changed-checks a second time for it,
/// as #2794 did for the same file half an hour later. A check reading
/// the whole set of checks, as nix-offload's reads their names, is left
/// for the net to catch, as two files meeting would be.
#[must_use]
pub fn checks_apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    let outside = |text: &str| checks(text).and_then(|entries| entries.get(OUTSIDE).cloned());
    let kept = outside(base);
    kept.is_some()
        && outside(branch) == kept
        && outside(main) == kept
        && apart_read(&checks, base, branch, main, head)
}

/// A checks.nix as an object of each check's lines by its name, with
/// every other line under a key no check can have. Checks are the
/// bindings at two spaces' indent after the file's `in`, or after its
/// arguments where it has no `let`. Each takes the blank lines and
/// comment just above it, so adding a check after another leaves that
/// one as it was, and every line after it up to the next check or the
/// next line at a shallower indent. A file of neither shape, or a check
/// given twice, reads as nothing.
fn checks(text: &str) -> Option<Value> {
    let lines: Vec<&str> = text.lines().collect();
    let opens = match lines.iter().position(|line| *line == "in") {
        Some(at) => at + 1,
        None if !lines.contains(&"let") => lines.iter().position(|line| *line == "}:")? + 1,
        None => return None,
    };
    let mut entries = serde_json::Map::new();
    let mut outside: String = lines[..opens]
        .iter()
        .flat_map(|line| [*line, "\n"])
        .collect();
    let mut open: Option<(String, String)> = None;
    // The blank lines and comment read since the last line kept, which
    // go with whatever comes after them.
    let mut above = String::new();
    for line in &lines[opens..] {
        let into = if let Some(name) = check(line) {
            if let Some((name, said)) = open.replace((name.to_owned(), String::new())) {
                entries
                    .insert(name, Value::String(said))
                    .is_none()
                    .then_some(())?;
            }
            open.as_mut().map(|(_, said)| said)
        } else if line.is_empty() || line.starts_with("  #") {
            above.push_str(line);
            above.push('\n');
            continue;
        } else if line.starts_with("  ") {
            open.as_mut().map(|(_, said)| said)
        } else {
            if let Some((name, said)) = open.take() {
                entries
                    .insert(name, Value::String(said))
                    .is_none()
                    .then_some(())?;
            }
            None
        };
        let into = into.unwrap_or(&mut outside);
        into.push_str(&std::mem::take(&mut above));
        into.push_str(line);
        into.push('\n');
    }
    if let Some((name, said)) = open {
        entries
            .insert(name, Value::String(said))
            .is_none()
            .then_some(())?;
    }
    outside.push_str(&above);
    entries.insert(OUTSIDE.to_owned(), Value::String(outside));
    Some(Value::Object(entries))
}

/// The check's name, when `line` opens one in a checks.nix.
fn check(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("  ")?;
    let (name, _) = rest.split_once(" =")?;
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'\'')))
    .then_some(name)
}

/// The lists a new module adds a line to: the flake's, the checks', the
/// example laptop's and the shell's checks'. On 12 November, by 07:00,
/// merge-net had held gates back 23 times on them, 8 on flake.nix alone,
/// and asked for 5 runs again over the shell's, most for two changes each
/// adding a line of its own.
pub const LISTS: [&str; 4] = [
    "flake.nix",
    "checks.nix",
    "examples/laptop.nix",
    "modules/quickshell/checks/imports.nix",
];

/// Whether one of the `LISTS`, which both sides changed, was changed
/// apart: each side only added and took away whole lines, never one the
/// other took away nor beside it, and the rebased `head` is `base` with
/// both sides' lines, those added at one place in either side's order. A
/// line both sides changed, or one changed in place, stays shared.
#[must_use]
pub fn lines_apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    let base: Vec<&str> = base.lines().collect();
    let head: Vec<&str> = head.lines().collect();
    let (Some(ours), Some(theirs)) = (edits(&base, branch), edits(&base, main)) else {
        return false;
    };
    meet_apart(&ours, &theirs)
        && [(&ours, &theirs), (&theirs, &ours)]
            .iter()
            .any(|(first, then)| merged(&base, first, then) == head)
}

/// Whether two changes from `base`, `one` and `other`, would meet as
/// `lines_apart` asks once either is rebased onto the other: what a gate
/// holding a change back on one of the `LISTS` is asked, before the
/// rebase there is to judge.
#[must_use]
pub fn would_be_apart(base: &str, one: &str, other: &str) -> bool {
    let base: Vec<&str> = base.lines().collect();
    match (edits(&base, one), edits(&base, other)) {
        (Some(one), Some(other)) => meet_apart(&one, &other),
        _ => false,
    }
}

/// Whether two changes from `base` to `file`, `one` and `other`, would
/// meet apart once either is rebased onto the other, for the files a gate
/// can judge so before the rebase: one of the `LISTS`, or a pytest file.
#[must_use]
pub fn would_stand_apart(file: &str, base: &str, one: &str, other: &str) -> bool {
    if LISTS.contains(&file) {
        would_be_apart(base, one, other)
    } else if is_cases(file) {
        cases::would_cases_be_apart(base, one, other)
    } else {
        false
    }
}

/// What a side did to its base's lines: those it added, by the index of
/// the base's line they come before, and the indexes of those it took
/// away.
#[derive(Default)]
struct Edits<'a> {
    added: BTreeMap<usize, Vec<&'a str>>,
    removed: BTreeSet<usize>,
}

/// The `Edits` taking `base` to `after`, by the longest run of lines they
/// keep in common, or none where a side changed a line in place, taking
/// lines away and adding others where they were.
fn edits<'a>(base: &[&str], after: &'a str) -> Option<Edits<'a>> {
    let after: Vec<&str> = after.lines().collect();
    // The lines both open and close with need no table, which leaves it
    // a few lines a side where a change lists a module.
    let start = base.iter().zip(&after).take_while(|(a, b)| a == b).count();
    let end = base[start..]
        .iter()
        .rev()
        .zip(after[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (old, new) = (
        &base[start..base.len() - end],
        &after[start..after.len() - end],
    );
    // How many lines `old[i..]` and `new[j..]` keep in common, at most.
    let mut common = vec![vec![0_u32; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            common[i][j] = if old[i] == new[j] {
                common[i + 1][j + 1] + 1
            } else {
                common[i + 1][j].max(common[i][j + 1])
            };
        }
    }
    let mut edits = Edits::default();
    let (mut i, mut j) = (0, 0);
    // Whether the lines since the last one kept took any away, or added any.
    let (mut took, mut gave) = (false, false);
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            (took, gave) = (false, false);
            (i, j) = (i + 1, j + 1);
        } else if j < new.len() && (i == old.len() || common[i][j + 1] >= common[i + 1][j]) {
            edits.added.entry(start + i).or_default().push(new[j]);
            gave = true;
            j += 1;
        } else {
            edits.removed.insert(start + i);
            took = true;
            i += 1;
        }
        if took && gave {
            return None;
        }
    }
    Some(edits)
}

/// Whether two sides' `Edits` touch no line in common: neither takes away
/// a line the other does, nor adds lines beside one the other takes away.
fn meet_apart(one: &Edits, other: &Edits) -> bool {
    let clear = |adding: &Edits, taking: &Edits| {
        adding.added.keys().all(|at| {
            !taking.removed.contains(at)
                && !at
                    .checked_sub(1)
                    .is_some_and(|before| taking.removed.contains(&before))
        })
    };
    one.removed.is_disjoint(&other.removed) && clear(one, other) && clear(other, one)
}

/// `base` with the lines `first` and then `then` add at each place, and
/// without those either takes away.
fn merged<'a>(base: &[&'a str], first: &Edits<'a>, then: &Edits<'a>) -> Vec<&'a str> {
    let mut lines = Vec::new();
    let add = |at: usize, lines: &mut Vec<&'a str>| {
        for side in [first, then] {
            lines.extend(side.added.get(&at).into_iter().flatten().copied());
        }
    };
    for (at, line) in base.iter().enumerate() {
        add(at, &mut lines);
        if !first.removed.contains(&at) && !then.removed.contains(&at) {
            lines.push(*line);
        }
    }
    add(base.len(), &mut lines);
    lines
}

/// Whether the four texts, each read by `read`, were changed apart.
fn apart_read(
    read: &dyn Fn(&str) -> Option<Value>,
    base: &str,
    branch: &str,
    main: &str,
    head: &str,
) -> bool {
    let (Some(base), Some(branch), Some(main), Some(head)) =
        (read(base), read(branch), read(main), read(head))
    else {
        return false;
    };
    let ours = changed(&base, &branch);
    let theirs = changed(&base, &main);
    let within = |paths: &[Vec<String>], of: &[Vec<String>]| {
        paths
            .iter()
            .all(|path| of.iter().any(|whole| path.starts_with(whole)))
    };
    !ours
        .iter()
        .any(|a| theirs.iter().any(|b| a.starts_with(b) || b.starts_with(a)))
        && within(&changed(&main, &head), &ours)
        && within(&changed(&branch, &head), &theirs)
}

/// The paths of the keys at which `after` differs from `before`. An array
/// whose items are objects each with its own `id` is read as keyed by it,
/// since adding one moves every index after it, and its order counts as
/// one change to the whole; any other array is read by index, and one
/// that changes length is changed whole.
fn changed(before: &Value, after: &Value) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    differ(before, after, &mut Vec::new(), &mut paths);
    paths
}

/// The paths under one key, one side or both having it.
fn under(
    key: String,
    before: Option<&Value>,
    after: Option<&Value>,
    at: &mut Vec<String>,
    paths: &mut Vec<Vec<String>>,
) {
    at.push(key);
    match (before, after) {
        (Some(before), Some(after)) => differ(before, after, at, paths),
        _ => paths.push(at.clone()),
    }
    at.pop();
}

fn differ(before: &Value, after: &Value, at: &mut Vec<String>, paths: &mut Vec<Vec<String>>) {
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys().filter(|key| !a.contains_key(*key))) {
                under(format!(".{key}"), a.get(key), b.get(key), at, paths);
            }
        }
        (Value::Array(a), Value::Array(b)) => match (by_id(a), by_id(b)) {
            (Some(a), Some(b)) => {
                let kept = |from: &[(&str, &Value)], to: &[(&str, &Value)]| -> Vec<String> {
                    from.iter()
                        .filter(|(id, _)| to.iter().any(|(other, _)| other == id))
                        .map(|(id, _)| (*id).to_owned())
                        .collect()
                };
                if kept(&a, &b) != kept(&b, &a) {
                    paths.push(at.clone());
                    return;
                }
                let mut ids: Vec<&str> = a.iter().map(|(id, _)| *id).collect();
                ids.extend(
                    b.iter()
                        .map(|(id, _)| *id)
                        .filter(|id| find(&a, id).is_none()),
                );
                for id in ids {
                    under(format!("[id={id}]"), find(&a, id), find(&b, id), at, paths);
                }
            }
            _ if a.len() == b.len() => {
                for (index, (a, b)) in a.iter().zip(b).enumerate() {
                    under(format!("[{index}]"), Some(a), Some(b), at, paths);
                }
            }
            _ => paths.push(at.clone()),
        },
        _ if before != after => paths.push(at.clone()),
        _ => {}
    }
}

/// The item with `id` among `items`.
fn find<'a>(items: &[(&str, &'a Value)], id: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|(other, _)| *other == id)
        .map(|(_, item)| *item)
}

/// An array's items by their `id`, when each is an object with its own.
fn by_id(items: &[Value]) -> Option<Vec<(&str, &Value)>> {
    let keyed: Vec<(&str, &Value)> = items
        .iter()
        .map(|item| item["id"].as_str().map(|id| (id, item)))
        .collect::<Option<_>>()?;
    let mut ids: Vec<&str> = keyed.iter().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    ids.dedup();
    (!keyed.is_empty() && ids.len() == keyed.len()).then_some(keyed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORDS: &str = r#"{"home": ["on"], "things": [
        {"id": "a", "said": {"x": "1"}}, {"id": "b", "said": {"y": "2"}}]}"#;

    #[test]
    fn a_json_file_changed_at_different_keys_is_apart() {
        // #2571's case: the branch changed one thing's words, and main added
        // another thing, which moves every index after it.
        let branch = WORDS.replace("\"1\"", "\"one\"");
        let main = WORDS.replace("[\n", "[\n{\"id\": \"new\"}, ");
        let head = main.replace("\"1\"", "\"one\"");
        assert!(apart(WORDS, &branch, &main, &head), "main added a thing");
        let main = WORDS.replace("[\"on\"]", "[\"on\", \"off\"]");
        let head = main.replace("\"1\"", "\"one\"");
        assert!(
            apart(WORDS, &branch, &main, &head),
            "a list elsewhere moved"
        );
    }

    #[test]
    fn a_json_file_changed_at_one_key_or_merged_oddly_is_not() {
        let branch = WORDS.replace("\"1\"", "\"one\"");
        let main = WORDS.replace("{\"id\": \"a\", \"said\": {\"x\": \"1\"}}, ", "");
        assert!(
            !apart(WORDS, &branch, &main, &main),
            "main took away what it changed"
        );
        let reordered = r#"{"home": ["on"], "things": [
            {"id": "b", "said": {"y": "2"}}, {"id": "a", "said": {"x": "1"}}]}"#;
        let head = reordered.replace("\"1\"", "\"one\"");
        assert!(
            !apart(WORDS, &branch, reordered, &head),
            "main reordered the things"
        );
        let main = WORDS.replace("[\"on\"]", "[\"on\", \"off\"]");
        let head = main.replace("\"1\"", "\"one\"").replace("\"2\"", "\"two\"");
        assert!(
            !apart(WORDS, &branch, &main, &head),
            "the head changed what neither did"
        );
        assert!(
            !apart(WORDS, &branch, "{", &head),
            "main's side doesn't read"
        );
    }

    const DESCRIBED: &str = "# What each check proves.\n{\n  a-check = ''\n    holds\n  '';\n  \
        c-check = ''\n    holds too\n  '';\n}\n";

    /// `DESCRIBED` with `name`'s entry put before `before`'s.
    fn described(text: &str, name: &str, before: &str) -> String {
        text.replace(
            &format!("  {before} = ''"),
            &format!("  {name} = ''\n    proves\n  '';\n  {before} = ''"),
        )
    }

    #[test]
    fn descriptions_added_for_different_checks_are_apart() {
        // #2763's case: the branch described its new check, and main
        // described another meanwhile.
        let branch = described(DESCRIBED, "b-check", "c-check");
        let main = described(DESCRIBED, "0-check", "a-check");
        let head = described(&main, "b-check", "c-check");
        assert!(
            descriptions_apart(DESCRIBED, &branch, &main, &head),
            "two checks described in their places"
        );
        let main = DESCRIBED.replace("holds too", "holds still");
        let head = described(&main, "b-check", "c-check");
        assert!(
            descriptions_apart(DESCRIBED, &branch, &main, &head),
            "another check's words changed"
        );
    }

    #[test]
    fn descriptions_changed_for_one_check_or_read_otherwise_are_not() {
        let branch = DESCRIBED.replace("holds too", "holds more");
        let main = DESCRIBED.replace("holds too", "holds still");
        assert!(
            !descriptions_apart(DESCRIBED, &branch, &main, &main),
            "both changed one check's words"
        );
        let branch = DESCRIBED.replace("# What", "# All");
        let main = DESCRIBED.replace("# What", "# Which");
        assert!(
            !descriptions_apart(DESCRIBED, &branch, &main, &main),
            "both changed what's outside the entries"
        );
        let branch = described(DESCRIBED, "b-check", "c-check");
        let open = DESCRIBED.replace("    holds too\n  '';\n", "");
        assert!(
            !descriptions_apart(DESCRIBED, &branch, &open, &branch),
            "main's side leaves an entry open"
        );
        let twice = described(DESCRIBED, "a-check", "c-check");
        assert!(
            !descriptions_apart(DESCRIBED, &branch, &twice, &branch),
            "main's side describes a check twice"
        );
    }

    /// A folder's checks.nix, as modules/quickshell/checks.nix was on 10
    /// November: its arguments, its helpers, and its checks after `in`.
    const CHECKS: &str = "# The checks of the shell.\n{\n  pkgs,\n  onLinux,\n}:\nlet\n  \
        lines = 10;\nin\n# Only Linux has the shell.\nonLinux {\n  # The launcher holds a \
        program's controls.\n  the-launcher-holds-a-programs-controls =\n    \
        pkgs.runCommandLocal \"controls\" { } ''\n      cp a.js a.js\n\n      \
        touch $out\n    '';\n\n  the-launcher-shows-the-moment-in-now = pkgs.hello;\n}\n";

    /// `CHECKS` with a check of `name`, and its comment, put before `before`.
    fn checked(text: &str, name: &str, before: &str) -> String {
        text.replace(
            &format!("  {before} ="),
            &format!("  # {name} holds.\n  {name} = pkgs.hello;\n\n  {before} ="),
        )
    }

    #[test]
    fn checks_added_or_changed_at_different_checks_are_apart() {
        // #2807's case: the branch added a check, and main had changed
        // another check's lines since.
        let branch = CHECKS.replace(
            "\n}\n",
            "\n\n  # Trials are kept.\n  the-launcher-keeps-its-trials = pkgs.hello;\n}\n",
        );
        let main = CHECKS.replace("cp a.js a.js", "cp a.js a.js\n      cp b.js b.js");
        let head = branch.replace("cp a.js a.js", "cp a.js a.js\n      cp b.js b.js");
        assert!(
            checks_apart(CHECKS, &branch, &main, &head),
            "a check added at the end while another changed"
        );
        let ours = checked(CHECKS, "b-check", "the-launcher-shows-the-moment-in-now");
        let theirs = checked(CHECKS, "a-check", "the-launcher-shows-the-moment-in-now");
        let both = checked(&ours, "a-check", "the-launcher-shows-the-moment-in-now");
        assert!(
            checks_apart(CHECKS, &ours, &theirs, &both),
            "two checks added side by side"
        );
        let open = CHECKS.replace("let\n  lines = 10;\nin\n", "");
        let ours = checked(&open, "b-check", "the-launcher-shows-the-moment-in-now");
        let theirs = open.replace("pkgs.hello", "pkgs.cowsay");
        let both = ours.replace("moment-in-now = pkgs.hello", "moment-in-now = pkgs.cowsay");
        assert!(
            checks_apart(&open, &ours, &theirs, &both),
            "a file with no helpers, its checks after its arguments"
        );
    }

    #[test]
    fn checks_changed_at_one_check_or_their_helpers_are_not() {
        let ours = CHECKS.replace("cp a.js a.js", "cp a.js b.js");
        let theirs = CHECKS.replace("touch $out", "touch $out $out");
        let head = ours.replace("touch $out", "touch $out $out");
        assert!(
            !checks_apart(CHECKS, &ours, &theirs, &head),
            "both changed one check's lines"
        );
        let ours = checked(CHECKS, "b-check", "the-launcher-shows-the-moment-in-now");
        let theirs = CHECKS.replace("lines = 10", "lines = 20");
        let head = ours.replace("lines = 10", "lines = 20");
        assert!(
            !checks_apart(CHECKS, &ours, &theirs, &head),
            "main changed a helper every check may read"
        );
        assert!(
            !checks_apart(CHECKS, &theirs, &ours, &head),
            "the branch changed a helper every check may read"
        );
        let twice = checked(
            CHECKS,
            "the-launcher-shows-the-moment-in-now",
            "the-launcher-holds-a-programs-controls",
        );
        assert!(
            !checks_apart(CHECKS, &ours, &twice, &ours),
            "main's side gives a check twice"
        );
        let helped = CHECKS.replace("in\n", "");
        assert!(
            !checks_apart(&helped, &helped, &helped, &helped),
            "a file with helpers and no in"
        );
    }

    /// Lists of modules, as flake.nix keeps them.
    const LISTED: &str = "{\n  home = [\n    ./modules/a/home\n    ./modules/c/home\n  ];\n  \
        system = [\n    ./modules/a\n  ];\n}\n";

    /// `text` with `line` listed after `after`.
    fn added(text: &str, after: &str, line: &str) -> String {
        text.replacen(
            &format!("    {after}\n"),
            &format!("    {after}\n    {line}\n"),
            1,
        )
    }

    /// `text` without `line`.
    fn taken(text: &str, line: &str) -> String {
        text.replacen(&format!("    {line}\n"), "", 1)
    }

    #[test]
    fn lines_added_or_taken_away_at_different_places_are_apart() {
        // 12 November's case: two changes each listed a module of its own.
        let ours = added(LISTED, "./modules/a/home", "./modules/b/home");
        let theirs = added(LISTED, "./modules/a", "./modules/s");
        let head = added(&theirs, "./modules/a/home", "./modules/b/home");
        assert!(
            lines_apart(LISTED, &ours, &theirs, &head),
            "two modules listed in two lists"
        );
        assert!(
            would_be_apart(LISTED, &ours, &theirs),
            "two modules listed in two lists, before the rebase"
        );
        let ours = added(LISTED, "./modules/c/home", "./modules/d/home");
        let theirs = added(LISTED, "./modules/c/home", "./modules/e/home");
        let first = added(&theirs, "./modules/c/home", "./modules/d/home");
        let then = added(&ours, "./modules/c/home", "./modules/e/home");
        for head in [&first, &then] {
            assert!(
                lines_apart(LISTED, &ours, &theirs, head),
                "two modules listed at one list's end, in either order: {head}"
            );
        }
        let theirs = taken(LISTED, "./modules/a");
        let head = taken(&ours, "./modules/a");
        assert!(
            lines_apart(LISTED, &ours, &theirs, &head),
            "a module listed while another list lost one"
        );
    }

    #[test]
    fn lines_both_sides_touched_or_changed_in_place_are_not_apart() {
        let ours = added(LISTED, "./modules/c/home", "./modules/d/home");
        assert!(
            !lines_apart(LISTED, &ours, &ours, &ours),
            "both listed one module"
        );
        let theirs = LISTED.replace("./modules/c/home", "./modules/c/home.nix");
        let head = added(&theirs, "./modules/c/home.nix", "./modules/d/home");
        assert!(
            !lines_apart(LISTED, &ours, &theirs, &head),
            "main changed a line in place"
        );
        assert!(
            !would_be_apart(LISTED, &ours, &theirs),
            "main changed a line in place, before the rebase"
        );
        let theirs = taken(LISTED, "./modules/c/home");
        let head = taken(&ours, "./modules/c/home");
        assert!(
            !lines_apart(LISTED, &ours, &theirs, &head),
            "main took away the line the branch listed a module beside"
        );
        let ours = taken(LISTED, "./modules/a");
        assert!(
            !lines_apart(LISTED, &ours, &ours, &ours),
            "both took away one line"
        );
        let ours = added(LISTED, "./modules/a/home", "./modules/b/home");
        let theirs = added(LISTED, "./modules/a", "./modules/s");
        let both = added(&theirs, "./modules/a/home", "./modules/b/home");
        let head = added(&both, "./modules/s", "./modules/x");
        assert!(
            !lines_apart(LISTED, &ours, &theirs, &head),
            "the head listed what neither did"
        );
    }
}
