//! The daemon's socket, as the shell and the daemon hold it: a listener's
//! asks and what it's told, the rows' watches among them.

use super::*;
use std::sync::mpsc;

/// A folder of the test's own. Every test in the crate runs in one
/// process, so the prefix keeps these apart from other files' folders.
fn folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("eero-listeners-{name}-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("the test's folder is made");
    folder
}

/// The test's own account, which a daemon it starts answers.
fn ours() -> u32 {
    // SAFETY: getuid has no failure and touches no memory.
    unsafe { libc::getuid() }
}

/// A socket in a folder of the test's own, and what wakes the daemon.
fn listening(name: &str) -> (Listeners, mpsc::Receiver<Call>, std::path::PathBuf) {
    listening_for(name, ours())
}

/// The same, answering only account `ours`.
fn listening_for(name: &str, ours: u32) -> (Listeners, mpsc::Receiver<Call>, std::path::PathBuf) {
    let folder = folder(name);
    let path = folder.join("socket");
    let log = folder.join("changes.jsonl");
    let _ = fs::remove_file(&log);
    let (wake, woken) = mpsc::channel();
    // Logging to the test's own folder, never the account's change log.
    let listeners = Listeners::listen_within(
        &path,
        wake,
        Vocabularies::default().mapped(None),
        Some(log),
        ours,
    )
    .expect("the socket listens");
    (listeners, woken, path)
}

/// The lines of the change log of the socket at `path`.
fn logged(path: &Path) -> Vec<Value> {
    fs::read_to_string(path.with_file_name("changes.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect()
}

/// A socket that serves rows in vocabularies read from a store of the
/// test's own, holding the launcher's words for the Home Network; the
/// path of those words, as a listener names them; and a connection.
fn serving(name: &str) -> (Listeners, String, UnixStream, BufReader<UnixStream>) {
    serving_mapped(name, None)
}

/// The same, its rows offering the map drawn from net's record at
/// `record`, if any.
fn serving_mapped(
    name: &str,
    record: Option<std::path::PathBuf>,
) -> (Listeners, String, UnixStream, BufReader<UnixStream>) {
    let folder = folder(name);
    let store = folder.join("store");
    fs::create_dir_all(&store).expect("the test's store is made");
    let words = store.join("words.json");
    fs::write(&words, include_str!("../tests/words.json")).expect("the words are kept");
    let store = store.canonicalize().expect("the store is found");
    let path = folder.join("socket");
    let (wake, _woken) = mpsc::channel();
    let map = record.map(|record| Mapped {
        page: "http://sites.localhost/network/".into(),
        record,
    });
    let vocabularies = Vocabularies::within(store).mapped(map);
    let listeners = Listeners::listen_within(&path, wake, vocabularies, None, ours())
        .expect("the socket listens");
    let (stream, reader) = connected(&path);
    (listeners, words.display().to_string(), stream, reader)
}

/// The home network the rows' cases read, as the daemon sends it.
fn home() -> Value {
    serde_json::from_str::<Value>(include_str!("../home-network.json"))
        .expect("the home network's fixture is JSON")
}

fn keys(records: &Value) -> Vec<&str> {
    records
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|record| record["key"].as_str())
        .collect()
}

/// A connection, as the shell holds one, and what it reads.
fn connected(path: &Path) -> (UnixStream, BufReader<UnixStream>) {
    let stream = UnixStream::connect(path).expect("the socket takes a connection");
    let reading = stream.try_clone().expect("the connection clones");
    // A tell that never comes fails the case rather than hanging it.
    reading
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("reads can time out");
    (stream, BufReader::new(reading))
}

fn ask(stream: &mut UnixStream, said: &Value) {
    write_line(stream, &said.to_string()).expect("the ask is written");
}

fn read(reader: &mut BufReader<UnixStream>) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).expect("the daemon answers");
    serde_json::from_str(&line).expect("the answer is JSON")
}

fn record(name: &str) -> Value {
    json!({"key": KIND, "kind": KIND, "network": {"name": name}})
}

