//! Records held for the shell and answered over the socket systemd holds
//! for a daemon, lifted from the to-do daemon's serve.rs. The daemon's
//! loop hands it every record whole each time something changes, and
//! each watch is told only the records that changed and the keys that
//! went, in one line. Nothing is read for an ask unless the daemon reads
//! on demand: everything said is otherwise already held.
//!
//! Each connection is served on a thread of its own, its lines written by
//! another, so a client that stops reading holds no other back; one with
//! more than a few hundred lines unread is hung up on, which is said once
//! on standard error, for the daemon's journal.
//!
//! A watch may be paced, as the shut launcher paces Agents': what moves
//! is held back by key and told at most once a pace, all of it in one
//! line, so the rows stay held for a fast opening while the shut shell
//! wakes for one line in ten seconds rather than one in three. A record
//! new to it is told at once, with all that was held back, so a row that
//! arrived while the launcher was shut is there as it opens, and so is
//! all that was held back just before a line saying the sources stand
//! anew, which a watcher reads as its records being current. Asked
//! again under its id for the same records, a watch keeps on at its new
//! pace, telling at once what it held back rather than everything whole,
//! as mycelium's node does (its PROTOCOL.md, The Daemon Interface).
//!
//! A daemon that serves something only while it's watched, as the notices
//! daemon keeps a notice read while Now's Desktop is looked at, is
//! told as each watch begins or ends, and asks which are watched.
//!
//! A daemon that reads on demand, as the programs daemon makes a
//! preview only once it's got, is handed what each get, list or watch
//! names before it's answered (`fresh`), and hands what it read to
//! `change` like anything else, so every watch hears it and no answer
//! says what a watch wasn't told. What a watch reads only after its
//! answer, it starts once the watch is held (`begun`), so a read asking
//! whether it's watched finds it. A record nothing should hold, as the
//! clipboard's history serves the text highlighted now, is read instead
//! as a get names its key (`read_on_get`), and never held, listed or
//! told to a watch.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::asks::{Ask, Domain, VERSION, error, place_of, trimmed};

mod change;
mod demand;
mod hang_up;
mod in_turn;

use demand::joined;
pub use demand::{Begun, Fresh, HealthToo, Read, Wanted};
use hang_up::{Peer, Saying, Unqueued, queue};
use in_turn::Turns;

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// What answers a daemon's own asks: the ask whole, and its answer
/// without its id, or none for an ask it doesn't know.
pub type Own = Box<dyn Fn(&Value) -> Option<Value> + Send + Sync>;

/// What answers those of a daemon's own asks that turn on who asked: the
/// ask whole and the connection it came on, whose peer the kernel names
/// (`SO_PEERPIDFD`), and its answer without its id, as [`Own`] answers.
/// The jobs broker moves only the process asking, so it must know that
/// process from the socket rather than from anything the ask says.
pub type Peered = Box<dyn Fn(&Value, &UnixStream) -> Option<Value> + Send + Sync>;

/// What's told as a watch begins or ends. It's told with every record
/// held, so it may only pass that on, as onto a channel, and never ask
/// anything of what serves it.
pub type Heed = Box<dyn Fn() + Send + Sync>;

/// One watch a client holds.
struct Watch {
    /// Its connection, as a hang-up names it.
    peer: Arc<Peer>,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    places: Option<Vec<String>>,
    /// The fields it's told of each record, as each moves, or all of them.
    fields: Option<Vec<String>>,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
    /// How long what moves waits to be told, or none to tell it at once.
    every: Option<Duration>,
    /// The keys that moved since it was last told, while it's paced.
    moved: BTreeSet<String>,
    /// When what moved is told, once something has.
    due: Option<Instant>,
}

impl Watch {
    /// Whether it's the watch an ask names: on this connection, under
    /// this id, of the same records and fields.
    fn is(&self, connection: u64, asked: &Watch) -> bool {
        self.peer.connection == connection
            && self.id == asked.id
            && self.kinds == asked.kinds
            && self.keys == asked.keys
            && self.places == asked.places
            && self.fields == asked.fields
    }

