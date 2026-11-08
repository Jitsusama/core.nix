//! passwords daemon SOCKET MANAGERS
//! passwords find WORDS | list | show ENTRY [DETAIL] | copy ENTRY [DETAIL]
//! passwords code ENTRY | let-in MANAGER | health
//!
//! Joel's passwords, from every password manager he keeps, held ready for
//! the shell and the command line over SOCKET in the shell's daemon
//! interface (lib.rs). Each manager in MANAGERS, a JSON file
//! `{"managers": [{"name", "command"}]}`, runs as a child on the passwords
//! line protocol: a JSON line in for each ask, `{"id", "ask"}` with `list`,
//! `get`, `reveal`, `code` or `let-in`, and a JSON line out answering it by
//! its id, or `{"state", "reason"}` unasked as how it stands changes. A
//! manager that stops is started again. How each manager stands goes to
//! clients as the interface's `sources`, the word every daemon's client
//! (Daemon.qml) reads, though the screen calls them password managers.
//!
//! Nothing is asked of a manager but what a client asks for, so with
//! nobody asking, nothing runs but the managers waiting on input, and a
//! manager that has to ask Joel's leave, as 1Password's command does, only
//! ever asks it in answer to his own key. What's held is each entry
//! without its secrets (lib.rs); a secret goes from its manager to the
//! client that asked to see it, or to the clipboard, marked sensitive and
//! cleared after CLEARS, and is never kept, written or logged here.
//!
//! Run with any other word, it's the command line, asking the daemon at
//! `$PASSWORDS_SOCKET`, else `$XDG_RUNTIME_DIR/daemons/passwords.sock`. ENTRY
//! is an entry's key or words that find just one.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use passwords::{
    Ask, Changes, DOMAIN, Held, KIND, PATIENCE, Said, Standing, VERSION, ask, error, named, now,
    said, standing,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// How long a copied secret stays on the clipboard, as 1Password's own
/// app clears it, unless `$PASSWORDS_CLEARS_MS` says otherwise.
const CLEARS: Duration = Duration::from_secs(90);

/// How long a manager that stopped waits before it's started again, at
/// first and at most.
const AGAIN: Duration = Duration::from_secs(2);
const AGAIN_AT_MOST: Duration = Duration::from_secs(60);

/// How many of the last asks' times health says.
const TIMES: usize = 32;

#[derive(Deserialize)]
struct Managers {
    managers: Vec<Manager>,
}

#[derive(Clone, Deserialize)]
struct Manager {
    name: String,
    command: Vec<String>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    told: Sender<String>,
}

/// Everything the threads share.
struct Daemon {
    held: Held,
    inputs: HashMap<String, ChildStdin>,
    /// The asks each manager hasn't answered, by the id it was asked with.
    pending: HashMap<u64, (String, Sender<Map<String, Value>>)>,
    next: u64,
    watches: Vec<Watch>,
    took: VecDeque<u64>,
    since: u64,
    /// The copy on the clipboard now, cleared when its time is up.
    copied: Option<Child>,
}

/// The daemon's state, and what wakes an ask waiting on a manager to start.
type Shared = Arc<(Mutex<Daemon>, Condvar)>;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("daemon") => {
            let (Some(socket), Some(managers)) = (args.next(), args.next()) else {
                eprintln!("usage: passwords daemon SOCKET MANAGERS");
                std::process::exit(64);
            };
            daemon(&PathBuf::from(socket), &managers);
        }
        Some(asked) => {
            let rest: Vec<String> = args.collect();
            std::process::exit(command_line(asked, &rest));
        }
        None => {
            eprintln!("{USAGE}");
            std::process::exit(64);
        }
    }
}

const USAGE: &str = "usage: passwords find WORDS | list | show ENTRY [DETAIL] | copy ENTRY [DETAIL]
       passwords code ENTRY | let-in MANAGER | health | daemon SOCKET MANAGERS";

fn daemon(socket: &std::path::Path, managers: &str) {
    let managers: Vec<Manager> = match std::fs::read_to_string(managers)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str::<Managers>(&text).map_err(|error| error.to_string()))
    {
        Ok(read) => read.managers,
        Err(why) => {
            eprintln!("<3>couldn't read the password managers: {why}");
            std::process::exit(78);
        }
    };
    let listener = match listen(socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let names: Vec<String> = managers
        .iter()
        .map(|manager| manager.name.clone())
        .collect();
    let shared: Shared = Arc::new((
        Mutex::new(Daemon {
            held: Held::of(&names),
            inputs: HashMap::new(),
            pending: HashMap::new(),
            next: 1,
            watches: Vec::new(),
            took: VecDeque::new(),
            since: now(),
            copied: None,
        }),
        Condvar::new(),
    ));
    for manager in managers {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || run(&shared, &manager));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket systemd holds for it (passwords.socket), so the shell never
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
    // A copy closed on exec, so the managers it runs don't hold the socket
    // open, which the descriptor as handed over isn't.
    given.try_clone().ok()
}

