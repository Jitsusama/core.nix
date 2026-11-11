//! passwords daemon MANAGERS [WORDS]
//! passwords keys | help [--json]
//!
//! Joel's passwords, from every password manager he keeps, held ready for
//! the shell, which runs the daemon as its own child and asks it over its
//! standard input and output in the shell's daemon interface (lib.rs). It
//! has no socket: every agent runs as Joel, so a socket any of them could
//! reach would hand them what a manager let the daemon read (2026-11-10,
//! Joel: "we can't have a password backdoor here"). A manager such as
//! 1Password lets in each process tree apart, so one an agent starts must
//! ask Joel itself. It keeps its memory from the account's other processes
//! and ends with the shell (`harden`), and it ends once the shell closes
//! its end. Each manager in MANAGERS, a JSON file
//! `{"managers": [{"name", "command"}]}`, runs as a child on the passwords
//! line protocol: a JSON line in for each ask, `{"id", "ask"}` with `list`,
//! `get`, `reveal`, `code`, `audit` or `let-in`, and a JSON line out answering it by
//! its id, or `{"state", "reason"}` unasked as how it stands changes. A
//! manager that stops is started again. How each manager stands goes to
//! the shell as the interface's `sources`, the word every daemon's client
//! (Daemon.qml) reads, though the screen calls them password managers.
//!
//! Nothing is asked of a manager but what the shell asks for, so with
//! nobody asking, nothing runs but the managers waiting on input, and a
//! manager that has to ask Joel's leave, as 1Password's command does, only
//! ever asks it in answer to his own key. What's held is each entry
//! without its secrets (lib.rs); a secret goes from its manager to the
//! ask that wanted to see it, or to the clipboard, marked sensitive and
//! cleared after CLEARS, and is never kept, written or logged here. Copying
//! is off while `$PASSWORDS_COPIER` names no copier, which nothing names
//! until agents are fenced off the clipboard any of them can read.
//!
//! Given the launcher's vocabulary, WORDS, it serves each entry's ready row
//! too, as kind `row` to list and watch: worded, ordered and with its acts
//! as rows.rs makes them, each watch told only the rows that changed, so
//! the launcher makes none of its own. Each row's drawer is kind
//! `drawer`, listed and watched by the keys the rows name, so only those
//! shown are made and told. Without it, or with a word it lacks, it
//! serves no rows or drawers and says why in the journal.
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
//! Run with any other word, it's the command line, which reaches no daemon
//! and so no secret, as `passwords help` says (cli.rs), completed in zsh by
//! what `passwords completion zsh` writes (completion.zsh). `passwords
//! keys` lists the public keys 1Password's SSH agent holds, through
//! ssh-add, only when asked: ssh and git use the TPM's agent.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use passwords::{
    Ask, Changes, DOMAIN, DRAWER, Held, KIND, Of, PATIENCE, Said, Standing, VERSION, ask, audit,
    cli, error, key, named, now, rows, said, standing,
};
use ready_rows::kept::Kept;
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

/// One watch the shell holds.
struct Watch {
    id: Value,
    told: Sender<String>,
    /// Whether it watches the entries, their rows or some drawers.
    of: Of,
}

impl Watch {
    /// Whether a row or drawer by this key is one it watches.
    fn hears(&self, key: &str) -> bool {
        hears(&self.of, key)
    }
}

/// Whether a row or drawer by this key is among those `of` names.
fn hears(of: &Of, key: &str) -> bool {
    match of {
        Of::Entries => false,
        Of::Rows => key.starts_with("row:"),
        Of::Drawers(keys) => keys.iter().any(|each| each == key),
    }
}

/// What rows are made with, and the rows and drawers as last told, each
/// row with its spaced order.
struct Rows {
    shown: Vec<rows::Shown>,
    words: Option<rows::Words>,
    kept: Kept,
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
        cli::Command::Daemon { managers, words } => daemon(&managers, words.as_deref()),
        command => std::process::exit(command_line(&command, typed.json)),
    }
}

fn daemon(managers: &str, words: Option<&str>) {
    harden();
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
                kept: Kept::default(),
            },
        }),
        Condvar::new(),
    ));
    for manager in managers {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || run(&shared, &manager));
    }
    serve(&shared, std::io::stdin().lock(), std::io::stdout());
    // The shell that asked has gone, or closed its end: nothing is left to
    // answer, and its managers end with it (`run`).
    std::process::exit(0);
}