    /// What it held back, told now: each key that moved as its record now
    /// reads, or as gone; none when nothing moved.
    fn held_back(&mut self, records: &BTreeMap<String, Value>) -> Option<Value> {
        self.due = None;
        if self.moved.is_empty() {
            return None;
        }
        let keys = std::mem::take(&mut self.moved);
        let mut changed = Vec::new();
        let mut gone = Vec::new();
        for key in &keys {
            match records.get(key) {
                Some(record) => changed.push(trimmed(record, self.fields.as_deref())),
                None => gone.push(key.as_str()),
            }
        }
        Some(moved(&self.id, &changed, &gone))
    }

    fn wants(&self, domain: &Domain, key: &str) -> bool {
        if let Some(keys) = &self.keys {
            return keys.iter().any(|one| one == key);
        }
        let kind = domain
            .kind_of(key)
            .is_some_and(|kind| self.kinds.iter().any(|k| k == kind));
        let place = self.places.as_ref().is_none_or(|places| {
            place_of(key).is_some_and(|place| places.iter().any(|one| one == place))
        });
        kind && place
    }
}

struct Held {
    /// Every record, by key.
    records: BTreeMap<String, Value>,
    /// Their keys in the order the daemon gave them, which a list and a
    /// whole answer keep within each kind, as windows are listed the one
    /// used last first.
    given: Vec<String>,
    /// How each source stands, in the interface's words.
    sources: Value,
    watches: Vec<Watch>,
    /// When it started, in milliseconds since the epoch.
    since: u64,
}

impl Held {
    /// Takes how the sources stand, answering it in the interface's
    /// words where that changed.
    fn stand(&mut self, sources: Value) -> Option<Value> {
        let standing = (words(&self.sources) != words(&sources)).then(|| words(&sources));
        self.sources = sources;
        standing
    }

    /// Drops the watches `going` holds of, answering whether any went.
    fn drop_watches(&mut self, going: impl Fn(&Watch) -> bool) -> bool {
        let before = self.watches.len();
        self.watches.retain(|watch| !going(watch));
        self.watches.len() != before
    }
}

/// The line telling a watch what moved: the records that changed, and
/// the keys that went, when any did.
fn moved(id: &Value, changed: &[Value], gone: &[&str]) -> Value {
    let mut told = json!({"id": id, "changed": changed});
    if !gone.is_empty() {
        told["gone"] = json!(gone);
    }
    told
}

/// What every connection shares with the daemon's loop.
pub struct Shared {
    domain: Domain,
    own: Own,
    held: Mutex<Held>,
    /// Woken as a paced watch first holds something back, so what tells
    /// it sleeps no longer than its pace.
    paced: Condvar,
    /// What's told as a watch begins or ends, once the daemon says.
    heed: OnceLock<Heed>,
    /// The daemon's own asks answered each on a thread of its own.
    slow: OnceLock<&'static [&'static str]>,
    /// The daemon's own asks answered in turn, off their connection's
    /// reading thread (`in_turn.rs`).
    in_turn: OnceLock<&'static [&'static str]>,
    /// The daemon's own asks answered knowing their connection, and what
    /// answers them.
    peered: OnceLock<(&'static [&'static str], Peered)>,
    /// What reads what an ask wants first, once the daemon says.
    fresh: OnceLock<Fresh>,
    /// What reads what a watch wants once it's begun, once the daemon
    /// says.
    begun: OnceLock<Begun>,
    /// What else health says, once the daemon says.
    health_too: OnceLock<HealthToo>,
    /// The kinds read as a get names them, and what reads them.
    read: OnceLock<(&'static [&'static str], Read)>,
    /// What writes each hang-up's line where a case hears it, when one
    /// says; `saying` otherwise.
    said: OnceLock<Saying>,
    /// What writes each hang-up's line to standard error.
    saying: Saying,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared
        .held
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

impl Shared {
    /// What serves `domain`, its own asks answered by `own`.
    pub fn new(domain: Domain, own: Own) -> Arc<Self> {
        let saying = Saying::to(domain.name, std::io::stderr());
        Arc::new(Shared {
            domain,
            own,
            held: Mutex::new(Held {
                records: BTreeMap::new(),
                given: Vec::new(),
                sources: json!({}),
                watches: Vec::new(),
                since: now_ms(),
            }),
            paced: Condvar::new(),
            heed: OnceLock::new(),
            slow: OnceLock::new(),
            in_turn: OnceLock::new(),
            peered: OnceLock::new(),
            fresh: OnceLock::new(),
            begun: OnceLock::new(),
            health_too: OnceLock::new(),
            read: OnceLock::new(),
            said: OnceLock::new(),
            saying,
        })
    }