fn lock(shared: &Shared) -> MutexGuard<'_, Daemon> {
    shared
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs a manager, starting it again whenever it stops.
fn run(shared: &Shared, manager: &Manager) {
    let mut again = AGAIN;
    loop {
        let Some((program, arguments)) = manager.command.split_first() else {
            stand(
                shared,
                &manager.name,
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
                if let Some(input) = child.stdin.take() {
                    lock(shared).inputs.insert(manager.name.clone(), input);
                    shared.1.notify_all();
                }
                if let Some(stdout) = stdout {
                    for line in BufReader::new(stdout).lines() {
                        let Ok(line) = line else { break };
                        took_in(shared, &manager.name, &line);
                    }
                }
                let _ = child.wait();
                let mut daemon = lock(shared);
                daemon.inputs.remove(&manager.name);
                // Whoever waits on it is let go at once, as refused.
                daemon
                    .pending
                    .retain(|_, (asked_of, _)| *asked_of != manager.name);
                drop(daemon);
                stand(
                    shared,
                    &manager.name,
                    Standing::Refused("it stopped".to_owned()),
                );
            }
            Err(why) => {
                eprintln!("<4>couldn't start {}: {why}", manager.name);
                stand(shared, &manager.name, Standing::Refused(why.to_string()));
            }
        }
        // A manager that ran a while is started again soon; one that keeps
        // stopping, less and less often.
        again = if started.elapsed() > AGAIN_AT_MOST {
            AGAIN
        } else {
            (again * 2).min(AGAIN_AT_MOST)
        };
        std::thread::sleep(again);
    }
}

/// Says how a manager stands, telling each watch when that changed.
fn stand(shared: &Shared, manager: &str, standing: Standing) {
    let mut daemon = lock(shared);
    if daemon.held.standing(manager) == Some(&standing) {
        return;
    }
    daemon.held.stand(manager, standing);
    let managers = daemon.held.standings();
    let state = daemon.held.state();
    for watch in &daemon.watches {
        let line = json!({"id": watch.id, "sources": managers, "state": state});
        let _ = watch.told.send(line.to_string());
    }
}

/// Takes one line a manager said: how it stands, or an answer, whose
/// entries are held without their secrets and told to each watch before
/// the answer goes to whoever asked.
fn took_in(shared: &Shared, manager: &str, line: &str) {
    match said(line) {
        Said::Stands(standing) => stand(shared, manager, standing),
        Said::Answer { id, answer } => {
            if let Some(standing) = standing(&answer) {
                stand(shared, manager, standing);
            }
            let mut daemon = lock(shared);
            let heard = now();
            let mut changes = Changes::default();
            if let Some(Value::Array(entries)) = answer.get("entries") {
                changes = daemon.held.list(manager, entries, heard);
            }
            if let Some(given) = answer.get("entry")
                && let Some(key) = daemon.held.entry(manager, given, heard)
            {
                changes.changed.push(key);
            }
            if !(changes.changed.is_empty() && changes.gone.is_empty()) {
                let records: Vec<Value> = changes
                    .changed
                    .iter()
                    .filter_map(|key| daemon.held.record(key))
                    .collect();
                for watch in &daemon.watches {
                    let line = json!({"id": watch.id, "changed": records, "gone": changes.gone});
                    let _ = watch.told.send(line.to_string());
                }
            }
            if let Some((_, waiting)) = daemon.pending.remove(&id) {
                let _ = waiting.send(answer);
            }
        }
        Said::Other => {}
    }
}

