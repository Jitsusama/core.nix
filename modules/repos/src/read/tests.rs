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
    // Loose files a reftable checkout doesn't keep its branches in.
    write(&common.join("refs/heads/jg/stray"), &format!("{COMMIT}\n"));
    write(&common.join("refs/heads/jg/other"), &format!("{COMMIT}\n"));
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
    // Its tables hold its one branch, feature. The two loose files beside
    // them are not where a reftable checkout keeps its refs, so counting
    // them too would say 3.
    assert_eq!(repository.checkouts[0].branches, Some(1), "{repository:#?}");
}

#[test]
fn a_checkout_counts_its_branches_once_each_and_its_configs_bytes() {
    let src = place("branches");
    let folder = src.join("heavy");
    checkout_at(&folder, Some("https://git.example.org/jg/heavy.git"), 100);
    let git = folder.join(".git");
    // main is loose already; jg/a is loose alone, jg/b packed alone, and
    // main packed too, as a branch written since the last pack is.
    write(&git.join("refs/heads/jg/a"), &format!("{COMMIT}\n"));
    write(&git.join("refs/heads/jg/half.lock"), &format!("{OTHER}\n"));
    // A dot-named file is no branch git would name, and a link is passed
    // over rather than followed, so neither is counted.
    write(&git.join("refs/heads/.stray"), &format!("{OTHER}\n"));
    std::os::unix::fs::symlink(git.join("refs/heads/jg/a"), git.join("refs/heads/jg/link"))
        .expect("a link");
    // The tag's peeled line opens with ^ and names no ref.
    write(
        &git.join("packed-refs"),
        &format!(
            "# pack-refs with: peeled fully-peeled sorted\n{COMMIT} refs/heads/jg/b\n\
             {COMMIT} refs/heads/main\n{OTHER} refs/remotes/origin/main\n\
             {OTHER} refs/tags/v1\n^{COMMIT}\n"
        ),
    );
    let config = fs::metadata(git.join("config")).expect("the config").len();
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(
        (checkout.branches, checkout.config_bytes),
        (Some(3), Some(config)),
        "{checkout:#?}"
    );
}

