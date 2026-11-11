//! people SOCKET SOURCES [WORDS]
//!
//! Everyone Joel deals with, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (lib.rs). Each
//! source in SOURCES, a JSON file
//! `{"sources": [{"name", "command", "stands"}]}`,
//! runs as a child on its own line protocol: an address written to it is
//! Joel stepping into someone, `?ADDRESS` asking ahead, and each line it
//! says back is what it knows of one of them, or how its sign-in stands.
//! A source that stops is started again, and asked again about everyone
//! watched. Nothing is asked of a source but what a client asks for, so
//! with nobody asking, nothing runs but the sources waiting on input.
//!
//! A `get` is a step-in: each source is asked about each address, and the
//! answer waits, at most its patience, for every source that can answer.
//! A `known` is the records held for its keys, with no source asked.
//! A `watch` asks ahead about its addresses, says each record held at
//! once, then each change, gathered and said at most four times a second,
//! so a burst of answers is one change to the client, and how every
//! source stands, `{"id", "sources"}` as hello says it, whenever that
//! changes. Each address is asked ahead once while a source runs; one a
//! source couldn't answer, signed out or saying later, is asked again
//! once that source next says who someone is.
//!
//! A source whose first line is `{"roster": true}`, as Tuple's is, says
//! everyone it knows without being asked, thousands of them, and is never
//! waited on. So the shell holds none of them: a `search` answers the few
//! dozen whose names or addresses the words find, and a `list` or `watch`
//! with `listed` only those a source lists with nothing typed.
//!
//! A `list` or `watch` of kind `listing` is those People lists with
//! nothing typed, as it shows them (listing.rs): a record for each, as
//! one person, with how they stand by each source's `stands` in People's
//! words as WORDS, the launcher's words.json, says them, and a
//! `people:order` record of their keys in order. A watch of it is told
//! again only once the book changed, or the clock reached a meeting's
//! start or end, and then only the persons that changed, so the shell
//! remakes nothing for everyone else.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use people::listing::{self, Stands, Told};
use people::{Ask, Book, Heard, KIND, Record, Standing, VERSION, address, ask, error, heard, now};
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
    /// How its lines say someone stands, in People's words (listing.rs).
    #[serde(default)]
    stands: Option<Stands>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The keys watched, in the order asked, or none for everyone.
    keys: Option<Vec<String>>,
    /// Whether, watching everyone, it's only those a source lists.
    listed: bool,
    told: Sender<String>,
    /// What changed since it was last told, by key.
    pending: HashMap<String, Value>,
    /// For a watch of those listed, what it was last told, and the book's
    /// revision then, so it's told again only once something changed.
    listing: Option<(Told, u64)>,
}

/// Everything the threads share.
struct Daemon {
    book: Book,
    /// Each source's input while it runs.
    inputs: HashMap<String, ChildStdin>,
    /// Each source's addresses asked ahead while it has run this time.
    ahead: HashMap<String, HashSet<String>>,
    /// Each source's addresses it said later of, to be asked again once
    /// it says who someone is.
    put_off: HashMap<String, HashSet<String>>,
    /// The addresses a get waits on, each with how many wait on it, so a
    /// source that starts while one waits is asked too.
    stepping: HashMap<String, usize>,
    watches: Vec<Watch>,
    /// How long the last gets took, in milliseconds.
    took: VecDeque<u64>,
    since: u64,
    /// The sources in the order they're consulted, each with how its lines
    /// say someone stands.
    sources: Vec<(String, Option<Stands>)>,
    /// Counts each change the book takes, for a watch of those listed.
    revision: u64,
}

type Shared = Arc<(Mutex<Daemon>, Condvar)>;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(sources)) = (args.next(), args.next()) else {
        eprintln!("usage: people SOCKET SOURCES [WORDS]");
        std::process::exit(64);
    };
    let said = args.next().map_or_else(BTreeMap::default, |words| {
        match std::fs::read_to_string(&words)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
        {
            Ok(vocabulary) => listing::words(&vocabulary),
            Err(why) => {
                eprintln!("<4>couldn't read the launcher's words, so each is said by name: {why}");
                BTreeMap::default()
            }
        }
    });
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
            put_off: HashMap::new(),
            stepping: HashMap::new(),
            watches: Vec::new(),
            took: VecDeque::new(),
            since: now(),
            sources: sources
                .iter()
                .map(|source| {
                    let stands = source.stands.clone().map(|stands| stands.worded(&said));
                    (source.name.clone(), stands)
                })
                .collect(),
            revision: 0,
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

