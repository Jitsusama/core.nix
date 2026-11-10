//! passwords daemon SOCKET MANAGERS [WORDS]
//! passwords find WORDS | list | details ENTRY | show ENTRY [DETAIL]
//! passwords copy ENTRY [DETAIL] | code ENTRY | let-in MANAGER | health
//! passwords audit | keys | help [--json]
//!
//! Joel's passwords, from every password manager he keeps, held ready for
//! the shell and the command line over SOCKET in the shell's daemon
//! interface (lib.rs). Each manager in MANAGERS, a JSON file
//! `{"managers": [{"name", "command"}]}`, runs as a child on the passwords
//! line protocol: a JSON line in for each ask, `{"id", "ask"}` with `list`,
//! `get`, `reveal`, `code`, `audit` or `let-in`, and a JSON line out answering it by
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
//! Given the launcher's vocabulary, WORDS, it serves each entry's ready row
//! too, as kind `row` to list and watch: worded, ordered and with its acts
//! as rows.rs makes them, each watch told only the rows that changed, so
//! the launcher makes none of its own. Without it, or with a word it
//! lacks, it serves no rows and says why in the journal.
//!
//! An audit is the one ask that reads every password: asked by Joel alone
//! (`asked`), never at start, on a timer or with a list, it asks each
//! manager for its passwords (`audit`, answered `{"passwords": [{"entry",
//! "value"}]}`), hashes each as it's taken, and asks Have I Been Pwned, at
//! `$PASSWORDS_BREACHES` or its own address, by each hash's first five
//! characters alone (audit.rs). Only each entry's two counts outlive it:
//! the troubled entries' counts, and when the checks found trouble and
//! cleared, are kept in `$XDG_STATE_HOME/passwords/checked.json`, else
//! `~/.local/state`'s, readable by Joel alone and written whole through a
//! file beside it, so what the last check found stands through a reboot
//! (lib.rs's `Held::kept`). The `health` ask answers it as `checked`, and
//! each watch is told it as it changes.
//!
//! Run with any other word, it's the command line, asking the daemon at
//! `$PASSWORDS_SOCKET`, else `$XDG_RUNTIME_DIR/daemons/passwords.sock`, its
//! words, output and exit statuses as `passwords help` says (cli.rs). ENTRY
//! is an entry's key, its title, or words that find just one. zsh completes
//! it with what `passwords completion zsh` writes (completion.zsh), which
//! asks `passwords names` for what the daemon holds, and never wakes a
//! manager. `passwords keys` lists the keys 1Password's SSH agent holds,
//! through ssh-add, only when asked: ssh and git use the TPM's agent.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use passwords::{
    Ask, Changes, DOMAIN, Held, KIND, PATIENCE, ROW, Said, Standing, VERSION, ask, audit, cli,
    error, key, named, now, rows, said, standing,
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

/// The managers as rows name them, from the same file.
#[derive(Deserialize)]
struct Shown {
    managers: Vec<rows::Shown>,
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
    /// Whether it watches the entries' rows rather than them.
    rows: bool,
}

/// What rows are made with, and the last each row watch was told.
struct Rows {
    shown: Vec<rows::Shown>,
    words: Option<rows::Words>,
    told: HashMap<String, Value>,
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
    /// Whether an audit is running, so a second is refused rather than
    /// reading every password again beside it.
    auditing: bool,
    /// Where what the checks found is kept, or none where the account
    /// names no state folder.
    kept: Option<PathBuf>,
    rows: Rows,
}

/// The daemon's state, and what wakes an ask waiting on a manager to start.
type Shared = Arc<(Mutex<Daemon>, Condvar)>;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let typed = match cli::parse(&args) {
        Ok(typed) => typed,
        Err(why) => {
            eprintln!("passwords: {why}\n\n{}", cli::USAGE);
            std::process::exit(cli::MISTYPED);
        }
    };
    match typed.command {
        cli::Command::Daemon {
            socket,
            managers,
            words,
        } => daemon(&PathBuf::from(socket), &managers, words.as_deref()),
        command => std::process::exit(command_line(command, typed.json)),
    }
}

