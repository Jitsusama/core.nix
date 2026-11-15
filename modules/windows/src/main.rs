//! windows SOCKET [--words FILE] [--numbers FILE] [--machine SOCKET]
//!
//! niri's windows, held ready for the shell and anything else that asks,
//! over SOCKET in the shell's daemon interface, which daemon-serve answers
//! as it does every daemon's (interface.rs names the kinds): each window
//! as a `window` record, named by its program and placed as its row says
//! it, each workspace as a `workspace` record, and what holds the keys as
//! the one `desktop` record (desktop.rs).
//!
//! It hears niri's event stream on `NIRI_SOCKET` for as long as it runs,
//! since a stream can't be paused, and works out every record again at
//! each event, which takes microseconds here where it took the shell's
//! main thread milliseconds; a watch is told only the records that
//! changed, and those gone, so a focus moving tells two windows and a
//! window resizing tells nothing. Each window's program is looked up once
//! by its app id, and again only once inotify says the desktop entries
//! changed. Nothing runs on a timer: with niri gone, it tries niri again
//! every two seconds until it's back, and started before niri, it waits
//! the same way for niri to name its socket (`niri_socket`).
//!
//! Handed the launcher's words (`--words`, its words.json), it serves
//! the launcher's Screens too, as rows of kind `row` in place `displays`
//! (screens.rs), asking niri for its outputs as the stream begins and at
//! each workspace change, since niri says nothing of a screen itself,
//! and again when asked to `look`, as the shell does once it's changed a
//! screen. Handed none, it makes no rows. Each screen's drawer says its
//! figures in the account's number settings (`--numbers`, `{ group,
//! point }`), or the comma and the full stop where it's handed none.
//! Handed the machine daemon's socket too (`--machine`), it watches the
//! screen's light there while a watch's keys name a screen's row, as the
//! shell's watch ahead does once a screen is selected, and the drawer of
//! the screen holding it says it (light.rs); the machine daemon reads the
//! light each second while it's watched, so it isn't watched otherwise.
//! It's read once as the daemon starts, so the Warmth inside that screen
//! is served from the first (light/warmth.rs), and a switch of the
//! Warmth's asks this daemon, which asks the machine daemon, `light`.
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `windows --help` says (`help()`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use daemon_serve::asks::error;
use daemon_serve::serve::{self, Own, Shared as Served};
use programs::notify::Notify;
use programs::{now_ms, xdg};
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};
use windows::desktop::{Desktop, Heard, Names};
use windows::entries::{self, Entries};
use windows::interface::DOMAIN;
use windows::light::{Held, Seen};
use windows::screens;
use windows::screens::acts::{self, Act};
use windows::screens::vocabulary::{numbered, worded};
use windows::shows;
use windows::socket;
use windows::timed::{self, Timed};
use windows::unasked::{self, Watched};
use windows::{open, order};

/// How long to wait before asking niri again once it's gone.
const NIRI_AGAIN: Duration = Duration::from_secs(2);
/// How long to wait before asking the machine daemon again while a
/// screen is looked at and it isn't answering.
const MACHINE_AGAIN: Duration = Duration::from_secs(2);
/// How long the machine daemon may take to answer: a whole answer of the
/// light as the daemon starts, or a switch of the Warmth's, which sets
/// one property on the session's bus.
const ASKING: Duration = Duration::from_secs(2);
/// The daemon's own asks, each answered on a thread of its own, since
/// each asks niri or the machine daemon, so no ask after one waits on
/// either.
const SLOW: &[&str] = &["look", "screen", "light"];

/// What the records are made from.
struct Daemon {
    desktop: Desktop,
    entries: Entries,
    names: Names,
    /// niri's outputs as it last said them, by connector.
    outputs: BTreeMap<String, Value>,
    /// Whether niri darkens one screen by name, which its patch says.
    each: bool,
    /// The words the launcher's rows are said in, or none, so no rows.
    words: Option<screens::Words>,
    /// What desk-facts says of each screen's EDID, by connector.
    desk: BTreeMap<String, Value>,
    light: Held,
    /// The rows as last told, so each keeps its order.
    rows: Kept,
    /// How niri's stream stands: "starting" until it first answers,
    /// "answering", or "not answering" while it's gone.
    niri: &'static str,
    events: u64,
}

