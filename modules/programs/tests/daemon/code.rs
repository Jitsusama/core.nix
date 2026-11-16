//! Code's rows over the socket: every repository under the roots, each
//! once, in a first watch's own answer; a repository cloned since told to
//! a later watch, read behind it; the rows searched by owner; the open
//! changes a refresh kept, and how far a counting kept its checkout
//! behind, said beside their repository; a checkout's files served
//! inside Open a File by Name while it's ahead, gone once it isn't, and
//! found there by name or folder as they're typed; New Terminal Here
//! started by a stand-in for niri as it's asked; and Pull bringing a
//! checkout forward by fast-forward alone, saying why where it won't.

use std::path::Path;
use std::process::Command;

use daemon_client::until;
use serde_json::{Value, json};

use super::Running;

fn git_in(folder: &Path, args: &[&str]) {
    std::fs::create_dir_all(folder).unwrap();
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .status();
    assert!(ran.unwrap().success(), "git {args:?} failed");
}

/// A checkout under the case's src with a remote on GitHub.
fn cloned(running: &Running, folder: &str, remote: &str) {
    let path = running.path(&format!("src/{folder}"));
    git_in(&path, &["init", "--quiet"]);
    git_in(&path, &["remote", "add", "origin", remote]);
}

/// A checkout's path as the daemon names it, with no link left in it,
/// since a test's temporary folder may sit behind one.
fn real(path: &Path) -> std::path::PathBuf {
    path.canonicalize().unwrap()
}

fn row_of<'a>(answer: &'a Value, key: &str) -> Option<&'a Value> {
    answer["records"]
        .as_array()?
        .iter()
        .find(|record| record["key"] == key)
}

#[test]
fn every_repository_is_a_row_of_codes_in_a_first_watchs_answer() {
    let running = Running::new("code");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    cloned(&running, "again/alpha", "https://github.com/someone/alpha");
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = row_of(&answer, "row:code:github:someone/alpha")
        .unwrap_or_else(|| panic!("a first watch isn't answered with Code's rows: {answer}"));
    assert_eq!(alpha["row"]["name"], "alpha");
    assert_eq!(
        alpha["row"]["acts"][0],
        json!({"key": "verb:code:page", "open": "https://github.com/someone/alpha"}),
        "a repository's Return doesn't open its page: {alpha}"
    );
    let rows = answer["records"]
        .as_array()
        .unwrap()
        .iter()
        // Its Open a File by Name row is inside it, not one of Code's own.
        .filter(|record| record["kind"] == "row" && record["row"]["parent"].is_null())
        .count();
    assert_eq!(
        rows, 1,
        "two checkouts of one repository aren't one row: {answer}"
    );

    cloned(&running, "beta", "git@github.com:other/beta.git");
    // The read behind the watch may end before the watch's answer is made,
    // which then holds it, or after, which tells it.
    let mut later = running.client();
    let answer = later.ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "code"}));
    let beta = row_of(&answer, "row:code:github:other/beta")
        .cloned()
        .unwrap_or_else(|| later.changed("row:code:github:other/beta"));
    assert_eq!(
        beta["row"]["name"], "beta",
        "a repository cloned since isn't told: {beta}"
    );

    let said = later.ask(&json!({"id": 3, "ask": "search", "place": "code", "words": "other"}));
    assert_eq!(
        said["keys"],
        json!(["row:code:github:other/beta"]),
        "a repository isn't found by its owner: {said}"
    );
}

