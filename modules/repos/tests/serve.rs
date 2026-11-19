//! `repos serve` as its unit runs it, over a checkout laid out by hand as
//! git lays it out: a watch is answered with the checkout as the disk
//! holds it, and told when its branch moves, as git moves one, and when a
//! tree is cut from it; and Code's homes are written at its first reading
//! and again as a tree is cut, none when none are named, and a writing that
//! fails is warned of, and said over, as it serves on. The daemon hears the
//! disk through inotify, so this is Linux's alone.
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
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
/// folder taken away with it, and what it's said to its journal so far.
struct Serving {
    child: Child,
    run: PathBuf,
    said: Arc<Mutex<String>>,
}

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.run);
    }
}

/// The daemon, looking under `at` and working in it, writing Code's homes
/// to `homes` as the local sites tell its unit to, and with none named
/// otherwise, as a machine without them runs it; serving in a runtime
/// folder of its own, named for `at` as `place` names it, so no two tests
/// share one. What it says is heard, and passed on to the test's own.
/// That folder is named short, and not past its link, since a socket's
/// path stops at 108 bytes. It's made afresh, readable by this account
/// alone, so nothing else sharing the temporary folder reaches the socket
/// or puts one of its own there first.
fn serve(at: &Path, homes: Option<&Path>) -> (Serving, PathBuf) {
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_repos"))
        .arg("serve")
        .env_clear()
        .current_dir(at)
        .env("HOME", at)
        .env("XDG_RUNTIME_DIR", &run)
        .env("REPOS_ROOTS", at.join("src"))
        // None named empty, which the daemon reads as it reads none set.
        .env(
            "REPOS_HOMES",
            homes.map(Path::as_os_str).unwrap_or_default(),
        )
        .stderr(Stdio::piped())
        .spawn()
        .expect("repos serve started");
    let said = Arc::new(Mutex::new(String::new()));
    let heard = Arc::clone(&said);
    let journal = child.stderr.take().expect("its journal");
    std::thread::spawn(move || {
        for line in BufReader::new(journal).lines().map_while(Result::ok) {
            eprintln!("{line}");
            let mut said = heard.lock().expect("what it said");
            said.push_str(&line);
            said.push('\n');
        }
    });
    let socket = run.join("daemons/repos.sock");
    (Serving { child, run, said }, socket)
}

