//! Forget Gone Trees and Gather Its Store over the socket: each offered
//! on a repository's row only where a checkout wants it, doing as git
//! would, saying what changed, and refused, saying why, where git would
//! lose what only a gone tree holds or run what a checkout's config names.

use std::path::Path;

use serde_json::{Value, json};

use super::Running;
use super::code::{Behind, real};

/// The acts on a repository's row, by their keys, as a watch is told them.
fn acts_of(row: &Value) -> Vec<String> {
    row["row"]["acts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|act| act["key"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn forget_gone_trees_has_git_forget_them_keeping_the_locked_and_the_branches() {
    let running = Running::new("code-forget");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let git = |folder: &Path, args: &[&str]| behind.git(folder, args);
    // Named as git lists them, with no link in the case's folder, which a
    // gone tree's own path can't be resolved through.
    let cases = real(&running.path(""));
    let cut = |name: &str| {
        let tree = cases.join(name);
        git(
            &here,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                name,
                tree.to_str().unwrap(),
            ],
        );
        tree
    };
    let (gone, locked, standing) = (cut("gone"), cut("locked"), cut("standing"));
    git(&here, &["worktree", "lock", locked.to_str().unwrap()]);
    std::fs::remove_dir_all(&gone).unwrap();
    std::fs::remove_dir_all(&locked).unwrap();
    let mut watcher = running.client();
    let answer = watcher.ask(&json!({"id": 9, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = answer["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["row"]["name"] == "alpha")
        })
        .cloned()
        .unwrap_or_else(|| panic!("the checkout has no row: {answer}"));
    assert!(
        acts_of(&alpha).contains(&"verb:code:forget".to_owned()),
        "Forget Gone Trees isn't offered where git lists a tree that's gone: {alpha}"
    );
    let key = alpha["key"].as_str().unwrap().to_owned();

    let mut client = running.client();
    let mut forget = |id: u64, folder: &Path| {
        client.ask(&json!({"id": id, "ask": "code:forget", "path": folder}))
    };
    let said = forget(1, &here);
    assert_eq!(
        said["said"], "forgot 1 gone tree, kept 1 locked",
        "Forget Gone Trees didn't say what git forgot and kept: {said}"
    );
    let listed = git(&here, &["worktree", "list", "--porcelain"]);
    assert!(
        !listed.contains(gone.to_str().unwrap()),
        "git still lists the gone tree: {listed}"
    );
    assert!(
        listed.contains(locked.to_str().unwrap()) && listed.contains(standing.to_str().unwrap()),
        "a locked or standing tree was forgotten: {listed}"
    );
    assert!(
        standing.join(".git").is_file(),
        "a standing tree was touched"
    );
    assert_eq!(
        git(&here, &["branch", "--list", "gone"]),
        "gone",
        "a gone tree's branch went with it"
    );
    let said = forget(2, &here);
    assert_eq!(
        said["said"], "no gone trees to forget, kept 1 locked",
        "{said}"
    );
    git(&here, &["worktree", "unlock", locked.to_str().unwrap()]);
    let said = forget(3, &here);
    assert_eq!(said["said"], "forgot 1 gone tree", "{said}");
    // The disk is read again, so the row no longer offers what's done.
    let forgotten = watcher.changed(&key);
    assert!(
        !acts_of(&forgotten).contains(&"verb:code:forget".to_owned()),
        "the row still offers Forget Gone Trees with none gone: {forgotten}"
    );
    let said = forget(4, &standing);
    assert_eq!(
        said["error"]["refused"], "tree",
        "a tree cut from a checkout was tended: {said}"
    );
}

#[test]
fn gather_its_store_runs_gits_upkeep_where_its_due_and_says_so() {
    let running = Running::new("code-gather");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let git = |folder: &Path, args: &[&str]| behind.git(folder, args);
    // Due at more than one pack, so two make it due.
    git(&here, &["config", "gc.autoPackLimit", "1"]);
    git(&here, &["repack", "--quiet", "-a", "-d"]);
    behind.commit(&here, "three");
    git(&here, &["repack", "--quiet", "-d"]);
    let packs = || {
        std::fs::read_dir(here.join(".git/objects/pack"))
            .unwrap()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|it| it == "idx"))
            .count()
    };
    assert_eq!(packs(), 2, "the case's store isn't in two packs");
    let mut watcher = running.client();
    let answer = watcher.ask(&json!({"id": 9, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = answer["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["row"]["name"] == "alpha")
        })
        .cloned()
        .unwrap_or_else(|| panic!("the checkout has no row: {answer}"));
    assert!(
        acts_of(&alpha).contains(&"verb:code:gather".to_owned()),
        "Gather Its Store isn't offered where upkeep is due: {alpha}"
    );
    let key = alpha["key"].as_str().unwrap().to_owned();

    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(said["said"], "gathered its store", "{said}");
    assert_eq!(packs(), 1, "git's upkeep didn't gather the packs");
    let gathered = watcher.changed(&key);
    assert!(
        !acts_of(&gathered).contains(&"verb:code:gather".to_owned()),
        "the row still offers Gather Its Store once gathered: {gathered}"
    );
    let said = client.ask(&json!({"id": 2, "ask": "code:gather", "path": here}));
    assert_eq!(said["said"], "nothing was due", "{said}");
}

#[test]
fn gather_its_store_says_so_when_git_leaves_it_as_it_was() {
    let running = Running::new("code-gather-untouched");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    // Due by loose objects in the folder git counts, each reached by
    // nothing and too young for git to drop: its upkeep runs, leaves the
    // store as it was, and exits 0 all the same. These three blobs' names
    // begin 17, the folder of 256 git's count reads.
    behind.git(&here, &["repack", "--quiet", "-a", "-d"]);
    behind.git(&here, &["config", "gc.auto", "1"]);
    let blobs = real(&running.path("")).join("blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    for number in [857, 868, 1169] {
        let blob = blobs.join(number.to_string());
        std::fs::write(&blob, format!("blob {number}\n")).unwrap();
        behind.git(&here, &["hash-object", "-w", blob.to_str().unwrap()]);
    }
    let counted = behind.git(&here, &["count-objects", "-v"]);
    assert!(
        counted.lines().any(|line| line == "count: 3"),
        "the store holds loose objects besides the three unreached blobs: {counted}"
    );
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["said"], "git left its store as it was",
        "Gather Its Store said it gathered a store git left: {said}"
    );
}

#[test]
fn gather_its_store_is_refused_while_another_upkeep_holds_it() {
    let running = Running::new("code-gather-held");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    due_by_its_packs(&behind);
    // Held by another upkeep's lock, which git's own upkeep passes by in
    // silence.
    let lock = here.join(".git/objects/maintenance.lock");
    std::fs::write(&lock, "").unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["error"]["said"], "not gathered: git is already at work in it",
        "Gather Its Store didn't say another upkeep holds the store: {said}"
    );
    // Past a gathering of Code's own, Joel's gc may still hold it.
    let held = std::fs::File::options().write(true).open(&lock).unwrap();
    let ago = |hours| std::time::SystemTime::now() - std::time::Duration::from_hours(hours);
    held.set_modified(ago(2)).unwrap();
    let said = client.ask(&json!({"id": 2, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["error"]["said"], "not gathered: git is already at work in it",
        "Gather Its Store called a long upkeep's lock left behind: {said}"
    );
    // A lock older than any upkeep is one a killed upkeep left.
    held.set_modified(ago(13)).unwrap();
    let said = client.ask(&json!({"id": 3, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["error"]["said"],
        "not gathered: git left its upkeep's lock behind, in .git/objects/maintenance.lock",
        "Gather Its Store called a lock left behind one at work: {said}"
    );
}

/// Makes the checkout behind `behind` due by its packs alone, two where
/// its config allows one.
fn due_by_its_packs(behind: &Behind) {
    let git = |args: &[&str]| behind.git(&behind.here, args);
    git(&["config", "gc.autoPackLimit", "1"]);
    git(&["repack", "--quiet", "-a", "-d"]);
    behind.commit(&behind.here, "three");
    git(&["repack", "--quiet", "-d"]);
}

#[test]
fn gather_its_store_runs_no_program_its_config_names() {
    let running = Running::new("code-gather-hooked");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    due_by_its_packs(&behind);
    let ran = real(&running.path("")).join("ran");
    let hook = format!("touch {}", ran.display());
    behind.git(&here, &["config", "gc.recentObjectsHook", &hook]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["error"]["said"], "not gathered: its config names a program git's gc would run",
        "Gather Its Store ran upkeep under a config naming a program: {said}"
    );
    assert!(!ran.exists(), "the program the checkout's config names ran");
}

#[test]
fn gather_its_store_forgets_no_tree_however_long_gone() {
    let running = Running::new("code-gather-keeps-trees");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    due_by_its_packs(&behind);
    let old = real(&running.path("")).join("old");
    let path = old.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    std::fs::remove_dir_all(&old).unwrap();
    // Gone past git's three months, which its upkeep forgets on its own.
    let index = here.join(".git/worktrees/old/index");
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(24 * 120);
    let file = std::fs::File::options().write(true).open(&index).unwrap();
    file.set_modified(long_ago).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(said["said"], "gathered its store", "{said}");
    let listed = behind.git(&here, &["worktree", "list", "--porcelain"]);
    assert!(
        listed.contains(path),
        "Gather Its Store forgot a gone tree: {listed}"
    );
}

#[test]
fn forget_gone_trees_keeps_a_tree_whose_commits_nothing_else_holds() {
    let running = Running::new("code-forget-stranded");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let cases = real(&running.path(""));
    let pinned = cases.join("pinned");
    let path = pinned.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&pinned, "only-here");
    let commit = behind.head(&pinned);
    std::fs::remove_dir_all(&pinned).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["error"]["said"], "not forgotten: a gone tree holds commits no branch has",
        "Forget Gone Trees forgot a tree holding the only way to its commits: {said}"
    );
    let listed = behind.git(&here, &["worktree", "list", "--porcelain"]);
    assert!(
        listed.contains(path),
        "git no longer lists the pinned tree: {listed}"
    );
    // Once a branch holds its commit, it's forgotten as any gone tree is.
    behind.git(&here, &["branch", "kept", &commit]);
    let said = client.ask(&json!({"id": 2, "ask": "code:forget", "path": here}));
    assert_eq!(said["said"], "forgot 1 gone tree", "{said}");
    // A tree worked in and moved back to a commit a branch has keeps
    // that work in its own reflog alone, until git would expire it.
    let moved = cases.join("moved");
    let path = moved.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&moved, "moved-off");
    let main = behind.head(&here);
    behind.git(&moved, &["checkout", "--quiet", "--detach", &main]);
    std::fs::remove_dir_all(&moved).unwrap();
    let said = client.ask(&json!({"id": 3, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["error"]["said"], "not forgotten: a gone tree holds commits no branch has",
        "Forget Gone Trees forgot a tree whose reflog alone holds its work: {said}"
    );
    behind.git(&here, &["config", "gc.reflogExpireUnreachable", "now"]);
    let said = client.ask(&json!({"id": 4, "ask": "code:forget", "path": here}));
    assert_eq!(said["said"], "forgot 1 gone tree", "{said}");
}

#[test]
fn forget_gone_trees_passes_a_locked_tree_by_whatever_it_holds() {
    let running = Running::new("code-forget-locked-stranded");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let cases = real(&running.path(""));
    // A locked tree's work no branch has: git never forgets it, so it
    // holds no Forget back.
    let locked = cases.join("locked");
    let path = locked.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&locked, "only-here");
    behind.git(&here, &["worktree", "lock", path]);
    std::fs::remove_dir_all(&locked).unwrap();
    let plain = cases.join("plain");
    let plain = plain.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "-b", "plain", plain]);
    std::fs::remove_dir_all(plain).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["said"], "forgot 1 gone tree, kept 1 locked",
        "Forget Gone Trees was held back by a tree git keeps: {said}"
    );
}

