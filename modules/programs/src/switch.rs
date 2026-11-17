//! Switch Branch inside a repository's row (`code_rows.rs`): its
//! checkout's branches, the one it's on first and the rest by the commit
//! written to last, and the act that checks one out. Like Pull
//! (`pull.rs`), it acts only on a checkout's own folder, never on a tree
//! cut from it, where an agent works, and it never carries changes
//! across: a checkout with changes of its own to its tracked files is
//! left on its branch, and so is one asked for a branch another tree has
//! checked out, which git would refuse; the answer says which tree.
//!
//! Git runs as `git.rs` runs it, with Joel's config and asking at no
//! terminal. A read past [`READ`] is stopped with all it started; the
//! switch itself never is, which would leave it half written.

use std::path::Path;

use serde_json::{Value, json};

use crate::git::ran;
pub use crate::pull::READ;

/// How a switch went when it changed nothing it shouldn't.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Switched {
    /// It's on the branch now.
    Switched,
    /// It was on the branch already.
    Here,
}

/// Why a switch left the checkout on its branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The folder isn't a checkout's own, as a tree cut from one isn't.
    Tree,
    /// A Pull or a switch of the same checkout is under way.
    Busy,
    /// No branch is called that now.
    Gone,
    /// Git couldn't read its branch, its changes or where a branch is.
    Unread,
    /// Its tracked files have changes of its own.
    Changed,
    /// Another tree has the branch checked out, as shown from home.
    Elsewhere(String),
    /// Git refused the switch itself.
    Failed,
}

impl Refused {
    fn code(&self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Busy => "busy",
            Refused::Gone => "gone",
            Refused::Unread => "unread",
            Refused::Changed => "changed",
            Refused::Elsewhere(_) => "elsewhere",
            Refused::Failed => "failed",
        }
    }
}

/// What a switch's answer says, from Switch to It's own words in the
/// vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    said: Value,
}

impl Words {
    /// Switch to It's words from the vocabulary, each it hasn't said by
    /// its name.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let said = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|thing| thing["id"] == "verb:code:switch")
            .map_or(Value::Null, |thing| thing["said"].clone());
        Words { said }
    }

    fn say(&self, word: &str) -> String {
        self.said[word].as_str().unwrap_or(word).to_owned()
    }

    /// The answer to a switch's ask: what it did, or why it didn't,
    /// refused as the interface codes it, with the words the row's foot
    /// says.
    #[must_use]
    pub fn answer(&self, id: &Value, branch: &str, switched: Result<Switched, Refused>) -> Value {
        let _ = (id, branch, switched);
        Value::Null
    }
}

/// One branch of a checkout, as Switch Branch lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    /// Its name, without `refs/heads/`.
    pub name: String,
    /// When the commit it's at was written, in seconds since 1970.
    pub at_s: Option<u64>,
    /// Whether the checkout is on it.
    pub here: bool,
    /// The other tree that has it checked out, as shown from home.
    pub elsewhere: Option<String>,
}

/// The branches of the checkout at `folder`, the one it's on first and
/// the rest by the commit written to last; none when git can't read them.
#[must_use]
pub fn branches(git: &str, folder: &Path, home: Option<&Path>) -> Vec<Branch> {
    let _ = (git, folder, home);
    Vec::new()
}

/// The branches as `for-each-ref` printed them, each `%(HEAD)`,
/// `%(refname)`, `%(committerdate:unix)` and `%(worktreepath)` apart by
/// a NUL, on a line of its own; the one `folder` is on first.
fn listed(printed: &str, folder: &Path, home: Option<&Path>) -> Vec<Branch> {
    let _ = (printed, folder, home);
    Vec::new()
}

/// Checks out `branch` in the checkout at `folder`, with the git at
/// `git`: it says why before switching where it can.
///
/// # Errors
///
/// Says why it left the checkout on its branch.
pub fn switch(
    git: &str,
    folder: &Path,
    branch: &str,
    home: Option<&Path>,
) -> Result<Switched, Refused> {
    let _ = (git, folder, branch, home, ran);
    Err(Refused::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::read(&json!({"things": [{"id": "verb:code:switch", "said": {
            "switched": "switched to {branch}",
            "here": "already on {branch}",
            "changed": "not switched: it has changes of its own",
            "elsewhere": "not switched: {tree} has it checked out",
        }}]}))
    }

    #[test]
    fn a_switch_says_which_branch_it_is_on_now() {
        let said = |switched| words().answer(&json!(3), "main", Ok(switched))["said"].clone();
        assert_eq!(said(Switched::Switched), "switched to main");
        assert_eq!(said(Switched::Here), "already on main");
    }

    #[test]
    fn a_refused_switch_says_why_in_its_own_words_and_by_its_code() {
        let refused = |why| words().answer(&json!(3), "main", Err(why))["error"].clone();
        assert_eq!(
            refused(Refused::Changed),
            json!({
                "code": "refused",
                "refused": "changed",
                "message": "not switched: it has changes of its own",
                "said": "not switched: it has changes of its own",
            })
        );
        assert_eq!(
            refused(Refused::Elsewhere("~/trees/fix".to_owned()))["said"],
            "not switched: ~/trees/fix has it checked out"
        );
        assert_eq!(refused(Refused::Gone)["said"], "gone");
    }

    #[test]
    fn the_branch_it_is_on_comes_first_and_one_another_tree_holds_names_it() {
        let home = Path::new("/home/joel");
        let printed = " \0refs/heads/fix\0200\0/home/joel/trees/fix\n\
                       *\0refs/heads/main\0100\0/home/joel/src/core\n\
                       \x20\0refs/heads/old\0\0\n";
        assert_eq!(
            listed(printed, Path::new("/home/joel/src/core"), Some(home)),
            vec![
                Branch {
                    name: "main".to_owned(),
                    at_s: Some(100),
                    here: true,
                    elsewhere: None,
                },
                Branch {
                    name: "fix".to_owned(),
                    at_s: Some(200),
                    here: false,
                    elsewhere: Some("~/trees/fix".to_owned()),
                },
                Branch {
                    name: "old".to_owned(),
                    at_s: None,
                    here: false,
                    elsewhere: None,
                },
            ]
        );
    }

    #[test]
    fn a_branch_named_like_an_option_is_never_handed_to_git() {
        let folder = std::env::temp_dir().join(format!("switch-option-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join(".git")).unwrap();
        std::fs::write(folder.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        // A git that would say yes to anything, so only the check refuses.
        assert_eq!(
            switch("true", &folder, "--orphan", None),
            Err(Refused::Gone)
        );
        assert_eq!(switch("true", &folder, "", None), Err(Refused::Gone));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_tree_cut_from_a_checkout_is_left_alone() {
        let folder = std::env::temp_dir().join(format!("switch-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(switch("true", &folder, "main", None), Err(Refused::Tree));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
