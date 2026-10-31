//! people SOCKET SOURCES
//!
//! Everyone Joel deals with, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (lib.rs). Each
//! source in SOURCES, a JSON file `{"sources": [{"name", "command"}]}`,
//! runs as a child on its own line protocol: an address written to it is
//! Joel stepping into someone, `?ADDRESS` asking ahead, and each line it
//! says back is what it knows of one of them, or how its sign-in stands.
//! A source that stops is started again, and asked again about everyone
//! watched. Nothing is asked of a source but what a client asks for, so
//! with nobody asking, nothing runs but the sources waiting on input.
//!
//! A `get` is a step-in: each source is asked about each address, and the
//! answer waits, at most its patience, for every source that can answer.
//! A `watch` asks ahead about its addresses, says each record held at
//! once, then each change, gathered and said at most four times a second,
//! so a burst of answers is one change to the client.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use people::{Ask, Book, KIND, Standing, VERSION, address, ask, error, heard, now};
use serde::Deserialize;
use serde_json::{Value, json};

/// How often what changed is said to each watch.
const GATHERED: Duration = Duration::from_millis(250);

/// How long a source that stopped waits before it's started again, at
/// first and at most.
const AGAIN: Duration = Duration::from_secs(2);
const AGAIN_AT_MOST: Duration = Duration::from_secs(60);

/// How many of the last gets' times health says.
const TIMES: usize = 32;

#[derive(Deserialize)]
struct Sources {
    sources: Vec<Source>,
}

#[derive(Clone, Deserialize)]
struct Source {
    name: String,
    command: Vec<String>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The keys watched, in the order asked, or none for everyone.
    keys: Option<Vec<String>>,
    told: Sender<String>,
    /// What changed since it was last told, by key.
    pending: HashMap<String, Value>,
}

/// Everything the threads share.
struct Daemon {
    book: Book,
    /// Each source's input while it runs.
    inputs: HashMap<String, ChildStdin>,
    /// Each source's addresses asked ahead while it has run this time.
    ahead: HashMap<String, HashSet<String>>,
    watches: Vec<Watch>,
    /// How long the last gets took, in milliseconds.
    took: VecDeque<u64>,
    since: u64,
}

type Shared = Arc<(Mutex<Daemon>, Condvar)>;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(sources)) = (args.next(), args.next()) else {
        eprintln!("usage: people SOCKET SOURCES");
        std::process::exit(64);
    };
    let sources: Vec<Source> = match std::fs::read_to_string(&sources)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str::<Sources>(&text).map_err(|error| error.to_string()))
    {
        Ok(read) => read.sources,
        Err(why) => {
            eprintln!("<3>couldn't read the sources: {why}");
            std::process::exit(78);
        }
    };
    let socket = PathBuf::from(socket);
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let names: Vec<String> = sources.iter().map(|source| source.name.clone()).collect();
    let shared: Shared = Arc::new((
        Mutex::new(Daemon {
            book: Book::of(&names),
            inputs: HashMap::new(),
            ahead: HashMap::new(),
            watches: Vec::new(),
            took: VecDeque::new(),
            since: now(),
        }),
        Condvar::new(),
    ));
    for source in sources {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || run(&shared, &source));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || gather(&shared));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &std::path::Path) -> std::io::Result<UnixListener> {
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

fn lock(shared: &Shared) -> MutexGuard<'_, Daemon> {
    shared
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs a source, starting it again whenever it stops.
fn run(shared: &Shared, source: &Source) {
    let mut again = AGAIN;
    loop {
        let Some((program, arguments)) = source.command.split_first() else {
            lock(shared).book.stand(
                &source.name,
                Standing::Refused("it names no program".to_owned()),
            );
            return;
        };
        let started = Instant::now();
        match Command::new(program)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(mut child) => {
                let stdout = child.stdout.take();
                {
                    let mut daemon = lock(shared);
                    daemon.book.restarted(&source.name);
                    if let Some(input) = child.stdin.take() {
                        daemon.inputs.insert(source.name.clone(), input);
                    }
                    daemon.ahead.remove(&source.name);
                    // Everyone watched is asked about again.
                    let watched: Vec<String> = daemon
                        .watches
                        .iter()
                        .filter_map(|watch| watch.keys.as_ref())
                        .flatten()
                        .cloned()
                        .collect();
                    ask_ahead(&mut daemon, &watched);
                }
                if let Some(stdout) = stdout {
                    for line in BufReader::new(stdout).lines() {
                        let Ok(line) = line else { break };
                        took_in(shared, &source.name, &line);
                    }
                }
                let _ = child.wait();
                let mut daemon = lock(shared);
                daemon.inputs.remove(&source.name);
                daemon
                    .book
                    .stand(&source.name, Standing::Refused("it stopped".to_owned()));
                shared.1.notify_all();
            }
            Err(why) => {
                eprintln!("<4>couldn't start {}: {why}", source.name);
                lock(shared)
                    .book
                    .stand(&source.name, Standing::Refused(why.to_string()));
                shared.1.notify_all();
            }
        }
        // A source that ran a while is started again soon; one that keeps
        // stopping, less and less often.
        again = if started.elapsed() > AGAIN_AT_MOST {
            AGAIN
        } else {
            (again * 2).min(AGAIN_AT_MOST)
        };
        std::thread::sleep(again);
    }
}

/// Takes one line a source said, telling each watch of what it changed.
fn took_in(shared: &Shared, source: &str, line: &str) {
    let mut daemon = lock(shared);
    if let Some(key) = daemon.book.take(source, heard(line), now())
        && let Some(record) = daemon.book.record(&key).map(people::Record::json)
    {
        for watch in &mut daemon.watches {
            if watch.keys.as_ref().is_none_or(|keys| keys.contains(&key)) {
                watch.pending.insert(key.clone(), record.clone());
            }
        }
    }
    shared.1.notify_all();
}