/// Everything the threads share.
struct Shared {
    daemon: Mutex<Daemon>,
    /// What serves the records, told each time they change.
    served: Arc<Served>,
    folders: Vec<PathBuf>,
    /// niri's socket, asked for its outputs when a client asks to look.
    niri: PathBuf,
    /// Wakes the light's thread as a screen comes to be looked at.
    looked: Condvar,
    /// The machine daemon's socket, which the Warmth's switches ask.
    machine: Option<PathBuf>,
}

/// What `windows --help` says: how its unit runs it, and how it ends.
fn help() -> Help {
    Help::new("windows", "holds niri's windows ready for the shell")
        .example("windows SOCKET", "its windows alone, with no rows")
        .example(
            "windows SOCKET --words FILE --machine SOCKET",
            "as its unit runs it, Screens' rows too",
        )
        .argument("<socket>", "where the shell asks, unless systemd hands one")
        .option("--words FILE", "the launcher's words, for Screens' rows")
        .option("--numbers FILE", "how the account writes its figures")
        .option(
            "--machine SOCKET",
            "the machine daemon, for a screen's light",
        )
        .exit(Exit::Failed, "its socket couldn't be listened on")
}

/// What its unit hands it.
struct Asked {
    socket: PathBuf,
    words: Option<String>,
    numbers: Option<String>,
    machine: Option<PathBuf>,
}

/// Its socket and its files, from what its unit hands it.
fn asked(args: &[String]) -> Result<Asked, Failure> {
    let mut args = args.iter().cloned();
    let (mut named, mut words, mut numbers, mut machine) = (Vec::new(), None, None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--words" => words = args.next(),
            "--numbers" => numbers = args.next(),
            "--machine" => machine = args.next().map(PathBuf::from),
            _ => named.push(arg),
        }
    }
    let socket = named.into_iter().next().map(PathBuf::from).ok_or_else(|| {
        Failure::new(Exit::Usage, "windows takes a socket to answer on").try_this("windows --help")
    })?;
    Ok(Asked {
        socket,
        words,
        numbers,
        machine,
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        hold(asked(&context.args)?)
    })
}

/// Holds the windows and answers the socket for as long as niri runs, and
/// after, waiting for the next.
fn hold(asked: Asked) -> Result<(), Failure> {
    let Asked {
        socket,
        words,
        numbers,
        machine,
    } = asked;
    let numbers = numbered(numbers.as_deref().map(Path::new));
    let words = words.map(|path| worded(Path::new(&path)).numbered(numbers));
    let niri = niri_socket();
    let listener = serve::listen(&socket).map_err(|why| {
        Failure::new(
            Exit::Failed,
            format!("it couldn't listen on {}", socket.display()),
        )
        .because(why.to_string())
    })?;
    let folders = xdg::folders_here();
    let (entries, looked) = Entries::read(&folders);
    // Screens' drawers say what desk-facts reads of each EDID, so only a
    // daemon making rows reads or hears it.
    let desk = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|_| words.is_some())
        .map(|runtime| PathBuf::from(runtime).join("desk"));
    let said = desk.as_deref().map_or_else(BTreeMap::new, read_desk);
    // The light is said only in a screen's drawer, so only where rows are
    // made.
    let machine = machine.filter(|_| words.is_some());
    // The daemon's own asks reach what it holds, which holds what serves
    // them, so they find it once it's made.
    let found: Arc<OnceLock<Arc<Shared>>> = Arc::default();
    let own: Own = {
        let found = Arc::clone(&found);
        Box::new(move |said| owned(found.get()?, said))
    };
    let served = Served::new(DOMAIN, own);
    served.slow(SLOW);
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            desktop: Desktop::default(),
            entries,
            names: Names::default(),
            outputs: BTreeMap::new(),
            each: std::env::var_os("NIRI_POWERS_OFF_EACH_OUTPUT").is_some_and(|set| set == "1"),
            words,
            desk: said,
            light: Held::default(),
            rows: Kept::default(),
            niri: "starting",
            events: 0,
        }),
        served: Arc::clone(&served),
        folders,
        niri: niri.clone(),
        looked: Condvar::new(),
        machine: machine.clone(),
    });
    let _ = found.set(Arc::clone(&shared));
    refresh(&served, &mut lock(&shared.daemon));
    follow_watches(&shared);
    // The entries' folders are watched before niri is heard, so no change
    // after the first reading goes unheard.
    let following = Arc::clone(&shared);
    if let Err(why) = entries::follow(&shared.folders, looked, move || {
        entries_changed(&following);
    }) {
        eprintln!("<4>inotify isn't there, so entries changing aren't heard: {why}");
    }
    if let Some(desk) = desk {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow_desk(&shared, &desk));
    }
    if let Some(machine) = machine {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow_light(&shared, &machine));
    }
    serve::accept(served, listener);
    follow(&shared, &niri);
    Ok(())
}

