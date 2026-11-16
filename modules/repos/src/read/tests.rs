//! The reading, over checkouts and trees laid out by hand in a temporary
//! folder as git lays them out, so no git runs in the build.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// A fresh folder of the test's own.
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a temporary folder");
    fs::canonicalize(&dir).expect("the folder canonicalizes")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("its folder");
    fs::write(path, text).expect("a file written");
}

/// Sets when a file was last written, in seconds since the epoch.
fn written_at(path: &Path, seconds: u64) {
    File::options()
        .write(true)
        .open(path)
        .expect("the file opens")
        .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
        .expect("its time is set");
}

/// A git folder at `git`, at `main`, with its remote at `url` when given.
fn git_folder(git: &Path, url: Option<&str>, at: u64) {
    let remote = url.map_or(String::new(), |url| {
        format!(
            "[remote \"origin\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"
        )
    });
    write(
        &git.join("config"),
        &format!("[core]\n\trepositoryformatversion = 0\n{remote}"),
    );
    write(&git.join("HEAD"), "ref: refs/heads/main\n");
    write(&git.join("refs/heads/main"), &format!("{COMMIT}\n"));
    written_at(&git.join("HEAD"), at);
}

/// A checkout at `folder` with its remote at `url`.
fn checkout_at(folder: &Path, url: Option<&str>, at: u64) {
    git_folder(&folder.join(".git"), url, at);
}

/// A tree cut from the git folder `common` at `folder`, at a branch or a
/// commit, as `git worktree add` leaves it.
fn tree_at(common: &Path, entry: &str, folder: &Path, head: &str, at: u64) {
    let own = common.join("worktrees").join(entry);
    write(
        &own.join("gitdir"),
        &format!("{}\n", folder.join(".git").display()),
    );
    write(&own.join("commondir"), "../..\n");
    write(&own.join("HEAD"), &format!("{head}\n"));
    written_at(&own.join("HEAD"), at);
    write(
        &folder.join(".git"),
        &format!("gitdir: {}\n", own.display()),
    );
}

fn reading(roots: &[PathBuf], home: Option<&Path>) -> Record {
    read(&Reading {
        roots,
        home,
        machine: "laptop",
        now: 2_000_000_000,
    })
}

#[test]
fn two_clones_of_one_remote_are_one_repository() {
    let src = place("two-clones");
    checkout_at(
        &src.join("core.nix"),
        Some("https://github.com/Jitsusama/core.nix.git"),
        100,
    );
    checkout_at(
        &src.join("github.com/Jitsusama/core.nix"),
        Some("git@github.com:Jitsusama/core.nix.git"),
        200,
    );
    let record = reading(std::slice::from_ref(&src), None);
    assert_eq!(record.repositories.len(), 1, "{record:#?}");
    let repository = &record.repositories[0];
    assert_eq!(
        repository.key, "github:Jitsusama/core.nix",
        "{repository:#?}"
    );
    assert_eq!(repository.name, "core.nix", "{repository:#?}");
    assert_eq!(repository.checkouts.len(), 2, "{repository:#?}");
    assert!(
        repository.checkouts[0]
            .path
            .ends_with("github.com/Jitsusama/core.nix"),
        "the checkout written to last isn't first: {repository:#?}"
    );
    assert_eq!(repository.written_at, Some(200), "{repository:#?}");
    assert_eq!(
        repository.remotes[0].url, "git@github.com:Jitsusama/core.nix.git",
        "the remotes aren't the newest checkout's: {repository:#?}"
    );
}

