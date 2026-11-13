//! Capture's Recent's rows as this daemon serves them (`taken_rows.rs`),
//! made from the list of what was taken, `listed:shared:taken`, which is
//! watched for as long as a watch wants them, so a new screenshot is
//! heard as inotify says it landed. A row the shell is about to show
//! (Daemon.qml's `ahead`) has what file-preview says of it read on the
//! watch's own thread, never under the lock, and is made again with its
//! drawer whole; file-preview is asked again of it only once the file
//! changed (preview.rs). What's typed inside Recent finds among
//! everything taken (`search`), not only the nine it lists.

use programs::taken_rows::{self, Placing, Taken};
use ready_rows::fuzzy::{Item, Typed, runs};
use serde_json::{Map, Value, json};

use super::{Daemon, Run, Shared, keep, keys_watched, lock, read_named, rows_watched, sources_of};

/// The place these rows are of.
pub const PLACE: &str = "taken";
/// The list the rows are made from, as modules/quickshell/home.nix names it.
pub const LIST: &str = "listed:shared:taken";
/// How many Capture's Recent lists, as every Recent does.
const MOST: usize = 9;
/// How many a search finds where the ask names no `most`: more than a
/// screen holds are never seen, as Capture.qml's `most` says.
pub const FOUND: usize = 50;

/// The launcher's vocabulary at `path`, read once as the daemon starts
/// for Capture's rows and the tabs' both, or none, saying why in the
/// journal.
pub fn vocabulary(path: Option<String>) -> Option<Value> {
    let path = path?;
    std::fs::read_to_string(&path)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|why| why.to_string()))
        .inspect_err(|why| eprintln!("<4>the launcher's words in {path} aren't read: {why}"))
        .ok()
}

/// The words Capture's rows are said in, from the vocabulary, or none,
/// saying why in the journal.
pub fn words(vocabulary: &Value) -> Option<taken_rows::Words> {
    taken_rows::Words::read(vocabulary)
        .inspect_err(|id| eprintln!("<4>Capture's rows aren't served: the words have no {id}"))
        .ok()
}

/// The source of what was taken, which any watch beginning reads too
/// while Capture's rows are watched, or none while they aren't.
pub fn lists(shared: &Shared) -> Vec<usize> {
    if !watched(&lock(&shared.daemon)) {
        return Vec::new();
    }
    sources_of(shared, Run::List, &[], Some(&[LIST.to_owned()]))
}

/// The path a row of these names, or none for another place's.
fn path_of(key: &str) -> Option<&str> {
    key.strip_prefix("row:taken:")
        .filter(|path| path.starts_with('/'))
}

/// Whether a watch wants Capture's rows: of their place, whole, or naming
/// one of them.
pub fn watched(daemon: &Daemon) -> bool {
    rows_watched(daemon, PLACE)
}

/// Reads what file-preview says of each row these keys name, as the shell
/// is about to show it, on this thread, then makes the rows again with
/// their drawers whole.
pub fn ahead(shared: &Shared, keys: &[String]) {
    let previews: Vec<String> = keys
        .iter()
        .filter_map(|key| path_of(key))
        .map(|path| format!("preview:{path}"))
        .collect();
    if previews.is_empty() {
        return;
    }
    read_named(shared, &previews);
    make(shared, &mut lock(&shared.daemon));
}

/// Everything the list held says was taken.
fn listed(daemon: &Daemon) -> Vec<Taken> {
    let lines = daemon.held.get(LIST).map(|record| &record["lines"]);
    taken_rows::listed(
        lines
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str),
    )
}

/// The folder most things are taken into, which a row needn't name.
fn usual(shared: &Shared) -> String {
    format!("{}/Pictures/Screenshots", shared.home)
}

/// Makes the rows again from the list held and the previews read, and
/// keeps what changed, telling each watch only the rows that read
/// differently and those gone, from the folders the settings name. With
/// no words handed, none is made. A thing
/// a search found past the nine (`search`) is made whole too while it's
/// ahead, beside them and with no `order`, so the shell's copy of it
/// takes its drawer whole without it joining Recent's list.
pub fn make(shared: &Shared, daemon: &mut Daemon) {
    let Some(words) = &shared.taken_words else {
        return;
    };
    let folders = daemon.folders.clone();
    let taken = listed(daemon);
    let watched = keys_watched(daemon);
    let ahead: Vec<&str> = watched.iter().filter_map(|key| path_of(key)).collect();
    let held = &daemon.held;
    let shown = |path: &str| -> (Option<Value>, bool) {
        let key = format!("preview:{path}");
        match held.get(&key) {
            // What's held is what was read last, made again by `ahead` once the file's time
            // moves, so it's served as it stands rather than waiting on that read.
            Some(record) if ahead.contains(&path) => {
                let said = Some(record["shown"].clone()).filter(|said| !said.is_null());
                (said, record["answering"] == true)
            }
            _ => (None, true),
        }
    };
    let usual = usual(shared);
    let placing = Placing {
        home: &shared.home,
        usual: &usual,
    };
    let within = taken_rows::within(&taken, &folders);
    let rows = taken_rows::rows(&taken, &folders, MOST, &shown, &placing, words);
    let found = within
        .iter()
        .skip(MOST)
        .filter(|one| ahead.contains(&one.path.as_str()))
        .map(|one| {
            let (said, answering) = shown(&one.path);
            let row = taken_rows::row(one, said.as_ref(), answering, &placing, words);
            json!({"key": row["key"], "kind": "row", "row": row})
        })
        .collect();
    let moved = daemon.taken.take(rows, found);
    keep(daemon, moved.changed, &moved.gone);
}

/// What a search of what was taken for `words` is answered with, as
/// Capture.qml's Taken.named found them before: each thing in Capture's
/// folders whose name the words find, newest first as Recent lists them,
/// up to `most`, so a key costs at most that many matches beyond those
/// passed over by their letters. Each found thing this daemon hasn't
/// served comes with its row, its drawer as the list says it, with no
/// `order`, as ready-rows.md's search says.
pub fn search(shared: &Shared, daemon: &Daemon, id: &Value, words: &str, most: usize) -> Value {
    let Some(said) = &shared.taken_words else {
        return json!({
            "id": id, "words": words,
            "error": {"code": "unserved", "message": "Capture's rows have no words, so none is found"},
        });
    };
    let typed = Typed::of(words);
    let taken = listed(daemon);
    let folders = folders(daemon, &[]);
    let usual = usual(shared);
    let placing = Placing {
        home: &shared.home,
        usual: &usual,
    };
    let mut keys: Vec<String> = Vec::new();
    let mut marks = Map::new();
    let mut rows = Map::new();
    let within = if typed.is_empty() {
        Vec::new()
    } else {
        taken_rows::within(&taken, &folders)
    };
    for one in within {
        if keys.len() >= most {
            break;
        }
        let name = one
            .path
            .rsplit_once('/')
            .map_or(one.path.as_str(), |(_, name)| name);
        let item = Item::of(name);
        // Scored first, which passes over a name without the letters typed
        // before it's matched; only what's found is marked.
        let Some((_, marked)) = item.score(&typed).and_then(|_| item.marked(&typed)) else {
            continue;
        };
        let key = format!("row:taken:{}", one.path);
        marks.insert(key.clone(), json!(runs(&marked)));
        if daemon.taken.get(&key).is_none() {
            rows.insert(
                key.clone(),
                taken_rows::row(one, None, true, &placing, said),
            );
        }
        keys.push(key);
    }
    json!({"id": id, "words": words, "keys": keys, "marks": marks, "rows": rows})
}
