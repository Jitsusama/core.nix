//! a-call-is-on against a stand-in pw-dump, which prints the graph in
//! graph.json in the runtime folder it's given, or later.json once asked
//! before when there is one, and fails while that folder holds `fails`,
//! and against a stand-in mycelium node in an account's runtime folder,
//! which tells one other machine's record. A Mac reads no PipeWire, so
//! these are Linux's; src/lib.rs's tests say what counts as a call on a
//! Mac. What this machine's PipeWire alone says is in pipewire.rs; what
//! the nodes here say of other machines is here.
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Beside this file rather than in tests/, where cargo would take it for a
// test binary of its own.
#[path = "a-call-is-on/pipewire.rs"]
mod pipewire;

struct World {
    _dir: tempfile::TempDir,
    runtimes: PathBuf,
    stand_ins: PathBuf,
}

impl World {
    fn new() -> World {
        let dir = tempfile::tempdir().expect("a folder");
        let runtimes = dir.path().join("user");
        let stand_ins = dir.path().join("bin");
        fs::create_dir_all(&runtimes).expect("runtimes");
        fs::create_dir_all(&stand_ins).expect("stand-ins");
        let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let dump = stand_ins.join("pw-dump");
        fs::write(
            &dump,
            format!(
                "#!{shell}\nPATH='{}'\nhere=$XDG_RUNTIME_DIR\n\
                 [ -e \"$here/fails\" ] && exit 1\n\
                 if [ -e \"$here/asked\" ] && [ -e \"$here/later.json\" ]; then \
                 cat \"$here/later.json\"; else cat \"$here/graph.json\"; fi\n\
                 touch \"$here/asked\"\n",
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .expect("pw-dump");
        fs::set_permissions(&dump, fs::Permissions::from_mode(0o755)).expect("runnable");
        World {
            _dir: dir,
            runtimes,
            stand_ins,
        }
    }

    /// An account's runtime folder with PipeWire running, showing `graph`.
    fn account(&self, uid: &str, graph: &str) -> PathBuf {
        let folder = self.runtimes.join(uid);
        fs::create_dir_all(&folder).expect("a runtime folder");
        fs::write(folder.join("pipewire-0"), "").expect("a socket");
        fs::write(folder.join("graph.json"), graph).expect("a graph");
        folder
    }

    /// What a-call-is-on exits with, and how long it took.
    fn asked(&self) -> (i32, Duration) {
        let started = Instant::now();
        let told = self.told(&[self.runtimes.to_str().expect("a folder as text")]);
        (told.status.code().expect("an exit code"), started.elapsed())
    }

    /// All a-call-is-on says when asked with `args`.
    fn told(&self, args: &[&str]) -> Output {
        self.told_within(args, Duration::from_secs(60))
            .expect("a-call-is-on answered within a minute")
    }

    /// All a-call-is-on says when asked with `args`, or none when it
    /// hasn't answered `within` and is stopped.
    fn told_within(&self, args: &[&str], within: Duration) -> Option<Output> {
        let path = format!(
            "{}:{}",
            self.stand_ins.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut asking = Command::new(env!("CARGO_BIN_EXE_a-call-is-on"))
            .args(args)
            .env("PATH", path)
            .env_remove("JOURNAL_STREAM")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("a-call-is-on ran");
        // Read as it writes, so a long answer can't fill a pipe and hold it
        // till it's stopped, which would read as too slow.
        let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
            thread::spawn(move || {
                let mut said = Vec::new();
                if let Some(mut pipe) = pipe {
                    let _ = pipe.read_to_end(&mut said);
                }
                said
            })
        };
        let stdout = drain(asking.stdout.take().map(|pipe| Box::new(pipe) as _));
        let stderr = drain(asking.stderr.take().map(|pipe| Box::new(pipe) as _));
        let started = Instant::now();
        let status = loop {
            if let Some(status) = asking.try_wait().expect("its state") {
                break status;
            }
            if started.elapsed() > within {
                let _ = asking.kill();
                let _ = asking.wait();
                return None;
            }
            thread::sleep(Duration::from_millis(20));
        };
        Some(Output {
            status,
            stdout: stdout.join().expect("its output"),
            stderr: stderr.join().expect("what it said beside"),
        })
    }

    /// The runtime folders, as a-call-is-on is given them.
    fn runtimes(&self) -> &str {
        self.runtimes.to_str().expect("a folder as text")
    }
}

fn recording(id: u64, binary: &str) -> String {
    format!(
        r#"[{{"id": {id}, "type": "PipeWire:Interface:Node", "info": {{"state": "running",
        "props": {{"media.class": "Stream/Input/Audio", "application.process.binary": "{binary}"}}}}}}]"#
    )
}