fn daemon(socket: &std::path::Path, managers: &str, words: Option<&str>) {
    let read = std::fs::read_to_string(managers).map_err(|error| error.to_string());
    let (managers, shown): (Vec<Manager>, Vec<rows::Shown>) = match read.and_then(|text| {
        let managers = serde_json::from_str::<Managers>(&text).map_err(|error| error.to_string());
        let shown = serde_json::from_str::<Shown>(&text).map_err(|error| error.to_string());
        Ok((managers?.managers, shown?.managers))
    }) {
        Ok(read) => read,
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
    let kept = kept_at();
    let mut held = Held::of(&names);
    if let Some(kept) = &kept {
        load(&mut held, kept);
    }
    let shared: Shared = Arc::new((
        Mutex::new(Daemon {
            held,
            inputs: HashMap::new(),
            pending: HashMap::new(),
            next: 1,
            watches: Vec::new(),
            took: VecDeque::new(),
            since: now(),
            copied: None,
            auditing: false,
            kept,
            rows: Rows {
                shown,
                words: words.and_then(vocabulary),
                told: HashMap::new(),
            },
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

/// The words rows are said in, from the launcher's vocabulary at `path`,
/// or none, saying why, where it can't be read or lacks one.
fn vocabulary(path: &str) -> Option<rows::Words> {
    let read = std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|error| error.to_string()))
        .and_then(|vocabulary| rows::Words::read(&vocabulary));
    read.map_err(|why| eprintln!("<3>serving no rows, {path}: {why}"))
        .ok()
}

/// Every entry's row as it stands, as a record of kind `row`.
fn rows_of(daemon: &Daemon) -> Vec<Value> {
    let Some(words) = &daemon.rows.words else {
        return Vec::new();
    };
    rows::rows(&daemon.held.records(), &daemon.rows.shown, words)
        .into_iter()
        .map(|row| json!({"key": row["key"], "kind": ROW, "row": row}))
        .collect()
}

/// Tells each row watch the rows that changed since it was last told,
/// and those gone: one entry changing can move every row's order.
fn tell_rows(daemon: &mut Daemon) {
    if !daemon.watches.iter().any(|watch| watch.rows) {
        return;
    }
    let now: HashMap<String, Value> = rows_of(daemon)
        .into_iter()
        .map(|row| (row["key"].as_str().unwrap_or_default().to_owned(), row))
        .collect();
    let changed: Vec<&Value> = now
        .iter()
        .filter(|(key, row)| daemon.rows.told.get(*key) != Some(*row))
        .map(|(_, row)| row)
        .collect();
    let gone: Vec<&String> = daemon
        .rows
        .told
        .keys()
        .filter(|key| !now.contains_key(*key))
        .collect();
    if !(changed.is_empty() && gone.is_empty()) {
        for watch in daemon.watches.iter().filter(|watch| watch.rows) {
            let line = json!({"id": watch.id, "changed": changed, "gone": gone});
            let _ = watch.told.send(line.to_string());
        }
    }
    daemon.rows.told = now;
}

/// Where what the checks found is kept: the account's state folder's
/// `passwords/checked.json`, `$XDG_STATE_HOME`, else `~/.local/state`.
fn kept_at() -> Option<PathBuf> {
    let absolute = |name: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let state = absolute("XDG_STATE_HOME")
        .or_else(|| absolute("HOME").map(|home| home.join(".local/state")))?;
    Some(state.join("passwords/checked.json"))
}

/// Takes what was kept of the checks before the daemon started, saying why
/// where it couldn't; none kept yet says nothing.
fn load(held: &mut Held, kept: &Path) {
    let text = match std::fs::read_to_string(kept) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return,
        Err(why) => {
            eprintln!("<4>couldn't read what the checks found: {why}");
            return;
        }
    };
    let read = serde_json::from_str::<Value>(&text).is_ok_and(|kept| held.load(&kept));
    if !read {
        eprintln!(
            "<4>what the checks found, at {}, couldn't be read",
            kept.display()
        );
    }
}

/// Writes what the checks found whole, readable by Joel alone: to a file
/// beside it first, renamed over it, so nothing reads half of one.
fn write_kept(kept: &Path, said: &Value) -> std::io::Result<()> {
    if let Some(folder) = kept.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let writing = kept.with_extension("json.writing");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&writing)?;
    // One left by a write that never ended keeps the mode it was made with.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(format!("{said}\n").as_bytes())?;
    file.sync_all()?;
    std::fs::rename(&writing, kept)
}

/// Keeps what the checks found once it has changed, and tells each watch,
/// so the shell's For You moves with it.
fn keep(daemon: &mut Daemon) {
    let Some(said) = daemon.held.unsaved() else {
        return;
    };
    if let Some(kept) = &daemon.kept
        && let Err(why) = write_kept(kept, &said)
    {
        eprintln!("<4>couldn't keep what the checks found: {why}");
    }
    let checked = daemon.held.checked();
    for watch in &daemon.watches {
        let line = json!({"id": watch.id, "checked": checked});
        let _ = watch.told.send(line.to_string());
    }
    // What a check found is said on each row it found troubled.
    tell_rows(daemon);
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
                for watch in daemon.watches.iter().filter(|watch| !watch.rows) {
                    let line = json!({"id": watch.id, "changed": records, "gone": changes.gone});
                    let _ = watch.told.send(line.to_string());
                }
                tell_rows(&mut daemon);
            }
            // A list finding an entry changed or gone takes what a check
            // found of it.
            keep(&mut daemon);
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
                   "entries": daemon.held.records().len(), "took": daemon.took,
                   "checked": daemon.held.checked()})
        }
        Ask::List {
            id,
            limit,
            fresh,
            asked,
            held,
            patience,
            rows,
        } => {
            if !held {
                list(shared, fresh, asked, patience);
            }
            listed(shared, &id, limit, rows)
        }
        Ask::Get {
            id,
            keys,
            asked,
            held,
            patience,
        } if !held => get(shared, &id, &keys, asked, patience),
        Ask::Get { id, keys, .. } => held_records(shared, &id, &keys),
        Ask::Watch { id, rows } => {
            watch(
                shared,
                Watch {
                    connection,
                    id,
                    told: told.clone(),
                    rows,
                },
            );
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
            Ok(secret) => secret.lasting(json!({"id": id, "value": secret.value,
                                                "detail": secret.label})),
            Err((code, message)) => error(&id, code, &message),
        },
        Ask::Copy { id, key, detail } => match secret(shared, &key, detail.as_deref()) {
            Ok(secret) => match copy(shared, &secret.value) {
                Ok(clears) => secret.lasting(json!({"id": id, "copied": true,
                                                    "detail": secret.label, "clears": clears})),
                Err(why) => error(&id, "refused", &why),
            },
            Err((code, message)) => error(&id, code, &message),
        },
        Ask::Code { id, key } => code(shared, &id, &key),
        Ask::Audit { id, asked } => audit_all(shared, &id, asked),
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
            &format!("this daemon keeps entries and their rows, not {kind}"),
        ),
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("can't answer {ask:?}")),
    }
}