/// The socket systemd holds for it (people.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &std::path::Path) -> std::io::Result<UnixListener> {
    if let Some(listener) = handed() {
        return Ok(listener);
    }
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

/// The listening socket systemd hands this process, as the first
/// descriptor after the standard three, when it says so (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    let given = unsafe { UnixListener::from_raw_fd(3) };
    // A copy closed on exec, so the sources it runs don't hold the socket
    // open, which the descriptor as handed over isn't.
    given.try_clone().ok()
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
                    let stood = standings(&daemon);
                    daemon.book.restarted(&source.name);
                    tell_standings(&daemon, &stood);
                    if let Some(input) = child.stdin.take() {
                        daemon.inputs.insert(source.name.clone(), input);
                    }
                    daemon.ahead.remove(&source.name);
                    // Whoever a get waits on is asked about now, since the
                    // get told only the sources running when it came.
                    let stepping: Vec<String> = daemon.stepping.keys().cloned().collect();
                    for address in &stepping {
                        tell(&mut daemon, std::slice::from_ref(&source.name), address);
                    }
                    // Everyone watched is asked about again.
                    daemon.put_off.remove(&source.name);
                    let watched = watched(&daemon);
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
                let stood = standings(&daemon);
                daemon
                    .book
                    .stand(&source.name, Standing::Refused("it stopped".to_owned()));
                tell_standings(&daemon, &stood);
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
    let stood = standings(&daemon);
    let heard = heard(line);
    // A source back, saying who someone is after it stood signed out or
    // refused, or after it put someone off, never said who anyone asked
    // ahead meanwhile was, nor will unasked: they're asked again.
    let back = match &heard {
        Heard::Person { .. } => {
            matches!(
                daemon.book.standings().get(source),
                Some(Standing::SignedOut | Standing::Refused(_))
            ) || daemon
                .put_off
                .get(source)
                .is_some_and(|put| !put.is_empty())
        }
        Heard::Later { address } => {
            daemon
                .put_off
                .entry(source.to_owned())
                .or_default()
                .insert(address.clone());
            false
        }
        _ => false,
    };
    // Whether they were listed before, so a watch of the listed hears of
    // one who stops being.
    let was_listed = daemon
        .book
        .about(source, &heard)
        .and_then(|key| daemon.book.record(&key))
        .is_some_and(Record::listed);
    if let Some(key) = daemon.book.take(source, heard, now()) {
        daemon.revision += 1;
        let record = daemon.book.record(&key);
        let listed = was_listed || record.is_some_and(Record::listed);
        let told = record.map_or_else(|| Record::gone(&key), Record::json);
        for watch in &mut daemon.watches {
            let wants = match &watch.keys {
                _ if watch.listing.is_some() => false,
                Some(keys) => keys.contains(&key),
                None => !watch.listed || listed,
            };
            if wants {
                watch.pending.insert(key.clone(), told.clone());
            }
        }
    }
    tell_standings(&daemon, &stood);
    if back {
        daemon.put_off.remove(source);
        // Only those it hasn't said, since one it said may be all it can
        // say yet, as someone kept is, and asking it again about everyone
        // would have it say them again, and be back again, without end.
        let unsaid: Vec<String> = watched(&daemon)
            .into_iter()
            .filter(|key| {
                !daemon
                    .book
                    .record(key)
                    .is_some_and(|record| record.sources.contains_key(source))
            })
            .collect();
        if let Some(ahead) = daemon.ahead.get_mut(source) {
            for key in &unsaid {
                if let Some(address) = address(key) {
                    ahead.remove(address);
                }
            }
        }
        ask_ahead_of(&mut daemon, &[source.to_owned()], &unsaid);
    }
    shared.1.notify_all();
}