/// Writes a line to each source running, dropping one that's gone.
fn tell(daemon: &mut Daemon, sources: &[String], line: &str) {
    for source in sources {
        if let Some(input) = daemon.inputs.get_mut(source)
            && writeln!(input, "{line}")
                .and_then(|()| input.flush())
                .is_err()
        {
            daemon.inputs.remove(source);
        }
    }
}

/// Asks each running source ahead about each key it hasn't been asked.
fn ask_ahead(daemon: &mut Daemon, keys: &[String]) {
    let sources: Vec<String> = daemon.inputs.keys().cloned().collect();
    for source in sources {
        let fresh: Vec<String> = keys
            .iter()
            .filter_map(|key| address(key))
            .filter(|address| {
                daemon
                    .ahead
                    .entry(source.clone())
                    .or_default()
                    .insert((*address).to_owned())
            })
            .map(|address| format!("?{address}"))
            .collect();
        for line in fresh {
            tell(daemon, std::slice::from_ref(&source), &line);
        }
    }
}

/// Says what changed to each watch, at most every GATHERED.
fn gather(shared: &Shared) {
    loop {
        std::thread::sleep(GATHERED);
        let mut daemon = lock(shared);
        daemon.watches.retain_mut(|watch| {
            if watch.pending.is_empty() {
                return true;
            }
            let changed: Vec<Value> = watch.pending.drain().map(|(_, record)| record).collect();
            let line = json!({"id": watch.id, "changed": changed}).to_string();
            watch.told.send(line).is_ok()
        });
    }
}

/// Answers one client until it goes.
fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let (told, lines) = mpsc::channel::<String>();
    let mut writing = stream;
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let answer = answer(shared, ask(&line), connection, &told);
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// What one ask is answered with.
fn answer(shared: &Shared, asked: Ask, connection: u64, told: &Sender<String>) -> Value {
    match asked {
        Ask::Hello { id } => {
            let daemon = lock(shared);
            json!({
                "id": id, "version": VERSION, "domain": "people", "state": "answering",
                "since": daemon.since, "sources": standings(&daemon),
            })
        }
        Ask::Get { id, keys, patience } => get(shared, &id, &keys, patience),
        Ask::List { id, limit } => {
            let daemon = lock(shared);
            let records: Vec<Value> = daemon
                .book
                .everyone()
                .take(limit.unwrap_or(usize::MAX))
                .map(people::Record::json)
                .collect();
            json!({"id": id, "records": records})
        }
        Ask::Watch { id, keys } => {
            let mut daemon = lock(shared);
            let held: Vec<Value> = match &keys {
                Some(keys) => keys
                    .iter()
                    .filter_map(|key| daemon.book.record(key))
                    .map(people::Record::json)
                    .collect(),
                None => daemon.book.everyone().map(people::Record::json).collect(),
            };
            if let Some(keys) = &keys {
                ask_ahead(&mut daemon, keys);
            }
            // A watch asked again by its id replaces the one before, as the
            // shell does each time the people it lists change.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                keys,
                told: told.clone(),
                pending: HashMap::new(),
            });
            json!({"id": id, "records": held})
        }
        Ask::Unwatch { id } => {
            lock(shared)
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => {
            let daemon = lock(shared);
            let sources: serde_json::Map<String, Value> = daemon
                .book
                .standings()
                .iter()
                .map(|(name, standing)| {
                    let reason = match standing {
                        Standing::Refused(why) => why.as_str(),
                        _ => "",
                    };
                    (
                        name.clone(),
                        json!({"state": standing.word(), "reason": reason}),
                    )
                })
                .collect();
            json!({
                "id": id, "version": VERSION, "state": "answering", "sources": sources,
                "people": daemon.book.everyone().count(), "watching": daemon.watches.len(),
                "took": daemon.took,
            })
        }
        Ask::UnknownKind { id, kind } => error(
            &id,
            "unknown-kind",
            &format!("this daemon keeps only {KIND}, not {kind}"),
        ),
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?}")),
    }
}

fn standings(daemon: &Daemon) -> Value {
    daemon
        .book
        .standings()
        .iter()
        .map(|(name, standing)| (name.clone(), json!(standing.word())))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// A step-in: each source asked about each key, the answer waiting at
/// most `patience` for every source that can answer, then saying what's
/// held, or not-ready when nothing is.
fn get(shared: &Shared, id: &Value, keys: &[String], patience: u64) -> Value {
    let began = Instant::now();
    let mut daemon = lock(shared);
    let sources: Vec<String> = daemon.inputs.keys().cloned().collect();
    for address in keys.iter().filter_map(|key| address(key)) {
        tell(&mut daemon, &sources, address);
    }
    let deadline = began + Duration::from_millis(patience);
    while !keys.iter().all(|key| daemon.book.ready(key)) {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        daemon = shared
            .1
            .wait_timeout(daemon, left)
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0;
    }
    let records: Vec<Value> = keys
        .iter()
        .filter_map(|key| daemon.book.record(key))
        .map(people::Record::json)
        .collect();
    let waiting: Vec<&String> = keys.iter().filter(|key| !daemon.book.ready(key)).collect();
    let took = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
    daemon.took.push_back(took);
    if daemon.took.len() > TIMES {
        daemon.took.pop_front();
    }
    if records.is_empty() && !waiting.is_empty() {
        return error(id, "not-ready", "no source has answered yet");
    }
    json!({"id": id, "records": records, "waiting": waiting, "took": took})
}
