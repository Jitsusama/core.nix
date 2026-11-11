//! programs SOCKET SOURCES
//!
//! What's installed and what each program had open, held ready for the
//! shell and anything else that asks, over SOCKET in the shell's daemon
//! interface (interface.rs): each desktop entry along the data folders
//! as a `program` record, each program's recent paths as `recent`, and
//! what it has open now as `opened`, by the sources SOURCES lists, a JSON
//! file of recents.nix's list (source.rs).
//!
//! Nothing is read while nobody watches, and nothing on a timer. An entry
//! is read as its key is first asked, and again when inotify says a
//! folder of entries or of file kinds changed while it's watched, or a
//! profile's link a data folder is reached through was replaced. A
//! source's list is read as a watch of its key begins if its file
//! changed since, or every time for a source that's `asked`, and on each
//! change of its file while it's watched; what a program has open, as a
//! watch of its key begins and on each get. A page's list of folders is
//! read by this daemon itself, with no command, as its watch begins if
//! one of them changed since, and on each change in them while watched.
//! Each command runs on a thread of its own, without a shell, so a slow
//! one never holds an answer back, and is stopped past ten seconds.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use programs::interface::{Ask, KINDS, VERSION, ask, error, kind_of};
use programs::mime::Mime;
use programs::notify::Notify;
use programs::source::{self, Source};
use programs::{entry, now_ms, xdg};
use serde_json::{Value, json};

/// The longest a source's command may run before it's stopped.
const COMMAND_WAIT: Duration = Duration::from_secs(10);
/// How many of the last answers' times health says.
const TIMES: usize = 32;
/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    told: SyncSender<String>,
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }
}

/// What a command a source runs is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Run {
    List,
    Opened,
}

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    watches: Vec<Watch>,
    /// Each entry's file by its id, as last looked for.
    entries: Option<BTreeMap<String, PathBuf>>,
    mime: Mime,
    /// Which sources' files changed since their lists were last read.
    stale: Vec<bool>,
    /// When each source's file had last changed as its list was read, so
    /// a change inotify hasn't said yet still counts.
    read_as_of: Vec<Option<SystemTime>>,
    /// The commands running now, and those asked for again meanwhile.
    running: HashSet<(usize, Run)>,
    again: HashSet<(usize, Run)>,
    /// How each source's last command stood, by its key.
    commands: BTreeMap<String, Value>,
    took: VecDeque<u64>,
    since: u64,
}

struct Shared {
    daemon: Mutex<Daemon>,
    sources: Vec<Source>,
    folders: Vec<PathBuf>,
    search: String,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(listed)) = (args.next(), args.next()) else {
        eprintln!("usage: programs SOCKET SOURCES");
        std::process::exit(64);
    };
    let sources: Vec<Source> = match std::fs::read_to_string(&listed)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()))
    {
        Ok(sources) => sources,
        Err(why) => {
            eprintln!("<3>couldn't read the sources in {listed}: {why}");
            std::process::exit(66);
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
    let folders = xdg::folders_here();
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            watches: Vec::new(),
            entries: None,
            mime: Mime::new(folders.clone()),
            stale: vec![true; sources.len()],
            read_as_of: vec![None; sources.len()],
            running: HashSet::new(),
            again: HashSet::new(),
            commands: BTreeMap::new(),
            took: VecDeque::new(),
            since: now_ms(),
        }),
        sources,
        folders,
        search: std::env::var("PATH").unwrap_or_default(),
    });
    match Notify::new() {
        Ok(notify) => {
            // Its folders are watched before the socket answers, so no
            // change after a first answer goes unheard.
            let folders = heard(&shared, &notify);
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || notice(&shared, &notify, &folders));
        }
        Err(why) => eprintln!("<4>inotify isn't there, so changes aren't heard: {why}"),
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket systemd holds for it (programs.socket), or, run by hand,
/// its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &Path) -> std::io::Result<UnixListener> {
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

/// The listening socket systemd hands this process (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Whether some watch wants this key.
fn watched(daemon: &Daemon, key: &str) -> bool {
    daemon.watches.iter().any(|watch| watch.wants(key))
}

