//! Whether two changes to one JSON file, or to check-descriptions.nix,
//! touch different keys, so the gate can count them as two files. It
//! changes with how such files are compared, not with what the gate asks
//! of a branch.

use serde_json::Value;

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
}