#[test]
fn forget_gone_trees_forgets_a_tree_whose_work_a_branch_or_a_standing_tree_holds() {
    let running = Running::new("code-forget-held-elsewhere");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let cases = real(&running.path(""));
    // Work replaced on a branch, as an amend or a rebase leaves it, which
    // the branch's own reflog still holds after its tree is gone.
    let replaced = cases.join("replaced");
    let path = replaced.to_str().unwrap();
    behind.git(
        &here,
        &["worktree", "add", "--quiet", "-b", "replaced", path],
    );
    behind.commit(&replaced, "first-try");
    behind.git(&replaced, &["reset", "--quiet", "--soft", "HEAD~1"]);
    behind.commit(&replaced, "second-try");
    std::fs::remove_dir_all(&replaced).unwrap();
    // A gone tree at a commit only a standing tree is at.
    let standing = cases.join("standing");
    let path = standing.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&standing, "standing-only");
    let commit = behind.head(&standing);
    let beside = cases.join("beside");
    let path = beside.to_str().unwrap();
    behind.git(
        &here,
        &["worktree", "add", "--quiet", "--detach", path, &commit],
    );
    std::fs::remove_dir_all(&beside).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["said"], "forgot 2 gone trees",
        "Forget Gone Trees was held back by work a branch or a standing tree holds: {said}"
    );
}