    /// Tells `heed` each time a watch begins or ends, from then on; only
    /// the first a daemon gives is kept.
    pub fn on_watches(&self, heed: Heed) {
        let _ = self.heed.set(heed);
    }

    /// Answers these of the daemon's own asks each on a thread of its
    /// own, from then on, so one that waits, as a screen share waits on
    /// the desktop's portal for Joel to pick, holds up no ask after it on
    /// its connection; only the first a daemon gives is kept.
    pub fn slow(&self, asks: &'static [&'static str]) {
        let _ = self.slow.set(asks);
    }

    /// Answers these of the daemon's own asks with `answer`, which is
    /// handed the connection each came on, from then on; only the first a
    /// daemon gives is kept. Each is answered on its connection's thread,
    /// on one of its own when it's slow too, or in turn when it's that.
    pub fn peered(&self, asks: &'static [&'static str], answer: Peered) {
        let _ = self.peered.set((asks, answer));
    }

    fn is_slow(&self, said: &Value) -> bool {
        let asked = said["ask"].as_str().unwrap_or("");
        self.slow.get().is_some_and(|slow| slow.contains(&asked))
    }

    /// Whether any watch is of `kind` in `place`, or of every place.
    pub fn watches(&self, kind: &str, place: &str) -> bool {
        lock(self).watches.iter().any(|watch| {
            watch.keys.is_none()
                && watch.kinds.iter().any(|one| one == kind)
                && watch
                    .places
                    .as_ref()
                    .is_none_or(|places| places.iter().any(|one| one == place))
        })
    }

    /// Every key a watch names by key, as the windows daemon reads its
    /// screens' light only while a screen's row is watched ahead.
    pub fn keys_watched(&self) -> BTreeSet<String> {
        let held = lock(self);
        let keyed = held.watches.iter().filter_map(|watch| watch.keys.as_ref());
        keyed.flatten().cloned().collect()
    }

    fn heeded(&self, moved: bool) {
        if let Some(heed) = self.heed.get().filter(|_| moved) {
            heed();
        }
    }

    /// Tells each paced watch what it held back once its pace is up, for
    /// as long as the program runs: asleep until the next is due, and for
    /// as long as nothing is held back, so a daemon whose watches are all
    /// told at once keeps no clock.
    fn pace(&self) {
        let mut guard = lock(self);
        loop {
            let held: &mut Held = &mut guard;
            let now = Instant::now();
            let mut stuck = Vec::new();
            for watch in &mut held.watches {
                if watch.due.is_some_and(|due| due <= now)
                    && let Some(line) = watch.held_back(&held.records)
                    && let Err(why) = queue(&watch.told, &line)
                {
                    stuck.push((Arc::clone(&watch.peer), why));
                }
            }
            self.hang_up(held, &stuck, "a paced watch was told what it held back");
            let next = held.watches.iter().filter_map(|watch| watch.due).min();
            guard = match next {
                Some(due) => {
                    let wait = due.saturating_duration_since(Instant::now());
                    self.paced
                        .wait_timeout(guard, wait)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
                None => self
                    .paced
                    .wait(guard)
                    .unwrap_or_else(PoisonError::into_inner),
            };
        }
    }
}

/// The socket systemd holds for the daemon, so the shell never finds it
/// missing while the daemon starts or starts again; or, run by hand, its
/// own, in a folder only Joel reads, replacing one left behind.
pub fn listen(socket: &Path) -> std::io::Result<UnixListener> {
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
    // A copy closed on exec, which the descriptor as handed over isn't,
    // so no program the daemon runs holds the socket open after it: one
    // of Tuple's commands starts Tuple's own daemon, which outlives it.
    given.try_clone().ok()
}