/// Asks a manager one thing, waiting at most `patience` for its answer:
/// the answer, or the interface's error code and what to say.
fn ask_of(
    shared: &Shared,
    manager: &str,
    mut asked: Map<String, Value>,
    patience: u64,
) -> Result<Map<String, Value>, (&'static str, String)> {
    let (sent, answered) = mpsc::channel();
    let started = Instant::now();
    let patience_for = Duration::from_millis(patience);
    let id = {
        let mut daemon = lock(shared);
        // One asked as the daemon starts, or as its manager starts again,
        // waits for it within its patience rather than being refused.
        while !daemon.inputs.contains_key(manager) {
            let left = patience_for.saturating_sub(started.elapsed());
            if left.is_zero() || !daemon.held.managers().iter().any(|name| name == manager) {
                return Err(("not-ready", format!("{manager} isn't running")));
            }
            daemon = shared
                .1
                .wait_timeout(daemon, left)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
        let id = daemon.next;
        daemon.next += 1;
        asked.insert("id".to_owned(), json!(id));
        let line = Value::Object(asked).to_string();
        let Some(input) = daemon.inputs.get_mut(manager) else {
            return Err(("refused", format!("{manager} isn't running")));
        };
        if writeln!(input, "{line}")
            .and_then(|()| input.flush())
            .is_err()
        {
            return Err(("refused", format!("{manager} stopped")));
        }
        daemon.pending.insert(id, (manager.to_owned(), sent));
        id
    };
    let answer = answered.recv_timeout(patience_for.saturating_sub(started.elapsed()));
    lock(shared).pending.remove(&id);
    let Ok(answer) = answer else {
        return Err(("not-ready", format!("{manager} didn't answer in time")));
    };
    if let Some(error) = answer.get("error") {
        let code = match error.get("code").and_then(Value::as_str) {
            Some("signed-out") => "signed-out",
            Some("not-ready") => "not-ready",
            _ => "refused",
        };
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("it refused")
            .to_owned();
        return Err((code, message));
    }
    Ok(answer)
}

fn asking(name: &str, pairs: &[(&str, Value)]) -> Map<String, Value> {
    let mut asked = Map::new();
    asked.insert("ask".to_owned(), Value::String(name.to_owned()));
    for (key, value) in pairs {
        asked.insert((*key).to_owned(), value.clone());
    }
    asked
}

/// Serves one client: each line an ask, answered on its own thread so a
/// slow manager holds up only that ask, every answer written in turn.
fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
    let Ok(writing) = stream.try_clone() else {
        return;
    };
    let (told, lines) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut writing = writing;
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        let shared = Arc::clone(shared);
        let told = told.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let answer = answer(&shared, ask(&line), connection, &told);
            if !answer.is_null() {
                let _ = told.send(answer.to_string());
            }
            let took = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let mut daemon = lock(&shared);
            daemon.took.push_back(took);
            if daemon.took.len() > TIMES {
                daemon.took.pop_front();
            }
        });
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// Answers one ask; null for one whose answers go by its watch alone.
fn answer(shared: &Shared, asked: Ask, connection: u64, told: &Sender<String>) -> Value {
    match asked {
        Ask::Hello { id } => {
            let daemon = lock(shared);
            json!({"id": id, "version": VERSION, "domain": DOMAIN, "kind": KIND,
                   "state": daemon.held.state(), "since": daemon.since,
                   "sources": daemon.held.standings()})
        }
        Ask::Health { id } => {
            let daemon = lock(shared);
            json!({"id": id, "version": VERSION, "domain": DOMAIN,
                   "state": daemon.held.state(), "since": daemon.since,
                   "sources": daemon.held.standings(),
                   "entries": daemon.held.records().len(), "took": daemon.took})
        }
        Ask::List {
            id,
            limit,
            fresh,
            asked,
            patience,
        } => {
            list(shared, fresh, asked, patience);
            let daemon = lock(shared);
            let mut records = daemon.held.records();
            if let Some(limit) = limit {
                records.truncate(limit);
            }
            json!({"id": id, "records": records, "sources": daemon.held.standings()})
        }
        Ask::Get {
            id,
            keys,
            asked,
            patience,
        } => get(shared, &id, &keys, asked, patience),
        Ask::Watch { id } => {
            let mut daemon = lock(shared);
            let records = daemon.held.records();
            let _ = told.send(
                json!({"id": id, "records": records,
                       "sources": daemon.held.standings(), "state": daemon.held.state()})
                .to_string(),
            );
            daemon.watches.push(Watch {
                connection,
                id,
                told: told.clone(),
            });
            Value::Null
        }
        Ask::Unwatch { id } => {
            lock(shared)
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Search { id, words, limit } => {
            json!({"id": id, "records": lock(shared).held.search(&words, limit)})
        }
        Ask::Reveal { id, key, detail } => match secret(shared, &key, detail.as_deref()) {
            Ok((value, label)) => json!({"id": id, "value": value, "detail": label}),
            Err((code, message)) => error(&id, code, &message),
        },
        Ask::Copy { id, key, detail } => match secret(shared, &key, detail.as_deref()) {
            Ok((value, label)) => match copy(shared, &value) {
                Ok(clears) => json!({"id": id, "copied": true, "detail": label, "clears": clears}),
                Err(why) => error(&id, "refused", &why),
            },
            Err((code, message)) => error(&id, code, &message),
        },
        Ask::Code { id, key } => code(shared, &id, &key),
        Ask::LetIn { id, manager } => {
            if !lock(shared).held.managers().contains(&manager) {
                return error(
                    &id,
                    "refused",
                    &format!("no password manager is called {manager}"),
                );
            }
            // Letting in waits on Joel, for as long as he takes to answer.
            match ask_of(shared, &manager, asking("let-in", &[]), 120_000) {
                Ok(_) => {
                    list(shared, true, true, PATIENCE);
                    json!({"id": id, "let": true})
                }
                Err((code, message)) => error(&id, code, &message),
            }
        }
        Ask::UnknownKind { id, kind } => error(
            &id,
            "unknown-kind",
            &format!("this daemon keeps entries, not {kind}"),
        ),
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("can't answer {ask:?}")),
    }
}