/// Keeps these records and says these keys gone, telling each watch what
/// changed of what it wants.
fn keep(daemon: &mut Daemon, records: Vec<Value>, gone: &[String]) {
    let gone: Vec<String> = gone
        .iter()
        .filter(|key| daemon.held.remove(*key).is_some())
        .cloned()
        .collect();
    let mut changed = Vec::new();
    for record in records {
        let Some(key) = record["key"].as_str().map(str::to_owned) else {
            continue;
        };
        if daemon.held.get(&key) != Some(&record) {
            daemon.held.insert(key, record.clone());
            changed.push(record);
        }
    }
    if changed.is_empty() && gone.is_empty() {
        return;
    }
    let mut stuck = HashSet::new();
    for watch in &daemon.watches {
        let records: Vec<&Value> = changed
            .iter()
            .filter(|record| record["key"].as_str().is_some_and(|key| watch.wants(key)))
            .collect();
        let went: Vec<&String> = gone.iter().filter(|key| watch.wants(key)).collect();
        if records.is_empty() && went.is_empty() {
            continue;
        }
        let mut told = json!({"id": watch.id, "changed": records});
        if !went.is_empty() {
            told["gone"] = json!(went);
        }
        if watch.told.try_send(told.to_string()).is_err() {
            stuck.insert(watch.connection);
            let _ = watch.stream.shutdown(Shutdown::Both);
        }
    }
    if !stuck.is_empty() {
        daemon
            .watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

/// Each entry's file by its id, looked for again once a folder changed.
fn entries<'a>(shared: &Shared, daemon: &'a mut Daemon) -> &'a BTreeMap<String, PathBuf> {
    daemon
        .entries
        .get_or_insert_with(|| xdg::entries(&shared.folders).0)
}

/// The `program` records of these ids, or of every entry.
fn programs(shared: &Shared, daemon: &mut Daemon, ids: Option<&[String]>) -> Vec<Value> {
    let found = entries(shared, daemon).clone();
    let wanted: Vec<(String, PathBuf)> = match ids {
        Some(ids) => ids
            .iter()
            .filter_map(|id| found.get(id).map(|path| (id.clone(), path.clone())))
            .collect(),
        None => found.into_iter().collect(),
    };
    wanted
        .iter()
        .filter_map(|(id, path)| {
            let text = std::fs::read_to_string(path).ok()?;
            entry::record(
                id,
                &path.to_string_lossy(),
                &text,
                &mut daemon.mime,
                &shared.search,
            )
        })
        .collect()
}

/// Reads the entries these keys name that aren't held, or every entry,
/// and keeps them.
fn read_programs(shared: &Shared, keys: Option<&[String]>) {
    let mut daemon = lock(&shared.daemon);
    let ids: Option<Vec<String>> = keys.map(|keys| {
        keys.iter()
            .filter(|key| kind_of(key) == Some("program") && !daemon.held.contains_key(*key))
            .filter_map(|key| key.strip_prefix("program:").map(str::to_owned))
            .collect()
    });
    if ids.as_ref().is_some_and(Vec::is_empty) {
        return;
    }
    let records = programs(shared, &mut daemon, ids.as_deref());
    keep(&mut daemon, records, &[]);
}

/// A folder of entries or of file kinds changed: the entries are looked
/// for again, every one watched read again, and those no longer there,
/// or no longer watched, let go.
fn entries_changed(shared: &Shared) {
    let mut daemon = lock(&shared.daemon);
    daemon.entries = None;
    daemon.mime.forget();
    let held: Vec<String> = daemon
        .held
        .keys()
        .filter(|key| kind_of(key) == Some("program"))
        .cloned()
        .collect();
    let (watched_keys, unwatched): (Vec<String>, Vec<String>) =
        held.into_iter().partition(|key| watched(&daemon, key));
    for key in &unwatched {
        daemon.held.remove(key);
    }
    let ids: Vec<String> = watched_keys
        .iter()
        .filter_map(|key| key.strip_prefix("program:").map(str::to_owned))
        .collect();
    let records = programs(shared, &mut daemon, Some(&ids));
    let said: HashSet<&str> = records.iter().filter_map(|r| r["key"].as_str()).collect();
    let gone: Vec<String> = watched_keys
        .iter()
        .filter(|key| !said.contains(key.as_str()))
        .cloned()
        .collect();
    keep(&mut daemon, records, &gone);
}