#[test]
fn a_watch_is_answered_with_the_network_and_told_every_change() {
    let (listeners, woken, path) = listening("changes");
    listeners.send(&json!({"name": "home"}));
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(
        &mut stream,
        &json!({"id": "hello", "ask": "hello", "version": 1}),
    );
    assert_eq!(
        read(&mut reader)["state"],
        "answering",
        "hello says how it stands"
    );
    ask(
        &mut stream,
        &json!({"id": "all", "ask": "watch", "kind": KIND}),
    );
    assert_eq!(
        read(&mut reader),
        json!({"id": "all", "records": [record("home")]}),
        "the watch is answered with the network whole"
    );
    listeners.send(&json!({"name": "cottage"}));

    assert_eq!(
        read(&mut reader),
        json!({"id": "all", "changed": [record("cottage")]}),
        "each change is told to the watch"
    );
    assert!(!listeners.listening(), "a watch alone isn't looking");
}

#[test]
fn only_a_watch_ahead_of_the_network_reads_it_at_the_fast_pace() {
    let (listeners, woken, path) = listening("ahead");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(
        &mut stream,
        &json!({"id": "ahead", "ask": "watch", "kind": KIND, "keys": [KIND]}),
    );
    assert_eq!(
        read(&mut reader)["records"],
        json!([]),
        "no network is read yet"
    );
    assert_eq!(woken.recv(), Ok(Call::Looked), "looking sets the pace");
    assert!(listeners.listening(), "a watch ahead is looking");
    ask(&mut stream, &json!({"id": "ahead", "ask": "unwatch"}));

    assert_eq!(
        read(&mut reader)["unwatched"],
        true,
        "the unwatch is answered"
    );
    assert_eq!(woken.recv(), Ok(Call::Looked), "stopping sets the pace");
    assert!(!listeners.listening(), "nobody looks once it's let go");
}

#[test]
fn an_ask_to_read_wakes_the_daemon_and_leaving_is_heard() {
    let (listeners, woken, path) = listening("asks");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(&mut stream, &json!({"id": "ask-1", "ask": "read"}));

    assert_eq!(
        read(&mut reader),
        json!({"id": "ask-1"}),
        "the ask is answered"
    );
    assert_eq!(
        woken.recv(),
        Ok(Call::Read),
        "the ask wakes the daemon to read"
    );
    drop((stream, reader));
    assert_eq!(woken.recv(), Ok(Call::Left), "leaving wakes the daemon");
    assert!(!listeners.listening(), "nobody looks once it's gone");
}

#[test]
fn an_act_is_answered_once_the_daemon_has_done_it() {
    let (listeners, woken, path) = listening("acts");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(
        &mut stream,
        &json!({"id": "act-1", "ask": "act", "act": "sqm", "on": true}),
    );
    let Ok(Call::Act(asked)) = woken.recv() else {
        panic!("the act wakes the daemon to do it");
    };
    assert_eq!(
        asked.act,
        Act::Conferencing(true),
        "the act is the one asked"
    );
    // The unit is read by the code under test whichever way, so only the
    // kernel's own answers are checked here; reading a unit is
    // changes.rs's to test.
    let pid = i32::try_from(std::process::id()).expect("a pid fits");
    assert_eq!(
        asked.peer.as_ref().map(|peer| (peer.pid, peer.uid)),
        // SAFETY: getuid has no failure and touches no memory.
        Some((Some(pid), unsafe { libc::getuid() })),
        "the act carries who asked, as the kernel says it"
    );
    assert!(
        listeners.waits(&asked),
        "a listener still connected isn't waiting"
    );
    listeners.answer_act(
        &asked,
        &Came::of(Err(Failure::Unreachable("eero is busy".into()))),
    );

    assert_eq!(
        read(&mut reader),
        json!({"id": "act-1", "error": {"code": "unreachable", "message": "eero is busy"}}),
        "the act is answered with what the daemon heard, coded by what mends it"
    );
    ask(
        &mut stream,
        &json!({"id": "act-2", "ask": "act", "act": "speedtest"}),
    );
    let Ok(Call::Act(asked)) = woken.recv() else {
        panic!("the act wakes the daemon to do it");
    };
    listeners.answer_act(&asked, &Came::of(Ok(Done::Made)));
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-2", "done": true}),
        "an act done is answered done"
    );
    listeners.answer_act(&asked, &Came::of(Ok(Done::Unchanged)));
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-2", "done": true, "unchanged": true}),
        "an act that already stood wasn't said unchanged, so its undo would undo another's"
    );
    listeners.answer_unlogged(&asked, &io::Error::other("the disk is full"));
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-2", "error": {"code": "unlogged", "message":
            "eero-daemon couldn't log the ask, so nothing was done: the disk is full"}}),
        "an ask the daemon couldn't log wasn't said as the daemon's own"
    );
    ask(
        &mut stream,
        &json!({"id": "act-3", "ask": "act", "act": "unreserve", "mac": "02:00:00:00:00:38"}),
    );
    let Ok(Call::Act(asked)) = woken.recv() else {
        panic!("the act wakes the daemon to do it");
    };
    let held = Some("10.0.0.208".parse().expect("an address"));
    listeners.answer_act(
        &asked,
        &Came {
            done: Ok(Done::Made),
            held,
        },
    );
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-3", "done": true, "held": "10.0.0.208"}),
        "an unreservation's answer doesn't say the address it kept"
    );
    listeners.answer_act(
        &asked,
        &Came {
            done: Err(Failure::Unknown("eero didn't say".into())),
            held,
        },
    );
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-3", "error": {"code": "unknown", "message": "eero didn't say"},
            "held": "10.0.0.208"}),
        "an unreservation no one knows the outcome of lost the address it kept"
    );
    drop((stream, reader));
    assert_eq!(woken.recv(), Ok(Call::Left), "leaving wakes the daemon");
    assert!(
        !listeners.waits(&asked),
        "a listener gone was still waiting"
    );
}

