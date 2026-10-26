use disk_guard::clock::Awake;
use disk_guard::plan::Class;
use disk_guard::workspaces::{Workspace, candidate, find};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY: f64 = 86_400.0;

fn local(stamp: &str) -> f64 {
    let out = Command::new("date")
        .args(["-d", stamp, "+%s"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn quest(quests: &Path, id: &str, frontmatter: &str) {
    fs::create_dir_all(quests.join(id)).unwrap();
    let readme = format!("---\nid: {id}\n{frontmatter}---\n\n# A Quest\n\nstatus: active\n");
    fs::write(quests.join(id).join("README.md"), readme).unwrap();
}

fn workspace(status: &str, finished: f64, born: f64) -> Workspace {
    Workspace {
        path: PathBuf::from("/home/u/ws/QEST-20260101-ABC123"),
        status: status.into(),
        finished,
        born,
    }
}

#[test]
fn each_workspace_is_read_with_what_its_quest_says() {
    let root = tempfile::tempdir().unwrap();
    let (spaces, quests) = (root.path().join("ws"), root.path().join("quests"));
    quest(
        &quests,
        "QEST-20261001-ABC123",
        "status: concluded\nupdated: 2026-10-03\n",
    );
    quest(&quests, "QEST-20261002-DEF456", "updated: 2026-10-03\n");
    for id in [
        "QEST-20261001-ABC123",
        "QEST-20261002-DEF456",
        "QEST-20261003-NOREAD",
        "notes",
    ] {
        fs::create_dir_all(spaces.join(id)).unwrap();
    }
    let found = find(&spaces, &quests);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, spaces.join("QEST-20261001-ABC123"));
    assert_eq!(found[0].status, "concluded");
    // The end of the day it was last updated, in local time.
    crate::assert_exactly(found[0].finished, local("2026-10-04"));
    assert!((found[0].born - now()).abs() < 60.0, "{}", found[0].born);
}

#[test]
fn a_concluded_quests_workspace_is_idle_from_its_last_write_not_its_day() {
    // A README dates only the day it was last updated, so keeping from
    // that day's end kept a quest concluded this morning until midnight.
    let made = candidate(
        &workspace("concluded", 10_000.0, 500.0),
        0.0,
        7,
        &Awake::default(),
        5_000.0,
    )
    .unwrap();
    assert_eq!(made.class, Class::Workspace);
    crate::assert_exactly(made.idle, 4_500.0);
    assert_eq!(made.size, 7);
}

#[test]
fn a_workspace_that_appeared_later_is_kept_from_then() {
    let made = candidate(
        &workspace("concluded", 1_000.0, 3_000.0),
        0.0,
        7,
        &Awake::default(),
        5_000.0,
    )
    .unwrap();
    crate::assert_exactly(made.idle, 2_000.0);
}

#[test]
fn a_write_since_restarts_the_keeping() {
    let made = candidate(
        &workspace("concluded", 1_000.0, 500.0),
        4_500.0,
        7,
        &Awake::default(),
        5_000.0,
    )
    .unwrap();
    crate::assert_exactly(made.idle, 500.0);
}

#[test]
fn a_quest_retired_before_its_workspace_appeared_has_nothing_to_keep() {
    let retired = workspace("retired", 10.0 * DAY, 9.0 * DAY);
    let made = candidate(&retired, 10.0 * DAY, 7, &Awake::default(), 10.0 * DAY).unwrap();
    crate::assert_exactly(made.idle, f64::INFINITY);
    // Retired after it appeared, it's kept like a concluded one.
    let later = workspace("retired", 10.0 * DAY, 11.0 * DAY);
    crate::assert_exactly(
        candidate(&later, 0.0, 7, &Awake::default(), 12.0 * DAY)
            .unwrap()
            .idle,
        DAY,
    );
}

#[test]
fn a_live_quests_workspace_is_no_candidate() {
    for status in ["active", "paused", "blocked", "anything else"] {
        assert_eq!(
            candidate(
                &workspace(status, 0.0, 0.0),
                0.0,
                7,
                &Awake::default(),
                9.0 * DAY
            ),
            None
        );
    }
}

#[test]
fn the_trees_a_finished_quest_lists_are_over_with_it() {
    // A tree's builds belong to the quest that cut it, so they go when it
    // ends, not a week of idle later.
    let root = tempfile::tempdir().unwrap();
    let quests = root.path().join("quests");
    let trees = |paths: &[&str]| {
        let entries: String = paths
            .iter()
            .map(|path| {
                format!("  - path: {path}\n    providerId: git-worktree\n    repoRoot: {path}\n")
            })
            .collect();
        format!("trees:\n{entries}")
    };
    quest(
        &quests,
        "QEST-20261001-ABC123",
        &format!(
            "status: concluded\nupdated: 2026-10-03\n{}",
            trees(&["/t/one", "/t/two"])
        ),
    );
    quest(
        &quests,
        "QEST-20261002-DEF456",
        &format!(
            "status: active\nupdated: 2026-10-03\n{}",
            trees(&["/t/three"])
        ),
    );
    quest(
        &quests,
        "QEST-20261003-GHI789",
        &format!(
            "status: retired\nupdated: 2026-10-03\n{}",
            trees(&["/t/four"])
        ),
    );
    let mut over = disk_guard::workspaces::finished_trees(&quests);
    over.sort();
    assert_eq!(
        over,
        ["/t/four", "/t/one", "/t/two"].map(PathBuf::from),
        "an active quest's trees are its own"
    );
}

/// A repository outside the workspace with a worktree linked into it, a
/// clone of its own inside, and a repository whose worktree is also inside.
pub(crate) fn linked_fixture(root: &Path) -> (PathBuf, PathBuf) {
    use crate::finished::git;
    let repo = root.join("repo");
    let workspace = root.join("QEST-20261012-ABC123");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(workspace.join("deep")).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "first"]);
    let tree = workspace.join("deep/tree");
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            tree.to_str().unwrap(),
        ],
    );
    let inner = workspace.join("inner");
    fs::create_dir_all(&inner).unwrap();
    git(&inner, &["init", "-q", "-b", "main"]);
    git(&inner, &["commit", "-q", "--allow-empty", "-m", "first"]);
    let inner_tree = workspace.join("inner-tree");
    git(
        &inner,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "x",
            inner_tree.to_str().unwrap(),
        ],
    );
    (repo, workspace)
}

#[test]
fn the_repositories_a_workspace_links_worktrees_from_are_found() {
    let root = tempfile::tempdir().unwrap();
    let (repo, workspace) = linked_fixture(root.path());
    let common = repo.join(".git").canonicalize().unwrap();
    assert_eq!(disk_guard::workspaces::linked(&workspace), [common]);
}

#[test]
fn once_a_workspace_goes_its_worktrees_are_pruned_from_their_repository() {
    use crate::finished::git;
    let root = tempfile::tempdir().unwrap();
    let (repo, workspace) = linked_fixture(root.path());
    let linked = disk_guard::workspaces::linked(&workspace);
    fs::remove_dir_all(&workspace).unwrap();
    for common in &linked {
        disk_guard::workspaces::prune(common).unwrap();
    }
    assert_eq!(
        git(&repo, &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1
    );
    git(&repo, &["checkout", "-q", "feature"]);
}
