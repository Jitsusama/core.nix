//! Whether two changes to a file read as JSON, or as an object any reader
//! makes of one (check-descriptions.nix's entries, a checks.nix's checks),
//! touch different keys, so the gate can count them as two files.

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

/// Whether the four texts, each read by `read`, were changed apart.
pub(super) fn apart_read(
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
/// whose items are objects each with its own `id`, or `test`, is read as
/// keyed by it,
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

/// The fields an array's items may be told apart by, the first all of
/// them have, each its own, winning. The quarantine list keys its
/// entries by `test`: on 13 November each entry the net added to it made
/// a change that only took subtests off another entry gate again, for
/// the list's length had moved.
const KEYS: [&str; 2] = ["id", "test"];

/// An array's items by their `id`, or their `test`, when each is an
/// object with its own.
fn by_id(items: &[Value]) -> Option<Vec<(&str, &Value)>> {
    KEYS.iter().find_map(|key| keyed_by(items, key))
}

/// An array's items by `key`, when each is an object with its own.
fn keyed_by<'a>(items: &'a [Value], key: &str) -> Option<Vec<(&'a str, &'a Value)>> {
    let keyed: Vec<(&str, &Value)> = items
        .iter()
        .map(|item| item[key].as_str().map(|id| (id, item)))
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

    const HELD: &str = r#"[
        {"test": "vm-a", "subtests": ["test_one", "test_two"]},
        {"test": "vm-b", "subtests": ["test_three"]}]"#;

    #[test]
    fn a_list_keyed_by_test_changed_at_different_tests_is_apart() {
        // #4031's case: the branch took a subtest off one test's entry, and
        // main quarantined another test meanwhile.
        let branch = HELD.replace(", \"test_two\"", "");
        let main = HELD.replace("]}]", "]},\n{\"test\": \"vm-c\", \"subtests\": []}]");
        let head = main.replace(", \"test_two\"", "");
        assert!(
            apart(HELD, &branch, &main, &head),
            "main quarantined another test"
        );
        let main = HELD.replace("test_three", "test_four");
        let head = main.replace(", \"test_two\"", "");
        assert!(
            apart(HELD, &branch, &main, &head),
            "another test's entry changed"
        );
        let twice = HELD.replace("vm-b", "vm-a");
        let branch = twice.replace(", \"test_two\"", "");
        let main = twice.replace("]}]", "]},\n{\"test\": \"vm-c\", \"subtests\": []}]");
        let head = main.replace(", \"test_two\"", "");
        assert!(
            !apart(&twice, &branch, &main, &head),
            "a list naming one test twice was keyed by it"
        );
    }
}
