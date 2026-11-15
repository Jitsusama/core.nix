//! What each watch a client holds is told, and when: each change
//! gathered and said at most every GATHERED, and a watch of those listed
//! told whole once every source has said what it will of them.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use people::listing::Told;
use people::{Book, address};
use serde_json::{Value, json};

use super::{Daemon, Shared, ask_ahead, clock, lock};

/// How often what changed is said to each watch.
const GATHERED: Duration = Duration::from_millis(250);

/// How long a watch of those listed waits, at most, to be told whole
/// until every source that can say something of each listed has said it
/// or put them off.
const WHOLE_AT_MOST: Duration = Duration::from_secs(2);

/// One watch a client holds.
pub(super) struct Watch {
    pub(super) connection: u64,
    pub(super) id: Value,
    /// The keys watched, in the order asked, or none for everyone.
    pub(super) keys: Option<Vec<String>>,
    /// Whether, watching everyone, it's only those a source lists.
    pub(super) listed: bool,
    pub(super) told: Sender<String>,
    /// What changed since it was last told, by key.
    pub(super) pending: HashMap<String, Value>,
    /// For a watch of those listed, what it was last told, and the book's
    /// revision then, so it's told again only once something changed.
    pub(super) listing: Option<(Told, u64)>,
    /// For a watch of those listed not yet told whole, when it's told
    /// whole even with a source still to answer.
    pub(super) whole_by: Option<Instant>,
}

/// Everyone a watch names, and everyone a watch of those listed was last
/// told of, as a source is asked ahead about once it can answer again.
pub(super) fn watched(daemon: &Daemon) -> Vec<String> {
    daemon
        .watches
        .iter()
        .filter_map(|watch| {
            watch
                .keys
                .as_ref()
                .or_else(|| watch.listing.as_ref().map(|(told, _)| &told.listed))
        })
        .flatten()
        .cloned()
        .collect()
}

/// Says what changed to each watch, at most every GATHERED.
pub(super) fn gather(shared: &Shared) {
    loop {
        std::thread::sleep(GATHERED);
        say_gathered(&mut lock(shared));
    }
}

/// Tells each watch what changed since it was last told, dropping one
/// that's gone.
pub(super) fn say_gathered(daemon: &mut Daemon) {
    let Daemon {
        watches,
        book,
        sources,
        named,
        holds,
        revision,
        put_off,
        ..
    } = daemon;
    let at = clock();
    let mut listed = Vec::new();
    watches.retain_mut(|watch| {
        if let Some(by) = watch.whole_by {
            let mut said = Told::default();
            said.changes(book, sources, named, holds, at);
            listed.extend(said.listed.iter().cloned());
            let all_said = !said.listed.is_empty()
                && said.listed.iter().all(|key| settled(book, put_off, key));
            if !all_said && Instant::now() < by {
                watch.listing = Some((said, *revision));
                return true;
            }
            let line = json!({"id": watch.id, "records": said.all()});
            watch.listing = Some((said, *revision));
            watch.whole_by = None;
            return watch.told.send(line.to_string()).is_ok();
        }
        if let Some((told, seen)) = &mut watch.listing {
            // Told again once the book changed, or once the clock reaches
            // something with someone starting or ending.
            if *seen == *revision && told.until.is_none_or(|until| until > at) {
                return true;
            }
            *seen = *revision;
            let moved = told.changes(book, sources, named, holds, at);
            if moved.is_empty() {
                return true;
            }
            listed.extend(told.listed.iter().cloned());
            let line = json!({"id": watch.id, "changed": moved.changed, "gone": moved.gone});
            return watch.told.send(line.to_string()).is_ok();
        }
        if watch.pending.is_empty() {
            return true;
        }
        let changed: Vec<Value> = watch.pending.drain().map(|(_, record)| record).collect();
        let line = json!({"id": watch.id, "changed": changed}).to_string();
        watch.told.send(line).is_ok()
    });
    // Whoever is listed anew is asked ahead of, as everyone listed was as
    // the watch began.
    ask_ahead(daemon, &listed);
}

/// Whether every source that can say something of a listed key has said
/// it or put it off, so a listing told whole now won't change for it as
/// soon as the next source answers. One with no address can't be asked.
fn settled(book: &Book, put_off: &HashMap<String, HashSet<String>>, key: &str) -> bool {
    let Some(address) = address(key) else {
        return true;
    };
    book.waiting(key)
        .all(|source| put_off.get(source).is_some_and(|put| put.contains(address)))
}

/// Those listed, in order, as a listing asks: answered once, or watched,
/// told whole now and after only who changed.
pub(super) fn listing(
    shared: &Shared,
    id: Value,
    watch: bool,
    connection: u64,
    told: &Sender<String>,
) -> Option<Value> {
    let mut daemon = lock(shared);
    let mut said = Told::default();
    said.changes(
        &daemon.book,
        &daemon.sources,
        &daemon.named,
        &daemon.holds,
        clock(),
    );
    if !watch {
        return Some(json!({"id": id, "records": said.all()}));
    }
    daemon
        .watches
        .retain(|watch| !(watch.connection == connection && watch.id == id));
    // Everyone listed is asked ahead of, since they're who Joel steps into
    // from the launcher's first rows: Slack is asked who they are and
    // their conversation before he does, so their insides are ready.
    ask_ahead(&mut daemon, &said.listed);
    // Told whole now only once every source has said what it will of
    // those listed; else as they have, or within WHOLE_AT_MOST, in
    // say_gathered. A daemon started again by a switch heard Tuple's
    // part before Slack's answers, and told whole with Tuple's alone,
    // then Slack's, the shell made every listed row twice (2026-11-15).
    // Empty, it waits too, since a roster still coming lists nobody yet.
    let all_said = !said.listed.is_empty() && {
        let daemon = &*daemon;
        said.listed
            .iter()
            .all(|key| settled(&daemon.book, &daemon.put_off, key))
    };
    let whole_by = if all_said {
        // Told under the lock that holds the watch, as a watch of records
        // is, so no change is said before the whole.
        let _ = told.send(json!({"id": id, "records": said.all()}).to_string());
        None
    } else {
        Some(Instant::now() + WHOLE_AT_MOST)
    };
    let seen = daemon.revision;
    daemon.watches.push(Watch {
        connection,
        id,
        keys: None,
        listed: false,
        told: told.clone(),
        pending: HashMap::new(),
        listing: Some((said, seen)),
        whole_by,
    });
    None
}