#[test]
fn a_checkout_whose_packed_refs_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("unread");
    let folder = src.join("locked");
    checkout_at(&folder, Some("https://git.example.org/jg/locked.git"), 100);
    let git = folder.join(".git");
    // A packed-refs that is a folder can't be read as a file, as one with
    // its permissions taken away can't, and a test can't run as anyone but
    // root may.
    fs::create_dir_all(git.join("packed-refs")).expect("a folder in its place");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_loose_branches_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("unwalked");
    let folder = src.join("locked");
    checkout_at(&folder, Some("https://git.example.org/jg/locked.git"), 100);
    let git = folder.join(".git");
    // A refs/heads that is a file can't be walked as a folder.
    fs::remove_dir_all(git.join("refs/heads")).expect("its branches gone");
    write(&git.join("refs/heads"), "not a folder\n");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_branches_folder_is_a_link_says_no_count_rather_than_walking_it() {
    let src = place("linked");
    let folder = src.join("linked");
    checkout_at(&folder, Some("https://git.example.org/jg/linked.git"), 100);
    let git = folder.join(".git");
    let elsewhere = src.join("elsewhere");
    write(&elsewhere.join("jg/a"), &format!("{COMMIT}\n"));
    fs::remove_dir_all(git.join("refs/heads")).expect("its branches gone");
    std::os::unix::fs::symlink(&elsewhere, git.join("refs/heads")).expect("a link");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_reftable_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("untabled");
    let folder = src.join("untabled");
    checkout_at(
        &folder,
        Some("https://git.example.org/jg/untabled.git"),
        100,
    );
    // A reftable folder with no tables.list is a stack that can't be read.
    fs::create_dir_all(folder.join(".git/reftable")).expect("a reftable folder");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_never_packed_counts_its_loose_branches() {
    let src = place("unpacked");
    let folder = src.join("unpacked");
    checkout_at(
        &folder,
        Some("https://git.example.org/jg/unpacked.git"),
        100,
    );
    let git = folder.join(".git");
    write(&git.join("refs/heads/jg/a"), &format!("{COMMIT}\n"));
    assert!(
        !git.join("packed-refs").exists(),
        "the checkout has no packed-refs"
    );
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    // main and jg/a, with no packed-refs to add to them.
    assert_eq!(checkout.branches, Some(2), "{checkout:#?}");
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
fn a_credential_in_a_web_remote_is_never_said_and_a_logins_name_stays() {
    let config = "[remote \"origin\"]\n\turl = https://joel:ghp_secret@github.com/a/x.git\n\
                  [remote \"token\"]\n\turl = https://ghp_secret@github.com/a/y\n\
                  [remote \"ssh\"]\n\turl = ssh://git@github.com:22/a/z\n\
                  [remote \"scp\"]\n\turl = git@github.com:a/w.git\n\
                  [remote \"upper\"]\n\turl = HTTPS://ghp_secret@github.com/a/v\n\
                  [remote \"six\"]\n\turl = https://u:t@[::1]:8443/a\n\
                  [remote \"query\"]\n\turl = https://t@host?x@y\n\
                  [remote \"path\"]\n\turl = https://host/a@b\n\
                  [remote \"ftp\"]\n\turl = ftp://u:p@host/a\n\
                  [remote \"ssh-pass\"]\n\turl = ssh://joel:pw@host/a\n";
    let urls: Vec<String> = remotes(config)
        .into_iter()
        .map(|remote| remote.url)
        .collect();
    assert_eq!(
        urls,
        [
            "https://github.com/a/x.git",
            "https://github.com/a/y",
            "ssh://git@github.com:22/a/z",
            "git@github.com:a/w.git",
            "HTTPS://github.com/a/v",
            "https://[::1]:8443/a",
            "https://host?x@y",
            "https://host/a@b",
            "ftp://host/a",
            "ssh://joel@host/a",
        ],
        "a remote said with its credential, or with more than it taken away"
    );
}

#[test]
fn a_tree_two_checkouts_name_is_listed_by_the_one_it_names() {
    let src = place("copied");
    let far = place("copied-far");
    let url = Some("https://github.com/Jitsusama/core.nix");
    let first = src.join("core.nix");
    checkout_at(&first, url, 100);
    tree_at(
        &first.join(".git"),
        "t",
        &far.join("t"),
        "ref: refs/heads/main",
        200,
    );
    // A copy made whole, its worktrees/ with it: its entry names the same
    // tree, whose .git file still names the first's entry.
    let copy = src.join("core.nix-copy");
    checkout_at(&copy, url, 300);
    let own = copy.join(".git/worktrees/t");
    write(
        &own.join("gitdir"),
        &format!("{}\n", far.join("t/.git").display()),
    );
    write(&own.join("commondir"), "../..\n");
    write(&own.join("HEAD"), "ref: refs/heads/main\n");
    let record = reading(std::slice::from_ref(&src), None);
    let trees: Vec<(String, usize)> = record.repositories[0]
        .checkouts
        .iter()
        .map(|checkout| (checkout.path.clone(), checkout.trees.len()))
        .collect();
    let (first, copy) = (first.display().to_string(), copy.display().to_string());
    assert_eq!(
        trees,
        [(copy, 0), (first, 1)],
        "a tree listed by a checkout it doesn't name"
    );
}

/// A checkout's entry for a tree at `folder`, written without the tree.
fn entry_at(common: &Path, folder: &Path) {
    let own = common.join("worktrees/t");
    write(
        &own.join("gitdir"),
        &format!("{}\n", folder.join(".git").display()),
    );
    write(&own.join("commondir"), "../..\n");
    write(&own.join("HEAD"), "ref: refs/heads/main\n");
}

#[test]
fn a_gone_tree_two_checkouts_name_is_listed_once_by_the_path_sorting_first() {
    // Whichever checkout was written last, the owner holds.
    for (first_at, copy_at) in [(100, 300), (300, 100)] {
        let src = place(&format!("gone-twice-{first_at}"));
        let url = Some("https://github.com/Jitsusama/core.nix");
        let gone = src.join("far/t");
        let first = src.join("core.nix");
        let copy = src.join("core.nix-copy");
        checkout_at(&first, url, first_at);
        checkout_at(&copy, url, copy_at);
        entry_at(&first.join(".git"), &gone);
        entry_at(&copy.join(".git"), &gone);
        let record = reading(std::slice::from_ref(&src), None);
        let listing: Vec<String> = record.repositories[0]
            .checkouts
            .iter()
            .filter(|checkout| !checkout.trees.is_empty())
            .map(|checkout| checkout.path.clone())
            .collect();
        assert_eq!(
            listing,
            [first.display().to_string()],
            "a gone tree's owner, with the copy written at {copy_at}"
        );
    }
}

#[test]
fn a_tree_naming_a_checkout_outside_the_roots_stays_the_copys() {
    let src = place("copied-in");
    let outside = place("copied-from");
    let url = Some("https://github.com/Jitsusama/core.nix");
    let original = outside.join("core.nix");
    checkout_at(&original, url, 100);
    tree_at(
        &original.join(".git"),
        "t",
        &outside.join("t"),
        "ref: refs/heads/main",
        200,
    );
    let copy = src.join("core.nix");
    checkout_at(&copy, url, 300);
    entry_at(&copy.join(".git"), &outside.join("t"));
    let record = reading(std::slice::from_ref(&src), None);
    let trees = &record.repositories[0].checkouts[0].trees;
    assert_eq!(
        trees.len(),
        1,
        "a tree whose checkout isn't read listed by none: {record:#?}"
    );
}

#[test]
fn a_branch_following_an_address_is_said_without_its_credential() {
    let src = place("follows-address");
    let checkout = src.join("core.nix");
    checkout_at(&checkout, None, 100);
    write(
        &checkout.join(".git/config"),
        "[core]\n\tbare = false\n\
         [branch \"main\"]\n\tremote = https://joel:ghp_secret@github.com/owl/core\n\
         \tmerge = refs/heads/main\n",
    );
    let record = reading(&[src], None);
    let head = record.repositories[0].checkouts[0]
        .head
        .as_ref()
        .expect("a head");
    let follows = head
        .upstream
        .as_ref()
        .map(|upstream| upstream.branch.as_str());
    assert_eq!(
        follows,
        Some("https://github.com/owl/core/main"),
        "a branch's address said with its credential"
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
fn each_checkout_carries_its_stores_weight_judged_by_its_own_config() {
    let src = place("weight");
    // Past git's default of 27 loose objects in the folder it counts.
    let heavy = |folder: &Path| {
        let objects = folder.join(".git/objects");
        fs::create_dir_all(objects.join("pack")).expect("a pack folder");
        for at in 0..28 {
            write(&objects.join("17").join(format!("{at:038x}")), "x");
        }
    };
    checkout_at(&src.join("due"), None, 100);
    heavy(&src.join("due"));
    checkout_at(&src.join("off"), None, 100);
    heavy(&src.join("off"));
    let config = src.join("off/.git/config");
    let said = fs::read_to_string(&config).expect("its config");
    // Quoted, and with a comment after it, as a hand-written config may be.
    write(&config, &format!("{said}[gc]\n\tauto = \"0\" # off\n"));
    checkout_at(&src.join("unweighed"), None, 100);
    let record = reading(&[src], None);
    let mut weights: Vec<(&str, Option<(u64, bool)>)> = record
        .repositories
        .iter()
        .map(|repository| {
            let weight = repository.checkouts[0].weight;
            (
                repository.name.as_str(),
                weight.map(|weight| (weight.loose, weight.due)),
            )
        })
        .collect();
    weights.sort_unstable();
    assert_eq!(
        weights,
        [
            ("due", Some((28 * 256, true))),
            ("off", Some((28 * 256, false))),
            ("unweighed", None),
        ],
        "{record:#?}"
    );
}

#[test]
fn a_remote_or_branch_section_with_no_name_names_nothing() {
    let src = place("unnamed");
    checkout_at(&src.join("plain"), None, 100);
    let config = src.join("plain/.git/config");
    let said = fs::read_to_string(&config).expect("its config");
    let unnamed = "[remote]\n\turl = https://github.com/owl/core\n[branch]\n\tremote = .\n";
    write(
        &config,
        &format!("{said}{unnamed}\tmerge = refs/heads/main\n"),
    );
    let record = reading(&[src], None);
    let repository = &record.repositories[0];
    assert!(repository.remotes.is_empty(), "{record:#?}");
    assert!(repository.key.starts_with("local:"), "{record:#?}");
    let read = fs::read_to_string(&config).expect("its config");
    assert!(follows(&read).is_empty(), "no branch follows anything");
}

#[test]
fn a_value_loses_the_comment_after_it_but_not_a_mark_inside_its_quotes() {
    let config = "[remote \"origin\"]\n\turl = \"a#b;c\" # where\n\tfetch = x ; why\n";
    let values: Vec<&str> = settings(config).map(|set| set.value).collect();
    assert_eq!(values, ["a#b;c", "x"]);
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