/// A stand-in mycelium node for the account `uid`, serving `realm`: it
/// answers each `Machines` request with a list holding `peer`, as a node
/// lists what it heard from another machine, for as long as the test runs.
fn node(world: &World, uid: &str, realm: &str, peer: &str) {
    let folder = world.runtimes.join(uid).join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    serve(&folder.join(format!("{realm}.sock")), peer);
}

/// A stand-in node's socket at `socket`, listing `peer`.
fn serve(socket: &std::path::Path, peer: &str) {
    let listener = UnixListener::bind(socket).expect("a socket");
    let answer = format!("{{\"Ok\":{{\"Machines\":[{peer}]}}}}\n");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut asked = String::new();
            let _ = BufReader::new(&stream).read_line(&mut asked);
            if asked.trim() == "\"Machines\"" {
                let _ = (&stream).write_all(answer.as_bytes());
            }
        }
    });
}

/// A peer's record, measured `ago` seconds since, or ahead when it's
/// less than none, declaring the call probe when `declares_call`, its
/// probes saying `reached`.
fn peer(ago: i64, declares_call: bool, reached: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let at = i64::try_from(now).expect("a time") - ago;
    let measured = Command::new("date")
        .args(["-u", "-d", &format!("@{at}"), "+%Y-%m-%dT%H:%M:%S.5Z"])
        .output()
        .expect("date ran");
    let measured = String::from_utf8(measured.stdout).expect("a time");
    let probes = if declares_call {
        r#"{"call":{"run":["a-call-is-on","--here"]}}"#
    } else {
        r#"{"home":{"run":["probe"]}}"#
    };
    format!(
        r#"{{"name":"skiff","measured_at":"{}","declared":{{"probes":{probes}}},"reached":{reached}}}"#,
        measured.trim()
    )
}

#[test]
fn a_call_on_another_machine_is_a_call_without_waiting_on_pipewire() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let (code, _) = world.asked();
    assert_eq!(code, 0, "a call on a peer wasn't a call");
    assert!(
        !world.runtimes.join("1000").join("asked").exists(),
        "it asked PipeWire though a peer had a call"
    );
}

#[test]
fn a_node_that_wont_answer_hides_no_call_another_heard() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    drop(UnixListener::bind(folder.join("a-dead.sock")).expect("a socket"));
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "a dead node hid a call: {told:?}"
    );
}

