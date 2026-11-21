//! The reader as its clients see it, over a socket of its own: what a
//! folder holds, then each change, one follower however many ask, and
//! letting go of the clients that go or fall behind.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use pi_mesh_reader::reader::{self, ROOM, Reader};
use serde_json::{Value, json};

/// How often the reader lists a folder whole in these tests.
const BACKSTOP: Duration = Duration::from_millis(200);

/// How long a test waits for a line it expects.
const PATIENCE: Duration = Duration::from_secs(10);

struct Here {
    root: PathBuf,
    socket: PathBuf,
    reader: Arc<Reader>,
}

fn here(name: &str) -> Here {
    let root = std::env::temp_dir().join(format!("pmr-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("presence")).unwrap();
    let socket = root.join("reader.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let reader = Reader::new(vec![root.clone()], BACKSTOP);
    let serving = Arc::clone(&reader);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let reader = Arc::clone(&serving);
            thread::spawn(move || reader::serve(&reader, &stream));
        }
    });
    Here {
        root,
        socket,
        reader,
    }
}

struct Client {
    lines: BufReader<UnixStream>,
}

impl Client {
    fn asking(socket: &Path, folder: &Path) -> Self {
        let mut stream = UnixStream::connect(socket).unwrap();
        writeln!(stream, "{}", json!({ "folder": folder, "suffix": ".json" })).unwrap();
        stream.set_read_timeout(Some(PATIENCE)).unwrap();
        Client {
            lines: BufReader::new(stream),
        }
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.lines
            .read_line(&mut line)
            .expect("a line came in time");
        serde_json::from_str(&line).unwrap_or(Value::Null)
    }

    /// The lines up to and with `ready`.
    fn listing(&mut self) -> Vec<Value> {
        let mut lines = Vec::new();
        loop {
            let line = self.next();
            if line == json!({ "ready": true }) {
                return lines;
            }
            lines.push(line);
        }
    }
}

fn put(name: &str, text: &str) -> Value {
    json!({ "put": name, "text": text })
}

fn gone(name: &str) -> Value {
    json!({ "gone": name })
}

/// Write a whole file as pi does: a hidden file renamed in.
fn publish(dir: &Path, name: &str, text: &str) {
    let staged = dir.join(format!(".{name}.tmp"));
    fs::write(&staged, text).unwrap();
    fs::rename(staged, dir.join(name)).unwrap();
}

fn until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < PATIENCE, "{what} in time");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn says_what_a_folder_holds_then_each_change() {
    let here = here("changes");
    let dir = here.root.join("presence");
    fs::write(dir.join("a.json"), "1").unwrap();
    fs::write(dir.join("b.txt"), "no").unwrap();
    let mut client = Client::asking(&here.socket, &dir);
    assert_eq!(client.listing(), vec![put("a.json", "1")]);
    publish(&dir, "b.json", "2");
    assert_eq!(client.next(), put("b.json", "2"));
    fs::write(dir.join("a.json"), "3").unwrap();
    assert_eq!(client.next(), put("a.json", "3"));
    // The same text again says nothing; the removal after it is next.
    publish(&dir, "a.json", "3");
    fs::remove_file(dir.join("b.json")).unwrap();
    assert_eq!(client.next(), gone("b.json"));
    fs::remove_dir_all(&here.root).unwrap();
}

#[test]
fn follows_a_folder_once_however_many_ask() {
    let here = here("once");
    let dir = here.root.join("presence");
    let mut one = Client::asking(&here.socket, &dir);
    let mut two = Client::asking(&here.socket, &dir);
    assert_eq!(one.listing(), Vec::<Value>::new());
    assert_eq!(two.listing(), Vec::<Value>::new());
    assert_eq!(here.reader.following(), 1, "one follower for both");
    publish(&dir, "a.json", "1");
    assert_eq!(one.next(), put("a.json", "1"));
    assert_eq!(two.next(), put("a.json", "1"));
    let mut late = Client::asking(&here.socket, &dir);
    assert_eq!(
        late.listing(),
        vec![put("a.json", "1")],
        "a late client is told all"
    );
    fs::remove_dir_all(&here.root).unwrap();
}