/// Starts a watch: what's held now, the entries or their rows, then each
/// change as it comes.
fn watch(shared: &Shared, watch: Watch) {
    let mut daemon = lock(shared);
    let records = if watch.rows {
        let told = rows_of(&daemon);
        daemon.rows.told = told
            .iter()
            .map(|row| {
                (
                    row["key"].as_str().unwrap_or_default().to_owned(),
                    row.clone(),
                )
            })
            .collect();
        told
    } else {
        daemon.held.records()
    };
    let _ = watch.told.send(
        json!({"id": watch.id, "records": records,
               "sources": daemon.held.standings(), "state": daemon.held.state()})
        .to_string(),
    );
    daemon.watches.push(watch);
}

/// Every entry held, or its row where `rows` asks, at most `limit`, and
/// how each manager stands.
fn listed(shared: &Shared, id: &Value, limit: Option<usize>, rows: bool) -> Value {
    let daemon = lock(shared);
    let mut records = if rows {
        rows_of(&daemon)
    } else {
        daemon.held.records()
    };
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    json!({"id": id, "records": records, "sources": daemon.held.standings()})
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
    held_records(shared, id, keys)
}

/// Entries by key as they're held now, whether in full or in summary.
fn held_records(shared: &Shared, id: &Value, keys: &[String]) -> Value {
    let daemon = lock(shared);
    let records: Vec<Value> = keys
        .iter()
        .filter_map(|key| daemon.held.record(key))
        .collect();
    json!({"id": id, "records": records, "sources": daemon.held.standings()})
}