#[test]
fn only_the_daemons_own_account_is_answered() {
    // SAFETY: getuid has no failure and touches no memory.
    let ours = unsafe { libc::getuid() };
    let peer = |uid| Peer {
        pid: Some(1),
        uid,
        unit: None,
    };
    assert_eq!(
        [
            admits(Some(&peer(ours)), ours),
            admits(Some(&peer(ours.wrapping_add(1))), ours),
            admits(None, ours),
        ],
        [true, false, false],
        "another account, or one the kernel wouldn't say, was answered, or ours refused"
    );
}

#[test]
fn a_connection_from_another_account_is_told_so_and_shut_before_anything_is_read() {
    let (_listeners, woken, path) = listening_for("not-ours", ours().wrapping_add(1));
    let (mut stream, mut reader) = connected(&path);
    // The daemon may shut the socket before the ask is written, which
    // then fails; what's told back proves the refusal either way.
    let _ = write_line(
        &mut stream,
        &json!({"id": "act-1", "ask": "act", "act": "reboot"}).to_string(),
    );

    let mut first = String::new();
    reader.read_line(&mut first).expect("the daemon answers");
    assert_eq!(
        first.trim_end(),
        NOT_OURS,
        "a connection from another account wasn't told it isn't answered"
    );
    // Closed with the ask still unread, so the kernel may reset it rather
    // than end it; either way nothing more comes.
    let mut rest = String::new();
    assert!(
        matches!(
            reader.read_line(&mut rest).map_err(|e| e.kind()),
            Ok(0) | Err(ErrorKind::ConnectionReset)
        ),
        "a connection from another account was kept: {rest:?}"
    );
    assert!(
        woken.recv_timeout(Duration::from_millis(300)).is_err(),
        "a connection from another account joined"
    );
    assert!(
        logged(&path).is_empty(),
        "what another account sent was read and logged"
    );
}

#[test]
fn a_listener_let_go_has_nothing_more_it_sent_answered_logged_or_done() {
    let (_listeners, woken, path) = listening("unasked-at-once");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    // Twelve asks that are no act, then one that is, in one write, so
    // every one is read before the eighth's answer shuts the stream.
    let mut asks: Vec<String> = (1..=12)
        .map(|n| json!({"id": n, "ask": "act", "act": "reboot"}).to_string())
        .collect();
    asks.push(json!({"id": 13, "ask": "act", "act": "speedtest"}).to_string());
    asks.push(String::new());
    stream
        .write_all(asks.join("\n").as_bytes())
        .expect("the asks are written");

    let mut answers = 0;
    let mut line = String::new();
    // A stream shut with anything unread may end in a reset, not EOF.
    loop {
        match reader.read_line(&mut line).map_err(|e| e.kind()) {
            Ok(0) | Err(ErrorKind::ConnectionReset) => break,
            Ok(_) => answers += 1,
            Err(other) => panic!("the answers weren't read: {other:?}"),
        }
        line.clear();
    }
    assert_eq!(
        woken.recv_timeout(Duration::from_secs(10)),
        Ok(Call::Left),
        "an ask after the listener was let go woke the daemon, or it never left"
    );
    assert_eq!(
        (answers, logged(&path).len()),
        (8, 8),
        "asks after the eighth were answered or logged"
    );
}