#[test]
fn a_socket_linked_from_elsewhere_is_never_asked() {
    let world = World::new();
    world.account("1000", "[]");
    let elsewhere = world
        .runtimes
        .parent()
        .expect("a parent")
        .join("elsewhere.sock");
    serve(&elsewhere, &peer(10, true, r#"{"call":true}"#));
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    std::os::unix::fs::symlink(&elsewhere, folder.join("work.sock")).expect("a link");
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a linked socket was asked: {told:?}"
    );
}

#[test]
fn asked_about_here_alone_a_call_elsewhere_is_none() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let runtimes = world.runtimes.to_str().expect("a folder as text");
    let told = world.told(&["--here", runtimes]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "here alone heard a peer: {told:?}"
    );
    // Here alone asks no node, so one that won't answer costs nothing.
    let world = World::new();
    world.account("1000", "[]");
    misbehaving(&world, "personal", Vec::new());
    let told = world
        .told_within(&["--here", world.runtimes()], Duration::from_secs(4))
        .expect("here alone waited on a node");
    assert!(
        told.status.code() == Some(1) && told.stderr.is_empty(),
        "here alone asked a node: {told:?}"
    );
    let world = World::new();
    world.account("1000", &recording(40, "zoom"));
    let told = world.told(&["--here", world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "here alone missed its own: {told:?}"
    );
}

#[test]
fn a_peer_saying_no_call_and_a_call_gone_stale_are_no_call() {
    for (record, why) in [
        (peer(10, true, r#"{"call":false}"#), "a fresh no"),
        (
            peer(240, true, r#"{"call":true}"#),
            "a yes four minutes old",
        ),
        (peer(240, true, "{}"), "a stale record with no reading"),
    ] {
        let world = World::new();
        world.account("1000", "[]");
        node(&world, "1000", "personal", &record);
        let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
        assert_eq!(
            told.status.code(),
            Some(1),
            "{why} wasn't no call: {told:?}"
        );
        assert!(told.stderr.is_empty(), "{why} said something: {told:?}");
    }
}

#[test]
fn this_machines_own_record_is_no_peers_call() {
    // mycelium names a machine by its hostname, as the kernel holds it.
    let here = fs::read_to_string("/proc/sys/kernel/hostname").expect("a hostname");
    let world = World::new();
    world.account("1000", "[]");
    let own =
        peer(10, true, r#"{"call":true}"#).replace("\"skiff\"", &format!("{:?}", here.trim()));
    node(&world, "1000", "work", &own);
    let told = world.told(&[world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "this machine's own record was heard as a peer's call: {told:?}"
    );
}

#[test]
fn a_peer_whose_probe_said_nothing_is_named_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, "{}"));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "no reading was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("skiff's call probe said nothing lately"),
        "it didn't name the peer: {told:?}"
    );
}

#[test]
fn a_peer_declaring_no_call_probe_is_named_unseen_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, false, r#"{"home":true}"#));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "an unseen peer was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("skiff declares no call probe"),
        "it didn't name the unseen peer: {told:?}"
    );
}

#[test]
fn a_node_that_wont_answer_is_named_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    // A socket left by a node that has stopped: nothing listens on it.
    drop(UnixListener::bind(folder.join("personal.sock")).expect("a socket"));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a silent node was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("so its peers' calls go unheard"),
        "it didn't name the silent node: {told:?}"
    );
}

/// A stand-in node for the account 1000 in `realm` that takes each request
/// and answers it with `says`, then holds the connection open for half a
/// minute.
fn misbehaving(world: &World, realm: &str, says: Vec<u8>) {
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    let listener = UnixListener::bind(folder.join(format!("{realm}.sock"))).expect("a socket");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let says = says.clone();
            thread::spawn(move || {
                let mut asked = String::new();
                let _ = BufReader::new(&stream).read_line(&mut asked);
                let _ = (&stream).write_all(&says);
                thread::sleep(Duration::from_secs(30));
            });
        }
    });
}

#[test]
fn a_node_that_never_answers_is_named_once_its_time_is_up() {
    let world = World::new();
    world.account("1000", "[]");
    misbehaving(&world, "work", Vec::new());
    let told = world
        .told_within(&[world.runtimes()], Duration::from_secs(9))
        .expect("a node that never answered held it past a shaper proof's ten seconds");
    assert_eq!(
        told.status.code(),
        Some(1),
        "a mute node was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("answered nothing within"),
        "it didn't say the node ran out of time: {told:?}"
    );
}

#[test]
fn a_mute_node_and_pipewire_looking_twice_still_answer_inside_a_proofs_time() {
    // The slowest a-call-is-on can be: a node that never answers, no call
    // elsewhere, and a recording here that PipeWire looks at again two
    // seconds on, which has stopped by then. A shaper proof gives it ten.
    let world = World::new();
    let folder = world.account("1000", &recording(40, "zoom"));
    fs::write(folder.join("later.json"), "[]").expect("a later graph");
    misbehaving(&world, "work", Vec::new());
    let started = Instant::now();
    let told = world
        .told_within(&[world.runtimes()], Duration::from_secs(9))
        .expect("a mute node and PipeWire's second look ran past a shaper proof's ten seconds");
    let took = started.elapsed();
    assert_eq!(
        told.status.code(),
        Some(1),
        "a recording gone two seconds on was a call: {told:?}"
    );
    // Both slow paths were taken: the node's whole five seconds, then
    // PipeWire's look two seconds on.
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("answered nothing within 5s"),
        "the mute node wasn't waited on: {told:?}"
    );
    assert!(
        took >= Duration::from_secs(7),
        "it answered in {took:?}, too soon to have waited on both"
    );
}