#[test]
fn the_changes_a_refresh_kept_are_counted_beside_their_repository() {
    let running = Running::new("code-changes");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    let change = |title: &str| {
        json!({"title": title, "author": "someone", "draft": false, "branch": title,
            "base": "main", "commit": null, "updated_at": 30,
            "checks": {"state": "passing", "passed": 1, "failed": [], "pending": []},
            "review": "waiting"})
    };
    let kept = running.path("state/repos/github.json");
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    let seen = json!({"schema": "repos.github/1", "read_at": 20, "unread": [],
        "changes": {"github:someone/alpha": [change("one"), change("two")]}});
    std::fs::write(&kept, seen.to_string()).unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = row_of(&answer, "row:code:github:someone/alpha")
        .unwrap_or_else(|| panic!("a first watch isn't answered with Code's rows: {answer}"));
    assert_eq!(
        alpha["row"]["side"][0],
        json!({"said": "2 changes"}),
        "the changes a refresh kept aren't counted beside their repository: {alpha}"
    );
    let acts = alpha["row"]["acts"].as_array().unwrap();
    assert!(
        acts.contains(
            &json!({"key": "verb:code:changes", "open": "https://github.com/someone/alpha/pulls"})
        ),
        "a repository with open changes doesn't offer their list: {alpha}"
    );
}

#[test]
fn a_checkouts_files_are_served_inside_open_a_file_by_name_only_while_its_ahead() {
    let running = Running::new("code-files");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    let checkout = real(&running.path("src/alpha"));
    std::fs::create_dir_all(checkout.join("docs")).unwrap();
    std::fs::write(checkout.join("docs/guide.md"), "a guide").unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let find = "row:code:github:someone/alpha:files";
    let row = row_of(&answer, find)
        .unwrap_or_else(|| panic!("a checkout has no Open a File by Name inside it: {answer}"));
    assert_eq!(row["row"]["parent"], "row:code:github:someone/alpha");
    let guide = format!("{find}:{}/docs/guide.md", checkout.display());
    assert!(
        row_of(&answer, &guide).is_none(),
        "a checkout's files were read before Open a File by Name was ahead: {answer}"
    );
    client.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [find]}));
    let file = client.changed(&guide);
    assert_eq!(
        file["row"]["name"], "guide.md",
        "a file ahead isn't served by its name: {file}"
    );
    assert_eq!(
        file["row"]["parent"], find,
        "a file isn't inside the row: {file}"
    );
    let said = client.ask(&json!({"id": 2, "ask": "search", "place": "code", "words": "guide"}));
    assert_eq!(
        said["keys"],
        json!([]),
        "what's typed at Code found a row inside a repository: {said}"
    );
    client.send(&json!({"id": "ahead", "ask": "unwatch"}));
    assert!(
        client.gone().contains(&json!(guide)),
        "a file's row stayed once Open a File by Name wasn't ahead"
    );
}

/// Git run in `folder` as none of Joel's config says, in the case's own
/// home, answering what it printed.
fn git_alone(home: &Path, folder: &Path, args: &[&str]) -> String {
    std::fs::create_dir_all(folder).unwrap();
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "someone")
        .env("GIT_AUTHOR_EMAIL", "someone@example.com")
        .env("GIT_COMMITTER_NAME", "someone")
        .env("GIT_COMMITTER_EMAIL", "someone@example.com")
        .output()
        .unwrap();
    assert!(ran.status.success(), "git {args:?} failed");
    String::from_utf8(ran.stdout).unwrap().trim().to_owned()
}

