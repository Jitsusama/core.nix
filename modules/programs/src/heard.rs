//! What the daemon hears of its folders through inotify, for as long as
//! it runs: the folders of entries and of file kinds, the profile links a
//! data folder is reached through, each source's file and folders, and the
//! session folder of a profile whose open pages a source reads, and that
//! profile's own folder, for its bookmark files. A
//! part of the daemon's own (main.rs), kept apart from how it answers, so
//! neither file grows past reading.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use programs::bookmark_rows;
use programs::notify::Notify;
use programs::xdg;

use super::{Run, Shared, bookmarks, entries_changed, lock, read_named, start, tabs, watched};

/// What a watched folder is to this daemon; one can be more than one.
#[derive(Default)]
pub struct Heard {
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
    /// The sources whose open pages are read from the session it's the
    /// folder of, whose every change counts, as Chrome's `Sessions`.
    sessions: Vec<usize>,
    /// Whether it's the folder of a profile whose bookmark files are
    /// read, where only a change to one of them counts.
    bookmarks: bool,
}

/// Watches every folder of entries and of file kinds, and the folder of
/// each source's file, answering what each watch is.
pub fn heard(shared: &Shared, notify: &Notify) -> HashMap<i32, Heard> {
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
    // A profile Chrome has never run in has no session folder to hear
    // until the daemon starts again, after the switch or the login that
    // follows; until then each step into it, and each get, reads it still.
    for (at, source) in shared.sources.iter().enumerate() {
        let Some(session) = &source.session else {
            continue;
        };
        if let Some(watch) = notify.watch(&Path::new(&session.profile).join("Sessions")) {
            folders.entry(watch).or_default().sessions.push(at);
        }
        // Chrome writes a bookmark file whole beside it and moves it over,
        // so its name is heard in the profile's folder.
        if let Some(watch) = notify.watch(Path::new(&session.profile)) {
            folders.entry(watch).or_default().bookmarks = true;
        }
    }
    folders
}

/// Hears every change inotify says, for as long as the daemon runs.
pub fn notice(shared: &Arc<Shared>, folders: &HashMap<i32, Heard>) {
    let Some(notify) = &shared.notify else {
        return;
    };
    loop {
        let Ok(changes) = notify.changes() else {
            return;
        };
        let looked: Vec<String> = {
            let daemon = lock(&shared.daemon);
            let keys: HashSet<&String> = changes
                .iter()
                .flat_map(|(watch, name)| daemon.looked.keys_of(*watch, name))
                .filter(|key| watched(&daemon, key))
                .collect();
            keys.into_iter().cloned().collect()
        };
        read_named(shared, &looked);
        let mut entries = false;
        let mut lists = HashSet::new();
        let mut sessions = HashSet::new();
        let mut marked = false;
        for (watch, name) in changes {
            let Some(heard) = folders.get(&watch) else {
                continue;
            };
            marked |= heard.bookmarks && bookmark_rows::FILES.contains(&name.as_str());
            entries |= heard.entries || heard.links.contains(&name);
            lists.extend(
                heard
                    .files
                    .iter()
                    .filter(|(file, _)| *file == name)
                    .map(|(_, at)| *at),
            );
            lists.extend(heard.lists.iter().copied());
            sessions.extend(heard.sessions.iter().copied());
        }
        if entries {
            entries_changed(shared);
        }
        if marked {
            let mut daemon = lock(&shared.daemon);
            if bookmarks::watched(&daemon) {
                bookmarks::make(shared, &mut daemon);
            }
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
        for at in sessions {
            let opened = shared.sources[at].opened_key();
            let daemon = lock(&shared.daemon);
            let watching =
                opened.is_some_and(|key| watched(&daemon, &key)) || tabs::wanted(&daemon);
            drop(daemon);
            if watching {
                start(shared, at, Run::Opened);
            }
        }
    }
}
