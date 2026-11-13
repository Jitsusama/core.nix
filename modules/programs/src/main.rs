//! programs SOCKET SOURCES [WORDS]
//!
//! What's installed and what each program had open, held ready for the
//! shell and anything else that asks, over SOCKET in the shell's daemon
//! interface (docs/daemons.md): each desktop entry along the data folders
//! as a `program` record, each program's recent paths as `recent`, and
//! what it has open now as `opened`, by the sources SOURCES lists, a JSON
//! file of recents.nix's list (source.rs).
//!
//! Nothing is read while nobody watches, and nothing on a timer. An entry
//! is read as its key is first asked, and again when inotify says a
//! folder of entries or of file kinds changed while it's watched, or a
//! profile's link a data folder is reached through was replaced. A
//! source's list is read as a watch of its key begins if its file
//! changed since, or every time for a source that's `asked`, or for one
//! that's `told` too, once a program said it changed, and on each
//! change of its file while it's watched; what a program has open, as a
//! watch of its key begins and on each get, and Chrome's, which this
//! daemon reads itself from its extension's host's socket or its session
//! file with no command, again on each change of that file while it's
//! watched (heard.rs, chrome-session). A page's list of folders is
//! read by this daemon itself, with no command, as its watch begins if
//! one of them changed since, and on each change in them while watched;
//! a folder asked for by its path, as `folder`, as each ask names it and
//! on each change in it while watched (folder.rs); and the repositories
//! under a folder, or one's files, as each ask names them, and the
//! repositories again as one is cloned or its git writes while watched
//! (repositories.rs); what a file shows of itself, by one file-preview
//! kept, as each ask names it (preview.rs); and what kitty's page reads
//! of zoxide, ssh and zsh's history, as each ask names it and on each
//! change of its file while watched (visited.rs, machines.rs). The
//! Programs collection's rows are made as a watch of them begins and again
//! as the windows daemon says a window opened, closed or was looked at, or
//! an entry changed (rows.rs), and searched as each `search` names what's
//! typed (search.rs). Capture's Recent's rows are made as a watch of them
//! begins and as what was taken changes, worded from the launcher's
//! vocabulary WORDS, and with no WORDS none is (taken.rs). A program that
//! tells its lists changed has them read again (told.rs). Chrome's and
//! kitty's open tabs are rows of their own while watched (tabs.rs), and
//! Chrome's bookmarks too, but for a page open now (bookmarks.rs). kitty's
//! open list leaves out the panes the agents daemon names, and Programs counts
//! their windows apart, as it tells (agents.rs).
//! Each command runs on a thread of its own, without a shell, so a slow one never holds an
//! answer back, and is stopped past ten seconds (command.rs).
//!
//! It speaks the shell's daemon interface through daemon-serve, as every
//! daemon does (docs/daemons.md): each change hands it every record held,
//! and it tells each watch what moved. What a get, a list or a watch
//! beginning names is read first (`read_for`), and what its rows are made
//! of comes in a `settings` ask, held until the next (`settle`).

mod agents;
mod bookmarks;
mod follow;
mod heard;
mod named;
mod rows;
mod search;
mod tabs;
mod taken;
mod told;

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Instant, SystemTime};

use daemon_serve::asks::place_of;
use daemon_serve::serve::{Shared as Served, Wanted, accept, listen};
use programs::folder::Looked;
use programs::interface::{DOMAIN, Own, Settings, kind_of, own_of};
use programs::mime::Mime;
use programs::notify::Notify;
use programs::preview::Previewer;
use programs::source::{self, Source};
use programs::{command, entry, xdg};
use programs::{tab_rows, taken_rows};
use ready_rows::kept::Kept;
use serde_json::{Value, json};

use heard::{heard, notice};
use named::read_named;

/// How many of the last answers' times health says.
const TIMES: usize = 32;

