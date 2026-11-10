//! The daemon itself, over a folder of to-dos and a stand-in machine: the
//! verdicts it writes as checks pass, files come, a boot is newer than a
//! to-do, and a to-do leaves its file; a check that hangs holding up no
//! other; a done to-do staying done through a failing check and a
//! restart; a check run only for an owner outside every sandbox; a
//! check no longer run leaving the daemon idle; a
//! check finding what the session's PATH holds; and over its socket, each
//! to-do, look and word of Joel's told to a watch as it changes.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use daemon_client::{Client, Daemon};
use serde_json::{Value, json};

struct Running {
    child: Child,
    verdicts: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts the daemon over `dir`, whose `root/proc/stat` says the boot,
/// asking no user manager for a session's environment.
fn start(dir: &Path) -> Running {
    start_with(dir, Path::new("none"))
}

/// Starts the daemon over `dir`, asking `systemctl` for a session's
/// environment.
fn start_with(dir: &Path, systemctl: &Path) -> Running {
    let child = command(dir, systemctl).spawn().unwrap();
    Running {
        child,
        verdicts: dir.join("verdicts.json"),
    }
}

/// The daemon over `dir`, whose `root/proc/stat` says the boot.
fn command(dir: &Path, systemctl: &Path) -> Command {
    let root = dir.join("root");
    fs::create_dir_all(root.join("proc")).unwrap();
    fs::write(root.join("proc/stat"), "cpu 1\nbtime 1794200000\n").unwrap();
    fs::create_dir_all(dir.join("waiting")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_to-do"));
    command
        .arg(dir.join("waiting"))
        .arg(dir.join("verdicts.json"))
        .arg(dir.join("signins"))
        .arg(dir.join("core.git"))
        .args(["--follower", "none", "--root"])
        .arg(&root)
        .arg("--systemctl")
        .arg(systemctl);
    command
}

/// The daemon over `dir`, serving its socket, with the harness's looks
/// and the shell's word of Joel's beside it.
fn served(dir: PathBuf) -> Daemon {
    let socket = dir.join("daemons/to-do.sock");
    let mut command = command(&dir, Path::new("none"));
    command
        .arg("--socket")
        .arg(&socket)
        .arg("--looks")
        .arg(dir.join("looks"))
        .arg("--settled")
        .arg(dir.join("settled.json"));
    Daemon::start(&mut command, socket, dir)
}

/// Writes a file whole, under another name, then renames it, as the
/// harness and the shell write theirs.
fn written(path: &Path, text: &str) {
    let writing = path.with_extension("writing");
    fs::write(&writing, text).unwrap();
    fs::rename(writing, path).unwrap();
}

/// What a watch holds once `wanted` holds of it, taking in each line the
/// daemon says: its records, those that changed and the keys gone.
fn told(
    client: &mut Client,
    held: &mut BTreeMap<String, Value>,
    wanted: impl Fn(&BTreeMap<String, Value>) -> bool,
) {
    while !wanted(held) {
        take(held, &client.heard());
    }
}

/// Takes in one line a watch is told: its records, those that changed and
/// the keys gone.
fn take(held: &mut BTreeMap<String, Value>, said: &Value) {
    for record in said["records"]
        .as_array()
        .or_else(|| said["changed"].as_array())
        .into_iter()
        .flatten()
    {
        held.insert(record["key"].as_str().unwrap().to_owned(), record.clone());
    }
    for key in said["gone"].as_array().into_iter().flatten() {
        held.remove(key.as_str().unwrap());
    }
}

/// Writes `quest`'s file of `items`, each owned by a session outside
/// every sandbox, as `mesh_wait` says one, unless it names its own owner.
fn to_dos(dir: &Path, quest: &str, items: &str) {
    let mut items: Vec<Value> = serde_json::from_str(&format!("[{items}]")).unwrap();
    for item in &mut items {
        if item.get("owner").is_none() {
            item["owner"] = json!({"sandbox": "none"});
        }
    }
    let file = dir.join("waiting").join(format!("{quest}.json"));
    written(
        &file,
        &json!({"v": 2, "quest": quest, "items": items}).to_string(),
    );
}

/// The shell on PATH, which runs a stand-in's script.
fn shell() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|folder| folder.join("sh"))
        .find(|sh| sh.is_file())
        .expect("PATH holds sh")
}

/// Makes `path` a program anyone may run, made runnable as it's made, so
/// nothing finds it there and not yet runnable.
fn runnable(path: &Path, text: &str) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o755)
        .open(path)
        .unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// The verdicts once `wanted` holds of them, within five seconds.
