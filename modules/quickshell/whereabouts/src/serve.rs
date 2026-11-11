//! Where the laptop is, held for the shell and clock-follows, and answered
//! over the socket systemd holds for it, in version 1 of the shell's daemon
//! interface (interface.rs). One record, `here:laptop`, says it whole: the
//! position GeoClue last found or Joel set, what it's called and the zone
//! it keeps, and beside it the zone the clock keeps and its last move.
//! Given the launcher's words, Location's rows are held beside it, made
//! again from what's known as it changes (rows.rs). Each change is told to
//! every watch at once, in one line, only the records it changed and those
//! it let go, and nothing is read for an ask: everything said is already
//! held. Joel's word on where the laptop is, `set` or `again`, is an act
//! that changes the pin (pin.rs), which the daemon then hears as it hears
//! any change of it.
//!
//! Each connection is served on a thread of its own, its lines written by
//! another, so a client that stops reading holds no other back; one with
//! more than a few hundred lines unread is hung up on.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::fix::Fix;
use crate::interface::{Ask, HERE, Narrowed, VERSION, ask, error, refused};
use crate::pin::{self, Unpinned};
use crate::rows;
use crate::words::Words;

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// What's known, from which every answer is made.
#[derive(Debug, Default)]
pub struct Known {
    /// Where the laptop is, or none while it isn't known.
    pub here: Option<Fix>,
    /// The zone the clock keeps, or none where it can't be read.
    pub clock_zone: Option<String>,
    /// The clock's last move, as clock.rs reads it.
    pub moved: Option<Value>,
    /// How GeoClue stands, in the interface's words, or none on a machine
    /// that doesn't follow the laptop and never asks it.
    pub geoclue: Option<Value>,
    /// How the last asking of a place's name went, likewise.
    pub names: Option<Value>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    keys: Option<Vec<String>>,
    narrowed: Narrowed,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => self.narrowed.holds(key),
        }
    }
}

/// Every record by its key, in the order its keys sort.
type Records = BTreeMap<String, Value>;

struct Held {
    known: Known,
    /// Each record as every watch was last told it.
    said: Records,
    watches: Vec<Watch>,
    /// When it started, in milliseconds since the epoch.
    since: u64,
}

/// What every connection shares.
pub struct Shared {
    held: Mutex<Held>,
    /// The launcher's words, which Location's rows are worded from; none
    /// serves no rows.
    words: Option<Words>,
    /// Where Joel's own word on where the laptop is kept; none where the
    /// laptop isn't followed, which takes no word.
    pin: Option<PathBuf>,
    /// Woken each time the records change, for `turn` to look again at
    /// when they next say something another way.
    turned: Condvar,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared.held.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The one record, made from what's known. Each figure GeoClue doesn't
/// know, as Wi-Fi never knows the altitude, is null.
pub fn record(known: &Known) -> Value {
    let here = known.here.as_ref();
    json!({
        "key": HERE,
        "kind": "here",
        "latitude": here.map(|fix| fix.latitude),
        "longitude": here.map(|fix| fix.longitude),
        "accuracy_m": here.map(|fix| fix.accuracy),
        "at_s": here.map(|fix| fix.when),
        "altitude_m": here.and_then(|fix| fix.altitude),
        "speed_m_s": here.and_then(|fix| fix.speed),
        "heading_deg": here.and_then(|fix| fix.heading),
        "source": here.and_then(|fix| fix.source.as_deref()),
        "place": here.and_then(|fix| fix.place.as_deref()),
        "country": here.and_then(|fix| fix.country.as_deref()),
        "zone": here.and_then(|fix| fix.zone.as_deref()),
        "zone_holds": here.and_then(|fix| fix.zone_holds),
        "hand": here.map(|fix| fix.hand),
        "clock_zone": known.clock_zone,
        "moved": known.moved,
    })
}

/// Every record made from what's known: here, and Location's rows where
/// there are words to say them in.
fn records(known: &Known, words: Option<&Words>, now: u64) -> Records {
    let rows = words
        .map(|words| rows::rows(known.here.as_ref(), known.clock_zone.as_deref(), words, now))
        .unwrap_or_default();
    std::iter::once(record(known))
        .chain(rows)
        .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record)))
        .collect()
}