/// niri's socket, from its own environment, or, started before niri as a
/// boot's first ask can start it, from the user manager's once niri names
/// it there (socket.rs). The socket's asks wait in systemd's queue till
/// then, and nothing is shown before niri anyway.
fn niri_socket() -> PathBuf {
    if let Some(socket) = std::env::var_os("NIRI_SOCKET") {
        return PathBuf::from(socket);
    }
    eprintln!("<5>niri hasn't named its socket yet, so its windows wait for it");
    loop {
        if let Some(socket) = socket::managed() {
            return socket;
        }
        std::thread::sleep(NIRI_AGAIN);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hears each watch begin or end, on a thread of its own, and wakes the
/// light's thread as a screen comes to be looked at, or lets it rest as
/// none is any more. daemon-serve says so with every record held, so
/// it's only passed on there, and the keys read here after.
fn follow_watches(shared: &Arc<Shared>) {
    let (moved, heard) = mpsc::channel();
    shared.served.on_watches(Box::new(move || {
        let _ = moved.send(());
    }));
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        while let Ok(()) = heard.recv() {
            let _ = heard.try_iter().count();
            let keys: Vec<String> = shared.served.keys_watched().into_iter().collect();
            let looking = screens::looks_at(Some(&keys));
            looks_changed(&shared, &mut lock(&shared.daemon), looking);
        }
    });
}

/// Hears niri's event stream, and asks again every two seconds while
/// niri's gone, as it is between one niri and the next.
fn follow(shared: &Shared, niri: &Path) {
    loop {
        match UnixStream::connect(niri) {
            Ok(stream) => {
                hear(shared, stream, niri);
                eprintln!("<4>niri's event stream ended; asking again");
            }
            Err(why) => {
                if lock(&shared.daemon).niri != "not answering" {
                    eprintln!("<4>niri isn't answering on {}: {why}", niri.display());
                }
            }
        }
        let mut daemon = lock(&shared.daemon);
        daemon.niri = "not answering";
        refresh(&shared.served, &mut daemon);
        drop(daemon);
        std::thread::sleep(NIRI_AGAIN);
    }
}

/// Hears each event of one stream until it ends. niri describes the
/// whole desktop as the stream begins, so what's held from a stream
/// before is replaced by it.
fn hear(shared: &Shared, mut stream: UnixStream, niri: &Path) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    if writeln!(stream, "\"EventStream\"").is_err() {
        return;
    }
    // How long each program takes to open and to end is said apart from
    // the desktop held, which it neither reads nor changes.
    let mut times = Timed::default();
    let mut watched = Watched::default();
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { return };
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            eprintln!("<4>niri said what isn't JSON: {line}");
            continue;
        };
        if let Some(why) = event.get("Err") {
            eprintln!("<3>niri refused the event stream: {why}");
            return;
        }
        if let Some(timing) = times.heard(&event, timed::now, timed::started) {
            timed::say(timing);
        }
        if let Some(unasked) = watched.heard(&event, unasked::cgroup, unasked::folder) {
            eprintln!("{}", unasked::line(&unasked));
        }
        // A screen coming or going changes the workspaces, so its outputs
        // are asked again then, before the lock is taken, so no ask waits
        // on niri; only where rows are made, since nothing else reads
        // them.
        let rowed = lock(&shared.daemon).words.is_some();
        let outputs = (rowed && event.get("WorkspacesChanged").is_some())
            .then(|| screens::asked(niri))
            .flatten();
        let mut daemon = lock(&shared.daemon);
        let answered = daemon.niri != "answering";
        daemon.niri = "answering";
        if event.get("Ok").is_some() {
            if answered {
                refresh(&shared.served, &mut daemon);
            }
            continue;
        }
        daemon.events += 1;
        let moved = outputs.is_some_and(|outputs| {
            let moved = daemon.outputs != outputs;
            daemon.outputs = outputs;
            moved
        });
        if daemon.desktop.hear(&event) == Heard::Changed || moved || answered {
            refresh(&shared.served, &mut daemon);
        }
        order::put_back(niri, daemon.desktop.put_back());
    }
}