/// Takes each connection on a thread of its own, for as long as the
/// program runs, and tells paced watches on another.
pub fn accept(shared: Arc<Shared>, listener: UnixListener) {
    let pacing = Arc::clone(&shared);
    std::thread::spawn(move || pacing.pace());
    std::thread::spawn(move || {
        for (connection, stream) in (1u64..).zip(listener.incoming()) {
            let Ok(stream) = stream else {
                std::thread::sleep(ACCEPT_BACKOFF);
                continue;
            };
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || serve(&shared, stream, connection));
        }
    });
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
    // How the sources stood as this connection's hello said them, so a
    // watch it asks for after is told once they've moved since, when no
    // watch was there to be told.
    let mut greeted: Option<Value> = None;
    // What its hang-up names it by, shared with its watches, its slow asks
    // and its turn, so whichever finds its queue full first says it, and
    // only it.
    let peer = Arc::new(Peer::of(connection, &hangup));
    let mut turns = Turns::default();
    let mut reading = BufReader::new(reading);
    loop {
        let line = match next_line(&mut reading, LONGEST) {
            Next::Line(line) => line,
            Next::Ended => break,
            Next::Overlong => {
                shared.say_overlong(&peer);
                let _ = hangup.shutdown(Shutdown::Both);
                break;
            }
        };
        let asked = shared.domain.ask(&line);
        if let Ask::Own { id, said } = &asked
            && shared.is_slow(said)
        {
            apart(shared, (id, said), &told, &hangup, &peer);
            continue;
        }
        if let Ask::Own { id, said } = &asked
            && shared.is_in_turn(said)
        {
            // Gone only once its turn hung up on it, which says so itself.
            if turns
                .take(shared, (id, said), &told, &hangup, &peer)
                .is_err()
            {
                let _ = hangup.shutdown(Shutdown::Both);
                break;
            }
            continue;
        }
        // The daemon's own asks are answered before the lock is taken, so
        // one handing its ask to the daemon's loop never waits on a change.
        let own = match &asked {
            Ask::Own { id, said } => Some(owned(shared, id, said, &hangup)),
            _ => None,
        };
        // Read on demand outside the lock too, so what's read is handed
        // to change as anything else is, and records read as a get names
        // them, never held, may wait on what the daemon reads them from.
        shared.read_for(&asked);
        let too = shared.said_too(&asked);
        let read = match &asked {
            Ask::Get { keys, .. } => shared.read_now(keys),
            _ => BTreeMap::new(),
        };
        let hello = matches!(asked, Ask::Hello { .. });
        let watch = match &asked {
            Ask::Watch { id, .. } => Some(id.clone()),
            _ => None,
        };
        // What a watch names, said again once it's held and answered, to
        // a daemon that asked to hear it.
        let begins = (watch.is_some() && shared.begun.get().is_some()).then(|| asked.clone());
        let mut held = lock(shared);
        let mut answer = match own {
            Some(own) => Some(own),
            None => match asked {
                Ask::Get { id, keys } => Some(got(&held, &id, &keys, &read)),
                asked => answer(shared, &mut held, asked, &peer, &told, &hangup),
            },
        };
        if let (Some(answer), Some(too)) = (answer.as_mut(), too) {
            joined(answer, too);
        }
        let mut lines: Vec<Value> = answer.into_iter().collect();
        let standing = words(&held.sources);
        if hello {
            greeted = Some(standing);
        } else if let Some(id) = watch
            && greeted.as_ref().is_some_and(|was| *was != standing)
        {
            lines.push(json!({"id": id, "sources": &standing}));
            greeted = Some(standing);
        }
        // Queued while the lock is held, so a change can't be queued before
        // the answer it follows; one that can't be queued is hung up on, as
        // a change would hang up on it. A watch kept on has nothing to say
        // when it held nothing back.
        let queued = lines.iter().try_for_each(|line| queue(&told, line));
        drop(held);
        if let Err(why) = queued {
            shared.say_once(&peer, why, "it was answered");
            // Shut, not just left, since the writer holds the socket open
            // for as long as its write waits on one that never reads.
            let _ = hangup.shutdown(Shutdown::Both);
            break;
        }
        if let Some(begins) = &begins {
            shared.watch_begun(begins);
        }
    }
    let went = lock(shared).drop_watches(|watch| watch.peer.connection == connection);
    shared.heeded(went);
}

