//! A change that passed and must run again because main moved under it
//! gets that run started by `ready` itself, in the asking tree, rather
//! than when its author next reads the answer. From 18:00 to midnight on
//! 14 November, 20 such runs again started a median 1.2 minutes after
//! `ready` asked for them, but a mean 8.9, 9 of them 2.5 to 64 minutes on,
//! while the run itself took a median 13.
//!
//! It runs as a user unit of its own, named for the tree, so it outlives
//! the command that asked, in the asker's own slice, writing its words to
//! a log named for the commit under the net's state folder. Nothing is
//! rebased or pushed: the head already sits on main. It starts only in a
//! tree whose head is the commit asked about, with nothing uncommitted and
//! no other gate going there; a run it started for a head the tree has
//! since left, as when its author rebased by hand, is stopped and started
//! again at the new head; and one that failed at this head is left to its
//! author rather than run again.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What `ready` does for a change that must run again.
#[derive(Debug, PartialEq, Eq)]
pub enum Rerun {
    /// Nothing goes in its tree: start the run.
    Start,
    /// The run started for an earlier head goes on: stop it, then start.
    Restart,
    /// The run started for this head goes on.
    Runs,
    /// Its own run at this head failed: its author reads why.
    Failed,
    /// Its tree isn't at the head, has changes not committed, or has a
    /// gate of its author's going: the author runs it.
    Leave,
}

/// The tree a change asks to merge from, as `ready` sees it.
#[derive(Debug)]
pub struct Tree<'a> {
    /// The commit `ready` was asked about.
    pub asked: &'a str,
    /// The tree's head now.
    pub head: &'a str,
    /// Whether every tracked file is as committed.
    pub clean: bool,
    /// The commit the run `ready` started here goes at, if one goes.
    pub ours: Option<&'a str>,
    /// Whether another gate goes here, its author's.
    pub theirs: bool,
    /// Whether a run `ready` started at `asked` already ended.
    pub ran: bool,
}

/// What to do for the change at `tree.asked`.
#[must_use]
pub fn decide(tree: &Tree) -> Rerun {
    if tree.ours == Some(tree.asked) {
        return Rerun::Runs;
    }
    if tree.theirs || tree.head != tree.asked || !tree.clean {
        return Rerun::Leave;
    }
    if tree.ours.is_some() {
        Rerun::Restart
    } else if tree.ran {
        Rerun::Failed
    } else {
        Rerun::Start
    }
}

/// The user unit a run started for the tree at `folder` goes as: one per
/// tree, so a second ask finds the first.
#[must_use]
pub fn unit(folder: &Path) -> String {
    let name: String = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("merge-net-rerun-{name}.service")
}

/// What the unit's description says of the commit it runs at.
const AT: &str = "merge-net runs changed-checks again at ";

/// The arguments `systemd-run` is given to start changed-checks at
/// `commit` in `folder`, in `slice`, its words going to `log`. The agent
/// socket is left out, as a gate run by hand leaves it, so nothing it
/// does asks for a key, and `path` is the asker's own.
#[must_use]
pub fn started(folder: &Path, commit: &str, slice: &str, log: &Path, path: &str) -> Vec<String> {
    vec![
        "--user".to_owned(),
        "--quiet".to_owned(),
        "--collect".to_owned(),
        "--no-block".to_owned(),
        format!("--unit={}", unit(folder)),
        format!("--description={AT}{commit}"),
        format!("--slice={slice}"),
        format!("--working-directory={}", folder.display()),
        format!("--setenv=PATH={path}"),
        "--property=UnsetEnvironment=SSH_AUTH_SOCK".to_owned(),
        format!("--property=StandardOutput=truncate:{}", log.display()),
        "--property=StandardError=inherit".to_owned(),
        "nix-offload".to_owned(),
        "there".to_owned(),
        "--lent-only".to_owned(),
        "--changed".to_owned(),
    ]
}