/// Whether a record is listed before others: windows first, the one used
/// last first, then everything else as it was made, so a whole answer is
/// in the order the shell lists it.
fn listed(record: &Value) -> (bool, std::cmp::Reverse<(u64, u64)>) {
    let window = DOMAIN.kind_of(record["key"].as_str().unwrap_or("")) == Some("window");
    let used = (
        record["order"].as_u64().unwrap_or(0),
        record["id"].as_u64().unwrap_or(0),
    );
    (
        !window,
        std::cmp::Reverse(if window { used } else { (0, 0) }),
    )
}

/// Works every record out again and hands them all to what serves them,
/// which tells each watch what changed of what it wants.
fn refresh(served: &Served, daemon: &mut Daemon) {
    let Daemon {
        desktop,
        entries,
        names,
        outputs,
        each,
        words,
        desk,
        light,
        rows,
        ..
    } = &mut *daemon;
    let mut records = desktop.records(entries, names);
    shows::add(&mut records, shows::folder().as_deref());
    records.push(open::record(&records));
    records.sort_by_key(listed);
    if let Some(words) = words {
        let held = light.light();
        let seen = Seen {
            light: &held,
            reading: light.reading,
            now: now_ms(),
            looked: light.looking,
        };
        rows.take(
            screens::rows(outputs, desk, desktop.output(), *each, &seen, words),
            Vec::new(),
        );
        records.extend(rows.all().cloned());
    }
    served.change(records, sources(daemon));
}

/// The entries changed: each is read again, every app id looked up again,
/// and the windows whose program that changed told.
fn entries_changed(shared: &Shared) {
    let (entries, _) = Entries::read(&shared.folders);
    let mut daemon = lock(&shared.daemon);
    daemon.entries = entries;
    daemon.names.forget();
    refresh(&shared.served, &mut daemon);
}

/// What desk-facts says of each screen in its folder, or nothing where it
/// has written nothing yet.
fn read_desk(folder: &Path) -> BTreeMap<String, Value> {
    std::fs::read_to_string(folder.join(screens::DESK))
        .map(|said| screens::desk(&said))
        .unwrap_or_default()
}

/// Hears desk-facts write each screen's EDID again, as when one is
/// plugged in, and has the screens' drawers say it. desk-facts writes the
/// file whole and renames it into place, so a reading never meets it half
/// written; the folder is made here where desk-facts hasn't yet, since it
/// removes only its files when it stops.
fn follow_desk(shared: &Shared, folder: &Path) {
    let notify = match Notify::new() {
        Ok(notify) => notify,
        Err(why) => {
            eprintln!("<4>inotify isn't there, so desk-facts writing isn't heard: {why}");
            return;
        }
    };
    let _ = std::fs::create_dir_all(folder);
    if notify.watch(folder).is_none() {
        eprintln!(
            "<4>couldn't watch {}, so desk-facts writing isn't heard",
            folder.display()
        );
        return;
    }
    // Read again once watched, so nothing written before the watch is
    // missed.
    desk_changed(shared, folder);
    while let Ok(changes) = notify.changes() {
        if changes.iter().any(|(_, name)| name == screens::DESK) {
            desk_changed(shared, folder);
        }
    }
}

/// desk-facts wrote its file: the screens' drawers say it where it
/// changed.
fn desk_changed(shared: &Shared, folder: &Path) {
    let desk = read_desk(folder);
    let mut daemon = lock(&shared.daemon);
    if daemon.desk != desk {
        daemon.desk = desk;
        refresh(&shared.served, &mut daemon);
    }
}