/// A detail's value as its manager revealed it, its label, and, for a
/// one-time code, its period and the seconds left in it. Not `Secret`,
/// which names which way the command sends one.
struct Revealed {
    value: String,
    label: String,
    lasts: Option<(Value, Value)>,
}

impl Revealed {
    /// An answer with how long a one-time code lasts beside it, so a
    /// client can say when the code it was given lapses.
    fn lasting(&self, mut answer: Value) -> Value {
        if let Some((period, left)) = &self.lasts {
            answer["period"] = period.clone();
            answer["left"] = left.clone();
        }
        answer
    }
}

/// One detail's value and its label, its secret included, asked of its
/// manager; the password when none is named. A one-time code's detail
/// gives the code it makes now, asked as `code` is, never its seed: the
/// seed makes every code after, so it's a secret above the password's,
/// and nothing Joel does with a code needs it. Never kept.
fn secret(
    shared: &Shared,
    key: &str,
    wanted: Option<&str>,
) -> Result<Revealed, (&'static str, String)> {
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
    let (detail, label, coded) = {
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
        let coded = detail.get("type").and_then(Value::as_str) == Some("code");
        (id, label, coded)
    };
    if coded {
        let answer = ask_of(
            shared,
            manager,
            asking("code", &[("entry", json!(entry))]),
            PATIENCE * 6,
        )?;
        let Some(code) = answer.get("code").and_then(Value::as_str) else {
            return Err(("refused", "its manager gave no code".to_owned()));
        };
        let lasts = |name: &str| answer.get(name).cloned().unwrap_or(Value::Null);
        return Ok(Revealed {
            value: code.to_owned(),
            label,
            lasts: Some((lasts("period"), lasts("left"))),
        });
    }
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
        Some(value) => Ok(Revealed {
            value: value.to_owned(),
            label,
            lasts: None,
        }),
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

/// How long a manager may take to give every password: 1Password asks
/// Joel to let it in first where its leave has lapsed, as letting in does.
const AUDIT_PATIENCE: u64 = 120_000;

/// Audits every password of every manager, Joel having asked (`asked`),
/// one audit at a time: each manager's passwords read at once, each
/// hashed as it's taken (audit.rs), and every entry's health told to each
/// watch. Answered with how many were audited, how many entries are
/// reused and breached, and each entry with a problem; never a secret.
fn audit_all(shared: &Shared, id: &Value, asked: bool) -> Value {
    if !asked {
        return error(id, "refused", "passwords are audited only when you ask");
    }
    {
        let mut daemon = lock(shared);
        if daemon.auditing {
            return error(id, "not-ready", "every password is being audited already");
        }
        daemon.auditing = true;
    }
    let answer = audit_read(shared, id);
    lock(shared).auditing = false;
    answer
}

fn audit_read(shared: &Shared, id: &Value) -> Value {
    // What's audited is said of the entries held, so a manager not listed
    // yet is listed first, as Joel's own ask.
    list(shared, false, true, AUDIT_PATIENCE);
    let managers: Vec<String> = lock(shared).held.managers().to_vec();
    let reading: Vec<_> = managers
        .into_iter()
        .map(|manager| {
            let shared = Arc::clone(shared);
            std::thread::spawn(move || {
                let asked = asking("audit", &[("asked", json!(true))]);
                let answer = ask_of(&shared, &manager, asked, AUDIT_PATIENCE);
                (manager, answer)
            })
        })
        .collect();
    let mut passwords = Vec::new();
    let mut read = Vec::new();
    let mut first_failure = None;
    for thread in reading {
        let Ok((manager, answer)) = thread.join() else {
            continue;
        };
        match answer {
            Ok(mut answer) => {
                // Each password is moved, never copied, into the audit,
                // which hashes it and drops it.
                if let Some(Value::Array(given)) = answer.remove("passwords") {
                    for one in given {
                        let Value::Object(mut one) = one else {
                            continue;
                        };
                        if let (Some(Value::String(entry)), Some(Value::String(value))) =
                            (one.remove("entry"), one.remove("value"))
                            && !value.is_empty()
                        {
                            passwords.push((key(&manager, &entry), value));
                        }
                    }
                }
                read.push(manager);
            }
            Err(failed) => {
                first_failure.get_or_insert(failed);
            }
        }
    }
    if read.is_empty() {
        let (code, message) =
            first_failure.unwrap_or(("refused", "no password manager could be read".to_owned()));
        return error(id, code, &message);
    }
    let audited = passwords.len();
    let asked_at =
        std::env::var("PASSWORDS_BREACHES").unwrap_or_else(|_| audit::BREACHES.to_owned());
    let found = audit::audit(passwords, &asked_at);
    let mut daemon = lock(shared);
    let whole = found.unreached.is_none();
    let changed = daemon.held.audited(&read, found.health, now(), whole);
    let records: Vec<Value> = changed
        .iter()
        .filter_map(|key| daemon.held.record(key))
        .collect();
    if !records.is_empty() {
        for watch in daemon.watches.iter().filter(|watch| !watch.rows) {
            let line = json!({"id": watch.id, "changed": records, "gone": []});
            let _ = watch.told.send(line.to_string());
        }
        tell_rows(&mut daemon);
    }
    keep(&mut daemon);
    let troubled: Vec<Value> = daemon
        .held
        .records()
        .into_iter()
        .filter(|record| {
            record["key"]
                .as_str()
                .and_then(|key| daemon.held.health(key))
                .is_some_and(audit::Health::troubled)
        })
        .collect();
    let reused = troubled
        .iter()
        .filter(|record| record["health"]["reused"].as_u64().unwrap_or(0) > 0)
        .count();
    let breached = troubled
        .iter()
        .filter(|record| record["health"]["breached"].as_u64().unwrap_or(0) > 0)
        .count();
    let mut answer = json!({"id": id, "audited": audited, "reused": reused,
                            "breached": breached, "records": troubled,
                            "sources": daemon.held.standings()});
    if let Some(why) = found.unreached {
        answer["breached"] = Value::Null;
        answer["unreached"] = json!(why);
    }
    answer
}

/// The command line: one command, its answer set for a person at a
/// terminal or for a program reading a pipe, or as `--json`, and its exit
/// status as the usage says.
fn command_line(command: cli::Command, json: bool) -> i32 {
    let terminal = std::io::stdout().is_terminal();
    match command {
        cli::Command::Help => {
            say(cli::USAGE);
            return 0;
        }
        cli::Command::Completion => {
            say(include_str!("completion.zsh"));
            return 0;
        }
        cli::Command::Keys => return keys(json, terminal),
        _ => {}
    }
    let Some(mut talk) = Talk::open(matches!(command, cli::Command::Names(_))) else {
        return cli::UNANSWERED;
    };
    match command {
        cli::Command::Find(words) => talk.found(&words, json, terminal),
        cli::Command::List => {
            let answer = talk.ask(json!({"ask": "list", "asked": true}));
            entries(&answer, json, terminal);
            0
        }
        cli::Command::Details(words) => {
            let Some(key) = talk.find(&words) else {
                return cli::FAILED;
            };
            match talk.entry(&key) {
                Ok(record) if json => say(&cli::as_json(&record)),
                Ok(record) => say(&cli::details(&record, terminal)),
                Err(failed) => return failed,
            }
            0
        }
        cli::Command::Show { entry, detail } => {
            talk.secret(&entry, detail.as_deref(), Secret::Show, json, terminal)
        }
        cli::Command::Copy { entry, detail } => {
            talk.secret(&entry, detail.as_deref(), Secret::Copy, json, terminal)
        }
        cli::Command::Code(words) => {
            let Some(key) = talk.find(&words) else {
                return cli::FAILED;
            };
            let answer = talk.ask(json!({"ask": "code", "key": key}));
            if let Some(failed) = failed(&answer) {
                return failed;
            }
            say(&if json {
                cli::as_json(&answer)
            } else {
                cli::code(&answer, terminal)
            });
            0
        }
        cli::Command::LetIn(manager) => {
            // Letting in waits on Joel, for as long as he takes to answer.
            let answer =
                talk.ask(json!({"ask": "let-in", "manager": manager, "patience": 120_000}));
            if let Some(failed) = failed(&answer) {
                return failed;
            }
            if json {
                say(&cli::as_json(&answer));
            } else if terminal {
                say(&format!("{manager} let passwords in.\n"));
            }
            0
        }
        cli::Command::Health => {
            let answer = talk.ask(json!({"ask": "health"}));
            say(&if json {
                cli::as_json(&answer)
            } else {
                cli::health(&answer, terminal)
            });
            0
        }
        cli::Command::Audit => talk.audit(json, terminal),
        cli::Command::Names(names) => {
            talk.names(&names);
            0
        }
        cli::Command::Daemon { .. }
        | cli::Command::Help
        | cli::Command::Completion
        | cli::Command::Keys => unreachable!("answered before the daemon was asked"),
    }
}

/// Writes what the command says, saying nothing more if whatever reads it
/// has gone, as `head` goes once it has its lines.
fn say(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes()).and_then(|()| out.flush());
}