/// The longest ask a connection's line may hold, its end not counted, far
/// past any a client asks, so what a connection that never ends a line
/// holds is bounded, at most twice this as its buffer grows.
const LONGEST: usize = 16 << 20;

/// What a connection's next line held.
#[derive(Debug, PartialEq, Eq)]
enum Next {
    /// An ask, without its line's end.
    Line(String),
    /// Nothing more: the connection ended, or sent what isn't words.
    Ended,
    /// More than `longest`, its line ended or not.
    Overlong,
}

/// The next line `reading` holds, as `lines` would give it, reading no
/// more than `longest` and its line's end, a newline or a carriage return
/// and a newline.
fn next_line(reading: &mut impl BufRead, longest: usize) -> Next {
    let mut bytes = Vec::new();
    let most = u64::try_from(longest).unwrap_or(u64::MAX).saturating_add(2);
    match std::io::Read::take(&mut *reading, most).read_until(b'\n', &mut bytes) {
        Ok(0) | Err(_) => return Next::Ended,
        Ok(_) => {}
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.len() > longest {
        return Next::Overlong;
    }
    String::from_utf8(bytes).map_or(Next::Ended, Next::Line)
}

/// One of the daemon's slow asks, answered on a thread of its own once
/// it's done, so nothing asked after it waits on it.
fn apart(
    shared: &Arc<Shared>,
    (id, said): (&Value, &Value),
    told: &SyncSender<String>,
    stream: &Arc<UnixStream>,
    peer: &Arc<Peer>,
) {
    let (shared, told) = (Arc::clone(shared), told.clone());
    let (id, said, stream) = (id.clone(), said.clone(), Arc::clone(stream));
    let peer = Arc::clone(peer);
    std::thread::spawn(move || {
        // One whose answer can't be queued is hung up on here, since the
        // next line it might be told may never come.
        if let Err(why) = queue(&told, &owned(&shared, &id, &said, &stream)) {
            shared.say_once(&peer, why, "a slow ask was answered");
            let _ = stream.shutdown(Shutdown::Both);
        }
    });
}

/// Each source by its word, as every line but health says it: one given
/// with its reason, as `{"state", "reason"}`, by its state.
fn words(sources: &Value) -> Value {
    let Some(sources) = sources.as_object() else {
        return sources.clone();
    };
    let words = sources.iter().map(|(name, said)| {
        let word = said.get("state").unwrap_or(said);
        (name.clone(), word.clone())
    });
    Value::Object(words.collect())
}

/// Each source's state and its reason, as health says them (the
/// interface's health, RSCH-20261029-0E8050): no reason for one given by
/// its word.
fn states(sources: &Value) -> Value {
    let Some(sources) = sources.as_object() else {
        return sources.clone();
    };
    let states = sources.iter().map(|(name, said)| {
        let state = said.get("state").unwrap_or(said);
        let reason = said.get("reason").and_then(Value::as_str).unwrap_or("");
        (name.clone(), json!({"state": state, "reason": reason}))
    });
    Value::Object(states.collect())
}

/// The answer to one of the daemon's own asks, or a refusal of an ask
/// it doesn't know.
fn owned(shared: &Shared, id: &Value, said: &Value, stream: &UnixStream) -> Value {
    let asked = said["ask"].as_str().unwrap_or("");
    let answered = match shared.peered.get() {
        Some((asks, answer)) if asks.contains(&asked) => answer(said, stream),
        _ => (shared.own)(said),
    };
    let Some(mut answer) = answered else {
        let ask = said["ask"].as_str().unwrap_or("");
        return error(id, "unknown-ask", &format!("no ask {ask:?} here"));
    };
    answer["id"] = id.clone();
    answer
}

/// What a get is answered: the records of these keys, in the order asked,
/// each read as it was asked or as it's held.
fn got(held: &Held, id: &Value, keys: &[String], read: &BTreeMap<String, Value>) -> Value {
    let records: Vec<&Value> = keys
        .iter()
        .filter_map(|key| read.get(key).or_else(|| held.records.get(key)))
        .collect();
    json!({"id": id, "records": records})
}

/// The records held of these kinds, those of them in these places, or
/// these keys, in the daemon's order: by kind as the domain lists them,
/// then as the daemon gave them; keys as asked.
fn records_of(
    domain: &Domain,
    held: &Held,
    kinds: &[String],
    keys: Option<&[String]>,
    places: Option<&[String]>,
) -> Vec<Value> {
    if let Some(keys) = keys {
        return keys
            .iter()
            .filter_map(|key| held.records.get(key))
            .cloned()
            .collect();
    }
    domain
        .kinds
        .iter()
        .filter(|kind| kinds.iter().any(|k| k == *kind))
        .flat_map(|kind| {
            held.given
                .iter()
                .filter_map(move |key| held.records.get_key_value(key))
                .filter(move |(key, _)| {
                    domain.kind_of(key) == Some(kind)
                        && places.is_none_or(|places| {
                            place_of(key).is_some_and(|place| places.iter().any(|p| p == place))
                        })
                })
                .map(|(_, record)| record.clone())
        })
        .collect()
}

/// What one ask is answered with, if anything.
fn answer(
    shared: &Shared,
    held: &mut Held,
    asked: Ask,
    peer: &Arc<Peer>,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    let domain = &shared.domain;
    let connection = peer.connection;
    if let Ask::Watch {
        id,
        kinds,
        keys,
        places,
        fields,
        every,
    } = asked
    {
        let watch = Watch {
            peer: Arc::clone(peer),
            id,
            kinds,
            keys,
            places,
            fields,
            told: told.clone(),
            stream: Arc::clone(hangup),
            every,
            moved: BTreeSet::new(),
            due: None,
        };
        if let Some(running) = held
            .watches
            .iter_mut()
            .find(|running| running.is(connection, &watch))
        {
            return kept_on(shared, running, &held.records, every);
        }
        let answer = watched(domain, held, watch);
        shared.heeded(true);
        return Some(answer);
    }
    Some(match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": domain.name, "state": "answering",
            "since": held.since, "sources": words(&held.sources),
        }),
        Ask::Get { id, keys } => got(held, &id, &keys, &BTreeMap::new()),
        Ask::List { id, kinds, places } => {
            let records = records_of(domain, held, &kinds, None, places.as_deref());
            json!({"id": id, "records": records})
        }
        Ask::Watch { .. } => unreachable!("a watch is answered above"),
        Ask::Unwatch { id } => {
            let went =
                held.drop_watches(|watch| watch.peer.connection == connection && watch.id == id);
            shared.heeded(went);
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => json!({
            "id": id, "version": VERSION, "domain": domain.name, "state": "answering",
            "since": held.since, "sources": states(&held.sources),
            "watches": held.watches.len(), "records": held.records.len(),
        }),
        Ask::Own { id, said } => owned(shared, &id, &said, hangup),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}

/// A new watch, answered with every record it wants whole; one asked
/// again by its id for other records replaces the one before.
fn watched(domain: &Domain, held: &mut Held, watch: Watch) -> Value {
    held.watches
        .retain(|was| !(was.peer.connection == watch.peer.connection && was.id == watch.id));
    let (kinds, keys, places) = (&watch.kinds, watch.keys.as_deref(), watch.places.as_deref());
    let records: Vec<Value> = records_of(domain, held, kinds, keys, places)
        .iter()
        .map(|record| trimmed(record, watch.fields.as_deref()))
        .collect();
    let answer = json!({"id": watch.id, "records": records});
    held.watches.push(watch);
    answer
}

/// A watch asked again for the same records, kept on at its new pace:
/// told at once what it held back when it's no longer paced, and
/// otherwise told it when its new pace says, never later than the old.
fn kept_on(
    shared: &Shared,
    running: &mut Watch,
    records: &BTreeMap<String, Value>,
    every: Option<Duration>,
) -> Option<Value> {
    running.every = every;
    let Some(every) = every else {
        return running.held_back(records);
    };
    if let Some(due) = running.due {
        running.due = Some(due.min(Instant::now() + every));
        shared.paced.notify_all();
    }
    None
}

#[cfg(test)]
mod tests;
