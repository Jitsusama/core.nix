//! `repos serve` as its unit runs it, over a checkout laid out by hand as
//! git lays it out: a watch is answered with the checkout as the disk
//! holds it, and told when its branch moves, as git moves one, and when a
//! tree is cut from it. The daemon hears the disk through inotify, so this
//! is Linux's alone.
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const FIRST: &str = "1111111111111111111111111111111111111111";
const MOVED: &str = "2222222222222222222222222222222222222222";

/// A folder of the test's own, emptied first, as a reading says it:
/// past any link, as a temporary folder often lies behind one.
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-serve-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    fs::canonicalize(&dir).expect("its path")
}

/// As git writes a file, a ref's included: its lock written, then renamed
/// over it, so the daemon hears each file as it hears git's.
fn written(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("its folder");
    let lock = path.with_extension("lock");
    fs::write(&lock, text).expect("its lock written");
    fs::rename(&lock, path).expect("renamed into place");
}

/// The daemon, stopped when the test ends however it ends, its runtime
/// folder taken away with it.
struct Serving {
    child: Child,
    run: PathBuf,
}

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.run);
    }
}

/// The daemon, looking under `at`, serving in a runtime folder of its
/// own, named for `at` as `place` names it, so no two tests share one.
/// That folder is named short, and not past its link, since a socket's
/// path stops at 108 bytes. It's made afresh, readable by this account
/// alone, so nothing else sharing the temporary folder reaches the socket
/// or puts one of its own there first.
fn serve(at: &Path) -> (Serving, PathBuf) {
    let name = at
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.rsplit('-').next())
        .expect("a place of the test's own");
    let run = std::env::temp_dir().join(format!("repos-run-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&run);
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&run)
        .expect("a runtime folder of its own");
    let child = Command::new(env!("CARGO_BIN_EXE_repos"))
        .arg("serve")
        .env_clear()
        .env("HOME", at)
        .env("XDG_RUNTIME_DIR", &run)
        .env("REPOS_ROOTS", at.join("src"))
        .spawn()
        .expect("repos serve started");
    let socket = run.join("daemons/repos.sock");
    (Serving { child, run }, socket)
}

/// A connection to the socket, once the daemon listens on it.
fn connect(socket: &Path) -> UnixStream {
    let began = Instant::now();
    loop {
        if let Ok(stream) = UnixStream::connect(socket) {
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .expect("a timeout");
            return stream;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "it never listened"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Lines from the daemon until one says what `wanted` looks for.
fn told_until(lines: &mut impl BufRead, wanted: impl Fn(&Value) -> bool) -> Value {
    let mut line = String::new();
    loop {
        line.clear();
        let read = lines
            .read_line(&mut line)
            .expect("a line within its timeout");
        assert!(read > 0, "the daemon hung up");
        let said: Value = serde_json::from_str(&line).expect("a JSON line");
        if wanted(&said) {
            return said;
        }
    }
}

#[test]
fn a_watch_is_told_as_a_branch_moves_and_a_tree_is_cut() {
    let at = place("watched");
    let git = at.join("src/core.nix/.git");
    written(
        &git.join("config"),
        "[remote \"origin\"]\n\turl = git@github.com:Jitsusama/core.nix.git\n",
    );
    written(&git.join("HEAD"), "ref: refs/heads/main\n");
    written(&git.join("refs/heads/main"), &format!("{FIRST}\n"));
    let (_serving, socket) = serve(&at);

    let stream = connect(&socket);
    let mut lines = BufReader::new(stream.try_clone().expect("a reader"));
    let mut writing = stream;
    let ask = json!({"id": 1, "ask": "watch", "kind": ["checkout", "tree"]});
    writeln!(writing, "{ask}").expect("asked");
    let first = told_until(&mut lines, |said| said.get("records").is_some());
    let checkout = format!("checkout:{}", at.join("src/core.nix").display());
    assert_eq!(first["records"][0]["key"], checkout.as_str());
    assert_eq!(first["records"][0]["head"]["commit"], FIRST);
    assert_eq!(
        first["records"][0]["repository"],
        "github:Jitsusama/core.nix"
    );

    written(&git.join("refs/heads/main"), &format!("{MOVED}\n"));
    let told = told_until(&mut lines, |said| said.get("changed").is_some());
    assert_eq!(told["changed"][0]["key"], checkout.as_str());
    assert_eq!(told["changed"][0]["head"]["commit"], MOVED);

    // As `git worktree add` cuts one: its entry under worktrees/, then the
    // tree's folder with its .git file.
    let tree = at.join("trees/say-trees");
    let own = git.join("worktrees/say-trees");
    written(&own.join("commondir"), "../..\n");
    written(
        &own.join("gitdir"),
        &format!("{}\n", tree.join(".git").display()),
    );
    written(&tree.join(".git"), &format!("gitdir: {}\n", own.display()));
    written(&own.join("HEAD"), "ref: refs/heads/main\n");
    let key = format!("tree:{}", tree.display());
    let told = told_until(&mut lines, |said| {
        said["changed"]
            .as_array()
            .is_some_and(|changed| changed.iter().any(|one| one["key"] == key.as_str()))
    });
    let cut = told["changed"]
        .as_array()
        .and_then(|changed| changed.iter().find(|one| one["key"] == key.as_str()))
        .expect("the tree told");
    assert_eq!(
        cut["checkout"],
        at.join("src/core.nix").display().to_string()
    );
    assert_eq!(cut["present"], true);
    fs::remove_dir_all(&at).expect("cleaned");
}