fn until(running: &Running, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let read = fs::read_to_string(&running.verdicts)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        if let Some(read) = read.as_ref().filter(|r| wanted(r)) {
            return read.clone();
        }
        assert!(Instant::now() < deadline, "never so: {read:?}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn state<'a>(verdicts: &'a Value, key: &str) -> Option<&'a str> {
    verdicts["items"][key]["state"].as_str()
}

#[test]
fn a_passing_check_is_done_and_a_failing_one_waits_with_its_figures() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-1",
        r#"{"id":"yes","title":"Y","doneWhen":{"check":"true"}},
           {"id":"no","title":"N","doneWhen":{"check":"echo not yet; exit 1"}}"#,
    );
    let found = until(&running, |v| {
        state(v, "QEST-1:yes").is_some() && state(v, "QEST-1:no").is_some()
    });
    assert_eq!(state(&found, "QEST-1:yes"), Some("done"));
    assert_eq!(state(&found, "QEST-1:no"), Some("waiting"));
    let how = &found["items"]["QEST-1:no"]["how"];
    assert_eq!(how["by"], "check");
    assert_eq!(how["exit"], 1);
    assert_eq!(how["said"], "not yet");
    assert_eq!(how["ran"], 1);
}

#[test]
fn a_file_waited_on_is_done_as_it_comes() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    let wanted = dir.path().join("deep/er/here");
    to_dos(
        dir.path(),
        "QEST-2",
        &format!(
            r#"{{"id":"f","title":"F","doneWhen":{{"signal":"file:{}"}}}}"#,
            wanted.display()
        ),
    );
    until(&running, |v| state(v, "QEST-2:f") == Some("waiting"));
    fs::create_dir_all(wanted.parent().unwrap()).unwrap();
    fs::write(&wanted, "").unwrap();
    until(&running, |v| state(v, "QEST-2:f") == Some("done"));
}

#[test]
fn a_boot_since_it_was_raised_makes_it_ready() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-3",
        r#"{"id":"old","title":"O","raisedAt":"2026-11-09T04:00:00Z",
            "doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}},
           {"id":"new","title":"N","raisedAt":"2026-11-09T05:00:00Z",
            "doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-3:new").is_some());
    assert_eq!(state(&found, "QEST-3:old"), Some("ready"));
    assert_eq!(state(&found, "QEST-3:new"), Some("waiting"));
    assert_eq!(
        found["items"]["QEST-3:new"]["how"]["boot_at"],
        "2026-11-09T04:53:20Z"
    );
}