#[test]
fn a_checkout_lists_its_trees_wherever_they_are() {
    let src = place("trees-src");
    let far = place("trees-far");
    let checkout = src.join("core.nix");
    checkout_at(
        &checkout,
        Some("https://github.com/Jitsusama/core.nix"),
        100,
    );
    let common = checkout.join(".git");
    tree_at(&common, "a", &far.join("a"), "ref: refs/heads/main", 300);
    tree_at(&common, "b", &far.join("b"), COMMIT, 200);
    tree_at(&common, "gone", &far.join("gone"), OTHER, 150);
    fs::remove_dir_all(far.join("gone")).expect("the gone tree's folder goes");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    let kinds: Vec<(TreeKind, bool, String)> = checkout
        .trees
        .iter()
        .map(|tree| (tree.kind, tree.present, tree.head.said()))
        .collect();
    assert_eq!(
        kinds,
        [
            (TreeKind::Worktree, true, "main".to_owned()),
            (TreeKind::Snapshot, true, "0123456".to_owned()),
            (TreeKind::Snapshot, false, "89abcde".to_owned()),
        ],
        "{checkout:#?}"
    );
    assert_eq!(
        checkout.trees[0].head.commit.as_deref(),
        Some(COMMIT),
        "a worktree's branch wasn't read through the checkout's refs"
    );
    assert_eq!(record.repositories[0].written_at, Some(300), "{record:#?}");
}

#[test]
fn a_tree_under_a_root_leads_to_its_bare_checkout() {
    let world = place("bare");
    let common = world.join("git");
    git_folder(&common, Some("https://git.example.org/shop/big.git"), 100);
    tree_at(
        &common,
        "src",
        &world.join("trees/root/src"),
        "ref: refs/heads/main",
        200,
    );
    tree_at(&common, "src1", &world.join("trees/s-1/src"), COMMIT, 300);
    let record = reading(&[world.join("trees")], None);
    assert_eq!(record.repositories.len(), 1, "{record:#?}");
    let repository = &record.repositories[0];
    assert_eq!(
        repository.key, "git.example.org:shop/big",
        "{repository:#?}"
    );
    let checkout = &repository.checkouts[0];
    assert!(checkout.bare, "{checkout:#?}");
    assert_eq!(
        checkout.head, None,
        "a bare checkout has no head: {checkout:#?}"
    );
    assert_eq!(checkout.trees.len(), 2, "{checkout:#?}");
}