#[test]
fn an_act_eero_has_none_of_is_answered_at_once() {
    let (_listeners, woken, path) = listening("bad-acts");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(
        &mut stream,
        &json!({"id": "act-1", "ask": "act", "act": "reboot"}),
    );

    assert_eq!(
        read(&mut reader)["error"],
        json!({"code": "bad-act", "message": "eero does nothing called reboot"}),
        "an act eero hasn't is refused by the daemon itself, saying back only its name"
    );
    assert!(
        woken.try_recv().is_err(),
        "an act refused doesn't wake the daemon"
    );
    // The line is written once the answer is out and the hub let go, so a
    // slow disk holds up no other listener; it's waited for, briefly.
    let log = path.with_file_name("changes.jsonl");
    let until = std::time::Instant::now() + Duration::from_secs(5);
    let line = loop {
        match fs::read_to_string(&log) {
            Ok(line) if line.ends_with('\n') => break line,
            _ if std::time::Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(10));
            }
            read => panic!("the refusal wasn't logged within 5 s: {read:?}"),
        }
    };
    let logged: Value = serde_json::from_str(&line).expect("the line is JSON");
    // SAFETY: getuid takes nothing and can't fail.
    let uid = unsafe { libc::getuid() };
    assert_eq!(
        (
            &logged["act"],
            &logged["result"],
            &logged["uid"],
            &logged["pid"]
        ),
        (
            &json!("reboot"),
            &json!("bad-act"),
            &json!(uid),
            &json!(std::process::id())
        ),
        "a refused act wasn't logged with who asked"
    );
}

#[test]
fn an_act_asked_wrongly_is_told_so_and_a_listener_asking_no_act_in_a_loop_is_let_go() {
    let (_listeners, woken, path) = listening("unasked");
    let (mut stream, mut reader) = connected(&path);
    assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

    ask(
        &mut stream,
        &json!({"id": "act-1", "ask": "act", "act": "unreserve", "mac": "a phone"}),
    );
    assert_eq!(
        read(&mut reader)["error"]["code"],
        json!("bad-ask"),
        "an act the daemon knows, asked wrongly, was said as one it doesn't know"
    );
    for n in 2..=8 {
        ask(
            &mut stream,
            &json!({"id": format!("act-{n}"), "ask": "act", "act": "reboot"}),
        );
        assert_eq!(
            read(&mut reader)["error"]["code"],
            json!("bad-act"),
            "ask {n} that was no act went unanswered"
        );
    }
    let mut rest = String::new();
    assert_eq!(
        reader.read_line(&mut rest).map_err(|e| e.kind()),
        Ok(0),
        "a listener asking no act eight times was kept: {rest:?}"
    );
}