/// Reads every manager's list, each on its own thread, where one hasn't
/// listed since it started or `fresh` asks, waiting at most `patience`.
fn list(shared: &Shared, fresh: bool, asked: bool, patience: u64) {
    let managers: Vec<String> = {
        let daemon = lock(shared);
        daemon
            .held
            .managers()
            .iter()
            .filter(|manager| fresh || !daemon.held.listed(manager))
            .cloned()
            .collect()
    };
    let reading: Vec<_> = managers
        .into_iter()
        .map(|manager| {
            let shared = Arc::clone(shared);
            std::thread::spawn(move || {
                let _ = ask_of(
                    &shared,
                    &manager,
                    asking("list", &[("asked", json!(asked))]),
                    patience,
                );
            })
        })
        .collect();
    for thread in reading {
        let _ = thread.join();
    }
}

/// Entries by key in full, each read from its manager where only its
/// summary is held.
fn get(shared: &Shared, id: &Value, keys: &[String], asked: bool, patience: u64) -> Value {
    let wanting: Vec<String> = {
        let daemon = lock(shared);
        keys.iter()
            .filter(|key| !daemon.held.get(key).is_some_and(passwords::Entry::detailed))
            .cloned()
            .collect()
    };
    let reading: Vec<_> = wanting
        .into_iter()
        .filter_map(|key| {
            let (manager, entry) = named(&key)?;
            let (manager, entry) = (manager.to_owned(), entry.to_owned());
            let shared = Arc::clone(shared);
            Some(std::thread::spawn(move || {
                let _ = ask_of(
                    &shared,
                    &manager,
                    asking("get", &[("entry", json!(entry)), ("asked", json!(asked))]),
                    patience,
                );
            }))
        })
        .collect();
    for thread in reading {
        let _ = thread.join();
    }
    let daemon = lock(shared);
    let records: Vec<Value> = keys
        .iter()
        .filter_map(|key| daemon.held.record(key))
        .collect();
    json!({"id": id, "records": records, "sources": daemon.held.standings()})
}

/// One detail's value and its label, its secret included, asked of its
/// manager; the password when none is named. Never kept.
fn secret(
    shared: &Shared,
    key: &str,
    wanted: Option<&str>,
) -> Result<(String, String), (&'static str, String)> {
    let Some((manager, entry)) = named(key) else {
        return Err(("refused", format!("{key} isn't an entry's key")));
    };
    if !lock(shared)
        .held
        .get(key)
        .is_some_and(passwords::Entry::detailed)
    {
        let _ = get(shared, &Value::Null, &[key.to_owned()], true, PATIENCE);
    }
    let (detail, label) = {
        let daemon = lock(shared);
        let Some(held) = daemon.held.get(key) else {
            return Err(("refused", format!("{key} isn't held")));
        };
        let Some(detail) = held.detail(wanted) else {
            let what = wanted.unwrap_or("password");
            return Err(("refused", format!("it has no {what}")));
        };
        let id = detail
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let label = detail
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_owned();
        (id, label)
    };
    let answer = ask_of(
        shared,
        manager,
        asking(
            "reveal",
            &[("entry", json!(entry)), ("detail", json!(detail))],
        ),
        PATIENCE * 6,
    )?;
    match answer.get("value").and_then(Value::as_str) {
        Some(value) => Ok((value.to_owned(), label)),
        None => Err(("refused", "its manager gave no value".to_owned())),
    }
}