#[test]
fn only_joels_word_has_no_verdict_and_a_to_do_gone_takes_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-4",
        r#"{"id":"his","title":"H","doneWhen":{"by":"person"}},
           {"id":"c","title":"C","doneWhen":{"check":"false"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-4:c").is_some());
    assert_eq!(state(&found, "QEST-4:his"), None);
    to_dos(
        dir.path(),
        "QEST-4",
        r#"{"id":"his","title":"H","doneWhen":{"by":"person"}}"#,
    );
    until(&running, |v| {
        v["items"]
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    });
}

#[test]
fn a_check_that_hangs_holds_up_no_other() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    // The hanging check stands for one that never answers, and is stopped
    // as the daemon is.
    to_dos(
        dir.path(),
        "QEST-5",
        r#"{"id":"hangs","title":"H","doneWhen":{"check":"sleep 20"}},
           {"id":"quick","title":"Q","doneWhen":{"check":"true"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-5:quick").is_some());
    assert_eq!(state(&found, "QEST-5:quick"), Some("done"));
    assert_eq!(state(&found, "QEST-5:hangs"), None);
}

#[test]
fn a_slow_fetch_of_cores_history_holds_up_no_check() {
    let dir = tempfile::tempdir().unwrap();
    // The stand-in follower's journal says the machine runs a core the
    // clone hasn't got. The stand-in git notes a clone or a fetch was
    // asked, then takes longer than the test waits, as a slow network
    // would; reading a clone it hasn't got finds nothing at once.
    let journalctl = dir.path().join("journalctl");
    runnable(
        &journalctl,
        &format!(
            "#!{}\necho '5 x: {{\"step\":\"unchanged\",\"followed\":{{\"core\":\"ccc\"}}}}'\n\
             exec sleep 30\n",
            shell().display()
        ),
    );
    let asked = dir.path().join("git-asked");
    let git = dir.path().join("git");
    runnable(
        &git,
        &format!(
            "#!{}\ncase \"$*\" in\n*clone*|*fetch*) touch {}; exec sleep 15 ;;\nesac\nexit 1\n",
            shell().display(),
            asked.display()
        ),
    );
    let mut command = command(dir.path(), Path::new("none"));
    command
        .arg("--follower")
        .arg("update-to-main-follows")
        .arg("--journalctl")
        .arg(&journalctl)
        .arg("--git")
        .arg(&git);
    let running = Running {
        child: command.spawn().unwrap(),
        verdicts: dir.path().join("verdicts.json"),
    };
    to_dos(
        dir.path(),
        "QEST-6",
        r#"{"id":"switched","title":"S","doneWhen":{"signal":"switch:ddd"}}"#,
    );
    daemon_client::until("the daemon asks git for core's history", || asked.exists());
    to_dos(
        dir.path(),
        "QEST-7",
        r#"{"id":"quick","title":"Q","doneWhen":{"check":"true"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-7:quick").is_some());
    assert_eq!(
        state(&found, "QEST-7:quick"),
        Some("done"),
        "a check is judged while the fetch runs: {found}"
    );
}

#[test]
fn a_done_to_do_stays_done_though_its_check_fails_after_and_across_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let once = dir.path().join("once");
    fs::write(&once, "").unwrap();
    // It passes the first time it runs and fails every time after.
    let done = format!(
        r#"{{"id":"once","title":"O","doneWhen":{{"check":"rm {}"}}}}"#,
        once.display()
    );
    // Each later to-do's check takes a second, so a check run again
    // beside it would be heard first.
    let later =
        |id: &str| format!(r#"{{"id":"{id}","title":"L","doneWhen":{{"check":"sleep 1"}}}}"#);
    let running = start(dir.path());
    to_dos(dir.path(), "QEST-6", &done);
    until(&running, |v| state(v, "QEST-6:once") == Some("done"));
    // A sign-in is a signal, which would run every check again.
    fs::create_dir_all(dir.path().join("signins")).unwrap();
    fs::write(dir.path().join("signins/stand-in.json"), "{}").unwrap();
    to_dos(dir.path(), "QEST-6", &format!("{done},{}", later("after")));
    let found = until(&running, |v| state(v, "QEST-6:after").is_some());
    assert_eq!(state(&found, "QEST-6:once"), Some("done"));
    drop(running);
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-6",
        &format!("{done},{},{}", later("after"), later("restarted")),
    );
    let found = until(&running, |v| state(v, "QEST-6:restarted").is_some());
    assert_eq!(state(&found, "QEST-6:once"), Some("done"));
}

#[test]
fn a_check_runs_only_for_an_owner_said_to_be_outside_every_sandbox() {
    let dir = tempfile::tempdir().unwrap();
    let ran = dir.path().join("ran");
    let running = start(dir.path());
    let check = |id: &str, owner: &str| {
        format!(
            r#"{{"id":"{id}","title":"T","owner":{owner},
                "doneWhen":{{"check":"touch {}"}}}}"#,
            ran.display()
        )
    };
    let open = r#"{"id":"open","title":"O","doneWhen":{"check":"true"}}"#;
    to_dos(
        dir.path(),
        "QEST-7",
        &format!(
            "{},{},{open}",
            check("strict", r#"{"sandbox":"strict"}"#),
            check("unsaid", r#"{"address":"ABC123"}"#)
        ),
    );
    // A refusal is said as its to-do is first judged, and the open one is
    // done only once its check has run, so by then the others are said.
    let found = until(&running, |v| state(v, "QEST-7:open") == Some("done"));
    let refused = |id: &str| found["items"][format!("QEST-7:{id}")]["how"]["refused"].clone();
    assert_eq!(refused("strict"), "strict sandbox");
    assert_eq!(refused("unsaid"), "sandbox unsaid");
    assert_eq!(state(&found, "QEST-7:strict"), Some("waiting"));
    assert!(!ran.exists(), "a refused check never ran");
}

/// The processor time `pid` has taken, user and system, in clock ticks.
fn ticks(pid: u32) -> u64 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    // Its name is in brackets and may hold spaces, so the fields are
    // counted from after it: utime and stime are the 14th and 15th.
    let (_, after) = stat.rsplit_once(')').unwrap();
    let fields: Vec<&str> = after.split_whitespace().collect();
    fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap()
}

#[test]
fn a_check_no_longer_run_leaves_the_daemon_idle() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    let item = |ready: bool| {
        format!(
            r#"{{"id":"ready","title":"R","ready":{ready},"doneWhen":{{"by":"person"}},
                "readyWhen":{{"check":"exit 1"}}}}"#
        )
    };
    to_dos(dir.path(), "QEST-8", &item(false));
    until(&running, |v| v["items"]["QEST-8:ready"]["how"]["ran"] == 1);
    // Its owner says it's ready, so its check is never run again, and a
    // sign-in hurries every check there is.
    to_dos(dir.path(), "QEST-8", &item(true));
    until(&running, |v| v["items"].get("QEST-8:ready").is_none());
    fs::write(dir.path().join("signins/github"), "signed in").unwrap();
    // The daemon hears the sign-in and hurries its checks, the stranded
    // one included, which is the moment it would begin spinning.
    thread::sleep(Duration::from_millis(200));
    let pid = running.child.id();
    let before = ticks(pid);
    // The second its processor time is measured over: idle takes none.
    thread::sleep(Duration::from_secs(1));
    let took = ticks(pid) - before;
    // A daemon waiting on nothing takes none of a second; one waking for
    // a check it never runs takes all of it.
    assert!(took < 20, "the daemon took {took} ticks of a second's 100");
}