/// Entries from a list or a search, as `--json`, a person or a program reads
/// them; a program is told on standard error of a manager not answering,
/// which a person reads in the head.
fn entries(answer: &Value, json: bool, terminal: bool) {
    if json {
        say(&cli::as_json(&answer["records"]));
        return;
    }
    say(&cli::entries(answer, terminal));
    if !terminal {
        for said in cli::standings(&answer["sources"]) {
            eprintln!("{said}");
        }
    }
}

/// ssh-add and ssh-keygen as the package names them, else as the PATH has
/// them.
const SSH_ADD: &str = match option_env!("PASSWORDS_SSH_ADD") {
    Some(program) => program,
    None => "ssh-add",
};
const SSH_KEYGEN: &str = match option_env!("PASSWORDS_SSH_KEYGEN") {
    Some(program) => program,
    None => "ssh-keygen",
};

/// The keys 1Password's SSH agent holds, read from its socket only when
/// Joel asks. ssh and git sign with the TPM's agent, never this one, so
/// nothing else here reaches it. Listing keys reads only their public
/// halves; nothing is signed and no key is added or taken away.
fn keys(json: bool, terminal: bool) -> i32 {
    let home = std::env::var_os("HOME").unwrap_or_default();
    let socket = PathBuf::from(home).join(".1password/agent.sock");
    let unanswered = |said: &str| {
        eprintln!(
            "1Password's SSH agent isn't answering at {}{said}; 1Password's Settings, under \
             Developer, turns it on.",
            socket.display()
        );
        cli::UNANSWERED
    };
    if !socket.exists() {
        return unanswered("");
    }
    let listed = match Command::new(SSH_ADD)
        .arg("-L")
        .env("SSH_AUTH_SOCK", &socket)
        .stdin(Stdio::null())
        .output()
    {
        Ok(listed) => listed,
        Err(why) => {
            eprintln!("couldn't run ssh-add: {why}");
            return cli::FAILED;
        }
    };
    let public = String::from_utf8_lossy(&listed.stdout).into_owned();
    let refused = String::from_utf8_lossy(&listed.stderr).trim().to_owned();
    let keys = match listed.status.code() {
        Some(0) => match fingerprints(&public) {
            Ok(fingerprints) => cli::keys(&public, &fingerprints),
            Err(why) => {
                eprintln!("couldn't read the keys' fingerprints: {why}");
                return cli::FAILED;
            }
        },
        // ssh-add says on standard output that an agent holds no key, and
        // on standard error why it couldn't list them.
        Some(1) if refused.is_empty() => Vec::new(),
        Some(1) => {
            eprintln!("1Password's SSH agent didn't list its keys: {refused}");
            return cli::FAILED;
        }
        _ => return unanswered(&format!(" ({refused})")),
    };
    say(&if json {
        cli::as_json(&Value::Array(keys))
    } else {
        cli::keys_text(&keys, terminal)
    });
    0
}