/// Puts a secret on the clipboard through wl-copy, marked sensitive so a
/// clipboard history leaves it out, held by a wl-copy in the foreground
/// that's ended when its time is up, which clears it; one copied over
/// meanwhile has ended it already, so what was copied since stays.
fn copy(shared: &Shared, value: &str) -> Result<u64, String> {
    let program = std::env::var("PASSWORDS_COPIER").unwrap_or_else(|_| "wl-copy".to_owned());
    let clears = std::env::var("PASSWORDS_CLEARS_MS")
        .ok()
        .and_then(|clears| clears.parse().ok())
        .map_or(CLEARS, Duration::from_millis);
    let mut child = Command::new(program)
        .args([
            "--sensitive",
            "--foreground",
            "--type",
            "text/plain;charset=utf-8",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|why| format!("couldn't copy: {why}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(value.as_bytes())
            .map_err(|why| format!("couldn't copy: {why}"))?;
    }
    let pid = child.id();
    {
        let mut daemon = lock(shared);
        if let Some(mut before) = daemon.copied.replace(child) {
            let _ = before.kill();
            let _ = before.wait();
        }
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        std::thread::sleep(clears);
        let mut daemon = lock(&shared);
        if daemon
            .copied
            .as_ref()
            .is_some_and(|child| child.id() == pid)
            && let Some(mut child) = daemon.copied.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    });
    Ok(u64::try_from(u128::from(now()) + clears.as_millis()).unwrap_or(u64::MAX))
}

/// The one-time code an entry gives now, with its period and the seconds
/// left in it.
fn code(shared: &Shared, id: &Value, key: &str) -> Value {
    let Some((manager, entry)) = named(key) else {
        return error(id, "refused", &format!("{key} isn't an entry's key"));
    };
    match ask_of(
        shared,
        manager,
        asking("code", &[("entry", json!(entry))]),
        PATIENCE * 6,
    ) {
        Ok(answer) => json!({
            "id": id,
            "code": answer.get("code"),
            "period": answer.get("period"),
            "left": answer.get("left"),
        }),
        Err((code, message)) => error(id, code, &message),
    }
}

/// The command line: one ask of the daemon, said for a person to read.
fn command_line(asked: &str, rest: &[String]) -> i32 {
    let socket = std::env::var("PASSWORDS_SOCKET").map_or_else(
        |_| {
            let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
            PathBuf::from(runtime).join("daemons/passwords.sock")
        },
        PathBuf::from,
    );
    let Ok(stream) = UnixStream::connect(&socket) else {
        eprintln!("passwords isn't answering at {}", socket.display());
        return 69;
    };
    let mut talk = Talk {
        reading: BufReader::new(
            stream
                .try_clone()
                .expect("a socket can be read and written"),
        ),
        writing: stream,
        next: 1,
    };
    let words = rest.join(" ");
    match asked {
        "find" | "list" => {
            let answer = if asked == "find" {
                talk.ask(json!({"ask": "list", "asked": true}));
                talk.ask(json!({"ask": "search", "words": words}))
            } else {
                talk.ask(json!({"ask": "list", "asked": true}))
            };
            print_entries(&answer);
            0
        }
        "show" | "copy" => {
            let Some((entry, detail)) = rest.split_first() else {
                eprintln!("{USAGE}");
                return 64;
            };
            let Some(key) = talk.find(entry) else {
                return 1;
            };
            let mut ask =
                json!({"ask": if asked == "show" { "reveal" } else { "copy" }, "key": key});
            if let Some(detail) = detail.first() {
                ask["detail"] = json!(detail);
            }
            let answer = talk.ask(ask);
            if let Some(failed) = failed(&answer) {
                return failed;
            }
            if asked == "show" {
                println!("{}", answer["value"].as_str().unwrap_or(""));
            } else {
                let left = answer["clears"].as_u64().unwrap_or(0).saturating_sub(now()) / 1000;
                println!(
                    "Copied its {}; the clipboard clears in {left} s.",
                    answer["detail"].as_str().unwrap_or("password")
                );
            }
            0
        }
        "code" => {
            let Some(key) = talk.find(&words) else {
                return 1;
            };
            let answer = talk.ask(json!({"ask": "code", "key": key}));
            if let Some(failed) = failed(&answer) {
                return failed;
            }
            println!(
                "{} ({} s left)",
                answer["code"].as_str().unwrap_or(""),
                answer["left"].as_u64().unwrap_or(0)
            );
            0
        }
        "let-in" => {
            let answer = talk.ask(json!({"ask": "let-in", "manager": words}));
            failed(&answer).unwrap_or_else(|| {
                println!("{words} let passwords in.");
                0
            })
        }
        "health" => {
            let answer = talk.ask(json!({"ask": "health"}));
            println!(
                "{}",
                serde_json::to_string_pretty(&answer).unwrap_or_default()
            );
            0
        }
        _ => {
            eprintln!("{USAGE}");
            64
        }
    }
}

/// One client's talk with the daemon over its socket.
struct Talk {
    reading: BufReader<UnixStream>,
    writing: UnixStream,
    next: u64,
}

impl Talk {
    /// Asks one thing and waits for its answer.
    fn ask(&mut self, mut asked: Value) -> Value {
        let id = self.next;
        self.next += 1;
        asked["id"] = json!(id);
        if writeln!(self.writing, "{asked}").is_err() {
            return json!({"error": {"code": "refused", "message": "the daemon went away"}});
        }
        let mut line = String::new();
        loop {
            line.clear();
            match self.reading.read_line(&mut line) {
                Ok(0) | Err(_) => {
                    return json!({"error": {"code": "refused", "message": "the daemon went away"}});
                }
                Ok(_) => {}
            }
            let Ok(answer) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if answer["id"] == json!(id) {
                return answer;
            }
        }
    }

    /// The key of the one entry these words name: a key as it is, else
    /// the one entry a search finds, else none, saying which it found.
    fn find(&mut self, words: &str) -> Option<String> {
        if named(words).is_some() {
            return Some(words.to_owned());
        }
        self.ask(json!({"ask": "list", "asked": true}));
        let answer = self.ask(json!({"ask": "search", "words": words}));
        let found = answer["records"].as_array().cloned().unwrap_or_default();
        match found.as_slice() {
            [one] => one["key"].as_str().map(str::to_owned),
            [] => {
                eprintln!("No entry is found by {words:?}.");
                None
            }
            many => {
                // The one titled exactly so, if just one is.
                let exact: Vec<&Value> = many
                    .iter()
                    .filter(|record| {
                        record["title"]
                            .as_str()
                            .is_some_and(|title| title.eq_ignore_ascii_case(words))
                    })
                    .collect();
                if let [one] = exact.as_slice() {
                    return one["key"].as_str().map(str::to_owned);
                }
                eprintln!("{} entries are found by {words:?}; say which:", many.len());
                print_entries(&json!({"records": many}));
                None
            }
        }
    }
}

/// Whether an answer failed, saying why and giving the exit status.
fn failed(answer: &Value) -> Option<i32> {
    let error = answer.get("error")?;
    eprintln!("{}", error["message"].as_str().unwrap_or("it failed"));
    Some(match error["code"].as_str() {
        Some("signed-out") => 77,
        Some("not-ready") => 75,
        _ => 1,
    })
}

/// Entries, one a line: the title, its kind, its username and website,
/// then its key, in columns when a person reads them, tabs when a program
/// does.
fn print_entries(answer: &Value) {
    let records = answer["records"].as_array().cloned().unwrap_or_default();
    let rows: Vec<[String; 5]> = records
        .iter()
        .map(|record| {
            let text = |name: &str| record[name].as_str().unwrap_or("").to_owned();
            let website = record["websites"]
                .as_array()
                .and_then(|websites| websites.first())
                .and_then(|website| website["address"].as_str())
                .unwrap_or("")
                .to_owned();
            [
                text("title"),
                text("category"),
                text("username"),
                website,
                text("key"),
            ]
        })
        .collect();
    if !std::io::stdout().is_terminal() {
        for row in rows {
            println!("{}", row.join("\t"));
        }
        return;
    }
    let mut widths = [0usize; 5];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .zip(widths)
            .map(|(cell, width)| format!("{cell:width$}"))
            .collect();
        println!("{}", cells.join("  ").trim_end());
    }
    if let Some(managers) = answer["sources"].as_object() {
        for (manager, standing) in managers {
            if standing["state"] != "answering" {
                eprintln!(
                    "{manager}: {} {}",
                    standing["state"].as_str().unwrap_or(""),
                    standing["reason"].as_str().unwrap_or("")
                );
            }
        }
    }
}