#[test]
fn a_row_watch_is_answered_ready_and_told_only_the_rows_that_changed() {
    let (listeners, words, mut stream, mut reader) = serving("rows");
    listeners.send(&home());

    ask(
        &mut stream,
        &json!({"id": "rows", "ask": "watch", "kind": ["row", "drawer"], "place": "eero",
            "words": words}),
    );
    let answered = read(&mut reader);
    let all = keys(&answered["records"]);
    assert!(
        all.contains(&"row:eero:internet") && all.contains(&"row:eero:node:2003"),
        "the watch is answered with the place's rows, the internet's and each node's: {all:?}"
    );
    let records = answered["records"].as_array().expect("records are a list");
    assert!(
        records
            .iter()
            .filter(|record| record["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("row:")))
            .all(|record| record["kind"] == "row"),
        "each row is a row's record"
    );
    assert!(
        all.contains(&"drawer:eero:place"),
        "the place's own drawer is served beside its rows: {all:?}"
    );
    let mut hall_off = home();
    hall_off["nodes"][0]["state"] = json!("OFFLINE");
    listeners.send(&hall_off);

    let told = read(&mut reader);
    let changed = keys(&told["changed"]);
    assert_eq!(told["id"], "rows", "the change is told to the row watch");
    assert!(
        changed.contains(&"row:eero:node:2003") && changed.len() < all.len(),
        "only the rows that read differently are told: {changed:?}"
    );
}

#[test]
fn a_watch_of_some_rows_is_answered_with_those_alone() {
    let (listeners, words, mut stream, mut reader) = serving("some");
    listeners.send(&home());

    ask(
        &mut stream,
        &json!({"id": "one", "ask": "get", "kind": "row", "words": words,
            "keys": ["row:eero:internet"]}),
    );

    assert_eq!(
        keys(&read(&mut reader)["records"]),
        vec!["row:eero:internet"],
        "the rows asked for by key are answered alone"
    );
}

#[test]
fn rows_are_refused_in_words_from_outside_the_store_or_of_another_kind() {
    let (_listeners, words, mut stream, mut reader) = serving("refused");
    let outside = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json");

    ask(
        &mut stream,
        &json!({"id": 1, "ask": "list", "kind": "row", "words": outside}),
    );
    assert_eq!(
        read(&mut reader)["error"]["code"],
        "no-words",
        "words from outside the store are refused"
    );
    ask(&mut stream, &json!({"id": 2, "ask": "list", "kind": "row"}));
    assert_eq!(
        read(&mut reader)["error"]["code"],
        "no-words",
        "rows asked for in no words are refused"
    );
    ask(
        &mut stream,
        &json!({"id": 3, "ask": "list", "kind": ["row", "song"], "words": words}),
    );
    assert_eq!(
        read(&mut reader)["error"]["code"],
        "unknown-kind",
        "a kind this socket doesn't keep is refused"
    );
    ask(
        &mut stream,
        &json!({"id": 4, "ask": "list", "place": "wifi", "words": words}),
    );
    assert_eq!(
        read(&mut reader)["error"]["code"],
        "unknown-kind",
        "another place is refused"
    );
}

/// net's record written with `concerns`, a second later than any write
/// before, as net writes it each minute.
fn net_says(record: &Path, concerns: &Value, written: std::time::SystemTime) {
    fs::write(record, json!({"concerns": concerns}).to_string()).expect("net's record");
    fs::File::options()
        .write(true)
        .open(record)
        .and_then(|file| file.set_modified(written))
        .expect("its time is set");
}

#[test]
fn the_map_row_follows_net_when_the_network_stays_the_same() {
    let record = folder("follows-net").join("network.json");
    let start = std::time::SystemTime::now();
    let lost = json!([{"kind": "lost", "says": "The modem has lost 878"}]);
    net_says(&record, &lost, start);
    let (listeners, words, mut stream, mut reader) =
        serving_mapped("follows-net", Some(record.clone()));
    listeners.send(&home());
    ask(
        &mut stream,
        &json!({"id": "map", "ask": "watch", "kind": "row", "words": words,
            "keys": ["row:eero:map"]}),
    );
    assert_eq!(
        read(&mut reader)["records"][0]["row"]["state"],
        "The modem has lost 878",
        "the map row warns of what net finds"
    );

    net_says(&record, &lost, start + Duration::from_secs(1));
    assert!(
        !listeners.follow_net(),
        "net writing the same worst again changes no row"
    );
    net_says(&record, &json!([]), start + Duration::from_secs(2));
    assert!(listeners.follow_net(), "net clearing it is followed");
    let told = read(&mut reader);
    assert_eq!(
        keys(&told["changed"]),
        vec!["row:eero:map"],
        "the map row is told, while eero's network stayed the same"
    );
    assert!(
        told["changed"][0]["row"].get("state").is_none(),
        "it no longer warns: {told}"
    );
    assert!(!listeners.follow_net(), "nothing more moved");
}

/// Asks to watch the Home Network's map row in the words at `words`.
fn watch_map(stream: &mut UnixStream, words: &str) {
    ask(
        stream,
        &json!({"id": "map", "ask": "watch", "kind": "row", "words": words,
            "keys": ["row:eero:map"]}),
    );
}

/// A shell after a switch asks for its new words the minute net writes,
/// so a second vocabulary is made with net's newer worst while the first
/// keeps the old one: net's record the case writes, when it began, the
/// listeners, and each watch's connection and reader, the first's
/// watching "The modem has lost 878" and the second's net cleared.
struct HeardFirst {
    record: std::path::PathBuf,
    start: std::time::SystemTime,
    listeners: Listeners,
    reader: BufReader<UnixStream>,
    later_reader: BufReader<UnixStream>,
    _streams: (UnixStream, UnixStream),
}

fn heard_first(name: &str) -> HeardFirst {
    let record = folder(name).join("network.json");
    let start = std::time::SystemTime::now();
    net_says(&record, &lost_modem(), start);
    let (listeners, words, mut stream, mut reader) = serving_mapped(name, Some(record.clone()));
    listeners.send(&home());
    watch_map(&mut stream, &words);
    let watched = read(&mut reader);
    assert_eq!(
        watched["records"][0]["row"]["state"], "The modem has lost 878",
        "the first words are made with net warning: {watched}"
    );

    let newer = Path::new(&words).with_file_name("newer-words.json");
    fs::copy(&words, &newer).expect("the newer words are kept");
    net_says(&record, &json!([]), start + Duration::from_secs(1));
    let (mut later, mut later_reader) = connected(&folder(name).join("socket"));
    watch_map(&mut later, &newer.display().to_string());
    let first = read(&mut later_reader);
    assert_eq!(
        first["records"][0]["key"], "row:eero:map",
        "the new words make the map row: {first}"
    );
    assert!(
        first["records"][0]["row"].get("state").is_none(),
        "the new words are made with net cleared: {first}"
    );
    HeardFirst {
        record,
        start,
        listeners,
        reader,
        later_reader,
        _streams: (stream, later),
    }
}

fn lost_modem() -> Value {
    json!([{"kind": "lost", "says": "The modem has lost 878"}])
}

#[test]
fn the_map_row_follows_net_though_a_new_vocabulary_heard_it_first() {
    let HeardFirst {
        listeners,
        mut reader,
        ..
    } = heard_first("heard-first");

    assert!(
        listeners.follow_net(),
        "the rows made before are still made with the old worst"
    );
    let told = read(&mut reader);
    assert!(
        told["changed"][0]["row"].get("state").is_none(),
        "the first watch hears net cleared it: {told}"
    );
    assert!(!listeners.follow_net(), "every vocabulary is now current");
}

#[test]
fn the_map_row_follows_net_going_back_before_the_follower_looks() {
    let HeardFirst {
        record,
        start,
        listeners,
        mut later_reader,
        ..
    } = heard_first("went-back");
    net_says(&record, &lost_modem(), start + Duration::from_secs(2));

    assert!(
        listeners.follow_net(),
        "the newer words are made with a worst net has dropped"
    );
    let told = read(&mut later_reader);
    assert_eq!(
        told["changed"][0]["row"]["state"], "The modem has lost 878",
        "the second watch hears net warn again: {told}"
    );
    assert!(!listeners.follow_net(), "every vocabulary is now current");
}

#[test]
fn an_unwatched_row_watch_is_told_nothing_more() {
    let (listeners, words, mut stream, mut reader) = serving("unwatched");
    listeners.send(&home());
    ask(
        &mut stream,
        &json!({"id": "rows", "ask": "watch", "kind": "row", "words": words}),
    );
    read(&mut reader);

    ask(&mut stream, &json!({"id": "rows", "ask": "unwatch"}));
    assert_eq!(
        read(&mut reader)["unwatched"],
        true,
        "the unwatch is answered"
    );
    let mut hall_off = home();
    hall_off["nodes"][0]["state"] = json!("OFFLINE");
    listeners.send(&hall_off);
    ask(
        &mut stream,
        &json!({"id": "after", "ask": "hello", "version": 1}),
    );

    assert_eq!(
        read(&mut reader)["id"],
        "after",
        "the next line is the hello's answer, no change told before it"
    );
}