/// What `ssh-keygen -l` says of each public key, a line each, in order.
fn fingerprints(public: &str) -> std::io::Result<String> {
    let mut child = Command::new(SSH_KEYGEN)
        .args(["-l", "-E", "sha256", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(public.as_bytes())?;
    }
    let said = child.wait_with_output()?;
    if !said.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&said.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&said.stdout).into_owned())
}

/// One client's talk with the daemon over its socket.
struct Talk {
    reading: BufReader<UnixStream>,
    writing: UnixStream,
    next: u64,
}

impl Talk {
    /// A talk with the daemon at `$PASSWORDS_SOCKET`, else in the account's
    /// runtime folder, saying so if it isn't answering, unless it's for
    /// completion, which says nothing and waits only a moment: it reads
    /// what's held alone, which answers at once, so a Tab is never held up.
    fn open(completing: bool) -> Option<Talk> {
        let socket = std::env::var("PASSWORDS_SOCKET").map_or_else(
            |_| {
                let runtime =
                    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
                PathBuf::from(runtime).join("daemons/passwords.sock")
            },
            PathBuf::from,
        );
        let Ok(stream) = UnixStream::connect(&socket) else {
            if !completing {
                eprintln!("passwords isn't answering at {}", socket.display());
            }
            return None;
        };
        if completing {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        }
        let reading = stream.try_clone().ok()?;
        Some(Talk {
            reading: BufReader::new(reading),
            writing: stream,
            next: 1,
        })
    }

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