/// The watches changed: where a screen has come to be looked at, the
/// light's thread is woken and the drawer says what's held of it, not yet
/// read; where none is any more, the drawer says no light.
fn looks_changed(shared: &Shared, daemon: &mut Daemon, looking: bool) {
    if daemon.light.look(looking) {
        shared.looked.notify_all();
        refresh(&shared.served, daemon);
    }
}

/// Watches the screen's light on the machine daemon's socket for as long
/// as a screen is looked at, and waits for the next look between.
fn follow_light(shared: &Shared, socket: &Path) {
    read_light(shared, socket);
    let mut warned = false;
    loop {
        let looked = shared
            .looked
            .wait_while(lock(&shared.daemon), |daemon| !daemon.light.looking);
        drop(looked);
        match UnixStream::connect(socket) {
            Ok(stream) => {
                warned = false;
                watch_light(shared, stream);
            }
            Err(why) => {
                if !warned {
                    eprintln!(
                        "<4>the machine daemon isn't answering on {}: {why}",
                        socket.display()
                    );
                    warned = true;
                }
                std::thread::sleep(windows::light::WAITED);
            }
        }
        // A look the machine daemon didn't answer, or stopped answering,
        // says the light isn't read, and asks again in a moment.
        let mut daemon = lock(&shared.daemon);
        daemon.light.stream = None;
        if daemon.light.looking {
            daemon.light.lapsed(true);
            refresh(&shared.served, &mut daemon);
            drop(daemon);
            std::thread::sleep(MACHINE_AGAIN);
        }
    }
}

/// Reads the light once as the daemon starts, though nothing looks, so
/// the Warmth inside the screen holding it is served from the first, as
/// the shell read it once as it started (Displays.qml, until 2026-11-14);
/// the machine daemon is hung up on at its first answer.
fn read_light(shared: &Shared, socket: &Path) {
    let Ok(mut stream) = UnixStream::connect(socket) else {
        return;
    };
    let mut line = String::new();
    let read = stream.set_read_timeout(Some(ASKING)).is_ok()
        && writeln!(stream, "{}", Held::watch()).is_ok()
        && BufReader::new(&stream)
            .read_line(&mut line)
            .is_ok_and(|read| read > 0);
    let _ = stream.shutdown(Shutdown::Both);
    let mut daemon = lock(&shared.daemon);
    if read && daemon.light.heard(&line) {
        refresh(&shared.served, &mut daemon);
    }
}

/// Hears the light on one connection until it ends or nothing looks: its
/// first second unread said as nothing, then as not read yet, and the
/// drawer said again at the night light's sunset or sunrise, which the
/// clock alone changes.
fn watch_light(shared: &Shared, mut stream: UnixStream) {
    let (Ok(reading), Ok(hangup)) = (stream.try_clone(), stream.try_clone()) else {
        return;
    };
    {
        let mut daemon = lock(&shared.daemon);
        if !daemon.light.looking {
            return;
        }
        daemon.light.stream = Some(hangup);
    }
    if writeln!(stream, "{}", Held::watch()).is_err() {
        return;
    }
    let began = Instant::now();
    let mut lines = BufReader::new(reading);
    // Kept across a wait that ends mid-line, so no line is lost.
    let mut line = String::new();
    loop {
        let wait = lock(&shared.daemon).light.wait(began.elapsed(), now_ms());
        if lines.get_ref().set_read_timeout(wait).is_err() {
            return;
        }
        match lines.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {
                let mut daemon = lock(&shared.daemon);
                if daemon.light.heard(&line) {
                    refresh(&shared.served, &mut daemon);
                }
                line.clear();
            }
            Err(why) if matches!(why.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                let mut daemon = lock(&shared.daemon);
                daemon.light.lapsed(false);
                refresh(&shared.served, &mut daemon);
            }
            Err(_) => return,
        }
    }
}

