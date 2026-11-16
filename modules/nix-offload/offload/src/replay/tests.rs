use super::*;

use std::process::Command;

/// A git tree of the test's own, whose commits it makes by name.
struct Tree {
    folder: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let tree = Self {
            folder: tempfile::tempdir().expect("no folder for a tree"),
        };
        tree.said(&["init", "-q", "-b", "main"]);
        tree
    }

    fn top(&self) -> &Path {
        self.folder.path()
    }

    /// git's answer in the tree, as one person at one time, failing the
    /// test when git does.
    fn said(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.top())
            .args(arguments)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "someone")
            .env("GIT_AUTHOR_EMAIL", "someone@example.com")
            .env("GIT_AUTHOR_DATE", "2026-11-12T10:00:00Z")
            .env("GIT_COMMITTER_NAME", "someone")
            .env("GIT_COMMITTER_EMAIL", "someone@example.com")
            .env("GIT_COMMITTER_DATE", "2026-11-12T10:00:00Z")
            .output()
            .expect("git didn't start");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {output:?}"
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// A commit writing `text` to `file`, its id.
    fn wrote(&self, file: &str, text: &str, subject: &str) -> String {
        let path = self.top().join(file);
        fs::create_dir_all(path.parent().expect("a file with no folder"))
            .expect("no folder for the file");
        fs::write(&path, text).expect("the file wasn't written");
        self.said(&["add", file]);
        self.said(&["commit", "-q", "-m", subject]);
        self.said(&["rev-parse", "HEAD"])
    }
}

/// main with a kept head, a newer one that moved `moved`, and a change on
/// the newer one touching `modules/one/a`: the kept head, the base and the
/// change's head.
fn moved_since_kept(tree: &Tree, moved: &str) -> (String, String, String) {
    tree.wrote("modules/one/a", "one\n", "start");
    let kept = tree.wrote("modules/two/b", "two\n", "kept");
    let base = tree.wrote(moved, "moved\n", "main moves");
    tree.said(&["checkout", "-q", "-b", "change"]);
    tree.wrote("modules/one/a", "one, changed\n", "change one");
    let head = tree.wrote("modules/one/c", "new\n", "add to one");
    tree.said(&["checkout", "-q", "main"]);
    (kept, base, head)
}

fn kept(heads: &[&str]) -> HashSet<String> {
    heads.iter().map(|head| (*head).to_owned()).collect()
}

#[test]
fn only_a_bases_checks_for_the_system_are_kept() {
    let base = "a".repeat(40);
    let listing = format!(
        "{base}-x86_64-linux.json\n{other}-x86_64-linux.reads.json\n\
         {other}-aarch64-darwin.json\nshort-x86_64-linux.json\n{base}-x86_64-linux.writing\n",
        other = "b".repeat(40)
    );
    assert_eq!(
        kept_in(&listing, "x86_64-linux"),
        kept(&[&base]),
        "kept more or less than the one base's checks for the system"
    );
}

#[test]
fn a_change_isnt_checked_where_main_moved_beside_it() {
    let change = ["modules/one/a".to_owned()];
    let none = Crates::default();
    for forward in [false, true] {
        assert!(
            refusal(&["flake.lock".to_owned()], &change, forward, &none).is_some(),
            "a moved lock reaches every check"
        );
        assert!(
            refusal(&["modules/one/z".to_owned()], &change, forward, &none).is_some(),
            "main moved a file in the change's own folder"
        );
        assert_eq!(
            refusal(
                &["modules/two/b".to_owned(), "README.md".to_owned()],
                &change,
                forward,
                &none
            ),
            None,
            "main moved only what the change doesn't reach"
        );
    }
}

#[test]
fn a_change_is_checked_where_main_moved_its_own_files_as_ready_takes() {
    // 49 of main's 200 merges before 18:30 on 12 November moved
    // check-descriptions.nix, which most changes touch too.
    let change = [
        "check-descriptions.nix".to_owned(),
        "modules/one/a".to_owned(),
        "modules/one/checks.nix".to_owned(),
    ];
    let none = Crates::default();
    for file in ["check-descriptions.nix", "modules/one/checks.nix"] {
        assert_eq!(
            refusal(&[file.to_owned()], &change, false, &none),
            None,
            "refused over {file}, whose lines ready reads apart"
        );
    }
    assert!(
        refusal(&["modules/one/a".to_owned()], &change, false, &none).is_some(),
        "replayed back over main moving its own code, where ready refuses the pass"
    );
    assert_eq!(
        refusal(&["modules/one/a".to_owned()], &change, true, &none),
        None,
        "refused forward over its own file, where a clean apply is enough"
    );
}

#[test]
fn a_main_head_with_no_change_on_it_is_never_replayed() {
    // On 15 November keep-main's keep of a new main head, whose base is
    // the head itself, went onto the older kept head instead, which had
    // passed already, so from 22:31 no newer main was kept at all.
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let (kept_head, base, _) = moved_since_kept(&tree, "modules/two/b");
    assert_eq!(
        replayed(
            tree.top(),
            &base,
            &base,
            "main",
            &kept(&[&kept_head]),
            scratch.path()
        ),
        Ok(None),
        "a head kept for itself was replayed onto an older kept one"
    );
}

