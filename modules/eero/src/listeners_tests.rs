//! The daemon's socket, as the shell and the daemon hold it: a listener's
//! asks and what it's told, the rows' watches among them.

use super::*;
use std::sync::mpsc;

/// A folder of the test's own.
fn folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("eero-{name}-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("the test's folder is made");
    folder
}

/// A socket in a folder of the test's own, and what wakes the daemon.
fn listening(name: &str) -> (Listeners, mpsc::Receiver<Call>, std::path::PathBuf) {
    let path = folder(name).join("socket");
    let (wake, woken) = mpsc::channel();
    let listeners = Listeners::listen(&path, wake, None).expect("the socket listens");
    (listeners, woken, path)
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
    let listeners = Listeners::listen_within(&path, wake, Vocabularies::within(store).mapped(map))
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
    listeners.answer_act(&asked, Err("eero is busy".into()));

    assert_eq!(
        read(&mut reader),
        json!({"id": "act-1", "error": {"code": "refused", "message": "eero is busy"}}),
        "the act is answered with what the daemon heard"
    );
    ask(
        &mut stream,
        &json!({"id": "act-2", "ask": "act", "act": "speedtest"}),
    );
    let Ok(Call::Act(asked)) = woken.recv() else {
        panic!("the act wakes the daemon to do it");
    };
    listeners.answer_act(&asked, Ok(()));
    assert_eq!(
        read(&mut reader),
        json!({"id": "act-2", "done": true}),
        "an act done is answered done"
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
        read(&mut reader)["error"]["code"],
        "bad-act",
        "an act eero hasn't is refused by the daemon itself"
    );
    assert!(
        woken.try_recv().is_err(),
        "an act refused doesn't wake the daemon"
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