/// Asks niri for its screens again, where rows are made, before the lock
/// is taken so no other ask waits on niri; what moved is told each watch
/// before the ask is answered, so a client watching on the same
/// connection holds the new rows once it is. Whether rows are made.
fn looked(shared: &Shared) -> bool {
    let rowed = lock(&shared.daemon).words.is_some();
    if let Some(outputs) = rowed.then(|| screens::asked(&shared.niri)).flatten() {
        let mut daemon = lock(&shared.daemon);
        if daemon.outputs != outputs {
            daemon.outputs = outputs;
            refresh(&shared.served, &mut daemon);
        }
    }
    rowed
}

fn sources(daemon: &Daemon) -> Value {
    json!({
        "niri": {"state": daemon.niri, "events": daemon.events},
        "entries": {"state": "answering", "count": daemon.entries.len()},
    })
}

/// The daemon's own asks: to look at the screens again, and an act on
/// one; none for an ask it doesn't know, which daemon-serve refuses.
fn owned(shared: &Arc<Shared>, said: &Value) -> Option<Value> {
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    match said["ask"].as_str()? {
        "look" => Some(json!({"id": id, "looked": looked(shared)})),
        "screen" => Some(screen_done(shared, &id, said)),
        "light" => Some(light_done(shared, &id, said)),
        _ => None,
    }
}

/// Has the machine daemon turn the night light on or off, or the warmth
/// follow the room or not, as a switch of the Warmth's asks
/// (light/warmth.rs), and answers as the machine daemon did. The warmth
/// changing is heard as the light is, while the screen is looked at.
fn light_done(shared: &Shared, id: &Value, said: &Value) -> Value {
    let Some(socket) = shared.machine.as_deref() else {
        return error(id, "no-light", "no machine daemon was handed, so no light");
    };
    let act = match said["do"].as_str() {
        Some(act @ "night-light") => {
            json!({"id": id, "ask": "act", "act": act, "on": said["on"]})
        }
        Some(act @ "follow-the-room") => {
            json!({"id": id, "ask": "act", "act": act, "follows": said["follows"]})
        }
        Some(act @ "brightness") => {
            json!({"id": id, "ask": "act", "act": act, "share": said["share"]})
        }
        _ => {
            let why = format!("no light act {} here", said["do"]);
            return error(id, "unknown-act", &why);
        }
    };
    asked_of(socket, &act).unwrap_or_else(|why| {
        let why = format!("the machine daemon didn't answer: {why}");
        error(id, "unanswered", &why)
    })
}

/// One ask of the machine daemon's, on a connection of its own, and its
/// answer.
fn asked_of(socket: &Path, ask: &Value) -> std::io::Result<Value> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(ASKING))?;
    writeln!(stream, "{ask}")?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let _ = stream.shutdown(Shutdown::Both);
    serde_json::from_str(&line).map_err(std::io::Error::other)
}

/// Does the act a screen's row asked, and answers it.
fn screen_done(shared: &Arc<Shared>, id: &Value, said: &Value) -> Value {
    let act = {
        let daemon = lock(&shared.daemon);
        match screens::line::asked(id, said, &daemon.outputs, daemon.words.as_ref()) {
            Ok(act) => act,
            Err(refused) => return refused,
        }
    };
    // Every screen goes dark, as does the last lit one, once the key that
    // asked is up, on a thread of its own so no other ask waits; the ask
    // is answered by then, so the journal says what niri didn't do.
    let waits = match &act {
        Act::Sleep => true,
        Act::Dark(name) => screens::last_lit(&lock(&shared.daemon).outputs, name),
        _ => false,
    };
    if waits {
        let shared = Arc::clone(shared);
        std::thread::spawn(move || {
            std::thread::sleep(acts::KEY_UP);
            if let Some(line) = acts::unheard(&act, acts::done(&shared.niri, &act).as_ref()) {
                eprintln!("{line}");
            }
            looked(&shared);
        });
        return json!({"id": id, "done": true});
    }
    let answer = acts::done(&shared.niri, &act);
    looked(shared);
    let daemon = lock(&shared.daemon);
    let Some(words) = daemon.words.as_ref() else {
        return error(id, "no-rows", "no rows are made, so no acts");
    };
    let named = screens::named_of(&daemon.outputs, act.output().unwrap_or(""), words);
    acts::answered(id, &act, answer.as_ref(), &named, words)
}