/// The commit a unit described as `description` runs at.
#[must_use]
pub fn running_at(description: &str) -> Option<&str> {
    description.strip_prefix(AT).map(str::trim)
}

/// The slice a process in `cgroup`, as `/proc/self/cgroup` gives it, runs
/// in under the user manager, so the run started for it goes beside it.
#[must_use]
pub fn slice_of(cgroup: &str) -> Option<&str> {
    cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .split('/')
        .rfind(|part| {
            Path::new(part)
                .extension()
                .is_some_and(|kind| kind == "slice")
                && !part.starts_with("user")
        })
}

/// Where the run at `commit` writes its words.
#[must_use]
pub fn log(root: &Path, commit: &str) -> PathBuf {
    root.join("reruns").join(format!("{commit}.log"))
}

/// What `ready` tells the asker, having done `done` for the change at
/// `commit`, with its run's words in `log`.
#[must_use]
pub fn said(done: &Rerun, commit: &str, log: &Path) -> Option<String> {
    let short = crate::state::short(commit);
    let log = log.display();
    match done {
        Rerun::Start | Rerun::Restart => Some(format!(
            "merge-net started changed-checks at {short} in this tree itself, its words \
             going to {log}, so start no other run here and change nothing in this tree \
             until it ends; it still runs, about 13 minutes, and merging again once it \
             has passed needs nothing more"
        )),
        Rerun::Runs => Some(format!(
            "merge-net's own run of changed-checks at {short} still runs, its words going \
             to {log}; merge again once it ends"
        )),
        Rerun::Failed => Some(format!(
            "merge-net's own run of changed-checks at {short} ended without passing; its \
             words are in {log}: fix what it says, commit, and run changed-checks yourself"
        )),
        Rerun::Leave => None,
    }
}

/// Start, or find, the run again for the change at `commit`, asked in the
/// tree at `folder`, and say what was done; nothing when it's left to the
/// author.
#[must_use]
pub fn offer(folder: &Path, commit: &str, gates: &[crate::turn::Gate]) -> Option<String> {
    let root = crate::state::State::from_env().root;
    let log = log(&root, commit);
    let unit = unit(folder);
    let shown = systemctl(&["show", &unit, "--property=Description,ActiveState,MainPID"]);
    let field = |name: &str| {
        shown
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
    };
    let active =
        field("ActiveState") == Some("active") || field("ActiveState") == Some("activating");
    let ours = field("Description").and_then(running_at).filter(|_| active);
    let main_pid: u32 = field("MainPID")
        .and_then(|pid| pid.parse().ok())
        .unwrap_or(0);
    let head = git_in(folder, &["rev-parse", "HEAD"])?;
    let dirty = git_in(folder, &["status", "--porcelain", "--untracked-files=no"])?;
    let theirs = gates
        .iter()
        .any(|gate| gate.folder == folder && gate.pid != main_pid && crate::turn::alive(gate));
    let tree = Tree {
        asked: commit,
        head: head.trim(),
        clean: dirty.trim().is_empty(),
        ours,
        theirs,
        ran: ours.is_none() && log.exists(),
    };
    let done = decide(&tree);
    let begun = match done {
        Rerun::Restart => {
            systemctl(&["stop", &unit]);
            start(folder, commit, &log)
        }
        Rerun::Start => start(folder, commit, &log),
        _ => true,
    };
    if !begun {
        return None;
    }
    said(&done, commit, &log)
}

/// Start the run at `commit` in `folder`, writing to `log`: whether
/// systemd took it.
fn start(folder: &Path, commit: &str, log: &Path) -> bool {
    let Some(parent) = log.parent() else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let slice = slice_of(&cgroup).unwrap_or("app.slice");
    let path = std::env::var("PATH").unwrap_or_default();
    Command::new("systemd-run")
        .args(started(folder, commit, slice, log, &path))
        .status()
        .is_ok_and(|status| status.success())
}

