//! Switch Branch inside a repository's row (`code_rows.rs`): its
//! checkout's branches, the one it's on first and the rest by the commit
//! written to last, and the act that checks one out. Like Pull
//! (`pull.rs`), it acts only on a checkout's own folder, never on a tree
//! cut from it, where an agent works, and it never carries changes
//! across: a checkout with changes of its own to its tracked files is
//! left on its branch, and so is one asked for a branch another tree has
//! checked out, which git would refuse; the answer says which tree. Nor
//! does it overwrite a file git doesn't track, ignored ones included, or
//! leave a head on no branch, or a merge or rebase under way, for another.
//!
//! Git runs as `git.rs` runs it, with Joel's config and asking at no
//! terminal. A read past [`READ`] is stopped with all it started; the
//! switch itself never is, which would leave it half written.

use std::path::Path;

use serde_json::{Value, json};

use repos::read::shown;

pub use crate::git::READ;
use crate::git::{ran, tried};

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
    Unknown,
    /// Git couldn't read its branch, its changes or where a branch is.
    Unread,
    /// Its head is on no branch, as Pull refuses it, so a switch would
    /// leave its commits on none.
    Detached,
    /// A merge, rebase, cherry-pick or revert is under way in it.
    Midway,
    /// Its tracked files have changes of its own.
    Changed,
    /// Another tree has the branch checked out, as shown from home.
    Elsewhere(String),
    /// Files git doesn't track, ignored ones included, stand where the
    /// branch has files of its own.
    InTheWay,
    /// Git refused the switch itself.
    Failed,
}

impl Refused {
    fn code(&self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Busy => "busy",
            Refused::Unknown => "unknown",
            Refused::Unread => "unread",
            Refused::Detached => "detached",
            Refused::Midway => "midway",
            Refused::Changed => "changed",
            Refused::Elsewhere(_) => "elsewhere",
            Refused::InTheWay => "intheway",
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
        let word = match &switched {
            Ok(Switched::Switched) => "switched",
            Ok(Switched::Here) => "here",
            Err(why) => why.code(),
        };
        let tree = match &switched {
            Err(Refused::Elsewhere(tree)) => tree.as_str(),
            _ => "",
        };
        let said = self
            .say(word)
            .replace("{branch}", branch)
            .replace("{tree}", tree);
        match switched {
            Ok(_) => json!({"id": id, "said": said}),
            Err(why) => json!({"id": id, "error": {
                "code": "refused",
                "refused": why.code(),
                "message": said,
                "said": said,
            }}),
        }
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
/// the rest by the commit written to last; none when git can't read them,
/// which is no list rather than a checkout with no branches.
#[must_use]
pub fn branches(git: &str, folder: &Path, home: Option<&Path>) -> Option<Vec<Branch>> {
    let format = "--format=%(HEAD)%00%(refname)%00%(committerdate:unix)%00%(worktreepath)";
    let args = [
        "for-each-ref",
        "--sort=-committerdate",
        format,
        "refs/heads",
    ];
    ran(git, folder, &args, Some(READ))
        .ok()
        .map(|printed| listed(&printed, folder, home))
}

/// The branches as `for-each-ref` printed them, each `%(HEAD)`,
/// `%(refname)`, `%(committerdate:unix)` and `%(worktreepath)` apart by
/// a NUL, on a line of its own; the one `folder` is on first.
fn listed(printed: &str, folder: &Path, home: Option<&Path>) -> Vec<Branch> {
    let mut branches: Vec<Branch> = printed
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\0');
            let here = fields.next()? == "*";
            let name = fields.next()?.strip_prefix("refs/heads/")?.to_owned();
            let at_s = fields.next().and_then(|at| at.parse().ok());
            let elsewhere = fields
                .next()
                .filter(|tree| !tree.is_empty() && !same(Path::new(tree), folder))
                .map(|tree| from_home(Path::new(tree), home));
            Some(Branch {
                name,
                at_s,
                here,
                elsewhere: elsewhere.filter(|_| !here),
            })
        })
        .collect();
    // Stable, so the rest keep the order of the commit written to last.
    branches.sort_by_key(|branch| !branch.here);
    branches
}