/// What a command a source runs is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Run {
    List,
    Opened,
}

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    /// What serves every record held, and knows what's watched.
    served: Arc<Served>,
    /// The hidden programs Programs lists all the same, the folders
    /// Capture's rows are taken from, and what each program's key binds
    /// do, as the last `settings` ask to name each said.
    shown: Vec<String>,
    folders: Vec<String>,
    also: BTreeMap<String, String>,
    /// Each entry's file by its id, as last looked for.
    entries: Option<BTreeMap<String, PathBuf>>,
    mime: Mime,
    /// Which sources' files changed since their lists were last read.
    stale: Vec<bool>,
    /// When each source's file had last changed as its list was read, so
    /// a change inotify hasn't said yet still counts.
    read_as_of: Vec<Option<SystemTime>>,
    /// The processes each told list's last read named, and those that
    /// have told this daemon they tell of their changes: a list naming
    /// one that hasn't is read as each ask comes, as an untold one is.
    listers: Vec<Vec<u64>>,
    tellers: HashSet<u64>,
    /// How many `changed` asks were heard, which health says, so how
    /// often kitty tells can be counted.
    told: u64,
    /// The commands running now, and those asked for again meanwhile.
    running: HashSet<(usize, Run)>,
    again: HashSet<(usize, Run)>,
    /// How each source's last command stood, by its key.
    commands: BTreeMap<String, Value>,
    took: VecDeque<u64>,
    /// The folders asked for by their paths, watched since first asked.
    looked: Looked,
    /// What the windows daemon last said of each window, by its key; none
    /// until it's first heard.
    windows: Option<BTreeMap<String, Value>>,
    /// Every entry's `program` record, as Programs' rows were last made
    /// from, until a folder of them changes.
    catalogue: Option<Vec<Value>>,
    /// Programs' rows as last told, Capture's and Chrome's tabs'.
    rows: Kept,
    taken: Kept,
    tabs: Kept,
    agents: agents::Agents,
    /// Chrome's bookmarks' rows as last told.
    bookmarks: Kept,
}

struct Shared {
    daemon: Mutex<Daemon>,
    /// Where changes are heard, unless inotify isn't there.
    notify: Option<Notify>,
    previewer: Mutex<Previewer>,
    sources: Vec<Source>,
    folders: Vec<PathBuf>,
    search: String,
    home: String,
    /// The words Capture's rows are said in, where they were handed.
    taken_words: Option<taken_rows::Words>,
    /// The words a tab's drawer and Tabs' head are said in.
    tab_words: tab_rows::Words,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(listed)) = (args.next(), args.next()) else {
        eprintln!("usage: programs SOCKET SOURCES [WORDS]");
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
    let vocabulary = taken::vocabulary(args.next()).unwrap_or_default();
    let notify = Notify::new()
        .inspect_err(|why| eprintln!("<4>inotify isn't there, so changes aren't heard: {why}"))
        .ok();
    let shared = Arc::new_cyclic(|this| Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            served: serving(this),
            shown: Vec::new(),
            folders: Vec::new(),
            also: BTreeMap::new(),
            entries: None,
            mime: Mime::new(folders.clone()),
            stale: vec![true; sources.len()],
            read_as_of: vec![None; sources.len()],
            listers: vec![Vec::new(); sources.len()],
            tellers: HashSet::new(),
            told: 0,
            running: HashSet::new(),
            again: HashSet::new(),
            commands: BTreeMap::new(),
            took: VecDeque::new(),
            looked: Looked::default(),
            windows: None,
            catalogue: None,
            rows: Kept::default(),
            taken: Kept::default(),
            tabs: Kept::default(),
            agents: agents::Agents::default(),
            bookmarks: Kept::default(),
        }),
        notify,
        previewer: Mutex::new(Previewer::new(std::env::var("PROGRAMS_PREVIEW").ok())),
        sources,
        folders,
        search: std::env::var("PATH").unwrap_or_default(),
        home: std::env::var("HOME").unwrap_or_default(),
        taken_words: taken::words(&vocabulary),
        tab_words: tab_rows::Words::read(&vocabulary),
    });
    shared.served().fresh(Box::new({
        let shared = Arc::downgrade(&shared);
        move |wanted| {
            if let Some(shared) = shared.upgrade() {
                read_for(&shared, wanted);
            }
        }
    }));
    shared.served().health_too(Box::new({
        let shared = Arc::downgrade(&shared);
        move || shared.upgrade().map_or(json!({}), |shared| health(&shared))
    }));
    if let Some(notify) = &shared.notify {
        // Its folders are watched before the socket answers, so no
        // change after a first answer goes unheard.
        let folders = heard(&shared, notify);
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || notice(&shared, &folders));
    }
    {
        let shared = Arc::clone(&shared);
        let windows = rows::socket(&socket);
        std::thread::spawn(move || rows::hear(&shared, &windows));
    }
    {
        let shared = Arc::clone(&shared);
        let agents = agents::socket(&socket);
        std::thread::spawn(move || agents::hear(&shared, &agents));
    }
    accept(shared.served(), listener);
    loop {
        std::thread::park();
    }
}

impl Shared {
    /// What serves the records.
    fn served(&self) -> Arc<Served> {
        Arc::clone(&lock(&self.daemon).served)
    }
}