/// What `systemctl --user` says, asked `arguments`, or nothing.
fn systemctl(arguments: &[&str]) -> String {
    Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

/// What git says in `folder`, or nothing when it fails.
fn git_in(folder: &Path, arguments: &[&str]) -> Option<String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(arguments)
        .output()
        .ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree<'a>(asked: &'a str, head: &'a str) -> Tree<'a> {
        Tree {
            asked,
            head,
            clean: true,
            ours: None,
            theirs: false,
            ran: false,
        }
    }

    #[test]
    fn a_clean_tree_at_the_head_with_nothing_going_starts_its_run() {
        assert_eq!(
            decide(&tree("c1", "c1")),
            Rerun::Start,
            "a change asked to run again had to start the run itself"
        );
    }

    #[test]
    fn a_run_left_behind_by_an_author_rebasing_by_hand_is_started_again_at_the_new_head() {
        // The author rebased by hand while the run ready started at c1
        // went on, and asks again at c2: c1's run builds what no merge
        // will use.
        let mut moved = tree("c2", "c2");
        moved.ours = Some("c1");
        assert_eq!(
            decide(&moved),
            Rerun::Restart,
            "the run for the head the author left went on beside the new one"
        );
        moved.clean = false;
        assert_eq!(
            decide(&moved),
            Rerun::Leave,
            "a run was started in a tree its author was still changing"
        );
    }

    #[test]
    fn its_own_run_going_at_the_head_is_waited_on() {
        let mut going = tree("c1", "c1");
        going.ours = Some("c1");
        assert_eq!(
            decide(&going),
            Rerun::Runs,
            "a second run was started beside one going at the same head"
        );
    }

    #[test]
    fn the_author_runs_it_where_the_tree_isnt_ready_or_is_theirs() {
        assert_eq!(
            decide(&tree("c1", "c2")),
            Rerun::Leave,
            "a run started in a tree whose head isn't the one asked about"
        );
        let mut dirty = tree("c1", "c1");
        dirty.clean = false;
        assert_eq!(
            decide(&dirty),
            Rerun::Leave,
            "a run started over changes not committed"
        );
        let mut busy = tree("c1", "c1");
        busy.theirs = true;
        assert_eq!(
            decide(&busy),
            Rerun::Leave,
            "a run started beside its author's own gate"
        );
    }

    #[test]
    fn a_run_that_failed_at_the_head_is_not_started_again() {
        let mut failed = tree("c1", "c1");
        failed.ran = true;
        assert_eq!(
            decide(&failed),
            Rerun::Failed,
            "a run that failed was started again at the same head"
        );
    }

    #[test]
    fn the_run_goes_as_one_unit_per_tree_beside_its_asker() {
        let folder = Path::new("/trees/worktree-x.y");
        let log = Path::new("/state/reruns/c1.log");
        let arguments = started(folder, "c1", "agents-pi.slice", log, "/bin");
        for wanted in [
            "--unit=merge-net-rerun-worktree-x-y.service",
            "--slice=agents-pi.slice",
            "--working-directory=/trees/worktree-x.y",
            "--property=UnsetEnvironment=SSH_AUTH_SOCK",
            "--property=StandardOutput=truncate:/state/reruns/c1.log",
        ] {
            assert!(
                arguments.iter().any(|argument| argument == wanted),
                "the run wasn't started with {wanted}: {arguments:?}"
            );
        }
        assert_eq!(
            arguments[arguments.len() - 4..],
            ["nix-offload", "there", "--lent-only", "--changed"],
            "the run started wasn't the gate a change runs by hand"
        );
        let described = arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--description="))
            .and_then(running_at);
        assert_eq!(
            described,
            Some("c1"),
            "the unit didn't say the commit it runs at"
        );
    }

    #[test]
    fn the_run_goes_in_the_askers_own_slice() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/agents.slice/\
                      agents-pi.slice/pi-host.service\n";
        assert_eq!(
            slice_of(cgroup),
            Some("agents-pi.slice"),
            "the run wasn't put in the slice its asker runs in"
        );
    }
}