    /// The entries some words find, every one read first, as Joel asked;
    /// failing when they find none.
    fn found(&mut self, words: &str, json: bool, terminal: bool) -> i32 {
        let listed = self.ask(json!({"ask": "list", "asked": true}));
        let mut answer = self.ask(json!({"ask": "search", "words": words, "limit": cli::EVERY}));
        if let Some(failed) = failed(&answer) {
            return failed;
        }
        // A search says nothing of the managers; the list it follows does.
        answer["sources"] = listed["sources"].clone();
        entries(&answer, json, terminal);
        if answer["records"].as_array().is_some_and(Vec::is_empty) {
            eprintln!("No entry is found by {words:?}.");
            return cli::FAILED;
        }
        0
    }

    /// The key of the one entry these words name: a key as it is, else
    /// the one entry a search finds, or the one of them titled so, else
    /// none, saying what it found. Joel typed it, so a manager not read
    /// yet may ask him to let passwords in.
    fn find(&mut self, words: &str) -> Option<String> {
        if named(words).is_some() {
            return Some(words.to_owned());
        }
        self.ask(json!({"ask": "list", "asked": true}));
        let answer = self.ask(json!({"ask": "search", "words": words, "limit": cli::EVERY}));
        let found = answer["records"].as_array().cloned().unwrap_or_default();
        match cli::resolve(words, &found) {
            cli::Resolved::One(key) => Some(key),
            cli::Resolved::None => {
                eprintln!("No entry is found by {words:?}.");
                None
            }
            cli::Resolved::Many(many) => {
                eprintln!("{words:?} finds more than one entry; name one by its title or key:");
                eprint!(
                    "{}",
                    cli::entries(&json!({"records": many}), std::io::stderr().is_terminal())
                );
                None
            }
        }
    }

    /// One entry in full, every detail but no hidden one's value, read
    /// from its manager if only its summary is held; or the exit status,
    /// having said why.
    fn entry(&mut self, key: &str) -> Result<Value, i32> {
        let answer = self.ask(json!({"ask": "get", "keys": [key], "asked": true}));
        if let Some(failed) = failed(&answer) {
            return Err(failed);
        }
        match answer["records"].get(0) {
            Some(record) if record["details"].is_array() => Ok(record.clone()),
            Some(_) => {
                for said in cli::standings(&answer["sources"]) {
                    eprintln!("{said}");
                }
                eprintln!("Its details couldn't be read.");
                Err(cli::FAILED)
            }
            None => {
                eprintln!("No entry is held as {key}.");
                Err(cli::FAILED)
            }
        }
    }