#[test]
fn pull_takes_what_the_branch_followed_has_and_never_merges() {
    let running = Running::new("code-pull");
    let home = running.path("");
    let (upstream, elsewhere, here) = (
        running.path("up.git"),
        running.path("elsewhere"),
        running.path("src/alpha"),
    );
    let git = |folder: &Path, args: &[&str]| git_alone(&home, folder, args);
    let commit = |folder: &Path, file: &str| {
        std::fs::write(folder.join(file), file).unwrap();
        git(folder, &["add", file]);
        git(folder, &["commit", "--quiet", "-m", file]);
    };
    git(&upstream, &["init", "--quiet", "--bare", "-b", "main"]);
    git(&elsewhere, &["init", "--quiet", "-b", "main"]);
    git(
        &elsewhere,
        &["remote", "add", "origin", upstream.to_str().unwrap()],
    );
    commit(&elsewhere, "one");
    git(&elsewhere, &["push", "--quiet", "-u", "origin", "main"]);
    git(
        &home,
        &[
            "clone",
            "--quiet",
            upstream.to_str().unwrap(),
            here.to_str().unwrap(),
        ],
    );
    commit(&elsewhere, "two");
    git(&elsewhere, &["push", "--quiet"]);
    let head = |folder: &Path| git(folder, &["rev-parse", "HEAD"]);
    // Fetched already, and counted one behind, as a refresh would say.
    git(&here, &["fetch", "--quiet"]);
    let kept = running.path("state/repos/upstreams.json");
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    let (two, one) = (head(&elsewhere), head(&here));
    let counted = json!({"schema": "repos.upstreams/1", "counted_at": 40,
        "counts": {format!("{two}...{one}"): {"behind": 1, "ahead": 0}}});
    std::fs::write(&kept, counted.to_string()).unwrap();
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
        .unwrap_or_else(|| panic!("the checkout pulled has no row: {answer}"));
    assert_eq!(
        alpha["row"]["side"][0],
        json!({"said": "1 behind"}),
        "{alpha}"
    );
    let key = alpha["key"].as_str().unwrap().to_owned();

    let mut client = running.client();
    let mut pull =
        |id: u64, folder: &Path| client.ask(&json!({"id": id, "ask": "code:pull", "path": folder}));
    // A file git doesn't track is no change of its own, and stays.
    std::fs::write(here.join("loose"), "loose").unwrap();
    let said = pull(1, &here);
    assert_eq!(
        said["said"], "pulled 1 commit",
        "Pull didn't fetch and take: {said}"
    );
    assert_eq!(head(&here), head(&elsewhere));
    assert!(
        here.join("loose").is_file(),
        "a file git doesn't track went"
    );
    let pulled = watcher.changed(&key);
    assert!(
        !pulled["row"]["side"].to_string().contains("behind"),
        "the row still says it's behind once pulled: {pulled}"
    );
    let said = pull(2, &here);
    assert_eq!(said["said"], "already up to date", "{said}");

    commit(&elsewhere, "three");
    git(&elsewhere, &["push", "--quiet"]);
    std::fs::write(here.join("one"), "changed").unwrap();
    // With nowhere to fetch from, a refusal made after fetching would say
    // the fetch failed.
    let gone = running.path("gone.git");
    git(
        &here,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let was = head(&here);
    let said = pull(3, &here);
    assert_eq!(
        (&said["error"]["refused"], &said["error"]["said"]),
        (
            &json!("changed"),
            &json!("not pulled: it has changes of its own")
        ),
        "a checkout with changes of its own was pulled, or fetched first: {said}"
    );
    assert_eq!(head(&here), was);
    git(
        &here,
        &["remote", "set-url", "origin", upstream.to_str().unwrap()],
    );

    git(&here, &["checkout", "--quiet", "--", "one"]);
    git(&here, &["checkout", "--quiet", "--detach"]);
    let said = pull(6, &here);
    assert_eq!(said["error"]["refused"], "detached", "{said}");
    assert_eq!(head(&here), was);
    git(&here, &["checkout", "--quiet", "main"]);

    // A folder holding a `.git` that's no repository is refused, rather
    // than git finding the checkout it sits in.
    let inside = here.join("inside");
    std::fs::create_dir_all(inside.join(".git")).unwrap();
    let said = pull(7, &inside);
    assert_eq!(said["error"]["refused"], "tree", "{said}");
    assert_eq!(head(&here), was);
    std::fs::remove_dir_all(&inside).unwrap();

    commit(&here, "mine");
    let was = head(&here);
    let said = pull(4, &here);
    assert_eq!(said["error"]["refused"], "diverged", "{said}");
    assert_eq!(
        head(&here),
        was,
        "a branch with commits of its own was merged or rebased"
    );

    let tree = running.path("tree");
    git(
        &here,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "work",
            tree.to_str().unwrap(),
        ],
    );
    git(
        &tree,
        &["branch", "--quiet", "--set-upstream-to", "origin/main"],
    );
    git(&tree, &["reset", "--quiet", "--hard", "origin/main~1"]);
    let was = head(&tree);
    let said = pull(5, &tree);
    assert_eq!(
        said["error"]["refused"], "tree",
        "a tree cut from a checkout was pulled: {said}"
    );
    assert_eq!(head(&tree), was);
}