/// What the daemon has said, once it's said `wanted`.
fn said_until(serving: &Serving, wanted: &str) -> String {
    let began = Instant::now();
    loop {
        let said = serving.said.lock().expect("what it said").clone();
        if said.contains(wanted) {
            return said;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "it never said {wanted:?}; it said {said:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A checkout of core.nix under `at`, on main at `FIRST`, laid out as git
/// lays one out; answers its git folder.
fn checkout(at: &Path) -> PathBuf {
    let git = at.join("src/core.nix/.git");
    written(
        &git.join("config"),
        "[remote \"origin\"]\n\turl = git@github.com:Jitsusama/core.nix.git\n",
    );
    written(&git.join("HEAD"), "ref: refs/heads/main\n");
    written(&git.join("refs/heads/main"), &format!("{FIRST}\n"));
    git
}

/// A tree cut from the checkout whose git folder is `git`, as `git worktree
/// add` cuts one: its entry under worktrees/, then the tree's folder with
/// its .git file; answers the tree.
fn cut(at: &Path, git: &Path) -> PathBuf {
    let tree = at.join("trees/say-trees");
    let own = git.join("worktrees/say-trees");
    written(&own.join("commondir"), "../..\n");
    written(
        &own.join("gitdir"),
        &format!("{}\n", tree.join(".git").display()),
    );
    written(&tree.join(".git"), &format!("gitdir: {}\n", own.display()));
    written(&own.join("HEAD"), "ref: refs/heads/main\n");
    tree
}

/// Where the daemon under `at` writes Code's homes: two folders down, in
/// none it was given, as the local sites' documents may be when it starts.
fn homes(at: &Path) -> PathBuf {
    at.join("documents/code/homes.json")
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

/// Code's homes as the daemon last wrote them to `file`, once they hold
/// what `wanted` looks for.
fn homes_until(file: &Path, wanted: impl Fn(&[Value]) -> bool) -> Vec<Value> {
    let began = Instant::now();
    loop {
        let things = fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|homes| homes["things"].as_array().cloned())
            .unwrap_or_default();
        if wanted(&things) {
            return things;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "the homes never came; they hold {things:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Every file under `folder`, followed past no link, that holds homes as
/// the daemon writes them, whatever it's named.
fn homes_under(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).expect("the folder read") {
        let path = entry.expect("an entry").path();
        let kind = fs::symlink_metadata(&path).expect("its kind").file_type();
        if kind.is_dir() {
            found.extend(homes_under(&path));
        } else if kind.is_file()
            && fs::read(&path)
                .is_ok_and(|bytes| String::from_utf8_lossy(&bytes).contains("code/#repo="))
        {
            found.push(path);
        }
    }
    found
}

/// Whether `things` hold one keyed `key`.
fn holding(things: &[Value], key: &str) -> bool {
    things.iter().any(|thing| thing["key"] == key)
}

#[test]
fn the_homes_are_written_at_the_first_reading_and_again_as_a_tree_is_cut() {
    let at = place("homes");
    let git = checkout(&at);
    let homes = homes(&at);
    let (serving, _socket) = serve(&at, Some(&homes));
    homes_until(&homes, |things| {
        holding(things, "github:Jitsusama/core.nix")
    });

    // Cut well inside the pace, so only a reading of what was heard writes it.
    let path = cut(&at, &git).display().to_string();
    let things = homes_until(&homes, |things| holding(things, &path));
    assert!(
        holding(&things, "github:Jitsusama/core.nix"),
        "the repository left the homes as the tree came"
    );
    // Stopped first, or a writing of its homes could land as the place goes.
    drop(serving);
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_watch_is_told_as_a_branch_moves_and_a_tree_is_cut() {
    let at = place("watched");
    let git = checkout(&at);
    let (serving, socket) = serve(&at, None);

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

    let key = format!("tree:{}", cut(&at, &git).display());
    let told = told_until(&mut lines, |said| {
        said["changed"]
            .as_array()
            .is_some_and(|changed| changed.iter().any(|one| one["key"] == key.as_str()))
    });
    let tree = told["changed"]
        .as_array()
        .and_then(|changed| changed.iter().find(|one| one["key"] == key.as_str()))
        .expect("the tree told");
    assert_eq!(
        tree["checkout"],
        at.join("src/core.nix").display().to_string()
    );
    assert_eq!(tree["present"], true);
    // Told none, as a machine without the local sites runs it, it writes none
    // in the home it's given or the folder it works in, where a default of its
    // own would go, nor in its runtime folder, and says nothing of them.
    let written: Vec<_> = homes_under(&at)
        .into_iter()
        .chain(homes_under(&serving.run))
        .collect();
    assert!(
        written.is_empty(),
        "homes were written with none named, to {written:?}"
    );
    let said = serving.said.lock().expect("what it said").clone();
    assert!(
        !said.contains("Code's homes"),
        "it spoke of homes with none named: {said:?}"
    );
    drop(serving);
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_writing_that_fails_is_warned_of_and_said_over_as_the_daemon_serves_on() {
    let at = place("failing");
    let git = checkout(&at);
    let homes = homes(&at);
    // A file where the homes' folder goes, so no writing can make it.
    let documents = at.join("documents");
    fs::write(&documents, "in the way").expect("a file in the folder's way");
    let (serving, socket) = serve(&at, Some(&homes));
    let warning = format!(
        "<4>repos: couldn't write Code's homes to {}",
        homes.display()
    );
    said_until(&serving, &warning);
    let said = said_until(&serving, "repos: ready in ");
    // Said ready first, since views are answered before the homes are written,
    // so writing them never holds up the first.
    let ready = said.find("repos: ready in ").expect("said ready");
    assert!(
        ready < said.find(&warning).expect("said the warning"),
        "it wrote the homes before it was ready: {said:?}"
    );

    // Still serving: a watch is answered with the checkout.
    let stream = connect(&socket);
    let mut lines = BufReader::new(stream.try_clone().expect("a reader"));
    let mut writing = stream;
    let ask = json!({"id": 1, "ask": "watch", "kind": ["checkout"]});
    writeln!(writing, "{ask}").expect("asked");
    let first = told_until(&mut lines, |said| said.get("records").is_some());
    assert_eq!(
        first["records"][0]["repository"],
        "github:Jitsusama/core.nix"
    );

    // With the way cleared, the next reading writes them and says so.
    fs::remove_file(&documents).expect("the way cleared");
    let path = cut(&at, &git).display().to_string();
    homes_until(&homes, |things| holding(things, &path));
    said_until(
        &serving,
        &format!("repos: Code's homes at {} are right again", homes.display()),
    );
    drop(serving);
    fs::remove_dir_all(&at).expect("cleaned");
}