    /// A detail's value shown or copied, the password unless one is named
    /// by its label or id.
    fn secret(
        &mut self,
        entry: &str,
        detail: Option<&str>,
        secret: Secret,
        json: bool,
        terminal: bool,
    ) -> i32 {
        let Some(key) = self.find(entry) else {
            return cli::FAILED;
        };
        let asked = match secret {
            Secret::Show => "reveal",
            Secret::Copy => "copy",
        };
        let mut ask = json!({"ask": asked, "key": key});
        if let Some(detail) = detail {
            let named = self.entry(&key).and_then(|record| {
                cli::detail_id(&record, detail).map_err(|why| {
                    eprintln!("{why}");
                    cli::FAILED
                })
            });
            match named {
                Ok(id) => ask["detail"] = json!(id),
                Err(failed) => return failed,
            }
        }
        let answer = self.ask(ask);
        if let Some(failed) = failed(&answer) {
            return failed;
        }
        say(&match (json, secret) {
            (true, _) => cli::as_json(&answer),
            (false, Secret::Show) => format!("{}\n", answer["value"].as_str().unwrap_or("")),
            (false, Secret::Copy) => cli::copied(&answer, now(), terminal),
        });
        0
    }

    /// Every password audited, as Joel asked, and each entry with a
    /// problem said; a program is told on standard error of a manager not
    /// answering, and anyone of breaches that couldn't be asked about.
    fn audit(&mut self, json: bool, terminal: bool) -> i32 {
        // Reading every password may wait on Joel letting a manager in.
        let answer = self.ask(json!({"ask": "audit", "asked": true}));
        if let Some(failed) = failed(&answer) {
            return failed;
        }
        say(&if json {
            cli::as_json(&answer)
        } else {
            cli::audit(&answer, terminal)
        });
        if !terminal {
            for said in cli::standings(&answer["sources"]) {
                eprintln!("{said}");
            }
        }
        if let Some(why) = answer["unreached"].as_str() {
            eprintln!("{why}; no password was checked for breaches.");
        }
        0
    }

    /// What zsh's completion offers, from what the daemon holds alone: no
    /// manager is asked anything, so a Tab never makes one ask Joel's
    /// leave. Saying nothing is enough when nothing's held or found.
    fn names(&mut self, names: &cli::Names) {
        let offered = match names {
            cli::Names::Entries => {
                let answer = self.ask(json!({"ask": "list", "held": true}));
                let records = answer["records"].as_array().cloned().unwrap_or_default();
                cli::entry_names(&records)
            }
            cli::Names::Details(entry) => {
                let key = if named(entry).is_some() {
                    entry.clone()
                } else {
                    // A search reads only what's held.
                    let answer =
                        self.ask(json!({"ask": "search", "words": entry, "limit": cli::EVERY}));
                    let found = answer["records"].as_array().cloned().unwrap_or_default();
                    let cli::Resolved::One(key) = cli::resolve(entry, &found) else {
                        return;
                    };
                    key
                };
                let answer = self.ask(json!({"ask": "get", "keys": [key], "held": true}));
                answer["records"]
                    .get(0)
                    .map(cli::detail_names)
                    .unwrap_or_default()
            }
            cli::Names::Managers => {
                let answer = self.ask(json!({"ask": "hello"}));
                cli::manager_names(&answer["sources"])
            }
        };
        say(&cli::for_zsh(&offered));
    }
}

/// Which way a secret goes: to the terminal, or onto the clipboard.
#[derive(Clone, Copy)]
enum Secret {
    Show,
    Copy,
}

/// Whether an answer failed, saying why and giving the exit status.
fn failed(answer: &Value) -> Option<i32> {
    let error = answer.get("error")?;
    eprintln!("{}", error["message"].as_str().unwrap_or("it failed"));
    Some(cli::status(error))
}
