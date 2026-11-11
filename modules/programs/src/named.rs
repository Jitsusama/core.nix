//! What a key names of a folder, the repositories under one, a file's
//! preview or kitty's page, read on the asking thread and kept, each
//! watched as it's read so its change is told rather than asked again.

use std::path::{Path, PathBuf};

use programs::{folder, machines, preview, repositories, visited};
use serde_json::Value;

use super::{Shared, keep, lock};

/// Reads each folder, repository, preview or file of kitty's a key names,
/// on this thread, and keeps its record, watching a folder from the first
/// time it's asked for, the repositories' folders each time they're read,
/// as one is cloned, and each file of kitty's as it's read, and asking a
/// preview again only of a file changed since.
pub fn read_named(shared: &Shared, keys: &[String]) {
    for key in keys {
        let notify = shared.notify.as_ref();
        let record = if let Some(path) = folder::path_of(key) {
            let _ = lock(&shared.daemon)
                .looked
                .watch(notify, key, Path::new(path));
            folder::record(key, path)
        } else if let Some(read) = watching(
            || repositories::read(key),
            |read: &repositories::Read| {
                let looked = &mut lock(&shared.daemon).looked;
                read.heard
                    .iter()
                    .fold(false, |any, folder| looked.watch(notify, key, folder) | any)
            },
        ) {
            read.record
        } else if let Some(record) = watching(
            || kittys(key),
            |record: &Value| {
                let looked = &mut lock(&shared.daemon).looked;
                files_of(record).iter().fold(false, |any, file| {
                    looked.watch_file(notify, key, file) | any
                })
            },
        ) {
            record
        } else if let Some(path) = preview::path_of(key) {
            if preview::current(lock(&shared.daemon).held.get(key), path) {
                continue;
            }
            let Some(record) = lock(&shared.previewer).record(key) else {
                continue;
            };
            record
        } else {
            continue;
        };
        keep(&mut lock(&shared.daemon), vec![record], &[]);
    }
}

/// Reads what a key names until reading it starts no new watch, so what
/// changed between a read and its watch isn't missed, as a folder made
/// while it was read, which is then watched itself. A path is only so
/// deep, so the reads stop; past `READS` the last stands.
fn watching<T>(read: impl Fn() -> Option<T>, watch: impl Fn(&T) -> bool) -> Option<T> {
    const READS: usize = 8;
    let mut held = read()?;
    for _ in 0..READS {
        if !watch(&held) {
            break;
        }
        match read() {
            Some(again) => held = again,
            None => break,
        }
    }
    Some(held)
}

/// What kitty's page reads of zoxide, ssh and zsh's history, by its key.
fn kittys(key: &str) -> Option<Value> {
    visited::record(key).or_else(|| machines::record(key))
}

/// The files a record of kitty's page was read from, each heard alone in
/// its folder: zoxide's database, ssh's config and `known_hosts`, and
/// zsh's history.
fn files_of(record: &Value) -> Vec<PathBuf> {
    let field = |name: &str| record[name].as_str().map(PathBuf::from);
    let ssh = field("folder").map(|folder| [folder.join("config"), folder.join("known_hosts")]);
    [field("database"), field("history")]
        .into_iter()
        .flatten()
        .chain(ssh.into_iter().flatten())
        .collect()
}