#[test]
fn refuses_a_folder_outside_its_roots_or_not_there() {
    let here = here("refuses");
    let outside = std::env::temp_dir();
    let mut client = Client::asking(&here.socket, &outside);
    assert_eq!(
        client.next(),
        json!({ "no": "the folder isn't under a root it reads" })
    );
    let mut client = Client::asking(&here.socket, &here.root.join("nowhere"));
    assert_eq!(client.next(), json!({ "no": "the folder isn't there" }));
    // A link inside the root to a folder outside it is outside it.
    std::os::unix::fs::symlink(&outside, here.root.join("out")).unwrap();
    let mut client = Client::asking(&here.socket, &here.root.join("out"));
    assert_eq!(
        client.next(),
        json!({ "no": "the folder isn't under a root it reads" })
    );
    fs::remove_dir_all(&here.root).unwrap();
}

#[test]
fn follows_a_folder_made_again_after_it_went() {
    let here = here("again");
    let dir = here.root.join("presence");
    fs::write(dir.join("a.json"), "1").unwrap();
    let mut client = Client::asking(&here.socket, &dir);
    assert_eq!(client.listing(), vec![put("a.json", "1")]);
    fs::rename(&dir, here.root.join("old")).unwrap();
    fs::create_dir(&dir).unwrap();
    publish(&dir, "b.json", "2");
    let mut said = vec![client.next(), client.next()];
    said.sort_by_key(Value::to_string);
    assert_eq!(said, vec![gone("a.json"), put("b.json", "2")]);
    fs::remove_dir_all(&here.root).unwrap();
}

#[test]
fn lets_a_folder_go_once_its_clients_have() {
    let here = here("lets-go");
    let dir = here.root.join("presence");
    let mut client = Client::asking(&here.socket, &dir);
    client.listing();
    assert_eq!(here.reader.following(), 1);
    drop(client);
    until("the folder let go", || here.reader.following() == 0);
    fs::remove_dir_all(&here.root).unwrap();
}

#[test]
fn lets_a_client_go_that_falls_behind_without_holding_the_rest() {
    let here = here("behind");
    let dir = here.root.join("presence");
    let mut stuck = Client::asking(&here.socket, &dir);
    stuck.listing();
    let mut keeping = Client::asking(&here.socket, &dir);
    keeping.listing();
    // The stuck client reads nothing more: its socket's buffer fills,
    // then its queue, and the reader lets it go rather than wait.
    let big = "x".repeat(8 * 1024);
    let changes = ROOM + 200;
    // inotify merges like events nobody has read yet, so the client
    // keeping up waits for the last change rather than counting them.
    let last = put("a.json", &format!("{}{big}", changes - 1));
    let expected = last.clone();
    let reading = thread::spawn(move || {
        loop {
            let line = keeping.next();
            if line == expected || line.is_null() {
                return line;
            }
        }
    });
    for change in 0..changes {
        fs::write(dir.join("a.json"), format!("{change}{big}")).unwrap();
        // Each write heard on its own, so each is a change said.
        thread::sleep(Duration::from_micros(300));
        if change % 64 == 0 {
            thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(
        reading.join().unwrap(),
        last,
        "the client keeping up heard the last change"
    );
    // The stuck one's connection ends once what was buffered is read.
    let mut lines = 0;
    let mut line = String::new();
    while matches!(stuck.lines.read_line(&mut line), Ok(read) if read > 0) {
        lines += 1;
        line.clear();
    }
    assert!(
        lines < changes,
        "the stuck client was let go: {lines} lines"
    );
    fs::remove_dir_all(&here.root).unwrap();
}