/// The sources whose `recent`, `listed` or `opened` key is one of these,
/// or every source with such a list when `keys` is none and its kind is
/// wanted.
fn sources_of(shared: &Shared, run: Run, kinds: &[String], keys: Option<&[String]>) -> Vec<usize> {
    shared
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| match run {
            Run::List => source.lists(),
            Run::Opened => source.opened_key().is_some(),
        })
        .filter(|(_, source)| {
            let (key, kind) = match run {
                Run::List => (Some(source.list_key()), source.list_kind()),
                Run::Opened => (source.opened_key(), "opened"),
            };
            match keys {
                Some(keys) => key.is_some_and(|key| keys.contains(&key)),
                None => kinds.iter().any(|k| k == kind),
            }
        })
        .map(|(at, _)| at)
        .collect()
}

/// Runs a command without a shell, answering what it printed, or why it
/// couldn't: it isn't there, it failed, or it ran past [`COMMAND_WAIT`].
fn run(argv: &[String]) -> Result<String, String> {
    let (program, arguments) = argv.split_first().ok_or("no command")?;
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|why| format!("{program} couldn't start: {why}"))?;
    let mut out = child.stdout.take().ok_or("no output")?;
    let (said, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let read = out.read_to_string(&mut text).map(|_| text);
        let _ = said.send(read);
    });
    let printed = if let Ok(read) = heard.recv_timeout(COMMAND_WAIT) {
        read.map_err(|why| format!("{program}'s output couldn't be read: {why}"))
    } else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{program} ran past {}s", COMMAND_WAIT.as_secs()));
    };
    let status = child.wait().map_err(|why| why.to_string())?;
    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    printed
}

/// Runs a source's command now, on this thread, and keeps its record. A
/// command that fails keeps the record from before, and says why.
fn read_source(shared: &Shared, at: usize, what: Run) {
    let source = &shared.sources[at];
    let (argv, key) = match what {
        Run::List => (&source.list, source.list_key()),
        Run::Opened => (&source.opened, source.opened_key().unwrap_or_default()),
    };
    if what == Run::List {
        let mut daemon = lock(&shared.daemon);
        daemon.stale[at] = false;
        daemon.read_as_of[at] = changed_at(source);
    }
    let started = Instant::now();
    let printed = if what == Run::List && source.list.is_empty() {
        Ok(source::files_in(&source.folders))
    } else {
        run(argv)
    };
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut daemon = lock(&shared.daemon);
    daemon.took.push_back(took);
    while daemon.took.len() > TIMES {
        daemon.took.pop_front();
    }
    match printed {
        Ok(printed) => {
            daemon.commands.insert(key, json!({"state": "answering"}));
            let record = match what {
                Run::List => Some(source::recent(source, &printed)),
                Run::Opened => source::opened(source, &printed),
            };
            keep(&mut daemon, record.into_iter().collect(), &[]);
        }
        Err(why) => {
            // A list never read says it's empty and why, so a watch
            // waiting to hear every list once hears this one too and lets
            // go as the launcher closes; one read before keeps what it
            // last said, as the old reader kept its lines.
            if what == Run::List && !daemon.held.contains_key(&key) {
                let mut record = source::recent(source, "");
                record["refused"] = json!(why);
                keep(&mut daemon, vec![record], &[]);
            }
            daemon
                .commands
                .insert(key, json!({"state": "refused", "reason": why}));
        }
    }
}

/// When a source's file last changed, or the latest of its folders, as a
/// file made, moved or removed in one changes it; none where it has none.
fn changed_at(source: &Source) -> Option<SystemTime> {
    let modified = |path: &str| {
        std::fs::metadata(path)
            .and_then(|file| file.modified())
            .ok()
    };
    match source.watch.as_deref() {
        Some(file) => modified(file),
        None => source
            .folders
            .iter()
            .filter_map(|folder| modified(folder))
            .max(),
    }
}

/// Whether a source's list is to be read again: it's asked, or its file
/// changed since it was last read, as inotify said or its time says.
fn due(shared: &Shared, daemon: &Daemon, at: usize) -> bool {
    let source = &shared.sources[at];
    source.asked || daemon.stale[at] || changed_at(source) != daemon.read_as_of[at]
}

/// Runs a source's command on a thread of its own, or, while it runs
/// already, once more after it, so the last change always counts.
fn start(shared: &Arc<Shared>, at: usize, what: Run) {
    {
        let mut daemon = lock(&shared.daemon);
        if !daemon.running.insert((at, what)) {
            daemon.again.insert((at, what));
            return;
        }
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        loop {
            read_source(&shared, at, what);
            let mut daemon = lock(&shared.daemon);
            if !daemon.again.remove(&(at, what)) {
                daemon.running.remove(&(at, what));
                return;
            }
        }
    });
}