#[test]
fn a_check_finds_what_the_sessions_path_holds_now() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("session-bin");
    fs::create_dir_all(&session).unwrap();
    runnable(&session.join("only-in-session"), "");
    // The stand-in user manager says a session's PATH, as the desktop
    // imports it after the daemon has started.
    let systemctl = dir.path().join("systemctl");
    runnable(
        &systemctl,
        &format!(
            "#!{}\necho \"PATH={}:$PATH\"\n",
            shell().display(),
            session.display()
        ),
    );
    let running = start_with(dir.path(), &systemctl);
    to_dos(
        dir.path(),
        "QEST-8",
        r#"{"id":"found","title":"F","doneWhen":{"check":"command -v only-in-session"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-8:found").is_some());
    assert_eq!(state(&found, "QEST-8:found"), Some("done"), "{found}");
}

#[test]
fn a_watch_is_told_each_to_do_look_and_word_of_joels_as_it_changes() {
    let dir = tempfile::tempdir().unwrap().keep();
    fs::create_dir_all(dir.join("waiting")).unwrap();
    to_dos(
        &dir,
        "QEST-9",
        r#"{"id":"now","title":"N","ready":true,"doneWhen":{"by":"person"}},
           {"id":"check","title":"C","doneWhen":{"check":"true"}}"#,
    );
    let daemon = served(dir);
    let mut client = daemon.client();
    let hello = client.ask(&json!({"id": 0, "ask": "hello"}));
    assert_eq!(
        (&hello["domain"], &hello["version"]),
        (&json!("to-do"), &json!(1)),
        "hello says whose it is: {hello}"
    );
    let mut held = BTreeMap::new();
    take(&mut held, &client.ask(&json!({"id": 1, "ask": "watch"})));
    let state =
        |held: &BTreeMap<String, Value>, key: &str| held.get(key).map(|r| r["state"].clone());
    told(&mut client, &mut held, |held| {
        state(held, "to-do:QEST-9:check") == Some(json!("done"))
    });
    assert_eq!(
        state(&held, "to-do:QEST-9:now"),
        Some(json!("now")),
        "a ready to-do is for now: {held:?}"
    );
    assert_eq!(
        held["to-do:QEST-9:check"]["verdict"]["state"], "done",
        "a to-do carries what the daemon found"
    );
    assert_eq!(
        held["waiting:summary"]["to_dos"],
        json!({"now": 1, "later": 0, "done": 1}),
        "the count of to-dos"
    );
    let look = json!({"v": 1, "id": "a1", "kind": "pr", "title": "Fix", "state": "waiting"});
    written(&daemon.folder().join("looks/a1.json"), &look.to_string());
    told(&mut client, &mut held, |held| held.contains_key("look:a1"));
    assert_eq!(held["look:a1"]["look"], look, "a look is told whole");
    told(&mut client, &mut held, |held| {
        held["waiting:summary"]["looks"]["waiting"] == 1
    });
    written(
        &daemon.folder().join("settled.json"),
        r#"{"done":{"QEST-9:now":1794000000000},"notNow":{}}"#,
    );
    told(&mut client, &mut held, |held| {
        state(held, "to-do:QEST-9:now") == Some(json!("done"))
    });
    written(&daemon.folder().join("settled.json"), r#"{"done":"#);
    let standing = loop {
        let line = client.heard();
        take(&mut held, &line);
        if line["sources"]["settled"]["state"] == "failing" {
            break line;
        }
    };
    assert_eq!(
        standing["sources"]["waiting"]["state"], "answering",
        "a watch is told how every source stands: {standing}"
    );
    assert_eq!(
        state(&held, "to-do:QEST-9:now"),
        Some(json!("done")),
        "Joel's word that can't be read keeps the last he said"
    );
    to_dos(
        daemon.folder(),
        "QEST-9",
        r#"{"id":"now","title":"N","ready":true,"doneWhen":{"by":"person"}}"#,
    );
    told(&mut client, &mut held, |held| {
        !held.contains_key("to-do:QEST-9:check")
    });
    let listed = client.ask(&json!({"id": 2, "ask": "list", "kind": "look"}));
    assert_eq!(
        listed["records"],
        json!([{"key": "look:a1", "kind": "look", "look": look}]),
        "a list of one kind"
    );
    let refused = client.ask(&json!({"id": 3, "ask": "list", "kind": "person"}));
    assert_eq!(
        refused["error"]["code"], "unknown-kind",
        "a kind kept elsewhere is refused"
    );
}