#[test]
fn a_reftable_checkouts_heads_and_trunk_are_read_from_its_tables() {
    use crate::reftable::Ref;
    use crate::reftable::tests::{table, write_stack};
    let world = place("reftable");
    let common = world.join("git");
    git_folder(&common, Some("https://git.example.org/shop/big.git"), 100);
    write(&common.join("HEAD"), "ref: refs/heads/.invalid\n");
    write_stack(
        &common.join("reftable"),
        &[table(&[
            ("HEAD", Some(Ref::Symbolic("refs/heads/main".to_owned()))),
            ("refs/heads/feature", Some(Ref::Commit(OTHER.to_owned()))),
            (
                "refs/remotes/origin/HEAD",
                Some(Ref::Symbolic("refs/remotes/origin/trunk".to_owned())),
            ),
            (
                "refs/remotes/origin/trunk",
                Some(Ref::Commit(COMMIT.to_owned())),
            ),
        ])],
    );
    let tree = world.join("trees/w-feature/src");
    tree_at(&common, "src", &tree, "ref: refs/heads/.invalid", 200);
    write_stack(
        &common.join("worktrees/src/reftable"),
        &[table(&[(
            "HEAD",
            Some(Ref::Symbolic("refs/heads/feature".to_owned())),
        )])],
    );
    let pinned = world.join("trees/s-pinned/src");
    tree_at(&common, "src1", &pinned, "ref: refs/heads/.invalid", 150);
    write_stack(
        &common.join("worktrees/src1/reftable"),
        &[table(&[("HEAD", Some(Ref::Commit(COMMIT.to_owned())))])],
    );
    let record = reading(&[world.join("trees")], None);
    let repository = &record.repositories[0];
    assert_eq!(
        repository.trunk.as_deref(),
        Some("trunk"),
        "{repository:#?}"
    );
    let heads: Vec<(TreeKind, Option<&str>, Option<&str>)> = repository.checkouts[0]
        .trees
        .iter()
        .map(|tree| {
            (
                tree.kind,
                tree.head.branch.as_deref(),
                tree.head.commit.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        heads,
        [
            (TreeKind::Worktree, Some("feature"), Some(OTHER)),
            (TreeKind::Snapshot, None, Some(COMMIT)),
        ],
        "{repository:#?}"
    );
}

#[test]
fn a_repository_with_no_remote_is_keyed_by_its_folder() {
    let home = place("local");
    let src = home.join("src");
    checkout_at(&src.join("backwater"), None, 100);
    checkout_at(&src.join("documents"), Some("."), 100);
    let record = reading(&[src], Some(&home));
    let keys: Vec<(&str, &str)> = record
        .repositories
        .iter()
        .map(|repository| (repository.key.as_str(), repository.name.as_str()))
        .collect();
    assert_eq!(
        keys,
        [
            ("local:~/src/backwater", "backwater"),
            ("local:~/src/documents", "documents"),
        ],
        "{record:#?}"
    );
    assert_eq!(
        record.repositories[0].trunk.as_deref(),
        Some("main"),
        "a local repository's trunk is its own main"
    );
}

#[test]
fn the_trunk_is_what_the_remote_names_or_its_main() {
    let src = place("trunk");
    let named = src.join("named");
    checkout_at(&named, Some("https://github.com/a/named"), 100);
    write(
        &named.join(".git/refs/remotes/origin/HEAD"),
        "ref: refs/remotes/origin/trunk\n",
    );
    let packed = src.join("packed");
    checkout_at(&packed, Some("https://github.com/a/packed"), 100);
    write(
        &packed.join(".git/packed-refs"),
        &format!("# pack-refs with: peeled\n{COMMIT} refs/remotes/origin/master\n"),
    );
    let none = src.join("none");
    checkout_at(&none, Some("https://github.com/a/none"), 100);
    let record = reading(&[src], None);
    let trunks: BTreeMap<&str, Option<&str>> = record
        .repositories
        .iter()
        .map(|repository| (repository.name.as_str(), repository.trunk.as_deref()))
        .collect();
    assert_eq!(
        trunks,
        BTreeMap::from([
            ("named", Some("trunk")),
            ("none", None),
            ("packed", Some("master")),
        ]),
        "{record:#?}"
    );
}

#[test]
fn hidden_folders_build_folders_and_depths_past_four_are_never_walked() {
    let src = place("walk");
    checkout_at(
        &src.join(".hidden/thing"),
        Some("https://github.com/a/hidden"),
        100,
    );
    checkout_at(
        &src.join("web/node_modules/dep"),
        Some("https://github.com/a/dep"),
        100,
    );
    checkout_at(
        &src.join("a/b/c/d/e"),
        Some("https://github.com/a/deep"),
        100,
    );
    checkout_at(&src.join("a/b/c/d"), Some("https://github.com/a/four"), 100);
    let record = reading(&[src], None);
    let names: Vec<&str> = record
        .repositories
        .iter()
        .map(|repository| repository.name.as_str())
        .collect();
    assert_eq!(names, ["four"], "{record:#?}");
}

#[test]
fn remotes_are_read_in_their_order_with_their_first_address() {
    let config = "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://github.com/b/x\n\
                  [remote \"origin\"]\n\turl = \"https://github.com/a/x\"\n\turl = https://elsewhere/a/x\n\
                  [branch \"main\"]\n\tremote = origin\n";
    let read = remotes(config);
    assert_eq!(
        read,
        [
            Remote {
                name: "upstream".to_owned(),
                url: "https://github.com/b/x".to_owned()
            },
            Remote {
                name: "origin".to_owned(),
                url: "https://github.com/a/x".to_owned()
            },
        ],
        "{read:#?}"
    );
}

#[test]
fn each_head_knows_the_branch_it_follows_and_where_that_is() {
    let src = place("upstream");
    let checkout = src.join("core.nix");
    checkout_at(
        &checkout,
        Some("https://github.com/Jitsusama/core.nix"),
        100,
    );
    let common = checkout.join(".git");
    write(
        &common.join("config"),
        "[core]\n\tbare = false\n\
         [remote \"origin\"]\n\turl = https://github.com/Jitsusama/core.nix\n\
         [branch \"main\"]\n\tremote = origin\n\tmerge = refs/heads/main\n\
         [branch \"feature\"]\n\tRemote = \"origin\"\n\tmerge = refs/heads/feature\n\
         [branch \"local\"]\n\tremote = .\n\tmerge = refs/heads/main\n\
         [branch \"gone\"]\n\tremote = origin\n\tmerge = refs/heads/gone\n",
    );
    write(
        &common.join("packed-refs"),
        &format!("# pack-refs with: peeled\n{OTHER} refs/remotes/origin/main\n"),
    );
    write(
        &common.join("refs/remotes/origin/feature"),
        &format!("{COMMIT}\n"),
    );
    for branch in ["feature", "local", "gone", "untracked"] {
        write(
            &common.join("refs/heads").join(branch),
            &format!("{COMMIT}\n"),
        );
        tree_at(
            &common,
            branch,
            &src.join(branch),
            &format!("ref: refs/heads/{branch}"),
            200,
        );
    }
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    let upstream = |head: &Head| {
        head.upstream
            .as_ref()
            .map(|upstream| (upstream.branch.clone(), upstream.commit.clone()))
    };
    assert_eq!(
        upstream(checkout.head.as_ref().expect("a head")),
        Some(("origin/main".to_owned(), Some(OTHER.to_owned()))),
        "the checkout's main doesn't follow origin's, from packed-refs: {checkout:#?}"
    );
    let trees: BTreeMap<String, Option<(String, Option<String>)>> = checkout
        .trees
        .iter()
        .map(|tree| (tree.head.said(), upstream(&tree.head)))
        .collect();
    assert_eq!(
        trees,
        BTreeMap::from([
            (
                "feature".to_owned(),
                Some(("origin/feature".to_owned(), Some(COMMIT.to_owned())))
            ),
            ("gone".to_owned(), Some(("origin/gone".to_owned(), None))),
            (
                "local".to_owned(),
                Some(("main".to_owned(), Some(COMMIT.to_owned())))
            ),
            ("untracked".to_owned(), None),
        ]),
        "{checkout:#?}"
    );
    let even: Vec<String> = checkout
        .trees
        .iter()
        .filter(|tree| {
            tree.head
                .upstream
                .as_ref()
                .is_some_and(|upstream| upstream.count == Some(Count::EVEN))
        })
        .map(|tree| tree.head.said())
        .collect();
    assert_eq!(
        even,
        ["feature", "local"],
        "only a head at its upstream's commit is even before counting: {checkout:#?}"
    );
    assert_eq!(
        checkout
            .head
            .as_ref()
            .and_then(|head| head.upstream.as_ref()?.count),
        None,
        "a head apart from its upstream was counted without git: {checkout:#?}"
    );
}

#[test]
fn a_snapshot_follows_nothing() {
    let src = place("upstream-snapshot");
    let checkout = src.join("x");
    checkout_at(&checkout, Some("https://github.com/a/x"), 100);
    tree_at(&checkout.join(".git"), "s", &src.join("s"), COMMIT, 200);
    let record = reading(&[src], None);
    let tree = &record.repositories[0].checkouts[0].trees[0];
    assert_eq!(tree.head.upstream, None, "{tree:#?}");
}

#[test]
fn a_path_is_said_from_home() {
    let home = Path::new("/home/someone");
    assert_eq!(
        shown(Path::new("/home/someone/src/x"), Some(home)),
        "~/src/x"
    );
    assert_eq!(shown(Path::new("/home/someone"), Some(home)), "~");
    assert_eq!(shown(Path::new("/srv/x"), Some(home)), "/srv/x");
    assert_eq!(
        shown(Path::new("/home/someone/src/x"), None),
        "/home/someone/src/x"
    );
}