/// Whether these kinds or keys name any entry.
fn names_programs(kinds: &[String], keys: Option<&[String]>) -> bool {
    match keys {
        Some(keys) => keys.iter().any(|key| kind_of(key) == Some("program")),
        None => kinds.iter().any(|kind| kind == "program"),
    }
}

/// What a watch beginning reads after its answer: each list whose file
/// changed since it was read, and, for a watch naming its program's key,
/// a list that's asked and what the program has open. A watch of a whole
/// kind, as the launcher holds while it's open, runs neither, so Chrome's
/// history is read only as Chrome is stepped into.
fn begin(shared: &Arc<Shared>, kinds: &[String], keys: Option<&[String]>) {
    for at in sources_of(shared, Run::List, kinds, keys) {
        if keys.is_none() && shared.sources[at].asked {
            continue;
        }
        let due = {
            let daemon = lock(&shared.daemon);
            due(shared, &daemon, at)
        };
        if due {
            start(shared, at, Run::List);
        }
    }
    if keys.is_some() {
        for at in sources_of(shared, Run::Opened, kinds, keys) {
            start(shared, at, Run::Opened);
        }
    }
}

/// What a get or a list reads first, on this thread: the entries not yet
/// held, each list not read since it changed or that's asked, and what
/// each program has open now.
fn fresh(shared: &Shared, kinds: &[String], keys: Option<&[String]>) {
    if names_programs(kinds, keys) {
        read_programs(shared, keys);
    }
    for at in sources_of(shared, Run::List, kinds, keys) {
        let due = {
            let daemon = lock(&shared.daemon);
            due(shared, &daemon, at) || !daemon.held.contains_key(&shared.sources[at].list_key())
        };
        if due {
            read_source(shared, at, Run::List);
        }
    }
    for at in sources_of(shared, Run::Opened, kinds, keys) {
        read_source(shared, at, Run::Opened);
    }
}

/// What a watched folder is to this daemon; one can be more than one.
#[derive(Default)]
struct Heard {
    /// Whether it's a folder of entries or of file kinds, whose every
    /// change counts.
    entries: bool,
    /// The links in it that a data folder resolves through, by name, as
    /// `current-system` in `/run`: replaced, the entries changed.
    links: Vec<String>,
    /// The sources' files in it, by name.
    files: Vec<(String, usize)>,
    /// The sources it's one of the folders of, whose every change counts.
    lists: Vec<usize>,
}

/// Watches every folder of entries and of file kinds, and the folder of
/// each source's file, answering what each watch is.
fn heard(shared: &Shared, notify: &Notify) -> HashMap<i32, Heard> {
    let mut folders: HashMap<i32, Heard> = HashMap::new();
    let (_, looked) = xdg::entries(&shared.folders);
    let kinds = shared.folders.iter().map(|folder| folder.join("mime"));
    for folder in looked.into_iter().chain(kinds) {
        if let Some(watch) = notify.watch(&folder) {
            folders.entry(watch).or_default().entries = true;
        }
    }
    // On NixOS each data folder is a store path reached through a
    // profile's links, so an upgrade says nothing in the folder itself:
    // the link it replaces is heard instead, and every watched entry is
    // read again, its version and store path with it.
    for (folder, name) in shared
        .folders
        .iter()
        .flat_map(|folder| xdg::links_toward(folder))
    {
        if let Some(watch) = notify.watch(&folder) {
            let links = &mut folders.entry(watch).or_default().links;
            if !links.contains(&name) {
                links.push(name);
            }
        }
    }
    for (at, source) in shared.sources.iter().enumerate() {
        let Some(file) = source
            .watch
            .as_deref()
            .filter(|_| !source.asked)
            .map(Path::new)
        else {
            continue;
        };
        let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
            continue;
        };
        let Some(watch) = notify.watch(folder) else {
            continue;
        };
        let name = name.to_string_lossy().into_owned();
        folders.entry(watch).or_default().files.push((name, at));
    }
    for (at, source) in shared.sources.iter().enumerate() {
        for folder in source.folders.iter().filter(|_| !source.asked) {
            if let Some(watch) = notify.watch(Path::new(folder)) {
                folders.entry(watch).or_default().lists.push(at);
            }
        }
    }
    folders
}