/// A tree as said from home, `~` for it, where home is named by a link
/// as well as where git names it, by the folder the link leads to.
fn from_home(tree: &Path, home: Option<&Path>) -> String {
    let real = home.and_then(|home| home.canonicalize().ok());
    match home
        .filter(|home| tree.starts_with(home))
        .or(real.as_deref())
    {
        Some(home) => shown(tree, Some(home)),
        None => shown(tree, None),
    }
}

/// Whether two paths name one folder, however each was reached.
fn same(one: &Path, other: &Path) -> bool {
    one == other
        || matches!(
            (one.canonicalize(), other.canonicalize()),
            (Ok(one), Ok(other)) if one == other
        )
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
    // As Pull: only a checkout's own folder holds `.git` as a folder.
    let own = folder.join(".git");
    if !own.is_dir() || !own.join("HEAD").is_file() {
        return Err(Refused::Tree);
    }
    // Never handed to git as an option, whatever it says.
    if branch.is_empty() || branch.starts_with('-') {
        return Err(Refused::Unknown);
    }
    // Before its head is read, which a rebase leaves on no branch.
    if midway(&own) {
        return Err(Refused::Midway);
    }
    let read = |args: &[&str]| ran(git, folder, args, Some(READ));
    let named = format!("refs/heads/{branch}");
    // A branch that isn't there exits 1; git failing exits otherwise.
    match read(&["show-ref", "--verify", "--quiet", &named]) {
        Ok(_) => {}
        Err(Some(1)) => return Err(Refused::Unknown),
        Err(_) => return Err(Refused::Unread),
    }
    // Named in full, so a branch called `heads/x` is never taken for `x`;
    // a head on no branch exits 1.
    match read(&["symbolic-ref", "--quiet", "HEAD"]) {
        Ok(on) if on.trim() == named => return Ok(Switched::Here),
        Ok(_) => {}
        Err(Some(1)) => return Err(Refused::Detached),
        Err(_) => return Err(Refused::Unread),
    }
    let held =
        read(&["for-each-ref", "--format=%(worktreepath)", &named]).map_err(|_| Refused::Unread)?;
    let held = held.trim();
    if !held.is_empty() && !same(Path::new(held), folder) {
        return Err(Refused::Elsewhere(from_home(Path::new(held), home)));
    }
    let changed =
        read(&["status", "--porcelain", "--untracked-files=no"]).map_err(|_| Refused::Unread)?;
    if !changed.trim().is_empty() {
        return Err(Refused::Changed);
    }
    // Never stopped part way, which would leave it half written, and
    // never over an ignored file, which git would overwrite unasked.
    let switching = [
        "switch",
        "--quiet",
        "--no-guess",
        "--no-overwrite-ignore",
        branch,
    ];
    match tried(git, folder, &switching, None) {
        Ok(_) => Ok(Switched::Switched),
        // Git says so in C's words, which `tried` has it speak.
        Err(failure) if failure.said.contains("would be overwritten") => Err(Refused::InTheWay),
        Err(_) => Err(Refused::Failed),
    }
}