/// Everyone a watch names, as a source is asked ahead about once it can
/// answer again.
fn watched(daemon: &Daemon) -> Vec<String> {
    daemon
        .watches
        .iter()
        .filter_map(|watch| watch.keys.as_ref())
        .flatten()
        .cloned()
        .collect()
}

/// Tells every watch how each source stands, once that's no longer as
/// `stood`, so a client hears a sign-in go or come as the source says so
/// rather than at its next ask: a step-in waits on every source, and one
/// that never answers would keep it from hearing for its whole patience.
fn tell_standings(daemon: &Daemon, stood: &Value) {
    let standing = standings(daemon);
    if standing == *stood {
        return;
    }
    for watch in &daemon.watches {
        let _ = watch
            .told
            .send(json!({"id": watch.id, "sources": standing}).to_string());
    }
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
    ask_ahead_of(daemon, &sources, keys);
}

/// Asks these sources ahead about each of the keys none has been asked
/// ahead about while it has run this time.
fn ask_ahead_of(daemon: &mut Daemon, sources: &[String], keys: &[String]) {
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
            tell(daemon, std::slice::from_ref(source), &line);
        }
    }
}

/// Milliseconds since the epoch, as the shell's times and the spans
/// sources say are numbered.
fn clock() -> f64 {
    Duration::from_millis(now()).as_secs_f64() * 1000.0
}

/// Says what changed to each watch, at most every GATHERED.
fn gather(shared: &Shared) {
    loop {
        std::thread::sleep(GATHERED);
        say_gathered(&mut lock(shared));
    }
}