/// What serves `shared`'s records, its own asks answered as `own` says.
fn serving(shared: &Weak<Shared>) -> Arc<Served> {
    let shared = Weak::clone(shared);
    Served::new(
        DOMAIN,
        Box::new(move |said| {
            let shared = shared.upgrade()?;
            own(&shared, said)
        }),
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Whether some watch wants this key, as what was taken is while
/// Capture's rows are watched.
fn watched(daemon: &Daemon, key: &str) -> bool {
    let watching = kind_of(key).is_some_and(|kind| {
        daemon
            .served
            .watches(kind, place_of(key).unwrap_or_default())
    });
    watching
        || daemon.served.keys_watched().contains(key)
        || (key == taken::LIST && taken::watched(daemon))
}

/// Whether a watch of a whole place's rows is of `place`, or a watch
/// names one of its rows by key.
fn rows_watched(daemon: &Daemon, place: &str) -> bool {
    let row = format!("row:{place}:");
    daemon.served.watches("row", place)
        || daemon
            .served
            .keys_watched()
            .iter()
            .any(|key| key.starts_with(&row))
}

/// Every key a watch names, as the shell watches ahead the rows it's
/// about to show.
fn keys_watched(daemon: &Daemon) -> Vec<String> {
    daemon.served.keys_watched().into_iter().collect()
}

/// Keeps these records and says these keys gone, handing every record
/// held to what serves them, which tells each watch what moved of what
/// it wants.
fn keep(daemon: &mut Daemon, records: Vec<Value>, gone: &[String]) {
    let mut moved = false;
    for key in gone {
        moved |= daemon.held.remove(key).is_some();
    }
    for record in records {
        let Some(key) = record["key"].as_str().map(str::to_owned) else {
            continue;
        };
        if daemon.held.get(&key) != Some(&record) {
            daemon.held.insert(key, record);
            moved = true;
        }
    }
    if moved {
        hand_over(daemon);
    }
}

/// Hands every record held, and how each source stands, to what serves
/// them: as records move, and as a source's command comes to stand
/// otherwise.
fn hand_over(daemon: &Daemon) {
    let records = daemon.held.values().cloned().collect();
    daemon.served.change(records, sources(daemon));
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
    daemon.catalogue = None;
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
    if rows::watched(&daemon) {
        rows::make(shared, &mut daemon);
    }
    tabs::renamed(shared, &mut daemon);
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
    let read = (what == Run::Opened).then(|| source::open_now(source));
    let printed = if what == Run::List && source.list.is_empty() {
        Ok(source::files_in(&source.folders))
    } else if let Some(Some(read)) = read {
        read
    } else {
        command::run(argv)
    };
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut daemon = lock(&shared.daemon);
    daemon.took.push_back(took);
    while daemon.took.len() > TIMES {
        daemon.took.pop_front();
    }
    match printed {
        Ok(printed) => {
            let taking = key == taken::LIST;
            let answering = json!({"state": "answering"});
            let was = daemon.commands.insert(key, answering.clone());
            if what == Run::List && source.told {
                daemon.listers[at] = source::listers(&printed);
            }
            let record = match what {
                Run::List => Some(agents::listed(&mut daemon, at, source, &printed)),
                Run::Opened => source::opened(source, &printed),
            };
            keep(&mut daemon, record.into_iter().collect(), &[]);
            // Keeping told only what moved, so a command answering again
            // after it refused is said as it now stands.
            if was != Some(answering) {
                hand_over(&daemon);
            }
            if taking && taken::watched(&daemon) {
                taken::make(shared, &mut daemon);
            }
            tabs::heard(shared, &mut daemon, at, what);
            // A page opened or shut leaves its bookmark's row or brings it
            // back, so home never lists one page twice.
            if what == Run::Opened && source.session.is_some() && bookmarks::watched(&daemon) {
                bookmarks::make(shared, &mut daemon);
            }
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
            hand_over(&daemon);
            tabs::heard(shared, &mut daemon, at, what);
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

/// Whether a source's list is to be read again: it's asked and not told,
/// or told but naming a process that hasn't told yet, or its file changed
/// since it was last read, as inotify said or its time says, or it was told
/// it changed.
fn due(shared: &Shared, daemon: &Daemon, at: usize) -> bool {
    let source = &shared.sources[at];
    let untold = || {
        daemon.listers[at]
            .iter()
            .any(|pid| !daemon.tellers.contains(pid))
    };
    (source.asked && (!source.told || untold()))
        || daemon.stale[at]
        || changed_at(source) != daemon.read_as_of[at]
}

/// Runs a source's command on a thread of its own, or, while it runs
/// already, once more after it, so the last change always counts. A list
/// is read once more only if it's still due as the read ends: one asked
/// for by a change the running read already took, as a watch beginning
/// does a moment after the change that's read, would read it twice.
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
            let again = daemon.again.remove(&(at, what))
                && (what == Run::Opened || due(&shared, &daemon, at));
            if !again {
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
    let lists = sources_of(shared, Run::List, kinds, keys);
    for at in lists.into_iter().chain(taken::lists(shared)) {
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
    if let Some(keys) = keys {
        for at in sources_of(shared, Run::Opened, kinds, Some(keys)) {
            start(shared, at, Run::Opened);
        }
        read_named(shared, keys);
        taken::ahead(shared, keys);
    }
    tabs::read(shared);
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
    read_named(shared, keys.unwrap_or_default());
    if names_rows(kinds, keys) {
        let mut daemon = lock(&shared.daemon);
        rows::make(shared, &mut daemon);
        taken::make(shared, &mut daemon);
    }
}

/// Whether an ask names Programs' rows.
fn names_rows(kinds: &[String], keys: Option<&[String]>) -> bool {
    match keys {
        Some(keys) => keys.iter().any(|key| kind_of(key) == Some("row")),
        None => kinds.iter().any(|kind| kind == "row"),
    }
}

/// What a get, a list or a watch beginning reads before it's answered,
/// on its connection's thread, handing what it read over as a change. A
/// watch of rows has them made first, from the settings held, so its
/// answer holds them; what a list's command reads it's told after, as a
/// change, since a command may be slow.
fn read_for(shared: &Arc<Shared>, wanted: &Wanted<'_>) {
    let (kinds, keys) = (wanted.kinds, wanted.keys);
    if !wanted.watching {
        fresh(shared, kinds, keys);
        return;
    }
    // An entry is read at once, a file or two, so the answer holds it.
    if names_programs(kinds, keys) {
        read_programs(shared, keys);
    }
    if names_rows(kinds, keys) {
        let mut daemon = lock(&shared.daemon);
        let places = wanted.places.unwrap_or_default();
        if places.is_empty() {
            rows::make(shared, &mut daemon);
        }
        for place in places {
            match place.as_str() {
                "taken" => taken::make(shared, &mut daemon),
                "tabs" => tabs::make(shared, &mut daemon),
                "bookmarks" => bookmarks::make(shared, &mut daemon),
                _ => rows::make(shared, &mut daemon),
            }
        }
    }
    let (shared, kinds) = (Arc::clone(shared), kinds.to_vec());
    let keys = keys.map(<[String]>::to_vec);
    std::thread::spawn(move || begin(&shared, &kinds, keys.as_deref()));
}

/// How each source stands, by its name: the entries, and each list's or
/// open list's command by its key, with why where it refused.
fn sources(daemon: &Daemon) -> Value {
    let mut sources = serde_json::Map::new();
    sources.insert("entries".to_owned(), json!({"state": "answering"}));
    for (key, stands) in &daemon.commands {
        sources.insert(key.clone(), stands.clone());
    }
    Value::Object(sources)
}

/// What health says beside what every daemon's says: how many entries
/// were found, how long the last reads took, and how often programs
/// told of their own changes.
fn health(shared: &Shared) -> Value {
    let daemon = lock(&shared.daemon);
    json!({
        "entries": daemon.entries.as_ref().map(BTreeMap::len),
        "took_us": daemon.took.iter().collect::<Vec<_>>(),
        "told": {"asks": daemon.told, "tellers": daemon.tellers.len()},
    })
}

/// Holds what the rows are made of, each field the ask names replacing
/// what the one before said, and makes the rows again with it.
fn settle(shared: &Shared, settings: Settings) -> Value {
    let mut daemon = lock(&shared.daemon);
    if let Some(shown) = settings.shown {
        daemon.shown = shown;
    }
    if let Some(folders) = settings.folders {
        daemon.folders = folders;
    }
    if let Some(also) = settings.also {
        daemon.also = also;
    }
    if rows::watched(&daemon) {
        rows::make(shared, &mut daemon);
    }
    if taken::watched(&daemon) {
        taken::make(shared, &mut daemon);
    }
    json!({"settings": {"shown": daemon.shown, "folders": daemon.folders, "also": daemon.also}})
}

/// The answer to one of the daemon's own asks, without its id, or none
/// for one it doesn't know.
fn own(shared: &Arc<Shared>, said: &Value) -> Option<Value> {
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    Some(match own_of(said)? {
        Own::Changed { keys, from } => told::heard(shared, &id, &keys, from),
        Own::Search { place, words, most } => {
            search::answer(shared, &lock(&shared.daemon), &id, &place, &words, most)
        }
        Own::Tab(tab) => tabs::act(&id, tab),
        Own::Settings(settings) => settle(shared, settings),
    })
}
