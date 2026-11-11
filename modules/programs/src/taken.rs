//! Capture's Recent's rows as this daemon serves them (`taken_rows.rs`),
//! made from the list of what was taken, `listed:shared:taken`, which is
//! watched for as long as a watch wants them, so a new screenshot is
//! heard as inotify says it landed. A row the shell is about to show
//! (Daemon.qml's `ahead`) has what file-preview says of it read on the
//! watch's own thread, never under the lock, and is made again with its
//! drawer whole; file-preview is asked again of it only once the file
//! changed (preview.rs).

use programs::taken_rows::{self, Placing, Taken};
use serde_json::Value;

use super::{Daemon, Run, Shared, keep, lock, read_named, sources_of};

/// The list the rows are made from, as modules/quickshell/home.nix names it.
pub const LIST: &str = "listed:shared:taken";
/// How many Capture's Recent lists, as every Recent does.
const MOST: usize = 9;

/// The words Capture's rows are said in, from the vocabulary at `path`,
/// or none, saying why in the journal.
pub fn words(path: Option<String>) -> Option<taken_rows::Words> {
    let path = path?;
    let words = std::fs::read_to_string(&path)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|why| why.to_string()))
        .and_then(|vocabulary| {
            taken_rows::Words::read(&vocabulary).map_err(|id| format!("it has no {id}"))
        });
    words
        .inspect_err(|why| eprintln!("<4>Capture's rows aren't served from {path}: {why}"))
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
    daemon.watches.iter().any(|watch| match &watch.keys {
        Some(keys) => keys.iter().any(|key| path_of(key).is_some()),
        None => {
            watch.place.as_deref() == Some("taken") && watch.kinds.iter().any(|kind| kind == "row")
        }
    })
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
    make(shared, &mut lock(&shared.daemon), &[]);
}

/// Makes the rows again from the list held and the previews read, and
/// keeps what changed, telling each watch only the rows that read
/// differently and those gone. `also` names the folders a watch about to
/// be held takes them from. With no words handed, none is made.
pub fn make(shared: &Shared, daemon: &mut Daemon, also: &[String]) {
    let Some(words) = &shared.taken_words else {
        return;
    };
    let folders: Vec<String> = daemon
        .watches
        .iter()
        .filter(|watch| watch.place.as_deref() == Some("taken"))
        .flat_map(|watch| watch.folders.iter().cloned())
        .chain(also.iter().cloned())
        .collect();
    let lines = daemon.held.get(LIST).map(|record| record["lines"].clone());
    let taken: Vec<Taken> = taken_rows::listed(
        lines
            .as_ref()
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str),
    );
    let ahead: Vec<&str> = daemon
        .watches
        .iter()
        .filter_map(|watch| watch.keys.as_ref())
        .flatten()
        .filter_map(|key| path_of(key))
        .collect();
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
    let placing = Placing {
        home: &shared.home,
        usual: &format!("{}/Pictures/Screenshots", shared.home),
    };
    let rows = taken_rows::rows(&taken, &folders, MOST, &shown, &placing, words);
    let moved = daemon.taken.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}