/// Tells each watch what changed since it was last told, dropping one
/// that's gone.
fn say_gathered(daemon: &mut Daemon) {
    let Daemon {
        watches,
        book,
        sources,
        revision,
        ..
    } = daemon;
    let at = clock();
    watches.retain_mut(|watch| {
        if let Some((told, seen)) = &mut watch.listing {
            // Told again once the book changed, or once the clock reaches
            // something with someone starting or ending.
            if *seen == *revision && told.until.is_none_or(|until| until > at) {
                return true;
            }
            *seen = *revision;
            let changed = told.changes(book, sources, at);
            if changed.is_empty() {
                return true;
            }
            let line = json!({"id": watch.id, "changed": changed}).to_string();
            return watch.told.send(line).is_ok();
        }
        if watch.pending.is_empty() {
            return true;
        }
        let changed: Vec<Value> = watch.pending.drain().map(|(_, record)| record).collect();
        let line = json!({"id": watch.id, "changed": changed}).to_string();
        watch.told.send(line).is_ok()
    });
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
        let Some(answer) = answer(shared, ask(&line), connection, &told) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// Those listed, in order, as a listing asks: answered once, or watched,
/// told whole now and after only who changed.
fn listing(
    shared: &Shared,
    id: Value,
    watch: bool,
    connection: u64,
    told: &Sender<String>,
) -> Option<Value> {
    let mut daemon = lock(shared);
    let mut said = Told::default();
    let records = said.changes(&daemon.book, &daemon.sources, clock());
    if !watch {
        return Some(json!({"id": id, "records": records}));
    }
    daemon
        .watches
        .retain(|watch| !(watch.connection == connection && watch.id == id));
    // Told under the lock that holds the watch, as a watch of records is,
    // so no change is said before the whole.
    let _ = told.send(json!({"id": id, "records": records}).to_string());
    let seen = daemon.revision;
    daemon.watches.push(Watch {
        connection,
        id,
        keys: None,
        listed: false,
        told: told.clone(),
        pending: HashMap::new(),
        listing: Some((said, seen)),
    });
    None
}

/// Who it is, and since when, as the shell's daemon interface opens.
fn hello(daemon: &Daemon, id: &Value) -> Value {
    json!({
        "id": id, "version": VERSION, "domain": "people", "state": "answering",
        "since": daemon.since, "sources": standings(daemon),
    })
}

/// What one ask is answered with, or nothing where it's already told, as a
/// watch's answer is.
fn answer(shared: &Shared, asked: Ask, connection: u64, told: &Sender<String>) -> Option<Value> {
    Some(match asked {
        Ask::Hello { id } => hello(&lock(shared), &id),
        Ask::Get { id, keys, patience } => get(shared, &id, &keys, patience),
        Ask::Known { id, keys } => json!({"id": id, "records": lock(shared).book.known(&keys)}),
        Ask::List { id, limit, listed } => {
            let daemon = lock(shared);
            let records: Vec<Value> = daemon
                .book
                .everyone()
                .filter(|record| !listed || record.listed())
                .take(limit.unwrap_or(usize::MAX))
                .map(Record::json)
                .collect();
            json!({"id": id, "records": records})
        }
        Ask::Search { id, words, limit } => {
            let daemon = lock(shared);
            let records: Vec<Value> = daemon
                .book
                .search(&words, limit)
                .into_iter()
                .map(|record| listing::found(record, &daemon.sources))
                .collect();
            json!({"id": id, "records": records})
        }
        Ask::Watch { id, keys, listed } => {
            let mut daemon = lock(shared);
            let held: Vec<Value> = match &keys {
                Some(keys) => keys
                    .iter()
                    .filter_map(|key| daemon.book.record(key))
                    .map(Record::json)
                    .collect(),
                None => daemon
                    .book
                    .everyone()
                    .filter(|record| !listed || record.listed())
                    .map(Record::json)
                    .collect(),
            };
            if let Some(keys) = &keys {
                ask_ahead(&mut daemon, keys);
            }
            // A watch asked again by its id replaces the one before, as the
            // shell does each time the people it lists change.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            // It's told under the lock that holds the watch, so what's
            // gathered once the lock is let go is told after it. Told after
            // the lock was let go, a change could be said first, and the
            // whole answer after it would put back what the change moved on
            // from. Where it can't be told, the connection's writer has
            // gone, and so has the connection.
            let _ = told.send(json!({"id": id, "records": held}).to_string());
            daemon.watches.push(Watch {
                connection,
                id,
                keys,
                listed,
                told: told.clone(),
                pending: HashMap::new(),
                listing: None,
            });
            return None;
        }
        Ask::Listing { id, watch } => return listing(shared, id, watch, connection, told),
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
    })
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
    let addresses: Vec<String> = keys
        .iter()
        .filter_map(|key| address(key))
        .map(str::to_owned)
        .collect();
    for address in &addresses {
        tell(&mut daemon, &sources, address);
        *daemon.stepping.entry(address.clone()).or_default() += 1;
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
    for address in &addresses {
        if let Some(count) = daemon.stepping.get_mut(address) {
            *count -= 1;
            if *count == 0 {
                daemon.stepping.remove(address);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_is_answered_whole_before_any_change_told_it() {
        let shared: Shared = Arc::new((
            Mutex::new(Daemon {
                book: Book::of(&[]),
                inputs: HashMap::new(),
                ahead: HashMap::new(),
                put_off: HashMap::new(),
                stepping: HashMap::new(),
                watches: Vec::new(),
                took: VecDeque::new(),
                since: now(),
                sources: Vec::new(),
                revision: 0,
            }),
            Condvar::new(),
        ));
        let (told, lines) = mpsc::channel::<String>();
        let first = answer(&shared, ask(r#"{"id":"all","ask":"watch"}"#), 1, &told);
        // A change gathered once the answer's lock is let go.
        {
            let mut daemon = lock(&shared);
            let someone = json!({"key": "someone@example.com"});
            for watch in &mut daemon.watches {
                watch
                    .pending
                    .insert("someone@example.com".to_owned(), someone.clone());
            }
            say_gathered(&mut daemon);
        }
        if let Some(first) = first {
            told.send(first.to_string()).expect("told");
        }
        let said: Vec<Value> = lines
            .try_iter()
            .map(|line| serde_json::from_str(&line).expect("json"))
            .collect();
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0]["records"].is_array(), "{said:?}");
        assert_eq!(
            said[1]["changed"][0]["key"], "someone@example.com",
            "{said:?}"
        );
    }
}