/// Hears every change inotify says, for as long as the daemon runs.
fn notice(shared: &Arc<Shared>, notify: &Notify, folders: &HashMap<i32, Heard>) {
    loop {
        let Ok(changes) = notify.changes() else {
            return;
        };
        let mut entries = false;
        let mut lists = HashSet::new();
        for (watch, name) in changes {
            let Some(heard) = folders.get(&watch) else {
                continue;
            };
            entries |= heard.entries || heard.links.contains(&name);
            lists.extend(
                heard
                    .files
                    .iter()
                    .filter(|(file, _)| *file == name)
                    .map(|(_, at)| *at),
            );
            lists.extend(heard.lists.iter().copied());
        }
        if entries {
            entries_changed(shared);
        }
        for at in lists {
            let watching = {
                let mut daemon = lock(&shared.daemon);
                daemon.stale[at] = true;
                watched(&daemon, &shared.sources[at].list_key())
            };
            if watching {
                start(shared, at, Run::List);
            }
        }
    }
}

fn serve(shared: &Arc<Shared>, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let Ok(hangup) = stream.try_clone().map(Arc::new) else {
        return;
    };
    let (told, lines) = mpsc::sync_channel::<String>(QUEUED);
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
        let Some(answer) = answer(shared, ask(&line), connection, &told, &hangup) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(&shared.daemon)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// The records held of these kinds or keys, in the daemon's order: by
/// kind, then by key.
fn held(daemon: &Daemon, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| daemon.held.get(key))
            .cloned()
            .collect(),
        None => KINDS
            .iter()
            .filter(|kind| kinds.iter().any(|k| k == *kind))
            .flat_map(|kind| {
                daemon
                    .held
                    .iter()
                    .filter(move |(key, _)| kind_of(key) == Some(kind))
                    .map(|(_, record)| record.clone())
            })
            .collect(),
    }
}

fn sources(daemon: &Daemon) -> Value {
    json!({
        "entries": {"state": "answering", "count": daemon.entries.as_ref().map(BTreeMap::len)},
        "commands": daemon.commands,
    })
}

/// What one ask is answered with.
fn answer(
    shared: &Arc<Shared>,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    Some(match asked {
        Ask::Hello { id } => {
            let daemon = lock(&shared.daemon);
            json!({
                "id": id, "version": VERSION, "domain": "programs", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
            })
        }
        Ask::Get { id, keys } => {
            fresh(shared, &[], Some(&keys));
            json!({"id": id, "records": held(&lock(&shared.daemon), &[], Some(&keys))})
        }
        Ask::List { id, kinds } => {
            fresh(shared, &kinds, None);
            json!({"id": id, "records": held(&lock(&shared.daemon), &kinds, None)})
        }
        Ask::Watch { id, kinds, keys } => {
            // An entry is read at once, a file or two, so the answer holds
            // it; a list's command may be slow, so it's told after.
            if names_programs(&kinds, keys.as_deref()) {
                read_programs(shared, keys.as_deref());
            }
            {
                let mut daemon = lock(&shared.daemon);
                // A watch asked again by its id replaces the one before.
                daemon
                    .watches
                    .retain(|watch| !(watch.connection == connection && watch.id == id));
                // What it's told first is queued under the lock that holds
                // the watch, so a change read since, as inotify's are, is
                // told after it. Queued once the lock was let go, a change
                // could be said first, and the whole answer after it would
                // put back what the change moved on from. One that can't
                // be queued is hung up on, as a change would hang up on it.
                let first = json!({"id": id, "records": held(&daemon, &kinds, keys.as_deref())});
                if told.try_send(first.to_string()).is_err() {
                    let _ = hangup.shutdown(Shutdown::Both);
                    return None;
                }
                daemon.watches.push(Watch {
                    connection,
                    id: id.clone(),
                    kinds: kinds.clone(),
                    keys: keys.clone(),
                    told: told.clone(),
                    stream: Arc::clone(hangup),
                });
            }
            // What it reads now it's told as a change, after this answer,
            // so the answer says what's held and the rest follows.
            let shared = Arc::clone(shared);
            std::thread::spawn(move || begin(&shared, &kinds, keys.as_deref()));
            return None;
        }
        Ask::Unwatch { id } => {
            let mut daemon = lock(&shared.daemon);
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => {
            let daemon = lock(&shared.daemon);
            json!({
                "id": id, "version": VERSION, "domain": "programs", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
                "watches": daemon.watches.len(),
                "took_us": daemon.took.iter().collect::<Vec<_>>(),
            })
        }
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}