#[test]
fn how_far_a_counting_kept_a_checkout_behind_is_said_beside_its_repository() {
    let running = Running::new("code-behind");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    // Its branch and the one it follows, at commits named by hand, since
    // a reading takes them from the refs alone and never opens a commit.
    let git = running.path("src/alpha/.git");
    let (head, upstream) = ("a".repeat(40), "b".repeat(40));
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git.join("refs/heads/main"), format!("{head}\n")).unwrap();
    std::fs::create_dir_all(git.join("refs/remotes/origin")).unwrap();
    std::fs::write(
        git.join("refs/remotes/origin/main"),
        format!("{upstream}\n"),
    )
    .unwrap();
    git_in(
        &running.path("src/alpha"),
        &["config", "branch.main.remote", "origin"],
    );
    git_in(
        &running.path("src/alpha"),
        &["config", "branch.main.merge", "refs/heads/main"],
    );
    let kept = running.path("state/repos/upstreams.json");
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    let counted = json!({"schema": "repos.upstreams/1", "counted_at": 40,
        "counts": {format!("{upstream}...{head}"): {"behind": 3, "ahead": 0}}});
    std::fs::write(&kept, counted.to_string()).unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let alpha = row_of(&answer, "row:code:github:someone/alpha")
        .unwrap_or_else(|| panic!("a first watch isn't answered with Code's rows: {answer}"));
    assert_eq!(
        alpha["row"]["side"][0],
        json!({"said": "3 behind"}),
        "how far a counting kept its checkout behind isn't said beside it: {alpha}"
    );
}

#[test]
fn whats_typed_in_open_a_file_by_name_finds_its_files_by_name_or_folder() {
    let running = Running::new("code-file-search");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    let checkout = real(&running.path("src/alpha"));
    std::fs::create_dir_all(checkout.join("docs")).unwrap();
    std::fs::write(checkout.join("docs/guide.md"), "a guide").unwrap();
    std::fs::write(checkout.join("readme.md"), "a readme").unwrap();
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let find = "row:code:github:someone/alpha:files";
    client.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [find]}));
    let guide = format!("{find}:{}/docs/guide.md", checkout.display());
    client.changed(&guide);
    let mut typed = |words: &str, most: Option<u64>| {
        client.ask(
            &json!({"id": 2, "ask": "search", "place": "code", "words": words,
            "within": find, "most": most}),
        )
    };
    let said = typed("guide", None);
    assert_eq!(
        said["keys"],
        json!([guide]),
        "what's typed in Open a File by Name didn't find a file by its name: {said}"
    );
    let said = typed("docs", None);
    assert_eq!(
        said["keys"],
        json!([guide]),
        "what's typed in Open a File by Name didn't find a file by its folder: {said}"
    );
    let said = typed("md", Some(1));
    assert_eq!(
        said["keys"].as_array().map(Vec::len),
        Some(1),
        "a search typed in a step found more than it was asked for: {said}"
    );
}

#[test]
fn new_terminal_here_has_niri_start_kitty_in_the_checkout() {
    let running = Running::new("code-terminal");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "code:terminal", "path": "/src/alpha"}));
    assert!(
        said.get("error").is_none(),
        "New Terminal Here was refused: {said}"
    );
    let asked = || std::fs::read_to_string(running.path("niri.asked"));
    until("niri wasn't asked to start a terminal", || asked().is_ok());
    assert_eq!(
        asked().unwrap(),
        "msg action spawn -- kitty --directory /src/alpha\n",
        "niri wasn't asked to start kitty in the checkout, so it outlives the daemon"
    );
}