/// Whether a merge, rebase, cherry-pick or revert is under way in the
/// checkout whose `.git` folder is `own`, by the files git keeps while
/// it is.
fn midway(own: &Path) -> bool {
    [
        "MERGE_HEAD",
        "rebase-merge",
        "rebase-apply",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
    ]
    .iter()
    .any(|kept| own.join(kept).exists())
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
        assert_eq!(refused(Refused::Unknown)["said"], "unknown");
    }

    #[test]
    fn the_branch_it_is_on_comes_first_and_one_another_tree_holds_names_it() {
        let home = Path::new("/home/joel");
        let printed = " \0refs/heads/fix\x00200\0/home/joel/trees/fix\n\
                       *\0refs/heads/main\x00100\0/home/joel/src/core\n\
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
            Err(Refused::Unknown)
        );
        assert_eq!(switch("true", &folder, "", None), Err(Refused::Unknown));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_tree_cut_from_a_checkout_is_left_alone() {
        let folder = std::env::temp_dir().join(format!("switch-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(switch("true", &folder, "main", None), Err(Refused::Tree));
        // Nor a `.git` folder with no head, which no checkout's own is.
        std::fs::remove_file(folder.join(".git")).unwrap();
        std::fs::create_dir(folder.join(".git")).unwrap();
        assert_eq!(switch("true", &folder, "main", None), Err(Refused::Tree));
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// The first sh on the path, which a stub git is run by, as the build
    /// sandbox has no /bin/sh.
    fn shell() -> std::path::PathBuf {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(|folder| Path::new(folder).join("sh"))
            .find(|sh| sh.is_file())
            .expect("no sh on the path")
    }

    /// A checkout on main whose git answers each subcommand as `answers`
    /// says, as sh's case arms, else as a clean checkout on main would,
    /// writing every run it's asked to `asked`.
    fn stubbed(name: &str, answers: &str) -> (std::path::PathBuf, String) {
        let folder = std::env::temp_dir().join(format!("switch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join(".git")).unwrap();
        std::fs::write(folder.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        let git = folder.join("git");
        std::fs::write(
            &git,
            format!(
                "#!{}\necho \"$*\" >> {}\n\
                 while [ \"$1\" = -C ] || [ \"$1\" = -c ]; do shift 2; done\n\
                 case \"$1\" in {answers}\n\
                 symbolic-ref) echo refs/heads/main;; esac\n",
                shell().display(),
                folder.join("asked").display(),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&git, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let git = git.to_str().unwrap().to_owned();
        (folder, git)
    }

    #[test]
    fn each_refusal_is_said_before_git_is_asked_to_switch() {
        let overwritten = "switch) echo 'error: The following untracked working tree files would be \
                           overwritten by checkout:' >&2; exit 1;;";
        let cases: [(&str, &str, &str, Result<Switched, Refused>); 12] = [
            ("switched", "", "fix", Ok(Switched::Switched)),
            ("here", "", "main", Ok(Switched::Here)),
            (
                "unknown",
                "show-ref) exit 1;;",
                "fix",
                Err(Refused::Unknown),
            ),
            (
                "unread-ref",
                "show-ref) exit 128;;",
                "fix",
                Err(Refused::Unread),
            ),
            (
                "detached",
                "symbolic-ref) exit 1;;",
                "fix",
                Err(Refused::Detached),
            ),
            (
                "unread-head",
                "symbolic-ref) exit 128;;",
                "fix",
                Err(Refused::Unread),
            ),
            (
                "unread-held",
                "for-each-ref) exit 128;;",
                "fix",
                Err(Refused::Unread),
            ),
            (
                "unread-status",
                "status) exit 128;;",
                "fix",
                Err(Refused::Unread),
            ),
            (
                "changed",
                "status) echo ' M a';;",
                "fix",
                Err(Refused::Changed),
            ),
            (
                "elsewhere",
                "for-each-ref) echo /trees/fix;; status) echo ' M a';;",
                "fix",
                Err(Refused::Elsewhere("/trees/fix".to_owned())),
            ),
            ("intheway", overwritten, "fix", Err(Refused::InTheWay)),
            ("failed", "switch) exit 128;;", "fix", Err(Refused::Failed)),
        ];
        for (name, answers, branch, expected) in cases {
            let (folder, git) = stubbed(name, answers);
            assert_eq!(switch(&git, &folder, branch, None), expected, "{name}");
            let asked = std::fs::read_to_string(folder.join("asked")).unwrap_or_default();
            let switching = asked.lines().any(|run| run.contains("/dev/null switch "));
            let reached = matches!(
                expected,
                Ok(Switched::Switched) | Err(Refused::InTheWay | Refused::Failed)
            );
            assert_eq!(switching, reached, "{name} asked git to switch: {asked}");
            let _ = std::fs::remove_dir_all(&folder);
        }
    }

    #[test]
    fn a_merge_or_rebase_under_way_is_left_to_end() {
        for kept in ["MERGE_HEAD", "rebase-merge", "CHERRY_PICK_HEAD"] {
            let (folder, git) = stubbed("midway", "");
            std::fs::write(folder.join(".git").join(kept), "").unwrap();
            assert_eq!(
                switch(&git, &folder, "fix", None),
                Err(Refused::Midway),
                "{kept}"
            );
            assert!(!folder.join("asked").exists(), "git was asked with {kept}");
            let _ = std::fs::remove_dir_all(&folder);
        }
    }
}
