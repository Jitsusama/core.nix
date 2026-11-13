//! Which of a change's files `merge-net needs` judges: not a Rust file
//! whose change only says something, in comments, since what a VM test
//! does can't move for it. On 13 November one doc comment added to
//! nix-offload's changed.rs made needs ask for a whole VM test and a
//! build of the change's own nix-offload; it was taken back out of #4015
//! instead. Clippy, which reads doc comments, still runs in the gate.

use std::path::Path;

/// Of `files`, those whose change can reach what a VM test does: every
/// one but a Rust file whose diff, as `diff_of` gives it, adds and takes
/// away only comments and blank lines. A file `diff_of` can't say stays.
pub fn reaching(files: Vec<String>, diff_of: impl Fn(&str) -> Option<String>) -> Vec<String> {
    files
        .into_iter()
        .filter(|file| {
            let rust = Path::new(file)
                .extension()
                .is_some_and(|it| it.eq_ignore_ascii_case("rs"));
            !(rust && diff_of(file).is_some_and(|diff| comments_only(&diff)))
        })
        .collect()
}

/// Whether a unified diff of a Rust file changes only `//` comments and
/// blank lines. A diff changing nothing is no comment change, since a
/// file renamed or its mode changed shows so. A line inside a string that
/// opens with `//` would read as a comment; the net, running every VM
/// test that moved after the merge, catches what that lets by.
fn comments_only(diff: &str) -> bool {
    let mut changed = diff
        .lines()
        .filter(|line| !line.starts_with("+++") && !line.starts_with("---"))
        .filter_map(|line| line.strip_prefix('+').or_else(|| line.strip_prefix('-')))
        .peekable();
    changed.peek().is_some()
        && changed.all(|line| {
            let line = line.trim();
            line.is_empty() || line.starts_with("//")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAID: &str = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,5 @@
 /// What it does.
+/// And why, in a sentence more.
+
 fn it() {
-    // a note
+    // a better note
     run();
";

    const DONE: &str = "\
diff --git a/src/b.rs b/src/b.rs
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,2 +1,2 @@
 fn it() {
-    run();
+    run(); // and now said
";

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_rust_file_whose_change_only_says_more_reaches_no_vm_test() {
        let diff_of = |file: &str| match file {
            "src/a.rs" => Some(SAID.to_owned()),
            "src/b.rs" => Some(DONE.to_owned()),
            _ => None,
        };
        assert_eq!(
            reaching(owned(&["src/a.rs", "src/b.rs", "src/c.rs"]), diff_of),
            owned(&["src/b.rs", "src/c.rs"]),
            "a comment-only file was judged, or a code change or an unread file was left out"
        );
    }

    #[test]
    fn a_file_in_another_language_is_judged_whatever_its_change() {
        let said = SAID.replace("a.rs", "a.py");
        assert_eq!(
            reaching(owned(&["src/a.py"]), |_| Some(said.clone())),
            owned(&["src/a.py"]),
            "a file that isn't Rust was read as comments"
        );
    }

    #[test]
    fn a_rename_alone_is_no_comment_change() {
        let renamed = "diff --git a/a.rs b/b.rs\nsimilarity index 100%\nrename from a.rs\n";
        assert!(
            !comments_only(renamed),
            "a file moved read as comments only"
        );
    }
}