#[test]
fn a_change_is_replayed_whole_onto_the_kept_main() {
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let (kept_head, base, head) = moved_since_kept(&tree, "modules/two/b");
    let replay = replayed(
        tree.top(),
        &base,
        &head,
        "main",
        &kept(&[&kept_head]),
        scratch.path(),
    )
    .expect("the replay was refused")
    .expect("nothing was replayed");
    assert_eq!(
        replay.onto, kept_head,
        "replayed onto another than the kept head"
    );
    let patch = |from: &str, to: &str| tree.said(&["diff", from, to]);
    assert_eq!(
        patch(&kept_head, &replay.commit),
        patch(&base, &head),
        "the replay changed what the change does"
    );
    assert_eq!(
        tree.said(&[
            "log",
            "--format=%s%n%an %aI",
            &format!("{kept_head}..{}", replay.commit)
        ]),
        tree.said(&["log", "--format=%s%n%an %aI", &format!("{base}..{head}")]),
        "the replay lost a commit's subject or who made it when"
    );
    let again = replayed(
        tree.top(),
        &base,
        &head,
        "main",
        &kept(&[&kept_head]),
        scratch.path(),
    )
    .expect("the second replay was refused")
    .expect("nothing was replayed the second time");
    assert_eq!(
        again, replay,
        "the same change replayed twice made two commits"
    );
    assert_eq!(
        replayed(
            tree.top(),
            &base,
            &head,
            "main",
            &kept(&[&base]),
            scratch.path()
        ),
        Ok(None),
        "a change whose own base is kept was replayed"
    );
    assert!(
        fs::read_dir(scratch.path())
            .expect("no scratch")
            .next()
            .is_none(),
        "a message was left in the scratch folder"
    );
}

#[test]
fn a_change_isnt_replayed_over_a_move_beside_it() {
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let (kept_head, base, head) = moved_since_kept(&tree, "modules/one/z");
    let why = replayed(
        tree.top(),
        &base,
        &head,
        "main",
        &kept(&[&kept_head]),
        scratch.path(),
    );
    assert!(
        why.as_ref().is_err_and(|why| why.contains("modules/one/z")),
        "replayed over main moving the change's folder: {why:?}"
    );
}

#[test]
fn a_change_isnt_replayed_back_over_a_crate_its_crate_reads() {
    // On 16 November a change to modules/programs, which reads
    // modules/repos, went back onto a main before 17 repos files moved,
    // and ready refused each pass of it for them, about 12 minutes a run.
    for (moved, refused) in [
        ("modules/two/src/lib.rs", true),
        ("modules/three/src/lib.rs", false),
    ] {
        let tree = Tree::new();
        let scratch = tempfile::tempdir().expect("no scratch");
        tree.wrote("modules/two/Cargo.toml", "[package]\n", "two");
        tree.wrote("modules/three/Cargo.toml", "[package]\n", "three");
        tree.wrote(
            "modules/one/Cargo.toml",
            "[package]\n\n[dependencies]\ntwo = { path = \"../two\" }\n",
            "one reads two",
        );
        let (kept_head, base, head) = moved_since_kept(&tree, moved);
        let replay = replayed(
            tree.top(),
            &base,
            &head,
            "main",
            &kept(&[&kept_head]),
            scratch.path(),
        );
        if refused {
            assert!(
                replay.as_ref().is_err_and(|why| why.contains(moved)),
                "replayed back over {moved}, in a crate the change's reads: {replay:?}"
            );
        } else {
            assert!(
                replay.as_ref().is_ok_and(Option::is_some),
                "not replayed back over {moved}, in a crate the change's never reads: \
                 {replay:?}"
            );
        }
    }
}

#[test]
fn a_change_is_replayed_forward_over_a_move_to_its_own_file() {
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let lines = "1\n2\n3\n4\n5\n6\n7\n8\n";
    let base = tree.wrote("modules/one/a", lines, "start");
    tree.said(&["checkout", "-q", "-b", "change"]);
    let head = tree.wrote("modules/one/a", &lines.replace('1', "one"), "change one");
    tree.said(&["checkout", "-q", "main"]);
    let kept_head = tree.wrote("modules/one/a", &lines.replace('8', "eight"), "main moves");
    let replay = replayed(
        tree.top(),
        &base,
        &head,
        "main",
        &kept(&[&kept_head]),
        scratch.path(),
    );
    assert!(
        replay.as_ref().is_ok_and(Option::is_some),
        "not replayed onto a newer main that moved its file elsewhere: {replay:?}"
    );
}

#[test]
fn a_change_holding_a_merge_isnt_replayed() {
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let (kept_head, base, _) = moved_since_kept(&tree, "modules/two/b");
    tree.wrote("modules/three/d", "three\n", "main moves again");
    tree.said(&["checkout", "-q", "change"]);
    tree.said(&["merge", "-q", "--no-ff", "-m", "merge main", "main"]);
    let merged = tree.said(&["rev-parse", "HEAD"]);
    let why = replayed(
        tree.top(),
        &base,
        &merged,
        "main",
        &kept(&[&kept_head]),
        scratch.path(),
    );
    assert!(
        why.as_ref().is_err_and(|why| why.contains("merge")),
        "replayed a change holding a merge: {why:?}"
    );
    let why = replayed(
        tree.top(),
        &base,
        &merged,
        "main",
        &kept(&[]),
        scratch.path(),
    );
    assert!(why.is_err(), "replayed with no main kept: {why:?}");
}

#[test]
fn a_commit_that_doesnt_apply_isnt_replayed() {
    let tree = Tree::new();
    let scratch = tempfile::tempdir().expect("no scratch");
    let start = tree.wrote("modules/one/a", "one\n", "start");
    let theirs = tree.wrote("modules/one/a", "theirs\n", "theirs");
    tree.said(&["checkout", "-q", "-b", "change", &start]);
    let ours = tree.wrote("modules/one/a", "ours\n", "ours");
    let why = applied(tree.top(), &ours, &theirs, scratch.path());
    assert!(
        why.as_ref().is_err_and(|why| why.contains("doesn't apply")),
        "a conflicting commit was replayed: {why:?}"
    );
}