/// Keeps what it holds from the account's other processes as far as one
/// uid lets it, before any secret is near: none may read its memory or
/// its descriptors through /proc or ptrace, as its dumpable flag off
/// bars them, no core dump writes one, and it ends as the shell that ran
/// it does, so no orphan keeps a password manager's leave. The same uid
/// can still open the shell's own end of its pipes, so this closes nothing
/// alone: agents fenced off Joel's processes do.
fn harden() {
    // SAFETY: getppid, prctl and setrlimit read only the integers and the
    // limit given here, and change only this process.
    let (parent, hardened) = unsafe {
        let parent = libc::getppid();
        let none = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let hardened = libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) == 0
            && libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0) == 0
            && libc::setrlimit(libc::RLIMIT_CORE, &raw const none) == 0;
        (parent, hardened)
    };
    if !hardened {
        eprintln!(
            "<3>couldn't keep its memory from the account: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(71);
    }
    // The shell went before it was told to end with it.
    // SAFETY: getppid only reads this process's parent.
    if unsafe { libc::getppid() } != parent {
        std::process::exit(0);
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

/// Every entry's row as it stands, in the order shown, for what's kept
/// to give each its order.
fn rows_of(daemon: &Daemon) -> Vec<Value> {
    rows::rows(
        &daemon.held.records(),
        &daemon.rows.shown,
        &daemon.rows.kept,
    )
}

/// The drawers by these keys as they stand, as records of kind `drawer`,
/// made only for the entries they name.
fn drawers_of(daemon: &Daemon, keys: &[String]) -> Vec<Value> {
    let Some(words) = &daemon.rows.words else {
        return Vec::new();
    };
    let shown: Vec<Value> = daemon
        .held
        .records()
        .into_iter()
        .filter(|record| {
            keys.contains(&rows::drawer_key(
                record["key"].as_str().unwrap_or_default(),
            ))
        })
        .collect();
    rows::drawers(&shown, &daemon.rows.shown, words)
        .into_iter()
        .map(|(key, drawer)| json!({"key": key, "kind": DRAWER, "drawer": drawer}))
        .collect()
}

/// The entries, or the rows and drawers `of` names as they stand now,
/// those told to every watch first where they changed, so what's kept is
/// what's answered and the next change is told against it.
fn made_of(daemon: &mut Daemon, of: &Of) -> Vec<Value> {
    if *of == Of::Entries {
        return daemon.held.records();
    }
    tell_rows(daemon, Some(of));
    let mut made: Vec<Value> = daemon
        .rows
        .kept
        .all()
        .filter(|record| hears(of, record["key"].as_str().unwrap_or_default()))
        .cloned()
        .collect();
    // Kept by key, rows are answered in their order, so a limit keeps the
    // first shown.
    made.sort_by_key(|record| record["row"]["order"].as_i64());
    made
}

/// Tells each row or drawer watch what it watches that changed since it
/// was last told, and what's gone: one entry moving is its row alone,
/// its order spaced between its neighbours', and a drawer changes with
/// its entry. Nothing is made that nobody watches nor `also` names, and a
/// watch nothing changed for is told nothing.
fn tell_rows(daemon: &mut Daemon, also: Option<&Of>) {
    let wanted: Vec<&Of> = daemon
        .watches
        .iter()
        .map(|watch| &watch.of)
        .chain(also)
        .collect();
    let rowing = wanted.iter().any(|of| **of == Of::Rows);
    let drawing: Vec<String> = wanted
        .iter()
        .filter_map(|of| match of {
            Of::Drawers(keys) => Some(keys.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    if !rowing && drawing.is_empty() {
        // Nobody is told against it, so the next watch starts it afresh.
        daemon.rows.kept.let_go();
        return;
    }
    let made = if rowing { rows_of(daemon) } else { Vec::new() };
    let beside = drawers_of(daemon, &drawing);
    let moved = daemon.rows.kept.take(made, beside);
    for watch in &daemon.watches {
        let heard: Vec<&Value> = moved
            .changed
            .iter()
            .filter(|record| watch.hears(record["key"].as_str().unwrap_or_default()))
            .collect();
        let ended: Vec<&String> = moved.gone.iter().filter(|key| watch.hears(key)).collect();
        if !(heard.is_empty() && ended.is_empty()) {
            let line = json!({"id": watch.id, "changed": heard, "gone": ended});
            let _ = watch.told.send(line.to_string());
        }
    }
}

/// Refuses a list or watch of rows or drawers where the launcher's words
/// weren't read, since rows are served only in them, whole, saying so
/// rather than answering none.
fn wordless(shared: &Shared, asked: &Ask) -> Option<Value> {
    let (Ask::List { id, of, .. } | Ask::Watch { id, of }) = asked else {
        return None;
    };
    (*of != Of::Entries && lock(shared).rows.words.is_none()).then(|| {
        error(
            id,
            "no-words",
            "rows are said only in the launcher's words, and they didn't read",
        )
    })
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
    tell_rows(daemon, None);
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
        let mut command = Command::new(program);
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        // It ends as this thread does, which runs as long as the daemon:
        // a manager left running would keep its leave with nobody asking.
        // SAFETY: prctl is async-signal-safe and touches only the child.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
        match command.spawn() {
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
                for watch in daemon
                    .watches
                    .iter()
                    .filter(|watch| watch.of == Of::Entries)
                {
                    let line = json!({"id": watch.id, "changed": records, "gone": changes.gone});
                    let _ = watch.told.send(line.to_string());
                }
                tell_rows(&mut daemon, None);
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

/// Serves the shell, the one client, on the pipes it ran this with: each
/// line an ask, answered on its own thread so a slow manager holds up only
/// that ask, every answer written in turn, until the shell closes its end.
fn serve(shared: &Shared, reading: impl BufRead, writing: impl Write + Send + 'static) {
    let (told, lines) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut writing = writing;
        for line in lines {
            if writeln!(writing, "{line}")
                .and_then(|()| writing.flush())
                .is_err()
            {
                break;
            }
        }
    });
    for line in reading.lines() {
        let Ok(line) = line else { break };
        let shared = Arc::clone(shared);
        let told = told.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let asked = ask(&line);
            let answer = wordless(&shared, &asked).unwrap_or_else(|| answer(&shared, asked, &told));
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
    lock(shared).watches.clear();
}

/// Answers one ask; null for one whose answers go by its watch alone.
fn answer(shared: &Shared, asked: Ask, told: &Sender<String>) -> Value {
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
            of,
        } => {
            if !held {
                list(shared, fresh, asked, patience);
            }
            listed(shared, &id, limit, &of)
        }
        Ask::Get {
            id,
            keys,
            asked,
            held,
            patience,
        } if !held => get(shared, &id, &keys, asked, patience),
        Ask::Get { id, keys, .. } => held_records(shared, &id, &keys),
        Ask::Watch { id, of } => {
            watch(
                shared,
                Watch {
                    id,
                    told: told.clone(),
                    of,
                },
            );
            Value::Null
        }
        Ask::Unwatch { id } => {
            lock(shared).watches.retain(|watch| watch.id != id);
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
        // Refused before any secret is read, while no copier is named.
        Ask::Copy { id, .. } if copier().is_none() => error(&id, "refused", COPY_OFF),
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
    // What it's told now is kept, and the next change told against it.
    let records = made_of(&mut daemon, &watch.of);
    let _ = watch.told.send(
        json!({"id": watch.id, "records": records,
               "sources": daemon.held.standings(), "state": daemon.held.state()})
        .to_string(),
    );
    daemon.watches.push(watch);
}

/// Every entry held, or its row or the drawers where `of` asks, at most
/// `limit`, and how each manager stands.
fn listed(shared: &Shared, id: &Value, limit: Option<usize>, of: &Of) -> Value {
    let mut daemon = lock(shared);
    let mut records = made_of(&mut daemon, of);
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

/// Why nothing is copied: every agent runs as Joel, and niri lets any
/// client of his read the clipboard, so a copied secret would reach any
/// of them until agents are fenced off it (2026-11-10). A browser's own
/// extension of the manager fills sign-ins without one.
const COPY_OFF: &str =
    "copying is off while it's made safe; use your password manager's browser extension";

/// The program that holds a copy on the clipboard, `$PASSWORDS_COPIER`,
/// or none while copying is off: nothing names it until agents are fenced
/// off the clipboard, so nothing is copied before then.
fn copier() -> Option<String> {
    std::env::var("PASSWORDS_COPIER")
        .ok()
        .filter(|program| !program.is_empty())
}

/// Puts a secret on the clipboard through wl-copy, marked sensitive so a
/// clipboard history leaves it out, held by a wl-copy in the foreground
/// that's ended when its time is up, which clears it; one copied over
/// meanwhile has ended it already, so what was copied since stays.
fn copy(shared: &Shared, value: &str) -> Result<u64, String> {
    let program = copier().ok_or_else(|| COPY_OFF.to_owned())?;
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
        for watch in daemon
            .watches
            .iter()
            .filter(|watch| watch.of == Of::Entries)
        {
            let line = json!({"id": watch.id, "changed": records, "gone": []});
            let _ = watch.told.send(line.to_string());
        }
        tell_rows(&mut daemon, None);
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
/// status as the usage says. None reaches the daemon, which answers the
/// shell alone.
fn command_line(command: &cli::Command, json: bool) -> i32 {
    let terminal = std::io::stdout().is_terminal();
    match command {
        cli::Command::Help => {
            say(cli::USAGE);
            0
        }
        cli::Command::Completion => {
            say(include_str!("completion.zsh"));
            0
        }
        cli::Command::Keys => keys(json, terminal),
        cli::Command::Daemon { .. } => unreachable!("the daemon isn't a command line"),
    }
}

/// Writes what the command says, saying nothing more if whatever reads it
/// has gone, as `head` goes once it has its lines.
fn say(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes()).and_then(|()| out.flush());
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