#[test]
fn machines_past_the_most_are_said_to_go_unnamed() {
    let world = World::new();
    world.account("1000", "[]");
    let skiff = peer(10, false, "{}");
    let fleet: Vec<String> = (0..70)
        .map(|n| skiff.replace("\"skiff\"", &format!("\"m{n:02}\"")))
        .collect();
    node(&world, "1000", "work", &fleet.join(","));
    let told = world.told(&[world.runtimes()]);
    assert_eq!(told.status.code(), Some(1), "no call was one: {told:?}");
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("so 6 machines past them go unnamed"),
        "the records past the most weren't said to go unnamed: {told:?}"
    );
}

#[test]
fn a_flag_is_read_wherever_it_stands() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let told = world.told(&[world.runtimes(), "--here"]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "--here after the folder heard a peer: {told:?}"
    );
    let told = world.told(&[world.runtimes(), "--hear"]);
    assert_eq!(
        told.status.code(),
        Some(64),
        "a mistyped flag after the folder wasn't refused: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("knows no --hear"),
        "the refusal doesn't name the mistyped flag: {told:?}"
    );
    let told = world.told(&["--", world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "a folder after -- wasn't read as one: {told:?}"
    );
    // Past --, --here is a second folder, not a flag asking about here.
    let told = world.told(&["--", world.runtimes(), "--here"]);
    assert_eq!(
        told.status.code(),
        Some(64),
        "a flag after -- was read as one: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("one folder at most"),
        "--here after -- wasn't read as a folder: {told:?}"
    );
}

#[test]
fn a_node_folder_crowded_past_its_most_entries_is_named() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    for n in 0..300 {
        fs::write(folder.join(format!("left-{n}")), "").expect("an entry");
    }
    let told = world.told(&[world.runtimes()]);
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("holds more than 256 entries"),
        "a crowded node folder wasn't named: {told:?}"
    );
}

#[test]
fn a_node_that_drips_is_named_once_its_time_is_up() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    let listener = UnixListener::bind(folder.join("work.sock")).expect("a socket");
    // A byte every half second, never a whole line: a time given to each
    // read rather than to the whole answer would wait on it for ever.
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let mut asked = String::new();
                let _ = BufReader::new(&stream).read_line(&mut asked);
                for _ in 0..60 {
                    if (&stream).write_all(b"[").is_err() {
                        return;
                    }
                    thread::sleep(Duration::from_millis(500));
                }
            });
        }
    });
    let told = world
        .told_within(&[world.runtimes()], Duration::from_secs(9))
        .expect("a dripping node held it past a shaper proof's ten seconds");
    assert_eq!(
        told.status.code(),
        Some(1),
        "a dripping node was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("only part of a line"),
        "it didn't say the node answered only part of a line: {told:?}"
    );
}

#[test]
fn under_json_stderr_holds_no_caution_in_words() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, false, "{}"));
    let told = world.told(&["--json", world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "an unseen peer was a wait: {told:?}"
    );
    let stderr = String::from_utf8_lossy(&told.stderr);
    assert!(
        stderr
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()),
        "stderr held words under --json: {told:?}"
    );
    let told = world.told(&[world.runtimes()]);
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("declares no call probe"),
        "without --json, the unseen peer wasn't named: {told:?}"
    );
}