#[test]
fn forget_gone_trees_keeps_what_a_gone_trees_own_ref_alone_holds() {
    let running = Running::new("code-forget-own-ref");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let marked = real(&running.path("")).join("marked");
    let path = marked.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&marked, "marked-only");
    let commit = behind.head(&marked);
    behind.git(&marked, &["update-ref", "refs/worktree/mark", &commit]);
    let main = behind.head(&here);
    behind.git(&marked, &["checkout", "--quiet", "--detach", &main]);
    std::fs::remove_dir_all(&marked).unwrap();
    // No reflog counts, so the tree's own ref alone holds its commit.
    behind.git(&here, &["config", "gc.reflogExpireUnreachable", "now"]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["error"]["said"], "not forgotten: a gone tree holds commits no branch has",
        "Forget Gone Trees forgot a tree whose own ref alone holds a commit: {said}"
    );
}

#[test]
fn forget_gone_trees_counts_no_reflog_entry_git_would_expire() {
    // gc expires an entry no ref reaches past the one, and any past the
    // other, so each alone lets the entry go.
    expiring("code-forget-unreached", "gc.reflogExpireUnreachable");
    expiring("code-forget-expiring", "gc.reflogExpire");
}

/// Forget, in a checkout whose `key` expires every reflog entry now, is
/// refused for a commit only the checkout's own reflog besides a gone
/// tree reaches.
fn expiring(name: &str, key: &str) {
    let running = Running::new(name);
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    // A commit the checkout's own HEAD reflog alone reaches, besides a
    // gone tree at it, which holds it for as long as its record stands.
    behind.git(&here, &["checkout", "--quiet", "--detach"]);
    behind.commit(&here, "passing");
    let commit = behind.head(&here);
    behind.git(&here, &["checkout", "--quiet", "main"]);
    let resting = real(&running.path("")).join("resting");
    let path = resting.to_str().unwrap();
    behind.git(
        &here,
        &["worktree", "add", "--quiet", "--detach", path, &commit],
    );
    std::fs::remove_dir_all(&resting).unwrap();
    // Every reflog entry past git's keeping, as its gc would expire them.
    behind.git(&here, &["config", key, "now"]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": here}));
    assert_eq!(
        said["error"]["said"], "not forgotten: a gone tree holds commits no branch has",
        "Forget Gone Trees counted a reflog entry {key} would expire: {said}"
    );
}

#[test]
fn a_reftable_checkouts_gone_trees_are_refused_unread() {
    let running = Running::new("code-reftable");
    let behind = Behind::new(&running);
    let cases = real(&running.path(""));
    let tabled = cases.join("src/tabled");
    let init = ["init", "--quiet", "--ref-format=reftable", "-b", "main"];
    behind.git(&cases, &[&init[..], &[tabled.to_str().unwrap()]].concat());
    behind.commit(&tabled, "one");
    let pinned = cases.join("pinned");
    let path = pinned.to_str().unwrap();
    behind.git(&tabled, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&pinned, "only-here");
    std::fs::remove_dir_all(&pinned).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:forget", "path": tabled}));
    assert_eq!(
        said["error"]["said"],
        "not forgotten: git's records of its gone trees couldn't all be read",
        "Forget Gone Trees took a reftable's records to hold nothing: {said}"
    );
    let listed = behind.git(&tabled, &["worktree", "list", "--porcelain"]);
    assert!(
        listed.contains(path),
        "git no longer lists the tree: {listed}"
    );
}

#[test]
fn gather_its_store_keeps_a_tree_git_cant_date_and_what_one_alone_holds() {
    let running = Running::new("code-gather-undated");
    let behind = Behind::new(&running);
    let here = behind.here.clone();
    let cases = real(&running.path(""));
    // Due by a second pack as large as the first, so git's geometric
    // repack merges them, holding the pinned tree's commit.
    behind.git(&here, &["config", "gc.autoPackLimit", "1"]);
    behind.git(&here, &["repack", "--quiet", "-a", "-d"]);
    let pinned = cases.join("pinned");
    let path = pinned.to_str().unwrap();
    behind.git(&here, &["worktree", "add", "--quiet", "--detach", path]);
    behind.commit(&pinned, "only-here");
    let commit = behind.head(&pinned);
    std::fs::remove_dir_all(&pinned).unwrap();
    behind.git(&here, &["repack", "--quiet", "-d"]);
    // Never checked out, so it has no index for git to date it by. Cut
    // last, as a commit's own upkeep would forget it.
    let unchecked = cases.join("unchecked");
    let unchecked = unchecked.to_str().unwrap();
    let add = ["worktree", "add", "--quiet", "--no-checkout", "--detach"];
    behind.git(&here, &[&add[..], &[unchecked]].concat());
    std::fs::remove_dir_all(unchecked).unwrap();
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:gather", "path": here}));
    assert_eq!(
        said["error"]["said"], "not gathered: a gone tree holds commits no branch has",
        "Gather Its Store ran where a gc could take what a gone tree alone holds: {said}"
    );
    behind.git(&here, &["branch", "kept", &commit]);
    let said = client.ask(&json!({"id": 2, "ask": "code:gather", "path": here}));
    assert_eq!(said["said"], "gathered its store", "{said}");
    let listed = behind.git(&here, &["worktree", "list", "--porcelain"]);
    assert!(
        listed.contains(unchecked) && listed.contains(path),
        "Gather Its Store forgot a gone tree: {listed}"
    );
}
