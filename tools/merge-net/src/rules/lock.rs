//! Which `flake.lock` changes may leave the lock's area untouched:
//! those moving only inputs `lock-after.txt` names, which nothing else in
//! the lock reads, so their VM tests run after the merge.

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
/// none. Refused, saying why, when it doesn't read, or when anything else
/// reads one of them, by its key or by a follow through its input: a node
/// other than those, since moving it would move what that node builds, or
/// another of the root's own inputs, which would move with it unexamined.
/// An input written in neither form counts as reading one.
fn let_through(text: &str, after: &[String]) -> Result<Vec<String>, String> {
    let unread = || "it doesn't read as a lock".to_owned();
    let nodes = nodes(text).ok_or_else(unread)?;
    let lock: serde_json::Value = serde_json::from_str(text).map_err(|_| unread())?;
    let root = lock
        .get("root")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(unread)?;
    let inputs = nodes
        .get(root)
        .and_then(|node| node.get("inputs"))
        .and_then(serde_json::Value::as_object)
        .ok_or_else(unread)?;
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
    let named = after.join(" ");
    let reader = nodes
        .iter()
        .filter(|(key, _)| *key != root && !keys.contains(key))
        .find(|(_, node)| {
            node.get("inputs")
                .and_then(serde_json::Value::as_object)
                .is_some_and(|inputs| inputs.values().any(&reads_one))
        });
    if let Some((key, _)) = reader {
        return Err(format!("its node {key} reads {named}"));
    }
    let follower = inputs
        .iter()
        .filter(|(name, _)| !after.contains(name))
        .find(|(_, input)| reads_one(input));
    if let Some((name, _)) = follower {
        return Err(format!("the root's input {name} reads {named}"));
    }
    Ok(keys)
}

/// Whether a lock moving from `base` to `head` may run its VM tests after
/// the merge: the nodes it moves, when it moves at least one, only those
/// of inputs `after` names on both sides, and keeps its version, which
/// says how Nix reads every node. Otherwise why not: a lock that doesn't
/// read, moves any other node (the root's own list of inputs among them),
/// or has anything else reading one of those, on either side, runs them
/// first.
pub fn lock_judged(base: &str, head: &str, after: &[String]) -> Result<Vec<String>, String> {
    let moved = lock_moved(base, head).ok_or("it doesn't read as a lock")?;
    if moved.is_empty() {
        return Err("it moves no input".to_owned());
    }
    if version(base) != version(head) {
        return Err(
            "it changes the lock's version, which says how Nix reads every node".to_owned(),
        );
    }
    let before = let_through(base, after).map_err(|why| format!("at the merge base, {why}"))?;
    let now = let_through(head, after)?;
    let others: Vec<&str> = moved
        .iter()
        .filter(|name| !(before.contains(name) && now.contains(name)))
        .map(String::as_str)
        .collect();
    if others.is_empty() {
        Ok(moved)
    } else {
        Err(format!(
            "it moves {}, which lock-after.txt doesn't name",
            others.join(" ")
        ))
    }
}

/// A lock's `version`, or nothing when it doesn't read or holds none.
fn version(text: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("version")
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
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

    /// Whether `lock_judged` lets a lock moving from `base` to `head` run
    /// its VM tests after the merge.
    fn lock_runs_after(base: &str, head: &str, after: &[String]) -> bool {
        lock_judged(base, head, after).is_ok()
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
            lock_after(include_str!("../../lock-after.txt")),
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
    fn a_root_input_reading_a_let_through_input_runs_its_vm_tests_first() {
        // An input of the root's own that follows mycelium, or names its
        // node, would move with each bump unexamined.
        let after = words("mycelium");
        for (reads, how) in [
            (serde_json::json!(["mycelium"]), "by a follow"),
            (serde_json::json!("mycelium"), "by its key"),
        ] {
            let inputs = serde_json::json!({ "mycelium": "mycelium", "foo": reads });
            let base = shaped(&inputs, &[("mycelium", at("a"))]);
            let head = shaped(&inputs, &[("mycelium", at("b"))]);
            let judged = lock_judged(&base, &head, &after);
            assert!(
                judged.as_ref().is_err_and(|why| why.contains("foo")),
                "a root input reading mycelium {how} should run first, naming it, not {judged:?}"
            );
        }
    }

    #[test]
    fn let_through_inputs_may_read_each_other() {
        // Both move only when the list names both, so one reading the
        // other is no reader from outside.
        let after = words("mycelium tool");
        let inputs = serde_json::json!({ "mycelium": "mycelium", "tool": "tool" });
        let tool = serde_json::json!({ "locked": { "rev": "t" }, "inputs": { "m": "mycelium" } });
        let base = shaped(&inputs, &[("mycelium", at("a")), ("tool", tool.clone())]);
        let head = shaped(&inputs, &[("mycelium", at("b")), ("tool", tool)]);
        assert!(
            lock_runs_after(&base, &head, &after),
            "a listed input reading another listed one should still be let through"
        );
    }

    #[test]
    fn an_input_written_in_neither_form_runs_its_vm_tests_first() {
        let after = words("mycelium");
        let inputs = serde_json::json!({ "mycelium": "mycelium", "odd": "odd" });
        let odd = serde_json::json!({ "locked": { "rev": "o" }, "inputs": { "x": 3 } });
        let base = shaped(&inputs, &[("mycelium", at("a")), ("odd", odd.clone())]);
        let head = shaped(&inputs, &[("mycelium", at("b")), ("odd", odd)]);
        assert!(
            !lock_runs_after(&base, &head, &after),
            "an input that is neither a key nor a follow should count as reading mycelium"
        );
    }

    #[test]
    fn a_lock_changing_its_version_runs_its_vm_tests_first() {
        let after = words("mycelium");
        let base = lock(&[("mycelium", "a")]);
        let mut head: serde_json::Value =
            serde_json::from_str(&lock(&[("mycelium", "b")])).unwrap();
        head["version"] = serde_json::json!(8);
        let judged = lock_judged(&base, &head.to_string(), &after);
        assert!(
            judged.as_ref().is_err_and(|why| why.contains("version")),
            "a lock changing its version beside mycelium should run first, not {judged:?}"
        );
    }

    #[test]
    fn an_empty_list_lets_no_lock_through() {
        let base = lock(&[("mycelium", "a")]);
        let head = lock(&[("mycelium", "b")]);
        let judged = lock_judged(&base, &head, &[]);
        assert!(
            judged.as_ref().is_err_and(|why| why.contains("mycelium")),
            "with nothing listed, a mycelium move should run first, naming it, not {judged:?}"
        );
    }
}