#[test]
fn a_node_folder_linked_from_elsewhere_is_never_asked() {
    let world = World::new();
    world.account("1000", "[]");
    let elsewhere = world.runtimes.parent().expect("a parent").join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("a folder elsewhere");
    serve(
        &elsewhere.join("work.sock"),
        &peer(10, true, r#"{"call":true}"#),
    );
    std::os::unix::fs::symlink(&elsewhere, world.runtimes.join("1000").join("mycelium"))
        .expect("a link");
    let told = world.told(&[world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a linked node folder was asked: {told:?}"
    );
}

#[test]
fn a_node_answering_without_end_is_named_past_its_limit() {
    let world = World::new();
    world.account("1000", "[]");
    // Past the most read by half again, and under what an earlier most let
    // through.
    misbehaving(&world, "work", vec![b'a'; 3 << 19]);
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a flooding node was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("answered more than"),
        "it didn't say the node answered too much: {told:?}"
    );
}

#[test]
fn the_nodes_are_asked_at_once_so_a_mute_one_starves_none() {
    let world = World::new();
    world.account("1000", "[]");
    // Asked one after another, each with its own time, two mute nodes
    // would take twice the time one does, past the ten seconds a shaper
    // proof gives the whole question.
    misbehaving(&world, "home", Vec::new());
    misbehaving(&world, "personal", Vec::new());
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let told = world
        .told_within(&[world.runtimes()], Duration::from_secs(8))
        .expect("the mute nodes held it past a shaper proof's time");
    assert_eq!(
        told.status.code(),
        Some(0),
        "a call heard after a mute node went unheard: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("personal.sock"),
        "it didn't name the mute node: {told:?}"
    );
}

#[test]
fn a_node_whose_queue_is_full_is_named_at_once() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    let socket = folder.join("work.sock");
    let listener = UnixListener::bind(&socket).expect("a socket");
    // A node that takes no connection, its queue holding one already.
    rustix::net::listen(&listener, 0).expect("a queue of none");
    let _queued = std::os::unix::net::UnixStream::connect(&socket).expect("one queued");
    let told = world
        .told_within(&[world.runtimes()], Duration::from_secs(5))
        .expect("a node with a full queue held it");
    assert_eq!(
        told.status.code(),
        Some(1),
        "a full node was a wait: {told:?}"
    );
    let stderr = String::from_utf8_lossy(&told.stderr);
    assert!(
        stderr.contains("work.sock") && stderr.contains("its queue is full"),
        "it didn't name the full node and why: {told:?}"
    );
}

#[test]
fn an_account_offering_too_many_nodes_has_its_first_asked() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    node(&world, "1000", "a", &peer(10, true, r#"{"call":true}"#));
    for realm in 0..20 {
        drop(UnixListener::bind(folder.join(format!("z{realm:02}.sock"))).expect("a socket"));
    }
    let told = world.told(&[world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "its first node's call went unheard: {told:?}"
    );
    let said = String::from_utf8_lossy(&told.stderr);
    assert!(
        said.contains("offers 21 nodes, more than 16, and the last 5 aren't asked"),
        "it didn't say how many went unasked: {told:?}"
    );
    assert!(
        said.contains("z14.sock") && !said.contains("z15.sock"),
        "it didn't ask exactly its first 16: {told:?}"
    );
}

#[test]
fn a_node_slowed_past_a_few_seconds_is_still_heard() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    let listener = UnixListener::bind(folder.join("work.sock")).expect("a socket");
    let answer = format!(
        "{{\"Ok\":{{\"Machines\":[{}]}}}}\n",
        peer(10, true, r#"{"call":true}"#)
    );
    // A node slowed by a build, answering after three and a half seconds.
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut asked = String::new();
            let _ = BufReader::new(&stream).read_line(&mut asked);
            thread::sleep(Duration::from_millis(3_500));
            let _ = (&stream).write_all(answer.as_bytes());
        }
    });
    let told = world.told(&[world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "a slow node's call went unheard: {told:?}"
    );
}

#[test]
fn a_peer_dated_ahead_is_named_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    node(
        &world,
        "1000",
        "work",
        &peer(-600, true, r#"{"call":true}"#),
    );
    let told = world.told(&[world.runtimes()]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "an undated yes was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("skiff's record is dated ahead"),
        "it didn't name the undated peer: {told:?}"
    );
}

#[test]
fn a_yes_is_a_call_whatever_the_record_declares() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, false, r#"{"call":true}"#));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "a yes from a record declaring no probe wasn't a call: {told:?}"
    );
}