impl Shared {
    pub fn new(known: Known, words: Option<Words>, pin: Option<PathBuf>) -> Arc<Shared> {
        let said = records(&known, words.as_ref(), now_ms());
        Arc::new(Shared {
            held: Mutex::new(Held {
                known,
                said,
                watches: Vec::new(),
                since: now_ms(),
            }),
            words,
            pin,
            turned: Condvar::new(),
        })
    }

    /// Changes what's known, and tells each watch the records it wants
    /// that changed and those let go, telling a watch none of them
    /// nothing. A connection whose lines aren't being read is hung up on,
    /// and its watches dropped, rather than told less than it asked for.
    pub fn change(&self, how: impl FnOnce(&mut Known)) {
        let mut held = lock(self);
        how(&mut held.known);
        let now = records(&held.known, self.words.as_ref(), now_ms());
        let changed: Vec<&Value> = now
            .iter()
            .filter(|(key, record)| held.said.get(*key) != Some(*record))
            .map(|(_, record)| record)
            .collect();
        let gone: Vec<&String> = held
            .said
            .keys()
            .filter(|key| !now.contains_key(*key))
            .collect();
        if changed.is_empty() && gone.is_empty() {
            return;
        }
        let mut stuck = Vec::new();
        for watch in &held.watches {
            let told: Vec<&&Value> = changed
                .iter()
                .filter(|record| record["key"].as_str().is_some_and(|key| watch.wants(key)))
                .collect();
            let let_go: Vec<&&String> = gone.iter().filter(|key| watch.wants(key)).collect();
            if told.is_empty() && let_go.is_empty() {
                continue;
            }
            let mut line = json!({"id": watch.id, "changed": told});
            if !let_go.is_empty() {
                line["gone"] = json!(let_go);
            }
            if watch.told.try_send(line.to_string()).is_err() {
                stuck.push(watch.connection);
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        held.said = now;
        held.watches
            .retain(|watch| !stuck.contains(&watch.connection));
        self.turned.notify_all();
    }

    /// Joel's word on where the laptop is, answered once it's kept: `set`
    /// waits on the web to find the town he named, so it's asked without
    /// holding what every other connection reads. What it changes is told
    /// as the pin is heard changing, as when it's changed by hand.
    fn act(&self, id: &Value, act: &str, words: &str) -> Value {
        let Some(pin) = self.pin.as_deref() else {
            return error(id, "not-following", "the laptop isn't followed here");
        };
        let done = match act {
            "again" => pin::again(pin).map_err(|why| error(id, "refused", &why.to_string())),
            "set" if words.trim().is_empty() => Err(error(id, "bad-act", "set names no place")),
            "set" => {
                pin::set(pin, words.trim()).map_err(|unpinned| self.unpinned(id, &unpinned, words))
            }
            _ => Err(error(id, "unknown-act", &format!("no act {act:?} here"))),
        };
        match done {
            Ok(()) => json!({"id": id, "done": true}),
            Err(refusal) => refusal,
        }
    }

    /// Why the town Joel named wasn't pinned: what went wrong, and as the
    /// launcher's foot says it, in Location's words (the phrase's id where
    /// none were handed over). A pin that couldn't be kept says why on the
    /// journal too, which isn't his to read.
    fn unpinned(&self, id: &Value, unpinned: &Unpinned, words: &str) -> Value {
        let words = words.trim();
        let (code, message, phrase) = match unpinned {
            Unpinned::Nowhere => ("nowhere", format!("nothing is called {words:?}"), "nowhere"),
            Unpinned::Unasked => (
                "unasked",
                format!("{words:?} couldn't be looked up"),
                "unasked",
            ),
            Unpinned::Unkept(why) => {
                let message = format!("the place found for {words:?} wasn't kept: {why}");
                eprintln!("<4>{message}");
                ("refused", message, "unkept")
            }
        };
        let said = self.words.as_ref().map_or_else(
            || phrase.to_owned(),
            |said| said.said(phrase, &[("words", words)]),
        );
        refused(id, code, &message, &said)
    }
}

/// Says each record again as it next says something another way though
/// nothing else changed, as when it was found turns from how long ago to
/// its day a day on, and otherwise sleeps until the records change: so it
/// never wakes but to change something.
pub fn turn(shared: &Shared) {
    if shared.words.is_none() {
        return;
    }
    let mut held = lock(shared);
    loop {
        let now = now_ms();
        held = match rows::turns(held.known.here.as_ref(), now) {
            Some(at) if at <= now => {
                drop(held);
                shared.change(|_| ());
                lock(shared)
            }
            Some(at) => {
                let until = Duration::from_millis(at - now);
                shared
                    .turned
                    .wait_timeout(held, until)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
            None => shared
                .turned
                .wait(held)
                .unwrap_or_else(PoisonError::into_inner),
        };
    }
}

/// The socket systemd holds for it (location.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
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
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

/// Takes each connection, for as long as the program runs.
pub fn accept(shared: &Arc<Shared>, listener: &UnixListener) {
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
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
        let asked = ask(&line);
        let acted = match &asked {
            Ask::Act { id, act, words } => Some(shared.act(id, act, words)),
            _ => None,
        };
        let mut held = lock(shared);
        let answer = match acted {
            Some(acted) => acted,
            None => answer(&mut held, asked, connection, &told, &hangup),
        };
        // Queued while the lock is held, so a change can't be queued before
        // the answer it follows; one that can't be queued is hung up on, as
        // a change would hang up on it.
        let queued = told.try_send(answer.to_string()).is_ok();
        drop(held);
        if !queued {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// Each of its sources by name, with its word for how it stands.
fn sources(known: &Known) -> Value {
    let mut sources = serde_json::Map::new();
    sources.insert("clock".into(), json!({"state": "answering"}));
    if let Some(geoclue) = &known.geoclue {
        sources.insert("geoclue".into(), geoclue.clone());
    }
    if let Some(names) = &known.names {
        sources.insert("names".into(), names.clone());
    }
    Value::Object(sources)
}

/// What one ask is answered with.
fn answer(
    held: &mut Held,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Value {
    let narrowed = |held: &Held, narrowed: &Narrowed| -> Vec<Value> {
        held.said
            .iter()
            .filter(|(key, _)| narrowed.holds(key))
            .map(|(_, record)| record.clone())
            .collect()
    };
    match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": "location", "state": "answering",
            "since": held.since, "sources": sources(&held.known),
        }),
        Ask::Get { id, keys } => {
            let records: Vec<&Value> = keys.iter().filter_map(|key| held.said.get(key)).collect();
            json!({"id": id, "records": records})
        }
        Ask::List { id, narrowed: by } => json!({"id": id, "records": narrowed(held, &by)}),
        Ask::Watch {
            id,
            keys,
            narrowed: by,
        } => {
            // A watch asked again by its id replaces the one before.
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            let watch = Watch {
                connection,
                id: id.clone(),
                keys,
                narrowed: by,
                told: told.clone(),
                stream: Arc::clone(hangup),
            };
            let records: Vec<&Value> = held
                .said
                .iter()
                .filter(|(key, _)| watch.wants(key))
                .map(|(_, record)| record)
                .collect();
            let records = json!(records);
            held.watches.push(watch);
            json!({"id": id, "records": records})
        }
        // Answered by `act`, which waits on the web without holding what
        // every connection reads, so it's never asked here.
        Ask::Act { id, act, .. } => error(&id, "unknown-act", &format!("no act {act:?} here")),
        Ask::Unwatch { id } => {
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => json!({
            "id": id, "version": VERSION, "domain": "location", "state": "answering",
            "since": held.since, "sources": sources(&held.known),
            "watches": held.watches.len(),
        }),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nowhere_is_null_and_the_clock_still_said() {
        let known = Known {
            clock_zone: Some("America/Halifax".into()),
            ..Known::default()
        };
        let said = record(&known);
        assert_eq!(said["key"], HERE);
        assert_eq!(said["kind"], "here");
        assert_eq!(said["latitude"], Value::Null);
        assert_eq!(said["hand"], Value::Null);
        assert_eq!(said["clock_zone"], "America/Halifax");
        assert_eq!(said["moved"], Value::Null);
    }

    #[test]
    fn a_fix_is_said_in_its_units() {
        let fix = Fix::new(12.3457, -98.7654, 26_000.0, 1_790_000_000)
            .unwrap()
            .named(
                Some("Kelpwater, Westmark".into()),
                Some("America/Toronto".into()),
            )
            .countried(Some("Canada".into()));
        let said = record(&Known {
            here: Some(fix),
            ..Known::default()
        });
        assert_eq!(said["latitude"], 12.3457);
        assert_eq!(said["accuracy_m"], 26_000.0);
        assert_eq!(said["at_s"], 1_790_000_000_u64);
        assert_eq!(said["place"], "Kelpwater, Westmark");
        assert_eq!(said["country"], "Canada");
        assert_eq!(said["zone"], "America/Toronto");
        assert_eq!(said["hand"], false);
        assert!(said["altitude_m"].is_null(), "Wi-Fi finds no altitude");
        assert!(said["source"].is_null(), "a fix GeoClue named no source of");
    }

    /// A watch held on `shared`, narrowed so, and the lines it's told.
    fn watching(
        shared: &Shared,
        id: &str,
        narrowed: Narrowed,
    ) -> (mpsc::Receiver<String>, UnixStream) {
        let (told, lines) = mpsc::sync_channel(QUEUED);
        let (one, two) = UnixStream::pair().unwrap();
        lock(shared).watches.push(Watch {
            connection: 1,
            id: json!(id),
            keys: None,
            narrowed,
            told,
            stream: Arc::new(one),
        });
        (lines, two)
    }

    fn kelpwater() -> Fix {
        Fix::new(12.3457, -98.7654, 26_000.0, 1_790_000_000)
            .unwrap()
            .named(
                Some("Kelpwater, Westmark".into()),
                Some("America/Toronto".into()),
            )
    }

    #[test]
    fn a_change_that_changes_nothing_tells_nobody() {
        let shared = Shared::new(Known::default(), None, None);
        let (lines, _other) = watching(&shared, "all", Narrowed::default());
        shared.change(|known| known.clock_zone = None);
        assert!(
            lines.try_recv().is_err(),
            "a record unchanged is told to no watch"
        );
        shared.change(|known| known.clock_zone = Some("UTC".into()));
        let line: Value = serde_json::from_str(&lines.try_recv().unwrap()).unwrap();
        assert_eq!(line["id"], "all");
        assert_eq!(line["changed"][0]["clock_zone"], "UTC");
        assert!(line.get("gone").is_none(), "nothing was let go");
    }

    #[test]
    fn a_watch_of_locations_rows_is_told_only_the_rows_that_changed() {
        let shared = Shared::new(Known::default(), Some(crate::words::fixture()), None);
        let place = Narrowed {
            kinds: Some(vec!["row".into()]),
            place: Some("location".into()),
        };
        let (rows, _other) = watching(&shared, "rows", place);
        let (here, _another) = watching(
            &shared,
            "here",
            Narrowed {
                kinds: Some(vec!["here".into()]),
                place: None,
            },
        );
        shared.change(|known| known.here = Some(kelpwater()));
        let line: Value = serde_json::from_str(&rows.try_recv().unwrap()).unwrap();
        let keys: Vec<&str> = line["changed"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row["key"].as_str())
            .collect();
        assert!(
            keys.contains(&"row:location:town"),
            "the town is told: {keys:?}"
        );
        assert!(keys.contains(&"row:location:copy"), "Copy comes: {keys:?}");
        assert!(
            !keys.contains(&"row:location:set"),
            "Set It by Hand, as it stood, isn't told again: {keys:?}"
        );
        assert!(
            keys.iter().all(|key| key.starts_with("row:location:")),
            "only Location's own place is told: {keys:?}"
        );
        let line: Value = serde_json::from_str(&here.try_recv().unwrap()).unwrap();
        assert_eq!(line["changed"].as_array().unwrap().len(), 1, "{line}");
        assert_eq!(line["changed"][0]["key"], HERE);

        shared.change(|known| known.here = None);
        let line: Value = serde_json::from_str(&rows.try_recv().unwrap()).unwrap();
        assert!(
            line["gone"]
                .as_array()
                .is_some_and(|gone| gone.contains(&json!("row:location:copy"))),
            "Copy goes once nowhere is known: {line}"
        );
    }

    #[test]
    fn joels_word_is_refused_where_the_laptop_isnt_followed_or_says_nothing() {
        let unfollowed = Shared::new(Known::default(), None, None);
        assert_eq!(
            unfollowed.act(&json!(1), "again", "")["error"]["code"],
            "not-following"
        );
        let folder = std::env::temp_dir().join(format!("whereabouts-act-{}", std::process::id()));
        let pin = folder.join("pin.json");
        let followed = Shared::new(Known::default(), None, Some(pin.clone()));
        assert_eq!(
            followed.act(&json!(2), "set", "  ")["error"]["code"],
            "bad-act"
        );
        assert_eq!(
            followed.act(&json!(3), "wander", "")["error"]["code"],
            "unknown-act"
        );
        std::fs::create_dir_all(&folder).unwrap();
        kelpwater().by_hand().write(&pin).unwrap();
        assert_eq!(
            followed.act(&json!(4), "again", ""),
            json!({"id": 4, "done": true})
        );
        assert!(!pin.exists(), "finding it again takes the pin away");
        assert_eq!(
            followed.act(&json!(5), "again", ""),
            json!({"id": 5, "done": true}),
            "no pin to take away is done too"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_town_not_pinned_is_refused_in_locations_words() {
        let worded = Shared::new(Known::default(), Some(crate::words::fixture()), None);
        let id = json!(6);
        let refusal = |code: &str, message: &str, said: &str| {
            let error = json!({"code": code, "message": message, "said": said});
            json!({"id": 6, "error": error})
        };
        assert_eq!(
            worded.unpinned(&id, &Unpinned::Nowhere, " Kelpwater "),
            refusal(
                "nowhere",
                "nothing is called \"Kelpwater\"",
                "no place is called Kelpwater"
            )
        );
        assert_eq!(
            worded.unpinned(&id, &Unpinned::Unasked, "Kelpwater"),
            refusal(
                "unasked",
                "\"Kelpwater\" couldn't be looked up",
                "couldn't ask where Kelpwater is"
            )
        );
        assert_eq!(
            worded.unpinned(&id, &Unpinned::Unkept("read-only".into()), "Kelpwater"),
            refusal(
                "refused",
                "the place found for \"Kelpwater\" wasn't kept: read-only",
                "Kelpwater couldn't be set"
            )
        );
        let unworded = Shared::new(Known::default(), None, None);
        assert_eq!(
            unworded.unpinned(&id, &Unpinned::Nowhere, "Kelpwater")["error"]["said"],
            "nowhere",
            "with no words handed over, the phrase's id"
        );
    }
}
